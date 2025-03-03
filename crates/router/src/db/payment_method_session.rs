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
        payment_methods_session: hyperswitch_domain_models::payment_methods::PaymentMethodSession,
        validity: i64,
    ) -> CustomResult<(), errors::StorageError>;

    async fn update_payment_method_session(
        &self,
        state: &common_utils::types::keymanager::KeyManagerState,
        key_store: &hyperswitch_domain_models::merchant_key_store::MerchantKeyStore,
        id: &common_utils::id_type::GlobalPaymentMethodSessionId,
        payment_methods_session: hyperswitch_domain_models::payment_methods::PaymentMethodsSessionUpdateEnum,
        current_session: hyperswitch_domain_models::payment_methods::PaymentMethodSession,
    ) -> CustomResult<
        hyperswitch_domain_models::payment_methods::PaymentMethodSession,
        errors::StorageError,
    >;

    async fn get_payment_methods_session(
        &self,
        state: &common_utils::types::keymanager::KeyManagerState,
        key_store: &hyperswitch_domain_models::merchant_key_store::MerchantKeyStore,
        id: &common_utils::id_type::GlobalPaymentMethodSessionId,
    ) -> CustomResult<
        hyperswitch_domain_models::payment_methods::PaymentMethodSession,
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
            payment_methods_session: hyperswitch_domain_models::payment_methods::PaymentMethodSession,
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
            hyperswitch_domain_models::payment_methods::PaymentMethodSession,
            errors::StorageError,
        > {
            let redis_key = id.get_redis_key();

            let redis_connection = self
                .get_redis_conn()
                .map_err(Into::<errors::StorageError>::into)?;

            let db_model = redis_connection
                .get_and_deserialize_key::<diesel_models::payment_methods_session::PaymentMethodSession>(&redis_key.into(), "PaymentMethodSession")
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

        #[instrument(skip_all)]
        async fn update_payment_method_session(
            &self,
            state: &common_utils::types::keymanager::KeyManagerState,
            key_store: &hyperswitch_domain_models::merchant_key_store::MerchantKeyStore,
            session_id: &common_utils::id_type::GlobalPaymentMethodSessionId,
            update_request: hyperswitch_domain_models::payment_methods::PaymentMethodsSessionUpdateEnum,
            current_session: hyperswitch_domain_models::payment_methods::PaymentMethodSession,
        ) -> CustomResult<
            hyperswitch_domain_models::payment_methods::PaymentMethodSession,
            errors::StorageError,
        > {
            let redis_key = session_id.get_redis_key();

            let internal_obj = hyperswitch_domain_models::payment_methods::PaymentMethodsSessionUpdateInternal::from(update_request);

            let update_state = current_session.apply_changeset(internal_obj);

            let db_model = update_state
                .construct_new()
                .await
                .change_context(errors::StorageError::EncryptionError)?;

            let redis_connection = self
                .get_redis_conn()
                .map_err(Into::<errors::StorageError>::into)?;

            redis_connection
                .serialize_and_set_key_without_modifying_ttl(&redis_key.into(), db_model.clone())
                .await
                .change_context(errors::StorageError::KVError)
                .attach_printable("Failed to insert payment methods session to redis");

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
        payment_methods_session: hyperswitch_domain_models::payment_methods::PaymentMethodSession,
        validity_in_seconds: i64,
    ) -> CustomResult<(), errors::StorageError> {
        Err(errors::StorageError::MockDbError)?
    }

    async fn update_payment_method_session(
        &self,
        state: &common_utils::types::keymanager::KeyManagerState,
        key_store: &hyperswitch_domain_models::merchant_key_store::MerchantKeyStore,
        id: &common_utils::id_type::GlobalPaymentMethodSessionId,
        payment_methods_session: hyperswitch_domain_models::payment_methods::PaymentMethodsSessionUpdateEnum,
        current_session: hyperswitch_domain_models::payment_methods::PaymentMethodSession,
    ) -> CustomResult<
        hyperswitch_domain_models::payment_methods::PaymentMethodSession,
        errors::StorageError,
    > {
        Err(errors::StorageError::MockDbError)?
    }

    #[cfg(feature = "v2")]
    async fn get_payment_methods_session(
        &self,
        state: &common_utils::types::keymanager::KeyManagerState,
        key_store: &hyperswitch_domain_models::merchant_key_store::MerchantKeyStore,
        id: &common_utils::id_type::GlobalPaymentMethodSessionId,
    ) -> CustomResult<
        hyperswitch_domain_models::payment_methods::PaymentMethodSession,
        errors::StorageError,
    > {
        Err(errors::StorageError::MockDbError)?
    }
}

#[cfg(feature = "v1")]
#[async_trait::async_trait]
impl PaymentMethodsSessionInterface for MockDb {}
