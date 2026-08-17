use std::{
    future::Future,
    io,
    sync::{Arc, LazyLock, Mutex},
};

use router_env::tracing::Instrument;
use tracing_subscriber::layer::SubscriberExt;

static CAPTURE_LOCK: LazyLock<tokio::sync::Mutex<()>> =
    LazyLock::new(|| tokio::sync::Mutex::new(()));
static FIXTURE: LazyLock<CaptureFixture> = LazyLock::new(CaptureFixture::install);

#[derive(Clone)]
struct SharedRecordSink(Arc<Mutex<Vec<deja::DejaRecord>>>);

impl deja::RecordSink<deja::DejaRecord> for SharedRecordSink {
    fn write_batch(&mut self, records: &[deja::DejaRecord]) -> io::Result<()> {
        self.0
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .extend_from_slice(records);
        Ok(())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

struct CaptureFixture {
    hook: Arc<deja::RecordingHook>,
    records: Arc<Mutex<Vec<deja::DejaRecord>>>,
}

impl CaptureFixture {
    fn install() -> Self {
        let records = Arc::new(Mutex::new(Vec::new()));
        let hook = Arc::new(deja::RecordingHook::with_sink(
            SharedRecordSink(Arc::clone(&records)),
            "router-unit-tests".to_owned(),
            deja::WriterConfig::default(),
        ));

        deja::set_global_runtime_hook(Some(deja::RuntimeHook::Recording(Arc::clone(&hook))))
            .expect("install the router unit-test Deja recording hook once");

        let graph_sink: Arc<dyn deja::GraphNodeSink> = hook.clone();
        let subscriber = tracing_subscriber::registry()
            .with(deja::ExecutionGraphLayer::new(graph_sink))
            .with(deja::DejaCorrelationLayer::new());
        router_env::tracing::subscriber::set_global_default(subscriber)
            .expect("install the router unit-test Deja tracing subscriber once");

        Self { hook, records }
    }

    fn record_count(&self) -> usize {
        self.records
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .len()
    }

    fn flush_records_since(
        &self,
        first_new_record: usize,
        correlation_id: &str,
    ) -> Vec<deja::DejaRecord> {
        self.hook
            .flush()
            .expect("flush router unit-test Deja records");

        self.records
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())[first_new_record..]
            .iter()
            .filter(|record| record_correlation(record) == Some(correlation_id))
            .cloned()
            .collect()
    }
}

struct RecordingDecision<'a>(&'a str);

impl Drop for RecordingDecision<'_> {
    fn drop(&mut self) {
        deja::clear_recording_decision(self.0);
    }
}

/// Run `future` in an isolated recording scope and return the records it emitted.
///
/// Capture sessions are serialized because Deja's runtime hook and tracing subscriber
/// are process-global and can only be installed once. A caller waiting for detached
/// work inside `future` therefore sees that child's records after the final flush.
pub(crate) async fn capture<T>(
    correlation_id: &str,
    future: impl Future<Output = T>,
) -> (T, Vec<deja::DejaRecord>) {
    let _capture_guard = CAPTURE_LOCK.lock().await;
    let fixture = &*FIXTURE;
    let first_new_record = fixture.record_count();

    deja::set_recording_decision(correlation_id, true);
    let decision = RecordingDecision(correlation_id);
    let span = router_env::tracing::info_span!(
        "deja_router_unit_test_capture",
        request_id = correlation_id
    );
    let output = future.instrument(span).await;
    drop(decision);

    let records = fixture.flush_records_since(first_new_record, correlation_id);

    (output, records)
}

/// Run `request_future` in a recording request scope, tear that scope down, and
/// then run `after_request_fn` outside it while retaining the same capture session.
///
/// This distinguishes work performed during the simulated request from detached
/// work that is deliberately released only after the request span and recording
/// decision have been dropped.
pub(crate) async fn capture_after_request<T, U, F, Fut>(
    correlation_id: &str,
    request_future: impl Future<Output = T>,
    after_request_fn: F,
) -> (U, Vec<deja::DejaRecord>)
where
    F: FnOnce(T) -> Fut,
    Fut: Future<Output = U>,
{
    let _capture_guard = CAPTURE_LOCK.lock().await;
    let fixture = &*FIXTURE;
    let first_new_record = fixture.record_count();

    deja::set_recording_decision(correlation_id, true);
    let request_output = {
        let decision = RecordingDecision(correlation_id);
        let request_output = {
            let span = router_env::tracing::info_span!(
                "deja_router_unit_test_capture",
                request_id = correlation_id
            );
            let instrumented_request = request_future.instrument(span);
            instrumented_request.await
        };
        drop(decision);
        request_output
    };

    let output = after_request_fn(request_output).await;

    let records = fixture.flush_records_since(first_new_record, correlation_id);

    (output, records)
}

fn record_correlation(record: &deja::DejaRecord) -> Option<&str> {
    match record {
        deja::DejaRecord::BoundaryEvent(event) => event.correlation_id.as_deref(),
        deja::DejaRecord::GraphNode(node) => node.correlation_id.as_deref(),
        deja::DejaRecord::Observed(call) => call.correlation_id.as_deref(),
    }
}
