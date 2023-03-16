use router_env::{counter_metric, global_meter, metrics_context};

metrics_context!(CONTEXT);
global_meter!(GLOBAL_METER, "ROUTER_API");

counter_metric!(HEALTH_METRIC, GLOBAL_METER); // No. of health API hits
counter_metric!(KV_MISS, GLOBAL_METER); // No. of KV misses
#[cfg(feature = "kms")]
counter_metric!(AWS_KMS_FAILURES, GLOBAL_METER); // No. of AWS KMS API failures
