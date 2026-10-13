//! Strict JSON parsing shared by adapters that hash or verify raw JSON bytes.

use std::{collections::BTreeSet, fmt};

use serde::{
    Deserialize,
    de::{self, MapAccess, SeqAccess, Visitor},
};
use serde_json::{Map, Number, Value};

/// Parse JSON while rejecting duplicate object keys at every nesting level.
///
/// Parsing the original text before converting it to `serde_json::Value` keeps
/// duplicate keys visible to the caller. This matters when a signature or
/// digest is checked over a canonicalized JSON body.
///
/// # Errors
///
/// Returns a JSON error for malformed input, duplicate object keys, or trailing
/// non-whitespace data.
pub fn parse_json_without_duplicate_keys(input: &str) -> Result<Value, serde_json::Error> {
    let mut deserializer = serde_json::Deserializer::from_str(input);
    let value = StrictValue::deserialize(&mut deserializer)?.0;
    deserializer.end()?;
    Ok(value)
}

struct StrictValue(Value);

struct StrictValueVisitor;

impl<'de> Deserialize<'de> for StrictValue {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_any(StrictValueVisitor)
    }
}

impl<'de> Visitor<'de> for StrictValueVisitor {
    type Value = StrictValue;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("JSON without duplicate object keys")
    }

    fn visit_bool<E>(self, value: bool) -> Result<Self::Value, E> {
        Ok(StrictValue(Value::Bool(value)))
    }

    fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E> {
        Ok(StrictValue(Value::Number(value.into())))
    }

    fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E> {
        Ok(StrictValue(Value::Number(value.into())))
    }

    fn visit_f64<E>(self, value: f64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Number::from_f64(value)
            .map(Value::Number)
            .map(StrictValue)
            .ok_or_else(|| E::custom("non-finite JSON number"))
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E> {
        Ok(StrictValue(Value::String(value.to_owned())))
    }

    fn visit_string<E>(self, value: String) -> Result<Self::Value, E> {
        Ok(StrictValue(Value::String(value)))
    }

    fn visit_unit<E>(self) -> Result<Self::Value, E> {
        Ok(StrictValue(Value::Null))
    }

    fn visit_none<E>(self) -> Result<Self::Value, E> {
        Ok(StrictValue(Value::Null))
    }

    fn visit_some<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        StrictValue::deserialize(deserializer)
    }

    fn visit_seq<A>(self, mut sequence: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let mut values = Vec::new();
        while let Some(value) = sequence.next_element::<StrictValue>()? {
            values.push(value.0);
        }
        Ok(StrictValue(Value::Array(values)))
    }

    fn visit_map<A>(self, mut object: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        let mut values = Map::new();
        let mut keys = BTreeSet::new();
        while let Some(key) = object.next_key::<String>()? {
            if !keys.insert(key.clone()) {
                return Err(de::Error::custom("duplicate JSON object key"));
            }
            let value = object.next_value::<StrictValue>()?;
            values.insert(key, value.0);
        }
        Ok(StrictValue(Value::Object(values)))
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::parse_json_without_duplicate_keys;

    #[test]
    fn strict_json_preserves_nested_values_and_floats() -> Result<(), serde_json::Error> {
        let parsed = parse_json_without_duplicate_keys(
            r#"{"items":[{"rate":1.25,"ok":true}],"none":null}"#,
        )?;
        assert_eq!(
            parsed,
            json!({"items": [{"rate": 1.25, "ok": true}], "none": null})
        );
        Ok(())
    }

    #[test]
    fn strict_json_rejects_duplicate_keys_at_any_depth() {
        assert!(parse_json_without_duplicate_keys(r#"{"a":1,"a":2}"#).is_err());
        assert!(parse_json_without_duplicate_keys(r#"{"outer":{"a":1,"a":2}}"#).is_err());
        assert!(parse_json_without_duplicate_keys(r#"{"a":1,"\u0061":2}"#).is_err());
    }

    #[test]
    fn strict_json_rejects_trailing_data() {
        assert!(parse_json_without_duplicate_keys(r#"{"a":1} {"b":2}"#).is_err());
    }
}
