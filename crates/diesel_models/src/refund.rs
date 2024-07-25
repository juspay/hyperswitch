use common_utils::{
    pii,
    types::{ChargeRefunds, MinorUnit},
};
use diesel::{AsChangeset, Identifiable, Insertable, Queryable, Selectable};
use serde::{Deserialize, Serialize};
use time::PrimitiveDateTime;

use crate::{enums as storage_enums, schema::refund};

#[derive(
    Clone,
    Debug,
    Eq,
    Identifiable,
    Queryable,
    Selectable,
    PartialEq,
    serde::Serialize,
    serde::Deserialize,
)]
#[diesel(table_name = refund, primary_key(refund_id), check_for_backend(diesel::pg::Pg))]
pub struct Refund {
    pub internal_reference_id: String,
    pub refund_id: String, //merchant_reference id
    pub payment_id: String,
    pub merchant_id: common_utils::id_type::MerchantId,
    pub connector_transaction_id: String,
    pub connector: String,
    pub connector_refund_id: Option<String>,
    pub external_reference_id: Option<String>,
    pub refund_type: storage_enums::RefundType,
    pub total_amount: MinorUnit,
    pub currency: storage_enums::Currency,
    pub refund_amount: MinorUnit,
    pub refund_status: storage_enums::RefundStatus,
    pub sent_to_gateway: bool,
    pub refund_error_message: Option<String>,
    pub metadata: Option<pii::SecretSerdeValue>,
    pub refund_arn: Option<String>,
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub created_at: PrimitiveDateTime,
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub modified_at: PrimitiveDateTime,
    pub description: Option<String>,
    pub attempt_id: String,
    pub refund_reason: Option<String>,
    pub refund_error_code: Option<String>,
    pub profile_id: Option<String>,
    pub updated_by: String,
    pub merchant_connector_id: Option<String>,
    pub charges: Option<ChargeRefunds>,
}

#[derive(
    Clone,
    Debug,
    Eq,
    PartialEq,
    Insertable,
    router_derive::DebugAsDisplay,
    serde::Serialize,
    serde::Deserialize,
    router_derive::Setter,
)]
#[diesel(table_name = refund)]
pub struct RefundNew {
    pub refund_id: String,
    pub payment_id: String,
    pub merchant_id: common_utils::id_type::MerchantId,
    pub internal_reference_id: String,
    pub external_reference_id: Option<String>,
    pub connector_transaction_id: String,
    pub connector: String,
    pub connector_refund_id: Option<String>,
    pub refund_type: storage_enums::RefundType,
    pub total_amount: MinorUnit,
    pub currency: storage_enums::Currency,
    pub refund_amount: MinorUnit,
    pub refund_status: storage_enums::RefundStatus,
    pub sent_to_gateway: bool,
    pub metadata: Option<pii::SecretSerdeValue>,
    pub refund_arn: Option<String>,
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub created_at: PrimitiveDateTime,
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub modified_at: PrimitiveDateTime,
    pub description: Option<String>,
    pub attempt_id: String,
    pub refund_reason: Option<String>,
    pub profile_id: Option<String>,
    pub updated_by: String,
    pub merchant_connector_id: Option<String>,
    pub charges: Option<ChargeRefunds>,
}

impl Default for RefundNew {
    fn default() -> Self {
        Self {
            refund_id: Default::default(),
            payment_id: Default::default(),
            merchant_id: Default::default(),
            internal_reference_id: Default::default(),
            external_reference_id: Default::default(),
            connector_transaction_id: Default::default(),
            connector: Default::default(),
            connector_refund_id: Default::default(),
            refund_type: Default::default(),
            total_amount: Default::default(),
            currency: Default::default(),
            refund_amount: Default::default(),
            refund_status: Default::default(),
            sent_to_gateway: Default::default(),
            metadata: Default::default(),
            refund_arn: Default::default(),
            created_at: common_utils::date_time::now(),
            modified_at: common_utils::date_time::now(),
            description: Default::default(),
            attempt_id: Default::default(),
            refund_reason: Default::default(),
            profile_id: Default::default(),
            updated_by: Default::default(),
            merchant_connector_id: Default::default(),
            charges: Default::default(),
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum RefundUpdate {
    Update {
        connector_refund_id: String,
        refund_status: storage_enums::RefundStatus,
        sent_to_gateway: bool,
        refund_error_message: Option<String>,
        refund_arn: String,
        updated_by: String,
    },
    MetadataAndReasonUpdate {
        metadata: Option<pii::SecretSerdeValue>,
        reason: Option<String>,
        updated_by: String,
    },
    StatusUpdate {
        connector_refund_id: Option<String>,
        sent_to_gateway: bool,
        refund_status: storage_enums::RefundStatus,
        updated_by: String,
    },
    ErrorUpdate {
        refund_status: Option<storage_enums::RefundStatus>,
        refund_error_message: Option<String>,
        refund_error_code: Option<String>,
        updated_by: String,
        connector_refund_id: Option<String>,
    },
    ManualUpdate {
        refund_status: Option<storage_enums::RefundStatus>,
        refund_error_message: Option<String>,
        refund_error_code: Option<String>,
        updated_by: String,
    },
}

#[derive(Clone, Debug, AsChangeset, router_derive::DebugAsDisplay)]
#[diesel(table_name = refund)]
pub struct RefundUpdateInternal {
    connector_refund_id: Option<String>,
    refund_status: Option<storage_enums::RefundStatus>,
    sent_to_gateway: Option<bool>,
    refund_error_message: Option<String>,
    refund_arn: Option<String>,
    metadata: Option<pii::SecretSerdeValue>,
    refund_reason: Option<String>,
    refund_error_code: Option<String>,
    updated_by: String,
    modified_at: PrimitiveDateTime,
}

impl RefundUpdateInternal {
    pub fn create_refund(self, source: Refund) -> Refund {
        Refund {
            connector_refund_id: self.connector_refund_id,
            refund_status: self.refund_status.unwrap_or_default(),
            sent_to_gateway: self.sent_to_gateway.unwrap_or_default(),
            refund_error_message: self.refund_error_message,
            refund_arn: self.refund_arn,
            metadata: self.metadata,
            refund_reason: self.refund_reason,
            refund_error_code: self.refund_error_code,
            updated_by: self.updated_by,
            modified_at: self.modified_at,
            ..source
        }
    }
}

impl From<RefundUpdate> for RefundUpdateInternal {
    fn from(refund_update: RefundUpdate) -> Self {
        match refund_update {
            RefundUpdate::Update {
                connector_refund_id,
                refund_status,
                sent_to_gateway,
                refund_error_message,
                refund_arn,
                updated_by,
            } => Self {
                connector_refund_id: Some(connector_refund_id),
                refund_status: Some(refund_status),
                sent_to_gateway: Some(sent_to_gateway),
                refund_error_message,
                refund_arn: Some(refund_arn),
                updated_by,
                metadata: None,
                refund_reason: None,
                refund_error_code: None,
                modified_at: common_utils::date_time::now(),
            },
            RefundUpdate::MetadataAndReasonUpdate {
                metadata,
                reason,
                updated_by,
            } => Self {
                metadata,
                refund_reason: reason,
                updated_by,
                connector_refund_id: None,
                refund_status: None,
                sent_to_gateway: None,
                refund_error_message: None,
                refund_arn: None,
                refund_error_code: None,
                modified_at: common_utils::date_time::now(),
            },
            RefundUpdate::StatusUpdate {
                connector_refund_id,
                sent_to_gateway,
                refund_status,
                updated_by,
            } => Self {
                connector_refund_id,
                sent_to_gateway: Some(sent_to_gateway),
                refund_status: Some(refund_status),
                updated_by,
                refund_error_message: None,
                refund_arn: None,
                metadata: None,
                refund_reason: None,
                refund_error_code: None,
                modified_at: common_utils::date_time::now(),
            },
            RefundUpdate::ErrorUpdate {
                refund_status,
                refund_error_message,
                refund_error_code,
                updated_by,
                connector_refund_id,
            } => Self {
                refund_status,
                refund_error_message,
                refund_error_code,
                updated_by,
                connector_refund_id,
                sent_to_gateway: None,
                refund_arn: None,
                metadata: None,
                refund_reason: None,
                modified_at: common_utils::date_time::now(),
            },
            RefundUpdate::ManualUpdate {
                refund_status,
                refund_error_message,
                refund_error_code,
                updated_by,
            } => Self {
                refund_status,
                refund_error_message,
                refund_error_code,
                updated_by,
                connector_refund_id: None,
                sent_to_gateway: None,
                refund_arn: None,
                metadata: None,
                refund_reason: None,
                modified_at: common_utils::date_time::now(),
            },
        }
    }
}

impl RefundUpdate {
    pub fn apply_changeset(self, source: Refund) -> Refund {
        let RefundUpdateInternal {
            connector_refund_id,
            refund_status,
            sent_to_gateway,
            refund_error_message,
            refund_arn,
            metadata,
            refund_reason,
            refund_error_code,
            updated_by,
            modified_at: _,
        } = self.into();
        Refund {
            connector_refund_id: connector_refund_id.or(source.connector_refund_id),
            refund_status: refund_status.unwrap_or(source.refund_status),
            sent_to_gateway: sent_to_gateway.unwrap_or(source.sent_to_gateway),
            refund_error_message: refund_error_message.or(source.refund_error_message),
            refund_error_code: refund_error_code.or(source.refund_error_code),
            refund_arn: refund_arn.or(source.refund_arn),
            metadata: metadata.or(source.metadata),
            refund_reason: refund_reason.or(source.refund_reason),
            updated_by,
            modified_at: common_utils::date_time::now(),
            ..source
        }
    }
}

#[derive(Debug, Eq, PartialEq, Deserialize, Serialize)]
pub struct RefundCoreWorkflow {
    pub refund_internal_reference_id: String,
    pub connector_transaction_id: String,
    pub merchant_id: common_utils::id_type::MerchantId,
    pub payment_id: String,
}

impl common_utils::events::ApiEventMetric for Refund {
    fn get_api_event_type(&self) -> Option<common_utils::events::ApiEventsType> {
        Some(common_utils::events::ApiEventsType::Refund {
            payment_id: Some(self.payment_id.clone()),
            refund_id: self.refund_id.clone(),
        })
    }
}

mod tests {
    #[test]
    fn test_backwards_compatibility() {
        let serialized_refund = r#"{
    "internal_reference_id": "internal_ref_123",
    "refund_id": "refund_456",
    "payment_id": "payment_789",
    "merchant_id": "merchant_123",
    "connector_transaction_id": "connector_txn_789",
    "connector": "stripe",
    "connector_refund_id": null,
    "external_reference_id": null,
    "refund_type": "instant_refund",
    "total_amount": 10000,
    "currency": "USD",
    "refund_amount": 9500,
    "refund_status": "Success",
    "sent_to_gateway": true,
    "refund_error_message": null,
    "metadata": null,
    "refund_arn": null,
    "created_at": "2024-02-26T12:00:00Z",
    "updated_at": "2024-02-26T12:00:00Z",
    "description": null,
    "attempt_id": "attempt_123",
    "refund_reason": null,
    "refund_error_code": null,
    "profile_id": null,
    "updated_by": "admin",
    "merchant_connector_id": null,
    "charges": null
}"#;
        let deserialized = serde_json::from_str::<super::Refund>(serialized_refund);

        assert!(deserialized.is_ok());
    }
}
