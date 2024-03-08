use error_stack::IntoReport;
use router_env::{instrument, tracing};

use super::{MockDb, Store};
use crate::{
    connection,
    core::errors::{self, CustomResult},
    types::storage,
};

#[async_trait::async_trait]
pub trait EventInterface {
    async fn insert_event(
        &self,
        event: storage::EventNew,
    ) -> CustomResult<storage::Event, errors::StorageError>;

    async fn find_event_by_event_id(
        &self,
        event_id: &str,
    ) -> CustomResult<storage::Event, errors::StorageError>;

    async fn update_event(
        &self,
        event_id: String,
        event: storage::EventUpdate,
    ) -> CustomResult<storage::Event, errors::StorageError>;
}

#[async_trait::async_trait]
impl EventInterface for Store {
    #[instrument(skip_all)]
    async fn insert_event(
        &self,
        event: storage::EventNew,
    ) -> CustomResult<storage::Event, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        event.insert(&conn).await.map_err(Into::into).into_report()
    }

    #[instrument(skip_all)]
    async fn find_event_by_event_id(
        &self,
        event_id: &str,
    ) -> CustomResult<storage::Event, errors::StorageError> {
        let conn = connection::pg_connection_read(self).await?;
        storage::Event::find_by_event_id(&conn, event_id)
            .await
            .map_err(Into::into)
            .into_report()
    }

    #[instrument(skip_all)]
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
            event_id: event.event_id.clone(),
            event_type: event.event_type,
            event_class: event.event_class,
            is_webhook_notified: event.is_webhook_notified,
            primary_object_id: event.primary_object_id,
            primary_object_type: event.primary_object_type,
            created_at: now,
            idempotent_event_id: Some(event.event_id.clone()),
            initial_attempt_id: Some(event.event_id),
            request: None,
            response: None,
        };

        locked_events.push(stored_event.clone());

        Ok(stored_event)
    }

    async fn find_event_by_event_id(
        &self,
        event_id: &str,
    ) -> CustomResult<storage::Event, errors::StorageError> {
        let locked_events = self.events.lock().await;
        locked_events
            .iter()
            .find(|event| event.event_id == event_id)
            .cloned()
            .ok_or(
                errors::StorageError::ValueNotFound(format!(
                    "No event available with event_id  = {event_id}"
                ))
                .into(),
            )
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
        #[allow(clippy::expect_used)]
        let mockdb = MockDb::new(&redis_interface::RedisSettings::default())
            .await
            .expect("Failed to create Mock store");
        let test_event_id = "test_event_id";

        let event1 = mockdb
            .insert_event(storage::EventNew {
                event_id: test_event_id.into(),
                event_type: enums::EventType::PaymentSucceeded,
                event_class: enums::EventClass::Payments,
                is_webhook_notified: false,
                primary_object_id: "primary_object_tet".into(),
                primary_object_type: enums::EventObjectType::PaymentDetails,
                idempotent_event_id: Some(test_event_id.into()),
                initial_attempt_id: Some(test_event_id.into()),
                request: None,
                response: None,
            })
            .await
            .unwrap();

        assert_eq!(event1.event_id, test_event_id);

        let updated_event = mockdb
            .update_event(
                test_event_id.into(),
                storage::EventUpdate::UpdateWebhookNotified {
                    is_webhook_notified: Some(true),
                },
            )
            .await
            .unwrap();

        assert!(updated_event.is_webhook_notified);
        assert_eq!(updated_event.primary_object_id, "primary_object_tet");
        assert_eq!(updated_event.event_id, test_event_id);
    }
}
