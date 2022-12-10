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

pub(crate) mod amount {
    use serde::de;

    use crate::types::api;
    struct AmountVisitor;
    struct OptionalAmountVisitor;

    // This is defined to provide guarded deserialization of amount
    // which itself handles zero and non-zero values internally
    impl<'de> de::Visitor<'de> for AmountVisitor {
        type Value = api::Amount;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            write!(formatter, "amount as i32")
        }

        fn visit_i32<E>(self, v: i32) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(match v {
                0 => api::Amount::Zero,
                amount => api::Amount::Value(amount),
            })
        }
    }

    impl<'de> de::Visitor<'de> for OptionalAmountVisitor {
        type Value = Option<api::Amount>;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            write!(formatter, "option of amount (as i32)")
        }

        fn visit_some<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
        where
            D: serde::Deserializer<'de>,
        {
            deserializer.deserialize_any(AmountVisitor).map(Some)
        }

        fn visit_none<E>(self) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(None)
        }
    }

    #[allow(dead_code)]
    pub(crate) fn deserialize<'de, D>(deserializer: D) -> Result<api::Amount, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        deserializer.deserialize_any(AmountVisitor)
    }
    pub(crate) fn deserialize_option<'de, D>(
        deserializer: D,
    ) -> Result<Option<api::Amount>, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        deserializer.deserialize_option(OptionalAmountVisitor)
    }
}
