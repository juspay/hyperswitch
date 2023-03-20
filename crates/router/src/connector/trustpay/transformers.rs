use std::collections::HashMap;

use api_models::payments::BankRedirectData;
use common_utils::{errors::CustomResult, pii::Email};
use error_stack::ResultExt;
use masking::Secret;
use reqwest::Url;
use serde::{Deserialize, Serialize};

use crate::{
    connector::utils::{self, AddressDetailsData, CardData, RouterData},
    consts,
    core::errors,
    pii::{self},
    services,
    types::{self, api, storage::enums, BrowserInformation},
};

pub struct TrustpayAuthType {
    pub(super) api_key: String,
    pub(super) project_id: String,
    pub(super) secret_key: String,
}

impl TryFrom<&types::ConnectorAuthType> for TrustpayAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &types::ConnectorAuthType) -> Result<Self, Self::Error> {
        if let types::ConnectorAuthType::SignatureKey {
            api_key,
            key1,
            api_secret,
        } = auth_type
        {
            Ok(Self {
                api_key: api_key.to_string(),
                project_id: key1.to_string(),
                secret_key: api_secret.to_string(),
            })
        } else {
            Err(errors::ConnectorError::FailedToObtainAuthType.into())
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Eq, PartialEq)]
pub enum TrustpayPaymentMethod {
    #[serde(rename = "EPS")]
    Eps,
    Giropay,
    IDeal,
    Sofort,
}

#[derive(Debug, Serialize, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub struct MerchantIdentification {
    pub project_id: String,
}

#[derive(Default, Debug, Serialize, Deserialize, Eq, PartialEq, Clone)]
#[serde(rename_all = "PascalCase")]
pub struct References {
    pub merchant_reference: String,
}

#[derive(Default, Debug, Serialize, Deserialize, Eq, PartialEq, Clone)]
#[serde(rename_all = "PascalCase")]
pub struct Amount {
    pub amount: String,
    pub currency: String,
}

#[derive(Default, Debug, Serialize, Deserialize, Eq, PartialEq, Clone)]
#[serde(rename_all = "PascalCase")]
pub struct Reason {
    pub code: String,
    pub reject_reason: Option<String>,
}

#[derive(Default, Debug, Serialize, Deserialize, Eq, PartialEq, Clone)]
#[serde(rename_all = "PascalCase")]
pub struct StatusReasonInformation {
    pub reason: Reason,
}

#[derive(Default, Debug, Serialize, Deserialize, Eq, PartialEq, Clone)]
#[serde(rename_all = "PascalCase")]
pub struct BankPaymentInformation {
    pub amount: Amount,
    pub references: References,
}

#[derive(Debug, Serialize, Deserialize, Eq, PartialEq, Clone)]
#[serde(rename_all = "PascalCase")]
pub struct BankPaymentInformationResponse {
    pub status: TrustpayBankRedirectPaymentStatus,
    pub status_reason_information: Option<StatusReasonInformation>,
    pub references: ReferencesResponse,
}

#[derive(Debug, Serialize, Eq, PartialEq)]
pub struct CallbackURLs {
    pub success: String,
    pub cancel: String,
    pub error: String,
}

#[derive(Default, Debug, Serialize, PartialEq)]
pub struct PaymentRequestCards {
    pub amount: String,
    pub currency: String,
    pub pan: Secret<String, pii::CardNumber>,
    pub cvv: Secret<String>,
    #[serde(rename = "exp")]
    pub expiry_date: String,
    pub cardholder: Secret<String>,
    pub reference: String,
    #[serde(rename = "redirectUrl")]
    pub redirect_url: String,
    #[serde(rename = "billing[city]")]
    pub billing_city: String,
    #[serde(rename = "billing[country]")]
    pub billing_country: String,
    #[serde(rename = "billing[street1]")]
    pub billing_street1: Secret<String>,
    #[serde(rename = "billing[postcode]")]
    pub billing_postcode: Secret<String>,
    #[serde(rename = "customer[email]")]
    pub customer_email: Option<Secret<String, Email>>,
    #[serde(rename = "customer[ipAddress]")]
    pub customer_ip_address: Option<std::net::IpAddr>,
    #[serde(rename = "browser[acceptHeader]")]
    pub browser_accept_header: String,
    #[serde(rename = "browser[language]")]
    pub browser_language: String,
    #[serde(rename = "browser[screenHeight]")]
    pub browser_screen_height: String,
    #[serde(rename = "browser[screenWidth]")]
    pub browser_screen_width: String,
    #[serde(rename = "browser[timezone]")]
    pub browser_timezone: String,
    #[serde(rename = "browser[userAgent]")]
    pub browser_user_agent: String,
    #[serde(rename = "browser[javaEnabled]")]
    pub browser_java_enabled: String,
    #[serde(rename = "browser[javaScriptEnabled]")]
    pub browser_java_script_enabled: String,
    #[serde(rename = "browser[screenColorDepth]")]
    pub browser_screen_color_depth: String,
    #[serde(rename = "browser[challengeWindow]")]
    pub browser_challenge_window: String,
    #[serde(rename = "browser[paymentAction]")]
    pub payment_action: Option<String>,
    #[serde(rename = "browser[paymentType]")]
    pub payment_type: String,
}

#[derive(Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PaymentRequestBankRedirect {
    pub payment_method: TrustpayPaymentMethod,
    pub merchant_identification: MerchantIdentification,
    pub payment_information: BankPaymentInformation,
    pub callback_urls: CallbackURLs,
}

#[derive(Debug, Serialize, PartialEq)]
#[serde(untagged)]
pub enum TrustpayPaymentsRequest {
    CardsPaymentRequest(Box<PaymentRequestCards>),
    BankRedirectPaymentRequest(Box<PaymentRequestBankRedirect>),
}

#[derive(Debug, Serialize, Eq, PartialEq)]
pub struct TrustpayMandatoryParams {
    pub billing_city: String,
    pub billing_country: String,
    pub billing_street1: Secret<String>,
    pub billing_postcode: Secret<String>,
}

fn get_trustpay_payment_method(bank_redirection_data: &BankRedirectData) -> TrustpayPaymentMethod {
    match bank_redirection_data {
        api_models::payments::BankRedirectData::Giropay { .. } => TrustpayPaymentMethod::Giropay,
        api_models::payments::BankRedirectData::Eps { .. } => TrustpayPaymentMethod::Eps,
        api_models::payments::BankRedirectData::Ideal { .. } => TrustpayPaymentMethod::IDeal,
        api_models::payments::BankRedirectData::Sofort { .. } => TrustpayPaymentMethod::Sofort,
    }
}

fn get_mandatory_fields(
    item: &types::PaymentsAuthorizeRouterData,
) -> Result<TrustpayMandatoryParams, error_stack::Report<errors::ConnectorError>> {
    let billing_address = item
        .get_billing()?
        .address
        .as_ref()
        .ok_or_else(utils::missing_field_err("billing.address"))?;
    Ok(TrustpayMandatoryParams {
        billing_city: billing_address.get_city()?.to_owned(),
        billing_country: billing_address.get_country()?.to_owned(),
        billing_street1: billing_address.get_line1()?.to_owned(),
        billing_postcode: billing_address.get_zip()?.to_owned(),
    })
}

fn get_card_request_data(
    item: &types::PaymentsAuthorizeRouterData,
    browser_info: &BrowserInformation,
    params: TrustpayMandatoryParams,
    amount: String,
    ccard: &api_models::payments::Card,
    return_url: String,
) -> TrustpayPaymentsRequest {
    TrustpayPaymentsRequest::CardsPaymentRequest(Box::new(PaymentRequestCards {
        amount,
        currency: item.request.currency.to_string(),
        pan: ccard.card_number.clone(),
        cvv: ccard.card_cvc.clone(),
        expiry_date: ccard.get_card_expiry_month_year_2_digit_with_delimiter("/".to_owned()),
        cardholder: ccard.card_holder_name.clone(),
        reference: item.payment_id.clone(),
        redirect_url: return_url,
        billing_city: params.billing_city,
        billing_country: params.billing_country,
        billing_street1: params.billing_street1,
        billing_postcode: params.billing_postcode,
        customer_email: item.request.email.clone(),
        customer_ip_address: browser_info.ip_address,
        browser_accept_header: browser_info.accept_header.clone(),
        browser_language: browser_info.language.clone(),
        browser_screen_height: browser_info.screen_height.clone().to_string(),
        browser_screen_width: browser_info.screen_width.clone().to_string(),
        browser_timezone: browser_info.time_zone.clone().to_string(),
        browser_user_agent: browser_info.user_agent.clone(),
        browser_java_enabled: browser_info.java_enabled.clone().to_string(),
        browser_java_script_enabled: browser_info.java_script_enabled.clone().to_string(),
        browser_screen_color_depth: browser_info.color_depth.clone().to_string(),
        browser_challenge_window: "1".to_string(),
        payment_action: None,
        payment_type: "Plain".to_string(),
    }))
}

fn get_bank_redirection_request_data(
    item: &types::PaymentsAuthorizeRouterData,
    bank_redirection_data: &BankRedirectData,
    amount: String,
    return_url: String,
    auth: TrustpayAuthType,
) -> TrustpayPaymentsRequest {
    TrustpayPaymentsRequest::BankRedirectPaymentRequest(Box::new(PaymentRequestBankRedirect {
        payment_method: get_trustpay_payment_method(bank_redirection_data),
        merchant_identification: MerchantIdentification {
            project_id: auth.project_id,
        },
        payment_information: BankPaymentInformation {
            amount: Amount {
                amount,
                currency: item.request.currency.to_string(),
            },
            references: References {
                merchant_reference: item.payment_id.clone(),
            },
        },
        callback_urls: CallbackURLs {
            success: format!("{return_url}?status=SuccessOk"),
            cancel: return_url.clone(),
            error: return_url,
        },
    }))
}

impl TryFrom<&types::PaymentsAuthorizeRouterData> for TrustpayPaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsAuthorizeRouterData) -> Result<Self, Self::Error> {
        let default_browser_info = BrowserInformation {
            color_depth: 24,
            java_enabled: false,
            java_script_enabled: true,
            language: "en-US".to_string(),
            screen_height: 1080,
            screen_width: 1920,
            time_zone: 3600,
            accept_header: "*".to_string(),
            user_agent: "none".to_string(),
            ip_address: None,
        };
        let browser_info = item
            .request
            .browser_info
            .as_ref()
            .unwrap_or(&default_browser_info);
        let params = get_mandatory_fields(item)?;
        let amount = format!(
            "{:.2}",
            utils::to_currency_base_unit(item.request.amount, item.request.currency)?
                .parse::<f64>()
                .ok()
                .ok_or_else(|| errors::ConnectorError::RequestEncodingFailed)?
        );
        let auth = TrustpayAuthType::try_from(&item.connector_auth_type)
            .change_context(errors::ConnectorError::FailedToObtainAuthType)?;
        Ok(match item.request.payment_method_data {
            api::PaymentMethodData::Card(ref ccard) => Ok(get_card_request_data(
                item,
                browser_info,
                params,
                amount,
                ccard,
                item.get_return_url()?,
            )),
            api::PaymentMethodData::BankRedirect(ref bank_redirection_data) => {
                Ok(get_bank_redirection_request_data(
                    item,
                    bank_redirection_data,
                    amount,
                    item.get_return_url()?,
                    auth,
                ))
            }
            _ => Err(errors::ConnectorError::NotImplemented(format!(
                "Current Payment Method - {:?}",
                item.request.payment_method_data
            ))),
        }?)
    }
}

fn is_payment_failed(payment_status: &str) -> (bool, &'static str) {
    match payment_status {
        "100.100.600" => (true, "Empty CVV for VISA, MASTER not allowed"),
        "100.350.100" => (true, "Referenced session is rejected (no action possible)."),
        "100.380.401" => (true, "User authentication failed."),
        "100.380.501" => (true, "Risk management transaction timeout."),
        "100.390.103" => (true, "PARes validation failed - problem with signature."),
        "100.390.111" => (
            true,
            "Communication error to VISA/Mastercard Directory Server",
        ),
        "100.390.112" => (true, "Technical error in 3D system"),
        "100.390.115" => (true, "Authentication failed due to invalid message format"),
        "100.390.118" => (true, "Authentication failed due to suspected fraud"),
        "100.400.304" => (true, "Invalid input data"),
        "200.300.404" => (true, "Invalid or missing parameter"),
        "300.100.100" => (
            true,
            "Transaction declined (additional customer authentication required)",
        ),
        "400.001.301" => (true, "Card not enrolled in 3DS"),
        "400.001.600" => (true, "Authentication error"),
        "400.001.601" => (true, "Transaction declined (auth. declined)"),
        "400.001.602" => (true, "Invalid transaction"),
        "400.001.603" => (true, "Invalid transaction"),
        "700.400.200" => (
            true,
            "Cannot refund (refund volume exceeded or tx reversed or invalid workflow)",
        ),
        "700.500.001" => (true, "Referenced session contains too many transactions"),
        "700.500.003" => (true, "Test accounts not allowed in production"),
        "800.100.151" => (true, "Transaction declined (invalid card)"),
        "800.100.152" => (true, "Transaction declined by authorization system"),
        "800.100.153" => (true, "Transaction declined (invalid CVV)"),
        "800.100.155" => (true, "Transaction declined (amount exceeds credit)"),
        "800.100.157" => (true, "Transaction declined (wrong expiry date)"),
        "800.100.162" => (true, "Transaction declined (limit exceeded)"),
        "800.100.163" => (
            true,
            "Transaction declined (maximum transaction frequency exceeded)",
        ),
        "800.100.168" => (true, "Transaction declined (restricted card)"),
        "800.100.170" => (true, "Transaction declined (transaction not permitted)"),
        "800.100.190" => (true, "Transaction declined (invalid configuration data)"),
        "800.120.100" => (true, "Rejected by throttling"),
        "800.300.401" => (true, "Bin blacklisted"),
        "800.700.100" => (
            true,
            "Transaction for the same session is currently being processed, please try again later",
        ),
        "900.100.300" => (true, "Timeout, uncertain result"),
        _ => (false, ""),
    }
}

fn is_payment_successful(payment_status: &str) -> CustomResult<bool, errors::ConnectorError> {
    match payment_status {
        "000.400.100" => Ok(true),
        _ => {
            let allowed_prefixes = [
                "000.000.",
                "000.100.1",
                "000.3",
                "000.6",
                "000.400.01",
                "000.400.02",
                "000.400.04",
                "000.400.05",
                "000.400.06",
                "000.400.07",
                "000.400.08",
                "000.400.09",
            ];
            let is_valid = allowed_prefixes
                .iter()
                .any(|&prefix| payment_status.starts_with(prefix));
            Ok(is_valid)
        }
    }
}

fn get_pending_status_based_on_redirect_url(redirect_url: Option<Url>) -> enums::AttemptStatus {
    match redirect_url {
        Some(_url) => enums::AttemptStatus::AuthenticationPending,
        None => enums::AttemptStatus::Authorizing,
    }
}

fn get_transaction_status(
    payment_status: &str,
    redirect_url: Option<Url>,
) -> CustomResult<(enums::AttemptStatus, Option<String>), errors::ConnectorError> {
    let (is_failed, failure_message) = is_payment_failed(payment_status);
    let pending_status = get_pending_status_based_on_redirect_url(redirect_url);
    if payment_status == "000.200.000" {
        return Ok((pending_status, None));
    }
    if is_failed {
        return Ok((
            enums::AttemptStatus::AuthorizationFailed,
            Some(failure_message.to_string()),
        ));
    }
    if is_payment_successful(payment_status)? {
        return Ok((enums::AttemptStatus::Charged, None));
    }
    Ok((pending_status, None))
}

#[derive(Debug, Serialize, Deserialize, Eq, PartialEq, Clone)]
pub enum TrustpayBankRedirectPaymentStatus {
    Paid,
    Authorized,
    Rejected,
    Authorizing,
    Pending,
}

impl From<TrustpayBankRedirectPaymentStatus> for enums::AttemptStatus {
    fn from(item: TrustpayBankRedirectPaymentStatus) -> Self {
        match item {
            TrustpayBankRedirectPaymentStatus::Paid => Self::Charged,
            TrustpayBankRedirectPaymentStatus::Rejected => Self::AuthorizationFailed,
            TrustpayBankRedirectPaymentStatus::Authorized => Self::Authorized,
            TrustpayBankRedirectPaymentStatus::Authorizing => Self::Authorizing,
            TrustpayBankRedirectPaymentStatus::Pending => Self::Authorizing,
        }
    }
}

impl From<TrustpayBankRedirectPaymentStatus> for enums::RefundStatus {
    fn from(item: TrustpayBankRedirectPaymentStatus) -> Self {
        match item {
            TrustpayBankRedirectPaymentStatus::Paid => Self::Success,
            TrustpayBankRedirectPaymentStatus::Rejected => Self::Failure,
            _ => Self::Pending,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PaymentsResponseCards {
    pub status: i64,
    pub description: Option<String>,
    pub instance_id: String,
    pub payment_status: String,
    pub payment_description: Option<String>,
    pub redirect_url: Option<Url>,
    pub redirect_params: Option<HashMap<String, String>>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub struct PaymentsResponseBankRedirect {
    pub payment_request_id: i64,
    pub gateway_url: Url,
    pub payment_result_info: Option<ResultInfo>,
    pub payment_method_response: Option<TrustpayPaymentMethod>,
    pub merchant_identification_response: Option<MerchantIdentification>,
    pub payment_information_response: Option<BankPaymentInformationResponse>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct ErrorResponseBankRedirect {
    #[serde(rename = "ResultInfo")]
    pub payment_result_info: ResultInfo,
}

#[derive(Debug, Serialize, Deserialize, Eq, PartialEq, Clone)]
#[serde(rename_all = "PascalCase")]
pub struct ReferencesResponse {
    pub payment_request_id: String,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
#[serde(rename_all = "PascalCase")]
pub struct SyncResponseBankRedirect {
    pub payment_information: BankPaymentInformationResponse,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum TrustpayPaymentsResponse {
    CardsPayments(Box<PaymentsResponseCards>),
    BankRedirectPayments(Box<PaymentsResponseBankRedirect>),
    BankRedirectSync(Box<SyncResponseBankRedirect>),
    BankRedirectError(Box<ErrorResponseBankRedirect>),
}

impl<F, T>
    TryFrom<types::ResponseRouterData<F, TrustpayPaymentsResponse, T, types::PaymentsResponseData>>
    for types::RouterData<F, T, types::PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<
            F,
            TrustpayPaymentsResponse,
            T,
            types::PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        let (status, error, payment_response_data) =
            get_trustpay_response(item.response, item.http_code)?;
        Ok(Self {
            status,
            response: error.map_or_else(|| Ok(payment_response_data), Err),
            ..item.data
        })
    }
}

fn handle_cards_response(
    response: PaymentsResponseCards,
    status_code: u16,
) -> CustomResult<
    (
        enums::AttemptStatus,
        Option<types::ErrorResponse>,
        types::PaymentsResponseData,
    ),
    errors::ConnectorError,
> {
    let (status, msg) = get_transaction_status(
        response.payment_status.as_str(),
        response.redirect_url.clone(),
    )?;
    let form_fields = response
        .redirect_params
        .unwrap_or(std::collections::HashMap::new());
    let redirection_data = response.redirect_url.map(|url| services::RedirectForm {
        endpoint: url.to_string(),
        method: services::Method::Post,
        form_fields,
    });
    let error = if msg.is_some() {
        Some(types::ErrorResponse {
            code: response.payment_status,
            message: msg.unwrap_or(consts::NO_ERROR_MESSAGE.to_string()),
            reason: None,
            status_code,
        })
    } else {
        None
    };
    let payment_response_data = types::PaymentsResponseData::TransactionResponse {
        resource_id: types::ResponseId::ConnectorTransactionId(response.instance_id),
        redirection_data,
        mandate_reference: None,
        connector_metadata: None,
    };
    Ok((status, error, payment_response_data))
}

fn handle_bank_redirects_response(
    response: PaymentsResponseBankRedirect,
) -> CustomResult<
    (
        enums::AttemptStatus,
        Option<types::ErrorResponse>,
        types::PaymentsResponseData,
    ),
    errors::ConnectorError,
> {
    let status = enums::AttemptStatus::AuthenticationPending;
    let error = None;
    let payment_response_data = types::PaymentsResponseData::TransactionResponse {
        resource_id: types::ResponseId::ConnectorTransactionId(
            response.payment_request_id.to_string(),
        ),
        redirection_data: Some(services::RedirectForm::from((
            response.gateway_url,
            services::Method::Get,
        ))),
        mandate_reference: None,
        connector_metadata: None,
    };
    Ok((status, error, payment_response_data))
}

fn handle_bank_redirects_error_response(
    response: ErrorResponseBankRedirect,
    status_code: u16,
) -> CustomResult<
    (
        enums::AttemptStatus,
        Option<types::ErrorResponse>,
        types::PaymentsResponseData,
    ),
    errors::ConnectorError,
> {
    let status = enums::AttemptStatus::AuthorizationFailed;
    let error = Some(types::ErrorResponse {
        code: response.payment_result_info.result_code.to_string(),
        message: response
            .payment_result_info
            .additional_info
            .unwrap_or(consts::NO_ERROR_MESSAGE.to_string()),
        reason: None,
        status_code,
    });
    let payment_response_data = types::PaymentsResponseData::TransactionResponse {
        resource_id: types::ResponseId::NoResponseId,
        redirection_data: None,
        mandate_reference: None,
        connector_metadata: None,
    };
    Ok((status, error, payment_response_data))
}

fn handle_bank_redirects_sync_response(
    response: SyncResponseBankRedirect,
    status_code: u16,
) -> CustomResult<
    (
        enums::AttemptStatus,
        Option<types::ErrorResponse>,
        types::PaymentsResponseData,
    ),
    errors::ConnectorError,
> {
    let status = enums::AttemptStatus::from(response.payment_information.status);
    let error = if status == enums::AttemptStatus::AuthorizationFailed {
        let reason_info = response
            .payment_information
            .status_reason_information
            .unwrap_or_default();
        Some(types::ErrorResponse {
            code: reason_info.reason.code,
            message: reason_info
                .reason
                .reject_reason
                .unwrap_or(consts::NO_ERROR_MESSAGE.to_string()),
            reason: None,
            status_code,
        })
    } else {
        None
    };
    let payment_response_data = types::PaymentsResponseData::TransactionResponse {
        resource_id: types::ResponseId::ConnectorTransactionId(
            response.payment_information.references.payment_request_id,
        ),
        redirection_data: None,
        mandate_reference: None,
        connector_metadata: None,
    };
    Ok((status, error, payment_response_data))
}

pub fn get_trustpay_response(
    response: TrustpayPaymentsResponse,
    status_code: u16,
) -> CustomResult<
    (
        enums::AttemptStatus,
        Option<types::ErrorResponse>,
        types::PaymentsResponseData,
    ),
    errors::ConnectorError,
> {
    match response {
        TrustpayPaymentsResponse::CardsPayments(response) => {
            handle_cards_response(*response, status_code)
        }
        TrustpayPaymentsResponse::BankRedirectPayments(response) => {
            handle_bank_redirects_response(*response)
        }
        TrustpayPaymentsResponse::BankRedirectSync(response) => {
            handle_bank_redirects_sync_response(*response, status_code)
        }
        TrustpayPaymentsResponse::BankRedirectError(response) => {
            handle_bank_redirects_error_response(*response, status_code)
        }
    }
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct TrustpayAuthUpdateRequest {
    pub grant_type: String,
}

impl TryFrom<&types::RefreshTokenRouterData> for TrustpayAuthUpdateRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(_item: &types::RefreshTokenRouterData) -> Result<Self, Self::Error> {
        Ok(Self {
            grant_type: "client_credentials".to_string(),
        })
    }
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub struct ResultInfo {
    pub result_code: i64,
    pub additional_info: Option<String>,
    pub correlation_id: Option<String>,
}

#[derive(Default, Debug, Clone, Deserialize, PartialEq)]
pub struct TrustpayAuthUpdateResponse {
    pub access_token: Option<String>,
    pub token_type: Option<String>,
    pub expires_in: Option<i64>,
    #[serde(rename = "ResultInfo")]
    pub result_info: ResultInfo,
}

#[derive(Default, Debug, Clone, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub struct TrustpayAccessTokenErrorResponse {
    pub result_info: ResultInfo,
}

impl<F, T> TryFrom<types::ResponseRouterData<F, TrustpayAuthUpdateResponse, T, types::AccessToken>>
    for types::RouterData<F, T, types::AccessToken>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<F, TrustpayAuthUpdateResponse, T, types::AccessToken>,
    ) -> Result<Self, Self::Error> {
        match (item.response.access_token, item.response.expires_in) {
            (Some(access_token), Some(expires_in)) => Ok(Self {
                response: Ok(types::AccessToken {
                    token: access_token,
                    expires: expires_in,
                }),
                ..item.data
            }),
            _ => Ok(Self {
                response: Err(types::ErrorResponse {
                    code: item.response.result_info.result_code.to_string(),
                    message: item
                        .response
                        .result_info
                        .additional_info
                        .unwrap_or(consts::NO_ERROR_MESSAGE.to_string()),
                    reason: None,
                    status_code: item.http_code,
                }),
                ..item.data
            }),
        }
    }
}

#[derive(Default, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TrustpayRefundRequestCards {
    instance_id: String,
    amount: String,
    currency: String,
    reference: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TrustpayRefundRequestBankRedirect {
    pub merchant_identification: MerchantIdentification,
    pub payment_information: BankPaymentInformation,
}

#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum TrustpayRefundRequest {
    CardsRefund(Box<TrustpayRefundRequestCards>),
    BankRedirectRefund(Box<TrustpayRefundRequestBankRedirect>),
}

impl<F> TryFrom<&types::RefundsRouterData<F>> for TrustpayRefundRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::RefundsRouterData<F>) -> Result<Self, Self::Error> {
        match item.payment_method {
            storage_models::enums::PaymentMethod::BankRedirect => {
                let auth = TrustpayAuthType::try_from(&item.connector_auth_type)
                    .change_context(errors::ConnectorError::FailedToObtainAuthType)?;
                let amount =
                    utils::to_currency_base_unit(item.request.amount, item.request.currency)?
                        .parse::<f64>()
                        .ok()
                        .ok_or_else(|| errors::ConnectorError::RequestEncodingFailed)?;
                Ok(Self::BankRedirectRefund(Box::new(
                    TrustpayRefundRequestBankRedirect {
                        merchant_identification: MerchantIdentification {
                            project_id: auth.project_id,
                        },
                        payment_information: BankPaymentInformation {
                            amount: Amount {
                                amount: format!("{amount:.2}"),
                                currency: item.request.currency.to_string(),
                            },
                            references: References {
                                merchant_reference: format!("{}_{}", item.payment_id, "1"),
                            },
                        },
                    },
                )))
            }
            _ => Ok(Self::CardsRefund(Box::new(TrustpayRefundRequestCards {
                instance_id: item.request.connector_transaction_id.clone(),
                amount: utils::to_currency_base_unit(item.request.amount, item.request.currency)?,
                currency: item.request.currency.to_string(),
                reference: item.payment_id.clone(),
            }))),
        }
    }
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CardsRefundResponse {
    pub status: i64,
    pub description: Option<String>,
    pub instance_id: String,
    pub payment_status: String,
    pub payment_description: Option<String>,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct BankRedirectRefundResponse {
    pub payment_request_id: i64,
    pub result_info: ResultInfo,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum RefundResponse {
    CardsRefund(Box<CardsRefundResponse>),
    BankRedirectRefund(Box<BankRedirectRefundResponse>),
    BankRedirectRefundSyncResponse(Box<SyncResponseBankRedirect>),
    BankRedirectError(Box<ErrorResponseBankRedirect>),
}

fn handle_cards_refund_response(
    response: CardsRefundResponse,
    status_code: u16,
) -> CustomResult<(Option<types::ErrorResponse>, types::RefundsResponseData), errors::ConnectorError>
{
    let (refund_status, msg) = get_refund_status(&response.payment_status)?;
    let error = if msg.is_some() {
        Some(types::ErrorResponse {
            code: response.payment_status,
            message: msg.unwrap_or(consts::NO_ERROR_MESSAGE.to_string()),
            reason: None,
            status_code,
        })
    } else {
        None
    };
    let refund_response_data = types::RefundsResponseData {
        connector_refund_id: response.instance_id,
        refund_status,
    };
    Ok((error, refund_response_data))
}

fn handle_bank_redirects_refund_response(
    response: BankRedirectRefundResponse,
    status_code: u16,
) -> (Option<types::ErrorResponse>, types::RefundsResponseData) {
    let (refund_status, msg) = get_refund_status_from_result_info(response.result_info.result_code);
    let error = if msg.is_some() {
        Some(types::ErrorResponse {
            code: response.result_info.result_code.to_string(),
            message: msg.unwrap_or(consts::NO_ERROR_MESSAGE).to_owned(),
            reason: None,
            status_code,
        })
    } else {
        None
    };
    let refund_response_data = types::RefundsResponseData {
        connector_refund_id: response.payment_request_id.to_string(),
        refund_status,
    };
    (error, refund_response_data)
}

fn handle_bank_redirects_refund_sync_response(
    response: SyncResponseBankRedirect,
    status_code: u16,
) -> (Option<types::ErrorResponse>, types::RefundsResponseData) {
    let refund_status = enums::RefundStatus::from(response.payment_information.status);
    let error = if refund_status == enums::RefundStatus::Failure {
        let reason_info = response
            .payment_information
            .status_reason_information
            .unwrap_or_default();
        Some(types::ErrorResponse {
            code: reason_info.reason.code,
            message: reason_info
                .reason
                .reject_reason
                .unwrap_or(consts::NO_ERROR_MESSAGE.to_string()),
            reason: None,
            status_code,
        })
    } else {
        None
    };
    let refund_response_data = types::RefundsResponseData {
        connector_refund_id: response.payment_information.references.payment_request_id,
        refund_status,
    };
    (error, refund_response_data)
}

fn handle_bank_redirects_refund_sync_error_response(
    response: ErrorResponseBankRedirect,
    status_code: u16,
) -> (Option<types::ErrorResponse>, types::RefundsResponseData) {
    let error = Some(types::ErrorResponse {
        code: response.payment_result_info.result_code.to_string(),
        message: response
            .payment_result_info
            .additional_info
            .unwrap_or(consts::NO_ERROR_MESSAGE.to_owned()),
        reason: None,
        status_code,
    });
    //unreachable case as we are sending error as Some()
    let refund_response_data = types::RefundsResponseData {
        connector_refund_id: "".to_string(),
        refund_status: enums::RefundStatus::Failure,
    };
    (error, refund_response_data)
}

impl<F> TryFrom<types::RefundsResponseRouterData<F, RefundResponse>>
    for types::RefundsRouterData<F>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::RefundsResponseRouterData<F, RefundResponse>,
    ) -> Result<Self, Self::Error> {
        let (error, response) = match item.response {
            RefundResponse::CardsRefund(response) => {
                handle_cards_refund_response(*response, item.http_code)?
            }
            RefundResponse::BankRedirectRefund(response) => {
                handle_bank_redirects_refund_response(*response, item.http_code)
            }
            RefundResponse::BankRedirectRefundSyncResponse(response) => {
                handle_bank_redirects_refund_sync_response(*response, item.http_code)
            }
            RefundResponse::BankRedirectError(response) => {
                handle_bank_redirects_refund_sync_error_response(*response, item.http_code)
            }
        };
        Ok(Self {
            response: error.map_or_else(|| Ok(response), Err),
            ..item.data
        })
    }
}

fn get_refund_status(
    payment_status: &str,
) -> CustomResult<(enums::RefundStatus, Option<String>), errors::ConnectorError> {
    let (is_failed, failure_message) = is_payment_failed(payment_status);
    if payment_status == "000.200.000" {
        Ok((enums::RefundStatus::Pending, None))
    } else if is_failed {
        Ok((
            enums::RefundStatus::Failure,
            Some(failure_message.to_string()),
        ))
    } else if is_payment_successful(payment_status)? {
        Ok((enums::RefundStatus::Success, None))
    } else {
        Ok((enums::RefundStatus::Pending, None))
    }
}

fn get_refund_status_from_result_info(
    result_code: i64,
) -> (enums::RefundStatus, Option<&'static str>) {
    match result_code {
        1001000 => (enums::RefundStatus::Success, None),
        1130001 => (enums::RefundStatus::Pending, Some("MapiPending")),
        1130000 => (enums::RefundStatus::Pending, Some("MapiSuccess")),
        1130004 => (enums::RefundStatus::Pending, Some("MapiProcessing")),
        1130002 => (enums::RefundStatus::Pending, Some("MapiAnnounced")),
        1130003 => (enums::RefundStatus::Pending, Some("MapiAuthorized")),
        1130005 => (enums::RefundStatus::Pending, Some("MapiAuthorizedOnly")),
        1112008 => (enums::RefundStatus::Failure, Some("InvalidPaymentState")),
        1112009 => (enums::RefundStatus::Failure, Some("RefundRejected")),
        1122006 => (
            enums::RefundStatus::Failure,
            Some("AccountCurrencyNotAllowed"),
        ),
        1132000 => (enums::RefundStatus::Failure, Some("InvalidMapiRequest")),
        1132001 => (enums::RefundStatus::Failure, Some("UnknownAccount")),
        1132002 => (
            enums::RefundStatus::Failure,
            Some("MerchantAccountDisabled"),
        ),
        1132003 => (enums::RefundStatus::Failure, Some("InvalidSign")),
        1132004 => (enums::RefundStatus::Failure, Some("DisposableBalance")),
        1132005 => (enums::RefundStatus::Failure, Some("TransactionNotFound")),
        1132006 => (enums::RefundStatus::Failure, Some("UnsupportedTransaction")),
        1132007 => (enums::RefundStatus::Failure, Some("GeneralMapiError")),
        1132008 => (
            enums::RefundStatus::Failure,
            Some("UnsupportedCurrencyConversion"),
        ),
        1132009 => (enums::RefundStatus::Failure, Some("UnknownMandate")),
        1132010 => (enums::RefundStatus::Failure, Some("CanceledMandate")),
        1132011 => (enums::RefundStatus::Failure, Some("MissingCid")),
        1132012 => (enums::RefundStatus::Failure, Some("MandateAlreadyPaid")),
        1132013 => (enums::RefundStatus::Failure, Some("AccountIsTesting")),
        1132014 => (enums::RefundStatus::Failure, Some("RequestThrottled")),
        1133000 => (enums::RefundStatus::Failure, Some("InvalidAuthentication")),
        1133001 => (enums::RefundStatus::Failure, Some("ServiceNotAllowed")),
        1133002 => (enums::RefundStatus::Failure, Some("PaymentRequestNotFound")),
        1133003 => (enums::RefundStatus::Failure, Some("UnexpectedGateway")),
        1133004 => (enums::RefundStatus::Failure, Some("MissingExternalId")),
        1152000 => (enums::RefundStatus::Failure, Some("RiskDecline")),
        _ => (enums::RefundStatus::Pending, None),
    }
}

#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
pub struct TrustpayRedirectResponse {
    pub status: Option<String>,
}

#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
pub struct Errors {
    pub code: i64,
    pub description: String,
}

#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
pub struct TrustpayErrorResponse {
    pub status: i64,
    pub description: Option<String>,
    pub errors: Vec<Errors>,
}
