use std::fs::{self, OpenOptions};
use std::path::{Path, PathBuf};

use color_eyre::Result;
use log::*;
use nix::mount::{mount, MsFlags};

pub struct FsDriver;

#[allow(unused)]
impl FsDriver {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self {}
    }

    pub fn all_containers_root(&self) -> PathBuf {
        PathBuf::from("/tmp/boxxy-containers")
    }

    pub fn container_root(&self, name: &str) -> PathBuf {
        append_all(&self.all_containers_root(), vec![name])
    }

    pub fn setup_root(&self, name: &str) -> Result<()> {
        debug!("setting up root for {}", name);
        fs::create_dir_all(self.container_root(name))?;
        Ok(())
    }

    pub fn cleanup_root(&self, name: &str) -> Result<()> {
        debug!("cleaning up root for {}", name);
        fs::remove_dir_all(self.container_root(name))?;
        Ok(())
    }

    pub fn bind_mount_ro(&self, src: &Path, target: &Path) -> Result<()> {
        debug!("bind mount {src:?} onto {target:?} as ro");
        // ro bindmount is a complicated procedure: https://unix.stackexchange.com/a/128388
        // tldr: You first do a normal bindmount, then remount bind+ro
        self.bind_mount(src, target, MsFlags::MS_BIND)?;
        self.remount_ro(target)?;
        Ok(())
    }

    pub fn remount_ro(&self, target: &Path) -> Result<()> {
        debug!("remount {target:?} as ro");
        mount::<Path, Path, str, str>(
            None,
            target,
            Some(""),
            MsFlags::MS_REMOUNT | MsFlags::MS_BIND | MsFlags::MS_RDONLY,
            Some(""),
        )?;
        Ok(())
    }

    pub fn bind_mount_rw(&self, src: &Path, target: &Path) -> Result<()> {
        debug!("bind mount {src:?} onto {target:?} as rw");
        self.bind_mount(src, target, MsFlags::MS_BIND)
    }

    fn bind_mount(&self, src: &Path, target: &Path, flags: MsFlags) -> Result<()> {
        debug!("bind mount {src:?} onto {target:?}");
        mount(
            Some(src),
            target,
            Some(""),
            MsFlags::MS_REC | flags,
            Some(""),
        )?;
        Ok(())
    }

    pub fn touch(&self, path: &Path) -> Result<()> {
        debug!("touching {path:?}");
        match OpenOptions::new().create(true).write(true).open(path) {
            Ok(_) => Ok(()),
            Err(e) => Err(e.into()),
        }
    }

    pub fn touch_dir(&self, path: &Path) -> Result<()> {
        debug!("touching dir {path:?}");
        match fs::create_dir_all(path) {
            Ok(_) => Ok(()),
            Err(e) => Err(e.into()),
        }
    }
}

pub fn append_all<P: AsRef<Path>>(buf: &Path, parts: Vec<P>) -> PathBuf {
    let mut buf = buf.to_path_buf();
    for part in parts {
        let path = part.as_ref();
        let path = if path.starts_with("/") {
            path.strip_prefix("/").unwrap()
        } else {
            path
        };

        buf.push(path);
    }
    buf
}

#[cfg(test)]
mod tests {
    use super::*;

    use color_eyre::Result;

    #[test]
    fn test_append_all() {
        let buf = PathBuf::from("/tmp");
        let parts = vec!["foo", "bar", "baz"];
        let expected = PathBuf::from("/tmp/foo/bar/baz");
        assert_eq!(append_all(&buf, parts), expected);
    }

    #[test]
    fn test_fs_driver_creates_and_destroys_roots() -> Result<()> {
        let driver = FsDriver::new();
        let name = "test-create-destroy-root";
        let root = driver.container_root(name);
        driver.setup_root(name)?;
        assert!(root.exists());
        driver.cleanup_root(name)?;
        assert!(!root.exists());

        Ok(())
    }

    #[test]
    fn test_fs_driver_touches_files() -> Result<()> {
        let driver = FsDriver::new();
        let name = "test-touch-file";
        let root = driver.container_root(name);
        driver.setup_root(name)?;
        let file = append_all(&root, vec!["foo"]);
        driver.touch(&file)?;
        assert!(file.exists());
        driver.cleanup_root(name)?;
        assert!(!root.exists());

        Ok(())
    }

    #[test]
    fn test_fs_driver_touches_dirs() -> Result<()> {
        let driver = FsDriver::new();
        let name = "test-touch-dir";
        let root = driver.container_root(name);
        driver.setup_root(name)?;
        let dir = append_all(&root, vec!["foo"]);
        driver.touch_dir(&dir)?;
        assert!(dir.exists());
        driver.cleanup_root(name)?;
        assert!(!root.exists());

        Ok(())
    }
}
