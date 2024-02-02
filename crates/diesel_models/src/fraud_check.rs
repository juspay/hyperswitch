use diesel::{AsChangeset, Identifiable, Insertable, Queryable};
use masking::{Deserialize, Serialize};
use time::PrimitiveDateTime;

use crate::{
    enums::{FraudCheckLastStep, FraudCheckStatus, FraudCheckType},
    schema::fraud_check,
};
#[derive(Clone, Debug, Identifiable, Queryable, Serialize, Deserialize)]
#[diesel(table_name = fraud_check,  primary_key(payment_id, merchant_id))]
pub struct FraudCheck {
    pub frm_id: String,
    pub payment_id: String,
    pub merchant_id: String,
    pub attempt_id: String,
    pub created_at: PrimitiveDateTime,
    pub frm_name: String,
    pub frm_transaction_id: Option<String>,
    pub frm_transaction_type: FraudCheckType,
    pub frm_status: FraudCheckStatus,
    pub frm_score: Option<i32>,
    pub frm_reason: Option<serde_json::Value>,
    pub frm_error: Option<String>,
    pub payment_details: Option<serde_json::Value>,
    pub metadata: Option<serde_json::Value>,
    pub modified_at: PrimitiveDateTime,
    pub last_step: FraudCheckLastStep,
}

#[derive(router_derive::Setter, Clone, Debug, Insertable, router_derive::DebugAsDisplay)]
#[diesel(table_name = fraud_check)]
pub struct FraudCheckNew {
    pub frm_id: String,
    pub payment_id: String,
    pub merchant_id: String,
    pub attempt_id: String,
    pub created_at: PrimitiveDateTime,
    pub frm_name: String,
    pub frm_transaction_id: Option<String>,
    pub frm_transaction_type: FraudCheckType,
    pub frm_status: FraudCheckStatus,
    pub frm_score: Option<i32>,
    pub frm_reason: Option<serde_json::Value>,
    pub frm_error: Option<String>,
    pub payment_details: Option<serde_json::Value>,
    pub metadata: Option<serde_json::Value>,
    pub modified_at: PrimitiveDateTime,
    pub last_step: FraudCheckLastStep,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FraudCheckUpdate {
    //Refer PaymentAttemptUpdate for other variants implementations
    ResponseUpdate {
        frm_status: FraudCheckStatus,
        frm_transaction_id: Option<String>,
        frm_reason: Option<serde_json::Value>,
        frm_score: Option<i32>,
        metadata: Option<serde_json::Value>,
        modified_at: PrimitiveDateTime,
        last_step: FraudCheckLastStep,
    },
    ErrorUpdate {
        status: FraudCheckStatus,
        error_message: Option<Option<String>>,
    },
}

#[derive(Clone, Debug, Default, AsChangeset, router_derive::DebugAsDisplay)]
#[diesel(table_name = fraud_check)]
pub struct FraudCheckUpdateInternal {
    frm_status: Option<FraudCheckStatus>,
    frm_transaction_id: Option<String>,
    frm_reason: Option<serde_json::Value>,
    frm_score: Option<i32>,
    frm_error: Option<Option<String>>,
    metadata: Option<serde_json::Value>,
    last_step: FraudCheckLastStep,
}

impl From<FraudCheckUpdate> for FraudCheckUpdateInternal {
        /// Converts a FraudCheckUpdate enum into a struct of the same type.
    fn from(fraud_check_update: FraudCheckUpdate) -> Self {
        match fraud_check_update {
            FraudCheckUpdate::ResponseUpdate {
                frm_status,
                frm_transaction_id,
                frm_reason,
                frm_score,
                metadata,
                modified_at: _,
                last_step,
            } => Self {
                frm_status: Some(frm_status),
                frm_transaction_id,
                frm_reason,
                frm_score,
                metadata,
                last_step,
                ..Default::default()
            },
            FraudCheckUpdate::ErrorUpdate {
                status,
                error_message,
            } => Self {
                frm_status: Some(status),
                frm_error: error_message,
                ..Default::default()
            },
        }
    }
}
