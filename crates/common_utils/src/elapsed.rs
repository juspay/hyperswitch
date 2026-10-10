//! Elapsed-time readings that reach a request the candidate builds.

#[cfg(feature = "deja")]
use crate::synth_shape::Synthesize;

/// Milliseconds since `started`, as a value a recording can serve back.
///
/// Seamed so a measured duration written into a request replays as recorded.
/// `skip_all` because an `Instant` differs on every call; a miss derives a value
/// rather than reading the clock.
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

    /// The miss arm's `u128` survives a serde_json round trip.
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
