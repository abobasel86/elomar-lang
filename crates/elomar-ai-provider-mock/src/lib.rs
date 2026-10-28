//! An offline, deterministic model provider (spec §2.4 P0 "Mock").
//!
//! Rules from `elomar.toml` are tried in order; the first whose
//! `input_contains` matches (or that has none) decides the answer. If no
//! rule matches, the mock synthesizes a placeholder of the requested type,
//! so a program can run before any rules are written. Latency is simulated,
//! never slept, so tests stay fast.

use elomar_ai_core::{
    AiError, AiErrorKind, AiRequest, Capabilities, MockRule, ModelProvider, Schema,
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
            if rule.error.is_some() && rule.response.is_some() {
                return Err(format!(
                    "mock rule {}: set either `response` or `error`, not both",
                    i + 1
                ));
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
            r.input_contains
                .as_ref()
                .is_none_or(|needle| input.contains(needle.as_str()))
        });
        let Some(rule) = rule else {
            return Ok(placeholder(&request.schema).to_string());
        };
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
