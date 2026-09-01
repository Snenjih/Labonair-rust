use serde::Serialize;

#[derive(Debug, Clone, Serialize, serde::Deserialize)]
pub struct SshConfigEntry {
    pub alias: String,
    pub host_address: String,
    pub port: i64,
    pub username: Option<String>,
    pub auth_method: String,
    pub private_key_path: Option<String>,
    pub proxy_jump: Option<String>,
}

fn flush_entry(
    alias: String,
    map: &std::collections::HashMap<String, String>,
) -> Option<SshConfigEntry> {
    if alias.contains('*') || alias.contains('?') {
        return None;
    }
    let host_address = map
        .get("hostname")
        .cloned()
        .unwrap_or_else(|| alias.clone());
    let port: i64 = map
        .get("port")
        .and_then(|p| p.parse().ok())
        .filter(|&p: &i64| p > 0 && p < 65536)
        .unwrap_or(22);
    let username = map.get("user").cloned();
    let (auth_method, private_key_path) = if let Some(key) = map.get("identityfile") {
        let expanded = if key.starts_with("~/") {
            dirs::home_dir()
                .map(|h| h.join(&key[2..]).to_string_lossy().to_string())
                .unwrap_or_else(|| key.clone())
        } else {
            key.clone()
        };
        ("key".to_string(), Some(expanded))
    } else {
        ("password".to_string(), None)
    };
    let proxy_jump = map.get("proxyjump").cloned().and_then(|pj| {
        let pj = pj.trim().to_string();
        if pj.eq_ignore_ascii_case("none") {
            None
        } else {
            Some(pj)
        }
    });
    Some(SshConfigEntry {
        alias,
        host_address,
        port,
        username,
        auth_method,
        private_key_path,
        proxy_jump,
    })
}

pub fn parse_ssh_config(content: &str) -> Vec<SshConfigEntry> {
    let mut entries: Vec<SshConfigEntry> = Vec::new();
    let mut current_alias: Option<String> = None;
    let mut current: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    // `Match` opens a conditional block whose directives we do not model —
    // suppress key capture until the next top-level `Host`. `Include` is read
    // but not recursed into (the referenced files are not followed).
    let mut in_match = false;

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        let sep = match trimmed.find(|c: char| c.is_whitespace() || c == '=') {
            Some(p) => p,
            None => continue,
        };
        let key = trimmed[..sep].to_lowercase();
        let value = trimmed[sep..]
            .trim_start_matches(|c: char| c.is_whitespace() || c == '=')
            .trim()
            .to_string();
        if value.is_empty() {
            continue;
        }

        if key == "host" {
            if let Some(alias) = current_alias.take() {
                if let Some(entry) = flush_entry(alias, &current) {
                    entries.push(entry);
                }
            }
            current.clear();
            current_alias = Some(value);
            in_match = false;
        } else if key == "match" {
            in_match = true;
        } else if key == "include" {
            // read but intentionally not followed
        } else if current_alias.is_some() && !in_match {
            // Only store first occurrence of each key (SSH config precedence)
            current.entry(key).or_insert(value);
        }
    }
    if let Some(alias) = current_alias {
        if let Some(entry) = flush_entry(alias, &current) {
            entries.push(entry);
        }
    }
    entries
}

pub async fn parse_ssh_config_cmd() -> Result<Vec<SshConfigEntry>, String> {
    let path = dirs::home_dir()
        .map(|h| h.join(".ssh").join("config"))
        .ok_or("Could not determine home directory")?;
    if !path.exists() {
        return Ok(vec![]);
    }
    let content = std::fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read ~/.ssh/config: {}", e))?;
    Ok(parse_ssh_config(&content))
}

/// How to treat an imported entry whose alias already names an app host.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ImportConflict {
    /// Leave the existing host untouched (the entry is not imported).
    #[default]
    Skip,
    /// Update the existing host's mapped fields (address, port, user, auth).
    Overwrite,
    /// Import under a fresh, de-duplicated alias (`alias-2`, `alias-3`, …).
    Rename,
}

/// Imports parsed SSH-config entries into the hosts table, mapping each
/// entry's fields onto the app host model (alias → name, `HostName` →
/// address, port, user, `IdentityFile` → key path + `key` auth,
/// `ProxyJump` → `jump_host_id`). `conflict` decides what happens when an
/// entry's alias already exists. Returns the ids of the hosts that were
/// created or overwritten (skipped conflicts are not included).
pub async fn import_ssh_config_entries(
    entries: Vec<SshConfigEntry>,
    conflict: ImportConflict,
    hosts_db: &crate::modules::hosts::HostsDb,
) -> Result<Vec<String>, String> {
    use uuid::Uuid;
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64;

    // Snapshot existing host name → id.
    let mut existing: std::collections::HashMap<String, String> = {
        let conn = hosts_db.0.lock().map_err(|e| e.to_string())?;
        let mut stmt = conn
            .prepare("SELECT name, id FROM hosts")
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))
            .map_err(|e| e.to_string())?;
        rows.filter_map(|r| r.ok()).collect()
    };

    // alias-as-written-in-config → resolved host id (used for ProxyJump).
    let mut alias_to_id: std::collections::HashMap<String, String> =
        std::collections::HashMap::new();
    // (entry index, host id) for the entries we may re-point in pass 2.
    let mut touched: Vec<(usize, String)> = Vec::new();
    let mut result_ids: Vec<String> = Vec::new();

    for (idx, entry) in entries.iter().enumerate() {
        let username = entry.username.as_deref().unwrap_or("");

        if let Some(existing_id) = existing.get(&entry.alias).cloned() {
            match conflict {
                ImportConflict::Skip => {
                    alias_to_id.insert(entry.alias.clone(), existing_id);
                    continue;
                }
                ImportConflict::Overwrite => {
                    let conn = hosts_db.0.lock().map_err(|e| e.to_string())?;
                    conn.execute(
                        "UPDATE hosts SET host_address = ?1, port = ?2, username = ?3, \
                         auth_method = ?4, private_key_path = ?5 WHERE id = ?6",
                        rusqlite::params![
                            entry.host_address,
                            entry.port,
                            username,
                            entry.auth_method,
                            entry.private_key_path,
                            existing_id,
                        ],
                    )
                    .map_err(|e| e.to_string())?;
                    alias_to_id.insert(entry.alias.clone(), existing_id.clone());
                    touched.push((idx, existing_id.clone()));
                    result_ids.push(existing_id);
                    continue;
                }
                ImportConflict::Rename => {}
            }
        }

        // Pick a free name: the alias itself, or `alias-2`, `alias-3`, … when
        // it collides (with a pre-existing host or an earlier entry in this
        // same batch).
        let mut name = entry.alias.clone();
        if existing.contains_key(&name) {
            let mut n = 2;
            loop {
                let cand = format!("{}-{}", entry.alias, n);
                if !existing.contains_key(&cand) {
                    name = cand;
                    break;
                }
                n += 1;
            }
        }

        let id = Uuid::new_v4().to_string();
        {
            let conn = hosts_db.0.lock().map_err(|e| e.to_string())?;
            let sort_order: i64 = conn
                .query_row(
                    "SELECT COALESCE(MAX(sort_order), 0) + 1 FROM hosts",
                    [],
                    |r| r.get(0),
                )
                .unwrap_or(0);
            conn.execute(
                "INSERT INTO hosts (id, name, host_address, port, username, auth_method, private_key_path, created_at, sort_order) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                rusqlite::params![
                    id,
                    name,
                    entry.host_address,
                    entry.port,
                    username,
                    entry.auth_method,
                    entry.private_key_path,
                    now,
                    sort_order,
                ],
            )
            .map_err(|e| e.to_string())?;
        }
        existing.insert(name.clone(), id.clone());
        alias_to_id.insert(entry.alias.clone(), id.clone());
        touched.push((idx, id.clone()));
        result_ids.push(id);
    }

    // Second pass: resolve proxy_jump aliases to host ids.
    for (idx, host_id) in &touched {
        if let Some(ref pj_alias) = entries[*idx].proxy_jump {
            // Extract just the host part from "user@host:port" format.
            let pj_host = pj_alias
                .split('@')
                .next_back()
                .unwrap_or(pj_alias)
                .split(':')
                .next()
                .unwrap_or(pj_alias)
                .trim();
            if let Some(jump_id) = alias_to_id.get(pj_host).or_else(|| existing.get(pj_host)) {
                let conn = hosts_db.0.lock().map_err(|e| e.to_string())?;
                let _ = conn.execute(
                    "UPDATE hosts SET jump_host_id = ?1 WHERE id = ?2",
                    rusqlite::params![jump_id, host_id],
                );
            }
        }
    }

    Ok(result_ids)
}

/// Writes an `export_ssh_config` text block to `~/.ssh/config`. With
/// `append = true` the block is appended after the existing file's content
/// (a trailing newline is inserted first if needed); otherwise the file is
/// replaced. The write is atomic (temp file + rename) and the resulting
/// file is chmod `0600` on Unix. Returns the config path. The caller is
/// responsible for gating `append = false` behind an explicit user action —
/// this never overwrites `~/.ssh/config` on its own.
pub async fn write_ssh_config_export(block: String, append: bool) -> Result<String, String> {
    let path = dirs::home_dir()
        .map(|h| h.join(".ssh").join("config"))
        .ok_or("Could not determine home directory")?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }

    let mut content = String::new();
    if append && path.exists() {
        content = std::fs::read_to_string(&path)
            .map_err(|e| format!("Failed to read ~/.ssh/config: {}", e))?;
        if !content.is_empty() && !content.ends_with('\n') {
            content.push('\n');
        }
    }
    content.push_str(&block);

    let tmp = path.with_extension("labonair-tmp");
    std::fs::write(&tmp, content.as_bytes()).map_err(|e| e.to_string())?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o600));
    }
    std::fs::rename(&tmp, &path).map_err(|e| e.to_string())?;
    Ok(path.to_string_lossy().to_string())
}

/// `(cred_type, key_path)` per credential id, used by `export_ssh_config` to
/// decide whether a `"credential"`-auth host's `IdentityFile` is safe to
/// emit (only for file-based, `cred_type == "key"` credentials).
type CredentialExportMap = std::collections::HashMap<String, (String, Option<String>)>;

/// A host row's fields relevant to SSH config export, keyed by host id in
/// `export_ssh_config` so a jump host's name can be resolved even when the
/// jump host itself isn't part of the requested `host_ids` batch.
struct ExportHostRow {
    name: String,
    host_address: String,
    port: i64,
    username: String,
    auth_method: String,
    private_key_path: Option<String>,
    credential_id: Option<String>,
    jump_host_id: Option<String>,
}

/// Generates an `~/.ssh/config`-format text block for the given hosts — the
/// exact reverse of `flush_entry`'s field mapping: `Host` <- name,
/// `HostName` <- host_address, `Port` <- port (omitted when 22, the
/// default), `User` <- username, `IdentityFile` <- private_key_path (only
/// for `auth_method == "key"`, or a `"credential"`-auth host whose
/// credential is itself file-based), `ProxyJump` <- the jump host's name.
///
/// `ProxyJump` resolution is best-effort against the *entire* hosts table,
/// not just `host_ids` — a jump host outside the exported batch still gets
/// a `ProxyJump <name>` line, it just won't be self-contained as a file on
/// its own. Never writes a secret: password-auth hosts and password-type
/// credentials always omit `IdentityFile`, since no plaintext value is ever
/// safe to embed in a `~/.ssh/config`-style file.
pub async fn export_ssh_config(
    host_ids: Vec<String>,
    hosts_db: &crate::modules::hosts::HostsDb,
) -> Result<String, String> {
    let (all_hosts, credentials): (
        std::collections::HashMap<String, ExportHostRow>,
        CredentialExportMap,
    ) = {
        let conn = hosts_db.0.lock().map_err(|e| e.to_string())?;

        let mut host_stmt = conn
            .prepare(
                "SELECT id, name, host_address, port, username, auth_method, \
                 private_key_path, credential_id, jump_host_id FROM hosts",
            )
            .map_err(|e| e.to_string())?;
        let hosts = host_stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    ExportHostRow {
                        name: row.get(1)?,
                        host_address: row.get(2)?,
                        port: row.get(3)?,
                        username: row.get(4)?,
                        auth_method: row.get(5)?,
                        private_key_path: row.get(6)?,
                        credential_id: row.get(7)?,
                        jump_host_id: row.get(8)?,
                    },
                ))
            })
            .map_err(|e| e.to_string())?
            .filter_map(|r| r.ok())
            .collect::<std::collections::HashMap<_, _>>();

        let mut cred_stmt = conn
            .prepare("SELECT id, cred_type, key_path FROM credentials")
            .map_err(|e| e.to_string())?;
        let creds = cred_stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    (row.get::<_, String>(1)?, row.get::<_, Option<String>>(2)?),
                ))
            })
            .map_err(|e| e.to_string())?
            .filter_map(|r| r.ok())
            .collect::<std::collections::HashMap<_, _>>();

        (hosts, creds)
    };

    let mut out = String::new();
    for id in &host_ids {
        let Some(host) = all_hosts.get(id) else {
            continue;
        };

        // IdentityFile: direct key auth, or a file-based ("key"-type)
        // credential. Everything else (password auth, password-type
        // credential, "none") omits the line entirely.
        let identity_file = match host.auth_method.as_str() {
            "key" => host.private_key_path.clone(),
            "credential" => host
                .credential_id
                .as_ref()
                .and_then(|cid| credentials.get(cid))
                .filter(|(cred_type, _)| cred_type.as_str() == "key")
                .and_then(|(_, key_path)| key_path.clone()),
            _ => None,
        };

        let proxy_jump = host
            .jump_host_id
            .as_ref()
            .and_then(|jid| all_hosts.get(jid))
            .map(|jh| jh.name.clone());

        out.push_str(&format!("Host {}\n", host.name));
        out.push_str(&format!("    HostName {}\n", host.host_address));
        if host.port != 22 {
            out.push_str(&format!("    Port {}\n", host.port));
        }
        if !host.username.is_empty() {
            out.push_str(&format!("    User {}\n", host.username));
        }
        if let Some(key) = identity_file {
            out.push_str(&format!("    IdentityFile {}\n", key));
        }
        if let Some(pj) = proxy_jump {
            out.push_str(&format!("    ProxyJump {}\n", pj));
        }
        out.push('\n');
    }

    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::hosts::HostsDb;
    use rusqlite::Connection;

    const SAMPLE: &str = r#"
# global-ish defaults live under a Host * block
Host *
    ServerAliveInterval 60

Host bastion
    HostName bastion.example.com
    User jump
    Port 2222

Host web
    # inline comment
    HostName 10.0.0.5
    User deploy
    IdentityFile ~/.ssh/id_web
    ProxyJump jump@bastion:2222

Host db
    Hostname=10.0.0.6
    Port = 5433

Match host db
    ForwardAgent yes

Include ~/.ssh/config.d/*
"#;

    fn mk_db() -> HostsDb {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE hosts (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                host_address TEXT NOT NULL,
                port INTEGER NOT NULL DEFAULT 22,
                username TEXT NOT NULL DEFAULT '',
                auth_method TEXT NOT NULL DEFAULT 'password',
                private_key_path TEXT,
                credential_id TEXT,
                jump_host_id TEXT,
                created_at INTEGER NOT NULL DEFAULT 0,
                sort_order INTEGER NOT NULL DEFAULT 0
            );
            CREATE TABLE credentials (
                id TEXT PRIMARY KEY,
                cred_type TEXT NOT NULL,
                key_path TEXT
            );",
        )
        .unwrap();
        HostsDb(std::sync::Mutex::new(conn))
    }

    /// (name, host_address, port, auth_method, private_key_path, jump_host_id)
    type Row = (String, String, i64, String, Option<String>, Option<String>);

    fn names(db: &HostsDb) -> Vec<Row> {
        let conn = db.0.lock().unwrap();
        let mut stmt = conn
            .prepare(
                "SELECT name, host_address, port, auth_method, private_key_path, jump_host_id \
                 FROM hosts ORDER BY name",
            )
            .unwrap();
        stmt.query_map([], |r| {
            Ok((
                r.get(0)?,
                r.get(1)?,
                r.get(2)?,
                r.get(3)?,
                r.get(4)?,
                r.get(5)?,
            ))
        })
        .unwrap()
        .map(|r| r.unwrap())
        .collect()
    }

    #[test]
    fn parses_representative_config() {
        let entries = parse_ssh_config(SAMPLE);
        let aliases: Vec<&str> = entries.iter().map(|e| e.alias.as_str()).collect();
        // `Host *` is a wildcard and must be dropped; Match/Include ignored.
        assert_eq!(aliases, ["bastion", "web", "db"]);

        let web = entries.iter().find(|e| e.alias == "web").unwrap();
        assert_eq!(web.host_address, "10.0.0.5");
        assert_eq!(web.username.as_deref(), Some("deploy"));
        assert_eq!(web.auth_method, "key");
        assert!(web.private_key_path.as_deref().unwrap().ends_with("id_web"));
        assert!(!web.private_key_path.as_deref().unwrap().starts_with("~"));
        assert_eq!(web.proxy_jump.as_deref(), Some("jump@bastion:2222"));

        let db = entries.iter().find(|e| e.alias == "db").unwrap();
        assert_eq!(db.host_address, "10.0.0.6"); // `Hostname=` form
        assert_eq!(db.port, 5433); // `Port = 5433` form
        assert_eq!(db.auth_method, "password");

        let bastion = entries.iter().find(|e| e.alias == "bastion").unwrap();
        assert_eq!(bastion.port, 2222);
    }

    #[tokio::test]
    async fn imports_and_maps_onto_app_hosts_including_proxyjump() {
        let db = mk_db();
        let ids = import_ssh_config_entries(parse_ssh_config(SAMPLE), ImportConflict::Skip, &db)
            .await
            .unwrap();
        assert_eq!(ids.len(), 3);

        let rows = names(&db);
        let web = rows.iter().find(|r| r.0 == "web").unwrap();
        assert_eq!(web.1, "10.0.0.5");
        assert_eq!(web.3, "key");
        assert!(web.4.as_deref().unwrap().ends_with("id_web"));
        // ProxyJump resolved to the bastion's row id.
        let bastion = rows.iter().find(|r| r.0 == "bastion").unwrap();
        let bastion_id = {
            let conn = db.0.lock().unwrap();
            conn.query_row("SELECT id FROM hosts WHERE name = 'bastion'", [], |r| {
                r.get::<_, String>(0)
            })
            .unwrap()
        };
        assert_eq!(web.5.as_deref(), Some(bastion_id.as_str()));
        assert!(bastion.5.is_none());
    }

    #[tokio::test]
    async fn conflict_skip_overwrite_rename() {
        let entry = SshConfigEntry {
            alias: "web".into(),
            host_address: "1.1.1.1".into(),
            port: 22,
            username: Some("a".into()),
            auth_method: "password".into(),
            private_key_path: None,
            proxy_jump: None,
        };
        let updated = SshConfigEntry {
            host_address: "2.2.2.2".into(),
            port: 2200,
            ..entry.clone()
        };

        // Skip: existing row untouched, nothing created.
        let db = mk_db();
        import_ssh_config_entries(vec![entry.clone()], ImportConflict::Skip, &db)
            .await
            .unwrap();
        let ids = import_ssh_config_entries(vec![updated.clone()], ImportConflict::Skip, &db)
            .await
            .unwrap();
        assert!(ids.is_empty());
        let rows = names(&db);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].1, "1.1.1.1");

        // Overwrite: same row, mapped fields updated.
        let db = mk_db();
        import_ssh_config_entries(vec![entry.clone()], ImportConflict::Skip, &db)
            .await
            .unwrap();
        let ids = import_ssh_config_entries(vec![updated.clone()], ImportConflict::Overwrite, &db)
            .await
            .unwrap();
        assert_eq!(ids.len(), 1);
        let rows = names(&db);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].1, "2.2.2.2");
        assert_eq!(rows[0].2, 2200);

        // Rename: new row under `web-2`.
        let db = mk_db();
        import_ssh_config_entries(vec![entry.clone()], ImportConflict::Skip, &db)
            .await
            .unwrap();
        import_ssh_config_entries(vec![updated.clone()], ImportConflict::Rename, &db)
            .await
            .unwrap();
        let rows = names(&db);
        assert_eq!(rows.len(), 2);
        assert!(rows.iter().any(|r| r.0 == "web-2" && r.1 == "2.2.2.2"));
    }

    #[tokio::test]
    async fn export_is_well_formed_and_round_trips() {
        let db = mk_db();
        {
            let conn = db.0.lock().unwrap();
            conn.execute(
                "INSERT INTO hosts (id, name, host_address, port, username, auth_method, private_key_path, jump_host_id) \
                 VALUES ('b','bastion','bastion.example.com',2222,'jump','password',NULL,NULL)",
                [],
            )
            .unwrap();
            conn.execute(
                "INSERT INTO hosts (id, name, host_address, port, username, auth_method, private_key_path, jump_host_id) \
                 VALUES ('w','web','10.0.0.5',22,'deploy','key','/home/me/.ssh/id_web','b')",
                [],
            )
            .unwrap();
        }

        let text = export_ssh_config(vec!["b".into(), "w".into()], &db)
            .await
            .unwrap();
        assert!(text.contains("Host bastion\n"));
        assert!(text.contains("    HostName 10.0.0.5\n"));
        assert!(text.contains("    Port 2222\n"));
        assert!(!text.contains("    Port 22\n")); // default omitted
        assert!(text.contains("    IdentityFile /home/me/.ssh/id_web\n"));
        assert!(text.contains("    ProxyJump bastion\n"));

        // Round-trip: parse the export back and re-import into a clean db.
        let reparsed = parse_ssh_config(&text);
        assert_eq!(reparsed.len(), 2);
        let db2 = mk_db();
        import_ssh_config_entries(reparsed, ImportConflict::Skip, &db2)
            .await
            .unwrap();
        let rows = names(&db2);
        let web = rows.iter().find(|r| r.0 == "web").unwrap();
        assert_eq!(web.1, "10.0.0.5");
        assert_eq!(web.2, 22);
        assert_eq!(web.3, "key");
        assert_eq!(web.4.as_deref(), Some("/home/me/.ssh/id_web"));
        assert!(web.5.is_some()); // ProxyJump preserved
    }
}
