//! Common types

use common_enums::enums;
use common_utils::{impl_to_sql_from_sql_json, types::MinorUnit};
use diesel::{sql_types::Jsonb, AsExpression, FromSqlRow};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(
    Serialize, Deserialize, Debug, Clone, PartialEq, Eq, FromSqlRow, AsExpression, ToSchema,
)]
#[diesel(sql_type = Jsonb)]
#[serde(deny_unknown_fields)]
/// Fee information for Split Payments to be charged on the payment being collected for Adyen
pub struct AdyenSplitData {
    /// The store identifier
    pub store: Option<String>,
    /// Data for the split items
    pub split_items: Vec<AdyenSplitItem>,
}
impl_to_sql_from_sql_json!(AdyenSplitData);

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
    #[schema(value_type = AdyenSplitType, example = "BalanceAccount")]
    pub split_type: enums::AdyenSplitType,
    /// The unique identifier of the account to which the split amount is allocated.
    pub account: Option<String>,
    /// Unique Identifier for the split item
    pub reference: String,
    /// Description for the part of the payment that will be allocated to the specified account.
    pub description: Option<String>,
}
impl_to_sql_from_sql_json!(AdyenSplitItem);
