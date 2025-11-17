use std::fmt::Debug;

use crate::{
    services,
    configs::settings,
    core::{errors, cards::mk_basilisk_req},
    headers,
};
#[cfg(all(feature = "v1", feature = "payouts"))]
use api_models::payouts::Bank as BankPayout;
use api_models::{
    enums as api_enums,
    payment_methods::{self as api, Card},
};
#[cfg(feature = "v2")]
use common_utils::encryption;
use common_utils::errors::CustomResult;
use common_utils::ext_traits::Encode;
use common_utils::request::RequestContent;
use common_utils::{
    encryption, id_type,
};
use error_stack::ResultExt;
use hyperswitch_domain_models::ext_traits::OptionExt;
use masking::{PeekInterface, Secret};
use router_env::RequestId;
use serde::{Deserialize, Serialize};

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
pub struct GetCardResponse {
    pub card: AddCardResponse,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CardReqBody {
    pub merchant_id: id_type::MerchantId,
    pub merchant_customer_id: id_type::CustomerId,
    pub card_reference: String,
}

#[cfg(feature = "v2")]
#[derive(Debug, Deserialize, Serialize)]
pub struct CardReqBodyV2 {
    pub merchant_id: id_type::MerchantId,
    pub merchant_customer_id: String, // Not changing this as it might lead to api contract failure
    pub card_reference: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AddCardResponse {
    pub card_id: String,
    pub external_id: String,
    pub card_fingerprint: Secret<String>,
    pub card_global_fingerprint: Secret<String>,
    #[serde(rename = "merchant_id")]
    pub merchant_id: Option<id_type::MerchantId>,
    pub card_number: Option<cards::CardNumber>,
    pub card_exp_year: Option<Secret<String>>,
    pub card_exp_month: Option<Secret<String>>,
    pub name_on_card: Option<Secret<String>>,
    pub nickname: Option<String>,
    pub customer_id: Option<id_type::CustomerId>,
    pub duplicate: Option<bool>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct StoreCardReq {
    pub merchant_id: id_type::MerchantId,
    pub merchant_customer_id: id_type::CustomerId,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub requestor_card_reference: Option<String>,
    pub card: Card,
    pub ttl: i64,
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
pub struct StoreGenericReq {
    pub merchant_id: id_type::MerchantId,
    pub merchant_customer_id: id_type::CustomerId,
    #[serde(rename = "enc_card_data")]
    pub enc_data: String,
    pub ttl: i64,
}

#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum StoreLockerReq {
    LockerCard(StoreCardReq),
    LockerGeneric(StoreGenericReq),
}

impl StoreLockerReq {
    pub fn update_requestor_card_reference(&mut self, card_reference: Option<String>) {
        match self {
            Self::LockerCard(c) => c.requestor_card_reference = card_reference,
            Self::LockerGeneric(_) => (),
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub async fn mk_get_card_request_hs(
    jwekey: &settings::Jwekey,
    locker: &settings::Locker,
    customer_id: &id_type::CustomerId,
    merchant_id: &id_type::MerchantId,
    card_reference: &str,
    locker_choice: Option<api_enums::LockerChoice>,
    tenant_id: id_type::TenantId,
    request_id: Option<RequestId>,
) -> CustomResult<services::Request, errors::VaultError> {
    let merchant_customer_id = customer_id.to_owned();
    let card_req_body = CardReqBody {
        merchant_id: merchant_id.to_owned(),
        merchant_customer_id,
        card_reference: card_reference.to_owned(),
    };
    let payload = card_req_body
        .encode_to_vec()
        .change_context(errors::VaultError::RequestEncodingFailed)?;

    let private_key = jwekey.vault_private_key.peek().as_bytes();

    let jws = encryption::jws_sign_payload(&payload, &locker.locker_signing_key_id, private_key)
        .await
        .change_context(errors::VaultError::RequestEncodingFailed)?;

    let target_locker = locker_choice.unwrap_or(api_enums::LockerChoice::HyperswitchCardVault);

    let jwe_payload = mk_basilisk_req(jwekey, &jws, target_locker).await?;
    let mut url = match target_locker {
        api_enums::LockerChoice::HyperswitchCardVault => locker.host.to_owned(),
    };
    url.push_str("/cards/retrieve");
    let mut request = services::Request::new(services::Method::Post, &url);
    request.add_header(headers::CONTENT_TYPE, "application/json".into());
    request.add_header(
        headers::X_TENANT_ID,
        tenant_id.get_string_repr().to_owned().into(),
    );
    if let Some(req_id) = request_id {
        request.add_header(headers::X_REQUEST_ID, req_id.to_string().into());
    }

    request.set_body(RequestContent::Json(Box::new(jwe_payload)));
    Ok(request)
}
pub fn mk_get_card_response(card: GetCardResponse) -> errors::PmResult<Card> {
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

#[cfg(feature = "v1")]
pub fn mk_add_card_response_hs(
    card: api::CardDetail,
    card_reference: String,
    req: api::PaymentMethodCreate,
    merchant_id: &id_type::MerchantId,
) -> api::PaymentMethodResponse {
    let card_number = card.card_number.clone();
    let last4_digits = card_number.get_last4();
    let card_isin = card_number.get_card_isin();

    let card = api::CardDetailFromLocker {
        scheme: card
            .card_network
            .clone()
            .map(|card_network| card_network.to_string()),
        last4_digits: Some(last4_digits),
        issuer_country: card.card_issuing_country,
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
        #[cfg(feature = "payouts")]
        bank_transfer: None,
        card: Some(card),
        metadata: req.metadata,
        created: Some(common_utils::date_time::now()),
        recurring_enabled: Some(false),           // [#256]
        installment_payment_enabled: Some(false), // #[#256]
        payment_experience: Some(vec![api_models::enums::PaymentExperience::RedirectToUrl]),
        last_used_at: Some(common_utils::date_time::now()), // [#256]
        client_secret: req.client_secret,
    }
}

#[cfg(feature = "v2")]
pub fn mk_add_card_response_hs(
    card: api::CardDetail,
    card_reference: String,
    req: api::PaymentMethodCreate,
    merchant_id: &id_type::MerchantId,
) -> api::PaymentMethodResponse {
    todo!()
}

pub async fn mk_add_locker_request_hs(
    jwekey: &settings::Jwekey,
    locker: &settings::Locker,
    payload: &StoreLockerReq,
    locker_choice: api_enums::LockerChoice,
    tenant_id: id_type::TenantId,
    request_id: Option<RequestId>,
) -> CustomResult<services::Request, errors::VaultError> {
    let payload = payload
        .encode_to_vec()
        .change_context(errors::VaultError::RequestEncodingFailed)?;

    let private_key = jwekey.vault_private_key.peek().as_bytes();

    let jws = encryption::jws_sign_payload(&payload, &locker.locker_signing_key_id, private_key)
        .await
        .change_context(errors::VaultError::RequestEncodingFailed)?;

    let jwe_payload = mk_basilisk_req(jwekey, &jws, locker_choice).await?;
    let mut url = match locker_choice {
        api_enums::LockerChoice::HyperswitchCardVault => locker.host.to_owned(),
    };
    url.push_str("/cards/add");
    let mut request = services::Request::new(services::Method::Post, &url);
    request.add_header(headers::CONTENT_TYPE, "application/json".into());
    request.add_header(
        headers::X_TENANT_ID,
        tenant_id.get_string_repr().to_owned().into(),
    );
    if let Some(req_id) = request_id {
        request.add_header(headers::X_REQUEST_ID, req_id.to_string().into());
    }
    request.set_body(RequestContent::Json(Box::new(jwe_payload)));
    Ok(request)
}

#[cfg(all(feature = "v1", feature = "payouts"))]
pub fn mk_add_bank_response_hs(
    bank: BankPayout,
    bank_reference: String,
    req: api::PaymentMethodCreate,
    merchant_id: &id_type::MerchantId,
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
        recurring_enabled: Some(false),           // [#256]
        installment_payment_enabled: Some(false), // #[#256]
        payment_experience: Some(vec![api_models::enums::PaymentExperience::RedirectToUrl]),
        last_used_at: Some(common_utils::date_time::now()),
        client_secret: None,
    }
}

#[cfg(all(feature = "v2", feature = "payouts"))]
pub fn mk_add_bank_response_hs(
    _bank: BankPayout,
    _bank_reference: String,
    _req: api::PaymentMethodCreate,
    _merchant_id: &id_type::MerchantId,
) -> api::PaymentMethodResponse {
    todo!()
}