use actrpc_core::error::{ActionExecutionError, InterceptorError, JorinError};
use serde_json::{Value, json};

#[test]
fn test_jorin_error_serde_roundtrip() {
    let cases: Vec<(JorinError, Value)> = vec![
        // InvalidMessage
        (
            JorinError::InvalidMessage {
                message: "malformed id field".to_string(),
            },
            json!({
                "type": "InvalidMessage",
                "message": "malformed id field"
            }),
        ),
        // Internal
        (
            JorinError::Internal {
                message: "database connection lost".to_string(),
            },
            json!({
                "type": "Internal",
                "message": "database connection lost"
            }),
        ),
        // Serialization
        (
            JorinError::Serialization {
                message: "cannot serialize f64::NAN".to_string(),
            },
            json!({
                "type": "Serialization",
                "message": "cannot serialize f64::NAN"
            }),
        ),
        // ActionExecution – nested enum uses variant name as key
        (
            JorinError::ActionExecution(ActionExecutionError::InvalidParams {
                action: "modify_params".to_string(),
            }),
            json!({
                "type": "ActionExecution",
                "InvalidParams": {
                    "action": "modify_params"
                }
            }),
        ),
        (
            JorinError::Interceptor(InterceptorError::ExecutionFailed {
                name: "auth-check".to_string(),
                reason: "token expired".to_string(),
            }),
            json!({
                "type": "Interceptor",
                "ExecutionFailed": {
                    "name": "auth-check",
                    "reason": "token expired"
                }
            }),
        ),
        (
            JorinError::Interceptor(InterceptorError::MalformedResponse),
            json!({
                "type": "Interceptor",
                "MalformedResponse": null
            }),
        ),
    ];

    for (original, expected_json) in cases {
        let serialized = serde_json::to_value(&original).expect("serialization failed");

        assert_eq!(
            serialized, expected_json,
            "Serialization shape mismatch for variant {:?}",
            original
        );

        let roundtripped: JorinError =
            serde_json::from_value(serialized.clone()).expect("deserialization failed");

        assert_eq!(roundtripped, original, "Round-trip equality failed");

        assert_eq!(
            roundtripped.to_string(),
            original.to_string(),
            "Display mismatch"
        );
    }
}

#[test]
fn test_interceptor_error_display() {
    let cases = vec![
        (
            InterceptorError::ExecutionFailed {
                name: "rate-limiter".to_string(),
                reason: "quota exceeded".to_string(),
            },
            "interceptor 'rate-limiter' failed: quota exceeded",
        ),
        (
            InterceptorError::MalformedResponse,
            "malformed response from interceptor",
        ),
    ];

    for (err, expected) in cases {
        assert_eq!(err.to_string(), expected);
    }
}
