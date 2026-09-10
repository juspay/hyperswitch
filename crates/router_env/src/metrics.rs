//! Utilities to easily create opentelemetry contexts, meters and metrics.

/// Create a global [`Meter`][Meter] with the specified name and an optional description.
///
/// [Meter]: opentelemetry::metrics::Meter
#[macro_export]
macro_rules! global_meter {
    ($name:ident) => {
        static $name: ::std::sync::LazyLock<$crate::opentelemetry::metrics::Meter> =
            ::std::sync::LazyLock::new(|| $crate::opentelemetry::global::meter(stringify!($name)));
    };
    ($meter:ident, $name:literal) => {
        static $meter: ::std::sync::LazyLock<$crate::opentelemetry::metrics::Meter> =
            ::std::sync::LazyLock::new(|| $crate::opentelemetry::global::meter(stringify!($name)));
    };
}

/// Create a [`Counter`][Counter] metric with the specified name and an optional description,
/// associated with the specified meter. Note that the meter must be a valid [`Meter`][Meter].
///
/// [Counter]: opentelemetry::metrics::Counter
/// [Meter]: opentelemetry::metrics::Meter
#[macro_export]
macro_rules! counter_metric {
    ($name:ident, $meter:ident) => {
        pub(crate) static $name: ::std::sync::LazyLock<
            $crate::opentelemetry::metrics::Counter<u64>,
        > = ::std::sync::LazyLock::new(|| $meter.u64_counter(stringify!($name)).build());
    };
    ($name:ident, $meter:ident, description:literal) => {
        #[doc = $description]
        pub(crate) static $name: ::std::sync::LazyLock<
            $crate::opentelemetry::metrics::Counter<u64>,
        > = ::std::sync::LazyLock::new(|| {
            $meter
                .u64_counter(stringify!($name))
                .with_description($description)
                .build()
        });
    };
    ($name:ident, $meter:ident, name: $metric_name:literal, description: $description:literal, unit: $unit:literal $(,)?) => {
        #[doc = $description]
        pub(crate) static $name: ::std::sync::LazyLock<
            $crate::opentelemetry::metrics::Counter<u64>,
        > = ::std::sync::LazyLock::new(|| {
            $meter
                .u64_counter($metric_name)
                .with_description($description)
                .with_unit($unit)
                .build()
        });
    };
}

/// Create a [`Histogram`][Histogram] f64 metric with the specified name and an optional description,
/// associated with the specified meter. Note that the meter must be a valid [`Meter`][Meter].
///
/// [Histogram]: opentelemetry::metrics::Histogram
/// [Meter]: opentelemetry::metrics::Meter
#[macro_export]
macro_rules! histogram_metric_f64 {
    ($name:ident, $meter:ident) => {
        pub(crate) static $name: ::std::sync::LazyLock<
            $crate::opentelemetry::metrics::Histogram<f64>,
        > = ::std::sync::LazyLock::new(|| {
            $meter
                .f64_histogram(stringify!($name))
                .with_boundaries($crate::metrics::f64_histogram_buckets())
                .build()
        });
    };
    ($name:ident, $meter:ident, $description:literal) => {
        #[doc = $description]
        pub(crate) static $name: ::std::sync::LazyLock<
            $crate::opentelemetry::metrics::Histogram<f64>,
        > = ::std::sync::LazyLock::new(|| {
            $meter
                .f64_histogram(stringify!($name))
                .with_description($description)
                .with_boundaries($crate::metrics::f64_histogram_buckets())
                .build()
        });
    };
    ($name:ident, $meter:ident, name: $metric_name:literal, description: $description:literal, unit: $unit:literal $(,)?) => {
        #[doc = $description]
        pub(crate) static $name: ::std::sync::LazyLock<
            $crate::opentelemetry::metrics::Histogram<f64>,
        > = ::std::sync::LazyLock::new(|| {
            $meter
                .f64_histogram($metric_name)
                .with_description($description)
                .with_unit($unit)
                .with_boundaries($crate::metrics::f64_histogram_buckets())
                .build()
        });
    };
}

/// Create a [`Histogram`][Histogram] u64 metric with the specified name and an optional description,
/// associated with the specified meter. Note that the meter must be a valid [`Meter`][Meter].
///
/// [Histogram]: opentelemetry::metrics::Histogram
/// [Meter]: opentelemetry::metrics::Meter
#[macro_export]
macro_rules! histogram_metric_u64 {
    ($name:ident, $meter:ident) => {
        pub(crate) static $name: ::std::sync::LazyLock<
            $crate::opentelemetry::metrics::Histogram<u64>,
        > = ::std::sync::LazyLock::new(|| {
            $meter
                .u64_histogram(stringify!($name))
                .with_boundaries($crate::metrics::f64_histogram_buckets())
                .build()
        });
    };
    ($name:ident, $meter:ident, $description:literal) => {
        #[doc = $description]
        pub(crate) static $name: ::std::sync::LazyLock<
            $crate::opentelemetry::metrics::Histogram<u64>,
        > = ::std::sync::LazyLock::new(|| {
            $meter
                .u64_histogram(stringify!($name))
                .with_description($description)
                .with_boundaries($crate::metrics::f64_histogram_buckets())
                .build()
        });
    };
}

/// Create a [`Gauge`][Gauge] metric with the specified name and an optional description,
/// associated with the specified meter. Note that the meter must be a valid [`Meter`][Meter].
///
/// [Gauge]: opentelemetry::metrics::Gauge
/// [Meter]: opentelemetry::metrics::Meter
#[macro_export]
macro_rules! gauge_metric {
    ($name:ident, $meter:ident) => {
        pub(crate) static $name: ::std::sync::LazyLock<$crate::opentelemetry::metrics::Gauge<u64>> =
            ::std::sync::LazyLock::new(|| $meter.u64_gauge(stringify!($name)).build());
    };
    ($name:ident, $meter:ident, description:literal) => {
        #[doc = $description]
        pub(crate) static $name: ::std::sync::LazyLock<$crate::opentelemetry::metrics::Gauge<u64>> =
            ::std::sync::LazyLock::new(|| {
                $meter
                    .u64_gauge(stringify!($name))
                    .with_description($description)
                    .build()
            });
    };
}

/// Create attributes to associate with a metric from key-value pairs.
#[macro_export]
macro_rules! metric_attributes {
    ($(($key:expr, $value:expr $(,)?)),+ $(,)?) => {
        &[$($crate::opentelemetry::KeyValue::new($key, $value)),+]
    };
}

pub use helpers::f64_histogram_buckets;

mod helpers {
    /// Returns the buckets to be used for a f64 histogram, in seconds.
    ///
    /// Seven buckets per doubling rather than one, so each is ~10% wider than the last and a
    /// percentile resolves to within ~10% of its true value instead of ~100%. The range starts at
    /// 100us — anything faster is instant for these purposes — and runs to ~52s, past the slowest
    /// connector timeout.
    #[inline(always)]
    pub fn f64_histogram_buckets() -> Vec<f64> {
        let growth_factor = 2_f64.powf(1.0 / 7.0);

        let mut init = 0.000_1;
        let mut buckets: [f64; 134] = [0.0; 134];

        for bucket in &mut buckets {
            *bucket = init;
            init *= growth_factor;
        }

        Vec::from(buckets)
    }
}
