use router_env::{counter_metric, global_meter, metrics_context};

metrics_context!(CONTEXT);
global_meter!(GLOBAL_METER, "ROUTER_API");

counter_metric!(INCOMING_DISPUTE_WEBHOOK_METRIC, GLOBAL_METER); // No. of incoming dispute webhooks
counter_metric!(
    INCOMING_DISPUTE_WEBHOOK_SIGNATURE_FAILURE_METRIC,
    GLOBAL_METER
); // No. of incoming dispute webhooks for which signature verification failed
counter_metric!(
    INCOMING_DISPUTE_WEBHOOK_VALIDATION_FAILURE_METRIC,
    GLOBAL_METER
); // No. of incoming dispute webhooks for which validation failed
counter_metric!(INCOMING_DISPUTE_WEBHOOK_NEW_RECORD_METRIC, GLOBAL_METER); // No. of incoming dispute webhooks for which new record is created in our db
counter_metric!(INCOMING_DISPUTE_WEBHOOK_UPDATE_RECORD_METRIC, GLOBAL_METER); // No. of incoming dispute webhooks for which we have updated the details to existing record
counter_metric!(
    INCOMING_DISPUTE_WEBHOOK_MERCHANT_NOTIFIED_METRIC,
    GLOBAL_METER
); // No. of incoming dispute webhooks which are notified to merchant
