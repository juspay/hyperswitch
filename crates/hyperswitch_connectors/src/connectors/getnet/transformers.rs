use api_models::webhooks::IncomingWebhookEvent;
use base64::Engine;
use cards::CardNumber;
use common_enums::{enums, AttemptStatus, CaptureMethod, CountryAlpha2};
use common_utils::{
    consts::BASE64_ENGINE,
    errors::CustomResult,
    pii::{Email, IpAddress},
    types::FloatMajorUnit,
};
use error_stack::ResultExt;
use hyperswitch_domain_models::{
    payment_method_data::PaymentMethodData,
    router_data::{ConnectorAuthType, RouterData},
    router_flow_types::refunds::{Execute, RSync},
    router_request_types::{
        PaymentsAuthorizeData, PaymentsCancelData, PaymentsCaptureData, ResponseId,
    },
    router_response_types::{PaymentsResponseData, RefundsResponseData},
    types::{
        PaymentsAuthorizeRouterData, PaymentsCancelRouterData, PaymentsCaptureRouterData,
        PaymentsSyncRouterData, RefundsRouterData,
    },
};
use hyperswitch_interfaces::errors;
use masking::{PeekInterface, Secret};
use serde::{Deserialize, Serialize};

use crate::{
    connectors::paybox::transformers::parse_url_encoded_to_struct,
    types::{PaymentsSyncResponseRouterData, RefundsResponseRouterData, ResponseRouterData},
    utils::{
        BrowserInformationData, PaymentsAuthorizeRequestData, PaymentsSyncRequestData,
        RouterData as _,
    },
};

pub struct GetnetRouterData<T> {
    pub amount: FloatMajorUnit,
    pub router_data: T,
}

impl<T> From<(FloatMajorUnit, T)> for GetnetRouterData<T> {
    fn from((amount, item): (FloatMajorUnit, T)) -> Self {
        Self {
            amount,
            router_data: item,
        }
    }
}

#[derive(Default, Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Amount {
    pub value: FloatMajorUnit,
    pub currency: enums::Currency,
}
#[derive(Default, Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Address {
    #[serde(rename = "street1")]
    pub street1: Option<Secret<String>>,
    pub city: Option<String>,
    pub state: Option<Secret<String>>,
    pub country: Option<CountryAlpha2>,
}

#[derive(Default, Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct AccountHolder {
    #[serde(rename = "first-name")]
    pub first_name: Option<Secret<String>>,
    #[serde(rename = "last-name")]
    pub last_name: Option<Secret<String>>,
    pub email: Option<Email>,
    pub phone: Option<Secret<String>>,
    pub address: Option<Address>,
}

#[derive(Default, Debug, Serialize, PartialEq)]
pub struct Card {
    #[serde(rename = "account-number")]
    pub account_number: CardNumber,
    #[serde(rename = "expiration-month")]
    pub expiration_month: Secret<String>,
    #[serde(rename = "expiration-year")]
    pub expiration_year: Secret<String>,
    #[serde(rename = "card-security-code")]
    pub card_security_code: Secret<String>,
    #[serde(rename = "card-type")]
    pub card_type: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum GetnetPaymentMethods {
    CreditCard,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct PaymentMethod {
    pub name: GetnetPaymentMethods,
}

#[derive(Default, Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Notification {
    pub url: Option<String>,
}

#[derive(Default, Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct PaymentMethodContainer {
    #[serde(rename = "payment-method")]
    pub payment_method: Vec<PaymentMethod>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum NotificationFormat {
    #[serde(rename = "application/json-signed")]
    JsonSigned,
    #[serde(rename = "application/json")]
    Json,
    #[serde(rename = "application/xml")]
    Xml,
    #[serde(rename = "application/html")]
    Html,
    #[serde(rename = "application/x-www-form-urlencoded")]
    Urlencoded,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct NotificationContainer {
    pub notification: Vec<Notification>,
    pub format: NotificationFormat,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct MerchantAccountId {
    pub value: Secret<String>,
}

#[derive(Debug, Serialize, PartialEq)]
pub struct PaymentData {
    #[serde(rename = "merchant-account-id")]
    pub merchant_account_id: MerchantAccountId,
    #[serde(rename = "request-id")]
    pub request_id: String,
    #[serde(rename = "transaction-type")]
    pub transaction_type: GetnetTransactionType,
    #[serde(rename = "requested-amount")]
    pub requested_amount: Amount,
    #[serde(rename = "account-holder")]
    pub account_holder: Option<AccountHolder>,
    pub card: Card,
    #[serde(rename = "ip-address")]
    pub ip_address: Option<Secret<String, IpAddress>>,
    #[serde(rename = "payment-methods")]
    pub payment_methods: PaymentMethodContainer,
    pub notifications: Option<NotificationContainer>,
}

#[derive(Debug, Serialize)]
pub struct GetnetPaymentsRequest {
    payment: PaymentData,
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
pub struct GetnetCard {
    number: CardNumber,
    expiry_month: Secret<String>,
    expiry_year: Secret<String>,
    cvc: Secret<String>,
    complete: bool,
}

impl TryFrom<enums::PaymentMethodType> for PaymentMethodContainer {
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(payment_method_type: enums::PaymentMethodType) -> Result<Self, Self::Error> {
        match payment_method_type {
            enums::PaymentMethodType::Credit => Ok(Self {
                payment_method: vec![PaymentMethod {
                    name: GetnetPaymentMethods::CreditCard,
                }],
            }),
            _ => Err(errors::ConnectorError::NotSupported {
                message: "Payment method type not supported".to_string(),
                connector: "Getnet",
            }
            .into()),
        }
    }
}

impl TryFrom<&GetnetRouterData<&PaymentsAuthorizeRouterData>> for GetnetPaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &GetnetRouterData<&PaymentsAuthorizeRouterData>,
    ) -> Result<Self, Self::Error> {
        match item.router_data.request.payment_method_data.clone() {
            PaymentMethodData::Card(ref req_card) => {
                if item.router_data.is_three_ds() {
                    return Err(errors::ConnectorError::NotSupported {
                        message: "3DS payments".to_string(),
                        connector: "Getnet",
                    }
                    .into());
                }
                let request = &item.router_data.request;
                let auth_type = GetnetAuthType::try_from(&item.router_data.connector_auth_type)
                    .change_context(errors::ConnectorError::FailedToObtainAuthType)?;
                let merchant_account_id = MerchantAccountId {
                    value: auth_type.merchant_id,
                };

                let requested_amount = Amount {
                    value: item.amount,
                    currency: request.currency,
                };

                let account_holder = AccountHolder {
                    first_name: item.router_data.get_optional_billing_first_name(),
                    last_name: item.router_data.get_optional_billing_last_name(),
                    email: item.router_data.request.get_optional_email(),
                    phone: item.router_data.get_optional_billing_phone_number(),
                    address: Some(Address {
                        street1: item.router_data.get_optional_billing_line2(),
                        city: item.router_data.get_optional_billing_city(),
                        state: item.router_data.get_optional_billing_state(),
                        country: item.router_data.get_optional_billing_country(),
                    }),
                };

                let card = Card {
                    account_number: req_card.card_number.clone(),
                    expiration_month: req_card.card_exp_month.clone(),
                    expiration_year: req_card.card_exp_year.clone(),
                    card_security_code: req_card.card_cvc.clone(),
                    card_type: req_card
                        .card_network
                        .as_ref()
                        .map(|network| network.to_string().to_lowercase())
                        .unwrap_or_default(),
                };

                let pmt = item.router_data.request.get_payment_method_type()?;
                let payment_method = PaymentMethodContainer::try_from(pmt)?;

                let notifications: NotificationContainer = NotificationContainer {
                    format: NotificationFormat::JsonSigned,

                    notification: vec![Notification {
                        url: Some(item.router_data.request.get_webhook_url()?),
                    }],
                };
                let transaction_type = if request.is_auto_capture()? {
                    GetnetTransactionType::Purchase
                } else {
                    GetnetTransactionType::Authorization
                };
                let payment_data = PaymentData {
                    merchant_account_id,
                    request_id: item.router_data.payment_id.clone(),
                    transaction_type,
                    requested_amount,
                    account_holder: Some(account_holder),
                    card,
                    ip_address: Some(request.get_browser_info()?.get_ip_address()?),
                    payment_methods: payment_method,
                    notifications: Some(notifications),
                };

                Ok(Self {
                    payment: payment_data,
                })
            }
            _ => Err(errors::ConnectorError::NotImplemented("Payment method".to_string()).into()),
        }
    }
}

pub struct GetnetAuthType {
    pub username: Secret<String>,
    pub password: Secret<String>,
    pub merchant_id: Secret<String>,
}

impl TryFrom<&ConnectorAuthType> for GetnetAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            ConnectorAuthType::SignatureKey {
                api_key,
                key1,
                api_secret,
            } => Ok(Self {
                username: key1.to_owned(),
                password: api_key.to_owned(),
                merchant_id: api_secret.to_owned(),
            }),
            _ => Err(errors::ConnectorError::FailedToObtainAuthType.into()),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum GetnetPaymentStatus {
    Success,
    Failed,
    #[default]
    InProgress,
}

impl From<GetnetPaymentStatus> for AttemptStatus {
    fn from(item: GetnetPaymentStatus) -> Self {
        match item {
            GetnetPaymentStatus::Success => Self::Charged,
            GetnetPaymentStatus::Failed => Self::Failure,
            GetnetPaymentStatus::InProgress => Self::Pending,
        }
    }
}

#[derive(Default, Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Status {
    pub code: String,
    pub description: String,
    pub severity: String,
}

#[derive(Default, Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Statuses {
    pub status: Vec<Status>,
}

#[derive(Default, Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct CardToken {
    #[serde(rename = "token-id")]
    pub token_id: Secret<String>,
    #[serde(rename = "masked-account-number")]
    pub masked_account_number: Secret<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct PaymentResponseData {
    pub statuses: Statuses,
    pub descriptor: Option<String>,
    pub notifications: NotificationContainer,
    #[serde(rename = "merchant-account-id")]
    pub merchant_account_id: MerchantAccountId,
    #[serde(rename = "transaction-id")]
    pub transaction_id: String,
    #[serde(rename = "request-id")]
    pub request_id: String,
    #[serde(rename = "transaction-type")]
    pub transaction_type: GetnetTransactionType,
    #[serde(rename = "transaction-state")]
    pub transaction_state: GetnetPaymentStatus,
    #[serde(rename = "completion-time-stamp")]
    pub completion_time_stamp: Option<i64>,
    #[serde(rename = "requested-amount")]
    pub requested_amount: Amount,
    #[serde(rename = "account-holder")]
    pub account_holder: Option<AccountHolder>,
    #[serde(rename = "card-token")]
    pub card_token: CardToken,
    #[serde(rename = "ip-address")]
    pub ip_address: Option<Secret<String, IpAddress>>,
    #[serde(rename = "payment-methods")]
    pub payment_methods: PaymentMethodContainer,
    #[serde(rename = "api-id")]
    pub api_id: String,
    #[serde(rename = "self")]
    pub self_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentsResponse {
    payment: PaymentResponseData,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum GetnetPaymentsResponse {
    PaymentsResponse(Box<PaymentsResponse>),
    GetnetWebhookNotificationResponse(Box<GetnetWebhookNotificationResponseBody>),
}

pub fn authorization_attempt_status_from_transaction_state(
    getnet_status: GetnetPaymentStatus,
    is_auto_capture: bool,
) -> AttemptStatus {
    match getnet_status {
        GetnetPaymentStatus::Success => {
            if is_auto_capture {
                AttemptStatus::Charged
            } else {
                AttemptStatus::Authorized
            }
        }
        GetnetPaymentStatus::InProgress => AttemptStatus::Pending,
        GetnetPaymentStatus::Failed => AttemptStatus::Failure,
    }
}

impl<F>
    TryFrom<
        ResponseRouterData<F, GetnetPaymentsResponse, PaymentsAuthorizeData, PaymentsResponseData>,
    > for RouterData<F, PaymentsAuthorizeData, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<
            F,
            GetnetPaymentsResponse,
            PaymentsAuthorizeData,
            PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        match item.response {
            GetnetPaymentsResponse::PaymentsResponse(ref payment_response) => Ok(Self {
                status: authorization_attempt_status_from_transaction_state(
                    payment_response.payment.transaction_state.clone(),
                    item.data.request.is_auto_capture()?,
                ),
                response: Ok(PaymentsResponseData::TransactionResponse {
                    resource_id: ResponseId::ConnectorTransactionId(
                        payment_response.payment.transaction_id.clone(),
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
            }),

            _ => Err(error_stack::Report::new(
                errors::ConnectorError::ResponseHandlingFailed,
            )),
        }
    }
}

pub fn psync_attempt_status_from_transaction_state(
    getnet_status: GetnetPaymentStatus,
    is_auto_capture: bool,
    transaction_type: GetnetTransactionType,
) -> AttemptStatus {
    match getnet_status {
        GetnetPaymentStatus::Success => {
            if is_auto_capture && transaction_type == GetnetTransactionType::CaptureAuthorization {
                AttemptStatus::Charged
            } else {
                AttemptStatus::Authorized
            }
        }
        GetnetPaymentStatus::InProgress => AttemptStatus::Pending,
        GetnetPaymentStatus::Failed => AttemptStatus::Failure,
    }
}

impl TryFrom<PaymentsSyncResponseRouterData<GetnetPaymentsResponse>> for PaymentsSyncRouterData {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: PaymentsSyncResponseRouterData<GetnetPaymentsResponse>,
    ) -> Result<Self, Self::Error> {
        match item.response {
            GetnetPaymentsResponse::PaymentsResponse(ref payment_response) => Ok(Self {
                status: authorization_attempt_status_from_transaction_state(
                    payment_response.payment.transaction_state.clone(),
                    item.data.request.is_auto_capture()?,
                ),
                response: Ok(PaymentsResponseData::TransactionResponse {
                    resource_id: ResponseId::ConnectorTransactionId(
                        payment_response.payment.transaction_id.clone(),
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
            }),

            GetnetPaymentsResponse::GetnetWebhookNotificationResponse(ref webhook_response) => {
                Ok(Self {
                    status: psync_attempt_status_from_transaction_state(
                        webhook_response.payment.transaction_state.clone(),
                        item.data.request.is_auto_capture()?,
                        webhook_response.payment.transaction_type.clone(),
                    ),
                    response: Ok(PaymentsResponseData::TransactionResponse {
                        resource_id: ResponseId::ConnectorTransactionId(
                            webhook_response.payment.transaction_id.clone(),
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
    }
}

#[derive(Debug, Serialize, PartialEq)]
pub struct CapturePaymentData {
    #[serde(rename = "merchant-account-id")]
    pub merchant_account_id: MerchantAccountId,
    #[serde(rename = "request-id")]
    pub request_id: String,
    #[serde(rename = "transaction-type")]
    pub transaction_type: GetnetTransactionType,
    #[serde(rename = "parent-transaction-id")]
    pub parent_transaction_id: String,
    #[serde(rename = "requested-amount")]
    pub requested_amount: Amount,
    pub notifications: NotificationContainer,
    #[serde(rename = "ip-address")]
    pub ip_address: Option<Secret<String, IpAddress>>,
}

#[derive(Debug, Serialize)]
pub struct GetnetCaptureRequest {
    pub payment: CapturePaymentData,
}
impl TryFrom<&GetnetRouterData<&PaymentsCaptureRouterData>> for GetnetCaptureRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &GetnetRouterData<&PaymentsCaptureRouterData>) -> Result<Self, Self::Error> {
        let request = &item.router_data.request;
        let auth_type = GetnetAuthType::try_from(&item.router_data.connector_auth_type)
            .change_context(errors::ConnectorError::FailedToObtainAuthType)?;
        let merchant_account_id = MerchantAccountId {
            value: auth_type.merchant_id,
        };

        let requested_amount = Amount {
            value: item.amount,
            currency: request.currency,
        };
        let req = &item.router_data.request;
        let webhook_url = &req.webhook_url;
        let notifications = NotificationContainer {
            format: NotificationFormat::JsonSigned,

            notification: vec![Notification {
                url: webhook_url.clone(),
            }],
        };
        let transaction_type = GetnetTransactionType::CaptureAuthorization;
        let ip_address = req
            .browser_info
            .as_ref()
            .and_then(|info| info.ip_address.as_ref())
            .map(|ip| Secret::new(ip.to_string()));
        let request_id = item.router_data.connector_request_reference_id.clone();
        let parent_transaction_id = item.router_data.request.connector_transaction_id.clone();
        let capture_payment_data = CapturePaymentData {
            merchant_account_id,
            request_id,
            transaction_type,
            parent_transaction_id,
            requested_amount,
            notifications,
            ip_address,
        };

        Ok(Self {
            payment: capture_payment_data,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CaptureResponseData {
    pub statuses: Statuses,
    pub descriptor: String,
    pub notifications: NotificationContainer,
    #[serde(rename = "merchant-account-id")]
    pub merchant_account_id: MerchantAccountId,
    #[serde(rename = "transaction-id")]
    pub transaction_id: String,
    #[serde(rename = "request-id")]
    pub request_id: String,
    #[serde(rename = "transaction-type")]
    pub transaction_type: GetnetTransactionType,
    #[serde(rename = "transaction-state")]
    pub transaction_state: GetnetPaymentStatus,
    #[serde(rename = "completion-time-stamp")]
    pub completion_time_stamp: Option<i64>,
    #[serde(rename = "requested-amount")]
    pub requested_amount: Amount,
    #[serde(rename = "parent-transaction-id")]
    pub parent_transaction_id: String,
    #[serde(rename = "account-holder")]
    pub account_holder: Option<AccountHolder>,
    #[serde(rename = "card-token")]
    pub card_token: CardToken,
    #[serde(rename = "ip-address")]
    pub ip_address: Option<Secret<String, IpAddress>>,
    #[serde(rename = "payment-methods")]
    pub payment_methods: PaymentMethodContainer,
    #[serde(rename = "parent-transaction-amount")]
    pub parent_transaction_amount: Amount,
    #[serde(rename = "authorization-code")]
    pub authorization_code: String,
    #[serde(rename = "api-id")]
    pub api_id: String,
    #[serde(rename = "self")]
    pub self_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetnetCaptureResponse {
    payment: CaptureResponseData,
}

pub fn capture_status_from_transaction_state(getnet_status: GetnetPaymentStatus) -> AttemptStatus {
    match getnet_status {
        GetnetPaymentStatus::Success => AttemptStatus::Charged,
        GetnetPaymentStatus::InProgress => AttemptStatus::Pending,
        GetnetPaymentStatus::Failed => AttemptStatus::Authorized,
    }
}

impl<F>
    TryFrom<ResponseRouterData<F, GetnetCaptureResponse, PaymentsCaptureData, PaymentsResponseData>>
    for RouterData<F, PaymentsCaptureData, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<
            F,
            GetnetCaptureResponse,
            PaymentsCaptureData,
            PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            status: capture_status_from_transaction_state(item.response.payment.transaction_state),
            response: Ok(PaymentsResponseData::TransactionResponse {
                resource_id: ResponseId::ConnectorTransactionId(
                    item.response.payment.transaction_id,
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

#[derive(Debug, Serialize, PartialEq)]
pub struct RefundPaymentData {
    #[serde(rename = "merchant-account-id")]
    pub merchant_account_id: MerchantAccountId,
    #[serde(rename = "request-id")]
    pub request_id: String,
    #[serde(rename = "transaction-type")]
    pub transaction_type: GetnetTransactionType,
    #[serde(rename = "parent-transaction-id")]
    pub parent_transaction_id: String,
    pub notifications: NotificationContainer,
    #[serde(rename = "ip-address")]
    pub ip_address: Option<Secret<String, IpAddress>>,
}
#[derive(Debug, Serialize)]
pub struct GetnetRefundRequest {
    pub payment: RefundPaymentData,
}

impl<F> TryFrom<&GetnetRouterData<&RefundsRouterData<F>>> for GetnetRefundRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &GetnetRouterData<&RefundsRouterData<F>>) -> Result<Self, Self::Error> {
        let request = &item.router_data.request;
        let auth_type = GetnetAuthType::try_from(&item.router_data.connector_auth_type)
            .change_context(errors::ConnectorError::FailedToObtainAuthType)?;
        let url = request.webhook_url.clone();

        let merchant_account_id = MerchantAccountId {
            value: auth_type.merchant_id,
        };
        let notifications = NotificationContainer {
            format: NotificationFormat::JsonSigned,
            notification: vec![Notification { url }],
        };
        let capture_method = request.capture_method;
        let transaction_type = match capture_method {
            Some(CaptureMethod::Automatic) => GetnetTransactionType::RefundPurchase,
            Some(CaptureMethod::Manual) => GetnetTransactionType::RefundCapture,
            Some(CaptureMethod::ManualMultiple)
            | Some(CaptureMethod::Scheduled)
            | Some(CaptureMethod::SequentialAutomatic)
            | None => {
                return Err(errors::ConnectorError::CaptureMethodNotSupported {}.into());
            }
        };
        let ip_address = request
            .browser_info
            .as_ref()
            .and_then(|browser_info| browser_info.ip_address.as_ref())
            .map(|ip| Secret::new(ip.to_string()));
        let request_id = item
            .router_data
            .refund_id
            .clone()
            .ok_or(errors::ConnectorError::MissingConnectorRefundID)?;

        let parent_transaction_id = item.router_data.request.connector_transaction_id.clone();
        let refund_payment_data = RefundPaymentData {
            merchant_account_id,
            request_id,
            transaction_type,
            parent_transaction_id,
            notifications,
            ip_address,
        };

        Ok(Self {
            payment: refund_payment_data,
        })
    }
}

#[allow(dead_code)]
#[derive(Debug, Serialize, Default, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum RefundStatus {
    Success,
    Failed,
    #[default]
    InProgress,
}

impl From<RefundStatus> for enums::RefundStatus {
    fn from(item: RefundStatus) -> Self {
        match item {
            RefundStatus::Success => Self::Success,
            RefundStatus::Failed => Self::Failure,
            RefundStatus::InProgress => Self::Pending,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RefundResponseData {
    pub statuses: Statuses,
    pub descriptor: String,
    pub notifications: NotificationContainer,
    #[serde(rename = "merchant-account-id")]
    pub merchant_account_id: MerchantAccountId,
    #[serde(rename = "transaction-id")]
    pub transaction_id: String,
    #[serde(rename = "request-id")]
    pub request_id: String,
    #[serde(rename = "transaction-type")]
    pub transaction_type: GetnetTransactionType,
    #[serde(rename = "transaction-state")]
    pub transaction_state: RefundStatus,
    #[serde(rename = "completion-time-stamp")]
    pub completion_time_stamp: Option<i64>,
    #[serde(rename = "requested-amount")]
    pub requested_amount: Amount,
    #[serde(rename = "parent-transaction-id")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_transaction_id: Option<String>,
    #[serde(rename = "account-holder")]
    pub account_holder: Option<AccountHolder>,
    #[serde(rename = "card-token")]
    pub card_token: CardToken,
    #[serde(rename = "ip-address")]
    pub ip_address: Option<Secret<String, IpAddress>>,
    #[serde(rename = "payment-methods")]
    pub payment_methods: PaymentMethodContainer,
    #[serde(rename = "parent-transaction-amount")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_transaction_amount: Option<Amount>,
    #[serde(rename = "api-id")]
    pub api_id: String,
    #[serde(rename = "self")]
    pub self_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefundResponse {
    payment: RefundResponseData,
}

impl TryFrom<RefundsResponseRouterData<Execute, RefundResponse>> for RefundsRouterData<Execute> {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: RefundsResponseRouterData<Execute, RefundResponse>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(RefundsResponseData {
                connector_refund_id: item.response.payment.transaction_id,
                refund_status: enums::RefundStatus::from(item.response.payment.transaction_state),
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
                connector_refund_id: item.response.payment.transaction_id,
                refund_status: enums::RefundStatus::from(item.response.payment.transaction_state),
            }),
            ..item.data
        })
    }
}

#[derive(Debug, Serialize, PartialEq)]
pub struct CancelPaymentData {
    #[serde(rename = "merchant-account-id")]
    pub merchant_account_id: MerchantAccountId,
    #[serde(rename = "request-id")]
    pub request_id: String,
    #[serde(rename = "transaction-type")]
    pub transaction_type: GetnetTransactionType,
    #[serde(rename = "parent-transaction-id")]
    pub parent_transaction_id: String,
    pub notifications: NotificationContainer,
    #[serde(rename = "ip-address")]
    pub ip_address: Option<Secret<String, IpAddress>>,
}

#[derive(Debug, Serialize)]
pub struct GetnetCancelRequest {
    pub payment: CancelPaymentData,
}

impl TryFrom<&PaymentsCancelRouterData> for GetnetCancelRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &PaymentsCancelRouterData) -> Result<Self, Self::Error> {
        let request = &item.request;
        let auth_type = GetnetAuthType::try_from(&item.connector_auth_type)
            .change_context(errors::ConnectorError::FailedToObtainAuthType)?;

        let merchant_account_id = MerchantAccountId {
            value: auth_type.merchant_id,
        };
        let webhook_url = &item.request.webhook_url;
        let notifications = NotificationContainer {
            format: NotificationFormat::JsonSigned,

            notification: vec![Notification {
                url: webhook_url.clone(),
            }],
        };
        let capture_method = &item.request.capture_method;
        let transaction_type = match capture_method {
            Some(CaptureMethod::Automatic) => GetnetTransactionType::VoidPurchase,
            Some(CaptureMethod::Manual) => GetnetTransactionType::VoidAuthorization,
            Some(CaptureMethod::ManualMultiple)
            | Some(CaptureMethod::Scheduled)
            | Some(CaptureMethod::SequentialAutomatic) => {
                return Err(errors::ConnectorError::CaptureMethodNotSupported {}.into());
            }
            None => {
                return Err(errors::ConnectorError::CaptureMethodNotSupported {}.into());
            }
        };
        let ip_address = request
            .browser_info
            .as_ref()
            .and_then(|browser_info| browser_info.ip_address.as_ref())
            .map(|ip| Secret::new(ip.to_string()));
        let request_id = &item.connector_request_reference_id.clone();
        let parent_transaction_id = item.request.connector_transaction_id.clone();
        let cancel_payment_data = CancelPaymentData {
            merchant_account_id,
            request_id: request_id.to_string(),
            transaction_type,
            parent_transaction_id,
            notifications,
            ip_address,
        };
        Ok(Self {
            payment: cancel_payment_data,
        })
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum GetnetTransactionType {
    Purchase,
    #[serde(rename = "capture-authorization")]
    CaptureAuthorization,
    #[serde(rename = "refund-purchase")]
    RefundPurchase,
    #[serde(rename = "refund-capture")]
    RefundCapture,
    #[serde(rename = "void-authorization")]
    VoidAuthorization,
    #[serde(rename = "void-purchase")]
    VoidPurchase,
    Authorization,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub struct CancelResponseData {
    pub statuses: Statuses,
    pub descriptor: String,
    pub notifications: NotificationContainer,
    #[serde(rename = "merchant-account-id")]
    pub merchant_account_id: MerchantAccountId,
    #[serde(rename = "transaction-id")]
    pub transaction_id: String,
    #[serde(rename = "request-id")]
    pub request_id: String,
    #[serde(rename = "transaction-type")]
    pub transaction_type: GetnetTransactionType,
    #[serde(rename = "transaction-state")]
    pub transaction_state: GetnetPaymentStatus,
    #[serde(rename = "completion-time-stamp")]
    pub completion_time_stamp: Option<i64>,
    #[serde(rename = "requested-amount")]
    pub requested_amount: Amount,
    #[serde(rename = "parent-transaction-id")]
    pub parent_transaction_id: String,
    #[serde(rename = "account-holder")]
    pub account_holder: Option<AccountHolder>,
    #[serde(rename = "card-token")]
    pub card_token: CardToken,
    #[serde(rename = "ip-address")]
    pub ip_address: Option<Secret<String, IpAddress>>,
    #[serde(rename = "payment-methods")]
    pub payment_methods: PaymentMethodContainer,
    #[serde(rename = "parent-transaction-amount")]
    pub parent_transaction_amount: Amount,
    #[serde(rename = "api-id")]
    pub api_id: String,
    #[serde(rename = "self")]
    pub self_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetnetCancelResponse {
    payment: CancelResponseData,
}

pub fn cancel_status_from_transaction_state(getnet_status: GetnetPaymentStatus) -> AttemptStatus {
    match getnet_status {
        GetnetPaymentStatus::Success => AttemptStatus::Voided,
        GetnetPaymentStatus::InProgress => AttemptStatus::Pending,
        GetnetPaymentStatus::Failed => AttemptStatus::VoidFailed,
    }
}

impl<F>
    TryFrom<ResponseRouterData<F, GetnetCancelResponse, PaymentsCancelData, PaymentsResponseData>>
    for RouterData<F, PaymentsCancelData, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, GetnetCancelResponse, PaymentsCancelData, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            status: cancel_status_from_transaction_state(item.response.payment.transaction_state),
            response: Ok(PaymentsResponseData::TransactionResponse {
                resource_id: ResponseId::ConnectorTransactionId(
                    item.response.payment.transaction_id,
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

#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
pub struct GetnetErrorResponse {
    pub status_code: u16,
    pub code: String,
    pub message: String,
    pub reason: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct GetnetWebhookNotificationResponse {
    #[serde(rename = "response-signature-base64")]
    pub response_signature_base64: Secret<String>,
    #[serde(rename = "response-signature-algorithm")]
    pub response_signature_algorithm: Secret<String>,
    #[serde(rename = "response-base64")]
    pub response_base64: Secret<String>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct WebhookResponseData {
    pub statuses: Statuses,
    pub descriptor: String,
    pub notifications: NotificationContainer,
    #[serde(rename = "merchant-account-id")]
    pub merchant_account_id: MerchantAccountId,
    #[serde(rename = "transaction-id")]
    pub transaction_id: String,
    #[serde(rename = "request-id")]
    pub request_id: String,
    #[serde(rename = "transaction-type")]
    pub transaction_type: GetnetTransactionType,
    #[serde(rename = "transaction-state")]
    pub transaction_state: GetnetPaymentStatus,
    #[serde(rename = "completion-time-stamp")]
    pub completion_time_stamp: u64,
    #[serde(rename = "requested-amount")]
    pub requested_amount: Amount,
    #[serde(rename = "parent-transaction-id")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_transaction_id: Option<String>,
    #[serde(rename = "account-holder")]
    pub account_holder: Option<AccountHolder>,
    #[serde(rename = "card-token")]
    pub card_token: CardToken,
    #[serde(rename = "ip-address")]
    pub ip_address: Option<Secret<String, IpAddress>>,
    #[serde(rename = "payment-methods")]
    pub payment_methods: PaymentMethodContainer,
    #[serde(rename = "parent-transaction-amount")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_transaction_amount: Option<Amount>,
    #[serde(rename = "authorization-code")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub authorization_code: Option<String>,
    #[serde(rename = "api-id")]
    pub api_id: String,
    #[serde(rename = "provider-account-id")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider_account_id: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct GetnetWebhookNotificationResponseBody {
    pub payment: WebhookResponseData,
}

pub fn is_refund_event(transaction_type: &GetnetTransactionType) -> bool {
    matches!(
        transaction_type,
        GetnetTransactionType::RefundPurchase | GetnetTransactionType::RefundCapture
    )
}

pub fn get_webhook_object_from_body(
    body: &[u8],
) -> CustomResult<GetnetWebhookNotificationResponseBody, errors::ConnectorError> {
    let body_bytes = bytes::Bytes::copy_from_slice(body);
    let parsed_param: GetnetWebhookNotificationResponse =
        parse_url_encoded_to_struct(body_bytes)
            .change_context(errors::ConnectorError::WebhookBodyDecodingFailed)?;
    let response_base64 = &parsed_param.response_base64.peek();
    let decoded_response = BASE64_ENGINE
        .decode(response_base64)
        .change_context(errors::ConnectorError::WebhookBodyDecodingFailed)?;

    let getnet_webhook_notification_response: GetnetWebhookNotificationResponseBody =
        match serde_json::from_slice::<GetnetWebhookNotificationResponseBody>(&decoded_response) {
            Ok(response) => response,
            Err(_e) => {
                return Err(errors::ConnectorError::WebhookBodyDecodingFailed)?;
            }
        };

    Ok(getnet_webhook_notification_response)
}

pub fn get_webhook_response(
    body: &[u8],
) -> CustomResult<GetnetWebhookNotificationResponse, errors::ConnectorError> {
    let body_bytes = bytes::Bytes::copy_from_slice(body);
    let parsed_param: GetnetWebhookNotificationResponse =
        parse_url_encoded_to_struct(body_bytes)
            .change_context(errors::ConnectorError::WebhookBodyDecodingFailed)?;
    Ok(parsed_param)
}

pub fn get_incoming_webhook_event(
    transaction_type: GetnetTransactionType,
    transaction_status: GetnetPaymentStatus,
) -> IncomingWebhookEvent {
    match transaction_type {
        GetnetTransactionType::Purchase => match transaction_status {
            GetnetPaymentStatus::Success => IncomingWebhookEvent::PaymentIntentSuccess,
            GetnetPaymentStatus::Failed => IncomingWebhookEvent::PaymentIntentFailure,
            GetnetPaymentStatus::InProgress => IncomingWebhookEvent::PaymentIntentProcessing,
        },

        GetnetTransactionType::Authorization => match transaction_status {
            GetnetPaymentStatus::Success => IncomingWebhookEvent::PaymentIntentAuthorizationSuccess,
            GetnetPaymentStatus::Failed => IncomingWebhookEvent::PaymentIntentAuthorizationFailure,
            GetnetPaymentStatus::InProgress => IncomingWebhookEvent::PaymentIntentProcessing,
        },

        GetnetTransactionType::CaptureAuthorization => match transaction_status {
            GetnetPaymentStatus::Success => IncomingWebhookEvent::PaymentIntentCaptureSuccess,
            GetnetPaymentStatus::Failed => IncomingWebhookEvent::PaymentIntentCaptureFailure,
            GetnetPaymentStatus::InProgress => IncomingWebhookEvent::PaymentIntentCaptureFailure,
        },

        GetnetTransactionType::RefundPurchase => match transaction_status {
            GetnetPaymentStatus::Success => IncomingWebhookEvent::RefundSuccess,
            GetnetPaymentStatus::Failed => IncomingWebhookEvent::RefundFailure,
            GetnetPaymentStatus::InProgress => IncomingWebhookEvent::RefundFailure,
        },

        GetnetTransactionType::RefundCapture => match transaction_status {
            GetnetPaymentStatus::Success => IncomingWebhookEvent::RefundSuccess,
            GetnetPaymentStatus::Failed => IncomingWebhookEvent::RefundFailure,
            GetnetPaymentStatus::InProgress => IncomingWebhookEvent::RefundFailure,
        },

        GetnetTransactionType::VoidAuthorization => match transaction_status {
            GetnetPaymentStatus::Success => IncomingWebhookEvent::PaymentIntentCancelled,
            GetnetPaymentStatus::Failed => IncomingWebhookEvent::PaymentIntentCancelFailure,
            GetnetPaymentStatus::InProgress => IncomingWebhookEvent::PaymentIntentCancelFailure,
        },

        GetnetTransactionType::VoidPurchase => match transaction_status {
            GetnetPaymentStatus::Success => IncomingWebhookEvent::PaymentIntentCancelled,
            GetnetPaymentStatus::Failed => IncomingWebhookEvent::PaymentIntentCancelFailure,
            GetnetPaymentStatus::InProgress => IncomingWebhookEvent::PaymentIntentCancelFailure,
        },
    }
}
