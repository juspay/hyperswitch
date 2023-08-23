pub use router_env::opentelemetry::KeyValue;
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
counter_metric!(
    ACCEPT_DISPUTE_STATUS_VALIDATION_FAILURE_METRIC,
    GLOBAL_METER
); //No. of status validation failures while accpeting a dispute
counter_metric!(
    EVIDENCE_SUBMISSION_DISPUTE_STATUS_VALIDATION_FAILURE_METRIC,
    GLOBAL_METER
); //No. of status validation failures while submitting evidence for a dispute
   //No. of status validation failures while attaching evidence for a dispute
counter_metric!(
    ATTACH_EVIDENCE_DISPUTE_STATUS_VALIDATION_FAILURE_METRIC,
    GLOBAL_METER
);

counter_metric!(WEBHOOK_INCOMING_COUNT, GLOBAL_METER);
counter_metric!(WEBHOOK_INCOMING_FILTERED_COUNT, GLOBAL_METER);
counter_metric!(WEBHOOK_SOURCE_VERIFIED_COUNT, GLOBAL_METER);
counter_metric!(WEBHOOK_OUTGOING_COUNT, GLOBAL_METER);
counter_metric!(WEBHOOK_OUTGOING_RECEIVED_COUNT, GLOBAL_METER);
counter_metric!(WEBHOOK_OUTGOING_NOT_RECEIVED_COUNT, GLOBAL_METER);
counter_metric!(WEBHOOK_PAYMENT_NOT_FOUND, GLOBAL_METER);
counter_metric!(
    WEBHOOK_EVENT_TYPE_IDENTIFICATION_FAILURE_COUNT,
    GLOBAL_METER
);
