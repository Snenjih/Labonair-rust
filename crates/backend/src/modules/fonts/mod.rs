use base64::Engine;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::path::PathBuf;
use std::sync::OnceLock;
use std::time::{SystemTime, UNIX_EPOCH};

const ALLOWED_EXTENSIONS: &[&str] = &["ttf", "otf", "woff", "woff2"];
const MAX_LABEL_LEN: usize = 80;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomFontInfo {
    pub filename: String,
    pub label: String,
    pub path: String,
    pub size_bytes: u64,
    pub imported_at_ms: u64,
}

fn fonts_dir() -> Result<PathBuf, String> {
    let dir = crate::modules::fs::paths::config_dir().join("fonts");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir)
}

fn manifest_path() -> Result<PathBuf, String> {
    Ok(fonts_dir()?.join("manifest.json"))
}

fn is_allowed_extension(ext: &str) -> bool {
    ALLOWED_EXTENSIONS.contains(&ext.to_lowercase().as_str())
}

/// Sniffs the first bytes of a font file against the known magic-byte
/// signatures for each allowed format — cheap, cheap enough to run before
/// ever copying the file, and rejects obviously-wrong files (e.g. a renamed
/// .txt) before they get persisted. Deliberately permissive within a format
/// family (e.g. also accepts 'true'/'typ1' sfnt variants for ttf) rather than
/// exhaustive — the frontend's FontFace.load() is the second, authoritative
/// validation layer for anything that slips past this.
fn sniff_font_signature(bytes: &[u8], ext: &str) -> bool {
    if bytes.len() < 4 {
        return false;
    }
    let sig = &bytes[0..4];
    match ext.to_lowercase().as_str() {
        "ttf" => matches!(
            sig,
            [0x00, 0x01, 0x00, 0x00] | [b't', b'r', b'u', b'e'] | [b't', b'y', b'p', b'1']
        ),
        "otf" => sig == *b"OTTO",
        "woff" => sig == *b"wOFF",
        "woff2" => sig == *b"wOF2",
        _ => false,
    }
}

fn read_manifest() -> Vec<CustomFontInfo> {
    let path = match manifest_path() {
        Ok(p) => p,
        Err(_) => return vec![],
    };
    let raw = match std::fs::read_to_string(&path) {
        Ok(r) => r,
        Err(_) => return vec![],
    };
    serde_json::from_str(&raw).unwrap_or_default()
}

/// Writes the manifest via temp-file + rename so a crash mid-write can never
/// leave a torn/corrupt manifest that silently drops every custom font entry
/// even though the font files themselves are still on disk.
fn write_manifest(entries: &[CustomFontInfo]) -> Result<(), String> {
    let path = manifest_path()?;
    let tmp_path = path.with_extension("json.tmp");
    let json = serde_json::to_string_pretty(entries).map_err(|e| e.to_string())?;
    std::fs::write(&tmp_path, json).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp_path, &path).map_err(|e| e.to_string())?;
    Ok(())
}

pub async fn fonts_list_custom() -> Result<Vec<CustomFontInfo>, String> {
    let dir = fonts_dir()?;
    let mut entries: Vec<CustomFontInfo> = read_manifest()
        .into_iter()
        .filter(|f| dir.join(&f.filename).exists())
        .collect();
    entries.sort_by_key(|a| a.label.to_lowercase());
    Ok(entries)
}

pub async fn font_import(source_path: String, label: String) -> Result<CustomFontInfo, String> {
    let label = label.trim().to_string();
    if label.is_empty() {
        return Err("Font name cannot be empty".to_string());
    }
    if label.chars().count() > MAX_LABEL_LEN {
        return Err(format!(
            "Font name must be {} characters or fewer",
            MAX_LABEL_LEN
        ));
    }

    let source = std::path::Path::new(&source_path);
    let ext = source.extension().and_then(|e| e.to_str()).unwrap_or("");
    if !is_allowed_extension(ext) {
        return Err(format!("Unsupported font format: {}", ext));
    }

    let bytes = std::fs::read(&source_path).map_err(|e| e.to_string())?;
    if !sniff_font_signature(&bytes, ext) {
        return Err("File does not look like a valid font (signature mismatch)".to_string());
    }

    // Only keep the filename, strip any path components (security).
    let raw_name = source
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or("Invalid source filename")?;

    let dir = fonts_dir()?;
    let dest = dir.join(raw_name);

    // Resolve collision: insert _<timestamp_ms> before extension.
    let dest = if dest.exists() {
        let stem = source
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("font");
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0);
        dir.join(format!("{}_{}.{}", stem, ts, ext.to_lowercase()))
    } else {
        dest
    };

    std::fs::write(&dest, &bytes).map_err(|e| e.to_string())?;

    let filename = dest
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("")
        .to_string();
    let size_bytes = bytes.len() as u64;
    let imported_at_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);

    let info = CustomFontInfo {
        filename,
        label,
        path: dest.to_string_lossy().to_string(),
        size_bytes,
        imported_at_ms,
    };

    let mut manifest = read_manifest();
    manifest.push(info.clone());
    write_manifest(&manifest)?;

    Ok(info)
}

/// Reads a custom font file and returns it as a base64 data URL. Deliberately
/// mirrors `background_read_data_url` rather than serving bytes via the
/// asset:// protocol (`convertFileSrc`) — the Font Loading API's
/// `FontFace.load()` performs a CORS-checked fetch of its source URL, unlike
/// passive `<img>`/`<iframe src>` references, and asset:// URLs fail that
/// check in WKWebView with a generic "NetworkError". A data: URL sidesteps
/// the network fetch (and any CORS check) entirely.
pub async fn font_read_data_url(filename: String) -> Result<String, String> {
    if filename.contains('/') || filename.contains('\\') {
        return Err("Invalid filename".to_string());
    }

    let dir = fonts_dir()?;
    let path = dir.join(&filename);

    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    let mime = match ext.as_str() {
        "ttf" => "font/ttf",
        "otf" => "font/otf",
        "woff" => "font/woff",
        "woff2" => "font/woff2",
        _ => return Err(format!("Unsupported font format: {}", ext)),
    };

    let bytes = std::fs::read(&path).map_err(|e| e.to_string())?;
    let b64 = base64::engine::general_purpose::STANDARD.encode(&bytes);
    Ok(format!("data:{};base64,{}", mime, b64))
}

pub async fn font_delete(filename: String) -> Result<(), String> {
    // Path traversal guard.
    if filename.contains('/') || filename.contains('\\') {
        return Err("Invalid filename".to_string());
    }

    let dir = fonts_dir()?;
    let path = dir.join(&filename);

    if path.exists() {
        let canonical_dir = dir.canonicalize().map_err(|e| e.to_string())?;
        let canonical_path = path
            .canonicalize()
            .map_err(|_| "File not found".to_string())?;
        if !canonical_path.starts_with(&canonical_dir) {
            return Err("Invalid filename".to_string());
        }
        std::fs::remove_file(&canonical_path).map_err(|e| e.to_string())?;
    }

    // Drop the manifest entry regardless of whether the file itself was still
    // present — an entry pointing at a missing file must not linger forever.
    let manifest: Vec<CustomFontInfo> = read_manifest()
        .into_iter()
        .filter(|f| f.filename != filename)
        .collect();
    write_manifest(&manifest)?;

    Ok(())
}

static SYSTEM_FONTS_CACHE: OnceLock<Vec<String>> = OnceLock::new();

/// Synchronous, potentially expensive (hundreds of files) system font scan —
/// only ever call this from inside spawn_blocking, never directly on an
/// async command's executor thread.
fn scan_system_fonts_sync() -> Vec<String> {
    let mut db = fontdb::Database::new();
    db.load_system_fonts();

    let mut names: BTreeSet<String> = BTreeSet::new();
    for face in db.faces() {
        let Some((family, _lang)) = face.families.first() else {
            continue;
        };
        let trimmed = family.trim();
        if trimmed.is_empty() {
            continue;
        }
        // macOS marks private/internal-use font families with a leading '.'
        // (e.g. ".SF NS", ".AppleSystemUIFont") — not meant to be user-selectable.
        if trimmed.starts_with('.') {
            continue;
        }
        let lower = trimmed.to_lowercase();
        if matches!(
            lower.as_str(),
            "serif" | "sans-serif" | "monospace" | "cursive" | "fantasy" | "system-ui"
        ) {
            continue;
        }
        names.insert(trimmed.to_string());
    }
    names.into_iter().collect()
}

pub async fn fonts_list_system() -> Result<Vec<String>, String> {
    tokio::task::spawn_blocking(|| {
        SYSTEM_FONTS_CACHE
            .get_or_init(scan_system_fonts_sync)
            .clone()
    })
    .await
    .map_err(|e| e.to_string())
}
