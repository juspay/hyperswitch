use async_bb8_diesel::AsyncRunQueryDsl;
use diesel::{associations::HasTable, BoolExpressionMethods, ExpressionMethods, QueryDsl};
use error_stack::{IntoReport, ResultExt};
use router_env::tracing::{self, instrument};
use time::PrimitiveDateTime;

use crate::{
    enums,
    errors::DatabaseError,
    query::generics,
    routing_algorithm::{RoutingAlgorithm, RoutingAlgorithmMetadata, RoutingProfileMetadata},
    schema::routing_algorithm::dsl,
    PgPooledConn, StorageResult,
};

impl RoutingAlgorithm {
    #[instrument(skip(conn))]
    pub async fn insert(self, conn: &PgPooledConn) -> StorageResult<Self> {
        generics::generic_insert(conn, self).await
    }

    #[instrument(skip(conn))]
    pub async fn find_by_algorithm_id_merchant_id(
        conn: &PgPooledConn,
        algorithm_id: &str,
        merchant_id: &str,
    ) -> StorageResult<Self> {
        generics::generic_find_one::<<Self as HasTable>::Table, _, _>(
            conn,
            dsl::algorithm_id
                .eq(algorithm_id.to_owned())
                .and(dsl::merchant_id.eq(merchant_id.to_owned())),
        )
        .await
    }

    #[instrument(skip(conn))]
    pub async fn find_by_algorithm_id_profile_id(
        conn: &PgPooledConn,
        algorithm_id: &str,
        profile_id: &str,
    ) -> StorageResult<Self> {
        generics::generic_find_one::<<Self as HasTable>::Table, _, _>(
            conn,
            dsl::algorithm_id
                .eq(algorithm_id.to_owned())
                .and(dsl::profile_id.eq(profile_id.to_owned())),
        )
        .await
    }

    #[instrument(skip(conn))]
    pub async fn find_metadata_by_algorithm_id_profile_id(
        conn: &PgPooledConn,
        algorithm_id: &str,
        profile_id: &str,
    ) -> StorageResult<RoutingProfileMetadata> {
        Self::table()
            .select((
                dsl::profile_id,
                dsl::algorithm_id,
                dsl::name,
                dsl::description,
                dsl::kind,
                dsl::created_at,
                dsl::modified_at,
            ))
            .filter(
                dsl::algorithm_id
                    .eq(algorithm_id.to_owned())
                    .and(dsl::profile_id.eq(profile_id.to_owned())),
            )
            .limit(1)
            .load_async::<(
                String,
                String,
                String,
                Option<String>,
                enums::RoutingAlgorithmKind,
                PrimitiveDateTime,
                PrimitiveDateTime,
            )>(conn)
            .await
            .into_report()
            .change_context(DatabaseError::Others)?
            .into_iter()
            .next()
            .ok_or(DatabaseError::NotFound)
            .into_report()
            .map(
                |(profile_id, algorithm_id, name, description, kind, created_at, modified_at)| {
                    RoutingProfileMetadata {
                        profile_id,
                        algorithm_id,
                        name,
                        description,
                        kind,
                        created_at,
                        modified_at,
                    }
                },
            )
    }

    #[instrument(skip(conn))]
    pub async fn list_metadata_by_profile_id(
        conn: &PgPooledConn,
        profile_id: &str,
        limit: i64,
        offset: i64,
    ) -> StorageResult<Vec<RoutingAlgorithmMetadata>> {
        Ok(Self::table()
            .select((
                dsl::algorithm_id,
                dsl::name,
                dsl::description,
                dsl::kind,
                dsl::created_at,
                dsl::modified_at,
            ))
            .filter(dsl::profile_id.eq(profile_id.to_owned()))
            .limit(limit)
            .offset(offset)
            .load_async::<(
                String,
                String,
                Option<String>,
                enums::RoutingAlgorithmKind,
                PrimitiveDateTime,
                PrimitiveDateTime,
            )>(conn)
            .await
            .into_report()
            .change_context(DatabaseError::Others)?
            .into_iter()
            .map(
                |(algorithm_id, name, description, kind, created_at, modified_at)| {
                    RoutingAlgorithmMetadata {
                        algorithm_id,
                        name,
                        description,
                        kind,
                        created_at,
                        modified_at,
                    }
                },
            )
            .collect())
    }

    #[instrument(skip(conn))]
    pub async fn list_metadata_by_merchant_id(
        conn: &PgPooledConn,
        merchant_id: &str,
        limit: i64,
        offset: i64,
    ) -> StorageResult<Vec<RoutingProfileMetadata>> {
        Ok(Self::table()
            .select((
                dsl::profile_id,
                dsl::algorithm_id,
                dsl::name,
                dsl::description,
                dsl::kind,
                dsl::created_at,
                dsl::modified_at,
            ))
            .filter(dsl::merchant_id.eq(merchant_id.to_owned()))
            .limit(limit)
            .offset(offset)
            .order(dsl::modified_at.desc())
            .load_async::<(
                String,
                String,
                String,
                Option<String>,
                enums::RoutingAlgorithmKind,
                PrimitiveDateTime,
                PrimitiveDateTime,
            )>(conn)
            .await
            .into_report()
            .change_context(DatabaseError::Others)?
            .into_iter()
            .map(
                |(profile_id, algorithm_id, name, description, kind, created_at, modified_at)| {
                    RoutingProfileMetadata {
                        profile_id,
                        algorithm_id,
                        name,
                        description,
                        kind,
                        created_at,
                        modified_at,
                    }
                },
            )
            .collect())
    }
}
