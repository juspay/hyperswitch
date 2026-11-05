use api_models::poll::PollResponse;
use common_utils::ext_traits::StringExt;
use error_stack::ResultExt;
use hyperswitch_masking::PeekInterface;
use router_env::{instrument, tracing};

use super::errors;
use crate::{
    core::errors::{RouterResponse, StorageErrorExt},
    services::ApplicationResponse,
    types::domain,
    SessionState,
};

#[cfg(feature = "v1")]
#[instrument(skip_all)]
pub async fn retrieve_poll_status(
    state: SessionState,
    req: crate::types::api::PollId,
    processor: domain::Processor,
) -> RouterResponse<PollResponse> {
    let client_secret =
        req.client_secret
            .as_ref()
            .ok_or(errors::ApiErrorResponse::MissingRequiredField {
                field_name: "client_secret".into(),
            })?;
    let resource_id = req
        .poll_id
        .strip_prefix("external_authentication_")
        .ok_or_else(|| errors::ApiErrorResponse::PollNotFound {
            id: req.poll_id.clone(),
        })?;
    // Bind authorization to the resource named by the poll, not to an arbitrary
    // payment from the same merchant whose client secret the caller knows.
    if resource_id.starts_with("authn_") {
        let authentication_id = common_utils::id_type::AuthenticationId::try_from(
            std::borrow::Cow::Owned(resource_id.to_owned()),
        )
        .change_context(errors::ApiErrorResponse::ClientSecretInvalid)?;
        let authentication = state
            .store
            .find_authentication_by_processor_merchant_id_authentication_id(
                processor.get_account().get_id(),
                &authentication_id,
                processor.get_key_store(),
                &(&state).into(),
                *processor.get_storage_scheme(),
            )
            .await
            .to_not_found_response(errors::ApiErrorResponse::AuthenticationNotFound {
                id: authentication_id.get_string_repr().to_owned(),
            })?;
        super::unified_authentication_service::utils::authenticate_authentication_client_secret_and_check_expiry(
            client_secret.peek(),
            &authentication,
        )?;
    } else {
        let payment_id = common_utils::id_type::PaymentId::wrap(resource_id.to_owned())
            .change_context(errors::ApiErrorResponse::ClientSecretInvalid)?;
        let payment_intent = state
            .store
            .find_payment_intent_by_payment_id_processor_merchant_id(
                &payment_id,
                &processor.get_processor_merchant_id(),
                processor.get_key_store(),
                processor.get_storage_scheme(),
            )
            .await
            .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)?;
        super::payments::helpers::authenticate_client_secret(
            Some(client_secret.peek()),
            &payment_intent,
        )?;
    }
    let redis_conn = state
        .store
        .get_redis_conn()
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to get redis connection")?;
    let request_poll_id = req.poll_id;
    // prepend 'poll_{processor_merchant_id}_' to restrict access to only fetching Poll IDs, as this is a freely passed string in the request
    let poll_id =
        super::utils::get_poll_id(processor.get_account().get_id(), request_poll_id.clone());
    let redis_value = redis_conn
        .get_key::<Option<String>>(&poll_id.as_str().into())
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable_lazy(|| {
            format!(
                "Error while fetching the value for {} from redis",
                poll_id.clone()
            )
        })?
        .ok_or(errors::ApiErrorResponse::PollNotFound {
            id: request_poll_id.clone(),
        })?;
    let status = redis_value
        .parse_enum("PollStatus")
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Error while parsing PollStatus")?;
    let poll_response = PollResponse {
        poll_id: request_poll_id,
        status,
    };
    Ok(ApplicationResponse::Json(poll_response))
}
