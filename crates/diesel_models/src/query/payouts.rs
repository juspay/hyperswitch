use async_bb8_diesel::AsyncRunQueryDsl;
use diesel::{
    associations::HasTable, debug_query, pg::Pg, BoolExpressionMethods, ExpressionMethods,
    JoinOnDsl, QueryDsl,
};
use error_stack::{report, ResultExt};

use super::generics;
use crate::{
    enums, errors,
    payouts::{Payouts, PayoutsNew, PayoutsUpdate, PayoutsUpdateInternal},
    query::generics::db_metrics,
    schema::{payout_attempt, payouts::dsl},
    PgPooledConn, StorageResult,
};

impl PayoutsNew {
    pub async fn insert(self, conn: &PgPooledConn) -> StorageResult<Payouts> {
        generics::generic_insert(conn, self).await
    }
}
impl Payouts {
    pub async fn update(
        self,
        conn: &PgPooledConn,
        payout_update: PayoutsUpdate,
    ) -> StorageResult<Self> {
        match generics::generic_update_with_results::<<Self as HasTable>::Table, _, _, _>(
            conn,
            dsl::payout_id
                .eq(self.payout_id.to_owned())
                .and(dsl::merchant_id.eq(self.merchant_id.to_owned())),
            PayoutsUpdateInternal::from(payout_update),
        )
        .await
        {
            Err(error) => match error.current_context() {
                errors::DatabaseError::NoFieldsToUpdate => Ok(self),
                _ => Err(error),
            },
            Ok(mut payouts) => payouts
                .pop()
                .ok_or(error_stack::report!(errors::DatabaseError::NotFound)),
        }
    }

    pub async fn find_by_merchant_id_payout_id(
        conn: &PgPooledConn,
        merchant_id: &common_utils::id_type::MerchantId,
        payout_id: &common_utils::id_type::PayoutId,
    ) -> StorageResult<Self> {
        generics::generic_find_one::<<Self as HasTable>::Table, _, _>(
            conn,
            dsl::merchant_id
                .eq(merchant_id.to_owned())
                .and(dsl::payout_id.eq(payout_id.to_owned())),
        )
        .await
    }

    pub async fn update_by_merchant_id_payout_id(
        conn: &PgPooledConn,
        merchant_id: &common_utils::id_type::MerchantId,
        payout_id: &common_utils::id_type::PayoutId,
        payout: PayoutsUpdate,
    ) -> StorageResult<Self> {
        generics::generic_update_with_results::<<Self as HasTable>::Table, _, _, _>(
            conn,
            dsl::merchant_id
                .eq(merchant_id.to_owned())
                .and(dsl::payout_id.eq(payout_id.to_owned())),
            PayoutsUpdateInternal::from(payout),
        )
        .await?
        .first()
        .cloned()
        .ok_or_else(|| {
            report!(errors::DatabaseError::NotFound).attach_printable("Error while updating payout")
        })
    }

    pub async fn find_optional_by_merchant_id_payout_id(
        conn: &PgPooledConn,
        merchant_id: &common_utils::id_type::MerchantId,
        payout_id: &common_utils::id_type::PayoutId,
    ) -> StorageResult<Option<Self>> {
        generics::generic_find_one_optional::<<Self as HasTable>::Table, _, _>(
            conn,
            dsl::merchant_id
                .eq(merchant_id.to_owned())
                .and(dsl::payout_id.eq(payout_id.to_owned())),
        )
        .await
    }

    pub async fn get_total_count_of_payouts(
        conn: &PgPooledConn,
        merchant_id: &common_utils::id_type::MerchantId,
        active_payout_ids: &[common_utils::id_type::PayoutId],
        connector: Option<Vec<String>>,
        currency: Option<Vec<enums::Currency>>,
        status: Option<Vec<enums::PayoutStatus>>,
        payout_type: Option<Vec<enums::PayoutType>>,
    ) -> StorageResult<i64> {
        let mut filter = <Self as HasTable>::table()
            .inner_join(payout_attempt::table.on(payout_attempt::dsl::payout_id.eq(dsl::payout_id)))
            .count()
            .filter(dsl::merchant_id.eq(merchant_id.to_owned()))
            .filter(dsl::payout_id.eq_any(active_payout_ids.to_vec()))
            .into_boxed();

        if let Some(connector) = connector {
            filter = filter.filter(payout_attempt::dsl::connector.eq_any(connector));
        }
        if let Some(currency) = currency {
            filter = filter.filter(dsl::destination_currency.eq_any(currency));
        }
        if let Some(status) = status {
            filter = filter.filter(dsl::status.eq_any(status));
        }
        if let Some(payout_type) = payout_type {
            filter = filter.filter(dsl::payout_type.eq_any(payout_type));
        }
        router_env::logger::debug!(query = %debug_query::<Pg, _>(&filter).to_string());

        db_metrics::track_database_call::<<Self as HasTable>::Table, _, _>(
            filter.get_result_async::<i64>(conn),
            db_metrics::DatabaseOperation::Filter,
        )
        .await
        .change_context(errors::DatabaseError::Others)
        .attach_printable("Error filtering count of payouts")
    }
}
