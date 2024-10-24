use common_utils::{
    crypto::{Encryptable, OptionalEncryptableSecretString},
    encryption::Encryption,
    type_name,
    types::keymanager::{KeyManagerState, ToEncryptable},
};
use diesel_models::{
    enums::{EventClass, EventObjectType, EventType, WebhookDeliveryAttempt},
    events::{EventMetadata, EventUpdateInternal},
};
use error_stack::ResultExt;
use masking::{PeekInterface, Secret};
use rustc_hash::FxHashMap;

use crate::{
    errors::{CustomResult, ValidationError},
    types::domain::types,
};

#[derive(Clone, Debug, router_derive::ToEncryption)]
pub struct Event {
    pub event_id: String,
    pub event_type: EventType,
    pub event_class: EventClass,
    pub is_webhook_notified: bool,
    pub primary_object_id: String,
    pub primary_object_type: EventObjectType,
    pub created_at: time::PrimitiveDateTime,
    pub merchant_id: Option<common_utils::id_type::MerchantId>,
    pub business_profile_id: Option<common_utils::id_type::ProfileId>,
    pub primary_object_created_at: Option<time::PrimitiveDateTime>,
    pub idempotent_event_id: Option<String>,
    pub initial_attempt_id: Option<String>,
    #[encrypt]
    pub request: Option<Encryptable<Secret<String>>>,
    #[encrypt]
    pub response: Option<Encryptable<Secret<String>>>,
    pub delivery_attempt: Option<WebhookDeliveryAttempt>,
    pub metadata: Option<EventMetadata>,
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
            merchant_id: self.merchant_id,
            business_profile_id: self.business_profile_id,
            primary_object_created_at: self.primary_object_created_at,
            idempotent_event_id: self.idempotent_event_id,
            initial_attempt_id: self.initial_attempt_id,
            request: self.request.map(Into::into),
            response: self.response.map(Into::into),
            delivery_attempt: self.delivery_attempt,
            metadata: self.metadata,
        })
    }

    async fn convert_back(
        state: &KeyManagerState,
        item: Self::DstType,
        key: &Secret<Vec<u8>>,
        key_manager_identifier: common_utils::types::keymanager::Identifier,
    ) -> CustomResult<Self, ValidationError>
    where
        Self: Sized,
    {
        let decrypted = types::crypto_operation(
            state,
            type_name!(Self::DstType),
            types::CryptoOperation::BatchDecrypt(EncryptedEvent::to_encryptable(EncryptedEvent {
                request: item.request.clone(),
                response: item.response.clone(),
            })),
            key_manager_identifier,
            key.peek(),
        )
        .await
        .and_then(|val| val.try_into_batchoperation())
        .change_context(ValidationError::InvalidValue {
            message: "Failed while decrypting event data".to_string(),
        })?;
        let encryptable_event = EncryptedEvent::from_encryptable(decrypted).change_context(
            ValidationError::InvalidValue {
                message: "Failed while decrypting event data".to_string(),
            },
        )?;
        Ok(Self {
            event_id: item.event_id,
            event_type: item.event_type,
            event_class: item.event_class,
            is_webhook_notified: item.is_webhook_notified,
            primary_object_id: item.primary_object_id,
            primary_object_type: item.primary_object_type,
            created_at: item.created_at,
            merchant_id: item.merchant_id,
            business_profile_id: item.business_profile_id,
            primary_object_created_at: item.primary_object_created_at,
            idempotent_event_id: item.idempotent_event_id,
            initial_attempt_id: item.initial_attempt_id,
            request: encryptable_event.request,
            response: encryptable_event.response,
            delivery_attempt: item.delivery_attempt,
            metadata: item.metadata,
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
            merchant_id: self.merchant_id,
            business_profile_id: self.business_profile_id,
            primary_object_created_at: self.primary_object_created_at,
            idempotent_event_id: self.idempotent_event_id,
            initial_attempt_id: self.initial_attempt_id,
            request: self.request.map(Into::into),
            response: self.response.map(Into::into),
            delivery_attempt: self.delivery_attempt,
            metadata: self.metadata,
        })
    }
}
