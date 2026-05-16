use std::path::PathBuf;

use anyhow::Context as _;
use anyhow::Result;
use octocrab::Octocrab;
use tokio::runtime::Runtime;

use crate::vcs_client::VcsClient;

pub struct GhClient;

impl VcsClient for GhClient {
    fn get_hosts_path() -> Result<PathBuf> {
        let config_dir = dirs::config_dir().context("could not determine config directory")?;

        Ok(config_dir.join("gh").join("hosts.yml"))
    }

    fn fetch_api_user(hostname: &str, token: &str) -> Result<String> {
        let async_runtime = Runtime::new().context("could not create async runtime")?;

        async_runtime.block_on(async {
            let mut octocrab_builder = Octocrab::builder().personal_token(token.to_owned());
            if hostname != "github.com" {
                octocrab_builder =
                    octocrab_builder.base_uri(format!("https://{hostname}/api/v3"))?;
            }
            let github_client = octocrab_builder.build()?;
            let current_user = github_client.current().user().await?;

            Ok(current_user.login)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::GhClient;
    use crate::vcs_client::VcsClient;

    #[test]
    fn test_get_hosts_path_ends_with_gh_hosts_yml() -> Result<(), anyhow::Error> {
        let path = GhClient::get_hosts_path()?;
        assert!(path.ends_with("gh/hosts.yml"));

        Ok(())
    }
}
