//! Provider-instance management + active model/provider persistence.
//!
//! Port of `reference-src/src/modules/ai/store/providersStore.ts` +
//! `lib/modelRef.ts`. Instances (name / provider / base URL / local model id)
//! and the active model ref persist to `~/.config/labonair/labonair-ai.json`.
//! API keys are **not** part of this file — they stay in the keyring.

use serde::{Deserialize, Serialize};

use crate::config::{ProviderId, DEFAULT_MODEL_ID};

/// One configured provider the user has set up. Multiple instances of the same
/// `provider_id` are allowed (e.g. two OpenAI keys).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProviderInstance {
    pub id: String,
    pub provider_id: ProviderId,
    /// Display name; auto-derived (`openai`, `openai2`, …) unless user-set.
    pub name: String,
    /// Base URL for local / custom providers.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base_url: Option<String>,
    /// Model id running on the local server (local providers only).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub local_model_id: Option<String>,
    /// Context-window override for openai-compatible endpoints.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context_window_size: Option<u32>,
}

impl ProviderInstance {
    pub fn new(provider_id: ProviderId, existing: &[ProviderInstance]) -> ProviderInstance {
        let default_base = provider_id.default_base_url();
        ProviderInstance {
            id: uuid::Uuid::new_v4().to_string(),
            provider_id,
            name: auto_name(provider_id, existing),
            base_url: if default_base.is_empty() {
                None
            } else {
                Some(default_base.to_string())
            },
            local_model_id: None,
            context_window_size: None,
        }
    }
}

/// Auto-generate a name for a new instance of `provider` given the existing set.
pub fn auto_name(provider: ProviderId, existing: &[ProviderInstance]) -> String {
    let n = existing
        .iter()
        .filter(|i| i.provider_id == provider)
        .count();
    if n == 0 {
        provider.as_str().to_string()
    } else {
        format!("{}{}", provider.as_str(), n + 1)
    }
}

/// When 2+ instances of one provider exist, rename any still-default-named ones
/// to `<provider><n>`. Mirrors `modelRef.ts::renameForDuplicates`.
pub fn rename_for_duplicates(mut instances: Vec<ProviderInstance>) -> Vec<ProviderInstance> {
    let mut count = std::collections::HashMap::<ProviderId, usize>::new();
    for i in &instances {
        *count.entry(i.provider_id).or_default() += 1;
    }
    let mut idx = std::collections::HashMap::<ProviderId, usize>::new();
    for inst in &mut instances {
        if count.get(&inst.provider_id).copied().unwrap_or(0) > 1 {
            let n = idx.entry(inst.provider_id).or_default();
            *n += 1;
            if is_default_name(&inst.name, inst.provider_id) {
                inst.name = format!("{}{}", inst.provider_id.as_str(), n);
            }
        }
    }
    instances
}

fn is_default_name(name: &str, provider: ProviderId) -> bool {
    let base = provider.as_str();
    name == base
        || (name.starts_with(base)
            && name[base.len()..].chars().all(|c| c.is_ascii_digit())
            && name.len() > base.len())
}

// ── Model references ────────────────────────────────────────────────────────

/// A model reference is `"<modelDefId>"` or `"<modelDefId>@<instanceId>"`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelRef {
    pub model_def_id: String,
    pub instance_id: Option<String>,
}

pub fn parse_model_ref(reference: &str) -> ModelRef {
    match reference.rfind('@') {
        Some(idx) => ModelRef {
            model_def_id: reference[..idx].to_string(),
            instance_id: Some(reference[idx + 1..].to_string()),
        },
        None => ModelRef {
            model_def_id: reference.to_string(),
            instance_id: None,
        },
    }
}

pub fn make_model_ref(model_def_id: &str, instance_id: Option<&str>) -> String {
    match instance_id {
        Some(id) => format!("{model_def_id}@{id}"),
        None => model_def_id.to_string(),
    }
}

/// Find the instance for a model ref: by id if present, else the first instance
/// of `provider`.
pub fn resolve_instance<'a>(
    provider: ProviderId,
    instance_id: Option<&str>,
    instances: &'a [ProviderInstance],
) -> Option<&'a ProviderInstance> {
    match instance_id {
        Some(id) => instances.iter().find(|i| i.id == id),
        None => instances.iter().find(|i| i.provider_id == provider),
    }
}

// ── Persistent store ───────────────────────────────────────────────────────

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct StoreFile {
    #[serde(default)]
    instances: Vec<ProviderInstance>,
    #[serde(default)]
    active_model_ref: Option<String>,
    #[serde(default)]
    recent_model_ids: Vec<String>,
}

/// In-memory instance list + active selection, backed by a JSON file.
#[derive(Debug)]
pub struct InstanceStore {
    path: std::path::PathBuf,
    file: StoreFile,
}

impl InstanceStore {
    /// Default location: `~/.config/labonair/labonair-ai.json`.
    pub fn default_path() -> std::path::PathBuf {
        let dir = dirs::home_dir()
            .map(|h| h.join(".config").join("labonair"))
            .unwrap_or_else(|| std::path::PathBuf::from("."));
        let _ = std::fs::create_dir_all(&dir);
        dir.join("labonair-ai.json")
    }

    pub fn load(path: impl Into<std::path::PathBuf>) -> InstanceStore {
        let path = path.into();
        let file = std::fs::read_to_string(&path)
            .ok()
            .and_then(|s| serde_json::from_str::<StoreFile>(&s).ok())
            .unwrap_or_default();
        let mut store = InstanceStore { path, file };
        // Drop instances for providers that no longer exist.
        store
            .file
            .instances
            .retain(|i| ProviderId::from_id(i.provider_id.as_str()).is_some());
        store
    }

    pub fn open_default() -> InstanceStore {
        Self::load(Self::default_path())
    }

    fn save(&self) -> Result<(), String> {
        let json = serde_json::to_string_pretty(&self.file).map_err(|e| e.to_string())?;
        let tmp = self.path.with_extension("json.tmp");
        std::fs::write(&tmp, json).map_err(|e| e.to_string())?;
        std::fs::rename(&tmp, &self.path).map_err(|e| e.to_string())
    }

    pub fn instances(&self) -> &[ProviderInstance] {
        &self.file.instances
    }

    pub fn add(&mut self, provider: ProviderId) -> Result<ProviderInstance, String> {
        let inst = ProviderInstance::new(provider, &self.file.instances);
        let mut updated = self.file.instances.clone();
        updated.push(inst.clone());
        self.file.instances = rename_for_duplicates(updated);
        self.save()?;
        // Return the freshly-added instance by id (its name may have changed).
        Ok(self
            .file
            .instances
            .iter()
            .find(|i| i.id == inst.id)
            .cloned()
            .unwrap_or(inst))
    }

    pub fn update(
        &mut self,
        id: &str,
        patch: impl FnOnce(&mut ProviderInstance),
    ) -> Result<(), String> {
        let inst = self
            .file
            .instances
            .iter_mut()
            .find(|i| i.id == id)
            .ok_or_else(|| format!("no instance {id}"))?;
        patch(inst);
        self.save()
    }

    pub fn remove(&mut self, id: &str) -> Result<(), String> {
        self.file.instances.retain(|i| i.id != id);
        self.file.instances = rename_for_duplicates(std::mem::take(&mut self.file.instances));
        if self
            .file
            .active_model_ref
            .as_deref()
            .map(parse_model_ref)
            .and_then(|r| r.instance_id)
            .as_deref()
            == Some(id)
        {
            self.file.active_model_ref = None;
        }
        self.save()
    }

    /// The persisted active model ref, or the default model id on first run.
    pub fn active_model_ref(&self) -> String {
        self.file
            .active_model_ref
            .clone()
            .unwrap_or_else(|| DEFAULT_MODEL_ID.to_string())
    }

    pub fn set_active_model_ref(&mut self, reference: &str) -> Result<(), String> {
        self.file.active_model_ref = Some(reference.to_string());
        let recents = &mut self.file.recent_model_ids;
        recents.retain(|r| r != reference);
        recents.insert(0, reference.to_string());
        recents.truncate(10);
        self.save()
    }

    pub fn recent_model_ids(&self) -> &[String] {
        &self.file.recent_model_ids
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp_path(name: &str) -> std::path::PathBuf {
        let mut p = std::env::temp_dir();
        p.push(format!(
            "labonair-ai-test-{}-{}.json",
            name,
            uuid::Uuid::new_v4()
        ));
        p
    }

    #[test]
    fn model_ref_parsing() {
        let r = parse_model_ref("gpt-5.5@abc-123");
        assert_eq!(r.model_def_id, "gpt-5.5");
        assert_eq!(r.instance_id.as_deref(), Some("abc-123"));
        let r = parse_model_ref("claude-opus-4-7");
        assert_eq!(r.instance_id, None);
        // openrouter "org/model" slugs contain no '@'
        assert_eq!(
            parse_model_ref("openai/gpt-oss-20b").model_def_id,
            "openai/gpt-oss-20b"
        );
        assert_eq!(make_model_ref("gpt-5.5", Some("abc")), "gpt-5.5@abc");
    }

    #[test]
    fn auto_name_and_dedup() {
        let mut list = vec![];
        let a = ProviderInstance::new(ProviderId::Openai, &list);
        assert_eq!(a.name, "openai");
        list.push(a);
        let b = ProviderInstance::new(ProviderId::Openai, &list);
        assert_eq!(b.name, "openai2");
        list.push(b);
        let renamed = rename_for_duplicates(list);
        assert_eq!(renamed[0].name, "openai1");
        assert_eq!(renamed[1].name, "openai2");
    }

    #[test]
    fn resolve_instance_by_id_or_provider() {
        let list = vec![
            ProviderInstance {
                id: "i1".into(),
                provider_id: ProviderId::Openai,
                name: "openai".into(),
                base_url: None,
                local_model_id: None,
                context_window_size: None,
            },
            ProviderInstance {
                id: "i2".into(),
                provider_id: ProviderId::Anthropic,
                name: "anthropic".into(),
                base_url: None,
                local_model_id: None,
                context_window_size: None,
            },
        ];
        assert_eq!(
            resolve_instance(ProviderId::Anthropic, None, &list)
                .unwrap()
                .id,
            "i2"
        );
        assert_eq!(
            resolve_instance(ProviderId::Openai, Some("i2"), &list)
                .unwrap()
                .id,
            "i2"
        );
        assert!(resolve_instance(ProviderId::Google, None, &list).is_none());
    }

    #[test]
    fn store_persists_instances_and_active_ref() {
        let path = tmp_path("persist");
        {
            let mut s = InstanceStore::load(&path);
            assert_eq!(s.active_model_ref(), DEFAULT_MODEL_ID);
            let inst = s.add(ProviderId::Openai).unwrap();
            s.set_active_model_ref(&make_model_ref("gpt-5.5", Some(&inst.id)))
                .unwrap();
        }
        let s = InstanceStore::load(&path);
        assert_eq!(s.instances().len(), 1);
        assert!(s.active_model_ref().starts_with("gpt-5.5@"));
        assert_eq!(s.recent_model_ids().len(), 1);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn removing_active_instance_clears_active_ref() {
        let path = tmp_path("remove");
        let mut s = InstanceStore::load(&path);
        let inst = s.add(ProviderId::Xai).unwrap();
        s.set_active_model_ref(&make_model_ref("grok-4.20-reasoning", Some(&inst.id)))
            .unwrap();
        s.remove(&inst.id).unwrap();
        assert_eq!(s.active_model_ref(), DEFAULT_MODEL_ID);
        assert!(s.instances().is_empty());
        let _ = std::fs::remove_file(&path);
    }
}
