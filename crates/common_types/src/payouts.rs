//! Payout related types.

use common_utils::impl_to_sql_from_sql_json;
use diesel::{sql_types::Jsonb, AsExpression, FromSqlRow};
use hyperswitch_masking::Secret;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Billing descriptor information for a payout.
#[derive(
    Serialize, Deserialize, Debug, Clone, PartialEq, Eq, AsExpression, FromSqlRow, ToSchema,
)]
#[diesel(sql_type = Jsonb)]
pub struct PayoutsBillingDescriptor {
    /// Name displayed in the billing descriptor.
    #[schema(value_type = Option<String>)]
    pub name: Option<Secret<String>>,
    /// City displayed in the billing descriptor.
    #[schema(value_type = Option<String>)]
    pub city: Option<Secret<String>>,
    /// Phone number displayed in the billing descriptor.
    #[schema(value_type = Option<String>)]
    pub phone: Option<Secret<String>>,
    /// Reference displayed on the beneficiary's bank statement.
    pub reference: Option<String>,
    /// Statement descriptor displayed for the payout.
    pub statement_descriptor: Option<String>,
}

impl_to_sql_from_sql_json!(PayoutsBillingDescriptor);
