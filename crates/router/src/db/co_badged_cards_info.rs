use common_utils::types::keymanager::KeyManagerState;
use diesel_models::{self, CoBadgedCardInfo, UpdateCoBadgedCardInfo};
use error_stack::{report, ResultExt};
use futures::future::try_join_all;
use hyperswitch_domain_models::{
    behaviour::{Conversion, ReverseConversion},
    co_badged_cards_info,
};
use storage_impl::MockDb;

use super::domain;
use crate::{
    connection,
    core::errors::{self, CustomResult},
    db::kafka_store::KafkaStore,
    services::Store,
};

#[async_trait::async_trait]
pub trait CoBadgedCardInfoInterface {
    async fn add_co_badged_cards_info(
        &self,
        key_manager_state: &KeyManagerState,
        merchant_key_store: &domain::MerchantKeyStore,
        data: co_badged_cards_info::CoBadgedCardInfo,
    ) -> CustomResult<co_badged_cards_info::CoBadgedCardInfo, errors::StorageError>;

    async fn find_co_badged_cards_info_by_card_bin(
        &self,
        key_manager_state: &KeyManagerState,
        merchant_key_store: &domain::MerchantKeyStore,
        card_number: i64,
    ) -> CustomResult<Vec<co_badged_cards_info::CoBadgedCardInfo>, errors::StorageError>;

    async fn update_co_badged_cards_info(
        &self,
        key_manager_state: &KeyManagerState,
        merchant_key_store: &domain::MerchantKeyStore,
        current_state: co_badged_cards_info::CoBadgedCardInfo,
        update_co_badged_cards_info: co_badged_cards_info::UpdateCoBadgedCardInfo,
    ) -> CustomResult<co_badged_cards_info::CoBadgedCardInfo, errors::StorageError>;
}

#[async_trait::async_trait]
impl CoBadgedCardInfoInterface for Store {
    async fn add_co_badged_cards_info(
        &self,
        key_manager_state: &KeyManagerState,
        merchant_key_store: &domain::MerchantKeyStore,
        data: co_badged_cards_info::CoBadgedCardInfo,
    ) -> CustomResult<co_badged_cards_info::CoBadgedCardInfo, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        data.construct_new()
            .await
            .change_context(errors::StorageError::EncryptionError)?
            .insert(&conn)
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))?
            .convert(
                key_manager_state,
                merchant_key_store.key.get_inner(),
                merchant_key_store.merchant_id.clone().into(),
            )
            .await
            .change_context(errors::StorageError::DecryptionError)
    }
    async fn find_co_badged_cards_info_by_card_bin(
        &self,
        key_manager_state: &KeyManagerState,
        merchant_key_store: &domain::MerchantKeyStore,
        card_bin: i64,
    ) -> CustomResult<Vec<co_badged_cards_info::CoBadgedCardInfo>, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        let co_badged_cards_info_list = try_join_all(
            CoBadgedCardInfo::find_by_bin(&conn, card_bin)
                .await
                .map_err(|error| report!(errors::StorageError::from(error)))?
                .into_iter()
                .map(|a| async {
                    a.convert(
                        key_manager_state,
                        merchant_key_store.key.get_inner(),
                        merchant_key_store.merchant_id.clone().into(),
                    )
                    .await
                    .change_context(errors::StorageError::DecryptionError)
                }),
        )
        .await?;
        Ok(co_badged_cards_info_list)
    }

    async fn update_co_badged_cards_info(
        &self,
        key_manager_state: &KeyManagerState,
        merchant_key_store: &domain::MerchantKeyStore,
        current_state: co_badged_cards_info::CoBadgedCardInfo,
        update_co_badged_cards_info: co_badged_cards_info::UpdateCoBadgedCardInfo,
    ) -> CustomResult<co_badged_cards_info::CoBadgedCardInfo, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        Conversion::convert(current_state)
            .await
            .change_context(errors::StorageError::EncryptionError)?
            .update(
                &conn,
                UpdateCoBadgedCardInfo::from(update_co_badged_cards_info),
            )
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))?
            .convert(
                key_manager_state,
                merchant_key_store.key.get_inner(),
                merchant_key_store.merchant_id.clone().into(),
            )
            .await
            .change_context(errors::StorageError::DecryptionError)
    }
}

#[async_trait::async_trait]
impl CoBadgedCardInfoInterface for MockDb {
    async fn add_co_badged_cards_info(
        &self,
        _key_manager_state: &KeyManagerState,
        _merchant_key_store: &domain::MerchantKeyStore,
        _data: co_badged_cards_info::CoBadgedCardInfo,
    ) -> CustomResult<co_badged_cards_info::CoBadgedCardInfo, errors::StorageError> {
        Err(errors::StorageError::MockDbError)?
    }

    async fn find_co_badged_cards_info_by_card_bin(
        &self,
        _key_manager_state: &KeyManagerState,
        _merchant_key_store: &domain::MerchantKeyStore,
        _card_number: i64,
    ) -> CustomResult<Vec<co_badged_cards_info::CoBadgedCardInfo>, errors::StorageError> {
        Err(errors::StorageError::MockDbError)?
    }

    async fn update_co_badged_cards_info(
        &self,
        _key_manager_state: &KeyManagerState,
        _merchant_key_store: &domain::MerchantKeyStore,
        _current_state: co_badged_cards_info::CoBadgedCardInfo,
        _update_co_badged_cards_info: co_badged_cards_info::UpdateCoBadgedCardInfo,
    ) -> CustomResult<co_badged_cards_info::CoBadgedCardInfo, errors::StorageError> {
        Err(errors::StorageError::MockDbError)?
    }
}

#[async_trait::async_trait]
impl CoBadgedCardInfoInterface for KafkaStore {
    async fn add_co_badged_cards_info(
        &self,
        key_manager_state: &KeyManagerState,
        merchant_key_store: &domain::MerchantKeyStore,
        data: co_badged_cards_info::CoBadgedCardInfo,
    ) -> CustomResult<co_badged_cards_info::CoBadgedCardInfo, errors::StorageError> {
        self.diesel_store
            .add_co_badged_cards_info(key_manager_state, merchant_key_store, data)
            .await
    }
    async fn find_co_badged_cards_info_by_card_bin(
        &self,
        key_manager_state: &KeyManagerState,
        merchant_key_store: &domain::MerchantKeyStore,
        card_number: i64,
    ) -> CustomResult<Vec<co_badged_cards_info::CoBadgedCardInfo>, errors::StorageError> {
        self.diesel_store
            .find_co_badged_cards_info_by_card_bin(
                key_manager_state,
                merchant_key_store,
                card_number,
            )
            .await
    }

    async fn update_co_badged_cards_info(
        &self,
        key_manager_state: &KeyManagerState,
        merchant_key_store: &domain::MerchantKeyStore,
        current_state: co_badged_cards_info::CoBadgedCardInfo,
        update_co_badged_cards_info: co_badged_cards_info::UpdateCoBadgedCardInfo,
    ) -> CustomResult<co_badged_cards_info::CoBadgedCardInfo, errors::StorageError> {
        self.diesel_store
            .update_co_badged_cards_info(
                key_manager_state,
                merchant_key_store,
                current_state,
                update_co_badged_cards_info,
            )
            .await
    }
}
