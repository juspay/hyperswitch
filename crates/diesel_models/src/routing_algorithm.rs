use diesel::{Identifiable, Insertable, Queryable};
use serde::{Deserialize, Serialize};

use crate::{enums, schema::routing_algorithm};

#[derive(Clone, Debug, Identifiable, Insertable, Queryable, Serialize, Deserialize)]
#[diesel(table_name = routing_algorithm, primary_key(algorithm_id))]
pub struct RoutingAlgorithm {
    pub algorithm_id: String,
    pub profile_id: String,
    pub merchant_id: String,
    pub name: String,
    pub description: Option<String>,
    pub kind: enums::RoutingAlgorithmKind,
    pub algorithm_data: serde_json::Value,
    pub created_at: time::PrimitiveDateTime,
    pub modified_at: time::PrimitiveDateTime,
}

pub struct RoutingAlgorithmMetadata {
    pub algorithm_id: String,
    pub name: String,
    pub description: Option<String>,
    pub kind: enums::RoutingAlgorithmKind,
    pub created_at: time::PrimitiveDateTime,
    pub modified_at: time::PrimitiveDateTime,
}

pub struct RoutingProfileMetadata {
    pub profile_id: String,
    pub algorithm_id: String,
    pub name: String,
    pub description: Option<String>,
    pub kind: enums::RoutingAlgorithmKind,
    pub created_at: time::PrimitiveDateTime,
    pub modified_at: time::PrimitiveDateTime,
}
