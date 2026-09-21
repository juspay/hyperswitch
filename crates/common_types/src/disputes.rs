//! Dispute related types

use common_utils::impl_to_sql_from_sql_json;
use diesel::{sql_types::Jsonb, AsExpression, FromSqlRow};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::primitive_wrappers::RapidDisputeResolutionAppliedBool;

/// Additional details of a dispute, stored as jsonb
#[derive(
    Debug, Clone, PartialEq, Eq, Serialize, Deserialize, FromSqlRow, AsExpression, ToSchema,
)]
#[diesel(sql_type = Jsonb)]
pub struct AdditionalDetails {
    /// Network specific details of the dispute
    pub network_details: Option<DisputeNetworkDetails>,
}
impl_to_sql_from_sql_json!(AdditionalDetails);

/// Card network specific details of a dispute
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum DisputeNetworkDetails {
    /// Visa specific dispute details
    Visa {
        /// Rapid Dispute Resolution details
        rapid_dispute_resolution: Option<RapidDisputeResolution>,
    },
}

/// Visa Rapid Dispute Resolution details
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct RapidDisputeResolution {
    /// Whether Rapid Dispute Resolution has been applied
    #[schema(value_type = bool)]
    pub applied: RapidDisputeResolutionAppliedBool,
}
