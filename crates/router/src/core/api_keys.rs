use common_utils::date_time;
use error_stack::{report, IntoReport, ResultExt};
#[cfg(feature = "kms")]
use external_services::kms;
use masking::{PeekInterface, StrongSecret};
use router_env::{instrument, tracing};

use crate::{
    configs::settings,
    consts,
    core::errors::{self, RouterResponse, StorageErrorExt},
    db::StorageInterface,
    routes::metrics,
    services::ApplicationResponse,
    types::{api, storage, transformers::ForeignInto},
    utils,
};

static HASH_KEY: tokio::sync::OnceCell<StrongSecret<[u8; PlaintextApiKey::HASH_KEY_LEN]>> =
    tokio::sync::OnceCell::const_new();

pub async fn get_hash_key(
    api_key_config: &settings::ApiKeys,
    #[cfg(feature = "kms")] kms_config: &kms::KmsConfig,
) -> errors::RouterResult<&'static StrongSecret<[u8; PlaintextApiKey::HASH_KEY_LEN]>> {
    HASH_KEY
        .get_or_try_init(|| async {
            #[cfg(feature = "kms")]
            let hash_key = kms::get_kms_client(kms_config)
                .await
                .decrypt(&api_key_config.kms_encrypted_hash_key)
                .await
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Failed to KMS decrypt API key hashing key")?;

            #[cfg(not(feature = "kms"))]
            let hash_key = &api_key_config.hash_key;

            <[u8; PlaintextApiKey::HASH_KEY_LEN]>::try_from(
                hex::decode(hash_key)
                    .into_report()
                    .change_context(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable("API key hash key has invalid hexadecimal data")?
                    .as_slice(),
            )
            .into_report()
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("The API hashing key has incorrect length")
            .map(StrongSecret::new)
        })
        .await
}

// Defining new types `PlaintextApiKey` and `HashedApiKey` in the hopes of reducing the possibility
// of plaintext API key being stored in the data store.
pub struct PlaintextApiKey(StrongSecret<String>);

#[derive(Debug, PartialEq, Eq)]
pub struct HashedApiKey(String);

impl PlaintextApiKey {
    const HASH_KEY_LEN: usize = 32;

    const PREFIX_LEN: usize = 12;

    pub fn new(length: usize) -> Self {
        let env = router_env::env::prefix_for_env();
        let key = common_utils::crypto::generate_cryptographically_secure_random_string(length);
        Self(format!("{env}_{key}").into())
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
}

#[instrument(skip_all)]
pub async fn create_api_key(
    store: &dyn StorageInterface,
    api_key_config: &settings::ApiKeys,
    #[cfg(feature = "kms")] kms_config: &kms::KmsConfig,
    api_key: api::CreateApiKeyRequest,
    merchant_id: String,
) -> RouterResponse<api::CreateApiKeyResponse> {
    let hash_key = get_hash_key(
        api_key_config,
        #[cfg(feature = "kms")]
        kms_config,
    )
    .await?;
    let plaintext_api_key = PlaintextApiKey::new(consts::API_KEY_LENGTH);
    let api_key = storage::ApiKeyNew {
        key_id: PlaintextApiKey::new_key_id(),
        merchant_id,
        name: api_key.name,
        description: api_key.description,
        hashed_api_key: plaintext_api_key.keyed_hash(hash_key.peek()).into(),
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

    metrics::API_KEY_CREATED.add(&metrics::CONTEXT, 1, &[]);

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
        .find_api_key_by_key_id_optional(key_id)
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
        .to_not_found_response(errors::ApiErrorResponse::ApiKeyNotFound)?;

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
        .to_not_found_response(errors::ApiErrorResponse::ApiKeyNotFound)?;

    metrics::API_KEY_REVOKED.add(&metrics::CONTEXT, 1, &[]);

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

impl From<&str> for PlaintextApiKey {
    fn from(s: &str) -> Self {
        Self(s.to_owned().into())
    }
}

impl From<String> for PlaintextApiKey {
    fn from(s: String) -> Self {
        Self(s.into())
    }
}

impl From<HashedApiKey> for storage::HashedApiKey {
    fn from(hashed_api_key: HashedApiKey) -> Self {
        hashed_api_key.0.into()
    }
}

impl From<storage::HashedApiKey> for HashedApiKey {
    fn from(hashed_api_key: storage::HashedApiKey) -> Self {
        Self(hashed_api_key.into_inner())
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used)]
    use super::*;

    #[tokio::test]
    async fn test_hashing_and_verification() {
        let settings = settings::Settings::new().expect("invalid settings");

        let plaintext_api_key = PlaintextApiKey::new(consts::API_KEY_LENGTH);
        let hash_key = get_hash_key(
            &settings.api_keys,
            #[cfg(feature = "kms")]
            &settings.kms,
        )
        .await
        .unwrap();
        let hashed_api_key = plaintext_api_key.keyed_hash(hash_key.peek());

        assert_ne!(
            plaintext_api_key.0.peek().as_bytes(),
            hashed_api_key.0.as_bytes()
        );

        let new_hashed_api_key = plaintext_api_key.keyed_hash(hash_key.peek());
        assert_eq!(hashed_api_key, new_hashed_api_key)
    }
}
