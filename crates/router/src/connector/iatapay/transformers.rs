use std::collections::HashMap;

use common_utils::{errors::CustomResult, ext_traits::Encode, types::FloatMajorUnit};
use error_stack::ResultExt;
use masking::{Secret, SwitchStrategy};
use serde::{Deserialize, Serialize};

use crate::{
    connector::utils::{self as connector_util, PaymentsAuthorizeRequestData, RefundsRequestData},
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
    amount: FloatMajorUnit,
    router_data: T,
}
impl<T> TryFrom<(FloatMajorUnit, T)> for IatapayRouterData<T> {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from((amount, item): (FloatMajorUnit, T)) -> Result<Self, Self::Error> {
        Ok(Self {
            amount,
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
#[serde(rename_all = "UPPERCASE")]
pub enum PreferredCheckoutMethod {
    Vpa, //Passing this in UPI_COLLECT will trigger an S2S payment call which is not required.
    Qr,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IatapayPaymentsRequest {
    merchant_id: Secret<String>,
    merchant_payment_id: Option<String>,
    amount: FloatMajorUnit,
    currency: common_enums::Currency,
    country: common_enums::CountryAlpha2,
    locale: String,
    redirect_urls: RedirectUrls,
    notification_url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    payer_info: Option<PayerInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    preferred_checkout_method: Option<PreferredCheckoutMethod>,
}

impl
    TryFrom<
        &IatapayRouterData<
            &types::RouterData<
                api::payments::Authorize,
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
                api::payments::Authorize,
                PaymentsAuthorizeData,
                types::PaymentsResponseData,
            >,
        >,
    ) -> Result<Self, Self::Error> {
        let return_url = item.router_data.request.get_router_return_url()?;
        // Iatapay processes transactions through the payment method selected based on the country
        let (country, payer_info, preferred_checkout_method) =
            match item.router_data.request.payment_method_data.clone() {
                domain::PaymentMethodData::Upi(upi_type) => match upi_type {
                    domain::UpiData::UpiCollect(upi_data) => (
                        common_enums::CountryAlpha2::IN,
                        upi_data.vpa_id.map(|id| PayerInfo {
                            token_id: id.switch_strategy(),
                        }),
                        None,
                    ),
                    domain::UpiData::UpiIntent(_) => (
                        common_enums::CountryAlpha2::IN,
                        None,
                        Some(PreferredCheckoutMethod::Qr),
                    ),
                },
                domain::PaymentMethodData::BankRedirect(bank_redirect_data) => {
                    match bank_redirect_data {
                        domain::BankRedirectData::Ideal { .. } => {
                            (common_enums::CountryAlpha2::NL, None, None)
                        }
                        domain::BankRedirectData::LocalBankRedirect {} => {
                            (common_enums::CountryAlpha2::AT, None, None)
                        }
                        domain::BankRedirectData::BancontactCard { .. }
                        | domain::BankRedirectData::Bizum {}
                        | domain::BankRedirectData::Blik { .. }
                        | domain::BankRedirectData::Eps { .. }
                        | domain::BankRedirectData::Giropay { .. }
                        | domain::BankRedirectData::Interac { .. }
                        | domain::BankRedirectData::OnlineBankingCzechRepublic { .. }
                        | domain::BankRedirectData::OnlineBankingFinland { .. }
                        | domain::BankRedirectData::OnlineBankingPoland { .. }
                        | domain::BankRedirectData::OnlineBankingSlovakia { .. }
                        | domain::BankRedirectData::OpenBankingUk { .. }
                        | domain::BankRedirectData::Przelewy24 { .. }
                        | domain::BankRedirectData::Sofort { .. }
                        | domain::BankRedirectData::Trustly { .. }
                        | domain::BankRedirectData::OnlineBankingFpx { .. }
                        | domain::BankRedirectData::OnlineBankingThailand { .. } => {
                            Err(errors::ConnectorError::NotImplemented(
                                connector_util::get_unimplemented_payment_method_error_message(
                                    "iatapay",
                                ),
                            ))?
                        }
                    }
                }
                domain::PaymentMethodData::RealTimePayment(real_time_payment_data) => {
                    match *real_time_payment_data {
                        domain::RealTimePaymentData::DuitNow {} => {
                            (common_enums::CountryAlpha2::MY, None, None)
                        }
                        domain::RealTimePaymentData::Fps {} => {
                            (common_enums::CountryAlpha2::HK, None, None)
                        }
                        domain::RealTimePaymentData::PromptPay {} => {
                            (common_enums::CountryAlpha2::TH, None, None)
                        }
                        domain::RealTimePaymentData::VietQr {} => {
                            (common_enums::CountryAlpha2::VN, None, None)
                        }
                    }
                }
                domain::PaymentMethodData::Card(_)
                | domain::PaymentMethodData::CardRedirect(_)
                | domain::PaymentMethodData::Wallet(_)
                | domain::PaymentMethodData::PayLater(_)
                | domain::PaymentMethodData::BankDebit(_)
                | domain::PaymentMethodData::BankTransfer(_)
                | domain::PaymentMethodData::Crypto(_)
                | domain::PaymentMethodData::MandatePayment
                | domain::PaymentMethodData::Reward
                | domain::PaymentMethodData::MobilePayment(_)
                | domain::PaymentMethodData::Voucher(_)
                | domain::PaymentMethodData::GiftCard(_)
                | domain::PaymentMethodData::CardToken(_)
                | domain::PaymentMethodData::OpenBanking(_)
                | domain::PaymentMethodData::NetworkToken(_)
                | domain::PaymentMethodData::CardDetailsForNetworkTransactionId(_) => {
                    Err(errors::ConnectorError::NotImplemented(
                        connector_util::get_unimplemented_payment_method_error_message("iatapay"),
                    ))?
                }
            };
        let payload = Self {
            merchant_id: IatapayAuthType::try_from(&item.router_data.connector_auth_type)?
                .merchant_id,
            merchant_payment_id: Some(item.router_data.connector_request_reference_id.clone()),
            amount: item.amount,
            currency: item.router_data.request.currency,
            country,
            locale: format!("en-{}", country),
            redirect_urls: get_redirect_url(return_url),
            payer_info,
            notification_url: item.router_data.request.get_webhook_url()?,
            preferred_checkout_method,
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
    pub amount: FloatMajorUnit,
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

    let payment_response_data = match response.checkout_methods {
        Some(checkout_methods) => {
            let (connector_metadata, redirection_data) =
                match checkout_methods.redirect.redirect_url.ends_with("qr") {
                    true => {
                        let qr_code_info = api_models::payments::FetchQrCodeInformation {
                            qr_code_fetch_url: url::Url::parse(
                                &checkout_methods.redirect.redirect_url,
                            )
                            .change_context(errors::ConnectorError::ResponseHandlingFailed)?,
                        };
                        (
                            Some(qr_code_info.encode_to_value())
                                .transpose()
                                .change_context(errors::ConnectorError::ResponseHandlingFailed)?,
                            None,
                        )
                    }
                    false => (
                        None,
                        Some(services::RedirectForm::Form {
                            endpoint: checkout_methods.redirect.redirect_url,
                            method: services::Method::Get,
                            form_fields,
                        }),
                    ),
                };

            types::PaymentsResponseData::TransactionResponse {
                resource_id: id,
                redirection_data: Box::new(redirection_data),
                mandate_reference: Box::new(None),
                connector_metadata,
                network_txn_id: None,
                connector_response_reference_id: connector_response_reference_id.clone(),
                incremental_authorization_allowed: None,
                charges: None,
            }
        }
        None => types::PaymentsResponseData::TransactionResponse {
            resource_id: id.clone(),
            redirection_data: Box::new(None),
            mandate_reference: Box::new(None),
            connector_metadata: None,
            network_txn_id: None,
            connector_response_reference_id: connector_response_reference_id.clone(),
            incremental_authorization_allowed: None,
            charges: None,
        },
    };

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
    pub amount: FloatMajorUnit,
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
    Cleared,
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
            RefundStatus::Cleared => Self::Success,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RefundResponse {
    iata_refund_id: String,
    status: RefundStatus,
    merchant_refund_id: String,
    amount: FloatMajorUnit,
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
    payment_amount: Option<FloatMajorUnit>,
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
    pub status: Option<u16>,
    pub error: String,
    pub message: Option<String>,
    pub reason: Option<String>,
}

#[derive(Deserialize, Debug, Serialize)]
pub struct IatapayAccessTokenErrorResponse {
    pub error: String,
    pub path: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IatapayPaymentWebhookBody {
    pub status: IatapayWebhookStatus,
    pub iata_payment_id: String,
    pub merchant_payment_id: Option<String>,
    pub failure_code: Option<String>,
    pub failure_details: Option<String>,
    pub amount: FloatMajorUnit,
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
    pub amount: FloatMajorUnit,
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
                IatapayRefundWebhookStatus::Cleared
                | IatapayRefundWebhookStatus::Authorized
                | IatapayRefundWebhookStatus::Settled => Ok(Self::RefundSuccess),
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
    Cleared,
    Locked,
    #[serde(other)]
    Unknown,
}
