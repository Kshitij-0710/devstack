use crate::project::FileEntry;
use anyhow::{Context, Result, bail};
use std::fs;
use std::path::{Path, PathBuf};

pub fn sanitize_name(name: &str) -> String {
    let mut slug = String::new();
    let mut previous_dash = false;

    for ch in name.chars() {
        if ch.is_ascii_alphanumeric() {
            slug.push(ch.to_ascii_lowercase());
            previous_dash = false;
        } else if !previous_dash {
            slug.push('-');
            previous_dash = true;
        }
    }

    slug.trim_matches('-').to_string()
}

pub fn validate_slug(slug: &str) -> Result<()> {
    if slug.is_empty() {
        bail!("project name must include at least one letter or number");
    }

    Ok(())
}

pub fn resolve_target_dir(cwd: &Path, slug: &str, dir: Option<PathBuf>) -> PathBuf {
    match dir {
        Some(path) if path.is_absolute() => path,
        Some(path) => cwd.join(path),
        None => cwd.join(slug),
    }
}

pub fn ensure_target_dir(path: &Path, force: bool) -> Result<()> {
    if !path.exists() {
        return Ok(());
    }

    let mut entries =
        fs::read_dir(path).with_context(|| format!("failed to read {}", path.display()))?;

    if entries.next().is_none() || force {
        return Ok(());
    }

    bail!(
        "target directory {} already exists and is not empty; use --force to continue",
        path.display()
    )
}

pub fn write_entry(root: &Path, entry: &FileEntry) -> Result<()> {
    let full_path = root.join(entry.path);

    if let Some(parent) = full_path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }

    fs::write(&full_path, &entry.contents)
        .with_context(|| format!("failed to write {}", full_path.display()))?;

    #[cfg(unix)]
    if entry.executable {
        use std::os::unix::fs::PermissionsExt;
        let mut permissions = fs::metadata(&full_path)?.permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&full_path, permissions)?;
    }

    Ok(())
}
