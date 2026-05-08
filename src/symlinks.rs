use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

/// Create a symlink in `link_dir` pointing to `target_dir`, both joined with `name`.
///
/// - `target_dir`: the directory that the symlink will point to
/// - `link_dir`: the directory where the symlink will be created
/// - `name`: the name of the symlink entry
///
/// If an existing symlink or file exists at the link path, it is removed first.
/// If `link_dir` does not exist, it is created.
pub fn create_symlink(target_dir: &str, link_dir: &str, name: &str) -> Result<()> {
    let link_path = Path::new(link_dir);
    if !link_path.exists() {
        fs::create_dir_all(link_path)
            .with_context(|| format!("Failed to create {} directory", link_dir))?;
    }

    let symlink_path = link_path.join(name);
    let target_path = Path::new(target_dir).join(name);

    #[cfg(unix)]
    {
        use std::os::unix::fs::symlink;
        if symlink_path.exists() || symlink_path.symlink_metadata().is_ok() {
            fs::remove_file(&symlink_path)
                .with_context(|| format!("Failed to remove existing symlink at {}", symlink_path.display()))?;
        }
        symlink(&target_path, &symlink_path)
            .with_context(|| format!("Failed to create symlink from {} to {}", symlink_path.display(), target_path.display()))?;
    }

    #[cfg(windows)]
    {
        use std::os::windows::fs::symlink_dir;
        if symlink_path.exists() || symlink_path.symlink_metadata().is_ok() {
            fs::remove_dir(&symlink_path)
                .with_context(|| format!("Failed to remove existing symlink at {}", symlink_path.display()))?;
        }
        symlink_dir(&target_path, &symlink_path)
            .with_context(|| format!("Failed to create symlink from {} to {}", symlink_path.display(), target_path.display()))?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn setup_test_dir() -> tempfile::TempDir {
        tempfile::tempdir().unwrap()
    }

    #[test]
    fn test_create_symlink_success() {
        let dir = setup_test_dir();
        let target_dir = dir.path().join("target");
        let link_dir = dir.path().join("links");

        fs::create_dir_all(&target_dir).unwrap();
        fs::write(target_dir.join("mylink"), "hello").unwrap();

        create_symlink(
            target_dir.to_str().unwrap(),
            link_dir.to_str().unwrap(),
            "mylink",
        )
        .unwrap();

        let symlink_path = link_dir.join("mylink");
        assert!(symlink_path.exists() || symlink_path.symlink_metadata().is_ok());
    }

    #[test]
    fn test_create_symlink_creates_link_dir() {
        let dir = setup_test_dir();
        let target_dir = dir.path().join("target");
        let link_dir = dir.path().join("nested/links");

        fs::create_dir_all(&target_dir).unwrap();
        fs::write(target_dir.join("mylink"), "hello").unwrap();

        create_symlink(
            target_dir.to_str().unwrap(),
            link_dir.to_str().unwrap(),
            "mylink",
        )
        .unwrap();

        assert!(link_dir.exists());
        let symlink_path = link_dir.join("mylink");
        assert!(symlink_path.exists() || symlink_path.symlink_metadata().is_ok());
    }

    #[test]
    fn test_create_symlink_idempotent() {
        let dir = setup_test_dir();
        let target_dir = dir.path().join("target");
        let link_dir = dir.path().join("links");

        fs::create_dir_all(&target_dir).unwrap();
        fs::write(target_dir.join("mylink"), "hello").unwrap();

        // First call
        create_symlink(
            target_dir.to_str().unwrap(),
            link_dir.to_str().unwrap(),
            "mylink",
        )
        .unwrap();

        // Second call should not fail
        create_symlink(
            target_dir.to_str().unwrap(),
            link_dir.to_str().unwrap(),
            "mylink",
        )
        .unwrap();

        let symlink_path = link_dir.join("mylink");
        assert!(symlink_path.exists() || symlink_path.symlink_metadata().is_ok());
    }

    #[test]
    fn test_create_symlink_missing_target() {
        let dir = setup_test_dir();
        let target_dir = dir.path().join("nonexistent");
        let link_dir = dir.path().join("links");

        // Symlink creation should succeed even if target doesn't exist (on Unix)
        // On Windows with symlink_dir, it may also succeed
        let result = create_symlink(
            target_dir.to_str().unwrap(),
            link_dir.to_str().unwrap(),
            "mylink",
        );

        // The function itself should not error; the symlink may be dangling
        assert!(result.is_ok());
    }
}
