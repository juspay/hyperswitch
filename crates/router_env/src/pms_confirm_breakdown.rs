//! Request-scoped latency accounting for the payment-method-session Confirm route.
//!
//! The collector is installed only by that route. Shared database, Redis, encryption,
//! superposition, and vault code can therefore record timings without emitting extra
//! logs or affecting the observability cardinality of other API flows.

use std::{
    future::Future,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
    time::Instant,
};

use serde::Serialize;

tokio::task_local! {
    static COLLECTOR: Arc<Collector>;
}

/// A timed dependency category in the PMS Confirm request.
#[derive(Clone, Copy, Debug)]
pub enum Operation {
    VaultPrimaryFingerprint,
    VaultAuxiliaryFingerprint,
    VaultAddCard,
    /// End-to-end payment-method lookup by locker fingerprint. This diagnostic spans
    /// pool acquisition, SQL execution, and successful row conversion/decryption.
    /// It is deliberately excluded from `instrumented_ms` because those nested
    /// operations are already accounted for separately.
    PaymentMethodFingerprintLookup,
    EncryptionService,
    DatabaseRead,
    DatabaseWrite,
    DatabasePoolReadWait,
    DatabasePoolWriteWait,
    /// Pool acquisition for the customer read performed by PMS Confirm.
    DatabasePoolCustomerLookupWait,
    /// Pool acquisition for the payment-method lookup by locker fingerprint.
    DatabasePoolPaymentMethodFingerprintLookupWait,
    /// Pool acquisition for the payment-method insert.
    DatabasePoolPaymentMethodInsertWait,
    /// Pool acquisition for the payment-method update after vault insertion.
    DatabasePoolPaymentMethodUpdateWait,
    Superposition,
    RedisRead,
    RedisWrite,
    RedisOther,
}

#[derive(Debug, Default)]
struct Aggregate {
    count: AtomicU64,
    nanos: AtomicU64,
}

impl Aggregate {
    fn record(&self, nanos: u64) {
        self.count.fetch_add(1, Ordering::Relaxed);
        self.nanos.fetch_add(nanos, Ordering::Relaxed);
    }

    fn snapshot(&self) -> OperationSnapshot {
        OperationSnapshot {
            count: self.count.load(Ordering::Relaxed),
            ms: nanos_to_ms(self.nanos.load(Ordering::Relaxed)),
        }
    }
}

#[derive(Debug, Default)]
struct Collector {
    vault_primary_fingerprint: Aggregate,
    vault_auxiliary_fingerprint: Aggregate,
    vault_add_card: Aggregate,
    payment_method_fingerprint_lookup: Aggregate,
    encryption_service: Aggregate,
    database_read: Aggregate,
    database_write: Aggregate,
    database_pool_read_wait: Aggregate,
    database_pool_write_wait: Aggregate,
    database_pool_customer_lookup_wait: Aggregate,
    database_pool_payment_method_fingerprint_lookup_wait: Aggregate,
    database_pool_payment_method_insert_wait: Aggregate,
    database_pool_payment_method_update_wait: Aggregate,
    superposition: Aggregate,
    superposition_success_count: AtomicU64,
    superposition_fallback_count: AtomicU64,
    redis_read: Aggregate,
    redis_write: Aggregate,
    redis_other: Aggregate,
}

impl Collector {
    fn aggregate(&self, operation: Operation) -> &Aggregate {
        match operation {
            Operation::VaultPrimaryFingerprint => &self.vault_primary_fingerprint,
            Operation::VaultAuxiliaryFingerprint => &self.vault_auxiliary_fingerprint,
            Operation::VaultAddCard => &self.vault_add_card,
            Operation::PaymentMethodFingerprintLookup => &self.payment_method_fingerprint_lookup,
            Operation::EncryptionService => &self.encryption_service,
            Operation::DatabaseRead => &self.database_read,
            Operation::DatabaseWrite => &self.database_write,
            Operation::DatabasePoolReadWait => &self.database_pool_read_wait,
            Operation::DatabasePoolWriteWait => &self.database_pool_write_wait,
            Operation::DatabasePoolCustomerLookupWait => &self.database_pool_customer_lookup_wait,
            Operation::DatabasePoolPaymentMethodFingerprintLookupWait => {
                &self.database_pool_payment_method_fingerprint_lookup_wait
            }
            Operation::DatabasePoolPaymentMethodInsertWait => {
                &self.database_pool_payment_method_insert_wait
            }
            Operation::DatabasePoolPaymentMethodUpdateWait => {
                &self.database_pool_payment_method_update_wait
            }
            Operation::Superposition => &self.superposition,
            Operation::RedisRead => &self.redis_read,
            Operation::RedisWrite => &self.redis_write,
            Operation::RedisOther => &self.redis_other,
        }
    }

    fn snapshot(&self, total_ms: f64) -> Snapshot {
        let database_pool_read_wait = self.database_pool_read_wait.snapshot();
        let database_pool_write_wait = self.database_pool_write_wait.snapshot();
        let database_pool_customer_lookup_wait = self.database_pool_customer_lookup_wait.snapshot();
        let database_pool_payment_method_fingerprint_lookup_wait = self
            .database_pool_payment_method_fingerprint_lookup_wait
            .snapshot();
        let database_pool_payment_method_insert_wait =
            self.database_pool_payment_method_insert_wait.snapshot();
        let database_pool_payment_method_update_wait =
            self.database_pool_payment_method_update_wait.snapshot();
        let database_pool_other_read_wait = database_pool_read_wait.saturating_sub(
            database_pool_customer_lookup_wait
                .saturating_add(database_pool_payment_method_fingerprint_lookup_wait),
        );
        let database_pool_other_write_wait = database_pool_write_wait.saturating_sub(
            database_pool_payment_method_insert_wait
                .saturating_add(database_pool_payment_method_update_wait),
        );

        let snapshot = Snapshot {
            total_ms,
            vault_primary_fingerprint: self.vault_primary_fingerprint.snapshot(),
            vault_auxiliary_fingerprint: self.vault_auxiliary_fingerprint.snapshot(),
            vault_add_card: self.vault_add_card.snapshot(),
            payment_method_fingerprint_lookup: self.payment_method_fingerprint_lookup.snapshot(),
            encryption_service: self.encryption_service.snapshot(),
            database_read: self.database_read.snapshot(),
            database_write: self.database_write.snapshot(),
            database_pool_read_wait,
            database_pool_write_wait,
            database_pool_customer_lookup_wait,
            database_pool_payment_method_fingerprint_lookup_wait,
            database_pool_payment_method_insert_wait,
            database_pool_payment_method_update_wait,
            database_pool_other_read_wait,
            database_pool_other_write_wait,
            superposition: self.superposition.snapshot(),
            superposition_success_count: self.superposition_success_count.load(Ordering::Relaxed),
            superposition_fallback_count: self.superposition_fallback_count.load(Ordering::Relaxed),
            redis_read: self.redis_read.snapshot(),
            redis_write: self.redis_write.snapshot(),
            redis_other: self.redis_other.snapshot(),
            instrumented_ms: 0.0,
            unattributed_ms: 0.0,
            parallel_overlap_ms: 0.0,
        };

        let instrumented_ms = snapshot.vault_primary_fingerprint.ms
            + snapshot.vault_auxiliary_fingerprint.ms
            + snapshot.vault_add_card.ms
            + snapshot.encryption_service.ms
            + snapshot.database_read.ms
            + snapshot.database_write.ms
            + snapshot.database_pool_read_wait.ms
            + snapshot.database_pool_write_wait.ms
            + snapshot.superposition.ms
            + snapshot.redis_read.ms
            + snapshot.redis_write.ms
            + snapshot.redis_other.ms;

        Snapshot {
            instrumented_ms,
            unattributed_ms: (total_ms - instrumented_ms).max(0.0),
            parallel_overlap_ms: (instrumented_ms - total_ms).max(0.0),
            ..snapshot
        }
    }
}

/// Count and cumulative duration for one category.
#[derive(Clone, Copy, Debug, Default, Serialize)]
pub struct OperationSnapshot {
    pub count: u64,
    pub ms: f64,
}

impl OperationSnapshot {
    fn saturating_add(self, other: Self) -> Self {
        Self {
            count: self.count.saturating_add(other.count),
            ms: self.ms + other.ms,
        }
    }

    fn saturating_sub(self, other: Self) -> Self {
        Self {
            count: self.count.saturating_sub(other.count),
            ms: (self.ms - other.ms).max(0.0),
        }
    }
}

/// Fixed-field summary emitted once for a PMS Confirm request.
#[derive(Clone, Debug, Default, Serialize)]
pub struct Snapshot {
    pub total_ms: f64,
    pub vault_primary_fingerprint: OperationSnapshot,
    pub vault_auxiliary_fingerprint: OperationSnapshot,
    pub vault_add_card: OperationSnapshot,
    /// Diagnostic nested measurement; excluded from `instrumented_ms`.
    pub payment_method_fingerprint_lookup: OperationSnapshot,
    pub encryption_service: OperationSnapshot,
    pub database_read: OperationSnapshot,
    pub database_write: OperationSnapshot,
    pub database_pool_read_wait: OperationSnapshot,
    pub database_pool_write_wait: OperationSnapshot,
    /// Nested diagnostics for named pool acquisitions. These are excluded from
    /// `instrumented_ms` because the aggregate pool-wait fields already include them.
    pub database_pool_customer_lookup_wait: OperationSnapshot,
    pub database_pool_payment_method_fingerprint_lookup_wait: OperationSnapshot,
    pub database_pool_payment_method_insert_wait: OperationSnapshot,
    pub database_pool_payment_method_update_wait: OperationSnapshot,
    /// Aggregate pool waits which were not classified as one of the operations above.
    pub database_pool_other_read_wait: OperationSnapshot,
    pub database_pool_other_write_wait: OperationSnapshot,
    pub superposition: OperationSnapshot,
    pub superposition_success_count: u64,
    pub superposition_fallback_count: u64,
    pub redis_read: OperationSnapshot,
    pub redis_write: OperationSnapshot,
    pub redis_other: OperationSnapshot,
    pub instrumented_ms: f64,
    pub unattributed_ms: f64,
    pub parallel_overlap_ms: f64,
}

/// Run a future with PMS Confirm accounting enabled.
pub async fn scope<F>(future: F) -> (F::Output, Snapshot)
where
    F: Future,
{
    let collector = Arc::new(Collector::default());
    let started_at = Instant::now();
    let output = COLLECTOR.scope(Arc::clone(&collector), future).await;
    let snapshot = collector.snapshot(started_at.elapsed().as_secs_f64() * 1000.0);
    (output, snapshot)
}

/// Start a category timer. Outside the PMS Confirm scope this is a zero-cost `None`.
pub fn start(operation: Operation) -> Option<OperationTimer> {
    COLLECTOR
        .try_with(|collector| OperationTimer {
            collector: Arc::clone(collector),
            operation,
            started_at: Instant::now(),
        })
        .ok()
}

/// Record whether a Superposition lookup succeeded or used its database/default fallback.
pub fn record_superposition_result(success: bool) {
    let _ = COLLECTOR.try_with(|collector| {
        let counter = if success {
            &collector.superposition_success_count
        } else {
            &collector.superposition_fallback_count
        };
        counter.fetch_add(1, Ordering::Relaxed);
    });
}

/// RAII timer which records on every return path, including errors.
#[derive(Debug)]
pub struct OperationTimer {
    collector: Arc<Collector>,
    operation: Operation,
    started_at: Instant,
}

impl Drop for OperationTimer {
    fn drop(&mut self) {
        let nanos = u64::try_from(self.started_at.elapsed().as_nanos()).unwrap_or(u64::MAX);
        self.collector.aggregate(self.operation).record(nanos);
    }
}

fn nanos_to_ms(nanos: u64) -> f64 {
    std::time::Duration::from_nanos(nanos).as_secs_f64() * 1000.0
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;

    #[tokio::test]
    async fn collector_is_inactive_outside_scope() {
        assert!(start(Operation::DatabaseRead).is_none());
    }

    #[tokio::test]
    async fn aggregates_operations_and_superposition_results() {
        let (_, snapshot) = scope(async {
            {
                let _timer = start(Operation::DatabaseRead);
                tokio::time::sleep(Duration::from_millis(2)).await;
            }
            record_superposition_result(false);
        })
        .await;

        assert_eq!(snapshot.database_read.count, 1);
        assert!(snapshot.database_read.ms >= 1.0);
        assert_eq!(snapshot.superposition_fallback_count, 1);
        assert!(snapshot.total_ms >= snapshot.database_read.ms);
    }

    #[tokio::test]
    async fn nested_fingerprint_lookup_is_reported_but_not_double_counted() {
        let (_, snapshot) = scope(async {
            let _lookup = start(Operation::PaymentMethodFingerprintLookup);
            let _read = start(Operation::DatabaseRead);
            tokio::time::sleep(Duration::from_millis(2)).await;
        })
        .await;

        assert_eq!(snapshot.payment_method_fingerprint_lookup.count, 1);
        assert_eq!(snapshot.database_read.count, 1);
        assert!(snapshot.payment_method_fingerprint_lookup.ms >= snapshot.database_read.ms);
        assert_eq!(snapshot.instrumented_ms, snapshot.database_read.ms);
    }

    #[tokio::test]
    async fn named_pool_waits_are_reported_without_double_counting() {
        let (_, snapshot) = scope(async {
            let _aggregate_read = start(Operation::DatabasePoolReadWait);
            let _customer = start(Operation::DatabasePoolCustomerLookupWait);
            tokio::time::sleep(Duration::from_millis(2)).await;
        })
        .await;

        assert_eq!(snapshot.database_pool_read_wait.count, 1);
        assert_eq!(snapshot.database_pool_customer_lookup_wait.count, 1);
        assert_eq!(snapshot.database_pool_other_read_wait.count, 0);
        assert_eq!(
            snapshot.instrumented_ms,
            snapshot.database_pool_read_wait.ms
        );
    }

    #[tokio::test]
    async fn concurrent_scopes_are_isolated() {
        let (first, second) = tokio::join!(
            scope(async {
                let _timer = start(Operation::RedisRead);
            }),
            scope(async {
                let _timer = start(Operation::RedisWrite);
            })
        );

        assert_eq!(first.1.redis_read.count, 1);
        assert_eq!(first.1.redis_write.count, 0);
        assert_eq!(second.1.redis_read.count, 0);
        assert_eq!(second.1.redis_write.count, 1);
    }
}
