//! Deja recording sink: Kafka is THE record transport (no JSONL primary).
//!
//! The `deja-record` crate intentionally has no Kafka dependency: it defines
//! the `RecordSink<SemanticEvent>` trait; this module supplies the transport.
//! The sink owns a DEDICATED `rdkafka` producer — deliberately not the shared
//! analytics producer — hardened for durability:
//!
//!   acks=all + enable.idempotence  → no acked-then-lost, no broker-side dupes
//!   bounded buffering              → backpressure surfaces as enqueue errors
//!                                    instead of unbounded memory
//!   real `flush()`                 → the writer's flush drains the producer,
//!                                    so shutdown (eof marker) means delivered
//!
//! Envelope: `deja.artifact_record/v2` — producer identity (`instance_id`),
//! capture window (`capture.mode`/`session_id`), code provenance (`code.sha`
//! from `DEJA_CODE_REF`, `code.deja_version`). Loss accounting rides the same
//! topic as `deja_sink_marker` envelopes (checkpoint / eof / dropped),
//! emitted by the writer through `write_marker` — the compactor skips them
//! when building sessions; auditors read them to verify delivery.
//!
//! Partition key: `correlation_id` when present, otherwise
//! `{recording_run_id}:{global_sequence}` so background-task events still
//! land deterministically.
//! Headers: `global_sequence`, `request_sequence`, `recording_run_id`,
//! `boundary`, `method_name` — sufficient for a Vector consumer to route
//! and structure without parsing the payload.
//!
//! Delivery semantics: enqueue errors surface as `io::Error` to the async
//! writer, which accounts the affected batch as dropped (with a `dropped`
//! marker) and keeps going; only a sustained failure streak disables
//! recording. Request threads are never failed by instrumentation.

use std::io;
use std::time::Duration;

use rdkafka::config::FromClientConfig;
use rdkafka::message::{Header, OwnedHeaders};
use rdkafka::producer::{BaseRecord, DefaultProducerContext, Producer, ThreadedProducer};
use serde::Serialize;

const SCHEMA_VERSION: u32 = 2;
const ARTIFACT_TYPE: &str = "deja_artifact_record";
const MARKER_ARTIFACT_TYPE: &str = "deja_sink_marker";
const FLUSH_TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Serialize)]
struct Capture<'a> {
    mode: &'static str,
    session_id: &'a str,
}

#[derive(Serialize)]
struct Code<'a> {
    sha: Option<&'a str>,
    deja_version: &'static str,
}

#[derive(Serialize)]
struct Envelope<'a> {
    schema_version: u32,
    artifact_type: &'static str,
    instance_id: &'a str,
    recording_run_id: &'a str,
    correlation_id: Option<&'a str>,
    event_time_ns: u64,
    capture: Capture<'a>,
    code: Code<'a>,
    event: &'a deja::SemanticEvent,
}

/// Marker envelope: same stream, same session identity, no event payload.
#[derive(Serialize)]
struct MarkerEnvelope<'a> {
    schema_version: u32,
    artifact_type: &'static str,
    instance_id: &'a str,
    recording_run_id: &'a str,
    capture: Capture<'a>,
    marker: MarkerBody<'a>,
}

#[derive(Serialize)]
struct MarkerBody<'a> {
    kind: &'static str,
    #[serde(flatten)]
    payload: &'a serde_json::Value,
}

/// `RecordSink<deja::SemanticEvent>` implementation over a deja-owned,
/// durability-hardened Kafka producer.
pub struct HyperswitchKafkaRecordSink {
    producer: ThreadedProducer<DefaultProducerContext>,
    topic: String,
    recording_run_id: String,
    /// `{service}-{host}-{boot_ms}` — distinguishes producers when several
    /// instances feed one session (and partitions the landing layout).
    instance_id: String,
    /// Code identity from `DEJA_CODE_REF` (the orchestrator resolves it from
    /// the candidate's git head); `None` when the env is absent/empty.
    code_sha: Option<String>,
}

impl HyperswitchKafkaRecordSink {
    /// Build the sink with its own hardened producer from the broker list of
    /// Hyperswitch's events config (the brokers are shared; the producer and
    /// its delivery guarantees are not).
    pub fn new(brokers: &[String], topic: String, recording_run_id: String) -> io::Result<Self> {
        let producer = ThreadedProducer::from_config(
            rdkafka::ClientConfig::new()
                .set("bootstrap.servers", brokers.join(","))
                .set("acks", "all")
                .set("enable.idempotence", "true")
                .set("message.timeout.ms", "30000")
                // Bounded buffering: a dead broker turns into enqueue errors
                // (counted + ledgered by the writer), not unbounded memory.
                .set("queue.buffering.max.messages", "100000")
                .set("queue.buffering.max.kbytes", "262144"),
        )
        .map_err(|e| io::Error::other(format!("deja kafka producer: {e}")))?;
        let host = std::env::var("HOSTNAME").unwrap_or_else(|_| "unknown".to_owned());
        let boot_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0);
        let code_sha = std::env::var("DEJA_CODE_REF")
            .ok()
            .filter(|s| !s.is_empty());
        Ok(Self {
            producer,
            topic,
            recording_run_id,
            instance_id: format!("router-{host}-{boot_ms}"),
            code_sha,
        })
    }

    fn send(&self, key: &str, payload: &[u8], headers: OwnedHeaders) -> io::Result<()> {
        self.producer
            .send(
                BaseRecord::to(&self.topic)
                    .key(key)
                    .payload(payload)
                    .headers(headers),
            )
            .map_err(|(error, _record)| io::Error::other(format!("kafka send: {error}")))
    }
}

impl deja::RecordSink<deja::SemanticEvent> for HyperswitchKafkaRecordSink {
    fn write_batch(&mut self, records: &[deja::SemanticEvent]) -> io::Result<()> {
        for event in records {
            let envelope = Envelope {
                schema_version: SCHEMA_VERSION,
                artifact_type: ARTIFACT_TYPE,
                instance_id: &self.instance_id,
                recording_run_id: &self.recording_run_id,
                correlation_id: event.correlation_id.as_deref(),
                event_time_ns: event.timestamp_ns,
                capture: Capture {
                    // Session capture only today; window mode arrives with the
                    // Phase 3 identity contract.
                    mode: "session",
                    session_id: &self.recording_run_id,
                },
                code: Code {
                    sha: self.code_sha.as_deref(),
                    deja_version: deja::PKG_VERSION,
                },
                event,
            };
            let payload = serde_json::to_vec(&envelope).map_err(io::Error::other)?;

            let key = match &event.correlation_id {
                Some(cid) => cid.clone(),
                None => format!("{}:{}", self.recording_run_id, event.global_sequence),
            };

            let global_seq = event.global_sequence.to_string();
            let request_seq = event.request_sequence.to_string();
            let headers = OwnedHeaders::new()
                .insert(Header {
                    key: "global_sequence",
                    value: Some(global_seq.as_str()),
                })
                .insert(Header {
                    key: "request_sequence",
                    value: Some(request_seq.as_str()),
                })
                .insert(Header {
                    key: "recording_run_id",
                    value: Some(self.recording_run_id.as_str()),
                })
                .insert(Header {
                    key: "boundary",
                    value: Some(event.boundary.as_str()),
                })
                .insert(Header {
                    key: "method_name",
                    value: Some(event.method_name.as_str()),
                });

            self.send(&key, &payload, headers)?;
        }
        Ok(())
    }

    fn flush(&mut self) -> io::Result<()> {
        self.producer
            .flush(FLUSH_TIMEOUT)
            .map_err(|e| io::Error::other(format!("kafka flush: {e}")))
    }

    fn write_marker(
        &mut self,
        kind: deja::MarkerKind,
        payload: &serde_json::Value,
    ) -> io::Result<()> {
        let envelope = MarkerEnvelope {
            schema_version: SCHEMA_VERSION,
            artifact_type: MARKER_ARTIFACT_TYPE,
            instance_id: &self.instance_id,
            recording_run_id: &self.recording_run_id,
            capture: Capture {
                mode: "session",
                session_id: &self.recording_run_id,
            },
            marker: MarkerBody {
                kind: kind.as_str(),
                payload,
            },
        };
        let bytes = serde_json::to_vec(&envelope).map_err(io::Error::other)?;
        let key = format!("{}:marker", self.recording_run_id);
        let headers = OwnedHeaders::new()
            .insert(Header {
                key: "recording_run_id",
                value: Some(self.recording_run_id.as_str()),
            })
            .insert(Header {
                key: "marker_kind",
                value: Some(kind.as_str()),
            });
        self.send(&key, &bytes, headers)?;
        // Markers bound delivery audits — make eof/checkpoint mean "landed".
        if matches!(kind, deja::MarkerKind::Eof) {
            let _ = self.producer.flush(FLUSH_TIMEOUT);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn envelope_serializes_artifact_record_v2_shape() {
        let event = deja::SemanticEvent {
            global_sequence: 7,
            request_sequence: 2,
            correlation_id: Some("c-123".to_string()),
            timestamp_ns: 1_000_000_000,
            recording_run_id: Some("run-abc".to_string()),
            graph_node_id: None,
            tracing_span_id: None,
            boundary: "redis".to_string(),
            trait_name: "RedisConnectionInterface".to_string(),
            method_name: "get_key".to_string(),
            call_file: "x.rs".to_string(),
            call_line: 10,
            call_column: 5,
            receiver: None,
            request: serde_json::Value::Null,
            args: serde_json::json!({"key": "foo"}),
            response: serde_json::Value::Null,
            result: serde_json::json!("bar"),
            is_error: false,
            duration_us: 42,
            event_schema_version: 1,
            callsite_identity: None,
        };
        let envelope = Envelope {
            schema_version: 2,
            artifact_type: "deja_artifact_record",
            instance_id: "router-testhost-1",
            recording_run_id: "run-abc",
            correlation_id: event.correlation_id.as_deref(),
            event_time_ns: event.timestamp_ns,
            capture: Capture {
                mode: "session",
                session_id: "run-abc",
            },
            code: Code {
                sha: Some("deadbeef"),
                deja_version: deja::PKG_VERSION,
            },
            event: &event,
        };
        let value: serde_json::Value =
            serde_json::from_slice(&serde_json::to_vec(&envelope).unwrap()).unwrap();
        assert_eq!(value["schema_version"], 2);
        assert_eq!(value["artifact_type"], "deja_artifact_record");
        assert_eq!(value["instance_id"], "router-testhost-1");
        assert_eq!(value["recording_run_id"], "run-abc");
        assert_eq!(value["correlation_id"], "c-123");
        assert_eq!(value["event_time_ns"], 1_000_000_000u64);
        assert_eq!(value["capture"]["mode"], "session");
        assert_eq!(value["capture"]["session_id"], "run-abc");
        assert_eq!(value["code"]["sha"], "deadbeef");
        assert_eq!(value["code"]["deja_version"], deja::PKG_VERSION);
        assert_eq!(value["event"]["boundary"], "redis");
        assert_eq!(value["event"]["method_name"], "get_key");
        assert_eq!(value["event"]["global_sequence"], 7);
    }

    #[test]
    fn marker_envelope_serializes_sink_marker_shape() {
        let payload = serde_json::json!({ "last_seq": 206, "records_written": 207 });
        let envelope = MarkerEnvelope {
            schema_version: 2,
            artifact_type: "deja_sink_marker",
            instance_id: "router-testhost-1",
            recording_run_id: "run-abc",
            capture: Capture {
                mode: "session",
                session_id: "run-abc",
            },
            marker: MarkerBody {
                kind: "eof",
                payload: &payload,
            },
        };
        let value: serde_json::Value =
            serde_json::from_slice(&serde_json::to_vec(&envelope).unwrap()).unwrap();
        assert_eq!(value["artifact_type"], "deja_sink_marker");
        assert_eq!(value["marker"]["kind"], "eof");
        assert_eq!(value["marker"]["last_seq"], 206);
        // No event payload on markers — the compactor skips them by type.
        assert!(value.get("event").is_none());
    }
}
