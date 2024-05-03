//! Utilities to easily create opentelemetry contexts, meters and metrics.

/// Create a metrics [`Context`][Context] with the specified name.
///
/// [Context]: opentelemetry::Context
#[macro_export]
macro_rules! metrics_context {
    ($name:ident) => {
        pub(crate) static $name: once_cell::sync::Lazy<$crate::opentelemetry::Context> =
            once_cell::sync::Lazy::new($crate::opentelemetry::Context::current);
    };
}

/// Create a global [`Meter`][Meter] with the specified name and an optional description.
///
/// [Meter]: opentelemetry::metrics::Meter
#[macro_export]
macro_rules! global_meter {
    ($name:ident) => {
        static $name: once_cell::sync::Lazy<$crate::opentelemetry::metrics::Meter> =
            once_cell::sync::Lazy::new(|| $crate::opentelemetry::global::meter(stringify!($name)));
    };
    ($name:ident, $description:literal) => {
        static $name: once_cell::sync::Lazy<$crate::opentelemetry::metrics::Meter> =
            once_cell::sync::Lazy::new(|| $crate::opentelemetry::global::meter($description));
    };
}

/// Create a [`Counter`][Counter] metric with the specified name and an optional description,
/// associated with the specified meter. Note that the meter must be to a valid [`Meter`][Meter].
///
/// [Counter]: opentelemetry::metrics::Counter
/// [Meter]: opentelemetry::metrics::Meter
#[macro_export]
macro_rules! counter_metric {
    ($name:ident, $meter:ident) => {
        pub(crate) static $name: once_cell::sync::Lazy<
            $crate::opentelemetry::metrics::Counter<u64>,
        > = once_cell::sync::Lazy::new(|| $meter.u64_counter(stringify!($name)).init());
    };
    ($name:ident, $meter:ident, description:literal) => {
        pub(crate) static $name: once_cell::sync::Lazy<
            $crate::opentelemetry::metrics::Counter<u64>,
        > = once_cell::sync::Lazy::new(|| $meter.u64_counter($description).init());
    };
}

/// Create a [`Histogram`][Histogram] metric with the specified name and an optional description,
/// associated with the specified meter. Note that the meter must be to a valid [`Meter`][Meter].
///
/// [Histogram]: opentelemetry::metrics::Histogram
/// [Meter]: opentelemetry::metrics::Meter
#[macro_export]
macro_rules! histogram_metric {
    ($name:ident, $meter:ident) => {
        pub(crate) static $name: once_cell::sync::Lazy<
            $crate::opentelemetry::metrics::Histogram<f64>,
        > = once_cell::sync::Lazy::new(|| $meter.f64_histogram(stringify!($name)).init());
    };
    ($name:ident, $meter:ident, $description:literal) => {
        pub(crate) static $name: once_cell::sync::Lazy<
            $crate::opentelemetry::metrics::Histogram<f64>,
        > = once_cell::sync::Lazy::new(|| $meter.f64_histogram($description).init());
    };
}

/// Create a [`Histogram`][Histogram] u64 metric with the specified name and an optional description,
/// associated with the specified meter. Note that the meter must be to a valid [`Meter`][Meter].
///
/// [Histogram]: opentelemetry::metrics::Histogram
/// [Meter]: opentelemetry::metrics::Meter
#[macro_export]
macro_rules! histogram_metric_u64 {
    ($name:ident, $meter:ident) => {
        pub(crate) static $name: once_cell::sync::Lazy<
            $crate::opentelemetry::metrics::Histogram<u64>,
        > = once_cell::sync::Lazy::new(|| $meter.u64_histogram(stringify!($name)).init());
    };
    ($name:ident, $meter:ident, $description:literal) => {
        pub(crate) static $name: once_cell::sync::Lazy<
            $crate::opentelemetry::metrics::Histogram<u64>,
        > = once_cell::sync::Lazy::new(|| $meter.u64_histogram($description).init());
    };
}

/// Create a [`Histogram`][Histogram] i64 metric with the specified name and an optional description,
/// associated with the specified meter. Note that the meter must be to a valid [`Meter`][Meter].
///
/// [Histogram]: opentelemetry::metrics::Histogram
/// [Meter]: opentelemetry::metrics::Meter
#[macro_export]
macro_rules! histogram_metric_i64 {
    ($name:ident, $meter:ident) => {
        pub(crate) static $name: once_cell::sync::Lazy<
            $crate::opentelemetry::metrics::Histogram<i64>,
        > = once_cell::sync::Lazy::new(|| $meter.i64_histogram(stringify!($name)).init());
    };
    ($name:ident, $meter:ident, $description:literal) => {
        pub(crate) static $name: once_cell::sync::Lazy<
            $crate::opentelemetry::metrics::Histogram<i64>,
        > = once_cell::sync::Lazy::new(|| $meter.i64_histogram($description).init());
    };
}
