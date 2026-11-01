//! Is there a newer release? Asks GitHub (the app decides when, usually in
//! the background at startup) and, when there is one, says how to get it on
//! this machine. Where the release archive fits and the program's folder is
//! writable, [`Release::install`] downloads it and puts the new program in
//! place of the running one.
//!
//! Each app describes itself with a [`Release`]: its name (the binary, the
//! tarball prefix `<name>-<arch>-linux.tar.gz` holding `<name>/<name>`), its
//! GitHub repo and its version.

use std::env::consts::{ARCH, OS};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::Duration;

use serde_json::Value;

/// Who is asking: one app's name, GitHub repo and version.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Release {
    /// The binary's name, e.g. `boxvibe`.
    pub name: &'static str,
    /// `owner/repo` on GitHub, e.g. `mortbrew/boxvibe`.
    pub repo: &'static str,
    /// This build's version (`env!("CARGO_PKG_VERSION")` of the app).
    pub version: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Update {
    /// `0.2.0`.
    pub version: String,
    /// A command to paste in a terminal, for when Update Now can't.
    pub how: String,
    /// Whether Update Now can put the new version in place here, or why not.
    pub replace: Replace,
}

/// Can the app update itself in place, and if not, why.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Replace {
    /// Download this archive and swap its program in for the running one.
    Ready(String),
    /// Running from a cargo `target` folder: build the new version instead.
    FromSource,
    /// Installed with `cargo install`: reinstall the same way.
    Cargo,
    /// Installed by a package manager (`/usr/bin`…): update the package.
    Package,
    /// The program's folder can't be written to.
    ReadOnly,
    /// No release file for this machine.
    NoAsset,
}

impl Replace {
    /// Why Update Now can't, in a few words (`None`: it can).
    pub fn why_not(&self) -> Option<&'static str> {
        match self {
            Self::Ready(_) => None,
            Self::FromSource => Some("Built from source: run git pull && cargo build --release"),
            Self::Cargo => Some("Installed with cargo install: reinstall it the same way"),
            Self::Package => Some("Installed through a package manager: update it there, not from GitHub"),
            Self::ReadOnly => Some("The program's folder isn't writable"),
            Self::NoAsset => Some("No download for this system"),
        }
    }
}

/// Where a self-update stands.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum Progress {
    #[default]
    Idle,
    /// Downloading: percent done.
    Downloading(u8),
    /// The new version is in place: Restart Now starts it.
    Done(String),
    Failed(String),
}

/// What asking GitHub found out.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Checked {
    Newer(Update),
    /// GitHub's latest release is this version (or older).
    Latest,
    /// Offline, no release yet, rate limited, or a tag that isn't a version.
    Unknown,
}

impl Release {
    /// `https://github.com/<repo>`.
    pub fn repo_url(&self) -> String {
        format!("https://github.com/{}", self.repo)
    }

    fn download_url(&self) -> String {
        format!("https://github.com/{}/releases/download", self.repo)
    }

    /// This build's version, or `<NAME>_FAKE_VERSION` (to try the update flow
    /// against an existing release, e.g. `BOXVIBE_FAKE_VERSION=0.0.1 boxvibe`).
    fn current_version(&self) -> String {
        std::env::var(format!("{}_FAKE_VERSION", self.name.to_uppercase()))
            .ok()
            .filter(|v| version(v).is_some())
            .unwrap_or_else(|| self.version.to_owned())
    }

    /// Asks GitHub for the latest release and compares it with this build.
    pub fn check(&self) -> Checked {
        let Ok(Some((tag, assets))) = self.fetch_latest() else { return Checked::Unknown };
        let current = self.current_version();
        let exe = running_exe();
        let exe_path = exe.display().to_string();
        match self.update_for(&current, &tag, &assets, &exe_path, Packaged::detect(&exe_path), OS, ARCH) {
            Some(mut update) => {
                if matches!(update.replace, Replace::Ready(_)) && !exe.parent().is_some_and(|d| self.writable(d)) {
                    update.replace = Replace::ReadOnly;
                }
                Checked::Newer(update)
            }
            None => latest_or_unknown(&current, &tag),
        }
    }

    fn writable(&self, dir: &Path) -> bool {
        let probe = dir.join(format!(".{}-write-test-{}", self.name, std::process::id()));
        let ok = std::fs::File::create(&probe).is_ok();
        let _ = std::fs::remove_file(&probe);
        ok
    }

    fn agent(&self, timeout: u64) -> ureq::Agent {
        ureq::Agent::config_builder()
            .timeout_global(Some(Duration::from_secs(timeout)))
            .user_agent(format!("{}/{}", self.name, self.version))
            .build()
            .into()
    }

    /// The latest release's tag and files; `None` when GitHub has nothing to say
    /// (no release yet: 404; rate limited: 403, 429).
    fn fetch_latest(&self) -> Result<Option<(String, Vec<String>)>, String> {
        let latest = format!("https://api.github.com/repos/{}/releases/latest", self.repo);
        let reply = match self.agent(10).get(&latest).header("Accept", "application/vnd.github+json").call() {
            Ok(reply) => reply,
            Err(ureq::Error::StatusCode(status)) if no_news(status) => return Ok(None),
            Err(e) => return Err(e.to_string()),
        };
        let text = reply.into_body().read_to_string().map_err(|e| e.to_string())?;
        let json: Value = serde_json::from_str(&text).map_err(|e| e.to_string())?;
        parse_latest(&json).map(Some).ok_or_else(|| "GitHub's answer has no version".into())
    }

    /// The release file for a machine: `<name>-<arch>-linux.tar.gz`.
    pub fn tarball(&self, os: &str, arch: &str) -> Option<String> {
        match (os, arch) {
            ("linux", "x86_64" | "aarch64") => Some(format!("{}-{arch}-linux.tar.gz", self.name)),
            _ => None,
        }
    }

    pub fn replace_for(
        &self,
        exe: &str,
        packaged: Packaged,
        os: &str,
        arch: &str,
        assets: &[String],
        tag: &str,
    ) -> Replace {
        if is_dev_build(exe) {
            Replace::FromSource
        } else if from_cargo(exe) {
            Replace::Cargo
        } else if packaged != Packaged::No {
            Replace::Package
        } else {
            match self.tarball(os, arch).filter(|name| assets.contains(name)) {
                Some(name) => Replace::Ready(format!("{}/{tag}/{name}", self.download_url())),
                None => Replace::NoAsset,
            }
        }
    }

    /// The update for a machine running `current` from `exe`, when `tag` is newer.
    #[allow(clippy::too_many_arguments)]
    pub fn update_for(
        &self,
        current: &str,
        tag: &str,
        assets: &[String],
        exe: &str,
        packaged: Packaged,
        os: &str,
        arch: &str,
    ) -> Option<Update> {
        if version(tag)? <= version(current)? {
            return None;
        }
        let name = self.name;
        let cargo = format!("cargo install --locked --git {} --tag {tag}", self.repo_url());
        let checkout = exe.strip_suffix(&format!("/target/release/{name}"));
        let how = if let Some(advice) = packaged.advice().filter(|_| !is_dev_build(exe) && !from_cargo(exe)) {
            advice.to_owned()
        } else if from_cargo(exe) {
            cargo
        } else if let Some(dir) = checkout {
            format!("cd {dir} && git pull && git checkout {tag} && cargo build --release")
        } else {
            match self.tarball(os, arch).filter(|t| assets.contains(t)) {
                // Only the program out of the archive, straight over the one running.
                Some(file) => {
                    let dir =
                        Path::new(exe).parent().map_or_else(|| "~/.local/bin".into(), |d| d.display().to_string());
                    format!(
                        "curl -fsSL {}/{tag}/{file} | tar xz -C {dir} --strip-components=1 {name}/{name}",
                        self.download_url()
                    )
                }
                None => cargo,
            }
        };
        let replace = self.replace_for(exe, packaged, os, arch, assets, tag);
        Some(Update { version: tag.trim_start_matches('v').to_owned(), how, replace })
    }

    /// Downloads the archive at `url`, takes the program out of it, checks that
    /// it runs and says it is `version`, and swaps it in for the running program
    /// (a rename, so a failure halfway leaves the old one).
    pub fn install(&self, url: &str, version: &str, mut progress: impl FnMut(u8)) -> Result<PathBuf, String> {
        let exe = running_exe();
        let shown = exe.display().to_string();
        if is_dev_build(&shown) {
            return Err("built from source: git pull && cargo build --release".into());
        }
        let dir = exe.parent().ok_or("no folder for the program")?;
        let archive = self.download(url, &mut progress)?;
        let program = self.from_tarball(&archive)?;
        let staged = dir.join(format!(".{}-update-{}", self.name, std::process::id()));
        let result = stage(&staged, &program)
            .and_then(|()| check_runs(&staged, version))
            .and_then(|()| self_replace::self_replace(&staged).map_err(|e| format!("replacing {shown}: {e}")));
        let _ = std::fs::remove_file(&staged);
        result.map(|()| exe)
    }

    fn download(&self, url: &str, progress: &mut impl FnMut(u8)) -> Result<Vec<u8>, String> {
        let reply = self.agent(300).get(url).call().map_err(|e| format!("download: {e}"))?;
        let total = reply.body().content_length().filter(|n| *n > 0);
        let mut reader = reply.into_body().into_with_config().limit(MAX_DOWNLOAD).reader();
        let mut data = Vec::with_capacity(total.unwrap_or(0) as usize);
        let mut chunk = [0u8; 64 * 1024];
        let mut shown = None;
        loop {
            let n = reader.read(&mut chunk).map_err(|e| format!("download: {e}"))?;
            if n == 0 {
                break;
            }
            data.extend_from_slice(&chunk[..n]);
            if let Some(total) = total {
                let percent = (data.len() as u64 * 100 / total).min(100) as u8;
                if shown != Some(percent) {
                    shown = Some(percent);
                    progress(percent);
                }
            }
        }
        if data.is_empty() {
            return Err("download: empty file".into());
        }
        Ok(data)
    }

    /// The program out of a release tarball (`<name>/<name>`).
    pub fn from_tarball(&self, gz: &[u8]) -> Result<Vec<u8>, String> {
        let inside = format!("{0}/{0}", self.name);
        let mut archive = tar::Archive::new(flate2::read::GzDecoder::new(gz));
        for entry in archive.entries().map_err(|e| format!("archive: {e}"))? {
            let mut entry = entry.map_err(|e| format!("archive: {e}"))?;
            let path = entry.path().map_err(|e| format!("archive: {e}"))?.into_owned();
            if entry.header().entry_type().is_file() && path.strip_prefix("./").unwrap_or(&path) == Path::new(&inside) {
                let mut program = Vec::new();
                entry.read_to_end(&mut program).map_err(|e| format!("archive: {e}"))?;
                return Ok(program);
            }
        }
        Err(format!("archive: no {inside} inside"))
    }
}

/// No newer release: up to date when the tag really is a version.
fn latest_or_unknown(current: &str, tag: &str) -> Checked {
    match (version(tag), version(current)) {
        (Some(latest), Some(mine)) if latest <= mine => Checked::Latest,
        _ => Checked::Unknown,
    }
}

fn running_exe() -> PathBuf {
    let exe = std::env::current_exe().unwrap_or_default();
    std::fs::canonicalize(&exe).unwrap_or(exe)
}

/// GitHub statuses that just mean "nothing to say": no release published
/// yet, or too many requests from this address for now.
fn no_news(status: u16) -> bool {
    matches!(status, 404 | 403 | 429)
}

/// Tag and asset names of GitHub's "latest release" answer.
pub fn parse_latest(json: &Value) -> Option<(String, Vec<String>)> {
    let tag = json["tag_name"].as_str()?.to_owned();
    let assets = json["assets"].as_array()?.iter().filter_map(|a| a["name"].as_str().map(str::to_owned)).collect();
    Some((tag, assets))
}

/// `1.2.3` → `(1, 2, 3)`; a leading `v` and anything after `-` are ignored.
fn version(text: &str) -> Option<(u64, u64, u64)> {
    let core = text.trim_start_matches('v').split(['-', '+']).next()?;
    let mut parts = core.split('.').map(|p| p.parse().ok());
    Some((parts.next()??, parts.next().flatten().unwrap_or(0), parts.next().flatten().unwrap_or(0)))
}

/// Built with cargo in a checkout: `…/target/release/` or `…/target/debug/`.
pub fn is_dev_build(exe: &str) -> bool {
    let parts: Vec<&str> = exe.split('/').collect();
    parts.iter().position(|p| *p == "target").is_some_and(|t| {
        let between = parts.get(t + 1..parts.len().saturating_sub(1)).unwrap_or_default();
        between.iter().any(|p| matches!(*p, "release" | "debug"))
    })
}

fn from_cargo(exe: &str) -> bool {
    exe.contains(".cargo/bin")
}

/// Put there by a package manager, which should be the one to update it.
pub fn is_package(exe: &str) -> bool {
    const MANAGED: &[&str] = &["/usr/bin/", "/usr/sbin/", "/bin/", "/usr/lib/", "/usr/share/", "/nix/store/", "/snap/"];
    MANAGED.iter().any(|dir| exe.starts_with(dir))
}

/// How a package manager owns this copy of the app.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Packaged {
    /// Not from a package: the tarball, cargo or a checkout.
    No,
    /// pacman owns the file (an AUR or Arch package).
    Pacman,
    /// Some other package manager's folder (`/usr/bin`, the Nix store…).
    Other,
}

impl Packaged {
    /// Asks the system: `pacman -Qo <exe>` succeeds when pacman owns the
    /// file; failing that, a path a package manager uses. Any trouble
    /// running pacman (not installed, no permission) counts as "no".
    pub fn detect(exe: &str) -> Self {
        if pacman_owns(exe) {
            Self::Pacman
        } else if is_package(exe) {
            Self::Other
        } else {
            Self::No
        }
    }

    /// What to tell the user to run instead of the download.
    fn advice(self) -> Option<&'static str> {
        match self {
            Self::No => None,
            Self::Pacman => Some("sudo pacman -Syu · or your AUR helper (for example: yay -Syu)"),
            Self::Other => Some("Use the package manager you installed it with"),
        }
    }
}

/// `pacman -Qo <exe>`: true only when it runs and says pacman owns the file.
fn pacman_owns(exe: &str) -> bool {
    use std::process::{Command, Stdio};
    Command::new("pacman")
        .args(["-Qo", "--", exe])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|status| status.success())
}

const MAX_DOWNLOAD: u64 = 200 * 1024 * 1024;

fn stage(path: &Path, program: &[u8]) -> Result<(), String> {
    std::fs::write(path, program).map_err(|e| format!("writing {}: {e}", path.display()))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755))
            .map_err(|e| format!("making {} runnable: {e}", path.display()))?;
    }
    Ok(())
}

/// The new program starts here and says it is `version`.
fn check_runs(program: &Path, version: &str) -> Result<(), String> {
    let out = std::process::Command::new(program)
        .arg("--version")
        .stdin(std::process::Stdio::null())
        .output()
        .map_err(|e| format!("the new version does not start here: {e}"))?;
    let said = String::from_utf8_lossy(&out.stdout);
    if out.status.success() && said.split_whitespace().any(|w| w == version) {
        Ok(())
    } else {
        Err(format!("the new version says {:?}, not {version}", said.trim()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const BOX: Release = Release { name: "boxvibe", repo: "mortbrew/boxvibe", version: "0.1.0" };
    const DOWNLOAD: &str = "https://github.com/mortbrew/boxvibe/releases/download";

    fn assets() -> Vec<String> {
        vec!["boxvibe-x86_64-linux.tar.gz".into(), "boxvibe-aarch64-linux.tar.gz".into()]
    }

    #[test]
    fn only_newer_versions_count() {
        let exe = "/home/t/.local/bin/boxvibe";
        assert!(BOX.update_for("0.1.0", "v0.2.0", &assets(), exe, Packaged::No, "linux", "x86_64").is_some());
        assert!(BOX.update_for("0.2.0", "v0.2.0", &assets(), exe, Packaged::No, "linux", "x86_64").is_none());
        assert!(BOX.update_for("0.3.0", "v0.2.0", &assets(), exe, Packaged::No, "linux", "x86_64").is_none());
        assert!(BOX.update_for("0.1.0", "nightly", &assets(), exe, Packaged::No, "linux", "x86_64").is_none());
        assert_eq!(version("v1.2"), Some((1, 2, 0)));
    }

    #[test]
    fn the_update_matches_how_it_was_installed() {
        let u = |exe: &str| BOX.update_for("0.1.0", "v0.2.0", &assets(), exe, Packaged::No, "linux", "x86_64").unwrap();
        let local = u("/home/t/.local/bin/boxvibe");
        assert_eq!(local.replace, Replace::Ready(format!("{DOWNLOAD}/v0.2.0/boxvibe-x86_64-linux.tar.gz")));
        assert!(local.how.contains("tar xz -C /home/t/.local/bin"), "{}", local.how);
        let built = u("/home/t/src/boxvibe/target/release/boxvibe");
        assert_eq!(built.replace, Replace::FromSource);
        assert_eq!(built.how, "cd /home/t/src/boxvibe && git pull && git checkout v0.2.0 && cargo build --release");
        assert_eq!(u("/home/t/.cargo/bin/boxvibe").replace, Replace::Cargo);
        // A path a package manager uses, by path alone (no pacman asked in a test).
        assert!(is_package("/usr/bin/boxvibe") && !is_package("/usr/local/bin/boxvibe"));
        let arm =
            BOX.update_for("0.1.0", "v0.2.0", &assets(), "/opt/b/boxvibe", Packaged::No, "linux", "riscv64").unwrap();
        assert_eq!(arm.replace, Replace::NoAsset);
        assert!(arm.how.starts_with("cargo install"));
    }

    #[test]
    fn a_packaged_install_still_hears_of_the_update_but_is_sent_to_its_package_manager() {
        let u = |exe: &str, packaged| BOX.update_for("0.3.0", "v0.4.0", &assets(), exe, packaged, "linux", "x86_64");
        // pacman owns it: the update is announced, the download is not offered.
        let arch = u("/usr/bin/boxvibe", Packaged::Pacman).expect("still says an update exists");
        assert_eq!(arch.version, "0.4.0");
        assert_eq!(arch.replace, Replace::Package);
        assert!(arch.how.contains("pacman") && arch.how.contains("AUR"), "{}", arch.how);
        assert!(!arch.how.contains("curl"), "no GitHub download for a package: {}", arch.how);
        assert!(Replace::Package.why_not().is_some_and(|w| w.contains("package manager")));
        // Even an exe path outside the usual folders is a package when pacman owns it.
        assert_eq!(u("/opt/boxvibe/boxvibe", Packaged::Pacman).unwrap().replace, Replace::Package);
        // Another package manager: the same, with generic advice.
        let nix = u("/nix/store/abc-boxvibe/bin/boxvibe", Packaged::Other).unwrap();
        assert_eq!(nix.replace, Replace::Package);
        assert!(nix.how.contains("package manager"), "{}", nix.how);
        // A manual install behaves as before.
        let manual = u("/home/t/.local/bin/boxvibe", Packaged::No).unwrap();
        assert!(matches!(manual.replace, Replace::Ready(_)));
        assert!(manual.how.contains("curl"), "{}", manual.how);
        // A build from source or cargo install is never a package, even if pacman were asked.
        assert_eq!(u("/home/t/src/boxvibe/target/release/boxvibe", Packaged::No).unwrap().replace, Replace::FromSource);
    }

    #[test]
    fn detect_is_no_for_a_path_no_package_manager_owns() {
        let exe = std::env::temp_dir().join("boxvibe-not-packaged").join("boxvibe");
        assert_eq!(Packaged::detect(&exe.display().to_string()), Packaged::No);
    }

    #[test]
    fn reads_githubs_answer() {
        let json = serde_json::json!({ "tag_name": "v0.2.0", "assets": [{ "name": "boxvibe-x86_64-linux.tar.gz" }] });
        assert_eq!(parse_latest(&json), Some(("v0.2.0".into(), vec!["boxvibe-x86_64-linux.tar.gz".into()])));
        assert_eq!(parse_latest(&serde_json::json!({})), None);
    }

    #[test]
    fn the_program_comes_out_of_the_archive() {
        let mut tar = tar::Builder::new(Vec::new());
        for (path, data) in [("boxvibe/README.md", &b"readme"[..]), ("boxvibe/boxvibe", &b"\x7fELF"[..])] {
            let mut header = tar::Header::new_gnu();
            header.set_size(data.len() as u64);
            header.set_mode(0o755);
            header.set_cksum();
            tar.append_data(&mut header, path, data).unwrap();
        }
        let mut gz = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::fast());
        std::io::Write::write_all(&mut gz, &tar.into_inner().unwrap()).unwrap();
        assert_eq!(BOX.from_tarball(&gz.finish().unwrap()).unwrap(), b"\x7fELF");
        assert!(BOX.from_tarball(b"junk").is_err());
    }

    #[test]
    fn up_to_date_only_on_a_real_version() {
        assert_eq!(latest_or_unknown("0.2.0", "v0.2.0"), Checked::Latest);
        assert_eq!(latest_or_unknown("0.2.0", "v0.1.0"), Checked::Latest);
        assert_eq!(latest_or_unknown("0.2.0", "nightly"), Checked::Unknown);
        assert!(no_news(404) && no_news(403) && no_news(429) && !no_news(500));
    }

    #[test]
    fn dev_builds_are_recognised() {
        assert!(is_dev_build("/w/boxvibe/target/release/boxvibe"));
        assert!(is_dev_build("/w/boxvibe/target/x86_64-unknown-linux-gnu/debug/boxvibe"));
        assert!(!is_dev_build("/home/t/.local/bin/boxvibe"));
    }
}

#[cfg(test)]
mod generic {
    use super::*;

    #[test]
    fn another_app_gets_its_own_names() {
        let pep = Release { name: "pepvibe", repo: "someone/pepvibe", version: "0.1.0" };
        assert_eq!(pep.tarball("linux", "aarch64").as_deref(), Some("pepvibe-aarch64-linux.tar.gz"));
        assert_eq!(pep.tarball("macos", "aarch64"), None);
        assert_eq!(pep.repo_url(), "https://github.com/someone/pepvibe");
        let assets = vec!["pepvibe-x86_64-linux.tar.gz".to_owned()];
        let u =
            pep.update_for("0.1.0", "v0.2.0", &assets, "/home/t/.local/bin/pepvibe", Packaged::No, "linux", "x86_64");
        let u = u.unwrap();
        assert_eq!(
            u.replace,
            Replace::Ready(
                "https://github.com/someone/pepvibe/releases/download/v0.2.0/pepvibe-x86_64-linux.tar.gz".into()
            )
        );
        assert!(u.how.ends_with("pepvibe/pepvibe"), "{}", u.how);
    }
}
