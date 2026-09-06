//! Host queries that do not require application composition.
//!
//! Connection lifecycle belongs to `labonair-persistence`; this module owns
//! host-specific reads, ordering, and group mutations. Secret-bearing writes
//! and MCP side effects remain behind the backend compatibility adapter until
//! their capability contracts are migrated.

use crate::{Group, Host, ReorderItem};
use labonair_errors::LabonairError;
use labonair_persistence::Database;
use labonair_secrets::{delete_password, get_password, store_password, SecretsState};

/// Input for creating a host. Secret values are accepted only by the store
/// boundary and are never part of the persisted [`Host`] read model.
#[derive(Debug, Clone)]
pub struct HostCreateRequest {
    pub name: String,
    pub host_address: String,
    pub port: i64,
    pub username: String,
    pub auth_method: String,
    pub private_key_path: Option<String>,
    pub group_id: Option<String>,
    pub tags: Option<String>,
    pub password: Option<String>,
    pub sudo_password: Option<String>,
    pub default_path_ssh: Option<String>,
    pub default_path_sftp: Option<String>,
    pub pin_to_top: Option<bool>,
    pub keep_alive_interval: Option<i64>,
    pub keep_alive_tries: Option<i64>,
    pub sort_order: Option<i64>,
    pub tunnels: Option<String>,
    pub startup_snippet_id: Option<String>,
    pub startup_snippet_mode: Option<String>,
    pub credential_id: Option<String>,
    pub jump_host_id: Option<String>,
    pub notes: Option<String>,
    pub icon: Option<String>,
    pub block_agent_access: Option<bool>,
}

/// Partial host update. `Some` means update the corresponding field; an empty
/// string clears nullable references where the existing product behavior uses
/// that convention.
#[derive(Debug, Clone)]
pub struct HostUpdateRequest {
    pub id: String,
    pub name: Option<String>,
    pub host_address: Option<String>,
    pub port: Option<i64>,
    pub username: Option<String>,
    pub auth_method: Option<String>,
    pub private_key_path: Option<String>,
    pub group_id: Option<String>,
    pub tags: Option<String>,
    pub password: Option<String>,
    pub sudo_password: Option<String>,
    pub default_path_ssh: Option<String>,
    pub default_path_sftp: Option<String>,
    pub pin_to_top: Option<bool>,
    pub keep_alive_interval: Option<i64>,
    pub keep_alive_tries: Option<i64>,
    pub sort_order: Option<i64>,
    pub tunnels: Option<String>,
    pub startup_snippet_id: Option<String>,
    pub startup_snippet_mode: Option<String>,
    pub credential_id: Option<String>,
    pub jump_host_id: Option<String>,
    pub notes: Option<String>,
    pub icon: Option<String>,
    pub block_agent_access: Option<bool>,
}

/// Side effects requested by the host store but owned by the integrating
/// capability. The store does not know how MCP grants are represented.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HostEvent {
    AgentAccessBlocked { host_id: String },
}

pub type HostEventHandler<'a> = dyn Fn(HostEvent) -> Result<(), LabonairError> + Send + Sync + 'a;

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

/// Duplicate a host and copy its two secret entries without exposing their
/// plaintext through the returned domain model.
pub async fn hosts_duplicate(
    database: &Database,
    secrets: &SecretsState,
    id: String,
) -> Result<Host, LabonairError> {
    let source = {
        let connection = database
            .0
            .lock()
            .map_err(|error| LabonairError::Internal(error.to_string()))?;
        connection.query_row(
            &format!("{SELECT_HOSTS} WHERE id=?1"),
            rusqlite::params![id],
            row_to_host,
        )?
    };

    let new_id = uuid::Uuid::new_v4().to_string();
    let created_at = now_millis();
    let name = format!("Copy of {}", source.name);
    {
        let connection = database
            .0
            .lock()
            .map_err(|error| LabonairError::Internal(error.to_string()))?;
        connection.execute(
            "INSERT INTO hosts (id, name, host_address, port, username, auth_method, \
             private_key_path, group_id, tags, created_at, default_path_ssh, default_path_sftp, \
             pin_to_top, sudo_password_set, keep_alive_interval, keep_alive_tries, sort_order, tunnels, \
             startup_snippet_id, startup_snippet_mode, credential_id, jump_host_id, notes, icon, \
             block_agent_access) \
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19,?20,?21,?22,?23,?24,?25)",
            rusqlite::params![
                new_id,
                name,
                source.host_address,
                source.port,
                source.username,
                source.auth_method,
                source.private_key_path,
                source.group_id,
                source.tags,
                created_at,
                source.default_path_ssh,
                source.default_path_sftp,
                0i64,
                source.sudo_password_set as i64,
                source.keep_alive_interval,
                source.keep_alive_tries,
                0i64,
                source.tunnels,
                source.startup_snippet_id,
                source.startup_snippet_mode,
                source.credential_id,
                source.jump_host_id,
                source.notes,
                source.icon,
                source.block_agent_access as i64
            ],
        )?;
    }

    if let Some(password) =
        get_password(secrets, "labonair-app", &id).map_err(LabonairError::Internal)?
    {
        store_password(secrets, "labonair-app", &new_id, &password)
            .map_err(LabonairError::Internal)?;
    }
    if let Some(password) =
        get_password(secrets, "labonair-sudo", &id).map_err(LabonairError::Internal)?
    {
        store_password(secrets, "labonair-sudo", &new_id, &password)
            .map_err(LabonairError::Internal)?;
    }

    let connection = database
        .0
        .lock()
        .map_err(|error| LabonairError::Internal(error.to_string()))?;
    Ok(connection.query_row(
        &format!("{SELECT_HOSTS} WHERE id=?1"),
        rusqlite::params![new_id],
        row_to_host,
    )?)
}

/// Create a host and store supplied secrets under opaque host-scoped keys.
pub async fn hosts_create(
    database: &Database,
    secrets: &SecretsState,
    request: HostCreateRequest,
) -> Result<Host, LabonairError> {
    let id = uuid::Uuid::new_v4().to_string();
    let created_at = now_millis();
    let pin = request.pin_to_top.unwrap_or(false) as i64;
    let order = request.sort_order.unwrap_or(0);
    let sudo_set = request.sudo_password.is_some() as i64;
    let blocked = request.block_agent_access.unwrap_or(false) as i64;

    {
        let connection = database
            .0
            .lock()
            .map_err(|error| LabonairError::Internal(error.to_string()))?;
        let snippet_id = request
            .startup_snippet_id
            .as_deref()
            .filter(|value| !value.is_empty());
        let icon = request.icon.as_deref().filter(|value| !value.is_empty());
        connection.execute(
            "INSERT INTO hosts (id, name, host_address, port, username, auth_method, \
             private_key_path, group_id, tags, created_at, default_path_ssh, default_path_sftp, \
             pin_to_top, sudo_password_set, keep_alive_interval, keep_alive_tries, sort_order, tunnels, \
             startup_snippet_id, startup_snippet_mode, credential_id, jump_host_id, notes, icon, \
             block_agent_access) \
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19,?20,?21,?22,?23,?24,?25)",
            rusqlite::params![
                id,
                request.name,
                request.host_address,
                request.port,
                request.username,
                request.auth_method,
                request.private_key_path,
                request.group_id,
                request.tags,
                created_at,
                request.default_path_ssh,
                request.default_path_sftp,
                pin,
                sudo_set,
                request.keep_alive_interval,
                request.keep_alive_tries,
                order,
                request.tunnels,
                snippet_id,
                request.startup_snippet_mode,
                request.credential_id,
                request.jump_host_id,
                request.notes,
                icon,
                blocked
            ],
        )?;
    }

    if let Some(password) = request.password {
        if !password.is_empty() {
            store_password(secrets, "labonair-app", &id, &password)
                .map_err(LabonairError::Internal)?;
        }
    }
    if let Some(password) = request.sudo_password {
        if !password.is_empty() {
            store_password(secrets, "labonair-sudo", &id, &password)
                .map_err(LabonairError::Internal)?;
        }
    }

    let connection = database
        .0
        .lock()
        .map_err(|error| LabonairError::Internal(error.to_string()))?;
    Ok(connection.query_row(
        &format!("{SELECT_HOSTS} WHERE id=?1"),
        rusqlite::params![id],
        row_to_host,
    )?)
}

/// Update a host, optionally emitting a typed request for capability-owned
/// side effects such as revoking MCP grants.
pub async fn hosts_update(
    database: &Database,
    secrets: &SecretsState,
    request: HostUpdateRequest,
    event_handler: Option<&HostEventHandler<'_>>,
) -> Result<Host, LabonairError> {
    let id = request.id.clone();
    {
        let connection = database
            .0
            .lock()
            .map_err(|error| LabonairError::Internal(error.to_string()))?;
        if let Some(value) = &request.name {
            connection.execute(
                "UPDATE hosts SET name=?1 WHERE id=?2",
                rusqlite::params![value, id],
            )?;
        }
        if let Some(value) = &request.host_address {
            connection.execute(
                "UPDATE hosts SET host_address=?1 WHERE id=?2",
                rusqlite::params![value, id],
            )?;
        }
        if let Some(value) = request.port {
            connection.execute(
                "UPDATE hosts SET port=?1 WHERE id=?2",
                rusqlite::params![value, id],
            )?;
        }
        if let Some(value) = &request.username {
            connection.execute(
                "UPDATE hosts SET username=?1 WHERE id=?2",
                rusqlite::params![value, id],
            )?;
        }
        if let Some(value) = &request.auth_method {
            connection.execute(
                "UPDATE hosts SET auth_method=?1 WHERE id=?2",
                rusqlite::params![value, id],
            )?;
        }
        if request.private_key_path.is_some() {
            connection.execute(
                "UPDATE hosts SET private_key_path=?1 WHERE id=?2",
                rusqlite::params![request.private_key_path, id],
            )?;
        }
        if request.group_id.is_some() {
            connection.execute(
                "UPDATE hosts SET group_id=?1 WHERE id=?2",
                rusqlite::params![request.group_id, id],
            )?;
        }
        if request.tags.is_some() {
            connection.execute(
                "UPDATE hosts SET tags=?1 WHERE id=?2",
                rusqlite::params![request.tags, id],
            )?;
        }
        if request.default_path_ssh.is_some() {
            connection.execute(
                "UPDATE hosts SET default_path_ssh=?1 WHERE id=?2",
                rusqlite::params![request.default_path_ssh, id],
            )?;
        }
        if request.default_path_sftp.is_some() {
            connection.execute(
                "UPDATE hosts SET default_path_sftp=?1 WHERE id=?2",
                rusqlite::params![request.default_path_sftp, id],
            )?;
        }
        if let Some(value) = request.pin_to_top {
            connection.execute(
                "UPDATE hosts SET pin_to_top=?1 WHERE id=?2",
                rusqlite::params![value as i64, id],
            )?;
        }
        if let Some(value) = request.keep_alive_interval {
            connection.execute(
                "UPDATE hosts SET keep_alive_interval=?1 WHERE id=?2",
                rusqlite::params![value, id],
            )?;
        }
        if let Some(value) = request.keep_alive_tries {
            connection.execute(
                "UPDATE hosts SET keep_alive_tries=?1 WHERE id=?2",
                rusqlite::params![value, id],
            )?;
        }
        if let Some(value) = request.sort_order {
            connection.execute(
                "UPDATE hosts SET sort_order=?1 WHERE id=?2",
                rusqlite::params![value, id],
            )?;
        }
        if request.tunnels.is_some() {
            connection.execute(
                "UPDATE hosts SET tunnels=?1 WHERE id=?2",
                rusqlite::params![request.tunnels, id],
            )?;
        }
        if let Some(value) = &request.startup_snippet_id {
            let value = (!value.is_empty()).then_some(value.as_str());
            connection.execute(
                "UPDATE hosts SET startup_snippet_id=?1 WHERE id=?2",
                rusqlite::params![value, id],
            )?;
        }
        if request.startup_snippet_mode.is_some() {
            connection.execute(
                "UPDATE hosts SET startup_snippet_mode=?1 WHERE id=?2",
                rusqlite::params![request.startup_snippet_mode, id],
            )?;
        }
        if request.credential_id.is_some() {
            let value = request.credential_id.filter(|value| !value.is_empty());
            connection.execute(
                "UPDATE hosts SET credential_id=?1 WHERE id=?2",
                rusqlite::params![value, id],
            )?;
        }
        if request.jump_host_id.is_some() {
            let value = request.jump_host_id.filter(|value| !value.is_empty());
            connection.execute(
                "UPDATE hosts SET jump_host_id=?1 WHERE id=?2",
                rusqlite::params![value, id],
            )?;
        }
        if request.notes.is_some() {
            connection.execute(
                "UPDATE hosts SET notes=?1 WHERE id=?2",
                rusqlite::params![request.notes, id],
            )?;
        }
        if request.icon.is_some() {
            let value = request.icon.filter(|value| !value.is_empty());
            connection.execute(
                "UPDATE hosts SET icon=?1 WHERE id=?2",
                rusqlite::params![value, id],
            )?;
        }
        if let Some(value) = request.block_agent_access {
            connection.execute(
                "UPDATE hosts SET block_agent_access=?1 WHERE id=?2",
                rusqlite::params![value as i64, id],
            )?;
        }
    }

    if request.block_agent_access == Some(true) {
        if let Some(handler) = event_handler {
            handler(HostEvent::AgentAccessBlocked {
                host_id: id.clone(),
            })?;
        }
    }
    if let Some(password) = request.password {
        if password.is_empty() {
            let _ = delete_password(secrets, "labonair-app", &id);
        } else {
            store_password(secrets, "labonair-app", &id, &password)
                .map_err(LabonairError::Internal)?;
        }
    }
    if let Some(password) = request.sudo_password {
        if password.is_empty() {
            let _ = delete_password(secrets, "labonair-sudo", &id);
            let connection = database
                .0
                .lock()
                .map_err(|error| LabonairError::Internal(error.to_string()))?;
            let _ = connection.execute(
                "UPDATE hosts SET sudo_password_set=0 WHERE id=?1",
                rusqlite::params![id],
            );
        } else {
            store_password(secrets, "labonair-sudo", &id, &password)
                .map_err(LabonairError::Internal)?;
            let connection = database
                .0
                .lock()
                .map_err(|error| LabonairError::Internal(error.to_string()))?;
            let _ = connection.execute(
                "UPDATE hosts SET sudo_password_set=1 WHERE id=?1",
                rusqlite::params![id],
            );
        }
    }

    let connection = database
        .0
        .lock()
        .map_err(|error| LabonairError::Internal(error.to_string()))?;
    Ok(connection.query_row(
        &format!("{SELECT_HOSTS} WHERE id=?1"),
        rusqlite::params![id],
        row_to_host,
    )?)
}

/// Delete a host and its associated secret entries.
pub async fn hosts_delete(
    database: &Database,
    secrets: &SecretsState,
    id: String,
) -> Result<(), LabonairError> {
    {
        let connection = database
            .0
            .lock()
            .map_err(|error| LabonairError::Internal(error.to_string()))?;
        connection.execute("DELETE FROM hosts WHERE id=?1", rusqlite::params![id])?;
    }
    let _ = delete_password(secrets, "labonair-app", &id);
    let _ = delete_password(secrets, "labonair-sudo", &id);
    Ok(())
}

/// Resolve a host's sudo password only when its persisted marker is set.
pub async fn get_sudo_password(
    database: &Database,
    secrets: &SecretsState,
    host_id: String,
) -> Result<Option<String>, LabonairError> {
    let sudo_set = {
        let connection = database
            .0
            .lock()
            .map_err(|error| LabonairError::Internal(error.to_string()))?;
        connection
            .query_row(
                "SELECT sudo_password_set FROM hosts WHERE id=?1",
                rusqlite::params![host_id],
                |row| row.get::<_, i64>(0),
            )
            .map(|value| value != 0)
            .unwrap_or(false)
    };
    if !sudo_set {
        return Ok(None);
    }
    get_password(secrets, "labonair-sudo", &host_id).map_err(LabonairError::Internal)
}

#[cfg(test)]
mod tests {
    use super::{hosts_create, hosts_get_all, HostCreateRequest};
    use labonair_persistence::{initialize_database, Database};
    use labonair_secrets::{get_password, SecretsState};
    use std::sync::Mutex;

    #[tokio::test]
    async fn create_persists_host_and_keeps_secret_out_of_read_model() {
        let data_dir =
            std::env::temp_dir().join(format!("labonair-host-store-{}", uuid::Uuid::new_v4()));
        let connection = initialize_database(data_dir.clone()).expect("database initializes");
        let database = Database(std::sync::Arc::new(Mutex::new(connection)));
        let secrets = SecretsState::new(data_dir.clone());

        let host = hosts_create(
            &database,
            &secrets,
            HostCreateRequest {
                name: "Build host".to_string(),
                host_address: "build.example.com".to_string(),
                port: 22,
                username: "builder".to_string(),
                auth_method: "password".to_string(),
                private_key_path: None,
                group_id: None,
                tags: None,
                password: Some("secret".to_string()),
                sudo_password: None,
                default_path_ssh: None,
                default_path_sftp: None,
                pin_to_top: None,
                keep_alive_interval: None,
                keep_alive_tries: None,
                sort_order: None,
                tunnels: None,
                startup_snippet_id: None,
                startup_snippet_mode: None,
                credential_id: None,
                jump_host_id: None,
                notes: None,
                icon: None,
                block_agent_access: None,
            },
        )
        .await
        .expect("host creates");

        assert_eq!(host.name, "Build host");
        assert_eq!(
            get_password(&secrets, "labonair-app", &host.id).unwrap(),
            Some("secret".to_string())
        );
        assert_eq!(hosts_get_all(&database).await.unwrap(), vec![host]);

        std::fs::remove_dir_all(data_dir).expect("cleanup host store test");
    }
}
