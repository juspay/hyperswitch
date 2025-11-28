use api_models::blocklist;
use error_stack::ResultExt;
use masking::StrongSecret;
use router_env::{instrument, tracing};

use crate::{
    core::{
        errors::{self, CustomResult},
        payment_methods::transformers as payment_methods,
    },
    routes,
    types::{storage, transformers::ForeignFrom},
};

const LOCKER_FINGERPRINT_PATH: &str = "/cards/fingerprint";

impl ForeignFrom<storage::Blocklist> for blocklist::AddToBlocklistResponse {
    fn foreign_from(from: storage::Blocklist) -> Self {
        Self {
            fingerprint_id: from.fingerprint_id,
            data_kind: from.data_kind,
            created_at: from.created_at,
        }
    }
}

#[instrument(skip_all)]
pub async fn generate_fingerprint(
    state: &routes::SessionState,
    card_number: StrongSecret<String>,
    hash_key: StrongSecret<String>,
) -> CustomResult<blocklist::GenerateFingerprintResponsePayload, errors::VaultError> {
    let payload = blocklist::GenerateFingerprintRequest {
        data: card_number,
        key: hash_key,
    };

    let generate_fingerprint_resp = call_to_locker_for_fingerprint(state, &payload).await?;

    Ok(generate_fingerprint_resp)
}

#[instrument(skip_all)]
async fn call_to_locker_for_fingerprint(
    state: &routes::SessionState,
    payload: &blocklist::GenerateFingerprintRequest,
) -> CustomResult<blocklist::GenerateFingerprintResponsePayload, errors::VaultError> {
    let locker = &state.conf.locker;
    let jwekey = state.conf.jwekey.get_inner();
    let generate_fingerprint_response: blocklist::GenerateFingerprintResponsePayload =
        payment_methods::mk_locker_api_request_and_call(
            state,
            jwekey,
            locker,
            payload,
            LOCKER_FINGERPRINT_PATH,
            state.tenant.tenant_id.clone(),
            state.request_id.clone(),
        )
        .await
        .change_context(errors::VaultError::GenerateFingerprintFailed)?;

    Ok(generate_fingerprint_response)
}
