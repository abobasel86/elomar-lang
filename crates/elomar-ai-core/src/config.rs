//! The AI-related parts of `elomar.toml` (spec §14.4). Unknown sections and
//! keys (e.g. `[package]`, `[permissions]`) are ignored here.

use std::collections::BTreeMap;

use serde::Deserialize;

#[derive(Debug, Clone, Default, Deserialize)]
pub struct ProjectConfig {
    #[serde(default)]
    pub ai: AiConfig,
    #[serde(default)]
    pub permissions: Permissions,
    #[serde(default)]
    pub mcp: McpConfig,
}

/// `[mcp.servers.NAME]`: the MCP servers the program may start. A server
/// that is not listed cannot be used.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct McpConfig {
    #[serde(default)]
    pub servers: BTreeMap<String, McpServerConfig>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct McpServerConfig {
    /// The program to run, e.g. `python` or `npx`.
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
    /// Extra environment variables for the server.
    #[serde(default)]
    pub env: BTreeMap<String, String>,
    /// Limit for each request to the server (default 30 s).
    #[serde(default)]
    pub timeout_ms: Option<u64>,
}

/// `[permissions]` (spec §14.4, §16.3). Everything not listed is denied.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct Permissions {
    /// Hosts the program may reach, as `host` or `host:port`.
    #[serde(default)]
    pub network_hosts: Vec<String>,
    /// Files or folders `std.fs` may read, relative to the project folder.
    #[serde(default)]
    pub filesystem_read: Vec<String>,
    /// Files or folders `std.fs` may create, overwrite or append to.
    #[serde(default)]
    pub filesystem_write: Vec<String>,
    /// Whether `std.py` may run Python (which is not sandboxed).
    #[serde(default)]
    pub python: bool,
}

impl Permissions {
    /// Whether `host` (optionally on `port`) is on the allowlist.
    /// Matching is exact and case-insensitive; there are no wildcards.
    pub fn allows_host(&self, host: &str, port: u16) -> bool {
        let with_port = format!("{host}:{port}");
        self.network_hosts.iter().any(|allowed| {
            allowed.eq_ignore_ascii_case(host) || allowed.eq_ignore_ascii_case(&with_port)
        })
    }
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct AiConfig {
    /// Logical model name → configuration.
    #[serde(default)]
    pub models: BTreeMap<String, ModelConfig>,
    #[serde(default)]
    pub policy: AiPolicy,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ModelConfig {
    /// `mock`, or a cloud provider name (not implemented yet).
    pub provider: String,
    /// The vendor's model id, for real providers.
    #[serde(default)]
    pub model: Option<String>,
    /// Name of the environment variable that holds the API key. The key
    /// itself never belongs in the file.
    #[serde(default)]
    pub api_key_env: Option<String>,
    /// API base URL, e.g. for an OpenAI-compatible gateway. Its host must
    /// be listed in `[permissions] network_hosts`.
    #[serde(default)]
    pub base_url: Option<String>,
    /// Canned answers for the `mock` provider, tried in order.
    #[serde(default)]
    pub mock: Vec<MockRule>,
}

/// One canned answer of the mock provider.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MockRule {
    /// The rule applies only if the request's input contains this text.
    #[serde(default)]
    pub input_contains: Option<String>,
    /// The model's raw answer (usually JSON).
    #[serde(default)]
    pub response: Option<String>,
    /// Simulate a failure: an `AIError` kind such as `Timeout`.
    #[serde(default)]
    pub error: Option<String>,
    #[serde(default)]
    pub message: Option<String>,
    /// Simulated latency; above the call's timeout it yields `Timeout`.
    #[serde(default)]
    pub delay_ms: Option<u64>,
    /// For `ai.run`: instead of answering, ask to call this tool.
    #[serde(default)]
    pub tool: Option<String>,
    /// The tool's arguments, e.g. `{ name = "Lina" }`.
    #[serde(default)]
    pub arguments: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct AiPolicy {
    #[serde(default)]
    pub max_calls_per_run: Option<u64>,
    /// Used when a call gives no `timeout:`.
    #[serde(default)]
    pub default_timeout_ms: Option<u64>,
}

/// Parses the text of an `elomar.toml` file.
pub fn parse_config(text: &str) -> Result<ProjectConfig, String> {
    toml::from_str(text).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_spec_example() {
        let cfg = parse_config(
            r#"
            [package]
            name = "ticket-system"

            [ai.models."support.classifier"]
            provider = "mock"

            [[ai.models."support.classifier".mock]]
            input_contains = "password"
            response = '{"category": "account"}'

            [[ai.models."support.classifier".mock]]
            error = "Timeout"

            [ai.policy]
            max_calls_per_run = 20
            estimated_budget_usd = 0.50

            [permissions]
            subprocess = false
            "#,
        )
        .unwrap();
        let m = &cfg.ai.models["support.classifier"];
        assert_eq!(m.provider, "mock");
        assert_eq!(m.mock.len(), 2);
        assert_eq!(m.mock[1].error.as_deref(), Some("Timeout"));
        assert_eq!(cfg.ai.policy.max_calls_per_run, Some(20));
    }

    #[test]
    fn permissions_allowlist() {
        let cfg = parse_config(
            "[permissions]\nnetwork_hosts = [\"api.openai.com\", \"127.0.0.1:8080\"]\nfilesystem_read = [\"./data\"]\n",
        )
        .unwrap();
        let p = &cfg.permissions;
        assert!(p.allows_host("api.openai.com", 443));
        assert!(p.allows_host("API.OpenAI.com", 443));
        assert!(p.allows_host("127.0.0.1", 8080));
        assert!(!p.allows_host("127.0.0.1", 9090));
        assert!(!p.allows_host("evil.example", 443));
        assert!(!Permissions::default().allows_host("api.openai.com", 443));
        assert_eq!(p.filesystem_read, vec!["./data".to_string()]);
    }

    #[test]
    fn parses_mcp_servers() {
        let cfg = parse_config(
            "[mcp.servers.notes]
command = \"python\"
args = [\"mcp/notes.py\"]
env = { LEVEL = \"1\" }
",
        )
        .unwrap();
        let s = &cfg.mcp.servers["notes"];
        assert_eq!(s.command, "python");
        assert_eq!(s.args, vec!["mcp/notes.py".to_string()]);
        assert_eq!(s.env["LEVEL"], "1");
        assert!(
            parse_config(
                "[mcp.servers.x]
command = \"a\"
arg = []
"
            )
            .is_err()
        );
    }

    #[test]
    fn rejects_typos_in_mock_rules() {
        let err = parse_config(
            "[ai.models.m]\nprovider = \"mock\"\n[[ai.models.m.mock]]\nresponce = \"x\"\n",
        )
        .unwrap_err();
        assert!(err.contains("responce"), "{err}");
    }
}
