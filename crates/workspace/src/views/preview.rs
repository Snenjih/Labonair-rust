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
//! * **HTML, PDF, SVG and remote URLs** cannot be rendered in-process; the pane
//!   shows the address plus an **Open in system browser** button
//!   (`/usr/bin/open` on macOS, `xdg-open` on Linux).
//!
//! This is the single documented deviation from 1:1 parity (see `ROADMAP.md`).

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use gpui::prelude::*;
use gpui::{
    div, img, px, App, ClickEvent, Context, Entity, FocusHandle, Focusable, Image, ImageFormat,
    InteractiveElement, ObjectFit, ScrollHandle, SharedString, StatefulInteractiveElement, Window,
};

use labonair_editor::{BufferSnapshot, Language};
use labonair_notifications::{notification_center, Notification};
use labonair_ui_kit::{button, divider, Axis, ButtonSize, ButtonVariant, Palette};

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
    Image(Arc<Image>),
    Markdown(Vec<MdBlock>),
    /// Needs a browser engine — offer "open externally" only. `why` explains it.
    External {
        why: SharedString,
    },
    Unsupported(SharedString),
    Error(SharedString),
}

/// A single preview tab. Feed it an address (local path or URL) via
/// [`Self::set_url`].
pub struct PreviewView {
    theme: Entity<ThemeStore>,
    focus_handle: FocusHandle,
    url: String,
    content: Content,
    source_bound: bool,
    markdown_scroll: ScrollHandle,
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
            markdown_scroll: ScrollHandle::new(),
        }
    }

    pub fn url(&self) -> &str {
        &self.url
    }

    /// Point the pane at a new address and re-resolve its content.
    pub fn set_url(&mut self, url: impl Into<String>, cx: &mut Context<Self>) {
        self.url = url.into();
        self.source_bound = false;
        self.resolve_content(cx);
        cx.notify();
    }

    /// Bind this preview tab to an immutable editor revision. Markdown source
    /// is parsed immediately; switching the source editor to another language
    /// leaves an explicit unavailable state instead of stale rendered content.
    pub fn set_editor_source(
        &mut self,
        path: Option<PathBuf>,
        language: Language,
        snapshot: BufferSnapshot,
        cx: &mut Context<Self>,
    ) {
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
        self.content = resolve(&self.url);
        if let Content::Error(message) = &self.content {
            let url = self.url.clone();
            notification_center(cx).update(cx, |center, cx| {
                center.push(
                    Notification::error("Preview unavailable", "Could not load the preview.")
                        .source("preview")
                        .details(message.clone())
                        .dedupe_key(format!("preview:load:{url}")),
                    cx,
                );
            });
        }
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
                self.set_url(path.to_string_lossy().into_owned(), cx);
                if let Some(fragment) = fragment {
                    self.scroll_to_heading(&fragment, cx);
                }
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
            Ok(image) => Content::Image(Arc::new(image)),
            Err(e) => Content::Error(format!("Could not load image: {e}").into()),
        },
        "md" | "markdown" | "txt" | "text" => match std::fs::read_to_string(url) {
            Ok(text) => Content::Markdown(parse_markdown(&text)),
            Err(e) => Content::Error(format!("Could not read file: {e}").into()),
        },
        "html" | "htm" => Content::External {
            why: "HTML pages need a browser engine — opening in your default browser.".into(),
        },
        "pdf" => Content::External {
            why: "PDF files open in your system PDF viewer.".into(),
        },
        "svg" => Content::External {
            why: "SVG documents open in your default browser.".into(),
        },
        _ => Content::External {
            why: "This file type opens in your default application.".into(),
        },
    }
}

fn load_image(path: &str) -> Result<Image, String> {
    let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
    // Validate + normalise via the `image` crate, re-encoding to PNG so GPUI's
    // decoder always gets a format it supports (mirrors `background.rs`).
    let decoded = image::load_from_memory(&bytes).map_err(|e| e.to_string())?;
    let mut out = std::io::Cursor::new(Vec::new());
    decoded
        .write_to(&mut out, image::ImageFormat::Png)
        .map_err(|e| e.to_string())?;
    Ok(Image::from_bytes(ImageFormat::Png, out.into_inner()))
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
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = self.theme.read(cx);
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
            Content::Image(image) => div()
                .flex_1()
                .overflow_hidden()
                .p_2()
                .child(
                    img(image.clone())
                        .object_fit(ObjectFit::Contain)
                        .size_full(),
                )
                .into_any_element(),
            Content::Markdown(blocks) => div()
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
                        .map(|b| render_block(b, fg, muted, border, accent, &self.url, cx)),
                )
                .into_any_element(),
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

        div()
            .track_focus(&self.focus_handle)
            .key_context("Preview")
            .size_full()
            .flex()
            .flex_col()
            .bg(bg)
            .child(
                div()
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
                            .on_click(
                                cx.listener(|this, _: &ClickEvent, _, _| this.open_external()),
                            )
                            .child("Open externally"),
                        )
                    }),
            )
            .child(body)
    }
}

fn render_inlines(
    spans: &[Inline],
    fg: gpui::Hsla,
    accent: gpui::Hsla,
    base: &str,
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
                    .text_color(fg)
                    .whitespace_normal()
                    .child(SharedString::from(text.clone()))
                    .into_any_element(),
                Inline::Code(text) => div()
                    .font_family("mono")
                    .text_color(fg)
                    .child(SharedString::from(text.clone()))
                    .into_any_element(),
                Inline::Bold(text) => div()
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(fg)
                    .child(SharedString::from(text.clone()))
                    .into_any_element(),
                Inline::Italic(text) => div()
                    .italic()
                    .text_color(fg)
                    .child(SharedString::from(text.clone()))
                    .into_any_element(),
                Inline::Link { text, href } => {
                    let href = href.clone();
                    let base = base.to_string();
                    div()
                        .id(SharedString::from(format!("preview-md-link-{index}")))
                        .cursor_pointer()
                        .underline()
                        .text_color(accent)
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
    block: &MdBlock,
    fg: gpui::Hsla,
    muted: gpui::Hsla,
    border: gpui::Hsla,
    accent: gpui::Hsla,
    base: &str,
    cx: &mut Context<PreviewView>,
) -> gpui::AnyElement {
    use gpui::FontWeight;
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
            .child(render_inlines(spans, fg, accent, base, cx))
            .into_any_element(),
        MdBlock::Paragraph(spans) => div()
            .whitespace_normal()
            .child(render_inlines(spans, fg, accent, base, cx))
            .into_any_element(),
        MdBlock::Quote(spans) => div()
            .border_l_2()
            .border_color(border)
            .pl_3()
            .text_color(muted)
            .child(render_inlines(spans, muted, accent, base, cx))
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
                            .child(render_inlines(it, fg, accent, base, cx)),
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
                            .child(render_inlines(it, fg, accent, base, cx)),
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
            .child(table_row(headers, fg, accent, true, base, cx))
            .children(
                rows.iter()
                    .map(|r| table_row(r, fg, accent, false, base, cx)),
            )
            .into_any_element(),
        MdBlock::Code { text, .. } => div()
            .font_family("mono")
            .text_size(px(12.0))
            .p_3()
            .rounded_sm()
            .border_1()
            .border_color(border)
            .whitespace_normal()
            .child(SharedString::from(text.clone()))
            .into_any_element(),
    }
}

fn table_row(
    cells: &[Vec<Inline>],
    fg: gpui::Hsla,
    accent: gpui::Hsla,
    header: bool,
    base: &str,
    cx: &mut Context<PreviewView>,
) -> gpui::AnyElement {
    use gpui::FontWeight;
    div()
        .flex()
        .when(header, |d| d.bg(accent))
        .children(cells.iter().map(|c| {
            div()
                .flex_1()
                .px_2()
                .py_1()
                .when(header, |d| d.font_weight(FontWeight::SEMIBOLD))
                .child(render_inlines(c, fg, accent, base, cx))
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
