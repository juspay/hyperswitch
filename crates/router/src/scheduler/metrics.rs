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

create_counter!(PAYMENT_COUNT, PT_METER); // No. of payments created
create_counter!(TASKS_ADDED_COUNT, PT_METER); // Tasks added to process tracker
create_counter!(TASKS_PICKED_COUNT, PT_METER); // Tasks picked by
create_counter!(BATCHES_CREATED, PT_METER); // Batches added to stream
create_counter!(BATCHES_CONSUMED, PT_METER); // Batches consumed by consumer
create_counter!(TASK_CONSUMED, PT_METER); // Tasks consumed by consumer
create_counter!(TASK_PROCESSED, PT_METER); // Tasks completed processing
create_counter!(TASK_FINISHED, PT_METER); // Tasks finished
create_counter!(TASK_RETRIED, PT_METER); // Tasks added for retries
