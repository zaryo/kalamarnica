use std::fs;
use std::path::PathBuf;

use anyhow::Context as _;
use anyhow::Result;
use anyhow::anyhow;
use anyhow::bail;
use serde_yaml::Mapping;
use serde_yaml::Value;

use crate::ensure_host_entry::ensure_host_entry;
use crate::ensure_users_map::ensure_users_map;
use crate::utils::string_value;

pub trait VcsClient {
    fn get_hosts_path() -> Result<PathBuf>;
    fn fetch_api_user(hostname: &str, token: &str) -> Result<String>;

    #[must_use]
    fn token_key() -> &'static str {
        "oauth_token"
    }

    fn read_hosts() -> Result<Mapping> {
        let hosts_path = Self::get_hosts_path()?;
        if !hosts_path.exists() {
            return Ok(Mapping::new());
        }

        let content = fs::read_to_string(&hosts_path).context("could not read hosts config")?;
        if content.trim().is_empty() {
            return Ok(Mapping::new());
        }

        let parsed: Value =
            serde_yaml::from_str(&content).context("could not parse hosts config")?;

        match parsed {
            Value::Mapping(mapping) => Ok(mapping),
            _ => bail!("hosts config is not a YAML mapping"),
        }
    }

    fn write_hosts(hosts_config: &Mapping) -> Result<()> {
        let hosts_path = Self::get_hosts_path()?;

        if let Some(parent_dir) = hosts_path.parent() {
            fs::create_dir_all(parent_dir).context("could not create config directory")?;
        }

        let content = serde_yaml::to_string(&Value::Mapping(hosts_config.clone()))
            .context("could not serialize hosts config")?;

        fs::write(&hosts_path, content).context("could not write hosts config")
    }

    fn set_default_vcs_host(_hostname: &str) -> Result<()> {
        Ok(())
    }

    fn write_host_credentials(vcs_hostname: &str, user: &str, token: &str) -> Result<()> {
        let mut hosts_config = Self::read_hosts()?;
        let host_mapping = ensure_host_entry(&mut hosts_config, vcs_hostname)?;

        host_mapping.insert(string_value("user"), string_value(user));
        host_mapping.insert(string_value(Self::token_key()), string_value(token));

        let users_mapping = ensure_users_map(host_mapping)?;

        let mut user_entry = Mapping::new();
        user_entry.insert(string_value(Self::token_key()), string_value(token));
        users_mapping.insert(string_value(user), Value::Mapping(user_entry));

        Self::write_hosts(&hosts_config)?;

        Ok(())
    }

    fn auth_status(vcs_hostname: &str) -> Result<String> {
        let hosts_config = Self::read_hosts()?;

        match hosts_config.get(string_value(vcs_hostname)) {
            Some(Value::Mapping(host_mapping)) => {
                let current_user = host_mapping
                    .get(string_value("user"))
                    .and_then(|user_value| user_value.as_str())
                    .unwrap_or("unknown");

                Ok(format!(
                    "Logged in to {vcs_hostname} account {current_user}"
                ))
            }
            _ => Ok(format!(
                "You are not logged into any hosts on {vcs_hostname}"
            )),
        }
    }

    fn auth_switch(vcs_hostname: &str, user: &str) -> Result<()> {
        let mut hosts_config = Self::read_hosts()?;

        let host_mapping = hosts_config
            .get_mut(string_value(vcs_hostname))
            .and_then(|host_value| host_value.as_mapping_mut())
            .ok_or_else(|| anyhow!("not logged in to {vcs_hostname}"))?;

        let users_mapping = host_mapping
            .get(string_value("users"))
            .and_then(|users_value| users_value.as_mapping())
            .ok_or_else(|| anyhow!("no users configured for {vcs_hostname}"))?;

        if !users_mapping.contains_key(string_value(user)) {
            bail!("account {user} not found on {vcs_hostname}");
        }

        let user_token = users_mapping
            .get(string_value(user))
            .and_then(|user_value| user_value.as_mapping())
            .and_then(|user_entry| user_entry.get(string_value(Self::token_key())))
            .and_then(|token_value| token_value.as_str())
            .map(ToOwned::to_owned);

        host_mapping.insert(string_value("user"), string_value(user));

        if let Some(stored_token) = user_token {
            host_mapping.insert(string_value(Self::token_key()), string_value(&stored_token));
        }

        Self::write_hosts(&hosts_config)?;

        Ok(())
    }

    fn api_user(hostname: &str) -> Result<String> {
        let hosts_config = Self::read_hosts()?;

        let host_mapping = hosts_config
            .get(string_value(hostname))
            .and_then(|host_value| host_value.as_mapping())
            .ok_or_else(|| anyhow!("not logged in to {hostname}"))?;

        let oauth_token = host_mapping
            .get(string_value(Self::token_key()))
            .and_then(|token_value| token_value.as_str())
            .ok_or_else(|| {
                anyhow!("no token found for {hostname} (token may be in system keyring)")
            })?;

        Self::fetch_api_user(hostname, oauth_token)
    }
}
