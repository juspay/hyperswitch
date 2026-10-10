//! Deterministic, panic-free stand-ins for a generator's output on a replay miss.
//!
//! Unlike [`deja::synth::id`], these honour the caller's alphabet and length, so the
//! service accepts the value, and each is a function of the miss alone.

/// Decimal digits, for the generators that promise only these.
const DIGITS: [char; 10] = ['0', '1', '2', '3', '4', '5', '6', '7', '8', '9'];

/// Step of a synthesized clock between misses at one site: one millisecond, in nanos.
const CLOCK_STEP_NS: i64 = 1_000_000;

/// The shapes a miss arm can synthesize, as methods on the miss itself.
///
/// An inherent method of the same name on `SubstituteMiss` would silently win;
/// the golden-value tests below catch that.
pub trait Synthesize {
    /// `length` characters drawn from `alphabet`; empty for an empty alphabet.
    fn over(&self, alphabet: &[char], length: usize) -> String;

    /// `length` characters over `consts::ALPHABETS`, as `generate_id_with_len` promises.
    fn alphanumeric(&self, length: usize) -> String;

    /// `prefix`, an underscore, then [`Self::alphanumeric`] of `length`.
    fn prefixed(&self, prefix: &str, length: usize) -> String;

    /// `count` distinct alphanumeric words of `length` characters, carved from one
    /// draw, since repeated calls on one miss return the same value.
    fn alphanumeric_words(&self, count: usize, length: usize) -> Vec<String>;

    /// `length` decimal digits, for the generators that promise only these.
    fn digits(&self, length: usize) -> String;

    /// A deterministic `f64` in `[0, 1)`, never `1.0`, matching the live generator.
    fn unit_f64(&self) -> f64;

    /// A deterministic value in `min..=max`, inclusive, matching `gen_range`.
    fn in_range(&self, min: i64, max: i64) -> i64;

    /// A deterministic index into `0..length`; `None` for an empty range.
    fn index(&self, length: usize) -> Option<usize>;

    /// `length` deterministic bytes, with the length chosen at runtime.
    fn byte_vec(&self, length: usize) -> Vec<u8>;

    /// A deterministic permutation of `0..length` (Fisher-Yates).
    fn permutation(&self, length: usize) -> Vec<usize>;

    /// A deterministic v8 UUID, disjoint from the v4 and v7 values real code produces.
    fn uuid(&self) -> uuid::Uuid;

    /// A deterministic `u32` with no marker; use only where nothing derives from it.
    fn opaque_u32(&self) -> u32;

    /// Nanoseconds since the Unix epoch, starting at zero and advancing per occurrence.
    fn epoch_nanos(&self) -> i64;

    /// A deterministic UTC instant for this miss, advancing by [`CLOCK_STEP_NS`].
    fn instant(&self) -> time::OffsetDateTime;

    /// A deterministic UTC instant advancing by a whole second per miss, for
    /// consumers that render only whole seconds.
    fn instant_at_second_resolution(&self) -> time::OffsetDateTime;

    /// Elapsed milliseconds, advancing by one per miss.
    fn elapsed_millis(&self) -> u128;
}

impl Synthesize for deja::SubstituteMiss {
    fn over(&self, alphabet: &[char], length: usize) -> String {
        let span = match u64::try_from(alphabet.len()) {
            Ok(span) if span > 0 => span,
            _ => return String::new(),
        };
        let seed = deja::synth::u64(self);
        (0..length)
            .filter_map(|index| {
                let index = u64::try_from(index).ok()?;
                let pick = usize::try_from(mix(seed, index) % span).ok()?;
                alphabet.get(pick).copied()
            })
            .collect()
    }

    fn alphanumeric(&self, length: usize) -> String {
        self.over(&crate::consts::ALPHABETS, length)
    }

    fn prefixed(&self, prefix: &str, length: usize) -> String {
        format!("{}_{}", prefix, self.alphanumeric(length))
    }

    fn alphanumeric_words(&self, count: usize, length: usize) -> Vec<String> {
        if length == 0 {
            return vec![String::new(); count];
        }
        let drawn = self.alphanumeric(count.saturating_mul(length));
        // ASCII alphabet, so byte chunks are character chunks.
        drawn
            .as_bytes()
            .chunks(length)
            .take(count)
            .map(|chunk| String::from_utf8_lossy(chunk).into_owned())
            .collect()
    }

    fn digits(&self, length: usize) -> String {
        self.over(&DIGITS, length)
    }

    fn unit_f64(&self) -> f64 {
        // Via u32 so the conversion is a lossless `f64::from`.
        let span = f64::from(u32::MAX) + 1.0;
        let draw = u32::try_from(deja::synth::u64(self) >> 32).unwrap_or(0);
        f64::from(draw) / span
    }

    fn in_range(&self, min: i64, max: i64) -> i64 {
        if min >= max {
            return min;
        }
        // `i128` so a full-`i64` range cannot overflow.
        let span = i128::from(max) - i128::from(min) + 1;
        let draw = i128::from(deja::synth::u64(self)) % span;
        i64::try_from(i128::from(min) + draw).unwrap_or(min)
    }

    fn index(&self, length: usize) -> Option<usize> {
        let span = u64::try_from(length).ok().filter(|n| *n > 0)?;
        usize::try_from(deja::synth::u64(self) % span).ok()
    }

    fn byte_vec(&self, length: usize) -> Vec<u8> {
        let seed = deja::synth::u64(self);
        (0..length)
            .filter_map(|position| {
                let position = u64::try_from(position).ok()?;
                u8::try_from(mix(seed, position) & 0xff).ok()
            })
            .collect()
    }

    fn permutation(&self, length: usize) -> Vec<usize> {
        let seed = deja::synth::u64(self);
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

    fn uuid(&self) -> uuid::Uuid {
        uuid::Uuid::parse_str(&deja::synth::uuid_v8(self)).unwrap_or(uuid::Uuid::nil())
    }

    fn opaque_u32(&self) -> u32 {
        u32::try_from(deja::synth::u64(self) % u64::from(u32::MAX)).unwrap_or(1)
    }

    fn epoch_nanos(&self) -> i64 {
        deja::synth::monotonic(self, 0, CLOCK_STEP_NS)
    }

    fn instant(&self) -> time::OffsetDateTime {
        time::OffsetDateTime::from_unix_timestamp_nanos(i128::from(self.epoch_nanos()))
            .unwrap_or(time::OffsetDateTime::UNIX_EPOCH)
    }

    fn instant_at_second_resolution(&self) -> time::OffsetDateTime {
        time::OffsetDateTime::from_unix_timestamp(deja::synth::monotonic(self, 0, 1))
            .unwrap_or(time::OffsetDateTime::UNIX_EPOCH)
    }

    fn elapsed_millis(&self) -> u128 {
        u128::try_from(deja::synth::monotonic(self, 0, 1)).unwrap_or(0)
    }
}

/// One digest per position, so extending the length keeps earlier characters.
fn mix(seed: u64, index: u64) -> u64 {
    // splitmix64's finalizer; not cryptographic, by design.
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

    /// One miss always synthesizes the same value.
    #[test]
    fn the_same_miss_gives_the_same_string() {
        let first = miss(serde_json::json!({"len": 12})).over(&HEX, 12);
        let second = miss(serde_json::json!({"len": 12})).over(&HEX, 12);
        assert_eq!(first, second, "one miss must synthesize one value");
        assert!(!first.is_empty(), "and it must not be vacuously equal");
    }

    /// Different misses synthesize different values.
    #[test]
    fn a_different_miss_gives_a_different_string() {
        let first = miss(serde_json::json!({"len": 12})).over(&HEX, 12);
        let second = miss(serde_json::json!({"len": 13})).over(&HEX, 12);
        assert_ne!(first, second, "different args must synthesize differently");
    }

    /// The value uses the caller's alphabet and length.
    #[test]
    fn the_value_honours_the_alphabet_and_the_length() {
        for length in [1_usize, 8, 32, 64] {
            let value = miss(serde_json::json!({"n": length})).over(&DIGITS, length);
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

    /// A longer draw extends a shorter one.
    #[test]
    fn extending_the_length_keeps_the_earlier_characters() {
        let short = miss(serde_json::json!({})).over(&HEX, 8);
        let long = miss(serde_json::json!({})).over(&HEX, 40);
        assert!(
            long.starts_with(&short),
            "extending rewrote earlier positions: {short} then {long}"
        );
    }

    #[test]
    fn an_empty_alphabet_yields_an_empty_string_rather_than_a_panic() {
        assert_eq!(miss(serde_json::json!({})).over(&[], 8), "");
    }

    /// Positions vary; catches a `mix` that ignores its index.
    #[test]
    fn positions_do_not_all_collapse_to_one_character() {
        let value = miss(serde_json::json!({})).over(&HEX, 32);
        let distinct: std::collections::BTreeSet<char> = value.chars().collect();
        assert!(
            distinct.len() > 4,
            "32 characters over a 16-symbol alphabet collapsed to {}: {value}",
            distinct.len()
        );
    }
}

/// Golden values: every synthesized shape pinned to its exact output, since a moved
/// value makes a replay diverge from its own recording.
#[cfg(test)]
mod golden {
    use std::num::NonZeroU8;

    use time::format_description::well_known::iso8601::{
        Config, EncodedConfig, Iso8601, TimePrecision,
    };

    use super::*;
    use crate::consts;

    /// One fixed miss, varied only by occurrence.
    fn miss(occurrence: u32) -> deja::SubstituteMiss {
        deja::SubstituteMiss {
            boundary: "id",
            component: "common_utils",
            method: "golden",
            args: serde_json::json!({"len": 12}),
            occurrence,
            correlation_id: None,
        }
    }

    const SYNTH_ISO: EncodedConfig = Config::DEFAULT
        .set_time_precision(TimePrecision::Second {
            decimal_digits: NonZeroU8::new(3),
        })
        .encode();

    #[test]
    fn over_an_alphabet_is_pinned() {
        assert_eq!(miss(0).over(&consts::ALPHABETS, 12), "77z992CVQZxg");
        assert_eq!(miss(0).alphanumeric(12), "77z992CVQZxg");
        assert_eq!(miss(0).prefixed("pay", 12), "pay_77z992CVQZxg");
        assert_eq!(miss(0).digits(10), "7133908345");
        assert_eq!(
            miss(0).over(&nanoid::alphabet::SAFE, 21),
            "DDBnXeIlmxl2UqmnXe2EI"
        );
        assert_eq!(miss(0).alphanumeric(0), "");
        assert_eq!(
            miss(0).alphanumeric_words(4, 4),
            ["77z9", "92CV", "QZxg", "68YZ"]
        );
        assert_eq!(miss(0).alphanumeric_words(0, 4), Vec::<String>::new());
        assert_eq!(miss(0).alphanumeric_words(2, 0), ["", ""]);
    }

    /// Words from one miss are mutually distinct.
    #[test]
    fn the_words_differ_from_one_another() {
        let words = miss(0).alphanumeric_words(8, 8);
        let distinct: std::collections::BTreeSet<&String> = words.iter().collect();
        assert_eq!(distinct.len(), words.len(), "words collapsed: {words:?}");
        assert!(words.iter().all(|word| word.chars().count() == 8));
    }

    #[test]
    fn the_numeric_draws_are_pinned() {
        assert_eq!(miss(0).unit_f64(), 0.400_051_809_847_354_9);
        assert_eq!(miss(0).in_range(-5, 5), -2);
        assert_eq!(
            miss(0).in_range(i64::MIN, i64::MAX),
            -1_843_718_680_693_841_055
        );
        assert_eq!(miss(0).in_range(7, 7), 7);
        assert_eq!(miss(0).index(10), Some(3));
        assert_eq!(miss(0).index(0), None);
        assert_eq!(miss(0).byte_vec(8), [105, 41, 231, 89, 125, 16, 174, 23]);
        assert_eq!(miss(0).permutation(6), [0, 3, 5, 1, 4, 2]);
        assert_eq!(miss(0).opaque_u32(), 1_105_702_658);
    }

    #[test]
    fn the_uuid_shapes_are_pinned() {
        assert_eq!(
            miss(0).uuid().to_string(),
            "c41247c7-5493-8f37-9a15-cd1ff03594e9"
        );
        assert_eq!(
            miss(0).uuid().simple().to_string(),
            "c41247c754938f379a15cd1ff03594e9"
        );
    }

    /// The v8 uuid always parses, so the nil fallback is unreachable.
    #[test]
    fn stripping_the_hyphens_and_parsing_agree() {
        for occurrence in 0..512_u32 {
            let stripped = deja::synth::uuid_v8(&miss(occurrence)).replace('-', "");
            assert_eq!(
                miss(occurrence).uuid().as_simple().to_string(),
                stripped,
                "the parse moved a value at occurrence {occurrence}"
            );
            assert_ne!(
                stripped,
                uuid::Uuid::nil().as_simple().to_string(),
                "and it must not be vacuously equal through the nil fallback"
            );
        }
    }

    /// The clock shapes, read as each arm reads them, at two occurrences.
    #[test]
    fn the_clock_arms_are_pinned() {
        let rfc7231 = time::macros::format_description!(
            "[weekday repr:short], [day padding:zero] [month repr:short] [year repr:full] [hour padding:zero repr:24]:[minute padding:zero]:[second padding:zero] GMT"
        );
        let render = |occurrence: u32| {
            let at = miss(occurrence).instant();
            let per_second = miss(occurrence).instant_at_second_resolution();
            (
                miss(occurrence).epoch_nanos(),
                time::PrimitiveDateTime::new(at.date(), at.time()).to_string(),
                per_second.unix_timestamp(),
                at.unix_timestamp_nanos() / 1_000_000,
                crate::date_time::convert_to_pdt(at)
                    .assume_utc()
                    .format(&Iso8601::<SYNTH_ISO>)
                    .expect("the synthesized instant formats"),
                per_second
                    .format(&rfc7231)
                    .expect("and renders as an HTTP date"),
            )
        };

        assert_eq!(
            render(0),
            (
                0,
                "1970-01-01 0:00:00.0".to_string(),
                0,
                0,
                "1970-01-01T00:00:00.000Z".to_string(),
                "Thu, 01 Jan 1970 00:00:00 GMT".to_string(),
            )
        );
        assert_eq!(
            render(1),
            (
                1_000_000,
                "1970-01-01 0:00:00.001".to_string(),
                1,
                1,
                "1970-01-01T00:00:00.001Z".to_string(),
                "Thu, 01 Jan 1970 00:00:01 GMT".to_string(),
            )
        );
    }

    /// Consecutive misses at a clock arm synthesize different values.
    #[test]
    fn consecutive_misses_at_a_clock_arm_differ() {
        for occurrence in [0_u32, 1, 998, 12_345] {
            let (here, next) = (miss(occurrence), miss(occurrence + 1));
            assert_ne!(
                here.instant(),
                next.instant(),
                "the nanosecond clock held still across occurrence {occurrence}"
            );
            assert_ne!(
                here.instant_at_second_resolution().unix_timestamp(),
                next.instant_at_second_resolution().unix_timestamp(),
                "the second clock held still across occurrence {occurrence}"
            );
        }
    }

    /// The counter behind `elapsed::millis_since`, stepping one millisecond.
    #[test]
    fn the_elapsed_counter_is_pinned() {
        let derived: Vec<u128> = (0..4_u32).map(|n| miss(n).elapsed_millis()).collect();
        assert_eq!(derived, vec![0, 1, 2, 3]);
    }
}
