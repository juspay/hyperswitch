use crate::errors::{CustomResult, ValidationError};

crate::id_type!(
    WebhookEndpointId,
    "A type for webhook_endpoint_id that can be used for unique identifier for a webhook_endpoint"
);
crate::impl_id_type_methods!(WebhookEndpointId, "webhook_endpoint_id");
crate::impl_generate_id_id_type!(WebhookEndpointId, "whe");
crate::impl_default_id_type!(WebhookEndpointId, "whe");

// This is to display the `WebhookEndpointId` as WebhookEndpointId(abcd)
crate::impl_debug_id_type!(WebhookEndpointId);
crate::impl_try_from_cow_str_id_type!(WebhookEndpointId, "webhook_endpoint_id");

crate::impl_serializable_secret_id_type!(WebhookEndpointId);
crate::impl_queryable_id_type!(WebhookEndpointId);
crate::impl_to_sql_from_sql_id_type!(WebhookEndpointId);

impl WebhookEndpointId {
    /// Get webhook_endpoint id from String
    pub fn try_from_string(webhook_endpoint_id: String) -> CustomResult<Self, ValidationError> {
        Self::try_from(std::borrow::Cow::from(webhook_endpoint_id))
    }
}
