use async_bb8_diesel::AsyncRunQueryDsl;
use diesel::{
    associations::HasTable,
    dsl::{count_star, sql},
    expression::SqlLiteral,
    sql_types::Integer,
    BoolExpressionMethods, ExpressionMethods, QueryDsl,
};
use error_stack::ResultExt;

use super::generics;
use crate::{
    blocklist::{Blocklist, BlocklistNew},
    errors,
    schema::blocklist::dsl,
    DatabaseConnectionWithContext, StorageResult,
};

/// Postgres `length(fingerprint_id)`, used to bucket blocklist entries by digit-string length for
/// `/blocklist/count`'s `counts_by_length` without fetching every row into the app.
///
/// Written as raw SQL rather than `define_sql_function!` because diesel cannot prove a function-call
/// expression is valid in its own `GROUP BY`: the generated `pg_length<T>: ValidGrouping<GB>`
/// delegates to `T: ValidGrouping<GB>`, and a bare column is only valid when
/// `GB: IsContainedInGroupBy<column>`, which nothing implements for `pg_length<fingerprint_id>`.
/// `SqlLiteral` sidesteps this with a blanket `impl<ST, T, GB> ValidGrouping<GB>`.
fn fingerprint_length() -> SqlLiteral<Integer> {
    sql::<Integer>("length(fingerprint_id)")
}

/// The generic kind also matches the deprecated fixed-width kinds, so rows written before
/// `generic_card_bin` existed stay visible.
fn equivalent_data_kinds(
    data_kind: common_enums::BlocklistDataKind,
) -> Vec<common_enums::BlocklistDataKind> {
    match data_kind {
        common_enums::BlocklistDataKind::GenericCardBin => vec![
            common_enums::BlocklistDataKind::GenericCardBin,
            common_enums::BlocklistDataKind::CardBin,
            common_enums::BlocklistDataKind::ExtendedCardBin,
        ],
        other => vec![other],
    }
}

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

    pub async fn find_by_processor_merchant_id_profile_id_fingerprint_ids(
        conn: &DatabaseConnectionWithContext<'_>,
        processor_merchant_id: &common_utils::id_type::MerchantId,
        profile_id: &common_utils::id_type::ProfileId,
        fingerprint_ids: Vec<String>,
    ) -> StorageResult<Option<Self>> {
        generics::generic_find_one_optional::<<Self as HasTable>::Table, _, _>(
            conn,
            dsl::processor_merchant_id
                .eq(processor_merchant_id.to_owned())
                .or(dsl::processor_merchant_id
                    .is_null()
                    .and(dsl::merchant_id.eq(processor_merchant_id.to_owned())))
                .and(dsl::fingerprint_id.eq_any(fingerprint_ids))
                .and(
                    dsl::profile_id
                        .eq(profile_id.to_owned())
                        .or(dsl::profile_id.is_null()),
                ),
        )
        .await
    }

    /// Batched BIN-only lookup: all BIN-kind blocklist entries for this merchant/profile whose
    /// stored BIN is in `card_bins`. PAN-fingerprint entries (`data_kind = payment_method`) are
    /// excluded — they hold vault HMACs of full card numbers, which cannot be matched from a
    /// BIN. Uses the same processor-merchant-with-legacy-fallback and profile-or-merchant-wide
    /// predicates as the single-entry lookups.
    pub async fn find_by_processor_merchant_id_profile_id_card_bins(
        conn: &DatabaseConnectionWithContext<'_>,
        processor_merchant_id: &common_utils::id_type::MerchantId,
        profile_id: &common_utils::id_type::ProfileId,
        card_bins: Vec<String>,
    ) -> StorageResult<Vec<Self>> {
        generics::generic_filter::<<Self as HasTable>::Table, _, _, _>(
            conn,
            dsl::processor_merchant_id
                .eq(processor_merchant_id.to_owned())
                .or(dsl::processor_merchant_id
                    .is_null()
                    .and(dsl::merchant_id.eq(processor_merchant_id.to_owned())))
                .and(dsl::data_kind.eq_any(equivalent_data_kinds(
                    common_enums::BlocklistDataKind::GenericCardBin,
                )))
                .and(dsl::fingerprint_id.eq_any(card_bins))
                .and(
                    dsl::profile_id
                        .eq(profile_id.to_owned())
                        .or(dsl::profile_id.is_null()),
                ),
            None,
            None,
            Some(dsl::created_at.desc()),
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
        let data_kinds = equivalent_data_kinds(data_kind);
        generics::generic_filter::<<Self as HasTable>::Table, _, _, _>(
            conn,
            dsl::processor_merchant_id
                .eq(processor_merchant_id.to_owned())
                .or(dsl::processor_merchant_id
                    .is_null()
                    .and(dsl::merchant_id.eq(processor_merchant_id.to_owned())))
                .and(dsl::data_kind.eq_any(data_kinds)),
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
        let data_kinds = equivalent_data_kinds(data_kind);
        generics::generic_filter::<<Self as HasTable>::Table, _, _, _>(
            conn,
            dsl::processor_merchant_id
                .eq(processor_merchant_id.to_owned())
                .or(dsl::processor_merchant_id
                    .is_null()
                    .and(dsl::merchant_id.eq(processor_merchant_id.to_owned())))
                .and(dsl::data_kind.eq_any(data_kinds))
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
        let data_kinds = equivalent_data_kinds(data_kind);
        generics::generic_count::<<Self as HasTable>::Table, _>(
            conn,
            dsl::processor_merchant_id
                .eq(processor_merchant_id.to_owned())
                .or(dsl::processor_merchant_id
                    .is_null()
                    .and(dsl::merchant_id.eq(processor_merchant_id.to_owned())))
                .and(dsl::data_kind.eq_any(data_kinds)),
        )
        .await
    }

    pub async fn get_count_by_processor_merchant_id_profile_id_data_kind(
        conn: &DatabaseConnectionWithContext<'_>,
        processor_merchant_id: &common_utils::id_type::MerchantId,
        profile_id: &common_utils::id_type::ProfileId,
        data_kind: common_enums::BlocklistDataKind,
    ) -> StorageResult<usize> {
        let data_kinds = equivalent_data_kinds(data_kind);
        generics::generic_count::<<Self as HasTable>::Table, _>(
            conn,
            dsl::processor_merchant_id
                .eq(processor_merchant_id.to_owned())
                .or(dsl::processor_merchant_id
                    .is_null()
                    .and(dsl::merchant_id.eq(processor_merchant_id.to_owned())))
                .and(dsl::data_kind.eq_any(data_kinds))
                .and(
                    dsl::profile_id
                        .eq(profile_id.to_owned())
                        .or(dsl::profile_id.is_null()),
                ),
        )
        .await
    }

    pub async fn count_by_fingerprint_length_processor_merchant_id_profile_id_data_kind(
        conn: &DatabaseConnectionWithContext<'_>,
        processor_merchant_id: &common_utils::id_type::MerchantId,
        profile_id: &common_utils::id_type::ProfileId,
        data_kind: common_enums::BlocklistDataKind,
    ) -> StorageResult<Vec<(i32, i64)>> {
        let data_kinds = equivalent_data_kinds(data_kind);
        let query = <Self as HasTable>::table()
            .filter(
                dsl::processor_merchant_id
                    .eq(processor_merchant_id.to_owned())
                    .or(dsl::processor_merchant_id
                        .is_null()
                        .and(dsl::merchant_id.eq(processor_merchant_id.to_owned())))
                    .and(dsl::data_kind.eq_any(data_kinds))
                    .and(
                        dsl::profile_id
                            .eq(profile_id.to_owned())
                            .or(dsl::profile_id.is_null()),
                    ),
            )
            .group_by(fingerprint_length())
            .select((fingerprint_length(), count_star()));

        generics::db_metrics::track_database_call::<<Self as HasTable>::Table, _, _>(
            conn.request_id(),
            conn.event_emitter(),
            generics::db_metrics::DatabaseOperation::Count,
            query.get_results_async(conn.raw_connection()),
        )
        .await
        .map_err(|e| error_stack::report!(e))
        .change_context(errors::DatabaseError::Others)
        .attach_printable("Failed to count blocklist entries by fingerprint length")
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
