use router_env::{counter_metric, global_meter, histogram_metric_f64};

global_meter!(GLOBAL_METER, "ROUTER_API");

counter_metric!(TASKS_ADDED_COUNT, GLOBAL_METER); // Tasks added to process tracker
counter_metric!(TASK_ADDITION_FAILURES_COUNT, GLOBAL_METER); // Failures in task addition to process tracker
counter_metric!(TASKS_RESET_COUNT, GLOBAL_METER); // Tasks reset in process tracker for requeue flow

counter_metric!(CREATED_TOKENIZED_CARD, GLOBAL_METER);
counter_metric!(DELETED_TOKENIZED_CARD, GLOBAL_METER);
counter_metric!(GET_TOKENIZED_CARD, GLOBAL_METER);
counter_metric!(TOKENIZED_DATA_COUNT, GLOBAL_METER); // Tokenized data added
counter_metric!(RETRIED_DELETE_DATA_COUNT, GLOBAL_METER); // Tokenized data retried

// Service Level
counter_metric!(CARD_LOCKER_FAILURES, GLOBAL_METER);
counter_metric!(CARD_LOCKER_SUCCESSFUL_RESPONSE, GLOBAL_METER);
counter_metric!(TEMP_LOCKER_FAILURES, GLOBAL_METER);
histogram_metric_f64!(CARD_ADD_TIME, GLOBAL_METER);
histogram_metric_f64!(CARD_GET_TIME, GLOBAL_METER);
histogram_metric_f64!(CARD_DELETE_TIME, GLOBAL_METER);

counter_metric!(STORED_TO_LOCKER, GLOBAL_METER);
counter_metric!(GET_FROM_LOCKER, GLOBAL_METER);
counter_metric!(DELETE_FROM_LOCKER, GLOBAL_METER);

// Network Tokenization metrics
histogram_metric_f64!(GENERATE_NETWORK_TOKEN_TIME, GLOBAL_METER);
histogram_metric_f64!(FETCH_NETWORK_TOKEN_TIME, GLOBAL_METER);
histogram_metric_f64!(DELETE_NETWORK_TOKEN_TIME, GLOBAL_METER);
histogram_metric_f64!(CHECK_NETWORK_TOKEN_STATUS_TIME, GLOBAL_METER);