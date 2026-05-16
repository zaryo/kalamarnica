use std::fs;
use std::path::Path;

use anyhow::Result;
use clap::Parser;
use toml::from_str;

use crate::apply_context::ApplyContext;
use crate::cmd::handler::Handler;
use crate::repo_root;
use crate::storage::Storage;

#[derive(Parser)]
pub struct Current;

impl Current {
    pub fn execute(&self, storage: &Storage) -> Result<()> {
        match storage.get_active()? {
            Some(active) => {
                let context = storage.read_context(&active.name, active.vcs)?;
                log::info!(
                    "{} ({}@{}, {}, {})",
                    active.name,
                    context.user,
                    context.hostname,
                    context.transport,
                    active.vcs
                );
            }
            None => log::info!("No active context"),
        }

        if let Some(repo_root_path) = repo_root::repo_root()? {
            let binding_path = Path::new(&repo_root_path).join(".vcs_context");
            if binding_path.exists() {
                let content = fs::read_to_string(&binding_path)?;
                let binding_context: ApplyContext = from_str(content.trim())?;
                log::info!(
                    "Repo-bound context: {} ({})",
                    binding_context.name,
                    binding_context.vcs
                );
            }
        }

        Ok(())
    }
}

impl Handler for Current {
    fn handle(&self, storage: &Storage) -> Result<()> {
        self.execute(storage)
    }
}

#[cfg(test)]
mod tests {
    use super::Current;
    use crate::apply_context::ApplyContext;
    use crate::cmd::handler::Handler;
    use crate::storage::Storage;
    use crate::test_utils::CWD_MUTEX;
    use crate::test_utils::sample_context;
    use crate::vcs::Vcs;

    #[test]
    fn test_no_active_context_succeeds() -> Result<(), anyhow::Error> {
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

        let result = Current.handle(&storage);

        std::env::set_current_dir(&original_cwd)?;
        result?;

        Ok(())
    }

    #[test]
    fn test_with_active_context_succeeds() -> Result<(), anyhow::Error> {
        let _guard = CWD_MUTEX
            .lock()
            .map_err(|poison_error| anyhow::anyhow!("{poison_error}"))?;
        let original_cwd = std::env::current_dir()?;

        let tmp = tempfile::tempdir()?;
        let storage = Storage::with_base_dir(tmp.path().to_path_buf())?;
        storage.write_context("work", Vcs::Github, &sample_context())?;
        let active = ApplyContext {
            name: "work".to_owned(),
            vcs: Vcs::Github,
        };
        storage.set_active(&active)?;

        let repo_dir = tmp.path().join("repo");
        std::fs::create_dir_all(&repo_dir)?;
        git2::Repository::init(&repo_dir)?;
        std::env::set_current_dir(&repo_dir)?;

        let result = Current.handle(&storage);

        std::env::set_current_dir(&original_cwd)?;
        result?;

        Ok(())
    }

    #[test]
    fn test_with_repo_bound_context_succeeds() -> Result<(), anyhow::Error> {
        let _guard = CWD_MUTEX
            .lock()
            .map_err(|poison_error| anyhow::anyhow!("{poison_error}"))?;
        let original_cwd = std::env::current_dir()?;

        let tmp = tempfile::tempdir()?;
        let storage = Storage::with_base_dir(tmp.path().to_path_buf())?;

        let repo_dir = tmp.path().join("repo");
        std::fs::create_dir_all(&repo_dir)?;
        git2::Repository::init(&repo_dir)?;
        std::fs::write(
            repo_dir.join(".vcs_context"),
            "name = \"work\"\nvcs = \"github\"",
        )?;
        std::env::set_current_dir(&repo_dir)?;

        let result = Current.handle(&storage);

        std::env::set_current_dir(&original_cwd)?;
        result?;

        Ok(())
    }

    #[test]
    fn test_outside_git_repo_succeeds() -> Result<(), anyhow::Error> {
        let _guard = CWD_MUTEX
            .lock()
            .map_err(|poison_error| anyhow::anyhow!("{poison_error}"))?;
        let original_cwd = std::env::current_dir()?;

        let tmp = tempfile::tempdir()?;
        let storage = Storage::with_base_dir(tmp.path().to_path_buf())?;
        std::env::set_current_dir(tmp.path())?;

        let result = Current.handle(&storage);

        std::env::set_current_dir(&original_cwd)?;
        result?;

        Ok(())
    }
}
