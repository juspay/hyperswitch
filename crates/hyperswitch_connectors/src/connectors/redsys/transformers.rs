use base64::{engine::general_purpose::STANDARD, Engine};
use common_enums::enums;
use common_utils::{ext_traits::ValueExt, types::StringMinorUnit};
use error_stack::ResultExt;
use hmac::{Hmac, Mac, NewMac};
use hyperswitch_domain_models::{
    payment_method_data::PaymentMethodData,
    router_data::{ConnectorAuthType, ErrorResponse, RouterData},
    router_flow_types::refunds::Execute,
    router_request_types::{
        BrowserInformation, CompleteAuthorizeData, PaymentsAuthorizeData, PaymentsCancelData,
        PaymentsCaptureData, PaymentsPreProcessingData, ResponseId,
    },
    router_response_types::{PaymentsResponseData, RedirectForm, RefundsResponseData},
    types::{
        PaymentsAuthorizeRouterData, PaymentsCancelRouterData, PaymentsCaptureRouterData,
        PaymentsCompleteAuthorizeRouterData, PaymentsPreProcessingRouterData, RefundsRouterData,
    },
};
use hyperswitch_interfaces::errors;
use masking::{ExposeInterface, Secret};
use openssl::symm::{encrypt, Cipher};
use serde::{Deserialize, Serialize};
use sha2::Sha256;

use crate::{
    types::{RefundsResponseRouterData, ResponseRouterData},
    utils::{
        generate_12_digit_number, get_unimplemented_payment_method_error_message,
        is_payment_failure, is_refund_failure, to_connector_meta, BrowserInformationData, CardData,
        PaymentsAuthorizeRequestData, PaymentsCompleteAuthorizeRequestData,
        PaymentsPreProcessingRequestData,
    },
};
type Error = error_stack::Report<errors::ConnectorError>;

pub struct RedsysRouterData<T> {
    pub amount: StringMinorUnit,
    pub currency: api_models::enums::Currency,
    pub router_data: T,
}

impl<T> From<(StringMinorUnit, T, api_models::enums::Currency)> for RedsysRouterData<T> {
    fn from((amount, item, currency): (StringMinorUnit, T, api_models::enums::Currency)) -> Self {
        Self {
            amount,
            currency,
            router_data: item,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub struct PaymentsRequest {
    ds_merchant_emv3ds: Option<EmvThreedsData>,
    ds_merchant_transactiontype: RedsysTransactionType,
    ds_merchant_currency: String,
    ds_merchant_pan: cards::CardNumber,
    ds_merchant_merchantcode: Secret<String>,
    ds_merchant_terminal: Secret<String>,
    ds_merchant_order: String,
    ds_merchant_amount: StringMinorUnit,
    ds_merchant_expirydate: Secret<String>,
    ds_merchant_cvv2: Secret<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EmvThreedsData {
    three_d_s_info: RedsysThreeDsInfo,
    protocol_version: Option<String>,
    browser_accept_header: Option<String>,
    browser_user_agent: Option<String>,
    browser_java_enabled: Option<bool>,
    browser_java_script_enabled: Option<bool>,
    browser_language: Option<String>,
    browser_color_depth: Option<String>,
    browser_screen_height: Option<String>,
    browser_screen_width: Option<String>,
    browser_t_z: Option<String>,
    three_d_s_server_trans_i_d: Option<String>,
    notification_u_r_l: Option<String>,
    three_d_s_comp_ind: Option<ThreeDSCompInd>,
    cres: Option<String>,
}

impl EmvThreedsData {
    pub fn new(three_d_s_info: RedsysThreeDsInfo) -> Self {
        Self {
            three_d_s_info,
            protocol_version: None,
            browser_accept_header: None,
            browser_user_agent: None,
            browser_java_enabled: None,
            browser_java_script_enabled: None,
            browser_language: None,
            browser_color_depth: None,
            browser_screen_height: None,
            browser_screen_width: None,
            browser_t_z: None,
            three_d_s_server_trans_i_d: None,
            notification_u_r_l: None,
            three_d_s_comp_ind: None,
            cres: None,
        }
    }

    pub fn add_browser_data(mut self, browser_info: BrowserInformation) -> Result<Self, Error> {
        self.browser_accept_header = Some(browser_info.get_accept_header()?);
        self.browser_user_agent = Some(browser_info.get_user_agent()?);
        self.browser_java_enabled = Some(browser_info.get_java_enabled()?);
        self.browser_java_script_enabled = browser_info.get_java_script_enabled().ok();
        self.browser_language = Some(browser_info.get_language()?);
        self.browser_color_depth = Some(browser_info.get_color_depth()?.to_string());
        self.browser_screen_height = Some(browser_info.get_screen_height()?.to_string());
        self.browser_screen_width = Some(browser_info.get_screen_width()?.to_string());
        self.browser_t_z = Some(browser_info.get_time_zone()?.to_string());
        Ok(self)
    }

    pub fn set_three_d_s_server_trans_i_d(mut self, three_d_s_server_trans_i_d: String) -> Self {
        self.three_d_s_server_trans_i_d = Some(three_d_s_server_trans_i_d);
        self
    }

    pub fn set_protocol_version(mut self, protocol_version: String) -> Self {
        self.protocol_version = Some(protocol_version);
        self
    }

    pub fn set_notification_u_r_l(mut self, notification_u_r_l: String) -> Self {
        self.notification_u_r_l = Some(notification_u_r_l);
        self
    }

    pub fn set_three_d_s_comp_ind(mut self, three_d_s_comp_ind: ThreeDSCompInd) -> Self {
        self.three_d_s_comp_ind = Some(three_d_s_comp_ind);
        self
    }

    pub fn set_three_d_s_cres(mut self, cres: String) -> Self {
        self.cres = Some(cres);
        self
    }
}

#[derive(Debug)]
pub struct RedsysCardData {
    card_number: cards::CardNumber,
    expiry_date: Secret<String>,
    cvv2: Secret<String>,
}

impl TryFrom<&Option<PaymentMethodData>> for RedsysCardData {
    type Error = Error;
    fn try_from(payment_method_data: &Option<PaymentMethodData>) -> Result<Self, Self::Error> {
        match payment_method_data {
            Some(PaymentMethodData::Card(card)) => {
                let year = card.get_card_expiry_year_2_digit()?.expose();
                let month = card.get_card_expiry_month_2_digit()?.expose();
                let expiry_date = Secret::new(format!("{}{}", year, month));
                Ok(Self {
                    card_number: card.card_number.clone(),
                    expiry_date,
                    cvv2: card.card_cvc.clone(),
                })
            }
            _ => Err(errors::ConnectorError::NotImplemented(
                get_unimplemented_payment_method_error_message("Redsys"),
            )
            .into()),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum RedsysThreeDsInfo {
    CardData,
    CardConfiguration,
    ChallengeRequest,
    ChallengeResponse,
    AuthenticationData,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum RedsysTransactionType {
    #[serde(rename = "0")]
    Payment,
    #[serde(rename = "1")]
    Preauthorization,
    #[serde(rename = "2")]
    Confirmation,
    #[serde(rename = "3")]
    Refund,
    #[serde(rename = "9")]
    Cancellation,
}

pub struct RedsysAuthType {
    pub(super) merchant_id: Secret<String>,
    pub(super) terminal_id: Secret<String>,
    pub(super) sha256_pwd: Secret<String>,
}

impl TryFrom<&ConnectorAuthType> for RedsysAuthType {
    type Error = Error;
    fn try_from(auth_type: &ConnectorAuthType) -> Result<Self, Self::Error> {
        if let ConnectorAuthType::SignatureKey {
            api_key,
            key1,
            api_secret,
        } = auth_type
        {
            Ok(Self {
                merchant_id: api_key.to_owned(),
                terminal_id: key1.to_owned(),
                sha256_pwd: api_secret.to_owned(),
            })
        } else {
            Err(errors::ConnectorError::FailedToObtainAuthType)?
        }
    }
}

impl TryFrom<&RedsysRouterData<&PaymentsPreProcessingRouterData>> for RedsysTransaction {
    type Error = Error;
    fn try_from(
        item: &RedsysRouterData<&PaymentsPreProcessingRouterData>,
    ) -> Result<Self, Self::Error> {
        let auth = RedsysAuthType::try_from(&item.router_data.connector_auth_type)?;
        let redsys_preprocessing_request =
            if item.router_data.auth_type == enums::AuthenticationType::ThreeDs {
                let ds_merchant_emv3ds = Some(EmvThreedsData::new(RedsysThreeDsInfo::CardData));
                let ds_merchant_transactiontype = if item.router_data.request.is_auto_capture()? {
                    RedsysTransactionType::Payment
                } else {
                    RedsysTransactionType::Preauthorization
                };
                let ds_merchant_order = generate_12_digit_number().to_string();
                let card_data =
                    RedsysCardData::try_from(&item.router_data.request.payment_method_data)?;
                Ok(PaymentsRequest {
                    ds_merchant_emv3ds,
                    ds_merchant_transactiontype,
                    ds_merchant_currency: item.currency.iso_4217().to_owned(),
                    ds_merchant_pan: card_data.card_number,
                    ds_merchant_merchantcode: auth.merchant_id.clone(),
                    ds_merchant_terminal: auth.terminal_id.clone(),
                    ds_merchant_order,
                    ds_merchant_amount: item.amount.clone(),
                    ds_merchant_expirydate: card_data.expiry_date,
                    ds_merchant_cvv2: card_data.cvv2,
                })
            } else {
                Err(errors::ConnectorError::FlowNotSupported {
                    flow: "PreProcessing".to_string(),
                    connector: "Redsys".to_string(),
                })
            }?;

        Self::try_from((&redsys_preprocessing_request, &auth))
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(untagged)]
pub enum RedsysResponse {
    RedsysResponse(RedsysTransaction),
    RedsysErrorResponse(RedsysErrorResponse),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct RedsysErrorResponse {
    pub error_code: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CardPSD2 {
    Y,
    N,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ThreeDSCompInd {
    Y,
    N,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RedsysPaymentsResponse {
    #[serde(rename = "Ds_Order")]
    ds_order: String,
    #[serde(rename = "Ds_EMV3DS")]
    ds_emv3ds: Option<RedsysEmv3DSData>,
    #[serde(rename = "Ds_Card_PSD2")]
    ds_card_psd2: Option<CardPSD2>,
    #[serde(rename = "Ds_Response")]
    ds_response: Option<DsResponse>,
    #[serde(rename = "Ds_AuthorisationCode")]
    ds_authorisationcode: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DsResponse(String);

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RedsysEmv3DSData {
    protocol_version: String,
    three_d_s_server_trans_i_d: Option<String>,
    three_d_s_info: Option<RedsysThreeDsInfo>,
    three_d_s_method_u_r_l: Option<String>,
    acs_u_r_l: Option<String>,
    creq: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThreedsInvokeRequest {
    three_d_s_server_trans_i_d: String,
    three_d_s_method_notification_u_r_l: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RedsysThreeDsInvokeData {
    pub three_ds_method_url: String,
    pub three_ds_method_data: String,
    pub message_version: String,
    pub directory_server_id: String,
    pub three_ds_method_data_submission: bool,
    pub next_action_url: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ThreeDsInvokeExempt {
    pub three_d_s_server_trans_i_d: String,
    pub message_version: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RedsysTransaction {
    #[serde(rename = "Ds_SignatureVersion")]
    ds_signature_version: String,
    #[serde(rename = "Ds_MerchantParameters")]
    ds_merchant_parameters: Secret<String>,
    #[serde(rename = "Ds_Signature")]
    ds_signature: Secret<String>,
}

fn to_connector_response_data<T>(connector_response: &str) -> Result<T, Error>
where
    T: serde::de::DeserializeOwned,
{
    let decoded_bytes = STANDARD
        .decode(connector_response)
        .change_context(errors::ConnectorError::ResponseDeserializationFailed)
        .attach_printable("Failed to decode Base64")?;

    let response_data: T = serde_json::from_slice(&decoded_bytes)
        .change_context(errors::ConnectorError::ResponseHandlingFailed)?;

    Ok(response_data)
}

impl<F>
    TryFrom<ResponseRouterData<F, RedsysResponse, PaymentsPreProcessingData, PaymentsResponseData>>
    for RouterData<F, PaymentsPreProcessingData, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(
        item: ResponseRouterData<
            F,
            RedsysResponse,
            PaymentsPreProcessingData,
            PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        match item.response.clone() {
            RedsysResponse::RedsysResponse(response) => {
                let response_data: RedsysPaymentsResponse =
                    to_connector_response_data(&response.ds_merchant_parameters.clone().expose())?;
                handle_redsys_preprocessing_response(item, &response_data)
            }
            RedsysResponse::RedsysErrorResponse(response) => {
                let response = Err(ErrorResponse {
                    code: response.error_code.clone(),
                    message: response.error_code.clone(),
                    reason: Some(response.error_code.clone()),
                    status_code: item.http_code,
                    attempt_status: None,
                    connector_transaction_id: None,
                });

                Ok(Self {
                    status: enums::AttemptStatus::Failure,
                    response,
                    ..item.data
                })
            }
        }
    }
}

fn handle_redsys_preprocessing_response<F>(
    item: ResponseRouterData<F, RedsysResponse, PaymentsPreProcessingData, PaymentsResponseData>,
    response_data: &RedsysPaymentsResponse,
) -> Result<
    RouterData<F, PaymentsPreProcessingData, PaymentsResponseData>,
    error_stack::Report<errors::ConnectorError>,
> {
    match (
        response_data
            .ds_emv3ds
            .as_ref()
            .and_then(|emv3ds_data| emv3ds_data.three_d_s_method_u_r_l.clone()),
        response_data
            .ds_emv3ds
            .as_ref()
            .and_then(|emv3ds_data| emv3ds_data.three_d_s_server_trans_i_d.clone()),
        response_data
            .ds_emv3ds
            .as_ref()
            .map(|emv3ds_data| emv3ds_data.protocol_version.clone()),
    ) {
        (
            Some(three_d_s_method_u_r_l),
            Some(three_d_s_server_trans_i_d),
            Some(protocol_version),
        ) => handle_threeds_invoke(
            item,
            response_data,
            three_d_s_method_u_r_l,
            three_d_s_server_trans_i_d,
            protocol_version,
        ),
        (None, Some(three_d_s_server_trans_i_d), Some(protocol_version)) => {
            handle_threeds_invoke_exempt(
                item,
                response_data,
                three_d_s_server_trans_i_d,
                protocol_version,
            )
        }
        _ => Err(errors::ConnectorError::NotSupported {
            message: "3DS payment with a non-3DS card".to_owned(),
            connector: "Redsys",
        }
        .into()),
    }
}

fn handle_threeds_invoke<F>(
    item: ResponseRouterData<F, RedsysResponse, PaymentsPreProcessingData, PaymentsResponseData>,
    response_data: &RedsysPaymentsResponse,
    three_d_s_method_u_r_l: String,
    three_d_s_server_trans_i_d: String,
    protocol_version: String,
) -> Result<
    RouterData<F, PaymentsPreProcessingData, PaymentsResponseData>,
    error_stack::Report<errors::ConnectorError>,
> {
    let three_d_s_method_notification_u_r_l = item.data.request.get_webhook_url()?;

    let threeds_invoke_data = ThreedsInvokeRequest {
        three_d_s_server_trans_i_d: three_d_s_method_u_r_l.clone(),
        three_d_s_method_notification_u_r_l,
    };

    let three_ds_data_string = serde_json::to_string(&threeds_invoke_data)
        .change_context(errors::ConnectorError::RequestEncodingFailed)?;

    let three_ds_method_data = STANDARD.encode(&three_ds_data_string);
    let next_action_url = item.data.request.get_complete_authorize_url()?;

    let three_ds_data = RedsysThreeDsInvokeData {
        three_ds_method_url: three_d_s_method_u_r_l,
        three_ds_method_data,
        message_version: protocol_version.clone(),
        directory_server_id: three_d_s_server_trans_i_d,
        three_ds_method_data_submission: true,
        next_action_url,
    };

    let connector_metadata = Some(
        serde_json::to_value(&three_ds_data)
            .change_context(errors::ConnectorError::RequestEncodingFailed)
            .attach_printable("Failed to serialize ThreeDsData")?,
    );

    Ok(RouterData {
        status: enums::AttemptStatus::AuthenticationPending,
        response: Ok(PaymentsResponseData::TransactionResponse {
            resource_id: ResponseId::ConnectorTransactionId(response_data.ds_order.clone()),
            redirection_data: Box::new(None),
            mandate_reference: Box::new(None),
            connector_metadata,
            network_txn_id: None,
            connector_response_reference_id: None,
            incremental_authorization_allowed: None,
            charges: None,
        }),
        ..item.data
    })
}

fn handle_threeds_invoke_exempt<F>(
    item: ResponseRouterData<F, RedsysResponse, PaymentsPreProcessingData, PaymentsResponseData>,
    response_data: &RedsysPaymentsResponse,
    three_d_s_server_trans_i_d: String,
    protocol_version: String,
) -> Result<
    RouterData<F, PaymentsPreProcessingData, PaymentsResponseData>,
    error_stack::Report<errors::ConnectorError>,
> {
    let three_ds_data = ThreeDsInvokeExempt {
        message_version: protocol_version.clone(),
        three_d_s_server_trans_i_d,
    };

    let connector_metadata = Some(
        serde_json::to_value(&three_ds_data)
            .change_context(errors::ConnectorError::RequestEncodingFailed)
            .attach_printable("Failed to serialize ThreeDsData")?,
    );

    Ok(RouterData {
        status: enums::AttemptStatus::AuthenticationPending,
        response: Ok(PaymentsResponseData::TransactionResponse {
            resource_id: ResponseId::ConnectorTransactionId(response_data.ds_order.clone()),
            redirection_data: Box::new(None),
            mandate_reference: Box::new(None),
            connector_metadata,
            network_txn_id: None,
            connector_response_reference_id: None,
            incremental_authorization_allowed: None,
            charges: None,
        }),
        ..item.data
    })
}

pub const SIGNATURE_VERSION: &str = "HMAC_SHA256_V1";
#[derive(Debug, Serialize)]
pub struct RedsysRequest {
    #[serde(rename = "Ds_SignatureVersion")]
    ds_signature_version: String,
    #[serde(rename = "Ds_MerchantParameters")]
    ds_merchant_parameters: Secret<String>,
    #[serde(rename = "Ds_Signature")]
    ds_signature: Secret<String>,
}

fn base64_decode(input: &str) -> Result<Vec<u8>, error_stack::Report<errors::ConnectorError>> {
    STANDARD
        .decode(input)
        .change_context(errors::ConnectorError::RequestEncodingFailed)
        .attach_printable("Base64 decoding failed")
}

fn des_encrypt(
    message: &str,
    key: &str,
) -> Result<String, error_stack::Report<errors::ConnectorError>> {
    let iv_array = [0u8; 8]; // IV of 8 zero bytes
    let iv = iv_array.to_vec();
    // Decode the Base64 key (must be 24 bytes for 3DES)
    let key_bytes = base64_decode(key)?;
    if key_bytes.len() != 24 {
        return Err(
            error_stack::Report::new(errors::ConnectorError::RequestEncodingFailed)
                .attach_printable("Key must be 24 bytes for 3DES"),
        );
    }

    // Prepare plaintext with ZeroPadding
    let block_size = 8;
    let mut buffer = message.as_bytes().to_vec();
    let pad_len = block_size - (buffer.len() % block_size);
    if pad_len != block_size {
        buffer.extend(vec![0u8; pad_len]); // ZeroPadding to match CryptoJS
    }
    // Encrypt using OpenSSL's 3DES CBC
    let cipher = Cipher::des_ede3_cbc();
    let encrypted = encrypt(cipher, &key_bytes, Some(&iv), &buffer)
        .change_context(errors::ConnectorError::RequestEncodingFailed)
        .attach_printable("Triple DES encryption failed")?;
    let expected_len = buffer.len();
    let encrypted_trimmed = encrypted
        .get(..expected_len)
        .ok_or(errors::ConnectorError::RequestEncodingFailed)
        .attach_printable("Failed to trim encrypted data to the expected length")?;
    let encoded = STANDARD.encode(encrypted_trimmed);
    Ok(encoded)
}

fn get_signature(
    order_id: &str,
    params: &str,
    clave: &str,
) -> Result<String, error_stack::Report<errors::ConnectorError>> {
    let secret_ko = des_encrypt(order_id, clave)?;
    let base_decoded = base64_decode(&secret_ko)?;

    // HMAC-SHA256
    let mut mac = Hmac::<Sha256>::new_from_slice(&base_decoded)
        .map_err(|_| errors::ConnectorError::RequestEncodingFailed)
        .attach_printable("HMAC-SHA256 initialization failed")?;
    mac.update(params.as_bytes());
    let result = mac.finalize().into_bytes();
    let encoded = STANDARD.encode(result);
    Ok(encoded)
}

pub trait SignatureCalculationData {
    fn get_merchant_parameters(&self) -> Result<String, Error>;
    fn get_order_id(&self) -> String;
}

impl SignatureCalculationData for PaymentsRequest {
    fn get_merchant_parameters(&self) -> Result<String, Error> {
        serde_json::to_string(self)
            .change_context(errors::ConnectorError::RequestEncodingFailed)
            .attach_printable("Failed Serialization of PaymentsRequest struct")
    }

    fn get_order_id(&self) -> String {
        self.ds_merchant_order.clone()
    }
}

impl SignatureCalculationData for RedsysOperationRequest {
    fn get_merchant_parameters(&self) -> Result<String, Error> {
        serde_json::to_string(self)
            .change_context(errors::ConnectorError::RequestEncodingFailed)
            .attach_printable("Failed Serialization of RedsysOperationRequest struct")
    }

    fn get_order_id(&self) -> String {
        self.ds_merchant_order.clone()
    }
}

impl<T> TryFrom<(&T, &RedsysAuthType)> for RedsysTransaction
where
    T: SignatureCalculationData,
{
    type Error = Error;
    fn try_from(data: (&T, &RedsysAuthType)) -> Result<Self, Self::Error> {
        let (request_data, auth) = data;
        let merchant_parameters = request_data.get_merchant_parameters()?;
        let ds_merchant_parameters = STANDARD.encode(&merchant_parameters);
        let sha256_pwd = auth.sha256_pwd.clone().expose();
        let ds_merchant_order = request_data.get_order_id();

        let signature = get_signature(&ds_merchant_order, &ds_merchant_parameters, &sha256_pwd)?;
        Ok(Self {
            ds_signature_version: SIGNATURE_VERSION.to_string(),
            ds_merchant_parameters: Secret::new(ds_merchant_parameters),
            ds_signature: Secret::new(signature),
        })
    }
}

// Yet to be confirmed from redsys support
fn map_redsys_attempt_status(
    ds_response: DsResponse,
    capture_method: Option<enums::CaptureMethod>,
) -> Result<enums::AttemptStatus, error_stack::Report<errors::ConnectorError>> {
    if ds_response.0.starts_with("00") {
        match capture_method {
            Some(enums::CaptureMethod::Automatic) | None => Ok(enums::AttemptStatus::Charged),
            Some(enums::CaptureMethod::Manual) => Ok(enums::AttemptStatus::Authorized),
            _ => Err(errors::ConnectorError::CaptureMethodNotSupported.into()),
        }
    } else {
        match ds_response.0.as_str() {
            "0900" => Ok(enums::AttemptStatus::Charged),
            "400" => Ok(enums::AttemptStatus::Voided),
            "950" => Ok(enums::AttemptStatus::VoidFailed),
            "9998" | "9999" => Ok(enums::AttemptStatus::Pending),
            "9256" | "9257" => Ok(enums::AttemptStatus::AuthenticationFailed),
            "101" | "102" | "106" | "125" | "129" | "172" | "173" | "174" | "180" | "184"
            | "190" | "191" | "195" | "202" | "904" | "909" | "913" | "944" | "9912" | "912"
            | "9064" | "9078" | "9093" | "9094" | "9104" | "9218" | "9253" | "9261" | "9915"
            | "9997" => Ok(enums::AttemptStatus::Failure),
            error => Err(errors::ConnectorError::ResponseHandlingFailed)
                .attach_printable(format!("Recieved Unknown Status:{}", error))?,
        }
    }
}

impl TryFrom<&RedsysRouterData<&PaymentsAuthorizeRouterData>> for RedsysTransaction {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &RedsysRouterData<&PaymentsAuthorizeRouterData>,
    ) -> Result<Self, Self::Error> {
        let auth = RedsysAuthType::try_from(&item.router_data.connector_auth_type)?;
        let ds_merchant_transactiontype = if item.router_data.request.is_auto_capture()? {
            RedsysTransactionType::Payment
        } else {
            RedsysTransactionType::Preauthorization
        };
        let card_data = RedsysCardData::try_from(&Some(
            item.router_data.request.payment_method_data.clone(),
        ))?;
        if item.router_data.auth_type == enums::AuthenticationType::ThreeDs {
            let (connector_meatadata, ds_merchant_order) = match &item.router_data.response {
                Ok(PaymentsResponseData::TransactionResponse {
                    resource_id,
                    connector_metadata,
                    ..
                }) => match resource_id {
                    ResponseId::ConnectorTransactionId(order_id) => {
                        (connector_metadata.clone(), order_id.clone())
                    }
                    _ => Err(errors::ConnectorError::ResponseHandlingFailed)?,
                },
                _ => Err(errors::ConnectorError::ResponseHandlingFailed)?,
            };

            let threeds_invoke_meta_data = to_connector_meta::<ThreeDsInvokeExempt>(connector_meatadata.clone()).change_context(errors::ConnectorError::InvalidConnectorConfig {
                config: "metadata",
            })?;
            let emv3ds_data = 
                EmvThreedsData::new(RedsysThreeDsInfo::AuthenticationData)
                    .set_three_d_s_server_trans_i_d(
                        threeds_invoke_meta_data.three_d_s_server_trans_i_d,
                    )
                    .set_protocol_version(threeds_invoke_meta_data.message_version)
                    .set_notification_u_r_l(item.router_data.request.get_complete_authorize_url()?)
                    .add_browser_data(item.router_data.request.get_browser_info()?)?
                    .set_three_d_s_comp_ind(ThreeDSCompInd::N);

            let payment_authorize_request = PaymentsRequest {
                ds_merchant_emv3ds: Some(emv3ds_data),
                ds_merchant_transactiontype,
                ds_merchant_currency: item.currency.iso_4217().to_owned(),
                ds_merchant_pan: card_data.card_number,
                ds_merchant_merchantcode: auth.merchant_id.clone(),
                ds_merchant_terminal: auth.terminal_id.clone(),
                ds_merchant_order,
                ds_merchant_amount: item.amount.clone(),
                ds_merchant_expirydate: card_data.expiry_date,
                ds_merchant_cvv2: card_data.cvv2,
            };
            Self::try_from((&payment_authorize_request, &auth))
        } else {
        
                Err(errors::ConnectorError::NotImplemented(
                    get_unimplemented_payment_method_error_message("Redsys"),
                ).into())
        }
    }
}

fn build_threeds_form(ds_emv3ds: &RedsysEmv3DSData) -> Result<RedirectForm, Error> {
    let creq = ds_emv3ds
        .creq
        .clone()
        .ok_or(errors::ConnectorError::ResponseDeserializationFailed)?;

    let endpoint = ds_emv3ds
        .acs_u_r_l
        .clone()
        .ok_or(errors::ConnectorError::ResponseDeserializationFailed)?;

    let mut form_fields = std::collections::HashMap::new();
    form_fields.insert("creq".to_string(), creq);

    Ok(RedirectForm::Form {
        endpoint,
        method: common_utils::request::Method::Post,
        form_fields,
    })
}

impl<F> TryFrom<ResponseRouterData<F, RedsysResponse, PaymentsAuthorizeData, PaymentsResponseData>>
    for RouterData<F, PaymentsAuthorizeData, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, RedsysResponse, PaymentsAuthorizeData, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        let (response, status) = match item.response.clone() {
            RedsysResponse::RedsysResponse(transaction_response) => {
                let connector_metadata = match item.data.response {
                    Ok(PaymentsResponseData::TransactionResponse {
                        connector_metadata, ..
                    }) => connector_metadata,
                    _ => None,
                };
                let response_data: RedsysPaymentsResponse = to_connector_response_data(
                    &transaction_response.ds_merchant_parameters.clone().expose(),
                )?;
                get_payments_response(
                    response_data,
                    item.data.request.capture_method,
                    connector_metadata,
                    item.http_code,
                )?
            }
            RedsysResponse::RedsysErrorResponse(response) => {
                let response = Err(ErrorResponse {
                    code: response.error_code.clone(),
                    message: response.error_code.clone(),
                    reason: Some(response.error_code.clone()),
                    status_code: item.http_code,
                    attempt_status: None,
                    connector_transaction_id: None,
                });

                (response, enums::AttemptStatus::Failure)
            }
        };
        Ok(Self {
            status,
            response,
            ..item.data
        })
    }
}
#[derive(Debug, Serialize, Deserialize)]
pub struct ThreedsChallengeResponse {
    cres: String,
}

impl TryFrom<&RedsysRouterData<&PaymentsCompleteAuthorizeRouterData>> for RedsysTransaction {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &RedsysRouterData<&PaymentsCompleteAuthorizeRouterData>,
    ) -> Result<Self, Self::Error> {
        let card_data = RedsysCardData::try_from(&item.router_data.request.payment_method_data.clone())?;
        if item.router_data.auth_type == enums::AuthenticationType::ThreeDs {
        let auth = RedsysAuthType::try_from(&item.router_data.connector_auth_type)?;
        let redirect_response = item
            .router_data
            .request
            .get_redirect_response_payload()
            .ok()
            .clone()
            .map(|payload_data| {
                payload_data
                    .parse_value::<ThreedsChallengeResponse>("Redsys ThreedsChallengeResponse")
                    .change_context(errors::ConnectorError::ResponseDeserializationFailed)
            })
            .transpose()?;

        let emv3ds_data = match redirect_response {
            Some(payload) => {
                if let Ok(threeds_invoke_meta_data) = to_connector_meta::<RedsysThreeDsInvokeData>(
                    item.router_data.request.connector_meta.clone(),
                ) {
                    EmvThreedsData::new(RedsysThreeDsInfo::ChallengeResponse)
                        .set_protocol_version(threeds_invoke_meta_data.message_version)
                        .set_three_d_s_cres(payload.cres)
                } else if let Ok(threeds_meta_data) = to_connector_meta::<ThreeDsInvokeExempt>(
                    item.router_data.request.connector_meta.clone(),
                ) {
                    EmvThreedsData::new(RedsysThreeDsInfo::ChallengeResponse)
                        .set_protocol_version(threeds_meta_data.message_version)
                        .set_three_d_s_cres(payload.cres)
                } else {
                    Err(errors::ConnectorError::RequestEncodingFailed)?
                }
            }
            None => {
                if let Ok(threeds_invoke_meta_data) = to_connector_meta::<RedsysThreeDsInvokeData>(
                    item.router_data.request.connector_meta.clone(),
                ) {
                    let three_d_s_comp_ind = ThreeDSCompInd::from(
                        item.router_data.request.get_threeds_method_comp_ind()?,
                    );
                    let browser_info = item.router_data.request.get_browser_info()?;
                    let complete_authorize_url =
                        item.router_data.request.get_complete_authorize_url()?;
                    EmvThreedsData::new(RedsysThreeDsInfo::AuthenticationData)
                        .set_three_d_s_server_trans_i_d(
                            threeds_invoke_meta_data.directory_server_id,
                        )
                        .set_protocol_version(threeds_invoke_meta_data.message_version)
                        .set_three_d_s_comp_ind(three_d_s_comp_ind)
                        .add_browser_data(browser_info)?
                        .set_notification_u_r_l(complete_authorize_url)
                } else {
                    Err(errors::ConnectorError::NoConnectorMetaData)?
                }
            }
        };

        let ds_merchant_transactiontype = if item.router_data.request.is_auto_capture()? {
            RedsysTransactionType::Payment
        } else {
            RedsysTransactionType::Preauthorization
        };
        let ds_merchant_order = item.router_data.request.connector_transaction_id.clone().ok_or(errors::ConnectorError::RequestEncodingFailed).attach_printable("Missing connector_transaction_id")?;

        let complete_authorize_response = PaymentsRequest {
            ds_merchant_emv3ds: Some(emv3ds_data),
            ds_merchant_transactiontype,
            ds_merchant_currency: item.currency.iso_4217().to_owned(),
            ds_merchant_pan: card_data.card_number,
            ds_merchant_merchantcode: auth.merchant_id.clone(),
            ds_merchant_terminal: auth.terminal_id.clone(),
            ds_merchant_order,
            ds_merchant_amount: item.amount.clone(),
            ds_merchant_expirydate: card_data.expiry_date,
            ds_merchant_cvv2: card_data.cvv2,
        };
        Self::try_from((&complete_authorize_response, &auth))
    } else {
        Err(errors::ConnectorError::NotImplemented(
            get_unimplemented_payment_method_error_message("Redsys"),
        )
        .into())
    }
    }
}

impl<F> TryFrom<ResponseRouterData<F, RedsysResponse, CompleteAuthorizeData, PaymentsResponseData>>
    for RouterData<F, CompleteAuthorizeData, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, RedsysResponse, CompleteAuthorizeData, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        let (response, status) = match item.response.clone() {
            RedsysResponse::RedsysResponse(transaction_response) => {
                let response_data: RedsysPaymentsResponse = to_connector_response_data(
                    &transaction_response.ds_merchant_parameters.clone().expose(),
                )?;

                get_payments_response(
                    response_data,
                    item.data.request.capture_method,
                    None,
                    item.http_code,
                )?
            }
            RedsysResponse::RedsysErrorResponse(response) => {
                let response = Err(ErrorResponse {
                    code: response.error_code.clone(),
                    message: response.error_code.clone(),
                    reason: Some(response.error_code.clone()),
                    status_code: item.http_code,
                    attempt_status: None,
                    connector_transaction_id: None,
                });

                (response, enums::AttemptStatus::Failure)
            }
        };
        Ok(Self {
            status,
            response,
            ..item.data
        })
    }
}

impl From<api_models::payments::ThreeDsCompletionIndicator> for ThreeDSCompInd {
    fn from(threeds_compl_flag: api_models::payments::ThreeDsCompletionIndicator) -> Self {
        match threeds_compl_flag {
            api_models::payments::ThreeDsCompletionIndicator::Success => Self::Y,
            api_models::payments::ThreeDsCompletionIndicator::Failure
            | api_models::payments::ThreeDsCompletionIndicator::NotAvailable => Self::N,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub struct RedsysOperationRequest {
    ds_merchant_order: String,
    ds_merchant_merchantcode: Secret<String>,
    ds_merchant_terminal: Secret<String>,
    ds_merchant_currency: String,
    ds_merchant_transactiontype: RedsysTransactionType,
    ds_merchant_amount: StringMinorUnit,
}

impl TryFrom<&RedsysRouterData<&PaymentsCaptureRouterData>> for RedsysTransaction {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &RedsysRouterData<&PaymentsCaptureRouterData>) -> Result<Self, Self::Error> {
        let auth = RedsysAuthType::try_from(&item.router_data.connector_auth_type)?;
        let redys_capture_request = RedsysOperationRequest {
            ds_merchant_order: item.router_data.request.connector_transaction_id.clone(),
            ds_merchant_merchantcode: auth.merchant_id.clone(),
            ds_merchant_terminal: auth.terminal_id.clone(),
            ds_merchant_currency: item.router_data.request.currency.iso_4217().to_owned(),
            ds_merchant_transactiontype: RedsysTransactionType::Confirmation,
            ds_merchant_amount: item.amount.clone(),
        };
        Self::try_from((&redys_capture_request, &auth))
    }
}

impl<F> TryFrom<ResponseRouterData<F, RedsysResponse, PaymentsCaptureData, PaymentsResponseData>>
    for RouterData<F, PaymentsCaptureData, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, RedsysResponse, PaymentsCaptureData, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        let (response, status) = match item.response {
            RedsysResponse::RedsysResponse(redsys_transaction_response) => {
                let response_data: RedsysPaymentsResponse = to_connector_response_data(
                    &redsys_transaction_response
                        .ds_merchant_parameters
                        .clone()
                        .expose(),
                )?;

                let ds_response = response_data.ds_response.ok_or(
                    errors::ConnectorError::UnexpectedResponseError(bytes::Bytes::from(
                        "Redsys response missing ds_response",
                    )),
                )?;

                let status = map_redsys_attempt_status(ds_response.clone(), None)?;

                let response = if is_payment_failure(status) {
                    Err(ErrorResponse {
                        code: ds_response.0.clone(),
                        message: ds_response.0.clone(),
                        reason: Some(ds_response.0.clone()),
                        status_code: item.http_code,
                        attempt_status: None,
                        connector_transaction_id: Some(response_data.ds_order.clone()),
                    })
                } else {
                    Ok(PaymentsResponseData::TransactionResponse {
                        resource_id: ResponseId::ConnectorTransactionId(
                            response_data.ds_order.clone(),
                        ),
                        redirection_data: Box::new(None),
                        mandate_reference: Box::new(None),
                        connector_metadata: None,
                        network_txn_id: None,
                        connector_response_reference_id: Some(response_data.ds_order.clone()),
                        incremental_authorization_allowed: None,
                        charges: None,
                    })
                };
                (response, status)
            }
            RedsysResponse::RedsysErrorResponse(error_response) => {
                let response = Err(ErrorResponse {
                    code: error_response.error_code.clone(),
                    message: error_response.error_code.clone(),
                    reason: Some(error_response.error_code.clone()),
                    status_code: item.http_code,
                    attempt_status: None,
                    connector_transaction_id: None,
                });
                (response, enums::AttemptStatus::Failure)
            }
        };
        Ok(Self {
            status,
            response,
            ..item.data
        })
    }
}

impl TryFrom<&RedsysRouterData<&PaymentsCancelRouterData>> for RedsysTransaction {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &RedsysRouterData<&PaymentsCancelRouterData>) -> Result<Self, Self::Error> {
        let auth = RedsysAuthType::try_from(&item.router_data.connector_auth_type)?;
        let redsys_cancel_request = RedsysOperationRequest {
            ds_merchant_order: item.router_data.request.connector_transaction_id.clone(),
            ds_merchant_merchantcode: auth.merchant_id.clone(),
            ds_merchant_terminal: auth.terminal_id.clone(),
            ds_merchant_currency: item.currency.iso_4217().to_owned(),
            ds_merchant_transactiontype: RedsysTransactionType::Cancellation,
            ds_merchant_amount: item.amount.clone(),
        };
        Self::try_from((&redsys_cancel_request, &auth))
    }
}

impl<F> TryFrom<&RedsysRouterData<&RefundsRouterData<F>>> for RedsysTransaction {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &RedsysRouterData<&RefundsRouterData<F>>) -> Result<Self, Self::Error> {
        let auth = RedsysAuthType::try_from(&item.router_data.connector_auth_type)?;
        let redsys_refund_request = RedsysOperationRequest {
            ds_merchant_order: item.router_data.request.connector_transaction_id.clone(),
            ds_merchant_merchantcode: auth.merchant_id.clone(),
            ds_merchant_terminal: auth.terminal_id.clone(),
            ds_merchant_currency: item.currency.iso_4217().to_owned(),
            ds_merchant_transactiontype: RedsysTransactionType::Refund,
            ds_merchant_amount: item.amount.clone(),
        };
        Self::try_from((&redsys_refund_request, &auth))
    }
}

impl<F> TryFrom<ResponseRouterData<F, RedsysResponse, PaymentsCancelData, PaymentsResponseData>>
    for RouterData<F, PaymentsCancelData, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, RedsysResponse, PaymentsCancelData, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        let (response, status) = match item.response {
            RedsysResponse::RedsysResponse(redsys_transaction_response) => {
                let response_data: RedsysPaymentsResponse = to_connector_response_data(
                    &redsys_transaction_response
                        .ds_merchant_parameters
                        .clone()
                        .expose(),
                )?;

                let ds_response = response_data.ds_response.ok_or(
                    errors::ConnectorError::UnexpectedResponseError(bytes::Bytes::from(
                        "Redsys response missing ds_response",
                    )),
                )?;

                let status = map_redsys_attempt_status(ds_response.clone(), None)?;

                let response = if is_payment_failure(status) {
                    Err(ErrorResponse {
                        code: ds_response.0.clone(),
                        message: ds_response.0.clone(),
                        reason: Some(ds_response.0.clone()),
                        status_code: item.http_code,
                        attempt_status: None,
                        connector_transaction_id: Some(response_data.ds_order.clone()),
                    })
                } else {
                    Ok(PaymentsResponseData::TransactionResponse {
                        resource_id: ResponseId::ConnectorTransactionId(
                            response_data.ds_order.clone(),
                        ),
                        redirection_data: Box::new(None),
                        mandate_reference: Box::new(None),
                        connector_metadata: None,
                        network_txn_id: None,
                        connector_response_reference_id: Some(response_data.ds_order.clone()),
                        incremental_authorization_allowed: None,
                        charges: None,
                    })
                };
                (response, status)
            }
            RedsysResponse::RedsysErrorResponse(error_response) => {
                let response = Err(ErrorResponse {
                    code: error_response.error_code.clone(),
                    message: error_response.error_code.clone(),
                    reason: Some(error_response.error_code.clone()),
                    status_code: item.http_code,
                    attempt_status: None,
                    connector_transaction_id: None,
                });

                (response, enums::AttemptStatus::Failure)
            }
        };
        Ok(Self {
            status,
            response,
            ..item.data
        })
    }
}

impl TryFrom<DsResponse> for enums::RefundStatus {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(ds_response: DsResponse) -> Result<Self, Self::Error> {
        match ds_response.0.as_str() {
            "0900" => Ok(Self::Success),
            "9998" | "9999" => Ok(Self::Pending),
            "950" => Ok(Self::Failure),
            error => Err(errors::ConnectorError::ResponseHandlingFailed)
                .attach_printable(format!("Recieved Unknown Status:{}", error))?,
        }
    }
}

impl TryFrom<RefundsResponseRouterData<Execute, RedsysResponse>> for RefundsRouterData<Execute> {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: RefundsResponseRouterData<Execute, RedsysResponse>,
    ) -> Result<Self, Self::Error> {
        let response = match item.response {
            RedsysResponse::RedsysResponse(redsys_transaction) => {
                let response_data: RedsysPaymentsResponse = to_connector_response_data(
                    &redsys_transaction.ds_merchant_parameters.clone().expose(),
                )?;

                let ds_response = response_data.ds_response.ok_or(
                    errors::ConnectorError::UnexpectedResponseError(bytes::Bytes::from(
                        "Redsys response missing ds_response",
                    )),
                )?;
                let refund_status = enums::RefundStatus::try_from(ds_response.clone())?;

                if is_refund_failure(refund_status) {
                    Err(ErrorResponse {
                        code: ds_response.0.clone(),
                        message: ds_response.0.clone(),
                        reason: Some(ds_response.0.clone()),
                        status_code: item.http_code,
                        attempt_status: None,
                        connector_transaction_id: None,
                    })
                } else {
                    Ok(RefundsResponseData {
                        connector_refund_id: response_data.ds_order, //Same as connector_transaction_id, as Redsys does not provide a refund id
                        refund_status: enums::RefundStatus::try_from(ds_response)?,
                    })
                }
            }
            RedsysResponse::RedsysErrorResponse(redsys_error_response) => Err(ErrorResponse {
                code: redsys_error_response.error_code.clone(),
                message: redsys_error_response.error_code.clone(),
                reason: Some(redsys_error_response.error_code.clone()),
                status_code: item.http_code,
                attempt_status: None,
                connector_transaction_id: None,
            }),
        };

        Ok(Self {
            response,
            ..item.data
        })
    }
}

fn get_payments_response(
    redsys_payments_response: RedsysPaymentsResponse,
    capture_method: Option<enums::CaptureMethod>,
    connector_metadata: Option<josekit::Value>,
    http_code: u16,
) -> Result<
    (
        Result<PaymentsResponseData, ErrorResponse>,
        enums::AttemptStatus,
    ),
    Error,
> {
    if let Some(ds_response) = redsys_payments_response.ds_response {
        let status = map_redsys_attempt_status(ds_response.clone(), capture_method)?;
        let response = if is_payment_failure(status) {
            Err(ErrorResponse {
                code: ds_response.0.clone(),
                message: ds_response.0.clone(),
                reason: Some(ds_response.0.clone()),
                status_code: http_code,
                attempt_status: None,
                connector_transaction_id: Some(redsys_payments_response.ds_order.clone()),
            })
        } else {
            Ok(PaymentsResponseData::TransactionResponse {
                resource_id: ResponseId::ConnectorTransactionId(
                    redsys_payments_response.ds_order.clone(),
                ),
                redirection_data: Box::new(None),
                mandate_reference: Box::new(None),
                connector_metadata,
                network_txn_id: None,
                connector_response_reference_id: None,
                incremental_authorization_allowed: None,
                charges: None,
            })
        };

        Ok((response, status))
    } else {
        let redirection_form = redsys_payments_response
            .ds_emv3ds
            .map(|ds_emv3ds| build_threeds_form(&ds_emv3ds))
            .transpose()?;
        let response = Ok(PaymentsResponseData::TransactionResponse {
            resource_id: ResponseId::ConnectorTransactionId(
                redsys_payments_response.ds_order.clone(),
            ),
            redirection_data: Box::new(redirection_form),
            mandate_reference: Box::new(None),
            connector_metadata,
            network_txn_id: None,
            connector_response_reference_id: None,
            incremental_authorization_allowed: None,
            charges: None,
        });

        Ok((response, enums::AttemptStatus::AuthenticationPending))
    }
}
