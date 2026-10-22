use std::sync::Arc;

use storage_impl::redis::cache::Caches;

const DEFAULT_BG_METRICS_COLLECTION_INTERVAL_IN_SECS: u16 = 15;

pub fn spawn_metrics_collector(
    metrics_collection_interval_in_secs: Option<u16>,
    caches: Arc<Caches>,
) {
    let metrics_collection_interval = metrics_collection_interval_in_secs
        .unwrap_or(DEFAULT_BG_METRICS_COLLECTION_INTERVAL_IN_SECS);

    tokio::spawn(async move {
        loop {
            for instance in caches.all() {
                instance.record_size_metrics().await
            }

            tokio::time::sleep(std::time::Duration::from_secs(
                metrics_collection_interval.into(),
            ))
            .await
        }
    });
}
