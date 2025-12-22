use std::collections::HashMap;

use common_enums::Currency;
use common_utils::types::MinorUnit;
use serde::{Deserialize, Serialize};
use strum::Display;
use time::PrimitiveDateTime;

use super::*;
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct FinixPaymentsResponse {
    pub id: String,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
    pub application: Option<Secret<String>>,
    pub amount: MinorUnit,
    pub captured_amount: Option<MinorUnit>,
    pub currency: Currency,
    pub is_void: Option<bool>,
    pub source: Option<Secret<String>>,
    pub state: FinixState,
    pub failure_code: Option<String>,
    pub messages: Option<Vec<String>>,
    pub failure_message: Option<String>,
    pub transfer: Option<String>,
    pub tags: FinixTags,
    #[serde(rename = "type")]
    pub payment_type: Option<FinixPaymentType>,
    // pub trace_id: String,
    pub three_d_secure: Option<FinixThreeDSecure>,
    // Add other fields from the API response as needed.
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum FinixCombinedPaymentResponse {
    SyncResponse(FinixPaymentsResponse),
    WebhookResponse(FinixEmbedded),
}
impl FinixCombinedPaymentResponse {
    pub fn get_payment_response(&self) -> Result<FinixPaymentsResponse, ConnectorError> {
        match self {
            Self::SyncResponse(txn_res) => Ok(txn_res.clone()),
            Self::WebhookResponse(FinixEmbedded::Authorizations { authorizations }) => {
                authorizations.get_first_event()
            }
            Self::WebhookResponse(FinixEmbedded::Transfers { transfers }) => {
                transfers.get_first_event()
            }
            Self::WebhookResponse(FinixEmbedded::Disputes { .. }) => {
                Err(ConnectorError::ResponseHandlingFailed)
            }
        }
    }
}
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct FinixIdentityResponse {
    pub id: String,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
    pub application: Option<String>,
    pub entity: Option<HashMap<String, serde_json::Value>>,
    pub tags: Option<FinixTags>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct FinixInstrumentResponse {
    pub id: String,
    pub created_at: String,
    pub updated_at: String,
    pub application: String,
    pub identity: Option<String>,
    #[serde(rename = "type")]
    pub instrument_type: FinixPaymentInstrumentType,
    pub tags: Option<FinixTags>,
    pub card_type: Option<FinixCardType>,
    pub card_brand: Option<String>,
    pub fingerprint: Option<String>,
    pub address: Option<FinixAddress>,
    pub name: Option<Secret<String>>,
    pub currency: Option<Currency>,
    pub enabled: bool,
}

#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
pub struct FinixErrorResponse {
    // pub status_code: u16,
    pub total: Option<i64>,
    #[serde(rename = "_embedded")]
    pub embedded: Option<FinixErrorEmbedded>,
}

#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
pub struct FinixErrorEmbedded {
    pub errors: Option<Vec<FinixError>>,
}

#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
pub struct FinixError {
    // pub logref: Option<String>,
    pub message: Option<String>,
    pub code: Option<String>,
}

//------------------- WEBHOOKS

#[derive(Clone, Display, Debug, Serialize, Deserialize)]
pub enum FinixDisputeState {
    INQUIRY,
    PENDING,
    LOST,
    WON,
}
#[derive(Clone, Debug, Serialize, Deserialize)]

pub struct FinixDisputes {
    pub transfer: String,
    pub reason: Option<String>,
    pub amount: MinorUnit,
    pub state: FinixDisputeState,
    pub currency: Currency,
    pub id: String,
    pub created_at: Option<PrimitiveDateTime>,
    pub updated_at: Option<PrimitiveDateTime>,
    pub respond_by: Option<PrimitiveDateTime>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum FinixWebhookEvent {}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SingleEventType<T>(Vec<T>);
impl<T: Clone> SingleEventType<T> {
    pub fn get_first_event(&self) -> Result<T, ConnectorError> {
        self.0
            .first()
            .cloned()
            .ok_or(ConnectorError::WebhookBodyDecodingFailed)
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum FinixEmbedded {
    Authorizations {
        authorizations: SingleEventType<FinixPaymentsResponse>,
    },
    Transfers {
        transfers: SingleEventType<FinixPaymentsResponse>,
    },
    Disputes {
        disputes: SingleEventType<FinixDisputes>,
    },
}
#[derive(Clone, Debug, Serialize, Deserialize)]

pub struct FinixWebhookBody {
    #[serde(rename = "type")]
    pub webhook_type: String,
    pub entity: String,
    #[serde(rename = "_embedded")]
    pub webhook_embedded: FinixEmbedded,
}

//--------------

#[derive(Debug, serde::Deserialize)]
pub struct FinixWebhookSignature {
    pub timestamp: String,
    pub sig: Vec<u8>,
}
