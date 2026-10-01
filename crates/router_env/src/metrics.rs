//! Utilities to easily create opentelemetry contexts, meters and metrics.

/// Create a global [`Meter`][Meter] with the specified name and an optional description.
///
/// [Meter]: opentelemetry::metrics::Meter
#[macro_export]
macro_rules! global_meter {
    ($name:ident) => {
        pub(crate) static $name: ::std::sync::LazyLock<$crate::opentelemetry::metrics::Meter> =
            ::std::sync::LazyLock::new(|| $crate::opentelemetry::global::meter(stringify!($name)));
    };
    ($meter:ident, $name:literal) => {
        pub(crate) static $meter: ::std::sync::LazyLock<$crate::opentelemetry::metrics::Meter> =
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
    ($name:ident, $meter:ident, boundaries: $boundaries:expr $(,)?) => {
        pub(crate) static $name: ::std::sync::LazyLock<
            $crate::opentelemetry::metrics::Histogram<f64>,
        > = ::std::sync::LazyLock::new(|| {
            $meter
                .f64_histogram(stringify!($name))
                .with_boundaries($boundaries)
                .build()
        });
    };
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

pub use helpers::{f64_histogram_buckets, latency_histogram_buckets_seconds};

mod helpers {
    /// Returns finer boundaries for floating-point latency histograms, in seconds.
    ///
    /// Seven intervals per doubling provide approximately 10.4% spacing, from 100us to
    /// 52.4288s. Values outside this range still contribute to the histogram, but have
    /// no finer resolution below the first boundary or above the last boundary.
    /// More boundaries increase aggregation memory and exported classic-histogram series.
    #[inline(always)]
    pub fn latency_histogram_buckets_seconds() -> Vec<f64> {
        let growth_factor = 2_f64.powf(1.0 / 7.0);
        let mut boundary = 0.000_1;
        let mut buckets = [0.0; 134];

        for bucket in &mut buckets {
            *bucket = boundary;
            boundary *= growth_factor;
        }

        Vec::from(buckets)
    }

    /// Preserves the historical default boundaries for existing histograms.
    #[inline(always)]
    pub fn f64_histogram_buckets() -> Vec<f64> {
        let mut init = 0.000_001;
        let mut buckets: [f64; 30] = [0.0; 30];

        for bucket in &mut buckets {
            *bucket = init;
            init *= 2.0;
        }

        Vec::from(buckets)
    }

    #[cfg(test)]
    mod tests {
        use super::{f64_histogram_buckets, latency_histogram_buckets_seconds};

        #[test]
        fn latency_boundaries_cover_the_expected_range() {
            let buckets = latency_histogram_buckets_seconds();
            assert_eq!(buckets.len(), 134);
            assert!((buckets[0] - 0.000_1).abs() < 1e-12);
            assert!((buckets[133] - 52.4288).abs() < 1e-9);
            assert!(buckets
                .iter()
                .all(|boundary| boundary.is_finite() && *boundary > 0.0));
        }

        #[test]
        fn latency_boundaries_have_seven_intervals_per_doubling() {
            let buckets = latency_histogram_buckets_seconds();
            for pair in buckets.windows(2) {
                assert!(pair[1] > pair[0]);
                assert!(pair[1] / pair[0] < 1.105);
            }
            for interval in buckets.windows(8) {
                assert!((interval[7] / interval[0] - 2.0).abs() < 1e-12);
            }
        }

        #[test]
        fn default_histograms_keep_the_original_boundaries() {
            let expected: Vec<f64> = (0..30).map(|index| 0.000_001 * 2_f64.powi(index)).collect();
            assert_eq!(f64_histogram_buckets(), expected);
        }
    }
}
