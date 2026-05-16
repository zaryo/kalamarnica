use anyhow::Context;
use anyhow::Result;
use serde_yaml::Mapping;
use serde_yaml::Value;

use crate::utils::string_value;

pub fn ensure_host_entry<'hosts_config>(
    hosts_config: &'hosts_config mut Mapping,
    hostname: &str,
) -> Result<&'hosts_config mut Mapping> {
    let hostname_key = string_value(hostname);
    if !hosts_config.contains_key(&hostname_key) {
        hosts_config.insert(hostname_key.clone(), Value::Mapping(Mapping::new()));
    }

    hosts_config
        .get_mut(&hostname_key)
        .and_then(Value::as_mapping_mut)
        .context("host entry is not a mapping")
}

#[cfg(test)]
mod tests {
    use serde_yaml::Mapping;
    use serde_yaml::Value;

    use super::ensure_host_entry;
    use crate::utils::string_value;

    #[test]
    fn test_host_entry_creates_new_entry() -> Result<(), anyhow::Error> {
        let mut config = Mapping::new();
        let entry = ensure_host_entry(&mut config, "github.com")?;
        assert!(entry.is_empty());
        assert!(config.contains_key(&string_value("github.com")));

        Ok(())
    }

    #[test]
    fn test_host_entry_returns_existing_entry() -> Result<(), anyhow::Error> {
        let mut config = Mapping::new();
        let mut existing = Mapping::new();
        existing.insert(string_value("user"), string_value("octocat"));
        config.insert(string_value("github.com"), Value::Mapping(existing));

        let entry = ensure_host_entry(&mut config, "github.com")?;
        assert_eq!(
            entry.get(string_value("user")),
            Some(&string_value("octocat"))
        );

        Ok(())
    }

    #[test]
    fn test_host_entry_does_not_overwrite_existing() -> Result<(), anyhow::Error> {
        let mut config = Mapping::new();
        let mut existing = Mapping::new();
        existing.insert(string_value("user"), string_value("octocat"));
        existing.insert(string_value("oauth_token"), string_value("ghp_abc"));
        config.insert(string_value("github.com"), Value::Mapping(existing));

        let entry = ensure_host_entry(&mut config, "github.com")?;
        assert_eq!(entry.len(), 2);

        Ok(())
    }

    #[test]
    fn test_host_entry_creates_entry_for_gitlab_hostname() -> Result<(), anyhow::Error> {
        let mut config = Mapping::new();
        let entry = ensure_host_entry(&mut config, "gitlab.com")?;
        assert!(entry.is_empty());
        assert!(config.contains_key(&string_value("gitlab.com")));

        Ok(())
    }
}
