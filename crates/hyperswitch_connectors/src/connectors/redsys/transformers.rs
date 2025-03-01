use common_utils::{types::StringMinorUnit};
use regex::Regex;
use error_stack::ResultExt;
use hyperswitch_domain_models::{
    payment_method_data::{Card, PaymentMethodData},
    router_data::{ConnectorAuthType, RouterData},
    router_flow_types::refunds::{Execute, RSync},
    router_request_types::ResponseId,
    router_response_types::{PaymentsResponseData, RefundsResponseData},
    types::{PaymentsAuthorizeRouterData, PaymentsPreProcessingRouterData, RefundsRouterData},
};
use hyperswitch_interfaces::errors;
use masking::{Secret, ExposeInterface};
use router_env::logger;
use serde::{Deserialize, Serialize};
use base64::{decode, encode};
use hmac::{Hmac, Mac, NewMac};
use openssl::symm::{encrypt, Cipher};
use sha2::Sha256;

use crate::{
    types::{RefundsResponseRouterData, ResponseRouterData},
    utils::{PaymentsAuthorizeRequestData, get_unimplemented_payment_method_error_message, CardData},
};


pub struct RedsysRouterData<T> {
    pub amount: StringMinorUnit, 
    pub currency: api_models::enums::Currency,
    pub router_data: T,
}

impl<T> From<(StringMinorUnit, T, api_models::enums::Currency,)> for RedsysRouterData<T> {
    fn from((amount, item, currency): (StringMinorUnit, T, api_models::enums::Currency,)) -> Self {
        Self {
            amount,
            currency,
            router_data: item,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub struct IniciaPeticionRequest {
    ds_merchant_emv3ds: EmvThreedsData,

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

fn get_redsys_order_id(connector_request_reference_id: String) -> Result<String, error_stack::Report<errors::ConnectorError>> {
    let removed_special_chars = connector_request_reference_id.replace("_", "");
    if removed_special_chars.len() < 4 {
        return Err(errors::ConnectorError::InvalidDataFormat { field_name: "connector_request_reference_id" }.into());
    };
    
    if  removed_special_chars.len() > 12 {
        Ok(removed_special_chars[removed_special_chars.len().saturating_sub(12)..].to_string())
    } else {
        Ok(removed_special_chars)
    }
}



impl TryFrom<&Option<PaymentMethodData>>
    for RedsysCardData
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        payment_method_data: &Option<PaymentMethodData>,
    ) -> Result<Self, Self::Error> {
        match payment_method_data {
            Some(PaymentMethodData::Card(card)) => {
                let yymm_expiry_date = card.get_expiry_date_as_yymm()?.expose();
                let expiry_date = Secret::new("2512".to_owned());
                Ok(Self {
                    card_number: card.card_number.clone(),
                    expiry_date,
                    cvv2: card.card_cvc.clone(),
                })
            },
            _ => Err(errors::ConnectorError::NotImplemented(
                get_unimplemented_payment_method_error_message("Redsys"),
            ).into()),
        }
    }
}


impl TryFrom<(&RedsysRouterData<&PaymentsPreProcessingRouterData>, &RedsysAuthType)>
    for IniciaPeticionRequest
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        data: (&RedsysRouterData<&PaymentsPreProcessingRouterData>, &RedsysAuthType),
    ) -> Result<Self, Self::Error> {
        let (item, auth) = data;
        let ds_merchant_emv3ds = if item.router_data.auth_type == common_enums::enums::AuthenticationType::ThreeDs {
            Ok(EmvThreedsData {
            three_d_s_info: RedsysThreeDsInfo::CardData,
        })
    } 
        else {
            Err(errors::ConnectorError::FlowNotSupported {
                flow: "PreProcessing".to_string(),
                connector: "Redsys".to_string(),
            })
        }?;

        let ds_merchant_transactiontype = RedsysTransactionType::Payment;
        let ds_merchant_order = get_redsys_order_id(item.router_data.connector_request_reference_id.clone())?; 
        let card_data = RedsysCardData::try_from(&item.router_data.request.payment_method_data)?;

        Ok(Self {
            ds_merchant_emv3ds,
            ds_merchant_transactiontype,
            ds_merchant_currency: "978".to_owned(),
            ds_merchant_pan:  card_data.card_number,
            ds_merchant_merchantcode: auth.merchant_id.clone(),
            ds_merchant_terminal: auth.terminal_id.clone(),
            ds_merchant_order ,
            ds_merchant_amount:  item.amount.clone(),
            ds_merchant_expirydate: card_data.expiry_date,
            ds_merchant_cvv2: card_data.cvv2,
        })
    }
}

pub const SIGNATURE_VERSION: &str = "HMAC_SHA256_V1";


#[derive(Debug, Serialize)]
pub struct RedsysRequest {
    #[serde(rename = "Ds_SignatureVersion")]
    ds_signature_version:  String,
    #[serde(rename = "Ds_MerchantParameters")]
    ds_merchant_parameters: Secret<String>,
    #[serde(rename = "Ds_Signature")]
    ds_signature: Secret<String>,
}


fn base64_decode(input: &str) -> Result<Vec<u8>, error_stack::Report<errors::ConnectorError>>  {
    decode(input).change_context(errors::ConnectorError::RequestEncodingFailed).attach_printable("Base 64 encoding failed")
}


fn convert_array_to_ascii(iv_array: &[u8]) -> String {
    iv_array.iter().map(|&c| c as char).collect()
}

fn des_encrypt(message: &str, key: &str) -> Result<String, error_stack::Report<errors::ConnectorError>>  {
    let iv = vec![0u8; 8]; // IV of 8 zero bytes
    let cipher = Cipher::des_ede3_cbc();
    let key_bytes = base64_decode(key)?;
    let encrypted = encrypt(cipher, &key_bytes, Some(&iv), message.as_bytes()).change_context(errors::ConnectorError::RequestEncodingFailed).attach_printable("Triple DES Encryption failed")?;
    let encoded = encode(encrypted);
    Ok(encoded) 
}

fn get_signature(order_id: &str, params: &str, clave: &str) -> Result<String, error_stack::Report<errors::ConnectorError>>  {
    let secret_ko = des_encrypt(order_id, clave)?;
    let base_decoded = base64_decode(&secret_ko)?;
    let mut mac = Hmac::<Sha256>::new_from_slice(&base_decoded)
    .map_err(|_| errors::ConnectorError::RequestEncodingFailed).attach_printable("HMAC SHA256 failed")?;
    mac.update(params.as_bytes());
    let result = mac.finalize().into_bytes();
    let encoded = encode(result) ;
    Ok(encoded)
}

impl TryFrom<(&IniciaPeticionRequest, 
    &RedsysAuthType)>
    for RedsysRequest
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        data: (&IniciaPeticionRequest, &RedsysAuthType),
    ) -> Result<Self, Self::Error> {
        let (request_data, auth) = data;
        let merchant_parameters = serde_json::to_string(&request_data).change_context(errors::ConnectorError::RequestEncodingFailed).attach_printable("Failed Serialization of IniciaPeticionRequest struct")?;
        logger::debug!("dsssssss_merchant_parameters: {:?}", merchant_parameters);
        let ds_merchant_parameters = encode(&merchant_parameters);
        logger::debug!("dsssssss_merchant_parameters: {:?}", ds_merchant_parameters);
        let sha256_pwd = auth.sha256_pwd.clone().expose();
        let signature = get_signature(&request_data.ds_merchant_order, &ds_merchant_parameters, &sha256_pwd)?;
        logger::debug!("dssssssss_merchant_parameters: {:?}", signature);
        Ok(Self {
            ds_signature_version: SIGNATURE_VERSION.to_string(),
            ds_merchant_parameters: Secret::new(ds_merchant_parameters),
            ds_signature: Secret::new(signature),
        })
    }
}