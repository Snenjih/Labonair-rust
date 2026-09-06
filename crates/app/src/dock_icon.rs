//! macOS Dock / cmd-tab icon.
//!
//! GPUI 0.2.2 exposes no runtime app-icon API, and this project runs
//! un-bundled (`cargo run`), so the Dock and app switcher would otherwise show
//! the generic executable icon. [`set_dock_icon`] loads the vendored PNG
//! (copied verbatim from the reference app's `src-tauri/icons/`) into an
//! `NSImage` and assigns it to `NSApplication.applicationIconImage` once, at
//! startup.
//!
//! A packaged `.app` build gets the same artwork from the embedded
//! `icon.icns` (see `[package.metadata.bundle]` in `Cargo.toml`); running this
//! there simply re-sets an identical image and is harmless.

/// The vendored master icon (512×512, `reference-src/src-tauri/icons/256x256@2x.png`).
#[cfg(target_os = "macos")]
const ICON_PNG: &[u8] = include_bytes!("../assets/app-icon/256x256@2x.png");

/// Set the running application's Dock / cmd-tab icon.
///
/// macOS only; a no-op on other platforms. Must be called on the main thread
/// (GPUI's `Application::run` closure is).
#[cfg(target_os = "macos")]
pub fn set_dock_icon() {
    use objc2::MainThreadMarker;
    use objc2_app_kit::{NSApplication, NSImage};
    use objc2_foundation::NSData;

    let Some(mtm) = MainThreadMarker::new() else {
        tracing::warn!("set_dock_icon called off the main thread; skipping");
        return;
    };

    let data = NSData::with_bytes(ICON_PNG);
    let Some(image) = NSImage::initWithData(mtm.alloc::<NSImage>(), &data) else {
        tracing::warn!("bundled app-icon PNG failed to decode; keeping default icon");
        return;
    };

    let app = NSApplication::sharedApplication(mtm);
    // SAFETY: `image` is a freshly-initialised, non-nil `NSImage`; AppKit
    // retains its own reference. No other invariants apply to this setter.
    unsafe { app.setApplicationIconImage(Some(&image)) };
}

/// Non-macOS no-op — the icon comes from the platform's own packaging.
#[cfg(not(target_os = "macos"))]
pub fn set_dock_icon() {}
