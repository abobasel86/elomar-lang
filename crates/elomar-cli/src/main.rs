//! The `elomar` command-line tool.

use std::io::{BufRead, IsTerminal, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use elomar_ai_core::{AiRuntime, ModelProvider, ProjectConfig, UnavailableProvider, parse_config};
use elomar_ai_provider_mock::MockProvider;
use elomar_ai_provider_openai::OpenAiProvider;
use elomar_diagnostics::{Diagnostic, Lang, RenderOptions, SourceFile, render};
use elomar_interpreter::{FsPolicy, Host};

#[derive(Parser)]
#[command(
    name = "elomar",
    version = concat!(env!("CARGO_PKG_VERSION"), " (Elomar-Lang spec 0.1.0-draft.2)"),
    about = "Elomar-Lang — a general-purpose, AI-native programming language\nلغة Elomar-Lang — لغة برمجة عامة الأغراض، أصلية التكامل مع الذكاء الاصطناعي"
)]
struct Cli {
    /// Language for diagnostics: `en` or `ar` (default: $ELOMAR_LANG, else `en`)
    #[arg(long, global = true, value_name = "LANG")]
    lang: Option<String>,

    /// Disable colored output (also honors $NO_COLOR)
    #[arg(long, global = true)]
    no_color: bool,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Check a program and run it
    Run {
        /// Path to a `.elo` source file
        file: PathBuf,
    },
    /// Check a program for errors without running it
    Check {
        /// Path to a `.elo` source file
        file: PathBuf,
    },
}

/// Real terminal input/output for running programs.
struct StdHost;

impl Host for StdHost {
    fn print(&mut self, text: &str) {
        let mut out = std::io::stdout().lock();
        let _ = out.write_all(text.as_bytes());
    }

    fn input(&mut self, prompt: &str) -> Option<String> {
        let mut out = std::io::stdout().lock();
        let _ = out.write_all(prompt.as_bytes());
        let _ = out.flush();
        drop(out);
        let mut line = String::new();
        match std::io::stdin().lock().read_line(&mut line) {
            Ok(0) | Err(_) => None,
            Ok(_) => Some(line.trim_end_matches(['\n', '\r']).to_string()),
        }
    }
}

struct Reporter {
    opts: RenderOptions,
}

impl Reporter {
    fn emit(&self, diag: &Diagnostic, file: &SourceFile) {
        eprintln!("{}", render(diag, file, self.opts));
    }

    fn say(&self, en: &str, ar: &str) {
        eprintln!("{}", if self.opts.lang == Lang::Ar { ar } else { en });
    }
}

fn main() -> ExitCode {
    let cli = Cli::parse();

    let lang = match cli
        .lang
        .clone()
        .or_else(|| std::env::var("ELOMAR_LANG").ok())
    {
        None => Lang::En,
        Some(s) => match Lang::parse(&s) {
            Some(l) => l,
            None => {
                eprintln!("error: unknown language `{s}`; use `en` or `ar`");
                return ExitCode::from(2);
            }
        },
    };
    let color =
        !cli.no_color && std::env::var_os("NO_COLOR").is_none() && std::io::stderr().is_terminal();
    let reporter = Reporter {
        opts: RenderOptions { lang, color },
    };

    let (path, execute) = match &cli.command {
        Command::Run { file } => (file.as_path(), true),
        Command::Check { file } => (file.as_path(), false),
    };
    run_file(path, execute, &reporter)
}

fn run_file(path: &Path, execute: bool, reporter: &Reporter) -> ExitCode {
    let text = match std::fs::read_to_string(path) {
        Ok(t) => t,
        Err(e) => {
            let shown = path.display();
            reporter.say(
                &format!("error: cannot read `{shown}`: {e}"),
                &format!("خطأ: تعذّرت قراءة `{shown}`: {e}"),
            );
            return ExitCode::from(2);
        }
    };
    let file = SourceFile::new(path.display().to_string(), text);

    // The project config decides which models are cloud models (`net`), so
    // it is loaded before checking.
    let ai = match load_ai_runtime(path) {
        Ok(ai) => ai,
        Err((config_path, message)) => {
            let shown = config_path.display();
            reporter.say(
                &format!("error: invalid `{shown}`: {message}"),
                &format!("خطأ: الملف `{shown}` غير صالح: {message}"),
            );
            return ExitCode::from(2);
        }
    };

    let (mut program, mut diags) = elomar_parser::parse(&file.text);
    if diags.iter().all(|d| !d.is_error()) {
        diags.extend(elomar_types::check(
            &mut program,
            elomar_types::CheckOptions {
                require_main: true,
                cloud_models: ai.cloud_models,
            },
        ));
    }
    diags.sort_by_key(|d| (d.label.span.start, !d.is_error()));
    for d in &diags {
        reporter.emit(d, &file);
    }
    let errors = diags.iter().filter(|d| d.is_error()).count();
    let warnings = diags.len() - errors;
    if errors > 0 {
        reporter.say(
            &format!(
                "error: `{}` has {errors} error(s) and {warnings} warning(s); it was not run",
                file.name
            ),
            &format!(
                "خطأ: في `{}` عدد {errors} من الأخطاء و{warnings} من التحذيرات؛ لم يُنفَّذ البرنامج",
                file.name
            ),
        );
        return ExitCode::from(1);
    }
    if !execute {
        reporter.say(
            &format!("`{}`: no errors ({warnings} warning(s))", file.name),
            &format!("`{}`: لا توجد أخطاء ({warnings} من التحذيرات)", file.name),
        );
        return ExitCode::SUCCESS;
    }

    let mut runtime = ai.runtime;
    let fs = ai.fs;
    let result = elomar_interpreter::with_large_stack(|| {
        elomar_interpreter::run_with(&program, &mut StdHost, &mut runtime, fs)
    });
    let _ = std::io::stdout().flush();
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            reporter.emit(&err.to_diagnostic(), &file);
            ExitCode::from(1)
        }
    }
}

/// The nearest `elomar.toml` in the source file's directory or above it.
fn find_config(source: &Path) -> Option<PathBuf> {
    let absolute = std::path::absolute(source).ok()?;
    absolute
        .ancestors()
        .skip(1)
        .map(|dir| dir.join("elomar.toml"))
        .find(|candidate| candidate.is_file())
}

struct LoadedAi {
    runtime: AiRuntime,
    /// Models whose provider uses the network (they need the `net` effect).
    cloud_models: Vec<String>,
    /// What `std.fs` may read and write.
    fs: FsPolicy,
}

/// Builds the models, policy and network permissions for `ai.infer` from
/// `elomar.toml`. The built-in model `mock` (placeholder answers) is always
/// available.
fn load_ai_runtime(source: &Path) -> Result<LoadedAi, (PathBuf, String)> {
    let mut config = ProjectConfig::default();
    let config_path = find_config(source);
    if let Some(p) = &config_path {
        let text = std::fs::read_to_string(p).map_err(|e| (p.clone(), e.to_string()))?;
        config = parse_config(&text).map_err(|e| (p.clone(), e))?;
    }
    let config_error = |message: String| (config_path.clone().unwrap_or_default(), message);
    // Relative paths (in the program and in `[permissions]`) start at the
    // project folder: where elomar.toml is, else the program's folder.
    let base = config_path
        .as_deref()
        .or(Some(source))
        .and_then(Path::parent)
        .and_then(|p| std::path::absolute(p).ok())
        .unwrap_or_else(|| PathBuf::from("."));
    let fs = FsPolicy::new(
        base,
        &config.permissions.filesystem_read,
        &config.permissions.filesystem_write,
    );
    let mut runtime =
        AiRuntime::new(config.ai.policy.clone()).with_permissions(config.permissions.clone());
    runtime.register(
        "mock",
        Box::new(MockProvider::new(Vec::new()).expect("no rules to validate")),
    );
    let mut cloud_models = Vec::new();
    for (name, model) in config.ai.models {
        let provider: Box<dyn ModelProvider> = match model.provider.as_str() {
            "mock" => Box::new(
                MockProvider::new(model.mock.clone())
                    .map_err(|e| config_error(format!("model `{name}`: {e}")))?,
            ),
            "openai" => Box::new(OpenAiProvider::from_config(&name, &model).map_err(config_error)?),
            other => Box::new(UnavailableProvider {
                provider: other.to_string(),
            }),
        };
        if provider.capabilities().uses_network {
            cloud_models.push(name.clone());
        }
        runtime.register(name, provider);
    }
    Ok(LoadedAi {
        runtime,
        cloud_models,
        fs,
    })
}
