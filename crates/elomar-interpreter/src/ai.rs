//! Running `ai.infer<T>(...)`: converting values to and from JSON and
//! turning provider failures into `Err(AIError)` values (spec §13.3).

use std::collections::HashMap;
use std::rc::Rc;

use elomar_ai_core::{AiError, AiRequest, AiRuntime, Schema};
use elomar_ast::{TypeExpr, TypeExprKind};
use serde_json::{Map, Number, Value as Json};

use crate::Value;

/// A pending `ai.infer` call, created when the call is evaluated and run
/// when it is awaited.
#[derive(Debug, Clone)]
pub struct InferCall {
    pub schema: Schema,
    pub model: String,
    pub instructions: String,
    pub input: Value,
    pub timeout_ms: Option<i64>,
}

/// One tool of an `ai.run` call.
#[derive(Debug, Clone)]
pub struct RunTool {
    /// The name the model sees (the function's own name).
    pub name: String,
    /// The function to call (with its module prefix).
    pub fn_name: String,
    /// Parameters in declaration order.
    pub params: Vec<(String, Schema)>,
    /// `None` for a function that returns nothing.
    pub ret: Option<Schema>,
}

/// A pending `ai.run` call.
#[derive(Debug, Clone)]
pub struct RunCall {
    pub infer: InferCall,
    pub tools: Vec<RunTool>,
    pub max_steps: i64,
}

impl InferCall {
    pub(crate) fn request(&self, ai: &AiRuntime) -> AiRequest {
        AiRequest {
            model: self.model.clone(),
            instructions: self.instructions.clone(),
            input: to_json(&self.input),
            schema: self.schema.clone(),
            timeout_ms: self
                .timeout_ms
                .map_or_else(|| ai.default_timeout_ms(), |ms| ms.max(0) as u64),
        }
    }
}

/// The checker has already expanded aliases, so this is purely structural;
/// a remaining name is an enum of plain variants.
pub(crate) fn schema_of(t: &TypeExpr, unit_enums: &HashMap<String, Vec<String>>) -> Schema {
    match &t.kind {
        TypeExprKind::Fn { .. } => unreachable!("checker rejects functions in AI answers"),
        TypeExprKind::Record(fields) => Schema::Record(
            fields
                .iter()
                .map(|(n, t)| (n.name.clone(), schema_of(t, unit_enums)))
                .collect(),
        ),
        TypeExprKind::Named { name, args } => match name.name.as_str() {
            "String" => Schema::String,
            "Int" => Schema::Int,
            "Float" => Schema::Float,
            "Bool" => Schema::Bool,
            "List" => Schema::List(Box::new(schema_of(&args[0], unit_enums))),
            "Option" => Schema::Option(Box::new(schema_of(&args[0], unit_enums))),
            // Keys are always strings here (checked).
            "Map" => Schema::Map(Box::new(schema_of(&args[1], unit_enums))),
            other => match unit_enums.get(other) {
                Some(variants) => Schema::Enum {
                    name: other.to_string(),
                    variants: variants.clone(),
                },
                None => unreachable!("checker allows only JSON data types, got `{other}`"),
            },
        },
    }
}

/// Performs the call. Never fails at the interpreter level: every problem
/// becomes an `Err(AIError)` value the program can `match` on.
pub(crate) fn run_infer(ai: &mut AiRuntime, call: &InferCall) -> Value {
    let request = call.request(ai);
    match ai.infer(&request) {
        Ok(json) => Value::Result(Ok(Rc::new(from_json(&call.schema, &json)))),
        Err(e) => Value::Result(Err(Rc::new(ai_error_value(&e)))),
    }
}

/// `AIError` as a value: the variant named by the error kind, carrying the
/// message, e.g. `AIError.Timeout("no answer within 10000 ms")`.
pub(crate) fn ai_error_value(e: &AiError) -> Value {
    Value::Enum {
        enum_name: "AIError".into(),
        variant: e.kind.name().into(),
        fields: Rc::new(vec![Value::str(e.message.as_str())]),
    }
}

pub(crate) fn to_json(v: &Value) -> Json {
    match v {
        Value::Int(n) => Json::from(*n),
        Value::Float(x) => Number::from_f64(*x).map_or(Json::Null, Json::Number),
        Value::Bool(b) => Json::Bool(*b),
        Value::Str(s) => Json::String(s.to_string()),
        Value::List(items) => Json::Array(items.iter().map(to_json).collect()),
        Value::Record(fields) => Json::Object(
            fields
                .iter()
                .map(|(n, v)| (n.to_string(), to_json(v)))
                .collect::<Map<_, _>>(),
        ),
        Value::Enum { variant, .. } => Json::String(variant.to_string()),
        Value::Map(entries) => Json::Object(
            entries
                .iter()
                .map(|(k, v)| {
                    let key = match k {
                        crate::MapKey::Str(s) => s.to_string(),
                        crate::MapKey::Int(n) => n.to_string(),
                    };
                    (key, to_json(v))
                })
                .collect::<Map<_, _>>(),
        ),
        Value::Option(None) => Json::Null,
        Value::Option(Some(v)) => to_json(v),
        // The checker only lets JSON data types reach a model.
        _ => Json::Null,
    }
}

/// Converts JSON that has passed `schema.validate`.
pub(crate) fn from_json(schema: &Schema, json: &Json) -> Value {
    match (schema, json) {
        (Schema::String, Json::String(s)) => Value::str(s.as_str()),
        (Schema::Int, Json::Number(n)) => Value::Int(
            n.as_i64()
                .unwrap_or_else(|| n.as_f64().unwrap_or(0.0) as i64),
        ),
        (Schema::Float, Json::Number(n)) => Value::Float(n.as_f64().unwrap_or(0.0)),
        (Schema::Bool, Json::Bool(b)) => Value::Bool(*b),
        (Schema::Enum { name, .. }, Json::String(s)) => Value::Enum {
            enum_name: name.as_str().into(),
            variant: s.as_str().into(),
            fields: Rc::new(Vec::new()),
        },
        (Schema::List(t), Json::Array(items)) => {
            Value::list(items.iter().map(|i| from_json(t, i)).collect())
        }
        (Schema::Map(t), Json::Object(obj)) => Value::Map(Rc::new(
            obj.iter()
                .map(|(k, v)| (crate::MapKey::Str(k.as_str().into()), from_json(t, v)))
                .collect(),
        )),
        (Schema::Option(_), Json::Null) => Value::none(),
        (Schema::Option(t), v) => Value::some(from_json(t, v)),
        (Schema::Record(fields), Json::Object(obj)) => Value::Record(Rc::new(
            fields
                .iter()
                .map(|(name, t)| {
                    let v = obj.get(name).map_or(Value::none(), |j| from_json(t, j));
                    (Rc::from(name.as_str()), v)
                })
                .collect(),
        )),
        _ => unreachable!("response was validated against {}", schema.describe()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn round_trips_records() {
        let schema = Schema::Record(vec![
            ("name".into(), Schema::String),
            ("score".into(), Schema::Option(Box::new(Schema::Float))),
            ("tags".into(), Schema::List(Box::new(Schema::Int))),
        ]);
        let v = from_json(&schema, &json!({"name": "Lina", "tags": [1, 2]}));
        assert_eq!(
            v.to_string(),
            "{ name: \"Lina\", score: None, tags: [1, 2] }"
        );
        assert_eq!(
            to_json(&v),
            json!({"name": "Lina", "score": null, "tags": [1, 2]})
        );
    }
}
