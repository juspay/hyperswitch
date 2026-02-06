use common_utils::ext_traits::{Encode, StringExt};
use error_stack::ResultExt;
use josekit::jwe;
use masking::PeekInterface;

use crate::{
    configs::settings,
    core::errors::{self, CustomResult},
    services::{encryption, EncryptionAlgorithm},
};

/// Consolidated JWE body creation for vault operations (used by both V1 and V2)
/// This function creates a JWE (JSON Web Encryption) body from a JWS (JSON Web Signature)
/// for secure communication with the vault service
pub(super) async fn create_jwe_body(
    jwekey: &settings::Jwekey,
    jws: &str,
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
        generate_jws_body(jws_payload).ok_or(errors::VaultError::RequestEncryptionFailed)?;

    let payload = jws_body
        .encode_to_vec()
        .change_context(errors::VaultError::RequestEncodingFailed)?;

    let public_key = jwekey.vault_encryption_key.peek().as_bytes();

    let jwe_encrypted =
        encryption::encrypt_jwe(&payload, public_key, EncryptionAlgorithm::A256GCM, None)
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
        generate_jwe_body(jwe_payload).ok_or(errors::VaultError::RequestEncodingFailed)?;

    Ok(jwe_body)
}

#[cfg(feature = "v2")]
pub(super) async fn create_jwe_body_for_vault(
    jwekey: &settings::Jwekey,
    jws: &str,
) -> CustomResult<encryption::JweBody, errors::VaultError> {
    create_jwe_body(jwekey, jws).await
}

pub(super) async fn mk_vault_req(
    jwekey: &settings::Jwekey,
    jws: &str,
) -> CustomResult<encryption::JweBody, errors::VaultError> {
    create_jwe_body(jwekey, jws).await
}

pub(super) async fn get_decrypted_response_payload(
    jwekey: &settings::Jwekey,
    jwe_body: encryption::JweBody,
    decryption_scheme: settings::DecryptionScheme,
) -> CustomResult<String, errors::VaultError> {
    let public_key = jwekey.vault_encryption_key.peek().as_bytes();

    let private_key = jwekey.vault_private_key.peek().as_bytes();

    let jwt = get_dotted_jwe(jwe_body);
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
    let jws_body = get_dotted_jws(jws);

    encryption::verify_sign(jws_body, public_key)
        .change_context(errors::VaultError::SaveCardFailed)
        .attach_printable("Jws Decryption failed for JwsBody for vault")
}

pub(super) fn get_dotted_jws(jws: encryption::JwsBody) -> String {
    let header = jws.header;
    let payload = jws.payload;
    let signature = jws.signature;
    format!("{header}.{payload}.{signature}")
}

pub(super) fn get_dotted_jwe(jwe: encryption::JweBody) -> String {
    let header = jwe.header;
    let encryption_key = jwe.encrypted_key;
    let iv = jwe.iv;
    let encryption_payload = jwe.encrypted_payload;
    let tag = jwe.tag;
    format!("{header}.{encryption_key}.{iv}.{encryption_payload}.{tag}")
}

pub(super) async fn get_decrypted_vault_response_payload(
    jwekey: &settings::Jwekey,
    jwe_body: encryption::JweBody,
    decryption_scheme: settings::DecryptionScheme,
) -> CustomResult<String, errors::VaultError> {
    let public_key = jwekey.vault_encryption_key.peek().as_bytes();

    let private_key = jwekey.vault_private_key.peek().as_bytes();

    let jwt = get_dotted_jwe(jwe_body);
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
    let jws_body = get_dotted_jws(jws);

    encryption::verify_sign(jws_body, public_key)
        .change_context(errors::VaultError::SaveCardFailed)
        .attach_printable("Jws Decryption failed for JwsBody for vault")
}
