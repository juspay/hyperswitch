// TODO: Move this to router_env https://juspay.atlassian.net/browse/ORCA-345

use once_cell::sync::Lazy;
use router_env::opentelemetry::{
    global,
    metrics::{Counter, Meter},
};

static GLOBAL_METER: Lazy<Meter> = Lazy::new(|| global::meter("ROUTER_API"));

pub(crate) static HEALTH_METRIC: Lazy<Counter<u64>> =
    Lazy::new(|| GLOBAL_METER.u64_counter("HEALTH_API").init());
