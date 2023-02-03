use once_cell::sync::Lazy;
pub use router_env::opentelemetry::KeyValue;
use router_env::opentelemetry::{
    global,
    metrics::{Counter, Meter, ObservableGauge},
    Context,
};

pub(crate) static CONTEXT: Lazy<Context> = Lazy::new(Context::current);
static DRAINER_METER: Lazy<Meter> = Lazy::new(|| global::meter("DRAINER"));

// macro_rules! create_counter {
//     ($name:ident, $meter:ident,) => {
//         pub(crate) static $name: Lazy<Counter<u64>> =
//             Lazy::new(|| $meter.u64_counter(stringify!($name)).init());
//     };
// }

// Time in (ms) milliseconds
pub(crate) static QUERY_EXECUTION_TIME: Lazy<ObservableGauge<f64>> = Lazy::new(|| {
    DRAINER_METER
        .f64_observable_gauge("QUERY_EXECUTION_TIME")
        .init()
});

pub(crate) static JOBS_PICKED_PER_STREAM: Lazy<ObservableGauge<u64>> = Lazy::new(|| {
    DRAINER_METER
        .u64_observable_gauge("JOBS_PICKED_PER_CYCLE")
        .init()
});

pub(crate) static CYCLES_COMPLETED_SUCCESSFULLY: Lazy<Counter<u64>> = Lazy::new(|| {
    DRAINER_METER
        .u64_counter("CYCLES_COMPLETED_SUCCESSFULLY")
        .init()
});

pub(crate) static CYCLES_COMPLETED_UNSUCCESSFULLY: Lazy<Counter<u64>> = Lazy::new(|| {
    DRAINER_METER
        .u64_counter("CYCLES_COMPLETED_UNSUCCESSFULLY")
        .init()
});

pub(crate) static ERRORS_WHILE_QUERY_EXECUTION: Lazy<Counter<u64>> = Lazy::new(|| {
    DRAINER_METER
        .u64_counter("ERRORS_WHILE_QUERY_EXECUTION")
        .init()
});

pub(crate) static SUCCESSFUL_QUERY_EXECUTION: Lazy<Counter<u64>> = Lazy::new(|| {
    DRAINER_METER
        .u64_counter("SUCCESSFUL_QUERY_EXECUTION")
        .init()
});

// Time in (ms) milliseconds
pub(crate) static REDIS_STREAM_READ_TIME: Lazy<ObservableGauge<f64>> = Lazy::new(|| {
    DRAINER_METER
        .f64_observable_gauge("REDIS_STREAM_READ_TIME")
        .init()
});

// Time in (ms) milliseconds
pub(crate) static REDIS_STREAM_TRIM_TIME: Lazy<ObservableGauge<f64>> = Lazy::new(|| {
    DRAINER_METER
        .f64_observable_gauge("REDIS_STREAM_TRIM_TIME")
        .init()
});

pub(crate) static _SHUTDOWN_SIGNAL_RECEIVED: Lazy<Counter<u64>> =
    Lazy::new(|| DRAINER_METER.u64_counter("SHUTDOWN_SIGNAL_RECEIVED").init());

pub(crate) static _SUCCESSFUL_SHUTDOWN: Lazy<Counter<u64>> =
    Lazy::new(|| DRAINER_METER.u64_counter("SUCCESSFUL_SHUTDOWN").init());

// Time in (ms) milliseconds
pub(crate) static _CLEANUP_TIME: Lazy<ObservableGauge<f64>> =
    Lazy::new(|| DRAINER_METER.f64_observable_gauge("CLEANUP_TIME").init());
