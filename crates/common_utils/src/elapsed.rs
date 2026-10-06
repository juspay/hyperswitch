//! Elapsed-time readings that reach a request the candidate builds.

#[cfg(feature = "deja")]
use crate::synth_shape::Synthesize;

/// Milliseconds since `started`, as a value a recording can serve back.
///
/// A duration a caller measures and then writes into an outgoing request is the
/// clock, not behaviour: a replayed call is served from tape and returns in
/// microseconds, so the number moves, the request body differs, and the call
/// fails to match its own recording at every address rank. Seaming the READING
/// keeps that narrow — the work is still really done and really timed, so a
/// candidate that genuinely stopped making the call still diverges.
///
/// `skip_all`: the parameter is an `Instant`, captured through `Debug` and
/// different on every call, so keeping it would re-key the site each time, the
/// lookup would never hit, and a fresh reading would land in every request. The
/// site is addressed by span path and occurrence instead.
///
/// `on_miss` is load-bearing: a tape recorded before this seam has no entry
/// here, so every call on an old tape misses and the substitute default is to
/// fail-stop. The arm is DERIVED rather than a live reading, because reading the
/// clock here would inject entropy at the site whose purpose is removing it.
#[cfg_attr(feature = "deja", track_caller)]
#[cfg_attr(
    feature = "deja",
    deja::time(
        component = "common_utils::elapsed",
        operation = "millis_since",
        codec = SerdeCodec,
        skip_all,
        on_miss = __deja_miss.elapsed_millis(),
    )
)]
pub fn millis_since(started: std::time::Instant) -> u128 {
    started.elapsed().as_millis()
}

#[cfg(all(test, feature = "deja"))]
mod tests {
    use crate::synth_shape::Synthesize;

    /// The value is a `u128` and serde_json represents it exactly only below
    /// `u64::MAX`, so the round trip is pinned rather than assumed — and pinned
    /// against what the MISS ARM produces, since that is what flows through the
    /// codec on a pre-seam tape.
    #[test]
    fn the_miss_arms_own_output_round_trips_through_the_codec() {
        let miss = |occurrence: u32| deja::SubstituteMiss {
            boundary: "time",
            component: "common_utils::elapsed",
            method: "millis_since",
            args: serde_json::json!({}),
            occurrence,
            correlation_id: None,
        };
        let derived: Vec<u128> = (0..4_u32).map(|n| miss(n).elapsed_millis()).collect();
        assert_eq!(
            derived,
            vec![0, 1, 2, 3],
            "the arm must advance by one per occurrence"
        );
        for value in derived.into_iter().chain([890, 4004, u128::from(u64::MAX)]) {
            let encoded = serde_json::to_string(&value).expect("a reading serializes");
            let decoded: u128 = serde_json::from_str(&encoded).expect("and comes back");
            assert_eq!(decoded, value, "a reading must survive the tape unchanged");
        }
    }
}
