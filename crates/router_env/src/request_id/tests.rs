use super::*;

/// Regression sentinel for the ingress circular-dependency bug.
///
/// The opt-in recording flip made `RecordingHook::mode()` return `Record`
/// only while a per-correlation decision is pushed. The ingress middleware
/// is what pushes that decision, so gating ingress on `mode()` was circular:
/// `mode()` is `Disabled` at ingress (no decision yet) → the predicate is
/// false → the sampler never runs → no decision is ever pushed → nothing is
/// ever recorded (the empty-tape bug).
///
/// The fix routes the ingress predicates through `process_mode()`, which
/// reflects the boot-time configuration and ignores the per-request gate.
/// This test pins the exact pivot at the type level, with no global install
/// and no async: a fresh recorder with no decision in scope must report
/// `process_mode() == Record` (ingress may proceed) while `mode() ==
/// Disabled` (the boundary gate still governs actual capture). On the old
/// code the first assertion is what the middleware needed and never got.
#[cfg(feature = "deja")]
#[test]
fn ingress_process_mode_is_record_before_any_decision_is_pushed() {
    struct NullSink;
    impl deja::RecordSink<deja::DejaRecord> for NullSink {
        fn write_batch(&mut self, _records: &[deja::DejaRecord]) -> std::io::Result<()> {
            Ok(())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    let hook = deja::RecordingHook::with_sink(
        NullSink,
        "ingress-sentinel".to_string(),
        deja::WriterConfig::default(),
    );

    // No per-correlation decision has been pushed for this thread.
    assert_eq!(
        deja::DejaHook::process_mode(&hook),
        deja::RuntimeMode::Record,
        "ingress predicate must see Record from process config alone, or the \
             middleware can never push the decision that flips the boundary gate"
    );
    assert_eq!(
        deja::DejaHook::capture_verdict(&hook),
        deja::CaptureVerdict::SkipNoDecision,
        "the per-boundary capture gate must stay closed until a decision is \
             pushed — the split is what breaks the ingress circular dependency"
    );
}

#[test]
fn request_identifier_builder_keeps_explicit_header_and_id_reuse_without_sampler() {
    let identifier =
        RequestIdentifier::new("x-correlation-id").use_incoming_id(IdReuse::IgnoreIncoming);

    assert_eq!(identifier.header_name(), "x-correlation-id");
    assert_eq!(identifier.id_reuse_strategy(), IdReuse::IgnoreIncoming);
}

#[cfg(feature = "deja")]
struct RejectAllSampler;

#[cfg(feature = "deja")]
impl RequestRecordingSampler for RejectAllSampler {
    fn should_record(&self, _facts: RequestRecordingFacts) -> RequestRecordingSamplerFuture<'_> {
        Box::pin(ready(false))
    }
}

#[cfg(feature = "deja")]
struct EmptyLookup;

#[cfg(feature = "deja")]
impl deja::LookupTableSource for EmptyLookup {
    fn load(&mut self) -> std::io::Result<deja::LookupTable> {
        Ok(deja::LookupTable {
            recording_id: "empty".to_string(),
            policy_version: 1,
            event_schema_version: Some(deja::CURRENT_EVENT_SCHEMA_VERSION),
            entries: Vec::new(),
            identity_entries: Vec::new(),
        })
    }
}

#[cfg(feature = "deja")]
struct ObservedRecordingHook {
    recorded: std::sync::Mutex<Vec<deja::BoundaryEvent>>,
}

#[cfg(feature = "deja")]
impl ObservedRecordingHook {
    fn new() -> Self {
        Self {
            recorded: std::sync::Mutex::new(Vec::new()),
        }
    }

    fn recorded_len(&self) -> usize {
        self.recorded.lock().expect("recording hook lock").len()
    }

    fn recorded_events(&self) -> Vec<deja::BoundaryEvent> {
        self.recorded.lock().expect("recording hook lock").clone()
    }
}

#[cfg(feature = "deja")]
impl deja::DejaHook for ObservedRecordingHook {
    fn process_mode(&self) -> deja::RuntimeMode {
        deja::RuntimeMode::Record
    }

    fn record(&self, event: deja::BoundaryEvent) {
        self.recorded
            .lock()
            .expect("recording hook lock")
            .push(event);
    }

    fn next_global_sequence(&self) -> u64 {
        150
    }

    fn next_request_sequence(&self, _correlation_id: Option<&str>) -> u64 {
        1
    }
}

#[cfg(feature = "deja")]
struct ExactSizedNeverEofBody {
    chunk: Option<bytes::Bytes>,
    size: u64,
}

#[cfg(feature = "deja")]
impl ExactSizedNeverEofBody {
    fn new(chunk: &'static [u8]) -> Self {
        Self {
            chunk: Some(bytes::Bytes::from_static(chunk)),
            size: u64::try_from(chunk.len()).unwrap_or(u64::MAX),
        }
    }
}

#[cfg(feature = "deja")]
impl actix_web::body::MessageBody for ExactSizedNeverEofBody {
    type Error = actix_web::Error;

    fn size(&self) -> actix_web::body::BodySize {
        actix_web::body::BodySize::Sized(self.size)
    }

    fn poll_next(
        mut self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
    ) -> Poll<Option<Result<bytes::Bytes, Self::Error>>> {
        match self.chunk.take() {
            Some(chunk) => Poll::Ready(Some(Ok(chunk))),
            None => Poll::Pending,
        }
    }
}

#[cfg(feature = "deja")]
#[test]
fn recording_body_finalizes_exact_sized_body_after_matching_chunk_without_eof() {
    let hook = Arc::new(ObservedRecordingHook::new());
    let hook_dyn: Arc<dyn deja::DejaHook> = hook.clone();
    let event = deja::EventBuilder::start_with_correlation_id(
        hook_dyn.as_ref(),
        "http_incoming",
        "RequestIdMiddleware",
        "call",
        std::panic::Location::caller(),
        Some("97cd".to_string()),
        serde_json::json!({ "path": "/payments/confirm" }),
    );
    let finalizer =
        deja::LazyEventFinalizer::new(event, hook_dyn, serde_json::json!({ "status": 200 }), false);
    let mut body = boundary::RecordingBody::for_test_with_finalizer(
        ExactSizedNeverEofBody::new(b"{\"ok\":true}"),
        finalizer,
    );
    let waker = std::task::Waker::noop();
    let mut context = Context::from_waker(waker);

    let first_poll = actix_web::body::MessageBody::poll_next(Pin::new(&mut body), &mut context);

    let Poll::Ready(Some(Ok(chunk))) = first_poll else {
        panic!("first poll must yield the exact-sized body chunk before the body parks");
    };
    assert_eq!(&chunk[..], br#"{"ok":true}"#);
    assert_eq!(
            hook.recorded_len(),
            1,
            "exact-sized bodies must finalize as soon as the declared bytes are captured, without waiting for EOF or drop"
        );

    let events = hook.recorded_events();
    let event = events.first().expect("finalized http_incoming event");
    assert_eq!(event.global_sequence, 150);
    assert_eq!(event.correlation_id.as_deref(), Some("97cd"));
    assert_eq!(event.boundary, "http_incoming");
    assert_eq!(
        event
            .result
            .get("response_body")
            .and_then(|body| body.get("text"))
            .and_then(serde_json::Value::as_str),
        Some(r#"{"ok":true}"#)
    );
}

#[cfg(feature = "deja")]
#[tokio::test]
async fn spawn_fork_runs_without_waiting_for_body_eof() {
    // Stamp-only model: fork work is spawned immediately with span-derived
    // lineage stamps — nothing queues, nothing drains at body EOF, and the
    // recorder never reshapes scheduling.
    let hook = deja::LookupTableHook::from_source(EmptyLookup, deja::InMemoryObservedSink::new())
        .expect("lookup replay hook");
    match deja::set_global_runtime_hook(Some(deja::RuntimeHook::LookupReplay(hook))) {
        Ok(()) => {}
        Err(_) if deja::runtime_mode().is_replay() => {}
        Err(err) => panic!("install replay hook: {err}"),
    }

    let (tx, rx) = std::sync::mpsc::channel();
    async move {
        deja::spawn_fork(async move {
            tx.send(()).expect("test receiver alive");
        });
    }
    .await;
    tokio::task::yield_now().await;
    assert!(
        rx.try_recv().is_ok(),
        "stamp-only spawn_fork must run immediately, not wait for a drain"
    );

    let body = boundary::RecordingBody::passthrough(actix_web::body::BoxBody::new("ok"));
    let bytes = actix_web::body::to_bytes(body).await.expect("body bytes");
    assert_eq!(&bytes[..], b"ok");
}

#[cfg(feature = "deja")]
#[test]
fn request_recording_facts_exposes_only_id_method_and_path_to_sampler() {
    let facts = RequestRecordingFacts {
        request_id: "req-123".to_string(),
        method: "POST".to_string(),
        path: "/payments/confirm".to_string(),
    };

    let RequestRecordingFacts {
        request_id,
        method,
        path,
    } = facts;

    assert_eq!(request_id, "req-123");
    assert_eq!(method, "POST");
    assert_eq!(path, "/payments/confirm");
}

#[cfg(feature = "deja")]
#[test]
fn recording_sampler_trait_object_can_be_attached_without_losing_configuration() {
    let sampler: Arc<dyn RequestRecordingSampler> = Arc::new(RejectAllSampler);

    let identifier = RequestIdentifier::new("x-correlation-id")
        .use_incoming_id(IdReuse::IgnoreIncoming)
        .with_recording_sampler(sampler);

    assert!(identifier.recording_sampler.is_some());

    assert_eq!(identifier.header_name(), "x-correlation-id");
    assert_eq!(identifier.id_reuse_strategy(), IdReuse::IgnoreIncoming);
}
