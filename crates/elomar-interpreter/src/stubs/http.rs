//! `std.http` without the `native` feature (the browser playground): every
//! request is an `Err(HttpError.Network(...))`.

use std::rc::Rc;

use elomar_ai_core::Permissions;

use crate::Value;

pub(crate) fn call(_permissions: &Permissions, _func: &str, _args: &[Value]) -> Value {
    Value::Result(Err(Rc::new(Value::enum_value(
        "HttpError",
        "Network",
        vec![Value::str(
            "HTTP requests are not available in this build (for example in the browser playground)",
        )],
    ))))
}
