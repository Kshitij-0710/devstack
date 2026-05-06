mod common;
mod shared;
mod stacks;

use crate::project::{FileEntry, Project, Stack};
use anyhow::{Context, Result};
use std::env;
use std::fs;
use std::path::PathBuf;

pub fn init(stack: Stack, name: Option<String>, dir: Option<PathBuf>, force: bool) -> Result<()> {
    let cwd = env::current_dir().context("failed to determine current directory")?;
    let project_name = name.unwrap_or_else(|| stack.default_project_name().to_string());
    let slug = common::sanitize_name(&project_name);

    common::validate_slug(&slug)?;

    let target_dir = common::resolve_target_dir(&cwd, &slug, dir);
    common::ensure_target_dir(&target_dir, force)?;

    let project = Project {
        stack,
        name: project_name,
        slug,
        dir: target_dir,
    };

    fs::create_dir_all(&project.dir)
        .with_context(|| format!("failed to create {}", project.dir.display()))?;

    for entry in files_for(&project) {
        common::write_entry(&project.dir, &entry)?;
    }

    println!("Created {} at {}", project.slug, project.dir.display());
    println!("Next steps:");
    println!("  cd {}", project.dir.display());
    println!("  docker compose up --build");
    println!("  visit http://localhost");

    Ok(())
}

fn files_for(project: &Project) -> Vec<FileEntry> {
    let mut files = shared::base_files(project);
    files.extend(stacks::files_for(project));
    files
}

#[cfg(test)]
mod tests {
    use super::common::sanitize_name;
    use crate::project::Stack;

    #[test]
    fn sanitizes_names() {
        assert_eq!(sanitize_name("My Fancy App"), "my-fancy-app");
        assert_eq!(sanitize_name("api__service"), "api-service");
        assert_eq!(sanitize_name("___"), "");
    }

    #[test]
    fn stack_metadata_is_stable() {
        assert_eq!(Stack::Node.default_project_name(), "node-service");
        assert_eq!(Stack::Django.default_project_name(), "django-service");
        assert_eq!(Stack::Fastapi.app_port(), 8000);
    }
}
