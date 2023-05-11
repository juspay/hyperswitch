use diesel::{AsChangeset, Identifiable, Insertable, Queryable};
use serde::{self, Deserialize, Serialize};

use crate::{enums as storage_enums, schema::payouts};

// Payouts
#[derive(Clone, Debug, Default, Eq, PartialEq, Identifiable, Queryable, Serialize, Deserialize)]
#[diesel(table_name = payouts)]
pub struct Payouts {
    pub id: i32,
    pub payout_id: String,
    pub customer_id: String,
    pub merchant_id: String,
    pub address_id: String,
    pub connector: String,
    pub connector_payout_id: String,
    pub payout_method_data: Option<serde_json::Value>,
    pub status: storage_enums::PayoutStatus,
    pub is_eligible: Option<bool>,
    pub encoded_data: Option<String>,
    pub error_message: Option<String>,
    pub error_code: Option<String>,
}

#[derive(
    Clone,
    Debug,
    Default,
    Eq,
    PartialEq,
    Insertable,
    Queryable,
    router_derive::DebugAsDisplay,
    router_derive::Setter,
)]
#[diesel(table_name = payouts)]
pub struct PayoutsNew {
    pub payout_id: String,
    pub customer_id: String,
    pub merchant_id: String,
    pub address_id: String,
    pub connector: String,
    pub connector_payout_id: String,
    pub payout_method_data: Option<serde_json::Value>,
    pub status: storage_enums::PayoutStatus,
    pub is_eligible: Option<bool>,
    pub encoded_data: Option<String>,
    pub error_message: Option<String>,
    pub error_code: Option<String>,
}

#[derive(Debug)]
pub enum PayoutsUpdate {
    StatusUpdate {
        connector_payout_id: String,
        status: storage_enums::PayoutStatus,
        error_message: Option<String>,
        error_code: Option<String>,
        is_eligible: Option<bool>
    },
}

#[derive(Clone, Debug, Default, AsChangeset, router_derive::DebugAsDisplay)]
#[diesel(table_name = payouts)]
pub struct PayoutsUpdateInternal {
    pub connector_payout_id: String,
    pub status: Option<storage_enums::PayoutStatus>,
    pub error_message: Option<String>,
    pub error_code: Option<String>,
    is_eligible: Option<bool>
}

impl From<PayoutsUpdate> for PayoutsUpdateInternal {
    fn from(payout_update: PayoutsUpdate) -> Self {
        match payout_update {
            PayoutsUpdate::StatusUpdate {
                connector_payout_id,
                status,
                error_message,
                error_code,
                is_eligible,
            } => Self {
                connector_payout_id,
                status: Some(status),
                error_message,
                error_code,
                is_eligible,
            },
        }
    }
}
