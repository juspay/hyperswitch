use common_enums::enums::{self, AttemptStatus, MandateStatus};
use common_utils::{
    errors::CustomResult,
    ext_traits::{Encode, ValueExt},
    types::MinorUnit,
};
use error_stack::{report, ResultExt};
use hyperswitch_domain_models::{
    payment_method_data::{Card, PaymentMethodData},
    router_data::{ConnectorAuthType, MandateReference, RouterData},
    router_flow_types::payments::PaymentMethodToken,
    router_request_types::{PaymentMethodTokenizationData, PaymentsAuthorizeData},
    types::{PaymentsAuthorizeRouterData, RefundsRouterData},
};
use hyperswitch_interfaces::errors;
use masking::{ExposeInterface, Secret};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::utils::{self, ForeignTryFrom};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PaymentwallAuthType {
    pub public_key: Secret<String>,
    pub private_key: Secret<String>,
}

impl TryFrom<&ConnectorAuthType> for PaymentwallAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            ConnectorAuthType::SignatureKey {
                api_key,
                key1,
                api_secret: _,
                key2: _,
            } => Ok(Self {
                public_key: api_key.to_owned(),
                private_key: key1.to_owned(),
            }),
            _ => Err(errors::ConnectorError::FailedToObtainAuthType.into()),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PaymentwallErrorResponse {
    pub error: PaymentwallError,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PaymentwallError {
    pub code: Option<String>,
    pub message: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PaymentwallTokenRequest {
    pub public_key: Secret<String>,
    pub card_number: Secret<String>,
    pub card_expiration_month: Secret<String>,
    pub card_expiration_year: Secret<String>,
    pub card_cvv: Secret<String>,
}

impl TryFrom<&RouterData<PaymentMethodToken, PaymentMethodTokenizationData, PaymentsResponseData>>
    for PaymentwallTokenRequest
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &RouterData<PaymentMethodToken, PaymentMethodTokenizationData, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        match item.request.payment_method_data.clone() {
            PaymentMethodData::Card(card_data) => {
                let auth_type = PaymentwallAuthType::try_from(&item.connector_auth_type)?;
                Ok(Self {
                    public_key: auth_type.public_key,
                    card_number: card_data.card_number,
                    card_expiration_month: card_data.card_exp_month,
                    card_expiration_year: card_data.card_exp_year,
                    card_cvv: card_data.card_cvc,
                })
            }
            _ => Err(errors::ConnectorError::NotImplemented(
                "Payment Method".to_string(),
            )
            .into()),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PaymentwallTokenResponse {
    pub type_: String,
    pub token: String,
    pub fingerprint: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PaymentwallChargeRequest {
    pub token: String,
    pub fingerprint: Option<String>,
    pub email: String,
    pub currency: String,
    pub amount: String,
    pub description: String,
    #[serde(rename = "options[capture]")]
    pub options_capture: Option<String>,
}

impl TryFrom<&PaymentsAuthorizeRouterData> for PaymentwallChargeRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &PaymentsAuthorizeRouterData) -> Result<Self, Self::Error> {
        let amount = utils::to_currency_minor_unit(
            item.request.amount,
            item.request.currency,
        )?;
        
        let token = match &item.request.connector_meta {
            Some(meta) => {
                let meta_map: HashMap<String, String> = meta
                    .parse_value("PaymentwallMeta")
                    .change_context(errors::ConnectorError::RequestEncodingFailed)?;
                meta_map
                    .get("token")
                    .ok_or(errors::ConnectorError::RequestEncodingFailed)?
                    .to_string()
            }
            None => Err(errors::ConnectorError::RequestEncodingFailed)?,
        };
        
        let fingerprint = match &item.request.connector_meta {
            Some(meta) => {
                let meta_map: HashMap<String, String> = meta
                    .parse_value("PaymentwallMeta")
                    .change_context(errors::ConnectorError::RequestEncodingFailed)?;
                meta_map.get("fingerprint").map(|f| f.to_string())
            }
            None => None,
        };
        
        let email = item.request.get_email()?;
        let options_capture = if item.request.capture_method == Some(enums::CaptureMethod::Manual) {
            Some("0".to_string())
        } else {
            None
        };
        
        Ok(Self {
            token,
            fingerprint,
            email,
            currency: item.request.currency.to_string(),
            amount,
            description: item.request.description.clone().unwrap_or_else(|| "Payment".to_string()),
            options_capture,
        })
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PaymentwallChargeResponse {
    pub id: String,
    pub object: String,
    pub created: i64,
    pub amount: String,
    pub currency: String,
    pub status: String,
    pub captured: bool,
    pub refunded: bool,
    pub description: String,
    pub card: PaymentwallCard,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PaymentwallCard {
    pub id: String,
    pub object: String,
    pub first6: String,
    pub last4: String,
    pub fingerprint: String,
    pub expiry_month: String,
    pub expiry_year: String,
    pub token: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PaymentwallRefundRequest {
    pub amount: String,
    pub reason: Option<String>,
}

impl TryFrom<&RefundsRouterData<api::Execute>> for PaymentwallRefundRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &RefundsRouterData<api::Execute>) -> Result<Self, Self::Error> {
        let amount = utils::to_currency_minor_unit(
            item.request.refund_amount,
            item.request.currency,
        )?;
        
        Ok(Self {
            amount,
            reason: item.request.reason.clone(),
        })
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PaymentwallRefundResponse {
    pub refund: PaymentwallRefund,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PaymentwallRefund {
    pub id: String,
    pub object: String,
    pub created: i64,
    pub amount: String,
    pub currency: String,
    pub status: String,
    pub reason: Option<String>,
    pub charge: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PaymentwallRouterData {
    pub status: enums::AttemptStatus,
    pub connector_transaction_id: String,
    pub connector_metadata: Option<serde_json::Value>,
    pub amount_captured: Option<MinorUnit>,
    pub connector_mandate_id: Option<String>,
    pub mandate_reference: Option<MandateReference>,
}

impl TryFrom<PaymentwallTokenResponse> for PaymentwallRouterData {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: PaymentwallTokenResponse) -> Result<Self, Self::Error> {
        let connector_meta = serde_json::json!({
            "token": item.token,
            "fingerprint": item.fingerprint,
        });
        
        Ok(Self {
            status: AttemptStatus::Pending,
            connector_transaction_id: String::new(),
            connector_metadata: Some(connector_meta),
            amount_captured: None,
            connector_mandate_id: None,
            mandate_reference: None,
        })
    }
}

impl TryFrom<(PaymentwallChargeResponse, bool)> for PaymentwallRouterData {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        (item, is_manual_capture): (PaymentwallChargeResponse, bool),
    ) -> Result<Self, Self::Error> {
        let status = match item.status.as_str() {
            "captured" => AttemptStatus::Charged,
            "authorized" => AttemptStatus::Authorized,
            "voided" => AttemptStatus::Voided,
            "failed" => AttemptStatus::Failure,
            "pending" => AttemptStatus::Pending,
            _ => AttemptStatus::Pending,
        };
        
        let amount_captured = if item.captured {
            Some(MinorUnit::new(item.amount.parse::<i64>().unwrap_or(0)))
        } else {
            None
        };
        
        let connector_mandate_id = Some(item.card.token.clone());
        let mandate_reference = Some(MandateReference {
            connector_mandate_id: Some(item.card.token.clone()),
            payment_method_id: Some(item.card.id.clone()),
            customer_id: None,
            mandate_status: MandateStatus::Active,
        });
        
        Ok(Self {
            status,
            connector_transaction_id: item.id,
            connector_metadata: None,
            amount_captured,
            connector_mandate_id,
            mandate_reference,
        })
    }
}

impl From<String> for enums::RefundStatus {
    fn from(status: String) -> Self {
        match status.as_str() {
            "succeeded" => Self::Success,
            "failed" => Self::Failure,
            "pending" => Self::Pending,
            _ => Self::Pending,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PaymentsResponseData {
    pub status: AttemptStatus,
    pub connector_transaction_id: String,
    pub connector_metadata: Option<serde_json::Value>,
    pub amount_captured: Option<MinorUnit>,
    pub connector_mandate_id: Option<String>,
    pub mandate_reference: Option<MandateReference>,
}

impl From<PaymentwallRouterData> for PaymentsResponseData {
    fn from(item: PaymentwallRouterData) -> Self {
        Self {
            status: item.status,
            connector_transaction_id: item.connector_transaction_id,
            connector_metadata: item.connector_metadata,
            amount_captured: item.amount_captured,
            connector_mandate_id: item.connector_mandate_id,
            mandate_reference: item.mandate_reference,
        }
    }
}