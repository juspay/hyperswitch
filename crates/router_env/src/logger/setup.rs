//! Setup logging subsystem.

use std::time::Duration;

use ::config::ConfigError;
use serde_json::ser::{CompactFormatter, PrettyFormatter};
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{fmt, prelude::*, util::SubscriberInitExt, EnvFilter, Layer};

use crate::{config, FormattingLayer, StorageSubscription};

/// Contains guards necessary for logging and metrics collection.
#[derive(Debug)]
pub struct TelemetryGuard {
    _log_guards: Vec<WorkerGuard>,
}

/// Setup logging sub-system specifying the logging configuration, service (binary) name, and a
/// list of external crates for which a more verbose logging must be enabled. All crates within the
/// current cargo workspace are automatically considered for verbose logging.
#[allow(clippy::print_stdout)] // The logger hasn't been initialized yet
pub fn setup(
    config: &config::Log,
    service_name: &str,
    crates_to_filter: impl AsRef<[&'static str]>,
) -> error_stack::Result<TelemetryGuard, ConfigError> {
    let mut guards = Vec::new();

    // Setup OpenTelemetry traces and metrics
    let traces_layer = if config.telemetry.traces_enabled {
        setup_tracing_pipeline(&config.telemetry, service_name)
    } else {
        None
    };

    if config.telemetry.metrics_enabled {
        setup_metrics_pipeline(&config.telemetry)
    };

    // Setup file logging
    let file_writer = if config.file.enabled {
        let mut path = crate::env::workspace_path();
        // Using an absolute path for file log path would replace workspace path with absolute path,
        // which is the intended behavior for us.
        path.push(&config.file.path);

        let file_appender = tracing_appender::rolling::hourly(&path, &config.file.file_name);
        let (file_writer, guard) = tracing_appender::non_blocking(file_appender);
        guards.push(guard);

        let file_filter = get_envfilter(
            config.file.filtering_directive.as_ref(),
            config::Level(tracing::Level::WARN),
            config.file.level,
            &crates_to_filter,
        );
        println!("Using file logging filter: {file_filter}");
        let layer = FormattingLayer::new(service_name, file_writer, CompactFormatter)?
            .with_filter(file_filter);
        Some(layer)
    } else {
        None
    };

    let subscriber = tracing_subscriber::registry()
        .with(traces_layer)
        .with(StorageSubscription)
        .with(file_writer);

    // Setup console logging
    if config.console.enabled {
        let (console_writer, guard) = tracing_appender::non_blocking(std::io::stdout());
        guards.push(guard);

        let console_filter = get_envfilter(
            config.console.filtering_directive.as_ref(),
            config::Level(tracing::Level::WARN),
            config.console.level,
            &crates_to_filter,
        );
        println!("Using console logging filter: {console_filter}");

        match config.console.log_format {
            config::LogFormat::Default => {
                let logging_layer = fmt::layer()
                    .with_timer(fmt::time::time())
                    .pretty()
                    .with_writer(console_writer)
                    .with_filter(console_filter);
                subscriber.with(logging_layer).init();
            }
            config::LogFormat::Json => {
                error_stack::Report::set_color_mode(error_stack::fmt::ColorMode::None);
                subscriber
                    .with(
                        FormattingLayer::new(service_name, console_writer, CompactFormatter)?
                            .with_filter(console_filter),
                    )
                    .init();
            }
            config::LogFormat::PrettyJson => {
                error_stack::Report::set_color_mode(error_stack::fmt::ColorMode::None);
                subscriber
                    .with(
                        FormattingLayer::new(service_name, console_writer, PrettyFormatter::new())?
                            .with_filter(console_filter),
                    )
                    .init();
            }
        }
    } else {
        subscriber.init();
    };

    // Returning the TelemetryGuard for logs to be printed and metrics to be collected until it is
    // dropped
    Ok(TelemetryGuard {
        _log_guards: guards,
    })
}

fn get_opentelemetry_exporter_config(
    config: &config::LogTelemetry,
) -> opentelemetry_otlp::ExportConfig {
    let mut exporter_config = opentelemetry_otlp::ExportConfig {
        protocol: opentelemetry_otlp::Protocol::Grpc,
        endpoint: config.otel_exporter_otlp_endpoint.clone(),
        ..Default::default()
    };

    if let Some(timeout) = config.otel_exporter_otlp_timeout {
        exporter_config.timeout = Duration::from_millis(timeout);
    }

    exporter_config
}

#[derive(Debug, Clone)]
enum TraceUrlAssert {
    Match(String),
    EndsWith(String),
}

impl TraceUrlAssert {
    fn compare_url(&self, url: &str) -> bool {
        match self {
            Self::Match(value) => url == value,
            Self::EndsWith(end) => url.ends_with(end),
        }
    }
}

impl From<String> for TraceUrlAssert {
    fn from(value: String) -> Self {
        match value {
            url if url.starts_with('*') => Self::EndsWith(url.trim_start_matches('*').to_string()),
            url => Self::Match(url),
        }
    }
}

#[derive(Debug, Clone)]
struct TraceAssertion {
    clauses: Option<Vec<TraceUrlAssert>>,
    /// default behaviour for tracing if no condition is provided
    default: bool,
}

impl TraceAssertion {
    /// Should the provided url be traced
    fn should_trace_url(&self, url: &str) -> bool {
        match &self.clauses {
            Some(clauses) => clauses.iter().all(|cur| cur.compare_url(url)),
            None => self.default,
        }
    }
}

/// Conditional Sampler for providing control on url based tracing
#[derive(Clone, Debug)]
struct ConditionalSampler<T: opentelemetry_sdk::trace::ShouldSample + Clone + 'static>(
    TraceAssertion,
    T,
);

impl<T: opentelemetry_sdk::trace::ShouldSample + Clone + 'static>
    opentelemetry_sdk::trace::ShouldSample for ConditionalSampler<T>
{
    fn should_sample(
        &self,
        parent_context: Option<&opentelemetry::Context>,
        trace_id: opentelemetry::trace::TraceId,
        name: &str,
        span_kind: &opentelemetry::trace::SpanKind,
        attributes: &[opentelemetry::KeyValue],
        links: &[opentelemetry::trace::Link],
    ) -> opentelemetry::trace::SamplingResult {
        use opentelemetry::trace::TraceContextExt;

        match attributes
            .iter()
            .find(|&kv| kv.key == opentelemetry::Key::new("http.route"))
            .map_or(self.0.default, |inner| {
                self.0.should_trace_url(&inner.value.as_str())
            }) {
            true => {
                self.1
                    .should_sample(parent_context, trace_id, name, span_kind, attributes, links)
            }
            false => opentelemetry::trace::SamplingResult {
                decision: opentelemetry::trace::SamplingDecision::Drop,
                attributes: Vec::new(),
                trace_state: match parent_context {
                    Some(ctx) => ctx.span().span_context().trace_state().clone(),
                    None => opentelemetry::trace::TraceState::default(),
                },
            },
        }
    }
}

fn setup_tracing_pipeline(
    config: &config::LogTelemetry,
    service_name: &str,
) -> Option<
    tracing_opentelemetry::OpenTelemetryLayer<
        tracing_subscriber::Registry,
        opentelemetry_sdk::trace::Tracer,
    >,
> {
    use opentelemetry::trace::TracerProvider;
    use opentelemetry_otlp::WithExportConfig;
    use opentelemetry_sdk::trace;

    opentelemetry::global::set_text_map_propagator(
        opentelemetry_sdk::propagation::TraceContextPropagator::new(),
    );

    // Set the export interval to 1 second
    let batch_config = trace::BatchConfigBuilder::default()
        .with_scheduled_delay(Duration::from_millis(1000))
        .build();

    let exporter_result = opentelemetry_otlp::SpanExporter::builder()
        .with_tonic()
        .with_export_config(get_opentelemetry_exporter_config(config))
        .build();

    let exporter = if config.ignore_errors {
        #[allow(clippy::print_stderr)] // The logger hasn't been initialized yet
        exporter_result
            .inspect_err(|error| eprintln!("Failed to build traces exporter: {error:?}"))
            .ok()?
    } else {
        // Safety: This is conditional, there is an option to avoid this behavior at runtime.
        #[allow(clippy::expect_used)]
        exporter_result.expect("Failed to build traces exporter")
    };

    let mut provider_builder = trace::TracerProvider::builder()
        .with_span_processor(
            trace::BatchSpanProcessor::builder(
                exporter,
                // The runtime would have to be updated if a different web framework is used
                opentelemetry_sdk::runtime::TokioCurrentThread,
            )
            .with_batch_config(batch_config)
            .build(),
        )
        .with_sampler(trace::Sampler::ParentBased(Box::new(ConditionalSampler(
            TraceAssertion {
                clauses: config
                    .route_to_trace
                    .clone()
                    .map(|inner| inner.into_iter().map(TraceUrlAssert::from).collect()),
                default: false,
            },
            trace::Sampler::TraceIdRatioBased(config.sampling_rate.unwrap_or(1.0)),
        ))))
        .with_resource(opentelemetry_sdk::Resource::new(vec![
            opentelemetry::KeyValue::new("service.name", service_name.to_owned()),
        ]));

    if config.use_xray_generator {
        provider_builder = provider_builder
            .with_id_generator(opentelemetry_aws::trace::XrayIdGenerator::default());
    }

    Some(
        tracing_opentelemetry::layer()
            .with_tracer(provider_builder.build().tracer(service_name.to_owned())),
    )
}

fn setup_metrics_pipeline(config: &config::LogTelemetry) {
    use opentelemetry_otlp::WithExportConfig;

    let exporter_result = opentelemetry_otlp::MetricExporter::builder()
        .with_tonic()
        .with_temporality(opentelemetry_sdk::metrics::Temporality::Cumulative)
        .with_export_config(get_opentelemetry_exporter_config(config))
        .build();

    let exporter = if config.ignore_errors {
        #[allow(clippy::print_stderr)] // The logger hasn't been initialized yet
        exporter_result
            .inspect_err(|error| eprintln!("Failed to build metrics exporter: {error:?}"))
            .ok();
        return;
    } else {
        // Safety: This is conditional, there is an option to avoid this behavior at runtime.
        #[allow(clippy::expect_used)]
        exporter_result.expect("Failed to build metrics exporter")
    };

    let reader = opentelemetry_sdk::metrics::PeriodicReader::builder(
        exporter,
        // The runtime would have to be updated if a different web framework is used
        opentelemetry_sdk::runtime::TokioCurrentThread,
    )
    .with_interval(Duration::from_secs(3))
    .with_timeout(Duration::from_secs(10))
    .build();

    let provider = opentelemetry_sdk::metrics::SdkMeterProvider::builder()
        .with_reader(reader)
        .with_resource(opentelemetry_sdk::Resource::new([
            opentelemetry::KeyValue::new(
                "pod",
                std::env::var("POD_NAME").unwrap_or(String::from("hyperswitch-server-default")),
            ),
        ]))
        .build();

    opentelemetry::global::set_meter_provider(provider);
}

fn get_envfilter(
    filtering_directive: Option<&String>,
    default_log_level: config::Level,
    filter_log_level: config::Level,
    crates_to_filter: impl AsRef<[&'static str]>,
) -> EnvFilter {
    filtering_directive
        .map(|filter| {
            // Try to create target filter from specified filtering directive, if set

            // Safety: If user is overriding the default filtering directive, then we need to panic
            // for invalid directives.
            #[allow(clippy::expect_used)]
            EnvFilter::builder()
                .with_default_directive(default_log_level.into_level().into())
                .parse(filter)
                .expect("Invalid EnvFilter filtering directive")
        })
        .unwrap_or_else(|| {
            // Construct a default target filter otherwise
            let mut workspace_members = crate::cargo_workspace_members!();
            workspace_members.extend(crates_to_filter.as_ref());

            workspace_members
                .drain()
                .zip(std::iter::repeat(filter_log_level.into_level()))
                .fold(
                    EnvFilter::default().add_directive(default_log_level.into_level().into()),
                    |env_filter, (target, level)| {
                        // Safety: This is a hardcoded basic filtering directive. If even the basic
                        // filter is wrong, it's better to panic.
                        #[allow(clippy::expect_used)]
                        env_filter.add_directive(
                            format!("{target}={level}")
                                .parse()
                                .expect("Invalid EnvFilter directive format"),
                        )
                    },
                )
        })
}
