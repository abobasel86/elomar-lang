//! `elomar doctor`: checks the project's AI setup before a run and says
//! what to fix: the configuration, each model's key, network permission,
//! whether its server answers, and whether the model id exists there.
//!
//! It also checks the rest of the project: packages, Python, MCP servers,
//! the programs `process.run` may start, and the folders `std.fs` may use.
//!
//! The only requests it makes are `GET .../models`, which cost nothing,
//! and only to hosts the project already allows. Keys are never printed.
//! Nothing is started except `python --version` when Python is enabled.

use std::path::Path;
use std::process::ExitCode;
use std::time::Duration;

use elomar_ai_core::{ModelConfig, ProjectConfig, parse_config};
use serde_json::Value;

use crate::Reporter;

#[derive(Clone, Copy, PartialEq)]
enum Level {
    Ok,
    Warn,
    Fail,
}

struct Report<'a> {
    reporter: &'a Reporter,
    warnings: usize,
    problems: usize,
}

impl Report<'_> {
    fn line(&mut self, level: Level, en: &str, ar: &str) {
        let mark = match level {
            Level::Ok => "[ok]",
            Level::Warn => {
                self.warnings += 1;
                "[! ]"
            }
            Level::Fail => {
                self.problems += 1;
                "[x ]"
            }
        };
        self.reporter
            .out(&format!("{mark} {en}"), &format!("{mark} {ar}"));
    }
}

pub(crate) fn run(reporter: &Reporter) -> ExitCode {
    let mut report = Report {
        reporter,
        warnings: 0,
        problems: 0,
    };
    let here = std::env::current_dir().unwrap_or_else(|_| ".".into());
    let Some(path) = crate::find_config(&here.join("_")) else {
        report.line(
            Level::Warn,
            "no elomar.toml in this folder or above: only the offline model \"mock\" is available",
            "لا يوجد elomar.toml في هذا المجلد أو فوقه: النموذج الوحيد المتاح هو \"mock\" الذي يعمل دون إنترنت",
        );
        return finish(&report);
    };
    let text = match std::fs::read_to_string(&path) {
        Ok(t) => t,
        Err(e) => {
            report.line(
                Level::Fail,
                &format!("cannot read {}: {e}", path.display()),
                &format!("تعذّرت قراءة {}: {e}", path.display()),
            );
            return finish(&report);
        }
    };
    let config = match parse_config(&text) {
        Ok(c) => c,
        Err(e) => {
            report.line(
                Level::Fail,
                &format!("{} is not valid: {e}", path.display()),
                &format!("{} غير صالح: {e}", path.display()),
            );
            return finish(&report);
        }
    };
    let root = path.parent().unwrap_or(Path::new(".")).to_path_buf();
    match crate::ai_runtime_from(Some(&text), root.clone()) {
        Ok(_) => report.line(
            Level::Ok,
            &format!("{} is valid", path.display()),
            &format!("{} صالح", path.display()),
        ),
        Err(e) => report.line(
            Level::Fail,
            &format!("configuration error: {e}"),
            &format!("خطأ في الإعداد: {e}"),
        ),
    }
    if config.ai.models.is_empty() {
        report.line(
            Level::Ok,
            "no models in [ai.models]; the offline model \"mock\" is always available",
            "لا نماذج في [ai.models]؛ والنموذج \"mock\" متاح دائمًا دون إنترنت",
        );
    }
    for (name, model) in &config.ai.models {
        check_model(&mut report, &config, name, model);
    }
    if let Some(file) = &config.ai.cache.embeddings {
        let target = root.join(file);
        let folder_ok = target.parent().is_some_and(Path::is_dir);
        if folder_ok {
            report.line(
                Level::Ok,
                &format!("embedding cache: {file}"),
                &format!("ذاكرة التضمينات: {file}"),
            );
        } else {
            report.line(
                Level::Fail,
                &format!("embedding cache {file}: its folder does not exist"),
                &format!("ذاكرة التضمينات {file}: مجلدها غير موجود"),
            );
        }
    }
    check_project(&mut report, &config, &root);
    finish(&report)
}

fn finish(report: &Report) -> ExitCode {
    let (p, w) = (report.problems, report.warnings);
    if p == 0 && w == 0 {
        report
            .reporter
            .out("\nEverything looks ready.", "\nكل شيء يبدو جاهزًا.");
    } else {
        report.reporter.out(
            &format!("\n{p} problem(s), {w} warning(s)."),
            &format!("\n{p} مشكلة، و{w} تنبيه."),
        );
    }
    if p == 0 {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    }
}

fn check_model(report: &mut Report, config: &ProjectConfig, name: &str, model: &ModelConfig) {
    let (default_base, default_key) = match model.provider.as_str() {
        "mock" => {
            report.line(
                Level::Ok,
                &format!("model `{name}`: mock, answers offline"),
                &format!("النموذج `{name}`: mock، يجيب دون إنترنت"),
            );
            return;
        }
        "openai" => (
            elomar_ai_provider_openai::DEFAULT_BASE_URL,
            elomar_ai_provider_openai::DEFAULT_KEY_ENV,
        ),
        "anthropic" => (
            elomar_ai_provider_anthropic::DEFAULT_BASE_URL,
            elomar_ai_provider_anthropic::DEFAULT_KEY_ENV,
        ),
        other => {
            report.line(
                Level::Fail,
                &format!(
                    "model `{name}`: unknown provider `{other}`; use mock, openai or anthropic"
                ),
                &format!(
                    "النموذج `{name}`: مزود غير معروف `{other}`؛ استخدم mock أو openai أو anthropic"
                ),
            );
            return;
        }
    };
    let anthropic = model.provider == "anthropic";
    let base = model.base_url.as_deref().unwrap_or(default_base);
    let Ok(url) = url::Url::parse(base) else {
        return; // already reported as a configuration error
    };
    let host = url.host_str().unwrap_or("").to_ascii_lowercase();
    let port = url.port_or_known_default().unwrap_or(443);
    let local = matches!(host.as_str(), "localhost" | "127.0.0.1" | "[::1]");

    let id = match (&model.model, anthropic) {
        (Some(id), _) if id.contains("REPLACE") || id.trim().is_empty() => {
            report.line(
                Level::Fail,
                &format!("model `{name}`: write the vendor's model id in `model` (now {id:?})"),
                &format!("النموذج `{name}`: اكتب معرّف النموذج لدى المزود في `model` (الآن {id:?})"),
            );
            return;
        }
        (Some(id), _) => id.clone(),
        (None, true) => elomar_ai_provider_anthropic::DEFAULT_MODEL.to_string(),
        (None, false) => return, // already reported as a configuration error
    };

    let key_env = model.api_key_env.as_deref().unwrap_or(default_key);
    let key = std::env::var(key_env).ok().filter(|k| !k.trim().is_empty());
    match (&key, local) {
        (_, true) => report.line(
            Level::Ok,
            &format!("model `{name}`: a server on this computer, no key needed"),
            &format!("النموذج `{name}`: خادم على هذا الجهاز، لا يلزم مفتاح"),
        ),
        (Some(_), false) => report.line(
            Level::Ok,
            &format!("model `{name}`: the key in `{key_env}` is set"),
            &format!("النموذج `{name}`: المفتاح في `{key_env}` موجود"),
        ),
        (None, false) => {
            report.line(
                Level::Fail,
                &format!(
                    "model `{name}`: `{key_env}` is not set; in your own terminal: $env:{key_env} = \"...\" (PowerShell) or export {key_env}=... "
                ),
                &format!(
                    "النموذج `{name}`: `{key_env}` غير موجود؛ ضعه في نافذتك أنت: $env:{key_env} = \"...\" (PowerShell) أو export {key_env}=..."
                ),
            );
            return;
        }
    }
    if !config.permissions.allows_host(&host, port) {
        let entry = if local {
            format!("{host}:{port}")
        } else {
            host.clone()
        };
        report.line(
            Level::Fail,
            &format!(
                "model `{name}`: `{entry}` is not allowed; add it: network_hosts = [\"{entry}\"] under [permissions]"
            ),
            &format!(
                "النموذج `{name}`: `{entry}` غير مسموح؛ أضفه: network_hosts = [\"{entry}\"] تحت [permissions]"
            ),
        );
        return;
    }

    // `GET .../models` answers whether the server is there, the key works,
    // and the model id exists, without making a paid call.
    let list_url = format!(
        "{}/models{}",
        base.trim_end_matches('/'),
        if anthropic { "?limit=1000" } else { "" }
    );
    let agent = elomar_tls::agent_builder()
        .redirects(0)
        .timeout(Duration::from_secs(15))
        .build();
    let mut request = agent.get(&list_url);
    if let Some(k) = &key {
        request = if anthropic {
            request.set("x-api-key", k).set(
                "anthropic-version",
                elomar_ai_provider_anthropic::API_VERSION,
            )
        } else {
            request.set("Authorization", &format!("Bearer {k}"))
        };
    }
    let body = match request.call() {
        Ok(r) => r.into_string().unwrap_or_default(),
        Err(ureq::Error::Status(code @ (401 | 403), _)) => {
            report.line(
                Level::Fail,
                &format!("model `{name}`: {host} rejected the key in `{key_env}` (HTTP {code})"),
                &format!("النموذج `{name}`: رفض {host} المفتاح في `{key_env}` (HTTP {code})"),
            );
            return;
        }
        Err(ureq::Error::Status(code, _)) => {
            report.line(
                Level::Warn,
                &format!("model `{name}`: could not list the models at {host} (HTTP {code})"),
                &format!("النموذج `{name}`: تعذّر عرض النماذج في {host} (HTTP {code})"),
            );
            return;
        }
        Err(ureq::Error::Transport(t)) => {
            let (en, ar) = if local {
                (
                    format!(
                        "model `{name}`: nothing answers at {base}; is the server (such as Ollama) running?"
                    ),
                    format!("النموذج `{name}`: لا شيء يجيب في {base}؛ هل الخادم (مثل Ollama) يعمل؟"),
                )
            } else {
                (
                    format!("model `{name}`: cannot reach {host}: {t}"),
                    format!("النموذج `{name}`: تعذّر الوصول إلى {host}: {t}"),
                )
            };
            report.line(Level::Fail, &en, &ar);
            return;
        }
    };
    let ids: Vec<String> = serde_json::from_str::<Value>(&body)
        .ok()
        .and_then(|j| {
            j["data"].as_array().map(|items| {
                items
                    .iter()
                    .filter_map(|i| i["id"].as_str().map(str::to_string))
                    .collect()
            })
        })
        .unwrap_or_default();
    let found = ids.iter().any(|i| *i == id || *i == format!("{id}:latest"));
    if found {
        report.line(
            Level::Ok,
            &format!("model `{name}`: `{id}` is available at {host}"),
            &format!("النموذج `{name}`: `{id}` متاح في {host}"),
        );
        return;
    }
    let stem: String = id
        .chars()
        .take_while(|c| c.is_ascii_alphabetic())
        .collect::<String>()
        .to_ascii_lowercase();
    let similar: Vec<&str> = ids
        .iter()
        .filter(|i| !stem.is_empty() && i.to_ascii_lowercase().starts_with(&stem))
        .take(6)
        .map(String::as_str)
        .collect();
    let some = if similar.is_empty() {
        String::new()
    } else {
        format!(": {}", similar.join(", "))
    };
    let (en, ar) = if local {
        (
            format!("model `{name}`: `{id}` is not on this computer; run: ollama pull {id}"),
            format!("النموذج `{name}`: `{id}` غير موجود على هذا الجهاز؛ شغّل: ollama pull {id}"),
        )
    } else {
        (
            format!(
                "model `{name}`: `{id}` is not among the {} models this key can use; check `model` in elomar.toml{}",
                ids.len(),
                if some.is_empty() {
                    String::new()
                } else {
                    format!(" (similar{some})")
                }
            ),
            format!(
                "النموذج `{name}`: `{id}` ليس بين النماذج المتاحة لهذا المفتاح ({})؛ راجع `model` في elomar.toml{}",
                ids.len(),
                if some.is_empty() {
                    String::new()
                } else {
                    format!(" (مشابهة{some})")
                }
            ),
        )
    };
    report.line(if local { Level::Fail } else { Level::Warn }, &en, &ar);
}

/// Packages, Python, MCP servers, programs and folders.
fn check_project(report: &mut Report, config: &ProjectConfig, root: &Path) {
    match elomar_pkg::dependencies(root) {
        Ok(deps) if deps.is_empty() => {}
        Ok(deps) => match elomar_pkg::resolve(root, elomar_pkg::Mode::Offline) {
            Ok(_) => report.line(
                Level::Ok,
                &format!("{} package(s) downloaded and locked", deps.len()),
                &format!("{} حزمة منزّلة ومثبّتة في elomar.lock", deps.len()),
            ),
            Err(e) => report.line(
                Level::Fail,
                &format!("packages: {e}; run `elomar update` (online)"),
                &format!("الحزم: {e}؛ شغّل `elomar update` (مع الإنترنت)"),
            ),
        },
        Err(e) => report.line(
            Level::Fail,
            &format!("packages: {e}"),
            &format!("الحزم: {e}"),
        ),
    }

    if config.permissions.python {
        let python = std::env::var("ELOMAR_PYTHON")
            .ok()
            .filter(|p| !p.trim().is_empty())
            .unwrap_or_else(|| if cfg!(windows) { "python" } else { "python3" }.to_string());
        let version = std::process::Command::new(&python)
            .arg("--version")
            .output()
            .ok()
            .filter(|o| o.status.success())
            .map(|o| {
                let text = if o.stdout.is_empty() {
                    o.stderr
                } else {
                    o.stdout
                };
                String::from_utf8_lossy(&text).trim().to_string()
            });
        match version {
            Some(v) => report.line(
                Level::Ok,
                &format!("python = true: `{python}` is {v}"),
                &format!("python = true: `{python}` هو {v}"),
            ),
            None => report.line(
                Level::Fail,
                &format!("python = true, but `{python}` does not run; install Python or set ELOMAR_PYTHON to its path"),
                &format!("python = true، لكن `{python}` لا يعمل؛ ثبّت Python أو اضبط ELOMAR_PYTHON على مساره"),
            ),
        }
    }

    for (name, server) in &config.mcp.servers {
        match find_program(&server.command, root) {
            Some(path) => report.line(
                Level::Ok,
                &format!(
                    "MCP server `{name}`: `{}` found ({})",
                    server.command,
                    path.display()
                ),
                &format!(
                    "خادم MCP `{name}`: `{}` موجود ({})",
                    server.command,
                    path.display()
                ),
            ),
            None => report.line(
                Level::Fail,
                &format!(
                    "MCP server `{name}`: `{}` is not installed or not on PATH",
                    server.command
                ),
                &format!(
                    "خادم MCP `{name}`: `{}` غير مثبّت أو ليس في PATH",
                    server.command
                ),
            ),
        }
    }

    for command in &config.permissions.commands {
        if find_program(command, root).is_none() {
            report.line(
                Level::Warn,
                &format!("commands: `{command}` is allowed but not found on PATH"),
                &format!("commands: `{command}` مسموح لكنه غير موجود في PATH"),
            );
        }
    }

    for folder in &config.permissions.filesystem_read {
        if !root.join(folder).exists() {
            report.line(
                Level::Warn,
                &format!("filesystem_read: `{folder}` does not exist (yet)"),
                &format!("filesystem_read: `{folder}` غير موجود (بعد)"),
            );
        }
    }
    for folder in &config.permissions.filesystem_write {
        let path = root.join(folder);
        let parent_ok = path.exists() || path.parent().is_some_and(Path::is_dir);
        if !parent_ok {
            report.line(
                Level::Warn,
                &format!("filesystem_write: neither `{folder}` nor its folder exists"),
                &format!("filesystem_write: لا `{folder}` ولا مجلده موجود"),
            );
        }
    }
}

/// Where `program` is: a path (relative to the project), or a name looked
/// up in PATH (with the Windows extensions).
fn find_program(program: &str, root: &Path) -> Option<std::path::PathBuf> {
    let given = Path::new(program);
    if given.components().count() > 1 || given.is_absolute() {
        let path = root.join(given);
        return path.is_file().then_some(path);
    }
    let extensions: Vec<String> = if cfg!(windows) {
        std::env::var("PATHEXT")
            .unwrap_or_else(|_| ".COM;.EXE;.BAT;.CMD".into())
            .split(';')
            .map(str::to_ascii_lowercase)
            .chain([String::new()])
            .collect()
    } else {
        vec![String::new()]
    };
    let path = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path) {
        for ext in &extensions {
            let candidate = dir.join(format!("{program}{ext}"));
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }
    None
}
