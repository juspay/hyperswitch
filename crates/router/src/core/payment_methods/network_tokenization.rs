use masking::{ExposeInterface, Mask, PeekInterface, Secret};
use std::{
    collections::{HashMap, HashSet},
    fmt::Debug,
    str::FromStr,
};

use api_models::{
    admin::PaymentMethodsEnabled,
    enums as api_enums,
    payment_methods::{Card, PaymentMethodsData},
};
use common_enums::enums::MerchantStorageScheme;
use common_utils::{
    consts,
    crypto::Encryptable,
    encryption::Encryption,
    errors::{self as common_utils_erros, CustomResult, ParsingError, ValidationError},
    ext_traits::{Encode, OptionExt},
    id_type,
    request::RequestContent,
    types::keymanager::Identifier,
};
use error_stack::{report, ResultExt};
use euclid::dssa::graph::{AnalysisContext, CgraphExt};
use strum::IntoEnumIterator;

use cards::CardNumber;
use josekit::jwe;
use serde::{Deserialize, Serialize};
use hyperswitch_domain_models::{
    payment_method_data::NetworkTokenData,
};

#[cfg(feature = "payouts")]
use crate::{
    core::errors::{self, ConnectorErrorExt, RouterResult, StorageErrorExt},
    headers, logger,
    routes::{self},
    services::{
        self, encryption,
        request::{self},
    },
    types::{
        api::{self},
        domain::{self, types::decrypt_optional},
        storage::{self, enums as storage_enums},
    },
    utils::{ ConnectorResponseExt},
};

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CardData {
    card_number: CardNumber,
    exp_month: Secret<String>,
    exp_year: Secret<String>,
    card_security_code: Secret<String>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OrderData {
    consent_id: String,
    customer_id: id_type::CustomerId,
    amount: String,
    currency: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiPayload {
    service: String,
    card_data: String,
    order_data: OrderData,
    sub_merchant_id: String,
    key_id: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CardNetworkTokenResponse {
    payload: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CardNetworkTokenResponsePayloadTemporary {
    pub card_brand: api_enums::CardNetwork,
    pub card_fingerprint: String,
    pub card_reference: String,
    pub correlation_id: String,
    pub customer_id: String,
    pub par: String,
    pub token_expiry_month: Secret<String>,
    pub token_expiry_year: Secret<String>,
    pub token_isin: String,
    pub token_last_four: String,
    pub token_status: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CardNetworkTokenResponsePayload {
    pub card_brand: api_enums::CardNetwork,
    pub card_fingerprint: String,
    pub card_reference: CardNumber,
    pub correlation_id: String,
    pub customer_id: String,
    pub par: String,
    pub token_expiry_month: Secret<String>,
    pub token_expiry_year: Secret<String>,
    pub token_isin: String,
    pub token_last_four: String,
    pub token_status: String,
}

pub async fn make_card_network_tokenization_request(
    state: &routes::SessionState,
    payment_method_data: Option<&domain::PaymentMethodData>,
    merchant_account: &domain::MerchantAccount,
    customer_id: &Option<id_type::CustomerId>,
    amount: Option<i64>,
    currency: Option<storage_enums::Currency>,
) -> errors::CustomResult<(CardNetworkTokenResponsePayload, Option<String>), errors::ApiErrorResponse>
{
    let customer_id = customer_id
        .clone()
        .get_required_value("customer_id")
        .change_context(errors::ApiErrorResponse::InternalServerError)?;
    let card_data = match payment_method_data {
        Some(pm_data) => match pm_data {
            domain::PaymentMethodData::Card(card) => CardData {
                card_number: card.card_number.clone(),
                exp_month: card.card_exp_month.clone(),
                exp_year: card.card_exp_year.clone(),
                card_security_code: card.card_cvc.clone(),
            },
            _ => todo!(),
        },
        _ => todo!(),
    };

    let payload = card_data
        .encode_to_string_of_json()
        .and_then(|x| x.encode_to_string_of_json())
        .change_context(errors::VaultError::FetchCardFailed)
        .map_err(|e| {
            logger::error!(fetch_err=?e);
            errors::ApiErrorResponse::InternalServerError
        })?;
    println!("payloaddd: {:?}", payload);
    let payload_bytes = payload.as_bytes();
    println!("payload_bytesss: {:?}", payload_bytes);
    let tokenization_service = &state.conf.network_tokenization_service.get_inner();

    let enc_key = tokenization_service.public_key.peek().clone();

    let key_id = tokenization_service.key_id.clone();
    let jwt = encryption::encrypt_jwe(payload_bytes, enc_key, "A128GCM", Some(key_id.as_str()))
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)?;
    println!("jwtt: {:?}", jwt);
    let amount_str = amount.map_or_else(String::new, |a| a.to_string());
    let currency_str = currency.map_or_else(String::new, |c| c.to_string());
    let order_data = OrderData {
        consent_id: "test12324".to_string(), // ??
        customer_id: customer_id.clone(),
        amount:amount_str,
        currency:currency_str,
    };

    let api_payload = ApiPayload {
        service: "NETWORK_TOKEN".to_string(),
        card_data: jwt,
        order_data,
        sub_merchant_id: "visa_sbx_working".to_string(),
        key_id,
    };

    let mut request = services::Request::new(
        services::Method::Post,
        tokenization_service.generate_token_url.as_str(),
    );
    request.add_header(headers::CONTENT_TYPE, "application/json".into());
    request.add_header(
        headers::AUTHORIZATION,
        tokenization_service.token_service_api_key.peek().clone().into_masked(),
    );
    request.set_body(RequestContent::Json(Box::new(api_payload)));
    logger::debug!("Requestt to euler: {:?}", request);

    let response = services::call_connector_api(state, request, "generate_token")
        .await
        .change_context(errors::VaultError::SaveCardFailed);

    logger::debug!("Responsee from euler: {:?}", response);

    let res: CardNetworkTokenResponse = response
        .get_response_inner("cardNetworkTokenResponse")
        .change_context(errors::VaultError::FetchCardFailed)
        .map_err(|e| {
            logger::error!(fetch_err=?e);
            errors::ApiErrorResponse::InternalServerError
        })?;
    let dec_key = tokenization_service.private_key.peek().clone();

    let card_network_token_response = services::decrypt_jwe(
        &res.payload,
        services::KeyIdCheck::SkipKeyIdCheck,
        dec_key,
        jwe::RSA_OAEP_256,
    )
    .await
    .unwrap();
    println!(
        "card_network_token_response: {:?}",
        card_network_token_response
    );

    let cn_response_temp: CardNetworkTokenResponsePayloadTemporary =
        serde_json::from_str(&card_network_token_response)
            .change_context(errors::VaultError::ResponseDeserializationFailed)
            .change_context(errors::ApiErrorResponse::InternalServerError)?;

    let cn_response = CardNetworkTokenResponsePayload {
        card_brand: cn_response_temp.card_brand,
        card_fingerprint: cn_response_temp.card_fingerprint,
        card_reference: CardNumber::from_str("xxxxxxxxxxxxx").unwrap(),
        correlation_id: cn_response_temp.correlation_id,
        customer_id: cn_response_temp.customer_id,
        par: cn_response_temp.par,
        token_expiry_month: cn_response_temp.token_expiry_month,
        token_expiry_year: cn_response_temp.token_expiry_year,
        token_isin: cn_response_temp.token_isin,
        token_last_four: cn_response_temp.token_last_four,
        token_status: cn_response_temp.token_status,
    };
    Ok((cn_response, Some(cn_response_temp.card_reference)))
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GetCardToken {
    card_reference: String,
    customer_id: id_type::CustomerId,
}
#[derive(Debug, Serialize, Deserialize)]
pub struct AuthenticationDetails {
    cryptogram: Secret<String>,
    token: CardNumber,
}
#[derive(Debug, Serialize, Deserialize)]
pub struct TokenResponse {
    authentication_details: AuthenticationDetails,
}

pub async fn get_token_from_tokenization_service(
    state: &routes::SessionState,
    merchant_account: &domain::MerchantAccount,
    key_store: &domain::MerchantKeyStore,
    pm_id: String, //to fetch from pm table
) -> errors::RouterResult<NetworkTokenData> {
    let db = state.store.as_ref();
    let key = key_store.key.get_inner().peek();
    let pm_data = db
        .find_payment_method(&pm_id, merchant_account.storage_scheme)
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)?;

    let token_ref = pm_data.clone().network_token_reference_id;

    let tokenization_service = &state.conf.network_tokenization_service.get_inner();
    let mut request = services::Request::new(
        services::Method::Post,
        tokenization_service.clone().fetch_token_url.as_str(),
    );
    let payload = GetCardToken {
        card_reference: token_ref.unwrap(),
        customer_id: pm_data.clone().customer_id,
    };

    request.add_header(headers::CONTENT_TYPE, "application/json".into());
    request.add_header(
        headers::AUTHORIZATION,
        tokenization_service.token_service_api_key.clone().peek().clone().into_masked(),
    );
    request.set_body(RequestContent::Json(Box::new(payload)));
    logger::debug!("reqq to euler: {:?}", request);

    // Send the request using `call_connector_api`
    let response = services::call_connector_api(state, request, "get network token")
        .await
        .change_context(errors::VaultError::SaveCardFailed);
    logger::debug!("Response from euler: {:?}", response);
    let res: TokenResponse = response
        .get_response_inner("cardNetworkTokenResponse")
        .change_context(errors::VaultError::FetchCardFailed)
        .map_err(|e| {
            logger::error!(fetch_err=?e);
            errors::ApiErrorResponse::InternalServerError
        })?;
    println!("ressss: {:?}", res);
    // let key = key_store.key.get_inner().peek();
    let identifier = Identifier::Merchant(key_store.merchant_id.clone());
    let card_decrypted = decrypt_optional::<serde_json::Value, masking::WithType>(
        &state.into(),
        pm_data.payment_method_data.clone(),
        identifier,
        key,
    )
    .await
    .change_context(errors::StorageError::DecryptionError)
    .attach_printable("unable to decrypt card details")
    .ok()
    .flatten()
    .map(|x| x.into_inner().expose())
    .and_then(|v| serde_json::from_value::<PaymentMethodsData>(v).ok())
    .and_then(|pmd| match pmd {
        PaymentMethodsData::Card(crd) => Some(api::CardDetailFromLocker::from(crd)),
        _ => None,
    });
    println!("card_decrypted: {:?}", card_decrypted);
    let card_data = NetworkTokenData {
        token_number: res.authentication_details.token,
        token_cryptogram:res.authentication_details.cryptogram,
        token_exp_month: card_decrypted
            .clone()
            .unwrap()
            .expiry_month
            .unwrap_or_default(),
        token_exp_year: card_decrypted
            .clone()
            .unwrap()
            .expiry_year
            .unwrap_or_default(),
        nick_name: card_decrypted.clone().unwrap().card_holder_name,
        card_issuer: None,
        card_network: Some(common_enums::CardNetwork::Visa),
        card_type: None,
        card_issuing_country: None,
        bank_code: None,
    };
    Ok(card_data)
}
