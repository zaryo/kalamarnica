use std::fs;
use std::path::Path;

use anyhow::Result;
use anyhow::anyhow;
use anyhow::bail;
use clap::Parser;

use crate::cmd::handler::Handler;
use crate::repo_root;
use crate::storage::Storage;
use crate::vcs::Vcs;

#[derive(Parser)]
pub struct Bind {
    /// Context name to bind to this repository
    name: String,

    #[arg(long)]
    /// Versioning code system used. Eg. Github, Gitlab
    vcs: Vcs,
}

impl Bind {
    pub fn execute(&self, storage: &Storage) -> Result<()> {
        if !storage.context_exists(&self.name, self.vcs)? {
            bail!("context '{}' does not exist", self.name);
        }

        let repo_root_path =
            repo_root::repo_root()?.ok_or_else(|| anyhow!("not inside a git repository"))?;

        let binding_path = Path::new(&repo_root_path).join(".vcs_context");
        let binding_content = format!("name = \"{}\"\nvcs = \"{}\"", self.name, self.vcs);

        fs::write(&binding_path, binding_content)?;
        log::info!(
            "Bound context '{}' ({}) to {repo_root_path}",
            self.name,
            self.vcs
        );

        Ok(())
    }
}

impl Handler for Bind {
    fn handle(&self, storage: &Storage) -> Result<()> {
        self.execute(storage)
    }
}

#[cfg(test)]
mod tests {
    use super::Bind;
    use crate::cmd::handler::Handler;
    use crate::storage::Storage;
    use crate::test_utils::CWD_MUTEX;
    use crate::test_utils::sample_context;
    use crate::vcs::Vcs;

    #[test]
    fn test_bind_nonexistent_context_fails() -> Result<(), anyhow::Error> {
        let tmp = tempfile::tempdir()?;
        let storage = Storage::with_base_dir(tmp.path().to_path_buf())?;

        let context_name = "nonexistent";
        let handler = Bind {
            name: context_name.to_owned(),
            vcs: Vcs::Github,
        };

        let error = handler.handle(&storage).unwrap_err();
        assert_eq!(
            error.to_string(),
            format!("context '{context_name}' does not exist")
        );

        Ok(())
    }

    #[test]
    fn test_bind_outside_git_repo_fails() -> Result<(), anyhow::Error> {
        let _guard = CWD_MUTEX
            .lock()
            .map_err(|poison_error| anyhow::anyhow!("{poison_error}"))?;
        let original_cwd = std::env::current_dir()?;

        let tmp = tempfile::tempdir()?;
        let storage = Storage::with_base_dir(tmp.path().to_path_buf())?;
        storage.write_context("work", Vcs::Github, &sample_context())?;

        let non_git_dir = tmp.path().join("not-a-repo");
        std::fs::create_dir_all(&non_git_dir)?;
        std::env::set_current_dir(&non_git_dir)?;

        let handler = Bind {
            name: "work".to_owned(),
            vcs: Vcs::Github,
        };
        let result = handler.handle(&storage);

        std::env::set_current_dir(&original_cwd)?;
        assert_eq!(
            result.unwrap_err().to_string(),
            "not inside a git repository"
        );

        Ok(())
    }

    #[test]
    fn test_bind_inside_git_repo_creates_vcs_context_file() -> Result<(), anyhow::Error> {
        let _guard = CWD_MUTEX
            .lock()
            .map_err(|poison_error| anyhow::anyhow!("{poison_error}"))?;
        let original_cwd = std::env::current_dir()?;

        let tmp = tempfile::tempdir()?;
        let storage = Storage::with_base_dir(tmp.path().to_path_buf())?;
        storage.write_context("work", Vcs::Github, &sample_context())?;

        let repo_dir = tmp.path().join("repo");
        std::fs::create_dir_all(&repo_dir)?;
        git2::Repository::init(&repo_dir)?;
        std::env::set_current_dir(&repo_dir)?;

        let handler = Bind {
            name: "work".to_owned(),
            vcs: Vcs::Github,
        };
        let result = handler.handle(&storage);

        std::env::set_current_dir(&original_cwd)?;
        result?;

        let vcs_context_content = std::fs::read_to_string(repo_dir.join(".vcs_context"))?;
        assert!(vcs_context_content.contains("name = \"work\""));
        assert!(vcs_context_content.contains("vcs = \"github\""));

        Ok(())
    }
}
