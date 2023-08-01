use common_utils::errors::{CustomResult};
use common_utils::pii::REDACTED;
use crate::services::{Store, MockDb};
use crate::cache::Cacheable;
use crate::db::cache::publish_and_redact;
use crate::{self as storage, cache, CardInfo, enums, EphemeralKeyNew, EphemeralKey};
use crate::{domain::behaviour::Conversion, connection};
use crate::AddressNew;
use crate::address::AddressUpdateInternal;
use error_stack::{IntoReport, ResultExt};
use crate::{domain, errors};
use crate::domain::CustomerUpdate;

#[async_trait::async_trait]
pub trait EventInterface {
    async fn insert_event(
        &self,
        event: storage::EventNew,
    ) -> CustomResult<storage::Event, errors::StorageError>;
    async fn update_event(
        &self,
        event_id: String,
        event: storage::EventUpdate,
    ) -> CustomResult<storage::Event, errors::StorageError>;
}

#[async_trait::async_trait]
impl EventInterface for Store {
    async fn insert_event(
        &self,
        event: storage::EventNew,
    ) -> CustomResult<storage::Event, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        event.insert(&conn).await.map_err(Into::into).into_report()
    }
    async fn update_event(
        &self,
        event_id: String,
        event: storage::EventUpdate,
    ) -> CustomResult<storage::Event, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        storage::Event::update(&conn, &event_id, event)
            .await
            .map_err(Into::into)
            .into_report()
    }
}

#[async_trait::async_trait]
impl EventInterface for MockDb {
    async fn insert_event(
        &self,
        event: storage::EventNew,
    ) -> CustomResult<storage::Event, errors::StorageError> {
        let mut locked_events = self.events.lock().await;
        let now = common_utils::date_time::now();

        let stored_event = storage::Event {
            #[allow(clippy::as_conversions)]
            id: locked_events.len() as i32,
            event_id: event.event_id,
            event_type: event.event_type,
            event_class: event.event_class,
            is_webhook_notified: event.is_webhook_notified,
            intent_reference_id: event.intent_reference_id,
            primary_object_id: event.primary_object_id,
            primary_object_type: event.primary_object_type,
            created_at: now,
        };

        locked_events.push(stored_event.clone());

        Ok(stored_event)
    }
    async fn update_event(
        &self,
        event_id: String,
        event: storage::EventUpdate,
    ) -> CustomResult<storage::Event, errors::StorageError> {
        let mut locked_events = self.events.lock().await;
        let event_to_update = locked_events
            .iter_mut()
            .find(|e| e.event_id == event_id)
            .ok_or(errors::StorageError::MockDbError)?;

        match event {
            storage::EventUpdate::UpdateWebhookNotified {
                is_webhook_notified,
            } => {
                if let Some(is_webhook_notified) = is_webhook_notified {
                    event_to_update.is_webhook_notified = is_webhook_notified;
                }
            }
        }

        Ok(event_to_update.clone())
    }
}

#[cfg(test)]
mod tests {
    use diesel_models::enums;

    use crate::{
        db::{events::EventInterface, MockDb},
        types::storage,
    };

    #[allow(clippy::unwrap_used)]
    #[tokio::test]
    async fn test_mockdb_event_interface() {
        let mockdb = MockDb::new(&Default::default()).await;

        let event1 = mockdb
            .insert_event(storage::EventNew {
                event_id: "test_event_id".into(),
                event_type: enums::EventType::PaymentSucceeded,
                event_class: enums::EventClass::Payments,
                is_webhook_notified: false,
                intent_reference_id: Some("test".into()),
                primary_object_id: "primary_object_tet".into(),
                primary_object_type: enums::EventObjectType::PaymentDetails,
            })
            .await
            .unwrap();

        assert_eq!(event1.id, 0);

        let updated_event = mockdb
            .update_event(
                "test_event_id".into(),
                storage::EventUpdate::UpdateWebhookNotified {
                    is_webhook_notified: Some(true),
                },
            )
            .await
            .unwrap();

        assert!(updated_event.is_webhook_notified);
        assert_eq!(updated_event.primary_object_id, "primary_object_tet");
        assert_eq!(updated_event.id, 0);
    }
}
