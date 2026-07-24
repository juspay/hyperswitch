//! Deja recording sink: Kafka is THE record transport (no JSONL primary).
//!
//! The `deja-record` crate intentionally has no Kafka dependency: it defines
//! the `RecordSink<DejaRecord>` trait; this module supplies the transport.
//! The sink owns a DEDICATED `rdkafka` producer — deliberately not the shared
//! analytics producer, though both are built from the same deployment-wide
//! base client config so they share cluster provisioning — hardened for
//! durability:
//!
//!   acks=all + enable.idempotence  → no acked-then-lost, no broker-side dupes
//!   bounded buffering              → backpressure surfaces as enqueue errors
//!                                    instead of unbounded memory
//!   flush = short poll             → cadence flushes never park the writer
//!                                    behind a slow broker; only the eof
//!                                    marker drains fully, so end-of-run
//!                                    means delivered
//!
//! Envelopes, all on the ONE topic: boundary events land as
//! `deja.artifact_record/v2`; execution-graph nodes land as
//! `deja.graph_node/v1`. Both carry producer identity (`instance_id`),
//! capture window (`capture.mode`/`session_id`), and code provenance
//! (`code.sha` from typed Deja identity settings, `code.deja_version`).
//! Loss accounting rides the same topic as `deja_sink_marker` envelopes
//! (checkpoint / eof / dropped), emitted by the writer through
//! `write_marker` — the compactor skips them when building sessions;
//! auditors read them to verify delivery.
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

use std::{io, time::Duration};

use rdkafka::{
    config::FromClientConfig,
    message::{Header, OwnedHeaders},
    producer::{BaseRecord, DefaultProducerContext, Producer, ThreadedProducer},
};
use serde::Serialize;

const SCHEMA_VERSION: u32 = 2;
const ARTIFACT_TYPE: &str = "deja_artifact_record";
const GRAPH_SCHEMA_VERSION: u32 = 1;
const GRAPH_ARTIFACT_TYPE: &str = "deja_graph_node";
const MARKER_ARTIFACT_TYPE: &str = "deja_sink_marker";
/// Cadence flushes are a short bounded poll: the threaded producer delivers on
/// its own background thread, so waiting out a long drain here only parks the
/// writer thread behind a slow broker.
const CADENCE_FLUSH_POLL: Duration = Duration::from_millis(50);
/// End-of-run drain: the eof marker means "everything before this landed", so
/// shutdown waits for real delivery.
const EOF_FLUSH_TIMEOUT: Duration = Duration::from_secs(10);

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
    event: &'a deja::BoundaryEvent,
}

/// Graph-node envelope (`deja.graph_node/v1`): same stream, same session
/// identity as the artifact envelope, the execution-graph node as payload.
/// Nested under `node` — the node carries its own `recording_run_id` /
/// `global_sequence`, so flattening would collide with the envelope's.
#[derive(Serialize)]
struct GraphEnvelope<'a> {
    schema_version: u32,
    artifact_type: &'static str,
    instance_id: &'a str,
    recording_run_id: &'a str,
    capture: Capture<'a>,
    code: Code<'a>,
    node: &'a deja_core::ExecutionGraphNode,
}

/// Marker envelope: same stream, same session identity, no event payload.
#[derive(Serialize)]
struct MarkerEnvelope<'a> {
    schema_version: u32,
    artifact_type: &'static str,
    instance_id: &'a str,
    recording_run_id: &'a str,
    capture: Capture<'a>,
    code: Code<'a>,
    marker: MarkerBody<'a>,
}

#[derive(Serialize)]
struct MarkerBody<'a> {
    kind: &'static str,
    #[serde(flatten)]
    payload: &'a serde_json::Value,
}

pub struct HyperswitchKafkaRecordSinkConfig<'a> {
    pub brokers: &'a [String],
    pub topic: &'a str,
    pub recording_run_id: &'a str,
    pub instance_id: String,
    pub code_sha: Option<String>,
    pub client_id: Option<&'a str>,
    pub acks: &'a str,
    pub enable_idempotence: bool,
    pub compression: Option<&'a str>,
    pub linger_ms: Option<u64>,
    pub message_timeout_ms: u64,
    pub queue_buffering_max_messages: usize,
}

/// `RecordSink<deja::DejaRecord>` implementation over a deja-owned,
/// durability-hardened Kafka producer.
pub struct HyperswitchKafkaRecordSink {
    producer: ThreadedProducer<DefaultProducerContext>,
    topic: String,
    recording_run_id: String,
    /// Typed producer identity resolved during Deja boot; distinguishes
    /// producers when several instances feed one session.
    instance_id: String,
    /// Typed code identity resolved during Deja boot; `None` when unavailable.
    code_sha: Option<String>,
}

impl HyperswitchKafkaRecordSink {
    /// Build the sink with its own hardened producer from the broker list of
    /// Hyperswitch's events config (the brokers are shared; the producer and
    /// its delivery guarantees are not).
    pub fn new(config: HyperswitchKafkaRecordSinkConfig<'_>) -> io::Result<Self> {
        // Start from the deployment-wide base client config (shared cluster
        // provisioning), then layer the recording sink's delivery guarantees
        // on top — a separate client with its own queue and delivery thread.
        let mut producer_config = super::base_client_config(config.brokers);
        producer_config
            .set("acks", config.acks)
            .set(
                "enable.idempotence",
                if config.enable_idempotence {
                    "true"
                } else {
                    "false"
                },
            )
            .set("message.timeout.ms", config.message_timeout_ms.to_string())
            // Bounded buffering: a dead broker turns into enqueue errors
            // (counted + ledgered by the writer), not unbounded memory.
            .set(
                "queue.buffering.max.messages",
                config.queue_buffering_max_messages.to_string(),
            )
            .set("queue.buffering.max.kbytes", "262144");

        if let Some(client_id) = config.client_id.filter(|value| !value.is_empty()) {
            producer_config.set("client.id", client_id);
        }
        if let Some(compression) = config.compression.filter(|value| !value.is_empty()) {
            producer_config.set("compression.type", compression);
        }
        if let Some(linger_ms) = config.linger_ms {
            producer_config.set("linger.ms", linger_ms.to_string());
        }

        let producer = ThreadedProducer::from_config(&producer_config)
            .map_err(|e| io::Error::other(format!("deja kafka producer: {e}")))?;
        Ok(Self {
            producer,
            topic: config.topic.to_owned(),
            recording_run_id: config.recording_run_id.to_owned(),
            instance_id: config.instance_id,
            code_sha: config.code_sha,
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

    fn write_boundary_event(&self, event: &deja::BoundaryEvent) -> io::Result<()> {
        let envelope = Envelope {
            schema_version: SCHEMA_VERSION,
            artifact_type: ARTIFACT_TYPE,
            instance_id: &self.instance_id,
            recording_run_id: &self.recording_run_id,
            correlation_id: event.correlation_id.as_deref(),
            event_time_ns: event.timestamp_ns,
            capture: Capture {
                // Session capture is the only mode today; a windowed
                // capture mode is a planned extension.
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

        self.send(&key, &payload, headers)
    }

    fn write_graph_node(&self, node: &deja_core::ExecutionGraphNode) -> io::Result<()> {
        let envelope = GraphEnvelope {
            schema_version: GRAPH_SCHEMA_VERSION,
            artifact_type: GRAPH_ARTIFACT_TYPE,
            instance_id: &self.instance_id,
            recording_run_id: &self.recording_run_id,
            capture: Capture {
                mode: "session",
                session_id: &self.recording_run_id,
            },
            code: Code {
                sha: self.code_sha.as_deref(),
                deja_version: deja::PKG_VERSION,
            },
            node,
        };
        let payload = serde_json::to_vec(&envelope).map_err(io::Error::other)?;

        // Partition alongside the request's boundary events when the span
        // carries the request id; background spans key on stream identity.
        let key = match node.request_id() {
            Some(request_id) => request_id.to_owned(),
            None => format!("{}:{}", self.recording_run_id, node.global_sequence),
        };

        let global_seq = node.global_sequence.to_string();
        let headers = OwnedHeaders::new()
            .insert(Header {
                key: "global_sequence",
                value: Some(global_seq.as_str()),
            })
            .insert(Header {
                key: "recording_run_id",
                value: Some(self.recording_run_id.as_str()),
            })
            .insert(Header {
                key: "span_name",
                value: Some(node.span_name.as_str()),
            });

        self.send(&key, &payload, headers)
    }
}

impl deja::RecordSink<deja::DejaRecord> for HyperswitchKafkaRecordSink {
    fn write_batch(&mut self, records: &[deja::DejaRecord]) -> io::Result<()> {
        for record in records {
            match record {
                deja::DejaRecord::BoundaryEvent(event) => self.write_boundary_event(event)?,
                deja::DejaRecord::GraphNode(node) => self.write_graph_node(node)?,
                // Record mode never produces observations; skip instead of
                // failing the writer if the library contract is breached.
                deja::DejaRecord::Observed(_) => {
                    debug_assert!(false, "observed record reached the record-mode Kafka sink");
                }
            }
        }
        Ok(())
    }

    fn flush(&mut self) -> io::Result<()> {
        // Cadence flush: a short bounded poll. Delivery happens continuously on
        // the producer's own background thread; a long synchronous drain here
        // would park the writer thread behind a slow broker, backing up the
        // record channel into the request path. Messages still in flight when
        // the poll expires are NOT a sink failure — enqueue errors in
        // write_batch are where saturation and broker loss actually surface.
        match self.producer.flush(CADENCE_FLUSH_POLL) {
            Ok(()) => Ok(()),
            Err(rdkafka::error::KafkaError::Flush(
                rdkafka::types::RDKafkaErrorCode::OperationTimedOut,
            )) => Ok(()),
            Err(e) => Err(io::Error::other(format!("kafka flush: {e}"))),
        }
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
            code: Code {
                sha: self.code_sha.as_deref(),
                deja_version: deja::PKG_VERSION,
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
        // The eof marker bounds delivery audits — drain for real so "eof
        // landed" means everything before it landed too.
        if matches!(kind, deja::MarkerKind::Eof) {
            let _ = self.producer.flush(EOF_FLUSH_TIMEOUT);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn envelope_serializes_artifact_record_v2_shape() {
        let event = deja::BoundaryEvent {
            global_sequence: 7,
            request_sequence: 2,
            correlation_id: Some("c-123".to_string()),
            timestamp_ns: 1_000_000_000,
            recording_run_id: Some("run-abc".to_string()),
            graph_node_id: None,
            tracing_span_id: None,
            task_id: None,
            parent_task_id: None,
            task_bucket: None,
            bucket_id: None,
            fork_seq: None,
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
            event_schema_version: deja::CURRENT_EVENT_SCHEMA_VERSION,
            callsite_identity: None,
            provenance: deja::Provenance::default(),
            fidelity: deja::Fidelity::default(),
            result_image: None,
            pre_image: None,
            read_set: Vec::new(),
            write_set: Vec::new(),
            value_digest: None,
            entropy_source: None,
            replay_strategy: deja::ReplayStrategy::Execute,
            kind: Some("redis".to_string()),
            declaration: Some(deja::BoundaryDeclaration::default().effect(deja::EffectKind::Redis)),
            raw_draw: None,
            end_timestamp_ns: Some(1_000_000_042),
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
        assert_eq!(
            value.pointer("/schema_version"),
            Some(&serde_json::json!(2))
        );
        assert_eq!(
            value.pointer("/artifact_type"),
            Some(&serde_json::json!("deja_artifact_record"))
        );
        assert_eq!(
            value.pointer("/instance_id"),
            Some(&serde_json::json!("router-testhost-1"))
        );
        assert_eq!(
            value.pointer("/recording_run_id"),
            Some(&serde_json::json!("run-abc"))
        );
        assert_eq!(
            value.pointer("/correlation_id"),
            Some(&serde_json::json!("c-123"))
        );
        assert_eq!(
            value.pointer("/event_time_ns"),
            Some(&serde_json::json!(1_000_000_000u64))
        );
        assert_eq!(
            value.pointer("/capture/mode"),
            Some(&serde_json::json!("session"))
        );
        assert_eq!(
            value.pointer("/capture/session_id"),
            Some(&serde_json::json!("run-abc"))
        );
        assert_eq!(
            value.pointer("/code/sha"),
            Some(&serde_json::json!("deadbeef"))
        );
        assert_eq!(
            value.pointer("/code/deja_version"),
            Some(&serde_json::json!(deja::PKG_VERSION))
        );
        assert_eq!(
            value.pointer("/event/boundary"),
            Some(&serde_json::json!("redis"))
        );
        assert_eq!(
            value.pointer("/event/method_name"),
            Some(&serde_json::json!("get_key"))
        );
        assert_eq!(
            value.pointer("/event/global_sequence"),
            Some(&serde_json::json!(7))
        );
        // The sink unwraps `DejaRecord` before enveloping: the payload is the
        // plain event, never the internally tagged record.
        assert!(value.pointer("/event/record_kind").is_none());
    }

    #[test]
    fn graph_envelope_serializes_graph_node_v1_shape() {
        let node = deja_core::ExecutionGraphNode {
            node_id: 7,
            global_sequence: 42,
            parent_id: Some(3),
            causal_parent_ids: vec![1],
            sequence: 5,
            recording_run_id: Some("run-abc".to_string()),
            span_name: "payment.request".to_string(),
            target: "router".to_string(),
            level: "INFO".to_string(),
            fields: [("request_id".to_string(), serde_json::json!("c-123"))]
                .into_iter()
                .collect(),
            started_ns: 1_000_000_000,
            closed_ns: Some(1_000_000_042),
        };
        let envelope = GraphEnvelope {
            schema_version: 1,
            artifact_type: "deja_graph_node",
            instance_id: "router-testhost-1",
            recording_run_id: "run-abc",
            capture: Capture {
                mode: "session",
                session_id: "run-abc",
            },
            code: Code {
                sha: Some("deadbeef"),
                deja_version: deja::PKG_VERSION,
            },
            node: &node,
        };
        let value: serde_json::Value =
            serde_json::from_slice(&serde_json::to_vec(&envelope).unwrap()).unwrap();
        assert_eq!(
            value.pointer("/schema_version"),
            Some(&serde_json::json!(1))
        );
        assert_eq!(
            value.pointer("/artifact_type"),
            Some(&serde_json::json!("deja_graph_node"))
        );
        assert_eq!(
            value.pointer("/instance_id"),
            Some(&serde_json::json!("router-testhost-1"))
        );
        assert_eq!(
            value.pointer("/recording_run_id"),
            Some(&serde_json::json!("run-abc"))
        );
        assert_eq!(
            value.pointer("/capture/mode"),
            Some(&serde_json::json!("session"))
        );
        assert_eq!(
            value.pointer("/capture/session_id"),
            Some(&serde_json::json!("run-abc"))
        );
        assert_eq!(
            value.pointer("/code/sha"),
            Some(&serde_json::json!("deadbeef"))
        );
        assert_eq!(
            value.pointer("/code/deja_version"),
            Some(&serde_json::json!(deja::PKG_VERSION))
        );
        assert_eq!(value.pointer("/node/node_id"), Some(&serde_json::json!(7)));
        assert_eq!(
            value.pointer("/node/global_sequence"),
            Some(&serde_json::json!(42))
        );
        assert_eq!(
            value.pointer("/node/span_name"),
            Some(&serde_json::json!("payment.request"))
        );
        assert_eq!(
            value.pointer("/node/fields/request_id"),
            Some(&serde_json::json!("c-123"))
        );
        assert_eq!(
            value.pointer("/node/closed_ns"),
            Some(&serde_json::json!(1_000_000_042u64))
        );
        assert!(value.pointer("/node/record_kind").is_none());
        // No event payload on graph envelopes — the compactor routes by type.
        assert!(value.get("event").is_none());
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
            code: Code {
                sha: Some("deadbeef"),
                deja_version: deja::PKG_VERSION,
            },
            marker: MarkerBody {
                kind: "eof",
                payload: &payload,
            },
        };
        let value: serde_json::Value =
            serde_json::from_slice(&serde_json::to_vec(&envelope).unwrap()).unwrap();
        assert_eq!(
            value.pointer("/artifact_type"),
            Some(&serde_json::json!("deja_sink_marker"))
        );
        assert_eq!(
            value.pointer("/marker/kind"),
            Some(&serde_json::json!("eof"))
        );
        assert_eq!(
            value.pointer("/marker/last_seq"),
            Some(&serde_json::json!(206))
        );
        assert_eq!(
            value.pointer("/code/sha"),
            Some(&serde_json::json!("deadbeef"))
        );
        assert_eq!(
            value.pointer("/code/deja_version"),
            Some(&serde_json::json!(deja::PKG_VERSION))
        );
        // No event payload on markers — the compactor skips them by type.
        assert!(value.get("event").is_none());
    }
}
