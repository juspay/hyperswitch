use std::collections::HashMap;

use api_models::enums::PaymentMethod;
use masking::{Secret, SwitchStrategy};
use serde::{Deserialize, Serialize};

use crate::{
    connector::utils::{self, PaymentsAuthorizeRequestData, RefundsRequestData, RouterData},
    core::errors,
    services,
    types::{self, api, storage::enums, PaymentsAuthorizeData},
};

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
        (_currency_unit, _currency, _amount, item): (
            &types::api::CurrencyUnit,
            types::storage::enums::Currency,
            i64,
            T,
        ),
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            amount: utils::to_currency_base_unit_asf64(_amount, _currency)?,
            router_data: item,
        })
    }
}
#[derive(Debug, Deserialize)]
pub struct IatapayAuthUpdateResponse {
    pub access_token: Secret<String>,
    pub token_type: String,
    pub expires_in: i64,
    pub scope: String,
    pub jti: String,
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
            api::PaymentMethodData::Upi(upi_data) => upi_data.vpa_id.map(|id| PayerInfo {
                token_id: id.switch_strategy(),
            }),
            _ => None,
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
    Tobeinvestigated,
    Blocked,
    Cleared,
    Failed,
    Locked,
    #[serde(rename = "UNEXPECTED SETTLED")]
    UnexpectedSettled,
    #[serde(other)]
    Unknown,
}

impl From<IatapayPaymentStatus> for enums::AttemptStatus {
    fn from(item: IatapayPaymentStatus) -> Self {
        match item {
            IatapayPaymentStatus::Authorized | IatapayPaymentStatus::Settled => Self::Charged,
            IatapayPaymentStatus::Failed | IatapayPaymentStatus::UnexpectedSettled => Self::Failure,
            IatapayPaymentStatus::Created => Self::AuthenticationPending,
            IatapayPaymentStatus::Initiated => Self::Pending,
            _ => Self::Voided,
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
    pub merchant_id: Option<String>,
    pub merchant_payment_id: Option<String>,
    pub amount: f64,
    pub currency: String,
    pub country: Option<String>,
    pub locale: Option<String>,
    pub bank_transfer_description: Option<String>,
    pub checkout_methods: Option<CheckoutMethod>,
    pub failure_code: Option<String>,
}

impl<F, T>
    TryFrom<types::ResponseRouterData<F, IatapayPaymentsResponse, T, types::PaymentsResponseData>>
    for types::RouterData<F, T, types::PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<F, IatapayPaymentsResponse, T, types::PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        let form_fields = HashMap::new();
        let id = match item.response.iata_payment_id.clone() {
            Some(s) => types::ResponseId::ConnectorTransactionId(s),
            None => types::ResponseId::NoResponseId,
        };
        let connector_response_reference_id = item
            .response
            .merchant_payment_id
            .or(item.response.iata_payment_id);
        Ok(Self {
            status: enums::AttemptStatus::from(item.response.status),
            response: item.response.checkout_methods.map_or(
                Ok(types::PaymentsResponseData::TransactionResponse {
                    resource_id: id.clone(),
                    redirection_data: None,
                    mandate_reference: None,
                    connector_metadata: None,
                    network_txn_id: None,
                    connector_response_reference_id: connector_response_reference_id.clone(),
                }),
                |checkout_methods| {
                    Ok(types::PaymentsResponseData::TransactionResponse {
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
                    })
                },
            ),
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
    merchant_id: Option<String>,
    account_country: Option<String>,
}

impl TryFrom<types::RefundsResponseRouterData<api::Execute, RefundResponse>>
    for types::RefundsRouterData<api::Execute>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::RefundsResponseRouterData<api::Execute, RefundResponse>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(types::RefundsResponseData {
                connector_refund_id: item.response.iata_refund_id.to_string(),
                refund_status: enums::RefundStatus::from(item.response.status),
            }),
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
        Ok(Self {
            response: Ok(types::RefundsResponseData {
                connector_refund_id: item.response.iata_refund_id.to_string(),
                refund_status: enums::RefundStatus::from(item.response.status),
            }),
            ..item.data
        })
    }
}

#[derive(Debug, Deserialize)]
pub struct IatapayErrorResponse {
    pub status: u16,
    pub error: String,
    pub message: String,
    pub reason: Option<String>,
}

#[derive(Deserialize, Debug)]
pub struct IatapayAccessTokenErrorResponse {
    pub error: String,
    pub path: String,
}
