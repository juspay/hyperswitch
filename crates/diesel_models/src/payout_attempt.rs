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
    pub straight_through_algorithm: Option<serde_json::Value>,
}

impl Default for PayoutAttempt {
    fn default() -> Self {
        let now = common_utils::date_time::now();

        Self {
            payout_attempt_id: String::default(),
            payout_id: String::default(),
            customer_id: String::default(),
            merchant_id: String::default(),
            address_id: String::default(),
            connector: None,
            connector_payout_id: String::default(),
            payout_token: None,
            status: storage_enums::PayoutStatus::default(),
            is_eligible: Some(true),
            error_message: None,
            error_code: None,
            business_country: Some(storage_enums::CountryAlpha2::default()),
            business_label: None,
            created_at: now,
            last_modified_at: now,
            profile_id: String::default(),
            merchant_connector_id: None,
            straight_through_algorithm: None,
        }
    }
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
    pub profile_id: Option<String>,
    pub merchant_connector_id: Option<String>,
    pub straight_through_algorithm: Option<serde_json::Value>,
}

#[derive(Debug)]
pub enum PayoutAttemptUpdate {
    StatusUpdate {
        connector_payout_id: String,
        status: storage_enums::PayoutStatus,
        error_message: Option<String>,
        error_code: Option<String>,
        is_eligible: Option<bool>,
        last_modified_at: Option<PrimitiveDateTime>,
    },
    PayoutTokenUpdate {
        payout_token: String,
        last_modified_at: Option<PrimitiveDateTime>,
    },
    BusinessUpdate {
        business_country: Option<storage_enums::CountryAlpha2>,
        business_label: Option<String>,
        last_modified_at: Option<PrimitiveDateTime>,
    },
    UpdateRouting {
        connector: String,
        straight_through_algorithm: serde_json::Value,
    },
}

#[derive(Clone, Debug, Default, AsChangeset, router_derive::DebugAsDisplay)]
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
    pub last_modified_at: Option<PrimitiveDateTime>,
    pub connector: Option<String>,
    pub straight_through_algorithm: Option<serde_json::Value>,
}

impl From<PayoutAttemptUpdate> for PayoutAttemptUpdateInternal {
    fn from(payout_update: PayoutAttemptUpdate) -> Self {
        match payout_update {
            PayoutAttemptUpdate::PayoutTokenUpdate {
                last_modified_at,
                payout_token,
            } => Self {
                last_modified_at,
                payout_token: Some(payout_token),
                ..Default::default()
            },
            PayoutAttemptUpdate::StatusUpdate {
                connector_payout_id,
                status,
                error_message,
                error_code,
                is_eligible,
                last_modified_at,
            } => Self {
                connector_payout_id: Some(connector_payout_id),
                status: Some(status),
                error_message,
                error_code,
                is_eligible,
                last_modified_at,
                ..Default::default()
            },
            PayoutAttemptUpdate::BusinessUpdate {
                business_country,
                business_label,
                last_modified_at,
            } => Self {
                business_country,
                business_label,
                last_modified_at,
                ..Default::default()
            },
            PayoutAttemptUpdate::UpdateRouting {
                connector,
                straight_through_algorithm,
            } => Self {
                connector: Some(connector),
                straight_through_algorithm: Some(straight_through_algorithm),
                ..Default::default()
            },
        }
    }
}
