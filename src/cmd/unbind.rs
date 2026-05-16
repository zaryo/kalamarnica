use std::fs;
use std::path::Path;

use anyhow::Result;
use anyhow::anyhow;
use clap::Parser;

use crate::cmd::handler::Handler;
use crate::repo_root;
use crate::storage::Storage;

#[derive(Parser)]
pub struct Unbind;

impl Unbind {
    pub fn execute(&self, _storage: &Storage) -> Result<()> {
        let repo_root_path =
            repo_root::repo_root()?.ok_or_else(|| anyhow!("not inside a git repository"))?;

        let binding_path = Path::new(&repo_root_path).join(".vcs_context");
        match binding_path.exists() {
            true => {
                fs::remove_file(&binding_path)?;
                log::info!("Unbound context from {repo_root_path}");
            }
            false => log::info!("No context bound to this repository"),
        }

        Ok(())
    }
}

impl Handler for Unbind {
    fn handle(&self, storage: &Storage) -> Result<()> {
        self.execute(storage)
    }
}

#[cfg(test)]
mod tests {
    use super::Unbind;
    use crate::cmd::handler::Handler;
    use crate::storage::Storage;
    use crate::test_utils::CWD_MUTEX;

    #[test]
    fn test_unbind_outside_git_repo_fails() -> Result<(), anyhow::Error> {
        let _guard = CWD_MUTEX
            .lock()
            .map_err(|poison_error| anyhow::anyhow!("{poison_error}"))?;
        let original_cwd = std::env::current_dir()?;

        let tmp = tempfile::tempdir()?;
        let storage = Storage::with_base_dir(tmp.path().to_path_buf())?;
        std::env::set_current_dir(tmp.path())?;

        let handler = Unbind;
        let result = handler.handle(&storage);

        std::env::set_current_dir(&original_cwd)?;
        let error = result.unwrap_err();
        assert_eq!(error.to_string(), "not inside a git repository");

        Ok(())
    }

    #[test]
    fn test_unbind_without_vcs_context_file_succeeds() -> Result<(), anyhow::Error> {
        let _guard = CWD_MUTEX
            .lock()
            .map_err(|poison_error| anyhow::anyhow!("{poison_error}"))?;
        let original_cwd = std::env::current_dir()?;

        let tmp = tempfile::tempdir()?;
        let storage = Storage::with_base_dir(tmp.path().to_path_buf())?;

        let repo_dir = tmp.path().join("repo");
        std::fs::create_dir_all(&repo_dir)?;
        git2::Repository::init(&repo_dir)?;
        std::env::set_current_dir(&repo_dir)?;

        let handler = Unbind;
        let result = handler.handle(&storage);

        std::env::set_current_dir(&original_cwd)?;
        result?;

        Ok(())
    }

    #[test]
    fn test_unbind_with_vcs_context_file_removes_it() -> Result<(), anyhow::Error> {
        let _guard = CWD_MUTEX
            .lock()
            .map_err(|poison_error| anyhow::anyhow!("{poison_error}"))?;
        let original_cwd = std::env::current_dir()?;

        let tmp = tempfile::tempdir()?;
        let storage = Storage::with_base_dir(tmp.path().to_path_buf())?;

        let repo_dir = tmp.path().join("repo");
        std::fs::create_dir_all(&repo_dir)?;
        git2::Repository::init(&repo_dir)?;
        let binding_content = "[github]\nname = \"work\"";
        std::fs::write(repo_dir.join(".vcs_context"), binding_content)?;
        std::env::set_current_dir(&repo_dir)?;

        let handler = Unbind;
        let result = handler.handle(&storage);

        std::env::set_current_dir(&original_cwd)?;
        result?;

        assert!(!repo_dir.join(".vcs_context").exists());

        Ok(())
    }
}
