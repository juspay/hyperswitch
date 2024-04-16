use diesel::{AsChangeset, Identifiable, Insertable, Queryable};
use serde::{self, Deserialize, Serialize};
use time::PrimitiveDateTime;

use crate::{enums as storage_enums, schema::payout_attempt};

#[derive(Clone, Debug, Eq, PartialEq, Identifiable, Queryable, Serialize, Deserialize)]
#[diesel(table_name = payout_attempt)]
#[diesel(primary_key(payout_attempt_id))]
pub struct PayoutAttempt {
    pub payout_attempt_id: String,
    pub payout_id: String,
    pub customer_id: String,
    pub merchant_id: String,
    pub address_id: String,
    pub connector: Option<String>,
    pub connector_payout_id: String,
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
    pub profile_id: String,
    pub merchant_connector_id: Option<String>,
    pub routing_info: Option<serde_json::Value>,
}

#[derive(
    Clone,
    Debug,
    Default,
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
    pub payout_id: String,
    pub customer_id: String,
    pub merchant_id: String,
    pub address_id: String,
    pub connector: Option<String>,
    pub connector_payout_id: String,
    pub payout_token: Option<String>,
    pub status: storage_enums::PayoutStatus,
    pub is_eligible: Option<bool>,
    pub error_message: Option<String>,
    pub error_code: Option<String>,
    pub business_country: Option<storage_enums::CountryAlpha2>,
    pub business_label: Option<String>,
    #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
    pub created_at: Option<PrimitiveDateTime>,
    #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
    pub last_modified_at: Option<PrimitiveDateTime>,
    pub profile_id: String,
    pub merchant_connector_id: Option<String>,
    pub routing_info: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PayoutAttemptUpdate {
    StatusUpdate {
        connector_payout_id: String,
        status: storage_enums::PayoutStatus,
        error_message: Option<String>,
        error_code: Option<String>,
        is_eligible: Option<bool>,
    },
    PayoutTokenUpdate {
        payout_token: String,
    },
    BusinessUpdate {
        business_country: Option<storage_enums::CountryAlpha2>,
        business_label: Option<String>,
    },
    UpdateRouting {
        connector: String,
        routing_info: Option<serde_json::Value>,
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
            last_modified_at: common_utils::date_time::now(),
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
            } => Self {
                connector_payout_id: Some(connector_payout_id),
                status: Some(status),
                error_message,
                error_code,
                is_eligible,
                ..Default::default()
            },
            PayoutAttemptUpdate::BusinessUpdate {
                business_country,
                business_label,
            } => Self {
                business_country,
                business_label,
                ..Default::default()
            },
            PayoutAttemptUpdate::UpdateRouting {
                connector,
                routing_info,
            } => Self {
                connector: Some(connector),
                routing_info,
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
        } = self.into();
        PayoutAttempt {
            payout_token: payout_token.or(source.payout_token),
            connector_payout_id: connector_payout_id.unwrap_or(source.connector_payout_id),
            status: status.unwrap_or(source.status),
            error_message: error_message.or(source.error_message),
            error_code: error_code.or(source.error_code),
            is_eligible: is_eligible.or(source.is_eligible),
            business_country: business_country.or(source.business_country),
            business_label: business_label.or(source.business_label),
            connector: connector.or(source.connector),
            routing_info: routing_info.or(source.routing_info),
            last_modified_at,
            ..source
        }
    }
}
