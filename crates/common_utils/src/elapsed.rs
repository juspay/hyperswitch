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
/// `on_miss` takes the real reading, and that arm is load-bearing rather than
/// defensive. Every tape recorded BEFORE this seam existed has no entry for it,
/// so replaying one against a candidate that does would miss here on the first
/// connector call — and the substitute default is to fail-stop, which would take
/// the whole correlation down. That would make this seam a regression for every
/// existing recording. Falling back to the live reading is exactly today's
/// behaviour, and it degrades to a leaf a scorer can still explain from the
/// execution graphs rather than to a stopped task. The miss is not swallowed:
/// the lookup emits its divergence before `on_miss` is reached.
#[cfg_attr(feature = "deja", track_caller)]
#[cfg_attr(
    feature = "deja",
    deja::time(
        component = "common_utils::elapsed",
        operation = "millis_since",
        codec = SerdeCodec,
        on_miss = started.elapsed().as_millis(),
    )
)]
pub fn millis_since(started: std::time::Instant) -> u128 {
    started.elapsed().as_millis()
}

#[cfg(test)]
mod tests {
    /// The codec is serde_json, and the value is a `u128`. serde_json represents
    /// `u128` exactly below `u64::MAX`, and a millisecond reading is many orders
    /// below it — but the type is wider than the guarantee, so the round trip is
    /// pinned rather than assumed.
    #[test]
    fn a_millisecond_reading_round_trips_through_the_codec() {
        for value in [0_u128, 1, 890, 4004, 1_788_799_789, u128::from(u64::MAX)] {
            let encoded = serde_json::to_string(&value).expect("a reading serializes");
            let decoded: u128 = serde_json::from_str(&encoded).expect("and comes back");
            assert_eq!(decoded, value, "a reading must survive the tape unchanged");
        }
    }
}
