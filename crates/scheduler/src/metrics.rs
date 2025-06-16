use router_env::{counter_metric, global_meter, histogram_metric_f64};

global_meter!(PT_METER, "PROCESS_TRACKER");

histogram_metric_f64!(CONSUMER_OPS, PT_METER);

counter_metric!(PAYMENT_COUNT, PT_METER); // No. of payments created
counter_metric!(TASKS_PICKED_COUNT, PT_METER); // Tasks picked by
counter_metric!(BATCHES_CREATED, PT_METER); // Batches added to stream
counter_metric!(BATCHES_CONSUMED, PT_METER); // Batches consumed by consumer
counter_metric!(TASK_CONSUMED, PT_METER); // Tasks consumed by consumer
counter_metric!(TASK_PROCESSED, PT_METER); // Tasks completed processing
counter_metric!(TASK_FINISHED, PT_METER); // Tasks finished
counter_metric!(TASK_RETRIED, PT_METER); // Tasks added for retries
