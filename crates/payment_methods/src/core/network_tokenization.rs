use crate::{core::errors, types as pm_types, services, state, headers, metrics, controller::{self, DeleteCardResp}};
use hyperswitch_domain_models::{merchant_context};
use hyperswitch_interfaces::configs as settings;
use common_utils::id_type;
use error_stack::ResultExt;
use router_env::logger;
use masking::PeekInterface;
use masking::maskable::Mask;
use common_utils::ext_traits::BytesExt;
use common_utils::metrics::utils::record_operation_time;

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
) -> errors::CustomResult<bool, errors::NetworkTokenizationError> {
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
