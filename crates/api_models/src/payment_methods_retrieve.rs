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
