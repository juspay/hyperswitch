#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(clippy::unwrap_used)]

pub mod address;
pub mod configs;
pub mod connector_response;
pub mod customers;
pub mod events;
pub mod locker_mock_up;
pub mod mandate;
pub mod merchant_account;
pub mod merchant_connector_account;
pub mod payment_attempt;
pub mod payment_intent;
pub mod payment_method;
pub mod process_tracker;
pub mod queue;
pub mod refund;
pub mod temp_card;

use std::{collections::HashMap, sync::Arc};

use futures_locks::{Mutex, MutexGuard};
#[cfg(feature = "sqlx")]
use sqlx::PgPool as SqlxPgPool;

#[cfg(any(feature = "diesel", feature = "sqlx"))]
use crate::configs::settings::Database;
#[cfg(feature = "diesel")]
use crate::connection::{diesel_make_pg_pool, PgPool as PgPoolDiesel};
#[cfg(feature = "diesel")]
use crate::services::Store;
use crate::types::storage::{
    ConnectorResponse, Customer, MerchantAccount, MerchantConnectorAccount, PaymentAttempt,
    PaymentIntent, ProcessTracker, Refund, TempCard,
};

#[derive(PartialEq, Eq)]
pub enum StorageImpl {
    #[cfg(feature = "diesel")]
    DieselPostgresql,
    #[cfg(feature = "diesel")]
    DieselPostgresqlTest,
    #[cfg(feature = "sqlx")]
    Sqlx,
    Mock,
}

#[async_trait::async_trait]
pub trait StorageInterface:
    Send
    + Sync
    + dyn_clone::DynClone
    + payment_attempt::PaymentAttemptInterface
    + mandate::MandateInterface
    + address::AddressInterface
    + configs::ConfigInterface
    + temp_card::TempCardInterface
    + customers::CustomerInterface
    + events::EventInterface
    + merchant_account::MerchantAccountInterface
    + merchant_connector_account::MerchantConnectorAccountInterface
    + locker_mock_up::LockerMockUpInterface
    + payment_intent::PaymentIntentInterface
    + payment_method::PaymentMethodInterface
    + process_tracker::ProcessTrackerInterface
    + refund::RefundInterface
    + queue::QueueInterface
    + connector_response::ConnectorResponseInterface
    + 'static
{
    async fn close(&mut self) {}
}

#[cfg(feature = "diesel")]
#[derive(Clone)]
pub struct SqlDb {
    pub conn: PgPoolDiesel,
}

#[cfg(feature = "diesel")]
impl SqlDb {
    pub async fn new(database: &Database) -> Self {
        Self {
            conn: diesel_make_pg_pool(database, false).await,
        }
    }

    pub async fn test(database: &Database) -> Self {
        Self {
            conn: diesel_make_pg_pool(
                &Database {
                    dbname: String::from("test_db"),
                    ..database.clone()
                },
                true,
            )
            .await,
        }
    }
}

#[cfg(feature = "diesel")]
#[async_trait::async_trait]
impl StorageInterface for Store {
    #[allow(clippy::expect_used)]
    async fn close(&mut self) {
        std::sync::Arc::get_mut(&mut self.redis_conn)
            .expect("Redis connection pool cannot be closed")
            .close_connections()
            .await;
    }
}

#[cfg(feature = "sqlx")]
#[derive(Clone)]
pub struct Sqlx {
    pool: SqlxPgPool,
}

#[cfg(feature = "sqlx")]
impl Sqlx {
    #[allow(clippy::expect_used)]
    pub async fn new(database: &Database) -> Sqlx {
        let database_url = format!(
            "postgres://{}:{}@{}:{}/{}",
            database.username, database.password, database.host, database.port, database.dbname
        );

        let pool = SqlxPgPool::connect(&database_url)
            .await
            .expect("Failed to create PostgreSQL connection pool");

        Sqlx { pool }
    }
}

#[cfg(feature = "sqlx")]
#[async_trait::async_trait]
impl StorageInterface for Sqlx {}

#[derive(Clone)]
pub struct MockDb {
    merchant_accounts: Arc<Mutex<Vec<MerchantAccount>>>,
    merchant_connector_accounts: Arc<Mutex<Vec<MerchantConnectorAccount>>>,
    payment_attempts: Arc<Mutex<Vec<PaymentAttempt>>>,
    payment_intents: Arc<Mutex<Vec<PaymentIntent>>>,
    customers: Arc<Mutex<Vec<Customer>>>,
    temp_cards: Arc<Mutex<Vec<TempCard>>>,
    refunds: Arc<Mutex<Vec<Refund>>>,
    processes: Arc<Mutex<Vec<ProcessTracker>>>,
    connector_response: Arc<Mutex<Vec<ConnectorResponse>>>,
    values: Arc<Mutex<HashMap<String, Vec<u8>>>>,
    redis: Arc<redis_interface::RedisConnectionPool>,
}

impl MockDb {
    pub async fn new(redis: &crate::configs::settings::Settings) -> Self {
        Self {
            merchant_accounts: Default::default(),
            merchant_connector_accounts: Default::default(),
            payment_attempts: Default::default(),
            payment_intents: Default::default(),
            customers: Default::default(),
            temp_cards: Default::default(),
            refunds: Default::default(),
            processes: Default::default(),
            connector_response: Default::default(),
            values: Default::default(),
            redis: Arc::new(crate::connection::redis_connection(redis).await),
        }
    }

    async fn merchant_accounts(&self) -> MutexGuard<Vec<MerchantAccount>> {
        self.merchant_accounts.lock().await
    }

    async fn merchant_connector_accounts(&self) -> MutexGuard<Vec<MerchantConnectorAccount>> {
        self.merchant_connector_accounts.lock().await
    }

    async fn payment_attempts(&self) -> MutexGuard<Vec<PaymentAttempt>> {
        self.payment_attempts.lock().await
    }

    async fn payment_intents(&self) -> MutexGuard<Vec<PaymentIntent>> {
        self.payment_intents.lock().await
    }

    async fn customers(&self) -> MutexGuard<Vec<Customer>> {
        self.customers.lock().await
    }

    async fn temp_cards(&self) -> MutexGuard<Vec<TempCard>> {
        self.temp_cards.lock().await
    }

    async fn refunds(&self) -> MutexGuard<Vec<Refund>> {
        self.refunds.lock().await
    }

    async fn processes(&self) -> MutexGuard<Vec<ProcessTracker>> {
        self.processes.lock().await
    }

    async fn connector_response(&self) -> MutexGuard<Vec<ConnectorResponse>> {
        self.connector_response.lock().await
    }
}

#[async_trait::async_trait]
impl StorageInterface for MockDb {
    #[allow(clippy::expect_used)]
    async fn close(&mut self) {
        std::sync::Arc::get_mut(&mut self.redis)
            .expect("Redis connection pool cannot be closed")
            .close_connections()
            .await;
    }
}

pub async fn get_and_deserialize_key<T>(
    db: &dyn StorageInterface,
    key: &str,
    type_name: &str,
) -> common_utils::errors::CustomResult<T, redis_interface::errors::RedisError>
where
    T: serde::de::DeserializeOwned,
{
    use common_utils::ext_traits::ByteSliceExt;
    use error_stack::ResultExt;

    let bytes = db.get_key(key).await?;
    bytes
        .parse_struct(type_name)
        .change_context(redis_interface::errors::RedisError::JsonDeserializationFailed)
}

dyn_clone::clone_trait_object!(StorageInterface);
