use common_utils::pii;
use diesel::{AsChangeset, Identifiable, Insertable, Queryable};
use masking::Secret;
use time::PrimitiveDateTime;

use crate::{enums as storage_enums, schema::mandate};

#[derive(Clone, Debug, Identifiable, Queryable)]
#[diesel(table_name = mandate)]
pub struct Mandate {
    pub id: i32,
    pub mandate_id: String,
    pub customer_id: String,
    pub merchant_id: String,
    pub payment_method_id: String,
    pub mandate_status: storage_enums::MandateStatus,
    pub mandate_type: storage_enums::MandateType,
    pub customer_accepted_at: Option<PrimitiveDateTime>,
    pub customer_ip_address: Option<Secret<String, pii::IpAddress>>,
    pub customer_user_agent: Option<String>,
    pub network_transaction_id: Option<String>,
    pub previous_attempt_id: Option<String>,
    pub created_at: PrimitiveDateTime,
    pub mandate_amount: Option<i64>,
    pub mandate_currency: Option<storage_enums::Currency>,
    pub amount_captured: Option<i64>,
    pub connector: String,
    pub connector_mandate_id: Option<String>,
    pub start_date: Option<PrimitiveDateTime>,
    pub end_date: Option<PrimitiveDateTime>,
    pub metadata: Option<pii::SecretSerdeValue>,
    pub connector_mandate_ids: Option<pii::SecretSerdeValue>,
    pub original_payment_id: Option<String>,
    pub merchant_connector_id: Option<String>,
}

#[derive(
    router_derive::Setter, Clone, Debug, Default, Insertable, router_derive::DebugAsDisplay,
)]
#[diesel(table_name = mandate)]
pub struct MandateNew {
    pub mandate_id: String,
    pub customer_id: String,
    pub merchant_id: String,
    pub payment_method_id: String,
    pub mandate_status: storage_enums::MandateStatus,
    pub mandate_type: storage_enums::MandateType,
    pub customer_accepted_at: Option<PrimitiveDateTime>,
    pub customer_ip_address: Option<Secret<String, pii::IpAddress>>,
    pub customer_user_agent: Option<String>,
    pub network_transaction_id: Option<String>,
    pub previous_attempt_id: Option<String>,
    pub created_at: Option<PrimitiveDateTime>,
    pub mandate_amount: Option<i64>,
    pub mandate_currency: Option<storage_enums::Currency>,
    pub amount_captured: Option<i64>,
    pub connector: String,
    pub connector_mandate_id: Option<String>,
    pub start_date: Option<PrimitiveDateTime>,
    pub end_date: Option<PrimitiveDateTime>,
    pub metadata: Option<pii::SecretSerdeValue>,
    pub connector_mandate_ids: Option<pii::SecretSerdeValue>,
    pub original_payment_id: Option<String>,
    pub merchant_connector_id: Option<String>,
}

#[derive(Debug)]
pub enum MandateUpdate {
    StatusUpdate {
        mandate_status: storage_enums::MandateStatus,
    },
    CaptureAmountUpdate {
        amount_captured: Option<i64>,
    },
    ConnectorReferenceUpdate {
        connector_mandate_ids: Option<pii::SecretSerdeValue>,
    },
    ConnectorMandateIdUpdate {
        connector_mandate_id: Option<String>,
        connector_mandate_ids: Option<pii::SecretSerdeValue>,
        payment_method_id: String,
        original_payment_id: Option<String>,
    },
}

#[derive(Clone, Eq, PartialEq, Copy, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct SingleUseMandate {
    pub amount: i64,
    pub currency: storage_enums::Currency,
}

#[derive(Clone, Debug, Default, AsChangeset, router_derive::DebugAsDisplay)]
#[diesel(table_name = mandate)]
pub struct MandateUpdateInternal {
    mandate_status: Option<storage_enums::MandateStatus>,
    amount_captured: Option<i64>,
    connector_mandate_ids: Option<pii::SecretSerdeValue>,
    connector_mandate_id: Option<String>,
    payment_method_id: Option<String>,
    original_payment_id: Option<String>,
}

impl From<MandateUpdate> for MandateUpdateInternal {
        /// Converts a MandateUpdate enum into a struct, initializing the fields based on the variant of the enum.
    fn from(mandate_update: MandateUpdate) -> Self {
        match mandate_update {
            MandateUpdate::StatusUpdate { mandate_status } => Self {
                mandate_status: Some(mandate_status),
                connector_mandate_ids: None,
                amount_captured: None,
                connector_mandate_id: None,
                payment_method_id: None,
                original_payment_id: None,
            },
            MandateUpdate::CaptureAmountUpdate { amount_captured } => Self {
                mandate_status: None,
                amount_captured,
                connector_mandate_ids: None,
                connector_mandate_id: None,
                payment_method_id: None,
                original_payment_id: None,
            },
            MandateUpdate::ConnectorReferenceUpdate {
                connector_mandate_ids,
            } => Self {
                connector_mandate_ids,
                ..Default::default()
            },
            MandateUpdate::ConnectorMandateIdUpdate {
                connector_mandate_id,
                connector_mandate_ids,
                payment_method_id,
                original_payment_id,
            } => Self {
                connector_mandate_id,
                connector_mandate_ids,
                payment_method_id: Some(payment_method_id),
                original_payment_id,
                ..Default::default()
            },
        }
    }
}
