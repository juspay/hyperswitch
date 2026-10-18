//! Dispute related types

use common_utils::impl_to_sql_from_sql_json;
use diesel::{sql_types::Jsonb, AsExpression, FromSqlRow};
use serde::{Deserialize, Serialize};
use smithy::SmithyModel;
use utoipa::ToSchema;

use crate::primitive_wrappers::RapidDisputeResolutionAppliedBool;

/// Additional details of a dispute, stored as jsonb
#[derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
    Serialize,
    Deserialize,
    FromSqlRow,
    AsExpression,
    ToSchema,
    SmithyModel,
)]
#[diesel(sql_type = Jsonb)]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub struct AdditionalDetails {
    /// Network specific details of the dispute
    #[smithy(value_type = "Option<DisputeNetworkDetails>")]
    pub network_details: Option<DisputeNetworkDetails>,
}
impl_to_sql_from_sql_json!(AdditionalDetails);

/// Card network specific details of a dispute
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema, SmithyModel)]
#[serde(tag = "type", rename_all = "lowercase")]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub enum DisputeNetworkDetails {
    /// Visa specific dispute details
    #[smithy(nested_value_type)]
    Visa {
        /// Rapid Dispute Resolution details
        #[smithy(value_type = "Option<RapidDisputeResolution>")]
        rapid_dispute_resolution: Option<RapidDisputeResolution>,
    },
}

/// Visa Rapid Dispute Resolution details
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema, SmithyModel)]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub struct RapidDisputeResolution {
    /// Whether Rapid Dispute Resolution has been applied
    #[schema(value_type = bool)]
    #[smithy(value_type = "bool")]
    pub applied: RapidDisputeResolutionAppliedBool,
}
