//! Compatibility adapters for the capability-owned host store.
//!
//! New host persistence belongs to `labonair-hosts::store`. This module keeps
//! the old backend function signatures temporarily so the current UI and
//! transport code can migrate independently. The only backend-specific work
//! retained here is translating the host event into MCP grant revocation.

use super::{Host, HostsDb};
use crate::modules::errors::LabonairError;
use crate::modules::secrets::SecretsState;
use labonair_hosts::store::{HostCreateRequest, HostEvent, HostEventHandler, HostUpdateRequest};

pub use labonair_hosts::store::{
    groups_create, groups_delete, groups_get_all, groups_update, hosts_get_all, hosts_reorder,
};
pub use labonair_persistence::initialize_database as initialize_db;

pub fn revoke_agent_access(app: &crate::App, event: HostEvent) -> Result<(), LabonairError> {
    let HostEvent::AgentAccessBlocked { host_id } = event;
    let expired: Vec<String> = {
        let grants = app
            .mcp
            .grants
            .lock()
            .map_err(|error| LabonairError::Internal(error.to_string()))?;
        grants
            .values()
            .filter(|grant| grant.host_id.as_deref() == Some(host_id.as_str()))
            .map(|grant| grant.tab_id.clone())
            .collect()
    };
    if expired.is_empty() {
        return Ok(());
    }

    let mut grants = app
        .mcp
        .grants
        .lock()
        .map_err(|error| LabonairError::Internal(error.to_string()))?;
    for tab_id in &expired {
        grants.remove(tab_id);
    }
    drop(grants);

    for tab_id in expired {
        app.emit_event(crate::AppEvent::McpGrantExpired { tab_id })
            .map_err(LabonairError::Internal)?;
    }
    Ok(())
}

pub async fn hosts_duplicate(
    _app: crate::App,
    db: &HostsDb,
    secrets: &SecretsState,
    id: String,
) -> Result<Host, LabonairError> {
    labonair_hosts::store::hosts_duplicate(db, secrets, id).await
}

#[allow(clippy::too_many_arguments)]
pub async fn hosts_create(
    _app: crate::App,
    db: &HostsDb,
    secrets: &SecretsState,
    name: String,
    host_address: String,
    port: i64,
    username: String,
    auth_method: String,
    private_key_path: Option<String>,
    group_id: Option<String>,
    tags: Option<String>,
    password: Option<String>,
    sudo_password: Option<String>,
    default_path_ssh: Option<String>,
    default_path_sftp: Option<String>,
    pin_to_top: Option<bool>,
    keep_alive_interval: Option<i64>,
    keep_alive_tries: Option<i64>,
    sort_order: Option<i64>,
    tunnels: Option<String>,
    startup_snippet_id: Option<String>,
    startup_snippet_mode: Option<String>,
    credential_id: Option<String>,
    jump_host_id: Option<String>,
    notes: Option<String>,
    icon: Option<String>,
    block_agent_access: Option<bool>,
) -> Result<Host, LabonairError> {
    labonair_hosts::store::hosts_create(
        db,
        secrets,
        HostCreateRequest {
            name,
            host_address,
            port,
            username,
            auth_method,
            private_key_path,
            group_id,
            tags,
            password,
            sudo_password,
            default_path_ssh,
            default_path_sftp,
            pin_to_top,
            keep_alive_interval,
            keep_alive_tries,
            sort_order,
            tunnels,
            startup_snippet_id,
            startup_snippet_mode,
            credential_id,
            jump_host_id,
            notes,
            icon,
            block_agent_access,
        },
    )
    .await
}

#[allow(clippy::too_many_arguments)]
pub async fn hosts_update(
    app: crate::App,
    db: &HostsDb,
    secrets: &SecretsState,
    id: String,
    name: Option<String>,
    host_address: Option<String>,
    port: Option<i64>,
    username: Option<String>,
    auth_method: Option<String>,
    private_key_path: Option<String>,
    group_id: Option<String>,
    tags: Option<String>,
    password: Option<String>,
    sudo_password: Option<String>,
    default_path_ssh: Option<String>,
    default_path_sftp: Option<String>,
    pin_to_top: Option<bool>,
    keep_alive_interval: Option<i64>,
    keep_alive_tries: Option<i64>,
    sort_order: Option<i64>,
    tunnels: Option<String>,
    startup_snippet_id: Option<String>,
    startup_snippet_mode: Option<String>,
    credential_id: Option<String>,
    jump_host_id: Option<String>,
    notes: Option<String>,
    icon: Option<String>,
    block_agent_access: Option<bool>,
) -> Result<Host, LabonairError> {
    let handler: &HostEventHandler<'_> = &|event| revoke_agent_access(&app, event);
    labonair_hosts::store::hosts_update(
        db,
        secrets,
        HostUpdateRequest {
            id,
            name,
            host_address,
            port,
            username,
            auth_method,
            private_key_path,
            group_id,
            tags,
            password,
            sudo_password,
            default_path_ssh,
            default_path_sftp,
            pin_to_top,
            keep_alive_interval,
            keep_alive_tries,
            sort_order,
            tunnels,
            startup_snippet_id,
            startup_snippet_mode,
            credential_id,
            jump_host_id,
            notes,
            icon,
            block_agent_access,
        },
        Some(handler),
    )
    .await
}

pub async fn hosts_delete(
    _app: crate::App,
    db: &HostsDb,
    secrets: &SecretsState,
    id: String,
) -> Result<(), LabonairError> {
    labonair_hosts::store::hosts_delete(db, secrets, id).await
}

pub async fn get_sudo_password(
    _app: crate::App,
    db: &HostsDb,
    secrets: &SecretsState,
    host_id: String,
) -> Result<Option<String>, LabonairError> {
    labonair_hosts::store::get_sudo_password(db, secrets, host_id).await
}
