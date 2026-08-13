//! Custom serialization/deserialization implementations.

/// Use the well-known ISO 8601 format when serializing and deserializing an
/// [`PrimitiveDateTime`][PrimitiveDateTime].
///
/// [PrimitiveDateTime]: ::time::PrimitiveDateTime
pub mod iso8601 {
    use serde::{ser::Error as _, Deserializer, Serialize, Serializer};
    use time::{serde::iso8601, PrimitiveDateTime, UtcOffset};

    /// The same rendering `Iso8601` with `decimal_digits: 3` was asked for — and
    /// deliberately not that formatter.
    ///
    /// `time` 0.3.41 formats ISO 8601 subseconds through `f64`
    /// (`src/formatting/iso8601.rs:108` builds `seconds + nanoseconds / 1e9` as a
    /// float, `src/formatting/mod.rs:85` truncates it), so a value whose
    /// fractional part is not exactly representable renders ONE MILLISECOND
    /// EARLY: 938 000 000 ns becomes `.937`. It is not fixable by asking for more
    /// digits, which merely exposes `.937999999`.
    ///
    /// **The rate depends on the whole-seconds value, so no single figure
    /// describes it.** The `seconds` term shifts where the sum lands in the
    /// double, so the set of broken milliseconds moves with it: none at `:00`, 86
    /// of 1000 at `:01`, 47 at `:02` (every fourth from `.002`), 185 at `:32`, and
    /// none again at `:59`. An earlier note here quoted the `:02` figure alone as
    /// though it were a global 4.7 %; it is near the best case, not the average.
    /// `iso8601_exactness` sweeps the seconds for exactly this reason.
    ///
    /// This item list renders the subsecond field from the integer nanosecond
    /// count instead, which is exact for every value at every second. The output
    /// is byte-identical to what the well-known formatter produces for the values
    /// it gets right, so no API response shape changes.
    ///
    /// Upstream fixed this by 0.3.55, but every release carrying the fix requires
    /// Rust 1.88 while this workspace declares 1.85, so bumping the dependency
    /// would trade a correctness bug for an MSRV failure.
    ///
    /// Found by deja's tape-conformance gate, which asserts a recorded value
    /// survives reconstruction: `Se(De(x)) == x` failed here, and a replayed row
    /// carried a timestamp the recording never had. The bug is not
    /// replay-specific — replay is just the only thing that compares two
    /// renderings of the same instant.
    const EXACT_FORMAT: &[time::format_description::FormatItem<'static>] = time::macros::format_description!(
        "[year]-[month]-[day]T[hour]:[minute]:[second].[subsecond digits:3]Z"
    );

    /// Serialize a [`PrimitiveDateTime`] using the well-known ISO 8601 format.
    pub fn serialize<S>(date_time: &PrimitiveDateTime, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        date_time
            .assume_utc()
            .format(EXACT_FORMAT)
            .map_err(S::Error::custom)?
            .serialize(serializer)
    }

    /// Deserialize an [`PrimitiveDateTime`] from its ISO 8601 representation.
    pub fn deserialize<'a, D>(deserializer: D) -> Result<PrimitiveDateTime, D::Error>
    where
        D: Deserializer<'a>,
    {
        iso8601::deserialize(deserializer).map(|offset_date_time| {
            let utc_date_time = offset_date_time.to_offset(UtcOffset::UTC);
            PrimitiveDateTime::new(utc_date_time.date(), utc_date_time.time())
        })
    }

    /// Use the well-known ISO 8601 format when serializing and deserializing an
    /// [`Option<PrimitiveDateTime>`][PrimitiveDateTime].
    ///
    /// [PrimitiveDateTime]: ::time::PrimitiveDateTime
    pub mod option {
        use serde::Serialize;

        use super::*;

        /// Serialize an [`Option<PrimitiveDateTime>`] using the well-known ISO 8601 format.
        pub fn serialize<S>(
            date_time: &Option<PrimitiveDateTime>,
            serializer: S,
        ) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            date_time
                // Same exact-subsecond rendering as the non-optional case; see
                // `EXACT_FORMAT`.
                .map(|date_time| date_time.assume_utc().format(EXACT_FORMAT))
                .transpose()
                .map_err(S::Error::custom)?
                .serialize(serializer)
        }

        /// Deserialize an [`Option<PrimitiveDateTime>`] from its ISO 8601 representation.
        pub fn deserialize<'a, D>(deserializer: D) -> Result<Option<PrimitiveDateTime>, D::Error>
        where
            D: Deserializer<'a>,
        {
            iso8601::option::deserialize(deserializer).map(|option_offset_date_time| {
                option_offset_date_time.map(|offset_date_time| {
                    let utc_date_time = offset_date_time.to_offset(UtcOffset::UTC);
                    PrimitiveDateTime::new(utc_date_time.date(), utc_date_time.time())
                })
            })
        }
    }
    /// Use the well-known ISO 8601 format which is without timezone when serializing and deserializing an
    /// [`Option<PrimitiveDateTime>`][PrimitiveDateTime].
    ///
    /// [PrimitiveDateTime]: ::time::PrimitiveDateTime
    pub mod option_without_timezone {
        use serde::{de, Deserialize, Serialize};
        use time::macros::format_description;

        use super::*;

        /// Serialize an [`Option<PrimitiveDateTime>`] using the well-known ISO 8601 format which is without timezone.
        pub fn serialize<S>(
            date_time: &Option<PrimitiveDateTime>,
            serializer: S,
        ) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            date_time
                .map(|date_time| {
                    let format =
                        format_description!("[year]-[month]-[day]T[hour]:[minute]:[second]");
                    date_time.assume_utc().format(format)
                })
                .transpose()
                .map_err(S::Error::custom)?
                .serialize(serializer)
        }

        /// Deserialize an [`Option<PrimitiveDateTime>`] from its ISO 8601 representation.
        pub fn deserialize<'a, D>(deserializer: D) -> Result<Option<PrimitiveDateTime>, D::Error>
        where
            D: Deserializer<'a>,
        {
            Option::deserialize(deserializer)?
                .map(|time_string| {
                    let format =
                        format_description!("[year]-[month]-[day]T[hour]:[minute]:[second]");
                    PrimitiveDateTime::parse(time_string, format).map_err(|_| {
                        de::Error::custom(format!(
                            "Failed to parse PrimitiveDateTime from {time_string}"
                        ))
                    })
                })
                .transpose()
        }
    }
}

/// Use the UNIX timestamp when serializing and deserializing an
/// [`PrimitiveDateTime`][PrimitiveDateTime].
///
/// [PrimitiveDateTime]: ::time::PrimitiveDateTime
pub mod timestamp {

    use serde::{Deserializer, Serialize, Serializer};
    use time::{serde::timestamp, PrimitiveDateTime, UtcOffset};

    /// Serialize a [`PrimitiveDateTime`] using UNIX timestamp.
    pub fn serialize<S>(date_time: &PrimitiveDateTime, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        date_time
            .assume_utc()
            .unix_timestamp()
            .serialize(serializer)
    }

    /// Deserialize an [`PrimitiveDateTime`] from UNIX timestamp.
    pub fn deserialize<'a, D>(deserializer: D) -> Result<PrimitiveDateTime, D::Error>
    where
        D: Deserializer<'a>,
    {
        timestamp::deserialize(deserializer).map(|offset_date_time| {
            let utc_date_time = offset_date_time.to_offset(UtcOffset::UTC);
            PrimitiveDateTime::new(utc_date_time.date(), utc_date_time.time())
        })
    }

    /// Use the UNIX timestamp when serializing and deserializing an
    /// [`Option<PrimitiveDateTime>`][PrimitiveDateTime].
    ///
    /// [PrimitiveDateTime]: ::time::PrimitiveDateTime
    pub mod option {
        use serde::Serialize;

        use super::*;

        /// Serialize an [`Option<PrimitiveDateTime>`] from UNIX timestamp.
        pub fn serialize<S>(
            date_time: &Option<PrimitiveDateTime>,
            serializer: S,
        ) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            date_time
                .map(|date_time| date_time.assume_utc().unix_timestamp())
                .serialize(serializer)
        }

        /// Deserialize an [`Option<PrimitiveDateTime>`] from UNIX timestamp.
        pub fn deserialize<'a, D>(deserializer: D) -> Result<Option<PrimitiveDateTime>, D::Error>
        where
            D: Deserializer<'a>,
        {
            timestamp::option::deserialize(deserializer).map(|option_offset_date_time| {
                option_offset_date_time.map(|offset_date_time| {
                    let utc_date_time = offset_date_time.to_offset(UtcOffset::UTC);
                    PrimitiveDateTime::new(utc_date_time.date(), utc_date_time.time())
                })
            })
        }
    }
}

/// <https://github.com/serde-rs/serde/issues/994#issuecomment-316895860>
pub mod json_string {
    use serde::{
        de::{self, Deserialize, DeserializeOwned, Deserializer},
        ser::{self, Serialize, Serializer},
    };

    /// Serialize a type to json_string format
    pub fn serialize<T, S>(value: &T, serializer: S) -> Result<S::Ok, S::Error>
    where
        T: Serialize,
        S: Serializer,
    {
        let j = serde_json::to_string(value).map_err(ser::Error::custom)?;
        j.serialize(serializer)
    }

    /// Deserialize a string which is in json format
    pub fn deserialize<'de, T, D>(deserializer: D) -> Result<T, D::Error>
    where
        T: DeserializeOwned,
        D: Deserializer<'de>,
    {
        let j = String::deserialize(deserializer)?;
        serde_json::from_str(&j).map_err(de::Error::custom)
    }
}

/// Use a custom ISO 8601 format when serializing and deserializing
/// [`PrimitiveDateTime`][PrimitiveDateTime].
///
/// [PrimitiveDateTime]: ::time::PrimitiveDateTime
pub mod iso8601custom {

    use serde::{ser::Error as _, Deserializer, Serialize, Serializer};
    use time::{serde::iso8601, PrimitiveDateTime, UtcOffset};

    /// `YYYY-MM-DD HH:MM:SS` — the shape this module has always emitted, now
    /// stated directly instead of derived from a different one.
    ///
    /// **This module never had the `f64` subsecond defect that
    /// [`super::iso8601`] documents**, and the reason is not the obvious one.
    /// `decimal_digits: None` does not request variable precision in `time`
    /// 0.3.41: `format_float`'s `None` arm is `let value = value.trunc() as u64`
    /// (`src/formatting/mod.rs:93`), which discards the fraction rather than
    /// rendering it at full precision. So this format emitted whole seconds and
    /// had no subsecond to get wrong — and the integer part of
    /// `seconds + nanoseconds / 1e9` is exact for every input, since the addend
    /// is always in `[0, 1)`. Verified across all 60 × 1000 second/millisecond
    /// combinations plus the nanosecond extremes.
    ///
    /// Rewritten anyway, for two reasons that are not "fix a live bug". The `f64`
    /// path is one `decimal_digits` edit away from acquiring the defect its
    /// sibling just had. And the previous implementation formatted a full ISO
    /// 8601 string and then reached for `.replace('T', " ").replace('Z', "")` —
    /// deriving this wire shape by string surgery on a different wire shape,
    /// which reads as a coincidence rather than an intent. `iso8601custom_shape`
    /// pins the output byte-for-byte against the old formatter.
    ///
    /// The precision loss is deliberate and unchanged: this format carries no
    /// subsecond at all. It is therefore not a round trip — `deserialize` below
    /// parses ISO 8601, which this output is not (space separator, no offset) —
    /// and the two ends are used against different wires.
    const WHOLE_SECOND_FORMAT: &[time::format_description::FormatItem<'static>] =
        time::macros::format_description!("[year]-[month]-[day] [hour]:[minute]:[second]");

    /// Serialize a [`PrimitiveDateTime`] as `YYYY-MM-DD HH:MM:SS`.
    pub fn serialize<S>(date_time: &PrimitiveDateTime, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        date_time
            .assume_utc()
            .format(WHOLE_SECOND_FORMAT)
            .map_err(S::Error::custom)?
            .serialize(serializer)
    }

    /// Deserialize an [`PrimitiveDateTime`] from its ISO 8601 representation.
    pub fn deserialize<'a, D>(deserializer: D) -> Result<PrimitiveDateTime, D::Error>
    where
        D: Deserializer<'a>,
    {
        iso8601::deserialize(deserializer).map(|offset_date_time| {
            let utc_date_time = offset_date_time.to_offset(UtcOffset::UTC);
            PrimitiveDateTime::new(utc_date_time.date(), utc_date_time.time())
        })
    }
}

#[cfg(test)]
mod tests {
    use serde::{Deserialize, Serialize};
    use serde_json::json;

    #[test]
    fn test_leap_second_parse() {
        #[derive(Serialize, Deserialize)]
        struct Try {
            #[serde(with = "crate::custom_serde::iso8601")]
            f: time::PrimitiveDateTime,
        }
        let leap_second_date_time = json!({"f": "2023-12-31T23:59:60.000Z"});
        let deser = serde_json::from_value::<Try>(leap_second_date_time);

        assert!(deser.is_ok())
    }
}

/// Use only the date part of a [`PrimitiveDateTime`] when serializing and deserializing.
pub mod date_only {
    use serde::{de, Deserialize, Deserializer, Serialize, Serializer};
    use time::{Date, PrimitiveDateTime, Time};

    /// Serialize a [`PrimitiveDateTime`] as a YYYY-MM-DD string.
    pub fn serialize<S>(date_time: &PrimitiveDateTime, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let date = date_time.date();
        // Format: 2026-09-10
        format!(
            "{}-{:02}-{:02}",
            date.year(),
            u8::from(date.month()),
            date.day()
        )
        .serialize(serializer)
    }

    /// Deserialize a [`PrimitiveDateTime`] from a YYYY-MM-DD string.
    pub fn deserialize<'a, D>(deserializer: D) -> Result<PrimitiveDateTime, D::Error>
    where
        D: Deserializer<'a>,
    {
        // 1. Deserialize into a Date object (handles YYYY-MM-DD)
        let date = Date::deserialize(deserializer)
            .map_err(|e| de::Error::custom(format!("Failed to parse Date (YYYY-MM-DD): {e}")))?;

        // 2. Combine with midnight time to create PrimitiveDateTime
        let time = Time::MIDNIGHT;
        Ok(PrimitiveDateTime::new(date, time))
    }
}

/// Use only the date part of an [`Option<PrimitiveDateTime>`] when serializing and deserializing.
pub mod date_only_optional {
    use serde::{Deserialize, Deserializer, Serializer}; // Added Deserialize here
    use time::PrimitiveDateTime;

    use super::date_only;

    /// Serialize an [`Option<PrimitiveDateTime>`] as a YYYY-MM-DD string or null.
    pub fn serialize<S>(
        date_time: &Option<PrimitiveDateTime>,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match date_time {
            Some(dt) => date_only::serialize(dt, serializer),
            None => serializer.serialize_none(),
        }
    }

    /// Deserialize an [`Option<PrimitiveDateTime>`] from a YYYY-MM-DD string or null.
    pub fn deserialize<'a, D>(deserializer: D) -> Result<Option<PrimitiveDateTime>, D::Error>
    where
        D: Deserializer<'a>,
    {
        // Use Option's built-in wrapper, but tell it to use our
        // specific date_only logic for the inner value
        #[derive(serde::Deserialize)]
        struct Helper(#[serde(with = "date_only")] PrimitiveDateTime);

        let helper = Option::<Helper>::deserialize(deserializer)?;
        Ok(helper.map(|h| h.0))
    }
}

#[cfg(test)]
mod iso8601_exactness {
    use serde::{Deserialize, Serialize};
    use time::{Date, Month, PrimitiveDateTime, Time};

    #[derive(Serialize, Deserialize)]
    struct Wrap {
        #[serde(with = "super::iso8601")]
        at: PrimitiveDateTime,
    }

    /// Every millisecond value must render as itself, at every second.
    ///
    /// `time` 0.3.41's well-known ISO 8601 formatter routes the subsecond through
    /// `f64` (`seconds + nanoseconds / 1e9`, truncated), so some millisecond
    /// values render one millisecond early. Serializing a value the recording
    /// never had is exactly what makes a replayed row diverge from its recording,
    /// and it is wrong in production too — a millisecond-early timestamp in every
    /// affected API response.
    ///
    /// **It sweeps the seconds, not just one.** The `seconds` term shifts where
    /// the sum lands in the double, so the broken set moves with it. Run against
    /// the old formatter, this sweep fails on **318 of 5000** pairs — none at
    /// `:00`, 86 at `:01`, 47 at `:02`, 185 at `:32`, none at `:59`. A test pinned
    /// to one fixture second would have caught 47 of those and passed while the
    /// bug was live at a six-times-worse second; pinned to `:59` it would have
    /// reported the formatter as clean. That is why the seconds are swept, and
    /// why no single percentage describes this defect.
    #[test]
    fn every_millisecond_renders_as_itself_at_every_second() {
        let date = Date::from_calendar_date(2026, Month::August, 12).expect("date");
        let mut wrong = Vec::new();
        for second in [0u8, 1, 2, 32, 59] {
            for ms in 0..1000u16 {
                let time = Time::from_hms_milli(13, 33, second, ms).expect("time");
                let json = serde_json::to_string(&Wrap {
                    at: PrimitiveDateTime::new(date, time),
                })
                .expect("serialize");
                let expected = format!(r#"{{"at":"2026-08-12T13:33:{second:02}.{ms:03}Z"}}"#);
                if json != expected {
                    wrong.push(format!(":{second:02}.{ms:03} rendered {json}"));
                }
            }
        }
        assert!(
            wrong.is_empty(),
            "{} of 5000 second/millisecond pairs do not render exactly: {:?}",
            wrong.len(),
            wrong.get(..6).unwrap_or(&wrong)
        );
    }

    /// `Se(De(x)) == x` over the values that exposed the defect on a sandbox tape.
    #[test]
    fn serialize_after_deserialize_is_a_fixed_point() {
        for wire in [
            "2026-08-12T13:33:02.938Z",
            "2026-08-12T13:48:32.135Z",
            "2026-08-12T13:33:02.002Z",
            "2026-08-12T13:33:02.999Z",
        ] {
            let json = format!(r#"{{"at":"{wire}"}}"#);
            let parsed: Wrap = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(
                serde_json::to_string(&parsed).expect("serialize"),
                json,
                "re-serializing {wire} must reproduce it"
            );
        }
    }

    /// `iso8601custom` must emit exactly what the formatter it replaced emitted.
    ///
    /// That module was rewritten off the `f64` path for robustness, not to fix a
    /// live defect — `decimal_digits: None` discarded the fraction outright
    /// (`format_float`'s `None` arm truncates to `u64`), so there was no
    /// subsecond to render early. A rewrite with no bug to fix must therefore
    /// prove it changed nothing, which this does by keeping the OLD formatter as
    /// an oracle and comparing byte for byte, across the seconds that move the
    /// `f64` sum and the nanosecond extremes that would expose any rounding.
    #[test]
    fn iso8601custom_shape() {
        use time::format_description::well_known::{
            iso8601::{Config, EncodedConfig, TimePrecision},
            Iso8601,
        };

        // The exact configuration `iso8601custom` used before the rewrite.
        const OLD_CONFIG: EncodedConfig = Config::DEFAULT
            .set_time_precision(TimePrecision::Second {
                decimal_digits: None,
            })
            .encode();

        #[derive(Serialize)]
        struct Custom {
            #[serde(with = "super::iso8601custom")]
            at: PrimitiveDateTime,
        }

        let date = Date::from_calendar_date(2026, Month::August, 12).expect("date");
        for second in [0u8, 1, 2, 32, 59] {
            for nanos in [0u32, 1, 999, 500_000_000, 938_000_000, 999_999_999] {
                let time = Time::from_hms_nano(13, 33, second, nanos).expect("time");
                let at = PrimitiveDateTime::new(date, time);

                let old = at
                    .assume_utc()
                    .format(&Iso8601::<OLD_CONFIG>)
                    .expect("old formatter")
                    .replace('T', " ")
                    .replace('Z', "");
                let new = serde_json::to_string(&Custom { at }).expect("serialize");

                assert_eq!(
                    new,
                    format!(r#"{{"at":"{old}"}}"#),
                    "the rewrite must be byte-identical at :{second:02} with {nanos} ns"
                );
                assert_eq!(
                    old,
                    format!("2026-08-12 13:33:{second:02}"),
                    "and the shape itself is whole seconds — this format carries no subsecond"
                );
            }
        }
    }
}
