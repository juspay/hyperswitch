#![cfg(feature = "deja")]
//! End-to-end regression test for the ingress recording circular dependency.
//!
//! The per-boundary capture gate answers from the per-correlation sampling
//! decision, and the ingress middleware is what pushes that decision. So the
//! ingress predicate must never consult the per-request gate: at ingress no
//! decision exists yet, the predicate would read false, the sampler would never
//! be consulted, the decision would never be pushed, and every boundary would
//! skip — an empty tape, with nothing to indicate why.
//!
//! The ingress predicates therefore read `process_mode()`, which is boot-time
//! configuration and consults no per-request state. This test drives the real
//! actix middleware end to end and asserts the two effects that disappear if
//! that split is ever lost: the sampler is consulted (so a decision is pushed)
//! and exactly one `http_incoming` boundary event is recorded.
//!
//! It runs in its own integration-test binary so the process-global runtime hook
//! and tracing subscriber install exactly once, with no cross-test contention.

use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};

use actix_web::{test, web, App, HttpResponse};
use router_env::request_id::{
    RequestIdentifier, RequestRecordingFacts, RequestRecordingSampler,
    RequestRecordingSamplerFuture,
};
use tracing_subscriber::prelude::*;

/// Synchronous in-memory sink so the test can read back what the recorder wrote.
#[derive(Clone)]
struct VecSink(Arc<Mutex<Vec<deja::DejaRecord>>>);

impl deja::RecordSink<deja::DejaRecord> for VecSink {
    fn write_batch(&mut self, records: &[deja::DejaRecord]) -> std::io::Result<()> {
        let mut sink = self
            .0
            .lock()
            .map_err(|_| std::io::Error::other("sink lock poisoned"))?;
        sink.extend(records.iter().cloned());
        Ok(())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

/// A sampler that records whether it was consulted and always votes to record.
/// On the pre-fix code the ingress predicate never passes, so `should_record`
/// is never called — which is exactly what the `sampler_seen` flag catches.
struct InvocationRecordingSampler(Arc<AtomicBool>);

impl RequestRecordingSampler for InvocationRecordingSampler {
    fn should_record(&self, _facts: RequestRecordingFacts) -> RequestRecordingSamplerFuture<'_> {
        self.0.store(true, Ordering::SeqCst);
        Box::pin(async { true })
    }
}

#[actix_web::test]
async fn ingress_records_http_incoming_end_to_end() {
    // The middleware stamps `request_id` on its span; this layer mirrors it into
    // deja-context so the http_incoming boundary gate can resolve the pushed
    // decision (correlation → decision lookup). Own process, so install once.
    tracing_subscriber::registry()
        .with(deja::DejaCorrelationLayer::new())
        .try_init()
        .expect("install correlation layer (own process)");

    // A real RecordingHook (the one with the sampling gate the bug lives in),
    // wired to a readable sink and installed as the process runtime hook.
    let records = Arc::new(Mutex::new(Vec::new()));
    let hook = Arc::new(deja::RecordingHook::with_sink(
        VecSink(records.clone()),
        "ingress-it".to_string(),
        deja::WriterConfig::default(),
    ));
    deja::set_global_runtime_hook(Some(deja::RuntimeHook::Recording(hook)))
        .expect("install record hook (own process)");

    let sampler_seen = Arc::new(AtomicBool::new(false));
    let sampler: Arc<dyn RequestRecordingSampler> =
        Arc::new(InvocationRecordingSampler(sampler_seen.clone()));

    let app = test::init_service(
        App::new()
            .wrap(RequestIdentifier::new("x-request-id").with_recording_sampler(sampler))
            .route(
                "/payments",
                web::post().to(|| async { HttpResponse::Ok().body(r#"{"ok":true}"#) }),
            ),
    )
    .await;

    let request = test::TestRequest::post().uri("/payments").to_request();
    let response = test::call_service(&app, request).await;
    assert!(response.status().is_success(), "handler should return 200");

    // Drive the body to EOF so the RecordingBody finalizes the http_incoming event.
    let _body = test::read_body(response).await;

    // Synchronous drain barrier: flush blocks until the async writer has handed
    // every queued record to the sink.
    deja::flush_global_runtime_hook().expect("flush recording hook");

    // (a) The sampler was consulted → the ingress predicate passed → a decision
    //     was pushed. False on the pre-fix code (the circular dependency).
    assert!(
        sampler_seen.load(Ordering::SeqCst),
        "recording sampler was never consulted: the ingress predicate did not pass, \
         so no recording decision was ever pushed"
    );

    // (b) Exactly one http_incoming boundary event reached the sink.
    let recorded = records.lock().expect("sink lock");
    let http_incoming: Vec<_> = recorded
        .iter()
        .filter_map(|record| match record {
            deja::DejaRecord::BoundaryEvent(event) if event.boundary == "http_incoming" => {
                Some(event)
            }
            _ => None,
        })
        .collect();
    assert_eq!(
        http_incoming.len(),
        1,
        "expected exactly one recorded http_incoming event; got {} (total records: {})",
        http_incoming.len(),
        recorded.len(),
    );
}
