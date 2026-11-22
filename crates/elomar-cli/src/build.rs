//! `elomar build`: one executable file for a program, to run on computers
//! without Elomar-Lang.
//!
//! The executable is a copy of `elomar` itself with the program appended:
//! its source files, `elomar.toml` and its packages' modules, as JSON,
//! followed by the JSON's length (8 bytes, little-endian) and the marker
//! [`MAGIC`]. When `elomar` starts, it looks for that marker at its own
//! end ([`embedded`]); if it is there, it runs the program inside instead of
//! reading a command line, and every argument goes to the program.
//!
//! Relative paths in a built program (files it opens, `[permissions]`)
//! start at the folder it is run from.

use std::collections::HashMap;
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use serde_json::{Value, json};

use crate::Reporter;

/// Ends every built executable.
const MAGIC: &[u8; 8] = b"ELOMARB1";

/// A program inside an executable.
pub(crate) struct Bundle {
    /// The program's name, for messages.
    name: String,
    /// The main file and the project folder, as they were when it was built.
    entry: PathBuf,
    root: PathBuf,
    module_dir: PathBuf,
    packages: HashMap<String, PathBuf>,
    files: HashMap<PathBuf, String>,
    /// The text of `elomar.toml`, if the project has one.
    config: Option<String>,
}

/// The program appended to the running executable, if there is one.
pub(crate) fn embedded() -> Option<Bundle> {
    let exe = std::env::current_exe().ok()?;
    let mut file = std::fs::File::open(exe).ok()?;
    let len = file.seek(SeekFrom::End(0)).ok()?;
    if len < 16 {
        return None;
    }
    let mut tail = [0u8; 16];
    file.seek(SeekFrom::End(-16)).ok()?;
    file.read_exact(&mut tail).ok()?;
    if &tail[8..] != MAGIC {
        return None;
    }
    let size = u64::from_le_bytes(tail[..8].try_into().ok()?);
    if size > len - 16 {
        return None;
    }
    file.seek(SeekFrom::End(-16 - size as i64)).ok()?;
    let mut data = vec![0u8; size as usize];
    file.read_exact(&mut data).ok()?;
    let v: Value = serde_json::from_slice(&data).ok()?;
    let path = |v: &Value| PathBuf::from(v.as_str().unwrap_or_default());
    let map = |v: &Value| -> HashMap<String, String> {
        v.as_object()
            .map(|o| {
                o.iter()
                    .map(|(k, v)| (k.clone(), v.as_str().unwrap_or_default().to_string()))
                    .collect()
            })
            .unwrap_or_default()
    };
    Some(Bundle {
        name: v["name"].as_str().unwrap_or("program").to_string(),
        entry: path(&v["entry"]),
        root: path(&v["root"]),
        module_dir: path(&v["module_dir"]),
        packages: map(&v["packages"])
            .into_iter()
            .map(|(k, v)| (k, PathBuf::from(v)))
            .collect(),
        files: map(&v["files"])
            .into_iter()
            .map(|(k, v)| (PathBuf::from(k), v))
            .collect(),
        config: v["config"].as_str().map(str::to_string),
    })
}

/// Runs the program inside this executable with all command-line arguments.
pub(crate) fn run_embedded(bundle: Bundle, reporter: &Reporter) -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let text = bundle.files.get(&bundle.entry).cloned().unwrap_or_default();
    let files = elomar_loader::InMemory {
        root: bundle.root.clone(),
        files: bundle.files,
    };
    let loaded = match elomar_loader::load_from(
        &files,
        &bundle.entry,
        text,
        &bundle.module_dir,
        bundle.packages,
        Vec::new(),
    ) {
        Ok(l) => l,
        Err(e) => {
            reporter.say(
                &format!("error: the program inside this file is damaged: {e}"),
                &format!("خطأ: البرنامج داخل هذا الملف تالف: {e}"),
            );
            return ExitCode::from(2);
        }
    };
    let base = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let ai = match crate::ai_runtime_from(bundle.config.as_deref(), base) {
        Ok(ai) => ai,
        Err(message) => {
            reporter.say(
                &format!("error: invalid elomar.toml inside the program: {message}"),
                &format!("خطأ: الملف elomar.toml داخل البرنامج غير صالح: {message}"),
            );
            return ExitCode::from(2);
        }
    };
    crate::check_and_run(loaded, ai, &bundle.name, true, args, reporter)
}

/// `elomar build <file> [-o <output>]`.
pub(crate) fn build(source: &Path, output: Option<&Path>, reporter: &Reporter) -> ExitCode {
    crate::packages::fetch_for(source, reporter);
    let root = crate::project_root(source);
    let loaded = match elomar_loader::load(source, &root) {
        Ok(l) => l,
        Err(e) => {
            let shown = source.display();
            reporter.say(
                &format!("error: cannot read `{shown}`: {e}"),
                &format!("خطأ: تعذّرت قراءة `{shown}`: {e}"),
            );
            return ExitCode::from(2);
        }
    };
    let config_path = root.join("elomar.toml");
    let config = std::fs::read_to_string(&config_path).ok();
    let ai = match crate::ai_runtime_from(config.as_deref(), root.clone()) {
        Ok(ai) => ai,
        Err(message) => {
            let shown = config_path.display();
            reporter.say(
                &format!("error: invalid `{shown}`: {message}"),
                &format!("خطأ: الملف `{shown}` غير صالح: {message}"),
            );
            return ExitCode::from(2);
        }
    };
    let name = source.display().to_string();
    // Only a program without errors is built.
    let paths = loaded.paths.clone();
    let status = crate::check_and_run(loaded, ai, &name, false, Vec::new(), reporter);
    if status != ExitCode::SUCCESS {
        return status;
    }

    let mut files = serde_json::Map::new();
    for path in &paths {
        match std::fs::read_to_string(path) {
            Ok(text) => {
                files.insert(path.display().to_string(), Value::String(text));
            }
            Err(e) => {
                let shown = path.display();
                reporter.say(
                    &format!("error: cannot read `{shown}`: {e}"),
                    &format!("خطأ: تعذّرت قراءة `{shown}`: {e}"),
                );
                return ExitCode::from(2);
            }
        }
    }
    let packages: serde_json::Map<String, Value> = elomar_loader::package_dirs(&root)
        .unwrap_or_default()
        .into_iter()
        .map(|(k, v)| (k, Value::String(v.display().to_string())))
        .collect();
    let entry = std::path::absolute(source).unwrap_or_else(|_| source.to_path_buf());
    let stem = source
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "program".into());
    let bundle = json!({
        "name": &stem,
        "entry": entry.display().to_string(),
        "root": root.display().to_string(),
        "module_dir": elomar_loader::module_dir(&root).display().to_string(),
        "packages": packages,
        "files": files,
        "config": config,
    });

    let out = match output {
        Some(o) => o.to_path_buf(),
        None => PathBuf::from(format!("{stem}{}", std::env::consts::EXE_SUFFIX)),
    };
    match write_executable(&out, &bundle.to_string()) {
        Ok(size) => {
            let shown = out.display();
            let mb = size as f64 / 1_048_576.0;
            reporter.out(
                &format!("built `{shown}` ({mb:.1} MB): it runs without Elomar-Lang installed"),
                &format!("بُني `{shown}` ({mb:.1} ميغابايت): يعمل دون تثبيت Elomar-Lang"),
            );
            ExitCode::SUCCESS
        }
        Err(e) => {
            let shown = out.display();
            reporter.say(
                &format!("error: cannot write `{shown}`: {e}"),
                &format!("خطأ: تعذّرت كتابة `{shown}`: {e}"),
            );
            ExitCode::from(2)
        }
    }
}

/// This executable without any program appended, then `bundle`.
fn write_executable(out: &Path, bundle: &str) -> std::io::Result<u64> {
    let exe = std::fs::read(std::env::current_exe()?)?;
    let base = strip_bundle(&exe);
    if let Some(parent) = out.parent().filter(|p| !p.as_os_str().is_empty()) {
        std::fs::create_dir_all(parent)?;
    }
    let mut file = std::fs::File::create(out)?;
    file.write_all(base)?;
    file.write_all(bundle.as_bytes())?;
    file.write_all(&(bundle.len() as u64).to_le_bytes())?;
    file.write_all(MAGIC)?;
    drop(file);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(out, std::fs::Permissions::from_mode(0o755))?;
    }
    Ok(std::fs::metadata(out)?.len())
}

/// `exe` without a program appended to it (when a built program builds).
fn strip_bundle(exe: &[u8]) -> &[u8] {
    let n = exe.len();
    if n >= 16 && &exe[n - 8..] == MAGIC {
        let size = u64::from_le_bytes(exe[n - 16..n - 8].try_into().unwrap_or_default()) as usize;
        if size <= n - 16 {
            return &exe[..n - 16 - size];
        }
    }
    exe
}
