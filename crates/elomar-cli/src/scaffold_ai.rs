//! `elomar new <name> --ai <provider>`: a project that calls a model and
//! runs at once, with `elomar.toml` already set for that provider.

/// The providers `--ai` accepts.
pub(crate) const PROVIDERS: [&str; 4] = ["mock", "ollama", "openai", "anthropic"];

const TICKETS: &str = r#"// A module: `use app.tickets` loads this file.
use std.ai

pub type Ticket = { category: String, urgent: Bool }

// A public function says that it calls a model (`ai`) over the network (`net`).
pub async fn classify(text: String) -> Result<Ticket, AIError> !{EFFECTS} {
    return await ai.infer<Ticket>(
        model: "assistant",
        instructions: "Classify the support request: a short category, and whether it is urgent.",
        input: text,
        timeout: 60s
    )
}

pub fn describe(t: Ticket) -> String {
    let mark = match t.urgent {
        true => "URGENT"
        false => "normal"
    }
    return "[{mark}] {t.category}"
}
"#;

const MAIN: &str = r#"// Run it with `elomar run src/main.elo`; `elomar doctor` checks the setup.
use std.ai
use app.tickets

async fn main() {
    match await tickets.classify("I was charged twice for my order!") {
        Ok(t) => print(tickets.describe(t))
        Err(e) => print("AI failed ({e.kind()}): {e.message()}")
    }
SEARCH}
"#;

const SEARCH: &str = r#"
    // Search notes by meaning (the vectors are kept in [ai.cache]).
    let notes = ["refund for a double charge", "reset my password", "the app crashes on start"]
    match await ai.search("vectors", "I paid twice", notes, 60s) {
        Ok(hits) => print("closest note:", hits[0].text)
        Err(e) => print("search failed ({e.kind()}): {e.message()}")
    }
"#;

const TEST: &str = r#"// Run with `elomar test`. Every `fn test_...()` in tests/ is a test.
use app.tickets

fn test_describes_urgent_tickets() {
    assert_eq(tickets.describe({ category: "billing", urgent: true }), "[URGENT] billing")
}

fn test_describes_normal_tickets() {
    assert_eq(tickets.describe({ category: "account", urgent: false }), "[normal] account")
}
"#;

fn config(name: &str, provider: &str) -> String {
    let package =
        format!("[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nentry = \"src/main.elo\"\n\n");
    let models = match provider {
        "mock" => r#"# "mock" answers offline from the rules below: no key, no network.
[ai.models.assistant]
provider = "mock"

[[ai.models.assistant.mock]]
input_contains = "charged"
response = '{"category": "billing", "urgent": true}'

[ai.models.vectors]
provider = "mock"

[ai.policy]
max_calls_per_run = 50

[permissions]
network_hosts = []
"#
        .to_string(),
        "ollama" => r#"# Models on this computer with Ollama (https://ollama.com): no key, no cost.
# First: ollama pull qwen2.5:7b   and   ollama pull nomic-embed-text
[ai.models.assistant]
provider = "openai"                     # Ollama speaks the OpenAI API
model = "qwen2.5:7b"
base_url = "http://localhost:11434/v1"

[ai.models.vectors]
provider = "openai"
model = "nomic-embed-text"
base_url = "http://localhost:11434/v1"

[ai.cache]
embeddings = "embeddings.cache.json"

[ai.policy]
max_calls_per_run = 50

[permissions]
network_hosts = ["localhost:11434"]
"#
        .to_string(),
        "openai" => r#"# OpenAI. The key comes from the environment variable, never from this file:
#   PowerShell: $env:OPENAI_API_KEY = "..."      other shells: export OPENAI_API_KEY=...
[ai.models.assistant]
provider = "openai"
model = "REPLACE_WITH_MODEL_ID"         # a chat model id from platform.openai.com/docs/models
api_key_env = "OPENAI_API_KEY"

[ai.models.vectors]
provider = "openai"
model = "text-embedding-3-small"
api_key_env = "OPENAI_API_KEY"

[ai.cache]
embeddings = "embeddings.cache.json"

[ai.policy]
max_calls_per_run = 20                  # each call costs money; this stops a runaway loop

[permissions]
network_hosts = ["api.openai.com"]
"#
        .to_string(),
        _ => r#"# Claude. The key comes from the environment variable, never from this file:
#   PowerShell: $env:ANTHROPIC_API_KEY = "..."   other shells: export ANTHROPIC_API_KEY=...
[ai.models.assistant]
provider = "anthropic"
model = "claude-haiku-5-5"
api_key_env = "ANTHROPIC_API_KEY"

[ai.policy]
max_calls_per_run = 20                  # each call costs money; this stops a runaway loop

[permissions]
network_hosts = ["api.anthropic.com"]
"#
        .to_string(),
    };
    package + &models
}

/// The files of an AI project for `provider`.
pub(crate) fn files(name: &str, provider: &str) -> Vec<(&'static str, String)> {
    let effects = if provider == "mock" { "ai" } else { "ai, net" };
    // Claude makes no embeddings, so its project has no search.
    let search = if provider == "anthropic" { "" } else { SEARCH };
    vec![
        ("elomar.toml", config(name, provider)),
        ("src/main.elo", MAIN.replace("SEARCH", search)),
        ("src/tickets.elo", TICKETS.replace("EFFECTS", effects)),
        ("tests/tickets_test.elo", TEST.to_string()),
        (".gitignore", "embeddings.cache.json\n".to_string()),
    ]
}

/// What to do before the first run, in English and Arabic.
pub(crate) fn before_running(provider: &str) -> (&'static str, &'static str) {
    match provider {
        "ollama" => (
            "  ollama pull qwen2.5:7b\n  ollama pull nomic-embed-text\n",
            "  ollama pull qwen2.5:7b\n  ollama pull nomic-embed-text\n",
        ),
        "openai" => (
            "  put a chat model id in `model` in elomar.toml\n  $env:OPENAI_API_KEY = \"...\"   (in your own terminal)\n",
            "  اكتب معرّف نموذج محادثة في `model` في elomar.toml\n  $env:OPENAI_API_KEY = \"...\"   (في نافذتك أنت)\n",
        ),
        "anthropic" => (
            "  $env:ANTHROPIC_API_KEY = \"...\"   (in your own terminal)\n",
            "  $env:ANTHROPIC_API_KEY = \"...\"   (في نافذتك أنت)\n",
        ),
        _ => ("", ""),
    }
}
