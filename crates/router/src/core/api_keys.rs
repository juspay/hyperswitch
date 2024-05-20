use common_utils::date_time;
#[cfg(feature = "email")]
use diesel_models::{api_keys::ApiKey, enums as storage_enums};
use error_stack::{report, ResultExt};
use masking::{PeekInterface, StrongSecret};
use router_env::{instrument, tracing};

use crate::{
    configs::settings,
    consts,
    core::errors::{self, RouterResponse, StorageErrorExt},
    routes::{metrics, AppState},
    services::ApplicationResponse,
    types::{api, storage, transformers::ForeignInto},
    utils,
};

#[cfg(feature = "email")]
const API_KEY_EXPIRY_TAG: &str = "API_KEY";
#[cfg(feature = "email")]
const API_KEY_EXPIRY_NAME: &str = "API_KEY_EXPIRY";
#[cfg(feature = "email")]
const API_KEY_EXPIRY_RUNNER: diesel_models::ProcessTrackerRunner =
    diesel_models::ProcessTrackerRunner::ApiKeyExpiryWorkflow;

static HASH_KEY: once_cell::sync::OnceCell<StrongSecret<[u8; PlaintextApiKey::HASH_KEY_LEN]>> =
    once_cell::sync::OnceCell::new();

impl settings::ApiKeys {
    pub fn get_hash_key(
        &self,
    ) -> errors::RouterResult<&'static StrongSecret<[u8; PlaintextApiKey::HASH_KEY_LEN]>> {
        HASH_KEY.get_or_try_init(|| {
            <[u8; PlaintextApiKey::HASH_KEY_LEN]>::try_from(
                hex::decode(self.hash_key.peek())
                    .change_context(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable("API key hash key has invalid hexadecimal data")?
                    .as_slice(),
            )
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("The API hashing key has incorrect length")
            .map(StrongSecret::new)
        })
    }
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
    state: AppState,
    api_key: api::CreateApiKeyRequest,
    merchant_id: String,
) -> RouterResponse<api::CreateApiKeyResponse> {
    let api_key_config = state.conf.api_keys.get_inner();
    let store = state.store.as_ref();
    // We are not fetching merchant account as the merchant key store is needed to search for a
    // merchant account.
    // Instead, we're only fetching merchant key store, as it is sufficient to identify
    // non-existence of a merchant account.
    store
        .get_merchant_key_store_by_merchant_id(
            merchant_id.as_str(),
            &store.get_master_key().to_vec().into(),
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::MerchantAccountNotFound)?;

    let hash_key = api_key_config.get_hash_key()?;
    let plaintext_api_key = PlaintextApiKey::new(consts::API_KEY_LENGTH);
    let api_key = storage::ApiKeyNew {
        key_id: PlaintextApiKey::new_key_id(),
        merchant_id: merchant_id.to_owned(),
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

    metrics::API_KEY_CREATED.add(
        &metrics::CONTEXT,
        1,
        &[metrics::request::add_attributes("merchant", merchant_id)],
    );

    // Add process to process_tracker for email reminder, only if expiry is set to future date
    // If the `api_key` is set to expire in less than 7 days, the merchant is not notified about it's expiry
    #[cfg(feature = "email")]
    {
        if api_key.expires_at.is_some() {
            let expiry_reminder_days = state.conf.api_keys.get_inner().expiry_reminder_days.clone();

            add_api_key_expiry_task(store, &api_key, expiry_reminder_days)
                .await
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Failed to insert API key expiry reminder to process tracker")?;
        }
    }

    Ok(ApplicationResponse::Json(
        (api_key, plaintext_api_key).foreign_into(),
    ))
}

// Add api_key_expiry task to the process_tracker table.
// Construct ProcessTrackerNew struct with all required fields, and schedule the first email.
// After first email has been sent, update the schedule_time based on retry_count in execute_workflow().
// A task is not scheduled if the time for the first email is in the past.
#[cfg(feature = "email")]
#[instrument(skip_all)]
pub async fn add_api_key_expiry_task(
    store: &dyn crate::db::StorageInterface,
    api_key: &ApiKey,
    expiry_reminder_days: Vec<u8>,
) -> Result<(), errors::ProcessTrackerError> {
    let current_time = date_time::now();

    let schedule_time = expiry_reminder_days
        .first()
        .and_then(|expiry_reminder_day| {
            api_key.expires_at.map(|expires_at| {
                expires_at.saturating_sub(time::Duration::days(i64::from(*expiry_reminder_day)))
            })
        })
        .ok_or(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to obtain initial process tracker schedule time")?;

    if schedule_time <= current_time {
        return Ok(());
    }

    let api_key_expiry_tracker = storage::ApiKeyExpiryTrackingData {
        key_id: api_key.key_id.clone(),
        merchant_id: api_key.merchant_id.clone(),
        api_key_name: api_key.name.clone(),
        prefix: api_key.prefix.clone(),
        // We need API key expiry too, because we need to decide on the schedule_time in
        // execute_workflow() where we won't be having access to the Api key object.
        api_key_expiry: api_key.expires_at,
        expiry_reminder_days: expiry_reminder_days.clone(),
    };

    let process_tracker_id = generate_task_id_for_api_key_expiry_workflow(api_key.key_id.as_str());
    let process_tracker_entry = storage::ProcessTrackerNew::new(
        process_tracker_id,
        API_KEY_EXPIRY_NAME,
        API_KEY_EXPIRY_RUNNER,
        [API_KEY_EXPIRY_TAG],
        api_key_expiry_tracker,
        schedule_time,
    )
    .change_context(errors::ApiErrorResponse::InternalServerError)
    .attach_printable("Failed to construct API key expiry process tracker task")?;

    store
        .insert_process(process_tracker_entry)
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable_lazy(|| {
            format!(
                "Failed while inserting API key expiry reminder to process_tracker: api_key_id: {}",
                api_key.key_id
            )
        })?;
    metrics::TASKS_ADDED_COUNT.add(
        &metrics::CONTEXT,
        1,
        &[metrics::request::add_attributes("flow", "ApiKeyExpiry")],
    );

    Ok(())
}

#[instrument(skip_all)]
pub async fn retrieve_api_key(
    state: AppState,
    merchant_id: &str,
    key_id: &str,
) -> RouterResponse<api::RetrieveApiKeyResponse> {
    let store = state.store.as_ref();
    let api_key = store
        .find_api_key_by_merchant_id_key_id_optional(merchant_id, key_id)
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError) // If retrieve failed
        .attach_printable("Failed to retrieve API key")?
        .ok_or(report!(errors::ApiErrorResponse::ApiKeyNotFound))?; // If retrieve returned `None`

    Ok(ApplicationResponse::Json(api_key.foreign_into()))
}

#[instrument(skip_all)]
pub async fn update_api_key(
    state: AppState,
    api_key: api::UpdateApiKeyRequest,
) -> RouterResponse<api::RetrieveApiKeyResponse> {
    let merchant_id = api_key.merchant_id.clone();
    let key_id = api_key.key_id.clone();
    let store = state.store.as_ref();

    let api_key = store
        .update_api_key(
            merchant_id.to_owned(),
            key_id.to_owned(),
            api_key.foreign_into(),
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::ApiKeyNotFound)?;

    #[cfg(feature = "email")]
    {
        let expiry_reminder_days = state.conf.api_keys.get_inner().expiry_reminder_days.clone();

        let task_id = generate_task_id_for_api_key_expiry_workflow(&key_id);
        // In order to determine how to update the existing process in the process_tracker table,
        // we need access to the current entry in the table.
        let existing_process_tracker_task = store
            .find_process_by_id(task_id.as_str())
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError) // If retrieve failed
            .attach_printable(
                "Failed to retrieve API key expiry reminder task from process tracker",
            )?;

        // If process exist
        if existing_process_tracker_task.is_some() {
            if api_key.expires_at.is_some() {
                // Process exist in process, update the process with new schedule_time
                update_api_key_expiry_task(store, &api_key, expiry_reminder_days)
                    .await
                    .change_context(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable(
                        "Failed to update API key expiry reminder task in process tracker",
                    )?;
            }
            // If an expiry is set to 'never'
            else {
                // Process exist in process, revoke it
                revoke_api_key_expiry_task(store, &key_id)
                    .await
                    .change_context(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable(
                        "Failed to revoke API key expiry reminder task in process tracker",
                    )?;
            }
        }
        // This case occurs if the expiry for an API key is set to 'never' during its creation. If so,
        // process in tracker was not created.
        else if api_key.expires_at.is_some() {
            // Process doesn't exist in process_tracker table, so create new entry with
            // schedule_time based on new expiry set.
            add_api_key_expiry_task(store, &api_key, expiry_reminder_days)
                .await
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable(
                    "Failed to insert API key expiry reminder task to process tracker",
                )?;
        }
    }

    Ok(ApplicationResponse::Json(api_key.foreign_into()))
}

// Update api_key_expiry task in the process_tracker table.
// Construct Update variant of ProcessTrackerUpdate with new tracking_data.
// A task is not scheduled if the time for the first email is in the past.
#[cfg(feature = "email")]
#[instrument(skip_all)]
pub async fn update_api_key_expiry_task(
    store: &dyn crate::db::StorageInterface,
    api_key: &ApiKey,
    expiry_reminder_days: Vec<u8>,
) -> Result<(), errors::ProcessTrackerError> {
    let current_time = date_time::now();

    let schedule_time = expiry_reminder_days
        .first()
        .and_then(|expiry_reminder_day| {
            api_key.expires_at.map(|expires_at| {
                expires_at.saturating_sub(time::Duration::days(i64::from(*expiry_reminder_day)))
            })
        });

    if let Some(schedule_time) = schedule_time {
        if schedule_time <= current_time {
            return Ok(());
        }
    }

    let task_id = generate_task_id_for_api_key_expiry_workflow(api_key.key_id.as_str());

    let task_ids = vec![task_id.clone()];

    let updated_tracking_data = &storage::ApiKeyExpiryTrackingData {
        key_id: api_key.key_id.clone(),
        merchant_id: api_key.merchant_id.clone(),
        api_key_name: api_key.name.clone(),
        prefix: api_key.prefix.clone(),
        api_key_expiry: api_key.expires_at,
        expiry_reminder_days,
    };

    let updated_api_key_expiry_workflow_model = serde_json::to_value(updated_tracking_data)
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable_lazy(|| {
            format!("unable to serialize API key expiry tracker: {updated_tracking_data:?}")
        })?;

    let updated_process_tracker_data = storage::ProcessTrackerUpdate::Update {
        name: None,
        retry_count: Some(0),
        schedule_time,
        tracking_data: Some(updated_api_key_expiry_workflow_model),
        business_status: Some("Pending".to_string()),
        status: Some(storage_enums::ProcessTrackerStatus::New),
        updated_at: Some(current_time),
    };
    store
        .process_tracker_update_process_status_by_ids(task_ids, updated_process_tracker_data)
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)?;

    Ok(())
}

#[instrument(skip_all)]
pub async fn revoke_api_key(
    state: AppState,
    merchant_id: &str,
    key_id: &str,
) -> RouterResponse<api::RevokeApiKeyResponse> {
    let store = state.store.as_ref();
    let revoked = store
        .revoke_api_key(merchant_id, key_id)
        .await
        .to_not_found_response(errors::ApiErrorResponse::ApiKeyNotFound)?;

    metrics::API_KEY_REVOKED.add(&metrics::CONTEXT, 1, &[]);

    #[cfg(feature = "email")]
    {
        let task_id = generate_task_id_for_api_key_expiry_workflow(key_id);
        // In order to determine how to update the existing process in the process_tracker table,
        // we need access to the current entry in the table.
        let existing_process_tracker_task = store
            .find_process_by_id(task_id.as_str())
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError) // If retrieve failed
            .attach_printable(
                "Failed to retrieve API key expiry reminder task from process tracker",
            )?;

        // If process exist, then revoke it
        if existing_process_tracker_task.is_some() {
            revoke_api_key_expiry_task(store, key_id)
                .await
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable(
                    "Failed to revoke API key expiry reminder task in process tracker",
                )?;
        }
    }

    Ok(ApplicationResponse::Json(api::RevokeApiKeyResponse {
        merchant_id: merchant_id.to_owned(),
        key_id: key_id.to_owned(),
        revoked,
    }))
}

// Function to revoke api_key_expiry task in the process_tracker table when API key is revoked.
// Construct StatusUpdate variant of ProcessTrackerUpdate by setting status to 'finish'.
#[cfg(feature = "email")]
#[instrument(skip_all)]
pub async fn revoke_api_key_expiry_task(
    store: &dyn crate::db::StorageInterface,
    key_id: &str,
) -> Result<(), errors::ProcessTrackerError> {
    let task_id = generate_task_id_for_api_key_expiry_workflow(key_id);
    let task_ids = vec![task_id];
    let updated_process_tracker_data = storage::ProcessTrackerUpdate::StatusUpdate {
        status: storage_enums::ProcessTrackerStatus::Finish,
        business_status: Some("Revoked".to_string()),
    };

    store
        .process_tracker_update_process_status_by_ids(task_ids, updated_process_tracker_data)
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)?;

    Ok(())
}

#[instrument(skip_all)]
pub async fn list_api_keys(
    state: AppState,
    merchant_id: String,
    limit: Option<i64>,
    offset: Option<i64>,
) -> RouterResponse<Vec<api::RetrieveApiKeyResponse>> {
    let store = state.store.as_ref();
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

#[cfg(feature = "email")]
fn generate_task_id_for_api_key_expiry_workflow(key_id: &str) -> String {
    format!("{API_KEY_EXPIRY_RUNNER}_{API_KEY_EXPIRY_NAME}_{key_id}")
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
        let hash_key = settings.api_keys.get_inner().get_hash_key().unwrap();
        let hashed_api_key = plaintext_api_key.keyed_hash(hash_key.peek());

        assert_ne!(
            plaintext_api_key.0.peek().as_bytes(),
            hashed_api_key.0.as_bytes()
        );

        let new_hashed_api_key = plaintext_api_key.keyed_hash(hash_key.peek());
        assert_eq!(hashed_api_key, new_hashed_api_key)
    }
}
