//! T19-010's single write path from the SQLite-backed
//! [`labonair_backend::modules::hosts::Host`] list into the
//! `SettingsContent.hosts.entries` (`labonair-settings-content`) projection.
//!
//! `HostManagerView`'s host add/edit/duplicate/delete/reorder flows are all
//! mature, pre-existing (T07-*/T16-008) and already correctly persist
//! secrets into `labonair_backend::modules::secrets` (never into
//! `settings.json`) via `hosts::db::hosts_create`/`hosts_update` — that part
//! is *not* duplicated or moved here. What is new for T19-010 is projecting
//! the resulting **non-secret** host state into `hosts.entries` (so
//! Settings › Hosts is a real `SettingsContent`-backed category, per
//! `docs/settings-guidelines.md`) plus the opaque `credential_ref`, and
//! doing so from exactly one place.
//!
//! [`apply_host_change`] is that one place: `HostManagerView::reload`/
//! `reload_list_only` (the only two functions that ever refresh
//! `HostManagerView::hosts` after a mutation — every create/update/
//! duplicate/delete/reorder path funnels through one of them) call it with
//! the freshly-reloaded host list. It fully replaces `hosts.entries` with a
//! fresh projection each time — simple, idempotent (a no-op write if
//! nothing actually changed, per `SettingsStore::update_user_settings`), and
//! avoids needing to thread a newly-created host's id back out of the
//! fire-and-forget `hosts_create` call.

use gpui::App;
use labonair_backend::modules::hosts::Host;
use labonair_backend::App as Backend;
use labonair_settings::SettingsStore;
use labonair_settings_content::hosts::{HostAuthMethod, HostEntry, HostTunnel};

fn map_auth_method(s: &str) -> HostAuthMethod {
    match s {
        "key" => HostAuthMethod::PublicKey,
        "credential" | "agent" => HostAuthMethod::Agent,
        _ => HostAuthMethod::Password,
    }
}

/// Same best-effort shape as `backend::modules::settings::migrate_v2`'s
/// tunnel parse — the SQLite `tunnels` column's JSON shape isn't
/// backend-enforced; a value that doesn't parse cleanly is dropped (never
/// fatal), the SQLite row itself is untouched.
fn parse_tunnels(raw: &str) -> Vec<HostTunnel> {
    #[derive(serde::Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct RawTunnel {
        local_port: u16,
        remote_host: String,
        remote_port: u16,
    }
    serde_json::from_str::<Vec<RawTunnel>>(raw)
        .map(|list| {
            list.into_iter()
                .map(|t| HostTunnel {
                    local_port: t.local_port,
                    remote_host: t.remote_host,
                    remote_port: t.remote_port,
                })
                .collect()
        })
        .unwrap_or_default()
}

fn parse_tags(raw: &str) -> Vec<String> {
    if let Ok(list) = serde_json::from_str::<Vec<String>>(raw) {
        return list;
    }
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Vec::new();
    }
    trimmed
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

/// Pure `Host` -> `HostEntry` projection (non-secret fields only).
/// `credential_ref` is resolved by the caller (needs the secret store).
fn host_entry_from(host: &Host, credential_ref: Option<String>) -> HostEntry {
    HostEntry {
        id: host.id.clone(),
        name: host.name.clone(),
        address: host.host_address.clone(),
        port: host.port.clamp(0, u16::MAX as i64) as u16,
        user: host.username.clone(),
        auth_method: map_auth_method(&host.auth_method),
        jump_host_ref: host.jump_host_id.clone(),
        tunnels: host
            .tunnels
            .as_deref()
            .map(parse_tunnels)
            .unwrap_or_default(),
        last_connected_at: host.last_connected_at,
        group: host.group_id.clone(),
        tags: host.tags.as_deref().map(parse_tags).unwrap_or_default(),
        credential_ref,
    }
}

/// The single write path (T19-010's Anweisung #3) from a freshly-reloaded
/// SQLite host list into `SettingsContent.hosts.entries` — never writes a
/// secret, only the opaque `credential_ref` (looked up from the existing
/// secret store, see [`labonair_backend::modules::hosts::credential_ref`]).
/// A no-op if `SettingsStore` isn't published as a global yet (headless /
/// hosts-ui's own unit tests construct a `HostManagerView` without one).
pub fn apply_host_change(app: &Backend, hosts: &[Host], cx: &mut App) {
    if !cx.has_global::<SettingsStore>() {
        return;
    }
    let entries: Vec<HostEntry> = hosts
        .iter()
        .map(|h| {
            let credential_ref = labonair_backend::modules::hosts::credential_ref(app, &h.id);
            host_entry_from(h, credential_ref)
        })
        .collect();
    let _ = cx.global_mut::<SettingsStore>().update_user_settings(|c| {
        c.hosts.entries = Some(entries);
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn host_stub(id: &str, name: &str) -> Host {
        Host {
            id: id.to_string(),
            name: name.to_string(),
            host_address: "example.com".to_string(),
            port: 22,
            username: "root".to_string(),
            auth_method: "password".to_string(),
            private_key_path: None,
            group_id: None,
            tags: None,
            created_at: 0,
            last_connected_at: Some(42),
            default_path_ssh: None,
            default_path_sftp: None,
            pin_to_top: false,
            sudo_password_set: false,
            keep_alive_interval: None,
            keep_alive_tries: None,
            sort_order: 0,
            tunnels: None,
            startup_snippet_id: None,
            startup_snippet_mode: None,
            credential_id: None,
            jump_host_id: None,
            notes: None,
            icon: None,
            block_agent_access: false,
        }
    }

    #[test]
    fn host_entry_from_never_carries_a_secret() {
        let host = host_stub("h1", "My Server");
        let entry = host_entry_from(&host, Some("secrets:labonair-app:h1".to_string()));
        assert_eq!(entry.id, "h1");
        assert_eq!(entry.name, "My Server");
        assert_eq!(entry.address, "example.com");
        assert_eq!(entry.port, 22);
        assert_eq!(entry.user, "root");
        assert_eq!(entry.auth_method, HostAuthMethod::Password);
        assert_eq!(
            entry.credential_ref.as_deref(),
            Some("secrets:labonair-app:h1")
        );
        // `HostEntry` has no field that could carry a plaintext secret in
        // the first place — this is a structural guarantee, not just a
        // runtime check, but assert the serialized JSON has no obviously
        // secret-shaped key as a regression guard.
        let json = serde_json::to_value(&entry).unwrap();
        let obj = json.as_object().unwrap();
        for key in obj.keys() {
            let lower = key.to_lowercase();
            assert!(
                !lower.contains("password") && !lower.contains("secret"),
                "HostEntry must never carry a secret-shaped field, found {key}"
            );
        }
    }

    #[test]
    fn map_auth_method_covers_all_backend_strings() {
        assert_eq!(map_auth_method("password"), HostAuthMethod::Password);
        assert_eq!(map_auth_method("key"), HostAuthMethod::PublicKey);
        assert_eq!(map_auth_method("credential"), HostAuthMethod::Agent);
        assert_eq!(map_auth_method("agent"), HostAuthMethod::Agent);
        assert_eq!(map_auth_method("none"), HostAuthMethod::Password);
    }

    #[test]
    fn parse_tunnels_roundtrip() {
        let raw = r#"[{"localPort":8080,"remoteHost":"127.0.0.1","remotePort":80}]"#;
        let tunnels = parse_tunnels(raw);
        assert_eq!(tunnels.len(), 1);
        assert_eq!(tunnels[0].local_port, 8080);
        assert_eq!(tunnels[0].remote_host, "127.0.0.1");
        assert_eq!(tunnels[0].remote_port, 80);
    }

    #[test]
    fn parse_tunnels_bad_json_drops_silently() {
        assert!(parse_tunnels("not json").is_empty());
    }

    /// Full integration test of the acceptance criterion: `apply_host_change`
    /// writes the non-secret fields into `settings.json`'s `User` layer +
    /// sets `credential_ref`, and never writes the secret itself anywhere in
    /// that file. Uses the real `hosts::db::hosts_create` (so the password
    /// really is stored in the secret store under the scheme
    /// `credential_ref` reads back) and a real `SettingsStore` rooted at a
    /// temp file (T19-010 made `SettingsStore::new` `pub` for exactly this).
    #[gpui::test]
    fn apply_host_change_writes_non_secret_fields_and_no_secret(cx: &mut gpui::TestAppContext) {
        let backend_dir =
            std::env::temp_dir().join(format!("labonair-apply-{}", uuid::Uuid::new_v4()));
        let app = labonair_backend::App::new(&backend_dir).unwrap();
        let settings_path = std::env::temp_dir().join(format!(
            "labonair-apply-settings-{}.json",
            uuid::Uuid::new_v4()
        ));

        let rt = tokio::runtime::Runtime::new().unwrap();
        let created = rt
            .block_on(labonair_backend::modules::hosts::db::hosts_create(
                app.clone(),
                &app.db,
                &app.secrets,
                "My Server".to_string(),
                "example.com".to_string(),
                22,
                "root".to_string(),
                "password".to_string(),
                None,
                None,
                None,
                Some("s3cr3t".to_string()),
                None,
                None,
                None,
                Some(false),
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                Some(false),
            ))
            .unwrap();

        cx.update(|cx| {
            let mut store = SettingsStore::new(settings_path.clone());
            store.reload_user_layer();
            cx.set_global(store);

            apply_host_change(&app, std::slice::from_ref(&created), cx);

            let store = cx.global::<SettingsStore>();
            let entries = store.merged().hosts.entries.clone().unwrap_or_default();
            assert_eq!(entries.len(), 1);
            let entry = &entries[0];
            assert_eq!(entry.id, created.id);
            assert_eq!(entry.name, "My Server");
            assert_eq!(entry.address, "example.com");
            assert_eq!(entry.port, 22);
            assert_eq!(entry.user, "root");
            assert_eq!(
                entry.credential_ref.as_deref(),
                Some(format!("secrets:labonair-app:{}", created.id)).as_deref()
            );
        });

        // The on-disk `User` layer file must contain the host's non-secret
        // fields, and never the plaintext password.
        let text = std::fs::read_to_string(&settings_path).unwrap();
        assert!(text.contains("My Server"));
        assert!(text.contains("example.com"));
        assert!(!text.contains("s3cr3t"));

        let _ = std::fs::remove_file(&settings_path);
        let _ = std::fs::remove_dir_all(&backend_dir);
    }
}
