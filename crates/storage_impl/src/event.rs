use common_utils::{
    errors::CustomResult, ext_traits::AsyncExt, types::keymanager::KeyManagerState,
};
use diesel_models::events as storage;
use error_stack::{report, ResultExt};
use hyperswitch_domain_models::{
    behaviour::{Conversion, ReverseConversion},
    merchant_key_store,
};
use router_env::{instrument, tracing};
use sample::{domain::event as domain, events::EventInterface};

use crate::{connection, errors, DatabaseStore, RouterStore};

#[async_trait::async_trait]
impl<T: DatabaseStore> EventInterface for RouterStore<T> {
    type Error = errors::StorageError;

    #[instrument(skip_all)]
    async fn insert_event(
        &self,
        state: &KeyManagerState,
        event: domain::Event,
        merchant_key_store: &merchant_key_store::MerchantKeyStore,
    ) -> CustomResult<domain::Event, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        event
            .construct_new()
            .await
            .change_context(errors::StorageError::EncryptionError)?
            .insert(&conn)
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))?
            .convert(
                state,
                merchant_key_store.key.get_inner(),
                merchant_key_store.merchant_id.clone().into(),
            )
            .await
            .change_context(errors::StorageError::DecryptionError)
    }

    #[instrument(skip_all)]
    async fn find_event_by_merchant_id_event_id(
        &self,
        state: &KeyManagerState,
        merchant_id: &common_utils::id_type::MerchantId,
        event_id: &str,
        merchant_key_store: &merchant_key_store::MerchantKeyStore,
    ) -> CustomResult<domain::Event, errors::StorageError> {
        let conn = connection::pg_connection_read(self).await?;
        storage::Event::find_by_merchant_id_event_id(&conn, merchant_id, event_id)
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))?
            .convert(
                state,
                merchant_key_store.key.get_inner(),
                merchant_key_store.merchant_id.clone().into(),
            )
            .await
            .change_context(errors::StorageError::DecryptionError)
    }

    #[instrument(skip_all)]
    async fn list_initial_events_by_merchant_id_primary_object_id(
        &self,
        state: &KeyManagerState,
        merchant_id: &common_utils::id_type::MerchantId,
        primary_object_id: &str,
        merchant_key_store: &merchant_key_store::MerchantKeyStore,
    ) -> CustomResult<Vec<domain::Event>, errors::StorageError> {
        let conn = connection::pg_connection_read(self).await?;
        storage::Event::list_initial_attempts_by_merchant_id_primary_object_id(
            &conn,
            merchant_id,
            primary_object_id,
        )
        .await
        .map_err(|error| report!(errors::StorageError::from(error)))
        .async_and_then(|events| async {
            let mut domain_events = Vec::with_capacity(events.len());
            for event in events.into_iter() {
                domain_events.push(
                    event
                        .convert(
                            state,
                            merchant_key_store.key.get_inner(),
                            merchant_key_store.merchant_id.clone().into(),
                        )
                        .await
                        .change_context(errors::StorageError::DecryptionError)?,
                );
            }
            Ok(domain_events)
        })
        .await
    }

    #[instrument(skip_all)]
    async fn list_initial_events_by_merchant_id_constraints(
        &self,
        state: &KeyManagerState,
        merchant_id: &common_utils::id_type::MerchantId,
        created_after: Option<time::PrimitiveDateTime>,
        created_before: Option<time::PrimitiveDateTime>,
        limit: Option<i64>,
        offset: Option<i64>,
        merchant_key_store: &merchant_key_store::MerchantKeyStore,
    ) -> CustomResult<Vec<domain::Event>, errors::StorageError> {
        let conn = connection::pg_connection_read(self).await?;
        storage::Event::list_initial_attempts_by_merchant_id_constraints(
            &conn,
            merchant_id,
            created_after,
            created_before,
            limit,
            offset,
        )
        .await
        .map_err(|error| report!(errors::StorageError::from(error)))
        .async_and_then(|events| async {
            let mut domain_events = Vec::with_capacity(events.len());
            for event in events.into_iter() {
                domain_events.push(
                    event
                        .convert(
                            state,
                            merchant_key_store.key.get_inner(),
                            merchant_key_store.merchant_id.clone().into(),
                        )
                        .await
                        .change_context(errors::StorageError::DecryptionError)?,
                );
            }
            Ok(domain_events)
        })
        .await
    }

    #[instrument(skip_all)]
    async fn list_events_by_merchant_id_initial_attempt_id(
        &self,
        state: &KeyManagerState,
        merchant_id: &common_utils::id_type::MerchantId,
        initial_attempt_id: &str,
        merchant_key_store: &merchant_key_store::MerchantKeyStore,
    ) -> CustomResult<Vec<domain::Event>, errors::StorageError> {
        let conn = connection::pg_connection_read(self).await?;
        storage::Event::list_by_merchant_id_initial_attempt_id(
            &conn,
            merchant_id,
            initial_attempt_id,
        )
        .await
        .map_err(|error| report!(errors::StorageError::from(error)))
        .async_and_then(|events| async {
            let mut domain_events = Vec::with_capacity(events.len());
            for event in events.into_iter() {
                domain_events.push(
                    event
                        .convert(
                            state,
                            merchant_key_store.key.get_inner(),
                            merchant_key_store.merchant_id.clone().into(),
                        )
                        .await
                        .change_context(errors::StorageError::DecryptionError)?,
                );
            }
            Ok(domain_events)
        })
        .await
    }

    #[instrument(skip_all)]
    async fn list_initial_events_by_profile_id_primary_object_id(
        &self,
        state: &KeyManagerState,
        profile_id: &common_utils::id_type::ProfileId,
        primary_object_id: &str,
        merchant_key_store: &merchant_key_store::MerchantKeyStore,
    ) -> CustomResult<Vec<domain::Event>, errors::StorageError> {
        let conn = connection::pg_connection_read(self).await?;
        storage::Event::list_initial_attempts_by_profile_id_primary_object_id(
            &conn,
            profile_id,
            primary_object_id,
        )
        .await
        .map_err(|error| report!(errors::StorageError::from(error)))
        .async_and_then(|events| async {
            let mut domain_events = Vec::with_capacity(events.len());
            for event in events.into_iter() {
                domain_events.push(
                    event
                        .convert(
                            state,
                            merchant_key_store.key.get_inner(),
                            merchant_key_store.merchant_id.clone().into(),
                        )
                        .await
                        .change_context(errors::StorageError::DecryptionError)?,
                );
            }
            Ok(domain_events)
        })
        .await
    }

    #[instrument(skip_all)]
    async fn list_initial_events_by_profile_id_constraints(
        &self,
        state: &KeyManagerState,
        profile_id: &common_utils::id_type::ProfileId,
        created_after: Option<time::PrimitiveDateTime>,
        created_before: Option<time::PrimitiveDateTime>,
        limit: Option<i64>,
        offset: Option<i64>,
        merchant_key_store: &merchant_key_store::MerchantKeyStore,
    ) -> CustomResult<Vec<domain::Event>, errors::StorageError> {
        let conn = connection::pg_connection_read(self).await?;
        storage::Event::list_initial_attempts_by_profile_id_constraints(
            &conn,
            profile_id,
            created_after,
            created_before,
            limit,
            offset,
        )
        .await
        .map_err(|error| report!(errors::StorageError::from(error)))
        .async_and_then(|events| async {
            let mut domain_events = Vec::with_capacity(events.len());
            for event in events.into_iter() {
                domain_events.push(
                    event
                        .convert(
                            state,
                            merchant_key_store.key.get_inner(),
                            common_utils::types::keymanager::Identifier::Merchant(
                                merchant_key_store.merchant_id.clone(),
                            ),
                        )
                        .await
                        .change_context(errors::StorageError::DecryptionError)?,
                );
            }
            Ok(domain_events)
        })
        .await
    }

    #[instrument(skip_all)]
    async fn update_event_by_merchant_id_event_id(
        &self,
        state: &KeyManagerState,
        merchant_id: &common_utils::id_type::MerchantId,
        event_id: &str,
        event: domain::EventUpdate,
        merchant_key_store: &merchant_key_store::MerchantKeyStore,
    ) -> CustomResult<domain::Event, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        storage::Event::update_by_merchant_id_event_id(&conn, merchant_id, event_id, event.into())
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))?
            .convert(
                state,
                merchant_key_store.key.get_inner(),
                merchant_key_store.merchant_id.clone().into(),
            )
            .await
            .change_context(errors::StorageError::DecryptionError)
    }
}
