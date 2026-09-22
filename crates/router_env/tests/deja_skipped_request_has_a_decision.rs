//! In record mode the middleware registers a decision for EVERY request, a
//! declined one included. The collections facade in `common_utils` depends on
//! that: it reads an absent decision as "no sampler is engaged" and derives a
//! request's hash keys from its correlation id, so a request the sampler
//! declined must reach it as `Skip`, never as nothing. Nothing else ties the
//! two files together, which is why the producer's side is asserted here.
#![cfg(feature = "deja")]
#![allow(clippy::panic, clippy::expect_used)]

use std::sync::Arc;

use actix_web::{test, web, App, HttpResponse};
use router_env::request_id::{
    RequestId, RequestIdentifier, RequestRecordingFacts, RequestRecordingSampler,
    RequestRecordingSamplerFuture,
};
use tracing_subscriber::prelude::*;

struct DecliningSampler;

#[derive(Clone)]
struct NullSink;

impl deja::RecordSink<deja::DejaRecord> for NullSink {
    fn write_batch(&mut self, _records: &[deja::DejaRecord]) -> std::io::Result<()> {
        Ok(())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl RequestRecordingSampler for DecliningSampler {
    fn should_record(&self, _facts: RequestRecordingFacts) -> RequestRecordingSamplerFuture<'_> {
        Box::pin(async { false })
    }
}

#[actix_web::test]
async fn a_declined_request_is_registered_as_skip_while_it_runs() {
    tracing_subscriber::registry()
        .with(deja::DejaCorrelationLayer::new())
        .try_init()
        .expect("install correlation layer (own process)");
    let hook = deja::RecordingHook::with_sink(
        NullSink,
        "skipped-request".to_string(),
        deja::WriterConfig::default(),
    );
    deja::set_global_runtime_hook(Some(deja::RuntimeHook::Recording(Arc::new(hook))))
        .expect("install record hook (own process)");

    let sampler: Arc<dyn RequestRecordingSampler> = Arc::new(DecliningSampler);
    let app = test::init_service(
        App::new()
            .wrap(RequestIdentifier::new("x-request-id").with_recording_sampler(sampler))
            .route(
                "/payments",
                web::post().to(|request_id: RequestId| async move {
                    // Read while the request is in flight: the entry is cleared
                    // when the request ends, so afterwards there is nothing to see.
                    match deja::recording_decision(request_id.as_str()) {
                        Some(deja::RecordDecision::Skip) => HttpResponse::Ok().finish(),
                        other => HttpResponse::InternalServerError().body(format!("{other:?}")),
                    }
                }),
            ),
    )
    .await;

    let response = test::call_service(
        &app,
        test::TestRequest::post().uri("/payments").to_request(),
    )
    .await;
    let status = response.status();
    let body = test::read_body(response).await;
    assert!(
        status.is_success(),
        "a declined request must be registered as Skip while it runs, found {}",
        String::from_utf8_lossy(&body)
    );
}
