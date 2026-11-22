//! `std.db` without the `native` feature (the browser playground): SQLite is
//! not available, so every call is an `Err(DbError.Open(...))`.

use std::collections::HashMap;
use std::fmt;
use std::rc::Rc;

use elomar_ast::TypeExpr;
use elomar_policy::FsPolicy;

use crate::Value;

/// Never constructed here; keeps `Value::Db` the same in every build.
pub struct Handle {
    pub path: String,
}

impl fmt::Debug for Handle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Db({})", self.path)
    }
}

pub(crate) fn call(
    _policy: &FsPolicy,
    _func: &str,
    _type_args: &[TypeExpr],
    _args: &[Value],
    _unit_enums: &HashMap<String, Vec<String>>,
) -> Value {
    Value::Result(Err(Rc::new(Value::enum_value(
        "DbError",
        "Open",
        vec![Value::str(
            "databases are not available in this build (for example in the browser playground)",
        )],
    ))))
}
