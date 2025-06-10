use common_enums::enums;
use common_utils::types::StringMinorUnit;
use hyperswitch_domain_models::{
    payment_method_data::PaymentMethodData,
    router_data::{ConnectorAuthType, RouterData},
    router_flow_types::refunds::{Execute, RSync},
    router_request_types::{PaymentsAuthorizeData, ResponseId},
    router_response_types::{PaymentsResponseData, RefundsResponseData},
    types::{RefundsRouterData},
};
use hyperswitch_interfaces::errors;
use masking::{ExposeInterface, Secret};
use serde::{Deserialize, Serialize};

use crate::{
    types::{RefundsResponseRouterData, ResponseRouterData},
    utils::{PaymentsAuthorizeRequestData, RouterData as _},
};

//TODO: Fill the struct with respective fields
pub struct PaysafeRouterData<T> {
    pub amount: StringMinorUnit, // The type of amount that a connector accepts, for example, String, i64, f64, etc.
    pub router_data: T,
}

impl<T> From<(StringMinorUnit, T)> for PaysafeRouterData<T> {
    fn from((amount, item): (StringMinorUnit, T)) -> Self {
        //Todo :  use utils to convert the amount to the type of amount that a connector accepts
        Self {
            amount,
            router_data: item,
        }
    }
}

// Payment Handle Structures
#[derive(Default, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PaysafePaymentHandleRequest {
    pub merchant_ref_num: String,
    pub transaction_type: String,
    pub payment_type: String,
    pub amount: i64,
    pub currency_code: String,
    pub card: PaysafeCardData,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub billing_details: Option<PaysafeBillingDetails>,
}

#[derive(Default, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PaysafeCardData {
    pub card_num: cards::CardNumber,
    pub card_expiry: PaysafeCardExpiry,
    pub cvv: Secret<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub holder_name: Option<Secret<String>>,
}

#[derive(Default, Debug, Serialize)]
pub struct PaysafeCardExpiry {
    pub month: Secret<String>,
    pub year: Secret<String>,
}

#[derive(Default, Debug, Serialize)]
pub struct PaysafeBillingDetails {
    pub street: Option<String>,
    pub city: Option<String>,
    pub state: Option<String>,
    pub country: Option<String>,
    pub zip: Option<String>,
}

#[derive(Default, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PaysafePaymentHandleResponse {
    pub id: String,
    pub payment_handle_token: String,
    pub status: String,
    pub action: Option<String>,
    pub merchant_ref_num: String,
    pub payment_type: String,
}

// Payment Process Structures
#[derive(Default, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PaysafePaymentsRequest {
    pub merchant_ref_num: String,
    pub amount: i64,
    pub currency_code: String,
    pub dup_check: bool,
    pub settle_with_auth: bool,
    pub payment_handle_token: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

// Legacy card structure (used only during transitions)
#[derive(Default, Debug, Serialize, Eq, PartialEq)]
pub struct PaysafeCard {
    number: cards::CardNumber,
    expiry_month: Secret<String>,
    expiry_year: Secret<String>,
    cvc: Secret<String>,
    complete: bool,
}

impl<F> TryFrom<&PaysafeRouterData<&RouterData<F, PaymentsAuthorizeData, PaymentsResponseData>>> for PaysafePaymentHandleRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &PaysafeRouterData<&RouterData<F, PaymentsAuthorizeData, PaymentsResponseData>>,
    ) -> Result<Self, Self::Error> {
        match item.router_data.request.payment_method_data.clone() {
            PaymentMethodData::Card(req_card) => {
                let card_expiry = PaysafeCardExpiry {
                    month: req_card.card_exp_month,
                    year: req_card.card_exp_year,
                };
                
                let card = PaysafeCardData {
                    card_num: req_card.card_number,
                    card_expiry,
                    cvv: req_card.card_cvc,
                    holder_name: req_card.card_holder_name,
                };
                
                let billing_details = match item.router_data.get_billing_address() {
                    Ok(billing) => Some(PaysafeBillingDetails {
                        street: billing.line1.clone().map(|line| line.expose()),
                        city: billing.city.clone(),
                        state: billing.state.clone().map(|state| state.expose()),
                        country: billing.country.clone().map(|country| country.to_string()),
                        zip: billing.zip.clone().map(|zip| zip.expose()),
                    }),
                    Err(_) => None,
                };
                
                Ok(Self {
                    merchant_ref_num: item.router_data.payment_id.clone(),
                    transaction_type: "PAYMENT".to_string(),
                    payment_type: "CARD".to_string(),
                    amount: item.router_data.request.minor_amount.get_amount_as_i64(),
                    currency_code: item.router_data.request.currency.to_string(),
                    card,
                    billing_details,
                })
            }
            _ => Err(errors::ConnectorError::NotImplemented("Payment method".to_string()).into()),
        }
    }
}

impl<F> TryFrom<(&PaysafePaymentHandleResponse, &PaysafeRouterData<&RouterData<F, PaymentsAuthorizeData, PaymentsResponseData>>)> for PaysafePaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        (payment_handle, item): (&PaysafePaymentHandleResponse, &PaysafeRouterData<&RouterData<F, PaymentsAuthorizeData, PaymentsResponseData>>),
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            merchant_ref_num: item.router_data.payment_id.clone(),
            amount: item.router_data.request.minor_amount.get_amount_as_i64(),
            currency_code: item.router_data.request.currency.to_string(),
            dup_check: false,
            settle_with_auth: item.router_data.request.is_auto_capture()?,
            payment_handle_token: payment_handle.payment_handle_token.clone(),
            description: Some("Payment Authorization".to_string()),
        })
    }
}

//TODO: Fill the struct with respective fields
// Auth Struct
pub struct PaysafeAuthType {
    pub(super) api_key: Secret<String>,
    pub(super) api_password: Secret<String>,
}

impl TryFrom<&ConnectorAuthType> for PaysafeAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            ConnectorAuthType::BodyKey { api_key, key1 } => Ok(Self {
                api_key: api_key.to_owned(),
                api_password: key1.to_owned(),
            }),
            _ => Err(errors::ConnectorError::FailedToObtainAuthType.into()),
        }
    }
}
// PaymentsResponse
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "UPPERCASE")]
pub enum PaysafePaymentStatus {
    #[default]
    #[serde(rename = "PENDING")]
    Processing,
    #[serde(rename = "COMPLETED")]
    Succeeded,
    #[serde(rename = "FAILED")]
    Failed,
}

impl From<PaysafePaymentStatus> for common_enums::AttemptStatus {
    fn from(item: PaysafePaymentStatus) -> Self {
        match item {
            PaysafePaymentStatus::Succeeded => Self::Charged,
            PaysafePaymentStatus::Failed => Self::Failure,
            PaysafePaymentStatus::Processing => Self::Pending,
        }
    }
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PaysafePaymentsResponse {
    pub id: String,
    pub status: PaysafePaymentStatus,
    pub payment_type: String,
    pub payment_handle_token: Option<String>,
    pub merchant_ref_num: String,
    pub currency_code: String,
    #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
    pub txn_time: Option<time::PrimitiveDateTime>,
    pub amount: i64,
    pub available_to_settle: Option<i64>,
}

impl<F, T> TryFrom<ResponseRouterData<F, PaysafePaymentsResponse, T, PaymentsResponseData>>
    for RouterData<F, T, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, PaysafePaymentsResponse, T, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            status: common_enums::AttemptStatus::from(item.response.status),
            response: Ok(PaymentsResponseData::TransactionResponse {
                resource_id: ResponseId::ConnectorTransactionId(item.response.id),
                redirection_data: Box::new(None),
                mandate_reference: Box::new(None),
                connector_metadata: None,
                network_txn_id: None,
                connector_response_reference_id: None,
                incremental_authorization_allowed: None,
                charges: None,
            }),
            ..item.data
        })
    }
}

//TODO: Fill the struct with respective fields
// REFUND :
// Type definition for RefundRequest
#[derive(Default, Debug, Serialize)]
pub struct PaysafeRefundRequest {
    pub amount: StringMinorUnit,
}

impl<F> TryFrom<&PaysafeRouterData<&RefundsRouterData<F>>> for PaysafeRefundRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &PaysafeRouterData<&RefundsRouterData<F>>) -> Result<Self, Self::Error> {
        Ok(Self {
            amount: item.amount.to_owned(),
        })
    }
}

// Type definition for Refund Response

#[derive(Debug, Serialize, Default, Deserialize, Clone)]
#[serde(rename_all = "UPPERCASE")]
pub enum RefundStatus {
    #[default]
    #[serde(rename = "PENDING")]
    Processing,
    #[serde(rename = "COMPLETED")]
    Succeeded,
    #[serde(rename = "FAILED")]
    Failed,
}

impl From<RefundStatus> for enums::RefundStatus {
    fn from(item: RefundStatus) -> Self {
        match item {
            RefundStatus::Succeeded => Self::Success,
            RefundStatus::Failed => Self::Failure,
            RefundStatus::Processing => Self::Pending,
        }
    }
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct RefundResponse {
    pub id: String,
    pub settlement_id: Option<String>,
    pub merchant_ref_num: String,
    pub amount: i64,
    pub currency_code: String,
    pub status: RefundStatus,
    #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
    pub txn_time: Option<time::PrimitiveDateTime>,
}

impl TryFrom<RefundsResponseRouterData<Execute, RefundResponse>> for RefundsRouterData<Execute> {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: RefundsResponseRouterData<Execute, RefundResponse>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(RefundsResponseData {
                connector_refund_id: item.response.id.to_string(),
                refund_status: enums::RefundStatus::from(item.response.status),
            }),
            ..item.data
        })
    }
}

impl TryFrom<RefundsResponseRouterData<RSync, RefundResponse>> for RefundsRouterData<RSync> {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: RefundsResponseRouterData<RSync, RefundResponse>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(RefundsResponseData {
                connector_refund_id: item.response.id.to_string(),
                refund_status: enums::RefundStatus::from(item.response.status),
            }),
            ..item.data
        })
    }
}

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
pub struct PaysafeErrorResponse {
    pub status_code: u16,
    pub code: String,
    pub message: String,
    pub reason: Option<String>,
}
