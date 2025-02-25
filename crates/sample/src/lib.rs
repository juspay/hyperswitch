pub mod domain;
pub mod address;
pub mod api_keys;
pub mod authentication;
pub mod authorization;
pub mod blocklist_fingerprint;
pub mod blocklist_lookup;
pub mod blocklist;
pub mod business_profile;
pub mod callback_mapper;
pub mod capture;
pub mod cards_info;
pub mod configs;
pub mod customers;
pub mod dashboard_metadata;
pub mod dispute;
pub mod dynamic_routing_stats;
pub mod ephemeral_key;
pub mod events;
pub mod file;
pub mod fraud_check;
pub mod generic_link;
pub mod gsm;
pub mod health_check;
pub mod locker_mock_up;
pub mod mandate;
pub mod merchant_account;
pub mod merchant_connector_account;
pub mod merchant_key_store;
pub mod organization;
pub mod payment_link;
pub mod payment_method;
pub mod process_tracker;
pub mod queue;
pub mod refund;
pub mod relay;
pub mod reverse_lookup;
pub mod role;
pub mod routing_algorithm;
pub mod unified_translations;
pub mod user_authentication_method;
pub mod user_key_store;
pub mod user;
pub mod user_role;
pub mod errors;



use std::sync::Arc;
#[cfg(feature = "email")]
use external_services::email::EmailSettings;
use hyperswitch_domain_models::{
    PayoutAttemptInterface, PayoutsInterface,
    payments::{payment_attempt, payment_intent}
};
// use storage_impl::{redis::kv_store};
use router_env::tracing_actix_web::RequestId;

#[derive(PartialEq, Eq)]
pub enum StorageImpl {
    Postgresql,
    PostgresqlTest,
    Mock,
}

#[async_trait::async_trait]
pub trait StorageInterface<E>:
    Send
    + Sync
    + dyn_clone::DynClone
    + address::AddressInterface<Error = E>
    + api_keys::ApiKeyInterface<Error = E>
    + authentication::AuthenticationInterface<Error = E>
    + authorization::AuthorizationInterface<Error = E>
    + blocklist_fingerprint::BlocklistFingerprintInterface<Error = E>
    + blocklist_lookup::BlocklistLookupInterface<Error = E>
    + blocklist::BlocklistInterface<Error = E>
    + business_profile::ProfileInterface<Error = E>
    + callback_mapper::CallbackMapperInterface<Error = E>
    + capture::CaptureInterface<Error = E>
    + cards_info::CardsInfoInterface<Error = E>
    + configs::ConfigInterface<Error = E>
    + customers::CustomerInterface<Error = E>
    + dashboard_metadata::DashboardMetadataInterface<Error = E>
    + dispute::DisputeInterface<Error = E>
    + dynamic_routing_stats::DynamicRoutingStatsInterface<Error = E>
    + ephemeral_key::EphemeralKeyInterface<Error = E>
    + events::EventInterface<Error = E>
    + file::FileMetadataInterface<Error = E>
    + fraud_check::FraudCheckInterface<Error = E>
    + generic_link::GenericLinkInterface<Error = E>
    + gsm::GsmInterface<Error = E>
    + locker_mock_up::LockerMockUpInterface<Error = E>
    + mandate::MandateInterface<Error = E>
    + merchant_account::MerchantAccountInterface<Error = E>
    + merchant_connector_account::ConnectorAccessToken<Error = E>
    + merchant_connector_account::MerchantConnectorAccountInterface<Error = E>
    + merchant_key_store::MerchantKeyStoreInterface<Error = E>
    + organization::OrganizationInterface<Error = E>
    + payment_link::PaymentLinkInterface<Error = E>
    + payment_method::PaymentMethodInterface<Error = E>
    + payment_attempt::PaymentAttemptInterface
    + payment_intent::PaymentIntentInterface
    + PayoutAttemptInterface
    + PayoutsInterface
    // + scheduler::SchedulerInterface
    + SchedulerInterface<E>
    + refund::RefundInterface<Error = E>
    + relay::RelayInterface<Error = E>
    + reverse_lookup::ReverseLookupInterface<Error = E>
    
    + MasterKeyInterface
    + RedisConnInterface
    // + RequestIdStore
    + routing_algorithm::RoutingAlgorithmInterface<Error = E>
    + unified_translations::UnifiedTranslationsInterface<Error = E>
    + user::BatchSampleDataInterface<Error = E>
    // + health_check::HealthCheckDbInterface
    + user_authentication_method::UserAuthenticationMethodInterface<Error = E>
    + user::ThemeInterface<Error = E>
    // // + 'static
    {
        // fn get_scheduler_db(&self) -> Box<dyn scheduler::SchedulerInterface>;
    
        // fn get_cache_store(&self) -> Box<(dyn kv_store::RedisConnInterface + Send + Sync + 'static)>;
    }

dyn_clone::clone_trait_object!(<E> StorageInterface<E>);

use redis_interface::errors as redis_errors;

pub trait RedisConnInterface {
    fn get_redis_conn(
        &self,
    ) -> error_stack::Result<Arc<redis_interface::RedisConnectionPool>, redis_errors::RedisError>;
}

#[async_trait::async_trait]
pub trait SchedulerInterface<E>:
    process_tracker::ProcessTrackerInterface<Error = E> + queue::QueueInterface<Error = E> + AsSchedulerInterface<E>
{
}

pub trait AsSchedulerInterface<E> {
    fn as_scheduler(&self) -> &dyn SchedulerInterface<E>;
}

impl<T: SchedulerInterface<E>, E> AsSchedulerInterface<E> for T {
    fn as_scheduler(&self) -> &dyn SchedulerInterface<E> {
        self
    }
}

#[async_trait::async_trait]
pub trait GlobalStorageInterface<E>:
    Send
    + Sync
    + dyn_clone::DynClone
    + user::UserInterface<Error = E>
    + user_role::UserRoleInterface<Error = E>
    + user_key_store::UserKeyStoreInterface<Error = E>
    + role::RoleInterface<Error = E>
    // + 'static
{
}
dyn_clone::clone_trait_object!(<E> GlobalStorageInterface<E>);

pub trait CommonStorageInterface<E>: StorageInterface<E> + GlobalStorageInterface<E> {
    fn get_storage_interface(&self) -> Box<dyn StorageInterface<E>>;
    fn get_global_storage_interface(&self) -> Box<dyn GlobalStorageInterface<E>>;
}

pub trait MasterKeyInterface {
    fn get_master_key(&self) -> &[u8];
}

pub trait RequestIdStore {
    fn add_request_id(&mut self, _request_id: String) {}
    fn get_request_id(&self) -> Option<String> {
        None
    }
}

use masking;
use reqwest::multipart;
use std::time::Duration;


pub trait RequestBuilder: Send + Sync {
    fn json(&mut self, body: serde_json::Value);
    fn url_encoded_form(&mut self, body: serde_json::Value);
    fn timeout(&mut self, timeout: Duration);
    fn multipart(&mut self, form: multipart::Form);
    fn header(&mut self, key: String, value: masking::Maskable<String>) -> CustomResult<(), ApiClientError>;
    fn send(
        self,
    ) -> CustomResult<
        Box<
            (dyn core::future::Future<Output = Result<reqwest::Response, reqwest::Error>>
                 + 'static),
        >,
        ApiClientError,
    >;
}

use common_enums::ApiClientError;
use common_utils::errors::CustomResult;
use common_utils::request::Request;
use common_utils::request::Method;

#[async_trait::async_trait]
pub trait ApiClient: dyn_clone::DynClone
where
    Self: Send + Sync,
{
    type State;
    fn request(
        &self,
        method: Method,
        url: String,
    ) -> CustomResult<Box<dyn RequestBuilder>, ApiClientError>;

    fn request_with_certificate(
        &self,
        method: Method,
        url: String,
        certificate: Option<masking::Secret<String>>,
        certificate_key: Option<masking::Secret<String>>,
    ) -> CustomResult<Box<dyn RequestBuilder>, ApiClientError>;

    // this function should be in different trait because using SessionState
    async fn send_request(
        &self,
        // state: &SessionState,
        state: &Self::State,
        request: Request,
        option_timeout_secs: Option<u64>,
        forward_to_kafka: bool,
    ) -> CustomResult<reqwest::Response, ApiClientError>;

    fn add_request_id(&mut self, request_id: RequestId);

    fn get_request_id(&self) -> Option<String>;

    fn add_flow_name(&mut self, flow_name: String);
}

dyn_clone::clone_trait_object!(<State> ApiClient<State = State>);

#[allow(unused_imports)]
use rdkafka::{
    config::FromClientConfig,
    message::{Header, OwnedHeaders},
    producer::{BaseRecord, DefaultProducerContext, Producer, ThreadedProducer},
};

#[derive(Clone, Debug)]
#[allow(dead_code)]
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
    // #[cfg(feature = "payouts")]
    // payout_analytics_topic: String,
    consolidated_events_topic: String,
    authentication_analytics_topic: String,
    ckh_database_name: Option<String>,
}

#[allow(dead_code)]
struct RdKafkaProducer(ThreadedProducer<DefaultProducerContext>);

impl std::fmt::Debug for RdKafkaProducer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("RdKafkaProducer")
    }
}

#[derive(Debug, Clone)]
pub enum EventsHandler {
    Kafka(KafkaProducer),
    // Logs(event_logger::EventLogger),
    Logs(EventLogger),
}

#[derive(Clone, Debug, Default)]
pub struct EventLogger {}

// impl EventLogger {
//     #[track_caller]
//     pub(super) fn log_event<T: KafkaMessage>(&self, event: &T) {
//         logger::info!(event = ?event.masked_serialize().unwrap_or_else(|e| serde_json::json!({"error": e.to_string()})), event_type =? event.event_type(), event_id =? event.key(), log_type =? "event");
//     }
// }




#[cfg(feature = "email")]
pub async fn create_email_client(
    email: &EmailSettings,
) -> Box<dyn EmailService> {
    match &email.client_config {
        EmailClientConfigs::Ses { aws_ses } => Box::new(
            AwsSes::create(
                &settings.email,
                aws_ses,
                settings.proxy.https_url.to_owned(),
            )
            .await,
        ),
        EmailClientConfigs::Smtp { smtp } => {
            Box::new(SmtpServer::create(&settings.email, smtp.clone()).await)
        }
        EmailClientConfigs::NoEmailClient => Box::new(NoEmailClient::create().await),
    }
}