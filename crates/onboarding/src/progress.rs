//! Persisted onboarding progress: which steps the user has already seen.
//!
//! Per-installation state, deliberately not a setting: `settings.json` is
//! synced between machines via dotfiles, and a new machine must run
//! onboarding again.

use std::collections::BTreeSet;
use std::path::Path;

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct OnboardingProgress {
    completed_steps: BTreeSet<String>,
}

impl OnboardingProgress {
    /// A missing or unreadable file means "fresh install"; it never blocks
    /// startup.
    pub fn load(path: &Path) -> Self {
        let raw = match std::fs::read_to_string(path) {
            Ok(raw) => raw,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Self::default(),
            Err(error) => {
                tracing::warn!(%error, path = %path.display(), "labonair-onboarding: cannot read progress file");
                return Self::default();
            }
        };
        serde_json::from_str(&raw).unwrap_or_else(|error| {
            tracing::warn!(%error, path = %path.display(), "labonair-onboarding: corrupt progress file");
            Self::default()
        })
    }

    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        let json = serde_json::to_string_pretty(self).map_err(std::io::Error::other)?;
        std::fs::write(path, json)
    }

    pub fn is_completed(&self, step_id: &str) -> bool {
        self.completed_steps.contains(step_id)
    }

    pub fn complete<'a>(&mut self, step_ids: impl IntoIterator<Item = &'a str>) {
        self.completed_steps
            .extend(step_ids.into_iter().map(str::to_owned));
    }

    /// The applicable steps (in the order given) the user has not completed.
    pub fn pending<'a>(&self, applicable: &[&'a str]) -> Vec<&'a str> {
        applicable
            .iter()
            .copied()
            .filter(|id| !self.is_completed(id))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fresh_progress_has_everything_pending_and_completion_clears_it() {
        let mut progress = OnboardingProgress::default();
        assert_eq!(progress.pending(&["a", "b"]), ["a", "b"]);
        progress.complete(["a", "b"]);
        assert!(progress.pending(&["a", "b"]).is_empty());
    }

    #[test]
    fn a_step_added_later_is_the_only_pending_one() {
        let mut progress = OnboardingProgress::default();
        progress.complete(["a", "b"]);
        assert_eq!(progress.pending(&["a", "b", "c"]), ["c"]);
    }

    #[test]
    fn round_trips_and_survives_a_corrupt_file() {
        let dir = std::env::temp_dir().join(format!("labonair-onboarding-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("onboarding.json");

        let mut progress = OnboardingProgress::default();
        progress.complete(["onboarding.look"]);
        progress.save(&path).unwrap();
        assert_eq!(OnboardingProgress::load(&path), progress);

        std::fs::write(&path, "{ not json").unwrap();
        assert_eq!(
            OnboardingProgress::load(&path),
            OnboardingProgress::default()
        );
        assert_eq!(
            OnboardingProgress::load(&dir.join("missing.json")),
            OnboardingProgress::default()
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
}
