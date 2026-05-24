use async_bb8_diesel::AsyncConnection;
use diesel::{
    associations::HasTable,
    debug_query,
    dsl::{Filter, Find},
    pg::Pg,
    query_builder::{
        bind_collector::RawBytesBindCollector, AsChangeset, AsQuery, CollectedQuery,
        InsertStatement, IntoUpdateTarget, MoveableBindCollector, QueryBuilder, QueryFragment,
        UpdateStatement,
    },
    query_dsl::methods::{ExecuteDsl, FilterDsl, FindDsl},
    query_source::Table,
    Insertable,
};
use error_stack::ResultExt;
use hyperswitch_masking::Secret;
use router_env::logger;

use crate::errors;

/// Masking strategy for binary data or raw bytes
#[derive(Debug)]
pub enum BinaryDataStrategy {}

impl<T> hyperswitch_masking::Strategy<T> for BinaryDataStrategy
where
    T: AsRef<[u8]>,
{
    fn fmt(value: &T, fmt: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(fmt, "*** Binary data ({} bytes) ***", value.as_ref().len())
    }
}

type SecretBinaryData = Secret<Vec<u8>, BinaryDataStrategy>;

/// The SQL query and its bind parameters, in a (de)serialization-friendly representation
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct SerializableQuery {
    /// The SQL query
    sql: String,

    /// The serialized bytes for each bind parameter
    #[serde(with = "bind_params")]
    binds: Vec<Option<SecretBinaryData>>,

    /// The metadata associated with each bind parameter
    #[serde(with = "pg_type_metadata")]
    metadata: Vec<diesel::pg::PgTypeMetadata>,

    /// Whether this query is safe to store in the prepared statement cache
    safe_to_cache_prepared: bool,

    /// Entity type
    entity_type: String,

    /// The type of database operation
    operation: DatabaseOperation,
}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize, strum::Display)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum DatabaseOperation {
    Insert,
    Update,
}

mod bind_params {
    use base64::Engine;
    use common_utils::consts::BASE64_ENGINE;
    use hyperswitch_masking::{PeekInterface, Secret};
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    use super::SecretBinaryData;

    pub fn serialize<S: Serializer>(
        binds: &[Option<SecretBinaryData>],
        s: S,
    ) -> Result<S::Ok, S::Error> {
        let encoded: Vec<Option<String>> = binds
            .iter()
            .map(|b| b.as_ref().map(|bytes| BASE64_ENGINE.encode(bytes.peek())))
            .collect();
        encoded.serialize(s)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(
        d: D,
    ) -> Result<Vec<Option<SecretBinaryData>>, D::Error> {
        let encoded: Vec<Option<String>> = Vec::deserialize(d)?;
        encoded
            .into_iter()
            .map(|b| {
                b.map(|s| {
                    BASE64_ENGINE
                        .decode(&s)
                        .map(Secret::new)
                        .map_err(serde::de::Error::custom)
                })
                .transpose()
            })
            .collect()
    }
}

mod pg_type_metadata {
    use diesel::pg::PgTypeMetadata;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn serialize<S: Serializer>(metadata: &[PgTypeMetadata], s: S) -> Result<S::Ok, S::Error> {
        let pairs: Vec<(u32, u32)> = metadata
            .iter()
            .map(|m| (m.oid().unwrap_or(0), m.array_oid().unwrap_or(0)))
            .collect();
        pairs.serialize(s)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Vec<PgTypeMetadata>, D::Error> {
        let pairs: Vec<(u32, u32)> = Vec::deserialize(d)?;
        Ok(pairs
            .into_iter()
            .map(|(oid, array_oid)| PgTypeMetadata::from_result(Ok((oid, array_oid))))
            .collect())
    }
}

impl SerializableQuery {
    pub fn entity_type(&self) -> String {
        self.entity_type.clone()
    }

    pub fn operation(&self) -> DatabaseOperation {
        self.operation
    }

    async fn from_query<Q>(
        conn: &mut crate::PgPooledConn,
        query: Q,
        entity_type: String,
        operation: DatabaseOperation,
    ) -> crate::StorageResult<Self>
    where
        Q: QueryFragment<Pg> + Send + 'static,
    {
        logger::debug!(%entity_type, %operation, query = %debug_query::<Pg, _>(&query).to_string());

        let mut qb = diesel::pg::PgQueryBuilder::new();
        query
            .to_sql(&mut qb, &Pg)
            .change_context(errors::DatabaseError::QueryGenerationFailed)
            .attach_printable("Failed to construct SQL query")?;
        let sql = qb.finish();

        let safe_to_cache_prepared = query
            .is_safe_to_cache_prepared(&Pg)
            .change_context(errors::DatabaseError::QueryGenerationFailed)
            .attach_printable(
                "Failed to determine whether query is safe to store in prepared statement cache",
            )?;

        let bind_collector = conn
            .run(move |c| {
                let mut bc = RawBytesBindCollector::<Pg>::new();
                query.collect_binds(&mut bc, c, &Pg)?;
                Ok::<RawBytesBindCollector<Pg>, diesel::result::Error>(bc)
            })
            .await
            .change_context(errors::DatabaseError::QueryGenerationFailed)
            .attach_printable("Failed to construct bind parameters")?;

        let serializable_query = Self {
            sql,
            binds: bind_collector
                .binds
                .into_iter()
                .map(|option| option.map(Secret::new))
                .collect(),
            metadata: bind_collector.metadata.clone(),
            safe_to_cache_prepared,
            entity_type,
            operation,
        };

        Ok(serializable_query)
    }

    fn to_collected_query(&self) -> CollectedQuery<RawBytesBindCollector<Pg>> {
        use hyperswitch_masking::ExposeInterface;

        let mut bind_collector = RawBytesBindCollector::<Pg>::new();
        bind_collector.binds = self
            .binds
            .clone()
            .into_iter()
            .map(|option| option.map(ExposeInterface::expose))
            .collect();
        bind_collector.metadata = self.metadata.clone();

        CollectedQuery::new(
            self.sql.clone(),
            self.safe_to_cache_prepared,
            bind_collector.moveable(),
        )
    }

    pub async fn execute(self, conn: &mut crate::PgPooledConn) -> crate::StorageResult<usize> {
        use common_utils::errors::ReportSwitchExt;

        let query = self.to_collected_query();

        logger::debug!(query = %debug_query::<Pg, _>(&query).to_string());

        conn.run(move |c| ExecuteDsl::execute(query, c))
            .await
            .attach_printable("Failed to execute drainer query")
            .switch()
    }

    pub fn to_field_value_pairs(
        &self,
        request_id: String,
        global_id: String,
    ) -> crate::StorageResult<Vec<(&str, String)>> {
        let pushed_at = common_utils::date_time::now_unix_timestamp();

        Ok(vec![
            (
                "query",
                serde_json::to_string(self)
                    .change_context(errors::DatabaseError::QueryGenerationFailed)?,
            ),
            ("global_id", global_id),
            ("request_id", request_id),
            ("pushed_at", pushed_at.to_string()),
        ])
    }
}

pub(crate) async fn generate_insert_query<T, N>(
    conn: &mut crate::PgPooledConn,
    new: N,
) -> crate::StorageResult<SerializableQuery>
where
    T: HasTable<Table = T> + Table + Send + 'static,
    N: Insertable<T> + entity_type::EntityType,
    <N as Insertable<T>>::Values: QueryFragment<Pg> + Send + 'static,
    InsertStatement<T, <N as Insertable<T>>::Values>: QueryFragment<Pg> + Send,
{
    let entity_type = N::ENTITY_TYPE.to_owned();
    let query = diesel::insert_into(<T as HasTable>::table()).values(new);
    SerializableQuery::from_query(conn, query, entity_type, DatabaseOperation::Insert)
        .await
        .attach_printable("Failed to generate insert query")
}

pub(crate) async fn generate_update_query_by_id<T, V, Pk>(
    conn: &mut crate::PgPooledConn,
    id: Pk,
    values: V,
) -> crate::StorageResult<SerializableQuery>
where
    T: FindDsl<Pk> + HasTable<Table = T> + Table + 'static,
    V: AsChangeset<Target = <Find<T, Pk> as HasTable>::Table> + entity_type::EntityType,
    Find<T, Pk>: IntoUpdateTarget + 'static,
    UpdateStatement<
        <Find<T, Pk> as HasTable>::Table,
        <Find<T, Pk> as IntoUpdateTarget>::WhereClause,
        <V as AsChangeset>::Changeset,
    >: AsQuery + QueryFragment<Pg> + Send + 'static,
    Pk: Clone,
{
    let entity_type = V::ENTITY_TYPE.to_owned();
    let query = diesel::update(<T as HasTable>::table().find(id)).set(values);
    SerializableQuery::from_query(conn, query, entity_type, DatabaseOperation::Update)
        .await
        .attach_printable("Failed to generate update query (with primary key)")
}

pub(crate) async fn generate_update_query_with_predicate<T, V, P>(
    conn: &mut crate::PgPooledConn,
    predicate: P,
    values: V,
) -> crate::StorageResult<SerializableQuery>
where
    T: FilterDsl<P> + HasTable<Table = T> + Table + 'static,
    V: AsChangeset<Target = <Filter<T, P> as HasTable>::Table> + entity_type::EntityType,
    Filter<T, P>: IntoUpdateTarget + 'static,
    UpdateStatement<
        <Filter<T, P> as HasTable>::Table,
        <Filter<T, P> as IntoUpdateTarget>::WhereClause,
        <V as AsChangeset>::Changeset,
    >: AsQuery + QueryFragment<Pg> + Send + 'static,
{
    let entity_type = V::ENTITY_TYPE.to_owned();
    let query = diesel::update(<T as HasTable>::table().filter(predicate)).set(values);
    SerializableQuery::from_query(conn, query, entity_type, DatabaseOperation::Update)
        .await
        .attach_printable("Failed to generate update query (with predicate)")
}

mod entity_type {
    /// Associates a database type name with an entity type.
    pub(crate) trait EntityType {
        const ENTITY_TYPE: &'static str;
    }

    macro_rules! entity_type {
        ($($entity_name:literal => { $($type:path),* $(,)? })*) => {
            $(
                $(
                    impl EntityType for $type {
                        const ENTITY_TYPE: &'static str = $entity_name;
                    }
                )*
            )*
        };
    }
    entity_type! {
        "payment_intent" => {
            crate::payment_intent::PaymentIntentNew,
            crate::payment_intent::PaymentIntentUpdateInternal,
        }
        "payment_attempt" => {
            crate::payment_attempt::PaymentAttemptNew,
            crate::payment_attempt::PaymentAttemptUpdateInternal,
        }
        "customer" => {
            crate::customers::CustomerNew,
            crate::customers::CustomerUpdateInternal,
        }
        "refund" => {
            crate::refund::RefundNew,
            crate::refund::RefundUpdate,
            crate::refund::RefundUpdateInternal,
        }
        "mandate" => {
            crate::mandate::MandateNew,
            crate::mandate::MandateUpdateInternal,
        }
        "address" => {
            crate::address::AddressNew,
            crate::address::AddressUpdateInternal,
        }
        "payout_attempt" => {
            crate::payout_attempt::PayoutAttemptNew,
            crate::payout_attempt::PayoutAttemptUpdateInternal,
        }
        "payout" => {
            crate::payouts::PayoutsNew,
            crate::payouts::PayoutsUpdateInternal,
        }
        "payment_method" => {
            crate::payment_method::PaymentMethodNew,
            crate::payment_method::PaymentMethodUpdateInternal,
        }
        "reverse_lookup" => {
            crate::reverse_lookup::ReverseLookupNew,
        }
    }
}
