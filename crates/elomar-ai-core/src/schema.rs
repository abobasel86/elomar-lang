//! The shapes an AI answer can have, derived from an Elomar type (spec §13.3).

use serde_json::{Map, Value, json};

/// A type that can cross the boundary to a model as JSON.
#[derive(Debug, Clone, PartialEq)]
pub enum Schema {
    String,
    Int,
    Float,
    Bool,
    List(Box<Schema>),
    /// May be `null` (or absent, as a record field).
    Option(Box<Schema>),
    /// Every field is required unless it is an `Option`; no extra fields.
    Record(Vec<(String, Schema)>),
}

impl Schema {
    /// The Elomar spelling of the type, for messages.
    pub fn describe(&self) -> String {
        match self {
            Schema::String => "String".into(),
            Schema::Int => "Int".into(),
            Schema::Float => "Float".into(),
            Schema::Bool => "Bool".into(),
            Schema::List(t) => format!("List<{}>", t.describe()),
            Schema::Option(t) => format!("Option<{}>", t.describe()),
            Schema::Record(fields) => {
                let inner: Vec<String> = fields
                    .iter()
                    .map(|(n, t)| format!("{n}: {}", t.describe()))
                    .collect();
                format!("{{ {} }}", inner.join(", "))
            }
        }
    }

    /// A JSON Schema (draft 2020-12 subset) to send to providers that
    /// support structured output.
    pub fn json_schema(&self) -> Value {
        match self {
            Schema::String => json!({ "type": "string" }),
            Schema::Int => json!({ "type": "integer" }),
            Schema::Float => json!({ "type": "number" }),
            Schema::Bool => json!({ "type": "boolean" }),
            Schema::List(t) => json!({ "type": "array", "items": t.json_schema() }),
            Schema::Option(t) => json!({ "anyOf": [t.json_schema(), { "type": "null" }] }),
            Schema::Record(fields) => {
                let mut props = Map::new();
                let mut required = Vec::new();
                for (name, t) in fields {
                    props.insert(name.clone(), t.json_schema());
                    if !matches!(t, Schema::Option(_)) {
                        required.push(Value::String(name.clone()));
                    }
                }
                json!({
                    "type": "object",
                    "properties": props,
                    "required": required,
                    "additionalProperties": false,
                })
            }
        }
    }

    /// The strict form required by OpenAI structured outputs: every record
    /// field is listed in `required`, and an optional field is a union with
    /// `null` instead of being omittable. [`Schema::validate`] accepts both.
    pub fn json_schema_strict(&self) -> Value {
        match self {
            Schema::List(t) => json!({ "type": "array", "items": t.json_schema_strict() }),
            Schema::Option(t) => json!({ "anyOf": [t.json_schema_strict(), { "type": "null" }] }),
            Schema::Record(fields) => {
                let props: Map<String, Value> = fields
                    .iter()
                    .map(|(n, t)| (n.clone(), t.json_schema_strict()))
                    .collect();
                let required: Vec<Value> = fields
                    .iter()
                    .map(|(n, _)| Value::String(n.clone()))
                    .collect();
                json!({
                    "type": "object",
                    "properties": props,
                    "required": required,
                    "additionalProperties": false,
                })
            }
            scalar => scalar.json_schema(),
        }
    }

    /// Checks `value`; the error names the first offending path, e.g.
    /// `$.items[2].price: expected Float, found "cheap"`.
    pub fn validate(&self, value: &Value) -> Result<(), String> {
        self.validate_at(value, "$")
    }

    fn validate_at(&self, value: &Value, path: &str) -> Result<(), String> {
        let ok = match (self, value) {
            (Schema::String, Value::String(_)) => true,
            (Schema::Int, Value::Number(n)) => {
                n.is_i64()
                    || n.as_f64()
                        .is_some_and(|f| f.fract() == 0.0 && f.abs() < 9.0e15)
            }
            (Schema::Float, Value::Number(_)) => true,
            (Schema::Bool, Value::Bool(_)) => true,
            (Schema::Option(_), Value::Null) => true,
            (Schema::Option(t), v) => return t.validate_at(v, path),
            (Schema::List(t), Value::Array(items)) => {
                for (i, item) in items.iter().enumerate() {
                    t.validate_at(item, &format!("{path}[{i}]"))?;
                }
                true
            }
            (Schema::Record(fields), Value::Object(obj)) => {
                for (name, t) in fields {
                    let field_path = format!("{path}.{name}");
                    match obj.get(name) {
                        Some(v) => t.validate_at(v, &field_path)?,
                        None if matches!(t, Schema::Option(_)) => {}
                        None => return Err(format!("{field_path}: missing")),
                    }
                }
                if let Some(extra) = obj.keys().find(|k| !fields.iter().any(|(n, _)| n == *k)) {
                    return Err(format!("{path}.{extra}: unexpected field"));
                }
                true
            }
            _ => false,
        };
        if ok {
            Ok(())
        } else {
            Err(format!(
                "{path}: expected {}, found {value}",
                self.describe()
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ticket() -> Schema {
        Schema::Record(vec![
            ("category".into(), Schema::String),
            ("tags".into(), Schema::List(Box::new(Schema::String))),
            ("score".into(), Schema::Option(Box::new(Schema::Float))),
        ])
    }

    #[test]
    fn json_schema_marks_optional_fields() {
        let s = ticket().json_schema();
        assert_eq!(s["required"], json!(["category", "tags"]));
        assert_eq!(s["additionalProperties"], json!(false));
        assert_eq!(s["properties"]["tags"]["items"]["type"], json!("string"));
    }

    #[test]
    fn strict_schema_requires_every_field() {
        let s = ticket().json_schema_strict();
        assert_eq!(s["required"], json!(["category", "tags", "score"]));
        assert_eq!(
            s["properties"]["score"]["anyOf"][1],
            json!({ "type": "null" })
        );
    }

    #[test]
    fn validation_paths() {
        let t = ticket();
        assert!(t.validate(&json!({"category": "a", "tags": []})).is_ok());
        assert!(
            t.validate(&json!({"category": "a", "tags": [], "score": null}))
                .is_ok()
        );
        assert_eq!(
            t.validate(&json!({"category": "a", "tags": ["x", 3]}))
                .unwrap_err(),
            "$.tags[1]: expected String, found 3"
        );
        assert_eq!(
            t.validate(&json!({"tags": []})).unwrap_err(),
            "$.category: missing"
        );
        assert_eq!(
            t.validate(&json!({"category": "a", "tags": [], "x": 1}))
                .unwrap_err(),
            "$.x: unexpected field"
        );
        assert!(Schema::Int.validate(&json!(3.0)).is_ok());
        assert!(Schema::Int.validate(&json!(3.5)).is_err());
    }
}
