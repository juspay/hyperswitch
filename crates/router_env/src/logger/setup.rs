//!
//! Setup logging subsystem.
//!
use std::{path::PathBuf, time::Duration};

use once_cell::sync::Lazy;
use opentelemetry::{
    global, runtime,
    sdk::{
        export::metrics::aggregation::cumulative_temporality_selector,
        metrics::{controllers::BasicController, selectors::simple},
        propagation::TraceContextPropagator,
        trace, Resource,
    },
    KeyValue,
};
use opentelemetry_otlp::WithExportConfig;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{filter, fmt, prelude::*, util::SubscriberInitExt, EnvFilter, Layer};

use crate::{config, FormattingLayer, Level, StorageSubscription};

/// TelemetryGuard which helps with
#[derive(Debug)]
pub struct TelemetryGuard {
    _log_guards: Vec<WorkerGuard>,
    _metric_controller: Option<BasicController>,
}

///
/// Setup logging sub-system specifying.
/// Expects config and list of names of crates to watch.
///
pub fn setup<Str: AsRef<str>>(
    conf: &config::Log,
    service_name: &str,
    crates_to_watch: Vec<Str>,
) -> Result<TelemetryGuard, opentelemetry::metrics::MetricsError> {
    let mut guards = Vec::new();

    global::set_text_map_propagator(TraceContextPropagator::new());

    let telemetry = if conf.telemetry.enabled {
        let trace_config = trace::config()
            .with_sampler(trace::Sampler::TraceIdRatioBased(
                conf.telemetry.sampling_rate.unwrap_or(1.0),
            ))
            .with_resource(Resource::new(vec![KeyValue::new("service.name", "router")]));
        let tracer = opentelemetry_otlp::new_pipeline()
            .tracing()
            .with_exporter(opentelemetry_otlp::new_exporter().tonic().with_env())
            .with_trace_config(trace_config)
            .install_simple();

        Some(tracer)
    } else {
        None
    };

    let file_writer = if conf.file.enabled {
        let mut path: PathBuf = conf.file.path.clone().into();
        path.push(crate::env::workspace_path());
        path.push(&conf.file.path);
        // println!("{:?} + {:?}", &path, &conf.file.file_name);
        let file_appender = tracing_appender::rolling::hourly(&path, &conf.file.file_name);
        let (file_writer, guard) = tracing_appender::non_blocking(file_appender);
        guards.push(guard);

        let file_filter = filter::Targets::new().with_default(conf.file.level.into_level());
        let file_layer = FormattingLayer::new(service_name, file_writer).with_filter(file_filter);
        Some(file_layer)
    } else {
        None
    };

    let telemetry_layer = match telemetry {
        Some(Ok(ref tracer)) => Some(tracing_opentelemetry::layer().with_tracer(tracer.clone())),
        _ => None,
    };

    // Use 'RUST_LOG' environment variable will override the config settings
    let subscriber = tracing_subscriber::registry()
        .with(telemetry_layer)
        .with(StorageSubscription)
        .with(file_writer)
        .with(
            EnvFilter::builder()
                .with_default_directive(Level::TRACE.into())
                .from_env_lossy(),
        );

    if conf.console.enabled {
        let (console_writer, guard) = tracing_appender::non_blocking(std::io::stdout());
        guards.push(guard);

        let level = conf.console.level.into_level();
        let mut console_filter = filter::Targets::new().with_default(Level::WARN);
        for acrate in crates_to_watch {
            console_filter = console_filter.with_target(acrate.as_ref(), level);
        }

        match conf.console.log_format {
            config::LogFormat::Default => {
                let logging_layer = fmt::layer()
                    .with_timer(fmt::time::time())
                    .with_span_events(fmt::format::FmtSpan::ACTIVE)
                    .pretty()
                    .with_writer(console_writer);

                subscriber
                    .with(logging_layer.with_filter(console_filter))
                    .init();
            }
            config::LogFormat::Json => {
                let logging_layer = FormattingLayer::new(service_name, console_writer);

                subscriber.with(logging_layer).init();
            }
        }
    } else {
        subscriber.init();
    };

    if let Some(Err(err)) = telemetry {
        tracing::error!("Failed to create an opentelemetry_otlp tracer: {err}");
    }

    // Returning the WorkerGuard for logs to be printed until it is dropped
    Ok(TelemetryGuard {
        _log_guards: guards,
        _metric_controller: setup_metrics(),
    })
}

static HISTOGRAM_BUCKETS: Lazy<[f64; 15]> = Lazy::new(|| {
    let mut init = 0.01;
    let mut buckets: [f64; 15] = [0.0; 15];

    for bucket in &mut buckets {
        init *= 2.0;
        *bucket = init;
    }
    buckets
});

fn setup_metrics() -> Option<BasicController> {
    opentelemetry_otlp::new_pipeline()
        .metrics(
            simple::histogram(*HISTOGRAM_BUCKETS),
            cumulative_temporality_selector(),
            runtime::TokioCurrentThread,
        )
        .with_exporter(
            opentelemetry_otlp::new_exporter().tonic().with_env(), // can also config it using with_* functions like the tracing part above.
        )
        .with_period(Duration::from_secs(3))
        .with_timeout(Duration::from_secs(10))
        .build()
        .map_err(|err| eprintln!("Failed to Setup Metrics with {err:?}"))
        .ok()
}
