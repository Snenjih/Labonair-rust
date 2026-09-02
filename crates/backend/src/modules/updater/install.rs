//! T15-005 — download, verify and apply an update.
//!
//! The reference app delegated this to `tauri-plugin-updater`: it downloaded the
//! artifact listed in `latest.json`, checked its **minisign** signature against
//! a compiled-in public key, unpacked the new `.app` over the running one and
//! relaunched. Tauri is gone, so this module reimplements that flow natively:
//!
//! * [`fetch_manifest`] — GET the `latest.json` manifest.
//! * [`download_update`] — stream the artifact with progress callbacks.
//! * [`verify_update`] — minisign (Ed25519, pre-hashed) signature check. Same
//!   format Tauri used, so the release signing key/tooling is unchanged.
//! * [`apply_macos_update`] — atomically swap the `.app` bundle.
//! * [`relaunch`] — re-open the freshly installed bundle and exit.
//!
//! The whole "download → verify → apply" chain refuses to touch anything on a
//! bad or missing signature (Critical Rule + task warning).

use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use base64::Engine as _;
use minisign_verify::{PublicKey, Signature};

use super::{AvailableUpdate, UpdateManifest};

/// Minisign public key the release artifacts are signed with (the single-line
/// base64 form — the second line of a `minisign.pub` file).
///
/// The matching secret key lives only in the release CI secrets
/// (`LABONAIR_UPDATER_PRIVATE_KEY` / `LABONAIR_UPDATER_KEY_PASSWORD`). Until the
/// first signed release this is the zeroed placeholder below; replace it with
/// the real key (`minisign -G`) at that point — [`verify_update`] rejects
/// everything while it is the placeholder, which is the safe default.
pub const UPDATE_PUBLIC_KEY: &str = "";

/// Auto-check cadence — identical to the reference `CHECK_INTERVAL_MS`
/// (6 hours).
pub const CHECK_INTERVAL: Duration = Duration::from_secs(6 * 60 * 60);

/// Progress of an in-flight download.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DownloadProgress {
    pub downloaded: u64,
    /// `None` when the server sends no `Content-Length`.
    pub total: Option<u64>,
}

/// Fetch and parse the `latest.json` update manifest.
pub async fn fetch_manifest(endpoint: &str) -> Result<UpdateManifest, String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(15))
        .build()
        .map_err(|e| e.to_string())?;
    let res = client
        .get(endpoint)
        .send()
        .await
        .map_err(|e| format!("update check failed: {e}"))?;
    if !res.status().is_success() {
        return Err(format!("update check failed: HTTP {}", res.status()));
    }
    let body = res.text().await.map_err(|e| e.to_string())?;
    UpdateManifest::parse(&body)
}

/// Download `url` into memory, invoking `on_progress` after every chunk.
pub async fn download_update(
    url: &str,
    mut on_progress: impl FnMut(DownloadProgress),
) -> Result<Vec<u8>, String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(300))
        .build()
        .map_err(|e| e.to_string())?;
    let mut res = client
        .get(url)
        .send()
        .await
        .map_err(|e| format!("download failed: {e}"))?;
    if !res.status().is_success() {
        return Err(format!("download failed: HTTP {}", res.status()));
    }
    let total = res.content_length();
    let mut buf = Vec::with_capacity(total.unwrap_or(0) as usize);
    on_progress(DownloadProgress {
        downloaded: 0,
        total,
    });
    while let Some(chunk) = res.chunk().await.map_err(|e| e.to_string())? {
        buf.extend_from_slice(&chunk);
        on_progress(DownloadProgress {
            downloaded: buf.len() as u64,
            total,
        });
    }
    Ok(buf)
}

/// Verify a downloaded artifact against a minisign signature.
///
/// `signature_b64` is the base64 of the whole `.minisig`/`.sig` file content
/// (the shape stored in `latest.json`, matching Tauri). `public_key_b64`
/// defaults to [`UPDATE_PUBLIC_KEY`]. An empty key or empty signature is a hard
/// error — the update is never applied unverified.
pub fn verify_update(
    artifact: &[u8],
    signature_b64: &str,
    public_key_b64: &str,
) -> Result<(), String> {
    if public_key_b64.trim().is_empty() {
        return Err("no updater public key configured — refusing unverified update".into());
    }
    if signature_b64.trim().is_empty() {
        return Err("update artifact has no signature — refusing to install".into());
    }
    let pk = PublicKey::from_base64(public_key_b64.trim())
        .map_err(|e| format!("invalid updater public key: {e}"))?;
    let sig_file = base64::engine::general_purpose::STANDARD
        .decode(signature_b64.trim())
        .map_err(|e| format!("invalid signature encoding: {e}"))?;
    let sig_file =
        String::from_utf8(sig_file).map_err(|_| "signature is not valid UTF-8".to_string())?;
    let sig = Signature::decode(&sig_file).map_err(|e| format!("malformed signature: {e}"))?;
    pk.verify(artifact, &sig, false)
        .map_err(|e| format!("signature verification failed: {e}"))
}

/// The `.app` bundle the running binary lives in
/// (`…/Labonair.app/Contents/MacOS/labonair` → `…/Labonair.app`).
pub fn current_app_bundle() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    exe.ancestors()
        .find(|p| p.extension().map(|e| e == "app").unwrap_or(false))
        .map(Path::to_path_buf)
}

/// Unpack a downloaded macOS update (`<name>.app.tar.gz`) and atomically swap it
/// over `current_bundle`.
///
/// Layout expected inside the archive: a single top-level `*.app` directory.
/// The running bundle is moved aside to `…/<name>.app.old-<ts>` first, then the
/// fresh bundle is renamed into place and the old copy removed. A failure to
/// write next to `current_bundle` (e.g. `/Applications` without permission)
/// surfaces as a descriptive error for the notification layer.
pub fn apply_macos_update(archive: &[u8], current_bundle: &Path) -> Result<(), String> {
    let parent = current_bundle
        .parent()
        .ok_or("cannot resolve install directory")?;
    let stage = parent.join(format!(".labonair-update-{}", unix_now()));
    std::fs::create_dir_all(&stage).map_err(|e| format!("cannot stage update: {e}"))?;

    let cleanup = |dir: &Path| {
        let _ = std::fs::remove_dir_all(dir);
    };

    let decoder = flate2::read::GzDecoder::new(archive);
    let mut tar = tar::Archive::new(decoder);
    if let Err(e) = tar.unpack(&stage) {
        cleanup(&stage);
        return Err(format!("cannot unpack update: {e}"));
    }

    let new_bundle = match find_app_bundle(&stage) {
        Some(p) => p,
        None => {
            cleanup(&stage);
            return Err("update archive contains no .app bundle".into());
        }
    };

    let backup = parent.join(format!(
        "{}.old-{}",
        current_bundle
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("Labonair.app"),
        unix_now()
    ));

    if let Err(e) = std::fs::rename(current_bundle, &backup) {
        cleanup(&stage);
        return Err(format!("cannot replace the current app (permission?): {e}"));
    }
    if let Err(e) = std::fs::rename(&new_bundle, current_bundle) {
        // Roll back so the app stays runnable.
        let _ = std::fs::rename(&backup, current_bundle);
        cleanup(&stage);
        return Err(format!("cannot move the new app into place: {e}"));
    }
    let _ = std::fs::remove_dir_all(&backup);
    cleanup(&stage);
    Ok(())
}

/// Relaunch `bundle` via `open` and exit the current process. Never returns.
pub fn relaunch(bundle: &Path) -> ! {
    let _ = std::process::Command::new("/usr/bin/open")
        .arg(bundle)
        .spawn();
    std::process::exit(0);
}

/// Convenience: does this manifest advertise an actionable update right now?
pub fn manifest_update(manifest: &UpdateManifest) -> Option<AvailableUpdate> {
    manifest.available()
}

fn find_app_bundle(dir: &Path) -> Option<PathBuf> {
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        for entry in std::fs::read_dir(&d).ok()?.flatten() {
            let path = entry.path();
            if path.is_dir() {
                if path.extension().map(|e| e == "app").unwrap_or(false) {
                    return Some(path);
                }
                stack.push(path);
            }
        }
    }
    None
}

fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

// ── Auto-check timestamp ─────────────────────────────────────────────────────

fn last_check_path(dir: &Path) -> PathBuf {
    dir.join("updater-last-check")
}

/// Record "an update check ran now" so [`should_auto_check`] backs off.
pub fn record_check_now() {
    record_check_now_in(&crate::modules::fs::paths::config_dir());
}

/// Should the app run a background update check? `true` when there is no record
/// or the last one is older than [`CHECK_INTERVAL`].
pub fn should_auto_check() -> bool {
    should_auto_check_in(&crate::modules::fs::paths::config_dir())
}

pub(crate) fn record_check_now_in(dir: &Path) {
    let _ = std::fs::write(last_check_path(dir), unix_now().to_string());
}

pub(crate) fn should_auto_check_in(dir: &Path) -> bool {
    let Ok(raw) = std::fs::read_to_string(last_check_path(dir)) else {
        return true;
    };
    let Ok(last) = raw.trim().parse::<u64>() else {
        return true;
    };
    unix_now().saturating_sub(last) >= CHECK_INTERVAL.as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    fn tmp() -> PathBuf {
        let d = std::env::temp_dir().join(format!("labonair-upd-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    fn sign(data: &[u8]) -> (String, String) {
        use minisign::{sign, KeyPair};
        let KeyPair { pk, sk } = KeyPair::generate_unencrypted_keypair().unwrap();
        let sig_box = sign(Some(&pk), &sk, Cursor::new(data), None, None).unwrap();
        let sig_b64 = base64::engine::general_purpose::STANDARD.encode(sig_box.into_string());
        (pk.to_base64(), sig_b64)
    }

    #[test]
    fn valid_signature_passes() {
        let data = b"labonair-update-artifact-bytes";
        let (pk, sig) = sign(data);
        assert!(verify_update(data, &sig, &pk).is_ok());
    }

    #[test]
    fn tampered_artifact_is_rejected() {
        let data = b"labonair-update-artifact-bytes";
        let (pk, sig) = sign(data);
        let mut bad = data.to_vec();
        bad[0] ^= 0xff;
        assert!(verify_update(&bad, &sig, &pk).is_err());
    }

    #[test]
    fn wrong_key_is_rejected() {
        let data = b"payload";
        let (_pk, sig) = sign(data);
        let (other_pk, _) = sign(b"unrelated");
        assert!(verify_update(data, &sig, &other_pk).is_err());
    }

    #[test]
    fn missing_signature_or_key_is_an_error() {
        assert!(verify_update(b"x", "", "RWQsomekey").is_err());
        assert!(verify_update(b"x", "c2ln", "").is_err());
    }

    #[test]
    fn placeholder_public_key_refuses_everything() {
        // Guard: while UPDATE_PUBLIC_KEY is unset the pipeline must never apply
        // an update. Flip this test's expectation when a real key is baked in.
        assert!(UPDATE_PUBLIC_KEY.trim().is_empty());
        assert!(verify_update(b"x", "c2ln", UPDATE_PUBLIC_KEY).is_err());
    }

    #[test]
    fn auto_check_backoff() {
        let dir = tmp();
        assert!(should_auto_check_in(&dir), "no record → check");
        record_check_now_in(&dir);
        assert!(!should_auto_check_in(&dir), "just checked → back off");
        std::fs::write(
            last_check_path(&dir),
            (unix_now() - CHECK_INTERVAL.as_secs() - 1).to_string(),
        )
        .unwrap();
        assert!(should_auto_check_in(&dir), "stale record → check");
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn apply_swaps_the_bundle() {
        let root = tmp();
        let install = root.join("Applications");
        std::fs::create_dir_all(&install).unwrap();
        let bundle = install.join("Labonair.app");
        std::fs::create_dir_all(bundle.join("Contents/MacOS")).unwrap();
        std::fs::write(bundle.join("Contents/MacOS/labonair"), b"OLD").unwrap();

        // Build a .app.tar.gz with a newer marker.
        let src = root.join("src");
        let new_app = src.join("Labonair.app/Contents/MacOS");
        std::fs::create_dir_all(&new_app).unwrap();
        std::fs::write(new_app.join("labonair"), b"NEW").unwrap();
        let mut ar = tar::Builder::new(flate2::write::GzEncoder::new(
            Vec::new(),
            flate2::Compression::default(),
        ));
        ar.append_dir_all("Labonair.app", src.join("Labonair.app"))
            .unwrap();
        let archive = ar.into_inner().unwrap().finish().unwrap();

        apply_macos_update(&archive, &bundle).unwrap();
        let got = std::fs::read(bundle.join("Contents/MacOS/labonair")).unwrap();
        assert_eq!(got, b"NEW");
        // No stray staging/backup dirs left behind.
        let leftovers: Vec<_> = std::fs::read_dir(&install)
            .unwrap()
            .flatten()
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .collect();
        assert_eq!(leftovers, vec!["Labonair.app".to_string()]);
        std::fs::remove_dir_all(&root).ok();
    }
}
