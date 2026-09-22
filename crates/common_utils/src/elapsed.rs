//! Elapsed-time readings that reach a request the candidate builds.

/// Milliseconds since `started`, as a value a recording can serve back.
///
/// A duration a caller measures and then writes into an outgoing request is the
/// clock, not behaviour. The live call took seconds; a replayed one is served
/// from tape and returns in microseconds, so the number moves, the request body
/// differs, and the call fails to match its own recording at every address rank.
///
/// Seaming the READING rather than the clock keeps that narrow. The work is
/// still really done and still really timed; only the number handed back is
/// substituted, so a recording measures live latency and a replay reproduces the
/// body it built when it did. A candidate that genuinely stopped making the call
/// still diverges, because nothing else about the call is touched.
///
/// `skip_all`: the parameter is an `Instant`, captured through `Debug` and
/// different on every call, so keeping it would re-key the site each time and
/// the lookup would never hit — putting a fresh reading in every request. The
/// site is addressed by span path and occurrence instead.
///
/// `on_miss` is load-bearing: a tape recorded before this seam has no entry
/// here, so every call on an old tape misses, and the substitute default is to
/// fail-stop. The miss is not swallowed — the lookup emits its divergence first.
///
/// The arm is DERIVED rather than a live reading, because reading the clock here
/// would inject fresh entropy at the site whose purpose is removing it: two
/// replays of one unchanged candidate would disagree. `monotonic` is unit
/// agnostic, so a step of `1` means one MILLISECOND, and base is zero because a
/// miss carries no correlation origin to start from.
#[cfg_attr(feature = "deja", track_caller)]
#[cfg_attr(
    feature = "deja",
    deja::time(
        component = "common_utils::elapsed",
        operation = "millis_since",
        codec = SerdeCodec,
        skip_all,
        // The step is deliberately below the timing rule's evidential floor
        // (`MIN_EVIDENTIAL_MS`, juspay/deja#188); above it, a derived value could
        // coincide with a span's duration and read as a measured one.
        on_miss = u128::try_from(deja::synth::monotonic(&__deja_miss, 0, 1)).unwrap_or(0),
    )
)]
pub fn millis_since(started: std::time::Instant) -> u128 {
    started.elapsed().as_millis()
}

#[cfg(test)]
mod tests {
    /// The codec is serde_json and the value is a `u128`, which serde_json
    /// represents exactly below `u64::MAX` — a millisecond reading is many
    /// orders below it, but the type is wider than the guarantee, so the round
    /// trip is pinned rather than assumed.
    ///
    /// Pinned against what the MISS ARM actually produces, not against chosen
    /// literals: the arm is what flows through the codec on a pre-seam tape, so
    /// a round trip that never sees its output is testing the wrong value.
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
        let derived: Vec<u128> = (0..4_u32)
            .map(|n| u128::try_from(deja::synth::monotonic(&miss(n), 0, 1)).unwrap_or(0))
            .collect();
        assert_eq!(derived, vec![0, 1, 2, 3], "the arm must advance by one per occurrence");
        for value in derived.into_iter().chain([890, 4004, u128::from(u64::MAX)]) {
            let encoded = serde_json::to_string(&value).expect("a reading serializes");
            let decoded: u128 = serde_json::from_str(&encoded).expect("and comes back");
            assert_eq!(decoded, value, "a reading must survive the tape unchanged");
        }
    }
}
