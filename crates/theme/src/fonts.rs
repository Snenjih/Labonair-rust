//! Bundled font assets.
//!
//! The defaults mirror Zed's own (`.ZedSans` → IBM Plex Sans for the UI,
//! `.ZedMono` → Lilex for the editor/terminal); the reference web app's
//! Inter Variable / JetBrains Mono families stay bundled so existing user
//! settings that name them keep resolving. There is no webview here, so every
//! family is embedded directly in the binary via [`include_bytes!`] and handed
//! to GPUI's text system with `cx.text_system().add_fonts(embedded_fonts())`
//! at startup.
//!
//! All bundled files are SIL OFL 1.1 licensed — see `assets/fonts/`.

use std::borrow::Cow;

/// UI / sans-serif family name (matches the `name` table of the IBM Plex Sans
/// files); mirrors Zed's `.ZedSans` alias.
pub const UI_FONT_FAMILY: &str = "IBM Plex Sans";

/// Monospace family name (matches the `name` table of the Lilex files), used for
/// both the terminal and the code editor; mirrors Zed's `.ZedMono` alias.
pub const MONO_FONT_FAMILY: &str = "Lilex";

/// Runtime fallbacks for the UI font when [`UI_FONT_FAMILY`] is unavailable.
pub const UI_FONT_FALLBACKS: &[&str] = &[".SystemUIFont", "sans-serif"];

/// Runtime fallbacks for the monospace font.
pub const MONO_FONT_FALLBACKS: &[&str] = &["SFMono-Regular", "Menlo", "monospace"];

macro_rules! font {
    ($path:literal) => {
        include_bytes!(concat!("../assets/fonts/", $path)).as_slice()
    };
}

/// Every bundled font file, ready to pass to `TextSystem::add_fonts`.
pub fn embedded_fonts() -> Vec<Cow<'static, [u8]>> {
    [
        // Defaults (mirror Zed): IBM Plex Sans for the UI, Lilex for mono.
        font!("IBMPlexSans-Regular.ttf"),
        font!("IBMPlexSans-Italic.ttf"),
        font!("IBMPlexSans-SemiBold.ttf"),
        font!("IBMPlexSans-SemiBoldItalic.ttf"),
        font!("Lilex-Regular.ttf"),
        font!("Lilex-Italic.ttf"),
        font!("Lilex-Bold.ttf"),
        font!("Lilex-BoldItalic.ttf"),
        // Reference web-app families, still selectable via settings.
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
        assert_eq!(fonts.len(), 15);
        for font in &fonts {
            assert!(font.len() > 10_000, "font asset suspiciously small");
            assert!(is_truetype(font), "font asset is not a TrueType sfnt");
        }
    }

    #[test]
    fn family_names_are_stable() {
        assert_eq!(UI_FONT_FAMILY, "IBM Plex Sans");
        assert_eq!(MONO_FONT_FAMILY, "Lilex");
        assert!(MONO_FONT_FALLBACKS.contains(&"Menlo"));
    }
}
