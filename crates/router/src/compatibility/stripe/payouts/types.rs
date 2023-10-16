use std::{convert::From, default::Default};

use api_models::enums;
use common_utils::pii;
use serde::Serialize;

use crate::types::api::payouts;

#[derive(Serialize, Debug)]
pub struct StripePayoutResponse {
    pub id: String,
    pub amount: i64,
    pub arrival_date: Option<i32>,
    pub balance_transaction: String,
    pub created: Option<i64>,
    pub currency: String,
    pub description: Option<String>,
    pub destination: String,
    pub failure_balance_transaction: Option<String>,
    pub failure_code: Option<String>,
    pub failure_message: Option<String>,
    pub livemode: bool,
    pub metadata: pii::SecretSerdeValue,
    pub method: StripePayoutMethod,
    pub original_payout: Option<String>,
    pub reconciliation_status: StripePayoutReconStatus,
    pub reversed_by: Option<String>,
    pub source_type: StripePayoutSource,
    pub statement_descriptor: Option<String>,
    pub status: StripePayoutStatus,
    #[serde(rename = "type")]
    pub account_type: StripeExternalAccountType,
}

#[derive(Serialize, Debug, Default)]
#[serde(rename_all = "snake_case")]
pub enum StripePayoutSource {
    #[default]
    BankAccount,
    Card,
    Fpx,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "lowercase")]
pub enum StripePayoutMethod {
    Standard,
    Instant,
}

#[derive(Serialize, Debug, Default)]
#[serde(rename_all = "snake_case")]
pub enum StripePayoutReconStatus {
    Completed,
    #[default]
    InProgress,
    NotApplicable,
}

#[derive(Serialize, Debug)]
pub enum StripePayoutStatus {
    #[serde(rename = "payout.canceled")]
    Cancelled,
    #[serde(rename = "payout.created")]
    Created,
    #[serde(rename = "payout.failed")]
    Failed,
    #[serde(rename = "payout.paid")]
    Paid,
    #[serde(rename = "payout.reconciliation_completed")]
    ReconCompleted,
    #[serde(rename = "payout.updated")]
    Updated,
}

#[derive(Serialize, Debug)]
pub enum StripeExternalAccountType {
    BankAccount,
    Card,
}

impl From<enums::PayoutType> for StripeExternalAccountType {
    fn from(payout_type: enums::PayoutType) -> Self {
        match payout_type {
            enums::PayoutType::Bank => Self::BankAccount,
            enums::PayoutType::Card => Self::Card,
        }
    }
}

impl From<enums::PayoutType> for StripePayoutMethod {
    fn from(payout_type: enums::PayoutType) -> Self {
        match payout_type {
            enums::PayoutType::Bank => Self::Standard,
            enums::PayoutType::Card => Self::Instant,
        }
    }
}

impl From<enums::PayoutStatus> for StripePayoutStatus {
    fn from(payout_type: enums::PayoutStatus) -> Self {
        match payout_type {
            enums::PayoutStatus::Success => Self::Paid,
            enums::PayoutStatus::Failed => Self::Failed,
            enums::PayoutStatus::Cancelled => Self::Cancelled,
            enums::PayoutStatus::Pending => Self::Paid,
            enums::PayoutStatus::Ineligible => Self::Failed,
            enums::PayoutStatus::RequiresCreation => Self::Created,
            enums::PayoutStatus::RequiresPayoutMethodData => Self::Created,
            enums::PayoutStatus::RequiresFulfillment => Self::Created,
        }
    }
}

impl From<payouts::PayoutCreateResponse> for StripePayoutResponse {
    fn from(payout: payouts::PayoutCreateResponse) -> Self {
        Self {
            id: payout.payout_id,
            amount: payout.amount,
            currency: payout.currency.to_ascii_lowercase(),
            status: payout.status.into(),
            created: payout.created.map(|t| t.assume_utc().unix_timestamp()),
            metadata: payout
                .metadata
                .unwrap_or_else(|| masking::Secret::new(serde_json::json!({}))),
            arrival_date: None,
            balance_transaction: "".to_string(),
            description: payout.description,
            destination: payout.customer_id,
            failure_balance_transaction: None,
            failure_code: payout.error_code,
            failure_message: payout.error_message,
            livemode: false,
            method: payout.payout_type.into(),
            original_payout: None,
            reconciliation_status: StripePayoutReconStatus::default(),
            reversed_by: None,
            source_type: StripePayoutSource::default(),
            statement_descriptor: None,
            account_type: payout.payout_type.into(),
        }
    }
}
