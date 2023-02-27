use std::collections::HashMap;

use api_models::payments::BankRedirectData;
use common_utils::{pii::Email, errors::{CustomResult}};
use error_stack::{report, ResultExt};
use serde::{Deserialize, Serialize};
use reqwest::Url;
use masking::Secret;
use once_cell::sync::Lazy;
use router_env::logger;
use regex::Regex;
use crate::{core::errors,types::{self,api, storage::enums, BrowserInformation}, pii::PeekInterface, connector::utils::{PaymentsRequestData}, services,};

#[derive(Debug, Serialize, Deserialize, Eq, PartialEq)]
pub enum TrustpayPaymentMethod {
    EPS,
    Giropay,
    IDeal,
    Sofort,
    // Bancontact,
}

#[derive(Debug, Serialize, Deserialize, Eq, PartialEq, Clone)]
pub enum TrustpayBankRedirectPaymentStatus{
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
            TrustpayBankRedirectPaymentStatus::Rejected => Self::AuthenticationFailed,
            TrustpayBankRedirectPaymentStatus::Authorized => Self::Authorized,
            TrustpayBankRedirectPaymentStatus::Authorizing => Self::Authorizing,
            TrustpayBankRedirectPaymentStatus::Pending => Self::Authorizing,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct MerchantIdentification {
    #[serde(rename = "ProjectId")]
    pub project_id: String,
}

#[derive(Default, Debug, Serialize, Deserialize, Eq, PartialEq, Clone)]
pub struct References {
    #[serde(rename = "MerchantReference")]
    pub merchant_reference: String,
}

#[derive(Default, Debug, Serialize, Deserialize, Eq, PartialEq, Clone)]
pub struct Amount {
    #[serde(rename = "Amount")]
    pub amount: String,
    #[serde(rename = "Currency")]
    pub currency: String,
}

#[derive(Default, Debug, Serialize, Deserialize, Eq, PartialEq, Clone)]
pub struct Reason {
    #[serde(rename = "Code")]
    pub code: String,
    #[serde(rename = "RejectReason")]
    pub reject_reason: Option<String>,
}

#[derive(Default, Debug, Serialize, Deserialize, Eq, PartialEq, Clone)]
pub struct StatusReasonInformation {
    #[serde(rename = "Reason")]
    pub reason: Reason,
}

#[derive(Default, Debug, Serialize, Deserialize, Eq, PartialEq, Clone)]
pub struct BankPaymentInformation {
    #[serde(rename = "Amount")]
    pub amount: Amount,
    #[serde(rename = "References")]
    pub references: References,
}

#[derive(Default, Debug, Serialize, Deserialize, Eq, PartialEq, Clone)]
pub struct BankPaymentInformationResponse {
    #[serde(rename = "Status")]
    pub status: Option<TrustpayBankRedirectPaymentStatus>,
    #[serde(rename = "StatusReasonInformation")]
    pub status_reason: Option<StatusReasonInformation>,
}

#[derive(Debug, Serialize, Eq, PartialEq)]
pub struct CallbackURLs {
    pub success: String,
    pub cancel: String,
    pub error: String,
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
pub struct TrustpayPaymentsRequest {
    pub amount                      : Option<i64>,
    pub currency                    : Option<String>,
    pub pan                         : Option<String>,
    pub cvv                         : Option<String>,
    #[serde(rename = "exp")]
    pub expiry_date                 : Option<String>,
    pub cardholder                  : Option<String>,
    pub reference                   : Option<String>,
    #[serde(rename = "redirectUrl")]
    pub redirect_url    : Option<String>,
    #[serde(rename = "billing[city]")]
    pub billing_city                : Option<String>,
    #[serde(rename = "billing[country]")]
    pub billing_country             : Option<String>,
    #[serde(rename = "billing[street1]")]
    pub billing_street1             : Option<Secret<String>>,
    #[serde(rename = "billing[postcode]")]
    pub billing_postcode            : Option<Secret<String>>,
    #[serde(rename = "customer[email]")]
    pub customer_email              : Option<Secret<String, Email>>,
    #[serde(rename = "customer[ipAddress]")]
    pub customer_ip_address         : Option<std::net::IpAddr>,
    #[serde(rename = "browser[acceptHeader]")]
    pub browser_accept_header       : Option<String>,
    #[serde(rename = "browser[language]")]
    pub browser_language            : Option<String>,
    #[serde(rename = "browser[screenHeight]")]
    pub browser_screen_height       : Option<String>,
    #[serde(rename = "browser[screenWidth]")]
    pub browser_screen_width        : Option<String>,
    #[serde(rename = "browser[timezone]")]
    pub browser_timezone            : Option<String>,
    #[serde(rename = "browser[userAgent]")]
    pub browser_user_agent          : Option<String>,
    #[serde(rename = "browser[javaEnabled]")]
    pub browser_java_enabled        : Option<String>,
    #[serde(rename = "browser[javaScriptEnabled]")]
    pub browser_java_script_enabled : Option<String>,
    #[serde(rename = "browser[screenColorDepth]")]
    pub browser_screen_color_depth  : Option<String>,
    #[serde(rename = "browser[challengeWindow]")]
    pub browser_challenge_window    : Option<String>,
    #[serde(rename = "browser[paymentAction]")]
    pub payment_action              : Option<String>,
    #[serde(rename = "browser[paymentType]")]
    pub payment_type                : Option<String>,
    #[serde(rename = "paymentMethod")]
    pub payment_method: Option<TrustpayPaymentMethod>,
    #[serde(rename = "merchantIdentification")]
    pub merchant_identification: Option<MerchantIdentification>,
    #[serde(rename = "paymentInformation")]
    pub payment_information: Option<BankPaymentInformation>,
    #[serde(rename = "callbackUrls")]
    pub callback_urls: Option<CallbackURLs>,
}

#[derive(Debug, Serialize, Eq, PartialEq)]
pub struct TrustpayMandatoryParams {
    pub billing_city        : String,
    pub billing_country     : String,
    pub billing_street1     : Secret<String>,
    pub billing_postcode    : Secret<String>,
}

fn get_trustpay_payment_method(bank_redirection_data: &BankRedirectData) -> TrustpayPaymentMethod {
    match bank_redirection_data {
        api_models::payments::BankRedirectData::Giropay{..} => TrustpayPaymentMethod:: Giropay,
        api_models::payments::BankRedirectData::Eps{..} => TrustpayPaymentMethod:: EPS,
        api_models::payments::BankRedirectData::Ideal{..} => TrustpayPaymentMethod:: IDeal,
        api_models::payments::BankRedirectData::Sofort{..} => TrustpayPaymentMethod:: Sofort,
    }
}

impl TryFrom<&types::PaymentsAuthorizeRouterData> for TrustpayPaymentsRequest  {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsAuthorizeRouterData) -> Result<Self,Self::Error> {
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
            ip_address : None,
        };
        let params = get_mandatory_fields(&item)?;
        Ok(match item.request.payment_method_data {
            api::PaymentMethod::Card(ref ccard) => Ok(Self {
                amount                      : Some(item.request.amount),
                currency                    : Some(item.request.currency.to_string()),
                pan                         : Some(ccard.card_number.peek().clone()),
                cvv                         : Some(ccard.card_cvc.peek().clone()),
                expiry_date                 : Some(format!(
                    "{}/{}",
                    ccard.card_exp_month.peek().clone(),
                    ccard.card_exp_year.peek().clone()
                )),
                cardholder                  : Some(ccard.card_holder_name.peek().clone()),
                reference                   : Some(format!("{}_{}_{}", item.merchant_id, item.payment_id, "1")),
                redirect_url                : Some(item.get_return_url()?),
                billing_city                : Some(params.billing_city),
                billing_country             : Some(params.billing_country),
                billing_street1             : Some(params.billing_street1),
                billing_postcode            : Some(params.billing_postcode),
                customer_email              : item.request.email.clone(),
                customer_ip_address         : item.request.browser_info.as_ref().unwrap_or_else(|| &default_browser_info).ip_address.clone(),
                browser_accept_header       : Some(item.request.browser_info.as_ref().unwrap_or_else(|| &default_browser_info).accept_header.clone()),
                browser_language            : Some(item.request.browser_info.as_ref().unwrap_or_else(|| &default_browser_info).language.clone()),
                browser_screen_height       : Some(item.request.browser_info.as_ref().unwrap_or_else(|| &default_browser_info).screen_height.clone().to_string()),
                browser_screen_width        : Some(item.request.browser_info.as_ref().unwrap_or_else(|| &default_browser_info).screen_width.clone().to_string()),
                browser_timezone            : Some(item.request.browser_info.as_ref().unwrap_or_else(|| &default_browser_info).time_zone.clone().to_string()),
                browser_user_agent          : Some(item.request.browser_info.as_ref().unwrap_or_else(|| &default_browser_info).user_agent.clone()),
                browser_java_enabled        : Some(item.request.browser_info.as_ref().unwrap_or_else(|| &default_browser_info).java_enabled.clone().to_string()),
                browser_java_script_enabled : Some(item.request.browser_info.as_ref().unwrap_or_else(|| &default_browser_info).java_script_enabled.clone().to_string()),
                browser_screen_color_depth  : Some(item.request.browser_info.as_ref().unwrap_or_else(|| &default_browser_info).color_depth.clone().to_string()),
                browser_challenge_window    : Some("1".to_string()),
                payment_action              : None,
                payment_type                : Some("Plain".to_string()),
                ..Default::default()
            }),
            api::PaymentMethod::BankRedirect(ref bank_redirection_data) => {
                let auth: TrustpayAuthType = (&item.connector_auth_type)
                    .try_into()
                    .change_context(errors::ConnectorError::FailedToObtainAuthType)?;
                Ok(Self {
                        payment_method: Some(get_trustpay_payment_method(bank_redirection_data)),
                        merchant_identification: Some(MerchantIdentification{
                            project_id: auth.project_id,
                        }),
                        payment_information: Some(BankPaymentInformation{
                            amount: Amount {
                                amount: format!("{}.00", item.request.amount),
                                currency: item.request.currency.to_string(),
                            },
                            references: References {
                                merchant_reference: format!("{}_{}", item.payment_id, "1"),
                            },
                        }),
                        callback_urls: Some(CallbackURLs{
                            success: item.get_return_url()?,
                            cancel: item.get_return_url()?,
                            error: item.get_return_url()?,
                        }),
                    ..Default::default()
                })
            },
            _ => Err(errors::ConnectorError::NotImplemented(format!(
                "Current Payment Method - {:?}",
                item.request.payment_method_data
            ))),
        }?)
    }
}

fn get_mandatory_fields(
    item: &types::PaymentsAuthorizeRouterData
) -> Result<TrustpayMandatoryParams, error_stack::Report<errors::ConnectorError>> {
    let billing_address = item.address.billing.clone().unwrap_or_default().address.unwrap_or_default();
    Ok(TrustpayMandatoryParams{
        billing_city: billing_address.city.ok_or(
            errors::ConnectorError::MissingRequiredField {
                field_name: "billing.address.city",
            },
        )?,
        billing_country: billing_address.country.ok_or(
            errors::ConnectorError::MissingRequiredField {
                field_name: "billing.address.country",
            },
        )?,
        billing_street1: billing_address.line1.ok_or(
            errors::ConnectorError::MissingRequiredField {
                field_name: "billing.address.line1",
            },
        )?,
        billing_postcode: billing_address.zip.ok_or(
            errors::ConnectorError::MissingRequiredField {
                field_name: "billing.address.postcode",
            },
        )?,
    })
}

// Auth Struct
pub struct TrustpayAuthType {
    pub(super) api_key: String,
    pub(super) project_id: String,
    pub(super) secret_key: String,
}

impl TryFrom<&types::ConnectorAuthType> for TrustpayAuthType  {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &types::ConnectorAuthType) -> Result<Self, Self::Error> {
        if let types::ConnectorAuthType::SignatureKey { api_key, key1, api_secret } = auth_type {
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

fn is_payment_failed(payment_status : &str) -> (bool, &'static str) {
    match payment_status {
        "100.100.600" => (true, "Empty CVV for VISA, MASTER not allowed"),
        "100.350.100" => (true, "Referenced session is rejected (no action possible)."),
        "100.380.401" => (true, "User authentication failed."),
        "100.380.501" => (true, "Risk management transaction timeout."),
        "100.390.103" => (true, "PARes validation failed - problem with signature."),
        "100.390.111" => (true, "Communication error to VISA/Mastercard Directory Server"),
        "100.390.112" => (true, "Technical error in 3D system"),
        "100.390.115" => (true, "Authentication failed due to invalid message format"),
        "100.390.118" => (true, "Authentication failed due to suspected fraud"),
        "100.400.304" => (true, "Invalid input data"),
        "200.300.404" => (true, "Invalid or missing parameter"),
        "300.100.100" => (true, "Transaction declined (additional customer authentication required)"),
        "400.001.301" => (true, "Card not enrolled in 3DS"),
        "400.001.600" => (true, "Authentication error"),
        "400.001.601" => (true, "Transaction declined (auth. declined)"),
        "400.001.602" => (true, "Invalid transaction"),
        "400.001.603" => (true, "Invalid transaction"),
        "700.400.200" => (true, "Cannot refund (refund volume exceeded or tx reversed or invalid workflow)"),
        "700.500.001" => (true, "Referenced session contains too many transactions"),
        "700.500.003" => (true, "Test accounts not allowed in production"),
        "800.100.151" => (true, "Transaction declined (invalid card)"),
        "800.100.152" => (true, "Transaction declined by authorization system"),
        "800.100.153" => (true, "Transaction declined (invalid CVV)"),
        "800.100.155" => (true, "Transaction declined (amount exceeds credit)"),
        "800.100.157" => (true, "Transaction declined (wrong expiry date)"),
        "800.100.162" => (true, "Transaction declined (limit exceeded)"),
        "800.100.163" => (true, "Transaction declined (maximum transaction frequency exceeded)"),
        "800.100.168" => (true, "Transaction declined (restricted card)"),
        "800.100.170" => (true, "Transaction declined (transaction not permitted)"),
        "800.100.190" => (true, "Transaction declined (invalid configuration data)"),
        "800.120.100" => (true, "Rejected by throttling"),
        "800.300.401" => (true, "Bin blacklisted"),
        "800.700.100" => (true, "Transaction for the same session is currently being processed, please try again later"),
        "900.100.300" => (true, "Timeout, uncertain result"),
        _             => (false, "")
    }
}

fn is_payment_successful(payment_status: &str) -> CustomResult<bool,errors::ConnectorError> {
    match payment_status {
        "000.400.100" => Ok(true),
        _             => {
            #[deny(clippy::invalid_regex)]
            static TXN_STATUS_REGEX: Lazy<Option<Regex>> = Lazy::new(|| {
                match Regex::new(
                    r"^000\\.000\\.|^000\\.100\\.1|^000\\.3|^000\\.6|^000\\.400\\.0[^3]",
                ) {
                    Ok(regex) => Some(regex),
                    Err(error) => {
                        logger::error!(?error);
                        None
                    }
                }
            });
            let txn_status_regex = match TXN_STATUS_REGEX.as_ref() {
                Some(regex) => Ok(regex),
                None => Err(report!(errors::ConnectorError::ResponseHandlingFailed)),
            }?;
            Ok(txn_status_regex.is_match(payment_status))
        }
    }
}

fn get_pending_status_based_on_redirect_url(redirect_url: Option<String>) -> enums::AttemptStatus {
    match redirect_url {
        Some(_url) => enums::AttemptStatus:: AuthenticationPending,
        None              => enums::AttemptStatus:: Authorizing,
    }
}

fn get_transaction_status(payment_status : &str, redirect_url : Option<String>) -> CustomResult<(enums:: AttemptStatus, Option<String>),errors::ConnectorError> {
    let (is_failed, failure_message) = is_payment_failed(payment_status);
    let pending_status = get_pending_status_based_on_redirect_url(redirect_url.clone());
    if payment_status == "000.200.000" {
        Ok((pending_status, None))
    }
    else if is_payment_successful(payment_status)? {
       Ok((enums:: AttemptStatus:: Charged, None))
    }
    else if is_failed {
        Ok((enums:: AttemptStatus:: AuthorizationFailed, Some(failure_message.to_string())))
    }
    else{
        Ok((pending_status, None))
    }
}

#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
pub struct TrustpayPaymentsResponse {
    pub status        : Option<i64>,
    pub description   : Option<String>,
    #[serde(rename = "instanceId")]
    pub instance_id   : Option<String>,
    #[serde(rename = "paymentStatus")]
    pub payment_status : Option<String>,
    #[serde(rename = "paymentDescription")]
    pub payment_description : Option<String>,
    #[serde(rename = "redirectUrl")]
    pub redirect_url : Option<String>,
    #[serde(rename = "redirectParams")]
    pub redirect_params : Option<HashMap<String, String>>,
    #[serde(rename = "PaymentRequestId")]
    pub payment_request_id: Option<i64>,
    #[serde(rename = "GatewayUrl")]
    pub gateway_url: Option<Url>,
    #[serde(rename = "ResultInfo")]
    pub payment_result_info: Option<ResultInfo>,
    #[serde(rename = "PaymentMethod")]
    pub payment_method_response: Option<TrustpayPaymentMethod>,
    #[serde(rename = "MerchantIdentification")]
    pub merchant_identification_response: Option<MerchantIdentification>,
    #[serde(rename = "PaymentInformation")]
    pub payment_information_response: Option<BankPaymentInformationResponse>,
}

impl<F,T> TryFrom<types::ResponseRouterData<F, TrustpayPaymentsResponse, T, types::PaymentsResponseData>> for types::RouterData<F, T, types::PaymentsResponseData> {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: types::ResponseRouterData<F, TrustpayPaymentsResponse, T, types::PaymentsResponseData>) -> Result<Self,Self::Error> {
        let (status, error, payment_response_data) = get_trustpay_response(item.response, item.http_code)?;
        Ok(Self {
            status,
            response: error.map_or_else(|| Ok(payment_response_data), Err),
            ..item.data
        })
    }
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
   errors::ConnectorError
>{
    match (response.instance_id, response.payment_status, response.payment_request_id, response.gateway_url, response.payment_result_info, response.payment_information_response.clone().unwrap_or_default().status) {
        (Some(instance_id), Some(payment_status), _, _, _, _) => {
            let (status, msg) = get_transaction_status(payment_status.as_str(), response.redirect_url.clone())?;
            let form_fields = response.redirect_params.unwrap_or_else(|| {
                std::collections::HashMap::new()
            });
            let redirection_data = match response.redirect_url {
                Some(url) => Some(services::RedirectForm {
                                        endpoint: url.to_string(),
                                        method: services::Method::Post,
                                        form_fields,
                                    }),
                None              => None,
            };
            print!(">>>redirection_data{:?}", redirection_data);
            let error = if msg.is_some() {
                Some(types::ErrorResponse {
                    code: payment_status,
                    message: msg.unwrap_or_default(),
                    reason: None,
                    status_code,
                })
            } else {
                None
            };
            let payment_response_data = types::PaymentsResponseData::TransactionResponse {
                resource_id: types::ResponseId::ConnectorTransactionId(instance_id),
                redirection_data,
                mandate_reference: None,
                connector_metadata: None,
            };
            Ok((status, error, payment_response_data))
        },
        (_, _, Some(payment_request_id), Some(gateway_url), _, _) => {
            let status = enums::AttemptStatus:: AuthenticationPending;
            let error = None;
            let form_fields: HashMap<String, String> =
                HashMap::from_iter(
                    gateway_url
                        .query_pairs()
                        .map(|(key, value)| (key.to_string(), value.to_string())),
                );
            let payment_response_data = types::PaymentsResponseData::TransactionResponse {
                resource_id: types::ResponseId::ConnectorTransactionId(payment_request_id.to_string()),
                redirection_data: Some(services::RedirectForm {
                    endpoint: gateway_url.to_string(),
                    method: services::Method::Get,
                    form_fields,
                }),
                mandate_reference: None,
                connector_metadata: None,
            };
            Ok((status, error, payment_response_data))
        },
        (_, _, _, _, Some(result_info), _) => {
            let status = enums::AttemptStatus:: AuthorizationFailed;
            let error = Some(types::ErrorResponse {
                code: result_info.result_code.to_string(),
                message: result_info.additional_info.unwrap_or_default(),
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
        (_, _, _, _, _, Some(status)) => {
            let status =  enums::AttemptStatus::from(status);
            let error = if status == enums::AttemptStatus:: AuthenticationFailed {
                let reason_info = response.payment_information_response.unwrap_or_default().status_reason.unwrap_or_default();
                Some(types::ErrorResponse {
                    code: reason_info.reason.code,
                    message: reason_info.reason.reject_reason.unwrap_or_default(),
                    reason: None,
                    status_code,
                })
            } else {
                None
            };
            let payment_response_data = types::PaymentsResponseData::TransactionResponse {
                resource_id: types::ResponseId::NoResponseId,
                redirection_data: None,
                mandate_reference: None,
                connector_metadata: None,
            };
            Ok((status, error, payment_response_data))
        }
        _ => Err(errors::ConnectorError::ResponseDeserializationFailed.into()),
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
pub struct ResultInfo {
    #[serde(rename = "ResultCode")]
    pub result_code: i64,
    #[serde(rename = "AdditionalInfo")]
    pub additional_info: Option<String>,
    #[serde(rename = "CorrelationId")]
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
pub struct TrustpayAccessTokenErrorResponse {
    #[serde(rename = "ResultInfo")]
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
            (Some(access_token), Some(expires_in)) =>
                Ok(Self {
                    response: Ok(types::AccessToken {
                        token: access_token,
                        expires: expires_in,
                    }),
                    ..item.data
                }),
            _ =>
            Ok(Self {
                response: Err(types::ErrorResponse {
                    code: item.response.result_info.result_code.to_string(),
                    message: item.response.result_info.additional_info.unwrap_or_default(),
                    reason: None,
                    status_code:item.http_code,
                }),
                ..item.data
            }),
        }
    }
}

//TODO: Fill the struct with respective fields
// REFUND :
// Type definition for RefundRequest
#[derive(Default, Debug, Serialize)]
pub struct TrustpayRefundRequest {}

impl<F> TryFrom<&types::RefundsRouterData<F>> for TrustpayRefundRequest {
    type Error = error_stack::Report<errors::ParsingError>;
    fn try_from(_item: &types::RefundsRouterData<F>) -> Result<Self,Self::Error> {
       todo!()
    }
}

// Type definition for Refund Response

#[allow(dead_code)]
#[derive(Debug, Serialize, Default, Deserialize, Clone)]
enum RefundStatus {
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
}

impl TryFrom<types::RefundsResponseRouterData<api::Execute, RefundResponse>>
    for types::RefundsRouterData<api::Execute>
{
    type Error = error_stack::Report<errors::ParsingError>;
    fn try_from(
        _item: types::RefundsResponseRouterData<api::Execute, RefundResponse>,
    ) -> Result<Self, Self::Error> {
        todo!()
    }
}

impl TryFrom<types::RefundsResponseRouterData<api::RSync, RefundResponse>> for types::RefundsRouterData<api::RSync>
{
     type Error = error_stack::Report<errors::ParsingError>;
    fn try_from(_item: types::RefundsResponseRouterData<api::RSync, RefundResponse>) -> Result<Self,Self::Error> {
         todo!()
     }
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
    pub errors: Vec<Errors>
}
