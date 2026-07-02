mod bind_params;
mod entity_type;
mod pg_type_metadata;

use diesel_async::RunQueryDsl;
use common_utils::pii;
use diesel::{
    associations::HasTable,
    debug_query,
    dsl::{Filter, Find},
    pg::{Pg, PgMetadataLookup, PgTypeMetadata},
    query_builder::{
        bind_collector::RawBytesBindCollector, AsChangeset, AsQuery, CollectedQuery,
        InsertStatement, IntoUpdateTarget, MoveableBindCollector, QueryBuilder, QueryFragment,
        UpdateStatement,
    },
    query_dsl::methods::{FilterDsl, FindDsl},
    query_source::Table,
    Insertable,
};
use error_stack::ResultExt;
use hyperswitch_masking::Secret;
use router_env::logger;

use crate::errors;

type SecretBinaryData = Secret<Vec<u8>, pii::BinaryDataStrategy>;

/// A no-op `PgMetadataLookup` for bind collection without a live connection.
///
/// For built-in PostgreSQL types (Text, Integer, Bool, etc.), `lookup_type` is
/// never called — their OIDs are statically known. Custom user-defined types
/// are not used in drainer queries, so this implementation returning placeholder
/// OIDs is safe.
struct StaticPgMetadataLookup;
impl PgMetadataLookup for StaticPgMetadataLookup {
    fn lookup_type(&mut self, _type_name: &str, _schema: Option<&str>) -> PgTypeMetadata {
        PgTypeMetadata::from_result(Ok((0, 0)))
    }
}

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
    metadata: Vec<PgTypeMetadata>,

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

impl SerializableQuery {
    pub fn entity_type(&self) -> String {
        self.entity_type.clone()
    }

    pub fn operation(&self) -> DatabaseOperation {
        self.operation
    }

    async fn from_query<Q>(
        _conn: &mut crate::PgPooledConn,
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

        let mut bc = RawBytesBindCollector::<Pg>::new();
        let mut metadata_lookup = StaticPgMetadataLookup;
        query
            .collect_binds(&mut bc, &mut metadata_lookup, &Pg)
            .change_context(errors::DatabaseError::QueryGenerationFailed)
            .attach_printable("Failed to construct bind parameters")?;
        let bind_collector = bc;

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

        query
            .execute(conn)
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
