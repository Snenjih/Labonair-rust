//! T01-002 acceptance: `AppState` can be initialized from a data directory.

use labonair_backend::AppState;

#[test]
fn app_state_initializes_from_a_data_dir() {
    let dir = std::env::temp_dir().join(format!("labonair-test-{}", uuid::Uuid::new_v4()));
    let app = AppState::new(&dir).expect("AppState::new should succeed");

    assert!(
        dir.join("labonair.db").exists(),
        "labonair.db should be created"
    );

    app.emit("test:event", serde_json::json!({ "ok": true }))
        .expect("emit should succeed");

    std::fs::remove_dir_all(&dir).ok();
}
