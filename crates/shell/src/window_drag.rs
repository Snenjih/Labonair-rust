//! Window move from the custom titlebar.
//!
//! GPUI 0.2.2's macOS `start_window_move` is a no-op, so the window is opened
//! with `is_movable: false` (no native titlebar drag that grabs the whole
//! window through a dragged tab) and a drag on the empty titlebar background
//! starts the move explicitly via `performWindowDragWithEvent:`.

use gpui::Window;

#[cfg(target_os = "macos")]
pub fn start(_window: &Window) {
    use objc2::MainThreadMarker;
    use objc2_app_kit::NSApplication;

    let Some(mtm) = MainThreadMarker::new() else {
        return;
    };
    let app = NSApplication::sharedApplication(mtm);
    let (Some(ns_window), Some(event)) = (app.keyWindow(), app.currentEvent()) else {
        return;
    };
    // `isMovable == false` also blocks the programmatic drag, so lift it for
    // the (synchronous) drag loop only.
    ns_window.setMovable(true);
    ns_window.performWindowDragWithEvent(&event);
    ns_window.setMovable(false);
}

#[cfg(not(target_os = "macos"))]
pub fn start(window: &Window) {
    window.start_window_move();
}
