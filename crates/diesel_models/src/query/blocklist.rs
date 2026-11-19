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
            // Legacy and NULL-profile rows use different unique indexes from processor/profile rows.
            .on_conflict_do_nothing();

        conn.track_execute::<<Blocklist as HasTable>::Table, _>(
            generics::db_metrics::DatabaseOperation::Insert,
            query,
        )
        .await
        .map_err(|e| error_stack::report!(e))
        .change_context(errors::DatabaseError::Others)
        .attach_printable("Failed to bulk insert blocklist entries")
    }
}

impl Blocklist {
    // Legacy NULL transaction namespaces match payment, never payout.

    pub async fn find_by_processor_merchant_id_fingerprint_id_for_payment(
        conn: &DatabaseConnectionWithContext<'_>,
        processor_merchant_id: &common_utils::id_type::MerchantId,
        fingerprint_id: &str,
    ) -> StorageResult<Self> {
        generics::generic_find_one::<<Self as HasTable>::Table, _, _>(
            conn,
            (dsl::processor_merchant_id
                .eq(processor_merchant_id.to_owned())
                .and(dsl::fingerprint_id.eq(fingerprint_id.to_owned())))
            .and(
                dsl::transaction_type
                    .eq(common_enums::BlocklistTransactionType::Payment)
                    .or(dsl::transaction_type.is_null()),
            ),
        )
        .await
    }
    pub async fn find_by_processor_merchant_id_fingerprint_id_for_payout(
        conn: &DatabaseConnectionWithContext<'_>,
        processor_merchant_id: &common_utils::id_type::MerchantId,
        fingerprint_id: &str,
    ) -> StorageResult<Self> {
        generics::generic_find_one::<<Self as HasTable>::Table, _, _>(
            conn,
            (dsl::processor_merchant_id
                .eq(processor_merchant_id.to_owned())
                .and(dsl::fingerprint_id.eq(fingerprint_id.to_owned())))
            .and(dsl::transaction_type.eq(common_enums::BlocklistTransactionType::Payout)),
        )
        .await
    }
    /// Returns payment and legacy NULL-namespace fingerprint rows for the resolved profile
    /// and merchant-wide entries.
    pub async fn list_by_processor_merchant_id_profile_id_fingerprint_id_for_payment(
        conn: &DatabaseConnectionWithContext<'_>,
        processor_merchant_id: &common_utils::id_type::MerchantId,
        profile_id: &common_utils::id_type::ProfileId,
        fingerprint_id: &str,
    ) -> StorageResult<Vec<Self>> {
        generics::generic_filter::<<Self as HasTable>::Table, _, _, _>(
            conn,
            (dsl::processor_merchant_id
                .eq(processor_merchant_id.to_owned())
                .or(dsl::processor_merchant_id
                    .is_null()
                    .and(dsl::merchant_id.eq(processor_merchant_id.to_owned())))
                .and(dsl::fingerprint_id.eq(fingerprint_id.to_owned()))
                .and(
                    dsl::profile_id
                        .eq(profile_id.to_owned())
                        .or(dsl::profile_id.is_null()),
                ))
            .and(
                dsl::transaction_type
                    .eq(common_enums::BlocklistTransactionType::Payment)
                    .or(dsl::transaction_type.is_null()),
            ),
            None,
            None,
            None::<dsl::created_at>,
        )
        .await
    }
    pub async fn list_by_processor_merchant_id_profile_id_fingerprint_id_for_payout(
        conn: &DatabaseConnectionWithContext<'_>,
        processor_merchant_id: &common_utils::id_type::MerchantId,
        profile_id: &common_utils::id_type::ProfileId,
        fingerprint_id: &str,
    ) -> StorageResult<Vec<Self>> {
        generics::generic_filter::<<Self as HasTable>::Table, _, _, _>(
            conn,
            (dsl::processor_merchant_id
                .eq(processor_merchant_id.to_owned())
                .or(dsl::processor_merchant_id
                    .is_null()
                    .and(dsl::merchant_id.eq(processor_merchant_id.to_owned())))
                .and(dsl::fingerprint_id.eq(fingerprint_id.to_owned()))
                .and(
                    dsl::profile_id
                        .eq(profile_id.to_owned())
                        .or(dsl::profile_id.is_null()),
                ))
            .and(dsl::transaction_type.eq(common_enums::BlocklistTransactionType::Payout)),
            None,
            None,
            None::<dsl::created_at>,
        )
        .await
    }
    pub async fn list_all_by_processor_merchant_id_profile_id_fingerprint_id(
        conn: &DatabaseConnectionWithContext<'_>,
        processor_merchant_id: &common_utils::id_type::MerchantId,
        profile_id: &common_utils::id_type::ProfileId,
        fingerprint_id: &str,
    ) -> StorageResult<Vec<Self>> {
        generics::generic_filter::<<Self as HasTable>::Table, _, _, _>(
            conn,
            dsl::processor_merchant_id
                .eq(processor_merchant_id.to_owned())
                .or(dsl::processor_merchant_id
                    .is_null()
                    .and(dsl::merchant_id.eq(processor_merchant_id.to_owned())))
                .and(dsl::fingerprint_id.eq(fingerprint_id.to_owned()))
                .and(
                    dsl::profile_id
                        .eq(profile_id.to_owned())
                        .or(dsl::profile_id.is_null()),
                ),
            None,
            None,
            None::<dsl::created_at>,
        )
        .await
    }
    // Rows written before profile scoping have a NULL profile_id and block the whole merchant,
    // so they must keep matching every profile.

    pub async fn find_by_processor_merchant_id_profile_id_fingerprint_id_for_payment(
        conn: &DatabaseConnectionWithContext<'_>,
        processor_merchant_id: &common_utils::id_type::MerchantId,
        profile_id: &common_utils::id_type::ProfileId,
        fingerprint_id: &str,
    ) -> StorageResult<Self> {
        generics::generic_find_one::<<Self as HasTable>::Table, _, _>(
            conn,
            (dsl::processor_merchant_id
                .eq(processor_merchant_id.to_owned())
                .and(dsl::fingerprint_id.eq(fingerprint_id.to_owned()))
                .and(
                    dsl::profile_id
                        .eq(profile_id.to_owned())
                        .or(dsl::profile_id.is_null()),
                ))
            .and(
                dsl::transaction_type
                    .eq(common_enums::BlocklistTransactionType::Payment)
                    .or(dsl::transaction_type.is_null()),
            ),
        )
        .await
    }

    pub async fn find_by_processor_merchant_id_profile_id_fingerprint_id_for_payout(
        conn: &DatabaseConnectionWithContext<'_>,
        processor_merchant_id: &common_utils::id_type::MerchantId,
        profile_id: &common_utils::id_type::ProfileId,
        fingerprint_id: &str,
    ) -> StorageResult<Self> {
        generics::generic_find_one::<<Self as HasTable>::Table, _, _>(
            conn,
            (dsl::processor_merchant_id
                .eq(processor_merchant_id.to_owned())
                .and(dsl::fingerprint_id.eq(fingerprint_id.to_owned()))
                .and(
                    dsl::profile_id
                        .eq(profile_id.to_owned())
                        .or(dsl::profile_id.is_null()),
                ))
            .and(dsl::transaction_type.eq(common_enums::BlocklistTransactionType::Payout)),
        )
        .await
    }

    pub async fn find_by_processor_merchant_id_profile_id_fingerprint_ids_for_payment(
        conn: &DatabaseConnectionWithContext<'_>,
        processor_merchant_id: &common_utils::id_type::MerchantId,
        profile_id: &common_utils::id_type::ProfileId,
        fingerprint_ids: Vec<String>,
    ) -> StorageResult<Option<Self>> {
        generics::generic_find_one_optional::<<Self as HasTable>::Table, _, _>(
            conn,
            (dsl::processor_merchant_id
                .eq(processor_merchant_id.to_owned())
                .or(dsl::processor_merchant_id
                    .is_null()
                    .and(dsl::merchant_id.eq(processor_merchant_id.to_owned())))
                .and(dsl::fingerprint_id.eq_any(fingerprint_ids))
                .and(
                    dsl::profile_id
                        .eq(profile_id.to_owned())
                        .or(dsl::profile_id.is_null()),
                ))
            .and(
                dsl::transaction_type
                    .eq(common_enums::BlocklistTransactionType::Payment)
                    .or(dsl::transaction_type.is_null()),
            ),
        )
        .await
    }

    pub async fn find_by_processor_merchant_id_profile_id_fingerprint_ids_for_payout(
        conn: &DatabaseConnectionWithContext<'_>,
        processor_merchant_id: &common_utils::id_type::MerchantId,
        profile_id: &common_utils::id_type::ProfileId,
        fingerprint_ids: Vec<String>,
    ) -> StorageResult<Option<Self>> {
        generics::generic_find_one_optional::<<Self as HasTable>::Table, _, _>(
            conn,
            (dsl::processor_merchant_id
                .eq(processor_merchant_id.to_owned())
                .or(dsl::processor_merchant_id
                    .is_null()
                    .and(dsl::merchant_id.eq(processor_merchant_id.to_owned())))
                .and(dsl::fingerprint_id.eq_any(fingerprint_ids))
                .and(
                    dsl::profile_id
                        .eq(profile_id.to_owned())
                        .or(dsl::profile_id.is_null()),
                ))
            .and(dsl::transaction_type.eq(common_enums::BlocklistTransactionType::Payout)),
        )
        .await
    }
    /// Batched BIN-only lookup: all BIN-kind blocklist entries for this merchant/profile whose
    /// stored BIN is in `card_bins`. PAN-fingerprint entries (`data_kind = payment_method`) are
    /// excluded — they hold vault HMACs of full card numbers, which cannot be matched from a
    /// BIN. Uses the same processor-merchant-with-legacy-fallback and profile-or-merchant-wide
    /// predicates as the single-entry lookups.
    pub async fn find_by_processor_merchant_id_profile_id_card_bins_for_payment(
        conn: &DatabaseConnectionWithContext<'_>,
        processor_merchant_id: &common_utils::id_type::MerchantId,
        profile_id: &common_utils::id_type::ProfileId,
        card_bins: Vec<String>,
    ) -> StorageResult<Vec<Self>> {
        generics::generic_filter::<<Self as HasTable>::Table, _, _, _>(
            conn,
            (dsl::processor_merchant_id
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
                ))
            .and(
                dsl::transaction_type
                    .eq(common_enums::BlocklistTransactionType::Payment)
                    .or(dsl::transaction_type.is_null()),
            ),
            None,
            None,
            Some(dsl::created_at.desc()),
        )
        .await
    }

    pub async fn find_by_processor_merchant_id_profile_id_card_bins_for_payout(
        conn: &DatabaseConnectionWithContext<'_>,
        processor_merchant_id: &common_utils::id_type::MerchantId,
        profile_id: &common_utils::id_type::ProfileId,
        card_bins: Vec<String>,
    ) -> StorageResult<Vec<Self>> {
        generics::generic_filter::<<Self as HasTable>::Table, _, _, _>(
            conn,
            (dsl::processor_merchant_id
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
                ))
            .and(dsl::transaction_type.eq(common_enums::BlocklistTransactionType::Payout)),
            None,
            None,
            Some(dsl::created_at.desc()),
        )
        .await
    }
    // Fallback function for stagger release - finds by merchant_id when processor_merchant_id is NULL

    pub async fn find_by_merchant_id_fingerprint_id_for_payment(
        conn: &DatabaseConnectionWithContext<'_>,
        processor_merchant_id: &common_utils::id_type::MerchantId,
        fingerprint_id: &str,
    ) -> StorageResult<Self> {
        generics::generic_find_one::<<Self as HasTable>::Table, _, _>(
            conn,
            (dsl::merchant_id
                .eq(processor_merchant_id.to_owned())
                .and(dsl::fingerprint_id.eq(fingerprint_id.to_owned())))
            .and(
                dsl::transaction_type
                    .eq(common_enums::BlocklistTransactionType::Payment)
                    .or(dsl::transaction_type.is_null()),
            ),
        )
        .await
    }

    pub async fn find_by_merchant_id_fingerprint_id_for_payout(
        conn: &DatabaseConnectionWithContext<'_>,
        processor_merchant_id: &common_utils::id_type::MerchantId,
        fingerprint_id: &str,
    ) -> StorageResult<Self> {
        generics::generic_find_one::<<Self as HasTable>::Table, _, _>(
            conn,
            (dsl::merchant_id
                .eq(processor_merchant_id.to_owned())
                .and(dsl::fingerprint_id.eq(fingerprint_id.to_owned())))
            .and(dsl::transaction_type.eq(common_enums::BlocklistTransactionType::Payout)),
        )
        .await
    }

    pub async fn find_by_merchant_id_profile_id_fingerprint_id_for_payment(
        conn: &DatabaseConnectionWithContext<'_>,
        processor_merchant_id: &common_utils::id_type::MerchantId,
        profile_id: &common_utils::id_type::ProfileId,
        fingerprint_id: &str,
    ) -> StorageResult<Self> {
        generics::generic_find_one::<<Self as HasTable>::Table, _, _>(
            conn,
            (dsl::merchant_id
                .eq(processor_merchant_id.to_owned())
                .and(dsl::fingerprint_id.eq(fingerprint_id.to_owned()))
                .and(
                    dsl::profile_id
                        .eq(profile_id.to_owned())
                        .or(dsl::profile_id.is_null()),
                ))
            .and(
                dsl::transaction_type
                    .eq(common_enums::BlocklistTransactionType::Payment)
                    .or(dsl::transaction_type.is_null()),
            ),
        )
        .await
    }

    pub async fn find_by_merchant_id_profile_id_fingerprint_id_for_payout(
        conn: &DatabaseConnectionWithContext<'_>,
        processor_merchant_id: &common_utils::id_type::MerchantId,
        profile_id: &common_utils::id_type::ProfileId,
        fingerprint_id: &str,
    ) -> StorageResult<Self> {
        generics::generic_find_one::<<Self as HasTable>::Table, _, _>(
            conn,
            (dsl::merchant_id
                .eq(processor_merchant_id.to_owned())
                .and(dsl::fingerprint_id.eq(fingerprint_id.to_owned()))
                .and(
                    dsl::profile_id
                        .eq(profile_id.to_owned())
                        .or(dsl::profile_id.is_null()),
                ))
            .and(dsl::transaction_type.eq(common_enums::BlocklistTransactionType::Payout)),
        )
        .await
    }

    pub async fn list_by_processor_merchant_id_data_kind_for_payment(
        conn: &DatabaseConnectionWithContext<'_>,
        processor_merchant_id: &common_utils::id_type::MerchantId,
        data_kind: common_enums::BlocklistDataKind,
        limit: i64,
        offset: i64,
    ) -> StorageResult<Vec<Self>> {
        let data_kinds = equivalent_data_kinds(data_kind);
        generics::generic_filter::<<Self as HasTable>::Table, _, _, _>(
            conn,
            (dsl::processor_merchant_id
                .eq(processor_merchant_id.to_owned())
                .or(dsl::processor_merchant_id
                    .is_null()
                    .and(dsl::merchant_id.eq(processor_merchant_id.to_owned())))
                .and(dsl::data_kind.eq_any(data_kinds)))
            .and(
                dsl::transaction_type
                    .eq(common_enums::BlocklistTransactionType::Payment)
                    .or(dsl::transaction_type.is_null()),
            ),
            Some(limit),
            Some(offset),
            Some(dsl::created_at.desc()),
        )
        .await
    }

    pub async fn list_by_processor_merchant_id_data_kind_for_payout(
        conn: &DatabaseConnectionWithContext<'_>,
        processor_merchant_id: &common_utils::id_type::MerchantId,
        data_kind: common_enums::BlocklistDataKind,
        limit: i64,
        offset: i64,
    ) -> StorageResult<Vec<Self>> {
        let data_kinds = equivalent_data_kinds(data_kind);
        generics::generic_filter::<<Self as HasTable>::Table, _, _, _>(
            conn,
            (dsl::processor_merchant_id
                .eq(processor_merchant_id.to_owned())
                .or(dsl::processor_merchant_id
                    .is_null()
                    .and(dsl::merchant_id.eq(processor_merchant_id.to_owned())))
                .and(dsl::data_kind.eq_any(data_kinds)))
            .and(dsl::transaction_type.eq(common_enums::BlocklistTransactionType::Payout)),
            Some(limit),
            Some(offset),
            Some(dsl::created_at.desc()),
        )
        .await
    }

    pub async fn list_by_processor_merchant_id_profile_id_data_kind_for_payment(
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
            (dsl::processor_merchant_id
                .eq(processor_merchant_id.to_owned())
                .or(dsl::processor_merchant_id
                    .is_null()
                    .and(dsl::merchant_id.eq(processor_merchant_id.to_owned())))
                .and(dsl::data_kind.eq_any(data_kinds))
                .and(
                    dsl::profile_id
                        .eq(profile_id.to_owned())
                        .or(dsl::profile_id.is_null()),
                ))
            .and(
                dsl::transaction_type
                    .eq(common_enums::BlocklistTransactionType::Payment)
                    .or(dsl::transaction_type.is_null()),
            ),
            Some(limit),
            Some(offset),
            Some(dsl::created_at.desc()),
        )
        .await
    }

    pub async fn list_by_processor_merchant_id_profile_id_data_kind_for_payout(
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
            (dsl::processor_merchant_id
                .eq(processor_merchant_id.to_owned())
                .or(dsl::processor_merchant_id
                    .is_null()
                    .and(dsl::merchant_id.eq(processor_merchant_id.to_owned())))
                .and(dsl::data_kind.eq_any(data_kinds))
                .and(
                    dsl::profile_id
                        .eq(profile_id.to_owned())
                        .or(dsl::profile_id.is_null()),
                ))
            .and(dsl::transaction_type.eq(common_enums::BlocklistTransactionType::Payout)),
            Some(limit),
            Some(offset),
            Some(dsl::created_at.desc()),
        )
        .await
    }

    pub async fn get_count_by_processor_merchant_id_data_kind_for_payment(
        conn: &DatabaseConnectionWithContext<'_>,
        processor_merchant_id: &common_utils::id_type::MerchantId,
        data_kind: common_enums::BlocklistDataKind,
    ) -> StorageResult<usize> {
        let data_kinds = equivalent_data_kinds(data_kind);
        generics::generic_count::<<Self as HasTable>::Table, _>(
            conn,
            (dsl::processor_merchant_id
                .eq(processor_merchant_id.to_owned())
                .or(dsl::processor_merchant_id
                    .is_null()
                    .and(dsl::merchant_id.eq(processor_merchant_id.to_owned())))
                .and(dsl::data_kind.eq_any(data_kinds)))
            .and(
                dsl::transaction_type
                    .eq(common_enums::BlocklistTransactionType::Payment)
                    .or(dsl::transaction_type.is_null()),
            ),
        )
        .await
    }

    pub async fn get_count_by_processor_merchant_id_data_kind_for_payout(
        conn: &DatabaseConnectionWithContext<'_>,
        processor_merchant_id: &common_utils::id_type::MerchantId,
        data_kind: common_enums::BlocklistDataKind,
    ) -> StorageResult<usize> {
        let data_kinds = equivalent_data_kinds(data_kind);
        generics::generic_count::<<Self as HasTable>::Table, _>(
            conn,
            (dsl::processor_merchant_id
                .eq(processor_merchant_id.to_owned())
                .or(dsl::processor_merchant_id
                    .is_null()
                    .and(dsl::merchant_id.eq(processor_merchant_id.to_owned())))
                .and(dsl::data_kind.eq_any(data_kinds)))
            .and(dsl::transaction_type.eq(common_enums::BlocklistTransactionType::Payout)),
        )
        .await
    }

    pub async fn get_all_count_by_processor_merchant_id_data_kind(
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
                .and(dsl::data_kind.eq_any(equivalent_data_kinds(data_kind))),
        )
        .await
    }

    pub async fn get_all_count_by_processor_merchant_id_profile_id_data_kind(
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
                .and(dsl::data_kind.eq_any(equivalent_data_kinds(data_kind)))
                .and(
                    dsl::profile_id
                        .eq(profile_id.to_owned())
                        .or(dsl::profile_id.is_null()),
                ),
        )
        .await
    }

    pub async fn get_count_by_processor_merchant_id_profile_id_data_kind_for_payment(
        conn: &DatabaseConnectionWithContext<'_>,
        processor_merchant_id: &common_utils::id_type::MerchantId,
        profile_id: &common_utils::id_type::ProfileId,
        data_kind: common_enums::BlocklistDataKind,
    ) -> StorageResult<usize> {
        let data_kinds = equivalent_data_kinds(data_kind);
        generics::generic_count::<<Self as HasTable>::Table, _>(
            conn,
            (dsl::processor_merchant_id
                .eq(processor_merchant_id.to_owned())
                .or(dsl::processor_merchant_id
                    .is_null()
                    .and(dsl::merchant_id.eq(processor_merchant_id.to_owned())))
                .and(dsl::data_kind.eq_any(data_kinds))
                .and(
                    dsl::profile_id
                        .eq(profile_id.to_owned())
                        .or(dsl::profile_id.is_null()),
                ))
            .and(
                dsl::transaction_type
                    .eq(common_enums::BlocklistTransactionType::Payment)
                    .or(dsl::transaction_type.is_null()),
            ),
        )
        .await
    }

    pub async fn get_count_by_processor_merchant_id_profile_id_data_kind_for_payout(
        conn: &DatabaseConnectionWithContext<'_>,
        processor_merchant_id: &common_utils::id_type::MerchantId,
        profile_id: &common_utils::id_type::ProfileId,
        data_kind: common_enums::BlocklistDataKind,
    ) -> StorageResult<usize> {
        let data_kinds = equivalent_data_kinds(data_kind);
        generics::generic_count::<<Self as HasTable>::Table, _>(
            conn,
            (dsl::processor_merchant_id
                .eq(processor_merchant_id.to_owned())
                .or(dsl::processor_merchant_id
                    .is_null()
                    .and(dsl::merchant_id.eq(processor_merchant_id.to_owned())))
                .and(dsl::data_kind.eq_any(data_kinds))
                .and(
                    dsl::profile_id
                        .eq(profile_id.to_owned())
                        .or(dsl::profile_id.is_null()),
                ))
            .and(dsl::transaction_type.eq(common_enums::BlocklistTransactionType::Payout)),
        )
        .await
    }

    pub async fn count_by_fingerprint_length_processor_merchant_id_profile_id_data_kind_for_payment(
        conn: &DatabaseConnectionWithContext<'_>,
        processor_merchant_id: &common_utils::id_type::MerchantId,
        profile_id: &common_utils::id_type::ProfileId,
        data_kind: common_enums::BlocklistDataKind,
    ) -> StorageResult<Vec<(i32, i64)>> {
        let data_kinds = equivalent_data_kinds(data_kind);
        let result = {
            let query = <Self as HasTable>::table()
                .filter(
                    (dsl::processor_merchant_id
                        .eq(processor_merchant_id.to_owned())
                        .or(dsl::processor_merchant_id
                            .is_null()
                            .and(dsl::merchant_id.eq(processor_merchant_id.to_owned())))
                        .and(dsl::data_kind.eq_any(data_kinds))
                        .and(
                            dsl::profile_id
                                .eq(profile_id.to_owned())
                                .or(dsl::profile_id.is_null()),
                        ))
                    .and(
                        dsl::transaction_type
                            .eq(common_enums::BlocklistTransactionType::Payment)
                            .or(dsl::transaction_type.is_null()),
                    ),
                )
                .group_by(fingerprint_length())
                .select((fingerprint_length(), count_star()));
            conn.track_get_results::<<Self as HasTable>::Table, _, _>(
                generics::db_metrics::DatabaseOperation::Count,
                query,
            )
            .await
        };
        result
            .map_err(|e| error_stack::report!(e))
            .change_context(errors::DatabaseError::Others)
            .attach_printable("Failed to count blocklist entries by fingerprint length")
    }

    pub async fn count_by_fingerprint_length_processor_merchant_id_profile_id_data_kind_for_payout(
        conn: &DatabaseConnectionWithContext<'_>,
        processor_merchant_id: &common_utils::id_type::MerchantId,
        profile_id: &common_utils::id_type::ProfileId,
        data_kind: common_enums::BlocklistDataKind,
    ) -> StorageResult<Vec<(i32, i64)>> {
        let data_kinds = equivalent_data_kinds(data_kind);
        let result = {
            let query = <Self as HasTable>::table()
                .filter(
                    (dsl::processor_merchant_id
                        .eq(processor_merchant_id.to_owned())
                        .or(dsl::processor_merchant_id
                            .is_null()
                            .and(dsl::merchant_id.eq(processor_merchant_id.to_owned())))
                        .and(dsl::data_kind.eq_any(data_kinds))
                        .and(
                            dsl::profile_id
                                .eq(profile_id.to_owned())
                                .or(dsl::profile_id.is_null()),
                        ))
                    .and(dsl::transaction_type.eq(common_enums::BlocklistTransactionType::Payout)),
                )
                .group_by(fingerprint_length())
                .select((fingerprint_length(), count_star()));
            conn.track_get_results::<<Self as HasTable>::Table, _, _>(
                generics::db_metrics::DatabaseOperation::Count,
                query,
            )
            .await
        };
        result
            .map_err(|e| error_stack::report!(e))
            .change_context(errors::DatabaseError::Others)
            .attach_printable("Failed to count blocklist entries by fingerprint length")
    }

    pub async fn count_all_by_fingerprint_length_processor_merchant_id_profile_id_data_kind(
        conn: &DatabaseConnectionWithContext<'_>,
        processor_merchant_id: &common_utils::id_type::MerchantId,
        profile_id: &common_utils::id_type::ProfileId,
        data_kind: common_enums::BlocklistDataKind,
    ) -> StorageResult<Vec<(i32, i64)>> {
        let query = <Self as HasTable>::table()
            .filter(
                dsl::processor_merchant_id
                    .eq(processor_merchant_id.to_owned())
                    .or(dsl::processor_merchant_id
                        .is_null()
                        .and(dsl::merchant_id.eq(processor_merchant_id.to_owned())))
                    .and(dsl::data_kind.eq_any(equivalent_data_kinds(data_kind)))
                    .and(
                        dsl::profile_id
                            .eq(profile_id.to_owned())
                            .or(dsl::profile_id.is_null()),
                    ),
            )
            .group_by(fingerprint_length())
            .select((fingerprint_length(), count_star()));
        conn.track_get_results::<<Self as HasTable>::Table, _, _>(
            generics::db_metrics::DatabaseOperation::Count,
            query,
        )
        .await
        .map_err(|e| error_stack::report!(e))
        .change_context(errors::DatabaseError::Others)
        .attach_printable("Failed to count blocklist entries by fingerprint length")
    }

    pub async fn list_by_processor_merchant_id_for_payment(
        conn: &DatabaseConnectionWithContext<'_>,
        processor_merchant_id: &common_utils::id_type::MerchantId,
    ) -> StorageResult<Vec<Self>> {
        generics::generic_filter::<<Self as HasTable>::Table, _, _, _>(
            conn,
            (dsl::processor_merchant_id
                .eq(processor_merchant_id.to_owned())
                .or(dsl::processor_merchant_id
                    .is_null()
                    .and(dsl::merchant_id.eq(processor_merchant_id.to_owned()))))
            .and(
                dsl::transaction_type
                    .eq(common_enums::BlocklistTransactionType::Payment)
                    .or(dsl::transaction_type.is_null()),
            ),
            None,
            None,
            Some(dsl::created_at.desc()),
        )
        .await
    }

    pub async fn list_by_processor_merchant_id_for_payout(
        conn: &DatabaseConnectionWithContext<'_>,
        processor_merchant_id: &common_utils::id_type::MerchantId,
    ) -> StorageResult<Vec<Self>> {
        generics::generic_filter::<<Self as HasTable>::Table, _, _, _>(
            conn,
            (dsl::processor_merchant_id
                .eq(processor_merchant_id.to_owned())
                .or(dsl::processor_merchant_id
                    .is_null()
                    .and(dsl::merchant_id.eq(processor_merchant_id.to_owned()))))
            .and(dsl::transaction_type.eq(common_enums::BlocklistTransactionType::Payout)),
            None,
            None,
            Some(dsl::created_at.desc()),
        )
        .await
    }

    pub async fn list_after_fingerprint_by_processor_merchant_id_profile_id_for_payment(
        conn: &DatabaseConnectionWithContext<'_>,
        processor_merchant_id: &common_utils::id_type::MerchantId,
        profile_id: &common_utils::id_type::ProfileId,
        after_fingerprint_id: String,
        snapshot_at: time::PrimitiveDateTime,
        limit: i64,
    ) -> StorageResult<Vec<Self>> {
        generics::generic_filter::<<Self as HasTable>::Table, _, _, _>(
            conn,
            (dsl::processor_merchant_id
                .eq(processor_merchant_id.to_owned())
                .and(dsl::fingerprint_id.gt(after_fingerprint_id))
                .and(dsl::created_at.le(snapshot_at))
                .and(
                    dsl::profile_id
                        .eq(profile_id.to_owned())
                        .or(dsl::profile_id.is_null()),
                ))
            .and(
                dsl::transaction_type
                    .eq(common_enums::BlocklistTransactionType::Payment)
                    .or(dsl::transaction_type.is_null()),
            ),
            Some(limit),
            None,
            Some(dsl::fingerprint_id.asc()),
        )
        .await
    }

    pub async fn list_after_fingerprint_by_processor_merchant_id_profile_id_for_payout(
        conn: &DatabaseConnectionWithContext<'_>,
        processor_merchant_id: &common_utils::id_type::MerchantId,
        profile_id: &common_utils::id_type::ProfileId,
        after_fingerprint_id: String,
        snapshot_at: time::PrimitiveDateTime,
        limit: i64,
    ) -> StorageResult<Vec<Self>> {
        generics::generic_filter::<<Self as HasTable>::Table, _, _, _>(
            conn,
            (dsl::processor_merchant_id
                .eq(processor_merchant_id.to_owned())
                .and(dsl::fingerprint_id.gt(after_fingerprint_id))
                .and(dsl::created_at.le(snapshot_at))
                .and(
                    dsl::profile_id
                        .eq(profile_id.to_owned())
                        .or(dsl::profile_id.is_null()),
                ))
            .and(dsl::transaction_type.eq(common_enums::BlocklistTransactionType::Payout)),
            Some(limit),
            None,
            Some(dsl::fingerprint_id.asc()),
        )
        .await
    }

    /// Clones all transaction types; used when no type filter is requested.
    pub async fn list_all_after_fingerprint_by_processor_merchant_id_profile_id(
        conn: &DatabaseConnectionWithContext<'_>,
        processor_merchant_id: &common_utils::id_type::MerchantId,
        profile_id: &common_utils::id_type::ProfileId,
        after_fingerprint_id: String,
        snapshot_at: time::PrimitiveDateTime,
        limit: i64,
    ) -> StorageResult<Vec<Self>> {
        generics::generic_filter::<<Self as HasTable>::Table, _, _, _>(
            conn,
            dsl::processor_merchant_id
                .eq(processor_merchant_id.to_owned())
                .and(dsl::fingerprint_id.gt(after_fingerprint_id))
                .and(dsl::created_at.le(snapshot_at))
                .and(
                    dsl::profile_id
                        .eq(profile_id.to_owned())
                        .or(dsl::profile_id.is_null()),
                ),
            Some(limit),
            None,
            Some(dsl::fingerprint_id.asc()),
        )
        .await
    }

    pub async fn list_after_fingerprint_by_legacy_merchant_id_profile_id_for_payment(
        conn: &DatabaseConnectionWithContext<'_>,
        merchant_id: &common_utils::id_type::MerchantId,
        profile_id: &common_utils::id_type::ProfileId,
        after_fingerprint_id: String,
        snapshot_at: time::PrimitiveDateTime,
        limit: i64,
    ) -> StorageResult<Vec<Self>> {
        generics::generic_filter::<<Self as HasTable>::Table, _, _, _>(
            conn,
            (dsl::merchant_id
                .eq(merchant_id.to_owned())
                .and(dsl::processor_merchant_id.is_null())
                .and(dsl::fingerprint_id.gt(after_fingerprint_id))
                .and(dsl::created_at.le(snapshot_at))
                .and(
                    dsl::profile_id
                        .eq(profile_id.to_owned())
                        .or(dsl::profile_id.is_null()),
                ))
            .and(
                dsl::transaction_type
                    .eq(common_enums::BlocklistTransactionType::Payment)
                    .or(dsl::transaction_type.is_null()),
            ),
            Some(limit),
            None,
            Some(dsl::fingerprint_id.asc()),
        )
        .await
    }

    pub async fn list_after_fingerprint_by_legacy_merchant_id_profile_id_for_payout(
        conn: &DatabaseConnectionWithContext<'_>,
        merchant_id: &common_utils::id_type::MerchantId,
        profile_id: &common_utils::id_type::ProfileId,
        after_fingerprint_id: String,
        snapshot_at: time::PrimitiveDateTime,
        limit: i64,
    ) -> StorageResult<Vec<Self>> {
        generics::generic_filter::<<Self as HasTable>::Table, _, _, _>(
            conn,
            (dsl::merchant_id
                .eq(merchant_id.to_owned())
                .and(dsl::processor_merchant_id.is_null())
                .and(dsl::fingerprint_id.gt(after_fingerprint_id))
                .and(dsl::created_at.le(snapshot_at))
                .and(
                    dsl::profile_id
                        .eq(profile_id.to_owned())
                        .or(dsl::profile_id.is_null()),
                ))
            .and(dsl::transaction_type.eq(common_enums::BlocklistTransactionType::Payout)),
            Some(limit),
            None,
            Some(dsl::fingerprint_id.asc()),
        )
        .await
    }

    /// Clones all transaction types from legacy rows; used when no type filter is requested.
    pub async fn list_all_after_fingerprint_by_legacy_merchant_id_profile_id(
        conn: &DatabaseConnectionWithContext<'_>,
        merchant_id: &common_utils::id_type::MerchantId,
        profile_id: &common_utils::id_type::ProfileId,
        after_fingerprint_id: String,
        snapshot_at: time::PrimitiveDateTime,
        limit: i64,
    ) -> StorageResult<Vec<Self>> {
        generics::generic_filter::<<Self as HasTable>::Table, _, _, _>(
            conn,
            dsl::merchant_id
                .eq(merchant_id.to_owned())
                .and(dsl::processor_merchant_id.is_null())
                .and(dsl::fingerprint_id.gt(after_fingerprint_id))
                .and(dsl::created_at.le(snapshot_at))
                .and(
                    dsl::profile_id
                        .eq(profile_id.to_owned())
                        .or(dsl::profile_id.is_null()),
                ),
            Some(limit),
            None,
            Some(dsl::fingerprint_id.asc()),
        )
        .await
    }

    pub async fn delete_by_processor_merchant_id_fingerprint_id_for_payment(
        conn: &DatabaseConnectionWithContext<'_>,
        processor_merchant_id: &common_utils::id_type::MerchantId,
        fingerprint_id: &str,
    ) -> StorageResult<Self> {
        generics::generic_delete_one_with_result::<<Self as HasTable>::Table, _, _>(
            conn,
            (dsl::processor_merchant_id
                .eq(processor_merchant_id.to_owned())
                .and(dsl::fingerprint_id.eq(fingerprint_id.to_owned())))
            .and(
                dsl::transaction_type
                    .eq(common_enums::BlocklistTransactionType::Payment)
                    .or(dsl::transaction_type.is_null()),
            ),
        )
        .await
    }

    pub async fn delete_by_processor_merchant_id_fingerprint_id_for_payout(
        conn: &DatabaseConnectionWithContext<'_>,
        processor_merchant_id: &common_utils::id_type::MerchantId,
        fingerprint_id: &str,
    ) -> StorageResult<Self> {
        generics::generic_delete_one_with_result::<<Self as HasTable>::Table, _, _>(
            conn,
            (dsl::processor_merchant_id
                .eq(processor_merchant_id.to_owned())
                .and(dsl::fingerprint_id.eq(fingerprint_id.to_owned())))
            .and(dsl::transaction_type.eq(common_enums::BlocklistTransactionType::Payout)),
        )
        .await
    }

    pub async fn delete_by_processor_merchant_id_profile_id_fingerprint_id_for_payment(
        conn: &DatabaseConnectionWithContext<'_>,
        processor_merchant_id: &common_utils::id_type::MerchantId,
        profile_id: &common_utils::id_type::ProfileId,
        fingerprint_id: &str,
    ) -> StorageResult<Self> {
        generics::generic_delete_one_with_result::<<Self as HasTable>::Table, _, _>(
            conn,
            (dsl::processor_merchant_id
                .eq(processor_merchant_id.to_owned())
                .and(dsl::fingerprint_id.eq(fingerprint_id.to_owned()))
                .and(
                    dsl::profile_id
                        .eq(profile_id.to_owned())
                        .or(dsl::profile_id.is_null()),
                ))
            .and(
                dsl::transaction_type
                    .eq(common_enums::BlocklistTransactionType::Payment)
                    .or(dsl::transaction_type.is_null()),
            ),
        )
        .await
    }

    pub async fn delete_by_processor_merchant_id_profile_id_fingerprint_id_for_payout(
        conn: &DatabaseConnectionWithContext<'_>,
        processor_merchant_id: &common_utils::id_type::MerchantId,
        profile_id: &common_utils::id_type::ProfileId,
        fingerprint_id: &str,
    ) -> StorageResult<Self> {
        generics::generic_delete_one_with_result::<<Self as HasTable>::Table, _, _>(
            conn,
            (dsl::processor_merchant_id
                .eq(processor_merchant_id.to_owned())
                .and(dsl::fingerprint_id.eq(fingerprint_id.to_owned()))
                .and(
                    dsl::profile_id
                        .eq(profile_id.to_owned())
                        .or(dsl::profile_id.is_null()),
                ))
            .and(dsl::transaction_type.eq(common_enums::BlocklistTransactionType::Payout)),
        )
        .await
    }
    // Fallback function for stagger release - deletes by merchant_id when processor_merchant_id is NULL

    pub async fn delete_by_merchant_id_fingerprint_id_for_payment(
        conn: &DatabaseConnectionWithContext<'_>,
        processor_merchant_id: &common_utils::id_type::MerchantId,
        fingerprint_id: &str,
    ) -> StorageResult<Self> {
        generics::generic_delete_one_with_result::<<Self as HasTable>::Table, _, _>(
            conn,
            (dsl::merchant_id
                .eq(processor_merchant_id.to_owned())
                .and(dsl::fingerprint_id.eq(fingerprint_id.to_owned())))
            .and(
                dsl::transaction_type
                    .eq(common_enums::BlocklistTransactionType::Payment)
                    .or(dsl::transaction_type.is_null()),
            ),
        )
        .await
    }

    pub async fn delete_by_merchant_id_fingerprint_id_for_payout(
        conn: &DatabaseConnectionWithContext<'_>,
        processor_merchant_id: &common_utils::id_type::MerchantId,
        fingerprint_id: &str,
    ) -> StorageResult<Self> {
        generics::generic_delete_one_with_result::<<Self as HasTable>::Table, _, _>(
            conn,
            (dsl::merchant_id
                .eq(processor_merchant_id.to_owned())
                .and(dsl::fingerprint_id.eq(fingerprint_id.to_owned())))
            .and(dsl::transaction_type.eq(common_enums::BlocklistTransactionType::Payout)),
        )
        .await
    }

    pub async fn delete_by_merchant_id_profile_id_fingerprint_id_for_payment(
        conn: &DatabaseConnectionWithContext<'_>,
        processor_merchant_id: &common_utils::id_type::MerchantId,
        profile_id: &common_utils::id_type::ProfileId,
        fingerprint_id: &str,
    ) -> StorageResult<Self> {
        generics::generic_delete_one_with_result::<<Self as HasTable>::Table, _, _>(
            conn,
            (dsl::merchant_id
                .eq(processor_merchant_id.to_owned())
                .and(dsl::fingerprint_id.eq(fingerprint_id.to_owned()))
                .and(
                    dsl::profile_id
                        .eq(profile_id.to_owned())
                        .or(dsl::profile_id.is_null()),
                ))
            .and(
                dsl::transaction_type
                    .eq(common_enums::BlocklistTransactionType::Payment)
                    .or(dsl::transaction_type.is_null()),
            ),
        )
        .await
    }

    pub async fn delete_by_merchant_id_profile_id_fingerprint_id_for_payout(
        conn: &DatabaseConnectionWithContext<'_>,
        processor_merchant_id: &common_utils::id_type::MerchantId,
        profile_id: &common_utils::id_type::ProfileId,
        fingerprint_id: &str,
    ) -> StorageResult<Self> {
        generics::generic_delete_one_with_result::<<Self as HasTable>::Table, _, _>(
            conn,
            (dsl::merchant_id
                .eq(processor_merchant_id.to_owned())
                .and(dsl::fingerprint_id.eq(fingerprint_id.to_owned()))
                .and(
                    dsl::profile_id
                        .eq(profile_id.to_owned())
                        .or(dsl::profile_id.is_null()),
                ))
            .and(dsl::transaction_type.eq(common_enums::BlocklistTransactionType::Payout)),
        )
        .await
    }
}
