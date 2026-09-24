//! Payout related types.

use common_utils::impl_to_sql_from_sql_json;
use diesel::{sql_types::Jsonb, AsExpression, FromSqlRow};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Billing descriptor information for a payout.
#[derive(
    Serialize, Deserialize, Debug, Clone, PartialEq, Eq, AsExpression, FromSqlRow, ToSchema,
)]
#[diesel(sql_type = Jsonb)]
pub struct PayoutsBillingDescriptor {
    /// Reference displayed on the beneficiary's bank statement.
    pub reference: Option<String>,
    /// Statement descriptor displayed for the payout.
    pub statement_descriptor: Option<String>,
}

impl_to_sql_from_sql_json!(PayoutsBillingDescriptor);
