use std::collections::HashSet;

use common_utils::ext_traits::AsyncExt;
use error_stack::{report, ResultExt};
use router_env::{instrument, tracing};

use super::{MockDb, Store};
use crate::{
    connection,
    core::errors::{self, CustomResult},
    types::{
        domain::{
            self,
            behaviour::{Conversion, ReverseConversion},
        },
        storage,
    },
};

#[async_trait::async_trait]
pub trait EventInterface
where
    domain::Event:
        Conversion<DstType = storage::events::Event, NewDstType = storage::events::EventNew>,
{
    async fn insert_event(
        &self,
        event: domain::Event,
        merchant_key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<domain::Event, errors::StorageError>;

    async fn find_event_by_merchant_id_event_id(
        &self,
        merchant_id: &common_utils::id_type::MerchantId,
        event_id: &str,
        merchant_key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<domain::Event, errors::StorageError>;

    async fn find_event_by_merchant_id_idempotent_event_id(
        &self,
        merchant_id: &common_utils::id_type::MerchantId,
        idempotent_event_id: &str,
        merchant_key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<domain::Event, errors::StorageError>;

    async fn list_initial_events_by_merchant_id_primary_object_or_initial_attempt_id(
        &self,
        merchant_id: &common_utils::id_type::MerchantId,
        primary_object_id: &str,
        initial_attempt_id: &str,
        merchant_key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<Vec<domain::Event>, errors::StorageError>;

    #[allow(clippy::too_many_arguments)]
    async fn list_initial_events_by_merchant_id_constraints(
        &self,
        merchant_id: &common_utils::id_type::MerchantId,
        created_after: time::PrimitiveDateTime,
        created_before: time::PrimitiveDateTime,
        limit: Option<i64>,
        offset: Option<i64>,
        event_types: HashSet<common_enums::EventType>,
        is_delivered: Option<bool>,
        merchant_key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<Vec<domain::Event>, errors::StorageError>;

    async fn list_events_by_merchant_id_initial_attempt_id(
        &self,
        merchant_id: &common_utils::id_type::MerchantId,
        initial_attempt_id: &str,
        merchant_key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<Vec<domain::Event>, errors::StorageError>;

    async fn list_initial_events_by_profile_id_primary_object_or_initial_attempt_id(
        &self,
        profile_id: &common_utils::id_type::ProfileId,
        primary_object_id: &str,
        initial_attempt_id: &str,
        merchant_key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<Vec<domain::Event>, errors::StorageError>;

    #[allow(clippy::too_many_arguments)]
    async fn list_initial_events_by_profile_id_constraints(
        &self,
        profile_id: &common_utils::id_type::ProfileId,
        created_after: time::PrimitiveDateTime,
        created_before: time::PrimitiveDateTime,
        limit: Option<i64>,
        offset: Option<i64>,
        event_types: HashSet<common_enums::EventType>,
        is_delivered: Option<bool>,
        merchant_key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<Vec<domain::Event>, errors::StorageError>;

    async fn update_event_by_merchant_id_event_id(
        &self,
        merchant_id: &common_utils::id_type::MerchantId,
        event_id: &str,
        event: domain::EventUpdate,
        merchant_key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<domain::Event, errors::StorageError>;

    async fn count_initial_events_by_constraints(
        &self,
        merchant_id: &common_utils::id_type::MerchantId,
        profile_id: Option<common_utils::id_type::ProfileId>,
        created_after: time::PrimitiveDateTime,
        created_before: time::PrimitiveDateTime,
        event_types: HashSet<common_enums::EventType>,
        is_delivered: Option<bool>,
    ) -> CustomResult<i64, errors::StorageError>;
}

#[async_trait::async_trait]
impl EventInterface for Store {
    #[instrument(skip_all)]
    async fn insert_event(
        &self,
        event: domain::Event,
        merchant_key_store: &domain::MerchantKeyStore,
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
                self.get_keymanager_state()
                    .attach_printable("Missing KeyManagerState")?,
                merchant_key_store.key.get_inner(),
                merchant_key_store.merchant_id.clone().into(),
            )
            .await
            .change_context(errors::StorageError::DecryptionError)
    }

    #[instrument(skip_all)]
    async fn find_event_by_merchant_id_event_id(
        &self,
        merchant_id: &common_utils::id_type::MerchantId,
        event_id: &str,
        merchant_key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<domain::Event, errors::StorageError> {
        let conn = connection::pg_connection_read(self).await?;
        storage::Event::find_by_merchant_id_event_id(&conn, merchant_id, event_id)
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))?
            .convert(
                self.get_keymanager_state()
                    .attach_printable("Missing KeyManagerState")?,
                merchant_key_store.key.get_inner(),
                merchant_key_store.merchant_id.clone().into(),
            )
            .await
            .change_context(errors::StorageError::DecryptionError)
    }

    #[instrument(skip_all)]
    async fn find_event_by_merchant_id_idempotent_event_id(
        &self,
        merchant_id: &common_utils::id_type::MerchantId,
        idempotent_event_id: &str,
        merchant_key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<domain::Event, errors::StorageError> {
        let conn = connection::pg_connection_read(self).await?;
        storage::Event::find_by_merchant_id_idempotent_event_id(
            &conn,
            merchant_id,
            idempotent_event_id,
        )
        .await
        .map_err(|error| report!(errors::StorageError::from(error)))?
        .convert(
            self.get_keymanager_state()
                .attach_printable("Missing KeyManagerState")?,
            merchant_key_store.key.get_inner(),
            merchant_key_store.merchant_id.clone().into(),
        )
        .await
        .change_context(errors::StorageError::DecryptionError)
    }

    #[instrument(skip_all)]
    async fn list_initial_events_by_merchant_id_primary_object_or_initial_attempt_id(
        &self,
        merchant_id: &common_utils::id_type::MerchantId,
        primary_object_id: &str,
        initial_attempt_id: &str,
        merchant_key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<Vec<domain::Event>, errors::StorageError> {
        let conn = connection::pg_connection_read(self).await?;
        storage::Event::list_initial_attempts_by_merchant_id_primary_object_id_or_initial_attempt_id(
            &conn,
            merchant_id,
            primary_object_id,
            initial_attempt_id,
        )
        .await
        .map_err(|error| report!(errors::StorageError::from(error)))
        .async_and_then(|events| async {
            let mut domain_events = Vec::with_capacity(events.len());
            for event in events.into_iter() {
                domain_events.push(
                    event
                        .convert(self.get_keymanager_state().attach_printable("Missing KeyManagerState")?,
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
        merchant_id: &common_utils::id_type::MerchantId,
        created_after: time::PrimitiveDateTime,
        created_before: time::PrimitiveDateTime,
        limit: Option<i64>,
        offset: Option<i64>,
        event_types: HashSet<common_enums::EventType>,
        is_delivered: Option<bool>,
        merchant_key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<Vec<domain::Event>, errors::StorageError> {
        let conn = connection::pg_connection_read(self).await?;
        storage::Event::list_initial_attempts_by_merchant_id_constraints(
            &conn,
            merchant_id,
            created_after,
            created_before,
            limit,
            offset,
            event_types,
            is_delivered,
        )
        .await
        .map_err(|error| report!(errors::StorageError::from(error)))
        .async_and_then(|events| async {
            let mut domain_events = Vec::with_capacity(events.len());
            for event in events.into_iter() {
                domain_events.push(
                    event
                        .convert(
                            self.get_keymanager_state()
                                .attach_printable("Missing KeyManagerState")?,
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
        merchant_id: &common_utils::id_type::MerchantId,
        initial_attempt_id: &str,
        merchant_key_store: &domain::MerchantKeyStore,
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
                            self.get_keymanager_state()
                                .attach_printable("Missing KeyManagerState")?,
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
    async fn list_initial_events_by_profile_id_primary_object_or_initial_attempt_id(
        &self,
        profile_id: &common_utils::id_type::ProfileId,
        primary_object_id: &str,
        initial_attempt_id: &str,
        merchant_key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<Vec<domain::Event>, errors::StorageError> {
        let conn = connection::pg_connection_read(self).await?;
        storage::Event::list_initial_attempts_by_profile_id_primary_object_id_or_initial_attempt_id(
            &conn,
            profile_id,
            primary_object_id,
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
                            self.get_keymanager_state()
                                .attach_printable("Missing KeyManagerState")?,
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
        profile_id: &common_utils::id_type::ProfileId,
        created_after: time::PrimitiveDateTime,
        created_before: time::PrimitiveDateTime,
        limit: Option<i64>,
        offset: Option<i64>,
        event_types: HashSet<common_enums::EventType>,
        is_delivered: Option<bool>,
        merchant_key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<Vec<domain::Event>, errors::StorageError> {
        let conn = connection::pg_connection_read(self).await?;
        storage::Event::list_initial_attempts_by_profile_id_constraints(
            &conn,
            profile_id,
            created_after,
            created_before,
            limit,
            offset,
            event_types,
            is_delivered,
        )
        .await
        .map_err(|error| report!(errors::StorageError::from(error)))
        .async_and_then(|events| async {
            let mut domain_events = Vec::with_capacity(events.len());
            for event in events.into_iter() {
                domain_events.push(
                    event
                        .convert(
                            self.get_keymanager_state()
                                .attach_printable("Missing KeyManagerState")?,
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
        merchant_id: &common_utils::id_type::MerchantId,
        event_id: &str,
        event: domain::EventUpdate,
        merchant_key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<domain::Event, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        storage::Event::update_by_merchant_id_event_id(&conn, merchant_id, event_id, event.into())
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))?
            .convert(
                self.get_keymanager_state()
                    .attach_printable("Missing KeyManagerState")?,
                merchant_key_store.key.get_inner(),
                merchant_key_store.merchant_id.clone().into(),
            )
            .await
            .change_context(errors::StorageError::DecryptionError)
    }

    async fn count_initial_events_by_constraints(
        &self,
        merchant_id: &common_utils::id_type::MerchantId,
        profile_id: Option<common_utils::id_type::ProfileId>,
        created_after: time::PrimitiveDateTime,
        created_before: time::PrimitiveDateTime,
        event_types: HashSet<common_enums::EventType>,
        is_delivered: Option<bool>,
    ) -> CustomResult<i64, errors::StorageError> {
        let conn = connection::pg_connection_read(self).await?;
        storage::Event::count_initial_attempts_by_constraints(
            &conn,
            merchant_id,
            profile_id,
            created_after,
            created_before,
            event_types,
            is_delivered,
        )
        .await
        .map_err(|error| report!(errors::StorageError::from(error)))
    }
}

#[async_trait::async_trait]
impl EventInterface for MockDb {
    async fn insert_event(
        &self,
        event: domain::Event,
        merchant_key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<domain::Event, errors::StorageError> {
        let mut locked_events = self.events.lock().await;

        let stored_event = Conversion::convert(event)
            .await
            .change_context(errors::StorageError::EncryptionError)?;

        locked_events.push(stored_event.clone());

        stored_event
            .convert(
                self.get_keymanager_state()
                    .attach_printable("Missing KeyManagerState")?,
                merchant_key_store.key.get_inner(),
                merchant_key_store.merchant_id.clone().into(),
            )
            .await
            .change_context(errors::StorageError::DecryptionError)
    }

    async fn find_event_by_merchant_id_event_id(
        &self,
        merchant_id: &common_utils::id_type::MerchantId,
        event_id: &str,
        merchant_key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<domain::Event, errors::StorageError> {
        let locked_events = self.events.lock().await;
        locked_events
            .iter()
            .find(|event| {
                event.merchant_id == Some(merchant_id.to_owned()) && event.event_id == event_id
            })
            .cloned()
            .async_map(|event| async {
                event
                    .convert(self.get_keymanager_state().attach_printable("Missing KeyManagerState")?,
                        merchant_key_store.key.get_inner(),
                        merchant_key_store.merchant_id.clone().into(),
                    )
                    .await
                    .change_context(errors::StorageError::DecryptionError)
            })
            .await
            .transpose()?
            .ok_or(
                errors::StorageError::ValueNotFound(format!(
                    "No event available with merchant_id = {merchant_id:?} and event_id  = {event_id}"
                ))
                .into(),
            )
    }

    async fn find_event_by_merchant_id_idempotent_event_id(
        &self,
        merchant_id: &common_utils::id_type::MerchantId,
        idempotent_event_id: &str,
        merchant_key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<domain::Event, errors::StorageError> {
        let locked_events = self.events.lock().await;
        locked_events
            .iter()
            .find(|event| {
                event.merchant_id == Some(merchant_id.to_owned()) && event.idempotent_event_id == Some(idempotent_event_id.to_string())
            })
            .cloned()
            .async_map(|event| async {
                event
                    .convert(self.get_keymanager_state().attach_printable("Missing KeyManagerState")?,
                        merchant_key_store.key.get_inner(),
                        merchant_key_store.merchant_id.clone().into(),
                    )
                    .await
                    .change_context(errors::StorageError::DecryptionError)
            })
            .await
            .transpose()?
            .ok_or(
                errors::StorageError::ValueNotFound(format!(
                    "No event available with merchant_id = {merchant_id:?} and idempotent_event_id  = {idempotent_event_id}"
                ))
                .into(),
            )
    }

    async fn list_initial_events_by_merchant_id_primary_object_or_initial_attempt_id(
        &self,
        merchant_id: &common_utils::id_type::MerchantId,
        primary_object_id: &str,
        initial_attempt_id: &str,
        merchant_key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<Vec<domain::Event>, errors::StorageError> {
        let locked_events = self.events.lock().await;
        let events = locked_events
            .iter()
            .filter(|event| {
                event.merchant_id == Some(merchant_id.to_owned())
                    && event.initial_attempt_id.as_deref() == Some(&event.event_id)
                    && (event.primary_object_id == primary_object_id
                        || event.initial_attempt_id.as_deref() == Some(initial_attempt_id))
            })
            .cloned()
            .collect::<Vec<_>>();

        let mut domain_events = Vec::with_capacity(events.len());

        for event in events {
            let domain_event = event
                .convert(
                    self.get_keymanager_state()
                        .attach_printable("Missing KeyManagerState")?,
                    merchant_key_store.key.get_inner(),
                    merchant_key_store.merchant_id.clone().into(),
                )
                .await
                .change_context(errors::StorageError::DecryptionError)?;
            domain_events.push(domain_event);
        }

        Ok(domain_events)
    }

    async fn list_initial_events_by_merchant_id_constraints(
        &self,
        merchant_id: &common_utils::id_type::MerchantId,
        created_after: time::PrimitiveDateTime,
        created_before: time::PrimitiveDateTime,
        limit: Option<i64>,
        offset: Option<i64>,
        event_types: HashSet<common_enums::EventType>,
        is_delivered: Option<bool>,
        merchant_key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<Vec<domain::Event>, errors::StorageError> {
        let locked_events = self.events.lock().await;
        let events_iter = locked_events.iter().filter(|event| {
            let check = event.merchant_id == Some(merchant_id.to_owned())
                && event.initial_attempt_id.as_ref() == Some(&event.event_id)
                && (event.created_at >= created_after)
                && (event.created_at <= created_before)
                && (event_types.is_empty() || event_types.contains(&event.event_type))
                && (event.is_overall_delivery_successful == is_delivered);

            check
        });

        let offset: usize = if let Some(offset) = offset {
            if offset < 0 {
                Err(errors::StorageError::MockDbError)?;
            }
            offset
                .try_into()
                .map_err(|_| errors::StorageError::MockDbError)?
        } else {
            0
        };

        let limit: usize = if let Some(limit) = limit {
            if limit < 0 {
                Err(errors::StorageError::MockDbError)?;
            }
            limit
                .try_into()
                .map_err(|_| errors::StorageError::MockDbError)?
        } else {
            usize::MAX
        };

        let events = events_iter
            .skip(offset)
            .take(limit)
            .cloned()
            .collect::<Vec<_>>();
        let mut domain_events = Vec::with_capacity(events.len());

        for event in events {
            let domain_event = event
                .convert(
                    self.get_keymanager_state()
                        .attach_printable("Missing KeyManagerState")?,
                    merchant_key_store.key.get_inner(),
                    merchant_key_store.merchant_id.clone().into(),
                )
                .await
                .change_context(errors::StorageError::DecryptionError)?;
            domain_events.push(domain_event);
        }

        Ok(domain_events)
    }

    async fn list_events_by_merchant_id_initial_attempt_id(
        &self,
        merchant_id: &common_utils::id_type::MerchantId,
        initial_attempt_id: &str,
        merchant_key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<Vec<domain::Event>, errors::StorageError> {
        let locked_events = self.events.lock().await;
        let events = locked_events
            .iter()
            .filter(|event| {
                event.merchant_id == Some(merchant_id.to_owned())
                    && event.initial_attempt_id == Some(initial_attempt_id.to_owned())
            })
            .cloned()
            .collect::<Vec<_>>();
        let mut domain_events = Vec::with_capacity(events.len());

        for event in events {
            let domain_event = event
                .convert(
                    self.get_keymanager_state()
                        .attach_printable("Missing KeyManagerState")?,
                    merchant_key_store.key.get_inner(),
                    merchant_key_store.merchant_id.clone().into(),
                )
                .await
                .change_context(errors::StorageError::DecryptionError)?;
            domain_events.push(domain_event);
        }

        Ok(domain_events)
    }

    async fn list_initial_events_by_profile_id_primary_object_or_initial_attempt_id(
        &self,
        profile_id: &common_utils::id_type::ProfileId,
        primary_object_id: &str,
        initial_attempt_id: &str,
        merchant_key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<Vec<domain::Event>, errors::StorageError> {
        let locked_events = self.events.lock().await;
        let events = locked_events
            .iter()
            .filter(|event| {
                event.business_profile_id == Some(profile_id.to_owned())
                    && event.initial_attempt_id.as_ref() == Some(&event.event_id)
                    && (event.primary_object_id == primary_object_id
                        || event.initial_attempt_id.as_deref() == Some(initial_attempt_id))
            })
            .cloned()
            .collect::<Vec<_>>();

        let mut domain_events = Vec::with_capacity(events.len());

        for event in events {
            let domain_event = event
                .convert(
                    self.get_keymanager_state()
                        .attach_printable("Missing KeyManagerState")?,
                    merchant_key_store.key.get_inner(),
                    merchant_key_store.merchant_id.clone().into(),
                )
                .await
                .change_context(errors::StorageError::DecryptionError)?;
            domain_events.push(domain_event);
        }

        Ok(domain_events)
    }

    async fn list_initial_events_by_profile_id_constraints(
        &self,
        profile_id: &common_utils::id_type::ProfileId,
        created_after: time::PrimitiveDateTime,
        created_before: time::PrimitiveDateTime,
        limit: Option<i64>,
        offset: Option<i64>,
        event_types: HashSet<common_enums::EventType>,
        is_delivered: Option<bool>,
        merchant_key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<Vec<domain::Event>, errors::StorageError> {
        let locked_events = self.events.lock().await;
        let events_iter = locked_events.iter().filter(|event| {
            let check = event.business_profile_id == Some(profile_id.to_owned())
                && event.initial_attempt_id.as_ref() == Some(&event.event_id)
                && (event.created_at >= created_after)
                && (event.created_at <= created_before)
                && (event_types.is_empty() || event_types.contains(&event.event_type))
                && (event.is_overall_delivery_successful == is_delivered);

            check
        });

        let offset: usize = if let Some(offset) = offset {
            if offset < 0 {
                Err(errors::StorageError::MockDbError)?;
            }
            offset
                .try_into()
                .map_err(|_| errors::StorageError::MockDbError)?
        } else {
            0
        };

        let limit: usize = if let Some(limit) = limit {
            if limit < 0 {
                Err(errors::StorageError::MockDbError)?;
            }
            limit
                .try_into()
                .map_err(|_| errors::StorageError::MockDbError)?
        } else {
            usize::MAX
        };

        let events = events_iter
            .skip(offset)
            .take(limit)
            .cloned()
            .collect::<Vec<_>>();
        let mut domain_events = Vec::with_capacity(events.len());

        for event in events {
            let domain_event = event
                .convert(
                    self.get_keymanager_state()
                        .attach_printable("Missing KeyManagerState")?,
                    merchant_key_store.key.get_inner(),
                    merchant_key_store.merchant_id.clone().into(),
                )
                .await
                .change_context(errors::StorageError::DecryptionError)?;
            domain_events.push(domain_event);
        }

        Ok(domain_events)
    }

    async fn update_event_by_merchant_id_event_id(
        &self,
        merchant_id: &common_utils::id_type::MerchantId,
        event_id: &str,
        event: domain::EventUpdate,
        merchant_key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<domain::Event, errors::StorageError> {
        let mut locked_events = self.events.lock().await;
        let event_to_update = locked_events
            .iter_mut()
            .find(|event| {
                event.merchant_id == Some(merchant_id.to_owned()) && event.event_id == event_id
            })
            .ok_or(errors::StorageError::MockDbError)?;

        match event {
            domain::EventUpdate::UpdateResponse {
                is_webhook_notified,
                response,
            } => {
                event_to_update.is_webhook_notified = is_webhook_notified;
                event_to_update.response = response.map(Into::into);
            }
            domain::EventUpdate::OverallDeliveryStatusUpdate {
                is_overall_delivery_successful,
            } => {
                event_to_update.is_overall_delivery_successful =
                    Some(is_overall_delivery_successful)
            }
        }

        event_to_update
            .clone()
            .convert(
                self.get_keymanager_state()
                    .attach_printable("Missing KeyManagerState")?,
                merchant_key_store.key.get_inner(),
                merchant_key_store.merchant_id.clone().into(),
            )
            .await
            .change_context(errors::StorageError::DecryptionError)
    }

    async fn count_initial_events_by_constraints(
        &self,
        merchant_id: &common_utils::id_type::MerchantId,
        profile_id: Option<common_utils::id_type::ProfileId>,
        created_after: time::PrimitiveDateTime,
        created_before: time::PrimitiveDateTime,
        event_types: HashSet<common_enums::EventType>,
        is_delivered: Option<bool>,
    ) -> CustomResult<i64, errors::StorageError> {
        let locked_events = self.events.lock().await;

        let iter_events = locked_events.iter().filter(|event| {
            let check = event.initial_attempt_id.as_ref() == Some(&event.event_id)
                && (event.merchant_id == Some(merchant_id.to_owned()))
                && (event.business_profile_id == profile_id)
                && (event.created_at >= created_after)
                && (event.created_at <= created_before)
                && (event_types.is_empty() || event_types.contains(&event.event_type))
                && (event.is_overall_delivery_successful == is_delivered);

            check
        });

        let events = iter_events.cloned().collect::<Vec<_>>();

        i64::try_from(events.len())
            .change_context(errors::StorageError::MockDbError)
            .attach_printable("Failed to convert usize to i64")
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use api_models::webhooks as api_webhooks;
    use common_enums::IntentStatus;
    use common_utils::{
        generate_organization_id_of_default_length, type_name,
        types::{
            keymanager::{Identifier, KeyManagerState},
            MinorUnit,
        },
    };
    use diesel_models::{
        business_profile::WebhookDetails,
        enums::{self},
        events::EventMetadata,
    };
    use futures::future::join_all;
    use hyperswitch_domain_models::{
        master_key::MasterKeyInterface, merchant_account::MerchantAccountSetter,
    };
    use time::macros::datetime;
    use tokio::time::{timeout, Duration};

    use crate::{
        core::webhooks as webhooks_core,
        db::{events::EventInterface, merchant_key_store::MerchantKeyStoreInterface, MockDb},
        routes::{
            self,
            app::{settings::Settings, StorageImpl},
        },
        services,
        types::{
            api,
            domain::{self, MerchantAccount},
        },
    };

    #[tokio::test]
    #[cfg(feature = "v1")]
    #[allow(clippy::panic_in_result_fn)]
    async fn test_mockdb_event_interface() -> Result<(), Box<dyn std::error::Error>> {
        let mockdb = MockDb::new(
            &redis_interface::RedisSettings::default(),
            KeyManagerState::new(),
        )
        .await
        .expect("Failed to create Mock store");
        let event_id = "test_event_id";
        let (tx, _) = tokio::sync::oneshot::channel();
        let app_state = Box::pin(routes::AppState::with_storage(
            Settings::new()?,
            StorageImpl::PostgresqlTest,
            tx,
            Box::new(services::MockApiClient),
        ))
        .await;
        let app_state_arc = Arc::new(app_state);
        let state = app_state_arc
            .get_session_state(
                &common_utils::id_type::TenantId::try_from_string("public".to_string()).unwrap(),
                None,
                || {},
            )
            .unwrap();
        let merchant_id =
            common_utils::id_type::MerchantId::try_from(std::borrow::Cow::from("merchant_1"))
                .unwrap();
        let business_profile_id =
            common_utils::id_type::ProfileId::try_from(std::borrow::Cow::from("profile1")).unwrap();
        let payment_id = "test_payment_id";
        let key_manager_state = &(&state).into();
        let master_key = mockdb.get_master_key();
        mockdb
            .insert_merchant_key_store(
                domain::MerchantKeyStore {
                    merchant_id: merchant_id.clone(),
                    key: domain::types::crypto_operation(
                        key_manager_state,
                        type_name!(domain::MerchantKeyStore),
                        domain::types::CryptoOperation::Encrypt(
                            services::generate_aes256_key().unwrap().to_vec().into(),
                        ),
                        Identifier::Merchant(merchant_id.to_owned()),
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
        let merchant_key_store = mockdb
            .get_merchant_key_store_by_merchant_id(&merchant_id, &master_key.to_vec().into())
            .await
            .unwrap();

        let event1 = mockdb
            .insert_event(
                domain::Event {
                    event_id: event_id.into(),
                    event_type: enums::EventType::PaymentSucceeded,
                    event_class: enums::EventClass::Payments,
                    is_webhook_notified: false,
                    primary_object_id: payment_id.into(),
                    primary_object_type: enums::EventObjectType::PaymentDetails,
                    created_at: common_utils::date_time::now(),
                    merchant_id: Some(merchant_id.to_owned()),
                    business_profile_id: Some(business_profile_id.to_owned()),
                    primary_object_created_at: Some(common_utils::date_time::now()),
                    idempotent_event_id: Some(event_id.into()),
                    initial_attempt_id: Some(event_id.into()),
                    request: None,
                    response: None,
                    delivery_attempt: Some(enums::WebhookDeliveryAttempt::InitialAttempt),
                    metadata: Some(EventMetadata::Payment {
                        payment_id: common_utils::id_type::PaymentId::try_from(
                            std::borrow::Cow::Borrowed(payment_id),
                        )
                        .unwrap(),
                    }),
                    is_overall_delivery_successful: Some(false),
                },
                &merchant_key_store,
            )
            .await
            .unwrap();

        assert_eq!(event1.event_id, event_id);

        let updated_event = mockdb
            .update_event_by_merchant_id_event_id(
                &merchant_id,
                event_id,
                domain::EventUpdate::UpdateResponse {
                    is_webhook_notified: true,
                    response: None,
                },
                &merchant_key_store,
            )
            .await
            .unwrap();

        assert!(updated_event.is_webhook_notified);
        assert_eq!(updated_event.primary_object_id, payment_id);
        assert_eq!(updated_event.event_id, event_id);
        Ok(())
    }

    #[tokio::test]
    #[cfg(feature = "v2")]
    #[allow(clippy::panic_in_result_fn)]
    async fn test_mockdb_event_interface() -> Result<(), Box<dyn std::error::Error>> {
        let mockdb = MockDb::new(
            &redis_interface::RedisSettings::default(),
            KeyManagerState::new(),
        )
        .await
        .expect("Failed to create Mock store");
        let event_id = "test_event_id";
        let (tx, _) = tokio::sync::oneshot::channel();
        let app_state = Box::pin(routes::AppState::with_storage(
            Settings::new()?,
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
        let merchant_id =
            common_utils::id_type::MerchantId::try_from(std::borrow::Cow::from("merchant_1"))
                .unwrap();
        let business_profile_id =
            common_utils::id_type::ProfileId::try_from(std::borrow::Cow::from("profile1")).unwrap();
        let payment_id = "test_payment_id";
        let key_manager_state = &state.into();
        let master_key = mockdb.get_master_key();
        mockdb
            .insert_merchant_key_store(
                domain::MerchantKeyStore {
                    merchant_id: merchant_id.clone(),
                    key: domain::types::crypto_operation(
                        key_manager_state,
                        type_name!(domain::MerchantKeyStore),
                        domain::types::CryptoOperation::Encrypt(
                            services::generate_aes256_key().unwrap().to_vec().into(),
                        ),
                        Identifier::Merchant(merchant_id.to_owned()),
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
        let merchant_key_store = mockdb
            .get_merchant_key_store_by_merchant_id(&merchant_id, &master_key.to_vec().into())
            .await
            .unwrap();

        let event1 = mockdb
            .insert_event(
                domain::Event {
                    event_id: event_id.into(),
                    event_type: enums::EventType::PaymentSucceeded,
                    event_class: enums::EventClass::Payments,
                    is_webhook_notified: false,
                    primary_object_id: payment_id.into(),
                    primary_object_type: enums::EventObjectType::PaymentDetails,
                    created_at: common_utils::date_time::now(),
                    merchant_id: Some(merchant_id.to_owned()),
                    business_profile_id: Some(business_profile_id.to_owned()),
                    primary_object_created_at: Some(common_utils::date_time::now()),
                    idempotent_event_id: Some(event_id.into()),
                    initial_attempt_id: Some(event_id.into()),
                    request: None,
                    response: None,
                    delivery_attempt: Some(enums::WebhookDeliveryAttempt::InitialAttempt),
                    metadata: Some(EventMetadata::Payment {
                        payment_id: common_utils::id_type::GlobalPaymentId::try_from(
                            std::borrow::Cow::Borrowed(payment_id),
                        )
                        .unwrap(),
                    }),
                    is_overall_delivery_successful: Some(false),
                },
                &merchant_key_store,
            )
            .await
            .unwrap();

        assert_eq!(event1.event_id, event_id);

        let updated_event = mockdb
            .update_event_by_merchant_id_event_id(
                &merchant_id,
                event_id,
                domain::EventUpdate::UpdateResponse {
                    is_webhook_notified: true,
                    response: None,
                },
                &merchant_key_store,
            )
            .await
            .unwrap();

        assert!(updated_event.is_webhook_notified);
        assert_eq!(updated_event.primary_object_id, payment_id);
        assert_eq!(updated_event.event_id, event_id);
        Ok(())
    }

    #[cfg(feature = "v1")]
    #[allow(clippy::panic_in_result_fn)]
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_concurrent_webhook_insertion_with_redis_lock(
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Test concurrent webhook insertion with a Redis lock to prevent race conditions
        let conf = Settings::new()?;
        let tx: tokio::sync::oneshot::Sender<()> = tokio::sync::oneshot::channel().0;

        let app_state = Box::pin(routes::AppState::with_storage(
            conf,
            StorageImpl::PostgresqlTest,
            tx,
            Box::new(services::MockApiClient),
        ))
        .await;

        let tenant_id = common_utils::id_type::TenantId::try_from_string("public".to_string())?;

        let state = Arc::new(app_state)
            .get_session_state(&tenant_id, None, || {})
            .map_err(|_| "failed to get session state")?;

        let merchant_id =
            common_utils::id_type::MerchantId::try_from(std::borrow::Cow::from("juspay_merchant"))?;
        let business_profile_id =
            common_utils::id_type::ProfileId::try_from(std::borrow::Cow::from("profile1"))?;
        let key_manager_state = &(&state).into();
        let master_key = state.store.get_master_key();

        let aes_key = services::generate_aes256_key()?;

        let merchant_key_store = state
            .store
            .insert_merchant_key_store(
                domain::MerchantKeyStore {
                    merchant_id: merchant_id.clone(),
                    key: domain::types::crypto_operation(
                        key_manager_state,
                        type_name!(domain::MerchantKeyStore),
                        domain::types::CryptoOperation::Encrypt(aes_key.to_vec().into()),
                        Identifier::Merchant(merchant_id.to_owned()),
                        master_key,
                    )
                    .await?
                    .try_into_operation()?,
                    created_at: datetime!(2023-02-01 0:00),
                },
                &master_key.to_vec().into(),
            )
            .await?;

        let merchant_account_to_insert = MerchantAccount::from(MerchantAccountSetter {
            merchant_id: merchant_id.clone(),
            merchant_name: None,
            merchant_details: None,
            return_url: None,
            webhook_details: Some(WebhookDetails {
                webhook_version: None,
                webhook_username: None,
                webhook_password: None,
                webhook_url: Some(masking::Secret::new(
                    "https://example.com/webhooks".to_string(),
                )),
                payment_created_enabled: None,
                payment_succeeded_enabled: Some(true),
                payment_failed_enabled: None,
                payment_statuses_enabled: None,
                refund_statuses_enabled: None,
                payout_statuses_enabled: None,
                multiple_webhooks_list: None,
            }),
            sub_merchants_enabled: None,
            parent_merchant_id: None,
            enable_payment_response_hash: true,
            payment_response_hash_key: None,
            redirect_to_merchant_with_http_post: false,
            publishable_key: "pk_test_11DviC2G2fb3lAJoes1q3A2222233327".to_string(),
            locker_id: None,
            storage_scheme: enums::MerchantStorageScheme::PostgresOnly,
            metadata: None,
            routing_algorithm: None,
            primary_business_details: serde_json::json!({ "country": "US", "business": "default" }),
            intent_fulfillment_time: Some(1),
            created_at: common_utils::date_time::now(),
            modified_at: common_utils::date_time::now(),
            frm_routing_algorithm: None,
            payout_routing_algorithm: None,
            organization_id: generate_organization_id_of_default_length(),
            is_recon_enabled: true,
            default_profile: None,
            recon_status: enums::ReconStatus::NotRequested,
            payment_link_config: None,
            pm_collect_link_config: None,
            is_platform_account: false,
            merchant_account_type: common_enums::MerchantAccountType::Standard,
            product_type: None,
            version: common_enums::ApiVersion::V1,
        });
        let merchant_account = state
            .store
            .insert_merchant(merchant_account_to_insert, &merchant_key_store)
            .await?;

        let platform = domain::Platform::new(
            merchant_account.clone(),
            merchant_key_store.clone(),
            merchant_account.clone(),
            merchant_key_store.clone(),
        );
        let merchant_id = merchant_id.clone(); // Clone merchant_id to avoid move

        let business_profile_to_insert = domain::Profile::from(domain::ProfileSetter {
            merchant_country_code: None,
            profile_id: business_profile_id.clone(),
            merchant_id: merchant_id.clone(),
            profile_name: "test_concurrent_profile".to_string(),
            created_at: common_utils::date_time::now(),
            modified_at: common_utils::date_time::now(),
            return_url: None,
            enable_payment_response_hash: true,
            payment_response_hash_key: None,
            redirect_to_merchant_with_http_post: false,
            webhook_details: Some(WebhookDetails {
                webhook_version: None,
                webhook_username: None,
                webhook_password: None,
                webhook_url: Some(masking::Secret::new(
                    "https://example.com/webhooks".to_string(),
                )),
                payment_created_enabled: None,
                payment_succeeded_enabled: Some(true),
                payment_failed_enabled: None,
                payment_statuses_enabled: None,
                refund_statuses_enabled: None,
                payout_statuses_enabled: None,
                multiple_webhooks_list: None,
            }),
            metadata: None,
            routing_algorithm: None,
            intent_fulfillment_time: None,
            frm_routing_algorithm: None,
            payout_routing_algorithm: None,
            is_recon_enabled: false,
            applepay_verified_domains: None,
            payment_link_config: None,
            session_expiry: None,
            authentication_connector_details: None,
            payout_link_config: None,
            is_extended_card_info_enabled: None,
            extended_card_info_config: None,
            is_connector_agnostic_mit_enabled: None,
            use_billing_as_payment_method_billing: None,
            collect_shipping_details_from_wallet_connector: None,
            collect_billing_details_from_wallet_connector: None,
            outgoing_webhook_custom_http_headers: None,
            always_collect_billing_details_from_wallet_connector: None,
            always_collect_shipping_details_from_wallet_connector: None,
            tax_connector_id: None,
            is_tax_connector_enabled: false,
            dynamic_routing_algorithm: None,
            is_network_tokenization_enabled: false,
            is_auto_retries_enabled: false,
            max_auto_retries_enabled: None,
            always_request_extended_authorization: None,
            is_click_to_pay_enabled: false,
            authentication_product_ids: None,
            card_testing_guard_config: None,
            card_testing_secret_key: None,
            is_clear_pan_retries_enabled: false,
            force_3ds_challenge: false,
            is_debit_routing_enabled: false,
            merchant_business_country: None,
            is_iframe_redirection_enabled: None,
            is_pre_network_tokenization_enabled: false,
            merchant_category_code: None,
            dispute_polling_interval: None,
            is_manual_retry_enabled: None,
            always_enable_overcapture: None,
            external_vault_details: domain::ExternalVaultDetails::Skip,
            billing_processor_id: None,
            is_l2_l3_enabled: false,
        });

        let business_profile = state
            .store
            .insert_business_profile(&merchant_key_store.clone(), business_profile_to_insert)
            .await?;

        // Same inputs for all threads
        let event_type = enums::EventType::PaymentSucceeded;
        let event_class = enums::EventClass::Payments;
        let primary_object_id = Arc::new("concurrent_payment_id".to_string());
        let initial_attempt_id = Arc::new("initial_attempt_id".to_string());
        let primary_object_type = enums::EventObjectType::PaymentDetails;
        let payment_id = common_utils::id_type::PaymentId::try_from(std::borrow::Cow::Borrowed(
            "pay_mbabizu24mvu3mela5njyhpit10",
        ))?;

        let primary_object_created_at = Some(common_utils::date_time::now());
        let expected_response = api::PaymentsResponse {
            payment_id,
            status: IntentStatus::Succeeded,
            amount: MinorUnit::new(6540),
            amount_capturable: MinorUnit::new(0),
            amount_received: None,
            client_secret: None,
            created: None,
            currency: "USD".to_string(),
            customer_id: None,
            description: Some("Its my first payment request".to_string()),
            refunds: None,
            mandate_id: None,
            merchant_id,
            net_amount: MinorUnit::new(6540),
            connector: None,
            customer: None,
            disputes: None,
            attempts: None,
            captures: None,
            mandate_data: None,
            setup_future_usage: None,
            off_session: None,
            capture_on: None,
            capture_method: None,
            payment_method: None,
            payment_method_data: None,
            payment_token: None,
            shipping: None,
            billing: None,
            order_details: None,
            email: None,
            name: None,
            phone: None,
            return_url: None,
            authentication_type: None,
            statement_descriptor_name: None,
            statement_descriptor_suffix: None,
            next_action: None,
            cancellation_reason: None,
            error_code: None,
            error_message: None,
            error_reason: None,
            unified_code: None,
            unified_message: None,
            payment_experience: None,
            payment_method_type: None,
            connector_label: None,
            business_country: None,
            business_label: None,
            business_sub_label: None,
            allowed_payment_method_types: None,
            ephemeral_key: None,
            manual_retry_allowed: None,
            connector_transaction_id: None,
            frm_message: None,
            metadata: None,
            connector_metadata: None,
            feature_metadata: None,
            reference_id: None,
            payment_link: None,
            profile_id: None,
            surcharge_details: None,
            attempt_count: 1,
            merchant_decision: None,
            merchant_connector_id: None,
            incremental_authorization_allowed: None,
            authorization_count: None,
            incremental_authorizations: None,
            external_authentication_details: None,
            external_3ds_authentication_attempted: None,
            expires_on: None,
            fingerprint: None,
            browser_info: None,
            payment_method_id: None,
            payment_method_status: None,
            updated: None,
            split_payments: None,
            frm_metadata: None,
            merchant_order_reference_id: None,
            capture_before: None,
            extended_authorization_applied: None,
            extended_authorization_last_applied_at: None,
            order_tax_amount: None,
            connector_mandate_id: None,
            shipping_cost: None,
            card_discovery: None,
            mit_category: None,
            tokenization: None,
            force_3ds_challenge: None,
            force_3ds_challenge_trigger: None,
            issuer_error_code: None,
            issuer_error_message: None,
            is_iframe_redirection_enabled: None,
            whole_connector_response: None,
            payment_channel: None,
            network_transaction_id: None,
            enable_partial_authorization: None,
            is_overcapture_enabled: None,
            enable_overcapture: None,
            network_details: None,
            is_stored_credential: None,
            request_extended_authorization: None,
            billing_descriptor: None,
            partner_merchant_identifier_details: None,
        };
        let content =
            api_webhooks::OutgoingWebhookContent::PaymentDetails(Box::new(expected_response));

        // Run 10 concurrent webhook creations
        let mut handles = vec![];
        for _ in 0..10 {
            let state_clone = state.clone();
            let platform_clone = platform.clone();
            let business_profile_clone = business_profile.clone();
            let content_clone = content.clone();
            let primary_object_id_clone = primary_object_id.clone();

            let handle = tokio::spawn(async move {
                webhooks_core::create_event_and_trigger_outgoing_webhook(
                    state_clone,
                    platform_clone,
                    business_profile_clone,
                    event_type,
                    event_class,
                    (*primary_object_id_clone).to_string(),
                    primary_object_type,
                    content_clone,
                    primary_object_created_at,
                )
                .await
                .map_err(|e| format!("create_event_and_trigger_outgoing_webhook failed: {e}"))
            });

            handles.push(handle);
        }

        // Await all tasks
        // We give the whole batch 20 s; if they don't finish something is wrong.
        let results = timeout(Duration::from_secs(20), join_all(handles))
            .await
            .map_err(|_| "tasks hung for >20 s – possible dead-lock / endless retry")?;

        for res in results {
            // Any task that panicked or returned Err will make the test fail here.
            let _ = res.map_err(|e| format!("task panicked: {e}"))?;
        }

        // Collect all initial-attempt events for this payment
        let events = state
            .store
            .list_initial_events_by_merchant_id_primary_object_or_initial_attempt_id(
                &business_profile.merchant_id,
                &primary_object_id.clone(),
                &initial_attempt_id.clone(),
                platform.get_processor().get_key_store(),
            )
            .await?;

        assert_eq!(
            events.len(),
            1,
            "Expected exactly 1 row in events table, found {}",
            events.len()
        );
        Ok(())
    }
}
