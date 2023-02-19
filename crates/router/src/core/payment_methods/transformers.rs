use error_stack::ResultExt;
use serde::{Deserialize, Serialize};

use crate::{
    configs::settings::Locker,
    core::errors::{self, CustomResult},
    headers,
    pii::{self, prelude::*, Secret},
    services::api as services,
    types::{api, storage},
    utils::{self, OptionExt},
};

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AddCardRequest<'a> {
    pub card_number: Secret<String, pii::CardNumber>,
    pub customer_id: &'a str,
    pub card_exp_month: Secret<String>,
    pub card_exp_year: Secret<String>,
    pub merchant_id: &'a str,
    pub email_address: Option<Secret<String, pii::Email>>,
    pub name_on_card: Option<Secret<String>>,
    pub nickname: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GetCard<'a> {
    merchant_id: &'a str,
    card_id: &'a str,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AddCardResponse {
    pub card_id: String,
    pub external_id: String,
    pub card_fingerprint: Secret<String>,
    pub card_global_fingerprint: Secret<String>,
    #[serde(rename = "merchant_id")]
    pub merchant_id: Option<String>,
    pub card_number: Option<Secret<String, pii::CardNumber>>,
    pub card_exp_year: Option<Secret<String>>,
    pub card_exp_month: Option<Secret<String>>,
    pub name_on_card: Option<Secret<String>>,
    pub nickname: Option<String>,
    pub customer_id: Option<String>,
    pub duplicate: Option<bool>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct GetCardResponse {
    pub card: AddCardResponse,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteCardResponse {
    pub card_id: Option<String>,
    pub external_id: Option<String>,
    pub card_isin: Option<Secret<String>>,
    pub status: String,
}

pub fn mk_add_card_request(
    locker: &Locker,
    card: &api::CardDetail,
    customer_id: &str,
    _req: &api::CreatePaymentMethod,
    locker_id: &str,
    merchant_id: &str,
) -> CustomResult<services::Request, errors::VaultError> {
    let customer_id = if cfg!(feature = "sandbox") {
        format!("{customer_id}::{merchant_id}")
    } else {
        customer_id.to_owned()
    };
    let add_card_req = AddCardRequest {
        card_number: card.card_number.clone(),
        customer_id: &customer_id,
        card_exp_month: card.card_exp_month.clone(),
        card_exp_year: card.card_exp_year.clone(),
        merchant_id: locker_id,
        email_address: Some("dummy@gmail.com".to_string().into()), //
        name_on_card: Some("juspay".to_string().into()),           // [#256]
        nickname: Some("router".to_string()),                      //
    };
    let body = utils::Encode::<AddCardRequest<'_>>::encode(&add_card_req)
        .change_context(errors::VaultError::RequestEncodingFailed)?;
    let mut url = locker.host.to_owned();
    url.push_str("/card/addCard");
    let mut request = services::Request::new(services::Method::Post, &url);
    request.add_header(headers::CONTENT_TYPE, "application/x-www-form-urlencoded");
    request.set_body(body);
    Ok(request)
}

pub fn mk_add_card_response(
    card: api::CardDetail,
    response: AddCardResponse,
    req: api::CreatePaymentMethod,
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
        card_holder_name: None,
    };
    api::PaymentMethodResponse {
        merchant_id: merchant_id.to_owned(),
        customer_id: req.customer_id,
        payment_method_id: response.card_id,
        payment_method: req.payment_method,
        payment_method_type: req.payment_method_type,
        payment_method_issuer: req.payment_method_issuer,
        card: Some(card),
        metadata: req.metadata,
        created: Some(common_utils::date_time::now()),
        payment_method_issuer_code: req.payment_method_issuer_code,
        recurring_enabled: false,           // [#256]
        installment_payment_enabled: false, // #[#256]
        payment_experience: Some(vec![api_models::enums::PaymentExperience::RedirectToUrl]), // [#256]
    }
}

pub fn mk_get_card_request<'a>(
    locker: &Locker,
    locker_id: &'a str,
    card_id: &'a str,
) -> CustomResult<services::Request, errors::VaultError> {
    let get_card_req = GetCard {
        merchant_id: locker_id,
        card_id,
    };

    let body = utils::Encode::<GetCard<'_>>::encode(&get_card_req)
        .change_context(errors::VaultError::RequestEncodingFailed)?;
    let mut url = locker.host.to_owned();
    url.push_str("/card/getCard");
    let mut request = services::Request::new(services::Method::Post, &url);
    request.add_header(headers::CONTENT_TYPE, "application/x-www-form-urlencoded");
    request.set_body(body);
    Ok(request)
}

pub fn mk_delete_card_request<'a>(
    locker: &Locker,
    merchant_id: &'a str,
    card_id: &'a str,
) -> CustomResult<services::Request, errors::VaultError> {
    let delete_card_req = GetCard {
        merchant_id,
        card_id,
    };
    let body = utils::Encode::<GetCard<'_>>::encode(&delete_card_req)
        .change_context(errors::VaultError::RequestEncodingFailed)?;
    let mut url = locker.host.to_owned();
    url.push_str("/card/deleteCard");
    let mut request = services::Request::new(services::Method::Post, &url);
    request.add_header(headers::X_ROUTER, "test");
    request.add_header(headers::CONTENT_TYPE, "application/x-www-form-urlencoded");
    //request.add_content_type(Content::FORMURLENCODED);
    request.set_body(body);
    Ok(request)
}

pub fn get_card_detail(
    pm: &storage::PaymentMethod,
    response: AddCardResponse,
) -> CustomResult<api::CardDetailFromLocker, errors::VaultError> {
    let card_number = response
        .card_number
        .get_required_value("card_number")
        .change_context(errors::VaultError::FetchCardFailed)?;
    let mut last4_digits = card_number.peek().to_owned();
    let card_detail = api::CardDetailFromLocker {
        scheme: pm.scheme.clone(),
        issuer_country: pm.issuer_country.clone(),
        last4_digits: Some(last4_digits.split_off(last4_digits.len() - 4)),
        card_number: Some(card_number),
        expiry_month: response.card_exp_month,
        expiry_year: response.card_exp_year,
        card_token: Some(response.external_id.into()), //TODO ?
        card_fingerprint: Some(response.card_fingerprint),
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
