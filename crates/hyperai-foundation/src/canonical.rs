use serde::Serialize;
use serde_json::Value;
use sha2::{Digest as _, Sha256};
use thiserror::Error;
use unicode_normalization::UnicodeNormalization;

use crate::Sha256Digest;

/// Failures produced before a protocol value can be hashed.
#[derive(Debug, Error)]
pub enum CanonicalizationError {
    /// The value could not be converted to JSON.
    #[error("value is not serializable JSON: {0}")]
    Serialization(#[from] serde_json::Error),
    /// A JSON floating-point number was found.
    #[error("floating-point values are forbidden at {path}")]
    FloatingPoint {
        /// JSON-pointer-like location of the invalid number.
        path: String,
    },
    /// A string or object key was not already NFC-normalized.
    #[error("string is not NFC-normalized at {path}")]
    NonNormalizedUnicode {
        /// JSON-pointer-like location of the invalid string.
        path: String,
    },
    /// The digest domain was empty or contained the NUL separator byte.
    #[error("digest domain must be non-empty ASCII without NUL bytes")]
    InvalidDomain,
}

/// Convert a serializable value to RFC 8785 JSON Canonicalization Scheme bytes.
///
/// Resolution-critical values reject floating-point tokens and strings that are
/// not already NFC-normalized before canonicalization.
///
/// # Errors
///
/// Returns an error when serialization fails or the value contains a float or
/// non-NFC string.
pub fn canonical_json<T: Serialize>(value: &T) -> Result<Vec<u8>, CanonicalizationError> {
    let json = serde_json::to_value(value)?;
    validate_value(&json, "$")?;
    Ok(serde_json_canonicalizer::to_vec(&json)?)
}

/// Hash canonical JSON with an ASCII domain and a NUL separator.
///
/// # Errors
///
/// Returns an error for an invalid domain or a value that cannot be safely
/// canonicalized.
pub fn domain_separated_digest<T: Serialize>(
    domain: &str,
    value: &T,
) -> Result<Sha256Digest, CanonicalizationError> {
    if domain.is_empty() || !domain.is_ascii() || domain.as_bytes().contains(&0) {
        return Err(CanonicalizationError::InvalidDomain);
    }
    let canonical = canonical_json(value)?;
    let mut hasher = Sha256::new();
    hasher.update(domain.as_bytes());
    hasher.update([0]);
    hasher.update(canonical);
    let bytes: [u8; 32] = hasher.finalize().into();
    Ok(Sha256Digest::from_bytes(bytes))
}

fn validate_value(value: &Value, path: &str) -> Result<(), CanonicalizationError> {
    match value {
        Value::Null | Value::Bool(_) => Ok(()),
        Value::Number(number) => {
            if number.is_i64() || number.is_u64() {
                Ok(())
            } else {
                Err(CanonicalizationError::FloatingPoint {
                    path: path.to_owned(),
                })
            }
        }
        Value::String(text) => validate_nfc(text, path),
        Value::Array(values) => values
            .iter()
            .enumerate()
            .try_for_each(|(index, item)| validate_value(item, &format!("{path}/{index}"))),
        Value::Object(values) => values.iter().try_for_each(|(key, item)| {
            let child_path = format!("{path}/{}", escape_pointer(key));
            validate_nfc(key, &child_path)?;
            validate_value(item, &child_path)
        }),
    }
}

fn validate_nfc(text: &str, path: &str) -> Result<(), CanonicalizationError> {
    if text.nfc().eq(text.chars()) {
        Ok(())
    } else {
        Err(CanonicalizationError::NonNormalizedUnicode {
            path: path.to_owned(),
        })
    }
}

fn escape_pointer(value: &str) -> String {
    value.replace('~', "~0").replace('/', "~1")
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use proptest::prelude::*;
    use serde_json::json;

    use super::*;

    #[test]
    fn canonical_json_orders_object_keys() {
        let first = json!({"z": 1, "a": 2});
        let second = json!({"a": 2, "z": 1});
        assert_eq!(canonical_json(&first).ok(), canonical_json(&second).ok());
        assert_eq!(
            canonical_json(&first).ok().as_deref(),
            Some(br#"{"a":2,"z":1}"#.as_slice())
        );
    }

    #[test]
    fn canonical_json_rejects_float_and_non_nfc() {
        assert!(matches!(
            canonical_json(&json!({"unsafe": 0.5})),
            Err(CanonicalizationError::FloatingPoint { .. })
        ));
        assert!(matches!(
            canonical_json(&json!({"text": "e\u{301}"})),
            Err(CanonicalizationError::NonNormalizedUnicode { .. })
        ));
    }

    proptest! {
        #[test]
        fn canonicalization_is_stable_across_insertion_order(
            pairs in prop::collection::btree_map("[a-z]{1,8}", any::<i32>(), 0..32)
        ) {
            let mut forward = serde_json::Map::new();
            let mut reverse = serde_json::Map::new();
            for (key, value) in &pairs {
                forward.insert(key.clone(), Value::from(*value));
            }
            for (key, value) in pairs.iter().rev() {
                reverse.insert(key.clone(), Value::from(*value));
            }
            prop_assert_eq!(
                canonical_json(&Value::Object(forward)).ok(),
                canonical_json(&Value::Object(reverse)).ok()
            );
        }

        #[test]
        fn btree_maps_produce_stable_digests(
            entries in prop::collection::btree_map("[a-z]{1,8}", any::<i32>(), 0..32)
        ) {
            let copy: BTreeMap<String, i32> = entries.clone();
            prop_assert_eq!(
                domain_separated_digest("hyperai.test.v1", &entries).ok(),
                domain_separated_digest("hyperai.test.v1", &copy).ok()
            );
        }
    }
}
