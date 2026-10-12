//! Editor-owned project diagnostics shared by the open document views.
//!
//! Each EditorView publishes only revision-accepted diagnostics for its own
//! file. The registry keeps that typed projection independent from Workspace;
//! Workspace supplies one shared handle to the views it composes.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

use crate::language_services::Diagnostic;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProjectDiagnosticLocation {
    pub path: PathBuf,
    pub uri: String,
    pub diagnostic: Diagnostic,
}

#[derive(Clone, Debug, Default)]
struct ProjectDiagnosticDocument {
    path: PathBuf,
    diagnostics: Vec<Diagnostic>,
}

/// A small in-memory registry for diagnostics from open Editor documents.
///
/// Updates replace one document's complete diagnostic set, which keeps push
/// and pull results from accumulating stale rows. Lock sections only clone or
/// replace in-memory values and never perform I/O.
#[derive(Clone, Debug, Default)]
pub struct ProjectDiagnosticRegistry {
    documents: Arc<RwLock<BTreeMap<String, ProjectDiagnosticDocument>>>,
}

impl ProjectDiagnosticRegistry {
    pub fn update_document(&self, path: PathBuf, uri: String, diagnostics: Vec<Diagnostic>) {
        self.documents
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .insert(uri, ProjectDiagnosticDocument { path, diagnostics });
    }

    pub fn remove_document(&self, uri: &str) {
        self.documents
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .remove(uri);
    }

    pub fn snapshot(&self) -> Vec<ProjectDiagnosticLocation> {
        let documents = self
            .documents
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        documents
            .iter()
            .flat_map(|(uri, document)| {
                document
                    .diagnostics
                    .iter()
                    .cloned()
                    .map(|diagnostic| ProjectDiagnosticLocation {
                        path: document.path.clone(),
                        uri: uri.clone(),
                        diagnostic,
                    })
                    .collect::<Vec<_>>()
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DiagnosticSeverity, LanguageServerId, LspPosition, LspRange};

    fn diagnostic(line: usize, message: &str) -> Diagnostic {
        let start = LspPosition { line, character: 0 };
        Diagnostic {
            range: LspRange { start, end: start },
            severity: DiagnosticSeverity::Warning,
            message: message.into(),
            source: LanguageServerId::new("fixture"),
            code: None,
        }
    }

    #[test]
    fn document_updates_replace_rows_and_close_removes_them() {
        let registry = ProjectDiagnosticRegistry::default();
        let first_path = PathBuf::from("src/first.rs");
        let second_path = PathBuf::from("src/second.rs");
        registry.update_document(
            first_path.clone(),
            "file:///src/first.rs".into(),
            vec![diagnostic(1, "old"), diagnostic(2, "other")],
        );
        registry.update_document(
            second_path.clone(),
            "file:///src/second.rs".into(),
            vec![diagnostic(3, "second")],
        );

        registry.update_document(
            first_path,
            "file:///src/first.rs".into(),
            vec![diagnostic(4, "fresh")],
        );
        let rows = registry.snapshot();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].diagnostic.message, "fresh");
        assert_eq!(rows[1].path, second_path);

        registry.remove_document("file:///src/first.rs");
        assert_eq!(registry.snapshot().len(), 1);
    }
}
