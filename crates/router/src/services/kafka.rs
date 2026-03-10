use std::{collections::HashMap, sync::Arc};

use common_utils::{errors::CustomResult, types::TenantConfig};
use error_stack::{report, ResultExt};
use events::{EventsError, Message, MessagingInterface};
use num_traits::ToPrimitive;
use rdkafka::{
    config::FromClientConfig,
    message::{Header, OwnedHeaders},
    producer::{BaseRecord, DefaultProducerContext, Producer, ThreadedProducer},
};
use serde_json::Value;
#[cfg(feature = "payouts")]
pub mod payout;
use diesel_models::fraud_check::FraudCheck;

use crate::{events::EventType, services::kafka::fraud_check_event::KafkaFraudCheckEvent};
mod authentication;
mod authentication_event;
mod dispute;
mod dispute_event;
mod fraud_check;
mod fraud_check_event;
mod payment_attempt;
mod payment_attempt_event;
mod payment_intent;
mod payment_intent_event;
mod refund;
mod refund_event;
pub mod revenue_recovery;
use diesel_models::{authentication::Authentication, refund::Refund};
use hyperswitch_domain_models::payments::{payment_attempt::PaymentAttempt, PaymentIntent};
use serde::Serialize;
use time::{OffsetDateTime, PrimitiveDateTime};

#[cfg(feature = "payouts")]
use self::payout::KafkaPayout;
use self::{
    authentication::KafkaAuthentication, authentication_event::KafkaAuthenticationEvent,
    dispute::KafkaDispute, dispute_event::KafkaDisputeEvent, payment_attempt::KafkaPaymentAttempt,
    payment_attempt_event::KafkaPaymentAttemptEvent, payment_intent::KafkaPaymentIntent,
    payment_intent_event::KafkaPaymentIntentEvent, refund::KafkaRefund,
    refund_event::KafkaRefundEvent,
};
use crate::{services::kafka::fraud_check::KafkaFraudCheck, types::storage::Dispute};

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
    clickhouse_database: Option<String>,
}

impl<'a, T: KafkaMessage> KafkaEvent<'a, T> {
    fn new(event: &'a T, tenant_id: TenantID, clickhouse_database: Option<String>) -> Self {
        Self {
            event,
            sign_flag: 1,
            tenant_id,
            clickhouse_database,
        }
    }
    fn old(event: &'a T, tenant_id: TenantID, clickhouse_database: Option<String>) -> Self {
        Self {
            event,
            sign_flag: -1,
            tenant_id,
            clickhouse_database,
        }
    }
}

impl<T: KafkaMessage> KafkaMessage for KafkaEvent<'_, T> {
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

#[derive(serde::Serialize, Debug)]
struct KafkaConsolidatedLog<'a, T: KafkaMessage> {
    #[serde(flatten)]
    event: &'a T,
    tenant_id: TenantID,
}

#[derive(serde::Serialize, Debug)]
struct KafkaConsolidatedEvent<'a, T: KafkaMessage> {
    log: KafkaConsolidatedLog<'a, T>,
    log_type: EventType,
}

impl<'a, T: KafkaMessage> KafkaConsolidatedEvent<'a, T> {
    fn new(event: &'a T, tenant_id: TenantID) -> Self {
        Self {
            log: KafkaConsolidatedLog { event, tenant_id },
            log_type: event.event_type(),
        }
    }
}

impl<T: KafkaMessage> KafkaMessage for KafkaConsolidatedEvent<'_, T> {
    fn key(&self) -> String {
        self.log.event.key()
    }

    fn event_type(&self) -> EventType {
        EventType::Consolidated
    }

    fn creation_timestamp(&self) -> Option<i64> {
        self.log.event.creation_timestamp()
    }
}

#[derive(Debug, serde::Deserialize, Clone, Default)]
#[serde(default)]
pub struct KafkaSettings {
    brokers: Vec<String>,
    fraud_check_analytics_topic: String,
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
    consolidated_events_topic: String,
    authentication_analytics_topic: String,
    routing_logs_topic: String,
    revenue_recovery_topic: String,
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

        common_utils::fp_utils::when(self.consolidated_events_topic.is_default_or_empty(), || {
            Err(ApplicationError::InvalidConfigurationValueError(
                "Consolidated Events topic must not be empty".into(),
            ))
        })?;

        common_utils::fp_utils::when(
            self.authentication_analytics_topic.is_default_or_empty(),
            || {
                Err(ApplicationError::InvalidConfigurationValueError(
                    "Kafka Authentication Analytics topic must not be empty".into(),
                ))
            },
        )?;

        common_utils::fp_utils::when(self.routing_logs_topic.is_default_or_empty(), || {
            Err(ApplicationError::InvalidConfigurationValueError(
                "Kafka Routing Logs topic must not be empty".into(),
            ))
        })?;

        Ok(())
    }
}

#[derive(Clone, Debug)]
pub struct KafkaProducer {
    producer: Arc<RdKafkaProducer>,
    intent_analytics_topic: String,
    fraud_check_analytics_topic: String,
    attempt_analytics_topic: String,
    refund_analytics_topic: String,
    api_logs_topic: String,
    connector_logs_topic: String,
    outgoing_webhook_logs_topic: String,
    dispute_analytics_topic: String,
    audit_events_topic: String,
    #[cfg(feature = "payouts")]
    payout_analytics_topic: String,
    consolidated_events_topic: String,
    authentication_analytics_topic: String,
    ckh_database_name: Option<String>,
    routing_logs_topic: String,
    revenue_recovery_topic: String,
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
    pub fn set_tenancy(&mut self, tenant_config: &dyn TenantConfig) {
        self.ckh_database_name = Some(tenant_config.get_clickhouse_database().to_string());
    }

    pub async fn create(conf: &KafkaSettings) -> MQResult<Self> {
        Ok(Self {
            producer: Arc::new(RdKafkaProducer(
                ThreadedProducer::from_config(
                    rdkafka::ClientConfig::new().set("bootstrap.servers", conf.brokers.join(",")),
                )
                .change_context(KafkaError::InitializationError)?,
            )),

            fraud_check_analytics_topic: conf.fraud_check_analytics_topic.clone(),
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
            consolidated_events_topic: conf.consolidated_events_topic.clone(),
            authentication_analytics_topic: conf.authentication_analytics_topic.clone(),
            ckh_database_name: None,
            routing_logs_topic: conf.routing_logs_topic.clone(),
            revenue_recovery_topic: conf.revenue_recovery_topic.clone(),
        })
    }

    pub fn log_event<T: KafkaMessage>(&self, event: &T) -> MQResult<()> {
        router_env::logger::debug!("Logging Kafka Event {event:?}");
        let topic = self.get_topic(event.event_type());
        self.producer
            .0
            .send(
                BaseRecord::to(topic)
                    .key(&event.key())
                    .payload(&event.value()?)
                    .timestamp(event.creation_timestamp().unwrap_or_else(|| {
                        (OffsetDateTime::now_utc().unix_timestamp_nanos() / 1_000_000)
                            .try_into()
                            .unwrap_or_else(|_| {
                                // kafka producer accepts milliseconds
                                // try converting nanos to millis if that fails convert seconds to millis
                                OffsetDateTime::now_utc().unix_timestamp() * 1_000
                            })
                    })),
            )
            .map_err(|(error, record)| report!(error).attach_printable(format!("{record:?}")))
            .change_context(KafkaError::GenericError)
    }
    pub async fn log_fraud_check(
        &self,
        attempt: &FraudCheck,
        old_attempt: Option<FraudCheck>,
        tenant_id: TenantID,
    ) -> MQResult<()> {
        if let Some(negative_event) = old_attempt {
            self.log_event(&KafkaEvent::old(
                &KafkaFraudCheck::from_storage(&negative_event),
                tenant_id.clone(),
                self.ckh_database_name.clone(),
            ))
            .attach_printable_lazy(|| {
                format!("Failed to add negative fraud check event {negative_event:?}")
            })?;
        };

        self.log_event(&KafkaEvent::new(
            &KafkaFraudCheck::from_storage(attempt),
            tenant_id.clone(),
            self.ckh_database_name.clone(),
        ))
        .attach_printable_lazy(|| {
            format!("Failed to add positive fraud check event {attempt:?}")
        })?;

        self.log_event(&KafkaConsolidatedEvent::new(
            &KafkaFraudCheckEvent::from_storage(attempt),
            tenant_id.clone(),
        ))
        .attach_printable_lazy(|| {
            format!("Failed to add consolidated fraud check  event {attempt:?}")
        })
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
                self.ckh_database_name.clone(),
            ))
            .attach_printable_lazy(|| {
                format!("Failed to add negative attempt event {negative_event:?}")
            })?;
        };

        self.log_event(&KafkaEvent::new(
            &KafkaPaymentAttempt::from_storage(attempt),
            tenant_id.clone(),
            self.ckh_database_name.clone(),
        ))
        .attach_printable_lazy(|| format!("Failed to add positive attempt event {attempt:?}"))?;

        self.log_event(&KafkaConsolidatedEvent::new(
            &KafkaPaymentAttemptEvent::from_storage(attempt),
            tenant_id.clone(),
        ))
        .attach_printable_lazy(|| format!("Failed to add consolidated attempt event {attempt:?}"))
    }

    pub async fn log_payment_attempt_delete(
        &self,
        delete_old_attempt: &PaymentAttempt,
        tenant_id: TenantID,
    ) -> MQResult<()> {
        self.log_event(&KafkaEvent::old(
            &KafkaPaymentAttempt::from_storage(delete_old_attempt),
            tenant_id.clone(),
            self.ckh_database_name.clone(),
        ))
        .attach_printable_lazy(|| {
            format!("Failed to add negative attempt event {delete_old_attempt:?}")
        })
    }

    pub async fn log_authentication(
        &self,
        authentication: &Authentication,
        old_authentication: Option<Authentication>,
        tenant_id: TenantID,
    ) -> MQResult<()> {
        if let Some(negative_event) = old_authentication {
            self.log_event(&KafkaEvent::old(
                &KafkaAuthentication::from_storage(&negative_event),
                tenant_id.clone(),
                self.ckh_database_name.clone(),
            ))
            .attach_printable_lazy(|| {
                format!("Failed to add negative authentication event {negative_event:?}")
            })?;
        };

        self.log_event(&KafkaEvent::new(
            &KafkaAuthentication::from_storage(authentication),
            tenant_id.clone(),
            self.ckh_database_name.clone(),
        ))
        .attach_printable_lazy(|| {
            format!("Failed to add positive authentication event {authentication:?}")
        })?;

        self.log_event(&KafkaConsolidatedEvent::new(
            &KafkaAuthenticationEvent::from_storage(authentication),
            tenant_id.clone(),
        ))
        .attach_printable_lazy(|| {
            format!("Failed to add consolidated authentication event {authentication:?}")
        })
    }

    pub async fn log_payment_intent(
        &self,
        intent: &PaymentIntent,
        old_intent: Option<PaymentIntent>,
        tenant_id: TenantID,
        infra_values: Option<Value>,
    ) -> MQResult<()> {
        if let Some(negative_event) = old_intent {
            self.log_event(&KafkaEvent::old(
                &KafkaPaymentIntent::from_storage(&negative_event, infra_values.clone()),
                tenant_id.clone(),
                self.ckh_database_name.clone(),
            ))
            .attach_printable_lazy(|| {
                format!("Failed to add negative intent event {negative_event:?}")
            })?;
        };

        self.log_event(&KafkaEvent::new(
            &KafkaPaymentIntent::from_storage(intent, infra_values.clone()),
            tenant_id.clone(),
            self.ckh_database_name.clone(),
        ))
        .attach_printable_lazy(|| format!("Failed to add positive intent event {intent:?}"))?;

        self.log_event(&KafkaConsolidatedEvent::new(
            &KafkaPaymentIntentEvent::from_storage(intent, infra_values.clone()),
            tenant_id.clone(),
        ))
        .attach_printable_lazy(|| format!("Failed to add consolidated intent event {intent:?}"))
    }

    pub async fn log_payment_intent_delete(
        &self,
        delete_old_intent: &PaymentIntent,
        tenant_id: TenantID,
        infra_values: Option<Value>,
    ) -> MQResult<()> {
        self.log_event(&KafkaEvent::old(
            &KafkaPaymentIntent::from_storage(delete_old_intent, infra_values),
            tenant_id.clone(),
            self.ckh_database_name.clone(),
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
                self.ckh_database_name.clone(),
            ))
            .attach_printable_lazy(|| {
                format!("Failed to add negative refund event {negative_event:?}")
            })?;
        };

        self.log_event(&KafkaEvent::new(
            &KafkaRefund::from_storage(refund),
            tenant_id.clone(),
            self.ckh_database_name.clone(),
        ))
        .attach_printable_lazy(|| format!("Failed to add positive refund event {refund:?}"))?;

        self.log_event(&KafkaConsolidatedEvent::new(
            &KafkaRefundEvent::from_storage(refund),
            tenant_id.clone(),
        ))
        .attach_printable_lazy(|| format!("Failed to add consolidated refund event {refund:?}"))
    }

    pub async fn log_refund_delete(
        &self,
        delete_old_refund: &Refund,
        tenant_id: TenantID,
    ) -> MQResult<()> {
        self.log_event(&KafkaEvent::old(
            &KafkaRefund::from_storage(delete_old_refund),
            tenant_id.clone(),
            self.ckh_database_name.clone(),
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
                self.ckh_database_name.clone(),
            ))
            .attach_printable_lazy(|| {
                format!("Failed to add negative dispute event {negative_event:?}")
            })?;
        };

        self.log_event(&KafkaEvent::new(
            &KafkaDispute::from_storage(dispute),
            tenant_id.clone(),
            self.ckh_database_name.clone(),
        ))
        .attach_printable_lazy(|| format!("Failed to add positive dispute event {dispute:?}"))?;

        self.log_event(&KafkaConsolidatedEvent::new(
            &KafkaDisputeEvent::from_storage(dispute),
            tenant_id.clone(),
        ))
        .attach_printable_lazy(|| format!("Failed to add consolidated dispute event {dispute:?}"))
    }

    pub async fn log_dispute_delete(
        &self,
        delete_old_dispute: &Dispute,
        tenant_id: TenantID,
    ) -> MQResult<()> {
        self.log_event(&KafkaEvent::old(
            &KafkaDispute::from_storage(delete_old_dispute),
            tenant_id.clone(),
            self.ckh_database_name.clone(),
        ))
        .attach_printable_lazy(|| {
            format!("Failed to add negative dispute event {delete_old_dispute:?}")
        })
    }

    #[cfg(feature = "payouts")]
    pub async fn log_payout(
        &self,
        payout: &KafkaPayout<'_>,
        old_payout: Option<KafkaPayout<'_>>,
        tenant_id: TenantID,
    ) -> MQResult<()> {
        if let Some(negative_event) = old_payout {
            self.log_event(&KafkaEvent::old(
                &negative_event,
                tenant_id.clone(),
                self.ckh_database_name.clone(),
            ))
            .attach_printable_lazy(|| {
                format!("Failed to add negative payout event {negative_event:?}")
            })?;
        };
        self.log_event(&KafkaEvent::new(
            payout,
            tenant_id.clone(),
            self.ckh_database_name.clone(),
        ))
        .attach_printable_lazy(|| format!("Failed to add positive payout event {payout:?}"))
    }

    #[cfg(feature = "payouts")]
    pub async fn log_payout_delete(
        &self,
        delete_old_payout: &KafkaPayout<'_>,
        tenant_id: TenantID,
    ) -> MQResult<()> {
        self.log_event(&KafkaEvent::old(
            delete_old_payout,
            tenant_id.clone(),
            self.ckh_database_name.clone(),
        ))
        .attach_printable_lazy(|| {
            format!("Failed to add negative payout event {delete_old_payout:?}")
        })
    }

    pub fn get_topic(&self, event: EventType) -> &str {
        match event {
            EventType::FraudCheck => &self.fraud_check_analytics_topic,
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
            EventType::Consolidated => &self.consolidated_events_topic,
            EventType::Authentication => &self.authentication_analytics_topic,
            EventType::RoutingApiLogs => &self.routing_logs_topic,
            EventType::RevenueRecovery => &self.revenue_recovery_topic,
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
        metadata: HashMap<String, String>,
        timestamp: PrimitiveDateTime,
    ) -> error_stack::Result<(), EventsError>
    where
        T: Message<Class = Self::MessageClass> + masking::ErasedMaskSerialize,
    {
        let topic = self.get_topic(data.get_message_class());
        let json_data = data
            .masked_serialize()
            .and_then(|mut value| {
                if let Value::Object(ref mut map) = value {
                    if let Some(db_name) = self.ckh_database_name.clone() {
                        map.insert("clickhouse_database".to_string(), Value::String(db_name));
                    }
                }
                serde_json::to_vec(&value)
            })
            .change_context(EventsError::SerializationError)?;
        let mut headers = OwnedHeaders::new();
        for (k, v) in metadata.iter() {
            headers = headers.insert(Header {
                key: k.as_str(),
                value: Some(v),
            });
        }
        headers = headers.insert(Header {
            key: "clickhouse_database",
            value: self.ckh_database_name.as_ref(),
        });
        self.producer
            .0
            .send(
                BaseRecord::to(topic)
                    .key(&data.identifier())
                    .payload(&json_data)
                    .headers(headers)
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
