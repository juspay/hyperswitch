//! The deja gRPC egress boundary — a tower `Service` wrapper around the
//! transport under every generated tonic client.
//!
//! The wrapper is installed at the transport CONSTRUCTION sites (the shared
//! hyper pool `Client` type alias and the UCS channels), so every unary rpc —
//! current and future, on any service client built over the wrapped transport
//! — crosses one deja boundary with zero changes to how gRPC client code is
//! written or called.
//!
//! Identity carries NO explicit call-site id: this is a generic transport
//! wrapper, so there is no per-call-site literal to name. It is
//! `CallsiteSource::SyntacticHash` with `scope = "grpc::/package.Service/Method"`
//! — the rpc path is the most version-independent name an egress call has, and
//! it rides in the scope rather than as a rank-1 id — plus the span path as the
//! calling-context disambiguator. Routing is HARDCODED
//! `ReplayStrategy::Substitute` — egress is never re-issued.
//!
//! Substitution replays the recorded wire bytes through tonic's own decoder
//! (see [`super::semantic_boundary`]), so typed `Status` errors, trailers-only
//! shapes, and metadata partitioning reproduce by construction.

use std::{
    future::Future,
    panic::Location,
    pin::Pin,
    task::{Context, Poll},
};

use tonic::{body::Body as TonicBody, codegen::http};

use super::semantic_boundary::{self, BufferedBody, GrpcResultEnvelope};

/// Boxed error type matching the `GrpcService` transport-error bound.
type BoxError = Box<dyn std::error::Error + Send + Sync>;

const BOUNDARY: &str = "grpc";
const COMPONENT: &str = "external_services::grpc_client::deja_transport";
const OPERATION: &str = "unary";

/// The transport wrapper. Generic over the inner tower service so the same
/// type serves the shared hyper pool (dynamic routing / health / recovery)
/// and the UCS `tonic::transport::Channel`s.
///
/// `inner` is `None` only for a boundary built by
/// [`DejaGrpcTransport::substituted`] — see there and [`connect_or_substitute`].
#[derive(Debug, Clone)]
pub struct DejaGrpcTransport<S> {
    inner: Option<S>,
}

impl<S> DejaGrpcTransport<S> {
    /// Wraps a live transport. Call at the construction site, never per request.
    pub fn new(inner: S) -> Self {
        Self { inner: Some(inner) }
    }

    /// A boundary with no transport beneath it.
    ///
    /// Valid ONLY under replay, where every call is served from the recording:
    /// readiness is answered directly, and a call the recording cannot answer
    /// fail-stops via [`fail_stop_absent_transport`] rather than connecting.
    pub fn substituted() -> Self {
        Self { inner: None }
    }
}

/// Wrap a transport the host builds eagerly, standing in for it under replay.
///
/// Under replay `connect` is never run: no rpc is issued live, so the transport
/// is dead weight and could not have connected anyway. In every other mode it
/// runs verbatim, and `None` propagates the host's own outcome unchanged.
pub async fn connect_or_substitute<S, F, Fut>(connect: F) -> Option<DejaGrpcTransport<S>>
where
    F: FnOnce() -> Fut,
    Fut: Future<Output = Option<S>>,
{
    if deja::__private::runtime_mode().is_replay() {
        Some(DejaGrpcTransport::substituted())
    } else {
        connect().await.map(DejaGrpcTransport::new)
    }
}

/// Replay fail-stop for a boundary that deliberately has no transport.
///
/// Distinct from `deja::__private::fail_stop_substitute_miss`, whose message
/// offers `replay_strategy = Execute` as the remedy: with nothing beneath the
/// boundary there is nothing to run, and the miss says nothing about the
/// candidate either way.
///
/// Names the rpc path and authority because `BOUNDARY` and `COMPONENT` are
/// module constants shared by every transport this wrapper is installed on.
// Panicking IS deja's fail-stop mechanism, not an accident: `catch_fail_stop_async`
// classifies the payload by `FAIL_STOP_SENTINEL` and renders the divergence. Returning
// an error instead would erase the stop from the diff artifact.
#[allow(clippy::panic)]
#[cold]
#[inline(never)]
fn fail_stop_absent_transport(
    boundary: &str,
    component: &str,
    rpc: &str,
    authority: Option<&str>,
) -> ! {
    let target = match authority {
        Some(authority) => format!("`{rpc}` at `{authority}`"),
        None => format!("`{rpc}`"),
    };
    panic!(
        "{} Substitute boundary `{boundary}` in `{component}` has no transport for \
         {target}. It was constructed for substitution only: the host builds this \
         transport eagerly, that connect did not succeed, and replay never issues \
         this call live. The recording has no entry for these args, so there is \
         nothing to substitute and nothing this boundary could have executed. This \
         says nothing about the candidate — the call could not have been served \
         here whatever the candidate did.",
        deja::FAIL_STOP_SENTINEL
    );
}

/// A boundary with no transport asked to issue a call live. Unreachable in
/// practice — [`DejaGrpcTransport::substituted`] exists only under replay, where
/// the `run` thunk is never invoked — but named so a broken invariant is legible.
#[derive(Debug)]
pub struct AbsentTransportError;

impl std::fmt::Display for AbsentTransportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(
            "deja: gRPC boundary has no transport (constructed for substitution only) \
             and cannot issue this call live",
        )
    }
}

impl std::error::Error for AbsentTransportError {}

/// Envelope smuggled through `http::Extensions` from the buffering `run`
/// closure to the recording `extract` closure — the same trick the HTTP
/// boundary uses for its response body (`CapturedResponseBody`).
#[derive(Clone)]
struct CapturedEnvelope(GrpcResultEnvelope);

/// A substituted transport error: the recorded display chain of the original
/// failure. Approximate by design (the original error struct is gone); tonic
/// maps it through `Status::from_error` exactly as it would a live failure.
#[derive(Debug)]
pub struct ReplayedTransportError(pub String);

impl std::fmt::Display for ReplayedTransportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "deja replayed transport error: {}", self.0)
    }
}

impl std::error::Error for ReplayedTransportError {}

impl<S, RB> tonic::codegen::Service<http::Request<TonicBody>> for DejaGrpcTransport<S>
where
    S: tonic::codegen::Service<http::Request<TonicBody>, Response = http::Response<RB>>
        + Clone
        + Send
        + 'static,
    S::Future: Send + 'static,
    S::Error: Into<BoxError>,
    RB: http_body::Body<Data = bytes::Bytes> + Send + 'static,
    RB::Error: Into<BoxError>,
{
    type Response = http::Response<TonicBody>;
    type Error = BoxError;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, BoxError>> + Send>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        match &mut self.inner {
            // KNOWN FRAGILITY, deliberately left alone. This delegates ungated by
            // `is_active()`, so the inner service answers readiness even on replay.
            // Safe only because tonic's readiness is buffer-backed, not
            // connection-backed: `Buffer::poll_ready` checks worker liveness and
            // queue capacity, not a connection (tonic 0.14.5). An inner service
            // that answered from a live connection would fail HERE, before `call`
            // can substitute. Tracked deja-side, where the mode lifecycle a gate
            // would have to depend on lives.
            Some(inner) => inner.poll_ready(cx).map_err(Into::into),
            // Nothing to drive: substitution answers every call.
            None => Poll::Ready(Ok(())),
        }
    }

    #[track_caller]
    fn call(&mut self, request: http::Request<TonicBody>) -> Self::Future {
        // Standard tower clone-dance: `self.inner` was driven to readiness by
        // poll_ready, so hand THAT instance to the future and keep the fresh
        // clone (which will be polled to readiness before its own use).
        let inner = match self.inner.as_mut() {
            Some(inner) => {
                let clone = inner.clone();
                Some(std::mem::replace(inner, clone))
            }
            None => None,
        };

        // Boundary engaged only when a deja hook is live (record or replay);
        // otherwise pure passthrough — no buffering, no dispatch, streaming
        // untouched.
        if !is_active() {
            return match inner {
                Some(inner) => Box::pin(passthrough(inner, request)),
                // A transport-less boundary outside replay cannot serve anything.
                None => Box::pin(async { Err(BoxError::from(AbsentTransportError)) }),
            };
        }
        let caller = Location::caller();
        Box::pin(boundary_call(inner, request, caller))
    }
}

fn is_active() -> bool {
    // Covers BOTH worlds: replay/record via the runtime hook (runtime_mode)
    // and record via the recording hook — substitution must engage in replay,
    // so this must never be a record-only check.
    deja::__private::observation_is_active()
}

async fn passthrough<S, RB>(
    mut inner: S,
    request: http::Request<TonicBody>,
) -> Result<http::Response<TonicBody>, BoxError>
where
    S: tonic::codegen::Service<http::Request<TonicBody>, Response = http::Response<RB>>,
    S::Error: Into<BoxError>,
    RB: http_body::Body<Data = bytes::Bytes> + Send + 'static,
    RB::Error: Into<BoxError>,
{
    let response = inner.call(request).await.map_err(Into::into)?;
    Ok(response.map(TonicBody::new))
}

/// One unary gRPC exchange as one deja boundary crossing.
async fn boundary_call<S, RB>(
    inner: Option<S>,
    request: http::Request<TonicBody>,
    caller: &'static Location<'static>,
) -> Result<http::Response<TonicBody>, BoxError>
where
    S: tonic::codegen::Service<http::Request<TonicBody>, Response = http::Response<RB>>
        + Send
        + 'static,
    S::Future: Send + 'static,
    S::Error: Into<BoxError>,
    RB: http_body::Body<Data = bytes::Bytes> + Send + 'static,
    RB::Error: Into<BoxError>,
{
    let (parts, body) = request.into_parts();

    // Unary request: buffer the frames before forwarding — args identity
    // needs the message bytes, and the rebuilt body is what the inner
    // transport actually sends.
    let request_bytes = match http_body_util::BodyExt::collect(body).await {
        Ok(collected) => collected.to_bytes(),
        Err(error) => return Err(error.into()),
    };

    let rpc: String = parts.uri.path().to_owned();
    let authority = parts.uri.authority().map(ToString::to_string);

    // Descriptor-decoded proto3-JSON for the local protos; UCS (unknown
    // schema) falls back to the canonical wire identity inside `grpc_args`.
    #[cfg(any(feature = "dynamic_routing", feature = "revenue_recovery"))]
    let decoded_request =
        semantic_boundary::descriptors::decode_unary_request(&rpc, &request_bytes);
    #[cfg(not(any(feature = "dynamic_routing", feature = "revenue_recovery")))]
    let decoded_request: Option<serde_json::Value> = None;

    let args_value = semantic_boundary::grpc_args(
        &rpc,
        authority.as_deref(),
        &parts.headers,
        &request_bytes,
        decoded_request,
    );
    let correlation = semantic_boundary::correlation(&parts.headers);

    // Identity: rank-2 span-path (from the ambient tracing context) — NO explicit
    // call-site id. rank-1 Explicit is a hand-out tag, not a burden to stamp on
    // call-sites; the rpc path lives in the ARGS (grpc_args), not the identity, so
    // replay matches within a span by occurrence + args — exactly like the
    // db / redis / superposition boundaries. The syntactic hash of "grpc::<path>"
    // is the rank-3 fallback (each in-span call is still disambiguated by
    // occurrence, so distinct rpcs never collapse onto one identity).
    let scope = format!("grpc::{rpc}");
    let identity = deja::__private::CallsiteIdentity {
        version: 1,
        source: deja::__private::CallsiteSource::SyntacticHash,
        id: None,
        scope: Some(scope.clone()),
        occurrence: deja::__private::next_boundary_occurrence(
            correlation.as_deref(),
            deja::__private::CallsiteSource::SyntacticHash,
            Some(&scope),
        ),
        caller_function: Some(COMPONENT.to_string()),
        lexical_path: Some(scope.clone()),
        syntax_hash: Some(deja::__private::stable_callsite_hash(&scope)),
        span_path: deja::__private::current_span_path(),
    };

    // Egress routing is hardcoded: never re-issued on replay.
    let semantics = deja::__private::BoundarySemantics {
        replay_strategy: deja::ReplayStrategy::Substitute,
        kind: Some(BOUNDARY.to_string()),
        declaration: Some(
            deja::BoundaryDeclaration::default().operation(deja::OperationKind::ExternalCall),
        ),
    };
    let spec =
        deja::__private::BoundarySpec::with_semantics(BOUNDARY, COMPONENT, OPERATION, semantics);
    let observation =
        deja::__private::CrossingObservation::with_correlation(spec, identity, caller, correlation);

    let rebuilt_request = http::Request::from_parts(
        parts,
        TonicBody::new(BufferedBody::new(request_bytes, None)),
    );

    match inner {
        // Transport present: a miss may be a genuine novel call, so this keeps
        // `dispatch_async`'s default substitute-miss fail-stop.
        Some(inner) => {
            deja::__private::dispatch_async(
                observation,
                move || args_value,
                move || run_and_capture(inner, rebuilt_request),
                reconstruct_from_recorded,
                extract_envelope,
            )
            .await
        }
        // Transport deliberately absent: a miss establishes nothing about the
        // candidate, so it gets its own reason. The `run` thunk is never invoked
        // here (replay + Substitute neither runs nor re-runs); an error rather
        // than an `unreachable!` so a broken invariant stays legible.
        None => {
            deja::__private::dispatch_async_or_miss(
                observation,
                move || args_value,
                move || async { Err(BoxError::from(AbsentTransportError)) },
                reconstruct_from_recorded,
                extract_envelope,
                move || fail_stop_absent_transport(BOUNDARY, COMPONENT, &rpc, authority.as_deref()),
            )
            .await
        }
    }
}

/// The `run` thunk: forward to the real transport, buffer the response
/// verbatim (data frames + trailers), and hand tonic a response rebuilt over
/// the SAME buffer the tape captures — byte-identical parity. The envelope
/// rides `http::Extensions` to the extractor.
async fn run_and_capture<S, RB>(
    mut inner: S,
    request: http::Request<TonicBody>,
) -> Result<http::Response<TonicBody>, BoxError>
where
    S: tonic::codegen::Service<http::Request<TonicBody>, Response = http::Response<RB>>,
    S::Error: Into<BoxError>,
    RB: http_body::Body<Data = bytes::Bytes> + Send + 'static,
    RB::Error: Into<BoxError>,
{
    let response = match inner.call(request).await {
        Ok(response) => response,
        Err(error) => return Err(error.into()),
    };
    let (parts, body) = response.into_parts();
    let collected = match http_body_util::BodyExt::collect(body).await {
        Ok(collected) => collected,
        Err(error) => return Err(error.into()),
    };
    let trailers = collected.trailers().cloned();
    let data = collected.to_bytes();

    let envelope = GrpcResultEnvelope::from_response_parts(
        parts.status.as_u16(),
        &parts.headers,
        &data,
        trailers.as_ref(),
    );
    let mut rebuilt =
        http::Response::from_parts(parts, TonicBody::new(BufferedBody::new(data, trailers)));
    rebuilt.extensions_mut().insert(CapturedEnvelope(envelope));
    Ok(rebuilt)
}

/// The `extract` closure: envelope out of the extensions (record path), or a
/// transport-error envelope from the `Err` arm.
fn extract_envelope(
    result: &Result<http::Response<TonicBody>, BoxError>,
) -> (serde_json::Value, bool) {
    let envelope = match result {
        Ok(response) => match response.extensions().get::<CapturedEnvelope>() {
            Some(CapturedEnvelope(envelope)) => envelope.clone(),
            // Unreachable on the record path (run_and_capture always stamps
            // it); recorded loudly rather than silently if it ever happens.
            None => GrpcResultEnvelope::TransportError {
                error: "deja: response envelope was not captured".to_owned(),
            },
        },
        Err(error) => GrpcResultEnvelope::TransportError {
            error: format!("{error}"),
        },
    };
    let is_error = envelope.is_err();
    let value = serde_json::to_value(&envelope).unwrap_or_else(
        |error| serde_json::json!({ "deja_grpc_capture_error": error.to_string() }),
    );
    (value, is_error)
}

/// The `reconstruct` closure: recorded envelope → the identical wire response
/// through tonic's own decoder, or the recorded transport failure.
fn reconstruct_from_recorded(
    recorded: serde_json::Value,
) -> deja::__private::Reconstructed<Result<http::Response<TonicBody>, BoxError>> {
    let Ok(envelope) = serde_json::from_value::<GrpcResultEnvelope>(recorded) else {
        return deja::__private::Reconstructed::Failed(::std::string::String::from(
            "recorded value is not a `GrpcResultEnvelope`",
        ));
    };
    match &envelope {
        GrpcResultEnvelope::Response { .. } => {
            match semantic_boundary::reconstruct_response(&envelope) {
                Some(response) => {
                    deja::__private::Reconstructed::Value(Ok(response.map(TonicBody::new)))
                }
                None => deja::__private::Reconstructed::Failed(::std::string::String::from(
                    "recorded gRPC response could not be rebuilt into an http::Response \
                     (status line, headers or body frame missing from the envelope)",
                )),
            }
        }
        GrpcResultEnvelope::TransportError { error } => deja::__private::Reconstructed::Value(Err(
            Box::new(ReplayedTransportError(error.clone())),
        )),
    }
}

#[cfg(all(test, feature = "dynamic_routing"))]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::{
        super::dynamic_routing::{
            success_rate_client::success_rate::{
                success_rate_calculator_server::{
                    SuccessRateCalculator, SuccessRateCalculatorServer,
                },
                CalGlobalSuccessRateRequest, CalGlobalSuccessRateResponse, CalSuccessRateRequest,
                CalSuccessRateResponse, InvalidateWindowsRequest, InvalidateWindowsResponse,
                LabelWithScore, UpdateSuccessRateWindowRequest, UpdateSuccessRateWindowResponse,
            },
            SuccessRateCalculatorClient,
        },
        *,
    };

    /// Canned scorer: a deterministic Ok and a typed decline.
    pub(crate) struct CannedScorer;

    #[tonic::async_trait]
    impl SuccessRateCalculator for CannedScorer {
        async fn fetch_success_rate(
            &self,
            request: tonic::Request<CalSuccessRateRequest>,
        ) -> Result<tonic::Response<CalSuccessRateResponse>, tonic::Status> {
            let id = request.into_inner().id;
            Ok(tonic::Response::new(CalSuccessRateResponse {
                labels_with_score: vec![LabelWithScore {
                    score: 0.75,
                    label: format!("stripe:{id}"),
                }],
                routing_approach: 0,
            }))
        }

        async fn update_success_rate_window(
            &self,
            _request: tonic::Request<UpdateSuccessRateWindowRequest>,
        ) -> Result<tonic::Response<UpdateSuccessRateWindowResponse>, tonic::Status> {
            Err(tonic::Status::invalid_argument("window rejected"))
        }

        async fn invalidate_windows(
            &self,
            _request: tonic::Request<InvalidateWindowsRequest>,
        ) -> Result<tonic::Response<InvalidateWindowsResponse>, tonic::Status> {
            Err(tonic::Status::unimplemented("not in this test"))
        }

        async fn fetch_entity_and_global_success_rate(
            &self,
            _request: tonic::Request<CalGlobalSuccessRateRequest>,
        ) -> Result<tonic::Response<CalGlobalSuccessRateResponse>, tonic::Status> {
            Err(tonic::Status::unimplemented("not in this test"))
        }
    }

    /// Inactive-hook passthrough: real tonic server + wrapped hyper pool,
    /// traffic must be untouched in both arms. (The RECORD side needs a
    /// separate process, because the global hook OnceLock latches on first
    /// read and this test must observe the pre-install state.)
    #[tokio::test]
    async fn inactive_wrapper_is_a_pure_passthrough() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        tokio::spawn(
            tonic::transport::Server::builder()
                .add_service(SuccessRateCalculatorServer::new(CannedScorer))
                .serve_with_incoming(tokio_stream::wrappers::TcpListenerStream::new(listener)),
        );

        let pool =
            hyper_util::client::legacy::Client::builder(hyper_util::rt::TokioExecutor::new())
                .http2_only(true)
                .build_http();
        let transport = DejaGrpcTransport::new(pool);
        let uri: http::Uri = format!("http://127.0.0.1:{port}").parse().unwrap();
        let mut client = SuccessRateCalculatorClient::with_origin(transport, uri);

        assert!(!is_active(), "no deja hook may be active in the lib tests");
        let response = client
            .fetch_success_rate(tonic::Request::new(CalSuccessRateRequest {
                id: "m1".to_owned(),
                params: "card".to_owned(),
                labels: vec![],
                config: None,
            }))
            .await
            .unwrap()
            .into_inner();
        assert_eq!(response.labels_with_score.len(), 1);
        assert_eq!(
            response.labels_with_score.first().unwrap().label,
            "stripe:m1"
        );

        let status = client
            .update_success_rate_window(UpdateSuccessRateWindowRequest {
                id: "m1".to_owned(),
                params: String::new(),
                labels_with_status: vec![],
                global_labels_with_status: vec![],
                config: None,
            })
            .await
            .unwrap_err();
        assert_eq!(status.code(), tonic::Code::InvalidArgument);
        assert_eq!(status.message(), "window rejected");
    }
}
