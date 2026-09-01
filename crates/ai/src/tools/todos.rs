//! Per-session agent todo lists (T11-004).
//!
//! Port of `reference-src/src/modules/ai/lib/todos.ts` +
//! `store/todoStore.ts`. The agent uses `todo_write` to structure multi-step
//! work; the list is persisted per chat session so it survives restarts.

use std::collections::HashMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TodoStatus {
    Pending,
    InProgress,
    Completed,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Todo {
    pub id: String,
    pub title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub status: TodoStatus,
}

pub fn new_todo_id() -> String {
    format!("t-{}", uuid::Uuid::new_v4().simple())
}

/// Validate a candidate list: at most one `in_progress`, non-empty titles.
/// Returns `Err(reason)` on invalid.
pub fn validate_todos(todos: &[Todo]) -> Result<(), String> {
    let mut in_progress = 0;
    for t in todos {
        if t.title.trim().is_empty() {
            return Err("todo title cannot be empty".to_string());
        }
        if t.status == TodoStatus::InProgress {
            in_progress += 1;
        }
    }
    if in_progress > 1 {
        return Err(format!(
            "only one todo may be in_progress at a time (got {in_progress})"
        ));
    }
    Ok(())
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct TodoFile {
    #[serde(default)]
    by_session: HashMap<String, Vec<Todo>>,
}

/// Persistent per-session todo store (`~/.config/labonair/labonair-todos.json`).
#[derive(Debug)]
pub struct TodoStore {
    path: PathBuf,
    file: TodoFile,
    autosave: bool,
}

impl TodoStore {
    pub fn default_path() -> PathBuf {
        let dir = dirs::home_dir()
            .map(|h| h.join(".config").join("labonair"))
            .unwrap_or_else(|| PathBuf::from("."));
        let _ = std::fs::create_dir_all(&dir);
        dir.join("labonair-todos.json")
    }

    pub fn load(path: impl Into<PathBuf>) -> TodoStore {
        let path = path.into();
        let file = std::fs::read_to_string(&path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();
        TodoStore {
            path,
            file,
            autosave: true,
        }
    }

    pub fn open_default() -> TodoStore {
        Self::load(Self::default_path())
    }

    pub fn set_autosave(&mut self, on: bool) {
        self.autosave = on;
    }

    pub fn get(&self, session_id: &str) -> &[Todo] {
        self.file
            .by_session
            .get(session_id)
            .map_or(&[], |v| v.as_slice())
    }

    /// Replace a session's list wholesale (the `todo_write` semantics). Returns
    /// the validated list on success.
    pub fn set_todos(&mut self, session_id: &str, todos: Vec<Todo>) -> Result<(), String> {
        validate_todos(&todos)?;
        self.file.by_session.insert(session_id.to_string(), todos);
        self.write();
        Ok(())
    }

    pub fn clear(&mut self, session_id: &str) {
        if self.file.by_session.remove(session_id).is_some() {
            self.write();
        }
    }

    fn write(&self) {
        if !self.autosave {
            return;
        }
        let Ok(json) = serde_json::to_string_pretty(&self.file) else {
            return;
        };
        if let Some(dir) = self.path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let tmp = self.path.with_extension("json.tmp");
        let _ = std::fs::write(&tmp, json).and_then(|()| std::fs::rename(&tmp, &self.path));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn t(title: &str, status: TodoStatus) -> Todo {
        Todo {
            id: new_todo_id(),
            title: title.into(),
            description: None,
            status,
        }
    }

    #[test]
    fn validation_rules() {
        assert!(
            validate_todos(&[t("a", TodoStatus::InProgress), t("b", TodoStatus::Pending)]).is_ok()
        );
        assert!(validate_todos(&[
            t("a", TodoStatus::InProgress),
            t("b", TodoStatus::InProgress)
        ])
        .is_err());
        assert!(validate_todos(&[t("  ", TodoStatus::Pending)]).is_err());
    }

    #[test]
    fn set_get_and_persist() {
        let path = std::env::temp_dir().join(format!("todos-{}.json", uuid::Uuid::new_v4()));
        {
            let mut s = TodoStore::load(&path);
            s.set_todos("sess1", vec![t("step one", TodoStatus::InProgress)])
                .unwrap();
            assert_eq!(s.get("sess1").len(), 1);
            assert!(s.get("other").is_empty());
        }
        {
            let s = TodoStore::load(&path);
            assert_eq!(s.get("sess1")[0].title, "step one");
        }
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn rejects_invalid_and_keeps_old() {
        let mut s =
            TodoStore::load(std::env::temp_dir().join(format!("t-{}.json", uuid::Uuid::new_v4())));
        s.set_autosave(false);
        s.set_todos("s", vec![t("keep", TodoStatus::Pending)])
            .unwrap();
        assert!(s
            .set_todos(
                "s",
                vec![
                    t("x", TodoStatus::InProgress),
                    t("y", TodoStatus::InProgress)
                ]
            )
            .is_err());
        assert_eq!(s.get("s")[0].title, "keep");
    }
}
