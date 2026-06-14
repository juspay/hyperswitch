//! Boot-time composition of the deja recording hook with the Kafka transport.
//!
//! On the RECORD side this wires the candidate's `SemanticEvent` stream out
//! through a deja-owned hardened Kafka producer (see
//! `services::kafka::deja_record_sink`) so the downstream pipeline
//! (Vector → S3 → compactor) is the recording's durable path. Kafka is THE
//! sink — there is no JSONL primary on the record path; the execution graph
//! (a separate artifact) keeps its own file writer.
//!
//! Ordering matters: the macro-generated instrumentation reads the hook
//! through `deja::global_runtime_hook_from_env()`, whose backing `OnceLock`
//! locks in on first read. So composition MUST happen before that getter is
//! first called — `install` therefore runs in `main` before `start_server`,
//! and is a no-op for the replay/none paths (which the getter then
//! initializes lazily).
//!
//! Known test limitation: end-to-end Kafka delivery is not covered by tests
//! (it needs a live broker); only envelope serialization and the pure helpers
//! here are unit-tested.

use crate::{
    events::EventsConfig, services::kafka::deja_record_sink::HyperswitchKafkaRecordSink,
};

const DEFAULT_RECORDING_TOPIC: &str = "hyperswitch-deja-recording-events";

/// Resolve the recording topic: explicit config → `DEJA_KAFKA_TOPIC` env →
/// built-in default. Pure, so it is unit-testable without a broker.
fn resolve_topic(configured: Option<String>, env_topic: Option<String>) -> String {
    configured
        .filter(|t| !t.is_empty())
        .or_else(|| env_topic.filter(|t| !t.is_empty()))
        .unwrap_or_else(|| DEFAULT_RECORDING_TOPIC.to_owned())
}

/// Whether this process is a deja record candidate (`DEJA_MODE=record`).
/// Kafka is the only record sink, so record mode IS the request.
fn wants_recording() -> bool {
    std::env::var("DEJA_MODE").as_deref() == Ok("record")
}

/// Compose and install the process-wide deja runtime hook.
///
/// In record mode, installs `RecordingHook` over the hardened Kafka sink.
/// Otherwise does nothing and lets `global_runtime_hook_from_env()` lazily
/// initialize the replay/none hook. Every failure path degrades to "no
/// recording" with a warning — a misconfigured broker never aborts router
/// boot, and never fails application requests.
pub async fn install(events: &EventsConfig) {
    if !wants_recording() {
        return;
    }

    let kafka = match events {
        EventsConfig::Kafka { kafka } => kafka.as_ref(),
        EventsConfig::Logs => {
            router_env::tracing::warn!(
                target: "deja",
                "DEJA_MODE=record but events.source != kafka; recording DISABLED (Kafka is the only sink)"
            );
            return;
        }
    };

    let topic = resolve_topic(
        kafka.deja_recording_topic(),
        std::env::var("DEJA_KAFKA_TOPIC").ok(),
    );
    let recording_run_id = std::env::var("DEJA_RECORDING_RUN_ID")
        .ok()
        .or_else(|| std::env::var("DEJA_RUN_ID").ok())
        .unwrap_or_else(deja::RecordingHook::resolve_recording_run_id_default);

    let sink = match HyperswitchKafkaRecordSink::new(
        kafka.brokers(),
        topic.clone(),
        recording_run_id.clone(),
    ) {
        Ok(sink) => sink,
        Err(e) => {
            router_env::tracing::warn!(
                target: "deja",
                error = %e,
                "failed to create deja Kafka producer; recording DISABLED"
            );
            return;
        }
    };

    // Wrap in Arc so the runtime hook and `global_hook_from_env()` share ONE
    // recorder: every boundary (db/redis/http via the env getter, id-gen via the
    // runtime getter) then writes through this single sink — one
    // global_sequence counter, one Kafka stream. (See the doc comment on
    // `RuntimeHook::Recording` and `global_hook_from_env`.)
    let hook = std::sync::Arc::new(deja::RecordingHook::with_sink(
        sink,
        recording_run_id.clone(),
    ));

    match deja::set_global_runtime_hook(Some(deja::RuntimeHook::Recording(hook))) {
        Ok(()) => router_env::tracing::info!(
            target: "deja",
            topic = %topic,
            recording_run_id = %recording_run_id,
            "deja Kafka recording sink installed (acks=all, idempotent, marker-audited)"
        ),
        Err(e) => router_env::tracing::warn!(
            target: "deja",
            error = %e,
            "deja runtime hook already initialized; Kafka sink NOT installed"
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn topic_resolution_prefers_config_then_env_then_default() {
        assert_eq!(resolve_topic(Some("cfg".into()), Some("env".into())), "cfg");
        assert_eq!(resolve_topic(None, Some("env".into())), "env");
        // Empty config string is ignored in favour of the env value.
        assert_eq!(
            resolve_topic(Some(String::new()), Some("env".into())),
            "env"
        );
        assert_eq!(resolve_topic(None, None), DEFAULT_RECORDING_TOPIC);
    }
}
