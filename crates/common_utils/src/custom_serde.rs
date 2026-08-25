//! Custom serialization/deserialization implementations.

/// Use the well-known ISO 8601 format when serializing and deserializing an
/// [`PrimitiveDateTime`][PrimitiveDateTime].
///
/// [PrimitiveDateTime]: ::time::PrimitiveDateTime
pub mod iso8601 {
    use serde::{ser::Error as _, Deserializer, Serialize, Serializer};
    use time::{serde::iso8601, PrimitiveDateTime, UtcOffset};

    /// The same rendering `Iso8601` with `decimal_digits: 3` produces, but with
    /// the subsecond rendered from the integer nanosecond count. `time` 0.3.41's
    /// well-known formatter routes subseconds through `f64` and can render a
    /// millisecond early (e.g. 938 ms as `.937`); the fixed releases require
    /// Rust 1.88 while this workspace declares 1.85.
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

    /// `YYYY-MM-DD HH:MM:SS` — the shape this module has always emitted
    /// (previously derived by string surgery on a full ISO 8601 rendering),
    /// stated directly. This format deliberately carries no subsecond, and
    /// `deserialize` below parses ISO 8601 — the two ends serve different wires.
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
