//! The secret-safe view model of the SSH-host import step (no GPUI).
//!
//! This is the redaction boundary: a [`HostImportRow`] is built from an
//! `SshConfigEntry` but has no field for the identity-file path or any
//! credential, so nothing in the view can print one. Address, port and user
//! are held privately and only leave through [`HostImportRow::details`], which
//! masks them unless the user explicitly reveals them.

use std::collections::HashSet;

use labonair_ssh::SshConfigEntry;

pub const MASK: &str = "\u{2022}\u{2022}\u{2022}\u{2022}\u{2022}\u{2022}";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AuthKind {
    Key,
    Password,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HostImportRow {
    pub alias: String,
    pub auth: AuthKind,
    pub via_jump: bool,
    /// A host with this name is already in the app; it is imported as `Skip`.
    pub exists: bool,
    address: String,
    port: i64,
    username: Option<String>,
}

impl HostImportRow {
    /// `user@address:port`, or [`MASK`] unless `reveal` is set.
    pub fn details(&self, reveal: bool) -> String {
        if !reveal {
            return MASK.to_owned();
        }
        match self.username.as_deref().filter(|u| !u.is_empty()) {
            Some(user) => format!("{user}@{}:{}", self.address, self.port),
            None => format!("{}:{}", self.address, self.port),
        }
    }

    /// The short, non-sensitive labels shown next to the alias.
    pub fn tags(&self) -> Vec<&'static str> {
        let mut tags = vec![match self.auth {
            AuthKind::Key => "Key",
            AuthKind::Password => "Asks for password",
        }];
        if self.via_jump {
            tags.push("Jump host");
        }
        if self.exists {
            tags.push("Already added");
        }
        tags
    }
}

/// One row per entry, in entry order.
pub fn build_rows(entries: &[SshConfigEntry], existing: &HashSet<String>) -> Vec<HostImportRow> {
    entries
        .iter()
        .map(|entry| HostImportRow {
            alias: entry.alias.clone(),
            auth: if entry.auth_method == "key" {
                AuthKind::Key
            } else {
                AuthKind::Password
            },
            via_jump: entry.proxy_jump.is_some(),
            exists: existing.contains(&entry.alias),
            address: entry.host_address.clone(),
            port: entry.port,
            username: entry.username.clone(),
        })
        .collect()
}

/// Everything that is not already in the app starts selected.
pub fn default_selection(rows: &[HostImportRow]) -> HashSet<String> {
    rows.iter()
        .filter(|row| !row.exists)
        .map(|row| row.alias.clone())
        .collect()
}

/// Indexes of the rows whose alias contains `query` (case-insensitive). The
/// address and user are never searched, so a hidden detail cannot be probed.
pub fn filter(rows: &[HostImportRow], query: &str) -> Vec<usize> {
    let query = query.trim().to_lowercase();
    rows.iter()
        .enumerate()
        .filter(|(_, row)| query.is_empty() || row.alias.to_lowercase().contains(&query))
        .map(|(index, _)| index)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(alias: &str, auth: &str, jump: Option<&str>) -> SshConfigEntry {
        SshConfigEntry {
            alias: alias.into(),
            host_address: "203.0.113.7".into(),
            port: 2222,
            username: Some("deploy".into()),
            auth_method: auth.into(),
            private_key_path: (auth == "key")
                .then(|| "/Users/secret-person/.ssh/id_ed25519".into()),
            proxy_jump: jump.map(str::to_owned),
        }
    }

    #[test]
    fn nothing_the_view_can_reach_contains_the_key_path() {
        let entries = [entry("prod", "key", Some("bastion"))];
        let rows = build_rows(&entries, &HashSet::new());
        let row = &rows[0];
        let everything = format!(
            "{row:?} {} {} {:?}",
            row.details(false),
            row.details(true),
            row.tags()
        );
        assert!(!everything.contains("secret-person"));
        assert!(!everything.contains("id_ed25519"));
    }

    #[test]
    fn details_are_masked_until_revealed() {
        let rows = build_rows(&[entry("prod", "key", None)], &HashSet::new());
        assert_eq!(rows[0].details(false), MASK);
        assert!(!rows[0].details(false).contains("203.0.113.7"));
        assert_eq!(rows[0].details(true), "deploy@203.0.113.7:2222");
    }

    #[test]
    fn tags_reflect_auth_jump_and_existing() {
        let existing = HashSet::from(["web".to_owned()]);
        let rows = build_rows(
            &[
                entry("web", "password", Some("bastion")),
                entry("db", "key", None),
            ],
            &existing,
        );
        assert_eq!(
            rows[0].tags(),
            ["Asks for password", "Jump host", "Already added"]
        );
        assert_eq!(rows[1].tags(), ["Key"]);
    }

    #[test]
    fn existing_hosts_start_unselected() {
        let existing = HashSet::from(["web".to_owned()]);
        let rows = build_rows(
            &[entry("web", "key", None), entry("db", "key", None)],
            &existing,
        );
        assert_eq!(default_selection(&rows), HashSet::from(["db".to_owned()]));
    }

    #[test]
    fn search_matches_alias_only_case_insensitively() {
        let rows = build_rows(
            &[
                entry("Prod-API", "key", None),
                entry("staging", "key", None),
            ],
            &HashSet::new(),
        );
        assert_eq!(filter(&rows, "prod"), [0]);
        assert_eq!(filter(&rows, ""), [0, 1]);
        assert!(
            filter(&rows, "203.0.113").is_empty(),
            "address is not searchable"
        );
        assert!(filter(&rows, "deploy").is_empty(), "user is not searchable");
    }
}
