//! Boot-time composition of the typed Deja runtime hook.
//!
//! The router owns the transport wiring: typed router settings select disabled,
//! Kafka recording, or lookup-table replay, and this module eagerly installs the
//! process-wide runtime hook before any boundary or logger layer can observe the
//! default environment-derived state.

use std::{path::PathBuf, sync::Arc};

use crate::{
    configs::settings::{DejaMode, DejaReplaySettings, DejaSettings},
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

/// The id a recording is known by when nothing configured one:
/// `rec-<short sha>-<MMDDhhmm>-<instance>`, e.g. `rec-dcb9f9e-07291352-a3`.
/// Without a known revision it falls back to the bare-timestamp form (with a
/// warning) rather than claiming a provenance it does not have.
fn fallback_run_id(settings: &DejaSettings) -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or(std::time::Duration::ZERO);
    match resolved_code_sha(settings) {
        Some(sha) => format!(
            "rec-{}-{}-{}",
            short_revision(&sha),
            recording_stamp(now.as_secs()),
            instance_discriminator(&resolved_instance_id(settings)),
        ),
        None => {
            router_env::logger::warn!(
                "deja: recording without a code revision — its id will carry no provenance. \
                 Set deja.identity.code_sha, or build with VERGEN_GIT_SHA."
            );
            format!("run-{}", now.as_nanos())
        }
    }
}

/// A git sha shortened to the length git itself uses for a short sha.
fn short_revision(sha: &str) -> String {
    sha.chars()
        .filter(char::is_ascii_alphanumeric)
        .take(7)
        .collect::<String>()
        .to_ascii_lowercase()
}

/// `MMDDhhmm` UTC; the instance discriminator separates recorders that start
/// in the same minute.
fn recording_stamp(unix_secs: u64) -> String {
    let (month, day) = civil_month_day(unix_secs / 86_400);
    let today = unix_secs % 86_400;
    format!(
        "{month:02}{day:02}{:02}{:02}",
        today / 3600,
        (today % 3600) / 60
    )
}

/// Two characters standing for the instance, so two pods that start in the
/// same minute do not share a recording id.
fn instance_discriminator(instance_id: &str) -> String {
    // FNV-1a, so the same pod is always the same two characters.
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in instance_id.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100_0000_01b3);
    }
    const ALPHABET: &[u8] = b"0123456789abcdefghijklmnopqrstuvwxyz";
    // `n % 36` always indexes the 36-byte alphabet; spelled out instead of
    // asserted because a boot-time naming helper must not panic.
    let pick = |n: u64| {
        usize::try_from(n % 36)
            .ok()
            .and_then(|i| ALPHABET.get(i))
            .map_or('0', |byte| char::from(*byte))
    };
    let a = pick(hash);
    let b = pick(hash / 36);
    format!("{a}{b}")
}

/// Days since the epoch to (month, day), civil-from-days.
fn civil_month_day(days_since_epoch: u64) -> (i64, i64) {
    // An impossible clock degrades to a valid date instead of panicking.
    let Some(z) = i64::try_from(days_since_epoch)
        .ok()
        .and_then(|days| days.checked_add(719_468))
    else {
        return (1, 1);
    };
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    (m, d)
}

fn configured_run_id(settings: &DejaSettings) -> String {
    settings
        .effective_run_id()
        .map(str::to_owned)
        .unwrap_or_else(|| fallback_run_id(settings))
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
    // No "unknown" placeholder: absence is a fact worth being able to observe.
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

fn install_record(settings: &DejaSettings, inherited_brokers: Option<&[String]>) -> InstallReport {
    let kafka = &settings.recording.kafka;
    let Some(topic) = kafka.effective_topic() else {
        return install_disabled(Some(
            "record mode requires deja.recording.kafka.topic".to_owned(),
        ));
    };

    // Broker resolution: an explicit deja broker list wins; an empty list
    // inherits the deployment's analytics Kafka brokers, so both producers
    // share cluster provisioning while remaining separate clients.
    let brokers: &[String] = if kafka.brokers.is_empty() {
        inherited_brokers.unwrap_or_default()
    } else {
        kafka.brokers.as_slice()
    };
    if brokers.is_empty() || brokers.iter().any(|broker| broker.trim().is_empty()) {
        return install_disabled(Some(
            "record mode requires Kafka brokers: set deja.recording.kafka.brokers, or \
             configure [events.kafka] brokers for the recording sink to inherit"
                .to_owned(),
        ));
    }

    let run_id = configured_run_id(settings);
    let sink = match HyperswitchKafkaRecordSink::new(HyperswitchKafkaRecordSinkConfig {
        brokers,
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
///
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
pub fn install(
    settings: &DejaSettings,
    inherited_brokers: Option<&[String]>,
) -> Result<InstallReport, String> {
    // Graph capture is coupled to the mode (the graph layer rides the installed
    // Record/Replay hook), so there is no separate graph dial to declare here.
    match &settings.mode {
        DejaMode::Disabled => Ok(install_disabled(None)),
        DejaMode::Record => Ok(install_record(settings, inherited_brokers)),
        DejaMode::Replay => install_replay(settings),
    }
}
