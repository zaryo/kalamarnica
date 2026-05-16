use std::collections::BTreeMap;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;

use anyhow::Context as _;
use anyhow::Result;
use anyhow::anyhow;

use crate::apply_context::ApplyContext;
use crate::context::Context;
use crate::vcs::Vcs;

pub struct Storage {
    base_dir: PathBuf,
}

impl Storage {
    pub fn new() -> Result<Self> {
        let base_dir = dirs::config_dir()
            .context("could not determine config directory")?
            .join("kalamarnica");

        Self::from_base_dir(base_dir)
    }

    fn from_base_dir(base_dir: PathBuf) -> Result<Self> {
        fs::create_dir_all(&base_dir).context("could not create config directory")?;

        Ok(Self { base_dir })
    }

    fn context_dir(&self, vcs: Vcs, name: &str) -> PathBuf {
        self.base_dir.join(vcs.to_string()).join(name)
    }

    fn configuration_path(&self, vcs: Vcs, name: &str) -> PathBuf {
        self.context_dir(vcs, name).join("configuration.toml")
    }

    fn token_path(&self, vcs: Vcs, name: &str) -> PathBuf {
        self.context_dir(vcs, name).join("token")
    }

    pub fn context_exists(&self, name: &str, vcs: Vcs) -> Result<bool> {
        Ok(self.configuration_path(vcs, name).exists())
    }

    pub fn read_context(&self, name: &str, vcs: Vcs) -> Result<Context> {
        let config_path = self.configuration_path(vcs, name);

        if !config_path.exists() {
            return Err(anyhow!("context '{name}' not found"));
        }

        let content = fs::read_to_string(&config_path)
            .with_context(|| format!("could not read configuration for '{name}'"))?;

        toml::from_str(&content)
            .with_context(|| format!("could not parse configuration for '{name}'"))
    }

    pub fn write_context(&self, name: &str, vcs: Vcs, context: &Context) -> Result<()> {
        let context_dir = self.context_dir(vcs, name);
        fs::create_dir_all(&context_dir)
            .with_context(|| format!("could not create directory for '{name}'"))?;

        let serialized = toml::to_string_pretty(context)
            .with_context(|| format!("could not serialize context '{name}'"))?;

        fs::write(self.configuration_path(vcs, name), serialized)
            .with_context(|| format!("could not write configuration for '{name}'"))
    }

    pub fn list_contexts(&self) -> Result<BTreeMap<String, Vec<String>>> {
        let mut result: BTreeMap<String, Vec<String>> = BTreeMap::new();

        for vcs in [Vcs::Github, Vcs::Gitlab] {
            let vcs_dir = self.base_dir.join(vcs.to_string());

            if !vcs_dir.exists() {
                continue;
            }

            let entries = fs::read_dir(&vcs_dir)
                .with_context(|| format!("could not read {vcs} directory"))?;

            for entry in entries {
                let entry = entry.context("could not read directory entry")?;
                let path = entry.path();

                if !path.is_dir() || !path.join("configuration.toml").exists() {
                    continue;
                }

                let name = entry.file_name().to_string_lossy().into_owned();
                result.entry(vcs.to_string()).or_default().push(name);
            }
        }

        for names in result.values_mut() {
            names.sort();
        }

        Ok(result)
    }

    pub fn delete_context(&self, name: &str, vcs: Vcs) -> Result<()> {
        let context_dir = self.context_dir(vcs, name);

        if context_dir.exists() {
            fs::remove_dir_all(&context_dir)
                .with_context(|| format!("could not delete context '{name}'"))?;
        }

        let is_active = self
            .get_active()?
            .is_some_and(|active| active.name == name && active.vcs == vcs);

        if is_active {
            let active_file = self.active_path();
            if active_file.exists() {
                fs::remove_file(&active_file).context("could not clear active context")?;
            }
        }

        Ok(())
    }

    pub fn read_token(&self, name: &str, vcs: Vcs) -> Result<Option<String>> {
        let token_path = self.token_path(vcs, name);
        if !token_path.exists() {
            return Ok(None);
        }

        let token = fs::read_to_string(&token_path)
            .with_context(|| format!("could not read token for '{name}'"))?;

        Ok(Some(token))
    }

    pub fn write_token(&self, name: &str, vcs: Vcs, token: &str) -> Result<()> {
        let context_dir = self.context_dir(vcs, name);
        fs::create_dir_all(&context_dir)
            .with_context(|| format!("could not create directory for '{name}'"))?;

        let token_path = self.token_path(vcs, name);

        fs::write(&token_path, token)
            .with_context(|| format!("could not write token for '{name}'"))?;

        fs::set_permissions(&token_path, fs::Permissions::from_mode(0o600))
            .with_context(|| format!("could not set permissions on token for '{name}'"))?;

        Ok(())
    }

    pub fn delete_token(&self, name: &str, vcs: Vcs) -> Result<()> {
        let token_path = self.token_path(vcs, name);
        if token_path.exists() {
            fs::remove_file(&token_path)
                .with_context(|| format!("could not delete token for '{name}'"))?;
        }

        Ok(())
    }

    pub fn get_active(&self) -> Result<Option<ApplyContext>> {
        let active_path = self.active_path();
        if !active_path.exists() {
            return Ok(None);
        }

        let content = fs::read_to_string(&active_path).context("could not read active context")?;
        let trimmed = content.trim();

        if trimmed.is_empty() {
            return Ok(None);
        }

        let active_context = toml::from_str(trimmed).context("could not parse active context")?;

        Ok(Some(active_context))
    }

    pub fn set_active(&self, active_context: &ApplyContext) -> Result<()> {
        let serialized =
            toml::to_string_pretty(active_context).context("could not serialize active context")?;

        fs::write(self.active_path(), serialized).context("could not set active context")
    }

    fn active_path(&self) -> PathBuf {
        self.base_dir.join("active")
    }
}

#[cfg(test)]
impl Storage {
    pub fn with_base_dir(base_dir: PathBuf) -> Result<Self> {
        Self::from_base_dir(base_dir)
    }
}

#[cfg(test)]
mod tests {
    use std::os::unix::fs::PermissionsExt as _;

    use super::Storage;
    use crate::apply_context::ApplyContext;
    use crate::test_utils::sample_context;
    use crate::test_utils::sample_gitlab_context;
    use crate::vcs::Vcs;

    #[test]
    fn test_context_does_not_exist_initially() -> Result<(), anyhow::Error> {
        let tmp = tempfile::tempdir()?;
        let storage = Storage::with_base_dir(tmp.path().to_path_buf())?;

        assert!(!storage.context_exists("work", Vcs::Github)?);

        Ok(())
    }

    #[test]
    fn test_context_exists_after_write() -> Result<(), anyhow::Error> {
        let tmp = tempfile::tempdir()?;
        let storage = Storage::with_base_dir(tmp.path().to_path_buf())?;
        storage.write_context("work", Vcs::Github, &sample_context())?;

        assert!(storage.context_exists("work", Vcs::Github)?);

        Ok(())
    }

    #[test]
    fn test_write_context_creates_configuration_toml() -> Result<(), anyhow::Error> {
        let tmp = tempfile::tempdir()?;
        let storage = Storage::with_base_dir(tmp.path().to_path_buf())?;
        storage.write_context("work", Vcs::Github, &sample_context())?;

        assert!(
            tmp.path()
                .join("github")
                .join("work")
                .join("configuration.toml")
                .exists()
        );

        Ok(())
    }

    #[test]
    fn test_write_and_read_context_roundtrip() -> Result<(), anyhow::Error> {
        let tmp = tempfile::tempdir()?;
        let storage = Storage::with_base_dir(tmp.path().to_path_buf())?;

        storage.write_context("work", Vcs::Github, &sample_context())?;
        let loaded = storage.read_context("work", Vcs::Github)?;

        assert_eq!(loaded.hostname, "github.com");
        assert_eq!(loaded.user, "testuser");

        Ok(())
    }

    #[test]
    fn test_read_nonexistent_context_fails() -> Result<(), anyhow::Error> {
        let tmp = tempfile::tempdir()?;
        let storage = Storage::with_base_dir(tmp.path().to_path_buf())?;

        let error = storage.read_context("missing", Vcs::Github).unwrap_err();
        assert_eq!(error.to_string(), "context 'missing' not found");

        Ok(())
    }

    #[test]
    fn test_contexts_with_different_vcs_coexist() -> Result<(), anyhow::Error> {
        let tmp = tempfile::tempdir()?;
        let storage = Storage::with_base_dir(tmp.path().to_path_buf())?;
        storage.write_context("github-work", Vcs::Github, &sample_context())?;
        storage.write_context("gitlab-work", Vcs::Gitlab, &sample_gitlab_context())?;

        assert!(storage.context_exists("github-work", Vcs::Github)?);
        assert!(storage.context_exists("gitlab-work", Vcs::Gitlab)?);

        Ok(())
    }

    #[test]
    fn test_same_name_different_vcs_contexts_coexist() -> Result<(), anyhow::Error> {
        let tmp = tempfile::tempdir()?;
        let storage = Storage::with_base_dir(tmp.path().to_path_buf())?;
        storage.write_context("work", Vcs::Github, &sample_context())?;
        storage.write_context("work", Vcs::Gitlab, &sample_gitlab_context())?;

        assert!(storage.context_exists("work", Vcs::Github)?);
        assert!(storage.context_exists("work", Vcs::Gitlab)?);

        Ok(())
    }

    #[test]
    fn test_context_exists_with_different_vcs_returns_false() -> Result<(), anyhow::Error> {
        let tmp = tempfile::tempdir()?;
        let storage = Storage::with_base_dir(tmp.path().to_path_buf())?;
        storage.write_context("work", Vcs::Github, &sample_context())?;

        assert!(!storage.context_exists("work", Vcs::Gitlab)?);

        Ok(())
    }

    #[test]
    fn test_multiple_names_under_same_vcs_coexist() -> Result<(), anyhow::Error> {
        let tmp = tempfile::tempdir()?;
        let storage = Storage::with_base_dir(tmp.path().to_path_buf())?;
        storage.write_context("personal", Vcs::Github, &sample_context())?;
        storage.write_context("work", Vcs::Github, &sample_context())?;

        assert!(storage.context_exists("personal", Vcs::Github)?);
        assert!(storage.context_exists("work", Vcs::Github)?);

        Ok(())
    }

    #[test]
    fn test_list_empty_contexts() -> Result<(), anyhow::Error> {
        let tmp = tempfile::tempdir()?;
        let storage = Storage::with_base_dir(tmp.path().to_path_buf())?;

        assert!(storage.list_contexts()?.is_empty());

        Ok(())
    }

    #[test]
    fn test_list_contexts_groups_by_vcs() -> Result<(), anyhow::Error> {
        let tmp = tempfile::tempdir()?;
        let storage = Storage::with_base_dir(tmp.path().to_path_buf())?;
        storage.write_context("personal", Vcs::Github, &sample_context())?;
        storage.write_context("work", Vcs::Github, &sample_context())?;
        storage.write_context("enterprise", Vcs::Gitlab, &sample_gitlab_context())?;

        let contexts = storage.list_contexts()?;
        assert_eq!(contexts["github"], vec!["personal", "work"]);
        assert_eq!(contexts["gitlab"], vec!["enterprise"]);

        Ok(())
    }

    #[test]
    fn test_list_contexts_includes_same_name_across_vcs() -> Result<(), anyhow::Error> {
        let tmp = tempfile::tempdir()?;
        let storage = Storage::with_base_dir(tmp.path().to_path_buf())?;
        storage.write_context("work", Vcs::Github, &sample_context())?;
        storage.write_context("work", Vcs::Gitlab, &sample_gitlab_context())?;

        let contexts = storage.list_contexts()?;
        assert_eq!(contexts["github"], vec!["work"]);
        assert_eq!(contexts["gitlab"], vec!["work"]);

        Ok(())
    }

    #[test]
    fn test_delete_context_removes_entry() -> Result<(), anyhow::Error> {
        let tmp = tempfile::tempdir()?;
        let storage = Storage::with_base_dir(tmp.path().to_path_buf())?;
        storage.write_context("work", Vcs::Github, &sample_context())?;

        storage.delete_context("work", Vcs::Github)?;

        assert!(!storage.context_exists("work", Vcs::Github)?);

        Ok(())
    }

    #[test]
    fn test_delete_context_leaves_other_entries() -> Result<(), anyhow::Error> {
        let tmp = tempfile::tempdir()?;
        let storage = Storage::with_base_dir(tmp.path().to_path_buf())?;
        storage.write_context("work", Vcs::Github, &sample_context())?;
        storage.write_context("personal", Vcs::Github, &sample_context())?;

        storage.delete_context("work", Vcs::Github)?;

        assert!(storage.list_contexts()?.values().flatten().eq(["personal"]));

        Ok(())
    }

    #[test]
    fn test_delete_nonexistent_context_succeeds() -> Result<(), anyhow::Error> {
        let tmp = tempfile::tempdir()?;
        let storage = Storage::with_base_dir(tmp.path().to_path_buf())?;

        storage.delete_context("nonexistent", Vcs::Github)?;

        Ok(())
    }

    #[test]
    fn test_delete_active_context_clears_active() -> Result<(), anyhow::Error> {
        let tmp = tempfile::tempdir()?;
        let storage = Storage::with_base_dir(tmp.path().to_path_buf())?;
        storage.write_context("work", Vcs::Github, &sample_context())?;
        let active = ApplyContext {
            name: "work".to_owned(),
            vcs: Vcs::Github,
        };
        storage.set_active(&active)?;

        storage.delete_context("work", Vcs::Github)?;

        assert!(storage.get_active()?.is_none());

        Ok(())
    }

    #[test]
    fn test_delete_non_active_context_preserves_active() -> Result<(), anyhow::Error> {
        let tmp = tempfile::tempdir()?;
        let storage = Storage::with_base_dir(tmp.path().to_path_buf())?;
        storage.write_context("work", Vcs::Github, &sample_context())?;
        storage.write_context("personal", Vcs::Github, &sample_context())?;
        let active = ApplyContext {
            name: "personal".to_owned(),
            vcs: Vcs::Github,
        };
        storage.set_active(&active)?;

        storage.delete_context("work", Vcs::Github)?;

        let retrieved = storage.get_active()?.unwrap();
        assert_eq!(retrieved.name, "personal");
        assert_eq!(retrieved.vcs, Vcs::Github);

        Ok(())
    }

    #[test]
    fn test_delete_context_with_one_vcs_preserves_same_name_other_vcs() -> Result<(), anyhow::Error>
    {
        let tmp = tempfile::tempdir()?;
        let storage = Storage::with_base_dir(tmp.path().to_path_buf())?;
        storage.write_context("work", Vcs::Github, &sample_context())?;
        storage.write_context("work", Vcs::Gitlab, &sample_gitlab_context())?;

        storage.delete_context("work", Vcs::Github)?;

        assert!(!storage.context_exists("work", Vcs::Github)?);
        assert!(storage.context_exists("work", Vcs::Gitlab)?);

        Ok(())
    }

    #[test]
    fn test_write_token_creates_token_file() -> Result<(), anyhow::Error> {
        let tmp = tempfile::tempdir()?;
        let storage = Storage::with_base_dir(tmp.path().to_path_buf())?;
        storage.write_token("work", Vcs::Github, "ghp_test123")?;

        assert!(
            tmp.path()
                .join("github")
                .join("work")
                .join("token")
                .exists()
        );

        Ok(())
    }

    #[test]
    fn test_read_token_returns_none_when_not_stored() -> Result<(), anyhow::Error> {
        let tmp = tempfile::tempdir()?;
        let storage = Storage::with_base_dir(tmp.path().to_path_buf())?;

        assert!(storage.read_token("work", Vcs::Github)?.is_none());

        Ok(())
    }

    #[test]
    fn test_write_and_read_token_roundtrip() -> Result<(), anyhow::Error> {
        let tmp = tempfile::tempdir()?;
        let storage = Storage::with_base_dir(tmp.path().to_path_buf())?;
        storage.write_token("work", Vcs::Github, "ghp_secret123")?;

        let token = storage.read_token("work", Vcs::Github)?;
        assert_eq!(token.as_deref(), Some("ghp_secret123"));

        Ok(())
    }

    #[test]
    fn test_token_file_has_restricted_permissions() -> Result<(), anyhow::Error> {
        let tmp = tempfile::tempdir()?;
        let storage = Storage::with_base_dir(tmp.path().to_path_buf())?;
        storage.write_token("work", Vcs::Github, "ghp_secret123")?;

        let token_path = tmp.path().join("github").join("work").join("token");
        let metadata = std::fs::metadata(token_path)?;
        assert_eq!(metadata.permissions().mode() & 0o777, 0o600);

        Ok(())
    }

    #[test]
    fn test_delete_existing_token() -> Result<(), anyhow::Error> {
        let tmp = tempfile::tempdir()?;
        let storage = Storage::with_base_dir(tmp.path().to_path_buf())?;
        storage.write_token("work", Vcs::Github, "ghp_test")?;

        storage.delete_token("work", Vcs::Github)?;

        assert!(storage.read_token("work", Vcs::Github)?.is_none());

        Ok(())
    }

    #[test]
    fn test_delete_token_preserves_context_configuration() -> Result<(), anyhow::Error> {
        let tmp = tempfile::tempdir()?;
        let storage = Storage::with_base_dir(tmp.path().to_path_buf())?;
        storage.write_context("work", Vcs::Github, &sample_context())?;
        storage.write_token("work", Vcs::Github, "ghp_test")?;

        storage.delete_token("work", Vcs::Github)?;

        assert!(storage.context_exists("work", Vcs::Github)?);
        let context = storage.read_context("work", Vcs::Github)?;
        assert_eq!(context.hostname, "github.com");

        Ok(())
    }

    #[test]
    fn test_delete_nonexistent_token_succeeds() -> Result<(), anyhow::Error> {
        let tmp = tempfile::tempdir()?;
        let storage = Storage::with_base_dir(tmp.path().to_path_buf())?;

        storage.delete_token("nonexistent", Vcs::Github)?;

        Ok(())
    }

    #[test]
    fn test_delete_context_also_removes_token() -> Result<(), anyhow::Error> {
        let tmp = tempfile::tempdir()?;
        let storage = Storage::with_base_dir(tmp.path().to_path_buf())?;
        storage.write_context("work", Vcs::Github, &sample_context())?;
        storage.write_token("work", Vcs::Github, "ghp_test123")?;

        storage.delete_context("work", Vcs::Github)?;

        assert!(storage.read_token("work", Vcs::Github)?.is_none());

        Ok(())
    }

    #[test]
    fn test_get_active_returns_none_initially() -> Result<(), anyhow::Error> {
        let tmp = tempfile::tempdir()?;
        let storage = Storage::with_base_dir(tmp.path().to_path_buf())?;

        assert!(storage.get_active()?.is_none());

        Ok(())
    }

    #[test]
    fn test_set_and_get_active_roundtrip() -> Result<(), anyhow::Error> {
        let tmp = tempfile::tempdir()?;
        let storage = Storage::with_base_dir(tmp.path().to_path_buf())?;
        let active = ApplyContext {
            name: "work".to_owned(),
            vcs: Vcs::Github,
        };
        storage.set_active(&active)?;

        let retrieved = storage.get_active()?.unwrap();
        assert_eq!(retrieved.name, "work");
        assert_eq!(retrieved.vcs, Vcs::Github);

        Ok(())
    }

    #[test]
    fn test_get_active_returns_none_for_empty_file() -> Result<(), anyhow::Error> {
        let tmp = tempfile::tempdir()?;
        let storage = Storage::with_base_dir(tmp.path().to_path_buf())?;
        std::fs::write(tmp.path().join("active"), "")?;

        assert!(storage.get_active()?.is_none());

        Ok(())
    }
}
