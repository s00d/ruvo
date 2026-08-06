//! Coerce query/params JSON strings using a JSON Schema `properties` map.

use serde_json::{Map, Value};

/// Coerce string values in `obj` according to `schema` property types.
/// Only mutates top-level properties (and one level of arrays of primitives).
pub fn coerce_object(obj: &mut Map<String, Value>, schema: &Value) {
    let Some(props) = schema.get("properties").and_then(|p| p.as_object()) else {
        return;
    };
    for (key, prop_schema) in props {
        let Some(val) = obj.get_mut(key) else {
            continue;
        };
        coerce_value(val, prop_schema);
    }
}

fn coerce_value(val: &mut Value, schema: &Value) {
    let ty = schema.get("type").and_then(|t| t.as_str());
    match (val.take(), ty) {
        (Value::String(s), Some("integer")) => {
            *val = s
                .parse::<i64>()
                .map(Value::from)
                .unwrap_or(Value::String(s));
        }
        (Value::String(s), Some("number")) => {
            *val = s
                .parse::<f64>()
                .map(Value::from)
                .unwrap_or(Value::String(s));
        }
        (Value::String(s), Some("boolean")) => {
            *val = match s.as_str() {
                "true" | "1" | "yes" => Value::Bool(true),
                "false" | "0" | "no" => Value::Bool(false),
                _ => Value::String(s),
            };
        }
        (Value::String(s), Some("array")) => {
            // single value → one-element array for `tags=a` vs `tags[]=a`
            let item = schema.get("items").cloned().unwrap_or(Value::Null);
            let mut one = Value::String(s);
            coerce_value(&mut one, &item);
            *val = Value::Array(vec![one]);
        }
        (Value::Array(mut arr), Some("array")) => {
            let item = schema.get("items").cloned().unwrap_or(Value::Null);
            for el in &mut arr {
                coerce_value(el, &item);
            }
            *val = Value::Array(arr);
        }
        (other, _) => *val = other,
    }
}
