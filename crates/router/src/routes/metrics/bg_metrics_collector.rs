use storage_impl::redis::cache;

const DEFAULT_BG_METRICS_COLLECTION_INTERVAL_IN_SECS: u64 = 15;

pub fn spawn_metrics_collector(metrics_collection_interval_in_secs: &Option<u64>) {
    let metrics_collection_interval = metrics_collection_interval_in_secs
        .unwrap_or(DEFAULT_BG_METRICS_COLLECTION_INTERVAL_IN_SECS);

    tokio::spawn(async move {
        loop {
            crate::gauge_metrics_for_imc!(
                CONFIG_CACHE,
                ACCOUNTS_CACHE,
                ROUTING_CACHE,
                CGRAPH_CACHE,
                PM_FILTERS_CGRAPH_CACHE
            );

            tokio::time::sleep(std::time::Duration::from_secs(metrics_collection_interval)).await
        }
    });
}

#[macro_export]
macro_rules! guaue_metrics_for_imc {
    ($($cache:ident),*) => {
        $(
            {
                cache::$cache.run_pending_tasks().await;

                super::CACHE_ENTRY_COUNT.observe(
                    &super::CONTEXT,
                    cache::$cache.get_entry_count(),
                    &[super::request::add_attributes(
                        "cache_type",
                        stringify!($cache),
                    )],
                );
            }
        )*
    };
}
