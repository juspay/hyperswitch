/// Use the well-known ISO 8601 format when serializing and deserializing an
/// [`PrimitiveDateTime`][PrimitiveDateTime].
///
/// [PrimitiveDateTime]: ::time::PrimitiveDateTime
pub(crate) mod iso8601 {
    use std::num::NonZeroU8;

    use serde::{ser::Error as _, Deserializer, Serialize, Serializer};
    use time::{
        format_description::well_known::{
            iso8601::{Config, EncodedConfig, TimePrecision},
            Iso8601,
        },
        serde::iso8601,
        PrimitiveDateTime, UtcOffset,
    };
    const FORMAT_CONFIG: EncodedConfig = Config::DEFAULT
        .set_time_precision(TimePrecision::Second {
            decimal_digits: NonZeroU8::new(3),
        })
        .encode();

    /// Serialize a [`PrimitiveDateTime`] using the well-known ISO 8601 format.
    pub(crate) fn serialize<S>(
        date_time: &PrimitiveDateTime,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // iso8601::serialize(&date_time.assume_utc(), serializer)
        date_time
            .assume_utc()
            .format(&Iso8601::<FORMAT_CONFIG>)
            .map_err(S::Error::custom)?
            .serialize(serializer)
    }

    /// Deserialize an [`PrimitiveDateTime`] from its ISO 8601 representation.
    pub(crate) fn deserialize<'a, D>(deserializer: D) -> Result<PrimitiveDateTime, D::Error>
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
    pub(crate) mod option {
        use serde::Serialize;
        use time::format_description::well_known::Iso8601;

        use super::*;

        /// Serialize an [`Option<PrimitiveDateTime>`] using the well-known ISO 8601 format.
        pub(crate) fn serialize<S>(
            date_time: &Option<PrimitiveDateTime>,
            serializer: S,
        ) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            date_time
                .map(|date_time| date_time.assume_utc().format(&Iso8601::<FORMAT_CONFIG>))
                .transpose()
                .map_err(S::Error::custom)?
                .serialize(serializer)
        }

        /// Deserialize an [`Option<PrimitiveDateTime>`] from its ISO 8601 representation.
        pub(crate) fn deserialize<'a, D>(
            deserializer: D,
        ) -> Result<Option<PrimitiveDateTime>, D::Error>
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
}

pub(crate) mod payment_id_type {
    use std::fmt;

    use serde::{
        de::{self, Visitor},
        Deserializer,
    };

    use crate::types::api::PaymentIdType;

    struct PaymentIdVisitor;
    struct OptionalPaymentIdVisitor;

    impl<'de> Visitor<'de> for PaymentIdVisitor {
        type Value = PaymentIdType;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("payment id")
        }

        fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(PaymentIdType::PaymentIntentId(value.to_string()))
        }
    }

    impl<'de> Visitor<'de> for OptionalPaymentIdVisitor {
        type Value = Option<PaymentIdType>;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("payment id")
        }

        fn visit_some<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
        where
            D: Deserializer<'de>,
        {
            deserializer.deserialize_any(PaymentIdVisitor).map(Some)
        }

        fn visit_none<E>(self) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(None)
        }

        fn visit_unit<E>(self) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(None)
        }
    }

    #[allow(dead_code)]
    pub(crate) fn deserialize<'a, D>(deserializer: D) -> Result<PaymentIdType, D::Error>
    where
        D: Deserializer<'a>,
    {
        deserializer.deserialize_any(PaymentIdVisitor)
    }

    pub(crate) fn deserialize_option<'a, D>(
        deserializer: D,
    ) -> Result<Option<PaymentIdType>, D::Error>
    where
        D: Deserializer<'a>,
    {
        deserializer.deserialize_option(OptionalPaymentIdVisitor)
    }
}
