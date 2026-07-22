#[cfg(feature = "v2")]
use std::fmt::Debug;
#[cfg(feature = "v2")]
use std::str::FromStr;

use ::payment_methods::controller::PaymentMethodsController;
#[cfg(feature = "v1")]
use ::payment_methods::types as ext_pm_types;
use api_models::payment_methods as api_payment_methods;
#[cfg(feature = "v2")]
use cards::{CardNumber, NetworkToken};
use common_utils::{
    errors::CustomResult,
    ext_traits::{BytesExt, Encode},
    id_type,
    metrics::utils::record_operation_time,
    request::RequestContent,
};
use error_stack::{report, ResultExt};
#[cfg(feature = "v2")]
use hyperswitch_domain_models::payment_method_data::{
    NetworkTokenDetails, NetworkTokenDetailsPaymentMethod,
};
use hyperswitch_masking::{ErasedMaskSerialize, ExposeInterface, Mask, PeekInterface, Secret};
use josekit::jwe;

use super::transformers::DeleteCardResp;
use crate::{
    core::{errors, payment_methods, payments::helpers},
    headers, logger,
    routes::{self, metrics},
    services::{self, encryption},
    settings,
    types::{api, domain, payment_methods as pm_types, Response},
    utils::ext_traits::OptionExt,
};

pub const NETWORK_TOKEN_SERVICE: &str = "NETWORK_TOKEN";

#[derive(Debug, Clone)]
pub enum AltIdDecision {
    Proceed, // Fetch Alt-ID for this transaction
    Skip, // Transaction not eligible for Alt-ID (merchant/card not Indian, or network/connector not supported)
    Error, // RBI compliance violation
}

impl AltIdDecision {
    pub fn evaluate(
        state: &routes::SessionState,
        card: &domain::Card,
        business_profile: &domain::Profile,
        connector: api_models::enums::Connector,
    ) -> Self {
        match (
            business_profile.is_alt_id_eligible_merchant(),
            card.is_indian_issued_card(),
        ) {
            (true, true) => {
                let alt_id_required = card.card_network.as_ref().is_some_and(|network| {
                    state
                        .conf
                        .alt_id_required_card_networks_and_connector
                        .networks
                        .get(network)
                        .map(|connectors| connectors.contains(&connector))
                        .unwrap_or(false)
                });

                match (
                    alt_id_required,
                    business_profile.is_network_tokenization_enabled,
                ) {
                    (true, true) => Self::Proceed,
                    (true, false) => Self::Error,
                    (false, _) => Self::Skip,
                }
            }
            (false, _) | (_, false) => Self::Skip,
        }
    }
}

async fn call_network_token_service(
    state: &routes::SessionState,
    tokenization_service: &settings::NetworkTokenizationService,
    method: services::Method,
    url: &str,
    body: Option<RequestContent>,
    operation_tag: &str,
) -> CustomResult<Result<Response, Response>, errors::NetworkTokenizationError> {
    let mut request = services::Request::new(method, url);
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

    if let Some(body_content) = body {
        request.set_body(body_content);
    }

    logger::info!(
        "Network token service request [{}]: {:?}",
        operation_tag,
        request
    );

    services::call_connector_api(state, request, operation_tag)
        .await
        .change_context(errors::NetworkTokenizationError::ApiError)
}

#[cfg(feature = "v1")]
pub async fn mk_tokenization_req(
    state: &routes::SessionState,
    payload_bytes: &[u8],
    customer_id: id_type::CustomerId,
    tokenization_service: &settings::NetworkTokenizationService,
) -> CustomResult<
    (pm_types::CardNetworkTokenResponsePayload, Option<String>),
    errors::NetworkTokenizationError,
> {
    let enc_key = tokenization_service.public_key.peek().clone();

    let key_id = tokenization_service.key_id.clone();

    let jwt = encryption::encrypt_jwe(
        payload_bytes,
        enc_key,
        services::EncryptionAlgorithm::A128GCM,
        Some(key_id.as_str()),
    )
    .await
    .change_context(errors::NetworkTokenizationError::SaveNetworkTokenFailed)
    .attach_printable("Error on jwe encrypt")?;

    let order_data = pm_types::OrderData {
        consent_id: uuid::Uuid::new_v4().to_string(),
        customer_id,
    };

    let api_payload = pm_types::ApiPayload {
        service: NETWORK_TOKEN_SERVICE.to_string(),
        card_data: Secret::new(jwt),
        order_data,
        should_send_token: true,
        key_id: Secret::new(key_id.clone()),
    };
    let masked_request_body = api_payload
        .masked_serialize()
        .inspect_err(|e| logger::error!(error=?e, "failed to mask serialize"))
        .unwrap_or(serde_json::json!({ "error": "failed to mask serialize"}));
    logger::info!(raw_network_token_service_request=?masked_request_body);

    let response = call_network_token_service(
        state,
        tokenization_service,
        services::Method::Post,
        tokenization_service.generate_token_url.as_str(),
        Some(RequestContent::Json(Box::new(api_payload))),
        "generate_token",
    )
    .await;

    let res = response
        .change_context(errors::NetworkTokenizationError::ResponseDeserializationFailed)
        .attach_printable("Error while receiving response")
        .and_then(|inner| match inner {
            Err(err_res) => {
                let parsed_error: pm_types::NetworkTokenErrorResponse = err_res
                    .response
                    .parse_struct("Card Network Tokenization Response")
                    .change_context(
                        errors::NetworkTokenizationError::ResponseDeserializationFailed,
                    )?;
                logger::error!(
                    error_code = %parsed_error.error_info.code,
                    developer_message = %parsed_error.error_info.developer_message,
                    "Network tokenization error: {:?}",
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

    let network_response: pm_types::CardNetworkTokenResponse = res
        .response
        .parse_struct("Card Network Tokenization Response")
        .change_context(errors::NetworkTokenizationError::ResponseDeserializationFailed)?;

    let dec_key = tokenization_service.private_key.peek().clone();

    let card_network_token_response = services::decrypt_jwe(
        network_response.payload.peek(),
        services::KeyIdCheck::SkipKeyIdCheck,
        dec_key,
        jwe::RSA_OAEP_256,
    )
    .await
    .change_context(errors::NetworkTokenizationError::SaveNetworkTokenFailed)
    .attach_printable(
        "Failed to decrypt the tokenization response from the tokenization service",
    )?;

    let cn_response: pm_types::CardNetworkTokenResponsePayload =
        serde_json::from_str(&card_network_token_response)
            .change_context(errors::NetworkTokenizationError::ResponseDeserializationFailed)?;
    Ok((cn_response.clone(), Some(cn_response.card_reference)))
}

#[cfg(feature = "v2")]
pub async fn make_nt_eligibility_call(
    state: &routes::SessionState,
    payload: api_payment_methods::NetworkTokenEligibilityRequest,
) -> CustomResult<pm_types::NTEligibilityResponse, errors::NetworkTokenizationError> {
    let tokenization_service = match &state.conf.network_tokenization_service {
        Some(nt_service) => Ok(nt_service.get_inner()),
        None => Err(report!(
            errors::NetworkTokenizationError::NetworkTokenizationServiceNotConfigured
        )),
    }?;

    let url_string = format!(
        "{}/{}?options.check_tokenize_support={}",
        tokenization_service.check_tokenize_eligibility_url.as_str(),
        payload.card_bin.clone(),
        true
    );

    let response = call_network_token_service(
        state,
        tokenization_service,
        services::Method::Get,
        &url_string,
        None,
        "fetch_nt_eligibility",
    )
    .await;

    let res = response
        .change_context(errors::NetworkTokenizationError::ResponseDeserializationFailed)
        .attach_printable("Error while receiving response")
        .and_then(|inner| match inner {
            Err(err_res) => {
                logger::error!("Error response from nt eligibility call: {:?}", err_res);
                let parsed_error: pm_types::NetworkTokenErrorResponse = err_res
                    .response
                    .parse_struct("Card Network Tokenization Response")
                    .change_context(
                        errors::NetworkTokenizationError::ResponseDeserializationFailed,
                    )?;
                logger::error!(
                    error_code = %parsed_error.error_info.code,
                    developer_message = %parsed_error.error_info.developer_message,
                    "Network tokenization error: {:?}",
                    parsed_error.error_message
                );
                Err(errors::NetworkTokenizationError::ApiError).attach_printable(format!(
                    "Network Tokenization ApiError : {:?}",
                    parsed_error.error_info.code
                ))
            }
            Ok(res) => Ok(res),
        })
        .inspect_err(|err| {
            logger::error!("Error while deserializing response: {:?}", err);
        })?;

    let network_response: pm_types::NTEligibilityResponse = res
        .response
        .parse_struct("NTEligibilityResponse")
        .change_context(errors::NetworkTokenizationError::ResponseDeserializationFailed)?;
    logger::debug!("Network Token Response: {:?}", network_response);

    Ok(network_response)
}

#[cfg(feature = "v2")]
pub async fn generate_network_token(
    state: &routes::SessionState,
    payload_bytes: &[u8],
    customer_id: id_type::GlobalCustomerId,
    tokenization_service: &settings::NetworkTokenizationService,
) -> CustomResult<
    (pm_types::GenerateNetworkTokenResponsePayload, String),
    errors::NetworkTokenizationError,
> {
    let enc_key = tokenization_service.public_key.peek().clone();

    let key_id = tokenization_service.key_id.clone();

    let jwt = encryption::encrypt_jwe(
        payload_bytes,
        enc_key,
        services::EncryptionAlgorithm::A128GCM,
        Some(key_id.as_str()),
    )
    .await
    .change_context(errors::NetworkTokenizationError::SaveNetworkTokenFailed)
    .attach_printable("Error on jwe encrypt")?;

    let order_data = pm_types::OrderData {
        consent_id: uuid::Uuid::new_v4().to_string(),
        customer_id,
    };

    let api_payload = pm_types::ApiPayload {
        service: NETWORK_TOKEN_SERVICE.to_string(),
        card_data: Secret::new(jwt),
        order_data,
        should_send_token: true,
        key_id: Secret::new(key_id.clone()),
    };
    let masked_request_body = api_payload
        .masked_serialize()
        .inspect_err(|e| logger::error!(error=?e, "failed to mask serialize"))
        .unwrap_or(serde_json::json!({ "error": "failed to mask serialize"}));
    logger::info!(raw_network_token_service_request=?masked_request_body);

    let response = call_network_token_service(
        state,
        tokenization_service,
        services::Method::Post,
        tokenization_service.generate_token_url.as_str(),
        Some(RequestContent::Json(Box::new(api_payload))),
        "generate_token",
    )
    .await;

    let res = response
        .change_context(errors::NetworkTokenizationError::ResponseDeserializationFailed)
        .attach_printable("Error while receiving response")
        .and_then(|inner| match inner {
            Err(err_res) => {
                let parsed_error: pm_types::NetworkTokenErrorResponse = err_res
                    .response
                    .parse_struct("Card Network Tokenization Response")
                    .change_context(
                        errors::NetworkTokenizationError::ResponseDeserializationFailed,
                    )?;
                logger::error!(
                    error_code = %parsed_error.error_info.code,
                    developer_message = %parsed_error.error_info.developer_message,
                    "Network tokenization error: {:?}",
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

    let network_response: pm_types::CardNetworkTokenResponse = res
        .response
        .parse_struct("Card Network Tokenization Response")
        .change_context(errors::NetworkTokenizationError::ResponseDeserializationFailed)?;
    logger::debug!("Network Token Response: {:?}", network_response);

    let dec_key = tokenization_service.private_key.peek().clone();

    let card_network_token_response = services::decrypt_jwe(
        network_response.payload.peek(),
        services::KeyIdCheck::SkipKeyIdCheck,
        dec_key,
        jwe::RSA_OAEP_256,
    )
    .await
    .change_context(errors::NetworkTokenizationError::SaveNetworkTokenFailed)
    .attach_printable(
        "Failed to decrypt the tokenization response from the tokenization service",
    )?;

    let cn_response: pm_types::GenerateNetworkTokenResponsePayload =
        serde_json::from_str(&card_network_token_response)
            .change_context(errors::NetworkTokenizationError::ResponseDeserializationFailed)?;
    Ok((cn_response.clone(), cn_response.card_reference))
}

#[cfg(feature = "v1")]
pub async fn make_card_network_tokenization_request(
    state: &routes::SessionState,
    card: &domain::CardDetail,
    optional_cvc: Option<Secret<String>>,
    customer_id: &id_type::CustomerId,
) -> CustomResult<
    (pm_types::CardNetworkTokenResponsePayload, Option<String>),
    errors::NetworkTokenizationError,
> {
    let card_data = pm_types::CardData {
        card_number: card.card_number.clone(),
        exp_month: card.card_exp_month.clone(),
        exp_year: card.card_exp_year.clone(),
        card_security_code: optional_cvc,
    };

    let payload = card_data
        .encode_to_string_of_json()
        .and_then(|x| x.encode_to_string_of_json())
        .change_context(errors::NetworkTokenizationError::RequestEncodingFailed)?;

    let payload_bytes = payload.as_bytes();
    if let Some(network_tokenization_service) = &state.conf.network_tokenization_service {
        record_operation_time(
            async {
                mk_tokenization_req(
                    state,
                    payload_bytes,
                    customer_id.clone(),
                    network_tokenization_service.get_inner(),
                )
                .await
                .inspect_err(
                    |e| logger::error!(error=?e, "Error while making tokenization request"),
                )
            },
            &metrics::GENERATE_NETWORK_TOKEN_TIME,
            router_env::metric_attributes!(("locker", "rust")),
        )
        .await
    } else {
        Err(errors::NetworkTokenizationError::NetworkTokenizationServiceNotConfigured)
            .inspect_err(|_| {
                logger::error!("Network Tokenization Service not configured");
            })
            .attach_printable("Network Tokenization Service not configured")
    }
}

#[cfg(feature = "v2")]
pub async fn make_card_network_tokenization_request(
    state: &routes::SessionState,
    card: &api_payment_methods::CardDetail,
    customer_id: &id_type::GlobalCustomerId,
) -> CustomResult<(NetworkTokenDetails, String), errors::NetworkTokenizationError> {
    let card_data = pm_types::CardData {
        card_number: card.card_number.clone(),
        exp_month: card.card_exp_month.clone(),
        exp_year: card.card_exp_year.clone(),
        card_security_code: None,
    };

    let payload = card_data
        .encode_to_string_of_json()
        .and_then(|x| x.encode_to_string_of_json())
        .change_context(errors::NetworkTokenizationError::RequestEncodingFailed)?;

    let payload_bytes = payload.as_bytes();
    let network_tokenization_service = match &state.conf.network_tokenization_service {
        Some(nt_service) => Ok(nt_service.get_inner()),
        None => Err(report!(
            errors::NetworkTokenizationError::NetworkTokenizationServiceNotConfigured
        )),
    }?;

    let (resp, network_token_req_ref_id) = record_operation_time(
        async {
            generate_network_token(
                state,
                payload_bytes,
                customer_id.clone(),
                network_tokenization_service,
            )
            .await
            .inspect_err(|e| logger::error!(error=?e, "Error while making tokenization request"))
        },
        &metrics::GENERATE_NETWORK_TOKEN_TIME,
        router_env::metric_attributes!(("locker", "rust")),
    )
    .await?;

    let network_token_details = NetworkTokenDetails {
        network_token: resp.token,
        network_token_exp_month: resp.token_expiry_month,
        network_token_exp_year: resp.token_expiry_year,
        card_issuer: card.card_issuer.clone(),
        card_network: Some(resp.card_brand),
        card_type: card.card_type.clone(),
        card_issuing_country: card.card_issuing_country,
        card_holder_name: card.card_holder_name.clone(),
        nick_name: card.nick_name.clone(),
        cryptogram: None,
        par: Some(Secret::new(resp.par)),
    };
    Ok((network_token_details, network_token_req_ref_id))
}

#[cfg(feature = "v1")]
pub async fn get_network_token(
    state: &routes::SessionState,
    customer_id: id_type::CustomerId,
    network_token_requestor_ref_id: String,
    tokenization_service: &settings::NetworkTokenizationService,
) -> CustomResult<pm_types::TokenResponse, errors::NetworkTokenizationError> {
    let payload = pm_types::GetCardToken {
        card_reference: network_token_requestor_ref_id,
        customer_id,
    };

    let masked_request_body = payload
        .masked_serialize()
        .inspect_err(|e| logger::error!(error=?e, "failed to mask serialize"))
        .unwrap_or(serde_json::json!({ "error": "failed to mask serialize"}));
    logger::info!(raw_network_token_service_request=?masked_request_body);

    let response = call_network_token_service(
        state,
        tokenization_service,
        services::Method::Post,
        tokenization_service.fetch_token_url.as_str(),
        Some(RequestContent::Json(Box::new(payload))),
        "get_network_token",
    )
    .await;

    let res = response
        .change_context(errors::NetworkTokenizationError::ResponseDeserializationFailed)
        .attach_printable("Error while receiving response")
        .and_then(|inner| match inner {
            Err(err_res) => {
                let parsed_error: pm_types::NetworkTokenErrorResponse = err_res
                    .response
                    .parse_struct("Card Network Tokenization Response")
                    .change_context(
                        errors::NetworkTokenizationError::ResponseDeserializationFailed,
                    )?;
                logger::error!(
                    error_code = %parsed_error.error_info.code,
                    developer_message = %parsed_error.error_info.developer_message,
                    "Network tokenization error: {:?}",
                    parsed_error.error_message
                );
                Err(errors::NetworkTokenizationError::ResponseDeserializationFailed)
                    .attach_printable(format!("Response Deserialization Failed: {err_res:?}"))
            }
            Ok(res) => Ok(res),
        })?;

    let token_response: pm_types::TokenResponse = res
        .response
        .parse_struct("Get Network Token Response")
        .change_context(errors::NetworkTokenizationError::ResponseDeserializationFailed)?;
    logger::info!("Fetch Network Token Response: {:?}", token_response);

    Ok(token_response)
}

#[cfg(feature = "v2")]
pub async fn get_network_token(
    state: &routes::SessionState,
    customer_id: &id_type::GlobalCustomerId,
    network_token_requestor_ref_id: String,
    tokenization_service: &settings::NetworkTokenizationService,
) -> CustomResult<pm_types::TokenResponse, errors::NetworkTokenizationError> {
    let payload = pm_types::GetCardToken {
        card_reference: network_token_requestor_ref_id,
        customer_id: customer_id.clone(),
    };

    let masked_request_body = payload
        .masked_serialize()
        .inspect_err(|e| logger::error!(error=?e, "failed to mask serialize"))
        .unwrap_or(serde_json::json!({ "error": "failed to mask serialize"}));
    logger::info!(raw_network_token_service_request=?masked_request_body);

    let response = call_network_token_service(
        state,
        tokenization_service,
        services::Method::Post,
        tokenization_service.fetch_token_url.as_str(),
        Some(RequestContent::Json(Box::new(payload))),
        "get_network_token",
    )
    .await;

    let res = response
        .change_context(errors::NetworkTokenizationError::ResponseDeserializationFailed)
        .attach_printable("Error while receiving response")
        .and_then(|inner| match inner {
            Err(err_res) => {
                let parsed_error: pm_types::NetworkTokenErrorResponse = err_res
                    .response
                    .parse_struct("Card Network Tokenization Response")
                    .change_context(
                        errors::NetworkTokenizationError::ResponseDeserializationFailed,
                    )?;
                logger::error!(
                    error_code = %parsed_error.error_info.code,
                    developer_message = %parsed_error.error_info.developer_message,
                    "Network tokenization error: {:?}",
                    parsed_error.error_message
                );
                Err(errors::NetworkTokenizationError::ResponseDeserializationFailed)
                    .attach_printable(format!("Response Deserialization Failed: {err_res:?}"))
            }
            Ok(res) => Ok(res),
        })?;

    let token_response: pm_types::TokenResponse = res
        .response
        .parse_struct("Get Network Token Response")
        .change_context(errors::NetworkTokenizationError::ResponseDeserializationFailed)?;
    logger::info!("Fetch Network Token Response: {:?}", token_response);

    Ok(token_response)
}

#[cfg(feature = "v1")]
pub async fn get_token_from_tokenization_service(
    state: &routes::SessionState,
    network_token_requestor_ref_id: String,
    pm_data: &domain::PaymentMethod,
) -> errors::RouterResult<domain::NetworkTokenData> {
    let token_response =
        if let Some(network_tokenization_service) = &state.conf.network_tokenization_service {
            record_operation_time(
                async {
                    get_network_token(
                state,
                pm_data.customer_id.clone().get_required_value("customer_id")?,
                network_token_requestor_ref_id,
                network_tokenization_service.get_inner(),
            )
            .await
            .inspect_err(
                |e| logger::error!(error=?e, "Error while fetching token from tokenization service")
            )
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Fetch network token failed")
                },
                &metrics::FETCH_NETWORK_TOKEN_TIME,
                &[],
            )
            .await
        } else {
            Err(errors::NetworkTokenizationError::NetworkTokenizationServiceNotConfigured)
                .inspect_err(|err| {
                    logger::error!(error=? err);
                })
                .change_context(errors::ApiErrorResponse::InternalServerError)
        }?;

    let token_decrypted = pm_data
        .network_token_payment_method_data
        .clone()
        .map(|x| x.into_inner().expose())
        .and_then(|v| serde_json::from_value::<api_payment_methods::PaymentMethodsData>(v).ok())
        .and_then(|pmd| match pmd {
            api_payment_methods::PaymentMethodsData::Card(token) => {
                Some(api::CardDetailFromLocker::from(token))
            }
            _ => None,
        })
        .ok_or(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to obtain decrypted token object from db")?;

    let network_token_data = domain::NetworkTokenData {
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
        eci: token_response.eci,
        par: token_response.card_details.map(|details| details.par),
    };
    Ok(network_token_data)
}

#[cfg(feature = "v2")]
pub async fn get_token_from_tokenization_service(
    state: &routes::SessionState,
    network_token_requestor_ref_id: String,
    pm_data: &domain::PaymentMethod,
) -> errors::RouterResult<domain::NetworkTokenData> {
    let customer_id = &pm_data
        .customer_id
        .clone()
        .get_required_value("GlobalCustomerId")?;
    let token_response =
        if let Some(network_tokenization_service) = &state.conf.network_tokenization_service {
            record_operation_time(
                async {
                    get_network_token(
                state,
                customer_id,
                network_token_requestor_ref_id,
                network_tokenization_service.get_inner(),
            )
            .await
            .inspect_err(
                |e| logger::error!(error=?e, "Error while fetching token from tokenization service")
            )
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Fetch network token failed")
                },
                &metrics::FETCH_NETWORK_TOKEN_TIME,
                &[],
            )
            .await
        } else {
            Err(errors::NetworkTokenizationError::NetworkTokenizationServiceNotConfigured)
                .inspect_err(|err| {
                    logger::error!(error=? err);
                })
                .change_context(errors::ApiErrorResponse::InternalServerError)
        }?;

    let token_decrypted = pm_data
        .network_token_payment_method_data
        .clone()
        .map(|value| value.into_inner())
        .and_then(|payment_method_data| match payment_method_data {
            hyperswitch_domain_models::payment_method_data::PaymentMethodsData::NetworkToken(
                token,
            ) => Some(token),
            _ => None,
        })
        .ok_or(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to obtain decrypted token object from db")?;

    let network_token_data = domain::NetworkTokenData {
        network_token: token_response.authentication_details.token,
        cryptogram: Some(token_response.authentication_details.cryptogram),
        network_token_exp_month: token_decrypted
            .network_token_expiry_month
            .unwrap_or(token_response.token_details.exp_month),
        network_token_exp_year: token_decrypted
            .network_token_expiry_year
            .unwrap_or(token_response.token_details.exp_year),
        card_holder_name: token_decrypted.card_holder_name,
        nick_name: token_decrypted.nick_name.or(token_response.nickname),
        card_issuer: token_decrypted.card_issuer.or(token_response.issuer),
        card_network: Some(token_response.network),
        card_type: token_decrypted
            .card_type
            .or(token_response.card_type)
            .as_ref()
            .map(|c| api_payment_methods::CardType::from_str(c))
            .transpose()
            .ok()
            .flatten(),
        card_issuing_country: token_decrypted.issuer_country,
        bank_code: None,
        eci: token_response.eci,
        par: token_response.card_details.map(|details| details.par),
    };
    Ok(network_token_data)
}

#[cfg(feature = "v1")]
pub async fn do_status_check_for_network_token(
    state: &routes::SessionState,
    payment_method_info: &domain::PaymentMethod,
) -> CustomResult<(Option<Secret<String>>, Option<Secret<String>>), errors::ApiErrorResponse> {
    let network_token_data_decrypted = payment_method_info
        .network_token_payment_method_data
        .clone()
        .map(|x| x.into_inner().expose())
        .and_then(|v| serde_json::from_value::<api_payment_methods::PaymentMethodsData>(v).ok())
        .and_then(|pmd| match pmd {
            api_payment_methods::PaymentMethodsData::Card(token) => {
                Some(api::CardDetailFromLocker::from(token))
            }
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
                let (token_exp_month, token_exp_year) = record_operation_time(
                    async {
                        check_token_status_with_tokenization_service(
                            state,
                            &payment_method_info.customer_id.clone().get_required_value("customer_id")?,
                            ref_id,
                            network_tokenization_service.get_inner(),
                        )
                        .await
                        .inspect_err(
                            |e| logger::error!(error=?e, "Error while fetching token from tokenization service")
                        )
                        .change_context(errors::ApiErrorResponse::InternalServerError)
                        .attach_printable(
                            "Check network token status with tokenization service failed",
                        )
                    },
                    &metrics::CHECK_NETWORK_TOKEN_STATUS_TIME,

                    &[],
                )
                .await?;
                Ok((token_exp_month, token_exp_year))
            } else {
                Err(errors::NetworkTokenizationError::NetworkTokenizationServiceNotConfigured)
                    .change_context(errors::ApiErrorResponse::InternalServerError)
                    .inspect_err(|_| {
                        logger::error!("Network Tokenization Service not configured");
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

#[cfg(feature = "v1")]
pub async fn check_token_status_with_tokenization_service(
    state: &routes::SessionState,
    customer_id: &id_type::CustomerId,
    network_token_requestor_reference_id: String,
    tokenization_service: &settings::NetworkTokenizationService,
) -> CustomResult<(Option<Secret<String>>, Option<Secret<String>>), errors::NetworkTokenizationError>
{
    let payload = pm_types::CheckTokenStatus {
        card_reference: network_token_requestor_reference_id,
        customer_id: customer_id.clone(),
    };

    let masked_request_body = payload
        .masked_serialize()
        .inspect_err(|e| logger::error!(error=?e, "failed to mask serialize"))
        .unwrap_or(serde_json::json!({ "error": "failed to mask serialize"}));
    logger::info!(raw_network_token_service_request=?masked_request_body);

    let response = call_network_token_service(
        state,
        tokenization_service,
        services::Method::Post,
        tokenization_service.check_token_status_url.as_str(),
        Some(RequestContent::Json(Box::new(payload))),
        "check_token_status",
    )
    .await;
    let res = response
        .change_context(errors::NetworkTokenizationError::ResponseDeserializationFailed)
        .attach_printable("Error while receiving response")
        .and_then(|inner| match inner {
            Err(err_res) => {
                let parsed_error: pm_types::NetworkTokenErrorResponse = err_res
                    .response
                    .parse_struct("Delete Network Tokenization Response")
                    .change_context(
                        errors::NetworkTokenizationError::ResponseDeserializationFailed,
                    )?;
                logger::error!(
                    error_code = %parsed_error.error_info.code,
                    developer_message = %parsed_error.error_info.developer_message,
                    "Network tokenization error: {:?}",
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

    let check_token_status_response: pm_types::CheckTokenStatusResponse = res
        .response
        .parse_struct("Delete Network Tokenization Response")
        .change_context(errors::NetworkTokenizationError::ResponseDeserializationFailed)?;

    match check_token_status_response.payload.token_status {
        pm_types::TokenStatus::Active => Ok((
            check_token_status_response.payload.token_expiry_month,
            check_token_status_response.payload.token_expiry_year,
        )),
        _ => Ok((None, None)),
    }
}

#[cfg(feature = "v2")]
pub async fn check_token_status_with_tokenization_service(
    state: &routes::SessionState,
    customer_id: &id_type::GlobalCustomerId,
    network_token_requestor_reference_id: String,
    tokenization_service: &settings::NetworkTokenizationService,
) -> CustomResult<pm_types::CheckTokenStatusResponse, errors::NetworkTokenizationError> {
    let payload = pm_types::CheckTokenStatus {
        card_reference: network_token_requestor_reference_id,
        customer_id: customer_id.clone(),
    };

    let masked_request_body = payload
        .masked_serialize()
        .inspect_err(|e| logger::error!(error=?e, "failed to mask serialize"))
        .unwrap_or(serde_json::json!({ "error": "failed to mask serialize"}));
    logger::info!(raw_network_token_service_request=?masked_request_body);

    let response = call_network_token_service(
        state,
        tokenization_service,
        services::Method::Post,
        tokenization_service.check_token_status_url.as_str(),
        Some(RequestContent::Json(Box::new(payload))),
        "check_token_status",
    )
    .await;
    let res = response
        .change_context(errors::NetworkTokenizationError::ResponseDeserializationFailed)
        .attach_printable("Error while receiving response")
        .and_then(|inner| match inner {
            Err(err_res) => {
                let parsed_error: pm_types::NetworkTokenErrorResponse = err_res
                    .response
                    .parse_struct("Network Tokenization Error Response")
                    .change_context(
                        errors::NetworkTokenizationError::ResponseDeserializationFailed,
                    )?;
                logger::error!(
                    error_code = %parsed_error.error_info.code,
                    developer_message = %parsed_error.error_info.developer_message,
                    "Network tokenization error: {:?}",
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

    let check_token_status_response: pm_types::CheckTokenStatusResponse = res
        .response
        .parse_struct("CheckTokenStatusResponse")
        .change_context(errors::NetworkTokenizationError::ResponseDeserializationFailed)?;

    Ok(check_token_status_response)
}

#[cfg(feature = "v2")]
pub async fn do_status_check_for_network_token(
    state: &routes::SessionState,
    payment_method_info: &domain::PaymentMethod,
) -> CustomResult<pm_types::CheckTokenStatusResponse, errors::ApiErrorResponse> {
    let network_token_requestor_reference_id = payment_method_info
        .network_token_requestor_reference_id
        .clone();

    let customer_id = &payment_method_info
        .customer_id
        .clone()
        .get_required_value("GlobalCustomerId")?;
    if let Some(ref_id) = network_token_requestor_reference_id {
        if let Some(network_tokenization_service) = &state.conf.network_tokenization_service {
            let network_token_details = record_operation_time(
                async {
                    check_token_status_with_tokenization_service(
                        state,
                        customer_id,
                        ref_id,
                        network_tokenization_service.get_inner(),
                    )
                    .await
                    .inspect_err(
                        |e| logger::error!(error=?e, "Error while fetching token from tokenization service")
                    )
                    .change_context(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable(
                    "Check network token status with tokenization service failed",
                    )
                },
                &metrics::CHECK_NETWORK_TOKEN_STATUS_TIME,
                &[],
            )
            .await?;
            Ok(network_token_details)
        } else {
            Err(errors::NetworkTokenizationError::NetworkTokenizationServiceNotConfigured)
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Network Tokenization Service not configured")
                .inspect_err(|_| {
                    logger::error!("Network Tokenization Service not configured");
                })
        }
    } else {
        Err(errors::NetworkTokenizationError::FetchNetworkTokenFailed)
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Check network token status failed")?
    }
}

#[cfg(feature = "v1")]
pub async fn delete_network_token_from_locker_and_token_service(
    state: &routes::SessionState,
    customer_id: &id_type::CustomerId,
    merchant_id: &id_type::MerchantId,
    payment_method_id: String,
    network_token_locker_id: Option<String>,
    network_token_requestor_reference_id: String,
    provider: &domain::Provider,
) -> errors::RouterResult<DeleteCardResp> {
    //deleting network token from locker
    let resp = payment_methods::cards::PmCards { state, provider }
        .delete_card_from_locker(
            customer_id,
            merchant_id,
            network_token_locker_id
                .as_ref()
                .unwrap_or(&payment_method_id),
        )
        .await?;
    if let Some(tokenization_service) = &state.conf.network_tokenization_service {
        let delete_token_resp = record_operation_time(
            async {
                delete_network_token_from_tokenization_service(
                    state,
                    network_token_requestor_reference_id,
                    customer_id,
                    tokenization_service.get_inner(),
                )
                .await
            },
            &metrics::DELETE_NETWORK_TOKEN_TIME,
            &[],
        )
        .await;
        match delete_token_resp {
            Ok(_) => logger::info!("Token From Tokenization Service deleted Successfully!"),
            Err(e) => {
                logger::error!(error=?e, "Error while deleting Token From Tokenization Service!")
            }
        };
    };

    Ok(resp)
}

#[cfg(feature = "v1")]
pub async fn delete_network_token_from_tokenization_service(
    state: &routes::SessionState,
    network_token_requestor_reference_id: String,
    customer_id: &id_type::CustomerId,
    tokenization_service: &settings::NetworkTokenizationService,
) -> CustomResult<bool, errors::NetworkTokenizationError> {
    let payload = pm_types::DeleteCardToken {
        card_reference: network_token_requestor_reference_id,
        customer_id: customer_id.clone(),
    };

    let masked_request_body = payload
        .masked_serialize()
        .inspect_err(|e| logger::error!(error=?e, "failed to mask serialize"))
        .unwrap_or(serde_json::json!({ "error": "failed to mask serialize"}));
    logger::info!(raw_network_token_service_request=?masked_request_body);

    let response = call_network_token_service(
        state,
        tokenization_service,
        services::Method::Post,
        tokenization_service.delete_token_url.as_str(),
        Some(RequestContent::Json(Box::new(payload))),
        "delete_network_token",
    )
    .await;
    let res = response
        .change_context(errors::NetworkTokenizationError::ResponseDeserializationFailed)
        .attach_printable("Error while receiving response")
        .and_then(|inner| match inner {
            Err(err_res) => {
                let parsed_error: pm_types::NetworkTokenErrorResponse = err_res
                    .response
                    .parse_struct("Delete Network Tokenization Response")
                    .change_context(
                        errors::NetworkTokenizationError::ResponseDeserializationFailed,
                    )?;
                logger::error!(
                    error_code = %parsed_error.error_info.code,
                    developer_message = %parsed_error.error_info.developer_message,
                    "Network tokenization error: {:?}",
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

    let delete_token_response: pm_types::DeleteNetworkTokenResponse = res
        .response
        .parse_struct("Delete Network Tokenization Response")
        .change_context(errors::NetworkTokenizationError::ResponseDeserializationFailed)?;

    logger::info!("Delete Network Token Response: {:?}", delete_token_response);

    if delete_token_response.status == pm_types::DeleteNetworkTokenStatus::Success {
        Ok(true)
    } else {
        Err(errors::NetworkTokenizationError::DeleteNetworkTokenFailed)
            .attach_printable("Delete Token at Token service failed")
    }
}

#[cfg(feature = "v2")]
pub async fn delete_network_token_from_locker_and_token_service(
    _state: &routes::SessionState,
    _customer_id: &id_type::GlobalCustomerId,
    _merchant_id: &id_type::MerchantId,
    _payment_method_id: String,
    _network_token_locker_id: Option<String>,
    _network_token_requestor_reference_id: String,
) -> errors::RouterResult<DeleteCardResp> {
    todo!()
}

// ==================== ALT-ID FUNCTIONS (Guest Checkout Tokenization) ====================

/// Fetch Alt-ID and cryptogram for guest checkout transactions
#[cfg(feature = "v1")]
pub async fn fetch_altid_and_cryptogram(
    state: &routes::SessionState,
    payload_bytes: &[u8],
    order_data: ext_pm_types::AltIdOrderData,
    tokenization_service: &settings::NetworkTokenizationService,
) -> CustomResult<ext_pm_types::AltIdResponsePayload, errors::NetworkTokenizationError> {
    let enc_key = tokenization_service.public_key.peek().clone();
    let key_id = tokenization_service.key_id.clone();

    // JWE encrypt the card data
    let encrypted_card_data = encryption::encrypt_jwe(
        payload_bytes,
        enc_key,
        services::EncryptionAlgorithm::A128GCM,
        Some(key_id.as_str()),
    )
    .await
    .change_context(errors::NetworkTokenizationError::CardDataEncryptionFailed)
    .attach_printable("Failed to JWE encrypt card data for Alt-ID")?;

    // Build the Alt-ID request payload
    let payload = ext_pm_types::FetchAltIdRequest {
        card_data: Secret::new(encrypted_card_data),
        order_data,
        key_id: Some(key_id),
    };

    let masked_request_body = payload
        .masked_serialize()
        .inspect_err(|e| logger::error!(error=?e, "failed to mask serialize altid request"))
        .unwrap_or(serde_json::json!({ "error": "failed to mask serialize"}));
    logger::info!(raw_altid_service_request=?masked_request_body);

    // Call the Alt-ID API
    let response = call_network_token_service(
        state,
        tokenization_service,
        services::Method::Post,
        tokenization_service.fetch_altid_url.as_str(),
        Some(RequestContent::Json(Box::new(payload))),
        "fetch_altid",
    )
    .await;

    let res = response
        .change_context(errors::NetworkTokenizationError::ResponseDeserializationFailed)
        .attach_printable("Error while receiving Alt-ID response")
        .and_then(|inner| match inner {
            Err(err_res) => {
                let parsed_error: pm_types::NetworkTokenErrorResponse = err_res
                    .response
                    .parse_struct("Alt-ID Error Response")
                    .change_context(
                        errors::NetworkTokenizationError::ResponseDeserializationFailed,
                    )?;
                logger::error!(
                    error_code = %parsed_error.error_info.code,
                    developer_message = %parsed_error.error_info.developer_message,
                    "Alt-ID generation error: {:?}",
                    parsed_error.error_message
                );
                Err(errors::NetworkTokenizationError::FetchAltIdFailed).attach_printable(format!(
                    "Alt-ID generation API error: {:?}",
                    parsed_error.error_message
                ))
            }
            Ok(res) => Ok(res),
        })?;

    let altid_response_raw: ext_pm_types::AltIdResponse = res
        .response
        .parse_struct("Alt-ID Response")
        .change_context(errors::NetworkTokenizationError::ResponseDeserializationFailed)?;

    logger::info!("Alt-ID Response status: {:?}", altid_response_raw.status);

    let dec_key = tokenization_service.private_key.peek().clone();
    let decrypted_altid_details_json = services::decrypt_jwe(
        altid_response_raw.payload.alt_id_details.peek(),
        services::KeyIdCheck::SkipKeyIdCheck,
        dec_key,
        jwe::RSA_OAEP_256,
    )
    .await
    .change_context(errors::NetworkTokenizationError::ResponseDecryptionFailed)
    .attach_printable("Failed to decrypt altIdDetails from Alt-ID response")?;

    let alt_id_details: ext_pm_types::AltIdDetails =
        serde_json::from_str(&decrypted_altid_details_json)
            .change_context(errors::NetworkTokenizationError::ResponseDeserializationFailed)
            .attach_printable("Failed to parse decrypted altIdDetails")?;

    Ok((altid_response_raw.payload, alt_id_details).into())
}

#[cfg(feature = "v1")]
pub async fn get_altid_for_card(
    state: &routes::SessionState,
    card: &domain::Card,
    amount: common_utils::types::MinorUnit,
    currency: api_models::enums::Currency,
    auth_ref_number: Option<String>,
) -> CustomResult<
    hyperswitch_domain_models::payment_method_data::NetworkTokenData,
    errors::NetworkTokenizationError,
> {
    let card_detail: domain::CardDetail = card.into();
    match &state.conf.network_tokenization_service {
        Some(nt_service) => {
            let tokenization_service = nt_service.get_inner();

            let float_amount = amount
                .to_major_unit_as_f64(currency)
                .change_context(errors::NetworkTokenizationError::RequestEncodingFailed)
                .attach_printable("Failed to convert amount to major unit")?;

            let card_data =
                ext_pm_types::AltIdCardData::from((&card_detail, Some(card.card_cvc.clone())));

            // Double-encode card data for JWE encryption (matches expected format)
            let payload = card_data
                .encode_to_string_of_json()
                .and_then(|x| x.encode_to_string_of_json())
                .change_context(errors::NetworkTokenizationError::RequestEncodingFailed)?;
            let payload_bytes = payload.as_bytes();

            let order_data = ext_pm_types::AltIdOrderData {
                amount: float_amount,
                currency,
                auth_ref_number,
            };

            let altid_response = record_operation_time(
                async {
                    fetch_altid_and_cryptogram(
                        state,
                        payload_bytes,
                        order_data,
                        tokenization_service,
                    )
                    .await
                    .inspect_err(|e| logger::error!(error=?e, "Error while fetching Alt-ID"))
                },
                &metrics::FETCH_ALTID_TIME,
                router_env::metric_attributes!(("service", "altid")),
            )
            .await?;

            Ok(altid_response.into())
        }
        None => Err(report!(
            errors::NetworkTokenizationError::NetworkTokenizationServiceNotConfigured
        )),
    }
}

#[cfg(feature = "v1")]
pub async fn evaluate_and_fetch_altid(
    state: &routes::SessionState,
    payment_method_data: Option<&domain::PaymentMethodData>,
    currency: Option<api_models::enums::Currency>,
    connector: &api_models::enums::Connector,
    is_guest_checkout: bool,
    amount: common_utils::types::MinorUnit,
    business_profile: &domain::Profile,
) -> CustomResult<Option<domain::NetworkTokenData>, errors::ApiErrorResponse> {
    if let (Some(domain::PaymentMethodData::Card(card)), Some(currency), true) =
        (payment_method_data, currency, is_guest_checkout)
    {
        match AltIdDecision::evaluate(state, card, business_profile, *connector) {
            AltIdDecision::Proceed => get_altid_for_card(state, card, amount, currency, None)
                .await
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Failed to fetch Alt-ID for guest checkout")
                .map(Some),
            AltIdDecision::Skip => Ok(None),
            AltIdDecision::Error => Err(report!(errors::ApiErrorResponse::InternalServerError))
                .attach_printable(
                    "Network tokenization disabled but required per RBI for this transaction",
                ),
        }
    } else {
        Ok(None)
    }
}

#[cfg(feature = "v1")]
/// Called from the `NetworkTokenizationWorkflow` process tracker job.
/// Fetches the card from the locker for the given PM, generates a network token,
/// and updates the payment method record with the token details.
pub async fn generate_network_token_for_payment_method(
    state: &routes::SessionState,
    platform: &domain::Platform,
    tracking_data: &crate::types::storage::NetworkTokenizationTrackingData,
    payment_method: domain::PaymentMethod,
) -> errors::RouterResult<()> {
    use common_utils::ext_traits::AsyncExt;

    use crate::{
        core::{
            payments::tokenization::{
                save_network_token_details_in_nt_mapper, save_network_token_in_locker,
            },
            utils::create_encrypted_data,
        },
        types::domain::PaymentMethodData,
    };

    let customer_id = &tracking_data.customer_id;
    let locker_id = payment_method
        .locker_id
        .as_ref()
        .unwrap_or(&payment_method.payment_method_id);

    // Fetch the raw card from the locker
    let card_from_locker =
        payment_methods::cards::get_card_from_locker(state, customer_id, &tracking_data.merchant_id, locker_id)
            .await
            .map_err(|err| {
                logger::error!(?err, payment_method_id=%payment_method.payment_method_id, "Failed to fetch card from locker for NT generation");
                err
            })?;

    let locker_card = card_from_locker.get_card();

    let card_data = domain::Card {
        card_number: locker_card.card_number.clone(),
        card_exp_month: locker_card.card_exp_month.clone(),
        card_exp_year: locker_card.card_exp_year.clone(),
        card_cvc: Secret::new("".to_string()),
        card_issuer: None,
        card_network: locker_card
            .card_brand
            .as_deref()
            .and_then(|s| s.parse::<common_enums::CardNetwork>().ok()),
        card_type: None,
        card_issuing_country: None,
        card_issuing_country_code: None,
        bank_code: None,
        card_holder_name: tracking_data.billing_name.clone(),
        nick_name: locker_card.nick_name.clone().map(Secret::new),
        co_badged_card_data: None,
    };

    logger::info!(
        payment_method_id = %payment_method.payment_method_id,
        locker_id = %locker_id,
        card_network = ?card_data.card_network,
        locker_card_brand = ?locker_card.card_brand,
        card_exp_month = ?card_data.card_exp_month,
        card_exp_year = ?card_data.card_exp_year,
        "NT request: card fetched from locker, initiating network token generation"
    );

    let payment_method_data = PaymentMethodData::Card(card_data.clone());

    let payment_method_create_request = payment_methods::get_payment_method_create_request(
        Some(&payment_method_data),
        Some(tracking_data.payment_method),
        tracking_data.payment_method_type,
        &Some(customer_id.clone()),
        tracking_data.billing_name.clone(),
        None,
    )
    .await?;

    let (network_token_resp, _dc, network_token_requestor_ref_id) =
        Box::pin(save_network_token_in_locker(
            state,
            platform.get_provider(),
            &card_data,
            None,
            payment_method_create_request,
        ))
        .await
        .map_err(|err| {
            logger::error!(?err, "Failed to save network token in locker");
            err
        })?;

    logger::info!(
        payment_method_id = %payment_method.payment_method_id,
        network_token_resp_present = network_token_resp.is_some(),
        network_token_locker_id = ?network_token_resp.as_ref().map(|resp| &resp.payment_method_id),
        network_token_requestor_ref_id = ?network_token_requestor_ref_id,
        "NT response: received result from save_network_token_in_locker"
    );

    if let (Some(token_resp), Some(_)) =
        (network_token_resp.as_ref(), &network_token_requestor_ref_id)
    {
        let network_token_locker_id = Some(token_resp.payment_method_id.clone());

        let key_manager_state = state.into();
        let pm_network_token_data_encrypted: Option<
            common_utils::crypto::Encryptable<Secret<serde_json::Value>>,
        > = {
            let pm_token_details = token_resp.card.as_ref().map(|card| {
                domain::PaymentMethodsData::Card(domain::CardDetailsPaymentMethod::from((
                    card.clone(),
                    None,
                )))
            });

            pm_token_details
                .async_map(|pm_card| {
                    create_encrypted_data(
                        &key_manager_state,
                        platform.get_provider().get_key_store(),
                        pm_card,
                        common_utils::type_name!(diesel_models::payment_method::PaymentMethod),
                    )
                })
                .await
                .transpose()
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Unable to encrypt network token payment method data")?
        };

        let db = &*state.store;
        let compat_action = payment_methods::payment_method_modular_forward_compat_action(
            state,
            &payment_method.merchant_id,
            payment_method.customer_id.as_ref(),
        )
        .await;

        payment_methods::cards::update_payment_method_network_token_data(
            platform.get_provider().get_key_store(),
            db,
            payment_method.clone(),
            network_token_requestor_ref_id.clone(),
            network_token_locker_id,
            pm_network_token_data_encrypted,
            platform.get_provider().get_account().storage_scheme,
            platform.get_initiator(),
            compat_action,
        )
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to update PM with network token details")?;

        if let Some(nt_ref_id) = network_token_requestor_ref_id {
            save_network_token_details_in_nt_mapper(
                state,
                platform.get_provider(),
                customer_id,
                payment_method.payment_method_id.clone(),
                nt_ref_id,
            )
            .await
            .attach_printable("Failed to save network token details in callback_mapper table")?;
        }
    } else {
        logger::info!(
            payment_method_id=%payment_method.payment_method_id,
            "Network token generation returned no token response — skipping PM update"
        );
    }

    Ok(())
}
