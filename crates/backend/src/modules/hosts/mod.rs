pub mod db;

pub use labonair_hosts::{Group, Host, ReorderItem};

pub struct HostsDb(pub std::sync::Mutex<rusqlite::Connection>);

/// The opaque `credential_ref` a `hosts.entries` (`SettingsContent`) row
/// carries when the host has a secret stored under the `"labonair-app"`
/// service in `backend::modules::secrets` (the scheme `db.rs`'s
/// `hosts_create`/`hosts_update` already use, and
/// `settings::migrate_v2::credential_ref_for` reads back) — `None` if no
/// such secret exists. Public wrapper (T19-010) so `labonair-hosts-ui`'s
/// `apply_host_change` can project the *reference* into `settings.json`
/// without ever touching the secret itself.
pub fn credential_ref(app: &crate::App, id: &str) -> Option<String> {
    crate::modules::secrets::get_password(app, &app.secrets, "labonair-app", id)
        .ok()
        .flatten()
        .map(|_| format!("secrets:labonair-app:{id}"))
}
