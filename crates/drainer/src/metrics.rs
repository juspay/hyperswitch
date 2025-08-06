use router_env::{counter_metric, global_meter, histogram_metric_f64, histogram_metric_u64};

global_meter!(DRAINER_METER, "DRAINER");

counter_metric!(JOBS_PICKED_PER_STREAM, DRAINER_METER);
counter_metric!(CYCLES_COMPLETED_SUCCESSFULLY, DRAINER_METER);
counter_metric!(CYCLES_COMPLETED_UNSUCCESSFULLY, DRAINER_METER);
counter_metric!(ERRORS_WHILE_QUERY_EXECUTION, DRAINER_METER);
counter_metric!(SUCCESSFUL_QUERY_EXECUTION, DRAINER_METER);
counter_metric!(SHUTDOWN_SIGNAL_RECEIVED, DRAINER_METER);
counter_metric!(SUCCESSFUL_SHUTDOWN, DRAINER_METER);
counter_metric!(STREAM_EMPTY, DRAINER_METER);
counter_metric!(STREAM_PARSE_FAIL, DRAINER_METER);
counter_metric!(DRAINER_HEALTH, DRAINER_METER);

histogram_metric_f64!(QUERY_EXECUTION_TIME, DRAINER_METER); // Time in (ms) milliseconds
histogram_metric_f64!(REDIS_STREAM_READ_TIME, DRAINER_METER); // Time in (ms) milliseconds
histogram_metric_f64!(REDIS_STREAM_TRIM_TIME, DRAINER_METER); // Time in (ms) milliseconds
histogram_metric_f64!(CLEANUP_TIME, DRAINER_METER); // Time in (ms) milliseconds
histogram_metric_u64!(DRAINER_DELAY_SECONDS, DRAINER_METER); // Time in (s) seconds
histogram_metric_f64!(REDIS_STREAM_DEL_TIME, DRAINER_METER); // Time in (ms) milliseconds
