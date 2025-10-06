use common_enums::Currency;
use common_utils::{pii::Email, types::MinorUnit};
use masking::Secret;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::connectors::finix::transformers::finix_common::*;

/// Request structure for creating an authorization
/// API Reference: https://docs.finix.com/api/authorizations/createauthorization
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FinixPaymentsRequest {
    pub amount: MinorUnit,
    pub currency: Currency,
    pub source: Secret<String>,
    #[serde(rename = "type")]
    pub auth_type: Option<String>,
    pub merchant: Secret<String>,
    pub tags: Option<FinixTags>,
    pub three_d_secure: Option<FinixThreeDSecure>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FinixCaptureRequest {
    pub amount: MinorUnit,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FinixCancelRequest {
    pub void_me: bool,
}

impl FinixCancelRequest {
    pub fn new() -> Self {
        Self { void_me: true }
    }
}
/// Request structure for capturing an authorization.
/// API Reference: https://docs.finix.com/api/authorizations/captureauthorization
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FinixCaptureAuthorizationRequest {
    pub amount: Option<MinorUnit>,
    pub tags: Option<FinixTags>,
}

/// Request structure for creating an identity.
/// API Reference: https://docs.finix.com/api/identities/createidentity
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FinixCreateIdentityRequest {
    pub entity: FinixIdentityEntity,
    pub tags: Option<FinixTags>,
    #[serde(rename = "type")]
    pub identity_type: FinixIdentityType,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FinixIdentityEntity {
    pub phone: Option<Secret<String>>,
    pub first_name: Option<Secret<String>>,
    pub last_name: Option<Secret<String>>,
    pub email: Option<Email>, // Make email type
    pub personal_address: Option<FinixAddress>,
}
/// Request structure for creating a payment instrument.
/// API Reference: https://docs.finix.com/api/payment-instruments/createpaymentinstrument
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FinixCreatePaymentInstrumentRequest {
    #[serde(rename = "type")]
    pub instrument_type: FinixPaymentInstrumentType,
    pub name: Option<Secret<String>>,
    pub number: Option<Secret<String>>,
    pub security_code: Option<Secret<String>>,
    pub expiration_month: Option<Secret<i32>>,
    pub expiration_year: Option<Secret<i32>>,
    pub identity: String,
    pub tags: Option<FinixTags>,
    pub address: Option<FinixAddress>,
    pub card_brand: Option<FinixCardBrand>,
    pub card_type: Option<FinixCardType>,
    pub additional_data: Option<HashMap<String, String>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FinixCreateRefundRequest {
    pub linked_transfers: String,
    pub refund_amount: MinorUnit,
    pub currency: Currency,
    pub _type: String,
}

impl FinixCreateRefundRequest {
    pub fn new(parent_txn_id: String, amount: MinorUnit, currency: Currency) -> Self {
        Self {
            linked_transfers: parent_txn_id,
            refund_amount: amount,
            currency,
            _type: "REVERSAL".to_string(),
        }
    }
}
