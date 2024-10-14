use common_utils::{custom_serde, encryption::Encryption};
use diesel::{
    expression::AsExpression, AsChangeset, Identifiable, Insertable, Queryable, Selectable,
};
use serde::{Deserialize, Serialize};
use time::PrimitiveDateTime;

use crate::{enums as storage_enums, schema::events};

#[derive(Clone, Debug, Insertable, router_derive::DebugAsDisplay)]
#[diesel(table_name = events)]
pub struct EventNew {
    pub event_id: String,
    pub event_type: storage_enums::EventType,
    pub event_class: storage_enums::EventClass,
    pub is_webhook_notified: bool,
    pub primary_object_id: String,
    pub primary_object_type: storage_enums::EventObjectType,
    pub created_at: PrimitiveDateTime,
    pub merchant_id: Option<common_utils::id_type::MerchantId>,
    pub business_profile_id: Option<common_utils::id_type::ProfileId>,
    pub primary_object_created_at: Option<PrimitiveDateTime>,
    pub idempotent_event_id: Option<String>,
    pub initial_attempt_id: Option<String>,
    pub request: Option<Encryption>,
    pub response: Option<Encryption>,
    pub delivery_attempt: Option<storage_enums::WebhookDeliveryAttempt>,
    pub metadata: Option<EventMetadata>,
}

#[derive(Clone, Debug, Default, AsChangeset, router_derive::DebugAsDisplay)]
#[diesel(table_name = events)]
pub struct EventUpdateInternal {
    pub is_webhook_notified: Option<bool>,
    pub response: Option<Encryption>,
}

#[derive(Clone, Debug, Deserialize, Serialize, Identifiable, Queryable, Selectable)]
#[diesel(table_name = events, primary_key(event_id), check_for_backend(diesel::pg::Pg))]
pub struct Event {
    pub event_id: String,
    pub event_type: storage_enums::EventType,
    pub event_class: storage_enums::EventClass,
    pub is_webhook_notified: bool,
    pub primary_object_id: String,
    pub primary_object_type: storage_enums::EventObjectType,
    #[serde(with = "custom_serde::iso8601")]
    pub created_at: PrimitiveDateTime,
    pub merchant_id: Option<common_utils::id_type::MerchantId>,
    pub business_profile_id: Option<common_utils::id_type::ProfileId>,
    // This column can be used to partition the database table, so that all events related to a
    // single object would reside in the same partition
    pub primary_object_created_at: Option<PrimitiveDateTime>,
    pub idempotent_event_id: Option<String>,
    pub initial_attempt_id: Option<String>,
    pub request: Option<Encryption>,
    pub response: Option<Encryption>,
    pub delivery_attempt: Option<storage_enums::WebhookDeliveryAttempt>,
    pub metadata: Option<EventMetadata>,
}

#[derive(Clone, Debug, Deserialize, Serialize, AsExpression, diesel::FromSqlRow)]
#[diesel(sql_type = diesel::sql_types::Jsonb)]
pub enum EventMetadata {
    Payment {
        payment_id: common_utils::id_type::PaymentId,
    },
    Payout {
        payout_id: String,
    },
    Refund {
        payment_id: common_utils::id_type::PaymentId,
        refund_id: String,
    },
    Dispute {
        payment_id: common_utils::id_type::PaymentId,
        attempt_id: String,
        dispute_id: String,
    },
    Mandate {
        payment_method_id: String,
        mandate_id: String,
    },
}

common_utils::impl_to_sql_from_sql_json!(EventMetadata);
