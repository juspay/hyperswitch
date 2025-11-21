use std::fmt::Debug;

use crate::{
    configs::settings,
    core::{cards::mk_basilisk_req, errors},
    headers, services, state,
};
use api_models::payment_methods;
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
use common_utils::{encryption, id_type};
use error_stack::report;
use error_stack::ResultExt;
use hyperswitch_domain_models::{
    ext_traits::OptionExt, merchant_connector_account, payment_method_data::PaymentMethodData,
    router_data_v2, router_request_types, router_response_types, types,
};
use masking::{PeekInterface, Secret};
use router_env::RequestId;
use router_env::{instrument, logger, tracing};
use serde::{Deserialize, Serialize};
use std::marker::PhantomData;

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

#[cfg(feature = "v1")]
#[instrument(skip_all)]
pub(crate) async fn get_payment_method_create_request(
    payment_method_data: Option<&PaymentMethodData>,
    payment_method: Option<common_enums::PaymentMethod>,
    payment_method_type: Option<common_enums::PaymentMethodType>,
    customer_id: &Option<id_type::CustomerId>,
    billing_name: Option<Secret<String>>,
    payment_method_billing_address: Option<&hyperswitch_domain_models::address::Address>,
) -> errors::PmResult<payment_methods::PaymentMethodCreate> {
    match payment_method_data {
        Some(pm_data) => match payment_method {
            Some(payment_method) => match pm_data {
                PaymentMethodData::Card(card) => {
                    let card_network = get_card_network_with_us_local_debit_network_override(
                        card.card_network.clone(),
                        card.co_badged_card_data.as_ref(),
                    );

                    let card_detail = payment_methods::CardDetail {
                        card_number: card.card_number.clone(),
                        card_exp_month: card.card_exp_month.clone(),
                        card_exp_year: card.card_exp_year.clone(),
                        card_holder_name: billing_name,
                        nick_name: card.nick_name.clone(),
                        card_issuing_country: card.card_issuing_country.clone(),
                        card_network: card_network.clone(),
                        card_issuer: card.card_issuer.clone(),
                        card_type: card.card_type.clone(),
                        card_cvc: None, // DO NOT POPULATE CVC FOR ADDITIONAL PAYMENT METHOD DATA
                    };
                    let payment_method_request = payment_methods::PaymentMethodCreate {
                        payment_method: Some(payment_method),
                        payment_method_type,
                        payment_method_issuer: card.card_issuer.clone(),
                        payment_method_issuer_code: None,
                        #[cfg(feature = "payouts")]
                        bank_transfer: None,
                        #[cfg(feature = "payouts")]
                        wallet: None,
                        card: Some(card_detail),
                        metadata: None,
                        customer_id: customer_id.clone(),
                        card_network: card_network
                            .clone()
                            .as_ref()
                            .map(|card_network| card_network.to_string()),
                        client_secret: None,
                        payment_method_data: None,
                        //TODO: why are we using api model in router internally
                        billing: payment_method_billing_address.cloned().map(From::from),
                        connector_mandate_details: None,
                        network_transaction_id: None,
                    };
                    Ok(payment_method_request)
                }
                _ => {
                    let payment_method_request = payment_methods::PaymentMethodCreate {
                        payment_method: Some(payment_method),
                        payment_method_type,
                        payment_method_issuer: None,
                        payment_method_issuer_code: None,
                        #[cfg(feature = "payouts")]
                        bank_transfer: None,
                        #[cfg(feature = "payouts")]
                        wallet: None,
                        card: None,
                        metadata: None,
                        customer_id: customer_id.clone(),
                        card_network: None,
                        client_secret: None,
                        payment_method_data: None,
                        billing: None,
                        connector_mandate_details: None,
                        network_transaction_id: None,
                    };

                    Ok(payment_method_request)
                }
            },
            None => Err(report!(errors::ApiErrorResponse::MissingRequiredField {
                field_name: "payment_method_type"
            })
            .attach_printable("PaymentMethodType Required")),
        },
        None => Err(report!(errors::ApiErrorResponse::MissingRequiredField {
            field_name: "payment_method_data"
        })
        .attach_printable("PaymentMethodData required Or Card is already saved")),
    }
}

/// Determines the appropriate card network to to be stored.
///
/// If the provided card network is a US local network, this function attempts to
/// override it with the first global network from the co-badged card data, if available.
/// Otherwise, it returns the original card network as-is.
///
fn get_card_network_with_us_local_debit_network_override(
    card_network: Option<common_enums::CardNetwork>,
    co_badged_card_data: Option<&payment_methods::CoBadgedCardData>,
) -> Option<common_enums::CardNetwork> {
    if let Some(true) = card_network
        .as_ref()
        .map(|network| network.is_us_local_network())
    {
        logger::debug!("Card network is a US local network, checking for global network in co-badged card data");
        let info: Option<api_models::open_router::CoBadgedCardNetworksInfo> = co_badged_card_data
            .and_then(|data| {
                data.co_badged_card_networks_info
                    .0
                    .iter()
                    .find(|info| info.network.is_signature_network())
                    .cloned()
            });
        info.map(|data| data.network)
    } else {
        card_network
    }
}

pub async fn construct_vault_router_data<F>(
    state: &state::PaymentMethodsState,
    merchant_id: &id_type::MerchantId,
    merchant_connector_account: &merchant_connector_account::MerchantConnectorAccount,
    payment_method_vaulting_data: Option<
        hyperswitch_domain_models::vault::PaymentMethodVaultingData,
    >,
    connector_vault_id: Option<String>,
    connector_customer_id: Option<String>,
    should_generate_multiple_tokens: Option<bool>,
) -> errors::PmResult<types::VaultRouterDataV2<F>> {
    let connector_auth_type = merchant_connector_account
        .get_connector_account_details()
        .change_context(errors::ApiErrorResponse::InternalServerError)?;

    let resource_common_data = router_data_v2::VaultConnectorFlowData {
        merchant_id: merchant_id.to_owned(),
    };

    let router_data = router_data_v2::RouterDataV2 {
        flow: PhantomData,
        resource_common_data,
        tenant_id: state.tenant.tenant_id.clone(),
        connector_auth_type,
        request: router_request_types::VaultRequestData {
            payment_method_vaulting_data,
            connector_vault_id,
            connector_customer_id,
            should_generate_multiple_tokens,
        },
        response: Ok(router_response_types::VaultResponseData::default()),
    };

    Ok(router_data)
}
