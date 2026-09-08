//! The bundled font-family names + platform fallbacks. The embedded font bytes
//! and system-font discovery stay in `labonair-theme::fonts`; only these
//! string constants are needed by the token defaults, so they live here.

/// Bundled UI-chrome font family (IBM Plex Sans).
pub const UI_FONT_FAMILY: &str = "IBM Plex Sans";

/// Bundled editor/terminal monospace font family (Lilex).
pub const MONO_FONT_FAMILY: &str = "Lilex";

/// Platform fallbacks appended after [`UI_FONT_FAMILY`].
pub const UI_FONT_FALLBACKS: &[&str] = &[".SystemUIFont", "sans-serif"];

/// Platform fallbacks appended after [`MONO_FONT_FAMILY`].
pub const MONO_FONT_FALLBACKS: &[&str] = &["SFMono-Regular", "Menlo", "monospace"];
