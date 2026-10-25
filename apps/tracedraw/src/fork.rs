//! The Arcano edition: what this build changes from the upstream project,
//! kept in one place so that merging upstream work stays easy.
//!
//! - Releases come from the edition's own repository (`fork_repo!`), whose
//!   builds carry the upstream version plus a build number
//!   (`0.13.0+arcano.7`), so a rebuild on newer upstream work is offered
//!   as an update even when the upstream version has not moved.
//! - On Windows a release's installer can be downloaded, checked and run
//!   from Help > Updates (see `updates.rs`).
//! - On the machine that builds the edition, Help > Updates also offers to
//!   bring in the upstream work and rebuild (`arcano/update.ps1` in the
//!   source folder, see [`source_script`]).

/// `owner/name` of the edition's repository on GitHub, where its releases
/// are published.
#[macro_export]
macro_rules! fork_repo {
    () => {
        "DanielSampaioRodrigues/tracedraw-arcano"
    };
}

/// The user's locale on Windows (`pt-BR`, `en-US`...), which sets no
/// `LANG`: `GetUserDefaultLocaleName` of kernel32. None elsewhere.
pub fn windows_locale() -> Option<String> {
    #[cfg(windows)]
    {
        windows_locale_name()
    }
    #[cfg(not(windows))]
    {
        None
    }
}

#[cfg(windows)]
#[link(name = "kernel32")]
extern "system" {
    fn GetUserDefaultLocaleName(name: *mut u16, len: i32) -> i32;
}

#[cfg(windows)]
fn windows_locale_name() -> Option<String> {
    let mut buf = [0u16; 85];
    // SAFETY: the buffer is valid for its whole length, which is the
    // length the call is given; it writes at most that many characters.
    let n = unsafe { GetUserDefaultLocaleName(buf.as_mut_ptr(), buf.len() as i32) };
    if n <= 1 {
        None
    } else {
        let end = ((n - 1) as usize).min(buf.len());
        String::from_utf16(&buf[..end]).ok()
    }
}

/// The interface language when none was chosen: the system's, and
/// Brazilian Portuguese where the system's is English or unknown (the
/// edition is made for a Brazilian publisher).
pub fn default_language() -> &'static str {
    // Tests read the interface in English, as upstream.
    if cfg!(test) {
        return crate::i18n::system_language();
    }
    match crate::i18n::system_language() {
        "en" => "pt-BR",
        other => other,
    }
}

/// The build number of this edition (set by the release script through
/// `ARCANO_BUILD` at compile time; 0 for a local build).
pub fn build() -> u64 {
    option_env!("ARCANO_BUILD")
        .and_then(|s| s.trim().parse().ok())
        .unwrap_or(0)
}

/// This build's version: the upstream version and the edition's build
/// number, `0.13.0+arcano.7`.
pub fn version() -> String {
    format!("{}+arcano.{}", env!("CARGO_PKG_VERSION"), build())
}

/// The edition's build number in a release tag or version
/// (`v0.13.0+arcano.7` gives 7); 0 when it has none.
pub fn build_of(tag: &str) -> u64 {
    tag.split_once("+arcano.")
        .and_then(|(_, n)| n.trim().parse().ok())
        .unwrap_or(0)
}

/// The update script of a source folder: `<dir>/arcano/update.ps1`, when
/// it is there (desktop only). An empty `dir` stands for the source
/// folder the program was built in, when it runs from its `target`
/// folder.
pub fn source_script(dir: &str) -> Option<std::path::PathBuf> {
    if crate::files::WEB {
        return None;
    }
    let dir = if dir.trim().is_empty() {
        running_source_dir()?
    } else {
        std::path::PathBuf::from(dir.trim())
    };
    let p = dir.join("arcano").join("update.ps1");
    p.is_file().then_some(p)
}

/// The source folder of a program run from `<src>/target/<profile>/`.
fn running_source_dir() -> Option<std::path::PathBuf> {
    #[cfg(target_arch = "wasm32")]
    return None;
    #[cfg(not(target_arch = "wasm32"))]
    {
        let exe = std::env::current_exe().ok()?;
        let profile = exe.parent()?;
        let target = profile.parent()?;
        if target.file_name()? != "target" {
            return None;
        }
        target.parent().map(|p| p.to_path_buf())
    }
}

impl crate::app::App {
    /// Help > Updates on the machine that builds the edition: run the
    /// source folder's update script in a console of its own.
    pub fn run_source_update(&mut self, script: Option<&std::path::Path>) {
        let Some(script) = script else { return };
        #[cfg(not(target_arch = "wasm32"))]
        if let Err(e) = run_source_update(script) {
            log::warn!("source update: {e}");
            self.status = crate::i18n::trf("updates.from_source_failed", &[("error", &e)]);
        }
        #[cfg(target_arch = "wasm32")]
        let _ = script;
    }
}

/// Run the update script in a console window of its own, so its progress
/// shows (Windows; elsewhere, PowerShell when it is installed).
#[cfg(not(target_arch = "wasm32"))]
pub fn run_source_update(script: &std::path::Path) -> Result<(), String> {
    let dir = script
        .parent()
        .and_then(|p| p.parent())
        .ok_or("the script has no source folder")?;
    let mut cmd = if cfg!(windows) {
        let mut c = std::process::Command::new("cmd");
        c.args(["/C", "start", "TraceDraw", "powershell", "-NoExit", "-ExecutionPolicy", "Bypass", "-File"]);
        c.arg(script);
        c
    } else {
        let mut c = std::process::Command::new("pwsh");
        c.arg("-File").arg(script);
        c
    };
    cmd.current_dir(dir).spawn().map(|_| ()).map_err(|e| e.to_string())
}

/// Start a downloaded installer quietly; it closes TraceDraw and installs
/// the new version, which then starts again (Windows).
///
/// The new version is started by a hidden PowerShell that waits for the
/// installer, as the user, rather than by the installer itself: an
/// installer that elevated itself and was started quietly cannot start a
/// program as the user ("CallSpawnServer: Unexpected response").
#[cfg(not(target_arch = "wasm32"))]
pub fn run_installer(setup: &std::path::Path) -> Result<(), String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    if cfg!(windows) {
        std::process::Command::new("powershell")
            .args(["-NoProfile", "-NonInteractive", "-WindowStyle", "Hidden", "-Command"])
            .arg(relaunch_script(setup, &exe))
            .spawn()
            .map(|_| ())
            .map_err(|e| e.to_string())
    } else {
        std::process::Command::new(setup)
            .args(INSTALLER_FLAGS)
            .spawn()
            .map(|_| ())
            .map_err(|e| e.to_string())
    }
}

/// How a downloaded installer runs: quietly, closing TraceDraw, without
/// restarting Windows.
const INSTALLER_FLAGS: [&str; 4] = ["/SILENT", "/SP-", "/CLOSEAPPLICATIONS", "/NORESTART"];

/// What stops the windowless TraceDraw processes AI apps keep running
/// (`--mcp-window` bridges and `--mcp` servers): they hold the program
/// file, so a quiet install could not replace it and gave up without a
/// word. Windows with drawings are left to the installer, which closes
/// them asking about unsaved work. No double quotes: the whole script is
/// one argument of the command line.
const STOP_AGENT_BRIDGES: &str = "Get-CimInstance Win32_Process -Filter 'Name=''tracedraw.exe''' | Where-Object { $_.CommandLine -match '--mcp' } | ForEach-Object { Stop-Process -Id $_.ProcessId -Force -ErrorAction SilentlyContinue }; Start-Sleep -Milliseconds 500";

/// The PowerShell that stops the agents' bridges, runs the installer,
/// waits for it and, when it succeeded, starts `exe` again.
fn relaunch_script(setup: &std::path::Path, exe: &std::path::Path) -> String {
    // A PowerShell string in single quotes: a quote is written twice.
    let quote = |p: &std::path::Path| format!("'{}'", p.display().to_string().replace('\'', "''"));
    let flags: Vec<String> = INSTALLER_FLAGS.iter().map(|f| format!("'{f}'")).collect();
    format!(
        "{STOP_AGENT_BRIDGES}; $p = Start-Process -FilePath {} -ArgumentList {} -Wait -PassThru; if ($p.ExitCode -eq 0) {{ Start-Process -FilePath {} }}",
        quote(setup),
        flags.join(","),
        quote(exe)
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_new_version_starts_after_the_installer() {
        let s = relaunch_script(
            std::path::Path::new(r"C:\Temp\it's-setup.exe"),
            std::path::Path::new(r"C:\Program Files\TraceDraw\tracedraw.exe"),
        );
        assert!(s.starts_with(STOP_AGENT_BRIDGES), "{s}");
        assert!(STOP_AGENT_BRIDGES.contains("--mcp") && STOP_AGENT_BRIDGES.contains("tracedraw.exe"));
        assert!(!s.contains('"'), "{s}");
        let s = s.trim_start_matches(STOP_AGENT_BRIDGES).trim_start_matches("; ");
        assert!(s.starts_with(r"$p = Start-Process -FilePath 'C:\Temp\it''s-setup.exe' -ArgumentList '/SILENT','/SP-','/CLOSEAPPLICATIONS','/NORESTART' -Wait -PassThru"), "{s}");
        assert!(s.ends_with(r"if ($p.ExitCode -eq 0) { Start-Process -FilePath 'C:\Program Files\TraceDraw\tracedraw.exe' }"), "{s}");
    }

    #[test]
    fn versions_carry_the_build_number() {
        assert!(version().starts_with(env!("CARGO_PKG_VERSION")));
        assert!(version().contains("+arcano."));
        assert_eq!(build_of("v0.13.0+arcano.7"), 7);
        assert_eq!(build_of("0.13.0"), 0);
        assert_eq!(build_of("v0.13.0+arcano.x"), 0);
    }

    #[test]
    fn a_folder_without_the_script_offers_no_source_update() {
        let dir = std::env::temp_dir().join(format!("arcano-src-{}", std::process::id()));
        let _ = std::fs::create_dir_all(dir.join("arcano"));
        let d = dir.to_string_lossy().to_string();
        assert!(source_script(&d).is_none());
        let _ = std::fs::write(dir.join("arcano").join("update.ps1"), "# test");
        assert!(source_script(&d).is_some());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
