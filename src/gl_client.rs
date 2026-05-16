use std::fs;
use std::path::Path;
use std::path::PathBuf;

use anyhow::Context as _;
use anyhow::Result;
use anyhow::bail;
use gitlab::GitlabBuilder;
use gitlab::api::AsyncQuery;
use gitlab::api::users::CurrentUser;
use serde::Deserialize;
use serde_yaml::Mapping;
use serde_yaml::Value;
use tokio::runtime::Runtime;

use crate::utils::string_value;
use crate::vcs_client::VcsClient;

fn strip_glab_null_tags(configuration_content: &str) -> String {
    // glab CLI tags token values with `!!null` to indicate optional keychain storage.
    // serde_yaml refuses to parse `!!null STRING` since it expects a null value.
    // Stripping these tags preserves the actual token string value.
    configuration_content.replace("!!null ", "")
}

fn read_full_configuration(configuration_path: &Path) -> Result<Mapping> {
    if !configuration_path.exists() {
        return Ok(Mapping::new());
    }

    let configuration_content =
        fs::read_to_string(configuration_path).context("could not read glab config")?;
    if configuration_content.trim().is_empty() {
        return Ok(Mapping::new());
    }

    let sanitized_configuration_content = strip_glab_null_tags(&configuration_content);
    let parsed_configuration: Value = serde_yaml::from_str(&sanitized_configuration_content)
        .context("could not parse glab config")?;

    match parsed_configuration {
        Value::Mapping(configuration_mapping) => Ok(configuration_mapping),
        _ => bail!("glab config is not a YAML mapping"),
    }
}

fn extract_hosts_section(full_configuration: &Mapping) -> Result<Mapping> {
    match full_configuration.get(string_value("hosts")) {
        None => Ok(Mapping::new()),
        Some(Value::Mapping(hosts_mapping)) => Ok(hosts_mapping.clone()),
        Some(_) => bail!("'hosts' key in glab config is not a mapping"),
    }
}

fn merge_hosts_into_configuration(full_configuration: &mut Mapping, hosts_configuration: &Mapping) {
    full_configuration.insert(
        string_value("hosts"),
        Value::Mapping(hosts_configuration.clone()),
    );
}

#[derive(Deserialize)]
struct GitLabCurrentUser {
    username: String,
}

pub struct GlClient;

impl VcsClient for GlClient {
    fn get_hosts_path() -> Result<PathBuf> {
        let configuration_directory =
            dirs::config_dir().context("could not determine config directory")?;

        Ok(configuration_directory.join("glab-cli").join("config.yml"))
    }

    fn token_key() -> &'static str {
        "token"
    }

    fn read_hosts() -> Result<Mapping> {
        let configuration_path = Self::get_hosts_path()?;
        let full_configuration = read_full_configuration(&configuration_path)?;

        extract_hosts_section(&full_configuration)
    }

    fn write_hosts(hosts_configuration: &Mapping) -> Result<()> {
        let configuration_path = Self::get_hosts_path()?;

        if let Some(parent_directory) = configuration_path.parent() {
            fs::create_dir_all(parent_directory).context("could not create config directory")?;
        }

        let mut full_configuration = read_full_configuration(&configuration_path)?;
        merge_hosts_into_configuration(&mut full_configuration, hosts_configuration);

        let serialized_configuration_content =
            serde_yaml::to_string(&Value::Mapping(full_configuration))
                .context("could not serialize glab config")?;

        fs::write(&configuration_path, serialized_configuration_content)
            .context("could not write glab config")
    }

    fn set_default_vcs_host(hostname: &str) -> Result<()> {
        let configuration_path = Self::get_hosts_path()?;
        let mut full_configuration = read_full_configuration(&configuration_path)?;
        full_configuration.insert(string_value("host"), string_value(hostname));
        let serialized_configuration_content =
            serde_yaml::to_string(&Value::Mapping(full_configuration))
                .context("could not serialize glab config")?;

        fs::write(&configuration_path, serialized_configuration_content)
            .context("could not write glab config")
    }

    fn fetch_api_user(hostname: &str, token: &str) -> Result<String> {
        let async_runtime = Runtime::new().context("could not create async runtime")?;

        async_runtime.block_on(async {
            let gitlab_client = GitlabBuilder::new(hostname, token)
                .build_async()
                .await
                .context("could not build GitLab client")?;

            let gitlab_user: GitLabCurrentUser = CurrentUser::builder()
                .build()
                .context("could not build current user endpoint")?
                .query_async(&gitlab_client)
                .await
                .context("could not fetch GitLab user")?;

            Ok(gitlab_user.username)
        })
    }
}

#[cfg(test)]
mod tests {
    use serde_yaml::Mapping;
    use serde_yaml::Value;

    use super::GlClient;
    use super::extract_hosts_section;
    use super::read_full_configuration;
    use super::strip_glab_null_tags;
    use crate::utils::string_value;
    use crate::vcs_client::VcsClient;

    #[test]
    fn test_get_hosts_path_ends_with_glab_cli_config_yml() -> Result<(), anyhow::Error> {
        let hosts_configuration_path = GlClient::get_hosts_path()?;
        assert!(hosts_configuration_path.ends_with("glab-cli/config.yml"));

        Ok(())
    }

    #[test]
    fn test_extract_hosts_section_returns_gitlab_entry() -> Result<(), anyhow::Error> {
        let mut gitlab_entry = Mapping::new();
        gitlab_entry.insert(string_value("token"), string_value("glpat-secret"));

        let mut hosts = Mapping::new();
        hosts.insert(string_value("gitlab.com"), Value::Mapping(gitlab_entry));

        let mut full_configuration = Mapping::new();
        full_configuration.insert(string_value("git_protocol"), string_value("ssh"));
        full_configuration.insert(string_value("host"), string_value("gitlab.com"));
        full_configuration.insert(string_value("hosts"), Value::Mapping(hosts));

        let extracted_hosts_section = extract_hosts_section(&full_configuration)?;
        let gitlab_hosts_entry = extracted_hosts_section
            .get(string_value("gitlab.com"))
            .and_then(|value| value.as_mapping())
            .expect("gitlab.com entry should exist");
        assert_eq!(
            gitlab_hosts_entry.get(string_value("token")),
            Some(&string_value("glpat-secret"))
        );

        Ok(())
    }

    #[test]
    fn test_extract_hosts_section_returns_empty_when_absent() -> Result<(), anyhow::Error> {
        let mut full_configuration = Mapping::new();
        full_configuration.insert(string_value("git_protocol"), string_value("ssh"));

        let extracted_hosts_section = extract_hosts_section(&full_configuration)?;
        assert!(extracted_hosts_section.is_empty());

        Ok(())
    }

    #[test]
    fn test_extract_hosts_section_errors_when_not_a_mapping() {
        let mut full_configuration = Mapping::new();
        full_configuration.insert(string_value("hosts"), string_value("not-a-mapping"));

        let extract_result = extract_hosts_section(&full_configuration);
        assert!(extract_result.is_err());
        assert_eq!(
            extract_result.unwrap_err().to_string(),
            "'hosts' key in glab config is not a mapping"
        );
    }

    #[test]
    fn test_strip_glab_null_tags_removes_null_prefix_from_token() {
        let configuration_content_with_null_tag =
            "token: !!null glpat-secret123\nuser: test_user\n";
        let sanitized_configuration_content =
            strip_glab_null_tags(configuration_content_with_null_tag);
        assert_eq!(
            sanitized_configuration_content,
            "token: glpat-secret123\nuser: test_user\n"
        );
    }

    #[test]
    fn test_strip_glab_null_tags_leaves_plain_token_unchanged() {
        let configuration_content = "token: glpat-secret123\nuser: test_user\n";
        let sanitized_configuration_content = strip_glab_null_tags(configuration_content);
        assert_eq!(
            sanitized_configuration_content,
            "token: glpat-secret123\nuser: test_user\n"
        );
    }

    #[test]
    fn test_strip_glab_null_tags_handles_null_without_value() {
        let configuration_content_with_null_tag = "token: !!null \nuser: test_user\n";
        let sanitized_configuration_content =
            strip_glab_null_tags(configuration_content_with_null_tag);
        assert_eq!(
            sanitized_configuration_content,
            "token: \nuser: test_user\n"
        );
    }

    #[test]
    fn test_read_full_configuration_parses_glab_config_with_null_tagged_token()
    -> Result<(), anyhow::Error> {
        let temporary_directory = tempfile::tempdir()?;
        let configuration_path = temporary_directory.path().join("config.yml");

        std::fs::write(
            &configuration_path,
            indoc::indoc! {"
                git_protocol: ssh
                host: gitlab.com
                hosts:
                    gitlab.com:
                        token: !!null glpat-secret123
                        user: test_user
            "},
        )?;

        let full_configuration = read_full_configuration(&configuration_path)?;
        let hosts_section = full_configuration
            .get(string_value("hosts"))
            .and_then(|value| value.as_mapping())
            .expect("hosts section should exist");
        let gitlab_entry = hosts_section
            .get(string_value("gitlab.com"))
            .and_then(|value| value.as_mapping())
            .expect("gitlab.com entry should exist");

        assert_eq!(
            gitlab_entry.get(string_value("token")),
            Some(&string_value("glpat-secret123"))
        );
        assert_eq!(
            gitlab_entry.get(string_value("user")),
            Some(&string_value("test_user"))
        );

        Ok(())
    }

    #[test]
    fn test_read_full_configuration_returns_empty_when_file_absent() -> Result<(), anyhow::Error> {
        let temporary_directory = tempfile::tempdir()?;
        let configuration_path = temporary_directory.path().join("nonexistent.yml");

        let parsed_configuration = read_full_configuration(&configuration_path)?;
        assert!(parsed_configuration.is_empty());

        Ok(())
    }

    #[test]
    fn test_read_full_configuration_returns_empty_for_empty_file() -> Result<(), anyhow::Error> {
        let temporary_directory = tempfile::tempdir()?;
        let configuration_path = temporary_directory.path().join("config.yml");
        std::fs::write(&configuration_path, "")?;

        let parsed_configuration = read_full_configuration(&configuration_path)?;
        assert!(parsed_configuration.is_empty());

        Ok(())
    }
}
