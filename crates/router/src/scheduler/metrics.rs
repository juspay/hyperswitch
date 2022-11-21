use once_cell::sync::Lazy;
use router_env::opentelemetry::{
    global,
    metrics::{Counter, Meter, ValueRecorder},
};

static PT_METER: Lazy<Meter> = Lazy::new(|| global::meter("PROCESS_TRACKER"));

// Using ValueRecorder till https://bitbucket.org/juspay/orca/pull-requests/319
// Histogram available in opentelemetry:0.18
pub(crate) static CONSUMER_STATS: Lazy<ValueRecorder<f64>> =
    Lazy::new(|| PT_METER.f64_value_recorder("CONSUMER_OPS").init());

macro_rules! create_counter {
    ($name:ident, $meter:ident) => {
        pub(crate) static $name: Lazy<Counter<u64>> =
            Lazy::new(|| $meter.u64_counter(stringify!($name)).init());
    };
}

create_counter!(PAYMENT_COUNT, PT_METER);
create_counter!(JOB_COUNT, PT_METER);
create_counter!(BATCHES_CREATED, PT_METER);
create_counter!(BATCHES_CONSUMED, PT_METER);
create_counter!(TASK_CONSUMED, PT_METER);
