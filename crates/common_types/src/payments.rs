//! Payment related types

use common_enums::enums;
use common_utils::{impl_to_sql_from_sql_json, types::MinorUnit};
use diesel::{sql_types::Jsonb, AsExpression, FromSqlRow};
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
    /// AdyenSplitPayment
    AdyenSplitPayment(AdyenSplitPaymentsRequest),
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
/// Fee information for Split Payments to be charged on the payment being collected for Adyen
pub struct AdyenSplitPaymentsRequest {
    /// The store identifier
    pub store_id: Option<String>,
    /// Data for the split items
    pub split_items: Vec<AdyenSplitItem>,
}
impl_to_sql_from_sql_json!(AdyenSplitPaymentsRequest);

#[derive(
    Serialize, Deserialize, Debug, Clone, PartialEq, Eq, FromSqlRow, AsExpression, ToSchema,
)]
#[diesel(sql_type = Jsonb)]
#[serde(deny_unknown_fields)]
/// Data for the split items
pub struct AdyenSplitItem {
    /// The amount of the split item
    #[schema(value_type = i64, example = 6540)]
    pub amount: Option<MinorUnit>,
    /// Defines type of split item
    #[schema(value_type = AdyenSplitType, example = "balance_account")]
    pub split_type: enums::AdyenSplitType,
    /// The unique identifier of the account to which the split amount is allocated.
    pub account: Option<String>,
    /// Unique Identifier for the split item
    pub reference: Option<String>,
    /// Description for the part of the payment that will be allocated to the specified account.
    pub description: Option<String>,
}
impl_to_sql_from_sql_json!(AdyenSplitItem);

/// Fee information to be charged on the payment being collected via Stripe
#[derive(
    Serialize, Deserialize, Debug, Clone, PartialEq, Eq, FromSqlRow, AsExpression, ToSchema,
)]
#[diesel(sql_type = Jsonb)]
#[serde(deny_unknown_fields)]
pub struct StripeChargeResponseData {
    /// Identifier for charge created for the payment
    pub charge_id: Option<String>,

    /// Type of charge (connector specific)
    #[schema(value_type = PaymentChargeType, example = "direct")]
    pub charge_type: enums::PaymentChargeType,

    /// Platform fees collected on the payment
    #[schema(value_type = i64, example = 6540)]
    pub application_fees: MinorUnit,

    /// Identifier for the reseller's account where the funds were transferred
    pub transfer_account_id: String,
}
impl_to_sql_from_sql_json!(StripeChargeResponseData);

/// Fee information to be charged on the payment being collected via Adyen
#[derive(
    Serialize, Deserialize, Debug, Clone, PartialEq, Eq, FromSqlRow, AsExpression, ToSchema,
)]
#[diesel(sql_type = Jsonb)]
#[serde(deny_unknown_fields)]
pub struct AdyenChargeResponseData {
    /// The store identifier
    pub store_id: Option<String>,
    /// Data for the split items
    pub split_items: Vec<AdyenSplitItem>,
}
impl_to_sql_from_sql_json!(AdyenChargeResponseData);

/// Charge Information
#[derive(
    Serialize, Deserialize, Debug, Clone, PartialEq, Eq, FromSqlRow, AsExpression, ToSchema,
)]
#[diesel(sql_type = Jsonb)]
#[serde(deny_unknown_fields)]
pub enum ConnectorChargeResponseData {
    /// StripeChargeResponseData
    StripeSplitPayment(StripeChargeResponseData),
    /// AdyenChargeResponseData
    AdyenSplitPayment(AdyenChargeResponseData),
}

impl_to_sql_from_sql_json!(ConnectorChargeResponseData);
