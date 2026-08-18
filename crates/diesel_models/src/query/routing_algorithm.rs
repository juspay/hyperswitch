use async_bb8_diesel::AsyncRunQueryDsl;
use diesel::{associations::HasTable, BoolExpressionMethods, ExpressionMethods, QueryDsl};
use error_stack::{report, ResultExt};
use time::PrimitiveDateTime;

use crate::{
    enums,
    errors::DatabaseError,
    query::generics,
    routing_algorithm::{RoutingAlgorithm, RoutingProfileMetadata},
    schema::routing_algorithm::dsl,
    PgPooledConn, StorageResult,
};

impl RoutingAlgorithm {
    pub async fn insert(self, conn: &PgPooledConn) -> StorageResult<Self> {
        generics::generic_insert(conn, self).await
    }

    pub async fn find_by_algorithm_id_merchant_id(
        conn: &PgPooledConn,
        algorithm_id: &common_utils::id_type::RoutingId,
        merchant_id: &common_utils::id_type::MerchantId,
    ) -> StorageResult<Self> {
        generics::generic_find_one::<<Self as HasTable>::Table, _, _>(
            conn,
            dsl::algorithm_id
                .eq(algorithm_id.to_owned())
                .and(dsl::merchant_id.eq(merchant_id.to_owned())),
        )
        .await
    }

    pub async fn find_by_algorithm_id_processor_merchant_id(
        conn: &PgPooledConn,
        algorithm_id: &common_utils::id_type::RoutingId,
        processor_merchant_id: &common_utils::id_type::MerchantId,
    ) -> StorageResult<Self> {
        generics::generic_find_one::<<Self as HasTable>::Table, _, _>(
            conn,
            dsl::algorithm_id
                .eq(algorithm_id.to_owned())
                .and(dsl::processor_merchant_id.eq(processor_merchant_id.to_owned())),
        )
        .await
    }

    pub async fn find_by_algorithm_id_profile_id(
        conn: &PgPooledConn,
        algorithm_id: &common_utils::id_type::RoutingId,
        profile_id: &common_utils::id_type::ProfileId,
    ) -> StorageResult<Self> {
        generics::generic_find_one::<<Self as HasTable>::Table, _, _>(
            conn,
            dsl::algorithm_id
                .eq(algorithm_id.to_owned())
                .and(dsl::profile_id.eq(profile_id.to_owned())),
        )
        .await
    }

    pub async fn find_metadata_by_algorithm_id_profile_id(
        conn: &PgPooledConn,
        algorithm_id: &common_utils::id_type::RoutingId,
        profile_id: &common_utils::id_type::ProfileId,
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
                dsl::algorithm_for,
            ))
            .filter(
                dsl::algorithm_id
                    .eq(algorithm_id.to_owned())
                    .and(dsl::profile_id.eq(profile_id.to_owned())),
            )
            .limit(1)
            .load_async::<(
                common_utils::id_type::ProfileId,
                common_utils::id_type::RoutingId,
                String,
                Option<String>,
                enums::RoutingAlgorithmKind,
                PrimitiveDateTime,
                PrimitiveDateTime,
                enums::TransactionType,
            )>(conn)
            .await
            .change_context(DatabaseError::Others)?
            .into_iter()
            .next()
            .ok_or(report!(DatabaseError::NotFound))
            .map(
                |(
                    profile_id,
                    algorithm_id,
                    name,
                    description,
                    kind,
                    created_at,
                    modified_at,
                    algorithm_for,
                )| {
                    RoutingProfileMetadata {
                        profile_id,
                        algorithm_id,
                        name,
                        description,
                        kind,
                        created_at,
                        modified_at,
                        algorithm_for,
                    }
                },
            )
    }

    pub async fn list_metadata_by_profile_id(
        conn: &PgPooledConn,
        profile_id: &common_utils::id_type::ProfileId,
        limit: i64,
        offset: i64,
    ) -> StorageResult<Vec<RoutingProfileMetadata>> {
        Ok(Self::table()
            .select((
                dsl::algorithm_id,
                dsl::profile_id,
                dsl::name,
                dsl::description,
                dsl::kind,
                dsl::created_at,
                dsl::modified_at,
                dsl::algorithm_for,
            ))
            .filter(dsl::profile_id.eq(profile_id.to_owned()))
            // `algorithm_id` breaks ties on `modified_at`, which bulk-created rules share. Without
            // a total order the pages of one limit/offset walk overlap and leave rules unvisited.
            .order((dsl::modified_at.desc(), dsl::algorithm_id.asc()))
            .limit(limit)
            .offset(offset)
            .load_async::<(
                common_utils::id_type::RoutingId,
                common_utils::id_type::ProfileId,
                String,
                Option<String>,
                enums::RoutingAlgorithmKind,
                PrimitiveDateTime,
                PrimitiveDateTime,
                enums::TransactionType,
            )>(conn)
            .await
            .change_context(DatabaseError::Others)?
            .into_iter()
            .map(
                |(
                    algorithm_id,
                    profile_id,
                    name,
                    description,
                    kind,
                    created_at,
                    modified_at,
                    algorithm_for,
                )| {
                    RoutingProfileMetadata {
                        algorithm_id,
                        name,
                        description,
                        kind,
                        created_at,
                        modified_at,
                        algorithm_for,
                        profile_id,
                    }
                },
            )
            .collect())
    }

    pub async fn list_metadata_by_merchant_id(
        conn: &PgPooledConn,
        merchant_id: &common_utils::id_type::MerchantId,
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
                dsl::algorithm_for,
            ))
            .filter(
                dsl::processor_merchant_id.eq(merchant_id.to_owned()).or(
                    dsl::processor_merchant_id
                        .is_null()
                        .and(dsl::merchant_id.eq(merchant_id.to_owned())),
                ),
            )
            .limit(limit)
            .offset(offset)
            // `algorithm_id` breaks ties on `modified_at`, so the pages of one walk stay disjoint.
            .order((dsl::modified_at.desc(), dsl::algorithm_id.asc()))
            .load_async::<(
                common_utils::id_type::ProfileId,
                common_utils::id_type::RoutingId,
                String,
                Option<String>,
                enums::RoutingAlgorithmKind,
                PrimitiveDateTime,
                PrimitiveDateTime,
                enums::TransactionType,
            )>(conn)
            .await
            .change_context(DatabaseError::Others)?
            .into_iter()
            .map(
                |(
                    profile_id,
                    algorithm_id,
                    name,
                    description,
                    kind,
                    created_at,
                    modified_at,
                    algorithm_for,
                )| {
                    RoutingProfileMetadata {
                        profile_id,
                        algorithm_id,
                        name,
                        description,
                        kind,
                        created_at,
                        modified_at,
                        algorithm_for,
                    }
                },
            )
            .collect())
    }

    pub async fn list_metadata_by_merchant_id_transaction_type(
        conn: &PgPooledConn,
        merchant_id: &common_utils::id_type::MerchantId,
        transaction_type: &enums::TransactionType,
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
                dsl::algorithm_for,
            ))
            .filter(
                dsl::processor_merchant_id.eq(merchant_id.to_owned()).or(
                    dsl::processor_merchant_id
                        .is_null()
                        .and(dsl::merchant_id.eq(merchant_id.to_owned())),
                ),
            )
            .filter(dsl::algorithm_for.eq(transaction_type.to_owned()))
            .limit(limit)
            .offset(offset)
            // `algorithm_id` breaks ties on `modified_at`, so the pages of one walk stay disjoint.
            .order((dsl::modified_at.desc(), dsl::algorithm_id.asc()))
            .load_async::<(
                common_utils::id_type::ProfileId,
                common_utils::id_type::RoutingId,
                String,
                Option<String>,
                enums::RoutingAlgorithmKind,
                PrimitiveDateTime,
                PrimitiveDateTime,
                enums::TransactionType,
            )>(conn)
            .await
            .change_context(DatabaseError::Others)?
            .into_iter()
            .map(
                |(
                    profile_id,
                    algorithm_id,
                    name,
                    description,
                    kind,
                    created_at,
                    modified_at,
                    algorithm_for,
                )| {
                    RoutingProfileMetadata {
                        profile_id,
                        algorithm_id,
                        name,
                        description,
                        kind,
                        created_at,
                        modified_at,
                        algorithm_for,
                    }
                },
            )
            .collect())
    }

    /// Every rule id held by the given profiles, with each profile's merchant — taken from here
    /// rather than `business_profile`, which is encrypted and needs the merchant to read. The
    /// kind rides along so callers can tell which rules a migration is expected to carry.
    pub async fn rule_ids_for_profiles(
        conn: &PgPooledConn,
        profile_ids: &[common_utils::id_type::ProfileId],
    ) -> StorageResult<
        Vec<(
            common_utils::id_type::ProfileId,
            common_utils::id_type::MerchantId,
            common_utils::id_type::RoutingId,
            enums::RoutingAlgorithmKind,
        )>,
    > {
        Self::table()
            .select((
                dsl::profile_id,
                dsl::merchant_id,
                dsl::algorithm_id,
                dsl::kind,
            ))
            .filter(dsl::profile_id.eq_any(profile_ids.to_vec()))
            .order((dsl::profile_id.asc(), dsl::algorithm_id.asc()))
            .load_async::<(
                common_utils::id_type::ProfileId,
                common_utils::id_type::MerchantId,
                common_utils::id_type::RoutingId,
                enums::RoutingAlgorithmKind,
            )>(conn)
            .await
            .change_context(DatabaseError::Others)
    }

    /// A page of the profiles that hold routing rules. Grouped so the page is of profiles —
    /// paginating per rule would split one across pages. Ordered so paging is stable.
    pub async fn list_scope_page(
        conn: &PgPooledConn,
        limit: i64,
        offset: i64,
    ) -> StorageResult<
        Vec<(
            common_utils::id_type::ProfileId,
            common_utils::id_type::MerchantId,
        )>,
    > {
        Self::table()
            .group_by((dsl::merchant_id, dsl::profile_id))
            .select((dsl::profile_id, dsl::merchant_id))
            .order((dsl::merchant_id.asc(), dsl::profile_id.asc()))
            .limit(limit)
            .offset(offset)
            .load_async::<(
                common_utils::id_type::ProfileId,
                common_utils::id_type::MerchantId,
            )>(conn)
            .await
            .change_context(DatabaseError::Others)
    }
}
