use common_utils::custom_serde;
use diesel::{AsChangeset, Identifiable, Insertable, Queryable};
use serde::Serialize;
use time::PrimitiveDateTime;

use crate::{enums as storage_enums, schema::dispute};

#[derive(Clone, Debug, Insertable, Serialize, router_derive::DebugAsDisplay)]
#[diesel(table_name = dispute)]
#[serde(deny_unknown_fields)]
pub struct DisputeNew {
    pub dispute_id: String,
    pub amount: String,
    pub currency: String,
    pub dispute_stage: storage_enums::DisputeStage,
    pub dispute_status: storage_enums::DisputeStatus,
    pub payment_id: String,
    pub attempt_id: String,
    pub merchant_id: String,
    pub connector_status: String,
    pub connector_dispute_id: String,
    pub connector_reason: Option<String>,
    pub connector_reason_code: Option<String>,
    pub challenge_required_by: Option<PrimitiveDateTime>,
    pub dispute_created_at: Option<PrimitiveDateTime>,
    pub updated_at: Option<PrimitiveDateTime>,
    pub connector: String,
}

#[derive(Clone, Debug, Serialize, Identifiable, Queryable)]
#[diesel(table_name = dispute)]
pub struct Dispute {
    #[serde(skip_serializing)]
    pub id: i32,
    pub dispute_id: String,
    pub amount: String,
    pub currency: String,
    pub dispute_stage: storage_enums::DisputeStage,
    pub dispute_status: storage_enums::DisputeStatus,
    pub payment_id: String,
    pub attempt_id: String,
    pub merchant_id: String,
    pub connector_status: String,
    pub connector_dispute_id: String,
    pub connector_reason: Option<String>,
    pub connector_reason_code: Option<String>,
    pub challenge_required_by: Option<PrimitiveDateTime>,
    pub dispute_created_at: Option<PrimitiveDateTime>,
    pub updated_at: Option<PrimitiveDateTime>,
    #[serde(with = "custom_serde::iso8601")]
    pub created_at: PrimitiveDateTime,
    #[serde(with = "custom_serde::iso8601")]
    pub modified_at: PrimitiveDateTime,
    pub connector: String,
}

#[derive(Debug)]
pub enum DisputeUpdate {
    Update {
        dispute_stage: storage_enums::DisputeStage,
        dispute_status: storage_enums::DisputeStatus,
        connector_status: String,
        connector_reason: Option<String>,
        connector_reason_code: Option<String>,
        challenge_required_by: Option<PrimitiveDateTime>,
        updated_at: Option<PrimitiveDateTime>,
    },
    StatusUpdate {
        dispute_status: storage_enums::DisputeStatus,
        connector_status: Option<String>,
    },
}

#[derive(Clone, Debug, Default, AsChangeset, router_derive::DebugAsDisplay)]
#[diesel(table_name = dispute)]
pub struct DisputeUpdateInternal {
    dispute_stage: Option<storage_enums::DisputeStage>,
    dispute_status: storage_enums::DisputeStatus,
    connector_status: Option<String>,
    connector_reason: Option<String>,
    connector_reason_code: Option<String>,
    challenge_required_by: Option<PrimitiveDateTime>,
    updated_at: Option<PrimitiveDateTime>,
    modified_at: Option<PrimitiveDateTime>,
}

impl From<DisputeUpdate> for DisputeUpdateInternal {
    fn from(merchant_account_update: DisputeUpdate) -> Self {
        match merchant_account_update {
            DisputeUpdate::Update {
                dispute_stage,
                dispute_status,
                connector_status,
                connector_reason,
                connector_reason_code,
                challenge_required_by,
                updated_at,
            } => Self {
                dispute_stage: Some(dispute_stage),
                dispute_status,
                connector_status: Some(connector_status),
                connector_reason,
                connector_reason_code,
                challenge_required_by,
                updated_at,
                modified_at: Some(common_utils::date_time::now()),
            },
            DisputeUpdate::StatusUpdate {
                dispute_status,
                connector_status,
            } => Self {
                dispute_status,
                connector_status,
                modified_at: Some(common_utils::date_time::now()),
                ..Default::default()
            },
        }
    }
}
