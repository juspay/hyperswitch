//! Deja gRPC egress semantics — the capture/reconstruct half of the gRPC
//! boundary.
//!
//! Mirrors `http_client::semantic_boundary` one layer down: the boundary sits
//! on the tonic *transport* (a tower `Service`), so everything here operates
//! on wire-level values — http/2 response parts, length-prefixed gRPC frames,
//! trailers — and never interprets gRPC framing. Recorded bytes are replayed
//! verbatim through tonic's own decoder, which is what makes substitution
//! byte-faithful: typed `Status` (including `grpc-status-details-bin`),
//! trailers-only error shapes, and compression all reproduce by construction.
//!
//! Lookup identity note: undecoded (descriptor-less) request messages MUST be
//! keyed by [`wire_canon`], never by raw bytes — prost encodes `map<..>`
//! fields in per-process `HashMap` iteration order, so the same logical
//! request produces different bytes across the recording and candidate
//! processes. Responses are never hashed: they replay verbatim.

use base64::Engine;
use tonic::codegen::http;

/// General purpose base64 engine used for wire payload capture.
const B64: base64::engine::GeneralPurpose = base64::engine::general_purpose::STANDARD;

/// Correlation for a gRPC egress call: the `x-request-id` request metadata,
/// attached by all three client families (mirrors the HTTP boundary's
/// `X-Request-Id` extractor).
pub fn correlation(headers: &http::HeaderMap) -> Option<String> {
    headers
        .get("x-request-id")
        .and_then(|value| value.to_str().ok())
        .map(ToOwned::to_owned)
}

/// Canonical boundary args for a unary gRPC call.
///
/// `decoded_request` is the descriptor-decoded proto3-JSON of the request
/// message when the rpc's schema is known (local protos); when `None` the
/// args instead carry the [`wire_canon`] form of each request message and an
/// `undecoded: true` marker (UCS — field-level diffs deferred by design D2).
pub fn grpc_args(
    rpc: &str,
    authority: Option<&str>,
    headers: &http::HeaderMap,
    request_body: &[u8],
    decoded_request: Option<serde_json::Value>,
) -> serde_json::Value {
    let mut args = serde_json::Map::new();
    args.insert("rpc".to_owned(), serde_json::Value::from(rpc));
    args.insert("authority".to_owned(), serde_json::Value::from(authority));
    args.insert(
        "metadata".to_owned(),
        serde_json::json!(header_pairs(headers)),
    );
    match decoded_request {
        Some(decoded) => {
            args.insert("request".to_owned(), decoded);
        }
        None => {
            // Canonicalize per message so the identity is stable under
            // prost's map-entry encode-order nondeterminism. A body that does
            // not parse as clean gRPC framing (or carries compressed frames)
            // falls back to canonicalizing the whole body — still
            // deterministic, weaker only in precision.
            let canon: Vec<String> = match unframe_messages(request_body) {
                Some(messages) => messages
                    .iter()
                    .map(|message| wire_canon::canonical_b64(message))
                    .collect(),
                None => vec![wire_canon::canonical_b64(request_body)],
            };
            args.insert("request_canon_b64".to_owned(), serde_json::json!(canon));
            args.insert("undecoded".to_owned(), serde_json::Value::from(true));
        }
    }
    serde_json::Value::Object(args)
}

/// Splits a raw gRPC message-stream body into its length-prefixed messages
/// (`[compressed: u8][len: u32 BE][payload]`*). Returns `None` on malformed
/// framing or any compressed frame (canonicalization cannot see inside
/// compressed bytes; hyperswitch clients never negotiate compression).
pub fn unframe_messages(body: &[u8]) -> Option<Vec<&[u8]>> {
    let mut out = Vec::new();
    let mut rest = body;
    while !rest.is_empty() {
        let (head, tail) = rest.split_at_checked(5)?;
        if *head.first()? != 0 {
            return None;
        }
        let len_bytes: [u8; 4] = head.get(1..5)?.try_into().ok()?;
        let len = usize::try_from(u32::from_be_bytes(len_bytes)).ok()?;
        let (message, remaining) = tail.split_at_checked(len)?;
        out.push(message);
        rest = remaining;
    }
    Some(out)
}

/// Schema-free canonicalization of protobuf wire bytes, used ONLY to derive
/// stable lookup identity for undecoded messages.
///
/// The contract is DETERMINISM, not schema-correctness: both the lookup
/// renderer and the replay hook apply the same procedure to their respective
/// bytes, so any parse heuristic is fine as long as identical payloads make
/// identical decisions. Canonicalization sorts repeated occurrences of the
/// same field number (map entries are repeated messages on the wire), which
/// erases order-sensitivity of genuinely ordered repeated fields FOR THE KEY
/// — an accepted, documented trade-off (order-only changes substitute instead
/// of re-keying).
pub mod wire_canon {
    use base64::Engine;

    /// Nesting cap: beyond this the level is kept verbatim (still
    /// deterministic — depth is a function of the data).
    const MAX_DEPTH: usize = 32;

    /// Canonical form of `bytes`; input that does not parse as protobuf wire
    /// format is returned verbatim.
    pub fn canonical(bytes: &[u8]) -> Vec<u8> {
        canon_level(bytes, 0).unwrap_or_else(|| bytes.to_vec())
    }

    /// Base64 of [`canonical`] — the shape embedded in boundary args.
    pub fn canonical_b64(bytes: &[u8]) -> String {
        super::B64.encode(canonical(bytes))
    }

    fn canon_level(bytes: &[u8], depth: usize) -> Option<Vec<u8>> {
        if depth >= MAX_DEPTH {
            return None;
        }
        let mut items: Vec<(u64, u8, Vec<u8>)> = Vec::new();
        let mut rest = bytes;
        while !rest.is_empty() {
            let (tag, remaining) = varint(rest)?;
            rest = remaining;
            let field = tag >> 3;
            let wire_type = u8::try_from(tag & 7).ok()?;
            if field == 0 {
                return None;
            }
            let payload: Vec<u8> = match wire_type {
                0 => {
                    let (raw, remaining) = varint_raw(rest)?;
                    rest = remaining;
                    raw.to_vec()
                }
                1 => take(&mut rest, 8)?.to_vec(),
                5 => take(&mut rest, 4)?.to_vec(),
                2 => {
                    let (len, remaining) = varint(rest)?;
                    rest = remaining;
                    let raw = take(&mut rest, usize::try_from(len).ok()?)?;
                    // Recurse when the payload itself parses as wire format.
                    // The decision depends only on the payload bytes, so it is
                    // identical on both sides of a record/replay pair.
                    canon_level(raw, depth.checked_add(1)?).unwrap_or_else(|| raw.to_vec())
                }
                // Groups (3/4) are deprecated and unused; anything else is
                // malformed — keep the whole level verbatim.
                _ => return None,
            };
            items.push((field, wire_type, payload));
        }
        items.sort();
        let mut out = Vec::with_capacity(bytes.len());
        for (field, wire_type, payload) in items {
            put_varint(field.checked_shl(3)? | u64::from(wire_type), &mut out);
            if wire_type == 2 {
                put_varint(u64::try_from(payload.len()).ok()?, &mut out);
            }
            out.extend_from_slice(&payload);
        }
        Some(out)
    }

    fn take<'a>(rest: &mut &'a [u8], n: usize) -> Option<&'a [u8]> {
        let (head, tail) = rest.split_at_checked(n)?;
        *rest = tail;
        Some(head)
    }

    /// Decodes a varint, returning (value, rest). Rejects >10-byte runs.
    fn varint(bytes: &[u8]) -> Option<(u64, &[u8])> {
        let (raw, rest) = varint_raw(bytes)?;
        let mut value: u64 = 0;
        for (index, byte) in raw.iter().enumerate() {
            let shift = u32::try_from(index).ok()?.checked_mul(7)?;
            value |= u64::from(byte & 0x7f).checked_shl(shift)?;
        }
        Some((value, rest))
    }

    /// Splits off the raw encoded bytes of one varint (preserved verbatim so
    /// re-emission never normalizes what the producer wrote).
    fn varint_raw(bytes: &[u8]) -> Option<(&[u8], &[u8])> {
        let end = bytes
            .iter()
            .take(10)
            .position(|byte| byte & 0x80 == 0)?
            .checked_add(1)?;
        bytes.split_at_checked(end)
    }

    fn put_varint(mut value: u64, out: &mut Vec<u8>) {
        loop {
            let byte = u8::try_from(value & 0x7f).unwrap_or(0x7f);
            value >>= 7;
            if value == 0 {
                out.push(byte);
                return;
            }
            out.push(byte | 0x80);
        }
    }
}

/// The recorded result of one unary gRPC exchange — everything needed to hand
/// tonic's decoder a byte-identical response on replay.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum GrpcResultEnvelope {
    /// An HTTP/2 response arrived (any grpc-status, including declines —
    /// gRPC errors are just recorded trailers, replayed verbatim).
    Response {
        /// HTTP status (gRPC uses 200 even for declines; transport-mapped
        /// failures may differ).
        http_status: u16,
        /// Response headers, sorted by (name, value). gRPC metadata is ASCII
        /// by spec (binary metadata rides base64 in `-bin` keys); non-UTF-8
        /// values are captured lossily.
        headers: Vec<(String, String)>,
        /// Raw gRPC body frames exactly as received — length prefixes,
        /// compression flags and all. Never parsed, never re-framed.
        body_b64: String,
        /// HTTP/2 trailers; `None` preserves the trailers-only/headers-only
        /// error shape so `Status::metadata()` partitions identically.
        trailers: Option<Vec<(String, String)>>,
    },
    /// The transport itself failed (connect refused, h2 reset, timeout).
    /// Replays approximately: the recorded display chain, not the original
    /// error struct.
    TransportError {
        /// Display chain of the transport error.
        error: String,
    },
}

impl GrpcResultEnvelope {
    /// Captures a buffered response into its recorded envelope.
    pub fn from_response_parts(
        http_status: u16,
        headers: &http::HeaderMap,
        body: &[u8],
        trailers: Option<&http::HeaderMap>,
    ) -> Self {
        Self::Response {
            http_status,
            headers: header_pairs(headers),
            body_b64: B64.encode(body),
            trailers: trailers.map(header_pairs),
        }
    }

    /// Whether this result is an error in gRPC terms: transport failure,
    /// non-zero `grpc-status` (trailers first, then the headers-only shape),
    /// or a missing status on a non-200 response.
    pub fn is_err(&self) -> bool {
        match self {
            Self::TransportError { .. } => true,
            Self::Response {
                http_status,
                headers,
                trailers,
                ..
            } => {
                let grpc_status = trailers
                    .as_ref()
                    .and_then(|pairs| find_pair(pairs, "grpc-status"))
                    .or_else(|| find_pair(headers, "grpc-status"));
                match grpc_status {
                    Some(status) => status != "0",
                    None => *http_status != 200,
                }
            }
        }
    }
}

fn find_pair<'a>(pairs: &'a [(String, String)], name: &str) -> Option<&'a str> {
    pairs
        .iter()
        .find(|(key, _)| key == name)
        .map(|(_, value)| value.as_str())
}

/// ALL request metadata, sorted by (name, value) so the serialized args are
/// canonical (`args_hash` is order-sensitive).
///
/// Full-fidelity capture BY POLICY: everything the request carried is recorded
/// — including connector auth metadata — exactly as the HTTP boundary records
/// all request headers. Tape protection/redaction is a separate, deferred
/// workstream; the capture layer never drops data. Replay stability holds
/// because the candidate rebuilds requests from SEEDED state, so recorded
/// values reproduce; a metadata change (auth, transport version, anything)
/// re-keys the lookup and surfaces as an honest divergence — same semantics
/// as HTTP.
fn header_pairs(headers: &http::HeaderMap) -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = headers
        .iter()
        .map(|(name, value)| {
            (
                name.as_str().to_owned(),
                String::from_utf8_lossy(value.as_bytes()).into_owned(),
            )
        })
        .collect();
    out.sort();
    out
}

fn headers_from_pairs(pairs: &[(String, String)]) -> Option<http::HeaderMap> {
    let mut map = http::HeaderMap::with_capacity(pairs.len());
    for (name, value) in pairs {
        map.append(
            http::header::HeaderName::from_bytes(name.as_bytes()).ok()?,
            http::header::HeaderValue::from_str(value).ok()?,
        );
    }
    Some(map)
}

/// Rebuilds the `http::Response` a substituted call returns — tonic's own
/// client stack decodes it exactly as it would a network response. `None`
/// (malformed recorded data, or the transport-error arm which the seam
/// handles separately) maps to a reconstruction failure at the boundary.
pub fn reconstruct_response(envelope: &GrpcResultEnvelope) -> Option<http::Response<BufferedBody>> {
    let GrpcResultEnvelope::Response {
        http_status,
        headers,
        body_b64,
        trailers,
    } = envelope
    else {
        return None;
    };
    let data = B64.decode(body_b64).ok()?;
    let trailer_map = match trailers {
        Some(pairs) => Some(headers_from_pairs(pairs)?),
        None => None,
    };
    let mut response =
        http::Response::new(BufferedBody::new(bytes::Bytes::from(data), trailer_map));
    *response.status_mut() = http::StatusCode::from_u16(*http_status).ok()?;
    *response.headers_mut() = headers_from_pairs(headers)?;
    Some(response)
}

/// A fully buffered http body: one data frame, then optional trailers, then
/// end-of-stream. Serves both sides of the boundary — the record path hands
/// tonic a response rebuilt over the same buffer it taped (byte-identical
/// parity), and the replay path reconstructs from the recorded envelope.
#[derive(Debug)]
pub struct BufferedBody {
    data: Option<bytes::Bytes>,
    trailers: Option<http::HeaderMap>,
}

impl BufferedBody {
    /// A body yielding `data` (skipped when empty) then `trailers`.
    pub fn new(data: bytes::Bytes, trailers: Option<http::HeaderMap>) -> Self {
        Self {
            data: (!data.is_empty()).then_some(data),
            trailers,
        }
    }
}

impl http_body::Body for BufferedBody {
    type Data = bytes::Bytes;
    type Error = std::convert::Infallible;

    fn poll_frame(
        self: std::pin::Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Result<http_body::Frame<Self::Data>, Self::Error>>> {
        let this = self.get_mut();
        if let Some(data) = this.data.take() {
            return std::task::Poll::Ready(Some(Ok(http_body::Frame::data(data))));
        }
        if let Some(trailers) = this.trailers.take() {
            return std::task::Poll::Ready(Some(Ok(http_body::Frame::trailers(trailers))));
        }
        std::task::Poll::Ready(None)
    }

    fn is_end_stream(&self) -> bool {
        self.data.is_none() && self.trailers.is_none()
    }

    fn size_hint(&self) -> http_body::SizeHint {
        match &self.data {
            Some(data) => {
                http_body::SizeHint::with_exact(u64::try_from(data.len()).unwrap_or(u64::MAX))
            }
            None => http_body::SizeHint::with_exact(0),
        }
    }
}

/// Descriptor-pool decoding for the local protos (dynamic routing, revenue
/// recovery, health) — the readable-JSON projection and field-level diffs.
/// UCS descriptors are deferred by design decision D2; UCS rpcs simply return
/// `None` here and fall back to the wire-canonical identity.
#[cfg(any(feature = "dynamic_routing", feature = "revenue_recovery"))]
pub mod descriptors {
    use std::sync::OnceLock;

    use prost_reflect::{DescriptorPool, DynamicMessage, MessageDescriptor};

    static POOLS: OnceLock<Vec<DescriptorPool>> = OnceLock::new();

    /// One pool per emitted descriptor set (kept separate so shared
    /// well-known files can never collide across sets).
    fn pools() -> &'static [DescriptorPool] {
        POOLS.get_or_init(|| {
            let mut pools = Vec::new();
            #[cfg(feature = "dynamic_routing")]
            if let Ok(pool) = DescriptorPool::decode(
                &include_bytes!(concat!(
                    env!("OUT_DIR"),
                    "/deja_dynamic_routing_descriptor.bin"
                ))[..],
            ) {
                pools.push(pool);
            }
            #[cfg(feature = "revenue_recovery")]
            if let Ok(pool) = DescriptorPool::decode(
                &include_bytes!(concat!(env!("OUT_DIR"), "/deja_recovery_descriptor.bin"))[..],
            ) {
                pools.push(pool);
            }
            pools
        })
    }

    /// `(input, output)` message descriptors for an rpc path of the form
    /// `/package.Service/Method`, when the schema is known.
    pub fn method_descriptors(rpc: &str) -> Option<(MessageDescriptor, MessageDescriptor)> {
        let (service, method) = rpc.strip_prefix('/')?.split_once('/')?;
        pools().iter().find_map(|pool| {
            let service = pool
                .services()
                .find(|candidate| candidate.full_name() == service)?;
            let method = service
                .methods()
                .find(|candidate| candidate.name() == method)?;
            Some((method.input(), method.output()))
        })
    }

    /// Proto3-JSON projection of one wire message.
    pub fn decode_to_json(
        descriptor: &MessageDescriptor,
        message: &[u8],
    ) -> Option<serde_json::Value> {
        let decoded = DynamicMessage::decode(descriptor.clone(), message).ok()?;
        serde_json::to_value(&decoded).ok()
    }

    /// Decodes a single-message request body (unary) into proto3-JSON using
    /// the rpc's input descriptor. `None` = unknown schema or malformed body;
    /// callers fall back to the wire-canonical identity.
    pub fn decode_unary_request(rpc: &str, request_body: &[u8]) -> Option<serde_json::Value> {
        let (input, _) = method_descriptors(rpc)?;
        let messages = super::unframe_messages(request_body)?;
        let (message, rest) = messages.split_first()?;
        if !rest.is_empty() {
            return None;
        }
        decode_to_json(&input, message)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]
mod tests {
    use super::*;

    /// Minimal wire-format writer for test fixtures.
    fn field(out: &mut Vec<u8>, number: u64, wire_type: u64, payload: &[u8]) {
        put_test_varint(number << 3 | wire_type, out);
        if wire_type == 2 {
            put_test_varint(u64::try_from(payload.len()).unwrap(), out);
        }
        out.extend_from_slice(payload);
    }

    fn put_test_varint(mut value: u64, out: &mut Vec<u8>) {
        loop {
            let byte = u8::try_from(value & 0x7f).unwrap();
            value >>= 7;
            if value == 0 {
                out.push(byte);
                return;
            }
            out.push(byte | 0x80);
        }
    }

    /// One `map<string, string>` entry: nested message {1: key, 2: value}.
    fn map_entry(key: &str, value: &str) -> Vec<u8> {
        let mut entry = Vec::new();
        field(&mut entry, 1, 2, key.as_bytes());
        field(&mut entry, 2, 2, value.as_bytes());
        entry
    }

    fn frame(message: &[u8]) -> Vec<u8> {
        let mut framed = vec![0u8];
        framed.extend_from_slice(&u32::try_from(message.len()).unwrap().to_be_bytes());
        framed.extend_from_slice(message);
        framed
    }

    #[test]
    fn map_reorder_is_canonically_stable() {
        // message { 1: "id", map<string,string> 2 } with entries in both orders
        let (mut ab, mut ba) = (Vec::new(), Vec::new());
        field(&mut ab, 1, 2, b"payment_123");
        field(&mut ab, 2, 2, &map_entry("alpha", "1"));
        field(&mut ab, 2, 2, &map_entry("beta", "2"));
        field(&mut ba, 1, 2, b"payment_123");
        field(&mut ba, 2, 2, &map_entry("beta", "2"));
        field(&mut ba, 2, 2, &map_entry("alpha", "1"));
        assert_ne!(ab, ba, "fixtures must differ before canonicalization");
        assert_eq!(wire_canon::canonical(&ab), wire_canon::canonical(&ba));
    }

    #[test]
    fn value_changes_change_the_canonical_form() {
        let (mut left, mut right) = (Vec::new(), Vec::new());
        field(&mut left, 2, 2, &map_entry("alpha", "1"));
        field(&mut right, 2, 2, &map_entry("alpha", "CHANGED"));
        assert_ne!(wire_canon::canonical(&left), wire_canon::canonical(&right));
    }

    #[test]
    fn nested_map_reorder_is_canonically_stable() {
        // outer message { 3: inner } where inner carries the reordered map
        let (mut inner_ab, mut inner_ba) = (Vec::new(), Vec::new());
        field(&mut inner_ab, 2, 2, &map_entry("alpha", "1"));
        field(&mut inner_ab, 2, 2, &map_entry("beta", "2"));
        field(&mut inner_ba, 2, 2, &map_entry("beta", "2"));
        field(&mut inner_ba, 2, 2, &map_entry("alpha", "1"));
        let (mut outer_ab, mut outer_ba) = (Vec::new(), Vec::new());
        field(&mut outer_ab, 3, 2, &inner_ab);
        field(&mut outer_ba, 3, 2, &inner_ba);
        assert_eq!(
            wire_canon::canonical(&outer_ab),
            wire_canon::canonical(&outer_ba)
        );
    }

    #[test]
    fn non_protobuf_input_passes_through_verbatim() {
        let text = b"hello world! this is not protobuf";
        assert_eq!(wire_canon::canonical(text), text.to_vec());
        assert_eq!(wire_canon::canonical(&[]), Vec::<u8>::new());
    }

    #[test]
    fn unframe_roundtrip_and_rejections() {
        let message = b"\x0a\x02hi".to_vec();
        let mut body = frame(&message);
        body.extend_from_slice(&frame(b"more"));
        let messages = unframe_messages(&body).unwrap();
        assert_eq!(messages, vec![&message[..], &b"more"[..]]);

        // compressed flag and truncation are both rejected
        let mut compressed = frame(&message);
        compressed[0] = 1;
        assert!(unframe_messages(&compressed).is_none());
        assert!(unframe_messages(&body[..body.len() - 1]).is_none());
    }

    #[test]
    fn args_capture_everything_sorted_full_fidelity() {
        let mut headers = http::HeaderMap::new();
        headers.insert("x-tenant-id", "t1".parse().unwrap());
        headers.insert("x-request-id", "req_9".parse().unwrap());
        headers.insert("x-api-key", "SECRET".parse().unwrap());
        headers.insert("content-type", "application/grpc".parse().unwrap());

        let args = grpc_args(
            "/success_rate.SuccessRateCalculator/FetchSuccessRate",
            Some("dyn-routing:7000"),
            &headers,
            &frame(b"\x0a\x02id"),
            None,
        );
        // Full fidelity: EVERYTHING the request carried is on the tape —
        // auth metadata included (protection is a deferred workstream, and
        // the HTTP boundary records all headers identically). Sorted for a
        // canonical args_hash.
        assert_eq!(
            args["metadata"],
            serde_json::json!([
                ["content-type", "application/grpc"],
                ["x-api-key", "SECRET"],
                ["x-request-id", "req_9"],
                ["x-tenant-id", "t1"]
            ])
        );
        assert_eq!(args["undecoded"], serde_json::json!(true));
        assert_eq!(correlation(&headers).as_deref(), Some("req_9"));
    }

    #[test]
    fn envelope_roundtrips_through_serde() {
        let envelope = GrpcResultEnvelope::Response {
            http_status: 200,
            headers: vec![("content-type".to_owned(), "application/grpc".to_owned())],
            body_b64: B64.encode(frame(b"\x08\x01")),
            trailers: Some(vec![("grpc-status".to_owned(), "0".to_owned())]),
        };
        let json = serde_json::to_value(&envelope).unwrap();
        let back: GrpcResultEnvelope = serde_json::from_value(json).unwrap();
        assert_eq!(envelope, back);
        assert!(!envelope.is_err());
        assert!(GrpcResultEnvelope::TransportError {
            error: "connect refused".to_owned()
        }
        .is_err());
    }

    #[tokio::test]
    async fn buffered_body_emits_data_then_trailers() {
        use http_body_util::BodyExt;

        let mut trailers = http::HeaderMap::new();
        trailers.insert("grpc-status", "0".parse().unwrap());
        let body = BufferedBody::new(bytes::Bytes::from_static(b"payload"), Some(trailers));
        let collected = body.collect().await.unwrap();
        assert_eq!(
            collected.trailers().and_then(|t| t.get("grpc-status")),
            Some(&"0".parse().unwrap())
        );
        assert_eq!(collected.to_bytes().as_ref(), b"payload");
    }

    #[tokio::test]
    async fn recorded_decline_reconstructs_the_same_typed_status() {
        use http_body_util::BodyExt;

        // A recorded decline: grpc-status 3 (InvalidArgument) in trailers.
        let envelope = GrpcResultEnvelope::Response {
            http_status: 200,
            headers: vec![("content-type".to_owned(), "application/grpc".to_owned())],
            body_b64: String::new(),
            trailers: Some(vec![
                ("grpc-message".to_owned(), "card declined".to_owned()),
                ("grpc-status".to_owned(), "3".to_owned()),
            ]),
        };
        assert!(envelope.is_err());

        let response = reconstruct_response(&envelope).unwrap();
        assert_eq!(response.status(), http::StatusCode::OK);
        let collected = response.into_body().collect().await.unwrap();
        let status = tonic::Status::from_header_map(collected.trailers().unwrap()).unwrap();
        assert_eq!(status.code(), tonic::Code::InvalidArgument);
        assert_eq!(status.message(), "card declined");
    }

    #[test]
    fn trailers_only_shape_is_preserved() {
        let mut headers = http::HeaderMap::new();
        headers.insert("grpc-status", "14".parse().unwrap());
        let envelope = GrpcResultEnvelope::from_response_parts(200, &headers, &[], None);
        assert!(envelope.is_err());
        let response = reconstruct_response(&envelope).unwrap();
        // Status stays in the HEADERS — no synthesized trailers frame.
        let status = tonic::Status::from_header_map(response.headers()).unwrap();
        assert_eq!(status.code(), tonic::Code::Unavailable);
    }

    #[cfg(feature = "dynamic_routing")]
    #[test]
    fn descriptor_decode_by_rpc_path() {
        use prost_reflect::prost::Message;

        let rpc = "/success_rate.SuccessRateCalculator/FetchSuccessRate";
        let (input, output) = descriptors::method_descriptors(rpc).unwrap();
        assert_eq!(input.full_name(), "success_rate.CalSuccessRateRequest");
        assert_eq!(output.full_name(), "success_rate.CalSuccessRateResponse");

        // Round-trip a request through the descriptor: set fields dynamically,
        // encode, decode back to proto3-JSON.
        let mut message = prost_reflect::DynamicMessage::new(input.clone());
        message.set_field_by_name("id", prost_reflect::Value::String("m1:gateway".to_owned()));
        message.set_field_by_name(
            "params",
            prost_reflect::Value::String("card&credit".to_owned()),
        );
        let wire = message.encode_to_vec();

        let json = descriptors::decode_unary_request(rpc, &frame(&wire)).unwrap();
        assert_eq!(json["id"], serde_json::json!("m1:gateway"));
        assert_eq!(json["params"], serde_json::json!("card&credit"));

        // Unknown schema (UCS) falls back to None — the wire-canonical path.
        assert!(descriptors::method_descriptors("/types.PaymentService/Authorize").is_none());
    }
}
