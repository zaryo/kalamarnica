use anyhow::Result;
use anyhow::bail;
use clap::Parser;

use crate::apply_context::ApplyContext;
use crate::cmd::handler::Handler;
use crate::gh_client::GhClient;
use crate::gl_client::GlClient;
use crate::storage::Storage;
use crate::vcs::Vcs;
use crate::vcs_client::VcsClient;

#[derive(Parser)]
pub struct Switch {
    /// Context name to switch to
    name: String,

    #[arg(long)]
    /// Versioning code system used. Eg. Github, Gitlab
    vcs: Vcs,
}

impl Switch {
    #[must_use]
    pub const fn for_context(name: String, vcs: Vcs) -> Self {
        Self { name, vcs }
    }

    pub fn execute(&self, storage: &Storage) -> Result<()> {
        if !storage.context_exists(&self.name, self.vcs)? {
            bail!("context '{}' does not exist", &self.name);
        }

        let context = storage.read_context(&self.name, self.vcs)?;
        let active_context = ApplyContext {
            name: self.name.clone(),
            vcs: self.vcs,
        };
        storage.set_active(&active_context)?;

        if let Some(stored_token) = storage.read_token(&self.name, self.vcs)? {
            let credential_result = match self.vcs {
                Vcs::Github => GhClient::write_host_credentials(
                    &context.hostname,
                    &context.user,
                    &stored_token,
                ),
                Vcs::Gitlab => GlClient::write_host_credentials(
                    &context.hostname,
                    &context.user,
                    &stored_token,
                ),
            };
            match credential_result {
                Ok(()) => log::warn!("Applied stored token for '{}'", &self.name),
                Err(credential_error) => {
                    log::warn!(
                        "Failed to apply stored token for '{}': {}",
                        &self.name,
                        credential_error
                    );
                }
            }
        }

        let default_host_result = match self.vcs {
            Vcs::Github => GhClient::set_default_vcs_host(&context.hostname),
            Vcs::Gitlab => GlClient::set_default_vcs_host(&context.hostname),
        };
        if let Err(default_host_error) = default_host_result {
            log::warn!(
                "Failed to set default VCS host for '{}': {}",
                &self.name,
                default_host_error
            );
        }

        match verify_auth(&context.hostname, &context.user, self.vcs) {
            Ok(()) => log::warn!("Authentication verified"),
            Err(verify_error) => {
                let instruction = match self.vcs {
                    Vcs::Github => format!(
                        "Run: gh auth login --hostname {} --user {} --scopes repo,read:org",
                        context.hostname, context.user
                    ),
                    Vcs::Gitlab => format!(
                        "Run: kalamarnica set-token --name {} --token <your-personal-access-token>",
                        self.name
                    ),
                };
                log::warn!(
                    "Authentication required for {}@{} ({})\n  {}",
                    context.user,
                    context.hostname,
                    verify_error,
                    instruction,
                );
            }
        }

        log::info!(
            "Switched to context '{}' ({}@{}, {}, {})",
            &self.name,
            context.user,
            context.hostname,
            context.transport,
            &self.vcs,
        );

        Ok(())
    }
}

impl Handler for Switch {
    fn handle(&self, storage: &Storage) -> Result<()> {
        self.execute(storage)
    }
}

fn verify_auth(hostname: &str, user: &str, vcs: Vcs) -> Result<()> {
    let auth_status_output = match vcs {
        Vcs::Github => GhClient::auth_status(hostname),
        Vcs::Gitlab => GlClient::auth_status(hostname),
    }?;

    if !auth_status_output.contains(&format!("Logged in to {hostname} account {user}")) {
        bail!("not logged in as {user}");
    }

    match vcs {
        Vcs::Github => GhClient::auth_switch(hostname, user),
        Vcs::Gitlab => GlClient::auth_switch(hostname, user),
    }?;

    let authenticated_user = match vcs {
        Vcs::Github => GhClient::api_user(hostname),
        Vcs::Gitlab => GlClient::api_user(hostname),
    }?;

    if authenticated_user != user {
        bail!("expected user {user}, got {authenticated_user}");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::Switch;
    use crate::apply_context::ApplyContext;
    use crate::storage::Storage;
    use crate::test_utils::sample_context;
    use crate::vcs::Vcs;

    #[test]
    fn test_switch_to_nonexistent_context_fails() -> Result<(), anyhow::Error> {
        let tmp = tempfile::tempdir()?;
        let storage = Storage::with_base_dir(tmp.path().to_path_buf())?;

        let context_name = "nonexistent".to_string();
        let switch_handler = Switch::for_context(context_name.to_owned(), Vcs::Github);

        let error = switch_handler.execute(&storage).unwrap_err();
        assert_eq!(
            error.to_string(),
            format!("context '{context_name}' does not exist")
        );

        Ok(())
    }

    #[test]
    fn test_switch_to_existing_context_succeeds() -> Result<(), anyhow::Error> {
        let tmp = tempfile::tempdir()?;
        let storage = Storage::with_base_dir(tmp.path().to_path_buf())?;
        storage.write_context("work", Vcs::Github, &sample_context())?;

        let switch_handler = Switch::for_context("work".to_owned(), Vcs::Github);
        switch_handler.execute(&storage)?;

        let active = storage.get_active()?;
        assert_eq!(
            active,
            Some(ApplyContext {
                name: "work".to_owned(),
                vcs: Vcs::Github,
            })
        );

        Ok(())
    }
}
