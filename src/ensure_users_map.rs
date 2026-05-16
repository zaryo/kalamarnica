use anyhow::Context;
use anyhow::Result;
use serde_yaml::Mapping;
use serde_yaml::Value;

use crate::utils::string_value;

pub fn ensure_users_map(host_mapping: &mut Mapping) -> Result<&mut Mapping> {
    let users_key = string_value("users");
    if !host_mapping.contains_key(&users_key) {
        host_mapping.insert(users_key.clone(), Value::Mapping(Mapping::new()));
    }

    host_mapping
        .get_mut(&users_key)
        .and_then(|users_value| users_value.as_mapping_mut())
        .context("users entry is not a mapping")
}

#[cfg(test)]
mod tests {
    use serde_yaml::Mapping;
    use serde_yaml::Value;

    use super::ensure_users_map;
    use crate::utils::string_value;

    #[test]
    fn test_users_map_creates_new_entry() -> Result<(), anyhow::Error> {
        let mut host = Mapping::new();
        let users = ensure_users_map(&mut host)?;
        assert!(users.is_empty());
        assert!(host.contains_key(&string_value("users")));

        Ok(())
    }

    #[test]
    fn test_users_map_returns_existing_entry() -> Result<(), anyhow::Error> {
        let mut host = Mapping::new();
        let mut users = Mapping::new();
        users.insert(string_value("octocat"), Value::Mapping(Mapping::new()));
        host.insert(string_value("users"), Value::Mapping(users));

        let users_ref = ensure_users_map(&mut host)?;
        assert!(users_ref.contains_key(&string_value("octocat")));

        Ok(())
    }

    #[test]
    fn test_ensure_users_map_does_not_overwrite_existing() -> Result<(), anyhow::Error> {
        let mut host = Mapping::new();
        let mut users = Mapping::new();
        users.insert(string_value("user1"), Value::Mapping(Mapping::new()));
        users.insert(string_value("user2"), Value::Mapping(Mapping::new()));
        host.insert(string_value("users"), Value::Mapping(users));

        let users_ref = ensure_users_map(&mut host)?;
        assert_eq!(users_ref.len(), 2);

        Ok(())
    }

    #[test]
    fn test_users_map_works_for_gitlab_users() -> Result<(), anyhow::Error> {
        let mut host = Mapping::new();
        let mut users = Mapping::new();
        users.insert(string_value("gitlabuser"), Value::Mapping(Mapping::new()));
        host.insert(string_value("users"), Value::Mapping(users));

        let users_ref = ensure_users_map(&mut host)?;
        assert!(users_ref.contains_key(&string_value("gitlabuser")));

        Ok(())
    }
}
