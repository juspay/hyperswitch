use cards::CardNumber;
use common_enums::enums;
use common_utils::{
    pii::{IpAddress, SecretSerdeValue},
    request::Method,
    types::MinorUnit,
};
use error_stack::ResultExt;
use hyperswitch_domain_models::{
    payment_method_data::PaymentMethodData,
    router_data::{ConnectorAuthType, RouterData},
    router_flow_types::refunds::{Execute, RSync},
    router_request_types::{
        PaymentsAuthorizeData, PaymentsPreProcessingData, PaymentsSyncData, ResponseId,
    },
    router_response_types::{PaymentsResponseData, RefundsResponseData},
    types::{
        PaymentsAuthorizeRouterData, PaymentsCancelRouterData, PaymentsCaptureRouterData,
        PaymentsPreProcessingRouterData, RefundsRouterData,
    },
};
use hyperswitch_interfaces::errors;
use masking::{ExposeInterface, PeekInterface, Secret};
use serde::{Deserialize, Serialize};

use crate::{
    types::{RefundsResponseRouterData, ResponseRouterData},
    utils::{
        self, BrowserInformationData, CardData, PaymentsAuthorizeRequestData,
        PaymentsPreProcessingRequestData, RouterData as _,
    },
};

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

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct PaysafeConnectorMetadataObject {
    pub account_id: Secret<String>,
}

impl TryFrom<&Option<SecretSerdeValue>> for PaysafeConnectorMetadataObject {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(meta_data: &Option<SecretSerdeValue>) -> Result<Self, Self::Error> {
        let metadata: Self = utils::to_connector_meta_from_secret::<Self>(meta_data.clone())
            .change_context(errors::ConnectorError::InvalidConnectorConfig {
                config: "merchant_connector_account.metadata",
            })?;
        Ok(metadata)
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PaysafePaymentHandleRequest {
    pub merchant_ref_num: String,
    pub amount: MinorUnit,
    pub settle_with_auth: bool,
    pub card: PaysafeCard,
    pub currency_code: enums::Currency,
    pub payment_type: PaysafePaymentType,
    pub transaction_type: TransactionType,
    pub return_links: Vec<ReturnLink>,
    pub account_id: Secret<String>,
}

#[derive(Debug, Serialize)]
pub struct ReturnLink {
    pub rel: LinkType,
    pub href: String,
    pub method: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LinkType {
    OnCompleted,
    OnFailed,
    OnCancelled,
    Default,
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
        if item.router_data.is_three_ds() {
            Err(errors::ConnectorError::NotSupported {
                message: "Card 3DS".to_string(),
                connector: "Paysafe",
            })?
        };
        let metadata: PaysafeConnectorMetadataObject =
            utils::to_connector_meta_from_secret(item.router_data.connector_meta_data.clone())
                .change_context(errors::ConnectorError::InvalidConnectorConfig {
                    config: "merchant_connector_account.metadata",
                })?;
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
                let account_id = metadata.account_id;

                let amount = item.amount;
                let payment_type = PaysafePaymentType::Card;
                let transaction_type = TransactionType::Payment;
                let redirect_url = item.router_data.request.get_router_return_url()?;
                let return_links = vec![
                    ReturnLink {
                        rel: LinkType::Default,
                        href: redirect_url.clone(),
                        method: Method::Get.to_string(),
                    },
                    ReturnLink {
                        rel: LinkType::OnCompleted,
                        href: redirect_url.clone(),
                        method: Method::Get.to_string(),
                    },
                    ReturnLink {
                        rel: LinkType::OnFailed,
                        href: redirect_url.clone(),
                        method: Method::Get.to_string(),
                    },
                    ReturnLink {
                        rel: LinkType::OnCancelled,
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
            preprocessing_id: Some(
                item.response
                    .payment_handle_token
                    .to_owned()
                    .peek()
                    .to_string(),
            ),
            response: Ok(PaymentsResponseData::TransactionResponse {
                resource_id: ResponseId::NoResponseId,
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
            status: get_paysafe_payment_status(
                item.response.status,
                item.data.request.capture_method,
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

impl TryFrom<&PaysafeRouterData<&PaymentsAuthorizeRouterData>> for PaysafePaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &PaysafeRouterData<&PaymentsAuthorizeRouterData>,
    ) -> Result<Self, Self::Error> {
        if item.router_data.is_three_ds() {
            Err(errors::ConnectorError::NotSupported {
                message: "Card 3DS".to_string(),
                connector: "Paysafe",
            })?
        };
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

// Paysafe Payment Status
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

pub fn get_paysafe_payment_status(
    status: PaysafePaymentStatus,
    capture_method: Option<common_enums::CaptureMethod>,
) -> common_enums::AttemptStatus {
    match status {
        PaysafePaymentStatus::Completed => match capture_method {
            Some(common_enums::CaptureMethod::Manual) => common_enums::AttemptStatus::Authorized,
            Some(common_enums::CaptureMethod::Automatic) | None => {
                common_enums::AttemptStatus::Charged
            }
            Some(common_enums::CaptureMethod::SequentialAutomatic)
            | Some(common_enums::CaptureMethod::ManualMultiple)
            | Some(common_enums::CaptureMethod::Scheduled) => {
                common_enums::AttemptStatus::Unresolved
            }
        },
        PaysafePaymentStatus::Failed => common_enums::AttemptStatus::Failure,
        PaysafePaymentStatus::Pending
        | PaysafePaymentStatus::Processing
        | PaysafePaymentStatus::Received
        | PaysafePaymentStatus::Held => common_enums::AttemptStatus::Pending,
        PaysafePaymentStatus::Cancelled => common_enums::AttemptStatus::Voided,
    }
}

// Paysafe Payments Response Structure
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PaysafePaymentsSyncResponse {
    pub payments: Vec<PaysafePaymentsResponse>,
}

// Paysafe Payments Response Structure
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PaysafePaymentsResponse {
    pub id: String,
    pub merchant_ref_num: Option<String>,
    pub status: PaysafePaymentStatus,
    pub settlements: Option<Vec<PaysafeSettlementResponse>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PaysafeSettlementResponse {
    pub merchant_ref_num: Option<String>,
    pub id: String,
    pub status: PaysafeSettlementStatus,
}

impl<F>
    TryFrom<
        ResponseRouterData<F, PaysafePaymentsSyncResponse, PaymentsSyncData, PaymentsResponseData>,
    > for RouterData<F, PaymentsSyncData, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<
            F,
            PaysafePaymentsSyncResponse,
            PaymentsSyncData,
            PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        let payment_handle = item
            .response
            .payments
            .first()
            .ok_or(errors::ConnectorError::ResponseDeserializationFailed)?;
        Ok(Self {
            status: get_paysafe_payment_status(
                payment_handle.status,
                item.data.request.capture_method,
            ),
            response: Ok(PaymentsResponseData::TransactionResponse {
                resource_id: ResponseId::NoResponseId,
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

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PaysafeCaptureRequest {
    pub merchant_ref_num: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub amount: Option<MinorUnit>,
}

impl TryFrom<&PaysafeRouterData<&PaymentsCaptureRouterData>> for PaysafeCaptureRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &PaysafeRouterData<&PaymentsCaptureRouterData>) -> Result<Self, Self::Error> {
        let amount = Some(item.amount);

        Ok(Self {
            merchant_ref_num: item.router_data.connector_request_reference_id.clone(),
            amount,
        })
    }
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "UPPERCASE")]
pub enum PaysafeSettlementStatus {
    Received,
    Initiated,
    Completed,
    Expired,
    Failed,
    #[default]
    Pending,
    Cancelled,
}

impl From<PaysafeSettlementStatus> for common_enums::AttemptStatus {
    fn from(item: PaysafeSettlementStatus) -> Self {
        match item {
            PaysafeSettlementStatus::Completed
            | PaysafeSettlementStatus::Pending
            | PaysafeSettlementStatus::Received => Self::Charged,
            PaysafeSettlementStatus::Failed | PaysafeSettlementStatus::Expired => Self::Failure,
            PaysafeSettlementStatus::Cancelled => Self::Voided,
            PaysafeSettlementStatus::Initiated => Self::Pending,
        }
    }
}

impl<F, T> TryFrom<ResponseRouterData<F, PaysafeSettlementResponse, T, PaymentsResponseData>>
    for RouterData<F, T, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, PaysafeSettlementResponse, T, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            status: common_enums::AttemptStatus::from(item.response.status),
            response: Ok(PaymentsResponseData::TransactionResponse {
                resource_id: ResponseId::ConnectorTransactionId(item.response.id.clone()),
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

impl TryFrom<&PaysafeRouterData<&PaymentsCancelRouterData>> for PaysafeCaptureRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &PaysafeRouterData<&PaymentsCancelRouterData>) -> Result<Self, Self::Error> {
        let amount = Some(item.amount);

        Ok(Self {
            merchant_ref_num: item.router_data.connector_request_reference_id.clone(),
            amount,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct VoidResponse {
    pub merchant_ref_num: Option<String>,
    pub id: String,
    pub status: PaysafeVoidStatus,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "UPPERCASE")]
pub enum PaysafeVoidStatus {
    Received,
    Completed,
    Held,
    Failed,
    #[default]
    Pending,
    Cancelled,
}

impl From<PaysafeVoidStatus> for common_enums::AttemptStatus {
    fn from(item: PaysafeVoidStatus) -> Self {
        match item {
            PaysafeVoidStatus::Completed
            | PaysafeVoidStatus::Pending
            | PaysafeVoidStatus::Received => Self::Voided,
            PaysafeVoidStatus::Failed | PaysafeVoidStatus::Held => Self::Failure,
            PaysafeVoidStatus::Cancelled => Self::Voided,
        }
    }
}

impl<F, T> TryFrom<ResponseRouterData<F, VoidResponse, T, PaymentsResponseData>>
    for RouterData<F, T, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, VoidResponse, T, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            status: common_enums::AttemptStatus::from(item.response.status),
            response: Ok(PaymentsResponseData::TransactionResponse {
                resource_id: ResponseId::NoResponseId,
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

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PaysafeRefundRequest {
    pub merchant_ref_num: String,
    pub amount: MinorUnit,
}

impl<F> TryFrom<&PaysafeRouterData<&RefundsRouterData<F>>> for PaysafeRefundRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &PaysafeRouterData<&RefundsRouterData<F>>) -> Result<Self, Self::Error> {
        let amount = item.amount;

        Ok(Self {
            merchant_ref_num: item.router_data.request.refund_id.clone(),
            amount,
        })
    }
}

// Type definition for Refund Response

#[derive(Debug, Copy, Serialize, Default, Deserialize, Clone)]
#[serde(rename_all = "UPPERCASE")]
pub enum RefundStatus {
    Received,
    Initiated,
    Completed,
    Expired,
    Failed,
    #[default]
    Pending,
    Cancelled,
}

impl From<RefundStatus> for enums::RefundStatus {
    fn from(item: RefundStatus) -> Self {
        match item {
            RefundStatus::Received | RefundStatus::Completed => Self::Success,
            RefundStatus::Failed | RefundStatus::Cancelled | RefundStatus::Expired => Self::Failure,
            RefundStatus::Pending | RefundStatus::Initiated => Self::Pending,
        }
    }
}

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

#[derive(Debug, Serialize, Deserialize)]
pub struct PaysafeErrorResponse {
    pub error: Error,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Error {
    pub code: String,
    pub message: String,
    pub details: Option<Vec<String>>,
    #[serde(rename = "fieldErrors")]
    pub field_errors: Option<Vec<FieldError>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FieldError {
    pub field: String,
    pub error: String,
}
