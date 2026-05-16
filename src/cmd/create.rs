use std::env::VarError;

use anyhow::Result;
use anyhow::anyhow;
use anyhow::bail;
use clap::Parser;

use crate::cmd::handler::Handler;
use crate::cmd::validate_name::validate_name;
use crate::context::Context;
use crate::gh_client::GhClient;
use crate::gl_client::GlClient;
use crate::storage::Storage;
use crate::transport::Transport;
use crate::vcs::Vcs;
use crate::vcs_client::VcsClient;

#[derive(Parser)]
pub struct Create {
    #[arg(long, value_parser = validate_name)]
    /// Name for the new context
    name: String,

    #[arg(long)]
    /// Detect hostname and user from current VCS session
    from_current: bool,

    #[arg(long)]
    /// VCS hostname (e.g., github.com)
    hostname: Option<String>,

    #[arg(long)]
    /// VCS username
    user: Option<String>,

    #[arg(long, default_value = "ssh")]
    /// Git transport protocol
    transport: Transport,

    #[arg(long)]
    /// Versioning code system used. Eg. Github, Gitlab
    vcs: Vcs,
}

impl Create {
    pub fn execute(&self, storage: &Storage) -> Result<()> {
        if storage.context_exists(&self.name, self.vcs)? {
            bail!("context '{}' already exists", self.name);
        }

        let (hostname, user) = match self.from_current {
            true => {
                let (hostname_env_var, default_hostname) = match self.vcs {
                    Vcs::Github => ("GH_HOST", "github.com"),
                    Vcs::Gitlab => ("GL_HOST", "gitlab.com"),
                };

                let hostname = match std::env::var(hostname_env_var) {
                    Ok(env_hostname) => env_hostname,
                    Err(VarError::NotPresent) => default_hostname.to_owned(),
                    Err(VarError::NotUnicode(raw_value)) => {
                        bail!(
                            "{hostname_env_var} contains invalid unicode: {}",
                            raw_value.display()
                        )
                    }
                };

                let user = match self.vcs {
                    Vcs::Github => GhClient::api_user(&hostname),
                    Vcs::Gitlab => GlClient::api_user(&hostname),
                }?;

                (hostname, user)
            }
            false => {
                let hostname = self
                    .hostname
                    .clone()
                    .ok_or_else(|| anyhow!("--hostname is required (or use --from-current)"))?;
                let user = self
                    .user
                    .clone()
                    .ok_or_else(|| anyhow!("--user is required (or use --from-current)"))?;

                (hostname, user)
            }
        };

        let context = Context {
            hostname,
            user,
            transport: self.transport.clone(),
        };

        storage.write_context(&self.name, self.vcs, &context)?;
        log::info!(
            "Created context '{}' ({}@{}, {}, {})",
            self.name,
            context.user,
            context.hostname,
            context.transport,
            self.vcs,
        );

        Ok(())
    }
}

impl Handler for Create {
    fn handle(&self, storage: &Storage) -> Result<()> {
        self.execute(storage)
    }
}

#[cfg(test)]
mod tests {
    use super::Create;
    use crate::cmd::handler::Handler;
    use crate::cmd::validate_name::validate_name;
    use crate::storage::Storage;
    use crate::test_utils::sample_context;
    use crate::transport::Transport;
    use crate::vcs::Vcs;

    #[test]
    fn test_empty_name_fails() -> Result<(), anyhow::Error> {
        let error = validate_name("").unwrap_err();
        assert_eq!(error.to_string(), "context name cannot be empty");

        Ok(())
    }

    #[test]
    fn test_alphanumeric_name_succeeds() -> Result<(), anyhow::Error> {
        let name = "work123";
        let result = validate_name(name)?;
        assert_eq!(result, name);

        Ok(())
    }

    #[test]
    fn test_name_with_hyphens() -> Result<(), anyhow::Error> {
        let name = "my-work";
        let result = validate_name(name)?;
        assert_eq!(result, name);

        Ok(())
    }

    #[test]
    fn test_name_with_underscores() -> Result<(), anyhow::Error> {
        let name = "my_work";
        let result = validate_name(name)?;
        assert_eq!(result, name);

        Ok(())
    }

    #[test]
    fn test_name_with_mixed_valid_chars() -> Result<(), anyhow::Error> {
        let name = "my-work_123";
        let result = validate_name(name)?;
        assert_eq!(result, name);

        Ok(())
    }

    #[test]
    fn test_name_with_spaces_fails() -> Result<(), anyhow::Error> {
        let error = validate_name("my work").unwrap_err();
        assert_eq!(
            error.to_string(),
            "context name must contain only alphanumeric characters, hyphens, and underscores"
        );

        Ok(())
    }

    #[test]
    fn test_name_with_dots_fails() -> Result<(), anyhow::Error> {
        let error = validate_name("my.work").unwrap_err();
        assert_eq!(
            error.to_string(),
            "context name must contain only alphanumeric characters, hyphens, and underscores"
        );

        Ok(())
    }

    #[test]
    fn test_name_with_slashes_fails() -> Result<(), anyhow::Error> {
        let error = validate_name("my/work").unwrap_err();
        assert_eq!(
            error.to_string(),
            "context name must contain only alphanumeric characters, hyphens, and underscores"
        );

        Ok(())
    }

    #[test]
    fn test_create_context_already_exists_fails() -> Result<(), anyhow::Error> {
        let tmp = tempfile::tempdir()?;
        let storage = Storage::with_base_dir(tmp.path().to_path_buf())?;
        let context_name = "work";
        storage.write_context(context_name, Vcs::Github, &sample_context())?;

        let create = Create {
            name: context_name.to_owned(),
            from_current: false,
            hostname: Some("github.com".to_owned()),
            user: Some("newuser".to_owned()),
            transport: Transport::Ssh,
            vcs: Vcs::Github,
        };

        let error = create.handle(&storage).unwrap_err();
        assert_eq!(
            error.to_string(),
            format!("context '{context_name}' already exists")
        );

        Ok(())
    }

    #[test]
    fn test_create_context_same_name_different_vcs_succeeds() -> Result<(), anyhow::Error> {
        let tmp = tempfile::tempdir()?;
        let storage = Storage::with_base_dir(tmp.path().to_path_buf())?;
        storage.write_context("work", Vcs::Github, &sample_context())?;

        let create = Create {
            name: "work".to_owned(),
            from_current: false,
            hostname: Some("gitlab.com".to_owned()),
            user: Some("gitlabuser".to_owned()),
            transport: Transport::Ssh,
            vcs: Vcs::Gitlab,
        };

        create.handle(&storage)?;

        assert!(storage.context_exists("work", Vcs::Github)?);
        assert!(storage.context_exists("work", Vcs::Gitlab)?);

        Ok(())
    }

    #[test]
    fn test_create_context_manual_without_hostname_fails() -> Result<(), anyhow::Error> {
        let tmp = tempfile::tempdir()?;
        let storage = Storage::with_base_dir(tmp.path().to_path_buf())?;

        let create = Create {
            name: "work".to_owned(),
            from_current: false,
            hostname: None,
            user: Some("testuser".to_owned()),
            transport: Transport::Ssh,
            vcs: Vcs::Github,
        };

        let error = create.handle(&storage).unwrap_err();
        assert_eq!(
            error.to_string(),
            "--hostname is required (or use --from-current)"
        );

        Ok(())
    }

    #[test]
    fn test_create_context_manual_without_user_fails() -> Result<(), anyhow::Error> {
        let tmp = tempfile::tempdir()?;
        let storage = Storage::with_base_dir(tmp.path().to_path_buf())?;

        let create = Create {
            name: "work".to_owned(),
            from_current: false,
            hostname: Some("github.com".to_owned()),
            user: None,
            transport: Transport::Ssh,
            vcs: Vcs::Github,
        };

        let error = create.handle(&storage).unwrap_err();
        assert_eq!(
            error.to_string(),
            "--user is required (or use --from-current)"
        );

        Ok(())
    }

    #[test]
    fn test_create_context_manual_succeeds() -> Result<(), anyhow::Error> {
        let tmp = tempfile::tempdir()?;
        let storage = Storage::with_base_dir(tmp.path().to_path_buf())?;

        let create = Create {
            name: "work".to_owned(),
            from_current: false,
            hostname: Some("github.com".to_owned()),
            user: Some("testuser".to_owned()),
            transport: Transport::Ssh,
            vcs: Vcs::Github,
        };

        create.handle(&storage)?;

        assert!(storage.context_exists("work", Vcs::Github)?);
        let context = storage.read_context("work", Vcs::Github)?;
        assert_eq!(context.hostname, "github.com");
        assert_eq!(context.user, "testuser");
        assert!(matches!(context.transport, Transport::Ssh));

        Ok(())
    }

    #[test]
    fn test_create_context_with_invalid_name_fails() -> Result<(), anyhow::Error> {
        let context_name = "invalid name!";
        let error = validate_name(context_name).unwrap_err();
        assert_eq!(
            error.to_string(),
            "context name must contain only alphanumeric characters, hyphens, and underscores"
        );

        Ok(())
    }

    #[test]
    fn test_create_multiple_contexts_under_same_vcs() -> Result<(), anyhow::Error> {
        let tmp = tempfile::tempdir()?;
        let storage = Storage::with_base_dir(tmp.path().to_path_buf())?;

        for name in ["personal", "work"] {
            let create = Create {
                name: name.to_owned(),
                from_current: false,
                hostname: Some("github.com".to_owned()),
                user: Some(format!("{name}user")),
                transport: Transport::Ssh,
                vcs: Vcs::Github,
            };
            create.handle(&storage)?;
        }

        assert!(storage.context_exists("personal", Vcs::Github)?);
        assert!(storage.context_exists("work", Vcs::Github)?);
        assert!(
            tmp.path()
                .join("github")
                .join("personal")
                .join("configuration.toml")
                .exists()
        );
        assert!(
            tmp.path()
                .join("github")
                .join("work")
                .join("configuration.toml")
                .exists()
        );

        Ok(())
    }
}
