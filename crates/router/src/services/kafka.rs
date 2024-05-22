use std::sync::Arc;

use bigdecimal::ToPrimitive;
use common_utils::errors::CustomResult;
use error_stack::{report, ResultExt};
use events::{EventsError, Message, MessagingInterface};
use rdkafka::{
    config::FromClientConfig,
    producer::{BaseRecord, DefaultProducerContext, Producer, ThreadedProducer},
};
#[cfg(feature = "payouts")]
pub mod payout;
use crate::events::EventType;
mod dispute;
mod payment_attempt;
mod payment_intent;
mod refund;
use diesel_models::refund::Refund;
use hyperswitch_domain_models::payments::{payment_attempt::PaymentAttempt, PaymentIntent};
use serde::Serialize;
use time::{OffsetDateTime, PrimitiveDateTime};

#[cfg(feature = "payouts")]
use self::payout::KafkaPayout;
use self::{
    dispute::KafkaDispute, payment_attempt::KafkaPaymentAttempt,
    payment_intent::KafkaPaymentIntent, refund::KafkaRefund,
};
use crate::types::storage::Dispute;

// Using message queue result here to avoid confusion with Kafka result provided by library
pub type MQResult<T> = CustomResult<T, KafkaError>;
use crate::db::kafka_store::TenantID;

pub trait KafkaMessage
where
    Self: Serialize + std::fmt::Debug,
{
    fn value(&self) -> MQResult<Vec<u8>> {
        // Add better error logging here
        serde_json::to_vec(&self).change_context(KafkaError::GenericError)
    }

    fn key(&self) -> String;

    fn event_type(&self) -> EventType;

    fn creation_timestamp(&self) -> Option<i64> {
        None
    }
}

#[derive(serde::Serialize, Debug)]
struct KafkaEvent<'a, T: KafkaMessage> {
    #[serde(flatten)]
    event: &'a T,
    sign_flag: i32,
    tenant_id: TenantID,
}

impl<'a, T: KafkaMessage> KafkaEvent<'a, T> {
    fn new(event: &'a T, tenant_id: TenantID) -> Self {
        Self {
            event,
            sign_flag: 1,
            tenant_id,
        }
    }
    fn old(event: &'a T, tenant_id: TenantID) -> Self {
        Self {
            event,
            sign_flag: -1,
            tenant_id,
        }
    }
}

impl<'a, T: KafkaMessage> KafkaMessage for KafkaEvent<'a, T> {
    fn key(&self) -> String {
        self.event.key()
    }

    fn event_type(&self) -> EventType {
        self.event.event_type()
    }

    fn creation_timestamp(&self) -> Option<i64> {
        self.event.creation_timestamp()
    }
}

#[derive(Debug, serde::Deserialize, Clone, Default)]
#[serde(default)]
pub struct KafkaSettings {
    brokers: Vec<String>,
    intent_analytics_topic: String,
    attempt_analytics_topic: String,
    refund_analytics_topic: String,
    api_logs_topic: String,
    connector_logs_topic: String,
    outgoing_webhook_logs_topic: String,
    dispute_analytics_topic: String,
    audit_events_topic: String,
    #[cfg(feature = "payouts")]
    payout_analytics_topic: String,
}

impl KafkaSettings {
    pub fn validate(&self) -> Result<(), crate::core::errors::ApplicationError> {
        use common_utils::ext_traits::ConfigExt;

        use crate::core::errors::ApplicationError;

        common_utils::fp_utils::when(self.brokers.is_empty(), || {
            Err(ApplicationError::InvalidConfigurationValueError(
                "Kafka brokers must not be empty".into(),
            ))
        })?;

        common_utils::fp_utils::when(self.intent_analytics_topic.is_default_or_empty(), || {
            Err(ApplicationError::InvalidConfigurationValueError(
                "Kafka Intent Analytics topic must not be empty".into(),
            ))
        })?;

        common_utils::fp_utils::when(self.attempt_analytics_topic.is_default_or_empty(), || {
            Err(ApplicationError::InvalidConfigurationValueError(
                "Kafka Attempt Analytics topic must not be empty".into(),
            ))
        })?;

        common_utils::fp_utils::when(self.refund_analytics_topic.is_default_or_empty(), || {
            Err(ApplicationError::InvalidConfigurationValueError(
                "Kafka Refund Analytics topic must not be empty".into(),
            ))
        })?;

        common_utils::fp_utils::when(self.api_logs_topic.is_default_or_empty(), || {
            Err(ApplicationError::InvalidConfigurationValueError(
                "Kafka API event Analytics topic must not be empty".into(),
            ))
        })?;

        common_utils::fp_utils::when(self.connector_logs_topic.is_default_or_empty(), || {
            Err(ApplicationError::InvalidConfigurationValueError(
                "Kafka Connector Logs topic must not be empty".into(),
            ))
        })?;

        common_utils::fp_utils::when(
            self.outgoing_webhook_logs_topic.is_default_or_empty(),
            || {
                Err(ApplicationError::InvalidConfigurationValueError(
                    "Kafka Outgoing Webhook Logs topic must not be empty".into(),
                ))
            },
        )?;

        common_utils::fp_utils::when(self.dispute_analytics_topic.is_default_or_empty(), || {
            Err(ApplicationError::InvalidConfigurationValueError(
                "Kafka Dispute Logs topic must not be empty".into(),
            ))
        })?;

        common_utils::fp_utils::when(self.audit_events_topic.is_default_or_empty(), || {
            Err(ApplicationError::InvalidConfigurationValueError(
                "Kafka Audit Events topic must not be empty".into(),
            ))
        })?;

        #[cfg(feature = "payouts")]
        common_utils::fp_utils::when(self.payout_analytics_topic.is_default_or_empty(), || {
            Err(ApplicationError::InvalidConfigurationValueError(
                "Kafka Payout Analytics topic must not be empty".into(),
            ))
        })?;

        Ok(())
    }
}

#[derive(Clone, Debug)]
pub struct KafkaProducer {
    producer: Arc<RdKafkaProducer>,
    intent_analytics_topic: String,
    attempt_analytics_topic: String,
    refund_analytics_topic: String,
    api_logs_topic: String,
    connector_logs_topic: String,
    outgoing_webhook_logs_topic: String,
    dispute_analytics_topic: String,
    audit_events_topic: String,
    #[cfg(feature = "payouts")]
    payout_analytics_topic: String,
}

struct RdKafkaProducer(ThreadedProducer<DefaultProducerContext>);

impl std::fmt::Debug for RdKafkaProducer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("RdKafkaProducer")
    }
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum KafkaError {
    #[error("Generic Kafka Error")]
    GenericError,
    #[error("Kafka not implemented")]
    NotImplemented,
    #[error("Kafka Initialization Error")]
    InitializationError,
}

#[allow(unused)]
impl KafkaProducer {
    pub async fn create(conf: &KafkaSettings) -> MQResult<Self> {
        Ok(Self {
            producer: Arc::new(RdKafkaProducer(
                ThreadedProducer::from_config(
                    rdkafka::ClientConfig::new().set("bootstrap.servers", conf.brokers.join(",")),
                )
                .change_context(KafkaError::InitializationError)?,
            )),

            intent_analytics_topic: conf.intent_analytics_topic.clone(),
            attempt_analytics_topic: conf.attempt_analytics_topic.clone(),
            refund_analytics_topic: conf.refund_analytics_topic.clone(),
            api_logs_topic: conf.api_logs_topic.clone(),
            connector_logs_topic: conf.connector_logs_topic.clone(),
            outgoing_webhook_logs_topic: conf.outgoing_webhook_logs_topic.clone(),
            dispute_analytics_topic: conf.dispute_analytics_topic.clone(),
            audit_events_topic: conf.audit_events_topic.clone(),
            #[cfg(feature = "payouts")]
            payout_analytics_topic: conf.payout_analytics_topic.clone(),
        })
    }

    pub fn log_event<T: KafkaMessage>(&self, event: &T) -> MQResult<()> {
        router_env::logger::debug!("Logging Kafka Event {event:?}");
        let topic = match event.event_type() {
            EventType::PaymentIntent => &self.intent_analytics_topic,
            EventType::PaymentAttempt => &self.attempt_analytics_topic,
            EventType::Refund => &self.refund_analytics_topic,
            EventType::ApiLogs => &self.api_logs_topic,
            EventType::ConnectorApiLogs => &self.connector_logs_topic,
            EventType::OutgoingWebhookLogs => &self.outgoing_webhook_logs_topic,
            EventType::Dispute => &self.dispute_analytics_topic,
            EventType::AuditEvent => &self.audit_events_topic,
            #[cfg(feature = "payouts")]
            EventType::Payout => &self.payout_analytics_topic,
        };
        self.producer
            .0
            .send(
                BaseRecord::to(topic)
                    .key(&event.key())
                    .payload(&event.value()?)
                    .timestamp(
                        event
                            .creation_timestamp()
                            .unwrap_or_else(|| OffsetDateTime::now_utc().unix_timestamp() * 1_000),
                    ),
            )
            .map_err(|(error, record)| report!(error).attach_printable(format!("{record:?}")))
            .change_context(KafkaError::GenericError)
    }

    pub async fn log_payment_attempt(
        &self,
        attempt: &PaymentAttempt,
        old_attempt: Option<PaymentAttempt>,
        tenant_id: TenantID,
    ) -> MQResult<()> {
        if let Some(negative_event) = old_attempt {
            self.log_event(&KafkaEvent::old(
                &KafkaPaymentAttempt::from_storage(&negative_event),
                tenant_id.clone(),
            ))
            .attach_printable_lazy(|| {
                format!("Failed to add negative attempt event {negative_event:?}")
            })?;
        };
        self.log_event(&KafkaEvent::new(
            &KafkaPaymentAttempt::from_storage(attempt),
            tenant_id.clone(),
        ))
        .attach_printable_lazy(|| format!("Failed to add positive attempt event {attempt:?}"))
    }

    pub async fn log_payment_attempt_delete(
        &self,
        delete_old_attempt: &PaymentAttempt,
        tenant_id: TenantID,
    ) -> MQResult<()> {
        self.log_event(&KafkaEvent::old(
            &KafkaPaymentAttempt::from_storage(delete_old_attempt),
            tenant_id.clone(),
        ))
        .attach_printable_lazy(|| {
            format!("Failed to add negative attempt event {delete_old_attempt:?}")
        })
    }

    pub async fn log_payment_intent(
        &self,
        intent: &PaymentIntent,
        old_intent: Option<PaymentIntent>,
        tenant_id: TenantID,
    ) -> MQResult<()> {
        if let Some(negative_event) = old_intent {
            self.log_event(&KafkaEvent::old(
                &KafkaPaymentIntent::from_storage(&negative_event),
                tenant_id.clone(),
            ))
            .attach_printable_lazy(|| {
                format!("Failed to add negative intent event {negative_event:?}")
            })?;
        };
        self.log_event(&KafkaEvent::new(
            &KafkaPaymentIntent::from_storage(intent),
            tenant_id.clone(),
        ))
        .attach_printable_lazy(|| format!("Failed to add positive intent event {intent:?}"))
    }

    pub async fn log_payment_intent_delete(
        &self,
        delete_old_intent: &PaymentIntent,
        tenant_id: TenantID,
    ) -> MQResult<()> {
        self.log_event(&KafkaEvent::old(
            &KafkaPaymentIntent::from_storage(delete_old_intent),
            tenant_id.clone(),
        ))
        .attach_printable_lazy(|| {
            format!("Failed to add negative intent event {delete_old_intent:?}")
        })
    }

    pub async fn log_refund(
        &self,
        refund: &Refund,
        old_refund: Option<Refund>,
        tenant_id: TenantID,
    ) -> MQResult<()> {
        if let Some(negative_event) = old_refund {
            self.log_event(&KafkaEvent::old(
                &KafkaRefund::from_storage(&negative_event),
                tenant_id.clone(),
            ))
            .attach_printable_lazy(|| {
                format!("Failed to add negative refund event {negative_event:?}")
            })?;
        };
        self.log_event(&KafkaEvent::new(
            &KafkaRefund::from_storage(refund),
            tenant_id.clone(),
        ))
        .attach_printable_lazy(|| format!("Failed to add positive refund event {refund:?}"))
    }

    pub async fn log_refund_delete(
        &self,
        delete_old_refund: &Refund,
        tenant_id: TenantID,
    ) -> MQResult<()> {
        self.log_event(&KafkaEvent::old(
            &KafkaRefund::from_storage(delete_old_refund),
            tenant_id.clone(),
        ))
        .attach_printable_lazy(|| {
            format!("Failed to add negative refund event {delete_old_refund:?}")
        })
    }

    pub async fn log_dispute(
        &self,
        dispute: &Dispute,
        old_dispute: Option<Dispute>,
        tenant_id: TenantID,
    ) -> MQResult<()> {
        if let Some(negative_event) = old_dispute {
            self.log_event(&KafkaEvent::old(
                &KafkaDispute::from_storage(&negative_event),
                tenant_id.clone(),
            ))
            .attach_printable_lazy(|| {
                format!("Failed to add negative dispute event {negative_event:?}")
            })?;
        };
        self.log_event(&KafkaEvent::new(
            &KafkaDispute::from_storage(dispute),
            tenant_id.clone(),
        ))
        .attach_printable_lazy(|| format!("Failed to add positive dispute event {dispute:?}"))
    }

    #[cfg(feature = "payouts")]
    pub async fn log_payout(
        &self,
        payout: &KafkaPayout<'_>,
        old_payout: Option<KafkaPayout<'_>>,
        tenant_id: TenantID,
    ) -> MQResult<()> {
        if let Some(negative_event) = old_payout {
            self.log_event(&KafkaEvent::old(&negative_event, tenant_id.clone()))
                .attach_printable_lazy(|| {
                    format!("Failed to add negative payout event {negative_event:?}")
                })?;
        };
        self.log_event(&KafkaEvent::new(payout, tenant_id.clone()))
            .attach_printable_lazy(|| format!("Failed to add positive payout event {payout:?}"))
    }

    #[cfg(feature = "payouts")]
    pub async fn log_payout_delete(
        &self,
        delete_old_payout: &KafkaPayout<'_>,
        tenant_id: TenantID,
    ) -> MQResult<()> {
        self.log_event(&KafkaEvent::old(delete_old_payout, tenant_id.clone()))
            .attach_printable_lazy(|| {
                format!("Failed to add negative payout event {delete_old_payout:?}")
            })
    }

    pub fn get_topic(&self, event: EventType) -> &str {
        match event {
            EventType::ApiLogs => &self.api_logs_topic,
            EventType::PaymentAttempt => &self.attempt_analytics_topic,
            EventType::PaymentIntent => &self.intent_analytics_topic,
            EventType::Refund => &self.refund_analytics_topic,
            EventType::ConnectorApiLogs => &self.connector_logs_topic,
            EventType::OutgoingWebhookLogs => &self.outgoing_webhook_logs_topic,
            EventType::Dispute => &self.dispute_analytics_topic,
            EventType::AuditEvent => &self.audit_events_topic,
            #[cfg(feature = "payouts")]
            EventType::Payout => &self.payout_analytics_topic,
        }
    }
}

impl Drop for RdKafkaProducer {
    fn drop(&mut self) {
        // Flush the producer to send any pending messages
        match self.0.flush(rdkafka::util::Timeout::After(
            std::time::Duration::from_secs(5),
        )) {
            Ok(_) => router_env::logger::info!("Kafka events flush Successful"),
            Err(error) => router_env::logger::error!("Failed to flush Kafka Events {error:?}"),
        }
    }
}

impl MessagingInterface for KafkaProducer {
    type MessageClass = EventType;

    fn send_message<T>(
        &self,
        data: T,
        timestamp: PrimitiveDateTime,
    ) -> error_stack::Result<(), EventsError>
    where
        T: Message<Class = Self::MessageClass> + masking::ErasedMaskSerialize,
    {
        let topic = self.get_topic(data.get_message_class());
        let json_data = data
            .masked_serialize()
            .and_then(|i| serde_json::to_vec(&i))
            .change_context(EventsError::SerializationError)?;
        self.producer
            .0
            .send(
                BaseRecord::to(topic)
                    .key(&data.identifier())
                    .payload(&json_data)
                    .timestamp(
                        (timestamp.assume_utc().unix_timestamp_nanos() / 1_000_000)
                            .to_i64()
                            .unwrap_or_else(|| {
                                // kafka producer accepts milliseconds
                                // try converting nanos to millis if that fails convert seconds to millis
                                timestamp.assume_utc().unix_timestamp() * 1_000
                            }),
                    ),
            )
            .map_err(|(error, record)| report!(error).attach_printable(format!("{record:?}")))
            .change_context(KafkaError::GenericError)
            .change_context(EventsError::PublishError)
    }
}
