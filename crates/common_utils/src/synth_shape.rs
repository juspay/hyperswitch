//! Deterministic stand-ins for a generator's output when a replay finds no
//! recorded value.
//!
//! [`deja::synth::id`] is the usual answer, and the right one where the shape is
//! unconstrained — it carries a visible marker. It is wrong here: its marker
//! contains `-` and its length is fixed, while `consts::ALPHABETS` forbids `-`
//! and `_`, `generate_random_numeric_string` promises digits, and connectors put
//! a nanoid on the wire. Deja's rule is that a value the service rejects is
//! worse than a stop, because the rejection is attributed to the candidate.
//!
//! So these derive over the CALLER's alphabet and length. Every value is a
//! function of the miss alone, so a replay gets the same answer every run —
//! without that, two replays of one candidate disagree and a divergence cannot
//! be attributed. The cost is the human-readable marker; the ledger still
//! records the outcome as synthesized, so the scorer is unaffected.

/// Decimal digits, for the generators that promise only these.
pub const DIGITS: [char; 10] = ['0', '1', '2', '3', '4', '5', '6', '7', '8', '9'];

/// `length` characters drawn from `alphabet`, derived from `miss`.
///
/// Returns an empty string for an empty alphabet rather than panicking: this
/// runs on the miss path, where a panic would take down a correlation that the
/// whole point is to keep alive.
pub fn over(miss: &deja::SubstituteMiss, alphabet: &[char], length: usize) -> String {
    let span = match u64::try_from(alphabet.len()) {
        Ok(span) if span > 0 => span,
        _ => return String::new(),
    };
    let seed = deja::synth::u64(miss);
    (0..length)
        .filter_map(|index| {
            let index = u64::try_from(index).ok()?;
            let pick = usize::try_from(mix(seed, index) % span).ok()?;
            alphabet.get(pick).copied()
        })
        .collect()
}

/// A deterministic `f64` in `[0, 1)`.
///
/// The half-open range matters: this seam feeds a comparison against a rollout
/// percentage, and a value of exactly `1.0` would fire a 100%-exclusive branch
/// that the live generator can never reach.
pub fn unit_f64(miss: &deja::SubstituteMiss) -> f64 {
    // Via u32 so the conversion is `f64::from`, which is lossless and total,
    // rather than an `as` cast.
    let span = f64::from(u32::MAX) + 1.0;
    let draw = u32::try_from(deja::synth::u64(miss) >> 32).unwrap_or(0);
    f64::from(draw) / span
}

/// A deterministic value in `min..=max`, inclusive, matching `gen_range`.
///
/// Computed in `i128` so a range spanning the whole of `i64` cannot overflow on
/// the way to being reduced.
pub fn in_range(miss: &deja::SubstituteMiss, min: i64, max: i64) -> i64 {
    if min >= max {
        return min;
    }
    let span = i128::from(max) - i128::from(min) + 1;
    let draw = i128::from(deja::synth::u64(miss)) % span;
    i64::try_from(i128::from(min) + draw).unwrap_or(min)
}

/// A deterministic index into `0..length`; `None` for an empty range, which is
/// what the live generator answers.
pub fn index(miss: &deja::SubstituteMiss, length: usize) -> Option<usize> {
    let span = u64::try_from(length).ok().filter(|n| *n > 0)?;
    usize::try_from(deja::synth::u64(miss) % span).ok()
}

/// `length` deterministic bytes.
///
/// Not [`deja::synth::bytes`], whose length is a const generic: these callers
/// choose a length at runtime.
pub fn byte_vec(miss: &deja::SubstituteMiss, length: usize) -> Vec<u8> {
    let seed = deja::synth::u64(miss);
    (0..length)
        .filter_map(|position| {
            let position = u64::try_from(position).ok()?;
            u8::try_from(mix(seed, position) & 0xff).ok()
        })
        .collect()
}

/// A deterministic permutation of `0..length`.
///
/// Fisher-Yates driven by the miss, so the result is a genuine permutation —
/// every index present exactly once — rather than a sequence of independent
/// draws, which is what the caller's `shuffle` promises.
pub fn permutation(miss: &deja::SubstituteMiss, length: usize) -> Vec<usize> {
    let seed = deja::synth::u64(miss);
    let mut out: Vec<usize> = (0..length).collect();
    for position in (1..length).rev() {
        let Ok(cursor) = u64::try_from(position) else {
            continue;
        };
        let Ok(span) = u64::try_from(position + 1) else {
            continue;
        };
        if let Ok(pick) = usize::try_from(mix(seed, cursor) % span) {
            out.swap(position, pick);
        }
    }
    out
}

/// A deterministic UUID in the **version 8** space.
///
/// Version 8 is RFC 9562's custom space, so a synthesized uuid is structurally
/// disjoint from the v4 and v7 values real code produces — a synthesized id can
/// never satisfy a lookup keyed on a recorded one. Falls back to nil rather than
/// panicking if the formatted value ever fails to parse.
pub fn uuid(miss: &deja::SubstituteMiss) -> uuid::Uuid {
    uuid::Uuid::parse_str(&deja::synth::uuid_v8(miss)).unwrap_or(uuid::Uuid::nil())
}

/// How far a synthesized clock advances between two misses at one call site.
///
/// `monotonic` is unit-agnostic, so this fixes the unit: one millisecond, in
/// nanoseconds. Kept small deliberately — a step above the timing rule's
/// evidential floor (`MIN_EVIDENTIAL_MS`, juspay/deja#188) would let a derived
/// value coincide with a span's duration and read as a measured one.
pub const CLOCK_STEP_NS: i64 = 1_000_000;

/// Nanoseconds since the Unix epoch, advancing with the occurrence at this site.
///
/// `base` is **zero**, not a correlation's time origin. The doc for
/// [`deja::synth::monotonic`] offers a correlation-scoped origin, but no such
/// origin is available at a miss, and inventing one would reintroduce exactly
/// the ambient dependency these arms exist to remove. Zero also makes the value
/// read as obviously synthetic to a human, which is a feature rather than a cost.
pub fn epoch_nanos(miss: &deja::SubstituteMiss) -> i64 {
    deja::synth::monotonic(miss, 0, CLOCK_STEP_NS)
}

/// A deterministic UTC instant for this miss.
///
/// Total: falls back to the epoch rather than panicking, because a panic inside
/// a miss arm would stop the request the arm exists to keep alive.
pub fn instant(miss: &deja::SubstituteMiss) -> time::OffsetDateTime {
    time::OffsetDateTime::from_unix_timestamp_nanos(i128::from(epoch_nanos(miss)))
        .unwrap_or(time::OffsetDateTime::UNIX_EPOCH)
}

/// One digest per position.
///
/// Mixing the index in rather than walking a single digest means two positions
/// never correlate, and extending the length never rewrites the characters
/// already produced — the same property [`deja::synth::bytes`] documents for its
/// blocks, which matters because a caller may ask for 8 characters at one site
/// and 40 at another from the same seed.
fn mix(seed: u64, index: u64) -> u64 {
    // splitmix64's finalizer. Chosen for having no fixed points worth worrying
    // about at this size, not for any cryptographic property: everything here is
    // derivable by anyone holding the query, which is the point.
    let mut state = seed ^ index.wrapping_mul(0x9E37_79B9_7F4A_7C15);
    state = (state ^ (state >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    state = (state ^ (state >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    state ^ (state >> 31)
}

#[cfg(test)]
mod tests {
    use super::*;

    const HEX: [char; 16] = [
        '0', '1', '2', '3', '4', '5', '6', '7', '8', '9', 'a', 'b', 'c', 'd', 'e', 'f',
    ];
    const DIGITS: [char; 10] = ['0', '1', '2', '3', '4', '5', '6', '7', '8', '9'];

    fn miss(args: serde_json::Value) -> deja::SubstituteMiss {
        deja::SubstituteMiss {
            boundary: "id",
            component: "test",
            method: "generate",
            args,
            occurrence: 0,
            correlation_id: None,
        }
    }

    /// The property replay depends on: one query, one answer, every run. Without
    /// it two replays of an unchanged candidate disagree and a divergence cannot
    /// be attributed.
    #[test]
    fn the_same_miss_gives_the_same_string() {
        let first = over(&miss(serde_json::json!({"len": 12})), &HEX, 12);
        let second = over(&miss(serde_json::json!({"len": 12})), &HEX, 12);
        assert_eq!(first, second, "one miss must synthesize one value");
        assert!(!first.is_empty(), "and it must not be vacuously equal");
    }

    /// Two different queries must not collide, or a downstream lookup keyed on
    /// one fabricated id would resolve another's.
    #[test]
    fn a_different_miss_gives_a_different_string() {
        let first = over(&miss(serde_json::json!({"len": 12})), &HEX, 12);
        let second = over(&miss(serde_json::json!({"len": 13})), &HEX, 12);
        assert_ne!(first, second, "different args must synthesize differently");
    }

    /// The whole reason this exists rather than `deja::synth::id`: the value has
    /// to satisfy the alphabet and length its caller promised.
    #[test]
    fn the_value_honours_the_alphabet_and_the_length() {
        for length in [1_usize, 8, 32, 64] {
            let value = over(&miss(serde_json::json!({"n": length})), &DIGITS, length);
            assert_eq!(
                value.chars().count(),
                length,
                "length is part of the promise"
            );
            assert!(
                value.chars().all(|c| DIGITS.contains(&c)),
                "a numeric generator must not return {value}"
            );
        }
    }

    /// Lengthening a value must not rewrite the characters already produced, so
    /// one seed serves an 8-character site and a 40-character site coherently.
    #[test]
    fn extending_the_length_keeps_the_earlier_characters() {
        let short = over(&miss(serde_json::json!({})), &HEX, 8);
        let long = over(&miss(serde_json::json!({})), &HEX, 40);
        assert!(
            long.starts_with(&short),
            "extending rewrote earlier positions: {short} then {long}"
        );
    }

    /// An empty alphabet is a caller error, but the miss path is the wrong place
    /// to panic — that would stop the correlation this exists to keep alive.
    #[test]
    fn an_empty_alphabet_yields_an_empty_string_rather_than_a_panic() {
        assert_eq!(over(&miss(serde_json::json!({})), &[], 8), "");
    }

    /// Positions must differ from each other, not merely be drawn from the
    /// alphabet.
    ///
    /// Written because every other test here passes when `mix` ignores its
    /// index: `"0000000000000000"` is deterministic, in-alphabet, the right
    /// length, and prefix-stable, so the whole suite stays green while the value
    /// carries four bits of information. A downstream uniqueness assumption
    /// would then be violated by a value that looks well-formed.
    #[test]
    fn positions_do_not_all_collapse_to_one_character() {
        let value = over(&miss(serde_json::json!({})), &HEX, 32);
        let distinct: std::collections::BTreeSet<char> = value.chars().collect();
        assert!(
            distinct.len() > 4,
            "32 characters over a 16-symbol alphabet collapsed to {}: {value}",
            distinct.len()
        );
    }
}
