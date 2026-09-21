//! `std::collections`, with the hasher made explicit.
//!
//! [`std::collections::HashMap`] defaults its `S` parameter to
//! [`RandomState`][std::collections::hash_map::RandomState], which draws keys
//! from per-process entropy. Iteration order therefore differs between two runs
//! of one binary. That is invisible in ordinary use and fatal to replay: when a
//! map's order reaches a response body, the difference is charged to the
//! candidate's logic rather than to the hasher.
//!
//! These aliases change nothing but that default. Under `feature = "deja"` it
//! becomes [`CorrelationHasher`], whose keys derive from the current
//! correlation id, so two runs of one request iterate identically. Without the
//! feature the default stays `RandomState` and these are `std` verbatim.
//!
//! # What changing the import costs
//!
//! `.collect()`, `Default::default()`, `insert`, `iter` and the rest are
//! unaffected. `HashMap::new()` and `HashMap::with_capacity()` are **not**:
//! `std` defines them only for `RandomState`, so those call sites must become
//! `HashMap::default()` and `HashMap::with_capacity_and_hasher(n,
//! CorrelationHasher::default())`. That is a compile error, not a silent
//! change, which is the point — the collections that move are named by the
//! compiler rather than guessed at.

pub use std::collections::hash_map::RandomState;

#[cfg(not(feature = "deja"))]
/// The hasher these aliases default to.
pub type DefaultHashBuilder = RandomState;

#[cfg(feature = "deja")]
/// The hasher these aliases default to.
pub type DefaultHashBuilder = CorrelationHasher;

/// [`std::collections::HashMap`] with [`DefaultHashBuilder`] in place of
/// `std`'s implicit [`RandomState`].
pub type HashMap<K, V, S = DefaultHashBuilder> = std::collections::HashMap<K, V, S>;

/// [`std::collections::HashSet`] with [`DefaultHashBuilder`] in place of
/// `std`'s implicit [`RandomState`].
pub type HashSet<T, S = DefaultHashBuilder> = std::collections::HashSet<T, S>;

#[cfg(feature = "deja")]
mod correlation {
    use std::hash::{BuildHasher, Hasher};

    use siphasher::sip::SipHasher13;

    /// A [`BuildHasher`] keyed by the current correlation rather than by
    /// per-process entropy.
    ///
    /// `Default` is the only constructor because `std` calls it from inside
    /// `FromIterator`, where no call site could pass anything. Outside a
    /// correlation the keys are random: a collection built outside a request is
    /// never replayed, so determinism buys nothing there and would give up
    /// hash-flooding resistance for free.
    ///
    /// SipHash-1-3 from `siphasher`, not `DefaultHasher`, because `std`
    /// declines to guarantee its algorithm across releases and record and
    /// replay are different builds.
    #[derive(Clone, Copy, Debug)]
    pub struct CorrelationHasher {
        k0: u64,
        k1: u64,
    }

    /// Domain-separated so the two keys cannot coincide, which would halve the
    /// key space.
    fn keys_from_correlation(correlation: &str) -> (u64, u64) {
        let digest = |domain: u8| {
            let mut h = SipHasher13::new_with_keys(u64::from(domain), 0);
            h.write(correlation.as_bytes());
            h.finish()
        };
        (digest(0), digest(1))
    }

    fn random_keys() -> (u64, u64) {
        let draw = |salt: u8| {
            let mut h = std::collections::hash_map::RandomState::new().build_hasher();
            h.write_u8(salt);
            h.finish()
        };
        (draw(0), draw(1))
    }

    impl Default for CorrelationHasher {
        fn default() -> Self {
            // `try_` because this runs from arbitrary code, destructors
            // included, and a plain `with` on a destroyed thread-local panics
            // inside a drop, which aborts the process.
            let (k0, k1) = match deja::try_current_correlation_id() {
                Some(correlation) if order_is_replayed(&correlation) => {
                    keys_from_correlation(&correlation)
                }
                // `None` also covers a destroyed or busy cell. Busy is unreachable
                // today; if it becomes reachable it randomises a live correlation.
                _ => random_keys(),
            };
            Self { k0, k1 }
        }
    }

    /// Whether this request's iteration order will ever be compared.
    ///
    /// Derived keys are predictable to whoever can choose the correlation id, so
    /// they are used only for a request deja is recording or replaying. The
    /// feature being compiled in is not enough: it ships in release builds.
    fn order_is_replayed(correlation: &str) -> bool {
        !deja::runtime_mode_is_disabled() && sampled(deja::recording_decision(correlation))
    }

    /// No decision means no sampler is engaged (replay, or record-everything),
    /// and then every correlation counts.
    fn sampled(decision: Option<deja::RecordDecision>) -> bool {
        !matches!(decision, Some(deja::RecordDecision::Skip))
    }

    impl BuildHasher for CorrelationHasher {
        type Hasher = SipHasher13;

        fn build_hasher(&self) -> Self::Hasher {
            SipHasher13::new_with_keys(self.k0, self.k1)
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn one_correlation_always_derives_the_same_keys() {
            assert_eq!(
                keys_from_correlation("corr-1"),
                keys_from_correlation("corr-1"),
                "derivation must be a pure function of the id"
            );
        }

        #[test]
        fn different_correlations_derive_different_keys() {
            assert_ne!(
                keys_from_correlation("corr-1"),
                keys_from_correlation("corr-2"),
                "two requests must not share an order"
            );
        }

        #[test]
        fn the_two_keys_are_domain_separated() {
            let (k0, k1) = keys_from_correlation("corr-1");
            assert_ne!(k0, k1, "identical keys would halve the key space");
        }

        #[test]
        fn outside_a_correlation_the_keys_are_not_a_fixed_constant() {
            assert_ne!(
                random_keys(),
                random_keys(),
                "a fixed fallback would give up hash-flooding resistance for maps \
                 that are never replayed anyway"
            );
        }

        /// The recorder and the candidate are different builds. If the derivation
        /// ever changes between them, only a fixed value notices.
        #[test]
        fn the_derivation_is_pinned() {
            assert_eq!(
                keys_from_correlation("corr-1"),
                (3_063_536_665_707_766_459, 3_140_978_512_309_736_511)
            );
        }

        fn rendered(hasher: CorrelationHasher) -> String {
            let mut map = std::collections::HashMap::with_hasher(hasher);
            for key in 0..32_u32 {
                map.insert(key, ());
            }
            format!("{map:?}")
        }

        fn hasher_for(keys: (u64, u64)) -> CorrelationHasher {
            CorrelationHasher {
                k0: keys.0,
                k1: keys.1,
            }
        }

        /// The divergence itself: equal maps, rendered by two processes.
        #[test]
        fn random_keys_render_equal_maps_differently() {
            assert_ne!(
                rendered(hasher_for(random_keys())),
                rendered(hasher_for(random_keys()))
            );
        }

        #[test]
        fn one_correlation_renders_equal_maps_identically() {
            assert_eq!(
                rendered(hasher_for(keys_from_correlation("corr-1"))),
                rendered(hasher_for(keys_from_correlation("corr-1")))
            );
        }

        #[test]
        fn only_a_skipped_request_keeps_random_keys() {
            assert!(sampled(None));
            assert!(sampled(Some(deja::RecordDecision::Record)));
            assert!(!sampled(Some(deja::RecordDecision::Skip)));
        }

        /// The keys reach the hasher: two correlations must hash one value
        /// differently, or the derivation is decorative.
        #[test]
        fn the_derived_keys_actually_key_the_hasher() {
            let hash_with = |k: (u64, u64)| {
                let mut h = CorrelationHasher { k0: k.0, k1: k.1 }.build_hasher();
                h.write(b"same-input");
                h.finish()
            };
            assert_ne!(
                hash_with(keys_from_correlation("corr-1")),
                hash_with(keys_from_correlation("corr-2")),
                "two correlations must not hash one value alike"
            );
        }
    }
}

#[cfg(feature = "deja")]
pub use correlation::CorrelationHasher;
