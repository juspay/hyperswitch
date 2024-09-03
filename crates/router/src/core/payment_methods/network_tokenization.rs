use std::fmt::Debug;

use api_models::{enums as api_enums, payment_methods::PaymentMethodsData};
use cards::CardNumber;
use common_utils::{
    errors::CustomResult,
    ext_traits::{BytesExt, Encode, ValueExt},
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
    services::{self, encryption, EncryptionAlgorithm},
    settings::NetworkTokenizationService,
    types::{
        api::{self},
        domain,
        storage::{self, enums as storage_enums},
    },
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
    sub_merchant_id: String,
}

#[derive(Debug, Deserialize, Serialize, Eq, PartialEq)]
pub struct CardNetworkTokenResponse {
    payload: String,
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
pub struct TokenDetails {
    exp_month: Secret<String>,
    exp_year: Secret<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TokenResponse {
    authentication_details: AuthenticationDetails,
    network: api_enums::CardNetwork,
    token_details: TokenDetails,
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

#[derive(Debug, Serialize, Deserialize)]
pub struct CheckTokenStatus {
    card_reference: String,
    customer_id: id_type::CustomerId,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum TokenStatus {
    Active,
    Inactive,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CheckTokenStatusResponsePayload {
    token_expiry_month: Secret<String>,
    token_expiry_year: Secret<String>,
    token_status: TokenStatus,
}

#[derive(Debug, Deserialize)]
pub struct CheckTokenStatusResponse {
    payload: CheckTokenStatusResponsePayload,
}

pub async fn mk_tokenization_req(
    state: &routes::SessionState,
    payload_bytes: &[u8],
    amount: String,
    currency: String,
    customer_id: id_type::CustomerId,
    tokenization_service: &NetworkTokenizationService,
) -> CustomResult<(CardNetworkTokenResponsePayload, Option<String>), errors::NetworkTokenizationError>
{
    let enc_key = tokenization_service.public_key.peek().clone();

    let key_id = tokenization_service.key_id.clone();

    let jwt = encryption::encrypt_jwe(
        payload_bytes,
        enc_key,
        EncryptionAlgorithm::A128GCM,
        Some(key_id.as_str()),
    )
    .await
    .change_context(errors::NetworkTokenizationError::SaveNetworkTokenFailed)
    .attach_printable("Error on jwe encrypt")?;

    let order_data = OrderData {
        consent_id: uuid::Uuid::new_v4().to_string(),
        customer_id,
        amount,
        currency,
    };

    let api_payload = ApiPayload {
        service: "NETWORK_TOKEN".to_string(),
        card_data: jwt,
        order_data,
        key_id,
        should_send_token: true,
        sub_merchant_id: "visa_sbx_working".to_string(),
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
    request.add_default_headers();

    request.set_body(RequestContent::Json(Box::new(api_payload)));

    logger::info!("Request to generate token: {:?}", request);

    let response = services::call_connector_api(state, request, "generate_token")
        .await
        .change_context(errors::NetworkTokenizationError::ResponseDeserializationFailed);

    let res = response
        .change_context(errors::NetworkTokenizationError::ResponseDeserializationFailed)
        .attach_printable("Error while receiving response")
        .and_then(|inner| match inner {
            Err(err_res) => {
                let parsed_error: NetworkTokenErrorResponse = err_res
                    .response
                    .parse_struct("Card Network Tokenization Response")
                    .change_context(
                        errors::NetworkTokenizationError::ResponseDeserializationFailed,
                    )?;
                logger::error!(
                    error_code = %parsed_error.error_info.code,
                    developer_message = %parsed_error.error_info.developer_message,
                    "Network tokenization error: {}",
                    parsed_error.error_message
                );
                Err(errors::NetworkTokenizationError::ResponseDeserializationFailed)
                    .attach_printable(format!("Response Deserialization Failed: {err_res:?}"))
            }
            Ok(res) => Ok(res),
        })
        .inspect_err(|err| {
            logger::error!("Error while deserializing response: {:?}", err);
        })?;

    let network_response: CardNetworkTokenResponse = res
        .response
        .parse_struct("Card Network Tokenization Response")
        .change_context(errors::NetworkTokenizationError::ResponseDeserializationFailed)?;
    logger::info!("Network Token Response: {:?}", network_response); //added for debugging

    let dec_key = tokenization_service.private_key.peek().clone();

    let card_network_token_response = services::decrypt_jwe(
        &network_response.payload,
        services::KeyIdCheck::SkipKeyIdCheck,
        dec_key,
        jwe::RSA_OAEP_256,
    )
    .await
    .change_context(errors::NetworkTokenizationError::SaveNetworkTokenFailed)
    .attach_printable(
        "Failed to decrypt the tokenization response from the tokenization service",
    )?;


    let cn_response: CardNetworkTokenResponsePayload =
        serde_json::from_str(&card_network_token_response)
            .change_context(errors::NetworkTokenizationError::ResponseDeserializationFailed)?;
    Ok((cn_response.clone(), Some(cn_response.card_reference)))
}

pub async fn make_card_network_tokenization_request(
    state: &routes::SessionState,
    card: &domain::Card,
    customer_id: &id_type::CustomerId,
    amount: Option<i64>,
    currency: Option<storage_enums::Currency>,
) -> CustomResult<(CardNetworkTokenResponsePayload, Option<String>), errors::NetworkTokenizationError>
{
    let card_data = CardData {
        card_number: card.card_number.clone(),
        exp_month: card.card_exp_month.clone(),
        exp_year: card.card_exp_year.clone(),
        card_security_code: card.card_cvc.clone(),
    };

    let payload = card_data
        .encode_to_string_of_json()
        .and_then(|x| x.encode_to_string_of_json())
        .change_context(errors::NetworkTokenizationError::RequestEncodingFailed)?;

    let payload_bytes = payload.as_bytes();
    let amount_str = amount.map_or_else(String::new, |a| a.to_string());
    let currency_str = currency.map_or_else(String::new, |c| c.to_string());
    if let Some(network_tokenization_service) = &state.conf.network_tokenization_service {
        mk_tokenization_req(
            state,
            payload_bytes,
            amount_str,
            currency_str,
            customer_id.clone(),
            network_tokenization_service.get_inner(),
        )
        .await
        .inspect_err(|e| logger::error!(error=?e, "Error while making tokenization request"))
    } else {
        Err(errors::NetworkTokenizationError::NetworkTokenizationServiceNotConfigured)
            .inspect_err(|_| {
                logger::error!("Network Tokenization Service not configured" );
            })
            .attach_printable("Network Tokenization Service not configured")
    }
}

pub async fn get_network_token(
    state: &routes::SessionState,
    customer_id: id_type::CustomerId,
    network_token_requestor_ref_id: String,
    tokenization_service: &NetworkTokenizationService,
) -> CustomResult<TokenResponse, errors::NetworkTokenizationError> {
    let mut request = services::Request::new(
        services::Method::Post,
        tokenization_service.fetch_token_url.as_str(),
    );
    let payload = GetCardToken {
        card_reference: network_token_requestor_ref_id,
        customer_id,
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
    request.add_default_headers();
    request.set_body(RequestContent::Json(Box::new(payload)));

    logger::info!("Request to fetch network token: {:?}", request);

    // Send the request using `call_connector_api`
    let response = services::call_connector_api(state, request, "get network token")
        .await
        .change_context(errors::NetworkTokenizationError::ResponseDeserializationFailed);

    let res = response
        .change_context(errors::NetworkTokenizationError::ResponseDeserializationFailed)
        .attach_printable("Error while receiving response")
        .and_then(|inner| match inner {
            Err(err_res) => {
                let parsed_error: NetworkTokenErrorResponse = err_res
                    .response
                    .parse_struct("Card Network Tokenization Response")
                    .change_context(
                        errors::NetworkTokenizationError::ResponseDeserializationFailed,
                    )?;
                logger::error!(
                    error_code = %parsed_error.error_info.code,
                    developer_message = %parsed_error.error_info.developer_message,
                    "Network tokenization error: {}",
                    parsed_error.error_message
                );
                Err(errors::NetworkTokenizationError::ResponseDeserializationFailed)
                    .attach_printable(format!("Response Deserialization Failed: {err_res:?}"))
            }
            Ok(res) => Ok(res),
        })
        .inspect_err(|err| {
            logger::error!("Error while deserializing response: {:?}", err);
        })?;

    let token_response: TokenResponse = res
        .response
        .parse_struct("Get Network Token Response")
        .change_context(errors::NetworkTokenizationError::ResponseDeserializationFailed)?;
    logger::info!("Fetch Network Token Response: {:?}", token_response);

    Ok(token_response)
}

pub async fn get_token_from_tokenization_service(
    state: &routes::SessionState,
    key_store: &domain::MerchantKeyStore,
    network_token_requestor_ref_id: String,
    pm_data: &storage::PaymentMethod,
) -> errors::RouterResult<NetworkTokenData> {
    let token_response =
        if let Some(network_tokenization_service) = &state.conf.network_tokenization_service {
            get_network_token(
            state,
            pm_data.customer_id.clone(),
            network_token_requestor_ref_id,
            network_tokenization_service.get_inner(),
        )
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Fetch network token failed")
        .inspect_err(
            |e| logger::error!(error=?e, "Error while fetching token from tokenization service"),
        )
        } else {
            Err(errors::NetworkTokenizationError::NetworkTokenizationServiceNotConfigured)
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .inspect_err(|_| {
                    logger::error!("Network Tokenization Service not configured" );
                })
        }?;

    let token_decrypted = domain::types::crypto_operation::<serde_json::Value, masking::WithType>(
        &state.into(),
        type_name!(payment_method::PaymentMethod),
        domain::types::CryptoOperation::DecryptOptional(
            pm_data.network_token_payment_method_data.clone(),
        ),
        Identifier::Merchant(key_store.merchant_id.clone()),
        key_store.key.get_inner().peek(),
    )
    .await
    .and_then(|val| val.try_into_optionaloperation())
    .change_context(errors::StorageError::DecryptionError)
    .change_context(errors::ApiErrorResponse::InternalServerError)
    .attach_printable("Failed to decrypt Network token details")?
    .map(|x| x.into_inner().expose())
    .map(
        |value| -> Result<PaymentMethodsData, error_stack::Report<errors::ApiErrorResponse>> {
            value
                .parse_value::<PaymentMethodsData>("PaymentMethodsData")
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Failed to deserialize payment methods data")
        },
    )
    .transpose()?
    .and_then(|pmd| match pmd {
        PaymentMethodsData::Card(token) => Some(api::CardDetailFromLocker::from(token)),
        _ => None,
    })
    .ok_or(errors::ApiErrorResponse::InternalServerError)
    .attach_printable("Failed to obtain decrypted token object from db")?;

    let network_token_data = NetworkTokenData {
        token_number: token_response.authentication_details.token,
        token_cryptogram: Some(token_response.authentication_details.cryptogram),
        token_exp_month: token_decrypted
            .expiry_month
            .unwrap_or(token_response.token_details.exp_month),
        token_exp_year: token_decrypted
            .expiry_year
            .unwrap_or(token_response.token_details.exp_year),
        nick_name: token_decrypted.card_holder_name,
        card_issuer: None,
        card_network: Some(token_response.network),
        card_type: None,
        card_issuing_country: None,
        bank_code: None,
    };
    Ok(network_token_data)
}

pub async fn do_status_check_for_network_token(
    state: &routes::SessionState,
    key_store: &domain::MerchantKeyStore,
    payment_method_info: &storage::PaymentMethod,
) -> CustomResult<(Option<Secret<String>>, Option<Secret<String>>), errors::ApiErrorResponse> {
    let key = key_store.key.get_inner().peek();
    let identifier = Identifier::Merchant(key_store.merchant_id.clone());
    let network_token_data_decrypted =
        domain::types::crypto_operation::<serde_json::Value, masking::WithType>(
            &state.into(),
            type_name!(payment_method::PaymentMethod),
            domain::types::CryptoOperation::DecryptOptional(
                payment_method_info
                    .network_token_payment_method_data
                    .clone(),
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
            PaymentMethodsData::Card(token) => Some(api::CardDetailFromLocker::from(token)),
            _ => None,
        });
    let network_token_requestor_reference_id = payment_method_info
        .network_token_requestor_reference_id
        .clone();
    if network_token_data_decrypted
        .and_then(|token_data| token_data.expiry_month.zip(token_data.expiry_year))
        .and_then(|(exp_month, exp_year)| helpers::validate_card_expiry(&exp_month, &exp_year).ok())
        .is_none()
    {
        if let Some(ref_id) = network_token_requestor_reference_id {
            if let Some(network_tokenization_service) = &state.conf.network_tokenization_service {
                let (token_exp_month, token_exp_year) =
                    check_token_status_with_tokenization_service(
                        state,
                        &payment_method_info.customer_id.clone(),
                        ref_id,
                        network_tokenization_service.get_inner(),
                    )
                    .await
                    .change_context(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable(
                        "Check network token status with tokenization service failed",
                    )?;
                Ok((token_exp_month, token_exp_year))
            } else {
                Err(errors::NetworkTokenizationError::NetworkTokenizationServiceNotConfigured)
                    .change_context(errors::ApiErrorResponse::InternalServerError)
                    .inspect_err(|_| {
                        logger::error!("Network Tokenization Service not configured" );
                    })
            }
        } else {
            Err(errors::NetworkTokenizationError::FetchNetworkTokenFailed)
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Check network token status failed")?
        }
    } else {
        Ok((None, None))
    }
}

pub async fn check_token_status_with_tokenization_service(
    state: &routes::SessionState,
    customer_id: &id_type::CustomerId,
    network_token_requestor_reference_id: String,
    tokenization_service: &NetworkTokenizationService,
) -> CustomResult<(Option<Secret<String>>, Option<Secret<String>>), errors::NetworkTokenizationError>
{
    let mut request = services::Request::new(
        services::Method::Post,
        tokenization_service.check_token_status_url.as_str(),
    );
    let payload = CheckTokenStatus {
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
    request.add_default_headers();
    request.set_body(RequestContent::Json(Box::new(payload)));

    // Send the request using `call_connector_api`
    let response = services::call_connector_api(state, request, "Check Network token Status")
        .await
        .change_context(errors::NetworkTokenizationError::ResponseDeserializationFailed);
    let res = response
        .change_context(errors::NetworkTokenizationError::ResponseDeserializationFailed)
        .attach_printable("Error while receiving response")
        .and_then(|inner| match inner {
            Err(err_res) => {
                let parsed_error: NetworkTokenErrorResponse = err_res
                    .response
                    .parse_struct("Delete Network Tokenization Response")
                    .change_context(
                        errors::NetworkTokenizationError::ResponseDeserializationFailed,
                    )?;
                logger::error!(
                    error_code = %parsed_error.error_info.code,
                    developer_message = %parsed_error.error_info.developer_message,
                    "Network tokenization error: {}",
                    parsed_error.error_message
                );
                Err(errors::NetworkTokenizationError::ResponseDeserializationFailed)
                    .attach_printable(format!("Response Deserialization Failed: {err_res:?}"))
            }
            Ok(res) => Ok(res),
        })
        .inspect_err(|err| {
            logger::error!("Error while deserializing response: {:?}", err);
        })?;

    let check_token_status_response: CheckTokenStatusResponse = res
        .response
        .parse_struct("Delete Network Tokenization Response")
        .change_context(errors::NetworkTokenizationError::ResponseDeserializationFailed)?;

    match check_token_status_response.payload.token_status {
        TokenStatus::Active => Ok((
            Some(check_token_status_response.payload.token_expiry_month),
            Some(check_token_status_response.payload.token_expiry_year),
        )),
        TokenStatus::Inactive => Ok((None, None)),
    }
}

pub async fn delete_network_token_from_locker_and_token_service(
    state: &routes::SessionState,
    customer_id: &id_type::CustomerId,
    merchant_id: &id_type::MerchantId,
    payment_method_id: String,
    network_token_locker_id: Option<String>,
    network_token_requestor_reference_id: String,
) -> errors::RouterResult<DeleteCardResp> {
    //deleting network token from locker
    let resp = payment_methods::cards::delete_card_from_locker(
        state,
        customer_id,
        merchant_id,
        network_token_locker_id
            .as_ref()
            .unwrap_or(&payment_method_id),
    )
    .await?;
    if let Some(tokenization_service) = &state.conf.network_tokenization_service {
        if delete_network_token_from_tokenization_service(
            state,
            network_token_requestor_reference_id,
            customer_id,
            tokenization_service.get_inner(),
        )
        .await
        .is_ok()
        {
            logger::info!("Token From Tokenization Service deleted Successfully!");
        } else {
            logger::error!("Error while deleting Token From Tokenization Service!");
        }
    };

    Ok(resp)
}

pub async fn delete_network_token_from_tokenization_service(
    state: &routes::SessionState,
    network_token_requestor_reference_id: String,
    customer_id: &id_type::CustomerId,
    tokenization_service: &NetworkTokenizationService,
) -> CustomResult<bool, errors::NetworkTokenizationError> {
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
    request.add_default_headers();
    request.set_body(RequestContent::Json(Box::new(payload)));

    logger::info!("Request to delete network token: {:?}", request);

    // Send the request using `call_connector_api`
    let response = services::call_connector_api(state, request, "delete network token")
        .await
        .change_context(errors::NetworkTokenizationError::DeleteNetworkTokenFailed);
    let res = response
        .change_context(errors::NetworkTokenizationError::ResponseDeserializationFailed)
        .attach_printable("Error while receiving response")
        .and_then(|inner| match inner {
            Err(err_res) => {
                let parsed_error: NetworkTokenErrorResponse = err_res
                    .response
                    .parse_struct("Delete Network Tokenization Response")
                    .change_context(
                        errors::NetworkTokenizationError::ResponseDeserializationFailed,
                    )?;
                logger::error!(
                    error_code = %parsed_error.error_info.code,
                    developer_message = %parsed_error.error_info.developer_message,
                    "Network tokenization error: {}",
                    parsed_error.error_message
                );
                Err(errors::NetworkTokenizationError::ResponseDeserializationFailed)
                    .attach_printable(format!("Response Deserialization Failed: {err_res:?}"))
            }
            Ok(res) => Ok(res),
        })
        .inspect_err(|err| {
            logger::error!("Error while deserializing response: {:?}", err);
        })?;

    let delete_token_response: DeleteNTResponse = res
        .response
        .parse_struct("Delete Network Tokenization Response")
        .change_context(errors::NetworkTokenizationError::ResponseDeserializationFailed)?;

    logger::info!("Delete Network Token Response: {:?}", delete_token_response);

    if delete_token_response
        == DeleteNTResponse::DeleteNetworkTokenResponse(DeleteNetworkTokenResponse {
            status: DeleteNetworkTokenStatus::Success,
        })
    {
        Ok(true)
    } else {
        Err(errors::NetworkTokenizationError::DeleteNetworkTokenFailed)
            .attach_printable("Delete Token at Token service failed")
    }
}
