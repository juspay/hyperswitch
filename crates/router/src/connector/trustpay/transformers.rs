use std::collections::HashMap;

use api_models::payments::BankRedirectData;
use common_utils::{
    errors::CustomResult,
    pii::{self, Email},
};
use error_stack::{report, ResultExt};
use masking::{ExposeInterface, PeekInterface, Secret};
use reqwest::Url;
use serde::{Deserialize, Serialize};

use crate::{
    connector::utils::{
        self, AddressDetailsData, BrowserInformationData, CardData, PaymentsAuthorizeRequestData,
        PaymentsPreProcessingData, RouterData,
    },
    consts,
    core::errors,
    services,
    types::{self, api, storage::enums, BrowserInformation},
};

type Error = error_stack::Report<errors::ConnectorError>;

#[derive(Debug, Serialize)]
pub struct TrustpayRouterData<T> {
    pub amount: String,
    pub router_data: T,
}

impl<T>
    TryFrom<(
        &types::api::CurrencyUnit,
        types::storage::enums::Currency,
        i64,
        T,
    )> for TrustpayRouterData<T>
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
        let amount = utils::get_amount_as_string(currency_unit, amount, currency)?;
        Ok(Self {
            amount,
            router_data: item,
        })
    }
}

pub struct TrustpayAuthType {
    pub(super) api_key: Secret<String>,
    pub(super) project_id: Secret<String>,
    pub(super) secret_key: Secret<String>,
}

impl TryFrom<&types::ConnectorAuthType> for TrustpayAuthType {
    type Error = Error;
    fn try_from(auth_type: &types::ConnectorAuthType) -> Result<Self, Self::Error> {
        if let types::ConnectorAuthType::SignatureKey {
            api_key,
            key1,
            api_secret,
        } = auth_type
        {
            Ok(Self {
                api_key: api_key.to_owned(),
                project_id: key1.to_owned(),
                secret_key: api_secret.to_owned(),
            })
        } else {
            Err(errors::ConnectorError::FailedToObtainAuthType.into())
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub enum TrustpayPaymentMethod {
    #[serde(rename = "EPS")]
    Eps,
    Giropay,
    IDeal,
    Sofort,
    Blik,
}

#[derive(Debug, Serialize, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub struct MerchantIdentification {
    pub project_id: Secret<String>,
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
pub struct DebtorInformation {
    pub name: Secret<String>,
    pub email: Email,
}

#[derive(Default, Debug, Serialize, Deserialize, Eq, PartialEq, Clone)]
#[serde(rename_all = "PascalCase")]
pub struct BankPaymentInformation {
    pub amount: Amount,
    pub references: References,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub debtor: Option<DebtorInformation>,
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

#[derive(Debug, Serialize, PartialEq)]
pub struct PaymentRequestCards {
    pub amount: String,
    pub currency: String,
    pub pan: cards::CardNumber,
    pub cvv: Secret<String>,
    #[serde(rename = "exp")]
    pub expiry_date: Secret<String>,
    pub cardholder: Secret<String>,
    pub reference: String,
    #[serde(rename = "redirectUrl")]
    pub redirect_url: String,
    #[serde(rename = "billing[city]")]
    pub billing_city: String,
    #[serde(rename = "billing[country]")]
    pub billing_country: api_models::enums::CountryAlpha2,
    #[serde(rename = "billing[street1]")]
    pub billing_street1: Secret<String>,
    #[serde(rename = "billing[postcode]")]
    pub billing_postcode: Secret<String>,
    #[serde(rename = "customer[email]")]
    pub customer_email: Email,
    #[serde(rename = "customer[ipAddress]")]
    pub customer_ip_address: Secret<String, pii::IpAddress>,
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
    pub descriptor: Option<String>,
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
    pub billing_country: api_models::enums::CountryAlpha2,
    pub billing_street1: Secret<String>,
    pub billing_postcode: Secret<String>,
    pub billing_first_name: Secret<String>,
}

impl TryFrom<&BankRedirectData> for TrustpayPaymentMethod {
    type Error = Error;
    fn try_from(value: &BankRedirectData) -> Result<Self, Self::Error> {
        match value {
            api_models::payments::BankRedirectData::Giropay { .. } => Ok(Self::Giropay),
            api_models::payments::BankRedirectData::Eps { .. } => Ok(Self::Eps),
            api_models::payments::BankRedirectData::Ideal { .. } => Ok(Self::IDeal),
            api_models::payments::BankRedirectData::Sofort { .. } => Ok(Self::Sofort),
            api_models::payments::BankRedirectData::Blik { .. } => Ok(Self::Blik),
            api_models::payments::BankRedirectData::BancontactCard { .. }
            | api_models::payments::BankRedirectData::Bizum {}
            | api_models::payments::BankRedirectData::Interac { .. }
            | api_models::payments::BankRedirectData::OnlineBankingCzechRepublic { .. }
            | api_models::payments::BankRedirectData::OnlineBankingFinland { .. }
            | api_models::payments::BankRedirectData::OnlineBankingPoland { .. }
            | api_models::payments::BankRedirectData::OnlineBankingSlovakia { .. }
            | api_models::payments::BankRedirectData::OpenBankingUk { .. }
            | api_models::payments::BankRedirectData::Przelewy24 { .. }
            | api_models::payments::BankRedirectData::Trustly { .. }
            | api_models::payments::BankRedirectData::OnlineBankingFpx { .. }
            | api_models::payments::BankRedirectData::OnlineBankingThailand { .. } => {
                Err(errors::ConnectorError::NotImplemented(
                    utils::get_unimplemented_payment_method_error_message("trustpay"),
                )
                .into())
            }
        }
    }
}

fn get_mandatory_fields(
    item: &types::PaymentsAuthorizeRouterData,
) -> Result<TrustpayMandatoryParams, Error> {
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
        billing_first_name: billing_address.get_first_name()?.to_owned(),
    })
}

fn get_card_request_data(
    item: &types::PaymentsAuthorizeRouterData,
    browser_info: &BrowserInformation,
    params: TrustpayMandatoryParams,
    amount: String,
    ccard: &api_models::payments::Card,
    return_url: String,
) -> Result<TrustpayPaymentsRequest, Error> {
    let email = item.request.get_email()?;
    let customer_ip_address = browser_info.get_ip_address()?;
    let billing_last_name = item
        .get_billing()?
        .address
        .as_ref()
        .and_then(|address| address.last_name.clone());
    Ok(TrustpayPaymentsRequest::CardsPaymentRequest(Box::new(
        PaymentRequestCards {
            amount,
            currency: item.request.currency.to_string(),
            pan: ccard.card_number.clone(),
            cvv: ccard.card_cvc.clone(),
            expiry_date: ccard.get_card_expiry_month_year_2_digit_with_delimiter("/".to_owned())?,
            cardholder: get_full_name(params.billing_first_name, billing_last_name),
            reference: item.connector_request_reference_id.clone(),
            redirect_url: return_url,
            billing_city: params.billing_city,
            billing_country: params.billing_country,
            billing_street1: params.billing_street1,
            billing_postcode: params.billing_postcode,
            customer_email: email,
            customer_ip_address,
            browser_accept_header: browser_info.get_accept_header()?,
            browser_language: browser_info.get_language()?,
            browser_screen_height: browser_info.get_screen_height()?.to_string(),
            browser_screen_width: browser_info.get_screen_width()?.to_string(),
            browser_timezone: browser_info.get_time_zone()?.to_string(),
            browser_user_agent: browser_info.get_user_agent()?,
            browser_java_enabled: browser_info.get_java_enabled()?.to_string(),
            browser_java_script_enabled: browser_info.get_java_script_enabled()?.to_string(),
            browser_screen_color_depth: browser_info.get_color_depth()?.to_string(),
            browser_challenge_window: "1".to_string(),
            payment_action: None,
            payment_type: "Plain".to_string(),
            descriptor: item.request.statement_descriptor.clone(),
        },
    )))
}

fn get_full_name(
    billing_first_name: Secret<String>,
    billing_last_name: Option<Secret<String>>,
) -> Secret<String> {
    match billing_last_name {
        Some(last_name) => format!("{} {}", billing_first_name.peek(), last_name.peek()).into(),
        None => billing_first_name,
    }
}

fn get_debtor_info(
    item: &types::PaymentsAuthorizeRouterData,
    pm: TrustpayPaymentMethod,
    params: TrustpayMandatoryParams,
) -> CustomResult<Option<DebtorInformation>, errors::ConnectorError> {
    let billing_last_name = item
        .get_billing()?
        .address
        .as_ref()
        .and_then(|address| address.last_name.clone());
    Ok(match pm {
        TrustpayPaymentMethod::Blik => Some(DebtorInformation {
            name: get_full_name(params.billing_first_name, billing_last_name),
            email: item.request.get_email()?,
        }),
        TrustpayPaymentMethod::Eps
        | TrustpayPaymentMethod::Giropay
        | TrustpayPaymentMethod::IDeal
        | TrustpayPaymentMethod::Sofort => None,
    })
}

fn get_bank_redirection_request_data(
    item: &types::PaymentsAuthorizeRouterData,
    bank_redirection_data: &BankRedirectData,
    params: TrustpayMandatoryParams,
    amount: String,
    auth: TrustpayAuthType,
) -> Result<TrustpayPaymentsRequest, error_stack::Report<errors::ConnectorError>> {
    let pm = TrustpayPaymentMethod::try_from(bank_redirection_data)?;
    let return_url = item.request.get_return_url()?;
    let payment_request =
        TrustpayPaymentsRequest::BankRedirectPaymentRequest(Box::new(PaymentRequestBankRedirect {
            payment_method: pm.clone(),
            merchant_identification: MerchantIdentification {
                project_id: auth.project_id,
            },
            payment_information: BankPaymentInformation {
                amount: Amount {
                    amount,
                    currency: item.request.currency.to_string(),
                },
                references: References {
                    merchant_reference: item.connector_request_reference_id.clone(),
                },
                debtor: get_debtor_info(item, pm, params)?,
            },
            callback_urls: CallbackURLs {
                success: format!("{return_url}?status=SuccessOk"),
                cancel: return_url.clone(),
                error: return_url,
            },
        }));
    Ok(payment_request)
}

impl TryFrom<&TrustpayRouterData<&types::PaymentsAuthorizeRouterData>> for TrustpayPaymentsRequest {
    type Error = Error;
    fn try_from(
        item: &TrustpayRouterData<&types::PaymentsAuthorizeRouterData>,
    ) -> Result<Self, Self::Error> {
        let browser_info = item
            .router_data
            .request
            .browser_info
            .clone()
            .unwrap_or_default();
        let default_browser_info = BrowserInformation {
            color_depth: Some(browser_info.color_depth.unwrap_or(24)),
            java_enabled: Some(browser_info.java_enabled.unwrap_or(false)),
            java_script_enabled: Some(browser_info.java_enabled.unwrap_or(true)),
            language: Some(browser_info.language.unwrap_or("en-US".to_string())),
            screen_height: Some(browser_info.screen_height.unwrap_or(1080)),
            screen_width: Some(browser_info.screen_width.unwrap_or(1920)),
            time_zone: Some(browser_info.time_zone.unwrap_or(3600)),
            accept_header: Some(browser_info.accept_header.unwrap_or("*".to_string())),
            user_agent: browser_info.user_agent,
            ip_address: browser_info.ip_address,
        };
        let params = get_mandatory_fields(item.router_data)?;
        let amount = item.amount.to_owned();
        let auth = TrustpayAuthType::try_from(&item.router_data.connector_auth_type)
            .change_context(errors::ConnectorError::FailedToObtainAuthType)?;
        match item.router_data.request.payment_method_data {
            api::PaymentMethodData::Card(ref ccard) => Ok(get_card_request_data(
                item.router_data,
                &default_browser_info,
                params,
                amount,
                ccard,
                item.router_data.request.get_return_url()?,
            )?),
            api::PaymentMethodData::BankRedirect(ref bank_redirection_data) => {
                get_bank_redirection_request_data(
                    item.router_data,
                    bank_redirection_data,
                    params,
                    amount,
                    auth,
                )
            }
            api::PaymentMethodData::CardRedirect(_)
            | api::PaymentMethodData::Wallet(_)
            | api::PaymentMethodData::PayLater(_)
            | api::PaymentMethodData::BankDebit(_)
            | api::PaymentMethodData::BankTransfer(_)
            | api::PaymentMethodData::Crypto(_)
            | api::PaymentMethodData::MandatePayment
            | api::PaymentMethodData::Reward
            | api::PaymentMethodData::Upi(_)
            | api::PaymentMethodData::Voucher(_)
            | api::PaymentMethodData::GiftCard(_)
            | api::PaymentMethodData::CardToken(_) => Err(errors::ConnectorError::NotImplemented(
                utils::get_unimplemented_payment_method_error_message("trustpay"),
            )
            .into()),
        }
    }
}

fn is_payment_failed(payment_status: &str) -> (bool, &'static str) {
    match payment_status {
        "100.100.600" => (true, "Empty CVV for VISA, MASTER not allowed"),
        "100.350.100" => (true, "Referenced session is rejected (no action possible)"),
        "100.380.401" => (true, "User authentication failed"),
        "100.380.501" => (true, "Risk management transaction timeout"),
        "100.390.103" => (true, "PARes validation failed - problem with signature"),
        "100.390.105" => (
            true,
            "Transaction rejected because of technical error in 3DSecure system",
        ),
        "100.390.111" => (
            true,
            "Communication error to VISA/Mastercard Directory Server",
        ),
        "100.390.112" => (true, "Technical error in 3D system"),
        "100.390.115" => (true, "Authentication failed due to invalid message format"),
        "100.390.118" => (true, "Authentication failed due to suspected fraud"),
        "100.400.304" => (true, "Invalid input data"),
        "100.550.312" => (true, "Amount is outside allowed ticket size boundaries"),
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
        "400.003.600" => (true, "No description available."),
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
        "800.100.158" => (true, "transaction declined (suspecting manipulation)"),
        "800.100.160" => (true, "transaction declined (card blocked)"),
        "800.100.162" => (true, "Transaction declined (limit exceeded)"),
        "800.100.163" => (
            true,
            "Transaction declined (maximum transaction frequency exceeded)",
        ),
        "800.100.165" => (true, "Transaction declined (card lost)"),
        "800.100.168" => (true, "Transaction declined (restricted card)"),
        "800.100.170" => (true, "Transaction declined (transaction not permitted)"),
        "800.100.171" => (true, "transaction declined (pick up card)"),
        "800.100.172" => (true, "Transaction declined (account blocked)"),
        "800.100.190" => (true, "Transaction declined (invalid configuration data)"),
        "800.100.202" => (true, "Account Closed"),
        "800.120.100" => (true, "Rejected by throttling"),
        "800.300.102" => (true, "Country blacklisted"),
        "800.300.401" => (true, "Bin blacklisted"),
        "800.700.100" => (
            true,
            "Transaction for the same session is currently being processed, please try again later",
        ),
        "900.100.100" => (
            true,
            "Unexpected communication error with connector/acquirer",
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
        None => enums::AttemptStatus::Pending,
    }
}

fn get_transaction_status(
    payment_status: Option<String>,
    redirect_url: Option<Url>,
) -> CustomResult<(enums::AttemptStatus, Option<String>), errors::ConnectorError> {
    // We don't get payment_status only in case, when the user doesn't complete the authentication step.
    // If we receive status, then return the proper status based on the connector response
    if let Some(payment_status) = payment_status {
        let (is_failed, failure_message) = is_payment_failed(&payment_status);
        if is_failed {
            return Ok((
                enums::AttemptStatus::Failure,
                Some(failure_message.to_string()),
            ));
        }

        if is_payment_successful(&payment_status)? {
            return Ok((enums::AttemptStatus::Charged, None));
        }

        let pending_status = get_pending_status_based_on_redirect_url(redirect_url);

        Ok((pending_status, None))
    } else {
        Ok((enums::AttemptStatus::AuthenticationPending, None))
    }
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
    pub payment_status: Option<String>,
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
    WebhookResponse(Box<WebhookPaymentInformation>),
}

impl<F, T>
    TryFrom<types::ResponseRouterData<F, TrustpayPaymentsResponse, T, types::PaymentsResponseData>>
    for types::RouterData<F, T, types::PaymentsResponseData>
{
    type Error = Error;
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
        response.payment_status.to_owned(),
        response.redirect_url.to_owned(),
    )?;

    let form_fields = response.redirect_params.unwrap_or_default();
    let redirection_data = response
        .redirect_url
        .map(|url| services::RedirectForm::Form {
            endpoint: url.to_string(),
            method: services::Method::Post,
            form_fields,
        });
    let error = if msg.is_some() {
        Some(types::ErrorResponse {
            code: response
                .payment_status
                .unwrap_or_else(|| consts::NO_ERROR_CODE.to_string()),
            message: msg
                .clone()
                .unwrap_or_else(|| consts::NO_ERROR_MESSAGE.to_string()),
            reason: msg,
            status_code,
            attempt_status: None,
            connector_transaction_id: Some(response.instance_id.clone()),
        })
    } else {
        None
    };
    let payment_response_data = types::PaymentsResponseData::TransactionResponse {
        resource_id: types::ResponseId::ConnectorTransactionId(response.instance_id),
        redirection_data,
        mandate_reference: None,
        connector_metadata: None,
        network_txn_id: None,
        connector_response_reference_id: None,
        incremental_authorization_allowed: None,
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
        network_txn_id: None,
        connector_response_reference_id: None,
        incremental_authorization_allowed: None,
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
        // message vary for the same code, so relying on code alone as it is unique
        message: response.payment_result_info.result_code.to_string(),
        reason: response.payment_result_info.additional_info,
        status_code,
        attempt_status: None,
        connector_transaction_id: None,
    });
    let payment_response_data = types::PaymentsResponseData::TransactionResponse {
        resource_id: types::ResponseId::NoResponseId,
        redirection_data: None,
        mandate_reference: None,
        connector_metadata: None,
        network_txn_id: None,
        connector_response_reference_id: None,
        incremental_authorization_allowed: None,
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
    let error = if utils::is_payment_failure(status) {
        let reason_info = response
            .payment_information
            .status_reason_information
            .unwrap_or_default();
        Some(types::ErrorResponse {
            code: reason_info.reason.code.clone(),
            // message vary for the same code, so relying on code alone as it is unique
            message: reason_info.reason.code,
            reason: reason_info.reason.reject_reason,
            status_code,
            attempt_status: None,
            connector_transaction_id: Some(
                response
                    .payment_information
                    .references
                    .payment_request_id
                    .clone(),
            ),
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
        network_txn_id: None,
        connector_response_reference_id: None,
        incremental_authorization_allowed: None,
    };
    Ok((status, error, payment_response_data))
}

pub fn handle_webhook_response(
    payment_information: WebhookPaymentInformation,
    status_code: u16,
) -> CustomResult<
    (
        enums::AttemptStatus,
        Option<types::ErrorResponse>,
        types::PaymentsResponseData,
    ),
    errors::ConnectorError,
> {
    let status = enums::AttemptStatus::try_from(payment_information.status)?;
    let error = if utils::is_payment_failure(status) {
        let reason_info = payment_information
            .status_reason_information
            .unwrap_or_default();
        Some(types::ErrorResponse {
            code: reason_info.reason.code.clone(),
            // message vary for the same code, so relying on code alone as it is unique
            message: reason_info.reason.code,
            reason: reason_info.reason.reject_reason,
            status_code,
            attempt_status: None,
            connector_transaction_id: payment_information.references.payment_request_id,
        })
    } else {
        None
    };
    let payment_response_data = types::PaymentsResponseData::TransactionResponse {
        resource_id: types::ResponseId::NoResponseId,
        redirection_data: None,
        mandate_reference: None,
        connector_metadata: None,
        network_txn_id: None,
        connector_response_reference_id: None,
        incremental_authorization_allowed: None,
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
        TrustpayPaymentsResponse::WebhookResponse(response) => {
            handle_webhook_response(*response, status_code)
        }
    }
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct TrustpayAuthUpdateRequest {
    pub grant_type: String,
}

impl TryFrom<&types::RefreshTokenRouterData> for TrustpayAuthUpdateRequest {
    type Error = Error;
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

#[derive(Default, Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct TrustpayAuthUpdateResponse {
    pub access_token: Option<Secret<String>>,
    pub token_type: Option<String>,
    pub expires_in: Option<i64>,
    #[serde(rename = "ResultInfo")]
    pub result_info: ResultInfo,
}

#[derive(Default, Debug, Clone, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct TrustpayAccessTokenErrorResponse {
    pub result_info: ResultInfo,
}

impl<F, T> TryFrom<types::ResponseRouterData<F, TrustpayAuthUpdateResponse, T, types::AccessToken>>
    for types::RouterData<F, T, types::AccessToken>
{
    type Error = Error;
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
                    // message vary for the same code, so relying on code alone as it is unique
                    message: item.response.result_info.result_code.to_string(),
                    reason: item.response.result_info.additional_info,
                    status_code: item.http_code,
                    attempt_status: None,
                    connector_transaction_id: None,
                }),
                ..item.data
            }),
        }
    }
}

#[derive(Default, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TrustpayCreateIntentRequest {
    pub amount: String,
    pub currency: String,
    // If true, Apple Pay will be initialized
    pub init_apple_pay: Option<bool>,
    // If true, Google pay will be initialized
    pub init_google_pay: Option<bool>,
    pub reference: String,
}

impl TryFrom<&TrustpayRouterData<&types::PaymentsPreProcessingRouterData>>
    for TrustpayCreateIntentRequest
{
    type Error = Error;
    fn try_from(
        item: &TrustpayRouterData<&types::PaymentsPreProcessingRouterData>,
    ) -> Result<Self, Self::Error> {
        let is_apple_pay = item
            .router_data
            .request
            .payment_method_type
            .as_ref()
            .map(|pmt| matches!(pmt, diesel_models::enums::PaymentMethodType::ApplePay));

        let is_google_pay = item
            .router_data
            .request
            .payment_method_type
            .as_ref()
            .map(|pmt| matches!(pmt, diesel_models::enums::PaymentMethodType::GooglePay));

        let currency = item.router_data.request.get_currency()?;
        let amount = item.amount.to_owned();

        Ok(Self {
            amount,
            currency: currency.to_string(),
            init_apple_pay: is_apple_pay,
            init_google_pay: is_google_pay,
            reference: item.router_data.connector_request_reference_id.clone(),
        })
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TrustpayCreateIntentResponse {
    // TrustPay's authorization secrets used by client
    pub secrets: SdkSecretInfo,
    // 	Data object to be used for Apple Pay or Google Pay
    #[serde(flatten)]
    pub init_result_data: InitResultData,
    // Unique operation/transaction identifier
    pub instance_id: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum InitResultData {
    AppleInitResultData(TrustpayApplePayResponse),
    GoogleInitResultData(TrustpayGooglePayResponse),
}

#[derive(Clone, Default, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GooglePayTransactionInfo {
    pub country_code: api_models::enums::CountryAlpha2,
    pub currency_code: api_models::enums::Currency,
    pub total_price_status: String,
    pub total_price: String,
}

#[derive(Clone, Default, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GooglePayMerchantInfo {
    pub merchant_name: Secret<String>,
    pub merchant_id: Secret<String>,
}

#[derive(Clone, Default, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GooglePayAllowedPaymentMethods {
    #[serde(rename = "type")]
    pub payment_method_type: String,
    pub parameters: GpayAllowedMethodsParameters,
    pub tokenization_specification: GpayTokenizationSpecification,
}

#[derive(Clone, Default, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GpayTokenParameters {
    pub gateway: String,
    pub gateway_merchant_id: Secret<String>,
}

#[derive(Clone, Default, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GpayTokenizationSpecification {
    #[serde(rename = "type")]
    pub token_specification_type: String,
    pub parameters: GpayTokenParameters,
}

#[derive(Clone, Default, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GpayAllowedMethodsParameters {
    pub allowed_auth_methods: Vec<String>,
    pub allowed_card_networks: Vec<String>,
}

#[derive(Clone, Default, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TrustpayGooglePayResponse {
    pub merchant_info: GooglePayMerchantInfo,
    pub allowed_payment_methods: Vec<GooglePayAllowedPaymentMethods>,
    pub transaction_info: GooglePayTransactionInfo,
}

#[derive(Clone, Default, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SdkSecretInfo {
    pub display: Secret<String>,
    pub payment: Secret<String>,
}

#[derive(Clone, Default, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TrustpayApplePayResponse {
    pub country_code: api_models::enums::CountryAlpha2,
    pub currency_code: api_models::enums::Currency,
    pub supported_networks: Vec<String>,
    pub merchant_capabilities: Vec<String>,
    pub total: ApplePayTotalInfo,
}

#[derive(Clone, Default, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplePayTotalInfo {
    pub label: String,
    pub amount: String,
}

impl<F>
    TryFrom<
        types::ResponseRouterData<
            F,
            TrustpayCreateIntentResponse,
            types::PaymentsPreProcessingData,
            types::PaymentsResponseData,
        >,
    > for types::RouterData<F, types::PaymentsPreProcessingData, types::PaymentsResponseData>
{
    type Error = Error;
    fn try_from(
        item: types::ResponseRouterData<
            F,
            TrustpayCreateIntentResponse,
            types::PaymentsPreProcessingData,
            types::PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        let create_intent_response = item.response.init_result_data.to_owned();
        let secrets = item.response.secrets.to_owned();
        let instance_id = item.response.instance_id.to_owned();
        let pmt = utils::PaymentsPreProcessingData::get_payment_method_type(&item.data.request)?;

        match (pmt, create_intent_response) {
            (
                diesel_models::enums::PaymentMethodType::ApplePay,
                InitResultData::AppleInitResultData(apple_pay_response),
            ) => get_apple_pay_session(instance_id, &secrets, apple_pay_response, item),
            (
                diesel_models::enums::PaymentMethodType::GooglePay,
                InitResultData::GoogleInitResultData(google_pay_response),
            ) => get_google_pay_session(instance_id, &secrets, google_pay_response, item),
            _ => Err(report!(errors::ConnectorError::InvalidWallet)),
        }
    }
}

pub fn get_apple_pay_session<F, T>(
    instance_id: String,
    secrets: &SdkSecretInfo,
    apple_pay_init_result: TrustpayApplePayResponse,
    item: types::ResponseRouterData<
        F,
        TrustpayCreateIntentResponse,
        T,
        types::PaymentsResponseData,
    >,
) -> Result<
    types::RouterData<F, T, types::PaymentsResponseData>,
    error_stack::Report<errors::ConnectorError>,
> {
    Ok(types::RouterData {
        response: Ok(types::PaymentsResponseData::PreProcessingResponse {
            connector_metadata: None,
            pre_processing_id: types::PreprocessingResponseId::ConnectorTransactionId(instance_id),
            session_token: Some(types::api::SessionToken::ApplePay(Box::new(
                api_models::payments::ApplepaySessionTokenResponse {
                    session_token_data:
                        api_models::payments::ApplePaySessionResponse::ThirdPartySdk(
                            api_models::payments::ThirdPartySdkSessionResponse {
                                secrets: secrets.to_owned().into(),
                            },
                        ),
                    payment_request_data: Some(api_models::payments::ApplePayPaymentRequest {
                        country_code: Some(apple_pay_init_result.country_code),
                        currency_code: apple_pay_init_result.currency_code,
                        supported_networks: Some(apple_pay_init_result.supported_networks.clone()),
                        merchant_capabilities: Some(
                            apple_pay_init_result.merchant_capabilities.clone(),
                        ),
                        total: apple_pay_init_result.total.into(),
                        merchant_identifier: None,
                    }),
                    connector: "trustpay".to_string(),
                    delayed_session_token: true,
                    sdk_next_action: {
                        api_models::payments::SdkNextAction {
                            next_action: api_models::payments::NextActionCall::Sync,
                        }
                    },
                    connector_reference_id: None,
                    connector_sdk_public_key: None,
                    connector_merchant_id: None,
                },
            ))),
            connector_response_reference_id: None,
        }),
        // We don't get status from TrustPay but status should be AuthenticationPending by default for session response
        status: diesel_models::enums::AttemptStatus::AuthenticationPending,
        ..item.data
    })
}

pub fn get_google_pay_session<F, T>(
    instance_id: String,
    secrets: &SdkSecretInfo,
    google_pay_init_result: TrustpayGooglePayResponse,
    item: types::ResponseRouterData<
        F,
        TrustpayCreateIntentResponse,
        T,
        types::PaymentsResponseData,
    >,
) -> Result<
    types::RouterData<F, T, types::PaymentsResponseData>,
    error_stack::Report<errors::ConnectorError>,
> {
    Ok(types::RouterData {
        response: Ok(types::PaymentsResponseData::PreProcessingResponse {
            connector_metadata: None,
            pre_processing_id: types::PreprocessingResponseId::ConnectorTransactionId(instance_id),
            session_token: Some(types::api::SessionToken::GooglePay(Box::new(
                api_models::payments::GpaySessionTokenResponse::GooglePaySession(
                    api_models::payments::GooglePaySessionResponse {
                        connector: "trustpay".to_string(),
                        delayed_session_token: true,
                        sdk_next_action: {
                            api_models::payments::SdkNextAction {
                                next_action: api_models::payments::NextActionCall::Sync,
                            }
                        },
                        merchant_info: google_pay_init_result.merchant_info.into(),
                        allowed_payment_methods: google_pay_init_result
                            .allowed_payment_methods
                            .into_iter()
                            .map(Into::into)
                            .collect(),
                        transaction_info: google_pay_init_result.transaction_info.into(),
                        secrets: Some((*secrets).clone().into()),
                    },
                ),
            ))),
            connector_response_reference_id: None,
        }),
        // We don't get status from TrustPay but status should be AuthenticationPending by default for session response
        status: diesel_models::enums::AttemptStatus::AuthenticationPending,
        ..item.data
    })
}

impl From<GooglePayTransactionInfo> for api_models::payments::GpayTransactionInfo {
    fn from(value: GooglePayTransactionInfo) -> Self {
        Self {
            country_code: value.country_code,
            currency_code: value.currency_code,
            total_price_status: value.total_price_status,
            total_price: value.total_price,
        }
    }
}

impl From<GooglePayMerchantInfo> for api_models::payments::GpayMerchantInfo {
    fn from(value: GooglePayMerchantInfo) -> Self {
        Self {
            merchant_id: Some(value.merchant_id.expose()),
            merchant_name: value.merchant_name.expose(),
        }
    }
}

impl From<GooglePayAllowedPaymentMethods> for api_models::payments::GpayAllowedPaymentMethods {
    fn from(value: GooglePayAllowedPaymentMethods) -> Self {
        Self {
            payment_method_type: value.payment_method_type,
            parameters: value.parameters.into(),
            tokenization_specification: value.tokenization_specification.into(),
        }
    }
}

impl From<GpayAllowedMethodsParameters> for api_models::payments::GpayAllowedMethodsParameters {
    fn from(value: GpayAllowedMethodsParameters) -> Self {
        Self {
            allowed_auth_methods: value.allowed_auth_methods,
            allowed_card_networks: value.allowed_card_networks,
        }
    }
}

impl From<GpayTokenizationSpecification> for api_models::payments::GpayTokenizationSpecification {
    fn from(value: GpayTokenizationSpecification) -> Self {
        Self {
            token_specification_type: value.token_specification_type,
            parameters: value.parameters.into(),
        }
    }
}

impl From<GpayTokenParameters> for api_models::payments::GpayTokenParameters {
    fn from(value: GpayTokenParameters) -> Self {
        Self {
            gateway: value.gateway,
            gateway_merchant_id: Some(value.gateway_merchant_id.expose()),
            stripe_version: None,
            stripe_publishable_key: None,
        }
    }
}

impl From<SdkSecretInfo> for api_models::payments::SecretInfoToInitiateSdk {
    fn from(value: SdkSecretInfo) -> Self {
        Self {
            display: value.display,
            payment: value.payment,
        }
    }
}

impl From<ApplePayTotalInfo> for api_models::payments::AmountInfo {
    fn from(value: ApplePayTotalInfo) -> Self {
        Self {
            label: value.label,
            amount: value.amount,
            total_type: None,
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

impl<F> TryFrom<&TrustpayRouterData<&types::RefundsRouterData<F>>> for TrustpayRefundRequest {
    type Error = Error;
    fn try_from(
        item: &TrustpayRouterData<&types::RefundsRouterData<F>>,
    ) -> Result<Self, Self::Error> {
        let amount = item.amount.to_owned();
        match item.router_data.payment_method {
            diesel_models::enums::PaymentMethod::BankRedirect => {
                let auth = TrustpayAuthType::try_from(&item.router_data.connector_auth_type)
                    .change_context(errors::ConnectorError::FailedToObtainAuthType)?;
                Ok(Self::BankRedirectRefund(Box::new(
                    TrustpayRefundRequestBankRedirect {
                        merchant_identification: MerchantIdentification {
                            project_id: auth.project_id,
                        },
                        payment_information: BankPaymentInformation {
                            amount: Amount {
                                amount,
                                currency: item.router_data.request.currency.to_string(),
                            },
                            references: References {
                                merchant_reference: item.router_data.request.refund_id.clone(),
                            },
                            debtor: None,
                        },
                    },
                )))
            }
            _ => Ok(Self::CardsRefund(Box::new(TrustpayRefundRequestCards {
                instance_id: item.router_data.request.connector_transaction_id.clone(),
                amount,
                currency: item.router_data.request.currency.to_string(),
                reference: item.router_data.request.refund_id.clone(),
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
    WebhookRefund(Box<WebhookPaymentInformation>),
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
            message: msg
                .clone()
                .unwrap_or_else(|| consts::NO_ERROR_MESSAGE.to_string()),
            reason: msg,
            status_code,
            attempt_status: None,
            connector_transaction_id: None,
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

fn handle_webhooks_refund_response(
    response: WebhookPaymentInformation,
    status_code: u16,
) -> CustomResult<(Option<types::ErrorResponse>, types::RefundsResponseData), errors::ConnectorError>
{
    let refund_status = diesel_models::enums::RefundStatus::try_from(response.status)?;
    let error = if utils::is_refund_failure(refund_status) {
        let reason_info = response.status_reason_information.unwrap_or_default();
        Some(types::ErrorResponse {
            code: reason_info.reason.code.clone(),
            // message vary for the same code, so relying on code alone as it is unique
            message: reason_info.reason.code,
            reason: reason_info.reason.reject_reason,
            status_code,
            attempt_status: None,
            connector_transaction_id: response.references.payment_request_id.clone(),
        })
    } else {
        None
    };
    let refund_response_data = types::RefundsResponseData {
        connector_refund_id: response
            .references
            .payment_request_id
            .ok_or(errors::ConnectorError::MissingConnectorRefundID)?,
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
            // message vary for the same code, so relying on code alone as it is unique
            message: response.result_info.result_code.to_string(),
            reason: msg.map(|message| message.to_string()),
            status_code,
            attempt_status: None,
            connector_transaction_id: None,
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
    let error = if utils::is_refund_failure(refund_status) {
        let reason_info = response
            .payment_information
            .status_reason_information
            .unwrap_or_default();
        Some(types::ErrorResponse {
            code: reason_info.reason.code.clone(),
            // message vary for the same code, so relying on code alone as it is unique
            message: reason_info.reason.code,
            reason: reason_info.reason.reject_reason,
            status_code,
            attempt_status: None,
            connector_transaction_id: None,
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
        // message vary for the same code, so relying on code alone as it is unique
        message: response.payment_result_info.result_code.to_string(),
        reason: response.payment_result_info.additional_info,
        status_code,
        attempt_status: None,
        connector_transaction_id: None,
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
    type Error = Error;
    fn try_from(
        item: types::RefundsResponseRouterData<F, RefundResponse>,
    ) -> Result<Self, Self::Error> {
        let (error, response) = match item.response {
            RefundResponse::CardsRefund(response) => {
                handle_cards_refund_response(*response, item.http_code)?
            }
            RefundResponse::WebhookRefund(response) => {
                handle_webhooks_refund_response(*response, item.http_code)?
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

#[derive(Default, Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct Errors {
    pub code: i64,
    pub description: String,
}

#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TrustpayErrorResponse {
    pub status: i64,
    pub description: Option<String>,
    pub errors: Option<Vec<Errors>>,
    pub instance_id: Option<String>,
    pub payment_description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "UPPERCASE")]
pub enum CreditDebitIndicator {
    Crdt,
    Dbit,
}

#[derive(strum::Display, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum WebhookStatus {
    Paid,
    Rejected,
    Refunded,
    Chargebacked,
    #[serde(other)]
    Unknown,
}

impl TryFrom<WebhookStatus> for enums::AttemptStatus {
    type Error = errors::ConnectorError;
    fn try_from(item: WebhookStatus) -> Result<Self, Self::Error> {
        match item {
            WebhookStatus::Paid => Ok(Self::Charged),
            WebhookStatus::Rejected => Ok(Self::AuthorizationFailed),
            _ => Err(errors::ConnectorError::WebhookEventTypeNotFound),
        }
    }
}

impl TryFrom<WebhookStatus> for diesel_models::enums::RefundStatus {
    type Error = errors::ConnectorError;
    fn try_from(item: WebhookStatus) -> Result<Self, Self::Error> {
        match item {
            WebhookStatus::Paid => Ok(Self::Success),
            WebhookStatus::Refunded => Ok(Self::Success),
            WebhookStatus::Rejected => Ok(Self::Failure),
            _ => Err(errors::ConnectorError::WebhookEventTypeNotFound),
        }
    }
}

#[derive(Default, Debug, Serialize, Deserialize, Eq, PartialEq, Clone)]
#[serde(rename_all = "PascalCase")]
pub struct WebhookReferences {
    pub merchant_reference: String,
    pub payment_id: Option<String>,
    pub payment_request_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub struct WebhookAmount {
    pub amount: f64,
    pub currency: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub struct WebhookPaymentInformation {
    pub credit_debit_indicator: CreditDebitIndicator,
    pub references: WebhookReferences,
    pub status: WebhookStatus,
    pub amount: WebhookAmount,
    pub status_reason_information: Option<StatusReasonInformation>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub struct TrustpayWebhookResponse {
    pub payment_information: WebhookPaymentInformation,
    pub signature: String,
}

impl From<Errors> for utils::ErrorCodeAndMessage {
    fn from(error: Errors) -> Self {
        Self {
            error_code: error.code.to_string(),
            error_message: error.description,
        }
    }
}
