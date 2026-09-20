//! `std::collections`, with a seam for the collections whose iteration order
//! reaches the wire.
//!
//! # Why this module exists
//!
//! `HashMap` and `HashSet` iterate in an order derived from a per-process random
//! seed. That is invisible in ordinary use and fatal to replay: a candidate
//! iterating a map in a different order than the recording produces a different
//! response body, and the divergence is attributed to the candidate's logic when
//! its real cause is `RandomState`. Card networks are the standing example —
//! `business_profile.card_networks` is a `HashSet<CardNetwork>` whose order
//! reaches a response.
//!
//! # Why this is not a drop-in replacement
//!
//! `deja::hash_seed` deliberately has **no `Default` impl**, so a collection
//! cannot become seeded by accident and will not construct without the caller
//! naming its seed. That decision is load-bearing — a seed that nobody asked for
//! is a seed nobody can attribute — but it has a consequence this module cannot
//! paper over: `FromIterator for HashMap<K, V, S>` requires `S: BuildHasher +
//! Default`, so **`.collect()` cannot produce a seeded map**. Neither can
//! `HashMap::new()`.
//!
//! So migration is two phases, and only the first is mechanical:
//!
//! 1. **Change the import.** `use common_utils::collections::{HashMap, HashSet}`
//!    is a type alias for the `std` types with their `std` defaults. Every
//!    construction, every `.collect()`, every method keeps working, byte for
//!    byte. This phase is provably behaviour-preserving and is what makes the
//!    seam reachable from a call site later.
//! 2. **Name the collections that matter**, one at a time, with [`seeded_map`]
//!    and [`seeded_set`]. A named collection records its seed and replays it,
//!    so its order is reproducible across candidates.
//!
//! Phase 2 is deliberately per-site rather than wholesale. The set of
//! collections whose order reaches the wire is small, and it is found from
//! order-only body diffs rather than guessed at — seeding everything would spend
//! event volume on maps nobody can observe, and would make an order difference
//! harder to attribute rather than easier.
//!
//! # What a seed name means
//!
//! The name is the collection's identity across candidates: two candidates
//! replaying one tape resolve the same name to the same recorded keys and so
//! iterate identically. It is an `Explicit` address — the strongest deja has —
//! which is why it must be a literal and why it should describe the collection
//! rather than the call site. Name it for what it holds
//! (`"profile.card_networks"`), not for where it was built.

/// The hasher `std` uses. Named here so a signature can spell the default
/// without importing from `std::collections::hash_map`.
pub use std::collections::hash_map::RandomState;

/// `std::collections::HashMap`, re-exported so changing an import is a no-op.
///
/// The `S` parameter defaults to [`RandomState`], so `HashMap::new()`,
/// `.collect()` and every other `std` affordance behave exactly as before.
/// Pass a seed from [`seeded_map`] to get a reproducible iteration order.
pub type HashMap<K, V, S = RandomState> = std::collections::HashMap<K, V, S>;

/// `std::collections::HashSet`, re-exported so changing an import is a no-op.
///
/// See [`HashMap`] for why the default is `std`'s and what changes when it is
/// not.
pub type HashSet<T, S = RandomState> = std::collections::HashSet<T, S>;

#[cfg(feature = "deja")]
mod correlation {
    use std::hash::{BuildHasher, Hasher};

    /// A `BuildHasher` whose keys come from the CURRENT CORRELATION, not from
    /// per-process entropy.
    ///
    /// This is the piece that makes an import-only migration possible. A seed
    /// needs an address — something both the recording and the replay can resolve
    /// to the same keys — and `Default` has neither a name nor a usable call site
    /// (`FromIterator` invokes it from inside `std`, so every `.collect()` in the
    /// program reports one identical location). The correlation id IS that
    /// address, and it is already known on both sides, so nothing has to be
    /// recorded at all: no boundary event, no lookup, no miss policy. The seam
    /// collapses into a derivation.
    ///
    /// Outside a correlation the keys are RANDOM, deliberately. A collection
    /// built outside a request is never replayed, so determinism there buys
    /// nothing and would give up hash-flooding resistance for free. The one case
    /// that is not ambient — a long-lived map built at startup whose order later
    /// reaches a response — is served by [`super::seeded_map`], which names it.
    #[derive(Clone, Copy, Debug)]
    pub struct CorrelationHasher(deja::DejaBuildHasher);

    /// Two independent 64-bit keys from one correlation id.
    ///
    /// Domain-separated so `k0` and `k1` cannot coincide for any input, which
    /// would halve the key space.
    fn keys_from_correlation(correlation: &str) -> deja::HashKeys {
        fn digest(domain: u8, s: &str) -> u64 {
            use std::hash::Hasher;
            // `RandomState::new()` is per-process random; `DefaultHasher::new()`
            // is FIXED-key and therefore stable across processes, which is what
            // a derivation needs. That distinction is the whole point here.
            let mut h = std::collections::hash_map::DefaultHasher::new();
            h.write_u8(domain);
            h.write(s.as_bytes());
            h.write_u8(0xff);
            h.finish()
        }
        deja::HashKeys {
            k0: digest(0, correlation),
            k1: digest(1, correlation),
        }
    }

    /// Per-instance random keys, for the no-correlation case.
    fn random_keys() -> deja::HashKeys {
        let draw = |salt: u8| {
            let state = std::collections::hash_map::RandomState::new();
            let mut h = state.build_hasher();
            h.write_u8(salt);
            h.finish()
        };
        deja::HashKeys {
            k0: draw(0),
            k1: draw(1),
        }
    }

    impl Default for CorrelationHasher {
        fn default() -> Self {
            // `try_` because this runs from arbitrary code, destructors included,
            // and a plain `with` on a destroyed thread-local panics inside a drop
            // -- which ABORTS the process rather than failing a request.
            let keys = match deja::try_current_correlation_id() {
                Some(correlation) => keys_from_correlation(&correlation),
                None => random_keys(),
            };
            // `from_keys` rather than `hash_seed`: the keys are DERIVED, not
            // drawn, so there is nothing to record. `hash_seed` exists for the
            // named case where the keys must be captured and looked up.
            Self(deja::DejaBuildHasher::from_keys(keys))
        }
    }

    impl BuildHasher for CorrelationHasher {
        type Hasher = <deja::DejaBuildHasher as BuildHasher>::Hasher;
        fn build_hasher(&self) -> Self::Hasher {
            self.0.build_hasher()
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn one_correlation_always_derives_the_same_keys() {
            let a = keys_from_correlation("corr-1");
            let b = keys_from_correlation("corr-1");
            assert_eq!(
                (a.k0, a.k1),
                (b.k0, b.k1),
                "derivation must be a pure function of the id"
            );
        }

        #[test]
        fn different_correlations_derive_different_keys() {
            let a = keys_from_correlation("corr-1");
            let b = keys_from_correlation("corr-2");
            assert_ne!(
                (a.k0, a.k1),
                (b.k0, b.k1),
                "two requests must not share an order"
            );
        }

        #[test]
        fn the_two_keys_are_domain_separated() {
            let k = keys_from_correlation("corr-1");
            assert_ne!(k.k0, k.k1, "identical keys would halve the key space");
        }

        #[test]
        fn outside_a_correlation_the_keys_are_not_a_fixed_constant() {
            let a = random_keys();
            let b = random_keys();
            assert_ne!(
                (a.k0, a.k1),
                (b.k0, b.k1),
                "a fixed fallback would give up hash-flooding resistance for maps \
                 that are never replayed anyway"
            );
        }
    }
}

#[cfg(feature = "deja")]
mod seeded {
    // deja already names these; re-export rather than redefine, so there is one
    // spelling of a seeded collection across the codebase.
    pub use deja::{DejaBuildHasher, SeededHashMap, SeededHashSet};

    /// An empty map whose iteration order is reproducible under replay.
    ///
    /// `name` identifies the collection across candidates and must be a literal;
    /// see the module docs for how to choose one. Under record the seed is drawn
    /// and captured; under replay it is the recorded seed, or — if the recording
    /// never held this collection — one synthesized from the name, which is
    /// stable run to run so an order difference stays attributable.
    #[track_caller]
    pub fn seeded_map<K, V>(name: &'static str) -> SeededHashMap<K, V> {
        std::collections::HashMap::with_hasher(deja::hash_seed(name))
    }

    /// An empty set whose iteration order is reproducible under replay.
    ///
    /// See [`seeded_map`]; the same rules apply to the name.
    #[track_caller]
    pub fn seeded_set<T>(name: &'static str) -> SeededHashSet<T> {
        std::collections::HashSet::with_hasher(deja::hash_seed(name))
    }

    /// Collect an iterator into a seeded map.
    ///
    /// `.collect()` cannot do this: `FromIterator` requires the hasher to be
    /// `Default`, and a deja seed deliberately is not. This is the explicit
    /// replacement — it costs a named seed at the site, which is the whole
    /// point.
    #[track_caller]
    pub fn seeded_map_from<K, V, I>(name: &'static str, items: I) -> SeededHashMap<K, V>
    where
        K: std::hash::Hash + Eq,
        I: IntoIterator<Item = (K, V)>,
    {
        let mut map = seeded_map(name);
        map.extend(items);
        map
    }

    /// Collect an iterator into a seeded set. See [`seeded_map_from`].
    #[track_caller]
    pub fn seeded_set_from<T, I>(name: &'static str, items: I) -> SeededHashSet<T>
    where
        T: std::hash::Hash + Eq,
        I: IntoIterator<Item = T>,
    {
        let mut set = seeded_set(name);
        set.extend(items);
        set
    }
}

#[cfg(feature = "deja")]
#[cfg(feature = "deja")]
pub use correlation::CorrelationHasher;
#[cfg(feature = "deja")]
pub use seeded::{
    seeded_map, seeded_map_from, seeded_set, seeded_set_from, DejaBuildHasher, SeededHashMap,
    SeededHashSet,
};
