pub use ::payment_methods::controller::{DataDuplicationCheck, DeleteCardResp};
use api_models::payment_methods::Card;
#[cfg(feature = "v2")]
use api_models::{enums as api_enums, payment_methods::PaymentMethodResponseItem};
use common_enums::CardNetwork;
#[cfg(feature = "v1")]
use common_utils::{crypto::Encryptable, request::Headers, types::keymanager::KeyManagerState};
use common_utils::{
    ext_traits::{AsyncExt, Encode, StringExt},
    id_type,
    pii::{Email, SecretSerdeValue},
    request::RequestContent,
};
use error_stack::ResultExt;
#[cfg(feature = "v2")]
use hyperswitch_domain_models::{payment_method_data, sdk_auth::SdkAuthorization};
use josekit::jwe;
#[cfg(feature = "v1")]
use masking::Mask;
use masking::{ExposeInterface, PeekInterface};
#[cfg(feature = "v1")]
use payment_methods::client::{
    self as pm_client,
    create::{CreatePaymentMethodResponse, CreatePaymentMethodV1Request},
    retrieve::{RetrievePaymentMethodResponse, RetrievePaymentMethodV1Request},
    UpdatePaymentMethod, UpdatePaymentMethodV1Payload, UpdatePaymentMethodV1Request,
};
use router_env::RequestId;
#[cfg(feature = "v1")]
use router_env::{logger, RequestIdentifier};
use serde::{Deserialize, Serialize};

use crate::{
    configs::settings,
    core::{
        errors::{self, CustomResult},
        payment_methods::cards::{call_vault_service, create_encrypted_data},
    },
    headers,
    pii::Secret,
    routes,
    services::{api as services, encryption, EncryptionAlgorithm},
    types::{api, domain},
    utils::OptionExt,
};
#[cfg(feature = "v2")]
use crate::{
    consts,
    types::{payment_methods as pm_types, transformers},
};

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
pub struct StoreGenericReq {
    pub merchant_id: id_type::MerchantId,
    pub merchant_customer_id: id_type::CustomerId,
    #[serde(rename = "enc_card_data")]
    pub enc_data: String,
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
    /// Additional metadata containing PAR, UPT, and other tokens   
    pub metadata: Option<SecretSerdeValue>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AddCardRequest {
    pub card_number: cards::CardNumber,
    pub customer_id: id_type::CustomerId,
    pub card_exp_month: Secret<String>,
    pub card_exp_year: Secret<String>,
    pub merchant_id: id_type::MerchantId,
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
    pub merchant_id: Option<id_type::MerchantId>,
    pub card_number: Option<cards::CardNumber>,
    pub card_exp_year: Option<Secret<String>>,
    pub card_exp_month: Option<Secret<String>>,
    pub name_on_card: Option<Secret<String>>,
    pub nickname: Option<String>,
    pub customer_id: Option<id_type::CustomerId>,
    pub duplicate: Option<bool>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AddPaymentMethodResponse {
    pub payment_method_id: String,
    pub external_id: String,
    #[serde(rename = "merchant_id")]
    pub merchant_id: Option<id_type::MerchantId>,
    pub nickname: Option<String>,
    pub customer_id: Option<id_type::CustomerId>,
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
    jwekey: &settings::Jwekey,
    jwe_body: encryption::JweBody,
    decryption_scheme: settings::DecryptionScheme,
) -> CustomResult<String, errors::VaultError> {
    let public_key = jwekey.vault_encryption_key.peek().as_bytes();

    let private_key = jwekey.vault_private_key.peek().as_bytes();

    let jwt = get_dotted_jwe(jwe_body);
    let alg = match decryption_scheme {
        settings::DecryptionScheme::RsaOaep => jwe::RSA_OAEP,
        settings::DecryptionScheme::RsaOaep256 => jwe::RSA_OAEP_256,
    };

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

pub async fn get_decrypted_vault_response_payload(
    jwekey: &settings::Jwekey,
    jwe_body: encryption::JweBody,
    decryption_scheme: settings::DecryptionScheme,
) -> CustomResult<String, errors::VaultError> {
    let public_key = jwekey.vault_encryption_key.peek().as_bytes();

    let private_key = jwekey.vault_private_key.peek().as_bytes();

    let jwt = get_dotted_jwe(jwe_body);
    let alg = match decryption_scheme {
        settings::DecryptionScheme::RsaOaep => jwe::RSA_OAEP,
        settings::DecryptionScheme::RsaOaep256 => jwe::RSA_OAEP_256,
    };

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

pub async fn create_jwe_body_for_vault(
    jwekey: &settings::Jwekey,
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

    let jws_body =
        generate_jws_body(jws_payload).ok_or(errors::VaultError::RequestEncryptionFailed)?;

    let payload = jws_body
        .encode_to_vec()
        .change_context(errors::VaultError::RequestEncodingFailed)?;

    let public_key = jwekey.vault_encryption_key.peek().as_bytes();

    let jwe_encrypted =
        encryption::encrypt_jwe(&payload, public_key, EncryptionAlgorithm::A256GCM, None)
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

    let jwe_body =
        generate_jwe_body(jwe_payload).ok_or(errors::VaultError::RequestEncodingFailed)?;

    Ok(jwe_body)
}

pub async fn mk_vault_req(
    jwekey: &settings::Jwekey,
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

    let payload = jws_body
        .encode_to_vec()
        .change_context(errors::VaultError::SaveCardFailed)?;

    let public_key = jwekey.vault_encryption_key.peek().as_bytes();

    let jwe_encrypted =
        encryption::encrypt_jwe(&payload, public_key, EncryptionAlgorithm::A256GCM, None)
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

pub async fn call_vault_api<'a, Req, Res>(
    state: &routes::SessionState,
    jwekey: &settings::Jwekey,
    locker: &settings::Locker,
    payload: &'a Req,
    endpoint_path: &str,
    tenant_id: id_type::TenantId,
    request_id: Option<RequestId>,
) -> CustomResult<Res, errors::VaultError>
where
    Req: Encode<'a> + Serialize,
    Res: serde::de::DeserializeOwned,
{
    let encoded_payload = payload
        .encode_to_vec()
        .change_context(errors::VaultError::RequestEncodingFailed)?;

    let private_key = jwekey.vault_private_key.peek().as_bytes();
    let jws =
        encryption::jws_sign_payload(&encoded_payload, &locker.locker_signing_key_id, private_key)
            .await
            .change_context(errors::VaultError::RequestEncodingFailed)?;

    let jwe_payload = mk_vault_req(jwekey, &jws).await?;

    let url = locker.get_host(endpoint_path);

    let mut request = services::Request::new(services::Method::Post, &url);
    request.add_header(headers::CONTENT_TYPE, "application/json".into());
    request.add_header(headers::X_TENANT_ID, tenant_id.get_string_repr().into());

    if let Some(req_id) = request_id {
        request.add_header(headers::X_REQUEST_ID, req_id.to_string().into());
    }

    request.set_body(RequestContent::Json(Box::new(jwe_payload)));

    let response = call_vault_service::<Res>(state, request, endpoint_path)
        .await
        .change_context(errors::VaultError::VaultAPIError)?;

    Ok(response)
}

#[cfg(all(feature = "v1", feature = "payouts"))]
pub fn mk_add_bank_response_hs(
    bank: api::BankPayout,
    bank_reference: String,
    req: api::PaymentMethodCreate,
    merchant_id: &id_type::MerchantId,
) -> domain::PaymentMethodResponse {
    domain::PaymentMethodResponse {
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
        locker_fingerprint_id: None,
    }
}

#[cfg(feature = "v1")]
pub fn mk_add_bank_debit_response_hs(
    bank_reference: String,
    req: api::PaymentMethodCreate,
    merchant_id: &id_type::MerchantId,
    locker_fingerprint_id: String,
) -> domain::PaymentMethodResponse {
    domain::PaymentMethodResponse {
        merchant_id: merchant_id.to_owned(),
        customer_id: req.customer_id.to_owned(),
        payment_method_id: bank_reference,
        payment_method: req.payment_method,
        payment_method_type: req.payment_method_type,
        bank_transfer: None,
        card: None,
        metadata: req.metadata,
        created: Some(common_utils::date_time::now()),
        recurring_enabled: Some(false),           // [#256]
        installment_payment_enabled: Some(false), // #[#256]
        payment_experience: Some(vec![api_models::enums::PaymentExperience::RedirectToUrl]),
        last_used_at: Some(common_utils::date_time::now()),
        client_secret: None,
        locker_fingerprint_id: Some(locker_fingerprint_id),
    }
}

#[cfg(all(feature = "v2", feature = "payouts"))]
pub fn mk_add_bank_response_hs(
    _bank: api::BankPayout,
    _bank_reference: String,
    _req: api::PaymentMethodCreate,
    _merchant_id: &id_type::MerchantId,
) -> api::PaymentMethodResponse {
    todo!()
}

#[cfg(feature = "v1")]
pub fn mk_add_card_response_hs(
    card: api::CardDetail,
    card_reference: String,
    req: api::PaymentMethodCreate,
    merchant_id: &id_type::MerchantId,
) -> domain::PaymentMethodResponse {
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
        issuer_country_code: card.card_issuing_country_code,
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
    domain::PaymentMethodResponse {
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
        locker_fingerprint_id: None,
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

#[cfg(feature = "v2")]
pub fn generate_pm_vaulting_req_from_update_request(
    pm_create: domain::PaymentMethodVaultingData,
    pm_update: api::PaymentMethodUpdateData,
) -> domain::PaymentMethodVaultingData {
    match (pm_create, pm_update) {
        (
            domain::PaymentMethodVaultingData::Card(card_create),
            api::PaymentMethodUpdateData::Card(update_card),
        ) => domain::PaymentMethodVaultingData::Card(api::CardDetail {
            card_number: card_create.card_number,
            card_exp_month: card_create.card_exp_month,
            card_exp_year: card_create.card_exp_year,
            card_issuing_country: card_create.card_issuing_country,
            card_network: card_create.card_network,
            card_issuer: card_create.card_issuer,
            card_type: card_create.card_type,
            card_holder_name: update_card
                .card_holder_name
                .or(card_create.card_holder_name),
            nick_name: update_card.nick_name.or(card_create.nick_name),
            card_cvc: None,
        }),
        _ => todo!(), //todo! - since support for network tokenization is not added PaymentMethodUpdateData. should be handled later.
    }
}

#[cfg(feature = "v2")]
#[allow(clippy::too_many_arguments)]
pub fn generate_payment_method_response(
    payment_method: &domain::PaymentMethod,
    single_use_token: &Option<payment_method_data::SingleUsePaymentMethodToken>,
    storage_type: common_enums::StorageType,
    card_cvc_token_storage: Option<api_models::payment_methods::CardCVCTokenStorageDetails>,
    customer_id: Option<id_type::GlobalCustomerId>,
    raw_payment_method_data: Option<api_models::payment_methods::RawPaymentMethodData>,
    billing: Option<api::Address>,
    acknowledgement_status: Option<common_enums::AcknowledgementStatus>,
) -> errors::RouterResult<api::PaymentMethodResponse> {
    let pmd = payment_method
        .payment_method_data
        .clone()
        .map(|data| data.into_inner())
        .and_then(|data| match data {
            api::PaymentMethodsData::Card(card) => {
                Some(api::PaymentMethodResponseData::Card(card.into()))
            }
            _ => None,
        });
    let mut connector_tokens = payment_method
        .connector_mandate_details
        .as_ref()
        .and_then(|connector_token_details| connector_token_details.payments.clone())
        .map(|payment_token_details| payment_token_details.0)
        .map(|payment_token_details| {
            payment_token_details
                .into_iter()
                .map(transformers::ForeignFrom::foreign_from)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    if let Some(token) = single_use_token {
        let connector_token_single_use = transformers::ForeignFrom::foreign_from(token);
        connector_tokens.push(connector_token_single_use);
    }
    let connector_tokens = if connector_tokens.is_empty() {
        None
    } else {
        Some(connector_tokens)
    };

    let network_token_pmd = payment_method
        .network_token_payment_method_data
        .clone()
        .map(|data| data.into_inner())
        .and_then(|data| match data {
            domain::PaymentMethodsData::NetworkToken(token) => {
                Some(api::NetworkTokenDetailsPaymentMethod::from(token))
            }
            _ => None,
        });

    let network_token = network_token_pmd.map(|pmd| api::NetworkTokenResponse {
        payment_method_data: pmd,
    });

    let resp = api::PaymentMethodResponse {
        merchant_id: payment_method.merchant_id.to_owned(),
        customer_id,
        id: payment_method.id.to_owned(),
        payment_method_type: payment_method.get_payment_method_type(),
        payment_method_subtype: payment_method.get_payment_method_subtype(),
        created: Some(payment_method.created_at),
        recurring_enabled: Some(false),
        last_used_at: Some(payment_method.last_used_at),
        payment_method_data: pmd,
        connector_tokens,
        network_token,
        storage_type,
        card_cvc_token_storage,
        network_transaction_id: payment_method
            .network_transaction_id
            .clone()
            .map(Secret::new),
        raw_payment_method_data,
        billing,
        acknowledgement_status,
    };

    Ok(resp)
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

// Need to fix this once we start moving to v2 completion
#[cfg(feature = "v2")]
#[allow(clippy::too_many_arguments)]
pub async fn mk_delete_card_request_hs_by_id(
    state: &routes::SessionState,
    jwekey: &settings::Jwekey,
    locker: &settings::Locker,
    id: &String,
    merchant_id: &id_type::MerchantId,
    card_reference: &str,
    tenant_id: id_type::TenantId,
    request_id: Option<RequestId>,
) -> CustomResult<DeleteCardResp, errors::VaultError> {
    let card_req_body = CardReqBodyV2 {
        merchant_id: merchant_id.to_owned(),
        merchant_customer_id: id.to_owned(),
        card_reference: card_reference.to_owned(),
    };

    call_vault_api(
        state,
        jwekey,
        locker,
        &card_req_body,
        consts::LOCKER_DELETE_CARD_PATH,
        tenant_id,
        request_id,
    )
    .await
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

#[cfg(feature = "v1")]
pub fn get_card_detail(
    pm: &domain::PaymentMethod,
    response: Card,
) -> CustomResult<api::CardDetailFromLocker, errors::VaultError> {
    let card_number = response.card_number;
    let last4_digits = card_number.clone().get_last4();
    //fetch form card bin

    let card_detail = api::CardDetailFromLocker {
        scheme: pm.scheme.to_owned(),
        issuer_country: pm.issuer_country.clone(),
        last4_digits: Some(last4_digits),
        card_number: Some(card_number),
        expiry_month: Some(response.card_exp_month),
        expiry_year: Some(response.card_exp_year),
        card_token: None,
        card_fingerprint: None,
        card_holder_name: response.name_on_card,
        nick_name: response.nick_name.map(Secret::new),
        card_isin: None,
        card_issuer: None,
        issuer_country_code: None,
        card_network: None,
        card_type: None,
        saved_to_locker: true,
    };
    Ok(card_detail)
}

#[cfg(feature = "v2")]
pub fn get_card_detail(
    _pm: &domain::PaymentMethod,
    response: Card,
) -> CustomResult<api::CardDetailFromLocker, errors::VaultError> {
    let card_number = response.card_number;
    let last4_digits = card_number.clone().get_last4();
    //fetch form card bin

    let card_detail = api::CardDetailFromLocker {
        issuer_country: None,
        last4_digits: Some(last4_digits),
        card_number: Some(card_number),
        expiry_month: Some(response.card_exp_month),
        expiry_year: Some(response.card_exp_year),
        card_fingerprint: None,
        card_holder_name: response.name_on_card,
        nick_name: response.nick_name.map(Secret::new),
        card_isin: None,
        card_issuer: None,
        card_network: None,
        card_type: None,
        saved_to_locker: true,
    };
    Ok(card_detail)
}

//------------------------------------------------TokenizeService------------------------------------------------
#[allow(clippy::too_many_arguments)]
pub fn mk_card_value1(
    card_number: cards::CardNumber,
    exp_year: String,
    exp_month: String,
    name_on_card: Option<String>,
    nickname: Option<String>,
    card_last_four: Option<String>,
    card_token: Option<String>,
    card_network: Option<CardNetwork>,
) -> CustomResult<String, errors::VaultError> {
    let value1 = api::TokenizedCardValue1 {
        card_number: card_number.peek().clone(),
        exp_year,
        exp_month,
        name_on_card,
        nickname,
        card_last_four,
        card_token,
        card_network,
    };
    let value1_req = value1
        .encode_to_string_of_json()
        .change_context(errors::VaultError::FetchCardFailed)?;
    Ok(value1_req)
}

pub fn mk_card_value2(
    card_security_code: Option<String>,
    card_fingerprint: Option<String>,
    external_id: Option<String>,
    customer_id: Option<id_type::CustomerId>,
    payment_method_id: Option<String>,
) -> CustomResult<String, errors::VaultError> {
    let value2 = api::TokenizedCardValue2 {
        card_security_code,
        card_fingerprint,
        external_id,
        customer_id,
        payment_method_id,
    };
    let value2_req = value2
        .encode_to_string_of_json()
        .change_context(errors::VaultError::FetchCardFailed)?;
    Ok(value2_req)
}

#[cfg(feature = "v2")]
impl transformers::ForeignTryFrom<(domain::PaymentMethod, String)>
    for api::CustomerPaymentMethodResponseItem
{
    type Error = error_stack::Report<errors::ValidationError>;

    fn foreign_try_from(
        (item, payment_token): (domain::PaymentMethod, String),
    ) -> Result<Self, Self::Error> {
        // For payment methods that are active we should always have the payment method subtype
        let payment_method_subtype =
            item.payment_method_subtype
                .ok_or(errors::ValidationError::MissingRequiredField {
                    field_name: "payment_method_subtype".to_string(),
                })?;

        // For payment methods that are active we should always have the payment method type
        let payment_method_type =
            item.payment_method_type
                .ok_or(errors::ValidationError::MissingRequiredField {
                    field_name: "payment_method_type".to_string(),
                })?;

        let payment_method_data = item
            .payment_method_data
            .map(|payment_method_data| payment_method_data.into_inner())
            .map(|payment_method_data| match payment_method_data {
                api_models::payment_methods::PaymentMethodsData::Card(
                    card_details_payment_method,
                ) => {
                    let card_details = api::CardDetailFromLocker::from(card_details_payment_method);
                    api_models::payment_methods::PaymentMethodListData::Card(card_details)
                }
                api_models::payment_methods::PaymentMethodsData::BankDetails(..) => todo!(),
                api_models::payment_methods::PaymentMethodsData::BankDebit(..) => todo!(),
                api_models::payment_methods::PaymentMethodsData::WalletDetails(..) => {
                    todo!()
                }
            });

        let payment_method_billing = item
            .payment_method_billing_address
            .clone()
            .map(|billing| billing.into_inner())
            .map(From::from);

        // TODO: check how we can get this field
        let recurring_enabled = true;

        Ok(Self {
            customer_id: item
                .customer_id
                .get_required_value("GlobalCustomerId")
                .change_context(errors::ValidationError::MissingRequiredField {
                    field_name: "customer_id".to_string(),
                })?,
            payment_method_type,
            payment_method_subtype,
            created: item.created_at,
            last_used_at: item.last_used_at,
            recurring_enabled,
            payment_method_data,
            bank: None,
            requires_cvv: true,
            is_default: false,
            billing: payment_method_billing,
            payment_method_token: payment_token,
        })
    }
}

#[cfg(feature = "v2")]
impl transformers::ForeignTryFrom<domain::PaymentMethod> for PaymentMethodResponseItem {
    type Error = error_stack::Report<errors::ValidationError>;

    fn foreign_try_from(item: domain::PaymentMethod) -> Result<Self, Self::Error> {
        // For payment methods that are active we should always have the payment method subtype
        let payment_method_subtype =
            item.payment_method_subtype
                .ok_or(errors::ValidationError::MissingRequiredField {
                    field_name: "payment_method_subtype".to_string(),
                })?;

        // For payment methods that are active we should always have the payment method type
        let payment_method_type =
            item.payment_method_type
                .ok_or(errors::ValidationError::MissingRequiredField {
                    field_name: "payment_method_type".to_string(),
                })?;

        let payment_method_data = item
            .payment_method_data
            .map(|payment_method_data| payment_method_data.into_inner())
            .map(|payment_method_data| match payment_method_data {
                api_models::payment_methods::PaymentMethodsData::Card(
                    card_details_payment_method,
                ) => {
                    let card_details = api::CardDetailFromLocker::from(card_details_payment_method);
                    api_models::payment_methods::PaymentMethodListData::Card(card_details)
                }
                api_models::payment_methods::PaymentMethodsData::BankDetails(..) => todo!(),
                api_models::payment_methods::PaymentMethodsData::BankDebit(..) => todo!(),
                api_models::payment_methods::PaymentMethodsData::WalletDetails(..) => {
                    todo!()
                }
            });

        let payment_method_billing = item
            .payment_method_billing_address
            .clone()
            .map(|billing| billing.into_inner())
            .map(From::from);

        let network_token_pmd = item
            .network_token_payment_method_data
            .clone()
            .map(|data| data.into_inner())
            .and_then(|data| match data {
                domain::PaymentMethodsData::NetworkToken(token) => {
                    Some(api::NetworkTokenDetailsPaymentMethod::from(token))
                }
                _ => None,
            });

        let network_token_resp = network_token_pmd.map(|pmd| api::NetworkTokenResponse {
            payment_method_data: pmd,
        });

        // TODO: check how we can get this field
        let recurring_enabled = Some(true);

        let psp_tokenization_enabled = item.connector_mandate_details.and_then(|details| {
            details.payments.map(|payments| {
                payments.values().any(|connector_token_reference| {
                    connector_token_reference.connector_token_status
                        == api_enums::ConnectorTokenStatus::Active
                })
            })
        });

        Ok(Self {
            id: item.id,
            customer_id: item
                .customer_id
                .get_required_value("GlobalCustomerId")
                .change_context(errors::ValidationError::MissingRequiredField {
                    field_name: "customer_id".to_string(),
                })?,
            payment_method_type,
            payment_method_subtype,
            created: item.created_at,
            last_used_at: item.last_used_at,
            recurring_enabled,
            payment_method_data,
            bank: None,
            requires_cvv: true,
            is_default: false,
            billing: payment_method_billing,
            network_tokenization: network_token_resp,
            psp_tokenization_enabled: psp_tokenization_enabled.unwrap_or(false),
        })
    }
}

#[cfg(feature = "v2")]
#[allow(clippy::too_many_arguments)]
pub fn generate_payment_method_session_response(
    payment_method_session: hyperswitch_domain_models::payment_methods::PaymentMethodSession,
    client_secret: Secret<String>,
    sdk_authorization: Option<hyperswitch_domain_models::sdk_auth::SdkAuthorization>,
    associated_payment: Option<api_models::payments::PaymentsResponse>,
    tokenization_service_response: Option<api_models::tokenization::GenericTokenizationResponse>,
    storage_type: common_enums::StorageType,
    card_cvc_token_storage: Option<api_models::payment_methods::CardCVCTokenStorageDetails>,
    payment_method_data: Option<api_models::payment_methods::PaymentMethodResponseData>,
) -> api_models::payment_methods::PaymentMethodSessionResponse {
    let next_action = associated_payment
        .as_ref()
        .and_then(|payment| payment.next_action.clone());

    let authentication_details =
        associated_payment.map(
            |payment| api_models::payment_methods::AuthenticationDetails {
                status: payment.status,
                error: payment.error,
            },
        );

    let token_id = tokenization_service_response
        .as_ref()
        .map(|tokenization_service_response| tokenization_service_response.id.clone());

    let sdk_authorization = sdk_authorization.and_then(|auth| auth.encode().ok());

    api_models::payment_methods::PaymentMethodSessionResponse {
        id: payment_method_session.id,
        customer_id: payment_method_session.customer_id,
        billing: payment_method_session
            .billing
            .map(|address| address.into_inner())
            .map(From::from),
        psp_tokenization: payment_method_session.psp_tokenization,
        network_tokenization: payment_method_session.network_tokenization,
        tokenization_data: payment_method_session.tokenization_data,
        expires_at: payment_method_session.expires_at,
        client_secret,
        next_action,
        return_url: payment_method_session.return_url,
        associated_payment_methods: payment_method_session.associated_payment_methods,
        authentication_details,
        associated_token_id: token_id,
        storage_type,
        card_cvc_token_storage,
        payment_method_data,
        sdk_authorization,
        keep_alive: payment_method_session.keep_alive,
    }
}

#[cfg(feature = "v2")]
impl transformers::ForeignFrom<api_models::payment_methods::ConnectorTokenDetails>
    for hyperswitch_domain_models::mandates::ConnectorTokenReferenceRecord
{
    fn foreign_from(item: api_models::payment_methods::ConnectorTokenDetails) -> Self {
        let api_models::payment_methods::ConnectorTokenDetails {
            status,
            connector_token_request_reference_id,
            original_payment_authorized_amount,
            original_payment_authorized_currency,
            metadata,
            token,
            ..
        } = item;

        Self {
            connector_token: token.expose().clone(),
            // TODO: check why do we need this field
            payment_method_subtype: None,
            original_payment_authorized_amount,
            original_payment_authorized_currency,
            metadata,
            connector_token_status: status,
            connector_token_request_reference_id,
        }
    }
}

#[cfg(feature = "v2")]
impl
    transformers::ForeignFrom<(
        id_type::MerchantConnectorAccountId,
        hyperswitch_domain_models::mandates::ConnectorTokenReferenceRecord,
    )> for api_models::payment_methods::ConnectorTokenDetails
{
    fn foreign_from(
        (connector_id, mandate_reference_record): (
            id_type::MerchantConnectorAccountId,
            hyperswitch_domain_models::mandates::ConnectorTokenReferenceRecord,
        ),
    ) -> Self {
        let hyperswitch_domain_models::mandates::ConnectorTokenReferenceRecord {
            connector_token_request_reference_id,
            original_payment_authorized_amount,
            original_payment_authorized_currency,
            metadata,
            connector_token,
            connector_token_status,
            ..
        } = mandate_reference_record;

        Self {
            connector_id,
            status: connector_token_status,
            connector_token_request_reference_id,
            original_payment_authorized_amount,
            original_payment_authorized_currency,
            metadata,
            token: Secret::new(connector_token),
            // Token that is derived from payments mandate reference will always be multi use token
            token_type: common_enums::TokenizationType::MultiUse,
        }
    }
}

#[cfg(feature = "v2")]
impl transformers::ForeignFrom<&payment_method_data::SingleUsePaymentMethodToken>
    for api_models::payment_methods::ConnectorTokenDetails
{
    fn foreign_from(token: &payment_method_data::SingleUsePaymentMethodToken) -> Self {
        Self {
            connector_id: token.clone().merchant_connector_id,
            token_type: common_enums::TokenizationType::SingleUse,
            status: api_enums::ConnectorTokenStatus::Active,
            connector_token_request_reference_id: None,
            original_payment_authorized_amount: None,
            original_payment_authorized_currency: None,
            metadata: None,
            token: token.clone().token,
        }
    }
}

#[cfg(feature = "v1")]
pub async fn call_modular_payment_method_update(
    state: &routes::SessionState,
    processor_merchant_id: &id_type::MerchantId,
    profile_id: &id_type::ProfileId,
    payment_method_id: &str,
    payload: UpdatePaymentMethodV1Payload,
) -> CustomResult<(), ::payment_methods::errors::ModularPaymentMethodError> {
    let mut parent_headers = Headers::new();
    parent_headers.insert((
        headers::X_PROFILE_ID.to_string(),
        profile_id.get_string_repr().to_string().into(),
    ));
    parent_headers.insert((
        headers::X_MERCHANT_ID.to_string(),
        processor_merchant_id.get_string_repr().to_string().into(),
    ));
    parent_headers.insert((
        headers::X_INTERNAL_API_KEY.to_string(),
        state
            .conf
            .internal_merchant_id_profile_id_auth
            .internal_api_key
            .clone()
            .expose()
            .to_string()
            .into_masked(),
    ));
    let trace = RequestIdentifier::new(&state.conf.trace_header.header_name)
        .use_incoming_id(state.conf.trace_header.id_reuse_strategy);
    let client = pm_client::PaymentMethodClient::new(
        &state.conf.micro_services.payment_methods_base_url,
        &parent_headers,
        &trace,
    );

    UpdatePaymentMethod::call(
        state,
        &client,
        UpdatePaymentMethodV1Request {
            payment_method_id: payment_method_id.to_string(),
            payload,
            modular_service_prefix: state.conf.micro_services.payment_methods_prefix.0.clone(),
        },
    )
    .await
    .map_err(|err| {
        logger::error!(error=?err, "modular payment method update failed");
        ::payment_methods::errors::ModularPaymentMethodError::UpdateFailed
    })?;
    Ok(())
}

#[cfg(feature = "v1")]
#[derive(Clone, Debug)]
pub struct DomainPaymentMethodWrapper(pub domain::PaymentMethod);

#[cfg(feature = "v1")]
pub struct DomainPaymentMethodDataWrapper(pub domain::PaymentMethodData);

#[derive(Clone, Debug)]
#[cfg(feature = "v1")]
pub struct PaymentMethodWithRawData {
    pub payment_method: DomainPaymentMethodWrapper,
    pub raw_payment_method_data: Option<domain::PaymentMethodData>,
}

#[cfg(feature = "v1")]
impl DomainPaymentMethodWrapper {
    pub async fn transform_pm_mod_retrieve_response(
        response: &RetrievePaymentMethodResponse,
        key_manager_state: &KeyManagerState,
        platform: &domain::Platform,
    ) -> errors::RouterResult<Self> {
        let encrypted_payment_method_billing_address: Option<
            Encryptable<Secret<serde_json::Value>>,
        > = response
            .billing
            .clone()
            .async_map(|address| {
                create_encrypted_data(
                    key_manager_state,
                    platform.get_provider().get_key_store(),
                    address.clone(),
                )
            })
            .await
            .transpose()
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Unable to encrypt payment method billing address")?;
        let _connector_mandate_details = response
            .connector_tokens
            .as_ref()
            .map(|connector_tokens| {
                let payments_map: std::collections::HashMap<
                    id_type::MerchantConnectorAccountId,
                    hyperswitch_domain_models::mandates::PaymentsMandateReferenceRecord,
                > = connector_tokens
                    .iter()
                    .map(|token_detail| {
                        (
                            token_detail.connector_id.clone(),
                            hyperswitch_domain_models::mandates::PaymentsMandateReferenceRecord {
                                connector_mandate_id: token_detail.token.clone().expose(),
                                payment_method_type: None,
                                original_payment_authorized_amount: token_detail
                                    .original_payment_authorized_amount
                                    .map(|amount| amount.get_amount_as_i64()),
                                original_payment_authorized_currency: token_detail
                                    .original_payment_authorized_currency,
                                mandate_metadata: token_detail.metadata.clone(),
                                connector_mandate_status: Some(match token_detail.status {
                                    common_enums::ConnectorTokenStatus::Active => {
                                        common_enums::ConnectorMandateStatus::Active
                                    }
                                    common_enums::ConnectorTokenStatus::Inactive => {
                                        common_enums::ConnectorMandateStatus::Inactive
                                    }
                                }),
                                connector_mandate_request_reference_id: token_detail
                                    .connector_token_request_reference_id
                                    .clone(),
                                connector_customer_id: None,
                            },
                        )
                    })
                    .collect();

                let mandate_reference =
                    hyperswitch_domain_models::mandates::CommonMandateReference {
                        payments: Some(
                            hyperswitch_domain_models::mandates::PaymentsMandateReference(
                                payments_map,
                            ),
                        ),
                        payouts: None,
                    };

                serde_json::to_value(mandate_reference)
            })
            .transpose()
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Failed to serialize connector mandate details")?;

        Ok(Self(domain::PaymentMethod {
            //for guest checkout, where customer id, this will fail.
            customer_id: response
                .customer_id
                .clone()
                .get_required_value("CustomerId")?,
            merchant_id: response.merchant_id.clone(),
            payment_method_id: response.payment_method_id.clone(),
            accepted_currency: None,
            scheme: None,
            token: None,
            cardholder_name: None,
            issuer_name: None,
            issuer_country: None,
            payer_country: None,
            is_stored: None,
            swift_code: None,
            direct_debit_token: None,
            created_at: response
                .created
                .unwrap_or_else(common_utils::date_time::now),
            last_modified: response
                .last_used_at
                .unwrap_or_else(common_utils::date_time::now),
            payment_method: Some(response.payment_method),
            payment_method_type: Some(response.payment_method_type),
            payment_method_issuer: None,
            payment_method_issuer_code: None,
            metadata: None,
            payment_method_data: None, //this is not required in any flow, hence None
            locker_id: None,           //This id will always be with PM Service
            last_used_at: response
                .last_used_at
                .unwrap_or_else(common_utils::date_time::now),
            connector_mandate_details: None,
            customer_acceptance: None,
            status: common_enums::PaymentMethodStatus::Active, //should be sent from PM service
            network_transaction_id: None,
            client_secret: None,
            payment_method_billing_address: encrypted_payment_method_billing_address,
            updated_by: None,
            version: common_enums::ApiVersion::V1, //to be updated later
            network_token_requestor_reference_id: None, //to be added later
            network_token_locker_id: None,
            network_token_payment_method_data: None,
            vault_source_details: domain::PaymentMethodVaultSourceDetails::InternalVault,
            created_by: platform
                .get_initiator()
                .and_then(|initiator| initiator.to_created_by()),
            last_modified_by: platform
                .get_initiator()
                .and_then(|initiator| initiator.to_created_by()),
            customer_details: None,
            locker_fingerprint_id: None,
        }))
    }

    pub async fn transform_pm_mod_create_response(
        response: &CreatePaymentMethodResponse,
        key_manager_state: &KeyManagerState,
        platform: &domain::Platform,
    ) -> errors::RouterResult<Self> {
        let encrypted_payment_method_billing_address: Option<
            Encryptable<Secret<serde_json::Value>>,
        > = response
            .billing
            .clone()
            .async_map(|address| {
                create_encrypted_data(
                    key_manager_state,
                    platform.get_provider().get_key_store(),
                    address.clone(),
                )
            })
            .await
            .transpose()
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Unable to encrypt payment method billing address")?;
        let connector_mandate_details = response
            .connector_tokens
            .as_ref()
            .map(|connector_tokens| {
                let payments_map: std::collections::HashMap<
                    id_type::MerchantConnectorAccountId,
                    hyperswitch_domain_models::mandates::PaymentsMandateReferenceRecord,
                > = connector_tokens
                    .iter()
                    .map(|token_detail| {
                        (
                            token_detail.connector_id.clone(),
                            hyperswitch_domain_models::mandates::PaymentsMandateReferenceRecord {
                                connector_mandate_id: token_detail.token.clone().expose(),
                                payment_method_type: None,
                                original_payment_authorized_amount: token_detail
                                    .original_payment_authorized_amount
                                    .map(|amount| amount.get_amount_as_i64()),
                                original_payment_authorized_currency: token_detail
                                    .original_payment_authorized_currency,
                                mandate_metadata: token_detail.metadata.clone(),
                                connector_mandate_status: Some(match token_detail.status {
                                    common_enums::ConnectorTokenStatus::Active => {
                                        common_enums::ConnectorMandateStatus::Active
                                    }
                                    common_enums::ConnectorTokenStatus::Inactive => {
                                        common_enums::ConnectorMandateStatus::Inactive
                                    }
                                }),
                                connector_mandate_request_reference_id: token_detail
                                    .connector_token_request_reference_id
                                    .clone(),
                                connector_customer_id: None,
                            },
                        )
                    })
                    .collect();

                let mandate_reference =
                    hyperswitch_domain_models::mandates::CommonMandateReference {
                        payments: Some(
                            hyperswitch_domain_models::mandates::PaymentsMandateReference(
                                payments_map,
                            ),
                        ),
                        payouts: None,
                    };

                serde_json::to_value(mandate_reference)
            })
            .transpose()
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Failed to serialize connector mandate details")?;

        Ok(Self(domain::PaymentMethod {
            //for guest checkout, where customer id, this will fail.
            customer_id: response
                .customer_id
                .clone()
                .get_required_value("CustomerId")?,
            merchant_id: response.merchant_id.clone(),
            payment_method_id: response.payment_method_id.clone(),
            accepted_currency: None,
            scheme: None,
            token: None,
            cardholder_name: None,
            issuer_name: None,
            issuer_country: None,
            payer_country: None,
            is_stored: None,
            swift_code: None,
            direct_debit_token: None,
            created_at: response
                .created
                .unwrap_or_else(common_utils::date_time::now),
            last_modified: response
                .last_used_at
                .unwrap_or_else(common_utils::date_time::now),
            payment_method: response.payment_method,
            payment_method_type: response.payment_method_type,
            payment_method_issuer: None,
            payment_method_issuer_code: None,
            metadata: None,
            payment_method_data: None, //this is not required in any flow, hence None
            locker_id: None,           //This id will always be with PM Service
            last_used_at: response
                .last_used_at
                .unwrap_or_else(common_utils::date_time::now),
            connector_mandate_details,
            customer_acceptance: None,
            status: common_enums::PaymentMethodStatus::Active, //should be sent from PM service
            network_transaction_id: None,
            client_secret: None,
            payment_method_billing_address: encrypted_payment_method_billing_address,
            updated_by: None,
            version: common_enums::ApiVersion::V1, //to be updated later
            network_token_requestor_reference_id: None, //to be added later
            network_token_locker_id: None,
            network_token_payment_method_data: None,
            vault_source_details: domain::PaymentMethodVaultSourceDetails::InternalVault,
            created_by: platform
                .get_initiator()
                .and_then(|initiator| initiator.to_created_by()),
            last_modified_by: platform
                .get_initiator()
                .and_then(|initiator| initiator.to_created_by()),
            customer_details: None,
            locker_fingerprint_id: None,
        }))
    }
}

#[cfg(feature = "v1")]
// from to convert payment method response to domain payment method
impl
    TryFrom<(
        payment_methods::types::RawPaymentMethodData,
        Option<domain::CardToken>,
    )> for DomainPaymentMethodDataWrapper
{
    type Error = error_stack::Report<errors::ApiErrorResponse>;

    fn try_from(
        (raw_data, card_token): (
            payment_methods::types::RawPaymentMethodData,
            Option<domain::CardToken>,
        ),
    ) -> Result<Self, Self::Error> {
        match raw_data {
            payment_methods::types::RawPaymentMethodData::Card(card_detail) => {
                // Use card_cvc from card_token if available, otherwise fall back to card_details.card_cvc
                let card_cvc = card_token
                    .as_ref()
                    .and_then(|token| token.card_cvc.clone())
                    .or(card_detail.card_cvc.clone())
                    .get_required_value("card_cvc")?;
                let card_holder_name = card_token
                    .and_then(|token| token.card_holder_name.clone())
                    .or(card_detail.card_holder_name.clone());

                Ok(Self(domain::PaymentMethodData::Card(
                    hyperswitch_domain_models::payment_method_data::Card {
                        card_number: card_detail.card_number,
                        card_exp_month: card_detail.card_exp_month,
                        card_exp_year: card_detail.card_exp_year,
                        card_cvc,
                        card_issuer: card_detail.card_issuer,
                        card_network: card_detail.card_network,
                        card_type: card_detail.card_type.map(|card_type| card_type.to_string()),
                        card_issuing_country: card_detail.card_issuing_country,
                        card_issuing_country_code: None,
                        bank_code: None,
                        nick_name: card_detail.nick_name,
                        card_holder_name,
                        co_badged_card_data: None,
                    },
                )))
            }
        }
    }
}

#[cfg(feature = "v1")]
impl TryFrom<CreatePaymentMethodResponse> for DomainPaymentMethodWrapper {
    type Error = error_stack::Report<errors::ApiErrorResponse>;
    fn try_from(response: CreatePaymentMethodResponse) -> Result<Self, Self::Error> {
        Ok(Self(domain::PaymentMethod {
            //for guest checkout, where customer id, this will fail.
            customer_id: response.customer_id.get_required_value("CustomerId")?,
            merchant_id: response.merchant_id,
            payment_method_id: response.payment_method_id,
            accepted_currency: None,
            scheme: None,
            token: None,
            cardholder_name: None,
            issuer_name: None,
            issuer_country: None,
            payer_country: None,
            is_stored: None,
            swift_code: None,
            direct_debit_token: None,
            created_at: response
                .created
                .unwrap_or_else(common_utils::date_time::now),
            last_modified: response
                .last_used_at
                .unwrap_or_else(common_utils::date_time::now),
            payment_method: response.payment_method,
            payment_method_type: response.payment_method_type,
            payment_method_issuer: None,
            payment_method_issuer_code: None,
            metadata: None,
            payment_method_data: None, //use response.card to convert to OptionalEncryptableValue
            locker_id: None,           //This id will always be with PM Service
            last_used_at: response
                .last_used_at
                .unwrap_or_else(common_utils::date_time::now),
            connector_mandate_details: None,
            customer_acceptance: None,
            status: common_enums::PaymentMethodStatus::Active, //should be sent from PM service
            network_transaction_id: None,
            client_secret: None,
            payment_method_billing_address: None, //Should be sent from PM service
            updated_by: None,
            version: common_enums::ApiVersion::V1,
            network_token_requestor_reference_id: None, //to be added later
            network_token_locker_id: None,
            network_token_payment_method_data: None,
            vault_source_details: domain::PaymentMethodVaultSourceDetails::InternalVault,
            created_by: None,
            last_modified_by: None,
            customer_details: None,
            locker_fingerprint_id: None,
        }))
    }
}

//Fetch Payment Method from Modular Service
#[cfg(feature = "v1")]
pub async fn fetch_payment_method_from_modular_service(
    state: &routes::SessionState,
    platform: &domain::Platform,
    profile_id: &id_type::ProfileId,
    payment_method_id: &str, //Currently PM id is string in v1
    pmd_card_token: Option<domain::CardToken>,
    is_off_session_payment: bool,
) -> CustomResult<PaymentMethodWithRawData, errors::ApiErrorResponse> {
    let payment_method_fetch_req = RetrievePaymentMethodV1Request {
        payment_method_id: api_models::payment_methods::PaymentMethodId {
            payment_method_id: payment_method_id.to_owned(),
        },
        fetch_raw_detail: true,
        modular_service_prefix: state.conf.micro_services.payment_methods_prefix.0.clone(),
    };

    //Fetch modular service call
    let pm_response = retrieve_pm_modular_service_call(
        state,
        platform.get_processor().get_account().get_id(),
        profile_id,
        payment_method_fetch_req,
    )
    .await?;

    //Convert PMResponse to PaymentMethodWithRawData
    let payment_method = DomainPaymentMethodWrapper::transform_pm_mod_retrieve_response(
        &pm_response,
        &state.into(),
        platform,
    )
    .await
    .attach_printable("Failed to transform payment method retrieve response")?;

    //Convert RawPaymentMethodData to domain::PaymentMethodData
    let raw_payment_method_data = (!is_off_session_payment)
        .then(|| {
            pm_response
                .raw_payment_method_data
                .map(|raw_data| {
                    DomainPaymentMethodDataWrapper::try_from((raw_data, pmd_card_token.clone()))
                })
                .transpose()
        })
        .transpose()
        .attach_printable("Failed to convert raw payment method data")?
        .flatten();

    let pm_wrapper = PaymentMethodWithRawData {
        payment_method,
        raw_payment_method_data: raw_payment_method_data.map(|wrapper| wrapper.0),
    };
    Ok(pm_wrapper)
}

#[cfg(feature = "v1")]
pub async fn retrieve_pm_modular_service_call(
    state: &routes::SessionState,
    processor_merchant_id: &id_type::MerchantId,
    profile_id: &id_type::ProfileId,
    payment_method_fetch_req: RetrievePaymentMethodV1Request,
) -> CustomResult<RetrievePaymentMethodResponse, errors::ApiErrorResponse> {
    let internal_api_key = &state
        .conf
        .internal_merchant_id_profile_id_auth
        .internal_api_key;
    let mut parent_headers = Headers::new();
    parent_headers.insert((
        headers::X_PROFILE_ID.to_string(),
        profile_id.get_string_repr().to_string().into_masked(),
    ));
    parent_headers.insert((
        headers::X_INTERNAL_API_KEY.to_string(),
        internal_api_key.clone().expose().to_string().into_masked(),
    ));
    parent_headers.insert((
        headers::X_MERCHANT_ID.to_string(),
        processor_merchant_id
            .get_string_repr()
            .to_string()
            .into_masked(),
    ));

    let trace = RequestIdentifier::new(&state.conf.trace_header.header_name)
        .use_incoming_id(state.conf.trace_header.id_reuse_strategy);

    //pm client construction
    let client = pm_client::PaymentMethodClient::new(
        &state.conf.micro_services.payment_methods_base_url,
        &parent_headers,
        &trace,
    );

    //Modular service call
    let pm_response =
        pm_client::RetrievePaymentMethod::call(state, &client, payment_method_fetch_req)
            .await
            .map_err(|err| {
                logger::debug!("Error in retrieving payment method: {:?}", err);
                errors::ApiErrorResponse::InternalServerError
            })
            .attach_printable("Failed to retrieve payment method from modular service")?;

    Ok(pm_response)
}

//Create Payment Method from Modular Service
#[cfg(feature = "v1")]
#[allow(clippy::too_many_arguments)]
pub async fn create_payment_method_in_modular_service(
    state: &routes::SessionState,
    provider_merchant_id: &id_type::MerchantId,
    processor_merchant_id: &id_type::MerchantId,
    profile_id: &id_type::ProfileId,
    payment_method: common_enums::PaymentMethod,
    payment_method_type: common_enums::PaymentMethodType,
    payment_method_data: domain::PaymentMethodData,
    billing_address: Option<hyperswitch_domain_models::address::Address>,
    customer_id: id_type::CustomerId,
) -> CustomResult<domain::PaymentMethod, errors::ApiErrorResponse> {
    let payment_method_request = CreatePaymentMethodV1Request {
        merchant_id: provider_merchant_id.clone(),
        payment_method,
        payment_method_type,
        metadata: None,
        customer_id,
        payment_method_data,
        billing: billing_address,
        network_tokenization: None,
        storage_type: Some(common_enums::StorageType::Persistent),
        modular_service_prefix: state.conf.micro_services.payment_methods_prefix.0.clone(),
    };

    //Create modular service call
    let pm_response = create_pm_modular_service_call(
        state,
        processor_merchant_id,
        profile_id,
        payment_method_request,
    )
    .await?;

    //Convert PMResponse to PaymentMethodWithRawData
    let payment_method_with_raw_data = DomainPaymentMethodWrapper::try_from(pm_response)?;

    Ok(payment_method_with_raw_data.0)
}

#[cfg(feature = "v1")]
pub async fn create_pm_modular_service_call(
    state: &routes::SessionState,
    merchant_id: &id_type::MerchantId,
    profile_id: &id_type::ProfileId,
    payment_method_create_req: CreatePaymentMethodV1Request,
) -> CustomResult<CreatePaymentMethodResponse, errors::ApiErrorResponse> {
    let internal_api_key = &state
        .conf
        .internal_merchant_id_profile_id_auth
        .internal_api_key;
    let mut parent_headers = Headers::new();
    parent_headers.insert((
        headers::X_PROFILE_ID.to_string(),
        profile_id.get_string_repr().to_string().into_masked(),
    ));
    parent_headers.insert((
        headers::X_INTERNAL_API_KEY.to_string(),
        internal_api_key.clone().expose().to_string().into_masked(),
    ));
    parent_headers.insert((
        headers::X_MERCHANT_ID.to_string(),
        merchant_id.get_string_repr().to_string().into_masked(),
    ));

    let trace = RequestIdentifier::new(&state.conf.trace_header.header_name)
        .use_incoming_id(state.conf.trace_header.id_reuse_strategy);

    //pm client construction
    let client = pm_client::PaymentMethodClient::new(
        &state.conf.micro_services.payment_methods_base_url,
        &parent_headers,
        &trace,
    );

    //Modular service call
    let pm_response =
        pm_client::CreatePaymentMethod::call(state, &client, payment_method_create_req)
            .await
            .map_err(|err| {
                logger::debug!("Error in creating payment method: {:?}", err);
                errors::ApiErrorResponse::InternalServerError
            })
            .attach_printable("Failed to create payment method in modular service")?;

    Ok(pm_response)
}
