//! Payment related types
use std::collections::HashMap;

use common_enums::enums;
use common_utils::{errors, impl_to_sql_from_sql_json, types::MinorUnit};
use diesel::{
    sql_types::{Json, Jsonb},
    AsExpression, FromSqlRow,
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(
    Serialize, Deserialize, Debug, Clone, PartialEq, Eq, FromSqlRow, AsExpression, ToSchema,
)]
#[diesel(sql_type = Jsonb)]
#[serde(rename_all = "snake_case")]
#[serde(deny_unknown_fields)]
/// Fee information for Split Payments to be charged on the payment being collected
pub enum SplitPaymentsRequest {
    /// StripeSplitPayment
    StripeSplitPayment(StripeSplitPaymentRequest),
}
impl_to_sql_from_sql_json!(SplitPaymentsRequest);

#[derive(
    Serialize, Deserialize, Debug, Clone, PartialEq, Eq, FromSqlRow, AsExpression, ToSchema,
)]
#[diesel(sql_type = Jsonb)]
#[serde(deny_unknown_fields)]
/// Fee information for Split Payments to be charged on the payment being collected for Stripe
pub struct StripeSplitPaymentRequest {
    /// Stripe's charge type
    #[schema(value_type = PaymentChargeType, example = "direct")]
    pub charge_type: enums::PaymentChargeType,

    /// Platform fees to be collected on the payment
    #[schema(value_type = i64, example = 6540)]
    pub application_fees: MinorUnit,

    /// Identifier for the reseller's account to send the funds to
    pub transfer_account_id: String,
}

impl_to_sql_from_sql_json!(StripeSplitPaymentRequest);

#[derive(
    Serialize, Deserialize, Debug, Clone, PartialEq, Eq, FromSqlRow, AsExpression, ToSchema,
)]
#[diesel(sql_type = Jsonb)]
#[serde(deny_unknown_fields)]
/// Hashmap to store mca_id's with product names
pub struct AuthenticationConnectorAccountMap(
    HashMap<enums::AuthenticationProduct, common_utils::id_type::MerchantConnectorAccountId>,
);
impl_to_sql_from_sql_json!(AuthenticationConnectorAccountMap);

impl AuthenticationConnectorAccountMap {
    /// fn to get click to pay connector_account_id
    pub fn get_click_to_pay_connector_account_id(
        &self,
    ) -> Result<common_utils::id_type::MerchantConnectorAccountId, errors::ValidationError> {
        self.0
            .get(&enums::AuthenticationProduct::ClickToPay)
            .ok_or(errors::ValidationError::MissingRequiredField {
                field_name: "authentication_product_id.click_to_pay".to_string(),
            })
            .cloned()
    }
}

#[cfg(feature = "v2")]
#[derive(
    Serialize, Deserialize, Debug, Clone, PartialEq, Eq, FromSqlRow, AsExpression, ToSchema,
)]
#[diesel(sql_type = Json)]
/// Metadata for the revenue recovery payment intent
pub struct PCRPaymentIntentFeatureMetadata {
    /// Number of attempts that have been made for the order
    #[schema(value_type = i32, example = 1)]
    pub retry_count: i32,
    /// Denotes whether the connector has been called
    pub called_connector: bool,
    /// The merchant connector account id for the billing connector
    pub billing_connector_mca_id: common_utils::id_type::MerchantConnectorAccountId,
    /// The merchant connector account id for the payment connector
    pub payment_connector_mca_id: common_utils::id_type::MerchantConnectorAccountId,
    /// The mandate details for the connector
    pub connector_mandate_details: PCRConnectorMandateDetails,
}

#[cfg(feature = "v2")]
#[derive(
    Serialize, Deserialize, Debug, Clone, PartialEq, Eq, FromSqlRow, AsExpression, ToSchema,
)]
#[diesel(sql_type = Json)]
/// Mandate details for the PCR connector
pub struct PCRConnectorMandateDetails {
    /// The payment processor token for the PCR connector
    pub payment_processor_token: String,
    /// The connector customer id for the PCR connector
    pub connector_customer_id: String,
}

#[cfg(feature = "v2")]
common_utils::impl_to_sql_from_sql_json!(PCRPaymentIntentFeatureMetadata);
#[cfg(feature = "v2")]
common_utils::impl_to_sql_from_sql_json!(PCRConnectorMandateDetails);
