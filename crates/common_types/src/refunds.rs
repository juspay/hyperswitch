//! Refund related types

use common_utils::impl_to_sql_from_sql_json;
use diesel::{sql_types::Jsonb, AsExpression, FromSqlRow};
use serde::{Deserialize, Serialize};
use smithy::SmithyModel;
use utoipa::ToSchema;

use crate::domain::{AdyenSplitData, XenditSplitSubMerchantData};

#[derive(
    Serialize,
    Deserialize,
    Debug,
    Clone,
    PartialEq,
    Eq,
    FromSqlRow,
    AsExpression,
    ToSchema,
    SmithyModel,
)]
#[diesel(sql_type = Jsonb)]
#[serde(rename_all = "snake_case")]
#[serde(deny_unknown_fields)]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
/// Charge specific fields for controlling the revert of funds from either platform or connected account. Check sub-fields for more details.
pub enum SplitRefund {
    /// StripeSplitRefundRequest
    #[smithy(value_type = "StripeSplitRefundRequest")]
    StripeSplitRefund(StripeSplitRefundRequest),
    /// AdyenSplitRefundRequest
    #[smithy(value_type = "AdyenSplitData")]
    AdyenSplitRefund(AdyenSplitData),
    /// XenditSplitRefundRequest
    #[smithy(value_type = "XenditSplitSubMerchantData")]
    XenditSplitRefund(XenditSplitSubMerchantData),
}
impl_to_sql_from_sql_json!(SplitRefund);

#[derive(
    Serialize,
    Deserialize,
    Debug,
    Clone,
    PartialEq,
    Eq,
    FromSqlRow,
    AsExpression,
    ToSchema,
    SmithyModel,
)]
#[diesel(sql_type = Jsonb)]
#[serde(deny_unknown_fields)]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
/// Charge specific fields for controlling the revert of funds from either platform or connected account for Stripe. Check sub-fields for more details.
pub struct StripeSplitRefundRequest {
    /// Toggle for reverting the application fee that was collected for the payment.
    /// If set to false, the funds are pulled from the destination account.
    #[smithy(value_type = "Option<bool>")]
    pub revert_platform_fee: Option<bool>,

    /// Toggle for reverting the transfer that was made during the charge.
    /// If set to false, the funds are pulled from the main platform's account.
    #[smithy(value_type = "Option<bool>")]
    pub revert_transfer: Option<bool>,
}
impl_to_sql_from_sql_json!(StripeSplitRefundRequest);
