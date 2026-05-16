use anyhow::Result;
use anyhow::bail;
use clap::Parser;

use crate::cmd::handler::Handler;
use crate::storage::Storage;
use crate::vcs::Vcs;

#[derive(Parser)]
pub struct Delete {
    /// Context name to delete
    name: String,

    #[arg(long)]
    /// Versioning code system used. Eg. Github, Gitlab
    vcs: Vcs,
}

impl Delete {
    pub fn execute(&self, storage: &Storage) -> Result<()> {
        if !storage.context_exists(&self.name, self.vcs)? {
            bail!("context '{}' does not exist", self.name);
        }

        storage.delete_context(&self.name, self.vcs)?;
        log::info!("Deleted context '{}'", self.name);

        Ok(())
    }
}

impl Handler for Delete {
    fn handle(&self, storage: &Storage) -> Result<()> {
        self.execute(storage)
    }
}

#[cfg(test)]
mod tests {
    use super::Delete;
    use crate::cmd::handler::Handler;
    use crate::storage::Storage;
    use crate::test_utils::sample_context;
    use crate::vcs::Vcs;

    #[test]
    fn test_delete_nonexistent_context_fails() -> Result<(), anyhow::Error> {
        let tmp = tempfile::tempdir()?;
        let storage = Storage::with_base_dir(tmp.path().to_path_buf())?;

        let context_name = "nonexistent";
        let handler = Delete {
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
    fn test_delete_existing_context_succeeds() -> Result<(), anyhow::Error> {
        let tmp = tempfile::tempdir()?;
        let storage = Storage::with_base_dir(tmp.path().to_path_buf())?;
        storage.write_context("work", Vcs::Github, &sample_context())?;

        let handler = Delete {
            name: "work".to_owned(),
            vcs: Vcs::Github,
        };

        handler.handle(&storage)?;
        assert!(!storage.context_exists("work", Vcs::Github)?);

        Ok(())
    }

    #[test]
    fn test_delete_context_with_token_removes_both() -> Result<(), anyhow::Error> {
        let tmp = tempfile::tempdir()?;
        let storage = Storage::with_base_dir(tmp.path().to_path_buf())?;
        storage.write_context("work", Vcs::Github, &sample_context())?;
        storage.write_token("work", Vcs::Github, "ghp_test123")?;

        let handler = Delete {
            name: "work".to_owned(),
            vcs: Vcs::Github,
        };

        handler.handle(&storage)?;
        assert!(!storage.context_exists("work", Vcs::Github)?);
        assert!(storage.read_token("work", Vcs::Github)?.is_none());

        Ok(())
    }

    #[test]
    fn test_delete_context_only_removes_specified_vcs() -> Result<(), anyhow::Error> {
        let tmp = tempfile::tempdir()?;
        let storage = Storage::with_base_dir(tmp.path().to_path_buf())?;
        storage.write_context("work", Vcs::Github, &sample_context())?;
        storage.write_context("work", Vcs::Gitlab, &sample_context())?;

        let handler = Delete {
            name: "work".to_owned(),
            vcs: Vcs::Github,
        };

        handler.handle(&storage)?;
        assert!(!storage.context_exists("work", Vcs::Github)?);
        assert!(storage.context_exists("work", Vcs::Gitlab)?);

        Ok(())
    }
}
