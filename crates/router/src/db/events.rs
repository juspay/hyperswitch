use common_utils::ext_traits::AsyncExt;
use error_stack::{IntoReport, ResultExt};
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
        merchant_id: &str,
        event_id: &str,
        merchant_key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<domain::Event, errors::StorageError>;

    async fn list_initial_events_by_merchant_id_primary_object_id(
        &self,
        merchant_id: &str,
        primary_object_id: &str,
        merchant_key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<Vec<domain::Event>, errors::StorageError>;

    async fn list_initial_events_by_merchant_id_constraints(
        &self,
        merchant_id: &str,
        created_after: Option<time::PrimitiveDateTime>,
        created_before: Option<time::PrimitiveDateTime>,
        limit: Option<i64>,
        offset: Option<i64>,
        merchant_key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<Vec<domain::Event>, errors::StorageError>;

    async fn list_events_by_merchant_id_initial_attempt_id(
        &self,
        merchant_id: &str,
        initial_attempt_id: &str,
        merchant_key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<Vec<domain::Event>, errors::StorageError>;

    async fn list_initial_events_by_profile_id_primary_object_id(
        &self,
        profile_id: &str,
        primary_object_id: &str,
        merchant_key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<Vec<domain::Event>, errors::StorageError>;

    async fn list_initial_events_by_profile_id_constraints(
        &self,
        profile_id: &str,
        created_after: Option<time::PrimitiveDateTime>,
        created_before: Option<time::PrimitiveDateTime>,
        limit: Option<i64>,
        offset: Option<i64>,
        merchant_key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<Vec<domain::Event>, errors::StorageError>;

    async fn list_events_by_profile_id_initial_attempt_id(
        &self,
        profile_id: &str,
        initial_attempt_id: &str,
        merchant_key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<Vec<domain::Event>, errors::StorageError>;

    async fn update_event_by_merchant_id_event_id(
        &self,
        merchant_id: &str,
        event_id: &str,
        event: domain::EventUpdate,
        merchant_key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<domain::Event, errors::StorageError>;
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
            .map_err(Into::into)
            .into_report()?
            .convert(merchant_key_store.key.get_inner())
            .await
            .change_context(errors::StorageError::DecryptionError)
    }

    #[instrument(skip_all)]
    async fn find_event_by_merchant_id_event_id(
        &self,
        merchant_id: &str,
        event_id: &str,
        merchant_key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<domain::Event, errors::StorageError> {
        let conn = connection::pg_connection_read(self).await?;
        storage::Event::find_by_merchant_id_event_id(&conn, merchant_id, event_id)
            .await
            .map_err(Into::into)
            .into_report()?
            .convert(merchant_key_store.key.get_inner())
            .await
            .change_context(errors::StorageError::DecryptionError)
    }

    #[instrument(skip_all)]
    async fn list_initial_events_by_merchant_id_primary_object_id(
        &self,
        merchant_id: &str,
        primary_object_id: &str,
        merchant_key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<Vec<domain::Event>, errors::StorageError> {
        let conn = connection::pg_connection_read(self).await?;
        storage::Event::list_initial_attempts_by_merchant_id_primary_object_id(
            &conn,
            merchant_id,
            primary_object_id,
        )
        .await
        .map_err(Into::into)
        .into_report()
        .async_and_then(|events| async {
            let mut domain_events = Vec::with_capacity(events.len());
            for event in events.into_iter() {
                domain_events.push(
                    event
                        .convert(merchant_key_store.key.get_inner())
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
        merchant_id: &str,
        created_after: Option<time::PrimitiveDateTime>,
        created_before: Option<time::PrimitiveDateTime>,
        limit: Option<i64>,
        offset: Option<i64>,
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
        )
        .await
        .map_err(Into::into)
        .into_report()
        .async_and_then(|events| async {
            let mut domain_events = Vec::with_capacity(events.len());
            for event in events.into_iter() {
                domain_events.push(
                    event
                        .convert(merchant_key_store.key.get_inner())
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
        merchant_id: &str,
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
        .map_err(Into::into)
        .into_report()
        .async_and_then(|events| async {
            let mut domain_events = Vec::with_capacity(events.len());
            for event in events.into_iter() {
                domain_events.push(
                    event
                        .convert(merchant_key_store.key.get_inner())
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
        profile_id: &str,
        primary_object_id: &str,
        merchant_key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<Vec<domain::Event>, errors::StorageError> {
        let conn = connection::pg_connection_read(self).await?;
        storage::Event::list_initial_attempts_by_profile_id_primary_object_id(
            &conn,
            profile_id,
            primary_object_id,
        )
        .await
        .map_err(Into::into)
        .into_report()
        .async_and_then(|events| async {
            let mut domain_events = Vec::with_capacity(events.len());
            for event in events.into_iter() {
                domain_events.push(
                    event
                        .convert(merchant_key_store.key.get_inner())
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
        profile_id: &str,
        created_after: Option<time::PrimitiveDateTime>,
        created_before: Option<time::PrimitiveDateTime>,
        limit: Option<i64>,
        offset: Option<i64>,
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
        )
        .await
        .map_err(Into::into)
        .into_report()
        .async_and_then(|events| async {
            let mut domain_events = Vec::with_capacity(events.len());
            for event in events.into_iter() {
                domain_events.push(
                    event
                        .convert(merchant_key_store.key.get_inner())
                        .await
                        .change_context(errors::StorageError::DecryptionError)?,
                );
            }
            Ok(domain_events)
        })
        .await
    }

    #[instrument(skip_all)]
    async fn list_events_by_profile_id_initial_attempt_id(
        &self,
        profile_id: &str,
        initial_attempt_id: &str,
        merchant_key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<Vec<domain::Event>, errors::StorageError> {
        let conn = connection::pg_connection_read(self).await?;
        storage::Event::list_by_profile_id_initial_attempt_id(&conn, profile_id, initial_attempt_id)
            .await
            .map_err(Into::into)
            .into_report()
            .async_and_then(|events| async {
                let mut domain_events = Vec::with_capacity(events.len());
                for event in events.into_iter() {
                    domain_events.push(
                        event
                            .convert(merchant_key_store.key.get_inner())
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
        merchant_id: &str,
        event_id: &str,
        event: domain::EventUpdate,
        merchant_key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<domain::Event, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        storage::Event::update_by_merchant_id_event_id(&conn, merchant_id, event_id, event.into())
            .await
            .map_err(Into::into)
            .into_report()?
            .convert(merchant_key_store.key.get_inner())
            .await
            .change_context(errors::StorageError::DecryptionError)
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
            .convert(merchant_key_store.key.get_inner())
            .await
            .change_context(errors::StorageError::DecryptionError)
    }

    async fn find_event_by_merchant_id_event_id(
        &self,
        merchant_id: &str,
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
                    .convert(merchant_key_store.key.get_inner())
                    .await
                    .change_context(errors::StorageError::DecryptionError)
            })
            .await
            .transpose()?
            .ok_or(
                errors::StorageError::ValueNotFound(format!(
                    "No event available with merchant_id = {merchant_id} and event_id  = {event_id}"
                ))
                .into(),
            )
    }

    async fn list_initial_events_by_merchant_id_primary_object_id(
        &self,
        merchant_id: &str,
        primary_object_id: &str,
        merchant_key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<Vec<domain::Event>, errors::StorageError> {
        let locked_events = self.events.lock().await;
        let events = locked_events
            .iter()
            .filter(|event| {
                event.merchant_id == Some(merchant_id.to_owned())
                    && event.initial_attempt_id.as_ref() == Some(&event.event_id)
                    && event.primary_object_id == primary_object_id
            })
            .cloned()
            .collect::<Vec<_>>();

        let mut domain_events = Vec::with_capacity(events.len());

        for event in events {
            let domain_event = event
                .convert(merchant_key_store.key.get_inner())
                .await
                .change_context(errors::StorageError::DecryptionError)?;
            domain_events.push(domain_event);
        }

        Ok(domain_events)
    }

    async fn list_initial_events_by_merchant_id_constraints(
        &self,
        merchant_id: &str,
        created_after: Option<time::PrimitiveDateTime>,
        created_before: Option<time::PrimitiveDateTime>,
        limit: Option<i64>,
        offset: Option<i64>,
        merchant_key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<Vec<domain::Event>, errors::StorageError> {
        let locked_events = self.events.lock().await;
        let events_iter = locked_events.iter().filter(|event| {
            let mut check = event.merchant_id == Some(merchant_id.to_owned())
                && event.initial_attempt_id.as_ref() == Some(&event.event_id);

            if let Some(created_after) = created_after {
                check = check && (event.created_at >= created_after);
            }

            if let Some(created_before) = created_before {
                check = check && (event.created_at <= created_before);
            }

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
                .convert(merchant_key_store.key.get_inner())
                .await
                .change_context(errors::StorageError::DecryptionError)?;
            domain_events.push(domain_event);
        }

        Ok(domain_events)
    }

    async fn list_events_by_merchant_id_initial_attempt_id(
        &self,
        merchant_id: &str,
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
                .convert(merchant_key_store.key.get_inner())
                .await
                .change_context(errors::StorageError::DecryptionError)?;
            domain_events.push(domain_event);
        }

        Ok(domain_events)
    }

    async fn list_initial_events_by_profile_id_primary_object_id(
        &self,
        profile_id: &str,
        primary_object_id: &str,
        merchant_key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<Vec<domain::Event>, errors::StorageError> {
        let locked_events = self.events.lock().await;
        let events = locked_events
            .iter()
            .filter(|event| {
                event.business_profile_id == Some(profile_id.to_owned())
                    && event.initial_attempt_id.as_ref() == Some(&event.event_id)
                    && event.primary_object_id == primary_object_id
            })
            .cloned()
            .collect::<Vec<_>>();

        let mut domain_events = Vec::with_capacity(events.len());

        for event in events {
            let domain_event = event
                .convert(merchant_key_store.key.get_inner())
                .await
                .change_context(errors::StorageError::DecryptionError)?;
            domain_events.push(domain_event);
        }

        Ok(domain_events)
    }

    async fn list_initial_events_by_profile_id_constraints(
        &self,
        profile_id: &str,
        created_after: Option<time::PrimitiveDateTime>,
        created_before: Option<time::PrimitiveDateTime>,
        limit: Option<i64>,
        offset: Option<i64>,
        merchant_key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<Vec<domain::Event>, errors::StorageError> {
        let locked_events = self.events.lock().await;
        let events_iter = locked_events.iter().filter(|event| {
            let mut check = event.business_profile_id == Some(profile_id.to_owned())
                && event.initial_attempt_id.as_ref() == Some(&event.event_id);

            if let Some(created_after) = created_after {
                check = check && (event.created_at >= created_after);
            }

            if let Some(created_before) = created_before {
                check = check && (event.created_at <= created_before);
            }

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
                .convert(merchant_key_store.key.get_inner())
                .await
                .change_context(errors::StorageError::DecryptionError)?;
            domain_events.push(domain_event);
        }

        Ok(domain_events)
    }

    async fn list_events_by_profile_id_initial_attempt_id(
        &self,
        profile_id: &str,
        initial_attempt_id: &str,
        merchant_key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<Vec<domain::Event>, errors::StorageError> {
        let locked_events = self.events.lock().await;
        let events = locked_events
            .iter()
            .filter(|event| {
                event.business_profile_id == Some(profile_id.to_owned())
                    && event.initial_attempt_id == Some(initial_attempt_id.to_owned())
            })
            .cloned()
            .collect::<Vec<_>>();
        let mut domain_events = Vec::with_capacity(events.len());

        for event in events {
            let domain_event = event
                .convert(merchant_key_store.key.get_inner())
                .await
                .change_context(errors::StorageError::DecryptionError)?;
            domain_events.push(domain_event);
        }

        Ok(domain_events)
    }

    async fn update_event_by_merchant_id_event_id(
        &self,
        merchant_id: &str,
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
        }

        event_to_update
            .clone()
            .convert(merchant_key_store.key.get_inner())
            .await
            .change_context(errors::StorageError::DecryptionError)
    }
}

#[cfg(test)]
mod tests {
    use diesel_models::enums;
    use time::macros::datetime;

    use crate::{
        db::{
            events::EventInterface, merchant_key_store::MerchantKeyStoreInterface,
            MasterKeyInterface, MockDb,
        },
        services,
        types::domain,
    };

    #[allow(clippy::unwrap_used)]
    #[tokio::test]
    async fn test_mockdb_event_interface() {
        #[allow(clippy::expect_used)]
        let mockdb = MockDb::new(&redis_interface::RedisSettings::default())
            .await
            .expect("Failed to create Mock store");
        let event_id = "test_event_id";
        let merchant_id = "merchant1";
        let business_profile_id = "profile1";

        let master_key = mockdb.get_master_key();
        mockdb
            .insert_merchant_key_store(
                domain::MerchantKeyStore {
                    merchant_id: merchant_id.into(),
                    key: domain::types::encrypt(
                        services::generate_aes256_key().unwrap().to_vec().into(),
                        master_key,
                    )
                    .await
                    .unwrap(),
                    created_at: datetime!(2023-02-01 0:00),
                },
                &master_key.to_vec().into(),
            )
            .await
            .unwrap();
        let merchant_key_store = mockdb
            .get_merchant_key_store_by_merchant_id(merchant_id, &master_key.to_vec().into())
            .await
            .unwrap();

        let event1 = mockdb
            .insert_event(
                domain::Event {
                    event_id: event_id.into(),
                    event_type: enums::EventType::PaymentSucceeded,
                    event_class: enums::EventClass::Payments,
                    is_webhook_notified: false,
                    primary_object_id: "primary_object_tet".into(),
                    primary_object_type: enums::EventObjectType::PaymentDetails,
                    created_at: common_utils::date_time::now(),
                    merchant_id: Some(merchant_id.to_owned()),
                    business_profile_id: Some(business_profile_id.to_owned()),
                    primary_object_created_at: Some(common_utils::date_time::now()),
                    idempotent_event_id: Some(event_id.into()),
                    initial_attempt_id: Some(event_id.into()),
                    request: None,
                    response: None,
                },
                &merchant_key_store,
            )
            .await
            .unwrap();

        assert_eq!(event1.event_id, event_id);

        let updated_event = mockdb
            .update_event_by_merchant_id_event_id(
                merchant_id,
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
        assert_eq!(updated_event.primary_object_id, "primary_object_tet");
        assert_eq!(updated_event.event_id, event_id);
    }
}
