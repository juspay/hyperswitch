use api_models::poll::{PollResponse, PollStatus};
use common_utils::ext_traits::StringExt;
use error_stack::ResultExt;
use router_env::{instrument, tracing};

use super::errors;
use crate::{
    core::errors::RouterResponse,
    services::{logger, ApplicationResponse},
    AppState,
};

#[instrument(skip_all)]
pub async fn retrieve_poll_status(
    state: AppState,
    req: crate::types::api::PollId,
) -> RouterResponse<PollResponse> {
    let redis_conn = state
        .store
        .get_redis_conn()
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to get redis connection")?;
    let poll_id = req.poll_id;
    let redis_value = redis_conn
        .get_key::<Option<String>>(poll_id.as_str())
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable_lazy(|| {
            format!(
                "Error while fetching the value for {} from redis",
                poll_id.clone()
            )
        })?;
    let status: PollStatus = redis_value
        .map(|value| {
            value
                .parse_enum("PollStatus")
                .map_err(|err| logger::warn!("error while parsing PollStatus: {err}"))
                .ok()
                .unwrap_or(PollStatus::NotFound)
        })
        .unwrap_or(PollStatus::NotFound);
    let poll_response = PollResponse { poll_id, status };
    Ok(ApplicationResponse::Json(poll_response))
}
