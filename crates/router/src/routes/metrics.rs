use once_cell::sync::Lazy;
use router_env::opentelemetry::{
    global,
    metrics::{Counter, Meter},
    Context,
};

use crate::create_counter;

pub static CONTEXT: Lazy<Context> = Lazy::new(Context::current);
static GLOBAL_METER: Lazy<Meter> = Lazy::new(|| global::meter("ROUTER_API"));

create_counter!(HEALTH_METRIC, GLOBAL_METER); // No. of health API hits
create_counter!(KV_MISS, GLOBAL_METER); // No. of KV misses
#[cfg(feature = "kms")]
create_counter!(AWS_KMS_FAILURES, GLOBAL_METER); // No. of AWS KMS API failures
