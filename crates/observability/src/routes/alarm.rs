//! Handler for the alarm evaluation route. The route tree that mounts it is in
//! [`crate::routes::app`].

use actix_web::{web, HttpRequest, HttpResponse};
use error_stack::report;

use crate::{
    auth, core,
    core::alarm::EvaluationOptions,
    errors::ObservabilityError,
    logger, services,
    state::AppState,
    types::{EvaluateAlarmsRequest, EvaluateAlarmsResponse},
};

/// `POST /alerts/cloudwatch/evaluate`.
///
/// The body is optional — a trigger that posts nothing gets a normal evaluation — but a body that
/// is *present and malformed* is rejected rather than defaulted. The difference matters because of
/// exactly one field: silently defaulting a body that said `{"dry_run": true}` but misspelled it
/// would send real announcements to a real channel in answer to a request that asked for none.
pub async fn evaluate(
    state: web::Data<AppState>,
    request: HttpRequest,
    body: web::Bytes,
) -> HttpResponse {
    services::server_wrap(
        state.get_ref().clone(),
        &request,
        body,
        |state, body| async move {
            let options = parse(&body)?;

            core::alarm::evaluate(state, options)
                .await
                .map(EvaluateAlarmsResponse::from)
        },
        &auth::InternalApiKeyAuth,
    )
    .await
}

/// Read the options out of a body that may not be there.
fn parse(body: &[u8]) -> error_stack::Result<EvaluationOptions, ObservabilityError> {
    let request = if body.iter().all(u8::is_ascii_whitespace) {
        EvaluateAlarmsRequest::default()
    } else {
        serde_json::from_slice::<EvaluateAlarmsRequest>(body).map_err(|error| {
            // The parse failure goes to the log rather than to the caller, matching
            // `routes::app::json_config`: serde quotes the offending part of the body.
            logger::warn!(error = %error, "Evaluation request rejected: the body could not be parsed");
            report!(ObservabilityError::InvalidRequest)
        })?
    };

    Ok(EvaluationOptions {
        dry_run: request.dry_run,
    })
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]
mod tests {
    use super::*;

    #[test]
    fn an_absent_body_is_a_normal_evaluation() {
        assert!(!parse(b"").unwrap().dry_run);
        assert!(!parse(b"  \n").unwrap().dry_run);
        assert!(!parse(b"{}").unwrap().dry_run);
    }

    #[test]
    fn a_dry_run_is_read_from_the_body() {
        assert!(parse(br#"{"dry_run": true}"#).unwrap().dry_run);
    }

    /// The failure this refuses to have: a misspelled `dry_run` that defaults to `false` would
    /// answer "please send nothing" by sending everything.
    #[test]
    fn a_body_that_does_not_parse_is_refused_rather_than_defaulted() {
        assert!(parse(br#"{"dryrun": true}"#).is_err());
        assert!(parse(b"not json").is_err());
    }
}
