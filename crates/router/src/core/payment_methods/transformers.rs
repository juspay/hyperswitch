use common_utils::ext_traits::StringExt;
use error_stack::ResultExt;
use josekit::jwe;
use serde::{Deserialize, Serialize};

use crate::{
    configs::settings::{Jwekey, Locker},
    core::errors::{self, CustomResult},
    headers,
    pii::{self, prelude::*, Secret},
    services::{api as services, encryption, kms},
    types::{api, storage},
    utils,
};

#[derive(Debug, Deserialize, Serialize)]
pub struct StoreCardReq<'a> {
    pub merchant_id: &'a str,
    pub merchant_customer_id: String,
    pub card: Card,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Card {
    pub card_number: Secret<String, pii::CardNumber>,
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
    pub enc_card_data: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct DeleteCardResp {
    pub status: String,
    pub error_message: Option<String>,
    pub error_code: Option<String>,
}

pub fn basilisk_hs_key_id() -> &'static str {
    "1"
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
    jwekey: &Jwekey,
    jwe_body: encryption::JweBody,
) -> CustomResult<String, errors::VaultError> {
    #[cfg(feature = "kms")]
    let public_key = kms::KeyHandler::get_kms_decrypted_key(
        &jwekey.aws_region,
        &jwekey.aws_key_id,
        jwekey.vault_encryption_key.to_string(),
    )
    .await
    .change_context(errors::VaultError::SaveCardFailed)
    .attach_printable("Fails to get public key of vault")?;
    #[cfg(not(feature = "kms"))]
    let public_key = jwekey.vault_encryption_key.to_owned();
    #[cfg(feature = "kms")]
    let private_key = kms::KeyHandler::get_kms_decrypted_key(
        &jwekey.aws_region,
        &jwekey.aws_key_id,
        jwekey.vault_private_key.to_string(),
    )
    .await
    .change_context(errors::VaultError::SaveCardFailed)
    .attach_printable("Error getting private key for signing jws")?;
    #[cfg(not(feature = "kms"))]
    let private_key = jwekey.vault_private_key.to_owned();
    let jwt = get_dotted_jwe(jwe_body);
    let key_id = basilisk_hs_key_id();
    let alg = jwe::RSA_OAEP;
    let jwe_decrypted = encryption::decrypt_jwe(jwekey, &jwt, key_id, key_id, private_key, alg)
        .await
        .change_context(errors::VaultError::SaveCardFailed)
        .attach_printable("Jwe Decryption failed for JweBody for vault")?;

    let jws = jwe_decrypted
        .parse_struct("JwsBody")
        .change_context(errors::VaultError::ResponseDeserializationFailed)?;
    let jws_body = get_dotted_jws(jws);

    encryption::verify_sign(jws_body, &public_key)
        .change_context(errors::VaultError::SaveCardFailed)
        .attach_printable("Jws Decryption failed for JwsBody for vault")
}

pub async fn mk_basilisk_req(
    jwekey: &Jwekey,
    jws: &str,
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
    #[cfg(feature = "kms")]
    let public_key = kms::KeyHandler::get_kms_decrypted_key(
        &jwekey.aws_region,
        &jwekey.aws_key_id,
        jwekey.vault_encryption_key.to_string(),
    )
    .await
    .change_context(errors::VaultError::SaveCardFailed)
    .attach_printable("Fails to get encryption key of vault")?;
    #[cfg(not(feature = "kms"))]
    let public_key = jwekey.vault_encryption_key.to_owned();

    let jwe_encrypted = encryption::encrypt_jwe(jwekey, &payload, public_key)
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

pub async fn mk_add_card_request_hs(
    jwekey: &Jwekey,
    locker: &Locker,
    card: &api::CardDetail,
    customer_id: &str,
    _req: &api::PaymentMethodCreate,
    _locker_id: &str,
    merchant_id: &str,
) -> CustomResult<services::Request, errors::VaultError> {
    let merchant_customer_id = if cfg!(feature = "sandbox") {
        format!("{customer_id}::{merchant_id}")
    } else {
        customer_id.to_owned()
    };
    let card = Card {
        card_number: card.card_number.to_owned(),
        name_on_card: card.card_holder_name.to_owned(),
        card_exp_month: card.card_exp_month.to_owned(),
        card_exp_year: card.card_exp_year.to_owned(),
        card_brand: None,
        card_isin: None,
        nick_name: None,
    };
    let store_card_req = StoreCardReq {
        merchant_id,
        merchant_customer_id,
        card,
    };
    let payload = utils::Encode::<StoreCardReq<'_>>::encode_to_vec(&store_card_req)
        .change_context(errors::VaultError::RequestEncodingFailed)?;
    #[cfg(feature = "kms")]
    let private_key = kms::KeyHandler::get_kms_decrypted_key(
        &jwekey.aws_region,
        &jwekey.aws_key_id,
        jwekey.vault_private_key.to_string(),
    )
    .await
    .change_context(errors::VaultError::SaveCardFailed)
    .attach_printable("Error getting private key for signing jws")?;
    #[cfg(not(feature = "kms"))]
    let private_key = jwekey.vault_private_key.to_owned();

    let jws = encryption::jws_sign_payload(&payload, basilisk_hs_key_id(), private_key)
        .await
        .change_context(errors::VaultError::RequestEncodingFailed)?;

    let jwe_payload = mk_basilisk_req(jwekey, &jws).await?;

    let body = utils::Encode::<encryption::JweBody>::encode_to_value(&jwe_payload)
        .change_context(errors::VaultError::RequestEncodingFailed)?;
    let mut url = locker.host.to_owned();
    url.push_str("/cards/add");
    let mut request = services::Request::new(services::Method::Post, &url);
    request.add_header(headers::CONTENT_TYPE, "application/json");
    request.set_body(body.to_string());
    Ok(request)
}

pub fn mk_add_card_response_hs(
    card: api::CardDetail,
    card_reference: String,
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
        card_token: None,       // [#256]
        card_fingerprint: None, // fingerprint not send by basilisk-hs need to have this feature in case we need it in future
        card_holder_name: card.card_holder_name,
    };
    api::PaymentMethodResponse {
        merchant_id: merchant_id.to_owned(),
        customer_id: req.customer_id,
        payment_method_id: card_reference,
        payment_method: req.payment_method,
        payment_method_type: req.payment_method_type,
        card: Some(card),
        metadata: req.metadata,
        created: Some(common_utils::date_time::now()),
        recurring_enabled: false,           // [#256]
        installment_payment_enabled: false, // #[#256]
        payment_experience: Some(vec![api_models::enums::PaymentExperience::RedirectToUrl]), // [#256]
    }
}

pub async fn mk_get_card_request_hs(
    jwekey: &Jwekey,
    locker: &Locker,
    customer_id: &str,
    merchant_id: &str,
    card_reference: &str,
) -> CustomResult<services::Request, errors::VaultError> {
    let merchant_customer_id = if cfg!(feature = "sandbox") {
        format!("{customer_id}::{merchant_id}")
    } else {
        customer_id.to_owned()
    };
    let card_req_body = CardReqBody {
        merchant_id,
        merchant_customer_id,
        card_reference: card_reference.to_owned(),
    };
    let payload = utils::Encode::<CardReqBody<'_>>::encode_to_vec(&card_req_body)
        .change_context(errors::VaultError::RequestEncodingFailed)?;
    #[cfg(feature = "kms")]
    let private_key = kms::KeyHandler::get_kms_decrypted_key(
        &jwekey.aws_region,
        &jwekey.aws_key_id,
        jwekey.vault_private_key.to_string(),
    )
    .await
    .change_context(errors::VaultError::SaveCardFailed)
    .attach_printable("Error getting private key for signing jws")?;
    #[cfg(not(feature = "kms"))]
    let private_key = jwekey.vault_private_key.to_owned();

    let jws = encryption::jws_sign_payload(&payload, basilisk_hs_key_id(), private_key)
        .await
        .change_context(errors::VaultError::RequestEncodingFailed)?;

    let jwe_payload = mk_basilisk_req(jwekey, &jws).await?;

    let body = utils::Encode::<encryption::JweBody>::encode_to_value(&jwe_payload)
        .change_context(errors::VaultError::RequestEncodingFailed)?;
    let mut url = locker.host.to_owned();
    url.push_str("/cards/retrieve");
    let mut request = services::Request::new(services::Method::Post, &url);
    request.add_header(headers::CONTENT_TYPE, "application/json");
    request.set_body(body.to_string());
    Ok(request)
}

pub async fn mk_delete_card_request_hs(
    jwekey: &Jwekey,
    locker: &Locker,
    customer_id: &str,
    merchant_id: &str,
    card_reference: &str,
) -> CustomResult<services::Request, errors::VaultError> {
    let merchant_customer_id = if cfg!(feature = "sandbox") {
        format!("{customer_id}::{merchant_id}")
    } else {
        customer_id.to_owned()
    };
    let card_req_body = CardReqBody {
        merchant_id,
        merchant_customer_id,
        card_reference: card_reference.to_owned(),
    };
    let payload = utils::Encode::<CardReqBody<'_>>::encode_to_vec(&card_req_body)
        .change_context(errors::VaultError::RequestEncodingFailed)?;
    #[cfg(feature = "kms")]
    let private_key = kms::KeyHandler::get_kms_decrypted_key(
        &jwekey.aws_region,
        &jwekey.aws_key_id,
        jwekey.vault_private_key.to_string(),
    )
    .await
    .change_context(errors::VaultError::SaveCardFailed)
    .attach_printable("Error getting private key for signing jws")?;

    #[cfg(not(feature = "kms"))]
    let private_key = jwekey.vault_private_key.to_owned();

    let jws = encryption::jws_sign_payload(&payload, basilisk_hs_key_id(), private_key)
        .await
        .change_context(errors::VaultError::RequestEncodingFailed)?;

    let jwe_payload = mk_basilisk_req(jwekey, &jws).await?;

    let body = utils::Encode::<encryption::JweBody>::encode_to_value(&jwe_payload)
        .change_context(errors::VaultError::RequestEncodingFailed)?;
    let mut url = locker.host.to_owned();
    url.push_str("/cards/delete");
    let mut request = services::Request::new(services::Method::Post, &url);
    request.add_header(headers::CONTENT_TYPE, "application/json");
    request.set_body(body.to_string());
    Ok(request)
}

pub fn get_card_detail(
    pm: &storage::PaymentMethod,
    response: Card,
) -> CustomResult<api::CardDetailFromLocker, errors::VaultError> {
    let card_number = response.card_number;
    let mut last4_digits = card_number.peek().to_owned();
    let card_detail = api::CardDetailFromLocker {
        scheme: pm.scheme.to_owned(),
        issuer_country: pm.issuer_country.clone(),
        last4_digits: Some(last4_digits.split_off(last4_digits.len() - 4)),
        card_number: Some(card_number),
        expiry_month: Some(response.card_exp_month),
        expiry_year: Some(response.card_exp_year),
        card_token: None,
        card_fingerprint: None,
        card_holder_name: None,
    };
    Ok(card_detail)
}

//------------------------------------------------TokenizeService------------------------------------------------
pub fn mk_crud_locker_request(
    locker: &Locker,
    path: &str,
    req: api::TokenizePayloadEncrypted,
) -> CustomResult<services::Request, errors::VaultError> {
    let body = utils::Encode::<api::TokenizePayloadEncrypted>::encode_to_value(&req)
        .change_context(errors::VaultError::RequestEncodingFailed)?;
    let mut url = locker.basilisk_host.to_owned();
    url.push_str(path);
    let mut request = services::Request::new(services::Method::Post, &url);
    request.add_header(headers::X_ROUTER, "test");
    request.add_header(headers::CONTENT_TYPE, "application/json");
    request.set_body(body.to_string());
    Ok(request)
}

pub fn mk_card_value1(
    card_number: String,
    exp_year: String,
    exp_month: String,
    name_on_card: Option<String>,
    nickname: Option<String>,
    card_last_four: Option<String>,
    card_token: Option<String>,
) -> CustomResult<String, errors::VaultError> {
    let value1 = api::TokenizedCardValue1 {
        card_number,
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
