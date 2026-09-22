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
/// `on_miss` is load-bearing rather than defensive. Every tape recorded BEFORE
/// this seam existed has no entry for it, so replaying one against a candidate
/// that does would miss here on the first connector call — and the substitute
/// default is to fail-stop, which would take the whole correlation down. The
/// miss is not swallowed: the lookup emits its divergence before `on_miss` is
/// reached.
///
/// The arm is DERIVED, not the live reading it used to be. Reading the clock
/// here injects fresh entropy at the one site whose purpose is removing it: two
/// replays of a single unchanged candidate would disagree, and "the candidate
/// changed" could no longer be told from "the fabrication changed". `on_miss`
/// requires a function of the miss and not of anything ambient, and a live
/// `elapsed()` is ambient.
///
/// `monotonic` is `base + occurrence * step` and is unit-agnostic — the `_ns` in
/// its parameter names is the caller's convention, so a step of `1` here means
/// one MILLISECOND, matching this function's return. Base is zero rather than a
/// correlation's time origin: no such origin is available at a miss, and
/// inventing one would restore the ambient dependency this removes.
///
/// One consequence for a reader of an old replay: a derived value is no longer
/// explainable by the timing rule, which requires the observed leaf to equal the
/// floored duration of the replay span at the matched path, and `occurrence *
/// step` matches no span. So on a PRE-SEAM tape the downstream connector call's
/// leaf goes from explained to unexplained. That does not change the verdict — a
/// synthesized pure miss makes the run inconclusive with the site named — and it
/// does not touch post-seam tapes at all, where the seam hits and none of this
/// is reached.
#[cfg_attr(feature = "deja", track_caller)]
#[cfg_attr(
    feature = "deja",
    deja::time(
        component = "common_utils::elapsed",
        operation = "millis_since",
        codec = SerdeCodec,
        // Step of 1 (one millisecond) is deliberately BELOW the timing rule's
        // evidential floor (`MIN_EVIDENTIAL_MS`, juspay/deja#188). Raising it
        // above that floor would let a derived
        // value coincide with a span's floored duration in a busy correlation,
        // and the rule would then explain a fabrication as a measured reading.
        // The two constants are coupled; this one cannot move alone.
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
