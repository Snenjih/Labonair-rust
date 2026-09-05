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

use std::path::Path;
use std::sync::Arc;

use gpui::prelude::*;
use gpui::{
    div, img, px, App, ClickEvent, Context, Entity, FocusHandle, Focusable, Image, ImageFormat,
    ObjectFit, SharedString, Window,
};

use labonair_ui_kit::{divider, Axis};

use crate::markdown::{parse_markdown, Inline, MdBlock};
use crate::theme::ThemeStore;

/// Extensions the reference explorer offers "Open in Preview" for, plus the
/// text/markdown kinds this native pane can additionally render.
pub const PREVIEW_EXTENSIONS: &[&str] = &[
    "html", "htm", "png", "jpg", "jpeg", "gif", "webp", "svg", "pdf", "bmp", "ico", "md",
    "markdown", "txt", "text",
];

/// Whether `path` is something the preview tab knows how to open.
pub fn is_previewable(path: &str) -> bool {
    ext_of(path).is_some_and(|e| PREVIEW_EXTENSIONS.contains(&e.as_str()))
}

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

/// Resolved preview payload for the current address.
enum Content {
    Empty,
    Image(Arc<Image>),
    Markdown(Vec<MdBlock>),
    /// Needs a browser engine — offer "open externally" only. `why` explains it.
    External {
        why: SharedString,
    },
    Error(SharedString),
}

/// A single preview tab. Feed it an address (local path or URL) via
/// [`Self::set_url`].
pub struct PreviewView {
    theme: Entity<ThemeStore>,
    focus_handle: FocusHandle,
    url: String,
    content: Content,
}

impl PreviewView {
    pub fn new(theme: Entity<ThemeStore>, cx: &mut Context<Self>) -> Self {
        cx.observe(&theme, |_, _, cx| cx.notify()).detach();
        Self {
            theme,
            focus_handle: cx.focus_handle(),
            url: String::new(),
            content: Content::Empty,
        }
    }

    pub fn url(&self) -> &str {
        &self.url
    }

    /// Point the pane at a new address and re-resolve its content.
    pub fn set_url(&mut self, url: impl Into<String>, cx: &mut Context<Self>) {
        self.url = url.into();
        self.content = resolve(&self.url);
        cx.notify();
    }

    /// Re-read the current address from disk.
    pub fn reload(&mut self, cx: &mut Context<Self>) {
        self.content = resolve(&self.url);
        cx.notify();
    }

    fn open_external(&self) {
        if !self.url.is_empty() {
            open_external(&self.url);
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
        let card = theme.card();

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
                .p_4()
                .flex()
                .flex_col()
                .gap_2()
                .text_color(fg)
                .text_size(px(13.0))
                .children(
                    blocks
                        .iter()
                        .map(|b| render_block(b, fg, muted, border, accent)),
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
                    div()
                        .id("preview-open-external")
                        .px_3()
                        .py_1p5()
                        .rounded_md()
                        .border_1()
                        .border_color(border)
                        .bg(card)
                        .text_color(fg)
                        .hover(|s| s.bg(accent))
                        .on_click(cx.listener(|this, _: &ClickEvent, _, _| this.open_external()))
                        .child("Open in system browser"),
                )
                .into_any_element(),
            Content::Error(msg) => div()
                .flex_1()
                .flex()
                .items_center()
                .justify_center()
                .p_6()
                .text_color(theme.status_error())
                .child(msg.clone())
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
                    .child(
                        div()
                            .id("preview-reload")
                            .px_2()
                            .text_size(px(11.0))
                            .text_color(muted)
                            .hover(|s| s.text_color(fg))
                            .on_click(cx.listener(|this, _: &ClickEvent, _, cx| this.reload(cx)))
                            .child("Reload"),
                    )
                    .when(!self.url.is_empty(), |d| {
                        d.child(
                            div()
                                .id("preview-external")
                                .px_2()
                                .text_size(px(11.0))
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

fn inline_string(spans: &[Inline]) -> String {
    spans.iter().map(Inline::plain).collect::<Vec<_>>().join("")
}

fn render_block(
    block: &MdBlock,
    fg: gpui::Hsla,
    muted: gpui::Hsla,
    border: gpui::Hsla,
    accent: gpui::Hsla,
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
            .child(SharedString::from(inline_string(spans)))
            .into_any_element(),
        MdBlock::Paragraph(spans) => div()
            .whitespace_normal()
            .child(SharedString::from(inline_string(spans)))
            .into_any_element(),
        MdBlock::Quote(spans) => div()
            .border_l_2()
            .border_color(border)
            .pl_3()
            .text_color(muted)
            .child(SharedString::from(inline_string(spans)))
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
                            .child(SharedString::from(inline_string(it))),
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
                            .child(SharedString::from(inline_string(it))),
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
            .child(table_row(headers, fg, accent, true))
            .children(rows.iter().map(|r| table_row(r, fg, accent, false)))
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
                .text_color(fg)
                .when(header, |d| d.font_weight(FontWeight::SEMIBOLD))
                .child(SharedString::from(inline_string(c)))
        }))
        .into_any_element()
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
