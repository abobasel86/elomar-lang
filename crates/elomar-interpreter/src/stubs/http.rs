//! `std.http` without the `native` feature (the browser playground): every
//! request is an `Err(HttpError.Network(...))`.

use std::rc::Rc;

use elomar_ai_core::Permissions;

use crate::Value;

pub(crate) fn call(_permissions: &Permissions, _func: &str, _args: &[Value]) -> Value {
    Value::Result(Err(Rc::new(Value::Enum {
        enum_name: "HttpError".into(),
        variant: "Network".into(),
        fields: Rc::new(vec![Value::str(
            "HTTP requests are not available in this build (for example in the browser playground)",
        )]),
    })))
}
