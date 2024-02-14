use error_stack::report;
use masking::{ExposeInterface, Secret};
use serde::{Deserialize, Serialize};

use crate::{
    connector::utils::PaymentsAuthorizeRequestData,
    core::errors,
    types::{self, api, storage::enums, BrowserInformation},
};

//TODO: Fill the struct with respective fields
pub struct TokenexRouterData<T> {
    pub amount: i64, // The type of amount that a connector accepts, for example, String, i64, f64, etc.
    pub router_data: T,
}

impl<T>
    TryFrom<(
        &types::api::CurrencyUnit,
        types::storage::enums::Currency,
        i64,
        T,
    )> for TokenexRouterData<T>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        (_currency_unit, _currency, amount, item): (
            &types::api::CurrencyUnit,
            types::storage::enums::Currency,
            i64,
            T,
        ),
    ) -> Result<Self, Self::Error> {
        //Todo :  use utils to convert the amount to the type of amount that a connector accepts
        Ok(Self {
            amount,
            router_data: item,
        })
    }
}

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Serialize, Eq, PartialEq)]
pub struct TokenexPaymentsRequest {
    amount: i64,
    card: TokenexCard,
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
pub struct TokenexCard {
    number: cards::CardNumber,
    expiry_month: Secret<String>,
    expiry_year: Secret<String>,
    cvc: Secret<String>,
    complete: bool,
}

impl TryFrom<&TokenexRouterData<&types::PaymentsAuthorizeRouterData>> for TokenexPaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &TokenexRouterData<&types::PaymentsAuthorizeRouterData>,
    ) -> Result<Self, Self::Error> {
        match item.router_data.request.payment_method_data.clone() {
            api::PaymentMethodData::Card(req_card) => {
                let card = TokenexCard {
                    number: req_card.card_number,
                    expiry_month: req_card.card_exp_month,
                    expiry_year: req_card.card_exp_year,
                    cvc: req_card.card_cvc,
                    complete: item.router_data.request.is_auto_capture()?,
                };
                Ok(Self {
                    amount: item.amount.to_owned(),
                    card,
                })
            }
            _ => Err(errors::ConnectorError::NotImplemented("Payment methods".to_string()).into()),
        }
    }
}

//TODO: Fill the struct with respective fields
// Auth Struct
pub struct TokenexAuthType {
    pub(super) api_key: Secret<String>,
    pub(super) tokenex_id: Secret<String>,
}

impl TryFrom<&types::ConnectorAuthType> for TokenexAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &types::ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            types::ConnectorAuthType::BodyKey { api_key, key1 } => Ok(Self {
                api_key: api_key.to_owned(),
                tokenex_id: key1.to_owned(),
            }),
            _ => Err(errors::ConnectorError::FailedToObtainAuthType.into()),
        }
    }
}
// PaymentsResponse
//TODO: Append the remaining status flags
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum TokenexPaymentStatus {
    Succeeded,
    Failed,
    #[default]
    Processing,
}

impl From<TokenexPaymentStatus> for enums::AttemptStatus {
    fn from(item: TokenexPaymentStatus) -> Self {
        match item {
            TokenexPaymentStatus::Succeeded => Self::Charged,
            TokenexPaymentStatus::Failed => Self::Failure,
            TokenexPaymentStatus::Processing => Self::Authorizing,
        }
    }
}

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TokenexPaymentsResponse {
    status: TokenexPaymentStatus,
    id: String,
}

impl<F, T>
    TryFrom<types::ResponseRouterData<F, TokenexPaymentsResponse, T, types::PaymentsResponseData>>
    for types::RouterData<F, T, types::PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<F, TokenexPaymentsResponse, T, types::PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            status: enums::AttemptStatus::from(item.response.status),
            response: Ok(types::PaymentsResponseData::TransactionResponse {
                resource_id: types::ResponseId::ConnectorTransactionId(item.response.id),
                redirection_data: None,
                mandate_reference: None,
                connector_metadata: None,
                network_txn_id: None,
                connector_response_reference_id: None,
                incremental_authorization_allowed: None,
            }),
            ..item.data
        })
    }
}

//TODO: Fill the struct with respective fields
// REFUND :
// Type definition for RefundRequest
#[derive(Default, Debug, Serialize)]
pub struct TokenexRefundRequest {
    pub amount: i64,
}

impl<F> TryFrom<&TokenexRouterData<&types::RefundsRouterData<F>>> for TokenexRefundRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &TokenexRouterData<&types::RefundsRouterData<F>>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            amount: item.amount.to_owned(),
        })
    }
}

// Type definition for Refund Response

#[allow(dead_code)]
#[derive(Debug, Serialize, Default, Deserialize, Clone)]
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

impl TryFrom<types::RefundsResponseRouterData<api::Execute, RefundResponse>>
    for types::RefundsRouterData<api::Execute>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::RefundsResponseRouterData<api::Execute, RefundResponse>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(types::RefundsResponseData {
                connector_refund_id: item.response.id.to_string(),
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
                connector_refund_id: item.response.id.to_string(),
                refund_status: enums::RefundStatus::from(item.response.status),
            }),
            ..item.data
        })
    }
}

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
pub struct TokenexErrorResponse {
    pub status_code: u16,
    pub code: String,
    pub message: String,
    pub reason: Option<String>,
}

// PreAuthentication

#[derive(Default, Debug, Serialize, PartialEq)]
pub struct TokenexPreAuthenticationRequest {
    pub data: cards::CardNumber,
}

impl TryFrom<&TokenexRouterData<&types::authentication::PreAuthNRouterData>>
    for TokenexPreAuthenticationRequest
{
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(
        value: &TokenexRouterData<&types::authentication::PreAuthNRouterData>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            data: value.router_data.request.card_holder_account_number.clone(),
        })
    }
}

#[derive(Default, Debug, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TokenexPreAuthenticationResponse {
    pub token: String,
    pub three_d_secure_response: Vec<ThreeDSecureResponse>,
    pub reference_number: String,
    pub success: bool,
    pub error: String,
    pub message: String,
    pub third_party_status_code: String,
}

#[derive(Default, Debug, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ThreeDSecureResponse {
    pub ds_start_protocol_version: String,
    pub ds_end_protocol_version: String,
    pub acs_start_protocol_version: String,
    pub acs_end_protocol_version: String,
    #[serde(rename = "threeDSMethodURL")]
    pub threeds_method_url: Option<String>,
    #[serde(rename = "threeDSServerTransID")]
    pub threeds_server_trans_id: String,
}

// Authentication

#[derive(Default, Debug, Serialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub struct TokenexAuthenticationRequest {
    pub server_transaction_id: String,
    pub method_completion_indicator: MethodCompletionIndicatorEnum,
    pub message_version: String,
    pub browser_info: TokenexBrowserInformation,
    pub acquirer_bin: String,
    pub card_details: TokenexCardDetails,
    pub cardholder_details: TokenexCardholderDetails,
    // value should be 5 for full screen
    pub challenge_window_size: u8,
    pub device_channel: DeviceChannel,
    /// "SANDBOX_DS",
    pub directory_server_identifier: String,
    pub generate_challenge_request: bool,
    pub merchant_details: MerchantDetails,
    pub message_category: MessageCategory,
    pub notification_url: String,
    pub authentication_indicator: AuthenticationIndicator,
    pub purchase_details: PurchaseDetails,
    // value is 1
    pub transaction_type: i64,
}
fn get_card_details(
    payment_method_data: &api_models::payments::PaymentMethodData,
) -> Result<api_models::payments::Card, errors::ConnectorError> {
    match payment_method_data {
        api_models::payments::PaymentMethodData::Card(details) => Ok(details.to_owned()),
        _ => Err(errors::ConnectorError::RequestEncodingFailed)?,
    }
}
impl TryFrom<&TokenexRouterData<&types::ConnectorAuthenticationRouterData>>
    for TokenexAuthenticationRequest
{
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(
        value: &TokenexRouterData<&types::ConnectorAuthenticationRouterData>,
    ) -> Result<Self, Self::Error> {
        let request = &value.router_data.request;
        let browser_info = TokenexBrowserInformation::from(&request.browser_details);
        let card = get_card_details(&request.payment_method_data)?;
        let (card_expiry_date, account_type) = {
            let year = card.card_exp_year.expose();
            let month = card.card_exp_month.expose();
            let card_type = card.card_type;
            let account_type = card_type
                .and_then(|card_type| match card_type.to_lowercase().as_str() {
                    "credit" => Some(TokenexAccountType::Credit),
                    "debit" => Some(TokenexAccountType::Debit),
                    _ => None,
                })
                .unwrap_or_default();
            (format!("{}{}", year, month), account_type)
        };
        let card_details = TokenexCardDetails {
            number: request
                .authentication_data
                .1
                .authentication_connector_id
                .clone()
                .ok_or(report!(errors::ConnectorError::MissingRequiredField {
                    field_name: "authentication_connector_id"
                }))?,
            card_expiry_date: Some(card_expiry_date),
            account_type,
        };
        let cardholder_details = TokenexCardholderDetails {
            name: card.card_holder_name.map(ExposeInterface::expose),
            email_address: None,
        };
        let merchant_details = MerchantDetails {
            acquirer_merchant_id: "Acquirer Merchant Id".into(),
            category_code: "0001".into(),
            country_code: "840".into(),
            name: "Merchant Name".into(),
        };
        let purchase_details = PurchaseDetails {
            amount: 1000,
            currency: "840".into(),
            exponent: 2,
            date: "20211201091950".to_string(),
        };
        Ok(Self {
            server_transaction_id: request
                .authentication_data
                .0
                .threeds_server_transaction_id
                .clone(),
            method_completion_indicator: MethodCompletionIndicatorEnum::ThreeDSMethodSuccess,
            message_version: request.authentication_data.0.message_version.clone(),
            browser_info,
            acquirer_bin: card.card_number.get_extended_card_bin(),
            card_details,
            cardholder_details,
            challenge_window_size: 5,
            device_channel: DeviceChannel::Browser,
            directory_server_identifier: "SANDBOX_DS".into(),
            generate_challenge_request: true,
            merchant_details,
            message_category: MessageCategory::Payment,
            notification_url: request
                .return_url
                .clone()
                .ok_or(errors::ConnectorError::RequestEncodingFailed)?,
            authentication_indicator: AuthenticationIndicator::PaymentTransaction,
            purchase_details,
            transaction_type: 1,
        })
    }
}

#[derive(Default, Debug, Serialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub struct PurchaseDetails {
    amount: i64,
    /// three digit currency code 840
    currency: String,
    /// 2
    exponent: i64,
    /// epoch representation
    date: String,
}

#[repr(u8)]
#[derive(Default, Debug, Serialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub enum MessageCategory {
    #[default]
    #[serde(rename = "1")]
    Payment = 1,
    #[serde(rename = "2")]
    NonPayment = 2,
}

#[repr(u8)]
#[derive(Default, Debug, Serialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub enum AuthenticationIndicator {
    #[default]
    #[serde(rename = "1")]
    PaymentTransaction = 1,
    #[serde(rename = "2")]
    RecurringTransaction = 2,
    #[serde(rename = "3")]
    InstallmentTransaction = 3,
    #[serde(rename = "4")]
    AddCard = 4,
    #[serde(rename = "5")]
    MaintainCard = 5,
    #[serde(rename = "6")]
    CardholderVerification = 6,
}

#[repr(u8)]
#[derive(Default, Debug, Serialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub enum DeviceChannel {
    #[default]
    #[serde(rename = "2")]
    Browser = 2,
    #[serde(rename = "3")]
    ThreeRI = 3,
}

#[derive(Default, Debug, Serialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub struct TokenexBrowserInformation {
    pub color_depth: Option<String>,
    pub java_enabled: Option<bool>,
    pub java_script_enabled: Option<bool>,
    pub language: Option<String>,
    pub screen_height: Option<String>,
    pub screen_width: Option<String>,
    pub time_zone: Option<String>,
    pub ip_address: Option<std::net::IpAddr>,
    pub accept_headers: Option<String>,
    pub user_agent: Option<String>,
}

#[derive(Default, Debug, Serialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub struct MerchantDetails {
    acquirer_merchant_id: String,
    category_code: String,
    country_code: String,
    name: String,
}

impl From<&BrowserInformation> for TokenexBrowserInformation {
    fn from(value: &BrowserInformation) -> Self {
        Self {
            color_depth: value.color_depth.as_ref().map(ToString::to_string),
            java_enabled: value.java_enabled,
            java_script_enabled: value.java_script_enabled,
            language: value.language.clone(),
            screen_height: value.screen_height.as_ref().map(ToString::to_string),
            screen_width: value.screen_width.as_ref().map(ToString::to_string),
            time_zone: value.time_zone.as_ref().map(ToString::to_string),
            ip_address: value.ip_address,
            accept_headers: value.accept_header.clone(),
            user_agent: value.user_agent.clone(),
        }
    }
}
#[derive(Default, Debug, Serialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub struct TokenexCardDetails {
    /// Tokenex token
    number: String,
    /// YYMM format
    card_expiry_date: Option<String>,
    /// debit or credit
    account_type: TokenexAccountType,
}

#[derive(Default, Debug, Serialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub struct TokenexCardholderDetails {
    name: Option<String>,
    email_address: Option<String>,
}

#[repr(u8)]
#[derive(Default, Debug, Serialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub enum TokenexAccountType {
    #[default]
    #[serde(rename = "1")]
    NotApplicable = 1,
    #[serde(rename = "2")]
    Credit = 2,
    #[serde(rename = "3")]
    Debit = 3,
}

#[repr(u8)]
#[derive(Default, Debug, Serialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub enum MethodCompletionIndicatorEnum {
    #[default]
    #[serde(rename = "1")]
    ThreeDSMethodSuccess = 1,
    #[serde(rename = "2")]
    ThreeDSMethodUnSuccessful = 2,
    #[serde(rename = "3")]
    ResultUnavailable = 3,
}

// TokenEx returns 2xx even for errors
#[derive(Debug, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum TokenexAuthenticationResponse {
    Success(Box<TokenexAuthenticationResponseBody>),
    Error(Box<TokenexAuthenticationErrorResponseBody>),
}

#[derive(Debug, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TokenexAuthenticationResponseBody {
    pub token: String,
    pub three_d_secure_response: ThreeDSecureAuthNResponseEnum,
    pub reference_number: String,
    pub success: bool,
    pub error: String,
    pub message: String,
    pub third_party_status_code: String,
}

#[derive(Default, Debug, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TokenexAuthenticationErrorResponseBody {
    pub reference_number: String,
    pub success: bool,
    pub error: String,
    pub message: String,
}

#[derive(Debug, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum ThreeDSecureAuthNResponseEnum {
    Success(Box<ThreeDSecureAuthNResponse>),
    Error(Box<ThreeDSecureAuthNErrorResponse>),
}
#[derive(Default, Debug, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ThreeDSecureAuthNResponse {
    #[serde(rename = "acsChallengeMandated")]
    pub acs_challenge_mandated: Option<String>,
    #[serde(rename = "acsOperatorID")]
    pub acs_operator_id: String,
    #[serde(rename = "acsReferenceNumber")]
    pub acs_reference_number: String,
    #[serde(rename = "acsTransID")]
    pub acs_trans_id: String,
    #[serde(rename = "acsURL")]
    pub acs_url: Option<url::Url>,
    #[serde(rename = "authenticationType")]
    pub authentication_type: Option<String>,
    #[serde(rename = "dsReferenceNumber")]
    pub ds_reference_number: String,
    #[serde(rename = "dsTransID")]
    pub ds_trans_id: String,
    #[serde(rename = "messageType")]
    pub message_type: Option<String>,
    #[serde(rename = "messageVersion")]
    pub message_version: String,
    #[serde(rename = "threeDSServerTransID")]
    pub three_dsserver_trans_id: String,
    #[serde(rename = "transStatus")]
    pub trans_status: String,
    pub encoded_c_req: String,
}

#[derive(Default, Debug, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ThreeDSecureAuthNErrorResponse {
    pub error_code: String,
    pub error_detail: String,
    pub error_description: String,
    #[serde(rename = "threeDSServerTransID")]
    pub three_ds_server_trans_id: String,
    pub error_component: String,
}

// Post Authentication

#[derive(Default, Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TokenexPostAuthenticationRequest {
    pub server_transaction_id: String,
}

#[derive(Default, Debug, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TokenexPostAuthenticationResponse {
    pub three_d_secure_response: TokenexThreeDSResponse,
}
#[derive(Default, Debug, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TokenexThreeDSResponse {
    pub authentication_value: Option<String>,
    pub trans_status: String,
    pub eci: Option<String>,
}

pub fn get_router_response_from_tokenex_authn_response(
    tokenex_response: &TokenexAuthenticationResponse,
    status_code: u16,
) -> Result<types::ConnectorAuthenticationResponse, types::ErrorResponse> {
    match tokenex_response {
        TokenexAuthenticationResponse::Success(tokenex_authn_response) => {
            match &tokenex_authn_response.three_d_secure_response {
                ThreeDSecureAuthNResponseEnum::Success(threeds_response) => {
                    println!("creq_base64 authn {}", &threeds_response.encoded_c_req);
                    Ok(types::ConnectorAuthenticationResponse {
                        trans_status: threeds_response.trans_status.clone(),
                        acs_url: threeds_response.acs_url.clone(),
                        challenge_request: if threeds_response.trans_status != "Y" {
                            Some(threeds_response.encoded_c_req.clone())
                        } else {
                            None
                        },
                        acs_reference_number: Some(threeds_response.acs_reference_number.clone()),
                        acs_trans_id: Some(threeds_response.acs_trans_id.clone()),
                        three_dsserver_trans_id: Some(
                            threeds_response.three_dsserver_trans_id.clone(),
                        ),
                        acs_signed_content: None,
                    })
                }
                ThreeDSecureAuthNResponseEnum::Error(threeds_error_response) => {
                    Err(types::ErrorResponse {
                        code: threeds_error_response.error_code.clone(),
                        message: threeds_error_response.error_description.clone(),
                        reason: Some(threeds_error_response.error_detail.clone()),
                        status_code,
                        attempt_status: None,
                        connector_transaction_id: Some(tokenex_authn_response.token.clone()),
                    })
                }
            }
        }
        TokenexAuthenticationResponse::Error(tokenex_error_response) => Err(types::ErrorResponse {
            code: tokenex_error_response.error.clone(),
            message: tokenex_error_response.error.clone(),
            reason: Some(tokenex_error_response.error.clone()),
            status_code,
            attempt_status: None,
            connector_transaction_id: None,
        }),
    }
}
