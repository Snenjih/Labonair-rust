//! Editor syntax colour schemes (T06-002).
//!
//! An [`EditorPalette`] maps every [`HighlightKind`] to a concrete [`Hsla`].
//! [`EditorPalette::resolve`] builds one for the current [`EditorThemeId`]:
//!
//! * [`EditorThemeId::Auto`] derives the colours from the active app theme's
//!   tokens, so it follows light/dark and imported themes automatically.
//! * The named schemes mirror Labonair's CodeMirror theme set
//!   (`reference-src/src/modules/editor/lib/themes.ts`) with fixed palettes.
//!
//! The editor view re-reads the palette every render and already observes the
//! [`ThemeStore`], so a theme change repaints the buffer with new colours.

use gpui::{rgb, Hsla};
use labonair_editor::HighlightKind;

use crate::theme::{EditorThemeId, ThemeStore};

/// A resolved colour for each token class.
#[derive(Debug, Clone, Copy)]
pub struct EditorPalette {
    pub comment: Hsla,
    pub keyword: Hsla,
    pub function: Hsla,
    pub macro_: Hsla,
    pub type_: Hsla,
    pub constructor: Hsla,
    pub namespace: Hsla,
    pub string: Hsla,
    pub escape: Hsla,
    pub number: Hsla,
    pub boolean: Hsla,
    pub constant: Hsla,
    pub property: Hsla,
    pub variable: Hsla,
    pub parameter: Hsla,
    pub operator: Hsla,
    pub punctuation: Hsla,
    pub tag: Hsla,
    pub attribute: Hsla,
    pub label: Hsla,
}

impl EditorPalette {
    /// The colour for a token class.
    pub fn color(&self, kind: HighlightKind) -> Hsla {
        match kind {
            HighlightKind::Keyword => self.keyword,
            HighlightKind::Function => self.function,
            HighlightKind::Macro => self.macro_,
            HighlightKind::Type => self.type_,
            HighlightKind::Constructor => self.constructor,
            HighlightKind::Namespace => self.namespace,
            HighlightKind::String => self.string,
            HighlightKind::Escape => self.escape,
            HighlightKind::Number => self.number,
            HighlightKind::Boolean => self.boolean,
            HighlightKind::Comment => self.comment,
            HighlightKind::Constant => self.constant,
            HighlightKind::Property => self.property,
            HighlightKind::Variable => self.variable,
            HighlightKind::Parameter => self.parameter,
            HighlightKind::Operator => self.operator,
            HighlightKind::Punctuation => self.punctuation,
            HighlightKind::Tag => self.tag,
            HighlightKind::Attribute => self.attribute,
            HighlightKind::Label => self.label,
        }
    }

    /// Resolve the palette for `id` against the active app theme.
    pub fn resolve(id: EditorThemeId, store: &ThemeStore) -> Self {
        match id {
            EditorThemeId::Auto => Self::from_app_theme(store),
            EditorThemeId::Atomone => Self::from_roles(&ATOMONE),
            EditorThemeId::Aura => Self::from_roles(&AURA),
            EditorThemeId::Copilot => Self::from_roles(&COPILOT),
            EditorThemeId::GithubDark => Self::from_roles(&GITHUB_DARK),
            EditorThemeId::GithubLight => Self::from_roles(&GITHUB_LIGHT),
            EditorThemeId::Nord => Self::from_roles(&NORD),
            EditorThemeId::TokyoNight => Self::from_roles(&TOKYO_NIGHT),
            EditorThemeId::XcodeDark => Self::from_roles(&XCODE_DARK),
            EditorThemeId::XcodeLight => Self::from_roles(&XCODE_LIGHT),
        }
    }

    fn from_app_theme(store: &ThemeStore) -> Self {
        let fg = store.foreground();
        let muted = store.muted_foreground();
        Self {
            comment: muted,
            keyword: store.primary(),
            function: store.accent(),
            macro_: store.accent(),
            type_: store.status_info(),
            constructor: store.status_info(),
            namespace: store.status_info(),
            string: store.status_success(),
            escape: store.status_warning(),
            number: store.status_warning(),
            boolean: store.status_warning(),
            constant: store.status_warning(),
            property: fg,
            variable: fg,
            parameter: fg,
            operator: muted,
            punctuation: muted,
            tag: store.primary(),
            attribute: store.accent(),
            label: store.status_error(),
        }
    }

    fn from_roles(r: &Roles) -> Self {
        let c = |v: u32| -> Hsla { rgb(v).into() };
        Self {
            comment: c(r.comment),
            keyword: c(r.keyword),
            function: c(r.function),
            macro_: c(r.function),
            type_: c(r.type_),
            constructor: c(r.type_),
            namespace: c(r.type_),
            string: c(r.string),
            escape: c(r.number),
            number: c(r.number),
            boolean: c(r.constant),
            constant: c(r.constant),
            property: c(r.property),
            variable: c(r.variable),
            parameter: c(r.variable),
            operator: c(r.operator),
            punctuation: c(r.operator),
            tag: c(r.tag),
            attribute: c(r.property),
            label: c(r.keyword),
        }
    }
}

/// The fixed role colours a named editor theme is built from.
struct Roles {
    comment: u32,
    keyword: u32,
    function: u32,
    type_: u32,
    string: u32,
    number: u32,
    constant: u32,
    property: u32,
    variable: u32,
    operator: u32,
    tag: u32,
}

const NORD: Roles = Roles {
    comment: 0x616e88,
    keyword: 0x81a1c1,
    function: 0x88c0d0,
    type_: 0x8fbcbb,
    string: 0xa3be8c,
    number: 0xb48ead,
    constant: 0xd08770,
    property: 0xd8dee9,
    variable: 0xd8dee9,
    operator: 0x81a1c1,
    tag: 0x81a1c1,
};

const TOKYO_NIGHT: Roles = Roles {
    comment: 0x565f89,
    keyword: 0xbb9af7,
    function: 0x7aa2f7,
    type_: 0x2ac3de,
    string: 0x9ece6a,
    number: 0xff9e64,
    constant: 0xff9e64,
    property: 0x73daca,
    variable: 0xc0caf5,
    operator: 0x89ddff,
    tag: 0xf7768e,
};

const GITHUB_DARK: Roles = Roles {
    comment: 0x8b949e,
    keyword: 0xff7b72,
    function: 0xd2a8ff,
    type_: 0xffa657,
    string: 0xa5d6ff,
    number: 0x79c0ff,
    constant: 0x79c0ff,
    property: 0x79c0ff,
    variable: 0xffa657,
    operator: 0xff7b72,
    tag: 0x7ee787,
};

const GITHUB_LIGHT: Roles = Roles {
    comment: 0x6e7781,
    keyword: 0xcf222e,
    function: 0x8250df,
    type_: 0x953800,
    string: 0x0a3069,
    number: 0x0550ae,
    constant: 0x0550ae,
    property: 0x0550ae,
    variable: 0x953800,
    operator: 0xcf222e,
    tag: 0x116329,
};

const ATOMONE: Roles = Roles {
    comment: 0x7d8799,
    keyword: 0xc678dd,
    function: 0x61afef,
    type_: 0xe5c07b,
    string: 0x98c379,
    number: 0xd19a66,
    constant: 0xd19a66,
    property: 0xe06c75,
    variable: 0xe06c75,
    operator: 0x56b6c2,
    tag: 0xe06c75,
};

const AURA: Roles = Roles {
    comment: 0x6d6d6d,
    keyword: 0xa277ff,
    function: 0xffca85,
    type_: 0x82e2ff,
    string: 0x61ffca,
    number: 0xffca85,
    constant: 0xffca85,
    property: 0xedecee,
    variable: 0xedecee,
    operator: 0xa277ff,
    tag: 0xa277ff,
};

const COPILOT: Roles = Roles {
    comment: 0x8b949e,
    keyword: 0xff7b72,
    function: 0xd2a8ff,
    type_: 0xf0883e,
    string: 0xa5d6ff,
    number: 0x79c0ff,
    constant: 0x79c0ff,
    property: 0x79c0ff,
    variable: 0xc9d1d9,
    operator: 0xff7b72,
    tag: 0x7ee787,
};

const XCODE_DARK: Roles = Roles {
    comment: 0x7f8c98,
    keyword: 0xfc5fa3,
    function: 0x67b7a4,
    type_: 0xd0a8ff,
    string: 0xfc6a5d,
    number: 0xd0bf69,
    constant: 0xd0bf69,
    property: 0x41a1c0,
    variable: 0xdfdfe0,
    operator: 0xdfdfe0,
    tag: 0xfc5fa3,
};

const XCODE_LIGHT: Roles = Roles {
    comment: 0x707f8c,
    keyword: 0xad3da4,
    function: 0x23575c,
    type_: 0x3900a0,
    string: 0xd12f1b,
    number: 0x272ad8,
    constant: 0x272ad8,
    property: 0x0f68a0,
    variable: 0x1c1c1e,
    operator: 0x1c1c1e,
    tag: 0xad3da4,
};

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::WindowAppearance;

    #[test]
    fn auto_palette_tracks_app_theme_mode() {
        let light = ThemeStore::new(WindowAppearance::Light);
        let dark = ThemeStore::new(WindowAppearance::Dark);
        let l = EditorPalette::resolve(EditorThemeId::Auto, &light);
        let d = EditorPalette::resolve(EditorThemeId::Auto, &dark);
        let kinds = [
            HighlightKind::Comment,
            HighlightKind::Keyword,
            HighlightKind::String,
            HighlightKind::Variable,
        ];
        assert!(
            kinds.iter().any(|&k| l.color(k) != d.color(k)),
            "auto scheme should differ between light and dark app themes"
        );
    }

    #[test]
    fn named_palette_is_stable_and_distinct() {
        let store = ThemeStore::new(WindowAppearance::Dark);
        let nord = EditorPalette::resolve(EditorThemeId::Nord, &store);
        assert_ne!(
            nord.color(HighlightKind::Keyword),
            nord.color(HighlightKind::String)
        );
        // Independent of the app theme.
        let light = ThemeStore::new(WindowAppearance::Light);
        let nord_light = EditorPalette::resolve(EditorThemeId::Nord, &light);
        assert_eq!(
            nord.color(HighlightKind::Keyword),
            nord_light.color(HighlightKind::Keyword)
        );
    }

    #[test]
    fn every_kind_has_a_color() {
        let store = ThemeStore::new(WindowAppearance::Dark);
        let p = EditorPalette::resolve(EditorThemeId::Auto, &store);
        for kind in [
            HighlightKind::Keyword,
            HighlightKind::Function,
            HighlightKind::Macro,
            HighlightKind::Type,
            HighlightKind::Constructor,
            HighlightKind::Namespace,
            HighlightKind::String,
            HighlightKind::Escape,
            HighlightKind::Number,
            HighlightKind::Boolean,
            HighlightKind::Comment,
            HighlightKind::Constant,
            HighlightKind::Property,
            HighlightKind::Variable,
            HighlightKind::Parameter,
            HighlightKind::Operator,
            HighlightKind::Punctuation,
            HighlightKind::Tag,
            HighlightKind::Attribute,
            HighlightKind::Label,
        ] {
            let _ = p.color(kind);
        }
    }
}
