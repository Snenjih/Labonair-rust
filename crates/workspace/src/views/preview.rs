//! Native replacement for the reference in-app web-preview tab
//! (`reference-src/src/modules/preview/`), delivered as part of the T15-006
//! feature-parity gate.
//!
//! GPUI cannot embed a WebView, so the reference `<iframe>` preview is replaced
//! with native rendering of the cases that do not need a browser engine:
//!
//! * **Images** (`png`, `jpg`, `jpeg`, `gif`, `webp`, `bmp`, `ico`) render
//!   natively via GPUI's `img()`.
//! * **Markdown / plain text** (`md`, `markdown`, `txt`, `text`) render through
//!   the native [`markdown`](crate::markdown) parser.
//! * **Delimited tables** (`csv`, `tsv`, `ssv`, `psv`) render in a bounded
//!   native table view with quoted-field parsing.
//! * **SVG** is rasterized through `resvg` with local and remote image
//!   references disabled. HTML, PDF and remote URLs show an external-open
//!   fallback because GPUI has no embedded WebView.
//!
//! Browser-dependent documents use the native external-open fallback.

use std::collections::{HashMap, HashSet};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use gpui::prelude::*;
use gpui::{
    div, img, px, App, ClickEvent, Context, Entity, FocusHandle, Focusable, Image, ImageFormat,
    InteractiveElement, KeyDownEvent, MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent,
    ObjectFit, ScrollHandle, ScrollWheelEvent, SharedString, StatefulInteractiveElement, Window,
};

use labonair_editor::{
    BufferSnapshot, EditorBuffer, HighlightKind, Language, StyledRun, SyntaxHighlighter,
};
use labonair_notifications::{notification_center, Notification};
use labonair_settings::{EditorSettings, Settings as _};
use labonair_ui_kit::{
    button, divider, text_field, text_field_surface, text_input, Axis, ButtonSize, ButtonVariant,
    InputEvent, InputState, Palette, TextFieldState, Tooltip,
};

use crate::markdown::{parse_markdown, Inline, MdBlock};
use crate::theme::ThemeStore;

/// The previewable-extension list and predicate are shared with the Explorer
/// panel through the `labonair-explorer-host` value crate.
pub use labonair_explorer_host::{is_previewable, PREVIEW_EXTENSIONS};

fn ext_of(target: &str) -> Option<String> {
    let path = match target.split_once("://") {
        Some((_, rest)) => rest,
        None => target,
    };
    Path::new(path)
        .extension()
        .and_then(|s| s.to_str())
        .map(|s| s.to_ascii_lowercase())
}

fn is_remote_url(target: &str) -> bool {
    target.starts_with("http://") || target.starts_with("https://")
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum MarkdownLinkTarget {
    External(String),
    Anchor(String),
    Local {
        path: PathBuf,
        fragment: Option<String>,
    },
}

fn resolve_markdown_link(base: &str, href: &str) -> Option<MarkdownLinkTarget> {
    let href = href.trim();
    if href.is_empty() {
        return None;
    }
    if let Some(fragment) = href.strip_prefix('#') {
        return Some(MarkdownLinkTarget::Anchor(fragment.to_string()));
    }
    if let Some((scheme, _)) = href.split_once(':') {
        let is_scheme = !scheme.is_empty()
            && scheme.chars().enumerate().all(|(index, character)| {
                character.is_ascii_alphabetic()
                    || (index > 0
                        && (character.is_ascii_digit() || matches!(character, '+' | '.' | '-')))
            });
        if is_scheme {
            return matches!(
                scheme.to_ascii_lowercase().as_str(),
                "http" | "https" | "mailto"
            )
            .then(|| MarkdownLinkTarget::External(href.to_string()));
        }
    }
    if is_remote_url(base) {
        return None;
    }
    let (target, fragment) = href
        .split_once('#')
        .map(|(target, fragment)| (target, Some(fragment.to_string())))
        .unwrap_or((href, None));
    let target = target.split('?').next().unwrap_or_default();
    if target.is_empty() {
        return None;
    }
    let target = Path::new(target);
    let path = if target.is_absolute() {
        target.to_path_buf()
    } else {
        Path::new(base).parent()?.join(target)
    };
    Some(MarkdownLinkTarget::Local { path, fragment })
}

fn resolve_markdown_image_path(base: &str, src: &str) -> Option<PathBuf> {
    match resolve_markdown_link(base, src)? {
        MarkdownLinkTarget::Local { path, .. } => Some(path),
        MarkdownLinkTarget::External(_) | MarkdownLinkTarget::Anchor(_) => None,
    }
}

fn markdown_fence_language(value: &str) -> Option<Language> {
    let value = value
        .trim()
        .trim_start_matches('{')
        .trim_start_matches('.')
        .split_whitespace()
        .next()?
        .trim_end_matches('}')
        .to_ascii_lowercase();
    Some(match value.as_str() {
        "rs" | "rust" => Language::Rust,
        "toml" => Language::Toml,
        "json" | "jsonc" => Language::Json,
        "yaml" | "yml" => Language::Yaml,
        "md" | "markdown" => Language::Markdown,
        "html" | "htm" => Language::Html,
        "xml" | "svg" => Language::Xml,
        "css" | "scss" | "less" => Language::Css,
        "js" | "javascript" | "jsx" | "mjs" | "cjs" => Language::JavaScript,
        "ts" | "typescript" | "tsx" => Language::TypeScript,
        "py" | "python" | "pyi" => Language::Python,
        "go" => Language::Go,
        "c" | "h" => Language::C,
        "cc" | "cpp" | "cxx" | "hpp" | "c++" => Language::Cpp,
        "sh" | "bash" | "shell" => Language::Shell,
        "sql" => Language::Sql,
        "java" => Language::Java,
        "php" => Language::Php,
        "rb" | "ruby" => Language::Ruby,
        "swift" => Language::Swift,
        "kt" | "kotlin" => Language::Kotlin,
        _ => return None,
    })
}

fn markdown_image_sources(blocks: &[MdBlock]) -> Vec<String> {
    fn append(spans: &[Inline], seen: &mut HashSet<String>, sources: &mut Vec<String>) {
        for span in spans {
            if let Inline::Image { src, .. } = span {
                if seen.insert(src.clone()) {
                    sources.push(src.clone());
                }
            }
        }
    }

    let mut seen = HashSet::new();
    let mut sources = Vec::new();
    for block in blocks {
        match block {
            MdBlock::Heading { spans, .. } | MdBlock::Paragraph(spans) | MdBlock::Quote(spans) => {
                append(spans, &mut seen, &mut sources)
            }
            MdBlock::Bullets(items) => {
                for item in items {
                    append(item, &mut seen, &mut sources);
                }
            }
            MdBlock::Ordered(items) => {
                for (_, item) in items {
                    append(item, &mut seen, &mut sources);
                }
            }
            MdBlock::Table { headers, rows } => {
                for cell in headers.iter().chain(rows.iter().flatten()) {
                    append(cell, &mut seen, &mut sources);
                }
            }
            MdBlock::Code { .. } | MdBlock::Rule => {}
        }
    }
    sources
}

fn heading_index_for_fragment(blocks: &[MdBlock], fragment: &str) -> Option<usize> {
    let fragment = fragment.strip_prefix('#').unwrap_or(fragment);
    let mut seen = HashMap::<String, usize>::new();
    for (index, block) in blocks.iter().enumerate() {
        let MdBlock::Heading { spans, .. } = block else {
            continue;
        };
        let text = spans.iter().map(Inline::plain).collect::<String>();
        let base = heading_slug(&text);
        if base.is_empty() {
            continue;
        }
        let duplicate_index = seen.entry(base.clone()).or_default();
        let slug = if *duplicate_index == 0 {
            base
        } else {
            format!("{base}-{duplicate_index}")
        };
        *duplicate_index += 1;
        if slug == fragment {
            return Some(index);
        }
    }
    None
}

fn heading_slug(text: &str) -> String {
    let mut slug = String::new();
    let mut pending_dash = false;
    for character in text.chars().flat_map(char::to_lowercase) {
        if character.is_alphanumeric() {
            if pending_dash && !slug.is_empty() {
                slug.push('-');
            }
            slug.push(character);
            pending_dash = false;
        } else {
            pending_dash = true;
        }
    }
    slug
}

/// Resolved preview payload for the current address.
enum Content {
    Empty,
    Loading,
    Image(PreviewImage),
    Markdown(Vec<MdBlock>),
    Table(TabularPreview),
    /// Needs a browser engine — offer "open externally" only. `why` explains it.
    External {
        why: SharedString,
    },
    Unsupported(SharedString),
    Error(SharedString),
}

#[derive(Clone)]
struct PreviewImage {
    image: Arc<Image>,
    width: u32,
    height: u32,
    format: String,
    bits_per_pixel: u16,
    byte_len: usize,
}

#[derive(Clone, Default)]
struct TabularPreview {
    rows: Vec<Vec<String>>,
    column_count: usize,
    total_rows: usize,
}

const MAX_TABLE_PREVIEW_ROWS: usize = 1_000;
const MAX_MARKDOWN_CODE_BLOCK_BYTES: usize = 512 * 1024;

#[derive(Clone, Copy, Debug, PartialEq)]
enum ImageZoom {
    Fit,
    ActualSize,
    Scale(f32),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PreviewImageAction {
    ZoomIn,
    ZoomOut,
    ResetZoom,
    ZoomToActualSize,
    ZoomToFit,
}

#[derive(Clone, Copy)]
struct ImagePan {
    pointer_x: f32,
    pointer_y: f32,
    scroll_x: f32,
    scroll_y: f32,
}

const MIN_IMAGE_ZOOM: f32 = 0.05;
const MAX_IMAGE_ZOOM: f32 = 8.0;
const MAX_SVG_EDGE_PIXELS: f32 = 2_048.0;
const MAX_SVG_SOURCE_BYTES: u64 = 16 * 1024 * 1024;

fn fit_image_scale(image_width: u32, image_height: u32, width: f32, height: f32) -> f32 {
    if image_width == 0 || image_height == 0 || width <= 0.0 || height <= 0.0 {
        return 1.0;
    }
    ((width / image_width as f32).min(height / image_height as f32))
        .min(1.0)
        .clamp(MIN_IMAGE_ZOOM, MAX_IMAGE_ZOOM)
}

fn image_zoom_scale(zoom: ImageZoom, fit_scale: f32) -> f32 {
    match zoom {
        ImageZoom::Fit => fit_scale,
        ImageZoom::ActualSize => 1.0,
        ImageZoom::Scale(scale) => scale.clamp(MIN_IMAGE_ZOOM, MAX_IMAGE_ZOOM),
    }
}

fn parse_image_zoom_percent(value: &str) -> Option<ImageZoom> {
    let value = value.trim();
    let value = value.strip_suffix('%').unwrap_or(value).trim();
    let percent = value.parse::<f32>().ok()?;
    if !percent.is_finite() {
        return None;
    }
    if (percent - 100.0).abs() < f32::EPSILON {
        Some(ImageZoom::ActualSize)
    } else {
        Some(ImageZoom::Scale(
            (percent / 100.0).clamp(MIN_IMAGE_ZOOM, MAX_IMAGE_ZOOM),
        ))
    }
}

fn image_zoom_input_seed(zoom: ImageZoom) -> String {
    match zoom {
        ImageZoom::Fit | ImageZoom::ActualSize => "100".into(),
        ImageZoom::Scale(scale) => format!("{:.0}", scale * 100.0),
    }
}

fn image_zoom_preset(action: PreviewImageAction) -> Option<ImageZoom> {
    match action {
        PreviewImageAction::ResetZoom | PreviewImageAction::ZoomToActualSize => {
            Some(ImageZoom::ActualSize)
        }
        PreviewImageAction::ZoomToFit => Some(ImageZoom::Fit),
        PreviewImageAction::ZoomIn | PreviewImageAction::ZoomOut => None,
    }
}

fn image_scroll_offset_after_zoom(anchor: f32, offset: f32, ratio: f32) -> f32 {
    anchor - (anchor - offset) * ratio
}

fn image_scroll_offset_after_pan(pointer_origin: f32, scroll_origin: f32, pointer_now: f32) -> f32 {
    scroll_origin + pointer_now - pointer_origin
}

fn format_image_size(
    byte_len: usize,
    unit: labonair_settings::content::editor::ImageViewerUnit,
) -> String {
    use labonair_settings::content::editor::ImageViewerUnit;

    let (base, units): (f64, &[&str]) = match unit {
        ImageViewerUnit::Binary => (1024.0, &["KiB", "MiB", "GiB", "TiB", "PiB", "EiB"]),
        ImageViewerUnit::Decimal => (1000.0, &["kB", "MB", "GB", "TB", "PB", "EB"]),
    };
    let mut size = byte_len as f64;
    let mut unit_index = 0;
    while size >= base && unit_index < units.len() {
        size /= base;
        unit_index += 1;
    }
    if unit_index == 0 {
        format!("{byte_len} B")
    } else {
        format!("{size:.1} {}", units[unit_index - 1])
    }
}

impl PreviewImage {
    fn metadata(&self, unit: labonair_settings::content::editor::ImageViewerUnit) -> String {
        format!(
            "{} × {} · {} · {} bpp · {}",
            self.width,
            self.height,
            self.format,
            self.bits_per_pixel,
            format_image_size(self.byte_len, unit)
        )
    }
}

/// A single preview tab. Feed it an address (local path or URL) via
/// [`Self::set_url`].
pub struct PreviewView {
    theme: Entity<ThemeStore>,
    focus_handle: FocusHandle,
    url: String,
    content: Content,
    source_bound: bool,
    load_generation: u64,
    pending_fragment: Option<String>,
    markdown_scroll: ScrollHandle,
    markdown_images: HashMap<String, PreviewImage>,
    markdown_source_path: Option<PathBuf>,
    markdown_syntax_runs: HashMap<usize, Vec<Vec<StyledRun>>>,
    image_scroll: ScrollHandle,
    image_zoom: ImageZoom,
    image_zoom_input: Option<Entity<InputState>>,
    image_pan: Option<ImagePan>,
}

impl PreviewView {
    pub fn new(theme: Entity<ThemeStore>, cx: &mut Context<Self>) -> Self {
        cx.observe(&theme, |_, _, cx| cx.notify()).detach();
        Self {
            theme,
            focus_handle: cx.focus_handle(),
            url: String::new(),
            content: Content::Empty,
            source_bound: false,
            load_generation: 0,
            pending_fragment: None,
            markdown_scroll: ScrollHandle::new(),
            markdown_images: HashMap::new(),
            markdown_source_path: None,
            markdown_syntax_runs: HashMap::new(),
            image_scroll: ScrollHandle::new(),
            image_zoom: ImageZoom::Fit,
            image_zoom_input: None,
            image_pan: None,
        }
    }

    pub fn url(&self) -> &str {
        &self.url
    }

    /// Point the pane at a new address and re-resolve its content.
    pub fn set_url(&mut self, url: impl Into<String>, cx: &mut Context<Self>) {
        self.url = url.into();
        self.source_bound = false;
        self.pending_fragment = None;
        self.markdown_source_path = None;
        self.markdown_images.clear();
        self.markdown_syntax_runs.clear();
        self.image_zoom = ImageZoom::Fit;
        self.image_zoom_input = None;
        self.image_pan = None;
        self.image_scroll.set_offset(gpui::point(px(0.0), px(0.0)));
        self.resolve_content(cx);
        cx.notify();
    }

    fn set_url_with_fragment(
        &mut self,
        url: impl Into<String>,
        fragment: Option<String>,
        cx: &mut Context<Self>,
    ) {
        self.url = url.into();
        self.source_bound = false;
        self.pending_fragment = fragment;
        self.markdown_source_path = None;
        self.markdown_images.clear();
        self.markdown_syntax_runs.clear();
        self.image_zoom = ImageZoom::Fit;
        self.image_zoom_input = None;
        self.image_pan = None;
        self.image_scroll.set_offset(gpui::point(px(0.0), px(0.0)));
        self.resolve_content(cx);
        cx.notify();
    }

    /// Bind this preview tab to an immutable Markdown editor revision.
    /// Switching the source editor to another language leaves an explicit
    /// unavailable state instead of stale rendered content.
    pub fn set_editor_source(
        &mut self,
        path: Option<PathBuf>,
        language: Language,
        snapshot: BufferSnapshot,
        cx: &mut Context<Self>,
    ) {
        self.load_generation = self.load_generation.wrapping_add(1);
        self.pending_fragment = None;
        if self.markdown_source_path != path || language != Language::Markdown {
            self.markdown_images.clear();
        }
        self.markdown_syntax_runs.clear();
        self.markdown_source_path = path.clone();
        self.image_zoom = ImageZoom::Fit;
        self.image_zoom_input = None;
        self.image_pan = None;
        self.image_scroll.set_offset(gpui::point(px(0.0), px(0.0)));
        self.url = path
            .as_ref()
            .map(|path| path.to_string_lossy().into_owned())
            .unwrap_or_else(|| "untitled.md".to_string());
        self.source_bound = true;
        self.content = if language == Language::Markdown {
            Content::Markdown(parse_markdown(&snapshot.text()))
        } else {
            Content::Unsupported("The followed editor buffer is not Markdown.".into())
        };
        self.load_markdown_images(cx);
        self.load_markdown_code_highlights(cx);
        cx.notify();
    }

    /// Render an immutable SVG editor revision in the background. This keeps
    /// following previews in sync with unsaved edits and never blocks GPUI.
    pub fn set_editor_svg_source(
        &mut self,
        path: Option<PathBuf>,
        snapshot: BufferSnapshot,
        cx: &mut Context<Self>,
    ) {
        self.load_generation = self.load_generation.wrapping_add(1);
        let generation = self.load_generation;
        let next_url = path
            .as_ref()
            .map(|path| path.to_string_lossy().into_owned())
            .unwrap_or_else(|| "untitled.svg".to_string());
        let changed_source = !self.source_bound || self.url != next_url;
        self.pending_fragment = None;
        self.markdown_images.clear();
        self.markdown_syntax_runs.clear();
        self.markdown_source_path = None;
        if changed_source {
            self.image_zoom = ImageZoom::Fit;
            self.image_zoom_input = None;
            self.image_pan = None;
            self.image_scroll.set_offset(gpui::point(px(0.0), px(0.0)));
        }
        self.url = next_url;
        self.source_bound = true;

        let is_svg = path
            .as_deref()
            .and_then(Path::extension)
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| extension.eq_ignore_ascii_case("svg"));
        if !is_svg {
            self.content = Content::Unsupported("The followed editor buffer is not SVG.".into());
            cx.notify();
            return;
        }

        self.content = Content::Loading;
        cx.spawn(async move |this, cx| {
            let result = cx
                .background_executor()
                .spawn(async move {
                    if snapshot.byte_len() as u64 > MAX_SVG_SOURCE_BYTES {
                        return Err(format!(
                            "SVG exceeds the {} MiB preview limit",
                            MAX_SVG_SOURCE_BYTES / (1024 * 1024)
                        ));
                    }
                    let source = snapshot.text();
                    render_svg(source.as_bytes())
                })
                .await;
            let _ = this.update(cx, |this, cx| {
                if this.load_generation != generation {
                    return;
                }
                match result {
                    Ok(image) => this.content = Content::Image(image),
                    Err(error) => {
                        let url = this.url.clone();
                        let message: SharedString = format!("Could not render SVG: {error}").into();
                        notification_center(cx).update(cx, |center, cx| {
                            center.push(
                                Notification::error(
                                    "Preview unavailable",
                                    "Could not render the SVG preview.",
                                )
                                .source("preview")
                                .details(message.clone())
                                .dedupe_key(format!("preview:svg:{url}")),
                                cx,
                            );
                        });
                        this.content = Content::Error(message);
                    }
                }
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }

    /// Parse an immutable table editor revision off the GPUI thread.
    pub fn set_editor_tabular_source(
        &mut self,
        path: Option<PathBuf>,
        snapshot: BufferSnapshot,
        cx: &mut Context<Self>,
    ) {
        self.load_generation = self.load_generation.wrapping_add(1);
        let generation = self.load_generation;
        let next_url = path
            .as_ref()
            .map(|path| path.to_string_lossy().into_owned())
            .unwrap_or_else(|| "untitled.csv".to_string());
        self.pending_fragment = None;
        self.markdown_images.clear();
        self.markdown_syntax_runs.clear();
        self.markdown_source_path = None;
        self.image_zoom = ImageZoom::Fit;
        self.image_zoom_input = None;
        self.image_pan = None;
        self.image_scroll.set_offset(gpui::point(px(0.0), px(0.0)));
        self.url = next_url;
        self.source_bound = true;

        let delimiter = path
            .as_deref()
            .and_then(Path::extension)
            .and_then(|extension| extension.to_str())
            .map(str::to_ascii_lowercase)
            .and_then(|extension| match extension.as_str() {
                "csv" => Some(','),
                "tsv" => Some('\t'),
                "ssv" => Some(';'),
                "psv" => Some('|'),
                _ => None,
            });
        let Some(delimiter) = delimiter else {
            self.content =
                Content::Unsupported("The followed editor buffer is not tabular data.".into());
            cx.notify();
            return;
        };

        self.content = Content::Loading;
        cx.spawn(async move |this, cx| {
            let result = cx
                .background_executor()
                .spawn(async move {
                    let text = snapshot.text();
                    parse_delimited(&text, delimiter)
                })
                .await;
            let _ = this.update(cx, |this, cx| {
                if this.load_generation != generation {
                    return;
                }
                match result {
                    Ok(table) => this.content = Content::Table(table),
                    Err(error) => {
                        let url = this.url.clone();
                        let message: SharedString = error.into();
                        notification_center(cx).update(cx, |center, cx| {
                            center.push(
                                Notification::error(
                                    "Preview unavailable",
                                    "Could not parse the table preview.",
                                )
                                .source("preview")
                                .details(message.clone())
                                .dedupe_key(format!("preview:table:{url}")),
                                cx,
                            );
                        });
                        this.content = Content::Error(message);
                    }
                }
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }

    /// Re-read the current address from disk.
    pub fn reload(&mut self, cx: &mut Context<Self>) {
        if self.source_bound {
            return;
        }
        self.resolve_content(cx);
        cx.notify();
    }

    fn resolve_content(&mut self, cx: &mut Context<Self>) {
        self.load_generation = self.load_generation.wrapping_add(1);
        let generation = self.load_generation;
        self.markdown_images.clear();
        self.markdown_syntax_runs.clear();
        self.markdown_source_path = None;
        let url = self.url.clone();
        if url.is_empty() || is_remote_url(&url) {
            self.content = resolve(&url);
            return;
        }

        self.content = Content::Loading;
        cx.spawn(async move |this, cx| {
            let content = cx
                .background_executor()
                .spawn(async move { resolve(&url) })
                .await;
            let _ = this.update(cx, |this, cx| {
                if this.load_generation != generation {
                    return;
                }
                if let Content::Error(message) = &content {
                    let url = this.url.clone();
                    notification_center(cx).update(cx, |center, cx| {
                        center.push(
                            Notification::error(
                                "Preview unavailable",
                                "Could not load the preview.",
                            )
                            .source("preview")
                            .details(message.clone())
                            .dedupe_key(format!("preview:load:{url}")),
                            cx,
                        );
                    });
                }
                this.content = content;
                this.load_markdown_images(cx);
                this.load_markdown_code_highlights(cx);
                if let Some(fragment) = this.pending_fragment.take() {
                    this.scroll_to_heading(&fragment, cx);
                }
                cx.notify();
            });
        })
        .detach();
    }

    fn load_markdown_images(&mut self, cx: &mut Context<Self>) {
        let Content::Markdown(blocks) = &self.content else {
            return;
        };
        let Some(base) = self
            .markdown_source_path
            .as_ref()
            .map(|path| path.to_string_lossy().into_owned())
            .or_else(|| (!self.source_bound).then(|| self.url.clone()))
        else {
            return;
        };
        let generation = self.load_generation;
        let referenced_sources = markdown_image_sources(blocks);
        self.markdown_images
            .retain(|source, _| referenced_sources.contains(source));
        let sources = referenced_sources
            .into_iter()
            .filter_map(|source| {
                if self.markdown_images.contains_key(&source) {
                    return None;
                }
                resolve_markdown_image_path(&base, &source).map(|path| (source, path))
            })
            .collect::<Vec<_>>();
        if sources.is_empty() {
            return;
        }
        cx.spawn(async move |this, cx| {
            let images = cx
                .background_executor()
                .spawn(async move {
                    sources
                        .into_iter()
                        .filter_map(|(source, path)| {
                            let path = path.to_string_lossy().into_owned();
                            load_image(&path).ok().map(|image| (source, image))
                        })
                        .collect::<HashMap<_, _>>()
                })
                .await;
            let _ = this.update(cx, |this, cx| {
                if this.load_generation != generation {
                    return;
                }
                this.markdown_images.extend(images);
                cx.notify();
            });
        })
        .detach();
    }

    fn load_markdown_code_highlights(&mut self, cx: &mut Context<Self>) {
        let Content::Markdown(blocks) = &self.content else {
            return;
        };
        let sources = blocks
            .iter()
            .enumerate()
            .filter_map(|(index, block)| {
                let MdBlock::Code { lang, text, .. } = block else {
                    return None;
                };
                if text.len() > MAX_MARKDOWN_CODE_BLOCK_BYTES {
                    return None;
                }
                Some((
                    index,
                    markdown_fence_language(lang.as_deref()?)?,
                    text.clone(),
                ))
            })
            .collect::<Vec<_>>();
        if sources.is_empty() {
            return;
        }
        let generation = self.load_generation;
        cx.spawn(async move |this, cx| {
            let highlights = cx
                .background_executor()
                .spawn(async move {
                    sources
                        .into_iter()
                        .map(|(index, language, text)| {
                            let buffer = EditorBuffer::from_text(&text);
                            let snapshot = buffer.snapshot();
                            let mut highlighter = SyntaxHighlighter::new(language);
                            highlighter.update(&snapshot, 0..snapshot.byte_len());
                            let lines = (0..snapshot.line_count())
                                .map(|line| highlighter.line_runs(&snapshot, line))
                                .collect();
                            (index, lines)
                        })
                        .collect::<HashMap<_, _>>()
                })
                .await;
            let _ = this.update(cx, |this, cx| {
                if this.load_generation != generation {
                    return;
                }
                this.markdown_syntax_runs = highlights;
                cx.notify();
            });
        })
        .detach();
    }

    fn open_external(&self) {
        if !self.url.is_empty() {
            open_external(&self.url);
        }
    }

    fn open_markdown_link(&mut self, base: &str, href: &str, cx: &mut Context<Self>) {
        match resolve_markdown_link(base, href) {
            Some(MarkdownLinkTarget::External(target)) => open_external(&target),
            Some(MarkdownLinkTarget::Anchor(fragment)) => {
                self.scroll_to_heading(&fragment, cx);
            }
            Some(MarkdownLinkTarget::Local { path, fragment }) => {
                self.set_url_with_fragment(path.to_string_lossy().into_owned(), fragment, cx);
            }
            None => {}
        }
    }

    fn scroll_to_heading(&mut self, fragment: &str, cx: &mut Context<Self>) {
        let Content::Markdown(blocks) = &self.content else {
            return;
        };
        if let Some(index) = heading_index_for_fragment(blocks, fragment) {
            self.markdown_scroll.scroll_to_item(index);
            cx.notify();
        }
    }

    fn zoom_image(
        &mut self,
        factor: f32,
        viewport_width: f32,
        viewport_height: f32,
        anchor: Option<gpui::Point<gpui::Pixels>>,
        cx: &mut Context<Self>,
    ) {
        let Content::Image(image) = &self.content else {
            return;
        };
        let bounds = self.image_scroll.bounds();
        let content_width = if f32::from(bounds.size.width) > 0.0 {
            f32::from(bounds.size.width) - 16.0
        } else {
            viewport_width - 32.0
        };
        let content_height = if f32::from(bounds.size.height) > 0.0 {
            f32::from(bounds.size.height) - 16.0
        } else {
            viewport_height - 64.0
        };
        let fit_scale = fit_image_scale(
            image.width,
            image.height,
            content_width.max(1.0),
            content_height.max(1.0),
        );
        let current = image_zoom_scale(self.image_zoom, fit_scale);
        let next = (current * factor).clamp(MIN_IMAGE_ZOOM, MAX_IMAGE_ZOOM);
        let offset = self.image_scroll.offset();
        let anchor = anchor
            .map(|point| {
                (
                    f32::from(point.x - bounds.origin.x),
                    f32::from(point.y - bounds.origin.y),
                )
            })
            .unwrap_or_else(|| {
                (
                    f32::from(bounds.size.width) / 2.0,
                    f32::from(bounds.size.height) / 2.0,
                )
            });
        let ratio = next / current.max(MIN_IMAGE_ZOOM);
        let offset_x = image_scroll_offset_after_zoom(anchor.0, f32::from(offset.x), ratio);
        let offset_y = image_scroll_offset_after_zoom(anchor.1, f32::from(offset.y), ratio);
        self.image_scroll
            .set_offset(gpui::point(px(offset_x), px(offset_y)));
        self.image_zoom = ImageZoom::Scale(next);
        cx.notify();
    }

    fn set_image_zoom(&mut self, zoom: ImageZoom, cx: &mut Context<Self>) {
        if matches!(self.content, Content::Image(_)) {
            self.image_zoom = zoom;
            self.image_zoom_input = None;
            self.image_scroll.set_offset(gpui::point(px(0.0), px(0.0)));
            cx.notify();
        }
    }

    fn edit_image_zoom(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !matches!(self.content, Content::Image(_)) || self.image_zoom_input.is_some() {
            return;
        }
        let input = cx
            .new(|cx| text_field(window, cx).default_value(image_zoom_input_seed(self.image_zoom)));
        let view = cx.entity();
        window
            .subscribe(&input, cx, move |input, event: &InputEvent, _window, cx| {
                if let InputEvent::PressEnter { secondary: false } = event {
                    let value = input.read(cx).value().to_string();
                    if let Some(zoom) = parse_image_zoom_percent(&value) {
                        view.update(cx, |this, cx| this.set_image_zoom(zoom, cx));
                    }
                }
            })
            .detach();
        window.focus(&input.read(cx).focus_handle(cx));
        self.image_zoom_input = Some(input);
        cx.notify();
    }

    pub fn perform_image_action(
        &mut self,
        action: PreviewImageAction,
        viewport_width: f32,
        viewport_height: f32,
        cx: &mut Context<Self>,
    ) {
        match action {
            PreviewImageAction::ZoomIn => {
                self.zoom_image(1.25, viewport_width, viewport_height, None, cx)
            }
            PreviewImageAction::ZoomOut => {
                self.zoom_image(0.8, viewport_width, viewport_height, None, cx)
            }
            PreviewImageAction::ResetZoom
            | PreviewImageAction::ZoomToActualSize
            | PreviewImageAction::ZoomToFit => {
                if let Some(zoom) = image_zoom_preset(action) {
                    self.set_image_zoom(zoom, cx);
                }
            }
        }
    }

    fn begin_image_pan(&mut self, position: gpui::Point<gpui::Pixels>, cx: &mut Context<Self>) {
        if !matches!(self.content, Content::Image(_)) {
            return;
        }
        let offset = self.image_scroll.offset();
        self.image_pan = Some(ImagePan {
            pointer_x: f32::from(position.x),
            pointer_y: f32::from(position.y),
            scroll_x: f32::from(offset.x),
            scroll_y: f32::from(offset.y),
        });
        cx.stop_propagation();
        cx.notify();
    }

    fn move_image_pan(&mut self, position: gpui::Point<gpui::Pixels>, cx: &mut Context<Self>) {
        let Some(pan) = self.image_pan else {
            return;
        };
        let offset_x =
            image_scroll_offset_after_pan(pan.pointer_x, pan.scroll_x, f32::from(position.x));
        let offset_y =
            image_scroll_offset_after_pan(pan.pointer_y, pan.scroll_y, f32::from(position.y));
        self.image_scroll
            .set_offset(gpui::point(px(offset_x), px(offset_y)));
        cx.stop_propagation();
        cx.notify();
    }

    fn end_image_pan(&mut self, cx: &mut Context<Self>) {
        if self.image_pan.take().is_some() {
            cx.stop_propagation();
            cx.notify();
        }
    }
}

fn resolve(url: &str) -> Content {
    if url.is_empty() {
        return Content::Empty;
    }
    if is_remote_url(url) {
        return Content::External {
            why: "Remote pages open in your default browser (GPUI has no embedded WebView).".into(),
        };
    }

    let ext = ext_of(url).unwrap_or_default();
    match ext.as_str() {
        "png" | "jpg" | "jpeg" | "gif" | "webp" | "bmp" | "ico" => match load_image(url) {
            Ok(image) => Content::Image(image),
            Err(e) => Content::Error(format!("Could not load image: {e}").into()),
        },
        "md" | "markdown" | "txt" | "text" => match std::fs::read_to_string(url) {
            Ok(text) => Content::Markdown(parse_markdown(&text)),
            Err(e) => Content::Error(format!("Could not read file: {e}").into()),
        },
        "csv" | "tsv" | "ssv" | "psv" => {
            let delimiter = match ext.as_str() {
                "tsv" => '\t',
                "ssv" => ';',
                "psv" => '|',
                _ => ',',
            };
            match std::fs::read_to_string(url).map_err(|error| error.to_string()) {
                Ok(text) => match parse_delimited(&text, delimiter) {
                    Ok(table) => Content::Table(table),
                    Err(error) => Content::Error(error.into()),
                },
                Err(error) => Content::Error(format!("Could not read file: {error}").into()),
            }
        }
        "html" | "htm" => Content::External {
            why: "HTML pages need a browser engine — opening in your default browser.".into(),
        },
        "pdf" => Content::External {
            why: "PDF files open in your system PDF viewer.".into(),
        },
        "svg" => match load_svg(url) {
            Ok(image) => Content::Image(image),
            Err(e) => Content::Error(format!("Could not render SVG: {e}").into()),
        },
        _ => Content::External {
            why: "This file type opens in your default application.".into(),
        },
    }
}

fn parse_delimited(text: &str, delimiter: char) -> Result<TabularPreview, String> {
    if text.is_empty() {
        return Ok(TabularPreview::default());
    }
    let normalized = text.replace("\r\n", "\n").replace('\r', "\n");
    let mut rows = Vec::new();
    let mut row = Vec::new();
    let mut field = String::new();
    let mut in_quotes = false;
    let mut ended_on_row_boundary = false;
    let mut column_count = 0;
    let mut total_rows = 0;
    let mut characters = normalized.chars().peekable();

    while let Some(character) = characters.next() {
        if in_quotes {
            if character == '"' {
                if characters.peek() == Some(&'"') {
                    characters.next();
                    field.push('"');
                } else {
                    in_quotes = false;
                }
            } else {
                field.push(character);
            }
            ended_on_row_boundary = false;
            continue;
        }

        match character {
            '"' if field.is_empty() => {
                in_quotes = true;
                ended_on_row_boundary = false;
            }
            '\n' => {
                row.push(std::mem::take(&mut field));
                total_rows += 1;
                column_count = column_count.max(row.len());
                if rows.len() < MAX_TABLE_PREVIEW_ROWS {
                    rows.push(std::mem::take(&mut row));
                } else {
                    row.clear();
                }
                ended_on_row_boundary = true;
            }
            value if value == delimiter => {
                row.push(std::mem::take(&mut field));
                ended_on_row_boundary = false;
            }
            value => {
                field.push(value);
                ended_on_row_boundary = false;
            }
        }
    }

    if in_quotes {
        return Err("The table contains an unterminated quoted field.".to_string());
    }
    if !ended_on_row_boundary {
        row.push(field);
        total_rows += 1;
        column_count = column_count.max(row.len());
        if rows.len() < MAX_TABLE_PREVIEW_ROWS {
            rows.push(row);
        }
    }

    Ok(TabularPreview {
        rows,
        column_count,
        total_rows,
    })
}

fn load_image(path: &str) -> Result<PreviewImage, String> {
    let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
    // Validate + normalise via the `image` crate, re-encoding to PNG so GPUI's
    // decoder always gets a format it supports (mirrors `background.rs`).
    let decoded = image::load_from_memory(&bytes).map_err(|e| e.to_string())?;
    let width = decoded.width();
    let height = decoded.height();
    let bits_per_pixel = decoded.color().bits_per_pixel();
    let format = image::guess_format(&bytes)
        .map(|format| format!("{format:?}").to_uppercase())
        .unwrap_or_else(|_| "IMAGE".to_string());
    let byte_len = bytes.len();
    let mut out = std::io::Cursor::new(Vec::new());
    decoded
        .write_to(&mut out, image::ImageFormat::Png)
        .map_err(|e| e.to_string())?;
    Ok(PreviewImage {
        image: Arc::new(Image::from_bytes(ImageFormat::Png, out.into_inner())),
        width,
        height,
        format,
        bits_per_pixel,
        byte_len,
    })
}

fn load_svg(path: &str) -> Result<PreviewImage, String> {
    let file = std::fs::File::open(path).map_err(|error| error.to_string())?;
    let metadata = file.metadata().map_err(|error| error.to_string())?;
    if metadata.len() > MAX_SVG_SOURCE_BYTES {
        return Err(format!(
            "SVG exceeds the {} MiB preview limit",
            MAX_SVG_SOURCE_BYTES / (1024 * 1024)
        ));
    }
    let mut bytes = Vec::new();
    file.take(MAX_SVG_SOURCE_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| error.to_string())?;
    if bytes.len() as u64 > MAX_SVG_SOURCE_BYTES {
        return Err(format!(
            "SVG exceeds the {} MiB preview limit",
            MAX_SVG_SOURCE_BYTES / (1024 * 1024)
        ));
    }
    render_svg(&bytes)
}

fn render_svg(bytes: &[u8]) -> Result<PreviewImage, String> {
    if bytes.len() as u64 > MAX_SVG_SOURCE_BYTES {
        return Err(format!(
            "SVG exceeds the {} MiB preview limit",
            MAX_SVG_SOURCE_BYTES / (1024 * 1024)
        ));
    }
    let mut options = resvg::usvg::Options::default();
    // Do not let a document read local or remote files through <image href>.
    // Embedded data URLs remain supported by the default data resolver.
    options.image_href_resolver.resolve_string = Box::new(|_, _| None);
    let tree = resvg::usvg::Tree::from_data(bytes, &options).map_err(|error| error.to_string())?;
    let size = tree.size();
    let scale = (MAX_SVG_EDGE_PIXELS / size.width())
        .min(MAX_SVG_EDGE_PIXELS / size.height())
        .min(1.0);
    let width = (size.width() * scale).round().max(1.0) as u32;
    let height = (size.height() * scale).round().max(1.0) as u32;
    let mut pixmap = resvg::tiny_skia::Pixmap::new(width, height)
        .ok_or_else(|| "SVG has invalid or unsupported dimensions".to_string())?;
    resvg::render(
        &tree,
        resvg::tiny_skia::Transform::default(),
        &mut pixmap.as_mut(),
    );
    let png = pixmap.encode_png().map_err(|error| error.to_string())?;
    Ok(PreviewImage {
        image: Arc::new(Image::from_bytes(ImageFormat::Png, png)),
        width,
        height,
        format: "SVG".to_string(),
        bits_per_pixel: 32,
        byte_len: bytes.len(),
    })
}

/// Hand `target` (a path or URL) to the OS "open" handler.
pub fn open_external(target: &str) {
    #[cfg(target_os = "macos")]
    let program = "/usr/bin/open";
    #[cfg(not(target_os = "macos"))]
    let program = "xdg-open";

    if let Err(e) = std::process::Command::new(program).arg(target).spawn() {
        tracing::warn!(%e, target, "preview: failed to open externally");
    }
}

impl Focusable for PreviewView {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for PreviewView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = self.theme.read(cx);
        let image_viewer_unit = EditorSettings::try_get(cx)
            .and_then(|settings| settings.content().image_viewer_unit)
            .unwrap_or_default();
        let bg = theme.background();
        let fg = theme.foreground();
        let muted = theme.muted_foreground();
        let border = theme.border();
        let accent = theme.accent();
        let c = Palette::from_theme(theme);

        let address = if self.url.is_empty() {
            SharedString::from("No address")
        } else {
            SharedString::from(self.url.clone())
        };

        let body: gpui::AnyElement = match &self.content {
            Content::Empty => div()
                .flex_1()
                .flex()
                .items_center()
                .justify_center()
                .text_color(muted)
                .child("Open a file from the Explorer to preview it here.")
                .into_any_element(),
            Content::Loading => div()
                .flex_1()
                .flex()
                .items_center()
                .justify_center()
                .text_color(muted)
                .child("Loading preview…")
                .into_any_element(),
            Content::Image(image) => {
                let viewport = window.viewport_size();
                let fallback_width = f32::from(viewport.width);
                let fallback_height = f32::from(viewport.height);
                let bounds = self.image_scroll.bounds();
                let viewport_width = if f32::from(bounds.size.width) > 0.0 {
                    f32::from(bounds.size.width)
                } else {
                    fallback_width
                };
                let viewport_height = if f32::from(bounds.size.height) > 0.0 {
                    f32::from(bounds.size.height)
                } else {
                    fallback_height
                };
                let mut container = div()
                    .id("preview-image-canvas")
                    .flex_1()
                    .p_2()
                    .track_scroll(&self.image_scroll)
                    .on_scroll_wheel(cx.listener(|this, event: &ScrollWheelEvent, window, cx| {
                        if !(event.modifiers.platform || event.modifiers.control) {
                            return;
                        }
                        let delta = event.delta.pixel_delta(px(18.0)).y;
                        let factor = if delta < px(0.0) { 1.1 } else { 0.9 };
                        let viewport = window.viewport_size();
                        this.zoom_image(
                            factor,
                            f32::from(viewport.width),
                            f32::from(viewport.height),
                            Some(event.position),
                            cx,
                        );
                        cx.stop_propagation();
                    }))
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(|this, event: &MouseDownEvent, window, cx| {
                            window.focus(&this.focus_handle);
                            this.begin_image_pan(event.position, cx);
                        }),
                    )
                    .on_mouse_down(
                        MouseButton::Middle,
                        cx.listener(|this, event: &MouseDownEvent, window, cx| {
                            window.focus(&this.focus_handle);
                            this.begin_image_pan(event.position, cx);
                        }),
                    )
                    .on_mouse_move(cx.listener(|this, event: &MouseMoveEvent, _window, cx| {
                        if matches!(
                            event.pressed_button,
                            Some(MouseButton::Left | MouseButton::Middle)
                        ) {
                            this.move_image_pan(event.position, cx);
                        }
                    }))
                    .on_mouse_up(
                        MouseButton::Left,
                        cx.listener(|this, _event: &MouseUpEvent, _window, cx| {
                            this.end_image_pan(cx);
                        }),
                    )
                    .on_mouse_up(
                        MouseButton::Middle,
                        cx.listener(|this, _event: &MouseUpEvent, _window, cx| {
                            this.end_image_pan(cx);
                        }),
                    )
                    .on_mouse_up_out(
                        MouseButton::Left,
                        cx.listener(|this, _event: &MouseUpEvent, _window, cx| {
                            this.end_image_pan(cx);
                        }),
                    )
                    .on_mouse_up_out(
                        MouseButton::Middle,
                        cx.listener(|this, _event: &MouseUpEvent, _window, cx| {
                            this.end_image_pan(cx);
                        }),
                    );
                let image_body = match self.image_zoom {
                    ImageZoom::Fit => container.overflow_hidden().child(
                        img(image.image.clone())
                            .object_fit(ObjectFit::Contain)
                            .size_full(),
                    ),
                    zoom => {
                        let scale = image_zoom_scale(zoom, 1.0);
                        let image_width = image.width as f32 * scale;
                        let image_height = image.height as f32 * scale;
                        let center_image = image_width <= viewport_width - 16.0
                            && image_height <= viewport_height - 16.0;
                        container = container
                            .overflow_x_scroll()
                            .overflow_y_scroll()
                            .flex()
                            .when(center_image, |element| {
                                element.items_center().justify_center()
                            });
                        container.child(
                            img(image.image.clone())
                                .w(px(image_width))
                                .h(px(image_height))
                                .flex_none(),
                        )
                    }
                };
                image_body.into_any_element()
            }
            Content::Markdown(blocks) => {
                let render = MarkdownRenderContext {
                    foreground: fg,
                    muted,
                    border,
                    accent,
                    primary: c.primary,
                    success: c.success,
                    warning: c.warning,
                    info: c.info,
                    base: &self.url,
                    images: &self.markdown_images,
                    syntax_runs: &self.markdown_syntax_runs,
                };
                div()
                    .id("preview-md")
                    .flex_1()
                    .overflow_y_scroll()
                    .track_scroll(&self.markdown_scroll)
                    .p_4()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .text_color(fg)
                    .text_size(px(13.0))
                    .children(
                        blocks
                            .iter()
                            .enumerate()
                            .map(|(index, block)| render_block(index, block, &render, cx)),
                    )
                    .into_any_element()
            }
            Content::Table(table) => {
                let row_status = if table.total_rows > table.rows.len() {
                    format!(
                        "Showing first {} of {} rows · {} columns",
                        table.rows.len(),
                        table.total_rows,
                        table.column_count
                    )
                } else {
                    format!("{} rows · {} columns", table.total_rows, table.column_count)
                };
                let rows = table.rows.iter().enumerate().map(|(row_index, row)| {
                    let cells = (0..table.column_count).map(|column_index| {
                        let value = row.get(column_index).cloned().unwrap_or_default();
                        div()
                            .w(px(180.0))
                            .flex_shrink_0()
                            .overflow_hidden()
                            .px_2()
                            .py_1()
                            .border_r_1()
                            .border_b_1()
                            .border_color(border)
                            .text_size(px(12.0))
                            .when(row_index == 0, |cell| {
                                cell.bg(accent)
                                    .font_weight(gpui::FontWeight::SEMIBOLD)
                                    .text_color(fg)
                            })
                            .child(SharedString::from(value))
                    });
                    div().flex().flex_none().children(cells)
                });
                div()
                    .id("preview-table")
                    .flex_1()
                    .overflow_x_scroll()
                    .overflow_y_scroll()
                    .track_scroll(&self.markdown_scroll)
                    .flex()
                    .flex_col()
                    .text_color(fg)
                    .child(
                        div()
                            .flex_shrink_0()
                            .px_3()
                            .py_1()
                            .text_size(px(10.0))
                            .text_color(muted)
                            .child(SharedString::from(row_status)),
                    )
                    .children(rows)
                    .into_any_element()
            }
            Content::External { why } => div()
                .flex_1()
                .flex()
                .flex_col()
                .items_center()
                .justify_center()
                .gap_3()
                .p_6()
                .child(div().text_color(muted).child(why.clone()))
                .child(
                    button(
                        "preview-open-external",
                        c,
                        ButtonVariant::Outlined,
                        ButtonSize::Sm,
                    )
                    .on_click(cx.listener(|this, _: &ClickEvent, _, _| this.open_external()))
                    .child("Open in system browser"),
                )
                .into_any_element(),
            Content::Error(_) => div()
                .flex_1()
                .flex()
                .items_center()
                .justify_center()
                .p_6()
                .text_color(muted)
                .child("Preview unavailable")
                .into_any_element(),
            Content::Unsupported(message) => div()
                .flex_1()
                .flex()
                .items_center()
                .justify_center()
                .p_6()
                .text_color(muted)
                .child(message.clone())
                .into_any_element(),
        };

        let header = div()
            .flex()
            .items_center()
            .gap_2()
            .h(px(30.0))
            .px_3()
            .border_b_1()
            .border_color(border)
            .child(
                div()
                    .flex_1()
                    .text_size(px(11.0))
                    .text_color(muted)
                    .overflow_hidden()
                    .child(address),
            )
            .when(matches!(self.content, Content::Image(_)), |element| {
                let zoom = match self.image_zoom {
                    ImageZoom::Fit => "Fit".to_string(),
                    ImageZoom::ActualSize => "100%".to_string(),
                    ImageZoom::Scale(scale) => format!("{:.0}%", scale * 100.0),
                };
                let metadata = match &self.content {
                    Content::Image(image) => image.metadata(image_viewer_unit),
                    _ => String::new(),
                };
                let zoom_control = if let Some(input) = self.image_zoom_input.as_ref() {
                    text_field_surface(
                        "preview-image-zoom-input",
                        c,
                        TextFieldState::Focused,
                        text_input(input, c),
                    )
                    .w(px(58.0))
                    .into_any_element()
                } else {
                    div()
                        .id("preview-image-zoom-level")
                        .min_w(px(38.0))
                        .text_center()
                        .text_size(px(10.0))
                        .text_color(muted)
                        .cursor_pointer()
                        .tooltip(|window, cx| {
                            Tooltip::new("Click to edit; right-click to reset to 100%.")
                                .build(window, cx)
                        })
                        .on_click(cx.listener(|this, _: &ClickEvent, window, cx| {
                            this.edit_image_zoom(window, cx);
                        }))
                        .on_mouse_down(
                            MouseButton::Right,
                            cx.listener(|this, _event: &MouseDownEvent, _, cx| {
                                this.set_image_zoom(ImageZoom::ActualSize, cx);
                                cx.stop_propagation();
                            }),
                        )
                        .child(zoom)
                        .into_any_element()
                };
                element
                    .child(div().text_size(px(10.0)).text_color(muted).child(metadata))
                    .child(
                        labonair_ui_kit::button_no_hover(
                            "preview-image-zoom-out",
                            c,
                            ButtonVariant::Subtle,
                            ButtonSize::Xs,
                        )
                        .text_color(muted)
                        .hover(|style| style.text_color(fg))
                        .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| {
                            let viewport = window.viewport_size();
                            this.zoom_image(
                                0.8,
                                f32::from(viewport.width),
                                f32::from(viewport.height),
                                None,
                                cx,
                            );
                        }))
                        .child("−"),
                    )
                    .child(zoom_control)
                    .child(
                        labonair_ui_kit::button_no_hover(
                            "preview-image-zoom-in",
                            c,
                            ButtonVariant::Subtle,
                            ButtonSize::Xs,
                        )
                        .text_color(muted)
                        .hover(|style| style.text_color(fg))
                        .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| {
                            let viewport = window.viewport_size();
                            this.zoom_image(
                                1.25,
                                f32::from(viewport.width),
                                f32::from(viewport.height),
                                None,
                                cx,
                            );
                        }))
                        .child("+"),
                    )
                    .child(
                        labonair_ui_kit::button_no_hover(
                            "preview-image-actual-size",
                            c,
                            ButtonVariant::Subtle,
                            ButtonSize::Xs,
                        )
                        .text_color(muted)
                        .hover(|style| style.text_color(fg))
                        .on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                            this.set_image_zoom(ImageZoom::ActualSize, cx);
                        }))
                        .child("100%"),
                    )
                    .child(
                        labonair_ui_kit::button_no_hover(
                            "preview-image-fit",
                            c,
                            ButtonVariant::Subtle,
                            ButtonSize::Xs,
                        )
                        .text_color(muted)
                        .hover(|style| style.text_color(fg))
                        .on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                            this.set_image_zoom(ImageZoom::Fit, cx);
                        }))
                        .child("Fit"),
                    )
            })
            .when(!self.source_bound, |d| {
                d.child(
                    labonair_ui_kit::button_no_hover(
                        "preview-reload",
                        c,
                        ButtonVariant::Subtle,
                        ButtonSize::Xs,
                    )
                    .text_color(muted)
                    .hover(|s| s.text_color(fg))
                    .on_click(cx.listener(|this, _: &ClickEvent, _, cx| this.reload(cx)))
                    .child("Reload"),
                )
            })
            .when(!self.url.is_empty(), |d| {
                d.child(
                    labonair_ui_kit::button_no_hover(
                        "preview-external",
                        c,
                        ButtonVariant::Subtle,
                        ButtonSize::Xs,
                    )
                    .text_color(muted)
                    .hover(|s| s.text_color(fg))
                    .on_click(cx.listener(|this, _: &ClickEvent, _, _| this.open_external()))
                    .child("Open externally"),
                )
            });

        div()
            .track_focus(&self.focus_handle)
            .key_context("Preview")
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                if event.keystroke.key == "escape" && this.image_zoom_input.is_some() {
                    this.image_zoom_input = None;
                    window.focus(&this.focus_handle);
                    cx.notify();
                    cx.stop_propagation();
                }
            }))
            .size_full()
            .flex()
            .flex_col()
            .bg(bg)
            .child(header)
            .child(body)
    }
}

struct MarkdownRenderContext<'a> {
    foreground: gpui::Hsla,
    muted: gpui::Hsla,
    border: gpui::Hsla,
    accent: gpui::Hsla,
    primary: gpui::Hsla,
    success: gpui::Hsla,
    warning: gpui::Hsla,
    info: gpui::Hsla,
    base: &'a str,
    images: &'a HashMap<String, PreviewImage>,
    syntax_runs: &'a HashMap<usize, Vec<Vec<StyledRun>>>,
}

fn render_inlines(
    spans: &[Inline],
    color: gpui::Hsla,
    render: &MarkdownRenderContext<'_>,
    cx: &mut Context<PreviewView>,
) -> gpui::AnyElement {
    use gpui::FontWeight;
    div()
        .flex()
        .flex_wrap()
        .items_baseline()
        .children(spans.iter().enumerate().map(|(index, span)| {
            match span {
                Inline::Text(text) => div()
                    .text_color(color)
                    .whitespace_normal()
                    .child(SharedString::from(text.clone()))
                    .into_any_element(),
                Inline::Code(text) => div()
                    .font_family("mono")
                    .text_color(color)
                    .child(SharedString::from(text.clone()))
                    .into_any_element(),
                Inline::Bold(text) => div()
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(color)
                    .child(SharedString::from(text.clone()))
                    .into_any_element(),
                Inline::Italic(text) => div()
                    .italic()
                    .text_color(color)
                    .child(SharedString::from(text.clone()))
                    .into_any_element(),
                Inline::Image { alt, src } => {
                    if let Some(image) = render.images.get(src) {
                        let scale = (640.0 / image.width.max(1) as f32)
                            .min(480.0 / image.height.max(1) as f32)
                            .min(1.0);
                        gpui::img(image.image.clone())
                            .w(px(image.width as f32 * scale))
                            .h(px(image.height as f32 * scale))
                            .max_w_full()
                            .flex_none()
                            .into_any_element()
                    } else {
                        div()
                            .italic()
                            .text_color(color)
                            .child(SharedString::from(alt.clone()))
                            .into_any_element()
                    }
                }
                Inline::Link { text, href } => {
                    let href = href.clone();
                    let base = render.base.to_string();
                    div()
                        .id(SharedString::from(format!("preview-md-link-{index}")))
                        .cursor_pointer()
                        .underline()
                        .text_color(render.accent)
                        .child(SharedString::from(text.clone()))
                        .on_click(cx.listener(move |this, _: &ClickEvent, _window, cx| {
                            this.open_markdown_link(&base, &href, cx);
                        }))
                        .into_any_element()
                }
            }
        }))
        .into_any_element()
}

fn render_block(
    block_index: usize,
    block: &MdBlock,
    render: &MarkdownRenderContext<'_>,
    cx: &mut Context<PreviewView>,
) -> gpui::AnyElement {
    use gpui::FontWeight;
    let fg = render.foreground;
    let muted = render.muted;
    let border = render.border;
    match block {
        MdBlock::Heading { level, spans } => div()
            .font_weight(FontWeight::SEMIBOLD)
            .text_size(px(match level {
                1 => 20.0,
                2 => 17.0,
                3 => 15.0,
                _ => 13.5,
            }))
            .text_color(fg)
            .child(render_inlines(spans, fg, render, cx))
            .into_any_element(),
        MdBlock::Paragraph(spans) => div()
            .whitespace_normal()
            .child(render_inlines(spans, fg, render, cx))
            .into_any_element(),
        MdBlock::Quote(spans) => div()
            .border_l_2()
            .border_color(border)
            .pl_3()
            .text_color(muted)
            .child(render_inlines(spans, muted, render, cx))
            .into_any_element(),
        // T20-001: shared `Divider` primitive.
        MdBlock::Rule => divider(Axis::Horizontal, border).into_any_element(),
        MdBlock::Bullets(items) => div()
            .flex()
            .flex_col()
            .gap_1()
            .children(items.iter().map(|it| {
                div()
                    .flex()
                    .gap_2()
                    .child(div().text_color(muted).child("\u{2022}"))
                    .child(
                        div()
                            .flex_1()
                            .whitespace_normal()
                            .child(render_inlines(it, fg, render, cx)),
                    )
            }))
            .into_any_element(),
        MdBlock::Ordered(items) => div()
            .flex()
            .flex_col()
            .gap_1()
            .children(items.iter().map(|(n, it)| {
                div()
                    .flex()
                    .gap_2()
                    .child(
                        div()
                            .text_color(muted)
                            .child(SharedString::from(format!("{n}."))),
                    )
                    .child(
                        div()
                            .flex_1()
                            .whitespace_normal()
                            .child(render_inlines(it, fg, render, cx)),
                    )
            }))
            .into_any_element(),
        MdBlock::Table { headers, rows } => div()
            .flex()
            .flex_col()
            .rounded_sm()
            .border_1()
            .border_color(border)
            .overflow_hidden()
            .child(table_row(headers, render, true, cx))
            .children(rows.iter().map(|row| table_row(row, render, false, cx)))
            .into_any_element(),
        MdBlock::Code { lang, text, .. } => {
            let code = if let Some(lines) = render.syntax_runs.get(&block_index) {
                div()
                    .flex()
                    .flex_col()
                    .children(lines.iter().map(|line| {
                        div()
                            .flex()
                            .h(px(16.0))
                            .whitespace_nowrap()
                            .children(line.iter().map(|run| {
                                let color = run
                                    .kind
                                    .map(|kind| markdown_syntax_color(kind, render))
                                    .unwrap_or(fg);
                                div()
                                    .text_color(color)
                                    .child(SharedString::from(run.text.clone()))
                            }))
                    }))
                    .into_any_element()
            } else {
                div()
                    .whitespace_normal()
                    .child(SharedString::from(text.clone()))
                    .into_any_element()
            };
            div()
                .flex()
                .flex_col()
                .gap_1()
                .p_3()
                .rounded_sm()
                .border_1()
                .border_color(border)
                .child(
                    div()
                        .text_size(px(10.0))
                        .text_color(muted)
                        .child(lang.as_deref().unwrap_or("text").to_string()),
                )
                .child(div().font_family("mono").text_size(px(12.0)).child(code))
                .into_any_element()
        }
    }
}

fn markdown_syntax_color(kind: HighlightKind, render: &MarkdownRenderContext<'_>) -> gpui::Hsla {
    match kind {
        HighlightKind::Keyword | HighlightKind::Tag => render.primary,
        HighlightKind::Function
        | HighlightKind::Type
        | HighlightKind::Constructor
        | HighlightKind::Namespace => render.info,
        HighlightKind::String | HighlightKind::Escape => render.success,
        HighlightKind::Number | HighlightKind::Boolean | HighlightKind::Constant => render.warning,
        HighlightKind::Comment => render.muted,
        _ => render.foreground,
    }
}

fn table_row(
    cells: &[Vec<Inline>],
    render: &MarkdownRenderContext<'_>,
    header: bool,
    cx: &mut Context<PreviewView>,
) -> gpui::AnyElement {
    use gpui::FontWeight;
    let fg = render.foreground;
    let accent = render.accent;
    div()
        .flex()
        .when(header, |d| d.bg(accent))
        .children(cells.iter().map(|c| {
            div()
                .flex_1()
                .px_2()
                .py_1()
                .when(header, |d| d.font_weight(FontWeight::SEMIBOLD))
                .child(render_inlines(c, fg, render, cx))
        }))
        .into_any_element()
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::AppContext;

    #[test]
    fn previewable_matches_reference_extensions() {
        assert!(is_previewable("/x/report.md"));
        assert!(is_previewable("/x/pic.PNG"));
        assert!(is_previewable("/x/data.csv"));
        assert!(is_previewable("https://example.com/a.html"));
        assert!(!is_previewable("/x/main.rs"));
        assert!(!is_previewable("/x/Makefile"));
    }

    #[test]
    fn remote_urls_resolve_to_external() {
        assert!(matches!(
            resolve("https://example.com"),
            Content::External { .. }
        ));
        assert!(matches!(resolve("/tmp/x.html"), Content::External { .. }));
        assert!(matches!(resolve(""), Content::Empty));
    }

    #[test]
    fn delimited_preview_parses_quotes_escaped_quotes_and_embedded_newlines() {
        let table = parse_delimited(
            "name,note\r\nAda,\"hello, \"\"world\"\"\"\r\nGrace,\"line 1\nline 2\"\r\n",
            ',',
        )
        .expect("valid CSV");
        assert_eq!(table.column_count, 2);
        assert_eq!(table.total_rows, 3);
        assert_eq!(table.rows[0], ["name", "note"]);
        assert_eq!(table.rows[1], ["Ada", "hello, \"world\""]);
        assert_eq!(table.rows[2], ["Grace", "line 1\nline 2"]);

        let tabular = parse_delimited("left\tright\n1\t2", '\t').expect("valid TSV");
        assert_eq!(tabular.rows[1], ["1", "2"]);
    }

    #[test]
    fn delimited_preview_reports_unclosed_quotes_and_caps_rendered_rows() {
        assert!(parse_delimited("name,comment\nAda,\"unfinished", ',').is_err());

        let text = (0..MAX_TABLE_PREVIEW_ROWS + 5)
            .map(|index| format!("{index},value"))
            .collect::<Vec<_>>()
            .join("\n");
        let table = parse_delimited(&text, ',').expect("valid CSV");
        assert_eq!(table.total_rows, MAX_TABLE_PREVIEW_ROWS + 5);
        assert_eq!(table.rows.len(), MAX_TABLE_PREVIEW_ROWS);
    }

    #[test]
    fn csv_resolves_to_a_native_table_preview() {
        let path = std::env::temp_dir().join(format!(
            "labonair-preview-table-{}-{}.csv",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ));
        std::fs::write(&path, "name,score\nAda,10\n").expect("write fixture");

        match resolve(path.to_str().expect("UTF-8 temp path")) {
            Content::Table(table) => {
                assert_eq!(table.total_rows, 2);
                assert_eq!(table.rows[1], ["Ada", "10"]);
            }
            _ => panic!("CSV should resolve to a table preview"),
        }

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn svg_preview_renders_a_bounded_native_image() {
        let svg = br##"<svg xmlns="http://www.w3.org/2000/svg" width="8192" height="4096"><rect width="100%" height="100%" fill="#d33"/></svg>"##;

        let image = render_svg(svg).expect("valid SVG should render");

        assert_eq!(image.format, "SVG");
        assert_eq!(image.width, 2_048);
        assert_eq!(image.height, 1_024);
        assert_eq!(image.byte_len, svg.len());
    }

    #[test]
    fn svg_file_resolves_to_a_native_preview_image() {
        let path = std::env::temp_dir().join(format!(
            "labonair-preview-svg-{}-{}.svg",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ));
        std::fs::write(
            &path,
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="16"><circle cx="8" cy="8" r="6" fill="blue"/></svg>"#,
        )
        .expect("write SVG fixture");

        match resolve(path.to_str().expect("UTF-8 temp path")) {
            Content::Image(image) => {
                assert_eq!(image.format, "SVG");
                assert_eq!((image.width, image.height), (24, 16));
            }
            _ => panic!("SVG should resolve to a native image preview"),
        }

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn svg_preview_rejects_malformed_documents() {
        assert!(render_svg(b"<svg><rect></svg>").is_err());
    }

    #[test]
    fn image_fit_preserves_aspect_ratio_and_zoom_stays_bounded() {
        assert_eq!(fit_image_scale(4000, 2000, 1000.0, 500.0), 0.25);
        assert_eq!(fit_image_scale(100, 100, 1000.0, 800.0), 1.0);
        assert_eq!(image_zoom_scale(ImageZoom::ActualSize, 0.25), 1.0);
        assert_eq!(image_zoom_scale(ImageZoom::Fit, 0.25), 0.25);
        assert_eq!(
            image_zoom_preset(PreviewImageAction::ResetZoom),
            Some(ImageZoom::ActualSize)
        );
        assert_eq!(
            image_zoom_preset(PreviewImageAction::ZoomToActualSize),
            Some(ImageZoom::ActualSize)
        );
        assert_eq!(
            image_zoom_preset(PreviewImageAction::ZoomToFit),
            Some(ImageZoom::Fit)
        );
        assert_eq!(
            parse_image_zoom_percent("75%"),
            Some(ImageZoom::Scale(0.75))
        );
        assert_eq!(parse_image_zoom_percent("100"), Some(ImageZoom::ActualSize));
        assert_eq!(
            parse_image_zoom_percent("2000%"),
            Some(ImageZoom::Scale(MAX_IMAGE_ZOOM))
        );
        assert_eq!(parse_image_zoom_percent("not a zoom"), None);
        assert_eq!(parse_image_zoom_percent("NaN"), None);
        assert_eq!(
            image_zoom_scale(ImageZoom::Scale(20.0), 0.25),
            MAX_IMAGE_ZOOM
        );
        assert_eq!(
            image_zoom_scale(ImageZoom::Scale(0.0), 0.25),
            MIN_IMAGE_ZOOM
        );
    }

    #[test]
    fn image_zoom_keeps_the_point_under_the_pointer_stationary() {
        assert_eq!(image_scroll_offset_after_zoom(100.0, 0.0, 2.0), -100.0);
        assert_eq!(image_scroll_offset_after_zoom(80.0, -20.0, 1.5), -70.0);
    }

    #[test]
    fn image_pan_tracks_pointer_delta_from_the_drag_origin() {
        assert_eq!(image_scroll_offset_after_pan(100.0, -20.0, 130.0), 10.0);
        assert_eq!(image_scroll_offset_after_pan(40.0, -60.0, 20.0), -80.0);
    }

    #[test]
    fn image_preview_records_decoded_metadata() {
        let mut encoded = std::io::Cursor::new(Vec::new());
        image::DynamicImage::new_rgb8(2, 3)
            .write_to(&mut encoded, image::ImageFormat::Png)
            .unwrap();
        let path = std::env::temp_dir().join(format!(
            "labonair-preview-image-{}-{}.png",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ));
        std::fs::write(&path, encoded.into_inner()).unwrap();

        let preview = load_image(path.to_str().unwrap()).unwrap();
        assert_eq!((preview.width, preview.height), (2, 3));
        assert_eq!(preview.format, "PNG");
        assert_eq!(preview.bits_per_pixel, 24);
        assert!(preview
            .metadata(labonair_settings::content::editor::ImageViewerUnit::Binary)
            .contains("2 × 3 · PNG · 24 bpp"));
        assert!(preview
            .metadata(labonair_settings::content::editor::ImageViewerUnit::Binary)
            .ends_with(&format_image_size(
                preview.byte_len,
                labonair_settings::content::editor::ImageViewerUnit::Binary
            )));

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn image_size_uses_binary_units() {
        use labonair_settings::content::editor::ImageViewerUnit;

        assert_eq!(format_image_size(0, ImageViewerUnit::Binary), "0 B");
        assert_eq!(format_image_size(1023, ImageViewerUnit::Binary), "1023 B");
        assert_eq!(format_image_size(1024, ImageViewerUnit::Binary), "1.0 KiB");
        assert_eq!(
            format_image_size(1024 * 1024, ImageViewerUnit::Binary),
            "1.0 MiB"
        );
    }

    #[test]
    fn image_size_uses_decimal_units_when_configured() {
        use labonair_settings::content::editor::ImageViewerUnit;

        assert_eq!(format_image_size(999, ImageViewerUnit::Decimal), "999 B");
        assert_eq!(format_image_size(1000, ImageViewerUnit::Decimal), "1.0 kB");
        assert_eq!(
            format_image_size(1_000_000, ImageViewerUnit::Decimal),
            "1.0 MB"
        );
    }

    #[test]
    fn markdown_links_resolve_relative_files_and_allow_only_known_external_schemes() {
        assert_eq!(
            resolve_markdown_link("/docs/readme.md", "guide/next.md#intro"),
            Some(MarkdownLinkTarget::Local {
                path: PathBuf::from("/docs/guide/next.md"),
                fragment: Some("intro".into()),
            })
        );
        assert_eq!(
            resolve_markdown_link("/docs/readme.md", "https://example.com/help"),
            Some(MarkdownLinkTarget::External(
                "https://example.com/help".into()
            ))
        );
        assert_eq!(
            resolve_markdown_link("/docs/readme.md", "mailto:docs@example.com"),
            Some(MarkdownLinkTarget::External(
                "mailto:docs@example.com".into()
            ))
        );
        assert_eq!(
            resolve_markdown_link("/docs/readme.md", "javascript:alert(1)"),
            None
        );
        assert_eq!(
            resolve_markdown_link("/docs/readme.md", "#section"),
            Some(MarkdownLinkTarget::Anchor("section".into()))
        );
    }

    #[test]
    fn markdown_images_resolve_only_to_local_files() {
        assert_eq!(
            resolve_markdown_image_path("/docs/readme.md", "images/diagram.png"),
            Some(PathBuf::from("/docs/images/diagram.png"))
        );
        assert_eq!(
            resolve_markdown_image_path("/docs/readme.md", "https://example.com/image.png"),
            None
        );
        assert_eq!(
            resolve_markdown_image_path("/docs/readme.md", "#section"),
            None
        );
    }

    #[test]
    fn markdown_fence_languages_map_to_editor_languages() {
        assert_eq!(markdown_fence_language("rust"), Some(Language::Rust));
        assert_eq!(markdown_fence_language("{.rs}").unwrap(), Language::Rust);
        assert_eq!(
            markdown_fence_language("typescript title=sample.ts"),
            Some(Language::TypeScript)
        );
        assert_eq!(markdown_fence_language("unknown"), None);
    }

    #[test]
    fn heading_fragments_normalize_text_and_distinguish_duplicates() {
        let blocks = parse_markdown("# Hello, World!\n\n## Hello World\n\n### Other");
        assert_eq!(heading_index_for_fragment(&blocks, "hello-world"), Some(0));
        assert_eq!(
            heading_index_for_fragment(&blocks, "hello-world-1"),
            Some(1)
        );
        assert_eq!(heading_index_for_fragment(&blocks, "other"), Some(2));
        assert_eq!(heading_index_for_fragment(&blocks, "missing"), None);
    }

    #[test]
    fn markdown_file_resolves_to_blocks() {
        let dir = std::env::temp_dir().join(format!("labonair-preview-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let f = dir.join("a.md");
        std::fs::write(&f, "# Title\n\nbody text\n").unwrap();
        match resolve(f.to_str().unwrap()) {
            Content::Markdown(blocks) => assert!(!blocks.is_empty()),
            _ => panic!("expected markdown"),
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[gpui::test]
    fn local_preview_shows_loading_then_resolves_file(cx: &mut gpui::TestAppContext) {
        let dir = std::env::temp_dir().join(format!(
            "labonair-preview-async-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("async.md");
        std::fs::write(&path, "# Loaded in background").unwrap();

        let view = cx.update(|cx| {
            let theme = cx.new(|_| ThemeStore::new(gpui::WindowAppearance::Light));
            cx.new(|cx| PreviewView::new(theme, cx))
        });
        cx.update(|cx| {
            view.update(cx, |preview, cx| {
                preview.set_url(path.to_string_lossy().into_owned(), cx);
                assert!(matches!(preview.content, Content::Loading));
            });
        });
        cx.background_executor.run_until_parked();
        cx.update(|cx| {
            let preview = view.read(cx);
            assert!(matches!(
                &preview.content,
                Content::Markdown(blocks)
                    if matches!(blocks.first(), Some(MdBlock::Heading { spans, .. })
                        if spans.iter().any(|span| span.plain() == "Loaded in background"))
            ));
        });

        let stale_path = dir.join("stale.md");
        std::fs::write(&stale_path, "# Stale disk content").unwrap();
        cx.update(|cx| {
            view.update(cx, |preview, cx| {
                preview.set_url(stale_path.to_string_lossy().into_owned(), cx);
                preview.set_editor_source(
                    Some(path.clone()),
                    Language::Markdown,
                    labonair_editor::EditorBuffer::from_text("# Current editor source").snapshot(),
                    cx,
                );
            });
        });
        cx.background_executor.run_until_parked();
        cx.update(|cx| {
            let preview = view.read(cx);
            assert!(matches!(
                &preview.content,
                Content::Markdown(blocks)
                    if matches!(blocks.first(), Some(MdBlock::Heading { spans, .. })
                        if spans.iter().any(|span| span.plain() == "Current editor source"))
            ));
            assert!(preview.source_bound);
        });

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[gpui::test]
    fn svg_preview_renders_the_current_editor_revision_in_background(
        cx: &mut gpui::TestAppContext,
    ) {
        let view = cx.update(|cx| {
            let theme = cx.new(|_| ThemeStore::new(gpui::WindowAppearance::Light));
            cx.new(|cx| PreviewView::new(theme, cx))
        });
        let source = labonair_editor::EditorBuffer::from_text(
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="16"><rect width="24" height="16" fill="red"/></svg>"#,
        )
        .snapshot();
        cx.update(|cx| {
            view.update(cx, |preview, cx| {
                preview.set_editor_svg_source(Some(PathBuf::from("/assets/icon.svg")), source, cx);
                assert!(matches!(preview.content, Content::Loading));
            });
        });

        cx.background_executor.run_until_parked();
        cx.update(|cx| {
            let preview = view.read(cx);
            assert_eq!(preview.url(), "/assets/icon.svg");
            assert!(preview.source_bound);
            assert!(matches!(
                &preview.content,
                Content::Image(image) if image.format == "SVG" && image.width == 24
            ));
        });

        let updated_source = labonair_editor::EditorBuffer::from_text(
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="30" height="16"><rect width="30" height="16" fill="blue"/></svg>"#,
        )
        .snapshot();
        cx.update(|cx| {
            view.update(cx, |preview, cx| {
                preview.image_zoom = ImageZoom::Scale(1.5);
                preview.set_editor_svg_source(
                    Some(PathBuf::from("/assets/icon.svg")),
                    updated_source,
                    cx,
                );
                assert_eq!(preview.image_zoom, ImageZoom::Scale(1.5));
            });
        });
        cx.background_executor.run_until_parked();
        cx.update(|cx| {
            assert!(matches!(
                &view.read(cx).content,
                Content::Image(image) if image.format == "SVG" && image.width == 30
            ));
        });

        let not_svg = labonair_editor::EditorBuffer::from_text("<root />").snapshot();
        cx.update(|cx| {
            view.update(cx, |preview, cx| {
                preview.set_editor_svg_source(Some(PathBuf::from("/docs/readme.xml")), not_svg, cx);
                assert!(matches!(
                    &preview.content,
                    Content::Unsupported(message) if message.contains("not SVG")
                ));
            });
        });
    }

    #[gpui::test]
    fn table_preview_parses_the_current_editor_revision_in_background(
        cx: &mut gpui::TestAppContext,
    ) {
        let view = cx.update(|cx| {
            let theme = cx.new(|_| ThemeStore::new(gpui::WindowAppearance::Light));
            cx.new(|cx| PreviewView::new(theme, cx))
        });
        let source =
            labonair_editor::EditorBuffer::from_text("name,note\nAda,\"hello, world\"").snapshot();
        cx.update(|cx| {
            view.update(cx, |preview, cx| {
                preview.set_editor_tabular_source(
                    Some(PathBuf::from("/data/people.csv")),
                    source,
                    cx,
                );
                assert!(matches!(preview.content, Content::Loading));
            });
        });

        cx.background_executor.run_until_parked();
        cx.update(|cx| {
            let preview = view.read(cx);
            assert!(preview.source_bound);
            assert!(matches!(
                &preview.content,
                Content::Table(table)
                    if table.total_rows == 2 && table.rows[1] == ["Ada", "hello, world"]
            ));
        });
    }

    #[gpui::test]
    fn local_markdown_images_load_in_the_background(cx: &mut gpui::TestAppContext) {
        let dir = std::env::temp_dir().join(format!(
            "labonair-preview-markdown-image-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ));
        std::fs::create_dir_all(dir.join("images")).unwrap();
        let image_path = dir.join("images/diagram.png");
        let markdown_path = dir.join("readme.md");
        let mut encoded = std::io::Cursor::new(Vec::new());
        image::DynamicImage::new_rgb8(2, 3)
            .write_to(&mut encoded, image::ImageFormat::Png)
            .unwrap();
        std::fs::write(&image_path, encoded.into_inner()).unwrap();
        std::fs::write(&markdown_path, "![Diagram](images/diagram.png)").unwrap();

        let view = cx.update(|cx| {
            let theme = cx.new(|_| ThemeStore::new(gpui::WindowAppearance::Light));
            cx.new(|cx| PreviewView::new(theme, cx))
        });
        cx.update(|cx| {
            view.update(cx, |preview, cx| {
                preview.set_url(markdown_path.to_string_lossy().into_owned(), cx);
            });
        });
        cx.background_executor.run_until_parked();
        cx.update(|cx| {
            let preview = view.read(cx);
            let image = preview
                .markdown_images
                .get("images/diagram.png")
                .expect("local Markdown image should be decoded");
            assert_eq!((image.width, image.height), (2, 3));
        });

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[gpui::test]
    fn markdown_code_blocks_use_editor_syntax_highlighting(cx: &mut gpui::TestAppContext) {
        let view = cx.update(|cx| {
            let theme = cx.new(|_| ThemeStore::new(gpui::WindowAppearance::Light));
            cx.new(|cx| PreviewView::new(theme, cx))
        });
        let source = labonair_editor::EditorBuffer::from_text(
            "```rust\nfn main() { let answer = 42; }\n```",
        )
        .snapshot();
        cx.update(|cx| {
            view.update(cx, |preview, cx| {
                preview.set_editor_source(
                    Some(PathBuf::from("/docs/readme.md")),
                    Language::Markdown,
                    source,
                    cx,
                );
            });
        });

        cx.background_executor.run_until_parked();
        cx.update(|cx| {
            let preview = view.read(cx);
            let lines = preview
                .markdown_syntax_runs
                .get(&0)
                .expect("the Rust fence should have highlighted runs");
            assert!(lines
                .iter()
                .flatten()
                .any(|run| { run.kind == Some(HighlightKind::Keyword) && run.text == "fn" }));
        });
    }

    #[gpui::test]
    fn markdown_preview_tracks_editor_buffer_revisions(cx: &mut gpui::TestAppContext) {
        let view = cx.update(|cx| {
            let theme = cx.new(|_| ThemeStore::new(gpui::WindowAppearance::Light));
            cx.new(|cx| PreviewView::new(theme, cx))
        });
        let first = labonair_editor::EditorBuffer::from_text("# Initial").snapshot();
        cx.update(|cx| {
            view.update(cx, |preview, cx| {
                preview.set_editor_source(
                    Some(PathBuf::from("/docs/readme.md")),
                    Language::Markdown,
                    first,
                    cx,
                );
            });
        });
        cx.update(|cx| {
            let preview = view.read(cx);
            assert_eq!(preview.url(), "/docs/readme.md");
            assert!(preview.source_bound);
            assert!(matches!(
                preview.content,
                Content::Markdown(ref blocks)
                    if matches!(blocks.first(), Some(MdBlock::Heading { spans, .. })
                        if spans.iter().any(|span| span.plain() == "Initial"))
            ));
        });

        let second = labonair_editor::EditorBuffer::from_text("# Updated").snapshot();
        cx.update(|cx| {
            view.update(cx, |preview, cx| {
                preview.set_editor_source(
                    Some(PathBuf::from("/docs/readme.md")),
                    Language::Markdown,
                    second,
                    cx,
                );
            });
        });
        cx.update(|cx| {
            let preview = view.read(cx);
            assert!(matches!(
                preview.content,
                Content::Markdown(ref blocks)
                    if matches!(blocks.first(), Some(MdBlock::Heading { spans, .. })
                        if spans.iter().any(|span| span.plain() == "Updated"))
            ));
        });

        let code = labonair_editor::EditorBuffer::from_text("fn main() {}").snapshot();
        cx.update(|cx| {
            view.update(cx, |preview, cx| {
                preview.set_editor_source(
                    Some(PathBuf::from("/docs/main.rs")),
                    Language::Rust,
                    code,
                    cx,
                );
            });
        });
        cx.update(|cx| {
            let preview = view.read(cx);
            assert_eq!(preview.url(), "/docs/main.rs");
            assert!(matches!(preview.content, Content::Unsupported(_)));
        });
    }
}
