//! Small pure helpers for the Settings UI. The `Settings` -> `ThemeStore`
//! bridge that used to live here moved to `labonair-theme-ui` in R08-005 so
//! the Settings UI stays a values-only editor.

pub(crate) fn char_of(ks: &gpui::Keystroke) -> Option<String> {
    ks.key_char
        .clone()
        .filter(|s| !s.is_empty() && !s.chars().any(|c| c.is_control()))
        .or_else(|| {
            (ks.key.chars().count() == 1 && !ks.key.chars().any(|c| c.is_control()))
                .then(|| ks.key.clone())
        })
}
