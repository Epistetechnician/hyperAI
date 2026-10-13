use serde_json::Value;

/// Schema identifier used for local-service error responses.
pub const API_ERROR_SCHEMA_VERSION: &str = "hyperai.interface.error.v1";

/// Fixed message returned to clients instead of untrusted response-body text.
pub const API_ERROR_MESSAGE: &str = "The local HyperAI contract operation could not complete.";

/// Normalized status, stable code, and generic message for a local interface error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NormalizedInterfaceErrorV1 {
    /// HTTP status returned by the local service.
    pub status: u16,
    /// Validated service code, or `UNKNOWN_ERROR` for an invalid envelope.
    pub code: String,
    /// Fixed, non-sensitive message defined by the interface contract.
    pub message: &'static str,
}

/// Decode the versioned local-service error contract without trusting its message.
///
/// Invalid or unsupported envelopes retain the HTTP status and normalize to
/// `UNKNOWN_ERROR`. The response body message is deliberately ignored so a
/// malformed or unexpected local response cannot disclose request data.
pub fn normalize_interface_error_v1(status: u16, body: &Value) -> NormalizedInterfaceErrorV1 {
    let code = body
        .as_object()
        .filter(|_| body["schema_version"] == API_ERROR_SCHEMA_VERSION && body["status"] == "error")
        .and_then(|_| body["code"].as_str())
        .filter(|code| valid_interface_error_code(code))
        .map_or_else(|| "UNKNOWN_ERROR".to_owned(), str::to_owned);
    NormalizedInterfaceErrorV1 {
        status,
        code,
        message: API_ERROR_MESSAGE,
    }
}

fn valid_interface_error_code(code: &str) -> bool {
    (1..=80).contains(&code.len())
        && code.bytes().all(|byte| {
            byte.is_ascii_uppercase() || byte.is_ascii_digit() || matches!(byte, b'_' | b'-')
        })
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{API_ERROR_MESSAGE, normalize_interface_error_v1};

    #[test]
    fn normalized_error_preserves_http_status_and_valid_service_code() {
        let error = normalize_interface_error_v1(
            422,
            &json!({
                "schema_version": "hyperai.interface.error.v1",
                "status": "error",
                "code": "DATA_QUERY_REJECTED",
                "message": "private request content",
            }),
        );

        assert_eq!(error.status, 422);
        assert_eq!(error.code, "DATA_QUERY_REJECTED");
        assert_eq!(error.message, API_ERROR_MESSAGE);
        assert!(!error.message.contains("private"));
    }

    #[test]
    fn malformed_or_unknown_error_envelopes_fail_closed() {
        for body in [
            json!(null),
            json!({"schema_version": "other.v1", "status": "error", "code": "DATA_QUERY_REJECTED"}),
            json!({"schema_version": "hyperai.interface.error.v1", "status": "ok", "code": "DATA_QUERY_REJECTED"}),
            json!({"schema_version": "hyperai.interface.error.v1", "status": "error", "code": "BAD\nCODE"}),
            json!({"schema_version": "hyperai.interface.error.v1", "status": "error", "code": "C".repeat(81)}),
            json!({"schema_version": "hyperai.interface.error.v1", "status": "error", "code": "Ü"}),
        ] {
            let error = normalize_interface_error_v1(500, &body);
            assert_eq!(error.status, 500);
            assert_eq!(error.code, "UNKNOWN_ERROR", "body={body}");
            assert_eq!(error.message, API_ERROR_MESSAGE);
        }
    }
}
