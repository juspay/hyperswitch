pub mod payment_attempt;
pub mod payment_intent;

use diesel_models::{payment_attempt::PaymentAttempt, payment_intent::PaymentIntent};

use crate::redis::kv_store::KvStorePartition;

impl KvStorePartition for PaymentIntent {}
impl KvStorePartition for PaymentAttempt {}
