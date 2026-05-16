use anyhow::Result;
use clap::Parser;
use indoc::formatdoc;

use crate::cmd::handler::Handler;
use crate::context::Context;
use crate::gh_client::GhClient;
use crate::gl_client::GlClient;
use crate::storage::Storage;
use crate::vcs::Vcs;
use crate::vcs_client::VcsClient;

#[derive(Parser)]
pub struct AuthStatus;

impl AuthStatus {
    pub fn execute(&self, storage: &Storage) -> Result<()> {
        let contexts = storage.list_contexts()?;
        let active = storage.get_active()?;

        if contexts.is_empty() {
            log::info!("No contexts found.");

            return Ok(());
        }

        for (vcs_str, names) in &contexts {
            let vcs: Vcs = vcs_str.parse()?;
            log::info!("{vcs_str}");

            for name in names {
                let context = storage.read_context(name, vcs)?;
                let active_marker = match active
                    .as_ref()
                    .is_some_and(|active_ctx| active_ctx.name == *name && active_ctx.vcs == vcs)
                {
                    true => " *",
                    false => "",
                };
                let has_stored_token = storage.read_token(name, vcs)?.is_some();
                let token_info = match has_stored_token {
                    true => "stored",
                    false => "none (using shared keyring)",
                };

                let is_authenticated = match vcs {
                    Vcs::Github => GhClient::auth_status(&context.hostname),
                    Vcs::Gitlab => GlClient::auth_status(&context.hostname),
                }
                .map(|output| {
                    output.contains(&format!(
                        "Logged in to {} account {}",
                        context.hostname, context.user
                    ))
                })
                .unwrap_or(false);

                let auth_info = match is_authenticated {
                    true => "verified".to_owned(),
                    false => {
                        format_unauthenticated_hint(vcs, name, &context.hostname, &context.user)
                    }
                };

                let entry =
                    format_context_entry(name, active_marker, &context, token_info, &auth_info);

                log::info!("{entry}");
            }
        }

        Ok(())
    }
}

impl Handler for AuthStatus {
    fn handle(&self, storage: &Storage) -> Result<()> {
        self.execute(storage)
    }
}

fn format_unauthenticated_hint(vcs: Vcs, name: &str, hostname: &str, user: &str) -> String {
    match vcs {
        Vcs::Github => formatdoc! {"
            not authenticated
                Run: gh auth login --hostname {hostname} --user {user} --scopes repo,read:org",
        },
        Vcs::Gitlab => formatdoc! {"
            not authenticated
                Run: kalamarnica set-token --name {name} --token <your-personal-access-token>",
        },
    }
}

fn format_context_entry(
    name: &str,
    active_marker: &str,
    context: &Context,
    token_info: &str,
    auth_info: &str,
) -> String {
    formatdoc! {"
        {name}{active_marker}
          Host: {hostname}
          User: {user}
          Transport: {transport}
          Token: {token_info}
          Auth: {auth_info}",
        hostname = context.hostname,
        user = context.user,
        transport = context.transport,
    }
}

#[cfg(test)]
mod tests {
    use super::AuthStatus;
    use super::format_context_entry;
    use super::format_unauthenticated_hint;
    use crate::cmd::handler::Handler;
    use crate::storage::Storage;
    use crate::test_utils::sample_context;
    use crate::test_utils::sample_gitlab_context;
    use crate::vcs::Vcs;

    #[test]
    fn test_auth_status_with_no_contexts_succeeds() -> Result<(), anyhow::Error> {
        let tmp = tempfile::tempdir()?;
        let storage = Storage::with_base_dir(tmp.path().to_path_buf())?;

        let handler = AuthStatus;
        handler.handle(&storage)?;

        Ok(())
    }

    #[test]
    fn test_auth_status_with_context_succeeds() -> Result<(), anyhow::Error> {
        let tmp = tempfile::tempdir()?;
        let storage = Storage::with_base_dir(tmp.path().to_path_buf())?;
        storage.write_context("work", Vcs::Github, &sample_context())?;

        let handler = AuthStatus;
        handler.handle(&storage)?;

        Ok(())
    }

    #[test]
    fn test_format_context_entry_verified() {
        let context = sample_context();
        let entry = format_context_entry("work", " *", &context, "stored", "verified");
        assert_eq!(
            entry,
            "work *\n  Host: github.com\n  User: testuser\n  Transport: ssh\n  Token: stored\n  Auth: verified"
        );
    }

    #[test]
    fn test_format_context_entry_unauthenticated() {
        let context = sample_context();
        let hint = format_unauthenticated_hint(Vcs::Github, "work", "github.com", "testuser");
        let entry =
            format_context_entry("work", "", &context, "none (using shared keyring)", &hint);
        assert_eq!(
            entry,
            "work\n  Host: github.com\n  User: testuser\n  Transport: ssh\n  Token: none (using shared keyring)\n  Auth: not authenticated\n    Run: gh auth login --hostname github.com --user testuser --scopes repo,read:org"
        );
    }

    #[test]
    fn test_format_unauthenticated_hint_github() {
        let hint = format_unauthenticated_hint(Vcs::Github, "work", "github.com", "testuser");
        assert_eq!(
            hint,
            "not authenticated\n    Run: gh auth login --hostname github.com --user testuser --scopes repo,read:org"
        );
    }

    #[test]
    fn test_format_unauthenticated_hint_gitlab() {
        let context = sample_gitlab_context();
        let hint =
            format_unauthenticated_hint(Vcs::Gitlab, "work", &context.hostname, &context.user);
        assert_eq!(
            hint,
            "not authenticated\n    Run: kalamarnica set-token --name work --token <your-personal-access-token>"
        );
    }
}
