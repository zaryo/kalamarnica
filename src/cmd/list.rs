use anyhow::Result;
use clap::Parser;

use crate::cmd::handler::Handler;
use crate::context::Context;
use crate::storage::Storage;
use crate::vcs::Vcs;

#[derive(Parser)]
pub struct List;

impl List {
    pub fn execute(&self, storage: &Storage) -> Result<()> {
        let contexts = storage.list_contexts()?;
        let active = storage.get_active()?;

        if contexts.is_empty() {
            log::info!(
                "No contexts found. Create one with: kalamarnica create --name <name> --vcs <vcs> --hostname <host> --user <user>"
            );

            return Ok(());
        }

        for (vcs_str, names) in &contexts {
            let vcs: Vcs = vcs_str.parse()?;

            for name in names {
                let context = storage.read_context(name, vcs)?;
                let active_marker = match active
                    .as_ref()
                    .is_some_and(|active_ctx| active_ctx.name == *name && active_ctx.vcs == vcs)
                {
                    true => " *",
                    false => "",
                };

                let context_line = format_context_line(name, active_marker, &context, vcs_str);

                log::info!("{context_line}");
            }
        }

        Ok(())
    }
}

impl Handler for List {
    fn handle(&self, storage: &Storage) -> Result<()> {
        self.execute(storage)
    }
}

fn format_context_line(
    name: &str,
    active_marker: &str,
    context: &Context,
    vcs_str: &str,
) -> String {
    format!(
        "{name}{active_marker} ({}@{}, {}, {vcs_str})",
        context.user, context.hostname, context.transport
    )
}

#[cfg(test)]
mod tests {
    use super::List;
    use super::format_context_line;
    use crate::apply_context::ApplyContext;
    use crate::cmd::handler::Handler;
    use crate::storage::Storage;
    use crate::test_utils::sample_context;
    use crate::test_utils::sample_gitlab_context;
    use crate::vcs::Vcs;

    #[test]
    fn test_list_empty_contexts_succeeds() -> Result<(), anyhow::Error> {
        let tmp = tempfile::tempdir()?;
        let storage = Storage::with_base_dir(tmp.path().to_path_buf())?;
        let handler = List;

        handler.handle(&storage)?;

        Ok(())
    }

    #[test]
    fn test_list_single_context_succeeds() -> Result<(), anyhow::Error> {
        let tmp = tempfile::tempdir()?;
        let storage = Storage::with_base_dir(tmp.path().to_path_buf())?;
        storage.write_context("work", Vcs::Github, &sample_context())?;

        let handler = List;
        handler.handle(&storage)?;

        Ok(())
    }

    #[test]
    fn test_list_multiple_contexts_succeeds() -> Result<(), anyhow::Error> {
        let tmp = tempfile::tempdir()?;
        let storage = Storage::with_base_dir(tmp.path().to_path_buf())?;
        storage.write_context("work", Vcs::Github, &sample_context())?;
        storage.write_context("personal", Vcs::Gitlab, &sample_gitlab_context())?;

        let handler = List;
        handler.handle(&storage)?;

        Ok(())
    }

    #[test]
    fn test_list_with_active_context_succeeds() -> Result<(), anyhow::Error> {
        let tmp = tempfile::tempdir()?;
        let storage = Storage::with_base_dir(tmp.path().to_path_buf())?;
        storage.write_context("work", Vcs::Github, &sample_context())?;
        storage.write_context("personal", Vcs::Github, &sample_context())?;
        let active = ApplyContext {
            name: "work".to_owned(),
            vcs: Vcs::Github,
        };
        storage.set_active(&active)?;

        let handler = List;
        handler.handle(&storage)?;

        Ok(())
    }

    #[test]
    fn test_list_same_name_across_vcs_succeeds() -> Result<(), anyhow::Error> {
        let tmp = tempfile::tempdir()?;
        let storage = Storage::with_base_dir(tmp.path().to_path_buf())?;
        storage.write_context("work", Vcs::Github, &sample_context())?;
        storage.write_context("work", Vcs::Gitlab, &sample_gitlab_context())?;

        let handler = List;
        handler.handle(&storage)?;

        Ok(())
    }

    #[test]
    fn test_format_context_line_active() {
        let context = sample_context();
        let line = format_context_line("work", " *", &context, "github");
        assert_eq!(line, "work * (testuser@github.com, ssh, github)");
    }

    #[test]
    fn test_format_context_line_inactive() {
        let context = sample_context();
        let line = format_context_line("work", "", &context, "github");
        assert_eq!(line, "work (testuser@github.com, ssh, github)");
    }
}
