use common_utils::crypto::OptionalEncryptableSecretString;
use diesel_models::{
    enums::{EventClass, EventObjectType, EventType},
    events::EventUpdateInternal,
};
use error_stack::ResultExt;
use masking::{PeekInterface, Secret};

use crate::{
    errors::{CustomResult, ValidationError},
    types::domain::types::{self, AsyncLift},
};

#[derive(Clone, Debug)]
pub struct Event {
    pub event_id: String,
    pub event_type: EventType,
    pub event_class: EventClass,
    pub is_webhook_notified: bool,
    pub primary_object_id: String,
    pub primary_object_type: EventObjectType,
    pub created_at: time::PrimitiveDateTime,
    pub primary_object_created_at: Option<time::PrimitiveDateTime>,
    pub idempotent_event_id: Option<String>,
    pub initial_attempt_id: Option<String>,
    pub request: OptionalEncryptableSecretString,
    pub response: OptionalEncryptableSecretString,
}

#[derive(Debug)]
pub enum EventUpdate {
    UpdateResponse {
        is_webhook_notified: bool,
        response: OptionalEncryptableSecretString,
    },
}

impl From<EventUpdate> for EventUpdateInternal {
    fn from(event_update: EventUpdate) -> Self {
        match event_update {
            EventUpdate::UpdateResponse {
                is_webhook_notified,
                response,
            } => Self {
                is_webhook_notified: Some(is_webhook_notified),
                response: response.map(Into::into),
            },
        }
    }
}

#[async_trait::async_trait]
impl super::behaviour::Conversion for Event {
    type DstType = diesel_models::events::Event;
    type NewDstType = diesel_models::events::EventNew;

    async fn convert(self) -> CustomResult<Self::DstType, ValidationError> {
        Ok(diesel_models::events::Event {
            event_id: self.event_id,
            event_type: self.event_type,
            event_class: self.event_class,
            is_webhook_notified: self.is_webhook_notified,
            primary_object_id: self.primary_object_id,
            primary_object_type: self.primary_object_type,
            created_at: self.created_at,
            primary_object_created_at: self.primary_object_created_at,
            idempotent_event_id: self.idempotent_event_id,
            initial_attempt_id: self.initial_attempt_id,
            request: self.request.map(Into::into),
            response: self.response.map(Into::into),
        })
    }

    async fn convert_back(
        item: Self::DstType,
        key: &Secret<Vec<u8>>,
    ) -> CustomResult<Self, ValidationError>
    where
        Self: Sized,
    {
        async {
            Ok(Self {
                event_id: item.event_id,
                event_type: item.event_type,
                event_class: item.event_class,
                is_webhook_notified: item.is_webhook_notified,
                primary_object_id: item.primary_object_id,
                primary_object_type: item.primary_object_type,
                created_at: item.created_at,
                primary_object_created_at: item.primary_object_created_at,
                idempotent_event_id: item.idempotent_event_id,
                initial_attempt_id: item.initial_attempt_id,
                request: item
                    .request
                    .async_lift(|inner| types::decrypt(inner, key.peek()))
                    .await?,
                response: item
                    .response
                    .async_lift(|inner| types::decrypt(inner, key.peek()))
                    .await?,
            })
        }
        .await
        .change_context(ValidationError::InvalidValue {
            message: "Failed while decrypting event data".to_string(),
        })
    }

    async fn construct_new(self) -> CustomResult<Self::NewDstType, ValidationError> {
        Ok(diesel_models::events::EventNew {
            event_id: self.event_id,
            event_type: self.event_type,
            event_class: self.event_class,
            is_webhook_notified: self.is_webhook_notified,
            primary_object_id: self.primary_object_id,
            primary_object_type: self.primary_object_type,
            created_at: self.created_at,
            primary_object_created_at: self.primary_object_created_at,
            idempotent_event_id: self.idempotent_event_id,
            initial_attempt_id: self.initial_attempt_id,
            request: self.request.map(Into::into),
            response: self.response.map(Into::into),
        })
    }
}
