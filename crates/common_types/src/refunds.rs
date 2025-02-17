//! Refund related types

use common_utils::impl_to_sql_from_sql_json;
use diesel::{sql_types::Jsonb, AsExpression, FromSqlRow};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::domain::AdyenSplitData;

#[derive(
    Serialize, Deserialize, Debug, Clone, PartialEq, Eq, FromSqlRow, AsExpression, ToSchema,
)]
#[diesel(sql_type = Jsonb)]
#[serde(rename_all = "snake_case")]
#[serde(deny_unknown_fields)]
/// Charge specific fields for controlling the revert of funds from either platform or connected account. Check sub-fields for more details.
pub enum SplitRefund {
    /// StripeSplitRefundRequest
    StripeSplitRefund(StripeSplitRefundRequest),
    /// AdyenSplitRefundRequest
    AdyenSplitRefund(AdyenSplitData),
}
impl_to_sql_from_sql_json!(SplitRefund);

#[derive(
    Serialize, Deserialize, Debug, Clone, PartialEq, Eq, FromSqlRow, AsExpression, ToSchema,
)]
#[diesel(sql_type = Jsonb)]
#[serde(deny_unknown_fields)]
/// Charge specific fields for controlling the revert of funds from either platform or connected account for Stripe. Check sub-fields for more details.
pub struct StripeSplitRefundRequest {
    /// Toggle for reverting the application fee that was collected for the payment.
    /// If set to false, the funds are pulled from the destination account.
    pub revert_platform_fee: Option<bool>,

    /// Toggle for reverting the transfer that was made during the charge.
    /// If set to false, the funds are pulled from the main platform's account.
    pub revert_transfer: Option<bool>,
}
impl_to_sql_from_sql_json!(StripeSplitRefundRequest);
