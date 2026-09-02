//! Auto-update foundation (T15-004).
//!
//! The reference app used the Tauri updater plugin: it polled a `latest.json`
//! manifest on GitHub Releases, compared versions and, on a newer one, offered
//! a signed download. Tauri is gone, so this module ports the *decision* layer
//! natively:
//!
//! * [`UpdateManifest`] — the same `latest.json` shape Tauri emits
//!   (`version` / `notes` / `pub_date` / `platforms.<target>.{url,signature}`),
//!   so existing release tooling keeps working.
//! * [`SemVer`] — a dependency-free `major.minor.patch` comparison
//!   (pre-release / build metadata are ignored, matching Tauri's default).
//! * [`UpdateManifest::available_for`] — returns [`AvailableUpdate`] when the
//!   manifest lists a strictly-newer version for the running platform.
//!
//! Downloading, signature verification and applying the update is deliberately
//! left to T15-005 (auto-updater); this is only the manifest + version-check
//! groundwork the packaging/release pipeline needs.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// Where the release pipeline publishes the update manifest.
///
/// Mirrors the reference `tauri.conf.json` updater endpoint, retargeted at this
/// fork's repo.
pub const DEFAULT_UPDATE_ENDPOINT: &str =
    "https://github.com/Snenjih/Labonair-rust/releases/latest/download/latest.json";

/// The manifest key for the platform this binary was built for.
///
/// Uses the same `<os>-<arch>` scheme Tauri's updater uses so a single
/// `latest.json` can serve every platform.
pub const UPDATE_TARGET: &str = current_target();

const fn current_target() -> &'static str {
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    {
        "darwin-aarch64"
    }
    #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
    {
        "darwin-x86_64"
    }
    #[cfg(all(target_os = "linux", target_arch = "aarch64"))]
    {
        "linux-aarch64"
    }
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    {
        "linux-x86_64"
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    {
        "unsupported"
    }
}

/// The running application version (`CARGO_PKG_VERSION` of the `labonair`
/// binary is the single source of truth; the packaging script reads the same
/// value via `cargo metadata`).
pub const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");

/// One platform entry inside a [`UpdateManifest`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UpdatePlatform {
    /// Direct download URL of the signed artifact for this platform.
    pub url: String,
    /// Detached signature (minisign, base64) produced at release time.
    #[serde(default)]
    pub signature: String,
}

/// The `latest.json` update manifest (Tauri-compatible shape).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UpdateManifest {
    /// Version string of the release, e.g. `"1.2.0"` or `"v1.2.0"`.
    pub version: String,
    /// Human-readable release notes / changelog excerpt.
    #[serde(default)]
    pub notes: Option<String>,
    /// RFC 3339 publish timestamp.
    #[serde(default, rename = "pub_date")]
    pub pub_date: Option<String>,
    /// Per-platform artifact table, keyed like [`UPDATE_TARGET`].
    #[serde(default)]
    pub platforms: BTreeMap<String, UpdatePlatform>,
}

/// A concrete, actionable update for the current platform.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AvailableUpdate {
    pub version: String,
    pub notes: Option<String>,
    pub pub_date: Option<String>,
    pub url: String,
    pub signature: String,
}

impl UpdateManifest {
    /// Parse a `latest.json` document.
    pub fn parse(json: &str) -> Result<Self, String> {
        serde_json::from_str(json).map_err(|e| format!("invalid update manifest: {e}"))
    }

    /// Return an [`AvailableUpdate`] iff this manifest advertises a strictly
    /// newer version *and* carries an artifact for `target`.
    pub fn available_for(&self, current: &str, target: &str) -> Option<AvailableUpdate> {
        let current = SemVer::parse(current)?;
        let remote = SemVer::parse(&self.version)?;
        if remote <= current {
            return None;
        }
        let platform = self.platforms.get(target)?;
        Some(AvailableUpdate {
            version: self.version.clone(),
            notes: self.notes.clone(),
            pub_date: self.pub_date.clone(),
            url: platform.url.clone(),
            signature: platform.signature.clone(),
        })
    }

    /// Convenience: check against the compiled-in version and platform target.
    pub fn available(&self) -> Option<AvailableUpdate> {
        self.available_for(CURRENT_VERSION, UPDATE_TARGET)
    }
}

/// Parse + compare a `major.minor.patch` version, ignoring any `-pre` / `+build`
/// suffix and an optional leading `v`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct SemVer {
    pub major: u64,
    pub minor: u64,
    pub patch: u64,
}

impl SemVer {
    pub fn parse(raw: &str) -> Option<Self> {
        let raw = raw.trim();
        let raw = raw.strip_prefix('v').unwrap_or(raw);
        let core = raw.split(['-', '+']).next().unwrap_or(raw);
        let mut parts = core.split('.');
        let major = parts.next()?.parse().ok()?;
        let minor = parts.next().unwrap_or("0").parse().ok()?;
        let patch = parts.next().unwrap_or("0").parse().ok()?;
        if parts.next().is_some() {
            return None;
        }
        Some(SemVer {
            major,
            minor,
            patch,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"{
        "version": "1.4.0",
        "notes": "Bug fixes and speedups.",
        "pub_date": "2026-09-02T10:00:00Z",
        "platforms": {
            "darwin-aarch64": { "url": "https://example.com/Labonair_1.4.0_aarch64.dmg", "signature": "sig-a" },
            "darwin-x86_64":  { "url": "https://example.com/Labonair_1.4.0_x64.dmg", "signature": "sig-b" }
        }
    }"#;

    #[test]
    fn semver_parses_and_orders() {
        assert_eq!(
            SemVer::parse("v2.0.6"),
            Some(SemVer {
                major: 2,
                minor: 0,
                patch: 6
            })
        );
        assert_eq!(
            SemVer::parse("1.2"),
            Some(SemVer {
                major: 1,
                minor: 2,
                patch: 0
            })
        );
        assert_eq!(SemVer::parse("1.2.3-beta.1"), SemVer::parse("1.2.3"));
        assert!(SemVer::parse("1.2.10").unwrap() > SemVer::parse("1.2.9").unwrap());
        assert!(SemVer::parse("2.0.0").unwrap() > SemVer::parse("1.99.99").unwrap());
        assert!(SemVer::parse("not-a-version").is_none());
        assert!(SemVer::parse("1.2.3.4").is_none());
    }

    #[test]
    fn newer_version_for_known_target_is_offered() {
        let m = UpdateManifest::parse(SAMPLE).unwrap();
        let update = m.available_for("1.3.9", "darwin-aarch64").unwrap();
        assert_eq!(update.version, "1.4.0");
        assert_eq!(update.url, "https://example.com/Labonair_1.4.0_aarch64.dmg");
        assert_eq!(update.signature, "sig-a");
        assert_eq!(update.notes.as_deref(), Some("Bug fixes and speedups."));
    }

    #[test]
    fn same_or_older_version_yields_nothing() {
        let m = UpdateManifest::parse(SAMPLE).unwrap();
        assert!(m.available_for("1.4.0", "darwin-aarch64").is_none());
        assert!(m.available_for("2.0.0", "darwin-aarch64").is_none());
    }

    #[test]
    fn unknown_target_yields_nothing() {
        let m = UpdateManifest::parse(SAMPLE).unwrap();
        assert!(m.available_for("1.0.0", "linux-x86_64").is_none());
    }

    #[test]
    fn malformed_manifest_is_an_error() {
        assert!(UpdateManifest::parse("{ not json").is_err());
    }

    #[test]
    fn current_target_is_one_of_the_supported_keys() {
        assert!(matches!(
            UPDATE_TARGET,
            "darwin-aarch64" | "darwin-x86_64" | "linux-aarch64" | "linux-x86_64" | "unsupported"
        ));
    }
}
