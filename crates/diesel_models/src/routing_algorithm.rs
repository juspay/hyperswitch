use common_utils::id_type;
use diesel::{Identifiable, Insertable, Queryable, Selectable};
use serde::{Deserialize, Serialize};

use crate::{enums, schema::routing_algorithm};

#[derive(Clone, Debug, Identifiable, Insertable, Queryable, Selectable, Serialize, Deserialize)]
#[diesel(table_name = routing_algorithm, primary_key(algorithm_id), check_for_backend(diesel::pg::Pg))]
pub struct RoutingAlgorithm {
    pub algorithm_id: id_type::RoutingId,
    pub profile_id: id_type::ProfileId,
    pub merchant_id: id_type::MerchantId,
    pub name: String,
    pub description: Option<String>,
    pub kind: enums::RoutingAlgorithmKind,
    pub algorithm_data: serde_json::Value,
    pub created_at: time::PrimitiveDateTime,
    pub modified_at: time::PrimitiveDateTime,
    pub algorithm_for: enums::TransactionType,
    pub decision_engine_routing_id: Option<String>,
}

pub struct RoutingAlgorithmMetadata {
    pub algorithm_id: id_type::RoutingId,
    pub name: String,
    pub description: Option<String>,
    pub kind: enums::RoutingAlgorithmKind,
    pub created_at: time::PrimitiveDateTime,
    pub modified_at: time::PrimitiveDateTime,
    pub algorithm_for: enums::TransactionType,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RoutingProfileMetadata {
    pub profile_id: id_type::ProfileId,
    pub algorithm_id: id_type::RoutingId,
    pub name: String,
    pub description: Option<String>,
    pub kind: enums::RoutingAlgorithmKind,
    pub created_at: time::PrimitiveDateTime,
    pub modified_at: time::PrimitiveDateTime,
    pub algorithm_for: enums::TransactionType,
}

fn extract_display_name(name: &str) -> String {
    const PREFIXES: &[&str] = &["SUCCESS:", "ELIMINATION:", "CONTRACT:"];

    for prefix in PREFIXES {
        if name.starts_with(prefix) {
            return name.strip_prefix(prefix).unwrap_or(name).to_string();
        }
    }
    name.to_string()
}

impl RoutingProfileMetadata {
    pub fn metadata_is_advanced_rule_for_payments(&self) -> bool {
        matches!(self.kind, enums::RoutingAlgorithmKind::Advanced)
            && matches!(self.algorithm_for, enums::TransactionType::Payment)
    }

    pub fn get_display_name(&self) -> String {
        extract_display_name(&self.name)
    }
}

impl RoutingAlgorithmMetadata {
    pub fn get_display_name(&self) -> String {
        extract_display_name(&self.name)
    }
}

impl RoutingAlgorithm {
    pub fn get_display_name(&self) -> String {
        extract_display_name(&self.name)
    }

    pub fn create_success_prefixed_name(name: &str) -> String {
        format!("SUCCESS:{}", name)
    }

    pub fn create_elimination_prefixed_name(name: &str) -> String {
        format!("ELIMINATION:{}", name)
    }

    pub fn create_contract_prefixed_name(name: &str) -> String {
        format!("CONTRACT:{}", name)
    }
}
