#![cfg(feature = "deja")]
//! A record-mode process with no sampler attached must record nothing.
//!
//! The ingress middleware asks the sampler whether to record, and the sampler is
//! optional. When it is absent there is no policy to consult, so the only two
//! answers available are "record everything" and "record nothing". Recording
//! everything makes a wiring mistake — a caller that built the application
//! without passing a sampler — indistinguishable from a deliberate record-all,
//! and produces a tape covering traffic no policy ever selected.
//!
//! So the absent-sampler arm skips, and this test pins that. It is the
//! complement of `deja_ingress_records`: same middleware, same real
//! `RecordingHook`, same drained body and flush barrier — the single difference
//! is that no sampler is wired in. That test asserts one `http_incoming` event
//! is recorded; this one asserts none is.
//!
//! It runs in its own integration-test binary so the process-global runtime hook
//! and tracing subscriber install exactly once, with no cross-test contention.

use std::sync::{Arc, Mutex};

use actix_web::{test, web, App, HttpResponse};
use router_env::request_id::RequestIdentifier;
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

#[actix_web::test]
async fn ingress_without_a_sampler_records_nothing() {
    tracing_subscriber::registry()
        .with(deja::DejaCorrelationLayer::new())
        .try_init()
        .expect("install correlation layer (own process)");

    // A real recording hook, so the process is genuinely in record mode: the
    // ingress predicate passes and the middleware reaches the sampler branch.
    // Only the sampler itself is missing.
    let records = Arc::new(Mutex::new(Vec::new()));
    let hook = Arc::new(deja::RecordingHook::with_sink(
        VecSink(records.clone()),
        "ingress-no-sampler-it".to_string(),
        deja::WriterConfig::default(),
    ));
    deja::set_global_runtime_hook(Some(deja::RuntimeHook::Recording(hook)))
        .expect("install record hook (own process)");

    // No `.with_recording_sampler(..)`: `recording_sampler` stays `None`.
    let app = test::init_service(
        App::new()
            .wrap(RequestIdentifier::new("x-request-id"))
            .route(
                "/payments",
                web::post().to(|| async { HttpResponse::Ok().body(r#"{"ok":true}"#) }),
            ),
    )
    .await;

    let request = test::TestRequest::post().uri("/payments").to_request();
    let response = test::call_service(&app, request).await;
    assert!(response.status().is_success(), "handler should return 200");

    // Drive the body to EOF: if the middleware had decided to record, this is
    // what would finalize the http_incoming event.
    let _body = test::read_body(response).await;

    // Flush blocks until the async writer has handed every queued record to the
    // sink, so an empty sink below means nothing was written — not that a write
    // is still in flight.
    deja::flush_global_runtime_hook().expect("flush recording hook");

    let recorded = records.lock().expect("sink lock");
    let http_incoming: Vec<_> = recorded
        .iter()
        .filter(|record| {
            matches!(
                record,
                deja::DejaRecord::BoundaryEvent(event) if event.boundary == "http_incoming"
            )
        })
        .collect();
    assert!(
        http_incoming.is_empty(),
        "a record-mode process with no sampler recorded {} http_incoming event(s); \
         with no sampling policy to consult it must record nothing (total records: {})",
        http_incoming.len(),
        recorded.len(),
    );
}
