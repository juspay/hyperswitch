use router_env::{counter_metric, global_meter, histogram_metric_f64};

global_meter!(GLOBAL_METER, "INJECTOR");

// Invocation metrics
counter_metric!(INJECTOR_INVOCATIONS_COUNT, GLOBAL_METER); // Total number of invocations
counter_metric!(INJECTOR_OUTGOING_CALLS_COUNT, GLOBAL_METER); // Total number of outgoing calls
counter_metric!(INJECTOR_SUCCESSFUL_TOKEN_REPLACEMENTS_COUNT, GLOBAL_METER); // Successful token replacements with status code dimensions
counter_metric!(INJECTOR_FAILED_TOKEN_REPLACEMENTS_COUNT, GLOBAL_METER); // Failed token replacements

// Performance metrics
histogram_metric_f64!(INJECTOR_REQUEST_TIME, GLOBAL_METER); // Time taken for complete injector operation
histogram_metric_f64!(INJECTOR_TOKEN_REPLACEMENT_TIME, GLOBAL_METER); // Time taken for token replacement operation

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_are_defined() {
        // This test ensures that all metrics are properly defined and accessible
        // The actual functionality will be tested through integration tests

        // Test that we can access the counters (this will compile-fail if metrics aren't properly defined)
        let _ = &INJECTOR_INVOCATIONS_COUNT;
        let _ = &INJECTOR_OUTGOING_CALLS_COUNT;
        let _ = &INJECTOR_SUCCESSFUL_TOKEN_REPLACEMENTS_COUNT;
        let _ = &INJECTOR_FAILED_TOKEN_REPLACEMENTS_COUNT;

        // Test that we can access the histograms
        let _ = &INJECTOR_REQUEST_TIME;
        let _ = &INJECTOR_TOKEN_REPLACEMENT_TIME;
    }
}
