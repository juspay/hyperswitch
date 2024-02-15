use api_models::{blocklist, enums as api_enums};
use common_utils::{ext_traits::StringExt, request::RequestContent};
use error_stack::ResultExt;
use josekit::jwe;
#[cfg(feature = "aws_kms")]
use masking::PeekInterface;
use router_env::{instrument, tracing};

use crate::{
    configs::settings,
    core::{
        errors::{self, CustomResult},
        payment_methods::transformers as payment_methods,
    },
    headers, routes,
    services::{api as services, encryption},
    types::{storage, transformers::ForeignFrom},
    utils::{self, ConnectorResponseExt},
};

const LOCKER_API_URL: &str = "/cards/fingerprint";

impl ForeignFrom<storage::Blocklist> for blocklist::AddToBlocklistResponse {
    fn foreign_from(from: storage::Blocklist) -> Self {
        Self {
            fingerprint_id: from.fingerprint_id,
            data_kind: from.data_kind,
            created_at: from.created_at,
        }
    }
}

async fn generate_fingerprint_request<'a>(
    #[cfg(not(feature = "aws_kms"))] jwekey: &settings::Jwekey,
    #[cfg(feature = "aws_kms")] jwekey: &settings::ActiveKmsSecrets,
    locker: &settings::Locker,
    payload: &blocklist::GenerateFingerprintRequest,
    locker_choice: api_enums::LockerChoice,
) -> CustomResult<services::Request, errors::VaultError> {
    let payload = utils::Encode::<blocklist::GenerateFingerprintRequest>::encode_to_vec(&payload)
        .change_context(errors::VaultError::RequestEncodingFailed)?;

    #[cfg(feature = "aws_kms")]
    let private_key = jwekey.jwekey.peek().vault_private_key.as_bytes();

    #[cfg(not(feature = "aws_kms"))]
    let private_key = jwekey.vault_private_key.as_bytes();

    let jws = encryption::jws_sign_payload(&payload, &locker.locker_signing_key_id, private_key)
        .await
        .change_context(errors::VaultError::RequestEncodingFailed)?;

    let jwe_payload = generate_jwe_payload_for_request(jwekey, &jws, locker_choice).await?;
    let mut url = match locker_choice {
        api_enums::LockerChoice::Basilisk => locker.host.to_owned(),
        api_enums::LockerChoice::Tartarus => locker.host_rs.to_owned(),
    };
    url.push_str(LOCKER_API_URL);
    let mut request = services::Request::new(services::Method::Post, &url);
    request.add_header(headers::CONTENT_TYPE, "application/json".into());
    request.set_body(RequestContent::Json(Box::new(jwe_payload)));
    Ok(request)
}

async fn generate_jwe_payload_for_request(
    #[cfg(feature = "aws_kms")] jwekey: &settings::ActiveKmsSecrets,
    #[cfg(not(feature = "aws_kms"))] jwekey: &settings::Jwekey,
    jws: &str,
    locker_choice: api_enums::LockerChoice,
) -> CustomResult<encryption::JweBody, errors::VaultError> {
    let jws_payload: Vec<&str> = jws.split('.').collect();

    let generate_jws_body = |payload: Vec<&str>| -> Option<encryption::JwsBody> {
        Some(encryption::JwsBody {
            header: payload.first()?.to_string(),
            payload: payload.get(1)?.to_string(),
            signature: payload.get(2)?.to_string(),
        })
    };

    let jws_body =
        generate_jws_body(jws_payload).ok_or(errors::VaultError::GenerateFingerprintFailed)?;

    let payload = utils::Encode::<encryption::JwsBody>::encode_to_vec(&jws_body)
        .change_context(errors::VaultError::GenerateFingerprintFailed)?;

    #[cfg(feature = "aws_kms")]
    let public_key = match locker_choice {
        api_enums::LockerChoice::Basilisk => jwekey.jwekey.peek().vault_encryption_key.as_bytes(),
        api_enums::LockerChoice::Tartarus => {
            jwekey.jwekey.peek().rust_locker_encryption_key.as_bytes()
        }
    };

    #[cfg(not(feature = "aws_kms"))]
    let public_key = match locker_choice {
        api_enums::LockerChoice::Basilisk => jwekey.vault_encryption_key.as_bytes(),
        api_enums::LockerChoice::Tartarus => jwekey.rust_locker_encryption_key.as_bytes(),
    };

    let jwe_encrypted = encryption::encrypt_jwe(&payload, public_key)
        .await
        .change_context(errors::VaultError::SaveCardFailed)
        .attach_printable("Error on jwe encrypt")?;
    let jwe_payload: Vec<&str> = jwe_encrypted.split('.').collect();

    let generate_jwe_body = |payload: Vec<&str>| -> Option<encryption::JweBody> {
        Some(encryption::JweBody {
            header: payload.first()?.to_string(),
            iv: payload.get(2)?.to_string(),
            encrypted_payload: payload.get(3)?.to_string(),
            tag: payload.get(4)?.to_string(),
            encrypted_key: payload.get(1)?.to_string(),
        })
    };

    let jwe_body =
        generate_jwe_body(jwe_payload).ok_or(errors::VaultError::GenerateFingerprintFailed)?;

    Ok(jwe_body)
}

#[instrument(skip_all)]
pub async fn generate_fingerprint(
    state: &routes::AppState,
    card_number: String,
    hash_key: String,
    locker_choice: api_enums::LockerChoice,
) -> CustomResult<blocklist::GenerateFingerprintResponsePayload, errors::VaultError> {
    let payload = blocklist::GenerateFingerprintRequest {
        card: blocklist::Card { card_number },
        hash_key,
    };

    let generate_fingerprint_resp =
        call_to_locker_for_fingerprint(state, &payload, locker_choice).await?;

    Ok(generate_fingerprint_resp)
}

#[instrument(skip_all)]
async fn call_to_locker_for_fingerprint(
    state: &routes::AppState,
    payload: &blocklist::GenerateFingerprintRequest,
    locker_choice: api_enums::LockerChoice,
) -> CustomResult<blocklist::GenerateFingerprintResponsePayload, errors::VaultError> {
    let locker = &state.conf.locker;
    #[cfg(not(feature = "aws_kms"))]
    let jwekey = &state.conf.jwekey;
    #[cfg(feature = "aws_kms")]
    let jwekey = &state.kms_secrets;

    let request = generate_fingerprint_request(jwekey, locker, payload, locker_choice).await?;
    let response = services::call_connector_api(state, request)
        .await
        .change_context(errors::VaultError::GenerateFingerprintFailed);
    let jwe_body: encryption::JweBody = response
        .get_response_inner("JweBody")
        .change_context(errors::VaultError::GenerateFingerprintFailed)?;

    let decrypted_payload =
        decrypt_generate_fingerprint_response_payload(jwekey, jwe_body, Some(locker_choice))
            .await
            .change_context(errors::VaultError::GenerateFingerprintFailed)
            .attach_printable("Error getting decrypted fingerprint response payload")?;
    let generate_fingerprint_response: blocklist::GenerateFingerprintResponsePayload = decrypted_payload
        .parse_struct("GenerateFingerprintResponse")
        .change_context(errors::VaultError::ResponseDeserializationFailed)?;

    Ok(generate_fingerprint_response)
}

async fn decrypt_generate_fingerprint_response_payload(
    #[cfg(not(feature = "aws_kms"))] jwekey: &settings::Jwekey,
    #[cfg(feature = "aws_kms")] jwekey: &settings::ActiveKmsSecrets,
    jwe_body: encryption::JweBody,
    locker_choice: Option<api_enums::LockerChoice>,
) -> CustomResult<String, errors::VaultError> {
    let target_locker = locker_choice.unwrap_or(api_enums::LockerChoice::Tartarus);

    #[cfg(feature = "aws_kms")]
    let public_key = match target_locker {
        api_enums::LockerChoice::Basilisk => jwekey.jwekey.peek().vault_encryption_key.as_bytes(),
        api_enums::LockerChoice::Tartarus => {
            jwekey.jwekey.peek().rust_locker_encryption_key.as_bytes()
        }
    };

    #[cfg(feature = "aws_kms")]
    let private_key = jwekey.jwekey.peek().vault_private_key.as_bytes();

    #[cfg(not(feature = "aws_kms"))]
    let public_key = match target_locker {
        api_enums::LockerChoice::Basilisk => jwekey.vault_encryption_key.as_bytes(),
        api_enums::LockerChoice::Tartarus => jwekey.rust_locker_encryption_key.as_bytes(),
    };

    #[cfg(not(feature = "aws_kms"))]
    let private_key = jwekey.vault_private_key.as_bytes();

    let jwt = payment_methods::get_dotted_jwe(jwe_body);
    let alg = jwe::RSA_OAEP;

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
