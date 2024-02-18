use common_utils::errors::CustomResult;
use hyperswitch_interfaces::secrets_interface::{
    secret_handler::SecretsHandler,
    secret_state::{RawSecret, SecretStateContainer, SecuredSecret},
    SecretManagementInterface, SecretsManagementError,
};

use crate::settings::{self, Settings};

#[async_trait::async_trait]
impl SecretsHandler for settings::Database {
    async fn convert_to_raw_secret(
        value: SecretStateContainer<Self, SecuredSecret>,
        secret_management_client: &dyn SecretManagementInterface,
    ) -> CustomResult<SecretStateContainer<Self, RawSecret>, SecretsManagementError> {
        let db = value.get_inner();
        let db_password = secret_management_client
            .get_secret(db.password.clone())
            .await?;

        Ok(value.transition_state(|db| Self {
            password: db_password,
            ..db
        }))
    }
}

#[async_trait::async_trait]
impl SecretsHandler for settings::Jwekey {
    async fn convert_to_raw_secret(
        value: SecretStateContainer<Self, SecuredSecret>,
        secret_management_client: &dyn SecretManagementInterface,
    ) -> CustomResult<SecretStateContainer<Self, RawSecret>, SecretsManagementError> {
        let jwekey = value.get_inner();
        let (
            vault_encryption_key,
            rust_locker_encryption_key,
            vault_private_key,
            tunnel_private_key,
        ) = tokio::try_join!(
            secret_management_client.get_secret(jwekey.vault_encryption_key.clone()),
            secret_management_client.get_secret(jwekey.rust_locker_encryption_key.clone()),
            secret_management_client.get_secret(jwekey.vault_private_key.clone()),
            secret_management_client.get_secret(jwekey.tunnel_private_key.clone())
        )?;
        Ok(value.transition_state(|_| Self {
            vault_encryption_key,
            rust_locker_encryption_key,
            vault_private_key,
            tunnel_private_key,
        }))
    }
}

#[cfg(feature = "olap")]
#[async_trait::async_trait]
impl SecretsHandler for settings::ConnectorOnboarding {
    async fn convert_to_raw_secret(
        value: SecretStateContainer<Self, SecuredSecret>,
        secret_management_client: &dyn SecretManagementInterface,
    ) -> CustomResult<SecretStateContainer<Self, RawSecret>, SecretsManagementError> {
        let onboarding_config = &value.get_inner().paypal;

        let (client_id, client_secret, partner_id) = tokio::try_join!(
            secret_management_client.get_secret(onboarding_config.client_id.clone()),
            secret_management_client.get_secret(onboarding_config.client_secret.clone()),
            secret_management_client.get_secret(onboarding_config.partner_id.clone())
        )?;

        Ok(value.transition_state(|onboarding_config| Self {
            paypal: settings::PayPalOnboarding {
                client_id,
                client_secret,
                partner_id,
                ..onboarding_config.paypal
            },
        }))
    }
}

#[async_trait::async_trait]
impl SecretsHandler for settings::ForexApi {
    async fn convert_to_raw_secret(
        value: SecretStateContainer<Self, SecuredSecret>,
        secret_management_client: &dyn SecretManagementInterface,
    ) -> CustomResult<SecretStateContainer<Self, RawSecret>, SecretsManagementError> {
        let forex_api = value.get_inner();

        let (api_key, fallback_api_key) = tokio::try_join!(
            secret_management_client.get_secret(forex_api.api_key.clone()),
            secret_management_client.get_secret(forex_api.fallback_api_key.clone()),
        )?;

        Ok(value.transition_state(|forex_api| Self {
            api_key,
            fallback_api_key,
            ..forex_api
        }))
    }
}

#[async_trait::async_trait]
impl SecretsHandler for settings::ApiKeys {
    async fn convert_to_raw_secret(
        value: SecretStateContainer<Self, SecuredSecret>,
        secret_management_client: &dyn SecretManagementInterface,
    ) -> CustomResult<SecretStateContainer<Self, RawSecret>, SecretsManagementError> {
        let api_keys = value.get_inner();

        let hash_key = secret_management_client
            .get_secret(api_keys.hash_key.clone())
            .await?;

        #[cfg(feature = "email")]
        let expiry_reminder_days = api_keys.expiry_reminder_days.clone();

        Ok(value.transition_state(|_| Self {
            hash_key,
            #[cfg(feature = "email")]
            expiry_reminder_days,
        }))
    }
}

#[async_trait::async_trait]
impl SecretsHandler for settings::ApplePayDecryptConifg {
    async fn convert_to_raw_secret(
        value: SecretStateContainer<Self, SecuredSecret>,
        secret_management_client: &dyn SecretManagementInterface,
    ) -> CustomResult<SecretStateContainer<Self, RawSecret>, SecretsManagementError> {
        let applepay_decrypt_keys = value.get_inner();

        let (
            apple_pay_ppc,
            apple_pay_ppc_key,
            apple_pay_merchant_cert,
            apple_pay_merchant_cert_key,
        ) = tokio::try_join!(
            secret_management_client.get_secret(applepay_decrypt_keys.apple_pay_ppc.clone()),
            secret_management_client.get_secret(applepay_decrypt_keys.apple_pay_ppc_key.clone()),
            secret_management_client
                .get_secret(applepay_decrypt_keys.apple_pay_merchant_cert.clone()),
            secret_management_client
                .get_secret(applepay_decrypt_keys.apple_pay_merchant_cert_key.clone()),
        )?;

        Ok(value.transition_state(|_| Self {
            apple_pay_ppc,
            apple_pay_ppc_key,
            apple_pay_merchant_cert,
            apple_pay_merchant_cert_key,
        }))
    }
}

#[async_trait::async_trait]
impl SecretsHandler for settings::ApplepayMerchantConfigs {
    async fn convert_to_raw_secret(
        value: SecretStateContainer<Self, SecuredSecret>,
        secret_management_client: &dyn SecretManagementInterface,
    ) -> CustomResult<SecretStateContainer<Self, RawSecret>, SecretsManagementError> {
        let applepay_merchant_configs = value.get_inner();

        let (merchant_cert, merchant_cert_key, common_merchant_identifier) = tokio::try_join!(
            secret_management_client.get_secret(applepay_merchant_configs.merchant_cert.clone()),
            secret_management_client
                .get_secret(applepay_merchant_configs.merchant_cert_key.clone()),
            secret_management_client
                .get_secret(applepay_merchant_configs.common_merchant_identifier.clone()),
        )?;

        Ok(value.transition_state(|applepay_merchant_configs| Self {
            merchant_cert,
            merchant_cert_key,
            common_merchant_identifier,
            ..applepay_merchant_configs
        }))
    }
}

#[async_trait::async_trait]
impl SecretsHandler for settings::PaymentMethodAuth {
    async fn convert_to_raw_secret(
        value: SecretStateContainer<Self, SecuredSecret>,
        secret_management_client: &dyn SecretManagementInterface,
    ) -> CustomResult<SecretStateContainer<Self, RawSecret>, SecretsManagementError> {
        let payment_method_auth = value.get_inner();

        let pm_auth_key = secret_management_client
            .get_secret(payment_method_auth.pm_auth_key.clone())
            .await?;

        Ok(value.transition_state(|payment_method_auth| Self {
            pm_auth_key,
            ..payment_method_auth
        }))
    }
}

#[async_trait::async_trait]
impl SecretsHandler for settings::Secrets {
    async fn convert_to_raw_secret(
        value: SecretStateContainer<Self, SecuredSecret>,
        secret_management_client: &dyn SecretManagementInterface,
    ) -> CustomResult<SecretStateContainer<Self, RawSecret>, SecretsManagementError> {
        let secrets = value.get_inner();
        let (jwt_secret, admin_api_key, recon_admin_api_key, master_enc_key) = tokio::try_join!(
            secret_management_client.get_secret(secrets.jwt_secret.clone()),
            secret_management_client.get_secret(secrets.admin_api_key.clone()),
            secret_management_client.get_secret(secrets.recon_admin_api_key.clone()),
            secret_management_client.get_secret(secrets.master_enc_key.clone())
        )?;

        Ok(value.transition_state(|_| Self {
            jwt_secret,
            admin_api_key,
            recon_admin_api_key,
            master_enc_key,
        }))
    }
}

/// # Panics
///
/// Will panic even if kms decryption fails for at least one field
pub(crate) async fn fetch_raw_secrets(
    conf: Settings<SecuredSecret>,
    secret_management_client: &dyn SecretManagementInterface,
) -> Settings<RawSecret> {
    #[allow(clippy::expect_used)]
    let master_database =
        settings::Database::convert_to_raw_secret(conf.master_database, secret_management_client)
            .await
            .expect("Failed to decrypt master database configuration");

    #[cfg(feature = "olap")]
    #[allow(clippy::expect_used)]
    let replica_database =
        settings::Database::convert_to_raw_secret(conf.replica_database, secret_management_client)
            .await
            .expect("Failed to decrypt replica database configuration");

    #[allow(clippy::expect_used)]
    let secrets = settings::Secrets::convert_to_raw_secret(conf.secrets, secret_management_client)
        .await
        .expect("Failed to decrypt secrets");

    #[allow(clippy::expect_used)]
    let forex_api =
        settings::ForexApi::convert_to_raw_secret(conf.forex_api, secret_management_client)
            .await
            .expect("Failed to decrypt forex api configs");

    #[allow(clippy::expect_used)]
    let jwekey = settings::Jwekey::convert_to_raw_secret(conf.jwekey, secret_management_client)
        .await
        .expect("Failed to decrypt jwekey configs");

    #[allow(clippy::expect_used)]
    let api_keys =
        settings::ApiKeys::convert_to_raw_secret(conf.api_keys, secret_management_client)
            .await
            .expect("Failed to decrypt api_keys configs");

    #[cfg(feature = "olap")]
    #[allow(clippy::expect_used)]
    let connector_onboarding = settings::ConnectorOnboarding::convert_to_raw_secret(
        conf.connector_onboarding,
        secret_management_client,
    )
    .await
    .expect("Failed to decrypt connector_onboarding configs");

    #[allow(clippy::expect_used)]
    let applepay_decrypt_keys = settings::ApplePayDecryptConifg::convert_to_raw_secret(
        conf.applepay_decrypt_keys,
        secret_management_client,
    )
    .await
    .expect("Failed to decrypt applepay decrypt configs");

    #[allow(clippy::expect_used)]
    let applepay_merchant_configs = settings::ApplepayMerchantConfigs::convert_to_raw_secret(
        conf.applepay_merchant_configs,
        secret_management_client,
    )
    .await
    .expect("Failed to decrypt applepay merchant configs");

    #[allow(clippy::expect_used)]
    let payment_method_auth = settings::PaymentMethodAuth::convert_to_raw_secret(
        conf.payment_method_auth,
        secret_management_client,
    )
    .await
    .expect("Failed to decrypt payment method auth configs");

    Settings {
        server: conf.server,
        master_database,
        redis: conf.redis,
        log: conf.log,
        #[cfg(feature = "kv_store")]
        drainer: conf.drainer,
        encryption_management: conf.encryption_management,
        secrets_management: conf.secrets_management,
        proxy: conf.proxy,
        env: conf.env,
        #[cfg(feature = "olap")]
        replica_database,
        secrets,
        locker: conf.locker,
        connectors: conf.connectors,
        forex_api,
        refund: conf.refund,
        eph_key: conf.eph_key,
        scheduler: conf.scheduler,
        jwekey,
        webhooks: conf.webhooks,
        pm_filters: conf.pm_filters,
        bank_config: conf.bank_config,
        api_keys,
        file_storage: conf.file_storage,
        tokenization: conf.tokenization,
        connector_customer: conf.connector_customer,
        #[cfg(feature = "dummy_connector")]
        dummy_connector: conf.dummy_connector,
        #[cfg(feature = "email")]
        email: conf.email,
        mandates: conf.mandates,
        required_fields: conf.required_fields,
        delayed_session_response: conf.delayed_session_response,
        webhook_source_verification_call: conf.webhook_source_verification_call,
        payment_method_auth,
        connector_request_reference_id_config: conf.connector_request_reference_id_config,
        #[cfg(feature = "payouts")]
        payouts: conf.payouts,
        applepay_decrypt_keys,
        multiple_api_version_supported_connectors: conf.multiple_api_version_supported_connectors,
        applepay_merchant_configs,
        lock_settings: conf.lock_settings,
        temp_locker_enable_config: conf.temp_locker_enable_config,
        payment_link: conf.payment_link,
        #[cfg(feature = "olap")]
        analytics: conf.analytics,
        #[cfg(feature = "kv_store")]
        kv_config: conf.kv_config,
        #[cfg(feature = "frm")]
        frm: conf.frm,
        #[cfg(feature = "olap")]
        report_download_config: conf.report_download_config,
        events: conf.events,
        #[cfg(feature = "olap")]
        connector_onboarding,
        cors: conf.cors,
    }
}
