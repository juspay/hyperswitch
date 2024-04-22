use common_utils::pii;
use diesel::{AsChangeset, Identifiable, Insertable, Queryable};
use masking::Secret;
use time::PrimitiveDateTime;

use crate::{enums as storage_enums, schema::mandate};

#[derive(Clone, Debug, Identifiable, Queryable, serde::Serialize, serde::Deserialize)]
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
    router_derive::Setter,
    Clone,
    Debug,
    Default,
    Insertable,
    router_derive::DebugAsDisplay,
    serde::Serialize,
    serde::Deserialize,
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

#[derive(
    Clone,
    Debug,
    Default,
    AsChangeset,
    router_derive::DebugAsDisplay,
    serde::Serialize,
    serde::Deserialize,
)]
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

impl MandateUpdateInternal {
    pub fn apply_changeset(self, source: Mandate) -> Mandate {
        let Self {
            mandate_status,
            amount_captured,
            connector_mandate_ids,
            connector_mandate_id,
            payment_method_id,
            original_payment_id,
        } = self;

        Mandate {
            mandate_status: mandate_status.unwrap_or(source.mandate_status),
            amount_captured: amount_captured.map_or(source.amount_captured, Some),
            connector_mandate_ids: connector_mandate_ids.map_or(source.connector_mandate_ids, Some),
            connector_mandate_id: connector_mandate_id.map_or(source.connector_mandate_id, Some),
            payment_method_id: payment_method_id.unwrap_or(source.payment_method_id),
            original_payment_id: original_payment_id.map_or(source.original_payment_id, Some),
            ..source
        }
    }
}

impl From<&MandateNew> for Mandate {
    fn from(mandate_new: &MandateNew) -> Self {
        Self {
            id: 0i32,
            mandate_id: mandate_new.mandate_id.clone(),
            customer_id: mandate_new.customer_id.clone(),
            merchant_id: mandate_new.merchant_id.clone(),
            payment_method_id: mandate_new.payment_method_id.clone(),
            mandate_status: mandate_new.mandate_status,
            mandate_type: mandate_new.mandate_type,
            customer_accepted_at: mandate_new.customer_accepted_at,
            customer_ip_address: mandate_new.customer_ip_address.clone(),
            customer_user_agent: mandate_new.customer_user_agent.clone(),
            network_transaction_id: mandate_new.network_transaction_id.clone(),
            previous_attempt_id: mandate_new.previous_attempt_id.clone(),
            created_at: mandate_new
                .created_at
                .unwrap_or_else(common_utils::date_time::now),
            mandate_amount: mandate_new.mandate_amount,
            mandate_currency: mandate_new.mandate_currency,
            amount_captured: mandate_new.amount_captured,
            connector: mandate_new.connector.clone(),
            connector_mandate_id: mandate_new.connector_mandate_id.clone(),
            start_date: mandate_new.start_date,
            end_date: mandate_new.end_date,
            metadata: mandate_new.metadata.clone(),
            connector_mandate_ids: mandate_new.connector_mandate_ids.clone(),
            original_payment_id: mandate_new.original_payment_id.clone(),
            merchant_connector_id: mandate_new.merchant_connector_id.clone(),
        }
    }
}
