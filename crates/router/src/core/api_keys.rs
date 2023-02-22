use common_utils::{date_time, errors::CustomResult, fp_utils};
use error_stack::{report, IntoReport, ResultExt};
use masking::{PeekInterface, Secret};
use router_env::{instrument, tracing};

use crate::{
    consts,
    core::errors::{self, RouterResponse, StorageErrorExt},
    db::StorageInterface,
    services::ApplicationResponse,
    types::{api, storage, transformers::ForeignInto},
    utils,
};

// Defining new types `PlaintextApiKey` and `HashedApiKey` in the hopes of reducing the possibility
// of plaintext API key being stored in the data store.
pub struct PlaintextApiKey(Secret<String>);
pub struct HashedApiKey(String);

impl PlaintextApiKey {
    const HASH_KEY_LEN: usize = 32;

    const PREFIX_LEN: usize = 8;

    pub fn new(length: usize) -> Self {
        let key = common_utils::crypto::generate_cryptographically_secure_random_string(length);
        Self(key.into())
    }

    pub fn new_hash_key() -> [u8; Self::HASH_KEY_LEN] {
        common_utils::crypto::generate_cryptographically_secure_random_bytes()
    }

    pub fn new_key_id() -> String {
        let env = router_env::env::prefix_for_env();
        utils::generate_id(consts::ID_LENGTH, env)
    }

    pub fn prefix(&self) -> String {
        self.0.peek().chars().take(Self::PREFIX_LEN).collect()
    }

    pub fn peek(&self) -> &str {
        self.0.peek()
    }

    pub fn keyed_hash(&self, key: &[u8; Self::HASH_KEY_LEN]) -> HashedApiKey {
        /*
        Decisions regarding API key hashing algorithm chosen:

        - Since API key hash verification would be done for each request, there is a requirement
          for the hashing to be quick.
        - Password hashing algorithms would not be suitable for this purpose as they're designed to
          prevent brute force attacks, considering that the same password could be shared  across
          multiple sites by the user.
        - Moreover, password hash verification happens once per user session, so the delay involved
          is negligible, considering the security benefits it provides.
          While with API keys (assuming uniqueness of keys across the application), the delay
          involved in hashing (with the use of a password hashing algorithm) becomes significant,
          considering that it must be done per request.
        - Since we are the only ones generating API keys and are able to guarantee their uniqueness,
          a simple hash algorithm is sufficient for this purpose.

        Hash algorithms considered:
        - Password hashing algorithms: Argon2id and PBKDF2
        - Simple hashing algorithms: HMAC-SHA256, HMAC-SHA512, BLAKE3

        After benchmarking the simple hashing algorithms, we decided to go with the BLAKE3 keyed
        hashing algorithm, with a randomly generated key for the hash key.
        */

        HashedApiKey(
            blake3::keyed_hash(key, self.0.peek().as_bytes())
                .to_hex()
                .to_string(),
        )
    }

    pub fn verify_hash(
        &self,
        key: &[u8; Self::HASH_KEY_LEN],
        stored_api_key: &HashedApiKey,
    ) -> CustomResult<(), errors::ApiKeyError> {
        // Converting both hashes to `blake3::Hash` since it provides constant-time equality checks
        let provided_api_key_hash = blake3::keyed_hash(key, self.0.peek().as_bytes());
        let stored_api_key_hash = blake3::Hash::from_hex(&stored_api_key.0)
            .into_report()
            .change_context(errors::ApiKeyError::FailedToReadHashFromHex)?;

        fp_utils::when(provided_api_key_hash != stored_api_key_hash, || {
            Err(errors::ApiKeyError::HashVerificationFailed).into_report()
        })
    }
}

#[instrument(skip_all)]
pub async fn create_api_key(
    store: &dyn StorageInterface,
    api_key: api::CreateApiKeyRequest,
    merchant_id: String,
) -> RouterResponse<api::CreateApiKeyResponse> {
    let hash_key = PlaintextApiKey::new_hash_key();
    let plaintext_api_key = PlaintextApiKey::new(consts::API_KEY_LENGTH);
    let api_key = storage::ApiKeyNew {
        key_id: PlaintextApiKey::new_key_id(),
        merchant_id,
        name: api_key.name,
        description: api_key.description,
        hash_key: Secret::from(hex::encode(hash_key)),
        hashed_api_key: plaintext_api_key.keyed_hash(&hash_key).into(),
        prefix: plaintext_api_key.prefix(),
        created_at: date_time::now(),
        expires_at: api_key.expiration.into(),
        last_used: None,
    };

    let api_key = store
        .insert_api_key(api_key)
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to insert new API key")?;

    Ok(ApplicationResponse::Json(
        (api_key, plaintext_api_key).foreign_into(),
    ))
}

#[instrument(skip_all)]
pub async fn retrieve_api_key(
    store: &dyn StorageInterface,
    key_id: &str,
) -> RouterResponse<api::RetrieveApiKeyResponse> {
    let api_key = store
        .find_api_key_optional(key_id)
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError) // If retrieve failed
        .attach_printable("Failed to retrieve new API key")?
        .ok_or(report!(errors::ApiErrorResponse::ApiKeyNotFound))?; // If retrieve returned `None`

    Ok(ApplicationResponse::Json(api_key.foreign_into()))
}

#[instrument(skip_all)]
pub async fn update_api_key(
    store: &dyn StorageInterface,
    key_id: &str,
    api_key: api::UpdateApiKeyRequest,
) -> RouterResponse<api::RetrieveApiKeyResponse> {
    let api_key = store
        .update_api_key(key_id.to_owned(), api_key.foreign_into())
        .await
        .map_err(|err| err.to_not_found_response(errors::ApiErrorResponse::ApiKeyNotFound))?;

    Ok(ApplicationResponse::Json(api_key.foreign_into()))
}

#[instrument(skip_all)]
pub async fn revoke_api_key(
    store: &dyn StorageInterface,
    key_id: &str,
) -> RouterResponse<api::RevokeApiKeyResponse> {
    let revoked = store
        .revoke_api_key(key_id)
        .await
        .map_err(|err| err.to_not_found_response(errors::ApiErrorResponse::ApiKeyNotFound))?;

    Ok(ApplicationResponse::Json(api::RevokeApiKeyResponse {
        key_id: key_id.to_owned(),
        revoked,
    }))
}

#[instrument(skip_all)]
pub async fn list_api_keys(
    store: &dyn StorageInterface,
    merchant_id: String,
    limit: Option<i64>,
    offset: Option<i64>,
) -> RouterResponse<Vec<api::RetrieveApiKeyResponse>> {
    let api_keys = store
        .list_api_keys_by_merchant_id(&merchant_id, limit, offset)
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to list merchant API keys")?;
    let api_keys = api_keys
        .into_iter()
        .map(ForeignInto::foreign_into)
        .collect();

    Ok(ApplicationResponse::Json(api_keys))
}

impl From<HashedApiKey> for storage::HashedApiKey {
    fn from(hashed_api_key: HashedApiKey) -> Self {
        hashed_api_key.0.into()
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;

    #[test]
    fn test_hashing_and_verification() {
        let plaintext_api_key = PlaintextApiKey::new(consts::API_KEY_LENGTH);
        let hash_key = PlaintextApiKey::new_hash_key();
        let hashed_api_key = plaintext_api_key.keyed_hash(&hash_key);

        assert_ne!(
            plaintext_api_key.0.peek().as_bytes(),
            hashed_api_key.0.as_bytes()
        );

        plaintext_api_key
            .verify_hash(&hash_key, &hashed_api_key)
            .unwrap();
    }
}
