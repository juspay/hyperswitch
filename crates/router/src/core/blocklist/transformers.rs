use api_models::{blocklist, enums as api_enums};
use common_utils::ext_traits::StringExt;
use error_stack::ResultExt;
use josekit::jwe;
use masking::{PeekInterface, StrongSecret};
use router_env::{instrument, tracing};

use crate::{
    configs::settings,
    core::{
        errors::{self, CustomResult},
        payment_methods::transformers as payment_methods,
    },
    routes,
    services::{api as services, encryption},
    types::{storage, transformers::ForeignFrom},
    utils::ConnectorResponseExt,
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
    locker_choice: api_enums::LockerChoice,
) -> CustomResult<blocklist::GenerateFingerprintResponsePayload, errors::VaultError> {
    let payload = blocklist::GenerateFingerprintRequest {
        data: card_number,
        key: hash_key,
    };

    let generate_fingerprint_resp =
        call_to_locker_for_fingerprint(state, &payload, locker_choice).await?;

    Ok(generate_fingerprint_resp)
}

#[instrument(skip_all)]
async fn call_to_locker_for_fingerprint(
    state: &routes::SessionState,
    payload: &blocklist::GenerateFingerprintRequest,
    locker_choice: api_enums::LockerChoice,
) -> CustomResult<blocklist::GenerateFingerprintResponsePayload, errors::VaultError> {
    let locker = &state.conf.locker;
    let jwekey = state.conf.jwekey.get_inner();

    let request = payment_methods::mk_generic_locker_request(
        jwekey,
        locker,
        payload,
        LOCKER_FINGERPRINT_PATH,
        Some(locker_choice),
        state.tenant.tenant_id.clone(),
        state.request_id.clone(),
    )
    .await?;
    let response = services::call_connector_api(state, request, "call_locker_to_get_fingerprint")
        .await
        .change_context(errors::VaultError::GenerateFingerprintFailed);
    let jwe_body: encryption::JweBody = response
        .get_response_inner("JweBody")
        .change_context(errors::VaultError::GenerateFingerprintFailed)?;

    let decrypted_payload = decrypt_generate_fingerprint_response_payload(
        jwekey,
        jwe_body,
        Some(locker_choice),
        locker.decryption_scheme.clone(),
    )
    .await
    .change_context(errors::VaultError::GenerateFingerprintFailed)
    .attach_printable("Error getting decrypted fingerprint response payload")?;
    let generate_fingerprint_response: blocklist::GenerateFingerprintResponsePayload =
        decrypted_payload
            .parse_struct("GenerateFingerprintResponse")
            .change_context(errors::VaultError::ResponseDeserializationFailed)?;

    Ok(generate_fingerprint_response)
}

async fn decrypt_generate_fingerprint_response_payload(
    jwekey: &settings::Jwekey,
    jwe_body: encryption::JweBody,
    locker_choice: Option<api_enums::LockerChoice>,
    decryption_scheme: settings::DecryptionScheme,
) -> CustomResult<String, errors::VaultError> {
    let target_locker = locker_choice.unwrap_or(api_enums::LockerChoice::HyperswitchCardVault);

    let public_key = match target_locker {
        api_enums::LockerChoice::HyperswitchCardVault => {
            jwekey.vault_encryption_key.peek().as_bytes()
        }
    };

    let private_key = jwekey.vault_private_key.peek().as_bytes();

    let jwt = payment_methods::get_dotted_jwe(jwe_body);
    let alg = match decryption_scheme {
        settings::DecryptionScheme::RsaOaep => jwe::RSA_OAEP,
        settings::DecryptionScheme::RsaOaep256 => jwe::RSA_OAEP_256,
    };

    let jwe_decrypted = encryption::decrypt_jwe(
        &jwt,
        encryption::KeyIdCheck::SkipKeyIdCheck,
        private_key,
        alg,
    )
    .await
    .change_context(errors::VaultError::SaveCardFailed)
    .attach_printable("Jwe Decryption failed for JweBody for vault")?;

    let jws = jwe_decrypted
        .parse_struct("JwsBody")
        .change_context(errors::VaultError::ResponseDeserializationFailed)?;
    let jws_body = payment_methods::get_dotted_jws(jws);

    encryption::verify_sign(jws_body, public_key)
        .change_context(errors::VaultError::SaveCardFailed)
        .attach_printable("Jws Decryption failed for JwsBody for vault")
}
