use std::collections::HashMap;

use crate::types::PaymentsCancelResponseRouterData;
use crate::types::PaymentsSyncResponseRouterData;
use crate::utils::PaymentsSyncRequestData;
use common_enums::{enums, CardNetwork};
use common_utils::{request::Method, types::StringMajorUnit};
use hyperswitch_domain_models::router_data::ErrorResponse;
use hyperswitch_domain_models::router_request_types::PaymentsAuthorizeData;
use hyperswitch_domain_models::types::PaymentsCancelRouterData;
use hyperswitch_domain_models::types::PaymentsCaptureRouterData;
use hyperswitch_domain_models::types::PaymentsSyncRouterData;
use hyperswitch_domain_models::{
    payment_method_data::PaymentMethodData,
    router_data::{ConnectorAuthType, PaymentMethodToken, RouterData},
    router_flow_types::refunds::{Execute, RSync},
    router_request_types::ResponseId,
    router_response_types::{PaymentsResponseData, RedirectForm, RefundsResponseData},
    types::{PaymentsAuthorizeRouterData, RefundsRouterData, TokenizationRouterData},
};
use hyperswitch_interfaces::consts::NO_ERROR_CODE;
use hyperswitch_interfaces::consts::NO_ERROR_MESSAGE;
use hyperswitch_interfaces::errors;
use masking::{ExposeInterface, Secret};
use serde::{Deserialize, Serialize};

use crate::types::PaymentsCaptureResponseRouterData;
use crate::{
    types::{RefundsResponseRouterData, ResponseRouterData},
    unimplemented_payment_method,
    utils::{self, PaymentsAuthorizeRequestData, RouterData as _},
};

pub struct HipayRouterData<T> {
    pub amount: StringMajorUnit,
    pub router_data: T,
}

impl<T> From<(StringMajorUnit, T)> for HipayRouterData<T> {
    fn from((amount, item): (StringMajorUnit, T)) -> Self {
        Self {
            amount,
            router_data: item,
        }
    }
}
#[derive(Debug, Serialize, Deserialize)]
pub enum Operation {
    Authorization,
    Sale,
    Capture,
    Refund,
    Cancel,
}
#[derive(Debug, Serialize, Deserialize)]
pub struct HipayPaymentsRequest {
    operation: Operation,
    authentication_indicator: u8,
    cardtoken: Secret<String>,
    orderid: String,
    currency: enums::Currency,
    payment_product: String,
    amount: StringMajorUnit,
    description: String,
    decline_url: Option<String>,
    pending_url: Option<String>,
    cancel_url: Option<String>,
    accept_url: Option<String>,
    notify_url: Option<String>,
}
#[derive(Debug, Serialize, Deserialize)]
pub struct HipayMaintenanceRequest {
    operation: Operation,
    currency: Option<enums::Currency>,
    amount: StringMajorUnit,
}

#[derive(Default, Debug, Serialize, Deserialize)]
pub struct HiPayTokenRequest {
    pub card_number: cards::CardNumber,
    pub card_expiry_month: Secret<String>,
    pub card_expiry_year: Secret<String>,
    pub card_holder: Secret<String>,
    pub cvc: Secret<String>,
}
impl TryFrom<&HipayRouterData<&PaymentsAuthorizeRouterData>> for HipayPaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &HipayRouterData<&PaymentsAuthorizeRouterData>) -> Result<Self, Self::Error> {
        match item.router_data.request.payment_method_data.clone() {
            PaymentMethodData::Card(req_card) => Ok(Self {
                operation: match item.router_data.request.is_auto_capture()? {
                    true => Operation::Sale,
                    false => Operation::Authorization,
                },
                authentication_indicator: match item.router_data.is_three_ds() {
                    true => 2,
                    false => 0,
                },
                cardtoken: match item.router_data.get_payment_method_token()? {
                    PaymentMethodToken::Token(token) => token,
                    PaymentMethodToken::ApplePayDecrypt(_) => {
                        Err(unimplemented_payment_method!("Apple Pay", "Hipay"))?
                    }
                    PaymentMethodToken::PazeDecrypt(_) => {
                        Err(unimplemented_payment_method!("Paze", "Hipay"))?
                    }
                    PaymentMethodToken::GooglePayDecrypt(_) => {
                        Err(unimplemented_payment_method!("Google Pay", "Hipay"))?
                    }
                },
                orderid: item.router_data.connector_request_reference_id.clone(),
                currency: item.router_data.request.currency,
                payment_product: match req_card.card_network {
                    Some(CardNetwork::Visa) => "visa".to_string(),
                    Some(CardNetwork::Mastercard) => "mastercard".to_string(),
                    Some(CardNetwork::AmericanExpress) => "american-express".to_string(),
                    Some(CardNetwork::JCB) => "jcb".to_string(),
                    Some(CardNetwork::DinersClub) => "diners".to_string(),
                    Some(CardNetwork::Discover) => "discover".to_string(),
                    Some(CardNetwork::CartesBancaires) => "cb".to_string(),
                    Some(CardNetwork::UnionPay) => "unionpay".to_string(),
                    Some(CardNetwork::Interac) => "interac".to_string(),
                    Some(CardNetwork::RuPay) => "rupay".to_string(),
                    Some(CardNetwork::Maestro) => "maestro".to_string(),
                    None => "".to_string(),
                },
                amount: item.amount.clone(),
                description: item
                    .router_data
                    .get_description()
                    .map(|s| s.to_string())
                    .unwrap_or("Short Description".to_string()),
                decline_url: item.router_data.request.router_return_url.clone(),
                pending_url: item.router_data.request.router_return_url.clone(),
                cancel_url: item.router_data.request.router_return_url.clone(),
                accept_url: item.router_data.request.router_return_url.clone(),
                notify_url: item.router_data.request.router_return_url.clone(),
            }),
            _ => Err(errors::ConnectorError::NotImplemented("Payment method".to_string()).into()),
        }
    }
}
impl TryFrom<&TokenizationRouterData> for HiPayTokenRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &TokenizationRouterData) -> Result<Self, Self::Error> {
        match item.request.payment_method_data.clone() {
            PaymentMethodData::Card(card_data) => Ok(Self {
                card_number: card_data.card_number,
                card_expiry_month: card_data.card_exp_month,
                card_expiry_year: card_data.card_exp_year,
                card_holder: card_data.card_holder_name.ok_or(
                    errors::ConnectorError::MissingRequiredField {
                        field_name: "card_holder_name",
                    },
                )?,
                cvc: card_data.card_cvc,
            }),
            _ => Err(errors::ConnectorError::NotImplemented(
                utils::get_unimplemented_payment_method_error_message("Hipay"),
            )
            .into()),
        }
    }
}
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct HipayTokenResponse {
    token: Secret<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct HipayErrorResponse {
    pub code: u8,
    pub message: String,
    pub description: Option<String>,
}

impl<F, T> TryFrom<ResponseRouterData<F, HipayTokenResponse, T, PaymentsResponseData>>
    for RouterData<F, T, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, HipayTokenResponse, T, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(PaymentsResponseData::TokenizationResponse {
                token: item.response.token.expose().clone(),
            }),
            ..item.data
        })
    }
}

pub struct HipayAuthType {
    pub(super) api_key: Secret<String>,
    pub(super) key1: Secret<String>,
}

impl TryFrom<&ConnectorAuthType> for HipayAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            ConnectorAuthType::BodyKey { api_key, key1 } => Ok(Self {
                api_key: api_key.clone(),
                key1: key1.clone(),
            }),
            _ => Err(errors::ConnectorError::FailedToObtainAuthType.into()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HipayPaymentsResponse {
    status: HipayPaymentStatus,
    message: String,
    order: PaymentOrder,
    forward_url: String,
    transaction_reference: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentOrder {
    id: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HipayMaintenanceResponse<S> {
    status: S,
    message: String,
    transaction_reference: String,
}
impl<F>
    TryFrom<
        ResponseRouterData<F, HipayPaymentsResponse, PaymentsAuthorizeData, PaymentsResponseData>,
    > for RouterData<F, PaymentsAuthorizeData, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<
            F,
            HipayPaymentsResponse,
            PaymentsAuthorizeData,
            PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            status: common_enums::AttemptStatus::from(item.response.status),
            response: Ok(PaymentsResponseData::TransactionResponse {
                resource_id: ResponseId::ConnectorTransactionId(
                    item.response.transaction_reference,
                ),
                redirection_data: match item.data.is_three_ds() {
                    true => Box::new(Some(RedirectForm::Form {
                        endpoint: item.response.forward_url,
                        method: Method::Get,
                        form_fields: HashMap::new(),
                    })),
                    false => Box::new(None),
                },
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

impl<F> TryFrom<&HipayRouterData<&RefundsRouterData<F>>> for HipayMaintenanceRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &HipayRouterData<&RefundsRouterData<F>>) -> Result<Self, Self::Error> {
        Ok(Self {
            amount: item.amount.to_owned(),
            operation: Operation::Refund,
            currency: Some(item.router_data.request.currency),
        })
    }
}
impl TryFrom<&HipayRouterData<&PaymentsCancelRouterData>> for HipayMaintenanceRequest {
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(item: &HipayRouterData<&PaymentsCancelRouterData>) -> Result<Self, Self::Error> {
        Ok(Self {
            operation: Operation::Cancel,
            currency: item.router_data.request.currency,
            amount: item.amount.clone(),
        })
    }
}
impl TryFrom<&HipayRouterData<&PaymentsCaptureRouterData>> for HipayMaintenanceRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &HipayRouterData<&PaymentsCaptureRouterData>) -> Result<Self, Self::Error> {
        Ok(Self {
            amount: item.amount.to_owned(),
            operation: Operation::Capture,
            currency: Some(item.router_data.request.currency),
        })
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum RefundStatus {
    #[serde(rename = "124")]
    RefundRequested,
    #[serde(rename = "125")]
    Refunded,
    #[serde(rename = "126")]
    PartiallyRefunded,
    #[serde(rename = "165")]
    RefundRefused,
}

impl From<RefundStatus> for enums::RefundStatus {
    fn from(item: RefundStatus) -> Self {
        match item {
            RefundStatus::RefundRequested => Self::Pending,
            RefundStatus::Refunded | RefundStatus::PartiallyRefunded => Self::Success,
            RefundStatus::RefundRefused => Self::Failure,
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum HipayPaymentStatus {
    #[serde(rename = "109")]
    AuthenticationFailed,
    #[serde(rename = "110")]
    Blocked,
    #[serde(rename = "111")]
    Denied,
    #[serde(rename = "112")]
    AuthorizedAndPending,
    #[serde(rename = "113")]
    Refused,
    #[serde(rename = "114")]
    Expired,
    #[serde(rename = "115")]
    Cancelled,
    #[serde(rename = "116")]
    Authorized,
    #[serde(rename = "117")]
    CaptureRequested,
    #[serde(rename = "118")]
    Captured,
    #[serde(rename = "119")]
    PartiallyCaptured,
    #[serde(rename = "129")]
    ChargedBack,
    #[serde(rename = "173")]
    CaptureRefused,
    #[serde(rename = "174")]
    AwaitingTerminal,
    #[serde(rename = "175")]
    AuthorizationCancellationRequested,
    #[serde(rename = "177")]
    ChallengeRequested,
    #[serde(rename = "178")]
    SoftDeclined,
    #[serde(rename = "200")]
    PendingPayment,
    #[serde(rename = "101")]
    Created,
    // #[serde(rename = "103")]
    // CardholderEnrolled,
    // #[serde(rename = "104")]
    // CardholderNotEnrolled,
    #[serde(rename = "105")]
    UnableToAuthenticate,
    #[serde(rename = "106")]
    CardholderAuthenticated,
    #[serde(rename = "107")]
    AuthenticationAttempted,
    #[serde(rename = "108")]
    CouldNotAuthenticate,
    #[serde(rename = "120")]
    Collected,
    #[serde(rename = "121")]
    PartiallyCollected,
    #[serde(rename = "122")]
    Settled,
    #[serde(rename = "123")]
    PartiallySettled,
    // #[serde(rename = "131")]
    // Debited,
    // #[serde(rename = "132")]
    // PartiallyDebited,
    #[serde(rename = "140")]
    AuthenticationRequested,
    #[serde(rename = "141")]
    Authenticated,
    // #[serde(rename = "150")]
    // AcquirerFound = 150,
    #[serde(rename = "151")]
    AcquirerNotFound,
    // #[serde(rename = "160")]
    // // CardholderEnrollmentUnknown = 160,
    #[serde(rename = "161")]
    RiskAccepted,
    #[serde(rename = "163")]
    AuthorizationRefused,
}
impl From<HipayPaymentStatus> for common_enums::AttemptStatus {
    fn from(status: HipayPaymentStatus) -> Self {
        match status {
            HipayPaymentStatus::AuthenticationFailed => Self::AuthenticationFailed,
            HipayPaymentStatus::Blocked
            | HipayPaymentStatus::Refused
            | HipayPaymentStatus::Expired
            | HipayPaymentStatus::Denied => Self::Failure,
            HipayPaymentStatus::AuthorizedAndPending => Self::Authorizing,
            HipayPaymentStatus::Cancelled => Self::Voided,
            HipayPaymentStatus::Authorized => Self::Authorized,
            HipayPaymentStatus::CaptureRequested => Self::CaptureInitiated,
            HipayPaymentStatus::Captured => Self::Charged,
            HipayPaymentStatus::PartiallyCaptured => Self::PartialCharged,
            HipayPaymentStatus::CaptureRefused => Self::CaptureFailed,
            HipayPaymentStatus::AwaitingTerminal => Self::Pending,
            HipayPaymentStatus::AuthorizationCancellationRequested => Self::VoidInitiated,
            HipayPaymentStatus::ChallengeRequested => Self::AuthenticationPending,
            HipayPaymentStatus::SoftDeclined => Self::AuthorizationFailed,
            HipayPaymentStatus::PendingPayment => Self::Pending,
            HipayPaymentStatus::ChargedBack => Self::Failure,
            HipayPaymentStatus::Created => Self::Started,
            // HipayPaymentStatus::CardholderEnrolled =>
            // HipayPaymentStatus::CardholderNotEnrolled => todo!(),
            HipayPaymentStatus::UnableToAuthenticate | HipayPaymentStatus::CouldNotAuthenticate => {
                Self::AuthenticationFailed
            }
            HipayPaymentStatus::CardholderAuthenticated => Self::Pending,
            HipayPaymentStatus::AuthenticationAttempted => Self::AuthenticationPending,
            HipayPaymentStatus::Collected
            | HipayPaymentStatus::PartiallySettled
            | HipayPaymentStatus::PartiallyCollected
            | HipayPaymentStatus::Settled => Self::Charged,
            // HipayPaymentStatus::Debited => todo!(),
            // HipayPaymentStatus::PartiallyDebited => todo!(),
            HipayPaymentStatus::AuthenticationRequested => Self::AuthenticationPending,
            HipayPaymentStatus::Authenticated => Self::AuthenticationSuccessful,
            // HipayPaymentStatus::AcquirerFound => todo!(),
            HipayPaymentStatus::AcquirerNotFound => Self::Failure,
            // HipayPaymentStatus::CardholderEnrollmentUnknown => todo!(),
            HipayPaymentStatus::RiskAccepted => Self::Pending,
            HipayPaymentStatus::AuthorizationRefused => Self::AuthorizationFailed,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefundResponse {
    id: String,
    status: RefundStatus,
}

impl TryFrom<RefundsResponseRouterData<Execute, HipayMaintenanceResponse<RefundStatus>>>
    for RefundsRouterData<Execute>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: RefundsResponseRouterData<Execute, HipayMaintenanceResponse<RefundStatus>>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(RefundsResponseData {
                connector_refund_id: item.response.transaction_reference,
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

impl TryFrom<PaymentsCaptureResponseRouterData<HipayMaintenanceResponse<HipayPaymentStatus>>>
    for PaymentsCaptureRouterData
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: PaymentsCaptureResponseRouterData<HipayMaintenanceResponse<HipayPaymentStatus>>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            status: common_enums::AttemptStatus::from(item.response.status),
            response: Ok(PaymentsResponseData::TransactionResponse {
                resource_id: ResponseId::ConnectorTransactionId(
                    item.response.transaction_reference.clone().to_string(),
                ),
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
impl TryFrom<PaymentsCancelResponseRouterData<HipayMaintenanceResponse<HipayPaymentStatus>>>
    for PaymentsCancelRouterData
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: PaymentsCancelResponseRouterData<HipayMaintenanceResponse<HipayPaymentStatus>>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            status: common_enums::AttemptStatus::from(item.response.status),
            response: Ok(PaymentsResponseData::TransactionResponse {
                resource_id: ResponseId::ConnectorTransactionId(
                    item.response.transaction_reference.clone().to_string(),
                ),
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
#[derive(Debug, Deserialize, Serialize, PartialEq, Eq, Clone)]
#[serde(rename_all = "lowercase")]
pub enum HipaySyncState {
    Completed,
    Waiting,
    Pending,
    Declined,
    Forwarding,
    Error,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Reason {
    reason: Option<String>,
    code: Option<u8>,
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum HipaySyncResponse {
    Response {
        state: HipaySyncState,
        reason: Reason,
    },
    Error {
        message: String,
        code: u32,
    },
}
fn get_sync_status(state: HipaySyncState, is_auto_capture: bool) -> enums::AttemptStatus {
    match state {
        HipaySyncState::Completed => {
            if is_auto_capture {
                enums::AttemptStatus::Charged
            } else {
                enums::AttemptStatus::Authorized
            }
        }
        HipaySyncState::Waiting | HipaySyncState::Pending => enums::AttemptStatus::Pending,
        HipaySyncState::Declined | HipaySyncState::Error => enums::AttemptStatus::Failure,
        HipaySyncState::Forwarding => enums::AttemptStatus::AuthenticationPending,
    }
}

impl TryFrom<PaymentsSyncResponseRouterData<HipaySyncResponse>> for PaymentsSyncRouterData {
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(
        item: PaymentsSyncResponseRouterData<HipaySyncResponse>,
    ) -> Result<Self, Self::Error> {
        match item.response {
            HipaySyncResponse::Error { message, code } => {
                let response = Err(ErrorResponse {
                    code: code.to_string(),
                    message: message.clone(),
                    reason: Some(message.clone()),
                    attempt_status: None,
                    connector_transaction_id: None,
                    status_code: item.http_code,
                });
                Ok(Self {
                    response,
                    ..item.data
                })
            }
            HipaySyncResponse::Response { state, reason } => {
                let status = get_sync_status(state, item.data.request.is_auto_capture()?);
                let response = if status == enums::AttemptStatus::Failure {
                    let error_code = reason
                        .code
                        .map_or(NO_ERROR_CODE.to_string(), |c| c.to_string());
                    let error_message = reason
                        .reason
                        .clone()
                        .unwrap_or_else(|| NO_ERROR_MESSAGE.to_owned());
                    Err(ErrorResponse {
                        code: error_code,
                        message: error_message.clone(),
                        reason: Some(error_message),
                        status_code: item.http_code,
                        attempt_status: None,
                        connector_transaction_id: None,
                    })
                } else {
                    Ok(PaymentsResponseData::TransactionResponse {
                        resource_id: ResponseId::NoResponseId,
                        redirection_data: Box::new(None),
                        mandate_reference: Box::new(None),
                        connector_metadata: None,
                        network_txn_id: None,
                        connector_response_reference_id: None,
                        incremental_authorization_allowed: None,
                        charges: None,
                    })
                };
                Ok(Self {
                    status,
                    response,
                    ..item.data
                })
            }
        }
    }
}
