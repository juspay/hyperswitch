use std::{
    sync::atomic::{AtomicUsize, Ordering},
    time,
};

use super::*;

/// Satisfies `get_or_populate_in_memory`'s `E: From<StorageError>` bound for these tests' `E = String`.
impl From<StorageError> for String {
    fn from(err: StorageError) -> Self {
        err.to_string()
    }
}

/// Long enough for normal coalescing and for a queued caller to proceed after a timeout.
const TEST_POPULATE_TIMEOUT: Duration = Duration::from_millis(500);

fn test_key(key: &str) -> CacheKey {
    CacheKey {
        key: key.to_string(),
        prefix: "prefix".to_string(),
    }
}

/// A populate that counts its own invocations, so a test can assert how many callers
/// actually reached the backend.
fn counting_populate(
    calls: &Arc<AtomicUsize>,
    delay: Duration,
    value: impl Into<String>,
) -> futures::future::BoxFuture<'static, Result<String, String>> {
    let calls = Arc::clone(calls);
    let value = value.into();
    Box::pin(async move {
        calls.fetch_add(1, Ordering::SeqCst);
        tokio::time::sleep(delay).await;
        Ok(value)
    })
}

/// A populate that always fails, counting its own invocations the same way.
fn failing_populate(
    calls: &Arc<AtomicUsize>,
    delay: Duration,
    message: impl Into<String>,
) -> futures::future::BoxFuture<'static, Result<String, String>> {
    let calls = Arc::clone(calls);
    let message = message.into();
    Box::pin(async move {
        calls.fetch_add(1, Ordering::SeqCst);
        tokio::time::sleep(delay).await;
        Err(message)
    })
}

#[tokio::test]
async fn concurrent_misses_for_one_key_populate_once() {
    let cache = Arc::new(Cache::new("test", 1800, 1800, None, TEST_POPULATE_TIMEOUT));
    let calls = Arc::new(AtomicUsize::new(0));

    let readers = (0..20).map(|_| {
        let cache = Arc::clone(&cache);
        let populate = counting_populate(&calls, Duration::from_millis(50), "val");
        tokio::spawn(async move {
            cache
                .get_or_populate_in_memory(test_key("key"), populate)
                .await
        })
    });

    for result in futures::future::join_all(readers).await {
        assert_eq!(result.unwrap(), Ok("val".to_string()));
    }
    assert_eq!(calls.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn a_reader_arriving_during_a_populate_waits_and_hits() {
    let cache = Arc::new(Cache::new("test", 1800, 1800, None, TEST_POPULATE_TIMEOUT));
    let populating_calls = Arc::new(AtomicUsize::new(0));
    let reader_calls = Arc::new(AtomicUsize::new(0));

    let populating = {
        let cache = Arc::clone(&cache);
        let populate = counting_populate(&populating_calls, Duration::from_millis(100), "val");
        tokio::spawn(async move {
            cache
                .get_or_populate_in_memory(test_key("key"), populate)
                .await
        })
    };

    // Let the first caller take the write side before the second one arrives.
    tokio::time::sleep(Duration::from_millis(20)).await;

    let reader = {
        let cache = Arc::clone(&cache);
        let populate = counting_populate(&reader_calls, Duration::ZERO, "other");
        tokio::spawn(async move {
            cache
                .get_or_populate_in_memory(test_key("key"), populate)
                .await
        })
    };

    assert_eq!(populating.await.unwrap(), Ok("val".to_string()));
    assert_eq!(reader.await.unwrap(), Ok("val".to_string()));
    assert_eq!(populating_calls.load(Ordering::SeqCst), 1);
    // The reader never reached the backend — it read what the populate wrote.
    assert_eq!(reader_calls.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn reads_of_a_populated_key_do_not_block_each_other() {
    let cache = Arc::new(Cache::new("test", 1800, 1800, None, TEST_POPULATE_TIMEOUT));
    cache.push(test_key("key"), "val".to_string()).await;
    let calls = Arc::new(AtomicUsize::new(0));

    // Far more concurrent readers than the runtime has worker threads: if the read side
    // serialized them, this could not finish well inside the wait budget.
    let started = time::Instant::now();
    let readers = (0..200).map(|_| {
        let cache = Arc::clone(&cache);
        let populate = counting_populate(&calls, Duration::ZERO, "other");
        tokio::spawn(async move {
            cache
                .get_or_populate_in_memory(test_key("key"), populate)
                .await
        })
    });

    for result in futures::future::join_all(readers).await {
        assert_eq!(result.unwrap(), Ok("val".to_string()));
    }
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    assert!(started.elapsed() < TEST_POPULATE_TIMEOUT);
}

#[tokio::test]
async fn a_capped_populate_finishes_while_a_queued_caller_proceeds_independently() {
    let cache = Arc::new(Cache::new(
        "test",
        1800,
        1800,
        None,
        Duration::from_millis(50),
    ));
    let slow_calls = Arc::new(AtomicUsize::new(0));
    let queued_calls = Arc::new(AtomicUsize::new(0));
    let (started_tx, started_rx) = tokio::sync::oneshot::channel();
    let (finish_tx, finish_rx) = tokio::sync::oneshot::channel();

    let slow = {
        let cache = Arc::clone(&cache);
        let calls = Arc::clone(&slow_calls);
        tokio::spawn(async move {
            cache
                .get_or_populate_in_memory(test_key("key"), async move {
                    calls.fetch_add(1, Ordering::SeqCst);
                    started_tx.send(()).unwrap();
                    finish_rx.await.unwrap();
                    Ok::<_, String>("slow".to_string())
                })
                .await
        })
    };

    // The first caller holds the key's lock, but cannot complete until we release it.
    started_rx.await.unwrap();

    let queued = cache.get_or_populate_in_memory(
        test_key("key"),
        counting_populate(&queued_calls, Duration::ZERO, "own"),
    );

    // The queued caller must finish while the slow future is still pending. Otherwise
    // the lock was not released at the cap and this outer timeout will fail.
    assert_eq!(
        tokio::time::timeout(TEST_POPULATE_TIMEOUT, queued)
            .await
            .expect("queued caller remained blocked after the populate timeout"),
        Ok("own".to_string())
    );
    assert!(!slow.is_finished());
    assert_eq!(queued_calls.load(Ordering::SeqCst), 1);

    // Resume the same partially polled future, returning its own result without
    // overwriting the entry that the queued caller has since populated.
    finish_tx.send(()).unwrap();
    assert_eq!(slow.await.unwrap(), Ok("slow".to_string()));
    assert_eq!(slow_calls.load(Ordering::SeqCst), 1);
    assert_eq!(
        cache.get_val::<String>(test_key("key")).await,
        Some("own".to_string())
    );
}

#[tokio::test]
async fn a_capped_populate_returns_its_value_without_a_late_insert() {
    let cache = Cache::new("test", 1800, 1800, None, Duration::ZERO);
    let calls = Arc::new(AtomicUsize::new(0));

    let result = cache
        .get_or_populate_in_memory(
            test_key("key"),
            counting_populate(&calls, Duration::from_millis(20), "slow"),
        )
        .await;

    assert_eq!(result, Ok("slow".to_string()));
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert_eq!(cache.get_val::<String>(test_key("key")).await, None);
}

#[tokio::test]
async fn a_capped_populate_returns_its_eventual_backend_error() {
    let cache = Cache::new("test", 1800, 1800, None, Duration::ZERO);
    let calls = Arc::new(AtomicUsize::new(0));

    let result = cache
        .get_or_populate_in_memory(
            test_key("key"),
            failing_populate(&calls, Duration::from_millis(20), "backend failed"),
        )
        .await;

    assert_eq!(result, Err("backend failed".to_string()));
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert_eq!(cache.get_val::<String>(test_key("key")).await, None);
}

#[tokio::test]
async fn a_populate_failure_fails_only_that_caller_not_a_queued_one() {
    let cache = Arc::new(Cache::new("test", 1800, 1800, None, TEST_POPULATE_TIMEOUT));
    let failing_calls = Arc::new(AtomicUsize::new(0));
    let queued_calls = Arc::new(AtomicUsize::new(0));

    let failing = {
        let cache = Arc::clone(&cache);
        // Delayed, so the second caller below arrives while this is still the active
        // turn, not after it has already finished and freed the key.
        let populate = failing_populate(&failing_calls, Duration::from_millis(100), "boom");
        tokio::spawn(async move {
            cache
                .get_or_populate_in_memory(test_key("key"), populate)
                .await
        })
    };

    tokio::time::sleep(Duration::from_millis(20)).await;

    let queued_started = time::Instant::now();
    let queued = {
        let cache = Arc::clone(&cache);
        let populate = counting_populate(&queued_calls, Duration::ZERO, "own");
        tokio::spawn(async move {
            cache
                .get_or_populate_in_memory(test_key("key"), populate)
                .await
        })
    };

    assert_eq!(failing.await.unwrap(), Err("boom".to_string()));
    assert_eq!(failing_calls.load(Ordering::SeqCst), 1);

    // Proves real queuing rather than lucky scheduling: the second caller's own populate
    // has no delay, so an uncontended run would resolve in well under 50ms.
    assert_eq!(queued.await.unwrap(), Ok("own".to_string()));
    assert!(queued_started.elapsed() >= Duration::from_millis(50));
    assert_eq!(queued_calls.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn distinct_keys_never_block_each_other() {
    let cache = Arc::new(Cache::new("test", 1800, 1800, None, TEST_POPULATE_TIMEOUT));
    let calls = Arc::new(AtomicUsize::new(0));

    let populates = (0..8).map(|index| {
        let cache = Arc::clone(&cache);
        let populate = counting_populate(&calls, Duration::from_millis(50), format!("val{index}"));
        tokio::spawn(async move {
            cache
                .get_or_populate_in_memory(test_key(&format!("key{index}")), populate)
                .await
        })
    });

    for (index, result) in futures::future::join_all(populates)
        .await
        .into_iter()
        .enumerate()
    {
        assert_eq!(result.unwrap(), Ok(format!("val{index}")));
    }
    // Every key is distinct, so every one of them populates.
    assert_eq!(calls.load(Ordering::SeqCst), 8);
}

/// The `[cache]` documentation in `config/*.toml` promises this syntax.
#[test]
fn the_documented_toml_syntax_deserializes() {
    let config: CacheConfig = config::Config::builder()
        .add_source(config::File::from_str(
            r#"
            invalidation_channel = "channel_from_toml"

            [accounts]
            ttl_in_secs = 120
            max_entries = 500000

            [config]
            max_entries = 0
            "#,
            config::FileFormat::Toml,
        ))
        .build()
        .expect("failed to build cache configuration")
        .try_deserialize()
        .expect("failed to deserialize cache configuration from TOML");

    assert_eq!(
        config.invalidation_channel.as_deref(),
        Some("channel_from_toml")
    );
    assert_eq!(config.accounts.time_to_live(), 120);
    assert_eq!(config.accounts.max_entries, Some(500_000));
    assert_eq!(config.config.max_entries, Some(0));
    // Untouched caches keep their own defaults.
    assert_eq!(config.routing.max_entries, None);
    assert_eq!(config.routing.time_to_live(), DEFAULT_CACHE_TTL);
}

/// Pinned against the same `Environment` source `Settings::with_config_path` builds.
///
/// The registered list-parse keys are load-bearing: without any of them `list_separator`
/// applies to every key and silently turns plain string values into single-element arrays,
/// which no string field will accept.
#[test]
fn the_documented_env_vars_deserialize() {
    let source: std::collections::HashMap<String, String> = [
        ("ROUTER__ACCOUNTS__MAX_ENTRIES", "500000"),
        ("ROUTER__CONFIG__MAX_ENTRIES", "0"),
        ("ROUTER__CGRAPH__TTL_IN_SECS", "120"),
        ("ROUTER__INVALIDATION_CHANNEL", "channel_from_env"),
    ]
    .into_iter()
    .map(|(key, value)| (key.to_string(), value.to_string()))
    .collect();

    let config: CacheConfig = config::Config::builder()
        .add_source(
            config::Environment::with_prefix("ROUTER")
                .try_parsing(true)
                .separator("__")
                .list_separator(",")
                .with_list_parse_key("log.telemetry.route_to_trace")
                .with_list_parse_key("redis.cluster_urls")
                .source(Some(source)),
        )
        .build()
        .expect("failed to build cache configuration")
        .try_deserialize()
        .expect("failed to deserialize cache configuration from the environment");

    assert_eq!(config.accounts.max_entries, Some(500_000));
    assert_eq!(config.config.max_entries, Some(0));
    assert_eq!(config.cgraph.time_to_live(), 120);
    assert_eq!(
        config.invalidation_channel.as_deref(),
        Some("channel_from_env")
    );
}

#[test]
fn redis_is_held_past_the_in_memory_lifetime() {
    let ttl_in_secs = 1800;
    let cache = Cache::new("test", ttl_in_secs, 600, None, Duration::from_secs(5));

    let redis_ttl = cache
        .redis_ttl_for("merchant_1")
        .expect("a cache built with a ttl reports one");

    // Doubled, then jittered upward: 3600s plus up to a tenth of it.
    assert!(
        (3600..=3960).contains(&redis_ttl),
        "redis ttl {redis_ttl} outside the jittered band"
    );

    // The property the multiple exists for, stated where retuning either number breaks it.
    let ttl_in_secs = i64::try_from(ttl_in_secs).expect("test ttl fits an i64");
    assert!(redis_ttl > ttl_in_secs);
}

#[test]
fn jitter_stays_within_the_upper_decile() {
    let ttl = 300;
    for i in 0..1000 {
        let jittered = Cache::jittered_ttl(&format!("merchant_{i}"), ttl);
        assert!(
            (ttl..=ttl + ttl * i64::from(REDIS_TTL_JITTER_MAX_PERCENT) / 100).contains(&jittered),
            "ttl {jittered} out of range for merchant_{i}"
        );
    }
}

#[test]
fn jitter_spreads_keys_and_repeats_for_one_key() {
    let ttl = 300;
    let jittered = (0..1000)
        .map(|i| Cache::jittered_ttl(&format!("merchant_{i}"), ttl))
        .collect::<std::collections::HashSet<_>>();

    // A 300s ttl offers 31 distinct expiries; 1000 keys should reach most of them.
    assert!(jittered.len() > 25, "keys bunched onto {jittered:?}");

    // Same key, same ttl — this is what keeps a deja replay byte-identical.
    assert_eq!(
        Cache::jittered_ttl("merchant_1", ttl),
        Cache::jittered_ttl("merchant_1", ttl)
    );
}

#[test]
fn ttl_too_small_or_negative_is_left_alone() {
    for ttl in [-1, 0, 1, 9] {
        assert_eq!(Cache::jittered_ttl("key", ttl), ttl);
    }
}

#[tokio::test]
async fn construct_and_get_cache() {
    let cache = Cache::new("test", 1800, 1800, None, TEST_POPULATE_TIMEOUT);
    cache
        .push(
            CacheKey {
                key: "key".to_string(),
                prefix: "prefix".to_string(),
            },
            "val".to_string(),
        )
        .await;
    assert_eq!(
        cache
            .get_val::<String>(CacheKey {
                key: "key".to_string(),
                prefix: "prefix".to_string()
            })
            .await,
        Some(String::from("val"))
    );
}

#[tokio::test]
async fn eviction_on_size_test() {
    let cache = Cache::new("test", 2, 2, Some(0), TEST_POPULATE_TIMEOUT);
    cache
        .push(
            CacheKey {
                key: "key".to_string(),
                prefix: "prefix".to_string(),
            },
            "val".to_string(),
        )
        .await;
    assert_eq!(
        cache
            .get_val::<String>(CacheKey {
                key: "key".to_string(),
                prefix: "prefix".to_string()
            })
            .await,
        None
    );
}

#[tokio::test]
async fn invalidate_cache_for_key() {
    let cache = Cache::new("test", 1800, 1800, None, TEST_POPULATE_TIMEOUT);
    cache
        .push(
            CacheKey {
                key: "key".to_string(),
                prefix: "prefix".to_string(),
            },
            "val".to_string(),
        )
        .await;

    cache
        .remove(CacheKey {
            key: "key".to_string(),
            prefix: "prefix".to_string(),
        })
        .await;

    assert_eq!(
        cache
            .get_val::<String>(CacheKey {
                key: "key".to_string(),
                prefix: "prefix".to_string()
            })
            .await,
        None
    );
}

#[tokio::test]
async fn eviction_on_time_test() {
    let cache = Cache::new("test", 2, 2, None, TEST_POPULATE_TIMEOUT);
    cache
        .push(
            CacheKey {
                key: "key".to_string(),
                prefix: "prefix".to_string(),
            },
            "val".to_string(),
        )
        .await;
    tokio::time::sleep(Duration::from_secs(3)).await;
    assert_eq!(
        cache
            .get_val::<String>(CacheKey {
                key: "key".to_string(),
                prefix: "prefix".to_string()
            })
            .await,
        None
    );
}
