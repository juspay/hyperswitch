use cards::CardNumber;
use common_enums::enums;
use common_utils::{pii::IpAddress, request::Method, types::MinorUnit};
use hyperswitch_domain_models::{
    payment_method_data::PaymentMethodData,
    router_data::{ConnectorAuthType, RouterData},
    router_flow_types::refunds::{Execute, RSync},
    router_request_types::{PaymentsAuthorizeData, PaymentsPreProcessingData, ResponseId},
    router_response_types::{PaymentsResponseData, RefundsResponseData},
    types::{
        PaymentsAuthorizeRouterData, PaymentsCaptureRouterData, PaymentsPreProcessingRouterData,
        RefundsRouterData,
    },
};
use hyperswitch_interfaces::errors;
use masking::{ExposeInterface, PeekInterface, Secret};
use serde::{Deserialize, Serialize};

use crate::{
    types::{RefundsResponseRouterData, ResponseRouterData},
    utils::{
        BrowserInformationData, CardData, PaymentsAuthorizeRequestData,
        PaymentsPreProcessingRequestData, RouterData as _,
    },
};

//TODO: Fill the struct with respective fields
pub struct PaysafeRouterData<T> {
    pub amount: MinorUnit, // The type of amount that a connector accepts, for example, String, i64, f64, etc.
    pub router_data: T,
}

impl<T> From<(MinorUnit, T)> for PaysafeRouterData<T> {
    fn from((amount, item): (MinorUnit, T)) -> Self {
        Self {
            amount,
            router_data: item,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PaysafePaymentHandleRequest {
    pub merchant_ref_num: String,
    pub amount: MinorUnit,
    pub settle_with_auth: bool,
    pub card: PaysafeCard,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub billing_details: Option<PaysafeBillingDetails>,
    pub currency_code: enums::Currency,
    pub payment_type: PaysafePaymentType,
    pub transaction_type: TransactionType,
    pub return_links: Vec<ReturnLink>,
    pub account_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ReturnLink {
    pub rel: String,
    pub href: String,
    pub method: String,
}

#[derive(Debug, Serialize)]
pub enum PaysafePaymentType {
    #[serde(rename = "CARD")]
    Card,
}

#[derive(Debug, Serialize)]
pub enum TransactionType {
    #[serde(rename = "PAYMENT")]
    Payment,
}

impl TryFrom<&PaysafeRouterData<&PaymentsPreProcessingRouterData>> for PaysafePaymentHandleRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &PaysafeRouterData<&PaymentsPreProcessingRouterData>,
    ) -> Result<Self, Self::Error> {
        match item.router_data.request.get_payment_method_data()?.clone() {
            PaymentMethodData::Card(req_card) => {
                let card = PaysafeCard {
                    card_num: req_card.card_number.clone(),
                    card_expiry: PaysafeCardExpiry {
                        month: req_card.card_exp_month.clone(),
                        year: req_card.get_expiry_year_4_digit(),
                    },
                    cvv: if req_card.card_cvc.clone().expose().is_empty() {
                        None
                    } else {
                        Some(req_card.card_cvc.clone())
                    },
                    holder_name: item.router_data.get_optional_billing_full_name(),
                };

                let billing_details = None;
                let account_id = "1002696790".to_string(); // Test account id

                let amount = item.amount;
                let payment_type = PaysafePaymentType::Card;
                let transaction_type = TransactionType::Payment;
                // let redirect_url = item.router_data.request.get_router_return_url()?;
                let redirect_url = "https://goole.com".to_string(); // Test redirect url
                let return_links = vec![
                    ReturnLink {
                        rel: "default".to_string(),
                        href: redirect_url.clone(),
                        method: Method::Get.to_string(),
                    },
                    ReturnLink {
                        rel: "on_completed".to_string(),
                        href: redirect_url.clone(),
                        method: Method::Get.to_string(),
                    },
                    ReturnLink {
                        rel: "on_failed".to_string(),
                        href: redirect_url.clone(),
                        method: Method::Get.to_string(),
                    },
                    ReturnLink {
                        rel: "on_cancelled".to_string(),
                        href: redirect_url.clone(),
                        method: Method::Get.to_string(),
                    },
                ];

                Ok(Self {
                    merchant_ref_num: item.router_data.connector_request_reference_id.clone(),
                    amount,
                    settle_with_auth: matches!(
                        item.router_data.request.capture_method,
                        Some(enums::CaptureMethod::Automatic) | None
                    ),
                    card,
                    billing_details,
                    currency_code: item.router_data.request.get_currency()?,
                    payment_type,
                    transaction_type,
                    return_links,
                    account_id,
                })
            }
            _ => Err(errors::ConnectorError::NotImplemented(
                "Payment Method".to_string(),
            ))?,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PaysafePaymentHandleResponse {
    pub id: String,
    pub merchant_ref_num: String,
    pub payment_handle_token: Secret<String>,
    pub status: PaysafePaymentHandleStatus,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "UPPERCASE")]
pub enum PaysafePaymentHandleStatus {
    Initiated,
    Payable,
    #[default]
    Processing,
    Failed,
    Expired,
    Completed,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PaysafeMeta {
    pub payment_handle_token: Secret<String>,
}

impl From<PaysafePaymentHandleStatus> for common_enums::AttemptStatus {
    fn from(item: PaysafePaymentHandleStatus) -> Self {
        match item {
            PaysafePaymentHandleStatus::Completed => Self::Charged,
            PaysafePaymentHandleStatus::Failed | PaysafePaymentHandleStatus::Expired => {
                Self::Failure
            }
            PaysafePaymentHandleStatus::Processing | PaysafePaymentHandleStatus::Payable => {
                Self::Pending
            }
            PaysafePaymentHandleStatus::Initiated => Self::AuthenticationPending,
        }
    }
}

impl<F>
    TryFrom<
        ResponseRouterData<
            F,
            PaysafePaymentHandleResponse,
            PaymentsPreProcessingData,
            PaymentsResponseData,
        >,
    > for RouterData<F, PaymentsPreProcessingData, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<
            F,
            PaysafePaymentHandleResponse,
            PaymentsPreProcessingData,
            PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            status: common_enums::AttemptStatus::from(item.response.status),
            preprocessing_id: Some(
                item.response
                    .payment_handle_token
                    .to_owned()
                    .peek()
                    .to_string(),
            ),
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

impl<F>
    TryFrom<
        ResponseRouterData<F, PaysafePaymentsResponse, PaymentsAuthorizeData, PaymentsResponseData>,
    > for RouterData<F, PaymentsAuthorizeData, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<
            F,
            PaysafePaymentsResponse,
            PaymentsAuthorizeData,
            PaymentsResponseData,
        >,
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

// Paysafe Card Payments Request Structure
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PaysafePaymentsRequest {
    pub merchant_ref_num: String,
    pub amount: MinorUnit,
    pub settle_with_auth: bool,
    pub payment_handle_token: Secret<String>,
    pub currency_code: enums::Currency,
    pub customer_ip: Option<Secret<String, IpAddress>>,
}

#[derive(Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PaysafeCard {
    pub card_num: CardNumber,
    pub card_expiry: PaysafeCardExpiry,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cvv: Option<Secret<String>>,
    pub holder_name: Option<Secret<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PaysafeCardExpiry {
    pub month: Secret<String>,
    pub year: Secret<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PaysafeBillingDetails {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub street: Option<Secret<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub city: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state: Option<Secret<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub country: Option<enums::CountryAlpha2>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub zip: Option<Secret<String>>,
}

impl TryFrom<&PaysafeRouterData<&PaymentsAuthorizeRouterData>> for PaysafePaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &PaysafeRouterData<&PaymentsAuthorizeRouterData>,
    ) -> Result<Self, Self::Error> {
        let payment_handle_token = Secret::new(item.router_data.get_preprocessing_id()?);
        let amount = item.amount;
        let customer_ip = Some(
            item.router_data
                .request
                .get_browser_info()?
                .get_ip_address()?,
        );

        Ok(Self {
            merchant_ref_num: item.router_data.connector_request_reference_id.clone(),
            payment_handle_token,
            amount,
            settle_with_auth: item.router_data.request.is_auto_capture()?,
            currency_code: item.router_data.request.currency,
            customer_ip,
        })
    }
}

pub struct PaysafeAuthType {
    pub(super) username: Secret<String>,
    pub(super) password: Secret<String>,
}

impl TryFrom<&ConnectorAuthType> for PaysafeAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            ConnectorAuthType::BodyKey { api_key, key1 } => Ok(Self {
                username: api_key.to_owned(),
                password: key1.to_owned(),
            }),
            _ => Err(errors::ConnectorError::FailedToObtainAuthType.into()),
        }
    }
}
// Paysafe Payment Status Mapping
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "UPPERCASE")]
pub enum PaysafePaymentStatus {
    Received,
    Completed,
    Held,
    Failed,
    #[default]
    Pending,
    Cancelled,
    Processing,
}

impl From<PaysafePaymentStatus> for common_enums::AttemptStatus {
    fn from(item: PaysafePaymentStatus) -> Self {
        match item {
            PaysafePaymentStatus::Completed => Self::Charged,
            PaysafePaymentStatus::Failed => Self::Failure,
            PaysafePaymentStatus::Pending
            | PaysafePaymentStatus::Processing
            | PaysafePaymentStatus::Received => Self::Pending,
            PaysafePaymentStatus::Cancelled => Self::Voided,
            PaysafePaymentStatus::Held => Self::Unresolved,
        }
    }
}

// Paysafe Card Response Structure
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PaysafeCardResponse {
    #[serde(rename = "type")]
    pub card_type: Option<String>,
    pub last_digits: Option<String>,
    pub card_expiry: Option<PaysafeCardExpiry>,
}

// Paysafe Payments Response Structure
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PaysafePaymentsSyncResponse {
    pub payment_handles: Vec<PaysafePaymentHandleResponse>,
}

// Paysafe Payments Response Structure
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PaysafePaymentsResponse {
    pub id: String,
    pub merchant_ref_num: Option<String>,
    pub status: PaysafePaymentStatus,
    pub settlements: Option<Vec<Settlements>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Settlements {
    pub merchant_ref_num: Option<String>,
    pub id: String,
}

impl<F, T> TryFrom<ResponseRouterData<F, PaysafePaymentsSyncResponse, T, PaymentsResponseData>>
    for RouterData<F, T, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, PaysafePaymentsSyncResponse, T, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        let payment_handle = item
            .response
            .payment_handles
            .first()
            .ok_or(errors::ConnectorError::ResponseDeserializationFailed)?;
        Ok(Self {
            status: common_enums::AttemptStatus::from(payment_handle.status),
            response: Ok(PaymentsResponseData::TransactionResponse {
                resource_id: ResponseId::ConnectorTransactionId(payment_handle.id.clone()),
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

// Paysafe Capture Request Structure
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PaysafeCaptureRequest {
    pub merchant_ref_num: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub amount: Option<MinorUnit>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dup_check: Option<bool>,
}

impl TryFrom<&PaysafeRouterData<&PaymentsCaptureRouterData>> for PaysafeCaptureRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &PaysafeRouterData<&PaymentsCaptureRouterData>) -> Result<Self, Self::Error> {
        let amount = Some(item.amount);

        Ok(Self {
            merchant_ref_num: item.router_data.connector_request_reference_id.clone(),
            amount,
            dup_check: Some(true),
        })
    }
}

// Paysafe Refund Request Structure
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PaysafeRefundRequest {
    pub merchant_ref_num: String,
    pub amount: MinorUnit,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dup_check: Option<bool>,
}

impl<F> TryFrom<&PaysafeRouterData<&RefundsRouterData<F>>> for PaysafeRefundRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &PaysafeRouterData<&RefundsRouterData<F>>) -> Result<Self, Self::Error> {
        let amount = item.amount;

        Ok(Self {
            merchant_ref_num: item.router_data.request.refund_id.clone(),
            amount,
            dup_check: Some(true),
        })
    }
}

// Type definition for Refund Response

#[allow(dead_code)]
#[derive(Debug, Copy, Serialize, Default, Deserialize, Clone)]
pub enum RefundStatus {
    Succeeded,
    Failed,
    #[default]
    Processing,
}

impl From<RefundStatus> for enums::RefundStatus {
    fn from(item: RefundStatus) -> Self {
        match item {
            RefundStatus::Succeeded => Self::Success,
            RefundStatus::Failed => Self::Failure,
            RefundStatus::Processing => Self::Pending,
            //TODO: Review mapping
        }
    }
}

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct RefundResponse {
    id: String,
    status: RefundStatus,
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
#[derive(Debug, Serialize, Deserialize)]
pub struct PaysafeErrorResponse {
    // pub status_code: u16,
    // pub code: String,
    // pub message: String,
    // pub reason: Option<String>,
    // pub network_advice_code: Option<String>,
    // pub network_decline_code: Option<String>,
    // pub network_error_message: Option<String>,
    pub error: Error,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Error {
    pub code: String,
    pub message: String,
    pub details: Vec<String>,
    #[serde(rename = "fieldErrors")]
    pub field_errors: Option<Vec<FieldError>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FieldError {
    pub field: String,
    pub error: String,
}
