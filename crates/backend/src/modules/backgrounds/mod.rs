use base64::Engine;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

const ALLOWED_EXTENSIONS: &[&str] = &["jpg", "jpeg", "png", "gif", "webp", "avif", "bmp"];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackgroundInfo {
    pub filename: String,
    pub path: String,
    pub size_bytes: u64,
}

pub fn backgrounds_dir() -> Result<PathBuf, String> {
    let dir = crate::modules::fs::paths::config_dir().join("backgrounds");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir)
}

fn is_allowed_extension(ext: &str) -> bool {
    ALLOWED_EXTENSIONS.contains(&ext.to_lowercase().as_str())
}

pub fn backgrounds_list() -> Result<Vec<BackgroundInfo>, String> {
    let dir = backgrounds_dir()?;
    let mut items = Vec::new();

    let entries = match std::fs::read_dir(&dir) {
        Ok(e) => e,
        Err(_) => return Ok(vec![]),
    };

    for entry in entries.flatten() {
        let path = entry.path();
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        if !is_allowed_extension(ext) {
            continue;
        }
        let filename = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_string();
        let size_bytes = entry.metadata().map(|m| m.len()).unwrap_or(0);
        items.push(BackgroundInfo {
            filename,
            path: path.to_string_lossy().to_string(),
            size_bytes,
        });
    }

    items.sort_by(|a, b| a.filename.cmp(&b.filename));
    Ok(items)
}

pub fn background_import(source_path: String) -> Result<BackgroundInfo, String> {
    let source = std::path::Path::new(&source_path);

    let ext = source.extension().and_then(|e| e.to_str()).unwrap_or("");
    if !is_allowed_extension(ext) {
        return Err(format!("Unsupported image format: {}", ext));
    }

    // Only keep the filename, strip any path components (security)
    let raw_name = source
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or("Invalid source filename")?;

    let dir = backgrounds_dir()?;
    let dest = dir.join(raw_name);

    // Resolve collision: insert _<timestamp_ms> before extension
    let dest = if dest.exists() {
        let stem = source
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("background");
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0);
        let new_name = format!("{}_{}.{}", stem, ts, ext.to_lowercase());
        dir.join(new_name)
    } else {
        dest
    };

    std::fs::copy(&source_path, &dest).map_err(|e| e.to_string())?;

    let filename = dest
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("")
        .to_string();
    let size_bytes = std::fs::metadata(&dest).map(|m| m.len()).unwrap_or(0);

    Ok(BackgroundInfo {
        filename,
        path: dest.to_string_lossy().to_string(),
        size_bytes,
    })
}

/// Read an image from the backgrounds directory and return it as a base64 data URL.
/// This bypasses the asset protocol entirely — no scope config needed.
pub fn background_read_data_url(filename: String) -> Result<String, String> {
    if filename.contains('/') || filename.contains('\\') {
        return Err("Invalid filename".to_string());
    }

    let dir = backgrounds_dir()?;
    let path = dir.join(&filename);

    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("jpeg")
        .to_lowercase();

    let mime = match ext.as_str() {
        "jpg" | "jpeg" => "image/jpeg",
        "png" => "image/png",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "avif" => "image/avif",
        "bmp" => "image/bmp",
        _ => "image/jpeg",
    };

    let bytes = std::fs::read(&path).map_err(|e| e.to_string())?;
    let b64 = base64::engine::general_purpose::STANDARD.encode(&bytes);
    Ok(format!("data:{};base64,{}", mime, b64))
}

pub fn background_delete(filename: String) -> Result<(), String> {
    // Path traversal guard
    if filename.contains('/') || filename.contains('\\') {
        return Err("Invalid filename".to_string());
    }

    let dir = backgrounds_dir()?;
    let path = dir.join(&filename);

    // Ensure the resolved path is still inside backgrounds_dir
    let canonical_dir = dir.canonicalize().map_err(|e| e.to_string())?;
    let canonical_path = path
        .canonicalize()
        .map_err(|_| "File not found".to_string())?;
    if !canonical_path.starts_with(&canonical_dir) {
        return Err("Invalid filename".to_string());
    }

    std::fs::remove_file(&canonical_path).map_err(|e| e.to_string())?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Background rendering preferences (T02-006)
//
// Ported from the reference `preferencesStore` background keys
// (`src/modules/settings/store.ts`): `backgroundImage`, `backgroundOpacity`,
// `backgroundBlur`, `backgroundTintColor`, `backgroundTintOpacity`. Two extra
// keys the pure-Rust renderer needs — `backgroundFit` and `backgroundTarget` —
// default to the reference's implicit behaviour (cover, whole window).
//
// Persisted into the same `config_dir()/labonair-settings.json` blob the rest
// of the app reads (see `super::settings`), merged key-by-key so unrelated
// settings survive.
// ---------------------------------------------------------------------------

use std::path::Path;

const SETTINGS_FILE: &str = "labonair-settings.json";

/// How the background image is scaled into its area.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum BackgroundFit {
    /// Scale to cover the whole area, cropping overflow (reference default).
    #[default]
    Cover,
    /// Scale so the entire image is visible, letterboxing if needed.
    Contain,
    /// Repeat the image at its native size.
    Tile,
}

/// Which surface(s) the background image sits behind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum BackgroundTarget {
    /// Whole app window (reference behaviour — the overlay always spanned everything).
    #[default]
    Both,
    /// Only the app chrome, not the terminal surface.
    App,
    /// Only the terminal surface.
    Terminal,
}

/// User-configurable background-image rendering preferences.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackgroundSettings {
    /// Filename inside `backgrounds_dir()`, or empty for "no background".
    pub background_image: String,
    /// Slider value 0..=100. Rendered opacity is halved on top of this.
    pub background_opacity: u8,
    /// Gaussian blur radius in pixels, 0..=100. Applied once at load time.
    pub background_blur: u8,
    /// Tint overlay color (`#rrggbb`).
    pub background_tint_color: String,
    /// Tint overlay opacity 0..=100.
    pub background_tint_opacity: u8,
    /// Image scaling mode.
    pub background_fit: BackgroundFit,
    /// Which surface(s) the image sits behind.
    pub background_target: BackgroundTarget,
}

impl Default for BackgroundSettings {
    fn default() -> Self {
        Self {
            background_image: String::new(),
            background_opacity: 30,
            background_blur: 0,
            background_tint_color: "#000000".to_string(),
            background_tint_opacity: 0,
            background_fit: BackgroundFit::default(),
            background_target: BackgroundTarget::default(),
        }
    }
}

fn read_settings_map(dir: &Path) -> serde_json::Map<String, serde_json::Value> {
    std::fs::read_to_string(dir.join(SETTINGS_FILE))
        .ok()
        .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
        .and_then(|v| v.as_object().cloned())
        .unwrap_or_default()
}

fn load_from(dir: &Path) -> BackgroundSettings {
    let map = read_settings_map(dir);
    let def = BackgroundSettings::default();

    let str_key = |k: &str, fallback: String| {
        map.get(k)
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .unwrap_or(fallback)
    };
    let num_key = |k: &str, fallback: u8| {
        map.get(k)
            .and_then(|v| v.as_u64())
            .map(|n| n.min(100) as u8)
            .unwrap_or(fallback)
    };
    fn enum_key<T: serde::de::DeserializeOwned>(
        map: &serde_json::Map<String, serde_json::Value>,
        k: &str,
    ) -> Option<T> {
        map.get(k)
            .cloned()
            .and_then(|v| serde_json::from_value(v).ok())
    }

    BackgroundSettings {
        background_image: str_key("backgroundImage", def.background_image),
        background_opacity: num_key("backgroundOpacity", def.background_opacity),
        background_blur: num_key("backgroundBlur", def.background_blur),
        background_tint_color: str_key("backgroundTintColor", def.background_tint_color),
        background_tint_opacity: num_key("backgroundTintOpacity", def.background_tint_opacity),
        background_fit: enum_key(&map, "backgroundFit").unwrap_or(def.background_fit),
        background_target: enum_key(&map, "backgroundTarget").unwrap_or(def.background_target),
    }
}

fn save_to(dir: &Path, settings: &BackgroundSettings) -> Result<(), String> {
    let mut map = read_settings_map(dir);
    let insert =
        |map: &mut serde_json::Map<String, serde_json::Value>, k: &str, v: serde_json::Value| {
            map.insert(k.to_string(), v);
        };
    insert(
        &mut map,
        "backgroundImage",
        settings.background_image.clone().into(),
    );
    insert(
        &mut map,
        "backgroundOpacity",
        settings.background_opacity.into(),
    );
    insert(&mut map, "backgroundBlur", settings.background_blur.into());
    insert(
        &mut map,
        "backgroundTintColor",
        settings.background_tint_color.clone().into(),
    );
    insert(
        &mut map,
        "backgroundTintOpacity",
        settings.background_tint_opacity.into(),
    );
    insert(
        &mut map,
        "backgroundFit",
        serde_json::to_value(settings.background_fit).map_err(|e| e.to_string())?,
    );
    insert(
        &mut map,
        "backgroundTarget",
        serde_json::to_value(settings.background_target).map_err(|e| e.to_string())?,
    );

    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let path = dir.join(SETTINGS_FILE);
    let tmp = path.with_extension("json.tmp");
    let json = serde_json::to_string_pretty(&map).map_err(|e| e.to_string())?;
    std::fs::write(&tmp, json).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, &path).map_err(|e| e.to_string())
}

/// Loads the persisted background preferences (defaults if none saved yet).
pub fn background_settings_load() -> BackgroundSettings {
    load_from(&crate::modules::fs::paths::config_dir())
}

/// Persists the background preferences, merging into the shared settings file.
pub fn background_settings_save(settings: &BackgroundSettings) -> Result<(), String> {
    save_to(&crate::modules::fs::paths::config_dir(), settings)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir() -> PathBuf {
        let dir = std::env::temp_dir().join(format!("labonair-bg-test-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn defaults_match_reference_preferences() {
        let d = BackgroundSettings::default();
        assert_eq!(d.background_image, "");
        assert_eq!(d.background_opacity, 30);
        assert_eq!(d.background_blur, 0);
        assert_eq!(d.background_tint_color, "#000000");
        assert_eq!(d.background_tint_opacity, 0);
        assert_eq!(d.background_fit, BackgroundFit::Cover);
        assert_eq!(d.background_target, BackgroundTarget::Both);
    }

    #[test]
    fn load_returns_defaults_when_file_missing() {
        let dir = temp_dir();
        assert_eq!(load_from(&dir), BackgroundSettings::default());
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn save_then_load_round_trips_and_preserves_other_keys() {
        let dir = temp_dir();
        std::fs::write(
            dir.join(SETTINGS_FILE),
            r#"{"barItemPlacements":{"x":1},"backgroundOpacity":99}"#,
        )
        .unwrap();

        let settings = BackgroundSettings {
            background_image: "wall.png".to_string(),
            background_opacity: 45,
            background_blur: 8,
            background_tint_color: "#112233".to_string(),
            background_tint_opacity: 20,
            background_fit: BackgroundFit::Contain,
            background_target: BackgroundTarget::Terminal,
        };
        save_to(&dir, &settings).unwrap();

        assert_eq!(load_from(&dir), settings);

        let raw: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(dir.join(SETTINGS_FILE)).unwrap())
                .unwrap();
        assert_eq!(raw["barItemPlacements"]["x"], 1);
        assert_eq!(raw["backgroundFit"], "contain");
        assert_eq!(raw["backgroundTarget"], "terminal");

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn out_of_range_numbers_are_clamped_on_load() {
        let dir = temp_dir();
        std::fs::write(
            dir.join(SETTINGS_FILE),
            r#"{"backgroundOpacity":5000,"backgroundBlur":300}"#,
        )
        .unwrap();
        let s = load_from(&dir);
        assert_eq!(s.background_opacity, 100);
        assert_eq!(s.background_blur, 100);
        std::fs::remove_dir_all(&dir).ok();
    }
}
