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
    AiConfig, AiPolicy, McpConfig, McpServerConfig, MockRule, ModelConfig, Permissions,
    ProjectConfig, parse_config,
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

/// A function the model may ask to call during `ai.run`.
#[derive(Debug, Clone, PartialEq)]
pub struct ToolSpec {
    pub name: String,
    /// Always a record: the tool's parameters by name.
    pub params: Schema,
    /// Shown to the model; may be empty.
    pub description: String,
    /// A JSON Schema written by someone else (an MCP server), used instead
    /// of `params`. Such tools are not strict: the server checks the
    /// arguments.
    pub raw_schema: Option<serde_json::Value>,
}

/// The model's request to call a tool.
#[derive(Debug, Clone, PartialEq)]
pub struct ToolCall {
    /// Pairs the call with its result.
    pub id: String,
    pub name: String,
    pub arguments: serde_json::Value,
}

/// Earlier turns of an `ai.run` conversation, after the first request.
#[derive(Debug, Clone, PartialEq)]
pub enum Message {
    /// The model asked for these calls.
    ToolCalls(Vec<ToolCall>),
    /// What a call returned, as JSON text.
    ToolResult { id: String, content: String },
}

/// One answer of the model in a tool conversation.
#[derive(Debug, Clone, PartialEq)]
pub enum Turn {
    /// The final answer, as raw text (validated by [`AiRuntime`]).
    Final(String),
    ToolCalls(Vec<ToolCall>),
}

/// One step of `ai.run`, after validation.
#[derive(Debug, Clone, PartialEq)]
pub enum Step {
    /// The answer, guaranteed to match the request's schema.
    Final(serde_json::Value),
    ToolCalls(Vec<ToolCall>),
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
pub trait ModelProvider: Send + Sync {
    fn provider_name(&self) -> &str;
    fn capabilities(&self) -> Capabilities;
    /// The host and port this provider connects to, if any. [`AiRuntime`]
    /// checks it against `[permissions] network_hosts` before every call.
    fn endpoint(&self) -> Option<(String, u16)> {
        None
    }
    fn infer(&mut self, request: &AiRequest) -> Result<String, AiError>;

    /// Like [`ModelProvider::infer`] through a shared reference, so several
    /// requests can run at once (`await all([...])`). `None` when the
    /// provider can only answer one request at a time; then each request
    /// goes through `infer` in turn.
    fn infer_shared(&self, _request: &AiRequest) -> Option<Result<String, AiError>> {
        None
    }

    /// The answer as text, given piece by piece to `on_text` as it arrives
    /// (`ai.stream`); returns the whole text. `on_text` returns `false` to
    /// stop, and the call then fails with `Cancelled`. By default the
    /// answer comes in one piece.
    fn stream(
        &mut self,
        request: &AiRequest,
        on_text: &mut dyn FnMut(&str) -> bool,
    ) -> Result<String, AiError> {
        let text = self.infer(request)?;
        let text = match serde_json::from_str::<serde_json::Value>(&text) {
            Ok(serde_json::Value::String(s)) => s,
            _ => text,
        };
        if !on_text(&text) {
            return Err(cancelled());
        }
        Ok(text)
    }

    /// The embedding of `request.input_text()`: a vector of numbers whose
    /// direction stands for the text's meaning (`ai.embed`).
    fn embed(&mut self, _request: &AiRequest) -> Result<Vec<f64>, AiError> {
        Err(AiError::new(
            AiErrorKind::UnsupportedCapability,
            format!(
                "provider `{}` does not make embeddings; use an `openai` or `mock` model for `ai.embed`",
                self.provider_name()
            ),
        ))
    }

    /// The embeddings of several texts, in order (`await all` over
    /// `ai.embed` calls to one model). By default one request each; a
    /// provider that can embed many texts in one request does.
    fn embed_batch(&mut self, requests: &[&AiRequest]) -> Vec<Result<Vec<f64>, AiError>> {
        requests.iter().map(|r| self.embed(r)).collect()
    }

    /// One step of a conversation with tools: the model either answers or
    /// asks for tool calls. `history` holds the earlier calls and results.
    fn step(
        &mut self,
        _request: &AiRequest,
        _tools: &[ToolSpec],
        _history: &[Message],
    ) -> Result<Turn, AiError> {
        Err(AiError::new(
            AiErrorKind::UnsupportedCapability,
            format!("provider `{}` does not support tools", self.provider_name()),
        ))
    }
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

/// An embedding, unless it is empty or not made of finite numbers.
fn checked_embedding(v: Result<Vec<f64>, AiError>) -> Result<Vec<f64>, AiError> {
    let v = v?;
    if v.is_empty() || v.iter().any(|x| !x.is_finite()) {
        return Err(AiError::new(
            AiErrorKind::InvalidStructuredOutput,
            "the provider returned an empty or invalid embedding",
        ));
    }
    Ok(v)
}

/// The error of a stream the program stopped.
pub fn cancelled() -> AiError {
    AiError::new(AiErrorKind::Cancelled, "the stream was stopped")
}

/// Reads server-sent events, calling `on_data` with each `data:` field
/// until it returns `Ok(false)`, an error, or the stream ends.
pub fn read_sse(
    reader: impl std::io::BufRead,
    mut on_data: impl FnMut(&str) -> Result<bool, AiError>,
) -> Result<(), AiError> {
    for line in reader.lines() {
        let line = line.map_err(|e| AiError::new(AiErrorKind::Network, e.to_string()))?;
        if let Some(data) = line.strip_prefix("data:")
            && !on_data(data.trim_start())?
        {
            break;
        }
    }
    Ok(())
}

/// Registered models plus the run's policy and counters.
#[derive(Default)]
pub struct AiRuntime {
    models: HashMap<String, Box<dyn ModelProvider>>,
    policy: AiPolicy,
    permissions: Permissions,
    mcp: McpConfig,
    calls: u64,
}

impl AiRuntime {
    pub fn new(policy: AiPolicy) -> Self {
        AiRuntime {
            models: HashMap::new(),
            policy,
            permissions: Permissions::default(),
            mcp: McpConfig::default(),
            calls: 0,
        }
    }

    /// Sets the network allowlist. Without one, no provider may connect.
    pub fn with_permissions(mut self, permissions: Permissions) -> Self {
        self.permissions = permissions;
        self
    }

    /// Sets the MCP servers the program may start.
    pub fn with_mcp(mut self, mcp: McpConfig) -> Self {
        self.mcp = mcp;
        self
    }

    /// `[mcp.servers]` of the run.
    pub fn mcp(&self) -> &McpConfig {
        &self.mcp
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

    /// One step of `ai.run`; checked like [`AiRuntime::infer`], and every
    /// step counts as a call against `max_calls_per_run`.
    pub fn step(
        &mut self,
        request: &AiRequest,
        tools: &[ToolSpec],
        history: &[Message],
    ) -> Result<Step, AiError> {
        let provider = self.checked_provider(request)?;
        match provider.step(request, tools, history)? {
            Turn::Final(text) => parse_response(&text, &request.schema).map(Step::Final),
            Turn::ToolCalls(calls) => Ok(Step::ToolCalls(calls)),
        }
    }

    /// Budget → model lookup → network permission; counts the call.
    fn checked_provider(
        &mut self,
        request: &AiRequest,
    ) -> Result<&mut Box<dyn ModelProvider>, AiError> {
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
        Ok(provider)
    }

    /// Runs several inferences, as for `await all([...])`. Each request is
    /// checked first, in order (budget, model, network permission); then the
    /// requests whose providers support it run at the same time, each on its
    /// own thread, and the rest one after another. Results keep the order of
    /// `requests`.
    pub fn infer_all(&mut self, requests: &[AiRequest]) -> Vec<Result<serde_json::Value, AiError>> {
        let mut texts: Vec<Option<Result<String, AiError>>> =
            (0..requests.len()).map(|_| None).collect();
        for (i, request) in requests.iter().enumerate() {
            if let Err(e) = self.checked_provider(request) {
                texts[i] = Some(Err(e));
            }
        }
        // WebAssembly (the browser playground) has no threads: there every
        // request goes through `infer` below, one after another.
        #[cfg(not(target_family = "wasm"))]
        let models = &self.models;
        #[cfg(not(target_family = "wasm"))]
        std::thread::scope(|scope| {
            let handles: Vec<_> = requests
                .iter()
                .enumerate()
                .filter(|(i, _)| texts[*i].is_none())
                .map(|(i, request)| {
                    let provider = &models[&request.model];
                    (i, scope.spawn(move || provider.infer_shared(request)))
                })
                .collect();
            for (i, handle) in handles {
                texts[i] = handle.join().unwrap_or_else(|_| {
                    Some(Err(AiError::new(
                        AiErrorKind::ProviderUnavailable,
                        "the request stopped unexpectedly",
                    )))
                });
            }
        });
        requests
            .iter()
            .zip(texts)
            .map(|(request, text)| {
                let text = match text {
                    Some(t) => t,
                    None => match self.models.get_mut(&request.model) {
                        Some(provider) => provider.infer(request),
                        None => unreachable!("checked model"),
                    },
                };
                parse_response(&text?, &request.schema)
            })
            .collect()
    }

    /// Takes out the provider of `request.model` after the checks of
    /// [`AiRuntime::infer`], so it can stream on another thread while the
    /// program handles the pieces; give it back with [`AiRuntime::put_back`].
    pub fn take_checked(&mut self, request: &AiRequest) -> Result<Box<dyn ModelProvider>, AiError> {
        self.checked_provider(request)?;
        Ok(self
            .models
            .remove(&request.model)
            .expect("checked_provider found it"))
    }

    pub fn put_back(&mut self, model: String, provider: Box<dyn ModelProvider>) {
        self.models.insert(model, provider);
    }

    /// The embedding of `request.input_text()`, after the same checks as
    /// [`AiRuntime::infer`] (budget, model, network permission).
    pub fn embed(&mut self, request: &AiRequest) -> Result<Vec<f64>, AiError> {
        let provider = self.checked_provider(request)?;
        checked_embedding(provider.embed(request))
    }

    /// Several embeddings, as for `await all([...])` over `ai.embed`: each
    /// request is checked first, in order, then the requests to one model
    /// go to it together, so a provider can answer them in one request.
    /// Results keep the order of `requests`.
    pub fn embed_all(&mut self, requests: &[AiRequest]) -> Vec<Result<Vec<f64>, AiError>> {
        let mut out: Vec<Option<Result<Vec<f64>, AiError>>> =
            (0..requests.len()).map(|_| None).collect();
        let mut groups: Vec<(String, Vec<usize>)> = Vec::new();
        for (i, request) in requests.iter().enumerate() {
            if let Err(e) = self.checked_provider(request) {
                out[i] = Some(Err(e));
                continue;
            }
            match groups.iter_mut().find(|(m, _)| *m == request.model) {
                Some((_, idx)) => idx.push(i),
                None => groups.push((request.model.clone(), vec![i])),
            }
        }
        for (model, idx) in groups {
            let provider = self.models.get_mut(&model).expect("checked model");
            let batch: Vec<&AiRequest> = idx.iter().map(|&i| &requests[i]).collect();
            let mut answers = provider.embed_batch(&batch).into_iter();
            for i in idx {
                out[i] = Some(checked_embedding(answers.next().unwrap_or_else(|| {
                    Err(AiError::new(
                        AiErrorKind::InvalidStructuredOutput,
                        "the provider returned fewer embeddings than texts",
                    ))
                })));
            }
        }
        out.into_iter()
            .map(|r| r.expect("every request answered"))
            .collect()
    }

    /// Runs one inference: budget → model lookup → network permission →
    /// provider → parse → validate.
    /// On success the JSON is guaranteed to match `request.schema`.
    pub fn infer(&mut self, request: &AiRequest) -> Result<serde_json::Value, AiError> {
        let provider = self.checked_provider(request)?;
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

    /// A provider that takes 300 ms per request and can serve several at once.
    struct Slow;

    impl ModelProvider for Slow {
        fn provider_name(&self) -> &str {
            "slow"
        }
        fn capabilities(&self) -> Capabilities {
            Capabilities {
                structured_output: true,
                uses_network: false,
            }
        }
        fn infer(&mut self, request: &AiRequest) -> Result<String, AiError> {
            self.infer_shared(request).expect("shared")
        }
        fn infer_shared(&self, request: &AiRequest) -> Option<Result<String, AiError>> {
            std::thread::sleep(std::time::Duration::from_millis(300));
            Some(Ok(format!("\"{}\"", request.input_text())))
        }
    }

    #[test]
    fn infer_all_runs_requests_together_and_keeps_order() {
        let mut rt = AiRuntime::new(AiPolicy {
            max_calls_per_run: Some(4),
            ..Default::default()
        });
        rt.register("slow", Box::new(Slow));
        let req = |input: &str, model: &str| AiRequest {
            model: model.into(),
            instructions: String::new(),
            input: serde_json::json!(input),
            schema: Schema::String,
            timeout_ms: 5_000,
        };
        let requests = vec![
            req("a", "slow"),
            req("b", "missing"),
            req("c", "slow"),
            req("d", "slow"),
            req("e", "slow"),
            req("f", "slow"),
        ];
        let start = std::time::Instant::now();
        let results = rt.infer_all(&requests);
        let took = start.elapsed();
        assert!(
            took.as_millis() < 900,
            "took {took:?}; requests did not overlap"
        );
        assert_eq!(results[0].as_ref().unwrap(), "a");
        assert_eq!(
            results[1].as_ref().unwrap_err().kind,
            AiErrorKind::ProviderUnavailable
        );
        assert_eq!(results[4].as_ref().unwrap(), "e");
        // The budget of 4 calls covers a, c, d and e; f is refused.
        assert_eq!(
            results[5].as_ref().unwrap_err().kind,
            AiErrorKind::BudgetExceeded
        );
    }
}
