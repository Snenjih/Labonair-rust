//! Bundled font assets.
//!
//! The reference web app pulls its fonts over CSS
//! (`reference-src/src/styles/globals.css` → `@fontsource-variable/inter`, and
//! the `preferencesStore` defaults `appFontFamily: '"Inter Variable", sans-serif'`
//! / `terminalFontFamily: '"JetBrains Mono", SFMono-Regular, Menlo, monospace'`).
//! There is no webview here, so the same families are embedded directly in the
//! binary via [`include_bytes!`] and handed to GPUI's text system with
//! `cx.text_system().add_fonts(embedded_fonts())` at startup.
//!
//! All bundled files are SIL OFL 1.1 licensed — see `assets/fonts/LICENSE`.

use std::borrow::Cow;

/// UI / sans-serif family name (matches the `name` table of `InterVariable.ttf`).
pub const UI_FONT_FAMILY: &str = "Inter Variable";

/// Monospace family name (matches the `name` table of the JetBrains Mono files),
/// used for both the terminal and the code editor.
pub const MONO_FONT_FAMILY: &str = "JetBrains Mono";

/// Runtime fallbacks for the UI font when [`UI_FONT_FAMILY`] is unavailable.
pub const UI_FONT_FALLBACKS: &[&str] = &[".SystemUIFont", "sans-serif"];

/// Runtime fallbacks for the monospace font, mirroring the reference app's
/// `"JetBrains Mono", SFMono-Regular, Menlo, monospace` chain.
pub const MONO_FONT_FALLBACKS: &[&str] = &["SFMono-Regular", "Menlo", "monospace"];

macro_rules! font {
    ($path:literal) => {
        include_bytes!(concat!("../assets/fonts/", $path)).as_slice()
    };
}

/// Every bundled font file, ready to pass to `TextSystem::add_fonts`.
pub fn embedded_fonts() -> Vec<Cow<'static, [u8]>> {
    [
        font!("InterVariable.ttf"),
        font!("InterVariable-Italic.ttf"),
        font!("JetBrainsMono-Regular.ttf"),
        font!("JetBrainsMono-Medium.ttf"),
        font!("JetBrainsMono-Bold.ttf"),
        font!("JetBrainsMono-Italic.ttf"),
        font!("JetBrainsMono-BoldItalic.ttf"),
    ]
    .into_iter()
    .map(Cow::Borrowed)
    .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// sfnt magic bytes for the bundled TrueType files.
    fn is_truetype(bytes: &[u8]) -> bool {
        matches!(
            bytes.get(0..4),
            Some([0x00, 0x01, 0x00, 0x00]) | Some(b"true")
        )
    }

    #[test]
    fn all_embedded_fonts_are_valid_truetype() {
        let fonts = embedded_fonts();
        assert_eq!(fonts.len(), 7);
        for font in &fonts {
            assert!(font.len() > 10_000, "font asset suspiciously small");
            assert!(is_truetype(font), "font asset is not a TrueType sfnt");
        }
    }

    #[test]
    fn family_names_are_stable() {
        assert_eq!(UI_FONT_FAMILY, "Inter Variable");
        assert_eq!(MONO_FONT_FAMILY, "JetBrains Mono");
        assert!(MONO_FONT_FALLBACKS.contains(&"Menlo"));
    }
}
