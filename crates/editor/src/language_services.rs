//! Editor-owned local language services.
//!
//! This module deliberately keeps the protocol boundary independent from
//! Workspace.  The bundled providers are syntax-derived and local: they give
//! useful diagnostics, completion, navigation, hover, rename, folding, and
//! formatting without pretending that a language server process is available.
//! A process adapter can implement [`LocalLanguageService`] later without
//! changing the editor-facing contract.

use std::collections::BTreeMap;
use std::fmt;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::thread::{self, JoinHandle};

use serde_json::Value;

use crate::buffer::{BufferSnapshot, Edit, Position, Revision, TextRange};
use crate::language::Language;
use crate::symbols::{document_symbols, DocumentSymbol};
use crate::syntax::{HighlightKind, SyntaxHighlighter};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct LanguageId(Language);

impl LanguageId {
    pub const fn new(language: Language) -> Self {
        Self(language)
    }
    pub const fn language(self) -> Language {
        self.0
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct LanguageServerId(String);

impl LanguageServerId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct DocumentVersion(u64);

impl DocumentVersion {
    pub const fn new(value: u64) -> Self {
        Self(value)
    }
    pub const fn value(self) -> u64 {
        self.0
    }
    pub const fn from_revision(revision: Revision) -> Self {
        Self(revision.value())
    }
}

/// LSP positions use UTF-16 code units, unlike the editor's Unicode-scalar
/// columns.  Mapping is centralized here so providers cannot disagree.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct LspPosition {
    pub line: usize,
    pub character: usize,
}

impl LspPosition {
    pub fn from_position(snapshot: &BufferSnapshot, position: Position) -> Self {
        let line = position.line.min(snapshot.line_count().saturating_sub(1));
        let text = snapshot.line(line);
        let character = text
            .chars()
            .take(position.column)
            .map(char::len_utf16)
            .sum();
        Self { line, character }
    }

    pub fn to_position(self, snapshot: &BufferSnapshot) -> Position {
        let line = self.line.min(snapshot.line_count().saturating_sub(1));
        let text = snapshot.line(line);
        let mut units: usize = 0;
        let mut column: usize = 0;
        for ch in text.chars() {
            let width = ch.len_utf16();
            if units.saturating_add(width) > self.character {
                break;
            }
            units += width;
            column += 1;
        }
        Position::new(line, column)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct LspRange {
    pub start: LspPosition,
    pub end: LspPosition,
}

impl LspRange {
    pub fn from_text_range(snapshot: &BufferSnapshot, range: TextRange) -> Self {
        Self {
            start: LspPosition::from_position(snapshot, snapshot.byte_to_position(range.start)),
            end: LspPosition::from_position(snapshot, snapshot.byte_to_position(range.end)),
        }
    }

    pub fn to_text_range(self, snapshot: &BufferSnapshot) -> TextRange {
        TextRange::new(
            snapshot.position_to_byte(self.start.to_position(snapshot)),
            snapshot.position_to_byte(self.end.to_position(snapshot)),
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum DiagnosticSeverity {
    Error,
    Warning,
    Information,
    Hint,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Diagnostic {
    pub range: LspRange,
    pub severity: DiagnosticSeverity,
    pub message: String,
    pub source: LanguageServerId,
    pub code: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CompletionKind {
    Keyword,
    Function,
    Variable,
    Type,
    Field,
    Snippet,
    Text,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CompletionTextFormat {
    #[default]
    PlainText,
    Snippet,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CompletionItem {
    pub label: String,
    pub kind: CompletionKind,
    pub text_format: CompletionTextFormat,
    pub detail: Option<String>,
    pub insert_text: String,
    pub text_edit_range: Option<LspRange>,
    pub filter_text: Option<String>,
    pub sort_text: Option<String>,
    pub documentation: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CompletionList {
    pub items: Vec<CompletionItem>,
    pub is_incomplete: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HoverContent {
    pub contents: String,
    pub range: Option<LspRange>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NavigationTarget {
    pub name: String,
    /// LSP file URI for remote targets; syntax-derived results stay local and
    /// leave this empty because their provider does not own a file path.
    pub uri: Option<String>,
    pub range: LspRange,
    pub selection_range: LspRange,
}

/// One LSP call-hierarchy symbol. `data` is opaque server state that must be
/// returned unchanged when requesting incoming or outgoing calls.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CallHierarchyItem {
    pub name: String,
    pub kind: u32,
    pub uri: String,
    pub range: LspRange,
    pub selection_range: LspRange,
    pub detail: Option<String>,
    pub tags: Vec<u32>,
    pub data: Option<Value>,
}

/// A caller/callee row and the source ranges that form the call site.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CallHierarchyCall {
    pub item: CallHierarchyItem,
    pub ranges: Vec<LspRange>,
}

/// LSP command exposed by a code lens. Arguments and unresolved server data
/// stay opaque so the Editor can return them to the originating server.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LanguageServiceCommand {
    pub title: String,
    pub command: String,
    pub arguments: Vec<Value>,
}

/// A Code Lens positioned on a document range. Servers may defer the command
/// until `codeLens/resolve` is requested.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CodeLens {
    pub range: LspRange,
    pub command: Option<LanguageServiceCommand>,
    pub data: Option<Value>,
}

/// A server-provided link over a document range. Servers may defer the target
/// until `documentLink/resolve` is requested.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DocumentLink {
    pub range: LspRange,
    pub target: Option<String>,
    pub tooltip: Option<String>,
    pub data: Option<Value>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CodeAction {
    pub title: String,
    pub kind: String,
    pub edits: Vec<Edit>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SemanticToken {
    pub range: LspRange,
    pub token_type: String,
    pub modifiers: Vec<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InlayHintKind {
    Type,
    Parameter,
    Other,
}

/// A virtual inline annotation returned by an LSP provider. Its source
/// position is tied to the request snapshot and it never edits the buffer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InlayHint {
    pub position: LspPosition,
    pub label: String,
    pub kind: InlayHintKind,
    pub tooltip: Option<String>,
    pub padding_left: bool,
    pub padding_right: bool,
}

/// sRGB color channels returned by LSP `textDocument/documentColor`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DocumentColorValue {
    pub red: u8,
    pub green: u8,
    pub blue: u8,
    pub alpha: u8,
}

/// A source range associated with a color preview by a language server.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DocumentColor {
    pub range: LspRange,
    pub color: DocumentColorValue,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FoldingRange {
    pub start_line: usize,
    pub end_line: usize,
    pub kind: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SignatureHelp {
    pub label: String,
    pub documentation: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum LanguageServiceCapability {
    Diagnostics,
    Completion,
    SignatureHelp,
    Hover,
    Definition,
    Declaration,
    Implementation,
    TypeDefinition,
    References,
    CallHierarchy,
    CodeLens,
    CodeLensResolve,
    ExecuteCommand,
    DocumentLinks,
    DocumentLinkResolve,
    Rename,
    CodeActions,
    SemanticTokens,
    InlayHints,
    DocumentColors,
    DocumentSymbols,
    FoldingRanges,
    Formatting,
}

/// Edit-oriented language-service capabilities that are intentionally kept
/// separate from the existing document-service protocol.  The latter is also
/// consumed by the stdio process adapter; this contract lets the local editor
/// provider grow without making an unsupported remote operation look like a
/// document-formatting request.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum LanguageServiceEditCapability {
    FormatSelection,
    OrganizeImports,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LanguageServiceEditCapabilities {
    pub format_selection: bool,
    pub organize_imports: bool,
}

impl LanguageServiceEditCapabilities {
    pub const NONE: Self = Self {
        format_selection: false,
        organize_imports: false,
    };

    pub const SYNTAX_DERIVED: Self = Self {
        format_selection: true,
        organize_imports: false,
    };

    pub fn supports(self, capability: LanguageServiceEditCapability) -> bool {
        match capability {
            LanguageServiceEditCapability::FormatSelection => self.format_selection,
            LanguageServiceEditCapability::OrganizeImports => self.organize_imports,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LanguageServiceCapabilities {
    pub diagnostics: bool,
    pub completion: bool,
    pub signature_help: bool,
    pub hover: bool,
    pub definition: bool,
    pub declaration: bool,
    pub implementation: bool,
    pub type_definition: bool,
    pub references: bool,
    pub call_hierarchy: bool,
    pub code_lens: bool,
    pub code_lens_resolve: bool,
    pub execute_command: bool,
    pub document_links: bool,
    pub document_link_resolve: bool,
    pub rename: bool,
    pub code_actions: bool,
    pub semantic_tokens: bool,
    pub inlay_hints: bool,
    pub document_colors: bool,
    pub document_symbols: bool,
    pub folding_ranges: bool,
    pub formatting: bool,
}

impl LanguageServiceCapabilities {
    pub const SYNTAX_DERIVED: Self = Self {
        diagnostics: true,
        completion: true,
        signature_help: false,
        hover: true,
        definition: true,
        declaration: false,
        implementation: false,
        type_definition: false,
        references: true,
        call_hierarchy: false,
        code_lens: false,
        code_lens_resolve: false,
        execute_command: false,
        document_links: false,
        document_link_resolve: false,
        rename: true,
        code_actions: true,
        semantic_tokens: true,
        inlay_hints: false,
        document_colors: false,
        document_symbols: true,
        folding_ranges: true,
        formatting: true,
    };

    pub fn supports(self, capability: LanguageServiceCapability) -> bool {
        match capability {
            LanguageServiceCapability::Diagnostics => self.diagnostics,
            LanguageServiceCapability::Completion => self.completion,
            LanguageServiceCapability::SignatureHelp => self.signature_help,
            LanguageServiceCapability::Hover => self.hover,
            LanguageServiceCapability::Definition => self.definition,
            LanguageServiceCapability::Declaration => self.declaration,
            LanguageServiceCapability::Implementation => self.implementation,
            LanguageServiceCapability::TypeDefinition => self.type_definition,
            LanguageServiceCapability::References => self.references,
            LanguageServiceCapability::CallHierarchy => self.call_hierarchy,
            LanguageServiceCapability::CodeLens => self.code_lens,
            LanguageServiceCapability::CodeLensResolve => self.code_lens_resolve,
            LanguageServiceCapability::ExecuteCommand => self.execute_command,
            LanguageServiceCapability::DocumentLinks => self.document_links,
            LanguageServiceCapability::DocumentLinkResolve => self.document_link_resolve,
            LanguageServiceCapability::Rename => self.rename,
            LanguageServiceCapability::CodeActions => self.code_actions,
            LanguageServiceCapability::SemanticTokens => self.semantic_tokens,
            LanguageServiceCapability::InlayHints => self.inlay_hints,
            LanguageServiceCapability::DocumentColors => self.document_colors,
            LanguageServiceCapability::DocumentSymbols => self.document_symbols,
            LanguageServiceCapability::FoldingRanges => self.folding_ranges,
            LanguageServiceCapability::Formatting => self.formatting,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LanguageServiceRequest {
    Diagnostics,
    Completion {
        position: LspPosition,
        trigger: Option<char>,
    },
    SignatureHelp {
        position: LspPosition,
        trigger: Option<char>,
    },
    Hover {
        position: LspPosition,
    },
    Definition {
        position: LspPosition,
    },
    Declaration {
        position: LspPosition,
    },
    Implementation {
        position: LspPosition,
    },
    TypeDefinition {
        position: LspPosition,
    },
    References {
        position: LspPosition,
    },
    PrepareCallHierarchy {
        position: LspPosition,
    },
    IncomingCalls {
        item: CallHierarchyItem,
    },
    OutgoingCalls {
        item: CallHierarchyItem,
    },
    CodeLenses,
    ResolveCodeLens {
        lens: CodeLens,
    },
    ExecuteCommand {
        command: String,
        arguments: Vec<Value>,
    },
    DocumentLinks,
    ResolveDocumentLink {
        link: DocumentLink,
    },
    Rename {
        position: LspPosition,
        new_name: String,
    },
    CodeActions {
        range: LspRange,
    },
    SemanticTokens,
    InlayHints {
        range: LspRange,
    },
    DocumentColors,
    DocumentSymbols,
    FoldingRanges,
    Formatting,
}

/// Typed local edit requests that return source edits against one immutable
/// buffer snapshot. The process adapter maps these operations to their own
/// LSP methods without folding them into [`LanguageServiceRequest`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LanguageServiceEditRequest {
    FormatSelection { range: LspRange },
    OrganizeImports,
}

impl LanguageServiceEditRequest {
    pub fn capability(&self) -> LanguageServiceEditCapability {
        match self {
            Self::FormatSelection { .. } => LanguageServiceEditCapability::FormatSelection,
            Self::OrganizeImports => LanguageServiceEditCapability::OrganizeImports,
        }
    }
}

impl LanguageServiceRequest {
    pub fn capability(&self) -> LanguageServiceCapability {
        match self {
            Self::Diagnostics => LanguageServiceCapability::Diagnostics,
            Self::Completion { .. } => LanguageServiceCapability::Completion,
            Self::SignatureHelp { .. } => LanguageServiceCapability::SignatureHelp,
            Self::Hover { .. } => LanguageServiceCapability::Hover,
            Self::Definition { .. } => LanguageServiceCapability::Definition,
            Self::Declaration { .. } => LanguageServiceCapability::Declaration,
            Self::Implementation { .. } => LanguageServiceCapability::Implementation,
            Self::TypeDefinition { .. } => LanguageServiceCapability::TypeDefinition,
            Self::References { .. } => LanguageServiceCapability::References,
            Self::PrepareCallHierarchy { .. }
            | Self::IncomingCalls { .. }
            | Self::OutgoingCalls { .. } => LanguageServiceCapability::CallHierarchy,
            Self::CodeLenses => LanguageServiceCapability::CodeLens,
            Self::ResolveCodeLens { .. } => LanguageServiceCapability::CodeLensResolve,
            Self::ExecuteCommand { .. } => LanguageServiceCapability::ExecuteCommand,
            Self::DocumentLinks => LanguageServiceCapability::DocumentLinks,
            Self::ResolveDocumentLink { .. } => LanguageServiceCapability::DocumentLinkResolve,
            Self::Rename { .. } => LanguageServiceCapability::Rename,
            Self::CodeActions { .. } => LanguageServiceCapability::CodeActions,
            Self::SemanticTokens => LanguageServiceCapability::SemanticTokens,
            Self::InlayHints { .. } => LanguageServiceCapability::InlayHints,
            Self::DocumentColors => LanguageServiceCapability::DocumentColors,
            Self::DocumentSymbols => LanguageServiceCapability::DocumentSymbols,
            Self::FoldingRanges => LanguageServiceCapability::FoldingRanges,
            Self::Formatting => LanguageServiceCapability::Formatting,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LanguageServiceResponse {
    Diagnostics(Vec<Diagnostic>),
    Completion(CompletionList),
    SignatureHelp(Option<SignatureHelp>),
    Hover(Option<HoverContent>),
    Definition(Vec<NavigationTarget>),
    TypeDefinition(Vec<NavigationTarget>),
    References(Vec<NavigationTarget>),
    CallHierarchyItems(Vec<CallHierarchyItem>),
    CallHierarchyCalls(Vec<CallHierarchyCall>),
    CodeLenses(Vec<CodeLens>),
    CodeLens(Option<CodeLens>),
    CommandResult(Option<Value>),
    DocumentLinks(Vec<DocumentLink>),
    DocumentLink(Option<DocumentLink>),
    Rename(Vec<Edit>),
    CodeActions(Vec<CodeAction>),
    SemanticTokens(Vec<SemanticToken>),
    InlayHints(Vec<InlayHint>),
    DocumentColors(Vec<DocumentColor>),
    DocumentSymbols(Vec<DocumentSymbol>),
    FoldingRanges(Vec<FoldingRange>),
    Formatting(Vec<Edit>),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LanguageServiceEditResponse {
    Formatting(Vec<Edit>),
    OrganizeImports(Vec<Edit>),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LanguageServiceError {
    UnsupportedLanguage(LanguageId),
    UnsupportedCapability(LanguageServiceCapability),
    UnsupportedEditCapability(LanguageServiceEditCapability),
    Cancelled,
    InvalidConfiguration(String),
    MalformedFrame(String),
    ProtocolEof,
    ProcessCrashed(Option<i32>),
    ProcessNotRunning,
    RequestTimeout,
    StaleDocument {
        expected: DocumentVersion,
        received: DocumentVersion,
    },
    ProviderFailed(String),
}

impl fmt::Display for LanguageServiceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedLanguage(language) => {
                write!(f, "unsupported language: {}", language.language().label())
            }
            Self::UnsupportedCapability(capability) => {
                write!(f, "unsupported language-service capability: {capability:?}")
            }
            Self::UnsupportedEditCapability(capability) => {
                write!(
                    f,
                    "unsupported language-service edit capability: {capability:?}"
                )
            }
            Self::Cancelled => f.write_str("language-service request cancelled"),
            Self::InvalidConfiguration(message)
            | Self::MalformedFrame(message)
            | Self::ProviderFailed(message) => f.write_str(message),
            Self::ProtocolEof => f.write_str("language-service protocol ended"),
            Self::ProcessCrashed(code) => write!(f, "language server crashed: exit code {code:?}"),
            Self::ProcessNotRunning => f.write_str("language server is not running"),
            Self::RequestTimeout => f.write_str("language-service request timed out"),
            Self::StaleDocument { expected, received } => {
                write!(
                    f,
                    "stale document version: expected {}, received {}",
                    expected.value(),
                    received.value()
                )
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LanguageServiceResult {
    pub server: LanguageServerId,
    pub version: DocumentVersion,
    pub generation: u64,
    pub response: Result<LanguageServiceResponse, LanguageServiceError>,
}

pub struct BackgroundRequest {
    pub language: LanguageId,
    pub snapshot: BufferSnapshot,
    pub version: DocumentVersion,
    pub generation: u64,
    pub request: LanguageServiceRequest,
    pub cancellation: CancellationToken,
}

pub trait LocalLanguageService: Send + Sync {
    fn id(&self) -> LanguageServerId;
    fn languages(&self) -> &[LanguageId];
    fn capabilities(&self) -> LanguageServiceCapabilities;
    fn request(
        &self,
        snapshot: &BufferSnapshot,
        version: DocumentVersion,
        request: LanguageServiceRequest,
        cancellation: &CancellationToken,
    ) -> Result<LanguageServiceResponse, LanguageServiceError>;

    fn edit_capabilities(&self) -> LanguageServiceEditCapabilities {
        LanguageServiceEditCapabilities::NONE
    }

    fn request_edit(
        &self,
        _snapshot: &BufferSnapshot,
        _version: DocumentVersion,
        request: LanguageServiceEditRequest,
        _cancellation: &CancellationToken,
    ) -> Result<LanguageServiceEditResponse, LanguageServiceError> {
        Err(LanguageServiceError::UnsupportedEditCapability(
            request.capability(),
        ))
    }
}

#[derive(Clone, Debug, Default)]
pub struct CancellationToken(Arc<AtomicBool>);

impl CancellationToken {
    pub fn cancel(&self) {
        self.0.store(true, Ordering::Release);
    }
    pub fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::Acquire)
    }
}

#[derive(Default)]
pub struct LanguageServiceRegistry {
    providers: Vec<Arc<dyn LocalLanguageService>>,
}

impl LanguageServiceRegistry {
    pub fn register(&mut self, provider: Arc<dyn LocalLanguageService>) {
        self.providers.push(provider);
    }

    pub fn provider_for(&self, language: LanguageId) -> Option<Arc<dyn LocalLanguageService>> {
        self.providers
            .iter()
            .find(|provider| provider.languages().contains(&language))
            .cloned()
    }

    pub fn capabilities(&self, language: LanguageId) -> Option<LanguageServiceCapabilities> {
        self.provider_for(language)
            .map(|provider| provider.capabilities())
    }

    pub fn edit_capabilities(
        &self,
        language: LanguageId,
    ) -> Option<LanguageServiceEditCapabilities> {
        self.provider_for(language)
            .map(|provider| provider.edit_capabilities())
    }

    pub fn request(
        &self,
        language: LanguageId,
        snapshot: &BufferSnapshot,
        version: DocumentVersion,
        request: LanguageServiceRequest,
        cancellation: &CancellationToken,
    ) -> Result<LanguageServiceResponse, LanguageServiceError> {
        let provider = self
            .provider_for(language)
            .ok_or(LanguageServiceError::UnsupportedLanguage(language))?;
        if !provider.capabilities().supports(request.capability()) {
            return Err(LanguageServiceError::UnsupportedCapability(
                request.capability(),
            ));
        }
        provider.request(snapshot, version, request, cancellation)
    }

    pub fn request_edit(
        &self,
        language: LanguageId,
        snapshot: &BufferSnapshot,
        version: DocumentVersion,
        request: LanguageServiceEditRequest,
        cancellation: &CancellationToken,
    ) -> Result<LanguageServiceEditResponse, LanguageServiceError> {
        let provider = self
            .provider_for(language)
            .ok_or(LanguageServiceError::UnsupportedLanguage(language))?;
        let capability = request.capability();
        if !provider.edit_capabilities().supports(capability) {
            return Err(LanguageServiceError::UnsupportedEditCapability(capability));
        }
        if cancellation.is_cancelled() {
            return Err(LanguageServiceError::Cancelled);
        }
        provider.request_edit(snapshot, version, request, cancellation)
    }

    pub fn request_in_background(
        &self,
        input: BackgroundRequest,
        callback: impl FnOnce(LanguageServiceResult) + Send + 'static,
    ) -> Option<JoinHandle<()>> {
        let provider = self.provider_for(input.language)?;
        let server = provider.id();
        Some(thread::spawn(move || {
            let response = if !provider.capabilities().supports(input.request.capability()) {
                Err(LanguageServiceError::UnsupportedCapability(
                    input.request.capability(),
                ))
            } else if input.cancellation.is_cancelled() {
                Err(LanguageServiceError::Cancelled)
            } else {
                provider.request(
                    &input.snapshot,
                    input.version,
                    input.request,
                    &input.cancellation,
                )
            };
            callback(LanguageServiceResult {
                server,
                version: input.version,
                generation: input.generation,
                response,
            });
        }))
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LanguageServiceSnapshot {
    pub version: DocumentVersion,
    pub generation: u64,
    pub diagnostics: Vec<Diagnostic>,
    pub semantic_tokens: Vec<SemanticToken>,
    pub inlay_hints: Vec<InlayHint>,
    pub document_colors: Vec<DocumentColor>,
    pub document_symbols: Option<Vec<DocumentSymbol>>,
    pub completion: Option<CompletionList>,
    pub signature_help: Option<SignatureHelp>,
    pub hover: Option<HoverContent>,
}

pub struct LanguageServiceState {
    version: DocumentVersion,
    generation: u64,
    diagnostics: Vec<Diagnostic>,
    semantic_tokens: Vec<SemanticToken>,
    inlay_hints: Vec<InlayHint>,
    document_colors: Vec<DocumentColor>,
    document_symbols: Option<Vec<DocumentSymbol>>,
    completion: Option<CompletionList>,
    signature_help: Option<SignatureHelp>,
    hover: Option<HoverContent>,
}

impl LanguageServiceState {
    pub fn new(version: DocumentVersion) -> Self {
        Self {
            version,
            generation: 0,
            diagnostics: Vec::new(),
            semantic_tokens: Vec::new(),
            inlay_hints: Vec::new(),
            document_colors: Vec::new(),
            document_symbols: None,
            completion: None,
            signature_help: None,
            hover: None,
        }
    }

    pub fn update_version(&mut self, version: DocumentVersion) {
        if version != self.version {
            self.version = version;
            self.generation = self.generation.saturating_add(1);
        }
    }

    /// Bind the result projection to the editor's current document
    /// generation. The editor allocates one generation per open/change, so
    /// every request for that immutable snapshot shares it.
    pub fn set_document_generation(&mut self, version: DocumentVersion, generation: u64) {
        self.version = version;
        self.generation = generation;
        self.diagnostics.clear();
        self.semantic_tokens.clear();
        self.inlay_hints.clear();
        self.document_colors.clear();
        self.document_symbols = None;
        self.completion = None;
        self.signature_help = None;
        self.hover = None;
    }

    pub fn begin_request(&mut self, version: DocumentVersion) -> (DocumentVersion, u64) {
        self.update_version(version);
        self.generation = self.generation.saturating_add(1);
        (self.version, self.generation)
    }

    pub fn apply(&mut self, result: LanguageServiceResult) -> bool {
        if result.version != self.version || result.generation != self.generation {
            return false;
        }
        let Ok(response) = result.response else {
            return false;
        };
        match response {
            LanguageServiceResponse::Diagnostics(value) => self.diagnostics = value,
            LanguageServiceResponse::SemanticTokens(value) => self.semantic_tokens = value,
            LanguageServiceResponse::InlayHints(value) => self.inlay_hints = value,
            LanguageServiceResponse::DocumentColors(value) => self.document_colors = value,
            LanguageServiceResponse::DocumentSymbols(value) => self.document_symbols = Some(value),
            LanguageServiceResponse::Completion(value) => self.completion = Some(value),
            LanguageServiceResponse::SignatureHelp(value) => self.signature_help = value,
            LanguageServiceResponse::Hover(value) => self.hover = value,
            _ => {}
        }
        true
    }

    pub fn snapshot(&self) -> LanguageServiceSnapshot {
        LanguageServiceSnapshot {
            version: self.version,
            generation: self.generation,
            diagnostics: self.diagnostics.clone(),
            semantic_tokens: self.semantic_tokens.clone(),
            inlay_hints: self.inlay_hints.clone(),
            document_colors: self.document_colors.clone(),
            document_symbols: self.document_symbols.clone(),
            completion: self.completion.clone(),
            signature_help: self.signature_help.clone(),
            hover: self.hover.clone(),
        }
    }

    /// Clear only the transient hover projection when the pointer moves. The
    /// document version and other language-service results remain intact.
    pub fn clear_hover(&mut self) {
        self.hover = None;
    }

    pub fn clear_signature_help(&mut self) {
        self.signature_help = None;
    }

    pub fn clear_inlay_hints(&mut self) {
        self.inlay_hints.clear();
    }

    pub fn clear_document_colors(&mut self) {
        self.document_colors.clear();
    }
}

/// A small, real local provider.  It is intentionally explicit about its
/// limits: it does not claim compiler/type-checker knowledge, but derives
/// useful editor behavior from the immutable syntax snapshot.
pub struct SyntaxLanguageService {
    id: LanguageServerId,
    language: LanguageId,
    languages: Vec<LanguageId>,
}

impl SyntaxLanguageService {
    pub fn for_language(language: Language) -> Self {
        Self {
            id: LanguageServerId::new(format!("syntax-{}", language.label().to_ascii_lowercase())),
            language: LanguageId::new(language),
            languages: vec![LanguageId::new(language)],
        }
    }

    fn word_at(snapshot: &BufferSnapshot, position: LspPosition) -> Option<(String, TextRange)> {
        let pos = position.to_position(snapshot);
        let line = snapshot.line(pos.line);
        let chars: Vec<char> = line.chars().collect();
        if chars.is_empty() {
            return None;
        }
        let at = pos.column.min(chars.len());
        let mut start = at;
        if start == chars.len() && start > 0 {
            start -= 1;
        }
        if start < chars.len() && !is_word(chars[start]) && start > 0 && is_word(chars[start - 1]) {
            start -= 1;
        }
        if start >= chars.len() || !is_word(chars[start]) {
            return None;
        }
        while start > 0 && is_word(chars[start - 1]) {
            start -= 1;
        }
        let mut end = start;
        while end < chars.len() && is_word(chars[end]) {
            end += 1;
        }
        let range = TextRange::new(
            snapshot.position_to_byte(Position::new(pos.line, start)),
            snapshot.position_to_byte(Position::new(pos.line, end)),
        );
        Some((chars[start..end].iter().collect(), range))
    }

    fn completion_prefix(snapshot: &BufferSnapshot, position: LspPosition) -> String {
        let pos = position.to_position(snapshot);
        let line = snapshot.line(pos.line);
        let chars: Vec<char> = line.chars().take(pos.column).collect();
        chars
            .iter()
            .rev()
            .copied()
            .take_while(|ch| is_word(*ch))
            .collect::<String>()
            .chars()
            .rev()
            .collect()
    }

    fn diagnostics(
        &self,
        snapshot: &BufferSnapshot,
        cancellation: &CancellationToken,
    ) -> Result<Vec<Diagnostic>, LanguageServiceError> {
        let text = snapshot.text();
        let mut stack: Vec<(char, usize)> = Vec::new();
        let mut diagnostics = Vec::new();
        let mut byte = 0;
        let mut in_string = false;
        let mut escaped = false;
        for ch in text.chars() {
            if cancellation.is_cancelled() {
                return Err(LanguageServiceError::Cancelled);
            }
            let len = ch.len_utf8();
            if ch == '"' && !escaped {
                in_string = !in_string;
            }
            if !in_string {
                if matches!(ch, '(' | '[' | '{') {
                    stack.push((ch, byte));
                }
                if matches!(ch, ')' | ']' | '}') {
                    let expected = match ch {
                        ')' => '(',
                        ']' => '[',
                        '}' => '{',
                        _ => unreachable!(),
                    };
                    if stack.last().is_some_and(|(open, _)| *open == expected) {
                        stack.pop();
                    } else {
                        let range = TextRange::new(byte, byte + len);
                        diagnostics.push(self.diagnostic(
                            snapshot,
                            range,
                            DiagnosticSeverity::Error,
                            format!("Unexpected '{ch}'"),
                            "unmatched-delimiter",
                        ));
                    }
                }
            }
            escaped = ch == '\\' && !escaped;
            if ch != '\\' {
                escaped = false;
            }
            byte += len;
        }
        for (open, at) in stack {
            diagnostics.push(self.diagnostic(
                snapshot,
                TextRange::new(at, (at + 1).min(snapshot.byte_len())),
                DiagnosticSeverity::Error,
                format!("Unclosed '{open}'"),
                "unclosed-delimiter",
            ));
        }
        if in_string && self.language.language() == Language::Json {
            let at = snapshot.byte_len().saturating_sub(1);
            diagnostics.push(self.diagnostic(
                snapshot,
                TextRange::new(at, snapshot.byte_len()),
                DiagnosticSeverity::Error,
                "Unclosed string".to_string(),
                "unclosed-string",
            ));
        }
        Ok(diagnostics)
    }

    fn diagnostic(
        &self,
        snapshot: &BufferSnapshot,
        range: TextRange,
        severity: DiagnosticSeverity,
        message: String,
        code: &str,
    ) -> Diagnostic {
        Diagnostic {
            range: LspRange::from_text_range(snapshot, range),
            severity,
            message,
            source: self.id(),
            code: Some(code.to_string()),
        }
    }

    fn identifiers(&self, snapshot: &BufferSnapshot) -> Vec<(String, TextRange)> {
        let text = snapshot.text();
        let bytes = text.as_bytes();
        let mut result = Vec::new();
        let mut start = None;
        for (index, ch) in text.char_indices() {
            if is_word(ch) {
                if start.is_none() {
                    start = Some(index);
                }
            } else if let Some(begin) = start.take() {
                result.push((text[begin..index].to_string(), TextRange::new(begin, index)));
            }
        }
        if let Some(begin) = start {
            result.push((
                text[begin..bytes.len()].to_string(),
                TextRange::new(begin, bytes.len()),
            ));
        }
        result
    }

    fn keywords(&self) -> &'static [&'static str] {
        match self.language.language() {
            Language::Rust => &[
                "fn", "let", "mut", "struct", "enum", "impl", "trait", "pub", "use", "match",
                "async", "await",
            ],
            Language::Python => &[
                "def", "class", "import", "from", "return", "if", "elif", "else", "for", "while",
                "async", "await",
            ],
            Language::Json => &["true", "false", "null"],
            _ => &[],
        }
    }

    fn completion(
        &self,
        snapshot: &BufferSnapshot,
        position: LspPosition,
        cancellation: &CancellationToken,
    ) -> Result<CompletionList, LanguageServiceError> {
        let prefix = Self::completion_prefix(snapshot, position).to_ascii_lowercase();
        let mut labels: BTreeMap<String, CompletionItem> = BTreeMap::new();
        for keyword in self.keywords() {
            if keyword.starts_with(&prefix) {
                labels.insert(
                    (*keyword).to_string(),
                    CompletionItem {
                        label: (*keyword).to_string(),
                        kind: CompletionKind::Keyword,
                        text_format: CompletionTextFormat::PlainText,
                        detail: Some("syntax keyword".to_string()),
                        insert_text: (*keyword).to_string(),
                        text_edit_range: None,
                        filter_text: None,
                        sort_text: Some(format!("0-{keyword}")),
                        documentation: None,
                    },
                );
            }
        }
        for (name, _) in self.identifiers(snapshot) {
            if cancellation.is_cancelled() {
                return Err(LanguageServiceError::Cancelled);
            }
            if name.len() >= prefix.len() && name.to_ascii_lowercase().starts_with(&prefix) {
                let sort_text = format!("1-{}", labels.len());
                labels
                    .entry(name.clone())
                    .or_insert_with(|| CompletionItem {
                        label: name.clone(),
                        kind: CompletionKind::Text,
                        text_format: CompletionTextFormat::PlainText,
                        detail: Some("document symbol".to_string()),
                        insert_text: name,
                        text_edit_range: None,
                        filter_text: None,
                        sort_text: Some(sort_text),
                        documentation: None,
                    });
            }
        }
        let mut items: Vec<_> = labels.into_values().collect();
        items.sort_by(|a, b| {
            a.sort_text
                .as_deref()
                .unwrap_or(&a.label)
                .cmp(b.sort_text.as_deref().unwrap_or(&b.label))
                .then_with(|| a.label.cmp(&b.label))
        });
        Ok(CompletionList {
            items,
            is_incomplete: false,
        })
    }

    fn semantic_tokens(
        &self,
        snapshot: &BufferSnapshot,
        cancellation: &CancellationToken,
    ) -> Result<Vec<SemanticToken>, LanguageServiceError> {
        let mut highlighter = SyntaxHighlighter::new(self.language.language());
        highlighter.update(snapshot, 0..snapshot.byte_len());
        let mut tokens = Vec::new();
        for span in highlighter.spans() {
            if cancellation.is_cancelled() {
                return Err(LanguageServiceError::Cancelled);
            }
            let token_type = match span.kind {
                HighlightKind::Function => "function",
                HighlightKind::Type | HighlightKind::Constructor => "type",
                HighlightKind::Keyword => "keyword",
                HighlightKind::String => "string",
                HighlightKind::Number => "number",
                HighlightKind::Comment => "comment",
                HighlightKind::Constant | HighlightKind::Boolean => "constant",
                HighlightKind::Property => "property",
                _ => continue,
            };
            tokens.push(SemanticToken {
                range: LspRange::from_text_range(snapshot, TextRange::new(span.start, span.end)),
                token_type: token_type.to_string(),
                modifiers: Vec::new(),
            });
        }
        Ok(tokens)
    }

    fn navigation(
        &self,
        snapshot: &BufferSnapshot,
        position: LspPosition,
        references: bool,
    ) -> Vec<NavigationTarget> {
        let Some((word, word_range)) = Self::word_at(snapshot, position) else {
            return Vec::new();
        };
        if references {
            return self
                .identifiers(snapshot)
                .into_iter()
                .filter(|(name, _)| *name == word)
                .map(|(name, range)| NavigationTarget {
                    name,
                    uri: None,
                    range: LspRange::from_text_range(snapshot, range),
                    selection_range: LspRange::from_text_range(snapshot, range),
                })
                .collect();
        }
        document_symbols(self.language.language(), snapshot)
            .into_iter()
            .filter(|symbol| symbol.name == word)
            .map(|symbol| NavigationTarget {
                name: symbol.name,
                uri: None,
                range: LspRange::from_text_range(snapshot, symbol.range),
                selection_range: LspRange::from_text_range(snapshot, symbol.name_range),
            })
            .collect::<Vec<_>>()
            .into_iter()
            .chain(
                (document_symbols(self.language.language(), snapshot).is_empty()).then(|| {
                    NavigationTarget {
                        name: word,
                        uri: None,
                        range: LspRange::from_text_range(snapshot, word_range),
                        selection_range: LspRange::from_text_range(snapshot, word_range),
                    }
                }),
            )
            .collect()
    }

    fn rename(
        &self,
        snapshot: &BufferSnapshot,
        position: LspPosition,
        new_name: String,
    ) -> Vec<Edit> {
        let Some((word, _)) = Self::word_at(snapshot, position) else {
            return Vec::new();
        };
        self.identifiers(snapshot)
            .into_iter()
            .filter(|(name, _)| *name == word)
            .map(|(_, range)| Edit::new(range, &new_name))
            .collect()
    }

    fn code_actions(&self, snapshot: &BufferSnapshot, range: LspRange) -> Vec<CodeAction> {
        let requested = range.to_text_range(snapshot);
        let start_line = snapshot.byte_to_position(requested.start).line;
        let end_line = snapshot.byte_to_position(requested.end).line;
        let mut edits = Vec::new();
        for line in start_line..=end_line.min(snapshot.line_count().saturating_sub(1)) {
            let text = snapshot.line(line);
            let trimmed = text.trim_end_matches([' ', '\t']);
            if trimmed.len() < text.len() {
                edits.push(Edit::new(
                    TextRange::new(
                        snapshot.position_to_byte(Position::new(line, trimmed.chars().count())),
                        snapshot.position_to_byte(Position::new(line, text.chars().count())),
                    ),
                    "",
                ));
            }
        }
        if edits.is_empty() {
            Vec::new()
        } else {
            vec![CodeAction {
                title: "Trim trailing whitespace".to_string(),
                kind: "source.fixAll".to_string(),
                edits,
            }]
        }
    }

    fn folding_ranges(
        &self,
        snapshot: &BufferSnapshot,
        cancellation: &CancellationToken,
    ) -> Vec<FoldingRange> {
        let mut stack: Vec<(char, usize)> = Vec::new();
        let mut result = Vec::new();
        for (line, text) in (0..snapshot.line_count()).map(|line| (line, snapshot.line(line))) {
            if cancellation.is_cancelled() {
                return result;
            }
            for ch in text.chars() {
                if matches!(ch, '{' | '[' | '(') {
                    stack.push((ch, line));
                }
                if matches!(ch, '}' | ']' | ')') {
                    if let Some((_, start)) = stack.pop() {
                        if line > start {
                            result.push(FoldingRange {
                                start_line: start,
                                end_line: line,
                                kind: Some("region".to_string()),
                            });
                        }
                    }
                }
            }
        }
        result.sort_by_key(|range| (range.start_line, range.end_line));
        result
    }

    fn formatting(&self, snapshot: &BufferSnapshot, cancellation: &CancellationToken) -> Vec<Edit> {
        self.formatting_in_range(snapshot, None, cancellation)
    }

    fn formatting_in_range(
        &self,
        snapshot: &BufferSnapshot,
        requested: Option<TextRange>,
        cancellation: &CancellationToken,
    ) -> Vec<Edit> {
        let mut edits = Vec::new();
        for line in 0..snapshot.line_count() {
            if cancellation.is_cancelled() {
                return Vec::new();
            }
            let text = snapshot.line(line);
            let trimmed = text.trim_end_matches([' ', '\t']);
            if trimmed.len() < text.len() {
                let edit = Edit::new(
                    TextRange::new(
                        snapshot.position_to_byte(Position::new(line, trimmed.chars().count())),
                        snapshot.position_to_byte(Position::new(line, text.chars().count())),
                    ),
                    "",
                );
                if requested.is_none_or(|range| {
                    !range.is_empty()
                        && edit.range.start >= range.start
                        && edit.range.end <= range.end
                }) {
                    edits.push(edit);
                }
            }
        }
        if requested.is_none() && snapshot.byte_len() > 0 && !snapshot.text().ends_with('\n') {
            edits.push(Edit::new(
                TextRange::new(snapshot.byte_len(), snapshot.byte_len()),
                "\n",
            ));
        }
        edits
    }

    fn organize_imports(
        &self,
        snapshot: &BufferSnapshot,
        cancellation: &CancellationToken,
    ) -> Result<Vec<Edit>, LanguageServiceError> {
        if self.language.language() != Language::Rust {
            return Err(LanguageServiceError::UnsupportedEditCapability(
                LanguageServiceEditCapability::OrganizeImports,
            ));
        }

        let mut edits = Vec::new();
        let mut line = 0;
        while line < snapshot.line_count() {
            if cancellation.is_cancelled() {
                return Err(LanguageServiceError::Cancelled);
            }
            if !is_simple_rust_import(&snapshot.line(line)) {
                line += 1;
                continue;
            }

            let start_line = line;
            let mut imports = Vec::new();
            while line < snapshot.line_count() && is_simple_rust_import(&snapshot.line(line)) {
                if cancellation.is_cancelled() {
                    return Err(LanguageServiceError::Cancelled);
                }
                imports.push(snapshot.line(line));
                line += 1;
            }

            let mut sorted = imports.clone();
            sorted.sort();
            for (offset, (original, replacement)) in imports.iter().zip(sorted).enumerate() {
                if original == &replacement {
                    continue;
                }
                let import_line = start_line + offset;
                edits.push(Edit::new(
                    TextRange::new(
                        snapshot.position_to_byte(Position::new(import_line, 0)),
                        snapshot
                            .position_to_byte(Position::new(import_line, original.chars().count())),
                    ),
                    replacement,
                ));
            }
        }
        Ok(edits)
    }
}

fn is_simple_rust_import(line: &str) -> bool {
    let is_top_level = line.trim_start() == line;
    let is_import = line.starts_with("use ") || line.starts_with("pub use ");
    is_top_level && is_import && line.trim_end().ends_with(';')
}

impl LocalLanguageService for SyntaxLanguageService {
    fn id(&self) -> LanguageServerId {
        self.id.clone()
    }
    fn languages(&self) -> &[LanguageId] {
        &self.languages
    }
    fn capabilities(&self) -> LanguageServiceCapabilities {
        LanguageServiceCapabilities::SYNTAX_DERIVED
    }

    fn edit_capabilities(&self) -> LanguageServiceEditCapabilities {
        LanguageServiceEditCapabilities {
            organize_imports: self.language.language() == Language::Rust,
            ..LanguageServiceEditCapabilities::SYNTAX_DERIVED
        }
    }

    fn request(
        &self,
        snapshot: &BufferSnapshot,
        _version: DocumentVersion,
        request: LanguageServiceRequest,
        cancellation: &CancellationToken,
    ) -> Result<LanguageServiceResponse, LanguageServiceError> {
        if cancellation.is_cancelled() {
            return Err(LanguageServiceError::Cancelled);
        }
        Ok(match request {
            LanguageServiceRequest::Diagnostics => {
                LanguageServiceResponse::Diagnostics(self.diagnostics(snapshot, cancellation)?)
            }
            LanguageServiceRequest::Completion { position, .. } => {
                LanguageServiceResponse::Completion(self.completion(
                    snapshot,
                    position,
                    cancellation,
                )?)
            }
            LanguageServiceRequest::SignatureHelp { .. } => {
                return Err(LanguageServiceError::UnsupportedCapability(
                    LanguageServiceCapability::SignatureHelp,
                ));
            }
            LanguageServiceRequest::Hover { position } => {
                let hover = Self::word_at(snapshot, position).map(|(word, range)| HoverContent {
                    contents: format!("`{word}` — syntax-derived symbol information"),
                    range: Some(LspRange::from_text_range(snapshot, range)),
                });
                LanguageServiceResponse::Hover(hover)
            }
            LanguageServiceRequest::Definition { position } => {
                LanguageServiceResponse::Definition(self.navigation(snapshot, position, false))
            }
            LanguageServiceRequest::Declaration { position } => {
                LanguageServiceResponse::Definition(self.navigation(snapshot, position, false))
            }
            LanguageServiceRequest::Implementation { position } => {
                LanguageServiceResponse::Definition(self.navigation(snapshot, position, false))
            }
            LanguageServiceRequest::TypeDefinition { position } => {
                LanguageServiceResponse::TypeDefinition(self.navigation(snapshot, position, false))
            }
            LanguageServiceRequest::References { position } => {
                LanguageServiceResponse::References(self.navigation(snapshot, position, true))
            }
            LanguageServiceRequest::PrepareCallHierarchy { .. }
            | LanguageServiceRequest::IncomingCalls { .. }
            | LanguageServiceRequest::OutgoingCalls { .. } => {
                return Err(LanguageServiceError::UnsupportedCapability(
                    LanguageServiceCapability::CallHierarchy,
                ));
            }
            LanguageServiceRequest::CodeLenses => {
                return Err(LanguageServiceError::UnsupportedCapability(
                    LanguageServiceCapability::CodeLens,
                ));
            }
            LanguageServiceRequest::ResolveCodeLens { .. } => {
                return Err(LanguageServiceError::UnsupportedCapability(
                    LanguageServiceCapability::CodeLensResolve,
                ));
            }
            LanguageServiceRequest::ExecuteCommand { .. } => {
                return Err(LanguageServiceError::UnsupportedCapability(
                    LanguageServiceCapability::ExecuteCommand,
                ));
            }
            LanguageServiceRequest::DocumentLinks => {
                return Err(LanguageServiceError::UnsupportedCapability(
                    LanguageServiceCapability::DocumentLinks,
                ));
            }
            LanguageServiceRequest::ResolveDocumentLink { .. } => {
                return Err(LanguageServiceError::UnsupportedCapability(
                    LanguageServiceCapability::DocumentLinkResolve,
                ));
            }
            LanguageServiceRequest::Rename { position, new_name } => {
                LanguageServiceResponse::Rename(self.rename(snapshot, position, new_name))
            }
            LanguageServiceRequest::CodeActions { range } => {
                LanguageServiceResponse::CodeActions(self.code_actions(snapshot, range))
            }
            LanguageServiceRequest::SemanticTokens => LanguageServiceResponse::SemanticTokens(
                self.semantic_tokens(snapshot, cancellation)?,
            ),
            LanguageServiceRequest::InlayHints { .. } => {
                LanguageServiceResponse::InlayHints(Vec::new())
            }
            LanguageServiceRequest::DocumentColors => {
                LanguageServiceResponse::DocumentColors(Vec::new())
            }
            LanguageServiceRequest::DocumentSymbols => LanguageServiceResponse::DocumentSymbols(
                document_symbols(self.language.language(), snapshot),
            ),
            LanguageServiceRequest::FoldingRanges => {
                LanguageServiceResponse::FoldingRanges(self.folding_ranges(snapshot, cancellation))
            }
            LanguageServiceRequest::Formatting => {
                LanguageServiceResponse::Formatting(self.formatting(snapshot, cancellation))
            }
        })
    }

    fn request_edit(
        &self,
        snapshot: &BufferSnapshot,
        _version: DocumentVersion,
        request: LanguageServiceEditRequest,
        cancellation: &CancellationToken,
    ) -> Result<LanguageServiceEditResponse, LanguageServiceError> {
        if cancellation.is_cancelled() {
            return Err(LanguageServiceError::Cancelled);
        }
        match request {
            LanguageServiceEditRequest::FormatSelection { range } => Ok(
                LanguageServiceEditResponse::Formatting(self.formatting_in_range(
                    snapshot,
                    Some(range.to_text_range(snapshot)),
                    cancellation,
                )),
            ),
            LanguageServiceEditRequest::OrganizeImports => {
                Ok(LanguageServiceEditResponse::OrganizeImports(
                    self.organize_imports(snapshot, cancellation)?,
                ))
            }
        }
    }
}

fn is_word(ch: char) -> bool {
    ch == '_' || ch.is_alphanumeric()
}

/// Build the local syntax providers shipped by the editor.  Unsupported
/// languages intentionally have no provider and return a typed empty/error
/// state through the registry.
pub fn default_registry() -> LanguageServiceRegistry {
    let mut registry = LanguageServiceRegistry::default();
    for language in [Language::Rust, Language::Python, Language::Json] {
        registry.register(Arc::new(SyntaxLanguageService::for_language(language)));
    }
    registry
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::EditorBuffer;

    fn service(language: Language) -> SyntaxLanguageService {
        SyntaxLanguageService::for_language(language)
    }
    fn snapshot(text: &str) -> BufferSnapshot {
        EditorBuffer::from_text(text).snapshot()
    }

    #[test]
    fn utf16_positions_round_trip_unicode() {
        let snap = snapshot("😀 café\nnext");
        let p = LspPosition::from_position(&snap, Position::new(0, 3));
        assert_eq!(p.character, 4);
        assert_eq!(p.to_position(&snap), Position::new(0, 3));
    }

    #[test]
    fn diagnostics_have_typed_ranges() {
        let snap = snapshot("fn main( {\n");
        let result = service(Language::Rust)
            .request(
                &snap,
                DocumentVersion::new(1),
                LanguageServiceRequest::Diagnostics,
                &CancellationToken::default(),
            )
            .unwrap();
        let LanguageServiceResponse::Diagnostics(items) = result else {
            panic!("wrong response")
        };
        assert!(!items.is_empty());
        assert!(items
            .iter()
            .all(|item| item.range.to_text_range(&snap).start <= snap.byte_len()));
    }

    #[test]
    fn semantic_tokens_map_to_source_ranges() {
        let snap = snapshot("fn main() { let answer = 1; }");
        let result = service(Language::Rust)
            .request(
                &snap,
                DocumentVersion::new(1),
                LanguageServiceRequest::SemanticTokens,
                &CancellationToken::default(),
            )
            .unwrap();
        let LanguageServiceResponse::SemanticTokens(items) = result else {
            panic!("wrong response")
        };
        assert!(items.iter().any(|item| item.token_type == "keyword"));
        assert!(items
            .iter()
            .all(|item| item.range.to_text_range(&snap).start < snap.byte_len()));
    }

    #[test]
    fn inlay_hints_are_capability_gated_and_revision_bound() {
        assert!(!service(Language::Rust)
            .capabilities()
            .supports(LanguageServiceCapability::InlayHints));

        let mut state = LanguageServiceState::new(DocumentVersion::new(1));
        let hint = InlayHint {
            position: LspPosition {
                line: 0,
                character: 2,
            },
            label: ": i32".into(),
            kind: InlayHintKind::Type,
            tooltip: None,
            padding_left: true,
            padding_right: false,
        };
        assert!(state.apply(LanguageServiceResult {
            server: LanguageServerId::new("test"),
            version: DocumentVersion::new(1),
            generation: 0,
            response: Ok(LanguageServiceResponse::InlayHints(vec![hint.clone()])),
        }));
        assert_eq!(state.snapshot().inlay_hints, vec![hint]);

        state.set_document_generation(DocumentVersion::new(2), 1);
        assert!(state.snapshot().inlay_hints.is_empty());
    }

    #[test]
    fn document_colors_are_revision_bound_and_cleared_on_document_sync() {
        let color = DocumentColor {
            range: LspRange {
                start: LspPosition {
                    line: 0,
                    character: 0,
                },
                end: LspPosition {
                    line: 0,
                    character: 4,
                },
            },
            color: DocumentColorValue {
                red: 255,
                green: 0,
                blue: 128,
                alpha: 255,
            },
        };
        let mut state = LanguageServiceState::new(DocumentVersion::new(1));
        assert!(state.apply(LanguageServiceResult {
            server: LanguageServerId::new("test"),
            version: DocumentVersion::new(1),
            generation: 0,
            response: Ok(LanguageServiceResponse::DocumentColors(vec![color])),
        }));
        assert_eq!(state.snapshot().document_colors, vec![color]);

        state.set_document_generation(DocumentVersion::new(2), 1);
        assert!(state.snapshot().document_colors.is_empty());
        assert!(!state.apply(LanguageServiceResult {
            server: LanguageServerId::new("test"),
            version: DocumentVersion::new(1),
            generation: 0,
            response: Ok(LanguageServiceResponse::DocumentColors(vec![color])),
        }));
    }

    #[test]
    fn document_symbols_are_revision_bound_and_distinguish_empty_from_unrequested() {
        let symbol = DocumentSymbol {
            name: "main".into(),
            kind: crate::SymbolKind::Function,
            line: 2,
            range: TextRange::new(12, 42),
            name_range: TextRange::new(15, 19),
        };
        let mut state = LanguageServiceState::new(DocumentVersion::new(1));
        assert_eq!(state.snapshot().document_symbols, None);
        assert!(state.apply(LanguageServiceResult {
            server: LanguageServerId::new("test"),
            version: DocumentVersion::new(1),
            generation: 0,
            response: Ok(LanguageServiceResponse::DocumentSymbols(vec![
                symbol.clone()
            ])),
        }));
        assert_eq!(state.snapshot().document_symbols, Some(vec![symbol]));

        state.set_document_generation(DocumentVersion::new(2), 1);
        assert_eq!(state.snapshot().document_symbols, None);
        assert!(!state.apply(LanguageServiceResult {
            server: LanguageServerId::new("test"),
            version: DocumentVersion::new(1),
            generation: 0,
            response: Ok(LanguageServiceResponse::DocumentSymbols(Vec::new())),
        }));
    }

    #[test]
    fn completion_is_filtered_and_sorted() {
        let snap = snapshot("fn main() { ma }");
        let result = service(Language::Rust)
            .request(
                &snap,
                DocumentVersion::new(1),
                LanguageServiceRequest::Completion {
                    position: LspPosition {
                        line: 0,
                        character: 16,
                    },
                    trigger: None,
                },
                &CancellationToken::default(),
            )
            .unwrap();
        let LanguageServiceResponse::Completion(items) = result else {
            panic!("wrong response")
        };
        assert!(items.items.iter().any(|item| item.label == "main"));
        assert!(items
            .items
            .windows(2)
            .all(|pair| pair[0].label <= pair[1].label || pair[0].sort_text <= pair[1].sort_text));
    }

    #[test]
    fn navigation_hover_and_rename_are_real_contracts() {
        let snap = snapshot("fn greet() {}\nfn main() { greet(); }");
        let pos = LspPosition {
            line: 1,
            character: 16,
        };
        let hover = service(Language::Rust)
            .request(
                &snap,
                DocumentVersion::new(1),
                LanguageServiceRequest::Hover { position: pos },
                &CancellationToken::default(),
            )
            .unwrap();
        assert!(matches!(hover, LanguageServiceResponse::Hover(Some(_))));
        assert_eq!(
            LanguageServiceRequest::TypeDefinition { position: pos }.capability(),
            LanguageServiceCapability::TypeDefinition
        );
        assert_eq!(
            LanguageServiceRequest::Declaration { position: pos }.capability(),
            LanguageServiceCapability::Declaration
        );
        assert_eq!(
            LanguageServiceRequest::Implementation { position: pos }.capability(),
            LanguageServiceCapability::Implementation
        );
        assert!(!LanguageServiceCapabilities::SYNTAX_DERIVED
            .supports(LanguageServiceCapability::Declaration));
        assert!(!LanguageServiceCapabilities::SYNTAX_DERIVED
            .supports(LanguageServiceCapability::Implementation));
        assert!(!LanguageServiceCapabilities::SYNTAX_DERIVED
            .supports(LanguageServiceCapability::TypeDefinition));
        let definition = service(Language::Rust)
            .request(
                &snap,
                DocumentVersion::new(1),
                LanguageServiceRequest::Definition { position: pos },
                &CancellationToken::default(),
            )
            .unwrap();
        assert!(
            matches!(definition, LanguageServiceResponse::Definition(ref items) if !items.is_empty())
        );
        let rename = service(Language::Rust)
            .request(
                &snap,
                DocumentVersion::new(1),
                LanguageServiceRequest::Rename {
                    position: pos,
                    new_name: "welcome".into(),
                },
                &CancellationToken::default(),
            )
            .unwrap();
        assert!(matches!(rename, LanguageServiceResponse::Rename(ref edits) if edits.len() == 2));
    }

    #[test]
    fn call_hierarchy_is_a_separate_lsp_capability() {
        let snapshot = snapshot("fn caller() { callee(); }\nfn callee() {}\n");
        let position = LspPosition {
            line: 0,
            character: 5,
        };
        let request = LanguageServiceRequest::PrepareCallHierarchy { position };
        assert_eq!(
            request.capability(),
            LanguageServiceCapability::CallHierarchy
        );
        assert!(!service(Language::Rust)
            .capabilities()
            .supports(LanguageServiceCapability::CallHierarchy));
        assert!(matches!(
            service(Language::Rust).request(
                &snapshot,
                DocumentVersion::new(1),
                request,
                &CancellationToken::default(),
            ),
            Err(LanguageServiceError::UnsupportedCapability(
                LanguageServiceCapability::CallHierarchy
            ))
        ));
    }

    #[test]
    fn code_lens_and_execute_command_are_explicit_lsp_capabilities() {
        let snapshot = snapshot("fn main() {}\n");
        for (request, capability) in [
            (
                LanguageServiceRequest::CodeLenses,
                LanguageServiceCapability::CodeLens,
            ),
            (
                LanguageServiceRequest::ExecuteCommand {
                    command: "fixture.run".to_string(),
                    arguments: Vec::new(),
                },
                LanguageServiceCapability::ExecuteCommand,
            ),
        ] {
            assert_eq!(request.capability(), capability);
            assert!(!service(Language::Rust).capabilities().supports(capability));
            assert!(matches!(
                service(Language::Rust).request(
                    &snapshot,
                    DocumentVersion::new(1),
                    request,
                    &CancellationToken::default(),
                ),
                Err(LanguageServiceError::UnsupportedCapability(actual)) if actual == capability
            ));
        }
    }

    #[test]
    fn document_links_and_resolution_are_explicit_lsp_capabilities() {
        let snapshot = snapshot("https://example.com\n");
        let range = LspRange {
            start: LspPosition {
                line: 0,
                character: 0,
            },
            end: LspPosition {
                line: 0,
                character: 19,
            },
        };
        for (request, capability) in [
            (
                LanguageServiceRequest::DocumentLinks,
                LanguageServiceCapability::DocumentLinks,
            ),
            (
                LanguageServiceRequest::ResolveDocumentLink {
                    link: DocumentLink {
                        range,
                        target: None,
                        tooltip: None,
                        data: Some(serde_json::json!({ "resolve": true })),
                    },
                },
                LanguageServiceCapability::DocumentLinkResolve,
            ),
        ] {
            assert_eq!(request.capability(), capability);
            assert!(!service(Language::Rust).capabilities().supports(capability));
            assert!(matches!(
                service(Language::Rust).request(
                    &snapshot,
                    DocumentVersion::new(1),
                    request,
                    &CancellationToken::default(),
                ),
                Err(LanguageServiceError::UnsupportedCapability(actual)) if actual == capability
            ));
        }
    }

    #[test]
    fn unsupported_languages_are_explicit() {
        let registry = default_registry();
        let snap = snapshot("[x]");
        let result = registry.request(
            LanguageId::new(Language::Toml),
            &snap,
            DocumentVersion::new(1),
            LanguageServiceRequest::Diagnostics,
            &CancellationToken::default(),
        );
        assert!(matches!(
            result,
            Err(LanguageServiceError::UnsupportedLanguage(_))
        ));
    }

    #[test]
    fn stale_results_are_rejected_by_version_and_generation() {
        let mut state = LanguageServiceState::new(DocumentVersion::new(1));
        let (_, generation) = state.begin_request(DocumentVersion::new(1));
        state.update_version(DocumentVersion::new(2));
        let result = LanguageServiceResult {
            server: LanguageServerId::new("test"),
            version: DocumentVersion::new(1),
            generation,
            response: Ok(LanguageServiceResponse::Diagnostics(Vec::new())),
        };
        assert!(!state.apply(result));
    }

    #[test]
    fn signature_help_state_is_versioned_and_cleared_for_empty_results() {
        let mut state = LanguageServiceState::new(DocumentVersion::new(1));
        let signature = SignatureHelp {
            label: "call(value: i32)".into(),
            documentation: Some("Calls the function.".into()),
        };
        assert!(state.apply(LanguageServiceResult {
            server: LanguageServerId::new("test"),
            version: DocumentVersion::new(1),
            generation: 0,
            response: Ok(LanguageServiceResponse::SignatureHelp(Some(
                signature.clone(),
            ))),
        }));
        assert_eq!(state.snapshot().signature_help, Some(signature));

        assert!(state.apply(LanguageServiceResult {
            server: LanguageServerId::new("test"),
            version: DocumentVersion::new(1),
            generation: 0,
            response: Ok(LanguageServiceResponse::SignatureHelp(None)),
        }));
        assert!(state.snapshot().signature_help.is_none());

        assert!(!state.apply(LanguageServiceResult {
            server: LanguageServerId::new("test"),
            version: DocumentVersion::new(0),
            generation: 0,
            response: Ok(LanguageServiceResponse::SignatureHelp(Some(
                SignatureHelp {
                    label: "stale()".into(),
                    documentation: None,
                },
            ))),
        }));
        assert!(state.snapshot().signature_help.is_none());
    }

    #[test]
    fn cancellation_is_mapped_to_typed_error() {
        let token = CancellationToken::default();
        token.cancel();
        let result = service(Language::Python).request(
            &snapshot("def x():"),
            DocumentVersion::new(1),
            LanguageServiceRequest::Diagnostics,
            &token,
        );
        assert_eq!(result, Err(LanguageServiceError::Cancelled));
    }

    #[test]
    fn local_provider_has_folding_and_formatting() {
        let snap = snapshot("fn main() {  \n}\n");
        let provider = service(Language::Rust);
        let folds = provider
            .request(
                &snap,
                DocumentVersion::new(1),
                LanguageServiceRequest::FoldingRanges,
                &CancellationToken::default(),
            )
            .unwrap();
        assert!(
            matches!(folds, LanguageServiceResponse::FoldingRanges(ref items) if !items.is_empty())
        );
        let formatting = provider
            .request(
                &snap,
                DocumentVersion::new(1),
                LanguageServiceRequest::Formatting,
                &CancellationToken::default(),
            )
            .unwrap();
        assert!(
            matches!(formatting, LanguageServiceResponse::Formatting(ref edits) if !edits.is_empty())
        );
    }

    #[test]
    fn local_edit_capabilities_are_reported_and_routed() {
        let registry = default_registry();
        let rust = LanguageId::new(Language::Rust);
        let python = LanguageId::new(Language::Python);
        assert_eq!(
            registry.edit_capabilities(rust),
            Some(LanguageServiceEditCapabilities {
                format_selection: true,
                organize_imports: true,
            })
        );
        assert_eq!(
            registry.edit_capabilities(python),
            Some(LanguageServiceEditCapabilities {
                format_selection: true,
                organize_imports: false,
            })
        );

        let result = registry
            .request_edit(
                rust,
                &snapshot("a  \nb  \n"),
                DocumentVersion::new(4),
                LanguageServiceEditRequest::FormatSelection {
                    range: LspRange {
                        start: LspPosition {
                            line: 0,
                            character: 0,
                        },
                        end: LspPosition {
                            line: 0,
                            character: 3,
                        },
                    },
                },
                &CancellationToken::default(),
            )
            .unwrap();
        assert_eq!(
            result,
            LanguageServiceEditResponse::Formatting(vec![Edit::new(TextRange::new(1, 3), "",)])
        );
    }

    #[test]
    fn rust_organize_imports_is_deterministic_and_safe() {
        let registry = default_registry();
        let result = registry
            .request_edit(
                LanguageId::new(Language::Rust),
                &snapshot("use z::Thing;\nuse a::Thing;\nfn main() {}\n"),
                DocumentVersion::new(1),
                LanguageServiceEditRequest::OrganizeImports,
                &CancellationToken::default(),
            )
            .unwrap();
        let LanguageServiceEditResponse::OrganizeImports(edits) = result else {
            panic!("wrong response")
        };
        assert_eq!(edits.len(), 2);
        assert_eq!(edits[0].range, TextRange::new(0, 13));
        assert_eq!(edits[0].replacement, "use a::Thing;");
        assert_eq!(edits[1].range, TextRange::new(14, 27));
        assert_eq!(edits[1].replacement, "use z::Thing;");
    }

    #[test]
    fn unsupported_edit_capability_is_typed_and_cancellation_is_honest() {
        let registry = default_registry();
        let unsupported = registry.request_edit(
            LanguageId::new(Language::Python),
            &snapshot("import z\nimport a\n"),
            DocumentVersion::new(1),
            LanguageServiceEditRequest::OrganizeImports,
            &CancellationToken::default(),
        );
        assert_eq!(
            unsupported,
            Err(LanguageServiceError::UnsupportedEditCapability(
                LanguageServiceEditCapability::OrganizeImports,
            ))
        );

        let cancellation = CancellationToken::default();
        cancellation.cancel();
        let cancelled = registry.request_edit(
            LanguageId::new(Language::Rust),
            &snapshot("use z::Thing;\n"),
            DocumentVersion::new(1),
            LanguageServiceEditRequest::FormatSelection {
                range: LspRange {
                    start: LspPosition {
                        line: 0,
                        character: 0,
                    },
                    end: LspPosition {
                        line: 0,
                        character: 1,
                    },
                },
            },
            &cancellation,
        );
        assert_eq!(cancelled, Err(LanguageServiceError::Cancelled));
    }

    #[test]
    fn background_request_returns_generation_metadata() {
        let registry = default_registry();
        let (tx, rx) = std::sync::mpsc::channel();
        let handle = registry
            .request_in_background(
                BackgroundRequest {
                    language: LanguageId::new(Language::Json),
                    snapshot: snapshot("{\"x\": 1}"),
                    version: DocumentVersion::new(7),
                    generation: 3,
                    request: LanguageServiceRequest::Diagnostics,
                    cancellation: CancellationToken::default(),
                },
                move |result| {
                    tx.send(result).expect("test receiver remains alive");
                },
            )
            .expect("provider exists");
        handle.join().expect("provider thread exits");
        let result = rx.recv().expect("background result");
        assert_eq!(result.version, DocumentVersion::new(7));
        assert_eq!(result.generation, 3);
    }
}
