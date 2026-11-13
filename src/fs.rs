//! Small filesystem helpers shared by *vibe apps.

use std::io::Write;
use std::path::Path;

/// Writes `text` to `path` with mode 600 (owner only), creating the parent
/// folder at mode 700 so others cannot list it for the file name.
///
/// Written next to the path and renamed, so a crash never leaves a half-written
/// file. The temporary name carries the pid, so two processes writing the same
/// path at once cannot clobber each other's temp.
pub fn write_private(path: &Path, text: &str) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
        #[cfg(unix)]
        std::fs::set_permissions(dir, std::os::unix::fs::PermissionsExt::from_mode(0o700))?;
    }
    let tmp = path.with_extension(format!("toml.{}.tmp", std::process::id()));
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create(true).truncate(true);
    #[cfg(unix)]
    std::os::unix::fs::OpenOptionsExt::mode(&mut options, 0o600);
    let mut file = options.open(&tmp)?;
    file.write_all(text.as_bytes())?;
    file.sync_all()?;
    drop(file);
    #[cfg(unix)]
    std::fs::set_permissions(&tmp, std::os::unix::fs::PermissionsExt::from_mode(0o600))?;
    std::fs::rename(&tmp, path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn write_private_creates_the_file_atomically() {
        let dir = crate::temp_dir("write-private");
        let path = dir.join("config.toml");
        write_private(&path, "hello = true\n").unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "hello = true\n");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(std::fs::metadata(&path).unwrap().permissions().mode() & 0o777, 0o600);
            assert_eq!(std::fs::metadata(&dir).unwrap().permissions().mode() & 0o777, 0o700);
        }
        let _ = std::fs::remove_dir_all(&dir);
    }
}
