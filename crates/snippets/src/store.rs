//! SQLite-backed snippet and snippet-group store.

use crate::{CommandSnippet, SnippetGroup, SnippetReorderItem};
use labonair_errors::LabonairError;
use labonair_persistence::Database;

const SELECT_SNIPPETS: &str =
    "SELECT id, name, description, command, target, host_id, default_exec_mode, \
     working_dir, group_id, tags, sort_order, created_at, updated_at FROM snippets";

fn now_millis() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}

fn row_to_snippet(row: &rusqlite::Row<'_>) -> rusqlite::Result<CommandSnippet> {
    Ok(CommandSnippet {
        id: row.get(0)?,
        name: row.get(1)?,
        description: row.get(2)?,
        command: row.get(3)?,
        target: row.get(4)?,
        host_id: row.get(5)?,
        default_exec_mode: row.get(6)?,
        working_dir: row.get(7)?,
        group_id: row.get(8)?,
        tags: row.get(9)?,
        sort_order: row.get(10).unwrap_or(0),
        created_at: row.get(11)?,
        updated_at: row.get(12)?,
    })
}

fn row_to_group(row: &rusqlite::Row<'_>) -> rusqlite::Result<SnippetGroup> {
    Ok(SnippetGroup {
        id: row.get(0)?,
        name: row.get(1)?,
        icon: row.get(2)?,
        color: row.get(3)?,
        sort_order: row.get(4).unwrap_or(0),
        created_at: row.get(5)?,
    })
}

pub async fn snippets_get_all(database: &Database) -> Result<Vec<CommandSnippet>, LabonairError> {
    let connection = database
        .0
        .lock()
        .map_err(|error| LabonairError::Internal(error.to_string()))?;
    let mut statement = connection.prepare(&format!(
        "{SELECT_SNIPPETS} ORDER BY sort_order ASC, name ASC"
    ))?;
    let snippets = statement
        .query_map([], row_to_snippet)?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(snippets)
}

#[allow(clippy::too_many_arguments)]
pub async fn snippets_create(
    database: &Database,
    name: String,
    command: String,
    target: String,
    description: Option<String>,
    host_id: Option<String>,
    default_exec_mode: Option<String>,
    working_dir: Option<String>,
    group_id: Option<String>,
    tags: Option<String>,
    sort_order: Option<i64>,
) -> Result<CommandSnippet, LabonairError> {
    let id = uuid::Uuid::new_v4().to_string();
    let now = now_millis();
    let exec_mode = default_exec_mode.unwrap_or_else(|| "terminal".to_string());
    let order = sort_order.unwrap_or(0);
    let connection = database
        .0
        .lock()
        .map_err(|error| LabonairError::Internal(error.to_string()))?;
    connection.execute(
        "INSERT INTO snippets (id, name, description, command, target, host_id, \
         default_exec_mode, working_dir, group_id, tags, sort_order, created_at, updated_at) \
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13)",
        rusqlite::params![
            id,
            name,
            description,
            command,
            target,
            host_id,
            exec_mode,
            working_dir,
            group_id,
            tags,
            order,
            now,
            now
        ],
    )?;
    Ok(connection.query_row(
        &format!("{SELECT_SNIPPETS} WHERE id=?1"),
        rusqlite::params![id],
        row_to_snippet,
    )?)
}

#[allow(clippy::too_many_arguments)]
pub async fn snippets_update(
    database: &Database,
    id: String,
    name: Option<String>,
    command: Option<String>,
    target: Option<String>,
    description: Option<String>,
    host_id: Option<String>,
    default_exec_mode: Option<String>,
    working_dir: Option<String>,
    group_id: Option<String>,
    tags: Option<String>,
    sort_order: Option<i64>,
) -> Result<CommandSnippet, LabonairError> {
    let now = now_millis();
    let connection = database
        .0
        .lock()
        .map_err(|error| LabonairError::Internal(error.to_string()))?;

    macro_rules! maybe_update {
        ($value:expr, $column:literal) => {
            if let Some(value) = $value {
                connection.execute(
                    concat!("UPDATE snippets SET ", $column, "=?1 WHERE id=?2"),
                    rusqlite::params![value, id],
                )?;
            }
        };
    }
    maybe_update!(name, "name");
    maybe_update!(command, "command");
    maybe_update!(target, "target");
    maybe_update!(default_exec_mode, "default_exec_mode");
    if description.is_some() {
        connection.execute(
            "UPDATE snippets SET description=?1 WHERE id=?2",
            rusqlite::params![description, id],
        )?;
    }
    if host_id.is_some() {
        connection.execute(
            "UPDATE snippets SET host_id=?1 WHERE id=?2",
            rusqlite::params![host_id, id],
        )?;
    }
    if working_dir.is_some() {
        connection.execute(
            "UPDATE snippets SET working_dir=?1 WHERE id=?2",
            rusqlite::params![working_dir, id],
        )?;
    }
    if group_id.is_some() {
        connection.execute(
            "UPDATE snippets SET group_id=?1 WHERE id=?2",
            rusqlite::params![group_id, id],
        )?;
    }
    if tags.is_some() {
        connection.execute(
            "UPDATE snippets SET tags=?1 WHERE id=?2",
            rusqlite::params![tags, id],
        )?;
    }
    if let Some(value) = sort_order {
        connection.execute(
            "UPDATE snippets SET sort_order=?1 WHERE id=?2",
            rusqlite::params![value, id],
        )?;
    }
    connection.execute(
        "UPDATE snippets SET updated_at=?1 WHERE id=?2",
        rusqlite::params![now, id],
    )?;
    Ok(connection.query_row(
        &format!("{SELECT_SNIPPETS} WHERE id=?1"),
        rusqlite::params![id],
        row_to_snippet,
    )?)
}

pub async fn snippets_delete(database: &Database, id: String) -> Result<(), LabonairError> {
    let connection = database
        .0
        .lock()
        .map_err(|error| LabonairError::Internal(error.to_string()))?;
    connection.execute("DELETE FROM snippets WHERE id=?1", rusqlite::params![id])?;
    Ok(())
}

pub async fn snippets_reorder(
    database: &Database,
    items: Vec<SnippetReorderItem>,
) -> Result<(), LabonairError> {
    let connection = database
        .0
        .lock()
        .map_err(|error| LabonairError::Internal(error.to_string()))?;
    for item in items {
        connection.execute(
            "UPDATE snippets SET sort_order=?1 WHERE id=?2",
            rusqlite::params![item.sort_order, item.id],
        )?;
    }
    Ok(())
}

pub async fn snippet_groups_get_all(
    database: &Database,
) -> Result<Vec<SnippetGroup>, LabonairError> {
    let connection = database
        .0
        .lock()
        .map_err(|error| LabonairError::Internal(error.to_string()))?;
    let mut statement = connection.prepare(
        "SELECT id, name, icon, color, sort_order, created_at FROM snippet_groups ORDER BY sort_order ASC, name ASC",
    )?;
    let groups = statement
        .query_map([], row_to_group)?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(groups)
}

pub async fn snippet_groups_create(
    database: &Database,
    name: String,
    icon: Option<String>,
    color: Option<String>,
) -> Result<SnippetGroup, LabonairError> {
    let id = uuid::Uuid::new_v4().to_string();
    let created_at = now_millis();
    let connection = database
        .0
        .lock()
        .map_err(|error| LabonairError::Internal(error.to_string()))?;
    connection.execute(
        "INSERT INTO snippet_groups (id, name, icon, color, sort_order, created_at) VALUES (?1,?2,?3,?4,0,?5)",
        rusqlite::params![id, name, icon, color, created_at],
    )?;
    Ok(SnippetGroup {
        id,
        name,
        icon,
        color,
        sort_order: 0,
        created_at,
    })
}

pub async fn snippet_groups_update(
    database: &Database,
    id: String,
    name: Option<String>,
    icon: Option<String>,
    color: Option<String>,
) -> Result<SnippetGroup, LabonairError> {
    let connection = database
        .0
        .lock()
        .map_err(|error| LabonairError::Internal(error.to_string()))?;
    if let Some(value) = name {
        connection.execute(
            "UPDATE snippet_groups SET name=?1 WHERE id=?2",
            rusqlite::params![value, id],
        )?;
    }
    if icon.is_some() {
        connection.execute(
            "UPDATE snippet_groups SET icon=?1 WHERE id=?2",
            rusqlite::params![icon, id],
        )?;
    }
    if color.is_some() {
        connection.execute(
            "UPDATE snippet_groups SET color=?1 WHERE id=?2",
            rusqlite::params![color, id],
        )?;
    }
    Ok(connection.query_row(
        "SELECT id, name, icon, color, sort_order, created_at FROM snippet_groups WHERE id=?1",
        rusqlite::params![id],
        row_to_group,
    )?)
}

pub async fn snippet_groups_delete(database: &Database, id: String) -> Result<(), LabonairError> {
    let connection = database
        .0
        .lock()
        .map_err(|error| LabonairError::Internal(error.to_string()))?;
    connection.execute(
        "UPDATE snippets SET group_id=NULL WHERE group_id=?1",
        rusqlite::params![id],
    )?;
    connection.execute(
        "DELETE FROM snippet_groups WHERE id=?1",
        rusqlite::params![id],
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use labonair_persistence::{initialize_database, Database};
    use std::sync::Mutex;

    #[tokio::test]
    async fn snippet_crud_round_trips_through_store() {
        let data_dir =
            std::env::temp_dir().join(format!("labonair-snippets-{}", uuid::Uuid::new_v4()));
        let connection = initialize_database(data_dir.clone()).expect("database initializes");
        let database = Database(Mutex::new(connection));
        let snippet = snippets_create(
            &database,
            "Deploy".to_string(),
            "cargo build".to_string(),
            "local".to_string(),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        )
        .await
        .expect("snippet creates");
        assert_eq!(
            snippets_get_all(&database).await.unwrap(),
            vec![snippet.clone()]
        );
        snippets_delete(&database, snippet.id)
            .await
            .expect("snippet deletes");
        assert!(snippets_get_all(&database).await.unwrap().is_empty());
        std::fs::remove_dir_all(data_dir).expect("cleanup snippet test");
    }
}
