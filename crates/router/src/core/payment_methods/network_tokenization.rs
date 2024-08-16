use std::fmt::Debug;

use api_models::{enums as api_enums, payment_methods::PaymentMethodsData};
use cards::CardNumber;
use common_utils::{
    errors::CustomResult,
    ext_traits::{Encode, OptionExt},
    id_type,
    request::RequestContent,
    type_name,
    types::keymanager::Identifier,
};
use diesel_models::payment_method;
use error_stack::ResultExt;
use hyperswitch_domain_models::payment_method_data::NetworkTokenData;
use josekit::jwe;
use masking::{ExposeInterface, Mask, PeekInterface, Secret};
use serde::{Deserialize, Serialize};

use super::transformers::DeleteCardResp;
use crate::{
    core::{
        errors,
        payment_methods::{self},
        payments::helpers,
    },
    headers, logger,
    routes::{self},
    services::{self, encryption},
    types::{
        api::{self},
        domain,
        storage::{self, enums as storage_enums},
    },
    utils::ConnectorResponseExt,
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
    key_id: String,
    should_send_token: bool,
}

#[derive(Debug, Deserialize, Serialize, Eq, PartialEq)]
pub struct CardNetworkTokenResponse {
    payload: String,
}

#[derive(Debug, Deserialize, Eq, PartialEq)]
#[serde(untagged)]
pub enum CardNTResponse {
    CardNetworkTokenResponse(CardNetworkTokenResponse),
    CardNetworkTokenErrorResponse(NetworkTokenErrorResponse),
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CardNetworkTokenResponsePayload {
    pub card_brand: api_enums::CardNetwork,
    pub card_fingerprint: String,
    pub card_reference: String,
    pub correlation_id: String,
    pub customer_id: String,
    pub par: String,
    pub token: CardNumber,
    pub token_expiry_month: Secret<String>,
    pub token_expiry_year: Secret<String>,
    pub token_isin: String,
    pub token_last_four: String,
    pub token_status: String,
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

#[derive(Debug, Serialize, Deserialize)]
pub struct DeleteCardToken {
    card_reference: String,
    customer_id: id_type::CustomerId,
}

#[derive(Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "UPPERCASE")]
pub enum DeleteNetworkTokenStatus {
    Success,
}

#[derive(Debug, Deserialize, Eq, PartialEq)]
#[serde(untagged)]
pub enum DeleteNTResponse {
    DeleteNetworkTokenResponse(DeleteNetworkTokenResponse),
    DeleteNetworkTokenErrorResponse(NetworkTokenErrorResponse),
}

#[derive(Debug, Deserialize, Eq, PartialEq)]
pub struct NetworkTokenErrorInfo {
    code: String,
    developer_message: String,
}

#[derive(Debug, Deserialize, Eq, PartialEq)]
pub struct NetworkTokenErrorResponse {
    error_message: String,
    error_info: NetworkTokenErrorInfo,
}

#[derive(Debug, Deserialize, Eq, PartialEq)]
pub struct DeleteNetworkTokenResponse {
    status: DeleteNetworkTokenStatus,
}

pub async fn make_card_network_tokenization_request(
    state: &routes::SessionState,
    card: &domain::Card,
    customer_id: &Option<id_type::CustomerId>,
    amount: Option<i64>,
    currency: Option<storage_enums::Currency>,
) -> CustomResult<(CardNetworkTokenResponsePayload, Option<String>), errors::ApiErrorResponse> {
    let customer_id = customer_id
        .clone()
        .get_required_value("customer_id")
        .change_context(errors::ApiErrorResponse::InternalServerError)?;
    let card_data = CardData {
        card_number: card.card_number.clone(),
        exp_month: card.card_exp_month.clone(),
        exp_year: card.card_exp_year.clone(),
        card_security_code: card.card_cvc.clone(),
    };

    let payload = card_data
        .encode_to_string_of_json()
        .and_then(|x| x.encode_to_string_of_json())
        .change_context(errors::VaultError::FetchCardFailed)
        .map_err(|e| {
            logger::error!(fetch_err=?e);
            errors::ApiErrorResponse::InternalServerError
        })?;
    let payload_bytes = payload.as_bytes();
    let tokenization_service = &state.conf.network_tokenization_service.get_inner();

    let enc_key = tokenization_service.public_key.peek().clone();

    let key_id = tokenization_service.key_id.clone();
    println!("payloadd bytess {:?}", payload_bytes);

    let jwt = encryption::encrypt_jwe(
        payload_bytes,
        enc_key,
        "A128GCM",
        Some(key_id.as_str()),
    )
    .await
    .change_context(errors::ApiErrorResponse::InternalServerError)?;
    println!("jwttt: {:?}", jwt);
    let amount_str = amount.map_or_else(String::new, |a| a.to_string());
    let currency_str = currency.map_or_else(String::new, |c| c.to_string());
    let order_data = OrderData {
        consent_id: "test12324".to_string(), // ??
        customer_id: customer_id.clone(),
        amount: amount_str,
        currency: currency_str,
    };

    let api_payload = ApiPayload {
        service: "NETWORK_TOKEN".to_string(),
        card_data: jwt,
        order_data,
        key_id,
        should_send_token: true,
    };

    let mut request = services::Request::new(
        services::Method::Post,
        tokenization_service.generate_token_url.as_str(),
    );
    request.add_header(headers::CONTENT_TYPE, "application/json".into());
    request.add_header(
        headers::AUTHORIZATION,
        tokenization_service
            .token_service_api_key
            .peek()
            .clone()
            .into_masked(),
    );
    request.set_body(RequestContent::Json(Box::new(api_payload)));

    println!("reqq to eulerr: {:?}", request);

    let response = services::call_connector_api(state, request, "generate_token")
        .await
        .change_context(errors::VaultError::SaveCardFailed);

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
    .change_context(errors::ApiErrorResponse::InternalServerError)
    .attach_printable(
        "Failed to decrypt the tokenization response from the tokenization service",
    )?;

    let cn_response: CardNetworkTokenResponsePayload =
        serde_json::from_str(&card_network_token_response)
            .change_context(errors::VaultError::ResponseDeserializationFailed)
            .change_context(errors::ApiErrorResponse::InternalServerError)?;

    Ok((cn_response.clone(), Some(cn_response.card_reference)))
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
        tokenization_service.fetch_token_url.as_str(),
    );
    let payload = GetCardToken {
        card_reference: token_ref.unwrap(),
        customer_id: pm_data.clone().customer_id,
    };

    request.add_header(headers::CONTENT_TYPE, "application/json".into());
    request.add_header(
        headers::AUTHORIZATION,
        tokenization_service
            .token_service_api_key
            .clone()
            .peek()
            .clone()
            .into_masked(),
    );
    request.set_body(RequestContent::Json(Box::new(payload)));

    // Send the request using `call_connector_api`
    let response = services::call_connector_api(state, request, "get network token")
        .await
        .change_context(errors::VaultError::SaveCardFailed);

    let res: TokenResponse = response
        .get_response_inner("cardNetworkTokenResponse")
        .change_context(errors::VaultError::FetchCardFailed)
        .map_err(|e| {
            logger::error!(fetch_err=?e);
            errors::ApiErrorResponse::InternalServerError
        })?;
    let identifier = Identifier::Merchant(key_store.merchant_id.clone());
    let card_decrypted = domain::types::crypto_operation::<serde_json::Value, masking::WithType>(
        &state.into(),
        type_name!(payment_method::PaymentMethod),
        domain::types::CryptoOperation::DecryptOptional(pm_data.payment_method_data.clone()),
        identifier,
        key,
    )
    .await
    .and_then(|val| val.try_into_optionaloperation())
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
    let card_data = NetworkTokenData {
        token_number: res.authentication_details.token,
        token_cryptogram: res.authentication_details.cryptogram,
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

pub async fn do_status_check_for_network_token(
    state: &routes::SessionState,
    key_store: &domain::MerchantKeyStore,
    payment_method_info: &storage::PaymentMethod,
    //input either network token ref or network token
) -> CustomResult<bool, errors::ApiErrorResponse> {
    let key = key_store.key.get_inner().peek();
    let identifier = Identifier::Merchant(key_store.merchant_id.clone());
    let token_data_decrypted =
        domain::types::crypto_operation::<serde_json::Value, masking::WithType>(
            &state.into(),
            type_name!(payment_method::PaymentMethod),
            domain::types::CryptoOperation::DecryptOptional(
                payment_method_info.token_payment_method_data.clone(),
            ),
            identifier,
            key,
        )
        .await
        .and_then(|val| val.try_into_optionaloperation())
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

    is_token_active(token_data_decrypted)
}

fn is_token_active(
    token_data_decrypted: Option<api::CardDetailFromLocker>,
) -> CustomResult<bool, errors::ApiErrorResponse> {
    if let Some(token_data) = token_data_decrypted {
        if let (Some(exp_month), Some(exp_year)) = (token_data.expiry_month, token_data.expiry_year)
        {
            helpers::validate_card_expiry(&exp_month, &exp_year)?;
        }
        Ok(true)
    } else {
        check_token_status_with_tokenization_service()
    }
}

fn check_token_status_with_tokenization_service() -> CustomResult<bool, errors::ApiErrorResponse> {
    Ok(true)
}

pub async fn delete_network_token_from_locker_and_token_service(
    state: &routes::SessionState,
    customer_id: &id_type::CustomerId,
    merchant_id: &id_type::MerchantId,
    payment_method_id: String,
    token_locker_id: Option<String>,
    network_token_requestor_reference_id: String,
) -> errors::RouterResult<DeleteCardResp> {
    let resp = payment_methods::cards::delete_card_from_locker(
        &state,
        customer_id,
        merchant_id,
        token_locker_id.as_ref().unwrap_or(&payment_method_id),
    )
    .await?;
    let _delete_token_resp = delete_network_token_from_tokenization_service(
        state,
        network_token_requestor_reference_id,
        customer_id,
    )
    .await?;

    if resp.status == "Ok" {
        logger::info!("Card From locker deleted Successfully!");
    } else {
        logger::error!("Error: Deleting Card From Locker!\n{:#?}", resp);
        Err(errors::ApiErrorResponse::InternalServerError)?
    }

    Ok(resp)
}

pub async fn delete_network_token_from_tokenization_service(
    state: &routes::SessionState,
    network_token_requestor_reference_id: String,
    customer_id: &id_type::CustomerId,
) -> CustomResult<bool, errors::ApiErrorResponse> {
    let tokenization_service = &state.conf.network_tokenization_service.get_inner();
    let mut request = services::Request::new(
        services::Method::Post,
        tokenization_service.delete_token_url.as_str(),
    );
    let payload = DeleteCardToken {
        card_reference: network_token_requestor_reference_id,
        customer_id: customer_id.clone(),
    };

    request.add_header(headers::CONTENT_TYPE, "application/json".into());
    request.add_header(
        headers::AUTHORIZATION,
        tokenization_service
            .token_service_api_key
            .clone()
            .peek()
            .clone()
            .into_masked(),
    );
    request.set_body(RequestContent::Json(Box::new(payload)));

    // Send the request using `call_connector_api`
    let response = services::call_connector_api(state, request, "delete network token")
        .await
        .change_context(errors::VaultError::SaveCardFailed);

    let res: DeleteNTResponse = response
        .get_response_inner("cardNetworkTokenResponse")
        .change_context(errors::VaultError::FetchCardFailed)
        .map_err(|e| {
            logger::error!(fetch_err=?e);
            errors::ApiErrorResponse::InternalServerError
        })?;

    if res
        == DeleteNTResponse::DeleteNetworkTokenResponse(DeleteNetworkTokenResponse {
            status: DeleteNetworkTokenStatus::Success,
        })
    {
        Ok(true)
    } else {
        Err(errors::ApiErrorResponse::InternalServerError)?
    }
}
