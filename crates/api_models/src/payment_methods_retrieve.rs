use utoipa::ToSchema;

/// Request to retrieve a single payment method.
///
/// In the HTTP layer this is typically taken from the path
/// (`GET /payment_methods/{id}`), but we keep a request shape here so that
/// API docs, tests, and client codegen can reuse the same model.
#[derive(Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct PaymentMethodsRetrieveRequest {
    /// Unique payment method identifier.
    ///
    /// Example: "pm_123456789"
    #[schema(example = "pm_123456789")]
    pub id: String,
}

/// Response returned after a payment method is retrieved.
///
/// Keep this aligned with what we expose in `create` so that
/// create → retrieve is round-trippable.
#[derive(Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct PaymentMethodsRetrieveResponse {
    pub id: String,
    pub payment_method_type: String,
    pub status: String,
    /// Optional metadata attached to the payment method.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;

    #[test]
    fn test_payment_methods_retrieve_request_deserialize() {
        let json = r#"{"id":"pm_123456789"}"#;
        let req: PaymentMethodsRetrieveRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.id, "pm_123456789");
    }

    #[test]
    fn test_payment_methods_retrieve_request_deny_unknown_fields() {
        let json = r#"{"id":"pm_test","unknown_field":"value"}"#;
        let result: Result<PaymentMethodsRetrieveRequest, _> = serde_json::from_str(json);
        assert!(result.is_err(), "Should reject unknown fields");
    }

    #[test]
    fn test_payment_methods_retrieve_response_roundtrip() {
        let response = PaymentMethodsRetrieveResponse {
            id: "pm_test_123".to_string(),
            payment_method_type: "card".to_string(),
            status: "active".to_string(),
            metadata: Some(serde_json::json!({"key": "value"})),
        };

        let serialized = serde_json::to_string(&response).unwrap();
        let deserialized: PaymentMethodsRetrieveResponse =
            serde_json::from_str(&serialized).unwrap();

        assert_eq!(response.id, deserialized.id);
        assert_eq!(response.payment_method_type, deserialized.payment_method_type);
        assert_eq!(response.status, deserialized.status);
        assert_eq!(response.metadata, deserialized.metadata);
    }

    #[test]
    fn test_payment_methods_retrieve_response_omits_none_metadata() {
        let response = PaymentMethodsRetrieveResponse {
            id: "pm_no_meta".to_string(),
            payment_method_type: "wallet".to_string(),
            status: "inactive".to_string(),
            metadata: None,
        };

        let serialized = serde_json::to_string(&response).unwrap();
        assert!(!serialized.contains("metadata"), "None metadata should be omitted");
    }
}
