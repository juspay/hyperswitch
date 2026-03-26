use std::sync::atomic::{AtomicU64, Ordering};

static CACHE_IN_MEM_HITS: AtomicU64 = AtomicU64::new(0);
static CACHE_REDIS_HITS: AtomicU64 = AtomicU64::new(0);
static CACHE_DB_FETCHES: AtomicU64 = AtomicU64::new(0);
static TOTAL_CACHE_OPS: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Default)]
pub struct RequestMetrics {
    pub cache_in_mem_hits: AtomicU64,
    pub cache_redis_hits: AtomicU64,
    pub cache_db_fetches: AtomicU64,
    pub total_cache_ops: AtomicU64,
}

impl RequestMetrics {
    pub fn new() -> Self {
        Self {
            cache_in_mem_hits: AtomicU64::new(0),
            cache_redis_hits: AtomicU64::new(0),
            cache_db_fetches: AtomicU64::new(0),
            total_cache_ops: AtomicU64::new(0),
        }
    }
}

#[inline]
pub fn increment_cache_hit() {
    CACHE_IN_MEM_HITS.fetch_add(1, Ordering::Relaxed);
    TOTAL_CACHE_OPS.fetch_add(1, Ordering::Relaxed);
}

#[inline]
pub fn increment_redis_hit() {
    CACHE_REDIS_HITS.fetch_add(1, Ordering::Relaxed);
    TOTAL_CACHE_OPS.fetch_add(1, Ordering::Relaxed);
}

#[inline]
pub fn increment_db_fetch() {
    CACHE_DB_FETCHES.fetch_add(1, Ordering::Relaxed);
    TOTAL_CACHE_OPS.fetch_add(1, Ordering::Relaxed);
}

pub fn get_request_metrics() -> Option<(u64, u64, u64, u64)> {
    Some((
        CACHE_IN_MEM_HITS.load(Ordering::Relaxed),
        CACHE_REDIS_HITS.load(Ordering::Relaxed),
        CACHE_DB_FETCHES.load(Ordering::Relaxed),
        TOTAL_CACHE_OPS.load(Ordering::Relaxed),
    ))
}
