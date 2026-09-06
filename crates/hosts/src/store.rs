//! Host queries that do not require application composition.
//!
//! Connection lifecycle belongs to `labonair-persistence`; this module owns
//! host-specific reads, ordering, and group mutations. Secret-bearing writes
//! and MCP side effects remain behind the backend compatibility adapter until
//! their capability contracts are migrated.

use crate::{Group, Host, ReorderItem};
use labonair_errors::LabonairError;
use labonair_persistence::Database;

const SELECT_HOSTS: &str = "SELECT id, name, host_address, port, username, auth_method, \
    private_key_path, group_id, tags, created_at, last_connected_at, \
    default_path_ssh, default_path_sftp, pin_to_top, sudo_password_set, \
    keep_alive_interval, keep_alive_tries, sort_order, tunnels, \
    startup_snippet_id, startup_snippet_mode, credential_id, \
    jump_host_id, notes, icon, block_agent_access FROM hosts";

fn row_to_host(row: &rusqlite::Row<'_>) -> rusqlite::Result<Host> {
    Ok(Host {
        id: row.get(0)?,
        name: row.get(1)?,
        host_address: row.get(2)?,
        port: row.get(3)?,
        username: row.get(4)?,
        auth_method: row.get(5)?,
        private_key_path: row.get(6)?,
        group_id: row.get(7)?,
        tags: row.get(8)?,
        created_at: row.get(9)?,
        last_connected_at: row.get(10)?,
        default_path_ssh: row.get(11)?,
        default_path_sftp: row.get(12)?,
        pin_to_top: row
            .get::<_, i64>(13)
            .map(|value| value != 0)
            .unwrap_or(false),
        sudo_password_set: row
            .get::<_, i64>(14)
            .map(|value| value != 0)
            .unwrap_or(false),
        keep_alive_interval: row.get(15)?,
        keep_alive_tries: row.get(16)?,
        sort_order: row.get(17).unwrap_or(0),
        tunnels: row.get(18)?,
        startup_snippet_id: row.get(19)?,
        startup_snippet_mode: row.get(20)?,
        credential_id: row.get(21)?,
        jump_host_id: row.get(22)?,
        notes: row.get(23)?,
        icon: row.get(24)?,
        block_agent_access: row
            .get::<_, i64>(25)
            .map(|value| value != 0)
            .unwrap_or(false),
    })
}

/// Return saved hosts in the same stable order used by the host manager.
pub async fn hosts_get_all(database: &Database) -> Result<Vec<Host>, LabonairError> {
    let connection = database
        .0
        .lock()
        .map_err(|error| LabonairError::Internal(error.to_string()))?;
    let mut statement = connection.prepare(&format!(
        "{SELECT_HOSTS} ORDER BY pin_to_top DESC, sort_order ASC, name ASC"
    ))?;
    let hosts = statement
        .query_map([], row_to_host)?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(hosts)
}

/// Persist the visible host ordering.
pub async fn hosts_reorder(
    database: &Database,
    items: Vec<ReorderItem>,
) -> Result<(), LabonairError> {
    let connection = database
        .0
        .lock()
        .map_err(|error| LabonairError::Internal(error.to_string()))?;
    for item in items {
        connection.execute(
            "UPDATE hosts SET sort_order=?1 WHERE id=?2",
            rusqlite::params![item.sort_order, item.id],
        )?;
    }
    Ok(())
}

/// Return host groups ordered for display.
pub async fn groups_get_all(database: &Database) -> Result<Vec<Group>, LabonairError> {
    let connection = database
        .0
        .lock()
        .map_err(|error| LabonairError::Internal(error.to_string()))?;
    let mut statement =
        connection.prepare("SELECT id, name, icon, color, created_at FROM groups ORDER BY name")?;
    let groups = statement
        .query_map([], row_to_group)?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(groups)
}

fn row_to_group(row: &rusqlite::Row<'_>) -> rusqlite::Result<Group> {
    Ok(Group {
        id: row.get(0)?,
        name: row.get(1)?,
        icon: row.get(2)?,
        color: row.get(3)?,
        created_at: row.get(4)?,
    })
}

/// Create a host group without involving application state.
pub async fn groups_create(
    database: &Database,
    name: String,
    icon: Option<String>,
    color: Option<String>,
) -> Result<Group, LabonairError> {
    let id = uuid::Uuid::new_v4().to_string();
    let created_at = now_millis();
    let connection = database
        .0
        .lock()
        .map_err(|error| LabonairError::Internal(error.to_string()))?;
    connection.execute(
        "INSERT INTO groups (id, name, icon, color, created_at) VALUES (?1, ?2, ?3, ?4, ?5)",
        rusqlite::params![id, name, icon, color, created_at],
    )?;
    Ok(Group {
        id,
        name,
        icon,
        color,
        created_at,
    })
}

/// Delete a group; SQLite clears its host references via the schema FK.
pub async fn groups_delete(database: &Database, id: String) -> Result<(), LabonairError> {
    let connection = database
        .0
        .lock()
        .map_err(|error| LabonairError::Internal(error.to_string()))?;
    connection.execute("DELETE FROM groups WHERE id=?1", rusqlite::params![id])?;
    Ok(())
}

/// Rename a host group and return its persisted representation.
pub async fn groups_update(
    database: &Database,
    id: String,
    name: String,
) -> Result<Group, LabonairError> {
    let connection = database
        .0
        .lock()
        .map_err(|error| LabonairError::Internal(error.to_string()))?;
    connection.execute(
        "UPDATE groups SET name=?1 WHERE id=?2",
        rusqlite::params![name, id],
    )?;
    Ok(connection.query_row(
        "SELECT id, name, icon, color, created_at FROM groups WHERE id=?1",
        rusqlite::params![id],
        row_to_group,
    )?)
}

fn now_millis() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}
