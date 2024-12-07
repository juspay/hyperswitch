//! Utilities to easily create opentelemetry contexts, meters and metrics.

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
        > = once_cell::sync::Lazy::new(|| $meter.u64_counter(stringify!($name)).build());
    };
    ($name:ident, $meter:ident, description:literal) => {
        pub(crate) static $name: once_cell::sync::Lazy<
            $crate::opentelemetry::metrics::Counter<u64>,
        > = once_cell::sync::Lazy::new(|| $meter.u64_counter($description).build());
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
        > = once_cell::sync::Lazy::new(|| $meter.f64_histogram(stringify!($name)).build());
    };
    ($name:ident, $meter:ident, $description:literal) => {
        pub(crate) static $name: once_cell::sync::Lazy<
            $crate::opentelemetry::metrics::Histogram<f64>,
        > = once_cell::sync::Lazy::new(|| $meter.f64_histogram($description).build());
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
        > = once_cell::sync::Lazy::new(|| $meter.u64_histogram(stringify!($name)).build());
    };
    ($name:ident, $meter:ident, $description:literal) => {
        pub(crate) static $name: once_cell::sync::Lazy<
            $crate::opentelemetry::metrics::Histogram<u64>,
        > = once_cell::sync::Lazy::new(|| $meter.u64_histogram($description).build());
    };
}

/// Create a [`Gauge`][Gauge] metric with the specified name and an optional description,
/// associated with the specified meter. Note that the meter must be to a valid [`Meter`][Meter].
///
/// [Gauge]: opentelemetry::metrics::Gauge
/// [Meter]: opentelemetry::metrics::Meter
#[macro_export]
macro_rules! gauge_metric {
    ($name:ident, $meter:ident) => {
        pub(crate) static $name: once_cell::sync::Lazy<$crate::opentelemetry::metrics::Gauge<u64>> =
            once_cell::sync::Lazy::new(|| $meter.u64_gauge(stringify!($name)).build());
    };
    ($name:ident, $meter:ident, description:literal) => {
        pub(crate) static $name: once_cell::sync::Lazy<$crate::opentelemetry::metrics::Gauge<u64>> =
            once_cell::sync::Lazy::new(|| $meter.u64_gauge($description).build());
    };
}

pub use helpers::add_attributes;

mod helpers {
    pub fn add_attributes<T, U>(attributes: U) -> Vec<opentelemetry::KeyValue>
    where
        T: Into<opentelemetry::Value>,
        U: IntoIterator<Item = (&'static str, T)>,
    {
        attributes
            .into_iter()
            .map(|(key, value)| opentelemetry::KeyValue::new(key, value))
            .collect::<Vec<_>>()
    }
}
