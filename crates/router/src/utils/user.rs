#[cfg(feature = "v1")]
use api_models::admin as admin_api;
use api_models::user as user_api;
#[cfg(feature = "v1")]
use common_enums::connector_enums;
use common_enums::UserAuthType;
use common_utils::{
    encryption::Encryption,
    errors::CustomResult,
    id_type, type_name,
    types::{
        keymanager::{Identifier, KeyManagerState},
        user::LineageContext,
    },
};
use diesel_models::organization::{self, OrganizationBridge};
use error_stack::ResultExt;
#[cfg(feature = "v1")]
use hyperswitch_domain_models::merchant_connector_account::MerchantConnectorAccount as DomainMerchantConnectorAccount;
use hyperswitch_masking::{ExposeInterface, Secret};
use redis_interface::RedisConnectionWithContext;
use router_env::{env, instrument, logger, tracing, tracing::Instrument};

use crate::{
    consts::user::{REDIS_SSO_PREFIX, REDIS_SSO_TTL},
    core::errors::{StorageError, UserErrors, UserResult},
    routes::SessionState,
    services::{
        authentication::{AuthToken, UserFromToken},
        authorization::roles::RoleInfo,
    },
    types::{
        domain::{self, MerchantAccount, UserFromStorage},
        transformers::{ForeignFrom, ForeignTryFrom},
    },
};

pub mod dashboard_metadata;
pub mod password;
#[cfg(feature = "dummy_connector")]
pub mod sample_data;
pub mod theme;
pub mod two_factor_auth;

impl UserFromToken {
    pub async fn get_merchant_account_from_db(
        &self,
        state: SessionState,
    ) -> UserResult<MerchantAccount> {
        let key_store = state
            .store
            .get_merchant_key_store_by_merchant_id(
                &self.merchant_id,
                &state.store.get_master_key().to_vec().into(),
            )
            .await
            .map_err(|e| {
                if e.current_context().is_db_not_found() {
                    e.change_context(UserErrors::MerchantIdNotFound)
                } else {
                    e.change_context(UserErrors::InternalServerError)
                }
            })?;
        let merchant_account = state
            .store
            .find_merchant_account_by_merchant_id(&self.merchant_id, &key_store)
            .await
            .map_err(|e| {
                if e.current_context().is_db_not_found() {
                    e.change_context(UserErrors::MerchantIdNotFound)
                } else {
                    e.change_context(UserErrors::InternalServerError)
                }
            })?;
        Ok(merchant_account)
    }

    pub async fn get_active_user_from_db(
        &self,
        state: &SessionState,
    ) -> UserResult<UserFromStorage> {
        let user = state
            .global_store
            .find_active_user_by_user_id(&self.user_id)
            .await
            .change_context(UserErrors::InternalServerError)?;
        Ok(user.into())
    }

    pub async fn get_role_info_from_db(&self, state: &SessionState) -> UserResult<RoleInfo> {
        RoleInfo::from_role_id_org_id_tenant_id(
            state,
            &self.role_id,
            &self.org_id,
            self.tenant_id.as_ref().unwrap_or(&state.tenant.tenant_id),
        )
        .await
        .change_context(UserErrors::InternalServerError)
    }
}

pub async fn generate_jwt_auth_token_with_attributes(
    state: &SessionState,
    user_id: String,
    merchant_id: id_type::MerchantId,
    org_id: id_type::OrganizationId,
    role_id: String,
    profile_id: id_type::ProfileId,
    tenant_id: Option<id_type::TenantId>,
) -> UserResult<Secret<String>> {
    let token = AuthToken::new_token(
        user_id,
        merchant_id,
        role_id,
        &state.conf,
        org_id,
        profile_id,
        tenant_id,
    )
    .await?;
    Ok(Secret::new(token))
}

#[allow(unused_variables)]
pub fn get_verification_days_left(
    state: &SessionState,
    user: &UserFromStorage,
) -> UserResult<Option<i64>> {
    #[cfg(feature = "email")]
    return user.get_verification_days_left(state);
    #[cfg(not(feature = "email"))]
    return Ok(None);
}

pub async fn get_active_user_from_db_by_email(
    state: &SessionState,
    email: domain::UserEmail,
) -> CustomResult<UserFromStorage, StorageError> {
    state
        .global_store
        .find_active_user_by_user_email(&email)
        .await
        .map(UserFromStorage::from)
}

pub fn get_redis_connection_for_global_tenant(
    state: &SessionState,
) -> UserResult<RedisConnectionWithContext> {
    state
        .global_store
        .get_redis_conn()
        .change_context(UserErrors::InternalServerError)
        .attach_printable("Failed to get redis connection")
}

impl ForeignFrom<&user_api::AuthConfig> for UserAuthType {
    fn foreign_from(from: &user_api::AuthConfig) -> Self {
        match *from {
            user_api::AuthConfig::OpenIdConnect { .. } => Self::OpenIdConnect,
            user_api::AuthConfig::Password => Self::Password,
            user_api::AuthConfig::MagicLink => Self::MagicLink,
        }
    }
}

pub async fn construct_public_and_private_db_configs(
    state: &SessionState,
    auth_config: &user_api::AuthConfig,
    encryption_key: &[u8],
    id: String,
) -> UserResult<(Option<Encryption>, Option<serde_json::Value>)> {
    encrypt_auth_config(&state.into(), auth_config, encryption_key, id).await
}

/// The body of [`construct_public_and_private_db_configs`], taking the key
/// manager state rather than a `SessionState` so the encryption path can be
/// exercised without standing up a session (see the `tests` module below).
async fn encrypt_auth_config(
    key_manager_state: &KeyManagerState,
    auth_config: &user_api::AuthConfig,
    encryption_key: &[u8],
    id: String,
) -> UserResult<(Option<Encryption>, Option<serde_json::Value>)> {
    match auth_config {
        user_api::AuthConfig::OpenIdConnect {
            private_config,
            public_config,
        } => {
            let private_config_value = serde_json::to_value(private_config.clone())
                .change_context(UserErrors::InternalServerError)
                .attach_printable("Failed to convert auth config to json")?;

            // Encrypt locally: no key is ever registered with the key manager for a `UserAuth`
            // identifier, so the key manager call would fail and fall back to exactly this,
            // while incrementing ENCRYPTION_API_FAILURES on the way. The identifier is still
            // required by `crypto_operation`, which ignores it for the local variants.
            let encrypted_config = domain::types::crypto_operation::<
                serde_json::Value,
                hyperswitch_masking::WithType,
            >(
                key_manager_state,
                type_name!(diesel_models::user::User),
                domain::types::CryptoOperation::EncryptLocally(private_config_value.into()),
                Identifier::UserAuth(id),
                encryption_key,
            )
            .await
            .and_then(|val| val.try_into_operation())
            .change_context(UserErrors::InternalServerError)
            .attach_printable("Failed to encrypt auth config")?;

            Ok((
                Some(encrypted_config.into()),
                Some(
                    serde_json::to_value(public_config.clone())
                        .change_context(UserErrors::InternalServerError)
                        .attach_printable("Failed to convert auth config to json")?,
                ),
            ))
        }
        user_api::AuthConfig::Password | user_api::AuthConfig::MagicLink => Ok((None, None)),
    }
}

pub fn parse_value<T>(value: serde_json::Value, type_name: &str) -> UserResult<T>
where
    T: serde::de::DeserializeOwned,
{
    serde_json::from_value::<T>(value)
        .change_context(UserErrors::InternalServerError)
        .attach_printable(format!("Unable to parse {type_name}"))
}

pub async fn decrypt_oidc_private_config(
    state: &SessionState,
    encrypted_config: Option<Encryption>,
    id: String,
) -> UserResult<user_api::OpenIdConnectPrivateConfig> {
    let user_auth_key = hex::decode(
        state
            .conf
            .user_auth_methods
            .get_inner()
            .encryption_key
            .clone()
            .expose(),
    )
    .change_context(UserErrors::InternalServerError)
    .attach_printable("Failed to decode DEK")?;

    decrypt_auth_config(&state.into(), encrypted_config, id, &user_auth_key).await
}

/// The body of [`decrypt_oidc_private_config`], taking the key manager state and
/// the already-decoded user-auth key so the decryption path can be exercised
/// without standing up a session (see the `tests` module below).
async fn decrypt_auth_config(
    key_manager_state: &KeyManagerState,
    encrypted_config: Option<Encryption>,
    id: String,
    user_auth_key: &[u8],
) -> UserResult<user_api::OpenIdConnectPrivateConfig> {
    // See `construct_public_and_private_db_configs` for why this decrypts locally.
    let encrypted_config = encrypted_config
        .ok_or(UserErrors::InternalServerError)
        .attach_printable("Private config not found")?;

    let private_config =
        domain::types::crypto_operation::<serde_json::Value, hyperswitch_masking::WithType>(
            key_manager_state,
            type_name!(diesel_models::user::User),
            domain::types::CryptoOperation::DecryptLocally(encrypted_config),
            Identifier::UserAuth(id),
            user_auth_key,
        )
        .await
        .and_then(|val| val.try_into_operation())
        .change_context(UserErrors::InternalServerError)
        .attach_printable("Failed to decrypt private config")?
        .into_inner()
        .expose();

    serde_json::from_value::<user_api::OpenIdConnectPrivateConfig>(private_config)
        .change_context(UserErrors::InternalServerError)
        .attach_printable("unable to parse OpenIdConnectPrivateConfig")
}

pub async fn set_sso_id_in_redis(
    state: &SessionState,
    oidc_state: Secret<String>,
    sso_id: String,
) -> UserResult<()> {
    let connection = get_redis_connection_for_global_tenant(state)?;
    let key = get_oidc_key(&oidc_state.expose());
    connection
        .set_key_with_expiry(&key.into(), sso_id, REDIS_SSO_TTL)
        .await
        .change_context(UserErrors::InternalServerError)
        .attach_printable("Failed to set sso id in redis")
}

pub async fn get_sso_id_from_redis(
    state: &SessionState,
    oidc_state: Secret<String>,
) -> UserResult<String> {
    let connection = get_redis_connection_for_global_tenant(state)?;
    let key = get_oidc_key(&oidc_state.expose());
    connection
        .get_key::<Option<String>>(&key.into())
        .await
        .change_context(UserErrors::InternalServerError)
        .attach_printable("Failed to get sso id from redis")?
        .ok_or(UserErrors::SSOFailed)
        .attach_printable("Cannot find oidc state in redis. Oidc state invalid or expired")
}

fn get_oidc_key(oidc_state: &str) -> String {
    format!("{REDIS_SSO_PREFIX}{oidc_state}")
}

pub fn get_oidc_sso_redirect_url(state: &SessionState, provider: &str) -> String {
    format!("{}/redirect/oidc/{}", state.conf.user.base_url, provider)
}

pub fn is_sso_auth_type(auth_type: UserAuthType) -> bool {
    match auth_type {
        UserAuthType::OpenIdConnect => true,
        UserAuthType::Password | UserAuthType::MagicLink => false,
    }
}

#[cfg(feature = "v1")]
pub fn create_merchant_account_request_for_org(
    req: user_api::UserOrgMerchantCreateRequest,
    org: organization::Organization,
    product_type: common_enums::MerchantProductType,
) -> UserResult<api_models::admin::MerchantAccountCreate> {
    let merchant_id = generate_env_specific_merchant_id(req.merchant_name.clone().expose())?;

    let company_name = domain::UserCompanyName::new(req.merchant_name.expose())?;
    Ok(api_models::admin::MerchantAccountCreate {
        merchant_id,
        metadata: None,
        locker_id: None,
        return_url: None,
        merchant_name: Some(Secret::new(company_name.get_secret())),
        webhook_details: None,
        publishable_key: None,
        organization_id: Some(org.get_organization_id()),
        merchant_details: None,
        routing_algorithm: None,
        parent_merchant_id: None,
        sub_merchants_enabled: None,
        frm_routing_algorithm: None,
        #[cfg(feature = "payouts")]
        payout_routing_algorithm: None,
        primary_business_details: None,
        payment_response_hash_key: None,
        enable_payment_response_hash: None,
        redirect_to_merchant_with_http_post: None,
        pm_collect_link_config: None,
        product_type: Some(product_type),
        merchant_account_type: None,
        network_tokenization_credentials: None,
    })
}

pub async fn validate_email_domain_auth_type_using_db(
    state: &SessionState,
    email: &domain::UserEmail,
    required_auth_type: UserAuthType,
) -> UserResult<()> {
    let domain = email.extract_domain()?;
    let user_auth_methods = state
        .store
        .list_user_authentication_methods_for_email_domain(domain)
        .await
        .change_context(UserErrors::InternalServerError)?;

    (user_auth_methods.is_empty()
        || user_auth_methods
            .iter()
            .any(|auth_method| auth_method.auth_type == required_auth_type))
    .then_some(())
    .ok_or(UserErrors::InvalidUserAuthMethodOperation.into())
}

pub fn spawn_async_lineage_context_update_to_db(
    state: &SessionState,
    user_id: &str,
    lineage_context: LineageContext,
) {
    let state = state.clone();
    let lineage_context = lineage_context.clone();
    let user_id = user_id.to_owned();
    let lineage_update = async move {
        match state
            .global_store
            .update_active_user_by_user_id(
                &user_id,
                diesel_models::user::UserUpdate::LineageContextUpdate { lineage_context },
            )
            .await
        {
            Ok(_) => {
                logger::debug!("Successfully updated lineage context for user {}", user_id);
            }
            Err(e) => {
                logger::error!(
                    "Failed to update lineage context for user {}: {:?}",
                    user_id,
                    e
                );
            }
        }
    };
    tokio::spawn(lineage_update.in_current_span());
}

pub fn generate_env_specific_merchant_id(value: String) -> UserResult<id_type::MerchantId> {
    if matches!(env::which(), env::Env::Production) {
        let raw_id = domain::MerchantId::new(value)?;
        Ok(id_type::MerchantId::try_from(raw_id)?)
    } else {
        Ok(id_type::MerchantId::new_from_unix_timestamp())
    }
}

pub fn get_base_url(state: &SessionState) -> &str {
    if !state.conf.multitenancy.enabled {
        &state.conf.user.base_url
    } else {
        &state.tenant.user.control_center_url
    }
}

#[cfg(feature = "v1")]
#[instrument(skip_all)]
pub async fn build_cloned_connector_create_request(
    source_mca: DomainMerchantConnectorAccount,
    destination_profile_id: id_type::ProfileId,
    destination_connector_label: Option<String>,
    payment_method_types: &std::collections::HashMap<
        common_enums::PaymentMethod,
        std::collections::HashSet<common_enums::PaymentMethodType>,
    >,
) -> UserResult<admin_api::MerchantConnectorCreate> {
    let source_mca_name = source_mca
        .connector_name
        .parse::<connector_enums::Connector>()
        .change_context(UserErrors::InternalServerError)
        .attach_printable("Invalid connector name received")?;

    let connector_account_details = source_mca.connector_account_details.clone().into_inner();

    let source_mca = admin_api::MerchantConnectorResponse::foreign_try_from(source_mca)
        .change_context(UserErrors::InternalServerError)
        .attach_printable("Unable to convert merchant connector account to response")?;

    let payment_methods_enabled = source_mca.payment_methods_enabled.map(|payment_methods| {
        payment_methods
            .into_iter()
            .filter_map(|mut payment_method| {
                let allowed_subtypes = payment_method_types.get(&payment_method.payment_method)?;
                if let Some(subtypes) = payment_method.payment_method_types.as_mut() {
                    subtypes
                        .retain(|subtype| allowed_subtypes.contains(&subtype.payment_method_type));
                    if subtypes.is_empty() {
                        return None;
                    }
                }
                Some(payment_method)
            })
            .collect::<Vec<_>>()
    });

    Ok(admin_api::MerchantConnectorCreate {
        connector_type: source_mca.connector_type,
        connector_name: source_mca_name,
        connector_label: destination_connector_label.or(source_mca.connector_label),
        merchant_connector_id: None,
        connector_account_details: Some(connector_account_details),
        test_mode: source_mca.test_mode,
        disabled: source_mca.disabled,
        payment_methods_enabled,
        metadata: source_mca.metadata,
        business_country: source_mca.business_country,
        business_label: source_mca.business_label,
        business_sub_label: source_mca.business_sub_label,
        frm_configs: source_mca.frm_configs,
        connector_webhook_details: source_mca.connector_webhook_details,
        profile_id: Some(destination_profile_id),
        pm_auth_config: None,
        connector_wallets_details: source_mca.connector_wallets_details,
        status: Some(source_mca.status),
        additional_merchant_data: source_mca.additional_merchant_data,
    })
}

#[cfg(test)]
// Test setup and assertions panic on purpose; these lints are denied by the
// clippy profiles this crate is built with.
#[allow(
    clippy::expect_used,
    clippy::panic,
    clippy::unwrap_used,
    reason = "test setup and assertions panic on purpose"
)]
/// Covers the local-only OIDC private-config path, collectively:
///
/// * it round-trips, and the key manager is never contacted,
/// * rows written before the switch to `EncryptLocally`/`DecryptLocally` still
///   decrypt, so no re-encryption is needed for existing data,
/// * a row with no private config errors rather than yielding a usable config,
/// * `Password` / `MagicLink` store no config at all.
///
/// Hermetic by construction: nothing here builds a `SessionState`, reads config,
/// or opens anything but its own loopback listener.
mod tests {
    use std::{
        io::{Read, Write},
        net::TcpListener,
        sync::{
            atomic::{AtomicUsize, Ordering},
            Arc,
        },
    };

    use common_utils::types::keymanager::KeyManagerState;
    use hyperswitch_masking::{PeekInterface, WithType};
    use serde_json::json;

    use super::*;

    /// 32 bytes of AES-256 key, hex encoded: `decrypt_oidc_private_config`
    /// hex-decodes `user_auth_methods.encryption_key`, and `GcmAes256` rejects
    /// any other length (`UnboundKey::new(&aead::AES_256_GCM, ..)` in
    /// `EncodeMessage::encode_message`) as an `Err`, not a panic.
    ///
    /// The test owns this value rather than reading it out of
    /// `config/development.toml`, so an unrelated edit to that file cannot
    /// change what this test verifies.
    const TEST_DEK_HEX: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

    const AUTH_METHOD_ID: &str = "auth_method_id_under_test";

    /// An address nothing listens on, for the cases that must not reach a key
    /// manager and so have no use for the spy.
    const CLOSED_KEY_MANAGER_URL: &str = "http://127.0.0.1:1";

    fn test_dek() -> Vec<u8> {
        hex::decode(TEST_DEK_HEX).expect("test DEK is valid hex")
    }

    fn test_private_config() -> user_api::OpenIdConnectPrivateConfig {
        user_api::OpenIdConnectPrivateConfig {
            base_url: "https://idp.example.com".to_string(),
            client_id: Secret::new("test-client-id".to_string()),
            client_secret: Secret::new("test-client-secret".to_string()),
            private_key: Some(Secret::new("test-private-key".to_string())),
        }
    }

    fn test_auth_config() -> user_api::AuthConfig {
        user_api::AuthConfig::OpenIdConnect {
            private_config: test_private_config(),
            public_config: user_api::OpenIdConnectPublicConfig {
                name: user_api::OpenIdProvider::Okta,
            },
        }
    }

    /// A key manager state that is *enabled* and pointed at `key_manager_url`,
    /// so that a call to the key manager would be observable.
    ///
    /// Built by hand rather than from `Settings`/`SessionState` on purpose:
    /// `AppState::with_storage` dials Redis, PostgreSQL and Superposition, which
    /// would make this a test that only passes on a machine with the full stack
    /// up -- and CI runs no test job, so it would never be exercised at all.
    /// `KeyManagerState::mock` is `enabled: false`, which is the one setting
    /// that would make the "no key-manager call" assertions vacuous.
    fn key_manager_state_pointing_at(key_manager_url: &str) -> KeyManagerState {
        KeyManagerState {
            enabled: true,
            url: key_manager_url.to_string(),
            ..KeyManagerState::mock()
        }
    }

    /// A throwaway loopback listener that counts connections and answers every
    /// one with a 500.
    ///
    /// This is the only way the "no key-manager call" assertion is observable: a
    /// key-manager call fails and `crypto_operation` falls back to the very same
    /// application encryption, so the round trip succeeds either way and its
    /// result says nothing about whether a call was attempted.
    fn key_manager_spy() -> (String, Arc<AtomicUsize>) {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind loopback listener");
        let url = format!(
            "http://{}",
            listener.local_addr().expect("listener local address")
        );
        let connections = Arc::new(AtomicUsize::new(0));
        let counter = Arc::clone(&connections);

        std::thread::spawn(move || {
            for stream in listener.incoming() {
                let mut stream = match stream {
                    Ok(stream) => stream,
                    Err(_) => continue,
                };
                counter.fetch_add(1, Ordering::SeqCst);
                // Drain the request head so the client's write can complete, then
                // fail the call.
                let mut head = [0_u8; 1024];
                let _ = stream.read(&mut head);
                let _ = stream
                    .write_all(b"HTTP/1.1 500 Internal Server Error\r\nContent-Length: 0\r\n\r\n");
                let _ = stream.flush();
            }
        });

        (url, connections)
    }

    /// Encrypts an OIDC private config through `CryptoOperation::Encrypt`, the
    /// operation this fix replaced, so a test can compare it against the current
    /// local path.
    async fn encrypt_via_key_manager_operation(
        key_manager_state: &KeyManagerState,
        private_config: &user_api::OpenIdConnectPrivateConfig,
    ) -> Encryption {
        let private_config_value =
            serde_json::to_value(private_config).expect("private config to json");
        let dek = test_dek();

        domain::types::crypto_operation::<serde_json::Value, WithType>(
            key_manager_state,
            type_name!(diesel_models::user::User),
            domain::types::CryptoOperation::Encrypt(private_config_value.into()),
            Identifier::UserAuth(AUTH_METHOD_ID.to_string()),
            &dek,
        )
        .await
        .and_then(|output| output.try_into_operation())
        .expect("encrypting via the key manager operation")
        .into()
    }

    fn assert_private_config_eq(
        decrypted: &user_api::OpenIdConnectPrivateConfig,
        expected: &user_api::OpenIdConnectPrivateConfig,
    ) {
        assert_eq!(decrypted.base_url, expected.base_url, "base_url");
        assert_eq!(
            decrypted.client_id.peek(),
            expected.client_id.peek(),
            "client_id"
        );
        assert_eq!(
            decrypted.client_secret.peek(),
            expected.client_secret.peek(),
            "client_secret"
        );
        assert_eq!(
            decrypted.private_key.as_ref().map(|key| key.peek()),
            expected.private_key.as_ref().map(|key| key.peek()),
            "private_key"
        );
    }

    #[tokio::test]
    async fn oidc_private_config_round_trips_without_calling_the_key_manager() {
        let (key_manager_url, key_manager_connections) = key_manager_spy();
        let key_manager_state = key_manager_state_pointing_at(&key_manager_url);

        let (private_config, public_config) = encrypt_auth_config(
            &key_manager_state,
            &test_auth_config(),
            &test_dek(),
            AUTH_METHOD_ID.to_string(),
        )
        .await
        .expect("encrypting the OIDC private config");

        assert_eq!(
            public_config,
            Some(json!({ "name": "okta" })),
            "the public config is stored as-is"
        );

        let private_config = private_config.expect("the private config is encrypted");
        assert!(
            !String::from_utf8_lossy(private_config.get_inner().peek())
                .contains("test-client-secret"),
            "the stored private config must not hold the client secret in the clear"
        );

        let decrypted = decrypt_auth_config(
            &key_manager_state,
            Some(private_config),
            AUTH_METHOD_ID.to_string(),
            &test_dek(),
        )
        .await
        .expect("decrypting the OIDC private config");

        assert_private_config_eq(&decrypted, &test_private_config());
        assert_eq!(
            key_manager_connections.load(Ordering::SeqCst),
            0,
            "OIDC private-config encryption and decryption must not call the key \
             manager: no key is ever registered for a `UserAuth` identifier, so \
             every call failed and fell back to exactly this local encryption. \
             See `the_spy_observes_the_key_manager_operation_the_fix_replaced`, \
             which validates that this assertion is not vacuous — and to what \
             degree, since the spy cannot observe a call under `keymanager_mtls`."
        );
    }

    /// Drives the key-manager path this fix replaced, to show the round-trip
    /// test's zero-connection assertion is not vacuous.
    ///
    /// Runs in every build rather than being compiled out under
    /// `keymanager_mtls`, because that is the build `make test` uses and a
    /// silently-absent control is worse than a partial one. What the spy can
    /// prove differs by build, so both halves are asserted here:
    ///
    /// * Always: with the key manager *enabled*, the replaced operation still
    ///   returns correct ciphertext via the application-encryption fallback,
    ///   rather than failing.
    /// * Where a call is observable: the spy sees the outbound request, which is
    ///   what makes the round-trip test's zero count meaningful.
    ///
    /// A call is only observable between two configurations. Without
    /// `encryption_service` the key manager is never consulted at all; under
    /// `keymanager_mtls` it is built with `https_only(true)` from the configured
    /// certificate, and the test config has none, so the call fails before a
    /// socket is opened. In those builds the count is asserted to stay at zero,
    /// which pins the documented limitation rather than letting it drift. No
    /// in-process check can do better: observing it under `keymanager_mtls`
    /// would need the key-manager HTTP client to be injectable.
    #[tokio::test]
    async fn the_spy_observes_the_key_manager_operation_the_fix_replaced() {
        let (key_manager_url, key_manager_connections) = key_manager_spy();
        let key_manager_state = key_manager_state_pointing_at(&key_manager_url);

        // Succeeds via the fallback, returning the right ciphertext either way.
        let encrypted =
            encrypt_via_key_manager_operation(&key_manager_state, &test_private_config()).await;

        assert!(
            !encrypted.get_inner().peek().is_empty(),
            "with the key manager enabled, the replaced operation must still return \
             ciphertext through the application-encryption fallback"
        );

        let spy_can_observe_a_call =
            cfg!(feature = "encryption_service") && !cfg!(feature = "keymanager_mtls");

        assert_eq!(
            key_manager_connections.load(Ordering::SeqCst) > 0,
            spy_can_observe_a_call,
            "the spy observes a key-manager call wherever one can be observed, so \
             the round-trip test's zero count is meaningful; where none can be \
             observed (no `encryption_service`, or `keymanager_mtls` building an \
             unusable client) the count must stay at zero"
        );
    }

    /// Rows written before the switch went through `CryptoOperation::Encrypt`
    /// and were encrypted by the fallback, which is the same application
    /// encryption `EncryptLocally` performs. Existing rows must keep decrypting.
    #[tokio::test]
    async fn decrypts_a_private_config_written_before_the_local_encryption_switch() {
        let key_manager_state = key_manager_state_pointing_at(CLOSED_KEY_MANAGER_URL);

        // A disabled key manager is what production reached after every failed
        // call: the fallback, and no request.
        let written_before_the_switch =
            encrypt_via_key_manager_operation(&KeyManagerState::mock(), &test_private_config())
                .await;

        let decrypted = decrypt_auth_config(
            &key_manager_state,
            Some(written_before_the_switch),
            AUTH_METHOD_ID.to_string(),
            &test_dek(),
        )
        .await
        .expect("a private config encrypted before the switch must still decrypt");

        assert_private_config_eq(&decrypted, &test_private_config());
    }

    #[tokio::test]
    async fn decrypting_a_missing_private_config_is_an_error() {
        let key_manager_state = key_manager_state_pointing_at(CLOSED_KEY_MANAGER_URL);

        let result = decrypt_auth_config(
            &key_manager_state,
            None,
            AUTH_METHOD_ID.to_string(),
            &test_dek(),
        )
        .await;

        assert!(
            result.is_err(),
            "a row with no private config must not decrypt to a usable config"
        );
    }

    #[tokio::test]
    async fn non_oidc_auth_configs_store_no_config() {
        let key_manager_state = key_manager_state_pointing_at(CLOSED_KEY_MANAGER_URL);

        for auth_config in [
            user_api::AuthConfig::Password,
            user_api::AuthConfig::MagicLink,
        ] {
            assert_eq!(
                encrypt_auth_config(
                    &key_manager_state,
                    &auth_config,
                    &test_dek(),
                    AUTH_METHOD_ID.to_string(),
                )
                .await
                .expect("a non-OIDC auth config needs no encryption"),
                (None, None),
            );
        }
    }
}
