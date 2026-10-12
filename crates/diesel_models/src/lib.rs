pub mod address;
pub mod api_keys;
pub mod batch_blocklist_job;
pub mod blocklist_lookup;
pub mod business_profile;
pub mod capture;
pub mod card_issuer;
pub mod cards_info;
pub mod configs;
pub mod connection_lease;
#[cfg(feature = "v2")]
pub mod revenue_recovery_retry_stats;

pub mod authentication;
pub mod authorization;
pub mod blocklist;
pub mod blocklist_fingerprint;
pub mod callback_mapper;
pub mod customers;
pub mod dispute;
pub mod dynamic_routing_stats;
pub mod enums;
pub mod ephemeral_key;
pub mod errors;
pub mod events;
pub mod file;
#[allow(unused)]
pub mod fraud_check;
pub mod generic_link;
pub mod gsm;
pub mod hierarchical_resource;
pub mod invoice;
#[cfg(feature = "kv_store")]
pub mod kv;
pub mod list;
pub mod locker_mock_up;
pub mod mandate;
pub mod merchant_account;
pub mod merchant_connector_account;
pub mod merchant_key_store;
pub mod organization;
pub mod payment_attempt;
pub mod payment_intent;
pub mod payment_link;
pub mod payment_method;
pub mod payout_attempt;
pub mod payouts;
pub mod process_tracker;
pub mod query;
pub mod refund;
pub mod relay;
pub mod reverse_lookup;
pub mod role;
pub mod routing_algorithm;
pub mod subscription;
pub mod types;
pub mod unified_translations;

#[cfg(feature = "v2")]
pub mod payment_methods_session;

#[allow(unused_qualifications)]
pub mod schema;
#[allow(unused_qualifications)]
pub mod schema_v2;

pub mod user;
pub mod user_authentication_method;
pub mod user_key_store;
pub mod user_role;

use std::{
    sync::{
        atomic::{self, AtomicU64},
        Arc,
    },
    time::Duration,
};

use common_utils::external_service::ExternalServiceEventEmitter;
use diesel_impl::{DieselArray, OptionalDieselArray};
#[cfg(feature = "v2")]
use diesel_impl::{RequiredFromNullable, RequiredFromNullableWithDefault};

pub type StorageResult<T> = error_stack::Result<T, errors::DatabaseError>;

/// The concrete diesel connection type every pg pool in the workspace is built
/// over. Under `deja` it is a delegating wrapper that captures each result
/// row's wire bytes before `FromSql` consumes them; feature-off it is exactly
/// `diesel::PgConnection`.
#[cfg(feature = "deja")]
pub type DejaPgConnection = deja::DejaLoadConnection<diesel::PgConnection>;
#[cfg(not(feature = "deja"))]
pub type DejaPgConnection = diesel::PgConnection;

pub type RawPgConnection = async_bb8_diesel::Connection<DejaPgConnection>;
pub type PgPooledConn = async_bb8_diesel::Connection<DejaPgConnection>;

/// A connection leased from a database pool, along with the request context that queries run on
/// it are attributed to. The connection is returned to the pool when this value is dropped.
///
/// Every query in this crate is issued through this type, so a query cannot be run without first
/// stating whether a request context exists for it.
///
/// The lease borrows the pool it was acquired from through `bb8::Pool::get`, so this wrapper
/// cannot outlive that pool. The pool lifetime is carried through query signatures in this crate.
pub struct DatabaseConnectionWithContext<'pool> {
    connection: bb8::PooledConnection<'pool, async_bb8_diesel::ConnectionManager<DejaPgConnection>>,
    request_id: Option<String>,
    event_emitter: Arc<dyn ExternalServiceEventEmitter>,
    /// Which pool this lease came from, for `DATABASE_CONNECTION_LEASE_TIME_MS`.
    lease_target: connection_lease::LeaseTarget,
    /// How long leasing this connection took, in nanoseconds, until taken by the first
    /// tracked query or by [`Drop`]. Zero means already taken — `new` floors the real
    /// value at 1.
    ///
    /// Atomic rather than `Cell`: queries hold `&DatabaseConnectionWithContext` across
    /// awaits in `Send` futures, which requires `Sync`.
    lease_time_nanos: AtomicU64,
}

impl<'pool> DatabaseConnectionWithContext<'pool> {
    pub fn new(
        connection: bb8::PooledConnection<
            'pool,
            async_bb8_diesel::ConnectionManager<DejaPgConnection>,
        >,
        request_id: Option<String>,
        event_emitter: Arc<dyn ExternalServiceEventEmitter>,
        lease_target: connection_lease::LeaseTarget,
        lease_time: Duration,
    ) -> Self {
        Self {
            connection,
            request_id,
            event_emitter,
            lease_target,
            // Floored at 1ns so zero stays free as the "already recorded" marker.
            lease_time_nanos: AtomicU64::new(
                u64::try_from(lease_time.as_nanos())
                    .unwrap_or(u64::MAX)
                    .max(1),
            ),
        }
    }

    pub fn raw_connection(&self) -> &RawPgConnection {
        &self.connection
    }

    pub fn request_id(&self) -> Option<&str> {
        self.request_id.as_deref()
    }

    pub fn event_emitter(&self) -> &dyn ExternalServiceEventEmitter {
        self.event_emitter.as_ref()
    }

    pub fn lease_target(&self) -> connection_lease::LeaseTarget {
        self.lease_target
    }

    /// Takes the lease duration — once. Later callers get `None`, so a lease serving
    /// several queries isn't counted more than once.
    pub fn take_lease_time(&self) -> Option<Duration> {
        let nanos = self.lease_time_nanos.swap(0, atomic::Ordering::Relaxed);
        (nanos != 0).then(|| Duration::from_nanos(nanos))
    }
}

impl Drop for DatabaseConnectionWithContext<'_> {
    /// Records a lease no query claimed — e.g. the KV path, which leases purely to
    /// serialize a query.
    fn drop(&mut self) {
        if let Some(elapsed) = self.take_lease_time() {
            connection_lease::record(
                self.lease_target,
                connection_lease::Outcome::Acquired,
                "unattributed",
                elapsed,
            );
        }
    }
}

pub use self::{
    address::*, api_keys::*, callback_mapper::*, capture::*, cards_info::*, configs::*,
    customers::*, dispute::*, ephemeral_key::*, events::*, file::*, generic_link::*,
    locker_mock_up::*, mandate::*, merchant_account::*, merchant_connector_account::*,
    payment_attempt::*, payment_intent::*, payment_method::*, payout_attempt::*, payouts::*,
    process_tracker::*, refund::*, reverse_lookup::*, user_authentication_method::*,
};
/// The types and implementations provided by this module are required for the schema generated by
/// `diesel_cli` 2.0 to work with the types defined in Rust code. This is because
/// [`diesel`][diesel] 2.0 [changed the nullability of array elements][diesel-2.0-array-nullability],
/// which causes [`diesel`][diesel] to deserialize arrays as `Vec<Option<T>>`. To prevent declaring
/// array elements as `Option<T>`, this module provides types and implementations to deserialize
/// arrays as `Vec<T>`, considering only non-null values (`Some(T)`) among the deserialized
/// `Option<T>` values.
///
/// [diesel-2.0-array-nullability]: https://diesel.rs/guides/migration_guide.html#2-0-0-nullability-of-array-elements
#[doc(hidden)]
pub(crate) mod diesel_impl {
    #[cfg(feature = "v2")]
    use common_utils::{id_type, types};
    use diesel::{
        deserialize::FromSql,
        pg::Pg,
        sql_types::{Array, Nullable},
        Queryable,
    };

    #[cfg(feature = "v2")]
    use crate::enums;

    pub struct DieselArray<T>(Vec<Option<T>>);

    impl<T> From<DieselArray<T>> for Vec<T> {
        fn from(array: DieselArray<T>) -> Self {
            array.0.into_iter().flatten().collect()
        }
    }

    impl<T, U> Queryable<Array<Nullable<U>>, Pg> for DieselArray<T>
    where
        T: FromSql<U, Pg>,
        Vec<Option<T>>: FromSql<Array<Nullable<U>>, Pg>,
    {
        type Row = Vec<Option<T>>;

        fn build(row: Self::Row) -> diesel::deserialize::Result<Self> {
            Ok(Self(row))
        }
    }

    pub struct OptionalDieselArray<T>(Option<Vec<Option<T>>>);

    impl<T> From<OptionalDieselArray<T>> for Option<Vec<T>> {
        fn from(option_array: OptionalDieselArray<T>) -> Self {
            option_array
                .0
                .map(|array| array.into_iter().flatten().collect())
        }
    }

    impl<T, U> Queryable<Nullable<Array<Nullable<U>>>, Pg> for OptionalDieselArray<T>
    where
        Option<Vec<Option<T>>>: FromSql<Nullable<Array<Nullable<U>>>, Pg>,
    {
        type Row = Option<Vec<Option<T>>>;

        fn build(row: Self::Row) -> diesel::deserialize::Result<Self> {
            Ok(Self(row))
        }
    }
    #[cfg(feature = "v2")]
    /// If the DB value is null, this wrapper will return an error when deserializing.
    ///
    /// This is useful when you want to ensure that a field is always present, even if the database
    /// value is NULL. If the database column contains a NULL value, an error will be returned.
    pub struct RequiredFromNullable<T>(T);

    #[cfg(feature = "v2")]
    impl<T> RequiredFromNullable<T> {
        /// Extracts the inner value from the wrapper
        pub fn into_inner(self) -> T {
            self.0
        }
    }

    #[cfg(feature = "v2")]
    impl<T, ST, DB> Queryable<Nullable<ST>, DB> for RequiredFromNullable<T>
    where
        DB: diesel::backend::Backend,
        T: Queryable<ST, DB>,
        Option<T::Row>: FromSql<Nullable<ST>, DB>,
        ST: diesel::sql_types::SingleValue,
    {
        type Row = Option<T::Row>;

        fn build(row: Self::Row) -> diesel::deserialize::Result<Self> {
            match row {
                Some(inner_row) => {
                    let value = T::build(inner_row)?;
                    Ok(Self(value))
                }
                None => Err("Cannot deserialize NULL value for required field. Check if the database column that should not be NULL contains a NULL value.".into()),
            }
        }
    }

    #[cfg(feature = "v2")]
    /// If the DB value is null, this wrapper will provide a default value for the type `T`.
    ///
    /// This is useful when you want to ensure that a field is always present, even if the database
    /// value is NULL. The default value is provided by the `Default` trait implementation of `T`.
    pub struct RequiredFromNullableWithDefault<T>(T);
    #[cfg(feature = "v2")]
    impl<T> RequiredFromNullableWithDefault<T> {
        /// Extracts the inner value from the wrapper
        pub fn into_inner(self) -> T {
            self.0
        }
    }
    #[cfg(feature = "v2")]
    impl<T, ST, DB> Queryable<Nullable<ST>, DB> for RequiredFromNullableWithDefault<T>
    where
        DB: diesel::backend::Backend,
        T: Queryable<ST, DB>,
        T: Default,
        Option<T::Row>: FromSql<Nullable<ST>, DB>,
        ST: diesel::sql_types::SingleValue,
    {
        type Row = Option<T::Row>;

        fn build(row: Self::Row) -> diesel::deserialize::Result<Self> {
            match row {
                Some(inner_row) => {
                    let value = T::build(inner_row)?;
                    Ok(Self(value))
                }
                None => Ok(Self(T::default())),
            }
        }
    }

    #[cfg(feature = "v2")]
    /// Macro to implement From trait for types wrapped in RequiredFromNullable
    #[macro_export]
    macro_rules! impl_from_required_from_nullable {
        ($($type:ty),* $(,)?) => {
            $(
                impl From<$crate::RequiredFromNullable<$type>> for $type {
                    fn from(wrapper: $crate::RequiredFromNullable<$type>) -> Self {
                        wrapper.into_inner()
                    }
                }
            )*
        };
    }

    #[cfg(feature = "v2")]
    /// Macro to implement From trait for types wrapped in RequiredFromNullableWithDefault
    #[macro_export]
    macro_rules! impl_from_required_from_nullable_with_default {
        ($($type:ty),* $(,)?) => {
            $(
                impl From<$crate::RequiredFromNullableWithDefault<$type>> for $type {
                    fn from(wrapper: $crate::RequiredFromNullableWithDefault<$type>) -> Self {
                        wrapper.into_inner()
                    }
                }
            )*
        };
    }
    #[cfg(feature = "v2")]
    crate::impl_from_required_from_nullable_with_default!(enums::DeleteStatus);

    #[cfg(feature = "v2")]
    crate::impl_from_required_from_nullable!(
        enums::AuthenticationType,
        types::MinorUnit,
        enums::PaymentMethod,
        enums::Currency,
        id_type::ProfileId,
        time::PrimitiveDateTime,
        id_type::RefundReferenceId,
    );
}

pub(crate) mod metrics {
    use router_env::{counter_metric, global_meter, histogram_metric_f64};

    global_meter!(GLOBAL_METER, "ROUTER_API");

    counter_metric!(DATABASE_CALLS_COUNT, GLOBAL_METER);
    histogram_metric_f64!(DATABASE_CALL_TIME, GLOBAL_METER);

    // Time spent queued for a pool slot — invisible to `DATABASE_CALL_TIME` above, whose
    // timer starts only once a connection is leased. See `crate::connection_lease`.
    //
    // Unit is MILLISECONDS, unlike `DATABASE_CALL_TIME` (seconds) — hence the `_MS` suffix.
    // Comparing the two needs a factor of 1000.
    histogram_metric_f64!(
        DATABASE_CONNECTION_LEASE_TIME_MS,
        GLOBAL_METER,
        boundaries: lease_time_buckets(),
    );

    /// Boundaries for [`DATABASE_CONNECTION_LEASE_TIME_MS`], in ms: 0.01 to ~25_000, four
    /// intervals per doubling — low end resolves an uncontended lease (tens of µs), high
    /// end covers a full `connection_timeout` wait (10s default) without overflow clipping.
    fn lease_time_buckets() -> Vec<f64> {
        (0..86)
            .map(|index| 0.01 * 2_f64.powf(f64::from(index) / 4.0))
            .collect()
    }
}

#[cfg(feature = "tokenization_v2")]
pub mod tokenization;
