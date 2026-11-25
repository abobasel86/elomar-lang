//! `elomar upgrade`: moves this `elomar` to the newest release.
//!
//! It asks GitHub for the latest version and, when it is newer, runs the
//! official installer (`install.ps1` on Windows, `install.sh` elsewhere)
//! for that version into the folder this `elomar` is in. The installer
//! checks the download against the release's SHA256SUMS.txt.

use std::path::Path;
use std::process::{Command, ExitCode};
use std::time::Duration;

use crate::Reporter;

const REPO: &str = "abobasel86/elomar-lang";

fn installer_url() -> String {
    let file = if cfg!(windows) {
        "install.ps1"
    } else {
        "install.sh"
    };
    format!("https://raw.githubusercontent.com/{REPO}/main/{file}")
}

/// The latest released version, such as `1.11.0`.
fn latest_version() -> Result<String, String> {
    let agent = elomar_tls::agent_builder()
        .timeout(Duration::from_secs(20))
        .user_agent(concat!("elomar-lang/", env!("CARGO_PKG_VERSION")))
        .build();
    let body = agent
        .get(&format!(
            "https://api.github.com/repos/{REPO}/releases/latest"
        ))
        .call()
        .map_err(|e| e.to_string())?
        .into_string()
        .map_err(|e| e.to_string())?;
    let json: serde_json::Value = serde_json::from_str(&body).map_err(|e| e.to_string())?;
    json["tag_name"]
        .as_str()
        .map(|t| t.trim_start_matches('v').to_string())
        .ok_or_else(|| "the answer has no `tag_name`".to_string())
}

/// `a` is newer than `b` (both `x.y.z`).
fn newer(a: &str, b: &str) -> bool {
    let parts = |v: &str| -> Vec<u64> { v.split('.').map(|p| p.parse().unwrap_or(0)).collect() };
    parts(a) > parts(b)
}

pub(crate) fn run(check_only: bool, force: bool, reporter: &Reporter) -> ExitCode {
    let current = env!("CARGO_PKG_VERSION");
    let latest = match latest_version() {
        Ok(v) => v,
        Err(e) => {
            reporter.say(
                &format!("error: could not ask GitHub for the latest version: {e}"),
                &format!("خطأ: تعذّر سؤال GitHub عن أحدث إصدار: {e}"),
            );
            return ExitCode::from(1);
        }
    };
    if !newer(&latest, current) && !force {
        reporter.out(
            &format!("Elomar-Lang {current} is the newest version."),
            &format!("Elomar-Lang {current} هو أحدث إصدار."),
        );
        return ExitCode::SUCCESS;
    }
    if check_only {
        reporter.out(
            &format!(
                "Elomar-Lang {latest} is available (this is {current}); run `elomar upgrade`."
            ),
            &format!("الإصدار {latest} متاح (هذا {current})؛ شغّل `elomar upgrade`."),
        );
        return ExitCode::SUCCESS;
    }
    let exe = match std::env::current_exe().and_then(|p| p.canonicalize()) {
        Ok(p) => p,
        Err(e) => {
            reporter.say(
                &format!("error: cannot find this program's file: {e}"),
                &format!("خطأ: تعذّر العثور على ملف هذا البرنامج: {e}"),
            );
            return ExitCode::from(1);
        }
    };
    let dir = exe.parent().unwrap_or(Path::new(".")).to_path_buf();
    // Left by an earlier upgrade on Windows, where a running file is renamed.
    let _ = std::fs::remove_file(exe.with_extension("exe.old"));
    let dir_text = dir
        .to_string_lossy()
        .trim_start_matches(r"\\?\")
        .to_string();
    if dir.ends_with(Path::new(".cargo").join("bin")) && !force {
        reporter.out(
            &format!(
                "This elomar was installed with cargo. Upgrade it the same way:\n  cargo install --git https://github.com/{REPO} elomar-cli --locked\n(or `elomar upgrade --force` to put the release build here instead)"
            ),
            &format!(
                "ثُبّت elomar هذا بـ cargo، فحدّثه بالطريقة نفسها:\n  cargo install --git https://github.com/{REPO} elomar-cli --locked\n(أو `elomar upgrade --force` لوضع نسخة الإصدار هنا بدلًا منه)"
            ),
        );
        return ExitCode::SUCCESS;
    }
    reporter.out(
        &format!("Upgrading Elomar-Lang {current} → {latest} in {dir_text} ..."),
        &format!("ترقية Elomar-Lang من {current} إلى {latest} في {dir_text} ..."),
    );
    // A local installer, for testing the scripts before they are published.
    let local = std::env::var("ELOMAR_INSTALL_SCRIPT").ok();
    let mut command = if cfg!(windows) {
        let script = match &local {
            Some(path) => format!("Get-Content -Raw -LiteralPath '{path}' | Invoke-Expression"),
            None => format!(
                "Invoke-RestMethod '{}' | Invoke-Expression",
                installer_url()
            ),
        };
        let mut c = Command::new("powershell");
        c.args([
            "-NoProfile",
            "-ExecutionPolicy",
            "Bypass",
            "-Command",
            &script,
        ]);
        // PowerShell 7 sets a module path that Windows PowerShell cannot use.
        c.env_remove("PSModulePath");
        c
    } else {
        let script = match &local {
            Some(path) => format!("sh '{path}'"),
            None => format!(
                "if command -v curl >/dev/null 2>&1; then curl -fsSL '{0}' | sh; else wget -qO- '{0}' | sh; fi",
                installer_url()
            ),
        };
        let mut c = Command::new("sh");
        c.args(["-c", &script]);
        c
    };
    command
        .env("ELOMAR_VERSION", &latest)
        .env("ELOMAR_INSTALL_DIR", &dir_text)
        .env("ELOMAR_NO_PATH", "1");
    match command.status() {
        Ok(status) if status.success() => ExitCode::SUCCESS,
        Ok(_) => {
            reporter.say(
                "error: the installer failed; nothing was changed unless it said so",
                "خطأ: فشل المثبّت؛ لم يتغير شيء ما لم يذكر ذلك",
            );
            ExitCode::from(1)
        }
        Err(e) => {
            reporter.say(
                &format!("error: could not start the installer: {e}"),
                &format!("خطأ: تعذّر تشغيل المثبّت: {e}"),
            );
            ExitCode::from(1)
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn compares_versions() {
        assert!(super::newer("1.12.0", "1.11.0"));
        assert!(super::newer("1.10.0", "1.9.3"));
        assert!(super::newer("2.0.0", "1.99.99"));
        assert!(!super::newer("1.11.0", "1.11.0"));
        assert!(!super::newer("1.9.0", "1.10.0"));
    }
}
