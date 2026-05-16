use anyhow::Result;
use git2::Repository;

pub fn repo_root() -> Result<Option<String>> {
    let Ok(repository) = Repository::discover(".") else {
        return Ok(None);
    };

    let workdir_path = match repository.workdir() {
        Some(workdir) => workdir.to_string_lossy().trim_end_matches('/').to_owned(),
        None => return Ok(None),
    };

    Ok(Some(workdir_path))
}

#[cfg(test)]
mod tests {
    use super::repo_root;
    use crate::test_utils::CWD_MUTEX;

    #[test]
    fn test_returns_some_inside_git_repo() -> Result<(), anyhow::Error> {
        let _guard = CWD_MUTEX
            .lock()
            .map_err(|poison_error| anyhow::anyhow!("{poison_error}"))?;
        let original_cwd = std::env::current_dir()?;

        let tmp = tempfile::tempdir()?;
        git2::Repository::init(tmp.path())?;
        std::env::set_current_dir(tmp.path())?;

        let result = repo_root()?;
        std::env::set_current_dir(&original_cwd)?;

        assert!(result.is_some());
        let root_path = result.as_deref().map_or("", |path| path);
        assert!(root_path.contains(tmp.path().to_string_lossy().as_ref()));

        Ok(())
    }

    #[test]
    fn test_returns_none_for_bare_repo() -> Result<(), anyhow::Error> {
        let _guard = CWD_MUTEX
            .lock()
            .map_err(|poison_error| anyhow::anyhow!("{poison_error}"))?;
        let original_cwd = std::env::current_dir()?;

        let tmp = tempfile::tempdir()?;
        git2::Repository::init_bare(tmp.path())?;
        std::env::set_current_dir(tmp.path())?;

        let result = repo_root()?;
        std::env::set_current_dir(&original_cwd)?;

        assert!(result.is_none());

        Ok(())
    }
}
