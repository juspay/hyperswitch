use api_models::poll::PollResponse;
use common_utils::ext_traits::StringExt;
use error_stack::ResultExt;
use router_env::{instrument, tracing};

use super::errors;
use crate::{core::errors::RouterResponse, services::ApplicationResponse, types::domain, AppState};

#[instrument(skip_all)]
pub async fn retrieve_poll_status(
    state: AppState,
    req: crate::types::api::PollId,
    merchant_account: domain::MerchantAccount,
) -> RouterResponse<PollResponse> {
    let redis_conn = state
        .store
        .get_redis_conn()
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to get redis connection")?;
    let request_poll_id = req.poll_id;
    // prepend 'poll_{merchant_id}_' to restrict access to only fetching Poll IDs, as this is a freely passed string in the request
    let poll_id = super::utils::get_poll_id(merchant_account.merchant_id, request_poll_id.clone());
    let redis_value = redis_conn
        .get_key::<Option<String>>(poll_id.as_str())
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
