//! Linux `.desktop` entry install / remove for app launchers.
//!
//! Apps supply identity via [`Entry`]; vibekit handles Exec quoting, the
//! generated-file marker, and `update-desktop-database`.

use std::io;
use std::path::{Path, PathBuf};

/// Identity of a desktop entry the app installs under
/// `$XDG_DATA_HOME/applications` (or `~/.local/share/applications`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    /// File stem: `{id}.desktop`.
    pub id: &'static str,
    pub name: &'static str,
    pub comment: &'static str,
    /// Full marker line, e.g. `X-Boxvibe-Generated=true`. Only a file with this
    /// line is overwritten or deleted.
    pub marker: &'static str,
    pub icon: &'static str,
    /// `Categories=` value (trailing semicolon included by convention).
    pub categories: &'static str,
    /// `Keywords=` value (trailing semicolon included by convention).
    pub keywords: &'static str,
}

impl Entry {
    /// `$XDG_DATA_HOME/applications`, else `~/.local/share/applications`.
    pub fn applications_dir() -> Option<PathBuf> {
        let data = std::env::var_os("XDG_DATA_HOME")
            .map(PathBuf::from)
            .filter(|p| p.is_absolute())
            .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/share")))?;
        Some(data.join("applications"))
    }

    pub fn path_in(&self, dir: &Path) -> PathBuf {
        dir.join(format!("{}.desktop", self.id))
    }

    pub fn installed(&self, dir: &Path) -> bool {
        self.path_in(dir).is_file()
    }

    /// `exe` as one Exec argument, quoted and escaped as the desktop entry spec
    /// asks: inside double quotes `"` `` ` `` `$` `\` get a backslash, then the
    /// value's own escaping doubles every backslash; `%` is written `%%`.
    pub fn exec_arg(exe: &Path) -> String {
        let s = exe.to_string_lossy();
        let reserved = |c: char| " \t\n\"'\\><~|&;$*?#()`".contains(c);
        let s = if s.contains(reserved) {
            let mut q = String::from("\"");
            for c in s.chars() {
                if matches!(c, '"' | '`' | '$' | '\\') {
                    q.push('\\');
                }
                q.push(c);
            }
            q.push('"');
            q.replace('\\', "\\\\")
        } else {
            s.into_owned()
        };
        s.replace('%', "%%")
    }

    pub fn contents(&self, exe: &Path) -> String {
        format!(
            "[Desktop Entry]\n\
             Type=Application\n\
             Name={}\n\
             Comment={}\n\
             Exec={}\n\
             Icon={}\n\
             Terminal=true\n\
             Categories={}\n\
             Keywords={}\n\
             {}\n",
            self.name,
            self.comment,
            Self::exec_arg(exe),
            self.icon,
            self.categories,
            self.keywords,
            self.marker
        )
    }

    /// Writes the entry (replacing one this app wrote) for `exe`.
    pub fn install(&self, dir: &Path, exe: &Path) -> io::Result<PathBuf> {
        let path = self.path_in(dir);
        if path.exists() && !self.generated(&path) {
            return Err(io::Error::other(format!("a {}.desktop this app didn't write is already there", self.id)));
        }
        std::fs::create_dir_all(dir)?;
        std::fs::write(&path, self.contents(exe))?;
        refresh(dir);
        Ok(path)
    }

    /// Deletes the entry, only if this app wrote it.
    pub fn remove(&self, dir: &Path) -> io::Result<PathBuf> {
        let path = self.path_in(dir);
        if !self.generated(&path) {
            return Err(io::Error::other("this app didn't write it, so it is left alone"));
        }
        std::fs::remove_file(&path)?;
        refresh(dir);
        Ok(path)
    }

    fn generated(&self, path: &Path) -> bool {
        std::fs::read_to_string(path).is_ok_and(|s| s.lines().any(|l| l.trim() == self.marker))
    }
}

/// Lets launchers that use the cache see the change; fine if it's missing.
fn refresh(dir: &Path) {
    let _ = std::process::Command::new("update-desktop-database")
        .arg(dir)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status();
}

/// The running binary, symlinks resolved.
pub fn current_exe() -> io::Result<PathBuf> {
    std::env::current_exe()?.canonicalize()
}

#[cfg(test)]
mod tests {
    use super::*;

    const DEMO: Entry = Entry {
        id: "vibekit-demo",
        name: "vibekit-demo",
        comment: "demo",
        marker: "X-Vibekit-Demo-Generated=true",
        icon: "application-x-executable",
        categories: "Utility;",
        keywords: "test;",
    };

    fn temp(name: &str) -> PathBuf {
        crate::temp_dir(&format!("desktop-{name}"))
    }

    #[test]
    fn installs_and_removes_its_own_entry() {
        let dir = temp("own").join("applications");
        assert!(!DEMO.installed(&dir));
        let path = DEMO.install(&dir, Path::new("/usr/bin/vibekit-demo")).unwrap();
        assert!(DEMO.installed(&dir));
        let text = std::fs::read_to_string(&path).unwrap();
        for line in [
            "Type=Application",
            "Name=vibekit-demo",
            "Exec=/usr/bin/vibekit-demo",
            "Terminal=true",
            "Categories=Utility;",
            "Keywords=test;",
            DEMO.marker,
        ] {
            assert!(text.lines().any(|l| l == line), "{line} in\n{text}");
        }
        DEMO.remove(&dir).unwrap();
        assert!(!DEMO.installed(&dir));
        let _ = std::fs::remove_dir_all(dir.parent().unwrap());
    }

    #[test]
    fn never_deletes_or_overwrites_a_file_it_didnt_write() {
        let dir = temp("theirs");
        std::fs::create_dir_all(&dir).unwrap();
        let mine = "[Desktop Entry]\nType=Application\nName=other\nExec=other\n";
        std::fs::write(DEMO.path_in(&dir), mine).unwrap();
        assert!(DEMO.remove(&dir).is_err());
        assert!(DEMO.install(&dir, Path::new("/x/demo")).is_err());
        assert_eq!(std::fs::read_to_string(DEMO.path_in(&dir)).unwrap(), mine);
        assert!(DEMO.remove(&temp("none")).is_err(), "nothing there");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn exec_paths_are_quoted_per_the_spec() {
        assert_eq!(Entry::exec_arg(Path::new("/home/t/.cargo/bin/app")), "/home/t/.cargo/bin/app");
        assert_eq!(Entry::exec_arg(Path::new("/home/t/my apps/app")), "\"/home/t/my apps/app\"");
        assert_eq!(Entry::exec_arg(Path::new("/a b/$x\"y")), r#""/a b/\\$x\\"y""#);
        assert_eq!(Entry::exec_arg(Path::new("/opt/100%/app")), "/opt/100%%/app");
    }
}
