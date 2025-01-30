//! Merchant connector account related types
#[cfg(feature = "v2")]
use diesel::{sql_types::Jsonb, AsExpression, FromSqlRow};
#[cfg(feature = "v2")]
use serde::{Deserialize, Serialize};
#[cfg(feature = "v2")]
use utoipa::{schema, ToSchema};

#[cfg(feature = "v2")]
#[derive(
    Serialize, Deserialize, Debug, Clone, PartialEq, Eq, FromSqlRow, AsExpression, ToSchema,
)]
#[diesel(sql_type = Jsonb)]
#[serde(rename_all = "snake_case")]
#[serde(deny_unknown_fields)]
/// Feature metadata for merchant connector account
pub struct MerchantConnectorAccountFeatureMetadata {
    /// Revenue recovery metadata specific to payment processors. ex: Stripe
    #[schema(value_type = Option<PaymentConnectorRecoveryMetadata>)]
    pub payment_connector_recovery_metadata: Option<PaymentConnectorRecoveryMetadata>,
    /// Revenue recovery metadata specific to billing processors. ex: Chargebee
    #[schema(value_type = Option<BillingConnectorRecoveryMetadata>)]
    pub billing_connector_recovery_metadata: Option<BillingConnectorRecoveryMetadata>,
}

#[cfg(feature = "v2")]
#[derive(
    Serialize, Deserialize, Debug, Clone, PartialEq, Eq, FromSqlRow, AsExpression, ToSchema,
)]
#[diesel(sql_type = Jsonb)]
#[serde(rename_all = "snake_case")]
#[serde(deny_unknown_fields)]
/// Revenue recovery metadata for payment connectors
pub struct PaymentConnectorRecoveryMetadata {
    /// connector account is reference id in an external system, which will be used to retrieve the merchant connector account id.
    #[schema(value_type = String, example = "993672945374576J")]
    pub connector_account_reference_id: String,
}

#[cfg(feature = "v2")]
#[derive(
    Serialize, Deserialize, Debug, Clone, PartialEq, Eq, FromSqlRow, AsExpression, ToSchema,
)]
#[diesel(sql_type = Jsonb)]
#[serde(rename_all = "snake_case")]
#[serde(deny_unknown_fields)]
/// Revenue recovery metadata for billing connectors
pub struct BillingConnectorRecoveryMetadata {
    /// Maximum number of retry attempts should be done for one intent.
    pub max_retry_count: i64,
    /// Retry count after which recovery attempts will be started.
    pub start_after_retry_count: i64,
}

#[cfg(feature = "v2")]
common_utils::impl_to_sql_from_sql_json!(MerchantConnectorAccountFeatureMetadata);

#[cfg(feature = "v2")]
common_utils::impl_to_sql_from_sql_json!(PaymentConnectorRecoveryMetadata);

#[cfg(feature = "v2")]
common_utils::impl_to_sql_from_sql_json!(BillingConnectorRecoveryMetadata);
