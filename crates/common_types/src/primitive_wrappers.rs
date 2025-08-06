pub use bool_wrappers::*;
pub use u32_wrappers::*;
mod bool_wrappers {
    use std::ops::Deref;

    use serde::{Deserialize, Serialize};
    /// Bool that represents if Extended Authorization is Applied or not
    #[derive(
        Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize, diesel::expression::AsExpression,
    )]
    #[diesel(sql_type = diesel::sql_types::Bool)]
    pub struct ExtendedAuthorizationAppliedBool(bool);
    impl Deref for ExtendedAuthorizationAppliedBool {
        type Target = bool;

        fn deref(&self) -> &Self::Target {
            &self.0
        }
    }
    impl From<bool> for ExtendedAuthorizationAppliedBool {
        fn from(value: bool) -> Self {
            Self(value)
        }
    }
    impl<DB> diesel::serialize::ToSql<diesel::sql_types::Bool, DB> for ExtendedAuthorizationAppliedBool
    where
        DB: diesel::backend::Backend,
        bool: diesel::serialize::ToSql<diesel::sql_types::Bool, DB>,
    {
        fn to_sql<'b>(
            &'b self,
            out: &mut diesel::serialize::Output<'b, '_, DB>,
        ) -> diesel::serialize::Result {
            self.0.to_sql(out)
        }
    }
    impl<DB> diesel::deserialize::FromSql<diesel::sql_types::Bool, DB>
        for ExtendedAuthorizationAppliedBool
    where
        DB: diesel::backend::Backend,
        bool: diesel::deserialize::FromSql<diesel::sql_types::Bool, DB>,
    {
        fn from_sql(value: DB::RawValue<'_>) -> diesel::deserialize::Result<Self> {
            bool::from_sql(value).map(Self)
        }
    }

    /// Bool that represents if Extended Authorization is Requested or not
    #[derive(
        Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize, diesel::expression::AsExpression,
    )]
    #[diesel(sql_type = diesel::sql_types::Bool)]
    pub struct RequestExtendedAuthorizationBool(bool);
    impl Deref for RequestExtendedAuthorizationBool {
        type Target = bool;

        fn deref(&self) -> &Self::Target {
            &self.0
        }
    }
    impl From<bool> for RequestExtendedAuthorizationBool {
        fn from(value: bool) -> Self {
            Self(value)
        }
    }
    impl RequestExtendedAuthorizationBool {
        /// returns the inner bool value
        pub fn is_true(&self) -> bool {
            self.0
        }
    }
    impl<DB> diesel::serialize::ToSql<diesel::sql_types::Bool, DB> for RequestExtendedAuthorizationBool
    where
        DB: diesel::backend::Backend,
        bool: diesel::serialize::ToSql<diesel::sql_types::Bool, DB>,
    {
        fn to_sql<'b>(
            &'b self,
            out: &mut diesel::serialize::Output<'b, '_, DB>,
        ) -> diesel::serialize::Result {
            self.0.to_sql(out)
        }
    }
    impl<DB> diesel::deserialize::FromSql<diesel::sql_types::Bool, DB>
        for RequestExtendedAuthorizationBool
    where
        DB: diesel::backend::Backend,
        bool: diesel::deserialize::FromSql<diesel::sql_types::Bool, DB>,
    {
        fn from_sql(value: DB::RawValue<'_>) -> diesel::deserialize::Result<Self> {
            bool::from_sql(value).map(Self)
        }
    }

    /// Bool that represents if Extended Authorization is always Requested or not
    #[derive(
        Clone, Copy, Debug, Eq, PartialEq, diesel::expression::AsExpression, Serialize, Deserialize,
    )]
    #[diesel(sql_type = diesel::sql_types::Bool)]
    pub struct AlwaysRequestExtendedAuthorization(bool);
    impl Deref for AlwaysRequestExtendedAuthorization {
        type Target = bool;

        fn deref(&self) -> &Self::Target {
            &self.0
        }
    }
    impl<DB> diesel::serialize::ToSql<diesel::sql_types::Bool, DB>
        for AlwaysRequestExtendedAuthorization
    where
        DB: diesel::backend::Backend,
        bool: diesel::serialize::ToSql<diesel::sql_types::Bool, DB>,
    {
        fn to_sql<'b>(
            &'b self,
            out: &mut diesel::serialize::Output<'b, '_, DB>,
        ) -> diesel::serialize::Result {
            self.0.to_sql(out)
        }
    }
    impl<DB> diesel::deserialize::FromSql<diesel::sql_types::Bool, DB>
        for AlwaysRequestExtendedAuthorization
    where
        DB: diesel::backend::Backend,
        bool: diesel::deserialize::FromSql<diesel::sql_types::Bool, DB>,
    {
        fn from_sql(value: DB::RawValue<'_>) -> diesel::deserialize::Result<Self> {
            bool::from_sql(value).map(Self)
        }
    }

    /// Bool that represents if Cvv should be collected during payment or not. Default is true
    #[derive(
        Clone, Copy, Debug, Eq, PartialEq, diesel::expression::AsExpression, Serialize, Deserialize,
    )]
    #[diesel(sql_type = diesel::sql_types::Bool)]
    pub struct ShouldCollectCvvDuringPayment(bool);
    impl Deref for ShouldCollectCvvDuringPayment {
        type Target = bool;

        fn deref(&self) -> &Self::Target {
            &self.0
        }
    }
    impl<DB> diesel::serialize::ToSql<diesel::sql_types::Bool, DB> for ShouldCollectCvvDuringPayment
    where
        DB: diesel::backend::Backend,
        bool: diesel::serialize::ToSql<diesel::sql_types::Bool, DB>,
    {
        fn to_sql<'b>(
            &'b self,
            out: &mut diesel::serialize::Output<'b, '_, DB>,
        ) -> diesel::serialize::Result {
            self.0.to_sql(out)
        }
    }
    impl<DB> diesel::deserialize::FromSql<diesel::sql_types::Bool, DB> for ShouldCollectCvvDuringPayment
    where
        DB: diesel::backend::Backend,
        bool: diesel::deserialize::FromSql<diesel::sql_types::Bool, DB>,
    {
        fn from_sql(value: DB::RawValue<'_>) -> diesel::deserialize::Result<Self> {
            bool::from_sql(value).map(Self)
        }
    }

    impl Default for ShouldCollectCvvDuringPayment {
        /// Default for `ShouldCollectCvvDuringPayment` is `true`
        fn default() -> Self {
            Self(true)
        }
    }
}

mod u32_wrappers {
    use std::ops::Deref;

    use serde::{de::Error, Deserialize, Serialize};

    use crate::consts::{
        DEFAULT_DISPUTE_POLLING_INTERVAL_IN_HOURS, MAX_DISPUTE_POLLING_INTERVAL_IN_HOURS,
    };
    /// Time interval in hours for polling disputes
    #[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, diesel::expression::AsExpression)]
    #[diesel(sql_type = diesel::sql_types::Integer)]
    pub struct DisputePollingIntervalInHours(i32);

    impl Deref for DisputePollingIntervalInHours {
        type Target = i32;

        fn deref(&self) -> &Self::Target {
            &self.0
        }
    }

    impl<'de> Deserialize<'de> for DisputePollingIntervalInHours {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: serde::Deserializer<'de>,
        {
            let val = i32::deserialize(deserializer)?;
            if val < 0 {
                Err(D::Error::custom(
                    "DisputePollingIntervalInHours cannot be negative",
                ))
            } else if val > MAX_DISPUTE_POLLING_INTERVAL_IN_HOURS {
                Err(D::Error::custom(
                    "DisputePollingIntervalInHours exceeds the maximum allowed value of 24",
                ))
            } else {
                Ok(Self(val))
            }
        }
    }

    impl diesel::deserialize::FromSql<diesel::sql_types::Integer, diesel::pg::Pg>
        for DisputePollingIntervalInHours
    {
        fn from_sql(value: diesel::pg::PgValue<'_>) -> diesel::deserialize::Result<Self> {
            i32::from_sql(value).map(Self)
        }
    }

    impl diesel::serialize::ToSql<diesel::sql_types::Integer, diesel::pg::Pg>
        for DisputePollingIntervalInHours
    {
        fn to_sql<'b>(
            &'b self,
            out: &mut diesel::serialize::Output<'b, '_, diesel::pg::Pg>,
        ) -> diesel::serialize::Result {
            <i32 as diesel::serialize::ToSql<diesel::sql_types::Integer, diesel::pg::Pg>>::to_sql(
                &self.0, out,
            )
        }
    }

    impl Default for DisputePollingIntervalInHours {
        /// Default for `ShouldCollectCvvDuringPayment` is `true`
        fn default() -> Self {
            Self(DEFAULT_DISPUTE_POLLING_INTERVAL_IN_HOURS)
        }
    }
}
