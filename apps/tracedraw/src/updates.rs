//! Checking for a newer release (Help > Updates, and once a day at start
//! when Options > Global > General allows it). The check asks GitHub for
//! the latest release of the project, compares its version with this one
//! and, when it is newer, offers the download for this system and the
//! release notes. Nothing is installed without the user: Download opens
//! the installer's link, and the system's installer does the rest.
//!
//! Desktop only: the browser build is always the latest.

use serde::Deserialize;
use std::sync::{Arc, Mutex};

/// The project's latest release, as GitHub describes it.
pub const LATEST_URL: &str = "https://api.github.com/repos/igtoth/tracedraw/releases/latest";
/// The release list, for when no download fits this system.
pub const RELEASES_PAGE: &str = "https://github.com/igtoth/tracedraw/releases";
/// How often the check at start runs, seconds.
pub const DAILY: u64 = 24 * 60 * 60;

/// A release newer than this version.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Release {
    pub version: String,
    /// The release's page (its notes).
    pub page: String,
    /// The download for this system, when the release has one.
    pub download: Option<String>,
}

/// Where a check stands.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum UpdateState {
    #[default]
    Idle,
    Checking,
    /// This version is the latest.
    Latest,
    Available(Release),
    Failed(String),
}

#[derive(Deserialize)]
struct Asset {
    name: String,
    browser_download_url: String,
}

#[derive(Deserialize)]
struct ApiRelease {
    tag_name: String,
    html_url: String,
    #[serde(default)]
    prerelease: bool,
    #[serde(default)]
    draft: bool,
    #[serde(default)]
    assets: Vec<Asset>,
}

/// `1.2.3` from `v1.2.3` (pre-release parts after `-` are dropped).
fn parse_version(s: &str) -> Option<(u64, u64, u64)> {
    let s = s.trim().trim_start_matches(['v', 'V']);
    let core = s.split(['-', '+']).next()?;
    let mut it = core.split('.').map(|p| p.parse::<u64>());
    let major = it.next()?.ok()?;
    let minor = it.next().unwrap_or(Ok(0)).ok()?;
    let patch = it.next().unwrap_or(Ok(0)).ok()?;
    Some((major, minor, patch))
}

/// Whether `tag` names a version later than `current`.
pub fn is_newer(tag: &str, current: &str) -> bool {
    match (parse_version(tag), parse_version(current)) {
        (Some(a), Some(b)) => a > b,
        _ => false,
    }
}

/// The asset names that fit a system (`os` as `std::env::consts::OS`,
/// `arch` as `ARCH`), best first.
fn wanted(os: &str, arch: &str) -> Vec<String> {
    match os {
        "windows" => vec!["-setup.exe".into(), "-portable.zip".into()],
        "macos" => {
            let a = if arch == "aarch64" { "arm64" } else { "x86_64" };
            vec![format!("macos-{a}.dmg")]
        }
        "linux" => vec![".AppImage".into(), "linux-x86_64.tar.gz".into()],
        _ => Vec::new(),
    }
}

/// Read GitHub's answer: the release when it is newer than `current`
/// (drafts and pre-releases never count), `None` when this version is the
/// latest.
pub fn parse_latest(
    json: &str,
    current: &str,
    os: &str,
    arch: &str,
) -> Result<Option<Release>, String> {
    let r: ApiRelease = serde_json::from_str(json).map_err(|e| e.to_string())?;
    if r.draft || r.prerelease || !is_newer(&r.tag_name, current) {
        return Ok(None);
    }
    let download = wanted(os, arch).iter().find_map(|end| {
        r.assets
            .iter()
            .find(|a| a.name.ends_with(end.as_str()))
            .map(|a| a.browser_download_url.clone())
    });
    Ok(Some(Release {
        version: r.tag_name.trim_start_matches(['v', 'V']).to_string(),
        page: r.html_url,
        download,
    }))
}

/// A check running in the background; its result lands in `state`.
#[derive(Debug, Clone, Default)]
pub struct Checker {
    pub state: Arc<Mutex<UpdateState>>,
}

impl Checker {
    pub fn get(&self) -> UpdateState {
        self.state.lock().map(|s| s.clone()).unwrap_or_default()
    }

    /// Start a check unless one is running; `ctx` gets a frame when it
    /// ends. Nothing happens in a browser.
    pub fn start(&self, ctx: Option<egui::Context>) {
        #[cfg(target_arch = "wasm32")]
        let _ = ctx;
        #[cfg(not(target_arch = "wasm32"))]
        {
            if let Ok(mut s) = self.state.lock() {
                if *s == UpdateState::Checking {
                    return;
                }
                *s = UpdateState::Checking;
            }
            let state = self.state.clone();
            std::thread::spawn(move || {
                let result = fetch().and_then(|json| {
                    parse_latest(
                        &json,
                        env!("CARGO_PKG_VERSION"),
                        std::env::consts::OS,
                        std::env::consts::ARCH,
                    )
                });
                let next = match result {
                    Ok(Some(r)) => UpdateState::Available(r),
                    Ok(None) => UpdateState::Latest,
                    Err(e) => UpdateState::Failed(e),
                };
                if let Ok(mut s) = state.lock() {
                    *s = next;
                }
                if let Some(ctx) = ctx {
                    ctx.request_repaint();
                }
            });
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn fetch() -> Result<String, String> {
    use ureq::tls::{RootCerts, TlsConfig};
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(std::time::Duration::from_secs(20)))
        .tls_config(
            TlsConfig::builder()
                .root_certs(RootCerts::PlatformVerifier)
                .build(),
        )
        .build()
        .into();
    let mut resp = agent
        .get(LATEST_URL)
        .header(
            "User-Agent",
            concat!("TraceDraw/", env!("CARGO_PKG_VERSION")),
        )
        .header("Accept", "application/vnd.github+json")
        .call()
        .map_err(|e| e.to_string())?;
    resp.body_mut()
        .with_config()
        .limit(1 << 20)
        .read_to_string()
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"{
        "tag_name": "v0.5.0",
        "html_url": "https://github.com/igtoth/tracedraw/releases/tag/v0.5.0",
        "prerelease": false,
        "draft": false,
        "assets": [
            {"name": "tracedraw-0.5.0-linux-x86_64.AppImage", "browser_download_url": "https://x/a.AppImage"},
            {"name": "tracedraw-0.5.0-linux-x86_64.tar.gz", "browser_download_url": "https://x/a.tar.gz"},
            {"name": "tracedraw-0.5.0-macos-arm64.dmg", "browser_download_url": "https://x/arm.dmg"},
            {"name": "tracedraw-0.5.0-macos-x86_64.dmg", "browser_download_url": "https://x/intel.dmg"},
            {"name": "tracedraw-0.5.0-windows-x86_64-portable.zip", "browser_download_url": "https://x/p.zip"},
            {"name": "tracedraw-0.5.0-windows-x86_64-setup.exe", "browser_download_url": "https://x/setup.exe"}
        ]
    }"#;

    #[test]
    fn versions_compare_by_number() {
        assert!(is_newer("v0.5.0", "0.4.0"));
        assert!(is_newer("v0.10.0", "0.9.9"));
        assert!(is_newer("1.0", "0.99.0"));
        assert!(!is_newer("v0.4.0", "0.4.0"));
        assert!(!is_newer("v0.3.9", "0.4.0"));
        assert!(!is_newer("nightly", "0.4.0"));
        assert!(!is_newer("v0.5.0-rc1", "0.5.0"));
    }

    #[test]
    fn the_download_fits_the_system() {
        let pick = |os, arch| {
            parse_latest(SAMPLE, "0.4.0", os, arch)
                .expect("parses")
                .expect("newer")
                .download
        };
        assert_eq!(
            pick("windows", "x86_64").as_deref(),
            Some("https://x/setup.exe")
        );
        assert_eq!(
            pick("macos", "aarch64").as_deref(),
            Some("https://x/arm.dmg")
        );
        assert_eq!(
            pick("macos", "x86_64").as_deref(),
            Some("https://x/intel.dmg")
        );
        assert_eq!(
            pick("linux", "x86_64").as_deref(),
            Some("https://x/a.AppImage")
        );
        assert_eq!(pick("freebsd", "x86_64"), None);
        let r = parse_latest(SAMPLE, "0.4.0", "linux", "x86_64")
            .expect("parses")
            .expect("newer");
        assert_eq!(r.version, "0.5.0");
        assert!(r.page.ends_with("v0.5.0"));
    }

    #[test]
    fn the_same_or_older_drafts_and_pre_releases_are_not_offered() {
        assert_eq!(parse_latest(SAMPLE, "0.5.0", "linux", "x86_64"), Ok(None));
        let pre = SAMPLE.replace("\"prerelease\": false", "\"prerelease\": true");
        assert_eq!(parse_latest(&pre, "0.4.0", "linux", "x86_64"), Ok(None));
        let draft = SAMPLE.replace("\"draft\": false", "\"draft\": true");
        assert_eq!(parse_latest(&draft, "0.4.0", "linux", "x86_64"), Ok(None));
        assert!(parse_latest("not json", "0.4.0", "linux", "x86_64").is_err());
        assert!(parse_latest("{}", "0.4.0", "linux", "x86_64").is_err());
    }
}
