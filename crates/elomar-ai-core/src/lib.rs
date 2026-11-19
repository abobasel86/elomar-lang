//! Provider-neutral AI runtime pieces (spec §13–§14, §16.5):
//! output schemas, requests, error kinds, project configuration, and the
//! [`AiRuntime`] that applies policy, calls a provider and validates the
//! response against the requested type.
//!
//! This crate knows nothing about the interpreter's values or about any
//! specific provider.

mod config;
mod schema;

use std::collections::HashMap;

pub use config::{
    AiConfig, AiPolicy, MockRule, ModelConfig, Permissions, ProjectConfig, parse_config,
};
pub use schema::Schema;

/// Error categories of spec §13.4. Elomar programs see them as the `kind`
/// field of an `AIError` value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AiErrorKind {
    Authentication,
    ProviderUnavailable,
    RateLimited,
    Timeout,
    Network,
    UnsupportedCapability,
    InvalidStructuredOutput,
    PolicyDenied,
    BudgetExceeded,
    ContextLimitExceeded,
    Cancelled,
}

impl AiErrorKind {
    pub const ALL: [AiErrorKind; 11] = [
        AiErrorKind::Authentication,
        AiErrorKind::ProviderUnavailable,
        AiErrorKind::RateLimited,
        AiErrorKind::Timeout,
        AiErrorKind::Network,
        AiErrorKind::UnsupportedCapability,
        AiErrorKind::InvalidStructuredOutput,
        AiErrorKind::PolicyDenied,
        AiErrorKind::BudgetExceeded,
        AiErrorKind::ContextLimitExceeded,
        AiErrorKind::Cancelled,
    ];

    pub fn name(self) -> &'static str {
        match self {
            AiErrorKind::Authentication => "Authentication",
            AiErrorKind::ProviderUnavailable => "ProviderUnavailable",
            AiErrorKind::RateLimited => "RateLimited",
            AiErrorKind::Timeout => "Timeout",
            AiErrorKind::Network => "Network",
            AiErrorKind::UnsupportedCapability => "UnsupportedCapability",
            AiErrorKind::InvalidStructuredOutput => "InvalidStructuredOutput",
            AiErrorKind::PolicyDenied => "PolicyDenied",
            AiErrorKind::BudgetExceeded => "BudgetExceeded",
            AiErrorKind::ContextLimitExceeded => "ContextLimitExceeded",
            AiErrorKind::Cancelled => "Cancelled",
        }
    }

    pub fn parse(name: &str) -> Option<AiErrorKind> {
        Self::ALL.into_iter().find(|k| k.name() == name)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AiError {
    pub kind: AiErrorKind,
    pub message: String,
}

impl AiError {
    pub fn new(kind: AiErrorKind, message: impl Into<String>) -> Self {
        AiError {
            kind,
            message: message.into(),
        }
    }
}

/// One `ai.infer<T>(...)` call, after the program's values have been
/// converted to JSON.
#[derive(Debug, Clone, PartialEq)]
pub struct AiRequest {
    /// Logical model name from `elomar.toml`, not a vendor model id.
    pub model: String,
    pub instructions: String,
    /// Treated as untrusted data, never as instructions.
    pub input: serde_json::Value,
    /// The shape the answer must have.
    pub schema: Schema,
    pub timeout_ms: u64,
}

impl AiRequest {
    /// The input as text: strings verbatim, anything else as JSON.
    pub fn input_text(&self) -> String {
        match &self.input {
            serde_json::Value::String(s) => s.clone(),
            other => other.to_string(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Capabilities {
    /// Can be asked to answer in a given JSON Schema.
    pub structured_output: bool,
    /// Sends data off the machine; such models need the `net` effect.
    pub uses_network: bool,
}

/// A model backend. Returns the model's raw text; validation against the
/// schema is done by [`AiRuntime`], identically for every provider.
pub trait ModelProvider: Send {
    fn provider_name(&self) -> &str;
    fn capabilities(&self) -> Capabilities;
    /// The host and port this provider connects to, if any. [`AiRuntime`]
    /// checks it against `[permissions] network_hosts` before every call.
    fn endpoint(&self) -> Option<(String, u16)> {
        None
    }
    fn infer(&mut self, request: &AiRequest) -> Result<String, AiError>;
}

/// Stands in for a provider named in `elomar.toml` that this build does not
/// implement, so the program gets a clear `ProviderUnavailable` error.
pub struct UnavailableProvider {
    pub provider: String,
}

impl ModelProvider for UnavailableProvider {
    fn provider_name(&self) -> &str {
        &self.provider
    }

    fn capabilities(&self) -> Capabilities {
        Capabilities {
            structured_output: false,
            uses_network: true,
        }
    }

    fn infer(&mut self, _: &AiRequest) -> Result<String, AiError> {
        Err(AiError::new(
            AiErrorKind::ProviderUnavailable,
            format!(
                "provider `{}` is not implemented in this version; only `mock` is available",
                self.provider
            ),
        ))
    }
}

pub const DEFAULT_TIMEOUT_MS: u64 = 30_000;

/// Registered models plus the run's policy and counters.
#[derive(Default)]
pub struct AiRuntime {
    models: HashMap<String, Box<dyn ModelProvider>>,
    policy: AiPolicy,
    permissions: Permissions,
    calls: u64,
}

impl AiRuntime {
    pub fn new(policy: AiPolicy) -> Self {
        AiRuntime {
            models: HashMap::new(),
            policy,
            permissions: Permissions::default(),
            calls: 0,
        }
    }

    /// Sets the network allowlist. Without one, no provider may connect.
    pub fn with_permissions(mut self, permissions: Permissions) -> Self {
        self.permissions = permissions;
        self
    }

    pub fn register(&mut self, model: impl Into<String>, provider: Box<dyn ModelProvider>) {
        self.models.insert(model.into(), provider);
    }

    /// The `[permissions]` of the run (for `std.http`).
    pub fn permissions(&self) -> &Permissions {
        &self.permissions
    }

    pub fn has_model(&self, model: &str) -> bool {
        self.models.contains_key(model)
    }

    /// Calls made so far in this run.
    pub fn calls(&self) -> u64 {
        self.calls
    }

    pub fn default_timeout_ms(&self) -> u64 {
        self.policy.default_timeout_ms.unwrap_or(DEFAULT_TIMEOUT_MS)
    }

    /// Runs one inference: budget → model lookup → network permission →
    /// provider → parse → validate.
    /// On success the JSON is guaranteed to match `request.schema`.
    pub fn infer(&mut self, request: &AiRequest) -> Result<serde_json::Value, AiError> {
        if let Some(max) = self.policy.max_calls_per_run
            && self.calls >= max
        {
            return Err(AiError::new(
                AiErrorKind::BudgetExceeded,
                format!("this run may make at most {max} AI call(s) (`max_calls_per_run`)"),
            ));
        }
        let Some(provider) = self.models.get_mut(&request.model) else {
            return Err(AiError::new(
                AiErrorKind::ProviderUnavailable,
                format!(
                    "model `{}` is not configured; add `[ai.models.\"{}\"]` to elomar.toml",
                    request.model, request.model
                ),
            ));
        };
        if let Some((host, port)) = provider.endpoint()
            && !self.permissions.allows_host(&host, port)
        {
            return Err(AiError::new(
                AiErrorKind::PolicyDenied,
                format!(
                    "network access to `{host}` is not permitted; add \"{host}\" to `network_hosts` under `[permissions]` in elomar.toml"
                ),
            ));
        }
        self.calls += 1;
        let text = provider.infer(request)?;
        parse_response(&text, &request.schema)
    }
}

/// Parses a model's text answer and checks it against `schema`. A `String`
/// schema also accepts plain (non-JSON) text.
pub fn parse_response(text: &str, schema: &Schema) -> Result<serde_json::Value, AiError> {
    let parsed = serde_json::from_str::<serde_json::Value>(text.trim());
    let value = match (parsed, schema) {
        (Ok(v @ serde_json::Value::String(_)), Schema::String) => v,
        (_, Schema::String) => serde_json::Value::String(text.to_string()),
        (Ok(v), _) => v,
        (Err(e), _) => {
            return Err(AiError::new(
                AiErrorKind::InvalidStructuredOutput,
                format!("the model's answer is not valid JSON: {e}"),
            ));
        }
    };
    schema.validate(&value).map_err(|problem| {
        AiError::new(
            AiErrorKind::InvalidStructuredOutput,
            format!(
                "the model's answer does not match `{}`: {problem}",
                schema.describe()
            ),
        )
    })?;
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Fixed(&'static str);

    impl ModelProvider for Fixed {
        fn provider_name(&self) -> &str {
            "fixed"
        }
        fn capabilities(&self) -> Capabilities {
            Capabilities {
                structured_output: true,
                uses_network: false,
            }
        }
        fn infer(&mut self, _: &AiRequest) -> Result<String, AiError> {
            Ok(self.0.to_string())
        }
    }

    fn request(model: &str, schema: Schema) -> AiRequest {
        AiRequest {
            model: model.into(),
            instructions: "x".into(),
            input: serde_json::json!("hello"),
            schema,
            timeout_ms: 1000,
        }
    }

    #[test]
    fn validates_answers() {
        let schema = Schema::Record(vec![
            ("name".into(), Schema::String),
            ("age".into(), Schema::Int),
        ]);
        let mut rt = AiRuntime::default();
        rt.register("good", Box::new(Fixed(r#"{"name": "Lina", "age": 30}"#)));
        rt.register(
            "bad",
            Box::new(Fixed(r#"{"name": "Lina", "age": "thirty"}"#)),
        );
        rt.register("broken", Box::new(Fixed("not json")));
        assert!(rt.infer(&request("good", schema.clone())).is_ok());
        let e = rt.infer(&request("bad", schema.clone())).unwrap_err();
        assert_eq!(e.kind, AiErrorKind::InvalidStructuredOutput);
        assert!(e.message.contains("$.age"), "{}", e.message);
        let e = rt.infer(&request("broken", schema)).unwrap_err();
        assert_eq!(e.kind, AiErrorKind::InvalidStructuredOutput);
    }

    #[test]
    fn plain_text_is_accepted_for_strings() {
        assert_eq!(
            parse_response("Hello there", &Schema::String).unwrap(),
            serde_json::json!("Hello there")
        );
        assert_eq!(
            parse_response("\"quoted\"", &Schema::String).unwrap(),
            serde_json::json!("quoted")
        );
    }

    #[test]
    fn unknown_models_and_budget() {
        let mut rt = AiRuntime::new(AiPolicy {
            max_calls_per_run: Some(1),
            default_timeout_ms: None,
        });
        assert_eq!(
            rt.infer(&request("nope", Schema::String)).unwrap_err().kind,
            AiErrorKind::ProviderUnavailable
        );
        rt.register("m", Box::new(Fixed("ok")));
        assert!(rt.infer(&request("m", Schema::String)).is_ok());
        assert_eq!(
            rt.infer(&request("m", Schema::String)).unwrap_err().kind,
            AiErrorKind::BudgetExceeded
        );
    }

    struct Remote;

    impl ModelProvider for Remote {
        fn provider_name(&self) -> &str {
            "remote"
        }
        fn capabilities(&self) -> Capabilities {
            Capabilities {
                structured_output: true,
                uses_network: true,
            }
        }
        fn endpoint(&self) -> Option<(String, u16)> {
            Some(("api.example.com".into(), 443))
        }
        fn infer(&mut self, _: &AiRequest) -> Result<String, AiError> {
            Ok("\"answer\"".into())
        }
    }

    #[test]
    fn network_access_is_denied_by_default() {
        let mut rt = AiRuntime::default();
        rt.register("r", Box::new(Remote));
        let e = rt.infer(&request("r", Schema::String)).unwrap_err();
        assert_eq!(e.kind, AiErrorKind::PolicyDenied);
        assert!(e.message.contains("api.example.com"));
        assert_eq!(rt.calls(), 0, "denied calls do not count");

        let mut rt = AiRuntime::default().with_permissions(Permissions {
            network_hosts: vec!["api.example.com".into()],
            ..Default::default()
        });
        rt.register("r", Box::new(Remote));
        assert_eq!(
            rt.infer(&request("r", Schema::String)).unwrap(),
            serde_json::json!("answer")
        );
    }

    #[test]
    fn error_kind_names_round_trip() {
        for k in AiErrorKind::ALL {
            assert_eq!(AiErrorKind::parse(k.name()), Some(k));
        }
    }
}
