use crate::{
    controller::{self, DeleteCardResp},
    core::errors,
    headers, metrics, services, state, types as pm_types,
};
use common_utils::{ext_traits::BytesExt, id_type, metrics::utils::record_operation_time};
use error_stack::ResultExt;
use hyperswitch_domain_models::merchant_context;
use hyperswitch_interfaces::configs as settings;
use api_models::user::TokenResponse;
use masking::{maskable::Mask, PeekInterface, Secret};
use router_env::logger;
use common_utils::ext_traits::Encode;

#[cfg(feature = "v1")]
pub async fn delete_network_token_from_locker_and_token_service(
    state: &state::PaymentMethodsState,
    customer_id: &id_type::CustomerId,
    merchant_id: &id_type::MerchantId,
    payment_method_id: String,
    network_token_locker_id: Option<String>,
    network_token_requestor_reference_id: String,
    merchant_context: &merchant_context::MerchantContext,
) -> errors::PmResult<DeleteCardResp> {
    //deleting network token from locker
    let resp = controller::PmCards {
        state,
        merchant_context,
    }
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
    state: &state::PaymentMethodsState,
    network_token_requestor_reference_id: String,
    customer_id: &id_type::CustomerId,
    tokenization_service: &settings::NetworkTokenizationService,
) -> errors::NetworkTokenizationResult<bool> {
    let mut request = services::Request::new(
        services::Method::Post,
        tokenization_service.delete_token_url.as_str(),
    );
    let payload = pm_types::DeleteCardToken {
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
    request.set_body(services::RequestContent::Json(Box::new(payload)));

    logger::info!("Request to delete network token: {:?}", request);

    // Send the request using `call_connector_api`
    let response = services::call_connector_api(state, request, "delete network token")
        .await
        .change_context(errors::NetworkTokenizationError::ApiError);
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

#[cfg(feature = "v1")]
pub async fn make_card_network_tokenization_request(
    state: &state::PaymentMethodsState,
    card: &domain::CardDetail,
    optional_cvc: Option<Secret<String>>,
    customer_id: &id_type::CustomerId,
) -> errors::NetworkTokenizationResult<(pm_types::CardNetworkTokenResponsePayload, Option<String>)>
{
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
) -> errors::NetworkTokenizationResult<(NetworkTokenDetails, String)> {
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
    };
    Ok((network_token_details, network_token_req_ref_id))
}
#[cfg(feature = "v1")]
pub async fn get_network_token(
    state: &state::PaymentMethodsState,
    customer_id: id_type::CustomerId,
    network_token_requestor_ref_id: String,
    tokenization_service: &settings::NetworkTokenizationService,
) -> errors::NetworkTokenizationResult<TokenResponse> {
    let mut request = services::Request::new(
        services::Method::Post,
        tokenization_service.fetch_token_url.as_str(),
    );
    let payload = pm_types::GetCardToken {
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
    request.set_body(services::RequestContent::Json(Box::new(payload)));

    logger::info!("Request to fetch network token: {:?}", request);

    // Send the request using `call_connector_api`
    let response = services::call_connector_api(state, request, "get network token")
        .await
        .change_context(errors::NetworkTokenizationError::ApiError);

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
                    "Network tokenization error: {}",
                    parsed_error.error_message
                );
                Err(errors::NetworkTokenizationError::ResponseDeserializationFailed)
                    .attach_printable(format!("Response Deserialization Failed: {err_res:?}"))
            }
            Ok(res) => Ok(res),
        })?;

    let token_response: TokenResponse = res
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
) -> errors::NetworkTokenizationResult<TokenResponse> {
    let mut request = services::Request::new(
        services::Method::Post,
        tokenization_service.fetch_token_url.as_str(),
    );
    let payload = pm_types::GetCardToken {
        card_reference: network_token_requestor_ref_id,
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

    logger::info!("Request to fetch network token: {:?}", request);

    // Send the request using `call_connector_api`
    let response = services::call_connector_api(state, request, "get network token")
        .await
        .change_context(errors::NetworkTokenizationError::ApiError);

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
                    "Network tokenization error: {}",
                    parsed_error.error_message
                );
                Err(errors::NetworkTokenizationError::ResponseDeserializationFailed)
                    .attach_printable(format!("Response Deserialization Failed: {err_res:?}"))
            }
            Ok(res) => Ok(res),
        })?;

    let token_response: TokenResponse = res
        .response
        .parse_struct("Get Network Token Response")
        .change_context(errors::NetworkTokenizationError::ResponseDeserializationFailed)?;
    logger::info!("Fetch Network Token Response: {:?}", token_response);

    Ok(token_response)
}
