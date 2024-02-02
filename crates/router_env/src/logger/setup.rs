//! Setup logging subsystem.

use std::time::Duration;

use opentelemetry::{
    global, runtime,
    sdk::{
        export::metrics::aggregation::cumulative_temporality_selector,
        metrics::{controllers::BasicController, selectors::simple},
        propagation::TraceContextPropagator,
        trace,
        trace::BatchConfig,
        Resource,
    },
    trace::{TraceContextExt, TraceState},
    KeyValue,
};
use opentelemetry_otlp::{TonicExporterBuilder, WithExportConfig};
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{fmt, prelude::*, util::SubscriberInitExt, EnvFilter, Layer};

use crate::{config, FormattingLayer, StorageSubscription};

/// Contains guards necessary for logging and metrics collection.
#[derive(Debug)]
pub struct TelemetryGuard {
    _log_guards: Vec<WorkerGuard>,
    _metrics_controller: Option<BasicController>,
}

/// Setup logging sub-system specifying the logging configuration, service (binary) name, and a
/// list of external crates for which a more verbose logging must be enabled. All crates within the
/// current cargo workspace are automatically considered for verbose logging.
pub fn setup(
    config: &config::Log,
    service_name: &str,
    crates_to_filter: impl AsRef<[&'static str]>,
) -> TelemetryGuard {
    let mut guards = Vec::new();

    // Setup OpenTelemetry traces and metrics
    let traces_layer = if config.telemetry.traces_enabled {
        setup_tracing_pipeline(&config.telemetry, service_name)
    } else {
        None
    };
    let _metrics_controller = if config.telemetry.metrics_enabled {
        setup_metrics_pipeline(&config.telemetry)
    } else {
        None
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

        Some(FormattingLayer::new(service_name, file_writer).with_filter(file_filter))
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
                let logging_layer =
                    FormattingLayer::new(service_name, console_writer).with_filter(console_filter);
                subscriber.with(logging_layer).init();
            }
        }
    } else {
        subscriber.init();
    };

    // Returning the TelemetryGuard for logs to be printed and metrics to be collected until it is
    // dropped
    TelemetryGuard {
        _log_guards: guards,
        _metrics_controller,
    }
}

/// Returns an OpenTelemetry exporter builder configured based on the provided LogTelemetry configuration.
fn get_opentelemetry_exporter(config: &config::LogTelemetry) -> TonicExporterBuilder {
    let mut exporter_builder = opentelemetry_otlp::new_exporter().tonic();

    if let Some(ref endpoint) = config.otel_exporter_otlp_endpoint {
        exporter_builder = exporter_builder.with_endpoint(endpoint);
    }
    if let Some(timeout) = config.otel_exporter_otlp_timeout {
        exporter_builder = exporter_builder.with_timeout(Duration::from_millis(timeout));
    }

    exporter_builder
}

#[derive(Debug, Clone)]
enum TraceUrlAssert {
    Match(String),
    EndsWith(String),
}

impl TraceUrlAssert {
        /// Compares the given URL with the value of the enum variant and returns true if they match or if the URL ends with the value of the enum variant, otherwise returns false.
    fn compare_url(&self, url: &str) -> bool {
        match self {
            Self::Match(value) => url == value,
            Self::EndsWith(end) => url.ends_with(end),
        }
    }
}

impl From<String> for TraceUrlAssert {
        /// This method takes a String value and returns an enum variant based on the value's prefix. If the value starts with '*', it returns the EndsWith variant with the trimmed value. Otherwise, it returns the Match variant with the original value.
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
    ///
    /// Should the provided url be traced
    ///
    fn should_trace_url(&self, url: &str) -> bool {
        match &self.clauses {
            Some(clauses) => clauses.iter().all(|cur| cur.compare_url(url)),
            None => self.default,
        }
    }
}

///
/// Conditional Sampler for providing control on url based tracing
///
#[derive(Clone, Debug)]
struct ConditionalSampler<T: trace::ShouldSample + Clone + 'static>(TraceAssertion, T);

impl<T: trace::ShouldSample + Clone + 'static> trace::ShouldSample for ConditionalSampler<T> {
        /// Determines whether a span should be sampled based on the provided parameters and the attributes of the span. If the span should be sampled, it delegates to the next sampler; otherwise, it returns a SamplingResult with the decision to drop the span.
    fn should_sample(
        &self,
        parent_context: Option<&opentelemetry::Context>,
        trace_id: opentelemetry::trace::TraceId,
        name: &str,
        span_kind: &opentelemetry::trace::SpanKind,
        attributes: &opentelemetry::trace::OrderMap<opentelemetry::Key, opentelemetry::Value>,
        links: &[opentelemetry::trace::Link],
        instrumentation_library: &opentelemetry::InstrumentationLibrary,
    ) -> opentelemetry::trace::SamplingResult {
        match attributes
            .get(&opentelemetry::Key::new("http.route"))
            .map_or(self.0.default, |inner| {
                self.0.should_trace_url(&inner.as_str())
            }) {
            true => self.1.should_sample(
                parent_context,
                trace_id,
                name,
                span_kind,
                attributes,
                links,
                instrumentation_library,
            ),
            false => opentelemetry::trace::SamplingResult {
                decision: opentelemetry::trace::SamplingDecision::Drop,
                attributes: Vec::new(),
                trace_state: match parent_context {
                    Some(ctx) => ctx.span().span_context().trace_state().clone(),
                    None => TraceState::default(),
                },
            },
        }
    }
}

/// Sets up a tracing pipeline with the specified configuration and service name, returning an optional OpenTelemetryLayer.
fn setup_tracing_pipeline(
    config: &config::LogTelemetry,
    service_name: &str,
) -> Option<tracing_opentelemetry::OpenTelemetryLayer<tracing_subscriber::Registry, trace::Tracer>>
{
    global::set_text_map_propagator(TraceContextPropagator::new());

    let mut trace_config = trace::config()
        .with_sampler(trace::Sampler::ParentBased(Box::new(ConditionalSampler(
            TraceAssertion {
                clauses: config
                    .route_to_trace
                    .clone()
                    .map(|inner| inner.into_iter().map(Into::into).collect()),
                default: false,
            },
            trace::Sampler::TraceIdRatioBased(config.sampling_rate.unwrap_or(1.0)),
        ))))
        .with_resource(Resource::new(vec![KeyValue::new(
            "service.name",
            service_name.to_owned(),
        )]));
    if config.use_xray_generator {
        trace_config = trace_config.with_id_generator(trace::XrayIdGenerator::default());
    }

    // Change the default export interval from 5 seconds to 1 second
    let batch_config = BatchConfig::default().with_scheduled_delay(Duration::from_millis(1000));

    let traces_layer_result = opentelemetry_otlp::new_pipeline()
        .tracing()
        .with_exporter(get_opentelemetry_exporter(config))
        .with_batch_config(batch_config)
        .with_trace_config(trace_config)
        .install_batch(opentelemetry::runtime::TokioCurrentThread)
        .map(|tracer| tracing_opentelemetry::layer().with_tracer(tracer));

    if config.ignore_errors {
        traces_layer_result
            .map_err(|error| {
                eprintln!("Failed to create an `opentelemetry_otlp` tracer: {error:?}")
            })
            .ok()
    } else {
        // Safety: This is conditional, there is an option to avoid this behavior at runtime.
        #[allow(clippy::expect_used)]
        Some(traces_layer_result.expect("Failed to create an `opentelemetry_otlp` tracer"))
    }
}

/// Sets up a metrics pipeline for logging telemetry based on the provided configuration. It creates a histogram with specified buckets, configures an OTLP pipeline with the given exporter and other parameters, and returns a `BasicController` for managing the created metrics pipeline. If `ignore_errors` is set to true in the configuration, any errors encountered during pipeline setup are logged and the method returns `None`. Otherwise, it returns `Some` containing the `BasicController` or panics if the pipeline setup fails.
fn setup_metrics_pipeline(config: &config::LogTelemetry) -> Option<BasicController> {
    let histogram_buckets = {
        let mut init = 0.01;
        let mut buckets: [f64; 15] = [0.0; 15];

        for bucket in &mut buckets {
            init *= 2.0;
            *bucket = init;
        }
        buckets
    };

    let metrics_controller_result = opentelemetry_otlp::new_pipeline()
        .metrics(
            simple::histogram(histogram_buckets),
            cumulative_temporality_selector(),
            // This would have to be updated if a different web framework is used
            runtime::TokioCurrentThread,
        )
        .with_exporter(get_opentelemetry_exporter(config))
        .with_period(Duration::from_secs(3))
        .with_timeout(Duration::from_secs(10))
        .with_resource(Resource::new(vec![KeyValue::new(
            "pod",
            std::env::var("POD_NAME").map_or(
                "hyperswitch-server-default".into(),
                Into::<opentelemetry::Value>::into,
            ),
        )]))
        .build();

    if config.ignore_errors {
        metrics_controller_result
            .map_err(|error| eprintln!("Failed to setup metrics pipeline: {error:?}"))
            .ok()
    } else {
        // Safety: This is conditional, there is an option to avoid this behavior at runtime.
        #[allow(clippy::expect_used)]
        Some(metrics_controller_result.expect("Failed to setup metrics pipeline"))
    }
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
