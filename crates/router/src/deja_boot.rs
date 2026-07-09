//! Boot-time composition of the typed Deja runtime hook.
//!
//! The router owns the transport wiring: typed router settings select disabled,
//! Kafka recording, or lookup-table replay, and this module eagerly installs the
//! process-wide runtime hook before any boundary or logger layer can observe the
//! default environment-derived state.

use std::{path::PathBuf, sync::Arc};

use crate::{
    configs::settings::{DejaGraphMode, DejaMode, DejaReplaySettings, DejaSettings},
    services::kafka::deja_record_sink::{
        HyperswitchKafkaRecordSink, HyperswitchKafkaRecordSinkConfig,
    },
};

#[derive(Debug, Clone)]
pub struct InstallReport {
    pub mode: &'static str,
    pub run_id: Option<String>,
    pub detail: Option<String>,
}

fn non_empty(value: Option<&str>) -> Option<&str> {
    value.map(str::trim).filter(|value| !value.is_empty())
}

fn fallback_run_id() -> String {
    let now_ns = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or(std::time::Duration::ZERO)
        .as_nanos();
    format!("run-{now_ns}")
}

fn configured_run_id(settings: &DejaSettings) -> String {
    settings
        .effective_run_id()
        .map(str::to_owned)
        .unwrap_or_else(fallback_run_id)
}

fn configured_value(value: Option<&str>) -> Option<String> {
    non_empty(value).map(str::to_owned)
}

fn env_value_named(name: &str) -> Option<String> {
    let name = non_empty(Some(name))?;
    configured_value(std::env::var(name).ok().as_deref())
}

fn fallback_instance_id() -> String {
    let now_ns = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or(std::time::Duration::ZERO)
        .as_nanos();
    format!("pi-{}-{now_ns}", std::process::id())
}

fn resolved_instance_id(settings: &DejaSettings) -> String {
    configured_value(settings.identity.instance_id.as_deref())
        .or_else(|| env_value_named(&settings.identity.pod_name_env))
        .unwrap_or_else(fallback_instance_id)
}

fn resolved_code_sha(settings: &DejaSettings) -> Option<String> {
    configured_value(settings.identity.code_sha.as_deref())
        .or_else(|| env_value_named(&settings.identity.git_sha_env))
        .or_else(|| option_env!("VERGEN_GIT_SHA").map(str::to_owned))
        .or_else(|| Some("unknown".to_owned()))
}

fn writer_config(settings: &DejaSettings) -> deja::WriterConfig {
    let writer = &settings.writer;
    deja::WriterConfig {
        queue_capacity: writer.queue_capacity.max(1),
        batch_size: writer.batch_size.max(1),
        flush_interval: std::time::Duration::from_millis(writer.flush_interval_ms.max(1)),
        flush_timeout: std::time::Duration::from_millis(writer.shutdown_flush_ms.max(1)),
        flush_after_records: (writer.flush_after_records > 0).then_some(writer.flush_after_records),
        policy: deja::SinkPolicy::FailOpen,
    }
}

fn disabled_report(detail: Option<String>) -> InstallReport {
    InstallReport {
        mode: "disabled",
        run_id: None,
        detail,
    }
}

#[allow(clippy::print_stderr)] // The logger may not be initialized yet.
fn print_configuration_error(error: &str) {
    eprintln!("deja configuration error: {error}; runtime hook disabled");
}

#[allow(clippy::print_stderr)] // The logger may not be initialized yet.
fn try_install_hook(
    hook: deja::RuntimeHook,
    report: InstallReport,
) -> Result<InstallReport, String> {
    deja::set_global_runtime_hook(Some(hook))
        .map_err(|error| error.to_owned())
        .map(|()| report)
}

#[allow(clippy::print_stderr)] // The logger may not be initialized yet.
fn install_hook(hook: deja::RuntimeHook, report: InstallReport) -> InstallReport {
    match try_install_hook(hook, report) {
        Ok(report) => report,
        Err(error) => {
            eprintln!(
                "deja configuration error: {error}; requested runtime hook was not installed"
            );
            disabled_report(Some(error))
        }
    }
}

fn install_disabled(detail: Option<String>) -> InstallReport {
    if let Some(error) = detail.as_deref() {
        print_configuration_error(error);
    }
    install_hook(
        deja::RuntimeHook::Disabled(deja::DisabledHook),
        disabled_report(detail),
    )
}

#[allow(clippy::print_stderr)] // The logger may not be initialized yet.
fn install_record(settings: &DejaSettings) -> InstallReport {
    if !settings.sampler.enabled {
        eprintln!(
            "deja: record mode with deja.sampler.enabled=false records NO requests; \
             enable the sampler (and set deja.sampler.fail_closed=false on rigs without \
             a superposition sampling source) to capture traffic"
        );
    }
    let kafka = &settings.recording.kafka;
    let Some(topic) = kafka.effective_topic() else {
        return install_disabled(Some(
            "record mode requires deja.recording.kafka.topic".to_owned(),
        ));
    };

    if kafka.brokers.is_empty() || kafka.brokers.iter().any(|broker| broker.trim().is_empty()) {
        return install_disabled(Some(
            "record mode requires non-empty deja.recording.kafka.brokers".to_owned(),
        ));
    }

    let run_id = configured_run_id(settings);
    let sink = match HyperswitchKafkaRecordSink::new(HyperswitchKafkaRecordSinkConfig {
        brokers: kafka.brokers.as_slice(),
        topic,
        recording_run_id: &run_id,
        instance_id: resolved_instance_id(settings),
        code_sha: resolved_code_sha(settings),
        client_id: kafka.client_id.as_deref(),
        acks: &kafka.acks,
        enable_idempotence: kafka.idempotence,
        compression: kafka.compression.as_deref(),
        linger_ms: kafka.linger,
        message_timeout_ms: kafka.message_timeout.unwrap_or(30_000),
        queue_buffering_max_messages: kafka.queue_buffering_max_messages,
    }) {
        Ok(sink) => sink,
        Err(error) => {
            return install_disabled(Some(format!(
                "failed to create Deja Kafka producer for topic '{topic}': {error}"
            )));
        }
    };

    let hook = Arc::new(deja::RecordingHook::with_sink(
        sink,
        run_id.clone(),
        writer_config(settings),
    ));
    install_hook(
        deja::RuntimeHook::Recording(hook),
        InstallReport {
            mode: "record",
            run_id: Some(run_id),
            detail: Some(format!("Kafka topic '{topic}'")),
        },
    )
}

/// Resolve the lookup-table path from `deja.replay.{source, lookup_dir}` with
/// ONE rule and no shape-guessing:
/// - absolute `source` → that file, `lookup_dir` ignored
/// - relative `source` → a file name under `lookup_dir` (required)
/// - `lookup_dir` alone → `<lookup_dir>/<run_id>.jsonl` (`run_id` required)
/// Anything else is a configuration error.
fn replay_lookup_path(
    settings: &DejaSettings,
    replay: &DejaReplaySettings,
) -> Result<PathBuf, String> {
    let lookup_dir = replay
        .lookup_dir
        .as_deref()
        .filter(|dir| !dir.as_os_str().is_empty());
    match (non_empty(replay.source.as_deref()), lookup_dir) {
        (Some(source), _) if PathBuf::from(&source).is_absolute() => Ok(PathBuf::from(source)),
        (Some(source), Some(lookup_dir)) => Ok(lookup_dir.join(source)),
        (Some(source), None) => Err(format!(
            "deja.replay.source '{source}' is relative; set deja.replay.lookup_dir or make it absolute"
        )),
        (None, Some(lookup_dir)) => match settings.effective_run_id() {
            Some(run_id) => Ok(lookup_dir.join(format!("{run_id}.jsonl"))),
            None => Err(
                "deja.replay.lookup_dir without deja.replay.source requires deja.run_id"
                    .to_owned(),
            ),
        },
        (None, None) => {
            Err("replay mode requires deja.replay.source or deja.replay.lookup_dir".to_owned())
        }
    }
}

fn install_replay(settings: &DejaSettings) -> Result<InstallReport, String> {
    let lookup_path = replay_lookup_path(settings, &settings.replay)?;

    let observed_sink = non_empty(settings.replay.observed_sink.as_deref());
    let hook = match observed_sink {
        Some(path) => match deja::FileObservedSink::create(path) {
            Ok(sink) => deja::LookupTableHook::from_source(
                deja::LocalFileLookupSource::new(lookup_path.clone()),
                sink,
            ),
            Err(error) => {
                return Err(format!(
                    "failed to open replay observed sink '{path}': {error}"
                ));
            }
        },
        None => deja::LookupTableHook::from_source(
            deja::LocalFileLookupSource::new(lookup_path.clone()),
            deja::InMemoryObservedSink::new(),
        ),
    };

    let hook = hook.map_err(|error| {
        format!(
            "failed to load replay lookup table '{}': {error}",
            lookup_path.display()
        )
    })?;
    let entries = hook.entry_count();

    try_install_hook(
        deja::RuntimeHook::LookupReplay(hook),
        InstallReport {
            mode: "replay",
            run_id: settings.effective_run_id().map(str::to_owned),
            detail: Some(format!(
                "lookup table '{}' with {entries} entries",
                lookup_path.display()
            )),
        },
    )
    .map_err(|error| format!("failed to install replay runtime hook: {error}"))
}

/// Compose and install the process-wide Deja runtime hook from typed settings.
///
/// A hook is installed for every configured mode. Record misconfiguration never
/// aborts router boot and never leaves the process to lazily infer a mode later:
/// invalid record configuration installs a disabled hook with a clear pre-logger
/// error. Replay misconfiguration is fail-loud and aborts boot with the replay
/// error before logger setup.
pub fn install(settings: &DejaSettings) -> Result<InstallReport, String> {
    deja::set_graph_recording_enabled(matches!(settings.recording.graph, DejaGraphMode::Enabled));
    match &settings.mode {
        DejaMode::Disabled => Ok(install_disabled(None)),
        DejaMode::Record => Ok(install_record(settings)),
        DejaMode::Replay => install_replay(settings),
    }
}
