use std::collections::HashMap;

use api_models::enums::PaymentMethod;
use common_utils::errors::CustomResult;
use masking::{Secret, SwitchStrategy};
use serde::{Deserialize, Serialize};

use crate::{
    connector::utils::{
        self as connector_util, PaymentsAuthorizeRequestData, RefundsRequestData, RouterData,
    },
    consts,
    core::errors,
    services,
    types::{self, api, domain, storage::enums, PaymentsAuthorizeData},
};

type Error = error_stack::Report<errors::ConnectorError>;

// Every access token will be valid for 5 minutes. It contains grant_type and scope for different type of access, but for our usecases it should be only 'client_credentials' and 'payment' resp(as per doc) for all type of api call.
#[derive(Debug, Serialize)]
pub struct IatapayAuthUpdateRequest {
    grant_type: String,
    scope: String,
}
impl TryFrom<&types::RefreshTokenRouterData> for IatapayAuthUpdateRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(_item: &types::RefreshTokenRouterData) -> Result<Self, Self::Error> {
        Ok(Self {
            grant_type: "client_credentials".to_string(),
            scope: "payment".to_string(),
        })
    }
}
#[derive(Debug, Serialize)]
pub struct IatapayRouterData<T> {
    amount: f64,
    router_data: T,
}
impl<T>
    TryFrom<(
        &types::api::CurrencyUnit,
        types::storage::enums::Currency,
        i64,
        T,
    )> for IatapayRouterData<T>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        (currency_unit, currency, amount, item): (
            &types::api::CurrencyUnit,
            types::storage::enums::Currency,
            i64,
            T,
        ),
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            amount: connector_util::get_amount_as_f64(currency_unit, amount, currency)?,
            router_data: item,
        })
    }
}
#[derive(Debug, Deserialize, Serialize)]
pub struct IatapayAuthUpdateResponse {
    pub access_token: Secret<String>,
    pub expires_in: i64,
}

impl<F, T> TryFrom<types::ResponseRouterData<F, IatapayAuthUpdateResponse, T, types::AccessToken>>
    for types::RouterData<F, T, types::AccessToken>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<F, IatapayAuthUpdateResponse, T, types::AccessToken>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(types::AccessToken {
                token: item.response.access_token,
                expires: item.response.expires_in,
            }),
            ..item.data
        })
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RedirectUrls {
    success_url: String,
    failure_url: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PayerInfo {
    token_id: Secret<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IatapayPaymentsRequest {
    merchant_id: Secret<String>,
    merchant_payment_id: Option<String>,
    amount: f64,
    currency: String,
    country: String,
    locale: String,
    redirect_urls: RedirectUrls,
    notification_url: String,
    payer_info: Option<PayerInfo>,
}

impl
    TryFrom<
        &IatapayRouterData<
            &types::RouterData<
                types::api::payments::Authorize,
                PaymentsAuthorizeData,
                types::PaymentsResponseData,
            >,
        >,
    > for IatapayPaymentsRequest
{
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(
        item: &IatapayRouterData<
            &types::RouterData<
                types::api::payments::Authorize,
                PaymentsAuthorizeData,
                types::PaymentsResponseData,
            >,
        >,
    ) -> Result<Self, Self::Error> {
        let payment_method = item.router_data.payment_method;
        let country = match payment_method {
            PaymentMethod::Upi => "IN".to_string(),
            PaymentMethod::Card
            | PaymentMethod::CardRedirect
            | PaymentMethod::PayLater
            | PaymentMethod::Wallet
            | PaymentMethod::BankRedirect
            | PaymentMethod::BankTransfer
            | PaymentMethod::Crypto
            | PaymentMethod::BankDebit
            | PaymentMethod::Reward
            | PaymentMethod::Voucher
            | PaymentMethod::GiftCard => item.router_data.get_billing_country()?.to_string(),
        };
        let return_url = item.router_data.get_return_url()?;
        let payer_info = match item.router_data.request.payment_method_data.clone() {
            domain::PaymentMethodData::Upi(upi_data) => upi_data.vpa_id.map(|id| PayerInfo {
                token_id: id.switch_strategy(),
            }),
            domain::PaymentMethodData::Card(_)
            | domain::PaymentMethodData::CardRedirect(_)
            | domain::PaymentMethodData::Wallet(_)
            | domain::PaymentMethodData::PayLater(_)
            | domain::PaymentMethodData::BankRedirect(_)
            | domain::PaymentMethodData::BankDebit(_)
            | domain::PaymentMethodData::BankTransfer(_)
            | domain::PaymentMethodData::Crypto(_)
            | domain::PaymentMethodData::MandatePayment
            | domain::PaymentMethodData::Reward
            | domain::PaymentMethodData::Voucher(_)
            | domain::PaymentMethodData::GiftCard(_)
            | domain::PaymentMethodData::CardToken(_) => None,
        };
        let payload = Self {
            merchant_id: IatapayAuthType::try_from(&item.router_data.connector_auth_type)?
                .merchant_id,
            merchant_payment_id: Some(item.router_data.connector_request_reference_id.clone()),
            amount: item.amount,
            currency: item.router_data.request.currency.to_string(),
            country: country.clone(),
            locale: format!("en-{}", country),
            redirect_urls: get_redirect_url(return_url),
            payer_info,
            notification_url: item.router_data.request.get_webhook_url()?,
        };
        Ok(payload)
    }
}

fn get_redirect_url(return_url: String) -> RedirectUrls {
    RedirectUrls {
        success_url: return_url.clone(),
        failure_url: return_url,
    }
}

// Auth Struct
pub struct IatapayAuthType {
    pub(super) client_id: Secret<String>,
    pub(super) merchant_id: Secret<String>,
    pub(super) client_secret: Secret<String>,
}

impl TryFrom<&types::ConnectorAuthType> for IatapayAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &types::ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            types::ConnectorAuthType::SignatureKey {
                api_key,
                key1,
                api_secret,
            } => Ok(Self {
                client_id: api_key.to_owned(),
                merchant_id: key1.to_owned(),
                client_secret: api_secret.to_owned(),
            }),
            _ => Err(errors::ConnectorError::FailedToObtainAuthType)?,
        }
    }
}
// PaymentsResponse
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum IatapayPaymentStatus {
    #[default]
    Created,
    Initiated,
    Authorized,
    Settled,
    Cleared,
    Failed,
    #[serde(rename = "UNEXPECTED SETTLED")]
    UnexpectedSettled,
}

impl From<IatapayPaymentStatus> for enums::AttemptStatus {
    fn from(item: IatapayPaymentStatus) -> Self {
        match item {
            IatapayPaymentStatus::Authorized
            | IatapayPaymentStatus::Settled
            | IatapayPaymentStatus::Cleared => Self::Charged,
            IatapayPaymentStatus::Failed | IatapayPaymentStatus::UnexpectedSettled => Self::Failure,
            IatapayPaymentStatus::Created => Self::AuthenticationPending,
            IatapayPaymentStatus::Initiated => Self::Pending,
        }
    }
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RedirectUrl {
    pub redirect_url: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CheckoutMethod {
    pub redirect: RedirectUrl,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IatapayPaymentsResponse {
    pub status: IatapayPaymentStatus,
    pub iata_payment_id: Option<String>,
    pub iata_refund_id: Option<String>,
    pub merchant_id: Option<Secret<String>>,
    pub merchant_payment_id: Option<String>,
    pub amount: f64,
    pub currency: String,
    pub checkout_methods: Option<CheckoutMethod>,
    pub failure_code: Option<String>,
    pub failure_details: Option<String>,
}

fn get_iatpay_response(
    response: IatapayPaymentsResponse,
    status_code: u16,
) -> CustomResult<
    (
        enums::AttemptStatus,
        Option<types::ErrorResponse>,
        types::PaymentsResponseData,
    ),
    errors::ConnectorError,
> {
    let status = enums::AttemptStatus::from(response.status);
    let error = if connector_util::is_payment_failure(status) {
        Some(types::ErrorResponse {
            code: response
                .failure_code
                .unwrap_or_else(|| consts::NO_ERROR_CODE.to_string()),
            message: response
                .failure_details
                .clone()
                .unwrap_or_else(|| consts::NO_ERROR_MESSAGE.to_string()),
            reason: response.failure_details,
            status_code,
            attempt_status: Some(status),
            connector_transaction_id: response.iata_payment_id.clone(),
        })
    } else {
        None
    };
    let form_fields = HashMap::new();
    let id = match response.iata_payment_id.clone() {
        Some(s) => types::ResponseId::ConnectorTransactionId(s),
        None => types::ResponseId::NoResponseId,
    };
    let connector_response_reference_id = response.merchant_payment_id.or(response.iata_payment_id);

    let payment_response_data = response.checkout_methods.map_or(
        types::PaymentsResponseData::TransactionResponse {
            resource_id: id.clone(),
            redirection_data: None,
            mandate_reference: None,
            connector_metadata: None,
            network_txn_id: None,
            connector_response_reference_id: connector_response_reference_id.clone(),
            incremental_authorization_allowed: None,
        },
        |checkout_methods| types::PaymentsResponseData::TransactionResponse {
            resource_id: id,
            redirection_data: Some(services::RedirectForm::Form {
                endpoint: checkout_methods.redirect.redirect_url,
                method: services::Method::Get,
                form_fields,
            }),
            mandate_reference: None,
            connector_metadata: None,
            network_txn_id: None,
            connector_response_reference_id: connector_response_reference_id.clone(),
            incremental_authorization_allowed: None,
        },
    );
    Ok((status, error, payment_response_data))
}

impl<F, T>
    TryFrom<types::ResponseRouterData<F, IatapayPaymentsResponse, T, types::PaymentsResponseData>>
    for types::RouterData<F, T, types::PaymentsResponseData>
{
    type Error = Error;
    fn try_from(
        item: types::ResponseRouterData<F, IatapayPaymentsResponse, T, types::PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        let (status, error, payment_response_data) =
            get_iatpay_response(item.response, item.http_code)?;
        Ok(Self {
            status,
            response: error.map_or_else(|| Ok(payment_response_data), Err),
            ..item.data
        })
    }
}

// REFUND :
// Type definition for RefundRequest
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IatapayRefundRequest {
    pub merchant_id: Secret<String>,
    pub merchant_refund_id: Option<String>,
    pub amount: f64,
    pub currency: String,
    pub bank_transfer_description: Option<String>,
    pub notification_url: String,
}

impl<F> TryFrom<&IatapayRouterData<&types::RefundsRouterData<F>>> for IatapayRefundRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &IatapayRouterData<&types::RefundsRouterData<F>>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            amount: item.amount,
            merchant_id: IatapayAuthType::try_from(&item.router_data.connector_auth_type)?
                .merchant_id,
            merchant_refund_id: Some(item.router_data.request.refund_id.clone()),
            currency: item.router_data.request.currency.to_string(),
            bank_transfer_description: item.router_data.request.reason.clone(),
            notification_url: item.router_data.request.get_webhook_url()?,
        })
    }
}

// Type definition for Refund Response

#[allow(dead_code)]
#[derive(Debug, Serialize, Default, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum RefundStatus {
    #[default]
    Created,
    Locked,
    Initiated,
    Authorized,
    Settled,
    Failed,
}

impl From<RefundStatus> for enums::RefundStatus {
    fn from(item: RefundStatus) -> Self {
        match item {
            RefundStatus::Created => Self::Pending,
            RefundStatus::Failed => Self::Failure,
            RefundStatus::Locked => Self::Pending,
            RefundStatus::Initiated => Self::Pending,
            RefundStatus::Authorized => Self::Pending,
            RefundStatus::Settled => Self::Success,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RefundResponse {
    iata_refund_id: String,
    status: RefundStatus,
    merchant_refund_id: String,
    amount: f64,
    currency: String,
    bank_transfer_description: Option<String>,
    failure_code: Option<String>,
    failure_details: Option<String>,
    lock_reason: Option<String>,
    creation_date_time: Option<String>,
    finish_date_time: Option<String>,
    update_date_time: Option<String>,
    clearance_date_time: Option<String>,
    iata_payment_id: Option<String>,
    merchant_payment_id: Option<String>,
    payment_amount: Option<f64>,
    merchant_id: Option<Secret<String>>,
    account_country: Option<String>,
}

impl TryFrom<types::RefundsResponseRouterData<api::Execute, RefundResponse>>
    for types::RefundsRouterData<api::Execute>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::RefundsResponseRouterData<api::Execute, RefundResponse>,
    ) -> Result<Self, Self::Error> {
        let refund_status = enums::RefundStatus::from(item.response.status);
        let response = if connector_util::is_refund_failure(refund_status) {
            Err(types::ErrorResponse {
                code: item
                    .response
                    .failure_code
                    .unwrap_or_else(|| consts::NO_ERROR_CODE.to_string()),
                message: item
                    .response
                    .failure_details
                    .clone()
                    .unwrap_or_else(|| consts::NO_ERROR_MESSAGE.to_string()),
                reason: item.response.failure_details,
                status_code: item.http_code,
                attempt_status: None,
                connector_transaction_id: Some(item.response.iata_refund_id.clone()),
            })
        } else {
            Ok(types::RefundsResponseData {
                connector_refund_id: item.response.iata_refund_id.to_string(),
                refund_status,
            })
        };

        Ok(Self {
            response,
            ..item.data
        })
    }
}

impl TryFrom<types::RefundsResponseRouterData<api::RSync, RefundResponse>>
    for types::RefundsRouterData<api::RSync>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::RefundsResponseRouterData<api::RSync, RefundResponse>,
    ) -> Result<Self, Self::Error> {
        let refund_status = enums::RefundStatus::from(item.response.status);
        let response = if connector_util::is_refund_failure(refund_status) {
            Err(types::ErrorResponse {
                code: item
                    .response
                    .failure_code
                    .unwrap_or_else(|| consts::NO_ERROR_CODE.to_string()),
                message: item
                    .response
                    .failure_details
                    .clone()
                    .unwrap_or_else(|| consts::NO_ERROR_MESSAGE.to_string()),
                reason: item.response.failure_details,
                status_code: item.http_code,
                attempt_status: None,
                connector_transaction_id: Some(item.response.iata_refund_id.clone()),
            })
        } else {
            Ok(types::RefundsResponseData {
                connector_refund_id: item.response.iata_refund_id.to_string(),
                refund_status,
            })
        };
        Ok(Self {
            response,
            ..item.data
        })
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct IatapayErrorResponse {
    pub status: u16,
    pub error: String,
    pub message: String,
    pub reason: Option<String>,
}

#[derive(Deserialize, Debug, Serialize)]
pub struct IatapayAccessTokenErrorResponse {
    pub error: String,
    pub path: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IatapayPaymentWebhookBody {
    pub status: IatapayWebhookStatus,
    pub iata_payment_id: String,
    pub merchant_payment_id: Option<String>,
    pub failure_code: Option<String>,
    pub failure_details: Option<String>,
    pub amount: f64,
    pub currency: String,
    pub checkout_methods: Option<CheckoutMethod>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IatapayRefundWebhookBody {
    pub status: IatapayRefundWebhookStatus,
    pub iata_refund_id: String,
    pub merchant_refund_id: Option<String>,
    pub failure_code: Option<String>,
    pub failure_details: Option<String>,
    pub amount: f64,
    pub currency: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum IatapayWebhookResponse {
    IatapayPaymentWebhookBody(IatapayPaymentWebhookBody),
    IatapayRefundWebhookBody(IatapayRefundWebhookBody),
}

impl TryFrom<IatapayWebhookResponse> for api::IncomingWebhookEvent {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(payload: IatapayWebhookResponse) -> CustomResult<Self, errors::ConnectorError> {
        match payload {
            IatapayWebhookResponse::IatapayPaymentWebhookBody(wh_body) => match wh_body.status {
                IatapayWebhookStatus::Authorized | IatapayWebhookStatus::Settled => {
                    Ok(Self::PaymentIntentSuccess)
                }
                IatapayWebhookStatus::Initiated => Ok(Self::PaymentIntentProcessing),
                IatapayWebhookStatus::Failed => Ok(Self::PaymentIntentFailure),
                IatapayWebhookStatus::Created
                | IatapayWebhookStatus::Cleared
                | IatapayWebhookStatus::Tobeinvestigated
                | IatapayWebhookStatus::Blocked
                | IatapayWebhookStatus::UnexpectedSettled
                | IatapayWebhookStatus::Unknown => Ok(Self::EventNotSupported),
            },
            IatapayWebhookResponse::IatapayRefundWebhookBody(wh_body) => match wh_body.status {
                IatapayRefundWebhookStatus::Authorized | IatapayRefundWebhookStatus::Settled => {
                    Ok(Self::RefundSuccess)
                }
                IatapayRefundWebhookStatus::Failed => Ok(Self::RefundFailure),
                IatapayRefundWebhookStatus::Created
                | IatapayRefundWebhookStatus::Locked
                | IatapayRefundWebhookStatus::Initiated
                | IatapayRefundWebhookStatus::Unknown => Ok(Self::EventNotSupported),
            },
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum IatapayWebhookStatus {
    Created,
    Initiated,
    Authorized,
    Settled,
    Cleared,
    Failed,
    Tobeinvestigated,
    Blocked,
    #[serde(rename = "UNEXPECTED SETTLED")]
    UnexpectedSettled,
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum IatapayRefundWebhookStatus {
    Created,
    Initiated,
    Authorized,
    Settled,
    Failed,
    Locked,
    #[serde(other)]
    Unknown,
}
