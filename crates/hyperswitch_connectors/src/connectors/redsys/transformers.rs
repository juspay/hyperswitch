use crate::{
    types::{RefundsResponseRouterData, ResponseRouterData},
    utils::{
        get_unimplemented_payment_method_error_message, to_connector_meta,
        to_connector_meta_from_secret, BrowserInformationData, CardData, ForeignTryFrom,
        PaymentsAuthorizeRequestData, PaymentsCompleteAuthorizeRequestData,
        PaymentsPreProcessingRequestData,
    },
};
use base64::{decode, encode};
use common_enums::enums;
use common_utils::{ext_traits::ValueExt, types::StringMinorUnit};
use error_stack::ResultExt;
use hmac::{Hmac, Mac, NewMac};
use hyperswitch_domain_models::{
    payment_method_data::PaymentMethodData,
    router_data::{ConnectorAuthType, ErrorResponse, RouterData},
    router_flow_types::refunds::{Execute, RSync},
    router_request_types::{
        CompleteAuthorizeData, PaymentsAuthorizeData, PaymentsCancelData, PaymentsCaptureData,
        PaymentsPreProcessingData, ResponseId,
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
use router_env::logger;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use sha2::Sha256;

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

#[derive(Debug)]
pub struct RedsysCardData {
    card_number: cards::CardNumber,
    expiry_date: Secret<String>,
    cvv2: Secret<String>,
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
    type Error = error_stack::Report<errors::ConnectorError>;
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

fn get_redsys_order_id(
    connector_request_reference_id: String,
) -> Result<String, error_stack::Report<errors::ConnectorError>> {
    let removed_special_chars = connector_request_reference_id.replace("_", "");
    if removed_special_chars.len() < 4 {
        return Err(errors::ConnectorError::InvalidDataFormat {
            field_name: "connector_request_reference_id",
        }
        .into());
    };

    if removed_special_chars.len() > 12 {
        Ok(removed_special_chars[removed_special_chars.len().saturating_sub(12)..].to_string())
    } else {
        Ok(removed_special_chars)
    }
}

impl TryFrom<&Option<PaymentMethodData>> for RedsysCardData {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(payment_method_data: &Option<PaymentMethodData>) -> Result<Self, Self::Error> {
        match payment_method_data {
            Some(PaymentMethodData::Card(card)) => {
                let year =  card.get_card_expiry_year_2_digit()?.expose();
                let month = 
                let yymm_expiry_date = card.get_expiry_date_as_yymm()?.expose();
                let expiry_date = Secret::new("2512".to_owned());
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

fn get_redsys_expiry_data(date: &str) -> Result<String, &'static str> {
    if date.len() != 4 {
        return Err("Invalid date format");
    }

    let month = &date[0..2];
    let year = &date[2..4];

    Ok(format!("{}{}", year, month))
}

impl
    TryFrom<(
        &RedsysRouterData<&PaymentsPreProcessingRouterData>,
        &RedsysAuthType,
    )> for PaymentsRequest
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        data: (
            &RedsysRouterData<&PaymentsPreProcessingRouterData>,
            &RedsysAuthType,
        ),
    ) -> Result<Self, Self::Error> {
        let (item, auth) = data;
        let browser_info = item.router_data.request.get_optional_browser_info();
        let ds_merchant_emv3ds =
            if item.router_data.auth_type == common_enums::enums::AuthenticationType::ThreeDs {
                Ok(Some(EmvThreedsData {
                    three_d_s_info: RedsysThreeDsInfo::CardData,
                    protocol_version: None,
                    browser_accept_header: browser_info
                        .clone()
                        .and_then(|browser_data| browser_data.get_accept_header().ok()),
                    browser_user_agent: browser_info
                        .clone()
                        .and_then(|browser_data| browser_data.get_user_agent().ok()),
                    browser_java_enabled: browser_info
                        .clone()
                        .and_then(|browser_data| browser_data.get_java_enabled().ok()),
                    browser_java_script_enabled: browser_info
                        .clone()
                        .and_then(|browser_data| browser_data.get_java_script_enabled().ok()),
                    browser_language: browser_info
                        .clone()
                        .and_then(|browser_data| browser_data.get_language().ok()),
                    browser_color_depth: browser_info
                        .clone()
                        .and_then(|browser_data| browser_data.get_color_depth().ok())
                        .map(|depth| depth.to_string()),
                    browser_screen_height: browser_info
                        .clone()
                        .and_then(|browser_data| browser_data.get_screen_height().ok())
                        .map(|height| height.to_string()),
                    browser_screen_width: browser_info
                        .clone()
                        .and_then(|browser_data| browser_data.get_screen_width().ok())
                        .map(|width| width.to_string()),
                    browser_t_z: browser_info
                        .clone()
                        .and_then(|browser_data| browser_data.get_time_zone().ok())
                        .map(|tz| tz.to_string()),
                    three_d_s_server_trans_i_d: None,
                    notification_u_r_l: None,
                    three_d_s_comp_ind: None,
                    cres: None,
                }))
            } else {
                Err(errors::ConnectorError::FlowNotSupported {
                    flow: "PreProcessing".to_string(),
                    connector: "Redsys".to_string(),
                })
            }?;

        let ds_merchant_transactiontype = if item.router_data.request.is_auto_capture()? {
            RedsysTransactionType::Payment
        } else {
            RedsysTransactionType::Preauthorization
        };

        let ds_merchant_order =
            get_redsys_order_id(item.router_data.connector_request_reference_id.clone())?;
        let card_data = RedsysCardData::try_from(&item.router_data.request.payment_method_data)?;
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

#[derive(Debug, Serialize, Deserialize)]
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
pub struct ThreeDsDataForDDC {
    pub three_ds_method_url: String,
    pub three_ds_method_data: String,
    pub message_version: String,
    pub directory_server_id: String,
    pub three_ds_method_data_submission: bool,
    pub next_action_url: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ThreeDsNoDDCData {
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
                    get_merchant_parameters(&response.ds_merchant_parameters.clone().expose())?;
                router_env::logger::info!(sssssssssss=?response_data);
                handle_redsys_response(item, &response_data)
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

                Ok(RouterData {
                    status: common_enums::enums::AttemptStatus::Failure,
                    response,
                    ..item.data
                })
            }
        }
    }
}

fn handle_redsys_response<F>(
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
        ) => handle_ddc_case(
            item,
            response_data,
            three_d_s_method_u_r_l,
            three_d_s_server_trans_i_d,
            protocol_version,
        ),
        (None, Some(three_d_s_server_trans_i_d), Some(protocol_version)) => handle_no_ddc_case(
            item,
            response_data,
            three_d_s_server_trans_i_d,
            protocol_version,
        ),
        _ => handle_non_3ds_case(item, response_data),
    }
}

fn handle_ddc_case<F>(
    item: ResponseRouterData<F, RedsysResponse, PaymentsPreProcessingData, PaymentsResponseData>,
    response_data: &RedsysPaymentsResponse,
    three_d_s_method_u_r_l: String,
    three_d_s_server_trans_i_d: String,
    protocol_version: String,
) -> Result<
    RouterData<F, PaymentsPreProcessingData, PaymentsResponseData>,
    error_stack::Report<errors::ConnectorError>,
> {
    router_env::logger::info!(sssssssssssddd=?protocol_version);
    let three_d_s_method_notification_u_r_l = item.data.request.get_webhook_url()?;

    let threeds_invoke_data = ThreedsInvokeRequest {
        three_d_s_server_trans_i_d: three_d_s_method_u_r_l.clone(),
        three_d_s_method_notification_u_r_l,
    };

    let three_ds_data_string = serde_json::to_string(&threeds_invoke_data)
        .change_context(errors::ConnectorError::RequestEncodingFailed)?;

    let three_ds_method_data = encode(&three_ds_data_string);
    let next_action_url = item.data.request.get_complete_authorize_url()?;

    let three_ds_data = ThreeDsDataForDDC {
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
        status: common_enums::enums::AttemptStatus::AuthenticationPending,
        response: Ok(PaymentsResponseData::TransactionResponse {
            resource_id: ResponseId::ConnectorTransactionId(response_data.ds_order.clone()),
            redirection_data: Box::new(None),
            mandate_reference: Box::new(None),
            connector_metadata,
            network_txn_id: None,
            connector_response_reference_id: Some(response_data.ds_order.clone()),
            incremental_authorization_allowed: None,
            charges: None,
        }),
        ..item.data
    })
}

fn handle_no_ddc_case<F>(
    item: ResponseRouterData<F, RedsysResponse, PaymentsPreProcessingData, PaymentsResponseData>,
    response_data: &RedsysPaymentsResponse,
    three_d_s_server_trans_i_d: String,
    protocol_version: String,
) -> Result<
    RouterData<F, PaymentsPreProcessingData, PaymentsResponseData>,
    error_stack::Report<errors::ConnectorError>,
> {
    let three_ds_data = ThreeDsNoDDCData {
        message_version: protocol_version.clone(),
        three_d_s_server_trans_i_d,
    };

    let connector_metadata = Some(
        serde_json::to_value(&three_ds_data)
            .change_context(errors::ConnectorError::RequestEncodingFailed)
            .attach_printable("Failed to serialize ThreeDsData")?,
    );

    Ok(RouterData {
        status: common_enums::enums::AttemptStatus::AuthenticationPending,
        response: Ok(PaymentsResponseData::TransactionResponse {
            resource_id: ResponseId::ConnectorTransactionId(response_data.ds_order.clone()),
            redirection_data: Box::new(None),
            mandate_reference: Box::new(None),
            connector_metadata,
            network_txn_id: None,
            connector_response_reference_id: Some(response_data.ds_order.clone()),
            incremental_authorization_allowed: None,
            charges: None,
        }),
        ..item.data
    })
}

fn handle_non_3ds_case<F>(
    item: ResponseRouterData<F, RedsysResponse, PaymentsPreProcessingData, PaymentsResponseData>,
    response_data: &RedsysPaymentsResponse,
) -> Result<
    RouterData<F, PaymentsPreProcessingData, PaymentsResponseData>,
    error_stack::Report<errors::ConnectorError>,
> {
    Ok(RouterData {
        status: common_enums::enums::AttemptStatus::AuthenticationPending,
        response: Ok(PaymentsResponseData::TransactionResponse {
            resource_id: ResponseId::ConnectorTransactionId(response_data.ds_order.clone()),
            redirection_data: Box::new(None),
            mandate_reference: Box::new(None),
            connector_metadata: None,
            network_txn_id: None,
            connector_response_reference_id: Some(response_data.ds_order.clone()),
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

fn get_merchant_parameters<T: DeserializeOwned>(
    encoded_str: &str,
) -> Result<T, error_stack::Report<errors::ConnectorError>> {
    let decoded_bytes = decode(encoded_str)
        .change_context(errors::ConnectorError::ResponseDeserializationFailed)
        .attach_printable("Failed to decode Base64")?;

    let response_data: T = serde_json::from_slice(&decoded_bytes)
        .change_context(errors::ConnectorError::ResponseHandlingFailed)?;

    Ok(response_data)
}

fn base64_decode(input: &str) -> Result<Vec<u8>, error_stack::Report<errors::ConnectorError>> {
    decode(input)
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
    let encrypted_trimmed = &encrypted[..expected_len];
    let encoded = encode(&encrypted_trimmed);
    Ok(encoded)
}

fn get_signature(
    order_id: &str,
    params: &str,
    clave: &str,
) -> Result<String, error_stack::Report<errors::ConnectorError>> {
    let secret_ko = des_encrypt(order_id, clave)?;
    logger::debug!("dssssssss_secret_ko: {:?}", secret_ko);
    let base_decoded = base64_decode(&secret_ko)?;

    // HMAC-SHA256
    let mut mac = Hmac::<Sha256>::new_from_slice(&base_decoded)
        .map_err(|_| errors::ConnectorError::RequestEncodingFailed)
        .attach_printable("HMAC-SHA256 initialization failed")?;
    mac.update(params.as_bytes());
    let result = mac.finalize().into_bytes();
    let encoded = encode(&result);
    Ok(encoded)
}

impl TryFrom<(&PaymentsRequest, &RedsysAuthType)> for RedsysTransaction {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(data: (&PaymentsRequest, &RedsysAuthType)) -> Result<Self, Self::Error> {
        let (request_data, auth) = data;
        let merchant_parameters = serde_json::to_string(&request_data)
            .change_context(errors::ConnectorError::RequestEncodingFailed)
            .attach_printable("Failed Serialization of PaymentsRequest struct")?;
        logger::debug!("sssssss_merchant_parameters: {:?}", merchant_parameters);
        let ds_merchant_parameters = encode(&merchant_parameters);
        logger::debug!("dsssssss_merchant_parameters: {:?}", ds_merchant_parameters);
        let sha256_pwd = auth.sha256_pwd.clone().expose();
        let signature = get_signature(
            &request_data.ds_merchant_order,
            &ds_merchant_parameters,
            &sha256_pwd,
        )?;
        logger::debug!("dssssssss_signature: {:?}", signature);
        Ok(Self {
            ds_signature_version: SIGNATURE_VERSION.to_string(),
            ds_merchant_parameters: Secret::new(ds_merchant_parameters),
            ds_signature: Secret::new(signature),
        })
    }
}

impl ForeignTryFrom<(DsResponse, Option<common_enums::enums::CaptureMethod>)>
    for common_enums::enums::AttemptStatus
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn foreign_try_from(
        (ds_response, capture_method): (DsResponse, Option<common_enums::enums::CaptureMethod>),
    ) -> Result<Self, Self::Error> {
        if ds_response.0.starts_with("00") {
            match capture_method {
                Some(common_enums::enums::CaptureMethod::Automatic) => {
                    Ok(common_enums::enums::AttemptStatus::Charged)
                }
                Some(common_enums::enums::CaptureMethod::Manual) => {
                    Ok(common_enums::enums::AttemptStatus::Authorized)
                }
                None => Err(errors::ConnectorError::ResponseHandlingFailed)
                    .attach_printable(format!("Recieved Unknown Status"))?,
                _ => Err(errors::ConnectorError::CaptureMethodNotSupported.into()),
            }
        } else {
            match ds_response.0.as_str() {
                "0900" => Ok(common_enums::enums::AttemptStatus::Charged),
                "400" => Ok(common_enums::enums::AttemptStatus::Voided),
                "950" => Ok(common_enums::enums::AttemptStatus::VoidFailed),
                "9998" | "9999" => Ok(common_enums::enums::AttemptStatus::Pending),
                "9256" | "9257" => Ok(common_enums::enums::AttemptStatus::AuthenticationFailed),
                "101" | "102" | "106" | "125" | "129" | "172" | "173" | "174" | "180" | "184"
                | "190" | "191" | "195" | "202" | "904" | "909" | "913" | "944" | "9912"
                | "912" | "9064" | "9078" | "9093" | "9094" | "9104" | "9218" | "9253" | "9261"
                | "9915" | "9997" => Ok(common_enums::enums::AttemptStatus::Failure),
                error => Err(errors::ConnectorError::ResponseHandlingFailed)
                    .attach_printable(format!("Recieved Unknown Status:{}", error))?,
            }
        }
    }
}

impl
    TryFrom<(
        &RedsysRouterData<&PaymentsAuthorizeRouterData>,
        &RedsysAuthType,
    )> for PaymentsRequest
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        (item, auth): (
            &RedsysRouterData<&PaymentsAuthorizeRouterData>,
            &RedsysAuthType,
        ),
    ) -> Result<Self, Self::Error> {
        let connector_meatadata = match &item.router_data.response {
            Ok(PaymentsResponseData::TransactionResponse {
                connector_metadata, ..
            }) => connector_metadata.clone(),
            _ => None,
        };

        let threeds_invoke_meta_data: Option<ThreeDsDataForDDC> =
            to_connector_meta(connector_meatadata.clone()).ok();
        let threeds_meta_data: Option<ThreeDsNoDDCData> =
            to_connector_meta(connector_meatadata.clone()).ok();

        let three_d_s_server_trans_i_d = threeds_invoke_meta_data
            .as_ref()
            .map(|meta_data| meta_data.directory_server_id.clone())
            .or_else(|| {
                threeds_meta_data
                    .as_ref()
                    .map(|meta_data| meta_data.three_d_s_server_trans_i_d.clone())
            });

        let protocol_version = threeds_invoke_meta_data
            .as_ref()
            .map(|meta_data| meta_data.message_version.clone())
            .or_else(|| {
                threeds_meta_data
                    .as_ref()
                    .map(|meta_data| meta_data.message_version.clone())
            });
        router_env::logger::info!(sssssssssssddd2=?protocol_version);

        let browser_info = item.router_data.request.get_browser_info().ok();
        let ds_merchant_emv3ds = if item.router_data.auth_type
            == common_enums::enums::AuthenticationType::ThreeDs
        {
            Some(EmvThreedsData {
                three_d_s_info: RedsysThreeDsInfo::AuthenticationData,
                protocol_version,
                browser_accept_header: browser_info
                    .clone()
                    .and_then(|browser_data| browser_data.get_accept_header().ok()),
                browser_user_agent: browser_info
                    .clone()
                    .and_then(|browser_data| browser_data.get_user_agent().ok()),
                browser_java_enabled: browser_info
                    .clone()
                    .and_then(|browser_data| browser_data.get_java_enabled().ok()),
                browser_java_script_enabled: browser_info
                    .clone()
                    .and_then(|browser_data| browser_data.get_java_script_enabled().ok()),
                browser_language: browser_info
                    .clone()
                    .and_then(|browser_data| browser_data.get_language().ok()),
                browser_color_depth: browser_info
                    .clone()
                    .and_then(|browser_data| browser_data.get_color_depth().ok())
                    .map(|depth| depth.to_string()),
                browser_screen_height: browser_info
                    .clone()
                    .and_then(|browser_data| browser_data.get_screen_height().ok())
                    .map(|height| height.to_string()),
                browser_screen_width: browser_info
                    .clone()
                    .and_then(|browser_data| browser_data.get_screen_width().ok())
                    .map(|width| width.to_string()),
                browser_t_z: browser_info
                    .clone()
                    .and_then(|browser_data| browser_data.get_time_zone().ok())
                    .map(|tz| tz.to_string()),
                three_d_s_server_trans_i_d,
                notification_u_r_l: Some(item.router_data.request.get_complete_authorize_url()?),
                three_d_s_comp_ind: Some(ThreeDSCompInd::N),
                cres: None,
            })
        } else {
            None
        };
        let ds_merchant_transactiontype = if item.router_data.request.is_auto_capture()? {
            RedsysTransactionType::Payment
        } else {
            RedsysTransactionType::Preauthorization
        };
        let ds_merchant_order =
            get_redsys_order_id(item.router_data.connector_request_reference_id.clone())?;
        let card_data =
            RedsysCardData::try_from(&Some(item.router_data.request.payment_method_data.clone()))?;
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
    }
}

impl<F> TryFrom<ResponseRouterData<F, RedsysResponse, PaymentsAuthorizeData, PaymentsResponseData>>
    for RouterData<F, PaymentsAuthorizeData, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, RedsysResponse, PaymentsAuthorizeData, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        match item.response.clone() {
            RedsysResponse::RedsysResponse(transaction_response) => {
                let connector_metadata = match item.data.response {
                    Ok(PaymentsResponseData::TransactionResponse {
                        connector_metadata, ..
                    }) => connector_metadata,
                    _ => None,
                };

                let response_data: RedsysPaymentsResponse = get_merchant_parameters(
                    &transaction_response.ds_merchant_parameters.clone().expose(),
                )?;

                router_env::logger::info!(sssssssssss=?response_data);

                if let Some(ds_response) = response_data.ds_response {
                    let status = common_enums::enums::AttemptStatus::foreign_try_from((
                        ds_response,
                        item.data.request.capture_method.clone(),
                    ))?;
                    let response = Ok(PaymentsResponseData::TransactionResponse {
                        resource_id: ResponseId::ConnectorTransactionId(
                            response_data.ds_order.clone(),
                        ),
                        redirection_data: Box::new(None),
                        mandate_reference: Box::new(None),
                        connector_metadata,
                        network_txn_id: None,
                        connector_response_reference_id: Some(response_data.ds_order.clone()),
                        incremental_authorization_allowed: None,
                        charges: None,
                    });

                    Ok(RouterData {
                        status,
                        response,
                        ..item.data
                    })
                } else {
                    let mut form_fields = std::collections::HashMap::new();
                    let creq = response_data
                        .ds_emv3ds
                        .as_ref()
                        .and_then(|emv3ds_data| emv3ds_data.creq.clone())
                        .ok_or(errors::ConnectorError::ResponseDeserializationFailed)?;
                    let endpoint = response_data
                        .ds_emv3ds
                        .as_ref()
                        .and_then(|emv3ds_data| emv3ds_data.acs_u_r_l.clone())
                        .ok_or(errors::ConnectorError::ResponseDeserializationFailed)?;
                    form_fields.insert(String::from("creq"), creq);

                    let response = Ok(PaymentsResponseData::TransactionResponse {
                        resource_id: ResponseId::ConnectorTransactionId(
                            response_data.ds_order.clone(),
                        ),
                        redirection_data: Box::new(Some(RedirectForm::Form {
                            endpoint,
                            method: common_utils::request::Method::Post,
                            form_fields,
                        })),
                        mandate_reference: Box::new(None),
                        connector_metadata,
                        network_txn_id: None,
                        connector_response_reference_id: Some(response_data.ds_order.clone()),
                        incremental_authorization_allowed: None,
                        charges: None,
                    });

                    Ok(RouterData {
                        status: common_enums::enums::AttemptStatus::AuthenticationPending,
                        response,
                        ..item.data
                    })
                }
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

                Ok(RouterData {
                    status: common_enums::enums::AttemptStatus::Failure,
                    response,
                    ..item.data
                })
            }
        }
    }
}
#[derive(Debug, Serialize, Deserialize)]
pub struct ThreedsChallengeResponse {
    cres: String,
}

impl
    TryFrom<(
        &RedsysRouterData<&PaymentsCompleteAuthorizeRouterData>,
        &RedsysAuthType,
    )> for PaymentsRequest
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        (item, auth): (
            &RedsysRouterData<&PaymentsCompleteAuthorizeRouterData>,
            &RedsysAuthType,
        ),
    ) -> Result<Self, Self::Error> {
        let payload = item
            .router_data
            .request
            .get_redirect_response_payload()
            .ok();
        router_env::logger::info!(sssssssssssddd=?item.router_data
        .request.redirect_response.as_ref().map(|a| a.params.clone().map(|b|b.clone().expose())));
        router_env::logger::info!(sssssssssssddd=?item.router_data
        .request.redirect_response.as_ref().map(|a| a.payload.clone().map(|e| e.clone().expose())));
        let redirect_response: Option<ThreedsChallengeResponse> =
            payload.clone().and_then(|payload_data| {
                payload_data
                    .parse_value("Redsys ThreedsChallengeResponse")
                    .ok()
            });

        match redirect_response {
            Some(payload) => {
                let threeds_invoke_meta_data: Option<ThreeDsDataForDDC> =
                    to_connector_meta(item.router_data.request.connector_meta.clone()).ok();
                let threeds_meta_data: Option<ThreeDsNoDDCData> =
                    to_connector_meta(item.router_data.request.connector_meta.clone()).ok();
                router_env::logger::info!(sssssssssssddd5=?threeds_invoke_meta_data);
                router_env::logger::info!(sssssssssssddd5=?threeds_meta_data);
                let three_d_s_server_trans_i_d = threeds_invoke_meta_data
                    .as_ref()
                    .map(|meta_data| meta_data.directory_server_id.clone())
                    .or_else(|| {
                        threeds_meta_data
                            .as_ref()
                            .map(|meta_data| meta_data.three_d_s_server_trans_i_d.clone())
                    });

                let protocol_version = threeds_invoke_meta_data
                    .as_ref()
                    .map(|meta_data| meta_data.message_version.clone())
                    .or_else(|| {
                        threeds_meta_data
                            .as_ref()
                            .map(|meta_data| meta_data.message_version.clone())
                    });

                router_env::logger::info!(sssssssssssddd3=?protocol_version);

                let ds_merchant_emv3ds = if item.router_data.auth_type
                    == common_enums::enums::AuthenticationType::ThreeDs
                {
                    Some(EmvThreedsData {
                        three_d_s_info: RedsysThreeDsInfo::ChallengeResponse,
                        protocol_version,
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
                        cres: Some(payload.cres),
                    })
                } else {
                    None
                };

                let ds_merchant_transactiontype = if item.router_data.request.is_auto_capture()? {
                    RedsysTransactionType::Payment
                } else {
                    RedsysTransactionType::Preauthorization
                };

                let ds_merchant_order =
                    get_redsys_order_id(item.router_data.connector_request_reference_id.clone())?;
                let card_data = RedsysCardData::try_from(
                    &item.router_data.request.payment_method_data.clone(),
                )?;
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
            }
            None => {
                let threeds_invoke_meta_data: Option<ThreeDsDataForDDC> =
                    to_connector_meta(item.router_data.request.connector_meta.clone()).ok();

                let three_d_s_server_trans_i_d = threeds_invoke_meta_data
                    .as_ref()
                    .map(|meta_data| meta_data.directory_server_id.clone());

                let protocol_version = threeds_invoke_meta_data
                    .as_ref()
                    .map(|meta_data| meta_data.message_version.clone());

                router_env::logger::info!(sssssssssssddd4=?protocol_version);
                let three_d_s_comp_ind =
                    ThreeDSCompInd::from(item.router_data.request.get_threeds_method_comp_ind()?);

                let browser_info = item.router_data.request.get_browser_info().ok();
                router_env::logger::info!(sssssssssssddd8=?browser_info);
                let ds_merchant_emv3ds = if item.router_data.auth_type
                    == common_enums::enums::AuthenticationType::ThreeDs
                {
                    Some(EmvThreedsData {
                        three_d_s_info: RedsysThreeDsInfo::AuthenticationData,
                        protocol_version,
                        browser_accept_header: browser_info
                            .clone()
                            .and_then(|browser_data| browser_data.get_accept_header().ok()),
                        browser_user_agent: browser_info
                            .clone()
                            .and_then(|browser_data| browser_data.get_user_agent().ok()),
                        browser_java_enabled: browser_info
                            .clone()
                            .and_then(|browser_data| browser_data.get_java_enabled().ok()),
                        browser_java_script_enabled: browser_info
                            .clone()
                            .and_then(|browser_data| browser_data.get_java_script_enabled().ok()),
                        browser_language: browser_info
                            .clone()
                            .and_then(|browser_data| browser_data.get_language().ok()),
                        browser_color_depth: browser_info
                            .clone()
                            .and_then(|browser_data| browser_data.get_color_depth().ok())
                            .map(|depth| depth.to_string()),
                        browser_screen_height: browser_info
                            .clone()
                            .and_then(|browser_data| browser_data.get_screen_height().ok())
                            .map(|height| height.to_string()),
                        browser_screen_width: browser_info
                            .clone()
                            .and_then(|browser_data| browser_data.get_screen_width().ok())
                            .map(|width| width.to_string()),
                        browser_t_z: browser_info
                            .clone()
                            .and_then(|browser_data| browser_data.get_time_zone().ok())
                            .map(|tz| tz.to_string()),
                        three_d_s_server_trans_i_d,
                        notification_u_r_l: Some(
                            item.router_data.request.get_complete_authorize_url()?,
                        ),
                        three_d_s_comp_ind: Some(three_d_s_comp_ind),
                        cres: None,
                    })
                } else {
                    None
                };

                let ds_merchant_transactiontype = if item.router_data.request.is_auto_capture()? {
                    RedsysTransactionType::Payment
                } else {
                    RedsysTransactionType::Preauthorization
                };

                let ds_merchant_order =
                    get_redsys_order_id(item.router_data.connector_request_reference_id.clone())?;
                let card_data = RedsysCardData::try_from(
                    &item.router_data.request.payment_method_data.clone(),
                )?;
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
            }
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
        match item.response.clone() {
            RedsysResponse::RedsysResponse(transaction_response) => {
                let response_data: RedsysPaymentsResponse = get_merchant_parameters(
                    &transaction_response.ds_merchant_parameters.clone().expose(),
                )?;

                router_env::logger::info!(sssssssssss=?response_data);

                if let Some(ds_response) = response_data.ds_response {
                    let status = common_enums::enums::AttemptStatus::foreign_try_from((
                        ds_response,
                        item.data.request.capture_method,
                    ))?;

                    let response = Ok(PaymentsResponseData::TransactionResponse {
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
                    });

                    Ok(RouterData {
                        status,
                        response,
                        ..item.data
                    })
                } else {
                    let mut form_fields = std::collections::HashMap::new();
                    let creq = response_data
                        .ds_emv3ds
                        .as_ref()
                        .and_then(|emv3ds_data| emv3ds_data.creq.clone())
                        .ok_or(errors::ConnectorError::ResponseDeserializationFailed)?;
                    let endpoint = response_data
                        .ds_emv3ds
                        .as_ref()
                        .and_then(|emv3ds_data| emv3ds_data.acs_u_r_l.clone())
                        .ok_or(errors::ConnectorError::ResponseDeserializationFailed)?;
                    form_fields.insert(String::from("creq"), creq);
                    let connector_metadata = item
                        .data
                        .connector_meta_data
                        .clone()
                        .map(|meta_data| meta_data.expose());

                    let response = Ok(PaymentsResponseData::TransactionResponse {
                        resource_id: ResponseId::ConnectorTransactionId(
                            response_data.ds_order.clone(),
                        ),
                        redirection_data: Box::new(Some(RedirectForm::Form {
                            endpoint,
                            method: common_utils::request::Method::Post,
                            form_fields,
                        })),
                        mandate_reference: Box::new(None),
                        connector_metadata: None,
                        network_txn_id: None,
                        connector_response_reference_id: Some(response_data.ds_order.clone()),
                        incremental_authorization_allowed: None,
                        charges: None,
                    });

                    Ok(RouterData {
                        status: common_enums::enums::AttemptStatus::AuthenticationPending,
                        response,
                        ..item.data
                    })
                }
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

                Ok(RouterData {
                    status: common_enums::enums::AttemptStatus::Failure,
                    response,
                    ..item.data
                })
            }
        }
    }
}

impl From<api_models::payments::ThreeDsCompletionIndicator> for ThreeDSCompInd {
    fn from(threeds_compl_flag: api_models::payments::ThreeDsCompletionIndicator) -> Self {
        match threeds_compl_flag {
            api_models::payments::ThreeDsCompletionIndicator::Success => ThreeDSCompInd::Y,
            api_models::payments::ThreeDsCompletionIndicator::Failure
            | api_models::payments::ThreeDsCompletionIndicator::NotAvailable => ThreeDSCompInd::N,
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

impl
    TryFrom<(
        &RedsysRouterData<&PaymentsCaptureRouterData>,
        &RedsysAuthType,
    )> for RedsysOperationRequest
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        (item, auth): (
            &RedsysRouterData<&PaymentsCaptureRouterData>,
            &RedsysAuthType,
        ),
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            ds_merchant_order: item.router_data.request.connector_transaction_id.clone(),
            ds_merchant_merchantcode: auth.merchant_id.clone(),
            ds_merchant_terminal: auth.terminal_id.clone(),
            ds_merchant_currency: item.router_data.request.currency.iso_4217().to_owned(),
            ds_merchant_transactiontype: RedsysTransactionType::Confirmation,
            ds_merchant_amount: item.amount.clone(),
        })
    }
}

impl TryFrom<(&RedsysOperationRequest, &RedsysAuthType)> for RedsysTransaction {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(data: (&RedsysOperationRequest, &RedsysAuthType)) -> Result<Self, Self::Error> {
        let (request_data, auth) = data;
        let merchant_parameters = serde_json::to_string(&request_data)
            .change_context(errors::ConnectorError::RequestEncodingFailed)
            .attach_printable("Failed Serialization of PaymentsRequest struct")?;
        logger::debug!("sssssss_merchant_parameters: {:?}", merchant_parameters);
        let ds_merchant_parameters = encode(&merchant_parameters);
        logger::debug!("dsssssss_merchant_parameters: {:?}", ds_merchant_parameters);
        let sha256_pwd = auth.sha256_pwd.clone().expose();
        let signature = get_signature(
            &request_data.ds_merchant_order,
            &ds_merchant_parameters,
            &sha256_pwd,
        )?;
        logger::debug!("dssssssss_signature: {:?}", signature);
        Ok(Self {
            ds_signature_version: SIGNATURE_VERSION.to_string(),
            ds_merchant_parameters: Secret::new(ds_merchant_parameters),
            ds_signature: Secret::new(signature),
        })
    }
}

impl<F> TryFrom<ResponseRouterData<F, RedsysResponse, PaymentsCaptureData, PaymentsResponseData>>
    for RouterData<F, PaymentsCaptureData, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, RedsysResponse, PaymentsCaptureData, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        match item.response {
            RedsysResponse::RedsysResponse(redsys_transaction_response) => {
                let response_data: RedsysPaymentsResponse = get_merchant_parameters(
                    &redsys_transaction_response
                        .ds_merchant_parameters
                        .clone()
                        .expose(),
                )?;

                let status = common_enums::enums::AttemptStatus::foreign_try_from((
                    response_data.ds_response.ok_or(
                        errors::ConnectorError::UnexpectedResponseError(bytes::Bytes::from(
                            "Missing ds_response in the response",
                        )),
                    )?,
                    None,
                ))?;

                let response = Ok(PaymentsResponseData::TransactionResponse {
                    resource_id: ResponseId::ConnectorTransactionId(response_data.ds_order.clone()),
                    redirection_data: Box::new(None),
                    mandate_reference: Box::new(None),
                    connector_metadata: None,
                    network_txn_id: None,
                    connector_response_reference_id: Some(response_data.ds_order.clone()),
                    incremental_authorization_allowed: None,
                    charges: None,
                });

                Ok(RouterData {
                    status,
                    response,
                    ..item.data
                })
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

                Ok(RouterData {
                    status: common_enums::enums::AttemptStatus::Failure,
                    response,
                    ..item.data
                })
            }
        }
    }
}

impl
    TryFrom<(
        &RedsysRouterData<&PaymentsCancelRouterData>,
        &RedsysAuthType,
    )> for RedsysOperationRequest
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        (item, auth): (
            &RedsysRouterData<&PaymentsCancelRouterData>,
            &RedsysAuthType,
        ),
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            ds_merchant_order: item.router_data.request.connector_transaction_id.clone(),
            ds_merchant_merchantcode: auth.merchant_id.clone(),
            ds_merchant_terminal: auth.terminal_id.clone(),
            ds_merchant_currency: item.currency.iso_4217().to_owned(),
            ds_merchant_transactiontype: RedsysTransactionType::Cancellation,
            ds_merchant_amount: item.amount.clone(),
        })
    }
}

impl<F> TryFrom<(&RedsysRouterData<&RefundsRouterData<F>>, &RedsysAuthType)>
    for RedsysOperationRequest
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        (item, auth): (&RedsysRouterData<&RefundsRouterData<F>>, &RedsysAuthType),
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            ds_merchant_order: item.router_data.request.connector_transaction_id.clone(),
            ds_merchant_merchantcode: auth.merchant_id.clone(),
            ds_merchant_terminal: auth.terminal_id.clone(),
            ds_merchant_currency: item.currency.iso_4217().to_owned(),
            ds_merchant_transactiontype: RedsysTransactionType::Refund,
            ds_merchant_amount: item.amount.clone(),
        })
    }
}

impl<F> TryFrom<ResponseRouterData<F, RedsysResponse, PaymentsCancelData, PaymentsResponseData>>
    for RouterData<F, PaymentsCancelData, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, RedsysResponse, PaymentsCancelData, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        match item.response {
            RedsysResponse::RedsysResponse(redsys_transaction_response) => {
                let response_data: RedsysPaymentsResponse = get_merchant_parameters(
                    &redsys_transaction_response
                        .ds_merchant_parameters
                        .clone()
                        .expose(),
                )?;

                let status = common_enums::enums::AttemptStatus::foreign_try_from((
                    response_data.ds_response.ok_or(
                        errors::ConnectorError::UnexpectedResponseError(bytes::Bytes::from(
                            "Missing ds_response in the response",
                        )),
                    )?,
                    None,
                ))?;

                let response = Ok(PaymentsResponseData::TransactionResponse {
                    resource_id: ResponseId::ConnectorTransactionId(response_data.ds_order.clone()),
                    redirection_data: Box::new(None),
                    mandate_reference: Box::new(None),
                    connector_metadata: None,
                    network_txn_id: None,
                    connector_response_reference_id: Some(response_data.ds_order.clone()),
                    incremental_authorization_allowed: None,
                    charges: None,
                });

                Ok(RouterData {
                    status,
                    response,
                    ..item.data
                })
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

                Ok(RouterData {
                    status: common_enums::enums::AttemptStatus::VoidFailed,
                    response,
                    ..item.data
                })
            }
        }
    }
}

impl TryFrom<DsResponse> for enums::RefundStatus {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(ds_response: DsResponse) -> Result<Self, Self::Error> {
        match ds_response.0.as_str() {
            "0900" => Ok(common_enums::enums::RefundStatus::Success),
            "9998" | "9999" => Ok(common_enums::enums::RefundStatus::Pending),
            "950" => Ok(common_enums::enums::RefundStatus::Failure),
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
                let response_data: RedsysPaymentsResponse = get_merchant_parameters(
                    &redsys_transaction.ds_merchant_parameters.clone().expose(),
                )?;
                router_env::logger::info!(sssssssssss=?response_data);
                let sd_response = response_data.ds_response.ok_or(
                    errors::ConnectorError::UnexpectedResponseError(bytes::Bytes::from(
                        "Missing ds_response in the response",
                    )),
                )?;

                Ok(RefundsResponseData {
                    connector_refund_id: response_data.ds_order, //Same as connector_transaction_id, as Redsys does not provide a refund id
                    refund_status: enums::RefundStatus::try_from(sd_response)?,
                })
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
