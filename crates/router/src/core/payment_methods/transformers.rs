use std::str::FromStr;

use api_models::enums as api_enums;
use common_utils::{ext_traits::StringExt, pii::Email, request::RequestContent};
use error_stack::ResultExt;
use josekit::jwe;
use serde::{Deserialize, Serialize};

use crate::{
    configs::settings,
    core::errors::{self, CustomResult},
    headers,
    pii::{prelude::*, Secret},
    services::{api as services, encryption},
    types::{api, storage},
    utils::{self, OptionExt},
};

#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum StoreLockerReq<'a> {
    LockerCard(StoreCardReq<'a>),
    LockerGeneric(StoreGenericReq<'a>),
}

#[derive(Debug, Deserialize, Serialize)]
pub struct StoreCardReq<'a> {
    pub merchant_id: &'a str,
    pub merchant_customer_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub requestor_card_reference: Option<String>,
    pub card: Card,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct StoreGenericReq<'a> {
    pub merchant_id: &'a str,
    pub merchant_customer_id: String,
    #[serde(rename = "enc_card_data")]
    pub enc_data: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Card {
    pub card_number: cards::CardNumber,
    pub name_on_card: Option<Secret<String>>,
    pub card_exp_month: Secret<String>,
    pub card_exp_year: Secret<String>,
    pub card_brand: Option<String>,
    pub card_isin: Option<String>,
    pub nick_name: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct StoreCardResp {
    pub status: String,
    pub error_message: Option<String>,
    pub error_code: Option<String>,
    pub payload: Option<StoreCardRespPayload>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct StoreCardRespPayload {
    pub card_reference: String,
    pub duplication_check: Option<DataDuplicationCheck>,
}

#[derive(Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum DataDuplicationCheck {
    Duplicated,
    MetaDataChanged,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CardReqBody<'a> {
    pub merchant_id: &'a str,
    pub merchant_customer_id: String,
    pub card_reference: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct RetrieveCardResp {
    pub status: String,
    pub error_message: Option<String>,
    pub error_code: Option<String>,
    pub payload: Option<RetrieveCardRespPayload>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct RetrieveCardRespPayload {
    pub card: Option<Card>,
    pub enc_card_data: Option<Secret<String>>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct DeleteCardResp {
    pub status: String,
    pub error_message: Option<String>,
    pub error_code: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AddCardRequest<'a> {
    pub card_number: cards::CardNumber,
    pub customer_id: String,
    pub card_exp_month: Secret<String>,
    pub card_exp_year: Secret<String>,
    pub merchant_id: &'a str,
    pub email_address: Option<Email>,
    pub name_on_card: Option<Secret<String>>,
    pub nickname: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AddCardResponse {
    pub card_id: String,
    pub external_id: String,
    pub card_fingerprint: Secret<String>,
    pub card_global_fingerprint: Secret<String>,
    #[serde(rename = "merchant_id")]
    pub merchant_id: Option<String>,
    pub card_number: Option<cards::CardNumber>,
    pub card_exp_year: Option<Secret<String>>,
    pub card_exp_month: Option<Secret<String>>,
    pub name_on_card: Option<Secret<String>>,
    pub nickname: Option<String>,
    pub customer_id: Option<String>,
    pub duplicate: Option<bool>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AddPaymentMethodResponse {
    pub payment_method_id: String,
    pub external_id: String,
    #[serde(rename = "merchant_id")]
    pub merchant_id: Option<String>,
    pub nickname: Option<String>,
    pub customer_id: Option<String>,
    pub duplicate: Option<bool>,
    pub payment_method_data: Secret<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct GetPaymentMethodResponse {
    pub payment_method: AddPaymentMethodResponse,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct GetCardResponse {
    pub card: AddCardResponse,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GetCard<'a> {
    merchant_id: &'a str,
    card_id: &'a str,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteCardResponse {
    pub card_id: Option<String>,
    pub external_id: Option<String>,
    pub card_isin: Option<Secret<String>>,
    pub status: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(transparent)]
pub struct PaymentMethodMetadata {
    pub payment_method_tokenization: std::collections::HashMap<String, String>,
}

pub fn get_dotted_jwe(jwe: encryption::JweBody) -> String {
    let header = jwe.header;
    let encryption_key = jwe.encrypted_key;
    let iv = jwe.iv;
    let encryption_payload = jwe.encrypted_payload;
    let tag = jwe.tag;
    format!("{header}.{encryption_key}.{iv}.{encryption_payload}.{tag}")
}

pub fn get_dotted_jws(jws: encryption::JwsBody) -> String {
    let header = jws.header;
    let payload = jws.payload;
    let signature = jws.signature;
    format!("{header}.{payload}.{signature}")
}

pub async fn get_decrypted_response_payload(
    #[cfg(not(feature = "aws_kms"))] jwekey: &settings::Jwekey,
    #[cfg(feature = "aws_kms")] jwekey: &settings::ActiveKmsSecrets,
    jwe_body: encryption::JweBody,
    locker_choice: Option<api_enums::LockerChoice>,
) -> CustomResult<String, errors::VaultError> {
    let target_locker = locker_choice.unwrap_or(api_enums::LockerChoice::Basilisk);

    #[cfg(feature = "aws_kms")]
    let public_key = match target_locker {
        api_enums::LockerChoice::Basilisk => jwekey.jwekey.peek().vault_encryption_key.as_bytes(),
        api_enums::LockerChoice::Tartarus => {
            jwekey.jwekey.peek().rust_locker_encryption_key.as_bytes()
        }
    };

    #[cfg(feature = "aws_kms")]
    let private_key = jwekey.jwekey.peek().vault_private_key.as_bytes();

    #[cfg(not(feature = "aws_kms"))]
    let public_key = match target_locker {
        api_enums::LockerChoice::Basilisk => jwekey.vault_encryption_key.as_bytes(),
        api_enums::LockerChoice::Tartarus => jwekey.rust_locker_encryption_key.as_bytes(),
    };

    #[cfg(not(feature = "aws_kms"))]
    let private_key = jwekey.vault_private_key.as_bytes();

    let jwt = get_dotted_jwe(jwe_body);
    let alg = jwe::RSA_OAEP;

    let jwe_decrypted = encryption::decrypt_jwe(
        &jwt,
        encryption::KeyIdCheck::SkipKeyIdCheck,
        private_key,
        alg,
    )
    .await
    .change_context(errors::VaultError::SaveCardFailed)
    .attach_printable("Jwe Decryption failed for JweBody for vault")?;

    let jws = jwe_decrypted
        .parse_struct("JwsBody")
        .change_context(errors::VaultError::ResponseDeserializationFailed)?;
    let jws_body = get_dotted_jws(jws);

    encryption::verify_sign(jws_body, public_key)
        .change_context(errors::VaultError::SaveCardFailed)
        .attach_printable("Jws Decryption failed for JwsBody for vault")
}

pub async fn mk_basilisk_req(
    #[cfg(feature = "aws_kms")] jwekey: &settings::ActiveKmsSecrets,
    #[cfg(not(feature = "aws_kms"))] jwekey: &settings::Jwekey,
    jws: &str,
    locker_choice: api_enums::LockerChoice,
) -> CustomResult<encryption::JweBody, errors::VaultError> {
    let jws_payload: Vec<&str> = jws.split('.').collect();

    let generate_jws_body = |payload: Vec<&str>| -> Option<encryption::JwsBody> {
        Some(encryption::JwsBody {
            header: payload.first()?.to_string(),
            payload: payload.get(1)?.to_string(),
            signature: payload.get(2)?.to_string(),
        })
    };

    let jws_body = generate_jws_body(jws_payload).ok_or(errors::VaultError::SaveCardFailed)?;

    let payload = utils::Encode::<encryption::JwsBody>::encode_to_vec(&jws_body)
        .change_context(errors::VaultError::SaveCardFailed)?;

    #[cfg(feature = "aws_kms")]
    let public_key = match locker_choice {
        api_enums::LockerChoice::Basilisk => jwekey.jwekey.peek().vault_encryption_key.as_bytes(),
        api_enums::LockerChoice::Tartarus => {
            jwekey.jwekey.peek().rust_locker_encryption_key.as_bytes()
        }
    };

    #[cfg(not(feature = "aws_kms"))]
    let public_key = match locker_choice {
        api_enums::LockerChoice::Basilisk => jwekey.vault_encryption_key.as_bytes(),
        api_enums::LockerChoice::Tartarus => jwekey.rust_locker_encryption_key.as_bytes(),
    };

    let jwe_encrypted = encryption::encrypt_jwe(&payload, public_key)
        .await
        .change_context(errors::VaultError::SaveCardFailed)
        .attach_printable("Error on jwe encrypt")?;
    let jwe_payload: Vec<&str> = jwe_encrypted.split('.').collect();

    let generate_jwe_body = |payload: Vec<&str>| -> Option<encryption::JweBody> {
        Some(encryption::JweBody {
            header: payload.first()?.to_string(),
            iv: payload.get(2)?.to_string(),
            encrypted_payload: payload.get(3)?.to_string(),
            tag: payload.get(4)?.to_string(),
            encrypted_key: payload.get(1)?.to_string(),
        })
    };

    let jwe_body = generate_jwe_body(jwe_payload).ok_or(errors::VaultError::SaveCardFailed)?;

    Ok(jwe_body)
}

pub async fn mk_add_locker_request_hs<'a>(
    #[cfg(not(feature = "aws_kms"))] jwekey: &settings::Jwekey,
    #[cfg(feature = "aws_kms")] jwekey: &settings::ActiveKmsSecrets,
    locker: &settings::Locker,
    payload: &StoreLockerReq<'a>,
    locker_choice: api_enums::LockerChoice,
) -> CustomResult<services::Request, errors::VaultError> {
    let payload = utils::Encode::<StoreCardReq<'_>>::encode_to_vec(&payload)
        .change_context(errors::VaultError::RequestEncodingFailed)?;

    #[cfg(feature = "aws_kms")]
    let private_key = jwekey.jwekey.peek().vault_private_key.as_bytes();

    #[cfg(not(feature = "aws_kms"))]
    let private_key = jwekey.vault_private_key.as_bytes();

    let jws = encryption::jws_sign_payload(&payload, &locker.locker_signing_key_id, private_key)
        .await
        .change_context(errors::VaultError::RequestEncodingFailed)?;

    let jwe_payload = mk_basilisk_req(jwekey, &jws, locker_choice).await?;
    let mut url = match locker_choice {
        api_enums::LockerChoice::Basilisk => locker.host.to_owned(),
        api_enums::LockerChoice::Tartarus => locker.host_rs.to_owned(),
    };
    url.push_str("/cards/add");
    let mut request = services::Request::new(services::Method::Post, &url);
    request.add_header(headers::CONTENT_TYPE, "application/json".into());
    request.set_body(RequestContent::Json(Box::new(jwe_payload)));
    Ok(request)
}

pub fn mk_add_bank_response_hs(
    bank: api::BankPayout,
    bank_reference: String,
    req: api::PaymentMethodCreate,
    merchant_id: &str,
) -> api::PaymentMethodResponse {
    api::PaymentMethodResponse {
        merchant_id: merchant_id.to_owned(),
        customer_id: req.customer_id,
        payment_method_id: bank_reference,
        payment_method: req.payment_method,
        payment_method_type: req.payment_method_type,
        bank_transfer: Some(bank),
        card: None,
        metadata: req.metadata,
        created: Some(common_utils::date_time::now()),
        recurring_enabled: false,           // [#256]
        installment_payment_enabled: false, // #[#256]
        payment_experience: Some(vec![api_models::enums::PaymentExperience::RedirectToUrl]), // [#256]
    }
}

pub fn mk_add_card_response_hs(
    card: api::CardDetail,
    card_reference: String,
    req: api::PaymentMethodCreate,
    merchant_id: &str,
) -> api::PaymentMethodResponse {
    let card_number = card.card_number.clone();
    let last4_digits = card_number.clone().get_last4();
    let card_isin = card_number.get_card_isin();

    let card = api::CardDetailFromLocker {
        scheme: None,
        last4_digits: Some(last4_digits),
        issuer_country: None,
        card_number: Some(card.card_number.clone()),
        expiry_month: Some(card.card_exp_month.clone()),
        expiry_year: Some(card.card_exp_year.clone()),
        card_token: None,
        card_fingerprint: None,
        card_holder_name: card.card_holder_name.clone(),
        nick_name: card.nick_name.clone(),
        card_isin: Some(card_isin),
        card_issuer: card.card_issuer,
        card_network: card.card_network,
        card_type: card.card_type,
        saved_to_locker: true,
    };
    api::PaymentMethodResponse {
        merchant_id: merchant_id.to_owned(),
        customer_id: req.customer_id,
        payment_method_id: card_reference,
        payment_method: req.payment_method,
        payment_method_type: req.payment_method_type,
        bank_transfer: None,
        card: Some(card),
        metadata: req.metadata,
        created: Some(common_utils::date_time::now()),
        recurring_enabled: false,           // [#256]
        installment_payment_enabled: false, // #[#256]
        payment_experience: Some(vec![api_models::enums::PaymentExperience::RedirectToUrl]), // [#256]
    }
}

pub fn mk_add_card_response(
    card: api::CardDetail,
    response: AddCardResponse,
    req: api::PaymentMethodCreate,
    merchant_id: &str,
) -> api::PaymentMethodResponse {
    let mut card_number = card.card_number.peek().to_owned();
    let card = api::CardDetailFromLocker {
        scheme: None,
        last4_digits: Some(card_number.split_off(card_number.len() - 4)),
        issuer_country: None, // [#256] bin mapping
        card_number: Some(card.card_number),
        expiry_month: Some(card.card_exp_month),
        expiry_year: Some(card.card_exp_year),
        card_token: Some(response.external_id.into()), // [#256]
        card_fingerprint: Some(response.card_fingerprint),
        card_holder_name: card.card_holder_name,
        nick_name: card.nick_name,
        card_isin: None,
        card_issuer: None,
        card_network: None,
        card_type: None,
        saved_to_locker: true,
    };
    api::PaymentMethodResponse {
        merchant_id: merchant_id.to_owned(),
        customer_id: req.customer_id,
        payment_method_id: response.card_id,
        payment_method: req.payment_method,
        payment_method_type: req.payment_method_type,
        bank_transfer: None,
        card: Some(card),
        metadata: req.metadata,
        created: Some(common_utils::date_time::now()),
        recurring_enabled: false,           // [#256]
        installment_payment_enabled: false, // [#256] Pending on discussion, and not stored in the card locker
        payment_experience: None,           // [#256]
    }
}

pub fn mk_add_card_request(
    locker: &settings::Locker,
    card: &api::CardDetail,
    customer_id: &str,
    _req: &api::PaymentMethodCreate,
    locker_id: &'static str,
    merchant_id: &str,
) -> CustomResult<services::Request, errors::VaultError> {
    let customer_id = if cfg!(feature = "release") {
        format!("{customer_id}::{merchant_id}")
    } else {
        customer_id.to_owned()
    };
    let add_card_req = AddCardRequest {
        card_number: card.card_number.clone(),
        customer_id,
        card_exp_month: card.card_exp_month.clone(),
        card_exp_year: card.card_exp_year.clone(),
        merchant_id: locker_id,
        email_address: match Email::from_str("dummy@gmail.com") {
            Ok(email) => Some(email),
            Err(_) => None,
        }, //
        name_on_card: Some("John Doe".to_string().into()), // [#256]
        nickname: Some("router".to_string()),              //
    };
    let mut url = locker.host.to_owned();
    url.push_str("/card/addCard");
    let mut request = services::Request::new(services::Method::Post, &url);
    request.set_body(RequestContent::FormUrlEncoded(Box::new(add_card_req)));
    Ok(request)
}

pub async fn mk_get_card_request_hs(
    #[cfg(not(feature = "aws_kms"))] jwekey: &settings::Jwekey,
    #[cfg(feature = "aws_kms")] jwekey: &settings::ActiveKmsSecrets,
    locker: &settings::Locker,
    customer_id: &str,
    merchant_id: &str,
    card_reference: &str,
    locker_choice: Option<api_enums::LockerChoice>,
) -> CustomResult<services::Request, errors::VaultError> {
    let merchant_customer_id = customer_id.to_owned();
    let card_req_body = CardReqBody {
        merchant_id,
        merchant_customer_id,
        card_reference: card_reference.to_owned(),
    };
    let payload = utils::Encode::<CardReqBody<'_>>::encode_to_vec(&card_req_body)
        .change_context(errors::VaultError::RequestEncodingFailed)?;

    #[cfg(feature = "aws_kms")]
    let private_key = jwekey.jwekey.peek().vault_private_key.as_bytes();

    #[cfg(not(feature = "aws_kms"))]
    let private_key = jwekey.vault_private_key.as_bytes();

    let jws = encryption::jws_sign_payload(&payload, &locker.locker_signing_key_id, private_key)
        .await
        .change_context(errors::VaultError::RequestEncodingFailed)?;

    let target_locker = locker_choice.unwrap_or(api_enums::LockerChoice::Basilisk);

    let jwe_payload = mk_basilisk_req(jwekey, &jws, target_locker).await?;
    let mut url = match target_locker {
        api_enums::LockerChoice::Basilisk => locker.host.to_owned(),
        api_enums::LockerChoice::Tartarus => locker.host_rs.to_owned(),
    };
    url.push_str("/cards/retrieve");
    let mut request = services::Request::new(services::Method::Post, &url);
    request.add_header(headers::CONTENT_TYPE, "application/json".into());
    request.set_body(RequestContent::Json(Box::new(jwe_payload)));
    Ok(request)
}

pub fn mk_get_card_request(
    locker: &settings::Locker,
    locker_id: &'static str,
    card_id: &'static str,
) -> CustomResult<services::Request, errors::VaultError> {
    let get_card_req = GetCard {
        merchant_id: locker_id,
        card_id,
    };

    let mut url = locker.host.to_owned();
    url.push_str("/card/getCard");
    let mut request = services::Request::new(services::Method::Post, &url);
    request.set_body(RequestContent::FormUrlEncoded(Box::new(get_card_req)));
    Ok(request)
}

pub fn mk_get_card_response(card: GetCardResponse) -> errors::RouterResult<Card> {
    Ok(Card {
        card_number: card.card.card_number.get_required_value("card_number")?,
        name_on_card: card.card.name_on_card,
        card_exp_month: card
            .card
            .card_exp_month
            .get_required_value("card_exp_month")?,
        card_exp_year: card
            .card
            .card_exp_year
            .get_required_value("card_exp_year")?,
        card_brand: None,
        card_isin: None,
        nick_name: card.card.nickname,
    })
}

pub async fn mk_delete_card_request_hs(
    #[cfg(feature = "aws_kms")] jwekey: &settings::ActiveKmsSecrets,
    #[cfg(not(feature = "aws_kms"))] jwekey: &settings::Jwekey,
    locker: &settings::Locker,
    customer_id: &str,
    merchant_id: &str,
    card_reference: &str,
) -> CustomResult<services::Request, errors::VaultError> {
    let merchant_customer_id = customer_id.to_owned();
    let card_req_body = CardReqBody {
        merchant_id,
        merchant_customer_id,
        card_reference: card_reference.to_owned(),
    };
    let payload = utils::Encode::<CardReqBody<'_>>::encode_to_vec(&card_req_body)
        .change_context(errors::VaultError::RequestEncodingFailed)?;

    #[cfg(feature = "aws_kms")]
    let private_key = jwekey.jwekey.peek().vault_private_key.as_bytes();

    #[cfg(not(feature = "aws_kms"))]
    let private_key = jwekey.vault_private_key.as_bytes();

    let jws = encryption::jws_sign_payload(&payload, &locker.locker_signing_key_id, private_key)
        .await
        .change_context(errors::VaultError::RequestEncodingFailed)?;

    let jwe_payload = mk_basilisk_req(jwekey, &jws, api_enums::LockerChoice::Basilisk).await?;

    let mut url = locker.host.to_owned();
    url.push_str("/cards/delete");
    let mut request = services::Request::new(services::Method::Post, &url);
    request.add_header(headers::CONTENT_TYPE, "application/json".into());
    request.set_body(RequestContent::Json(Box::new(jwe_payload)));
    Ok(request)
}

pub fn mk_delete_card_request(
    locker: &settings::Locker,
    merchant_id: &'static str,
    card_id: &'static str,
) -> CustomResult<services::Request, errors::VaultError> {
    let delete_card_req = GetCard {
        merchant_id,
        card_id,
    };
    let mut url = locker.host.to_owned();
    url.push_str("/card/deleteCard");
    let mut request = services::Request::new(services::Method::Post, &url);
    request.add_default_headers();

    request.set_body(RequestContent::FormUrlEncoded(Box::new(delete_card_req)));
    Ok(request)
}

pub fn mk_delete_card_response(
    response: DeleteCardResponse,
) -> errors::RouterResult<DeleteCardResp> {
    Ok(DeleteCardResp {
        status: response.status,
        error_message: None,
        error_code: None,
    })
}

pub fn get_card_detail(
    pm: &storage::PaymentMethod,
    response: Card,
) -> CustomResult<api::CardDetailFromLocker, errors::VaultError> {
    let card_number = response.card_number;
    let mut last4_digits = card_number.peek().to_owned();
    //fetch form card bin

    let card_detail = api::CardDetailFromLocker {
        scheme: pm.scheme.to_owned(),
        issuer_country: pm.issuer_country.clone(),
        last4_digits: Some(last4_digits.split_off(last4_digits.len() - 4)),
        card_number: Some(card_number),
        expiry_month: Some(response.card_exp_month),
        expiry_year: Some(response.card_exp_year),
        card_token: None,
        card_fingerprint: None,
        card_holder_name: response.name_on_card,
        nick_name: response.nick_name.map(masking::Secret::new),
        card_isin: None,
        card_issuer: None,
        card_network: None,
        card_type: None,
        saved_to_locker: true,
    };
    Ok(card_detail)
}

//------------------------------------------------TokenizeService------------------------------------------------
pub fn mk_crud_locker_request(
    locker: &settings::Locker,
    path: &str,
    req: api::TokenizePayloadEncrypted,
) -> CustomResult<services::Request, errors::VaultError> {
    let mut url = locker.basilisk_host.to_owned();
    url.push_str(path);
    let mut request = services::Request::new(services::Method::Post, &url);
    request.add_default_headers();
    request.add_header(headers::CONTENT_TYPE, "application/json".into());
    request.set_body(RequestContent::Json(Box::new(req)));
    Ok(request)
}

pub fn mk_card_value1(
    card_number: cards::CardNumber,
    exp_year: String,
    exp_month: String,
    name_on_card: Option<String>,
    nickname: Option<String>,
    card_last_four: Option<String>,
    card_token: Option<String>,
) -> CustomResult<String, errors::VaultError> {
    let value1 = api::TokenizedCardValue1 {
        card_number: card_number.peek().clone(),
        exp_year,
        exp_month,
        name_on_card,
        nickname,
        card_last_four,
        card_token,
    };
    let value1_req = utils::Encode::<api::TokenizedCardValue1>::encode_to_string_of_json(&value1)
        .change_context(errors::VaultError::FetchCardFailed)?;
    Ok(value1_req)
}

pub fn mk_card_value2(
    card_security_code: Option<String>,
    card_fingerprint: Option<String>,
    external_id: Option<String>,
    customer_id: Option<String>,
    payment_method_id: Option<String>,
) -> CustomResult<String, errors::VaultError> {
    let value2 = api::TokenizedCardValue2 {
        card_security_code,
        card_fingerprint,
        external_id,
        customer_id,
        payment_method_id,
    };
    let value2_req = utils::Encode::<api::TokenizedCardValue2>::encode_to_string_of_json(&value2)
        .change_context(errors::VaultError::FetchCardFailed)?;
    Ok(value2_req)
}
