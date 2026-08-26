use async_bb8_diesel::AsyncRunQueryDsl;
use diesel::{associations::HasTable, BoolExpressionMethods, ExpressionMethods};
use error_stack::ResultExt;

use super::generics;
use crate::{
    blocklist::{Blocklist, BlocklistNew},
    errors,
    schema::blocklist::dsl,
    DatabaseConnectionWithContext, StorageResult,
};

impl BlocklistNew {
    pub async fn insert(
        self,
        conn: &DatabaseConnectionWithContext<'_>,
    ) -> StorageResult<Blocklist> {
        generics::generic_insert(conn, self).await
    }

    pub async fn bulk_insert_on_conflict_do_nothing(
        conn: &DatabaseConnectionWithContext<'_>,
        entries: Vec<Self>,
    ) -> StorageResult<usize> {
        let query = diesel::insert_into(<Blocklist as HasTable>::table())
            .values(entries)
            .on_conflict((
                dsl::processor_merchant_id,
                dsl::profile_id,
                dsl::fingerprint_id,
            ))
            .do_nothing();

        generics::db_metrics::track_database_call::<<Blocklist as HasTable>::Table, _, _>(
            conn.request_id(),
            conn.event_emitter(),
            generics::db_metrics::DatabaseOperation::Insert,
            query.execute_async(conn.raw_connection()),
        )
        .await
        .map_err(|e| error_stack::report!(e))
        .change_context(errors::DatabaseError::Others)
        .attach_printable("Failed to bulk insert blocklist entries")
    }
}

impl Blocklist {
    pub async fn find_by_processor_merchant_id_fingerprint_id(
        conn: &DatabaseConnectionWithContext<'_>,
        processor_merchant_id: &common_utils::id_type::MerchantId,
        fingerprint_id: &str,
    ) -> StorageResult<Self> {
        generics::generic_find_one::<<Self as HasTable>::Table, _, _>(
            conn,
            dsl::processor_merchant_id
                .eq(processor_merchant_id.to_owned())
                .and(dsl::fingerprint_id.eq(fingerprint_id.to_owned())),
        )
        .await
    }

    // Rows written before profile scoping have a NULL profile_id and block the whole merchant,
    // so they must keep matching every profile.
    pub async fn find_by_processor_merchant_id_profile_id_fingerprint_id(
        conn: &DatabaseConnectionWithContext<'_>,
        processor_merchant_id: &common_utils::id_type::MerchantId,
        profile_id: &common_utils::id_type::ProfileId,
        fingerprint_id: &str,
    ) -> StorageResult<Self> {
        generics::generic_find_one::<<Self as HasTable>::Table, _, _>(
            conn,
            dsl::processor_merchant_id
                .eq(processor_merchant_id.to_owned())
                .and(dsl::fingerprint_id.eq(fingerprint_id.to_owned()))
                .and(
                    dsl::profile_id
                        .eq(profile_id.to_owned())
                        .or(dsl::profile_id.is_null()),
                ),
        )
        .await
    }

    // Fallback function for stagger release - finds by merchant_id when processor_merchant_id is NULL
    pub async fn find_by_merchant_id_fingerprint_id(
        conn: &DatabaseConnectionWithContext<'_>,
        processor_merchant_id: &common_utils::id_type::MerchantId,
        fingerprint_id: &str,
    ) -> StorageResult<Self> {
        generics::generic_find_one::<<Self as HasTable>::Table, _, _>(
            conn,
            dsl::merchant_id
                .eq(processor_merchant_id.to_owned())
                .and(dsl::fingerprint_id.eq(fingerprint_id.to_owned())),
        )
        .await
    }

    pub async fn find_by_merchant_id_profile_id_fingerprint_id(
        conn: &DatabaseConnectionWithContext<'_>,
        processor_merchant_id: &common_utils::id_type::MerchantId,
        profile_id: &common_utils::id_type::ProfileId,
        fingerprint_id: &str,
    ) -> StorageResult<Self> {
        generics::generic_find_one::<<Self as HasTable>::Table, _, _>(
            conn,
            dsl::merchant_id
                .eq(processor_merchant_id.to_owned())
                .and(dsl::fingerprint_id.eq(fingerprint_id.to_owned()))
                .and(
                    dsl::profile_id
                        .eq(profile_id.to_owned())
                        .or(dsl::profile_id.is_null()),
                ),
        )
        .await
    }

    pub async fn list_by_processor_merchant_id_data_kind(
        conn: &DatabaseConnectionWithContext<'_>,
        processor_merchant_id: &common_utils::id_type::MerchantId,
        data_kind: common_enums::BlocklistDataKind,
        limit: i64,
        offset: i64,
    ) -> StorageResult<Vec<Self>> {
        generics::generic_filter::<<Self as HasTable>::Table, _, _, _>(
            conn,
            dsl::processor_merchant_id
                .eq(processor_merchant_id.to_owned())
                .or(dsl::processor_merchant_id
                    .is_null()
                    .and(dsl::merchant_id.eq(processor_merchant_id.to_owned())))
                .and(dsl::data_kind.eq(data_kind.to_owned())),
            Some(limit),
            Some(offset),
            Some(dsl::created_at.desc()),
        )
        .await
    }

    pub async fn list_by_processor_merchant_id_profile_id_data_kind(
        conn: &DatabaseConnectionWithContext<'_>,
        processor_merchant_id: &common_utils::id_type::MerchantId,
        profile_id: &common_utils::id_type::ProfileId,
        data_kind: common_enums::BlocklistDataKind,
        limit: i64,
        offset: i64,
    ) -> StorageResult<Vec<Self>> {
        generics::generic_filter::<<Self as HasTable>::Table, _, _, _>(
            conn,
            dsl::processor_merchant_id
                .eq(processor_merchant_id.to_owned())
                .or(dsl::processor_merchant_id
                    .is_null()
                    .and(dsl::merchant_id.eq(processor_merchant_id.to_owned())))
                .and(dsl::data_kind.eq(data_kind.to_owned()))
                .and(
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

    pub async fn get_count_by_processor_merchant_id_data_kind(
        conn: &DatabaseConnectionWithContext<'_>,
        processor_merchant_id: &common_utils::id_type::MerchantId,
        data_kind: common_enums::BlocklistDataKind,
    ) -> StorageResult<usize> {
        generics::generic_count::<<Self as HasTable>::Table, _>(
            conn,
            dsl::processor_merchant_id
                .eq(processor_merchant_id.to_owned())
                .or(dsl::processor_merchant_id
                    .is_null()
                    .and(dsl::merchant_id.eq(processor_merchant_id.to_owned())))
                .and(dsl::data_kind.eq(data_kind.to_owned())),
        )
        .await
    }

    pub async fn get_count_by_processor_merchant_id_profile_id_data_kind(
        conn: &DatabaseConnectionWithContext<'_>,
        processor_merchant_id: &common_utils::id_type::MerchantId,
        profile_id: &common_utils::id_type::ProfileId,
        data_kind: common_enums::BlocklistDataKind,
    ) -> StorageResult<usize> {
        generics::generic_count::<<Self as HasTable>::Table, _>(
            conn,
            dsl::processor_merchant_id
                .eq(processor_merchant_id.to_owned())
                .or(dsl::processor_merchant_id
                    .is_null()
                    .and(dsl::merchant_id.eq(processor_merchant_id.to_owned())))
                .and(dsl::data_kind.eq(data_kind.to_owned()))
                .and(
                    dsl::profile_id
                        .eq(profile_id.to_owned())
                        .or(dsl::profile_id.is_null()),
                ),
        )
        .await
    }

    pub async fn list_by_processor_merchant_id(
        conn: &DatabaseConnectionWithContext<'_>,
        processor_merchant_id: &common_utils::id_type::MerchantId,
    ) -> StorageResult<Vec<Self>> {
        generics::generic_filter::<<Self as HasTable>::Table, _, _, _>(
            conn,
            dsl::processor_merchant_id
                .eq(processor_merchant_id.to_owned())
                .or(dsl::processor_merchant_id
                    .is_null()
                    .and(dsl::merchant_id.eq(processor_merchant_id.to_owned()))),
            None,
            None,
            Some(dsl::created_at.desc()),
        )
        .await
    }

    pub async fn delete_by_processor_merchant_id_fingerprint_id(
        conn: &DatabaseConnectionWithContext<'_>,
        processor_merchant_id: &common_utils::id_type::MerchantId,
        fingerprint_id: &str,
    ) -> StorageResult<Self> {
        generics::generic_delete_one_with_result::<<Self as HasTable>::Table, _, _>(
            conn,
            dsl::processor_merchant_id
                .eq(processor_merchant_id.to_owned())
                .and(dsl::fingerprint_id.eq(fingerprint_id.to_owned())),
        )
        .await
    }

    pub async fn delete_by_processor_merchant_id_profile_id_fingerprint_id(
        conn: &DatabaseConnectionWithContext<'_>,
        processor_merchant_id: &common_utils::id_type::MerchantId,
        profile_id: &common_utils::id_type::ProfileId,
        fingerprint_id: &str,
    ) -> StorageResult<Self> {
        generics::generic_delete_one_with_result::<<Self as HasTable>::Table, _, _>(
            conn,
            dsl::processor_merchant_id
                .eq(processor_merchant_id.to_owned())
                .and(dsl::fingerprint_id.eq(fingerprint_id.to_owned()))
                .and(
                    dsl::profile_id
                        .eq(profile_id.to_owned())
                        .or(dsl::profile_id.is_null()),
                ),
        )
        .await
    }

    // Fallback function for stagger release - deletes by merchant_id when processor_merchant_id is NULL
    pub async fn delete_by_merchant_id_fingerprint_id(
        conn: &DatabaseConnectionWithContext<'_>,
        processor_merchant_id: &common_utils::id_type::MerchantId,
        fingerprint_id: &str,
    ) -> StorageResult<Self> {
        generics::generic_delete_one_with_result::<<Self as HasTable>::Table, _, _>(
            conn,
            dsl::merchant_id
                .eq(processor_merchant_id.to_owned())
                .and(dsl::fingerprint_id.eq(fingerprint_id.to_owned())),
        )
        .await
    }

    pub async fn delete_by_merchant_id_profile_id_fingerprint_id(
        conn: &DatabaseConnectionWithContext<'_>,
        processor_merchant_id: &common_utils::id_type::MerchantId,
        profile_id: &common_utils::id_type::ProfileId,
        fingerprint_id: &str,
    ) -> StorageResult<Self> {
        generics::generic_delete_one_with_result::<<Self as HasTable>::Table, _, _>(
            conn,
            dsl::merchant_id
                .eq(processor_merchant_id.to_owned())
                .and(dsl::fingerprint_id.eq(fingerprint_id.to_owned()))
                .and(
                    dsl::profile_id
                        .eq(profile_id.to_owned())
                        .or(dsl::profile_id.is_null()),
                ),
        )
        .await
    }
}
