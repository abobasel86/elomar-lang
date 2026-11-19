//! `std.db` at run time: SQLite databases.
//!
//! A database file must be allowed by `[permissions] filesystem_write`
//! (SQLite also writes a journal next to it); `":memory:"` is always
//! allowed. Query parameters are a record whose fields bind the named
//! placeholders (`{ min: 60 }` binds `:min`). Rows become records whose
//! fields are matched to columns by name. Every failure is an
//! `Err(DbError...)` value.

use std::collections::HashMap;
use std::fmt;
use std::rc::Rc;

use elomar_ai_core::Schema;
use elomar_ast::TypeExpr;
use elomar_policy::{Access, FsPolicy};
use rusqlite::types::{Value as SqlValue, ValueRef};

use crate::Value;
use crate::ai::schema_of;

/// An open database: the value of type `Db`. Copies share the connection.
pub struct Handle {
    pub conn: rusqlite::Connection,
    pub path: String,
}

impl fmt::Debug for Handle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Db({})", self.path)
    }
}

fn error(kind: &str, message: String) -> Value {
    Value::Result(Err(Rc::new(Value::Enum {
        enum_name: "DbError".into(),
        variant: kind.into(),
        fields: Rc::new(vec![Value::str(message)]),
    })))
}

fn ok(v: Value) -> Value {
    Value::Result(Ok(Rc::new(v)))
}

fn sql_error(e: &rusqlite::Error) -> Value {
    let kind = match e.sqlite_error_code() {
        Some(rusqlite::ErrorCode::ConstraintViolation) => "Constraint",
        _ => "Sql",
    };
    error(kind, e.to_string())
}

fn text(v: &Value) -> &str {
    match v {
        Value::Str(s) => s,
        _ => "",
    }
}

fn to_sql(v: &Value) -> SqlValue {
    match v {
        Value::Int(n) => SqlValue::Integer(*n),
        Value::Float(x) => SqlValue::Real(*x),
        Value::Bool(b) => SqlValue::Integer(i64::from(*b)),
        Value::Str(s) => SqlValue::Text(s.to_string()),
        Value::Option(Some(inner)) => to_sql(inner),
        _ => SqlValue::Null,
    }
}

pub(crate) fn call(
    policy: &FsPolicy,
    func: &str,
    type_args: &[TypeExpr],
    args: &[Value],
    unit_enums: &HashMap<String, Vec<String>>,
) -> Value {
    if func == "open" {
        return open(policy, text(&args[0]));
    }
    let Value::Db(db) = &args[0] else {
        unreachable!("checked Db argument");
    };
    let sql = text(&args[1]);
    let params = args.get(2);
    match func {
        "execute" => execute(&db.conn, sql, params),
        "query" | "query_one" => {
            let schema = schema_of(&type_args[0], unit_enums);
            match query(&db.conn, sql, params, &schema) {
                Ok(rows) if func == "query" => ok(Value::list(rows)),
                Ok(rows) => ok(rows
                    .into_iter()
                    .next()
                    .map_or_else(Value::none, Value::some)),
                Err(e) => e,
            }
        }
        other => unreachable!("checked db function `{other}`"),
    }
}

fn open(policy: &FsPolicy, path: &str) -> Value {
    let conn = if path == ":memory:" {
        rusqlite::Connection::open_in_memory()
    } else {
        let resolved = match policy.check(path, Access::Write) {
            Ok(p) => p,
            Err(message) => return error("PolicyDenied", message),
        };
        rusqlite::Connection::open(resolved)
    };
    match conn {
        Ok(conn) => ok(Value::Db(Rc::new(Handle {
            conn,
            path: path.to_string(),
        }))),
        Err(e) => error("Open", format!("cannot open database `{path}`: {e}")),
    }
}

/// Prepares `sql` and binds the fields of the `params` record.
fn prepare<'c>(
    conn: &'c rusqlite::Connection,
    sql: &str,
    params: Option<&Value>,
) -> Result<rusqlite::Statement<'c>, Value> {
    let mut stmt = conn.prepare(sql).map_err(|e| sql_error(&e))?;
    if let Some(Value::Record(fields)) = params {
        for (name, v) in fields.iter() {
            let placeholder = format!(":{name}");
            let index = match stmt.parameter_index(&placeholder) {
                Ok(Some(i)) => i,
                Ok(None) => {
                    return Err(error(
                        "Sql",
                        format!("the SQL has no parameter `{placeholder}` for field `{name}`"),
                    ));
                }
                Err(e) => return Err(sql_error(&e)),
            };
            stmt.raw_bind_parameter(index, to_sql(v))
                .map_err(|e| sql_error(&e))?;
        }
    }
    Ok(stmt)
}

fn execute(conn: &rusqlite::Connection, sql: &str, params: Option<&Value>) -> Value {
    if params.is_none() {
        // Without parameters a script of several statements is allowed.
        return match conn.execute_batch(sql) {
            Ok(()) => ok(Value::Int(conn.changes() as i64)),
            Err(e) => sql_error(&e),
        };
    }
    let mut stmt = match prepare(conn, sql, params) {
        Ok(s) => s,
        Err(e) => return e,
    };
    match stmt.raw_execute() {
        Ok(n) => ok(Value::Int(n as i64)),
        Err(e) => sql_error(&e),
    }
}

fn query(
    conn: &rusqlite::Connection,
    sql: &str,
    params: Option<&Value>,
    schema: &Schema,
) -> Result<Vec<Value>, Value> {
    let mut stmt = prepare(conn, sql, params)?;
    let columns: Vec<String> = stmt.column_names().iter().map(|c| c.to_string()).collect();
    // For a record, the column index of each field.
    let layout: Vec<(String, usize, Schema)> = match schema {
        Schema::Record(fields) => {
            let mut out = Vec::new();
            for (name, s) in fields {
                match columns.iter().position(|c| c == name) {
                    Some(i) => out.push((name.clone(), i, s.clone())),
                    None if matches!(s, Schema::Option(_)) => {}
                    None => {
                        return Err(error(
                            "TypeMismatch",
                            format!(
                                "the query has no column `{name}`; its columns are: {}",
                                columns.join(", ")
                            ),
                        ));
                    }
                }
            }
            out
        }
        _ => Vec::new(),
    };
    let mut rows = stmt.raw_query();
    let mut out = Vec::new();
    loop {
        let row = match rows.next() {
            Ok(Some(r)) => r,
            Ok(None) => break,
            Err(e) => return Err(sql_error(&e)),
        };
        let cell = |i: usize| row.get_ref(i).map_err(|e| sql_error(&e));
        let value = match schema {
            Schema::Record(fields) => {
                let mut values = Vec::with_capacity(fields.len());
                for (name, s) in fields {
                    let v = match layout.iter().find(|(n, _, _)| n == name) {
                        Some((_, i, _)) => convert(cell(*i)?, s, name)?,
                        None => Value::none(),
                    };
                    values.push((Rc::from(name.as_str()), v));
                }
                Value::Record(Rc::new(values))
            }
            _ => {
                if columns.is_empty() {
                    return Err(error("TypeMismatch", "the query returns no columns".into()));
                }
                convert(cell(0)?, schema, &columns[0])?
            }
        };
        out.push(value);
    }
    Ok(out)
}

fn convert(v: ValueRef<'_>, schema: &Schema, column: &str) -> Result<Value, Value> {
    let mismatch = |found: &str| {
        error(
            "TypeMismatch",
            format!(
                "column `{column}` holds {found}, which is not a `{}`",
                schema.describe()
            ),
        )
    };
    Ok(match (schema, v) {
        (Schema::Option(_), ValueRef::Null) => Value::none(),
        (Schema::Option(inner), v) => Value::some(convert(v, inner, column)?),
        (_, ValueRef::Null) => {
            return Err(error(
                "TypeMismatch",
                format!(
                    "column `{column}` is NULL; use `Option<{}>` for it",
                    schema.describe()
                ),
            ));
        }
        (Schema::Int, ValueRef::Integer(n)) => Value::Int(n),
        (Schema::Float, ValueRef::Real(x)) => Value::Float(x),
        (Schema::Float, ValueRef::Integer(n)) => Value::Float(n as f64),
        (Schema::Bool, ValueRef::Integer(n)) => Value::Bool(n != 0),
        (Schema::String, ValueRef::Text(t)) => match std::str::from_utf8(t) {
            Ok(s) => Value::str(s),
            Err(_) => return Err(mismatch("text that is not UTF-8")),
        },
        (_, ValueRef::Integer(_)) => return Err(mismatch("an integer")),
        (_, ValueRef::Real(_)) => return Err(mismatch("a real number")),
        (_, ValueRef::Text(_)) => return Err(mismatch("text")),
        (_, ValueRef::Blob(_)) => return Err(mismatch("a blob")),
    })
}
