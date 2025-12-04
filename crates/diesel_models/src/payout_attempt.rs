use common_utils::{
    payout_method_utils, pii,
    types::{UnifiedCode, UnifiedMessage},
};
use diesel::{AsChangeset, Identifiable, Insertable, Queryable, Selectable};
use serde::{self, Deserialize, Serialize};
use time::PrimitiveDateTime;

use crate::{enums as storage_enums, schema::payout_attempt};

#[derive(
    Clone, Debug, Eq, PartialEq, Identifiable, Queryable, Selectable, Serialize, Deserialize,
)]
#[diesel(table_name = payout_attempt, primary_key(payout_attempt_id), check_for_backend(diesel::pg::Pg))]
pub struct PayoutAttempt {
    pub payout_attempt_id: String,
    pub payout_id: common_utils::id_type::PayoutId,
    pub customer_id: Option<common_utils::id_type::CustomerId>,
    pub merchant_id: common_utils::id_type::MerchantId,
    pub address_id: Option<String>,
    pub connector: Option<String>,
    pub connector_payout_id: Option<String>,
    pub payout_token: Option<String>,
    pub status: storage_enums::PayoutStatus,
    pub is_eligible: Option<bool>,
    pub error_message: Option<String>,
    pub error_code: Option<String>,
    pub business_country: Option<storage_enums::CountryAlpha2>,
    pub business_label: Option<String>,
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub created_at: PrimitiveDateTime,
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub last_modified_at: PrimitiveDateTime,
    pub profile_id: common_utils::id_type::ProfileId,
    pub merchant_connector_id: Option<common_utils::id_type::MerchantConnectorAccountId>,
    pub routing_info: Option<serde_json::Value>,
    pub unified_code: Option<UnifiedCode>,
    pub unified_message: Option<UnifiedMessage>,
    pub additional_payout_method_data: Option<payout_method_utils::AdditionalPayoutMethodData>,
    pub merchant_order_reference_id: Option<String>,
    pub payout_connector_metadata: Option<pii::SecretSerdeValue>,
}

#[derive(
    Clone,
    Debug,
    Eq,
    PartialEq,
    Insertable,
    serde::Serialize,
    serde::Deserialize,
    router_derive::DebugAsDisplay,
    router_derive::Setter,
)]
#[diesel(table_name = payout_attempt)]
pub struct PayoutAttemptNew {
    pub payout_attempt_id: String,
    pub payout_id: common_utils::id_type::PayoutId,
    pub customer_id: Option<common_utils::id_type::CustomerId>,
    pub merchant_id: common_utils::id_type::MerchantId,
    pub address_id: Option<String>,
    pub connector: Option<String>,
    pub connector_payout_id: Option<String>,
    pub payout_token: Option<String>,
    pub status: storage_enums::PayoutStatus,
    pub is_eligible: Option<bool>,
    pub error_message: Option<String>,
    pub error_code: Option<String>,
    pub business_country: Option<storage_enums::CountryAlpha2>,
    pub business_label: Option<String>,
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub created_at: PrimitiveDateTime,
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub last_modified_at: PrimitiveDateTime,
    pub profile_id: common_utils::id_type::ProfileId,
    pub merchant_connector_id: Option<common_utils::id_type::MerchantConnectorAccountId>,
    pub routing_info: Option<serde_json::Value>,
    pub unified_code: Option<UnifiedCode>,
    pub unified_message: Option<UnifiedMessage>,
    pub additional_payout_method_data: Option<payout_method_utils::AdditionalPayoutMethodData>,
    pub merchant_order_reference_id: Option<String>,
    pub payout_connector_metadata: Option<pii::SecretSerdeValue>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PayoutAttemptUpdate {
    StatusUpdate {
        connector_payout_id: Option<String>,
        status: storage_enums::PayoutStatus,
        error_message: Option<String>,
        error_code: Option<String>,
        is_eligible: Option<bool>,
        unified_code: Option<UnifiedCode>,
        unified_message: Option<UnifiedMessage>,
        payout_connector_metadata: Option<pii::SecretSerdeValue>,
    },
    PayoutTokenUpdate {
        payout_token: String,
    },
    BusinessUpdate {
        business_country: Option<storage_enums::CountryAlpha2>,
        business_label: Option<String>,
        address_id: Option<String>,
        customer_id: Option<common_utils::id_type::CustomerId>,
    },
    UpdateRouting {
        connector: String,
        routing_info: Option<serde_json::Value>,
        merchant_connector_id: Option<common_utils::id_type::MerchantConnectorAccountId>,
    },
    AdditionalPayoutMethodDataUpdate {
        additional_payout_method_data: Option<payout_method_utils::AdditionalPayoutMethodData>,
    },
}

#[derive(Clone, Debug, AsChangeset, router_derive::DebugAsDisplay)]
#[diesel(table_name = payout_attempt)]
pub struct PayoutAttemptUpdateInternal {
    pub payout_token: Option<String>,
    pub connector_payout_id: Option<String>,
    pub status: Option<storage_enums::PayoutStatus>,
    pub error_message: Option<String>,
    pub error_code: Option<String>,
    pub is_eligible: Option<bool>,
    pub business_country: Option<storage_enums::CountryAlpha2>,
    pub business_label: Option<String>,
    pub connector: Option<String>,
    pub routing_info: Option<serde_json::Value>,
    pub last_modified_at: PrimitiveDateTime,
    pub address_id: Option<String>,
    pub customer_id: Option<common_utils::id_type::CustomerId>,
    pub merchant_connector_id: Option<common_utils::id_type::MerchantConnectorAccountId>,
    pub unified_code: Option<UnifiedCode>,
    pub unified_message: Option<UnifiedMessage>,
    pub additional_payout_method_data: Option<payout_method_utils::AdditionalPayoutMethodData>,
    pub merchant_order_reference_id: Option<String>,
    pub payout_connector_metadata: Option<pii::SecretSerdeValue>,
}

impl Default for PayoutAttemptUpdateInternal {
    fn default() -> Self {
        Self {
            payout_token: None,
            connector_payout_id: None,
            status: None,
            error_message: None,
            error_code: None,
            is_eligible: None,
            business_country: None,
            business_label: None,
            connector: None,
            routing_info: None,
            merchant_connector_id: None,
            last_modified_at: common_utils::date_time::now(),
            address_id: None,
            customer_id: None,
            unified_code: None,
            unified_message: None,
            additional_payout_method_data: None,
            merchant_order_reference_id: None,
            payout_connector_metadata: None,
        }
    }
}

impl From<PayoutAttemptUpdate> for PayoutAttemptUpdateInternal {
    fn from(payout_update: PayoutAttemptUpdate) -> Self {
        match payout_update {
            PayoutAttemptUpdate::PayoutTokenUpdate { payout_token } => Self {
                payout_token: Some(payout_token),
                ..Default::default()
            },
            PayoutAttemptUpdate::StatusUpdate {
                connector_payout_id,
                status,
                error_message,
                error_code,
                is_eligible,
                unified_code,
                unified_message,
                payout_connector_metadata,
            } => Self {
                connector_payout_id,
                status: Some(status),
                error_message,
                error_code,
                is_eligible,
                unified_code,
                unified_message,
                payout_connector_metadata,
                ..Default::default()
            },
            PayoutAttemptUpdate::BusinessUpdate {
                business_country,
                business_label,
                address_id,
                customer_id,
            } => Self {
                business_country,
                business_label,
                address_id,
                customer_id,
                ..Default::default()
            },
            PayoutAttemptUpdate::UpdateRouting {
                connector,
                routing_info,
                merchant_connector_id,
            } => Self {
                connector: Some(connector),
                routing_info,
                merchant_connector_id,
                ..Default::default()
            },
            PayoutAttemptUpdate::AdditionalPayoutMethodDataUpdate {
                additional_payout_method_data,
            } => Self {
                additional_payout_method_data,
                ..Default::default()
            },
        }
    }
}

impl PayoutAttemptUpdate {
    pub fn apply_changeset(self, source: PayoutAttempt) -> PayoutAttempt {
        let PayoutAttemptUpdateInternal {
            payout_token,
            connector_payout_id,
            status,
            error_message,
            error_code,
            is_eligible,
            business_country,
            business_label,
            connector,
            routing_info,
            last_modified_at,
            address_id,
            customer_id,
            merchant_connector_id,
            unified_code,
            unified_message,
            additional_payout_method_data,
            merchant_order_reference_id,
            payout_connector_metadata,
        } = self.into();
        PayoutAttempt {
            payout_token: payout_token.or(source.payout_token),
            connector_payout_id: connector_payout_id.or(source.connector_payout_id),
            status: status.unwrap_or(source.status),
            error_message: error_message.or(source.error_message),
            error_code: error_code.or(source.error_code),
            is_eligible: is_eligible.or(source.is_eligible),
            business_country: business_country.or(source.business_country),
            business_label: business_label.or(source.business_label),
            connector: connector.or(source.connector),
            routing_info: routing_info.or(source.routing_info),
            last_modified_at,
            address_id: address_id.or(source.address_id),
            customer_id: customer_id.or(source.customer_id),
            merchant_connector_id: merchant_connector_id.or(source.merchant_connector_id),
            unified_code: unified_code.or(source.unified_code),
            unified_message: unified_message.or(source.unified_message),
            additional_payout_method_data: additional_payout_method_data
                .or(source.additional_payout_method_data),
            merchant_order_reference_id: merchant_order_reference_id
                .or(source.merchant_order_reference_id),
            payout_connector_metadata: payout_connector_metadata
                .or(source.payout_connector_metadata),
            ..source
        }
    }
}
