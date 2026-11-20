//! An offline, deterministic model provider (spec §2.4 P0 "Mock").
//!
//! Rules from `elomar.toml` are tried in order; the first whose
//! `input_contains` matches (or that has none) decides the answer. If no
//! rule matches, the mock synthesizes a placeholder of the requested type,
//! so a program can run before any rules are written. Latency is simulated,
//! never slept, so tests stay fast.

use elomar_ai_core::{
    AiError, AiErrorKind, AiRequest, Capabilities, Message, MockRule, ModelProvider, Schema,
    ToolCall, ToolSpec, Turn,
};
use serde_json::{Map, Value, json};

pub struct MockProvider {
    rules: Vec<MockRule>,
}

impl MockProvider {
    /// Validates the rules (e.g. that `error` names a real `AIError` kind).
    pub fn new(rules: Vec<MockRule>) -> Result<Self, String> {
        for (i, rule) in rules.iter().enumerate() {
            if let Some(kind) = &rule.error
                && AiErrorKind::parse(kind).is_none()
            {
                let known: Vec<_> = AiErrorKind::ALL.iter().map(|k| k.name()).collect();
                return Err(format!(
                    "mock rule {}: unknown error kind `{kind}`; expected one of {}",
                    i + 1,
                    known.join(", ")
                ));
            }
            let kinds = [
                rule.error.is_some(),
                rule.response.is_some(),
                rule.tool.is_some(),
            ];
            if kinds.iter().filter(|k| **k).count() > 1 {
                return Err(format!(
                    "mock rule {}: set only one of `response`, `error` and `tool`",
                    i + 1
                ));
            }
            if rule.arguments.is_some() && rule.tool.is_none() {
                return Err(format!("mock rule {}: `arguments` needs `tool`", i + 1));
            }
        }
        Ok(MockProvider { rules })
    }
}

impl ModelProvider for MockProvider {
    fn provider_name(&self) -> &str {
        "mock"
    }

    fn capabilities(&self) -> Capabilities {
        Capabilities {
            structured_output: true,
            uses_network: false,
        }
    }

    fn infer(&mut self, request: &AiRequest) -> Result<String, AiError> {
        let input = request.input_text();
        let rule = self.rules.iter().find(|r| {
            r.tool.is_none()
                && r.input_contains
                    .as_ref()
                    .is_none_or(|needle| input.contains(needle.as_str()))
        });
        let Some(rule) = rule else {
            return Ok(placeholder(&request.schema).to_string());
        };
        answer(rule, request)
    }

    /// The rules whose `input_contains` matches form a script: step `k`
    /// (after `k` rounds of tool calls) uses the `k`-th of them. A `tool`
    /// rule asks for that call; other rules answer as in `infer`, where
    /// `{{last_tool_result}}` in a response is replaced by the latest
    /// result. Past the end of the script the answer is a placeholder.
    fn step(
        &mut self,
        request: &AiRequest,
        tools: &[ToolSpec],
        history: &[Message],
    ) -> Result<Turn, AiError> {
        let input = request.input_text();
        let k = history
            .iter()
            .filter(|m| matches!(m, Message::ToolCalls(_)))
            .count();
        let rule = self
            .rules
            .iter()
            .filter(|r| {
                r.input_contains
                    .as_ref()
                    .is_none_or(|needle| input.contains(needle.as_str()))
            })
            .nth(k);
        let Some(rule) = rule else {
            return Ok(Turn::Final(placeholder(&request.schema).to_string()));
        };
        if let Some(tool) = &rule.tool {
            if !tools.iter().any(|t| &t.name == tool) {
                return Err(AiError::new(
                    AiErrorKind::InvalidStructuredOutput,
                    format!("mock rule asks for tool `{tool}`, which this call does not offer"),
                ));
            }
            return Ok(Turn::ToolCalls(vec![ToolCall {
                id: format!("call_{k}"),
                name: tool.clone(),
                arguments: rule.arguments.clone().unwrap_or_else(|| json!({})),
            }]));
        }
        let text = answer(rule, request)?;
        let last = history.iter().rev().find_map(|m| match m {
            Message::ToolResult { content, .. } => Some(content.as_str()),
            _ => None,
        });
        Ok(Turn::Final(match last {
            Some(result) => text.replace("{{last_tool_result}}", result),
            None => text,
        }))
    }
}

/// The answer of a non-tool rule: its response, error or delay.
fn answer(rule: &MockRule, request: &AiRequest) -> Result<String, AiError> {
    if let Some(delay) = rule.delay_ms
        && delay > request.timeout_ms
    {
        return Err(AiError::new(
            AiErrorKind::Timeout,
            format!(
                "no answer within {} ms (mock latency {delay} ms)",
                request.timeout_ms
            ),
        ));
    }
    if let Some(kind) = &rule.error {
        let kind = AiErrorKind::parse(kind).expect("validated in new()");
        let message = rule
            .message
            .clone()
            .unwrap_or_else(|| format!("simulated {} error", kind.name()));
        return Err(AiError::new(kind, message));
    }
    Ok(match &rule.response {
        Some(text) => text.clone(),
        None => placeholder(&request.schema).to_string(),
    })
}

/// A value of the requested shape: `"mock"`, `0`, `0.0`, `false`, `[]`,
/// `null`, and records of those.
pub fn placeholder(schema: &Schema) -> Value {
    match schema {
        Schema::String => json!("mock"),
        Schema::Int => json!(0),
        Schema::Float => json!(0.0),
        Schema::Bool => json!(false),
        Schema::Enum { variants, .. } => json!(variants.first().cloned().unwrap_or_default()),
        Schema::List(_) => json!([]),
        Schema::Option(_) => Value::Null,
        Schema::Map(_) => json!({}),
        Schema::Record(fields) => Value::Object(
            fields
                .iter()
                .map(|(n, t)| (n.clone(), placeholder(t)))
                .collect::<Map<_, _>>(),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(input: &str, schema: Schema) -> AiRequest {
        AiRequest {
            model: "m".into(),
            instructions: "classify".into(),
            input: json!(input),
            schema,
            timeout_ms: 1_000,
        }
    }

    fn rule(contains: Option<&str>, response: Option<&str>, error: Option<&str>) -> MockRule {
        MockRule {
            input_contains: contains.map(Into::into),
            response: response.map(Into::into),
            error: error.map(Into::into),
            ..Default::default()
        }
    }

    #[test]
    fn first_matching_rule_wins() {
        let mut m = MockProvider::new(vec![
            rule(Some("password"), Some("\"account\""), None),
            rule(None, Some("\"other\""), None),
        ])
        .unwrap();
        assert_eq!(
            m.infer(&request("lost my password", Schema::String))
                .unwrap(),
            "\"account\""
        );
        assert_eq!(
            m.infer(&request("hello", Schema::String)).unwrap(),
            "\"other\""
        );
    }

    #[test]
    fn simulated_errors_and_timeouts() {
        let mut m = MockProvider::new(vec![
            rule(Some("slow"), Some("1"), None),
            rule(Some("limit"), None, Some("RateLimited")),
        ])
        .unwrap();
        m.rules[0].delay_ms = Some(5_000);
        assert_eq!(
            m.infer(&request("slow please", Schema::Int))
                .unwrap_err()
                .kind,
            AiErrorKind::Timeout
        );
        assert_eq!(
            m.infer(&request("rate limit", Schema::Int))
                .unwrap_err()
                .kind,
            AiErrorKind::RateLimited
        );
    }

    #[test]
    fn placeholder_matches_schema() {
        let schema = Schema::Record(vec![
            ("name".into(), Schema::String),
            ("tags".into(), Schema::List(Box::new(Schema::String))),
            ("score".into(), Schema::Option(Box::new(Schema::Float))),
        ]);
        let mut m = MockProvider::new(Vec::new()).unwrap();
        let text = m.infer(&request("x", schema.clone())).unwrap();
        let value: Value = serde_json::from_str(&text).unwrap();
        assert!(schema.validate(&value).is_ok());
    }

    #[test]
    fn rejects_bad_rules() {
        assert!(MockProvider::new(vec![rule(None, None, Some("Oops"))]).is_err());
        assert!(MockProvider::new(vec![rule(None, Some("1"), Some("Timeout"))]).is_err());
    }
}

#[cfg(test)]
mod tool_tests {
    use super::*;
    use elomar_ai_core::Message;

    #[test]
    fn scripted_tool_calls() {
        let rules = vec![
            MockRule {
                tool: Some("lookup".into()),
                arguments: Some(json!({ "name": "Lina" })),
                ..Default::default()
            },
            MockRule {
                response: Some("\"got {{last_tool_result}}\"".into()),
                ..Default::default()
            },
        ];
        let mut m = MockProvider::new(rules).unwrap();
        let request = AiRequest {
            model: "m".into(),
            instructions: "i".into(),
            input: json!("q"),
            schema: Schema::String,
            timeout_ms: 1_000,
        };
        let tools = [ToolSpec {
            name: "lookup".into(),
            params: Schema::Record(vec![("name".into(), Schema::String)]),
            description: String::new(),
            raw_schema: None,
        }];
        let Turn::ToolCalls(calls) = m.step(&request, &tools, &[]).unwrap() else {
            panic!("expected a tool call")
        };
        assert_eq!(calls[0].arguments, json!({ "name": "Lina" }));
        let history = [
            Message::ToolCalls(calls),
            Message::ToolResult {
                id: "call_0".into(),
                content: "91".into(),
            },
        ];
        assert_eq!(
            m.step(&request, &tools, &history).unwrap(),
            Turn::Final("\"got 91\"".into())
        );
        // Tool rules are not answers for `ai.infer`.
        assert_eq!(m.infer(&request).unwrap(), "\"got {{last_tool_result}}\"");
        // A tool the call does not offer is an error.
        assert!(m.step(&request, &[], &[]).is_err());
        // Rules are validated.
        assert!(
            MockProvider::new(vec![MockRule {
                tool: Some("t".into()),
                response: Some("x".into()),
                ..Default::default()
            }])
            .is_err()
        );
    }
}
