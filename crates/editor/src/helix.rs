//! A compact selection-first Helix mode over the canonical [`Document`].
//!
//! Select mode extends the current selection for every motion. Delete and
//! change operate on that selection, while insert mode lets the GPUI editor
//! handle ordinary text input and routes Escape back through this state
//! machine.

use crate::document::{Document, Motion};
use crate::editing::EditIntent;

/// The mode shown by the editor status surface.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HelixMode {
    Select,
    Insert,
}

/// Keys translated by the GPUI adapter while Helix mode is active.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HelixKey {
    Char(char),
    Escape,
    Enter,
    Backspace,
    Delete,
    Tab,
    Redo,
}

/// UI-free selection-first modal state.
#[derive(Clone, Debug)]
pub struct Helix {
    mode: HelixMode,
}

impl Default for Helix {
    fn default() -> Self {
        Self::new()
    }
}

impl Helix {
    pub fn new() -> Self {
        Self {
            mode: HelixMode::Select,
        }
    }

    pub fn mode(&self) -> HelixMode {
        self.mode
    }

    pub fn status(&self) -> &'static str {
        match self.mode {
            HelixMode::Select => "HELIX SELECT",
            HelixMode::Insert => "HELIX INSERT",
        }
    }

    /// Handle one translated key. `false` means insert mode should pass the
    /// key to the regular Editor input path.
    pub fn on_key(&mut self, doc: &mut Document, key: HelixKey) -> bool {
        if self.mode == HelixMode::Insert {
            if key != HelixKey::Escape {
                return false;
            }
            self.mode = HelixMode::Select;
            ensure_character_selection_before_caret(doc);
            return true;
        }

        ensure_character_selection(doc);

        match key {
            HelixKey::Escape => doc.set_caret(doc.cursor, false),
            HelixKey::Redo => {
                doc.redo();
                ensure_character_selection(doc);
            }
            HelixKey::Char('i') => self.enter_insert(doc, false),
            HelixKey::Char('a') => self.enter_insert(doc, true),
            HelixKey::Char('c') => {
                ensure_character_selection(doc);
                let position = doc
                    .selection()
                    .map(|(start, _)| start)
                    .unwrap_or(doc.cursor);
                doc.execute_edit(EditIntent::DeleteSelections);
                doc.set_caret(position, false);
                self.mode = HelixMode::Insert;
            }
            HelixKey::Char('d' | 'x') | HelixKey::Delete => {
                ensure_character_selection(doc);
                doc.execute_edit(EditIntent::DeleteSelections);
                ensure_character_selection(doc);
            }
            HelixKey::Char('u') => {
                doc.undo();
                ensure_character_selection(doc);
            }
            HelixKey::Char('h') | HelixKey::Backspace => {
                doc.move_caret(Motion::Left, true);
            }
            HelixKey::Char('j') => doc.move_caret(Motion::Down, true),
            HelixKey::Char('k') => doc.move_caret(Motion::Up, true),
            HelixKey::Char('l') => doc.move_caret(Motion::Right, true),
            HelixKey::Char('w') => doc.move_caret(Motion::WordRight, true),
            HelixKey::Char('b') => doc.move_caret(Motion::WordLeft, true),
            HelixKey::Char('e') => doc.move_caret(Motion::WordRight, true),
            HelixKey::Char('0') => doc.move_caret(Motion::LineStart, true),
            HelixKey::Char('$') => doc.move_caret(Motion::LineEnd, true),
            HelixKey::Enter | HelixKey::Tab => {}
            HelixKey::Char(_) => {}
        }
        true
    }

    fn enter_insert(&mut self, doc: &mut Document, after_selection: bool) {
        let position =
            doc.selection().map_or(
                doc.cursor,
                |(start, end)| {
                    if after_selection {
                        end
                    } else {
                        start
                    }
                },
            );
        doc.set_caret(position, false);
        self.mode = HelixMode::Insert;
    }
}

/// A collapsed caret becomes a one-character selection when there is text at
/// or before it. At an empty document this leaves the only valid empty range.
fn ensure_character_selection(doc: &mut Document) {
    let snapshot = doc.snapshot();
    let selection = doc.selection_set().primary().range().resolve(&snapshot);
    if !selection.is_empty() {
        return;
    }

    let cursor = snapshot.position_to_byte(doc.cursor);
    if cursor < snapshot.byte_len() {
        doc.move_caret(Motion::Right, true);
    } else if cursor > 0 {
        doc.move_caret(Motion::Left, true);
    }
}

fn ensure_character_selection_before_caret(doc: &mut Document) {
    let snapshot = doc.snapshot();
    let selection = doc.selection_set().primary().range().resolve(&snapshot);
    if !selection.is_empty() {
        return;
    }

    let cursor = snapshot.position_to_byte(doc.cursor);
    if cursor > 0 {
        doc.move_caret(Motion::Left, true);
    } else if cursor < snapshot.byte_len() {
        doc.move_caret(Motion::Right, true);
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::Position;

    fn document(text: &str) -> Document {
        Document::from_file(PathBuf::from("test.txt"), text, 1)
    }

    #[test]
    fn motions_extend_selection_and_delete_removes_the_selected_text() {
        let mut doc = document("abc");
        let mut helix = Helix::new();

        assert!(helix.on_key(&mut doc, HelixKey::Char('l')));
        assert_eq!(doc.selected_text().as_deref(), Some("ab"));
        assert!(helix.on_key(&mut doc, HelixKey::Char('d')));
        assert_eq!(doc.text(), "c");
        assert_eq!(doc.selected_text().as_deref(), Some("c"));

        helix.on_key(&mut doc, HelixKey::Char('u'));
        assert_eq!(doc.text(), "abc");
        assert_eq!(doc.selected_text().as_deref(), Some("ab"));
    }

    #[test]
    fn insert_mode_places_the_caret_before_or_after_the_selection() {
        let mut doc = document("abc");
        doc.set_caret(Position::new(0, 1), false);
        let mut helix = Helix::new();

        helix.on_key(&mut doc, HelixKey::Char('l'));
        assert_eq!(doc.selected_text().as_deref(), Some("bc"));
        helix.on_key(&mut doc, HelixKey::Char('i'));
        assert_eq!(helix.mode(), HelixMode::Insert);
        assert_eq!(doc.cursor, Position::new(0, 1));
        doc.insert("x");
        assert_eq!(doc.text(), "axbc");
        assert!(helix.on_key(&mut doc, HelixKey::Escape));
        assert_eq!(helix.mode(), HelixMode::Select);
        assert_eq!(doc.selected_text().as_deref(), Some("x"));
    }

    #[test]
    fn change_deletes_selection_and_enters_insert_mode() {
        let mut doc = document("abc");
        let mut helix = Helix::new();
        helix.on_key(&mut doc, HelixKey::Char('l'));

        helix.on_key(&mut doc, HelixKey::Char('c'));
        assert_eq!(doc.text(), "c");
        assert_eq!(doc.cursor, Position::new(0, 0));
        assert_eq!(helix.mode(), HelixMode::Insert);
    }
}
