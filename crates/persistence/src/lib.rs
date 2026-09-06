//! Shared local database infrastructure.
//!
//! This crate owns the SQLite connection and schema lifecycle only. Feature
//! modules own their queries and domain models; the schema remains here while
//! the existing database is shared by hosts, credentials, and snippets.

use std::path::PathBuf;

/// The process-local SQLite connection shared by capability stores.
#[derive(Clone)]
pub struct Database(pub std::sync::Arc<std::sync::Mutex<rusqlite::Connection>>);

/// Open the application database, create its shared schema, and apply
/// idempotent migrations.
pub fn initialize_database(app_local_data_dir: PathBuf) -> Result<rusqlite::Connection, String> {
    std::fs::create_dir_all(&app_local_data_dir).map_err(|e| e.to_string())?;
    // Migrate nexum.db → labonair.db on first launch after rename.
    let labonair_path = app_local_data_dir.join("labonair.db");
    let nexum_path = app_local_data_dir.join("nexum.db");
    if nexum_path.exists() && !labonair_path.exists() {
        let _ = std::fs::rename(&nexum_path, &labonair_path);
    }

    let conn = rusqlite::Connection::open(&labonair_path).map_err(|e| e.to_string())?;
    conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys = ON;")
        .map_err(|e| e.to_string())?;
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS groups (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            icon TEXT,
            color TEXT,
            created_at INTEGER NOT NULL
        );
        CREATE TABLE IF NOT EXISTS hosts (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            host_address TEXT NOT NULL,
            port INTEGER NOT NULL DEFAULT 22,
            username TEXT NOT NULL,
            auth_method TEXT NOT NULL DEFAULT 'password',
            private_key_path TEXT,
            group_id TEXT REFERENCES groups(id) ON DELETE SET NULL,
            tags TEXT,
            created_at INTEGER NOT NULL,
            last_connected_at INTEGER
        );
        CREATE TABLE IF NOT EXISTS snippet_groups (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            icon TEXT,
            color TEXT,
            sort_order INTEGER NOT NULL DEFAULT 0,
            created_at INTEGER NOT NULL
        );
        CREATE TABLE IF NOT EXISTS snippets (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            description TEXT,
            command TEXT NOT NULL,
            target TEXT NOT NULL DEFAULT 'local',
            host_id TEXT REFERENCES hosts(id) ON DELETE SET NULL,
            default_exec_mode TEXT NOT NULL DEFAULT 'terminal',
            working_dir TEXT,
            group_id TEXT REFERENCES snippet_groups(id) ON DELETE SET NULL,
            tags TEXT,
            sort_order INTEGER NOT NULL DEFAULT 0,
            created_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL
        );",
    )
    .map_err(|e| e.to_string())?;

    // Idempotent migrations — ignore errors if a column already exists.
    for sql in &[
        "ALTER TABLE hosts ADD COLUMN tags TEXT",
        "ALTER TABLE hosts ADD COLUMN private_key_path TEXT",
        "ALTER TABLE hosts ADD COLUMN last_connected_at INTEGER",
        "ALTER TABLE hosts ADD COLUMN default_path_ssh TEXT",
        "ALTER TABLE hosts ADD COLUMN default_path_sftp TEXT",
        "ALTER TABLE hosts ADD COLUMN pin_to_top INTEGER NOT NULL DEFAULT 0",
        "ALTER TABLE hosts ADD COLUMN sudo_password_set INTEGER NOT NULL DEFAULT 0",
        "ALTER TABLE hosts ADD COLUMN keep_alive_interval INTEGER",
        "ALTER TABLE hosts ADD COLUMN keep_alive_tries INTEGER",
        "ALTER TABLE hosts ADD COLUMN sort_order INTEGER NOT NULL DEFAULT 0",
        "ALTER TABLE hosts ADD COLUMN tunnels TEXT",
        "ALTER TABLE hosts ADD COLUMN startup_snippet_id TEXT",
        "ALTER TABLE hosts ADD COLUMN startup_snippet_mode TEXT",
        "ALTER TABLE groups ADD COLUMN icon TEXT",
        "ALTER TABLE groups ADD COLUMN color TEXT",
        "ALTER TABLE groups ADD COLUMN created_at INTEGER NOT NULL DEFAULT 0",
        "CREATE TABLE IF NOT EXISTS credentials (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            cred_type TEXT NOT NULL DEFAULT 'password',
            key_path TEXT,
            key_type TEXT,
            public_key TEXT,
            has_secret INTEGER NOT NULL DEFAULT 0,
            created_at INTEGER NOT NULL
        )",
        "ALTER TABLE hosts ADD COLUMN credential_id TEXT REFERENCES credentials(id) ON DELETE SET NULL",
        "ALTER TABLE hosts ADD COLUMN jump_host_id TEXT REFERENCES hosts(id) ON DELETE SET NULL",
        "ALTER TABLE hosts ADD COLUMN notes TEXT",
        "ALTER TABLE hosts ADD COLUMN icon TEXT",
        "UPDATE hosts SET keep_alive_interval = 25 WHERE keep_alive_interval IS NULL",
        "UPDATE hosts SET keep_alive_tries = 3 WHERE keep_alive_tries IS NULL",
        "ALTER TABLE hosts ADD COLUMN block_agent_access INTEGER NOT NULL DEFAULT 0",
    ] {
        let _ = conn.execute_batch(sql);
    }

    Ok(conn)
}

#[cfg(test)]
mod tests {
    use super::initialize_database;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn initializes_all_shared_tables() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock")
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("labonair-persistence-{unique}"));

        let conn = initialize_database(dir.clone()).expect("database initializes");
        let count: i64 = conn
            .query_row(
                "SELECT count(*) FROM sqlite_master WHERE type = 'table' AND name IN ('groups', 'hosts', 'credentials', 'snippet_groups', 'snippets')",
                [],
                |row| row.get(0),
            )
            .expect("table query");
        assert_eq!(count, 5);

        std::fs::remove_dir_all(dir).expect("cleanup test database");
    }
}
