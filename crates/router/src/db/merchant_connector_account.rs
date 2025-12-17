use common_utils::ext_traits::{ByteSliceExt, Encode};
use error_stack::ResultExt;
pub use hyperswitch_domain_models::merchant_connector_account::MerchantConnectorAccountInterface;
use router_env::{instrument, tracing};
use storage_impl::redis::kv_store::RedisConnInterface;

use super::{MockDb, Store};
use crate::{
    core::errors::{self, CustomResult},
    types,
};

#[async_trait::async_trait]
pub trait ConnectorAccessToken {
    async fn get_access_token(
        &self,
        &key: String,
    ) -> CustomResult<Option<types::AccessToken>, errors::StorageError>;

    async fn set_access_token(
        &self,
        key: String,
        access_token: types::AccessToken,
    ) -> CustomResult<(), errors::StorageError>;
}

#[async_trait::async_trait]
impl ConnectorAccessToken for Store {
    #[instrument(skip_all)]
    async fn get_access_token(
        &self,
        key: String,
    ) -> CustomResult<Option<types::AccessToken>, errors::StorageError> {
        //TODO: Handle race condition
        // This function should acquire a global lock on some resource, if access token is already
        // being refreshed by other request then wait till it finishes and use the same access token

        let maybe_token = self
            .get_redis_conn()
            .map_err(Into::<errors::StorageError>::into)?
            .get_key::<Option<Vec<u8>>>(&key.into())
            .await
            .change_context(errors::StorageError::KVError)
            .attach_printable("DB error when getting access token")?;

        let access_token = maybe_token
            .map(|token| token.parse_struct::<types::AccessToken>("AccessToken"))
            .transpose()
            .change_context(errors::StorageError::DeserializationFailed)?;

        Ok(access_token)
    }

    #[instrument(skip_all)]
    async fn set_access_token(
        &self,
        key: String,
        access_token: types::AccessToken,
    ) -> CustomResult<(), errors::StorageError> {
        let serialized_access_token = access_token
            .encode_to_string_of_json()
            .change_context(errors::StorageError::SerializationFailed)?;
        self.get_redis_conn()
            .map_err(Into::<errors::StorageError>::into)?
            .set_key_with_expiry(&key.into(), serialized_access_token, access_token.expires)
            .await
            .change_context(errors::StorageError::KVError)
    }
}

#[async_trait::async_trait]
impl ConnectorAccessToken for MockDb {
    async fn get_access_token(
        &self,
        _key: String,
    ) -> CustomResult<Option<types::AccessToken>, errors::StorageError> {
        Ok(None)
    }

    async fn set_access_token(
        &self,
        _key: String,
        _access_token: types::AccessToken,
    ) -> CustomResult<(), errors::StorageError> {
        Ok(())
    }
}

#[cfg(feature = "accounts_cache")]
#[cfg(test)]
mod merchant_connector_account_cache_tests {
    use std::sync::Arc;

    #[cfg(feature = "v1")]
    use api_models::enums::CountryAlpha2;
    use common_utils::{
        date_time, type_name,
        types::keymanager::{Identifier, KeyManagerState},
    };
    use diesel_models::enums::ConnectorType;
    use error_stack::ResultExt;
    use hyperswitch_domain_models::master_key::MasterKeyInterface;
    use masking::PeekInterface;
    use storage_impl::redis::{
        cache::{self, CacheKey, CacheKind, ACCOUNTS_CACHE},
        kv_store::RedisConnInterface,
        pub_sub::PubSubInterface,
    };
    use time::macros::datetime;
    use tokio::sync::oneshot;

    use crate::{
        core::errors,
        db::{
            merchant_connector_account::MerchantConnectorAccountInterface,
            merchant_key_store::MerchantKeyStoreInterface, MockDb,
        },
        routes::{
            self,
            app::{settings::Settings, StorageImpl},
        },
        services,
        types::{
            domain::{self, behaviour::Conversion},
            storage,
        },
    };

    #[tokio::test]
    #[cfg(feature = "v1")]
    async fn test_connector_profile_id_cache() {
        let conf = Settings::new().unwrap();
        let tx: oneshot::Sender<()> = oneshot::channel().0;

        let app_state = Box::pin(routes::AppState::with_storage(
            conf,
            StorageImpl::PostgresqlTest,
            tx,
            Box::new(services::MockApiClient),
        ))
        .await;

        let state = &Arc::new(app_state)
            .get_session_state(
                &common_utils::id_type::TenantId::try_from_string("public".to_string()).unwrap(),
                None,
                || {},
            )
            .unwrap();
        let db = MockDb::new(
            &redis_interface::RedisSettings::default(),
            KeyManagerState::new(),
        )
        .await
        .expect("Failed to create Mock store");

        let redis_conn = db.get_redis_conn().unwrap();
        let master_key = db.get_master_key();
        redis_conn
            .subscribe("hyperswitch_invalidate")
            .await
            .unwrap();

        let merchant_id =
            common_utils::id_type::MerchantId::try_from(std::borrow::Cow::from("test_merchant"))
                .unwrap();

        let connector_label = "stripe_USA";
        let merchant_connector_id = "simple_merchant_connector_id";
        let profile_id =
            common_utils::id_type::ProfileId::try_from(std::borrow::Cow::from("pro_max_ultra"))
                .unwrap();
        let key_manager_state = &state.into();
        db.insert_merchant_key_store(
            domain::MerchantKeyStore {
                merchant_id: merchant_id.clone(),
                key: domain::types::crypto_operation(
                    key_manager_state,
                    type_name!(domain::MerchantKeyStore),
                    domain::types::CryptoOperation::Encrypt(
                        services::generate_aes256_key().unwrap().to_vec().into(),
                    ),
                    Identifier::Merchant(merchant_id.clone()),
                    master_key,
                )
                .await
                .and_then(|val| val.try_into_operation())
                .unwrap(),
                created_at: datetime!(2023-02-01 0:00),
            },
            &master_key.to_vec().into(),
        )
        .await
        .unwrap();

        let merchant_key = db
            .get_merchant_key_store_by_merchant_id(&merchant_id, &master_key.to_vec().into())
            .await
            .unwrap();

        let mca = domain::MerchantConnectorAccount {
            merchant_id: merchant_id.to_owned(),
            connector_name: "stripe".to_string(),
            connector_account_details: domain::types::crypto_operation(
                key_manager_state,
                type_name!(domain::MerchantConnectorAccount),
                domain::types::CryptoOperation::Encrypt(serde_json::Value::default().into()),
                Identifier::Merchant(merchant_key.merchant_id.clone()),
                merchant_key.key.get_inner().peek(),
            )
            .await
            .and_then(|val| val.try_into_operation())
            .unwrap(),
            test_mode: None,
            disabled: None,
            merchant_connector_id: common_utils::id_type::MerchantConnectorAccountId::wrap(
                merchant_connector_id.to_string(),
            )
            .unwrap(),
            payment_methods_enabled: None,
            connector_type: ConnectorType::FinOperations,
            metadata: None,
            frm_configs: None,
            connector_label: Some(connector_label.to_string()),
            business_country: Some(CountryAlpha2::US),
            business_label: Some("cloth".to_string()),
            business_sub_label: None,
            created_at: date_time::now(),
            modified_at: date_time::now(),
            connector_webhook_details: None,
            profile_id: profile_id.to_owned(),
            applepay_verified_domains: None,
            pm_auth_config: None,
            status: common_enums::ConnectorStatus::Inactive,
            connector_wallets_details: Some(
                domain::types::crypto_operation(
                    key_manager_state,
                    type_name!(domain::MerchantConnectorAccount),
                    domain::types::CryptoOperation::Encrypt(serde_json::Value::default().into()),
                    Identifier::Merchant(merchant_key.merchant_id.clone()),
                    merchant_key.key.get_inner().peek(),
                )
                .await
                .and_then(|val| val.try_into_operation())
                .unwrap(),
            ),
            additional_merchant_data: None,
            version: common_types::consts::API_VERSION,
        };

        db.insert_merchant_connector_account(mca.clone(), &merchant_key)
            .await
            .unwrap();

        let find_call = || async {
            Conversion::convert(
                db.find_merchant_connector_account_by_profile_id_connector_name(
                    &profile_id,
                    &mca.connector_name,
                    &merchant_key,
                )
                .await
                .unwrap(),
            )
            .await
            .change_context(errors::StorageError::DecryptionError)
        };
        let _: storage::MerchantConnectorAccount = cache::get_or_populate_in_memory(
            &db,
            &format!(
                "{}_{}",
                merchant_id.get_string_repr(),
                profile_id.get_string_repr(),
            ),
            find_call,
            &ACCOUNTS_CACHE,
        )
        .await
        .unwrap();

        let delete_call = || async {
            db.delete_merchant_connector_account_by_merchant_id_merchant_connector_id(
                &merchant_id,
                &common_utils::id_type::MerchantConnectorAccountId::wrap(
                    merchant_connector_id.to_string(),
                )
                .unwrap(),
            )
            .await
        };

        cache::publish_and_redact(
            &db,
            CacheKind::Accounts(
                format!("{}_{}", merchant_id.get_string_repr(), connector_label).into(),
            ),
            delete_call,
        )
        .await
        .unwrap();

        assert!(ACCOUNTS_CACHE
            .get_val::<domain::MerchantConnectorAccount>(CacheKey {
                key: format!("{}_{}", merchant_id.get_string_repr(), connector_label),
                prefix: String::default(),
            },)
            .await
            .is_none())
    }

    #[tokio::test]
    #[cfg(feature = "v2")]
    async fn test_connector_profile_id_cache() {
        let conf = Settings::new().unwrap();
        let tx: oneshot::Sender<()> = oneshot::channel().0;

        let app_state = Box::pin(routes::AppState::with_storage(
            conf,
            StorageImpl::PostgresqlTest,
            tx,
            Box::new(services::MockApiClient),
        ))
        .await;
        let state = &Arc::new(app_state)
            .get_session_state(
                &common_utils::id_type::TenantId::try_from_string("public".to_string()).unwrap(),
                None,
                || {},
            )
            .unwrap();
        let db = MockDb::new(
            &redis_interface::RedisSettings::default(),
            KeyManagerState::new(),
        )
        .await
        .expect("Failed to create Mock store");

        let redis_conn = db.get_redis_conn().unwrap();
        let master_key = db.get_master_key();
        redis_conn
            .subscribe("hyperswitch_invalidate")
            .await
            .unwrap();

        let merchant_id =
            common_utils::id_type::MerchantId::try_from(std::borrow::Cow::from("test_merchant"))
                .unwrap();
        let connector_label = "stripe_USA";
        let id = common_utils::generate_merchant_connector_account_id_of_default_length();
        let profile_id =
            common_utils::id_type::ProfileId::try_from(std::borrow::Cow::from("pro_max_ultra"))
                .unwrap();
        let key_manager_state = &state.into();
        db.insert_merchant_key_store(
            domain::MerchantKeyStore {
                merchant_id: merchant_id.clone(),
                key: domain::types::crypto_operation(
                    key_manager_state,
                    type_name!(domain::MerchantConnectorAccount),
                    domain::types::CryptoOperation::Encrypt(
                        services::generate_aes256_key().unwrap().to_vec().into(),
                    ),
                    Identifier::Merchant(merchant_id.clone()),
                    master_key,
                )
                .await
                .and_then(|val| val.try_into_operation())
                .unwrap(),
                created_at: datetime!(2023-02-01 0:00),
            },
            &master_key.to_vec().into(),
        )
        .await
        .unwrap();

        let merchant_key = db
            .get_merchant_key_store_by_merchant_id(&merchant_id, &master_key.to_vec().into())
            .await
            .unwrap();

        let mca = domain::MerchantConnectorAccount {
            id: id.clone(),
            merchant_id: merchant_id.clone(),
            connector_name: common_enums::connector_enums::Connector::Stripe,
            connector_account_details: domain::types::crypto_operation(
                key_manager_state,
                type_name!(domain::MerchantConnectorAccount),
                domain::types::CryptoOperation::Encrypt(serde_json::Value::default().into()),
                Identifier::Merchant(merchant_key.merchant_id.clone()),
                merchant_key.key.get_inner().peek(),
            )
            .await
            .and_then(|val| val.try_into_operation())
            .unwrap(),
            disabled: None,
            payment_methods_enabled: None,
            connector_type: ConnectorType::FinOperations,
            metadata: None,
            frm_configs: None,
            connector_label: Some(connector_label.to_string()),
            created_at: date_time::now(),
            modified_at: date_time::now(),
            connector_webhook_details: None,
            profile_id: profile_id.to_owned(),
            applepay_verified_domains: None,
            pm_auth_config: None,
            status: common_enums::ConnectorStatus::Inactive,
            connector_wallets_details: Some(
                domain::types::crypto_operation(
                    key_manager_state,
                    type_name!(domain::MerchantConnectorAccount),
                    domain::types::CryptoOperation::Encrypt(serde_json::Value::default().into()),
                    Identifier::Merchant(merchant_key.merchant_id.clone()),
                    merchant_key.key.get_inner().peek(),
                )
                .await
                .and_then(|val| val.try_into_operation())
                .unwrap(),
            ),
            additional_merchant_data: None,
            version: common_types::consts::API_VERSION,
            feature_metadata: None,
        };

        db.insert_merchant_connector_account(mca.clone(), &merchant_key)
            .await
            .unwrap();

        let find_call = || async {
            #[cfg(feature = "v1")]
            let mca = db
                .find_merchant_connector_account_by_profile_id_connector_name(
                    profile_id,
                    &mca.connector_name,
                    &merchant_key,
                )
                .await
                .unwrap();
            #[cfg(feature = "v2")]
            let mca: domain::MerchantConnectorAccount = { todo!() };
            Conversion::convert(mca)
                .await
                .change_context(errors::StorageError::DecryptionError)
        };

        let _: storage::MerchantConnectorAccount = cache::get_or_populate_in_memory(
            &db,
            &format!(
                "{}_{}",
                merchant_id.clone().get_string_repr(),
                profile_id.get_string_repr()
            ),
            find_call,
            &ACCOUNTS_CACHE,
        )
        .await
        .unwrap();

        let delete_call = || async { db.delete_merchant_connector_account_by_id(&id).await };

        cache::publish_and_redact(
            &db,
            CacheKind::Accounts(
                format!("{}_{}", merchant_id.get_string_repr(), connector_label).into(),
            ),
            delete_call,
        )
        .await
        .unwrap();

        assert!(ACCOUNTS_CACHE
            .get_val::<domain::MerchantConnectorAccount>(CacheKey {
                key: format!("{}_{}", merchant_id.get_string_repr(), connector_label),
                prefix: String::default(),
            },)
            .await
            .is_none())
    }
}
