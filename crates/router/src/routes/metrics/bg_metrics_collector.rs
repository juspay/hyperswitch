use storage_impl::redis::cache;

const DEFAULT_BG_METRICS_COLLECTION_INTERVAL_IN_SECS: u16 = 15;

pub fn spawn_metrics_collector(metrics_collection_interval_in_secs: Option<u16>) {
    let metrics_collection_interval = metrics_collection_interval_in_secs
        .unwrap_or(DEFAULT_BG_METRICS_COLLECTION_INTERVAL_IN_SECS);

    let cache_instances = [
        &cache::CONFIG_CACHE,
        &cache::ACCOUNTS_CACHE,
        &cache::ROUTING_CACHE,
        &cache::CGRAPH_CACHE,
        &cache::PM_FILTERS_CGRAPH_CACHE,
        &cache::DECISION_MANAGER_CACHE,
        &cache::SURCHARGE_CACHE,
        &cache::SUCCESS_BASED_DYNAMIC_ALGORITHM_CACHE,
        &cache::CONTRACT_BASED_DYNAMIC_ALGORITHM_CACHE,
        &cache::ELIMINATION_BASED_DYNAMIC_ALGORITHM_CACHE,
    ];

    tokio::spawn(async move {
        loop {
            for instance in cache_instances {
                instance.record_entry_count_metric().await
            }

            tokio::time::sleep(std::time::Duration::from_secs(
                metrics_collection_interval.into(),
            ))
            .await
        }
    });
}
