use once_cell::sync::Lazy;
use router_env::opentelemetry::{
    global,
    metrics::{Counter, Meter},
    Context,
};

pub static CONTEXT: Lazy<Context> = Lazy::new(Context::current);
static GLOBAL_METER: Lazy<Meter> = Lazy::new(|| global::meter("ROUTER_API"));

pub(crate) static HEALTH_METRIC: Lazy<Counter<u64>> =
    Lazy::new(|| GLOBAL_METER.u64_counter("HEALTH_API").init());

pub(crate) static KV_MISS: Lazy<Counter<u64>> =
    Lazy::new(|| GLOBAL_METER.u64_counter("KV_MISS").init());
