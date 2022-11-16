//!
//! Setup logging subsystem.
//!
use std::{path::PathBuf, time::Duration};

use opentelemetry::{
    global,
    sdk::{
        metrics::{selectors, PushController},
        propagation::TraceContextPropagator,
        trace, Resource,
    },
    util::tokio_interval_stream,
    KeyValue,
};
use opentelemetry_otlp::WithExportConfig;
use tracing_appender::non_blocking::WorkerGuard;
// use tracing_subscriber::fmt::format::FmtSpan;
// use tracing_bunyan_formatter::JsonStorageLayer;
use tracing_subscriber::{
    filter, fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter, Layer,
};

use crate::{config, FormattingLayer, Level, StorageSubscription};

// FIXME: xxx: clean
pub struct DebugLayer;
impl<S> Layer<S> for DebugLayer
where
    S: tracing::Subscriber,
{
    fn on_event(
        &self,
        event: &tracing::Event<'_>,
        _ctx: tracing_subscriber::layer::Context<'_, S>,
    ) {
        if event.metadata().level() == &Level::TRACE {
            return;
        }
        println!("Got event!");
        println!("  level={:?}", event.metadata().level());
        println!("  target={:?}", event.metadata().target());
        println!("  name={:?}", event.metadata().name());
        for field in event.fields() {
            println!("  field={}", field.name());
        }
    }
}

/// TelemetryGuard which helps with
#[derive(Debug)]
pub struct TelemetryGuard {
    _log_guards: Vec<WorkerGuard>,
    _metric_controller: Option<PushController>,
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
        // let fmt_layer = fmt::layer()
        //     .with_writer(file_writer)
        //     .with_target(true)
        //     .with_level(true)
        //     .with_span_events(FmtSpan::ACTIVE)
        //     .json();

        // Some(fmt_layer)
        //Some(FormattingLayer::new(service_name, file_writer))
        Some(file_layer)
        // Some(BunyanFormattingLayer::new("router".into(), file_writer))
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

fn setup_metrics() -> Option<PushController> {
    opentelemetry_otlp::new_pipeline()
        .metrics(tokio::spawn, tokio_interval_stream)
        .with_exporter(
            opentelemetry_otlp::new_exporter().tonic().with_env(), // can also config it using with_* functions like the tracing part above.
        )
        .with_period(Duration::from_secs(3))
        .with_timeout(Duration::from_secs(10))
        .with_aggregator_selector(selectors::simple::Selector::Exact)
        .build()
        .map_err(|err| eprintln!("Failed to Setup Metrics with {:?}", err))
        .ok()
}
