use common_enums::BatchBlocklistJobType;
use common_utils::id_type;
use diesel::{
    associations::HasTable,
    pg::Pg,
    sql_types::{Bool, Nullable},
    BoolExpressionMethods, ExpressionMethods, NullableExpressionMethods,
};

use super::generics;
use crate::{
    batch_blocklist_job::{BatchBlocklistJob, BatchBlocklistJobNew, BatchBlocklistJobUpdate},
    schema::batch_blocklist_jobs::dsl,
    DatabaseConnectionWithContext, StorageResult,
};

impl BatchBlocklistJobNew {
    pub async fn insert(
        self,
        conn: &DatabaseConnectionWithContext<'_>,
    ) -> StorageResult<BatchBlocklistJob> {
        generics::generic_insert(conn, self).await
    }
}

impl BatchBlocklistJob {
    fn merchant_job_type_filter(
        merchant_id: &str,
        job_type: Option<BatchBlocklistJobType>,
    ) -> Box<
        dyn diesel::BoxableExpression<<Self as HasTable>::Table, Pg, SqlType = Nullable<Bool>>
            + 'static,
    > {
        let merchant_filter = dsl::merchant_id.eq(merchant_id.to_owned());

        match job_type {
            // Omitting the filter must include every job, including legacy rows with NULL job_type.
            None => Box::new(merchant_filter.nullable()),
            // Rows written before job types were introduced are uploads.
            Some(BatchBlocklistJobType::Upload) => Box::new(
                merchant_filter.and(
                    dsl::job_type
                        .eq(BatchBlocklistJobType::Upload)
                        .or(dsl::job_type.is_null()),
                ),
            ),
            Some(job_type) => Box::new(merchant_filter.and(dsl::job_type.eq(job_type))),
        }
    }

    pub async fn find_by_id_merchant_id(
        conn: &DatabaseConnectionWithContext<'_>,
        id: &str,
        merchant_id: &str,
    ) -> StorageResult<Self> {
        generics::generic_find_one::<<Self as HasTable>::Table, _, _>(
            conn,
            dsl::id
                .eq(id.to_owned())
                .and(dsl::merchant_id.eq(merchant_id.to_owned())),
        )
        .await
    }

    // The listing and the count must be given the same filter, or the page and the total disagree.
    pub async fn list_by_merchant_id(
        conn: &DatabaseConnectionWithContext<'_>,
        merchant_id: &str,
        job_type: Option<BatchBlocklistJobType>,
        limit: i64,
        offset: i64,
    ) -> StorageResult<Vec<Self>> {
        generics::generic_filter::<<Self as HasTable>::Table, _, _, _>(
            conn,
            Self::merchant_job_type_filter(merchant_id, job_type),
            Some(limit),
            Some(offset),
            Some(dsl::created_at.desc()),
        )
        .await
    }

    pub async fn count_by_merchant_id(
        conn: &DatabaseConnectionWithContext<'_>,
        merchant_id: &str,
        job_type: Option<BatchBlocklistJobType>,
    ) -> StorageResult<usize> {
        generics::generic_count::<<Self as HasTable>::Table, _>(
            conn,
            Self::merchant_job_type_filter(merchant_id, job_type),
        )
        .await
    }

    // Jobs with a NULL profile_id predate profile scoping, so they stay visible under all profiles.
    pub async fn list_by_merchant_id_profile_id(
        conn: &DatabaseConnectionWithContext<'_>,
        merchant_id: &str,
        profile_id: &id_type::ProfileId,
        job_type: Option<BatchBlocklistJobType>,
        limit: i64,
        offset: i64,
    ) -> StorageResult<Vec<Self>> {
        generics::generic_filter::<<Self as HasTable>::Table, _, _, _>(
            conn,
            Self::merchant_job_type_filter(merchant_id, job_type).and(
                dsl::profile_id
                    .eq(profile_id.to_owned())
                    .or(dsl::profile_id.is_null()),
            ),
            Some(limit),
            Some(offset),
            Some(dsl::created_at.desc()),
        )
        .await
    }

    pub async fn count_by_merchant_id_profile_id(
        conn: &DatabaseConnectionWithContext<'_>,
        merchant_id: &str,
        profile_id: &id_type::ProfileId,
        job_type: Option<BatchBlocklistJobType>,
    ) -> StorageResult<usize> {
        generics::generic_count::<<Self as HasTable>::Table, _>(
            conn,
            Self::merchant_job_type_filter(merchant_id, job_type).and(
                dsl::profile_id
                    .eq(profile_id.to_owned())
                    .or(dsl::profile_id.is_null()),
            ),
        )
        .await
    }

    pub async fn update_by_id_merchant_id(
        conn: &DatabaseConnectionWithContext<'_>,
        id: &str,
        merchant_id: &str,
        update: BatchBlocklistJobUpdate,
    ) -> StorageResult<Self> {
        generics::generic_update_with_unique_predicate_get_result::<
            <Self as HasTable>::Table,
            _,
            _,
            _,
        >(
            conn,
            dsl::id
                .eq(id.to_owned())
                .and(dsl::merchant_id.eq(merchant_id.to_owned())),
            update,
        )
        .await
    }
}
