use std::collections::BTreeMap;

use dim_lang::Value;
use napi::bindgen_prelude::*;

pub fn json_to_value(value: serde_json::Value) -> Result<Value> {
    match value {
        serde_json::Value::Null => Err(Error::from_reason(
            "cannot convert null to dim value",
        )),
        serde_json::Value::Bool(_) => Err(Error::from_reason("boolean values are not supported")),
        serde_json::Value::Number(n) => {
            let i = n
                .as_i64()
                .ok_or_else(|| Error::from_reason(format!("number out of i64 range: {n}")))?;
            Ok(Value::Int(i))
        }
        serde_json::Value::String(_) => Err(Error::from_reason("string values are not supported")),
        serde_json::Value::Array(items) => {
            let mut list = Vec::with_capacity(items.len());
            for item in items {
                list.push(json_to_value(item)?);
            }
            Ok(Value::List(list))
        }
        serde_json::Value::Object(fields) => {
            let all_int_keys = !fields.is_empty()
                && fields.keys().all(|k| k.parse::<i64>().is_ok());
            if all_int_keys {
                let mut map = BTreeMap::new();
                for (key, item) in fields {
                    let k: i64 = key
                        .parse()
                        .map_err(|_| Error::from_reason(format!("invalid map key `{key}`")))?;
                    map.insert(k, json_to_value(item)?);
                }
                Ok(Value::Map(map))
            } else {
                let mut record = BTreeMap::new();
                for (key, item) in fields {
                    record.insert(key, json_to_value(item)?);
                }
                Ok(Value::Record(record))
            }
        }
    }
}
