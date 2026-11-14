use std::path::{Path, PathBuf};

/// The on-disk layout under the application data directory
/// (`~/Library/Application Support/Milano Pen` on macOS).
#[derive(Debug, Clone)]
pub struct AppPaths {
    pub root: PathBuf,
    pub database_dir: PathBuf,
    pub assets: PathBuf,
    pub thumbnails: PathBuf,
    pub cache: PathBuf,
    pub backups: PathBuf,
}

impl AppPaths {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        let root = root.into();
        Self {
            database_dir: root.join("database"),
            assets: root.join("assets"),
            thumbnails: root.join("thumbnails"),
            cache: root.join("cache"),
            backups: root.join("backups"),
            root,
        }
    }

    pub fn database_file(&self) -> PathBuf {
        self.database_dir.join("milano-pen.sqlite")
    }

    /// Create every directory. User data directories are private to the user.
    pub fn ensure(&self) -> std::io::Result<()> {
        for dir in [&self.root, &self.database_dir, &self.assets, &self.thumbnails, &self.cache, &self.backups] {
            std::fs::create_dir_all(dir)?;
            restrict(dir)?;
        }
        Ok(())
    }
}

#[cfg(unix)]
fn restrict(dir: &Path) -> std::io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(dir, std::fs::Permissions::from_mode(0o700))
}

#[cfg(not(unix))]
fn restrict(_dir: &Path) -> std::io::Result<()> {
    Ok(())
}
