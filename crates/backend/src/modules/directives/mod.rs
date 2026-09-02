//! AI directives — reusable `#handle` instruction blocks the composer can
//! reference (port of `reference-src/src/modules/ai/lib/directives.ts` +
//! `store/directivesStore.ts`).
//!
//! Persisted as a plain `labonair-directives.json` object in the config dir.

use serde::{Deserialize, Serialize};

use crate::modules::fs::paths::config_dir;

const DIRECTIVES_FILE: &str = "labonair-directives.json";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Directive {
    pub id: String,
    /// The `#handle` used in the composer. Lowercase `[a-z0-9-]+`.
    pub handle: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub content: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct DirectivesFile {
    #[serde(default)]
    directives: Vec<Directive>,
}

fn directives_path() -> std::path::PathBuf {
    config_dir().join(DIRECTIVES_FILE)
}

fn load_from(path: &std::path::Path) -> Vec<Directive> {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|raw| serde_json::from_str::<DirectivesFile>(&raw).ok())
        .map(|f| f.directives)
        .unwrap_or_default()
}

fn save_to(path: &std::path::Path, list: &[Directive]) -> Result<(), String> {
    let file = DirectivesFile {
        directives: list.to_vec(),
    };
    let json = serde_json::to_string_pretty(&file).map_err(|e| e.to_string())?;
    std::fs::write(path, json).map_err(|e| e.to_string())
}

/// Load all directives from the config dir.
pub fn load() -> Vec<Directive> {
    load_from(&directives_path())
}

/// Persist the directive list.
pub fn save(list: &[Directive]) -> Result<(), String> {
    save_to(&directives_path(), list)
}

/// Normalise a raw string into a valid `#handle` (`[a-z0-9-]+`, collapsed
/// dashes, trimmed). Port of `normalizeHandle`.
pub fn normalize_handle(raw: &str) -> String {
    let lowered = raw.trim().to_lowercase();
    let mut out = String::new();
    let mut last_dash = false;
    for ch in lowered.chars() {
        let c = if ch.is_ascii_alphanumeric() {
            last_dash = false;
            ch
        } else if ch == '-' || ch.is_whitespace() {
            if last_dash {
                continue;
            }
            last_dash = true;
            '-'
        } else {
            continue;
        };
        out.push(c);
    }
    out.trim_matches('-').to_string()
}

/// Insert-or-replace a directive by id. Pure.
pub fn upsert(current: &[Directive], directive: Directive) -> Vec<Directive> {
    let mut next: Vec<Directive> = current
        .iter()
        .filter(|d| d.id != directive.id)
        .cloned()
        .collect();
    next.push(directive);
    next
}

/// Remove a directive by id. Pure.
pub fn remove(current: &[Directive], id: &str) -> Vec<Directive> {
    current.iter().filter(|d| d.id != id).cloned().collect()
}

/// A fresh directive id.
pub fn new_directive_id() -> String {
    format!(
        "dir-{:x}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0)
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_handle_slugifies() {
        assert_eq!(normalize_handle("  Deploy Prod!! "), "deploy-prod");
        assert_eq!(normalize_handle("a__b--c"), "ab-c");
        assert_eq!(normalize_handle("a b  c"), "a-b-c");
        assert_eq!(normalize_handle("---x---"), "x");
        assert_eq!(normalize_handle("Ünïcodé"), "ncod");
    }

    #[test]
    fn upsert_and_remove_are_pure() {
        let d = |id: &str| Directive {
            id: id.to_string(),
            handle: id.to_string(),
            name: id.to_string(),
            description: String::new(),
            content: String::new(),
        };
        let list = upsert(&[], d("a"));
        assert_eq!(list.len(), 1);
        let list = upsert(&list, d("a"));
        assert_eq!(list.len(), 1);
        let list = upsert(&list, d("b"));
        assert_eq!(list.len(), 2);
        assert_eq!(remove(&list, "a").len(), 1);
    }
}
