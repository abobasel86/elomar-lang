//! The `elomar` command-line tool.

use std::io::{BufRead, IsTerminal, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use elomar_ai_core::{
    AiRuntime, EmbeddingCache, ModelProvider, ProjectConfig, UnavailableProvider, parse_config,
};
use elomar_ai_provider_anthropic::AnthropicProvider;
use elomar_ai_provider_mock::MockProvider;
use elomar_ai_provider_openai::OpenAiProvider;
use elomar_diagnostics::{Diagnostic, Lang, RenderOptions, SourceMap, render_in};
use elomar_interpreter::{FsPolicy, Host};

#[derive(Parser)]
#[command(
    name = "elomar",
    version = concat!(env!("CARGO_PKG_VERSION"), " (Elomar-Lang spec 1.0.0)"),
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
        /// Arguments for the program, read with `os.args()`
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// Make one executable file of a program, to run without Elomar-Lang
    Build {
        /// Path to the program's main `.elo` file
        file: PathBuf,
        /// The executable to write (default: the file's name, in this folder)
        #[arg(short, long, value_name = "FILE")]
        output: Option<PathBuf>,
    },
    /// Check a program for errors without running it
    Check {
        /// Path to a `.elo` source file
        file: PathBuf,
        /// Output format: `text`, or `json` for editors (does not require `main`)
        #[arg(long, default_value = "text", value_name = "FORMAT")]
        format: String,
        /// Read the file's text from standard input (an unsaved editor buffer)
        #[arg(long)]
        stdin: bool,
    },
    /// Run the `test_*` functions in the project's `tests/` folder
    Test {
        /// Project folder, or a single test file
        #[arg(default_value = ".")]
        path: PathBuf,
        /// Only run tests whose name contains this text
        #[arg(long, value_name = "TEXT")]
        filter: Option<String>,
    },
    /// Format .elo files in place (folders are searched recursively)
    Fmt {
        /// Files or folders (default: the current folder)
        #[arg(default_value = ".")]
        paths: Vec<PathBuf>,
        /// Only report files that need formatting; exit 1 if any do
        #[arg(long)]
        check: bool,
        /// Format standard input and print the result (for editors)
        #[arg(long)]
        stdin: bool,
        /// Write every keyword in one language: `ar` (`دالة`, `ليكن`, …)
        /// or `en` (`fn`, `let`, …). Without it, each keeps its spelling.
        #[arg(long, value_name = "ar|en", value_parser = ["ar", "en"])]
        keywords: Option<String>,
    },
    /// Create a new project folder with a program, a module and a test
    New {
        /// Name of the project (and its folder)
        name: String,
        /// Make a project that calls a model, set up for this provider:
        /// mock (offline), ollama (on this computer), openai or anthropic
        #[arg(long, value_name = "PROVIDER")]
        ai: Option<String>,
    },
    /// Check the project's setup: AI keys, permissions, servers and model
    /// ids, MCP servers, Python, packages and folders
    Doctor,
    /// Move this `elomar` to the newest release
    Upgrade {
        /// Only say whether a newer version exists
        #[arg(long)]
        check: bool,
        /// Install the latest release even if this one is as new
        #[arg(long)]
        force: bool,
    },
    /// Type code and see the result at once (an interactive prompt)
    Repl,
    /// Run the language server (LSP) on standard input/output, for editors
    Lsp,
    /// Run the debugger (Debug Adapter Protocol) on standard input/output,
    /// for editors
    Dap,
    /// Add a dependency to elomar.toml and download it: from the registry
    /// (`strings` or `strings@1.2`), a folder (`--path`) or git (`--git`)
    Add {
        /// The package name, used as `use <name>.<module>`; for registry
        /// packages optionally with a version requirement: `name@1.2`
        name: String,
        /// A local folder with the package
        #[arg(long, value_name = "FOLDER", conflicts_with = "git")]
        path: Option<String>,
        /// A git repository URL
        #[arg(long, value_name = "URL")]
        git: Option<String>,
        /// Git tag to use
        #[arg(long, requires = "git", conflicts_with_all = ["branch", "rev"])]
        tag: Option<String>,
        /// Git branch to use
        #[arg(long, requires = "git", conflicts_with = "rev")]
        branch: Option<String>,
        /// Git commit to use
        #[arg(long, requires = "git")]
        rev: Option<String>,
    },
    /// Remove a dependency from elomar.toml
    Remove { name: String },
    /// Move dependencies to the newest allowed versions and commits, and
    /// update elomar.lock
    Update,
    /// List packages in the registry whose name or description matches
    Search {
        /// Text to look for (default: list all)
        query: Option<String>,
    },
    /// Add this package's version (from `[package]`) to a registry index
    /// folder; then commit and push the index to publish it
    Publish {
        /// A local clone of the registry index (default: the project's
        /// registry, if it is a folder)
        #[arg(long, value_name = "FOLDER")]
        index: Option<String>,
        /// The repository URL others download from (default: `origin`)
        #[arg(long, value_name = "URL")]
        git: Option<String>,
    },
}

/// Real terminal input/output for running programs.
struct StdHost {
    args: Vec<String>,
}

impl Host for StdHost {
    fn args(&self) -> Vec<String> {
        self.args.clone()
    }

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
    fn emit(&self, diag: &Diagnostic, sources: &SourceMap) {
        eprintln!("{}", render_in(diag, sources, self.opts));
    }

    /// Renders to standard output (for `elomar test` failures).
    fn emit_out(&self, diag: &Diagnostic, sources: &SourceMap) {
        print!("{}", render_in(diag, sources, self.opts));
    }

    /// A message on standard error, in the chosen language.
    fn say(&self, en: &str, ar: &str) {
        eprintln!("{}", if self.opts.lang == Lang::Ar { ar } else { en });
    }

    /// A message on standard output, in the chosen language.
    fn out(&self, en: &str, ar: &str) {
        println!("{}", if self.opts.lang == Lang::Ar { ar } else { en });
    }
}

/// A faster allocator: the interpreter allocates many small values.
#[global_allocator]
static ALLOCATOR: mimalloc::MiMalloc = mimalloc::MiMalloc;

fn main() -> ExitCode {
    // `ELOMAR_ENGINE=tree` runs programs in the tree-walking interpreter
    // only, to compare it with the bytecode engine.
    if std::env::var("ELOMAR_ENGINE").is_ok_and(|e| e == "tree") {
        elomar_interpreter::set_engine(elomar_interpreter::Engine::Tree);
    }
    // `ELOMAR_PARALLEL=0` runs the calls in `await all([...])` one after
    // another, never on several cores.
    if std::env::var("ELOMAR_PARALLEL").is_ok_and(|p| p == "0") {
        elomar_interpreter::set_parallel(false);
    }
    // A program made by `elomar build`: run it, with every argument its own.
    if let Some(bundle) = build::embedded() {
        let lang = std::env::var("ELOMAR_LANG")
            .ok()
            .and_then(|s| Lang::parse(&s))
            .unwrap_or(Lang::En);
        let color = std::env::var_os("NO_COLOR").is_none() && std::io::stderr().is_terminal();
        return build::run_embedded(
            bundle,
            &Reporter {
                opts: RenderOptions { lang, color },
            },
        );
    }
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

    let mut program_args = Vec::new();
    let (path, execute) = match &cli.command {
        Command::Run { file, args } => {
            program_args = args.clone();
            (file.as_path(), true)
        }
        Command::Test { path, filter } => {
            return testing::run_tests(path, filter.as_deref(), &reporter);
        }
        Command::New { name, ai } => return scaffold::new_project(name, ai.as_deref(), &reporter),
        Command::Build { file, output } => return build::build(file, output.as_deref(), &reporter),
        Command::Lsp => return lsp::serve(lang),
        Command::Repl => return repl::run(&reporter),
        Command::Doctor => return doctor::run(&reporter),
        Command::Upgrade { check, force } => return upgrade::run(*check, *force, &reporter),
        Command::Dap => return dap::serve(lang),
        Command::Add {
            name,
            path,
            git,
            tag,
            branch,
            rev,
        } => {
            return packages::add(
                &packages::AddArgs {
                    name,
                    path: path.as_deref(),
                    git: git.as_deref(),
                    tag: tag.as_deref(),
                    branch: branch.as_deref(),
                    rev: rev.as_deref(),
                },
                &reporter,
            );
        }
        Command::Remove { name } => return packages::remove(name, &reporter),
        Command::Update => return packages::update(&reporter),
        Command::Search { query } => {
            return packages::search(query.as_deref().unwrap_or(""), &reporter);
        }
        Command::Publish { index, git } => {
            return packages::publish(index.as_deref(), git.as_deref(), &reporter);
        }
        Command::Fmt {
            paths,
            check,
            stdin,
            keywords,
        } => {
            let keywords = keywords.as_deref().map(|k| match k {
                "ar" => elomar_fmt::Keywords::Arabic,
                _ => elomar_fmt::Keywords::English,
            });
            return fmt::run_fmt(paths, *check, *stdin, keywords, &reporter);
        }
        Command::Check {
            file,
            format,
            stdin,
        } => {
            match format.as_str() {
                "text" if !stdin => {}
                "text" => {
                    eprintln!("error: `--stdin` is only supported with `--format json`");
                    return ExitCode::from(2);
                }
                "json" => return editor::check_json(file, *stdin, lang),
                other => {
                    eprintln!("error: unknown format `{other}`; use `text` or `json`");
                    return ExitCode::from(2);
                }
            }
            (file.as_path(), false)
        }
    };
    run_file(path, execute, program_args, &reporter)
}

fn run_file(path: &Path, execute: bool, args: Vec<String>, reporter: &Reporter) -> ExitCode {
    packages::fetch_for(path, reporter);
    let loaded = match elomar_loader::load(path, &project_root(path)) {
        Ok(l) => l,
        Err(e) => {
            let shown = path.display();
            reporter.say(
                &format!("error: cannot read `{shown}`: {e}"),
                &format!("خطأ: تعذّرت قراءة `{shown}`: {e}"),
            );
            return ExitCode::from(2);
        }
    };
    let name = path.display().to_string();

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
    check_and_run(loaded, ai, &name, execute, args, reporter)
}

/// Checks a loaded program and, if `execute` and it has no errors, runs it.
pub(crate) fn check_and_run(
    loaded: elomar_loader::Loaded,
    ai: LoadedAi,
    name: &str,
    execute: bool,
    args: Vec<String>,
    reporter: &Reporter,
) -> ExitCode {
    let sources = loaded.sources;
    let mut program = loaded.program;
    let mut diags = loaded.diagnostics;
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
        reporter.emit(d, &sources);
    }
    let errors = diags.iter().filter(|d| d.is_error()).count();
    let warnings = diags.len() - errors;
    if errors > 0 {
        reporter.say(
            &format!(
                "error: `{name}` has {errors} error(s) and {warnings} warning(s); it was not run"
            ),
            &format!(
                "خطأ: في `{name}` عدد {errors} من الأخطاء و{warnings} من التحذيرات؛ لم يُنفَّذ البرنامج"
            ),
        );
        return ExitCode::from(1);
    }
    if !execute {
        reporter.say(
            &format!("`{name}`: no errors ({warnings} warning(s))"),
            &format!("`{name}`: لا توجد أخطاء ({warnings} من التحذيرات)"),
        );
        return ExitCode::SUCCESS;
    }

    let mut runtime = ai.runtime;
    let fs = ai.fs;
    let result = elomar_interpreter::with_large_stack(|| {
        elomar_interpreter::run_with(&program, &mut StdHost { args }, &mut runtime, fs)
    });
    let _ = std::io::stdout().flush();
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) if err.exit.is_some() => ExitCode::from(err.exit.unwrap_or(0) as u8),
        Err(err) => {
            reporter.emit(&err.to_diagnostic(), &sources);
            ExitCode::from(1)
        }
    }
}

mod build;
mod dap;
mod doctor;
mod editor;
mod fmt;
mod lsp;
mod packages;
mod repl;
mod scaffold;
mod scaffold_ai;
mod testing;
mod upgrade;

/// The project folder: where `elomar.toml` is, else the program's folder.
/// Relative paths and modules (`src/`) are looked up from here.
pub(crate) fn project_root(source: &Path) -> PathBuf {
    find_config(source)
        .as_deref()
        .or(Some(source))
        .and_then(Path::parent)
        .and_then(|p| std::path::absolute(p).ok())
        .unwrap_or_else(|| PathBuf::from("."))
}

/// The nearest `elomar.toml` in the source file's directory or above it.
pub(crate) fn find_config(source: &Path) -> Option<PathBuf> {
    let absolute = std::path::absolute(source).ok()?;
    absolute
        .ancestors()
        .skip(1)
        .map(|dir| dir.join("elomar.toml"))
        .find(|candidate| candidate.is_file())
}

pub(crate) struct LoadedAi {
    pub(crate) runtime: AiRuntime,
    /// Models whose provider uses the network (they need the `net` effect).
    pub(crate) cloud_models: Vec<String>,
    /// What `std.fs` may read and write.
    fs: FsPolicy,
}

/// Builds the models, policy and network permissions for `ai.infer` from
/// `elomar.toml`. The built-in model `mock` (placeholder answers) is always
/// available.
pub(crate) fn load_ai_runtime(source: &Path) -> Result<LoadedAi, (PathBuf, String)> {
    let config_path = find_config(source);
    let text = match &config_path {
        Some(p) => Some(std::fs::read_to_string(p).map_err(|e| (p.clone(), e.to_string()))?),
        None => None,
    };
    // Relative paths (in the program and in `[permissions]`) start at the
    // project folder: where elomar.toml is, else the program's folder.
    ai_runtime_from(text.as_deref(), project_root(source))
        .map_err(|message| (config_path.unwrap_or_default(), message))
}

/// Like [`load_ai_runtime`], from the text of `elomar.toml` (if any) and
/// the folder relative paths start at.
pub(crate) fn ai_runtime_from(
    config_text: Option<&str>,
    base: PathBuf,
) -> Result<LoadedAi, String> {
    let config = match config_text {
        Some(text) => parse_config(text)?,
        None => ProjectConfig::default(),
    };
    let config_error = |message: String| message;
    let project = base.clone();
    let fs = FsPolicy::new(
        base,
        &config.permissions.filesystem_read,
        &config.permissions.filesystem_write,
    );
    let mut runtime = AiRuntime::new(config.ai.policy.clone())
        .with_permissions(config.permissions.clone())
        .with_mcp(config.mcp.clone());
    runtime.register(
        "mock",
        Box::new(MockProvider::new(Vec::new()).expect("no rules to validate")),
    );
    let mut cache = match &config.ai.cache.embeddings {
        Some(file) => {
            let relative = Path::new(file);
            if relative.is_absolute()
                || relative
                    .components()
                    .any(|c| matches!(c, std::path::Component::ParentDir))
            {
                return Err(format!(
                    "`[ai.cache] embeddings = {file:?}` must be a path inside the project folder"
                ));
            }
            Some(EmbeddingCache::open(project.join(relative)))
        }
        None => None,
    };
    let mut cloud_models = Vec::new();
    for (name, model) in config.ai.models {
        let provider: Box<dyn ModelProvider> = match model.provider.as_str() {
            "mock" => Box::new(
                MockProvider::new(model.mock.clone())
                    .map_err(|e| config_error(format!("model `{name}`: {e}")))?,
            ),
            "openai" => Box::new(OpenAiProvider::from_config(&name, &model).map_err(config_error)?),
            "anthropic" => {
                Box::new(AnthropicProvider::from_config(&name, &model).map_err(config_error)?)
            }
            other => Box::new(UnavailableProvider {
                provider: other.to_string(),
            }),
        };
        if provider.capabilities().uses_network {
            cloud_models.push(name.clone());
            if let Some(cache) = cache.as_mut() {
                cache.add_model(
                    &name,
                    format!(
                        "{} {} {}",
                        model.provider,
                        model.model.as_deref().unwrap_or(""),
                        model.base_url.as_deref().unwrap_or("")
                    ),
                );
            }
        }
        runtime.register(name, provider);
    }
    if let Some(cache) = cache {
        runtime = runtime.with_embedding_cache(cache);
    }
    Ok(LoadedAi {
        runtime,
        cloud_models,
        fs,
    })
}
