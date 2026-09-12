//! Successful resource lookups shared only within one request.
use std::{collections::HashMap, future::Future, sync::Arc};
use tokio::sync::{Mutex, OnceCell};

type Entry<T> = Arc<OnceCell<Option<T>>>;

#[derive(Clone)]
pub struct RequestCache<T> {
    entries: Arc<Mutex<HashMap<String, Entry<T>>>>,
}

impl<T> Default for RequestCache<T> {
    fn default() -> Self {
        Self {
            entries: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

impl<T: Clone> RequestCache<T> {
    /// Cache rows and successful misses. Errors remain retryable; concurrent
    /// same-key callers share one lookup. Callers receive cloned snapshots.
    pub async fn get_or_try_init<E, F, Fut>(&self, key: &str, load: F) -> Result<Option<T>, E>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = Result<Option<T>, E>>,
    {
        let entry = self
            .entries
            .lock()
            .await
            .entry(key.to_owned())
            .or_default()
            .clone();
        entry.get_or_try_init(load).await.cloned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[derive(Clone, Debug)]
    struct Row {
        card_iin: String,
        bank_code: Option<String>,
        card_network: Option<String>,
    }
    type CardInfoCache = RequestCache<Row>;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[tokio::test]
    async fn sparse_rows_are_cached_without_mutating_metadata() {
        let cache = CardInfoCache::default();
        let row = Row {
            card_iin: "123456".to_owned(),
            bank_code: None,
            card_network: None,
        };
        let mut first = cache
            .get_or_try_init("123456", || async { Ok::<_, ()>(Some(row)) })
            .await
            .unwrap()
            .unwrap();
        first.bank_code = Some("request-specific change".to_owned());
        let second = cache
            .get_or_try_init("123456", || async { Err::<Option<Row>, _>(()) })
            .await
            .unwrap()
            .unwrap();
        assert_eq!(first.card_iin, second.card_iin);
        assert!(second.bank_code.is_none());
        assert!(second.card_network.is_none());
    }

    #[tokio::test]
    async fn concurrent_clones_share_successful_missing_result() {
        let cache = CardInfoCache::default();
        let count = Arc::new(AtomicUsize::new(0));
        let mut tasks = Vec::new();
        for _ in 0..8 {
            let cache = cache.clone();
            let count = count.clone();
            tasks.push(tokio::spawn(async move {
                cache
                    .get_or_try_init("123456", || async {
                        count.fetch_add(1, Ordering::SeqCst);
                        tokio::task::yield_now().await;
                        Ok::<_, ()>(None)
                    })
                    .await
                    .unwrap()
            }));
        }
        for task in tasks {
            assert!(task.await.unwrap().is_none());
        }
        assert_eq!(count.load(Ordering::SeqCst), 1);
        cache
            .get_or_try_init("654321", || async {
                count.fetch_add(1, Ordering::SeqCst);
                Ok::<_, ()>(None)
            })
            .await
            .unwrap();
        assert_eq!(count.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn cancelled_initialization_can_be_retried() {
        let cache = CardInfoCache::default();
        let clone = cache.clone();
        let (started, ready) = tokio::sync::oneshot::channel();
        let task = tokio::spawn(async move {
            clone
                .get_or_try_init("123456", || async {
                    started.send(()).unwrap();
                    std::future::pending::<Result<Option<Row>, ()>>().await
                })
                .await
        });
        ready.await.unwrap();
        task.abort();
        assert!(task.await.unwrap_err().is_cancelled());
        assert!(cache
            .get_or_try_init("123456", || async { Ok::<_, ()>(None) })
            .await
            .unwrap()
            .is_none());
    }

    #[tokio::test]
    async fn failures_remain_retryable_and_new_requests_are_independent() {
        let cache = CardInfoCache::default();
        assert!(cache
            .get_or_try_init("123456", || async { Err::<Option<Row>, _>("transient") })
            .await
            .is_err());
        assert!(cache
            .get_or_try_init("123456", || async { Ok::<_, ()>(None) })
            .await
            .unwrap()
            .is_none());
        assert!(CardInfoCache::default()
            .get_or_try_init("123456", || async { Err::<Option<Row>, _>("fresh lookup") })
            .await
            .is_err());
    }
}
