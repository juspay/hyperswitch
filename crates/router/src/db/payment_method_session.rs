#[cfg(feature = "v2")]
use crate::core::errors::{self, CustomResult};
use crate::db::MockDb;

#[cfg(feature = "v2")]
#[async_trait::async_trait]
pub trait PaymentMethodsSessionInterface {
    async fn insert_payment_methods_session(
        &self,
        state: &common_utils::types::keymanager::KeyManagerState,
        key_store: &hyperswitch_domain_models::merchant_key_store::MerchantKeyStore,
        payment_methods_session: hyperswitch_domain_models::payment_methods::PaymentMethodsSession,
        validity: i64,
    ) -> CustomResult<(), errors::StorageError>;

    async fn get_payment_methods_session(
        &self,
        state: &common_utils::types::keymanager::KeyManagerState,
        key_store: &hyperswitch_domain_models::merchant_key_store::MerchantKeyStore,
        id: &common_utils::id_type::GlobalPaymentMethodSessionId,
    ) -> CustomResult<
        hyperswitch_domain_models::payment_methods::PaymentMethodsSession,
        errors::StorageError,
    >;
}

#[cfg(feature = "v1")]
pub trait PaymentMethodsSessionInterface {}

#[cfg(feature = "v1")]
impl PaymentMethodsSessionInterface for crate::services::Store {}

#[cfg(feature = "v2")]
mod storage {
    use error_stack::ResultExt;
    use hyperswitch_domain_models::behaviour::{Conversion, ReverseConversion};
    use router_env::{instrument, tracing};
    use storage_impl::redis::kv_store::RedisConnInterface;

    use super::PaymentMethodsSessionInterface;
    use crate::{
        core::errors::{self, CustomResult},
        services::Store,
    };

    #[async_trait::async_trait]
    impl PaymentMethodsSessionInterface for Store {
        #[instrument(skip_all)]
        async fn insert_payment_methods_session(
            &self,
            _state: &common_utils::types::keymanager::KeyManagerState,
            _key_store: &hyperswitch_domain_models::merchant_key_store::MerchantKeyStore,
            payment_methods_session: hyperswitch_domain_models::payment_methods::PaymentMethodsSession,
            validity_in_seconds: i64,
        ) -> CustomResult<(), errors::StorageError> {
            let redis_key = payment_methods_session.id.get_redis_key();

            let db_model = payment_methods_session
                .construct_new()
                .await
                .change_context(errors::StorageError::EncryptionError)?;

            let redis_connection = self
                .get_redis_conn()
                .map_err(Into::<errors::StorageError>::into)?;

            redis_connection
                .serialize_and_set_key_with_expiry(&redis_key.into(), db_model, validity_in_seconds)
                .await
                .change_context(errors::StorageError::KVError)
                .attach_printable("Failed to insert payment methods session to redis")
        }

        #[instrument(skip_all)]
        async fn get_payment_methods_session(
            &self,
            state: &common_utils::types::keymanager::KeyManagerState,
            key_store: &hyperswitch_domain_models::merchant_key_store::MerchantKeyStore,
            id: &common_utils::id_type::GlobalPaymentMethodSessionId,
        ) -> CustomResult<
            hyperswitch_domain_models::payment_methods::PaymentMethodsSession,
            errors::StorageError,
        > {
            let redis_key = id.get_redis_key();

            let redis_connection = self
                .get_redis_conn()
                .map_err(Into::<errors::StorageError>::into)?;

            let db_model = redis_connection
                .get_and_deserialize_key::<diesel_models::payment_methods_session::PaymentMethodsSession>(&redis_key.into(), "PaymentMethodsSession")
                .await
                .change_context(errors::StorageError::KVError)?;

            let key_manager_identifier = common_utils::types::keymanager::Identifier::Merchant(
                key_store.merchant_id.clone(),
            );

            db_model
                .convert(state, &key_store.key, key_manager_identifier)
                .await
                .change_context(errors::StorageError::DecryptionError)
                .attach_printable("Failed to decrypt payment methods session")
        }
    }
}

#[cfg(feature = "v2")]
#[async_trait::async_trait]
impl PaymentMethodsSessionInterface for MockDb {
    async fn insert_payment_methods_session(
        &self,
        state: &common_utils::types::keymanager::KeyManagerState,
        key_store: &hyperswitch_domain_models::merchant_key_store::MerchantKeyStore,
        payment_methods_session: hyperswitch_domain_models::payment_methods::PaymentMethodsSession,
        validity_in_seconds: i64,
    ) -> CustomResult<(), errors::StorageError> {
        Err(errors::StorageError::MockDbError)?
    }

    #[cfg(feature = "v2")]
    async fn get_payment_methods_session(
        &self,
        state: &common_utils::types::keymanager::KeyManagerState,
        key_store: &hyperswitch_domain_models::merchant_key_store::MerchantKeyStore,
        id: &common_utils::id_type::GlobalPaymentMethodSessionId,
    ) -> CustomResult<
        hyperswitch_domain_models::payment_methods::PaymentMethodsSession,
        errors::StorageError,
    > {
        Err(errors::StorageError::MockDbError)?
    }
}

#[cfg(feature = "v1")]
#[async_trait::async_trait]
impl PaymentMethodsSessionInterface for MockDb {}
