use common_utils::{
    custom_serde,
    types::{MinorUnit, StringMinorUnit},
};
use diesel::{AsChangeset, Identifiable, Insertable, Queryable, Selectable};
use masking::Secret;
use serde::Serialize;
use time::PrimitiveDateTime;

use crate::{enums as storage_enums, schema::dispute};

#[derive(Clone, Debug, Insertable, Serialize, router_derive::DebugAsDisplay)]
#[diesel(table_name = dispute)]
#[serde(deny_unknown_fields)]
pub struct DisputeNew {
    pub dispute_id: String,
    pub amount: StringMinorUnit,
    pub currency: String,
    pub dispute_stage: storage_enums::DisputeStage,
    pub dispute_status: storage_enums::DisputeStatus,
    pub payment_id: common_utils::id_type::PaymentId,
    pub attempt_id: String,
    pub merchant_id: common_utils::id_type::MerchantId,
    pub connector_status: String,
    pub connector_dispute_id: String,
    pub connector_reason: Option<String>,
    pub connector_reason_code: Option<String>,
    pub challenge_required_by: Option<PrimitiveDateTime>,
    pub connector_created_at: Option<PrimitiveDateTime>,
    pub connector_updated_at: Option<PrimitiveDateTime>,
    pub connector: String,
    pub evidence: Option<Secret<serde_json::Value>>,
    pub profile_id: Option<common_utils::id_type::ProfileId>,
    pub merchant_connector_id: Option<common_utils::id_type::MerchantConnectorAccountId>,
    pub dispute_amount: MinorUnit,
    pub organization_id: common_utils::id_type::OrganizationId,
    pub dispute_currency: Option<storage_enums::Currency>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Identifiable, Queryable, Selectable)]
#[diesel(table_name = dispute, primary_key(dispute_id), check_for_backend(diesel::pg::Pg))]
pub struct Dispute {
    pub dispute_id: String,
    pub amount: StringMinorUnit,
    pub currency: String,
    pub dispute_stage: storage_enums::DisputeStage,
    pub dispute_status: storage_enums::DisputeStatus,
    pub payment_id: common_utils::id_type::PaymentId,
    pub attempt_id: String,
    pub merchant_id: common_utils::id_type::MerchantId,
    pub connector_status: String,
    pub connector_dispute_id: String,
    pub connector_reason: Option<String>,
    pub connector_reason_code: Option<String>,
    pub challenge_required_by: Option<PrimitiveDateTime>,
    pub connector_created_at: Option<PrimitiveDateTime>,
    pub connector_updated_at: Option<PrimitiveDateTime>,
    #[serde(with = "custom_serde::iso8601")]
    pub created_at: PrimitiveDateTime,
    #[serde(with = "custom_serde::iso8601")]
    pub modified_at: PrimitiveDateTime,
    pub connector: String,
    pub evidence: Secret<serde_json::Value>,
    pub profile_id: Option<common_utils::id_type::ProfileId>,
    pub merchant_connector_id: Option<common_utils::id_type::MerchantConnectorAccountId>,
    pub dispute_amount: MinorUnit,
    pub organization_id: common_utils::id_type::OrganizationId,
    pub dispute_currency: Option<storage_enums::Currency>,
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
        connector_updated_at: Option<PrimitiveDateTime>,
    },
    StatusUpdate {
        dispute_status: storage_enums::DisputeStatus,
        connector_status: Option<String>,
    },
    EvidenceUpdate {
        evidence: Secret<serde_json::Value>,
    },
}

#[derive(Clone, Debug, AsChangeset, router_derive::DebugAsDisplay)]
#[diesel(table_name = dispute)]
pub struct DisputeUpdateInternal {
    dispute_stage: Option<storage_enums::DisputeStage>,
    dispute_status: Option<storage_enums::DisputeStatus>,
    connector_status: Option<String>,
    connector_reason: Option<String>,
    connector_reason_code: Option<String>,
    challenge_required_by: Option<PrimitiveDateTime>,
    connector_updated_at: Option<PrimitiveDateTime>,
    modified_at: PrimitiveDateTime,
    evidence: Option<Secret<serde_json::Value>>,
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
                connector_updated_at,
            } => Self {
                dispute_stage: Some(dispute_stage),
                dispute_status: Some(dispute_status),
                connector_status: Some(connector_status),
                connector_reason,
                connector_reason_code,
                challenge_required_by,
                connector_updated_at,
                modified_at: common_utils::date_time::now(),
                evidence: None,
            },
            DisputeUpdate::StatusUpdate {
                dispute_status,
                connector_status,
            } => Self {
                dispute_status: Some(dispute_status),
                connector_status,
                modified_at: common_utils::date_time::now(),
                dispute_stage: None,
                connector_reason: None,
                connector_reason_code: None,
                challenge_required_by: None,
                connector_updated_at: None,
                evidence: None,
            },
            DisputeUpdate::EvidenceUpdate { evidence } => Self {
                evidence: Some(evidence),
                dispute_stage: None,
                dispute_status: None,
                connector_status: None,
                connector_reason: None,
                connector_reason_code: None,
                challenge_required_by: None,
                connector_updated_at: None,
                modified_at: common_utils::date_time::now(),
            },
        }
    }
}
