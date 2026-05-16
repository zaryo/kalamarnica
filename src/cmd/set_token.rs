use anyhow::Result;
use anyhow::bail;
use clap::Parser;

use crate::cmd::handler::Handler;
use crate::storage::Storage;
use crate::vcs::Vcs;

#[derive(Parser)]
pub struct SetToken {
    #[arg(long)]
    /// Context name
    name: String,

    /// Personal access token
    token: String,

    #[arg(long)]
    /// Versioning code system used. Eg. Github, Gitlab
    vcs: Vcs,
}

impl SetToken {
    pub fn execute(&self, storage: &Storage) -> Result<()> {
        if !storage.context_exists(&self.name, self.vcs)? {
            bail!("context '{}' does not exist", self.name);
        }

        storage.write_token(&self.name, self.vcs, &self.token)?;
        log::info!("Stored token for context '{}'", self.name);

        Ok(())
    }
}

impl Handler for SetToken {
    fn handle(&self, storage: &Storage) -> Result<()> {
        self.execute(storage)
    }
}

#[cfg(test)]
mod tests {
    use super::SetToken;
    use crate::cmd::handler::Handler;
    use crate::storage::Storage;
    use crate::test_utils::sample_context;
    use crate::vcs::Vcs;

    #[test]
    fn test_set_token_for_nonexistent_context_fails() -> Result<(), anyhow::Error> {
        let tmp = tempfile::tempdir()?;
        let storage = Storage::with_base_dir(tmp.path().to_path_buf())?;

        let context_name = "nonexistent";
        let handler = SetToken {
            name: context_name.to_owned(),
            token: "ghp_test123".to_owned(),
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
    fn test_set_token_for_existing_context_succeeds() -> Result<(), anyhow::Error> {
        let tmp = tempfile::tempdir()?;
        let storage = Storage::with_base_dir(tmp.path().to_path_buf())?;
        storage.write_context("work", Vcs::Github, &sample_context())?;

        let handler = SetToken {
            name: "work".to_owned(),
            token: "ghp_secret123".to_owned(),
            vcs: Vcs::Github,
        };

        handler.handle(&storage)?;

        let stored_token = storage.read_token("work", Vcs::Github)?;
        assert_eq!(stored_token.as_deref(), Some("ghp_secret123"));

        Ok(())
    }
}
