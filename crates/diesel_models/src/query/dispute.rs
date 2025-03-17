use async_bb8_diesel::AsyncRunQueryDsl;
use diesel::{associations::HasTable, BoolExpressionMethods, ExpressionMethods, QueryDsl, Table};
use error_stack::ResultExt;
use router_env::logger;

use super::generics;
use crate::{
    dispute::{Dispute, DisputeNew, DisputeUpdate, DisputeUpdateInternal},
    errors,
    schema::dispute::dsl,
    PgPooledConn, StorageResult,
};

pub struct DisputeListConstraints {
    pub dispute_id: Option<String>,
    pub payment_id: Option<common_utils::id_type::PaymentId>,
    pub limit: Option<u32>,
    pub offset: Option<u32>,
    pub profile_id: Option<Vec<common_utils::id_type::ProfileId>>,
    pub dispute_status: Option<Vec<common_enums::DisputeStatus>>,
    pub dispute_stage: Option<Vec<common_enums::DisputeStage>>,
    pub reason: Option<String>,
    pub connector: Option<Vec<String>>,
    pub merchant_connector_id: Option<common_utils::id_type::MerchantConnectorAccountId>,
    pub currency: Option<Vec<common_enums::Currency>>,
    pub time_range: Option<common_utils::types::TimeRange>,
}

impl DisputeNew {
    pub async fn insert(self, conn: &PgPooledConn) -> StorageResult<Dispute> {
        generics::generic_insert(conn, self).await
    }
}

impl Dispute {
    pub async fn find_by_merchant_id_payment_id_connector_dispute_id(
        conn: &PgPooledConn,
        merchant_id: &common_utils::id_type::MerchantId,
        payment_id: &common_utils::id_type::PaymentId,
        connector_dispute_id: &str,
    ) -> StorageResult<Option<Self>> {
        generics::generic_find_one_optional::<<Self as HasTable>::Table, _, _>(
            conn,
            dsl::merchant_id
                .eq(merchant_id.to_owned())
                .and(dsl::payment_id.eq(payment_id.to_owned()))
                .and(dsl::connector_dispute_id.eq(connector_dispute_id.to_owned())),
        )
        .await
    }

    pub async fn find_by_merchant_id_dispute_id(
        conn: &PgPooledConn,
        merchant_id: &common_utils::id_type::MerchantId,
        dispute_id: &str,
    ) -> StorageResult<Self> {
        generics::generic_find_one::<<Self as HasTable>::Table, _, _>(
            conn,
            dsl::merchant_id
                .eq(merchant_id.to_owned())
                .and(dsl::dispute_id.eq(dispute_id.to_owned())),
        )
        .await
    }

    pub async fn find_by_merchant_id_payment_id(
        conn: &PgPooledConn,
        merchant_id: &common_utils::id_type::MerchantId,
        payment_id: &common_utils::id_type::PaymentId,
    ) -> StorageResult<Vec<Self>> {
        generics::generic_filter::<
            <Self as HasTable>::Table,
            _,
            <<Self as HasTable>::Table as Table>::PrimaryKey,
            _,
        >(
            conn,
            dsl::merchant_id
                .eq(merchant_id.to_owned())
                .and(dsl::payment_id.eq(payment_id.to_owned())),
            None,
            None,
            None,
        )
        .await
    }

    pub async fn update(self, conn: &PgPooledConn, dispute: DisputeUpdate) -> StorageResult<Self> {
        match generics::generic_update_with_unique_predicate_get_result::<
            <Self as HasTable>::Table,
            _,
            _,
            _,
        >(
            conn,
            dsl::dispute_id.eq(self.dispute_id.to_owned()),
            DisputeUpdateInternal::from(dispute),
        )
        .await
        {
            Err(error) => match error.current_context() {
                errors::DatabaseError::NoFieldsToUpdate => Ok(self),
                _ => Err(error),
            },
            result => result,
        }
    }

    pub async fn filter_by_constraints(
        conn: &PgPooledConn,
        merchant_id: &common_utils::id_type::MerchantId,
        dispute_list_constraints: &DisputeListConstraints,
    ) -> StorageResult<Vec<Self>> {
        let mut filter = <Self as HasTable>::table()
            .filter(dsl::merchant_id.eq(merchant_id.to_owned()))
            .order(dsl::modified_at.desc())
            .into_boxed();

        let mut search_by_payment_or_dispute_id = false;

        if let (Some(payment_id), Some(dispute_id)) = (
            &dispute_list_constraints.payment_id,
            &dispute_list_constraints.dispute_id,
        ) {
            search_by_payment_or_dispute_id = true;
            filter = filter.filter(
                dsl::payment_id
                    .eq(payment_id.to_owned())
                    .or(dsl::dispute_id.eq(dispute_id.to_owned())),
            );
        };

        if !search_by_payment_or_dispute_id {
            if let Some(payment_id) = &dispute_list_constraints.payment_id {
                filter = filter.filter(dsl::payment_id.eq(payment_id.to_owned()));
            };
        }
        if !search_by_payment_or_dispute_id {
            if let Some(dispute_id) = &dispute_list_constraints.dispute_id {
                filter = filter.filter(dsl::dispute_id.eq(dispute_id.clone()));
            };
        }

        if let Some(time_range) = dispute_list_constraints.time_range {
            filter = filter.filter(dsl::created_at.ge(time_range.start_time));

            if let Some(end_time) = time_range.end_time {
                filter = filter.filter(dsl::created_at.le(end_time));
            }
        }

        if let Some(profile_id) = &dispute_list_constraints.profile_id {
            filter = filter.filter(dsl::profile_id.eq_any(profile_id.clone()));
        }
        if let Some(connector_list) = &dispute_list_constraints.connector {
            filter = filter.filter(dsl::connector.eq_any(connector_list.clone()));
        }

        if let Some(reason) = &dispute_list_constraints.reason {
            filter = filter.filter(dsl::connector_reason.eq(reason.clone()));
        }
        if let Some(dispute_stage) = &dispute_list_constraints.dispute_stage {
            filter = filter.filter(dsl::dispute_stage.eq_any(dispute_stage.clone()));
        }
        if let Some(dispute_status) = &dispute_list_constraints.dispute_status {
            filter = filter.filter(dsl::dispute_status.eq_any(dispute_status.clone()));
        }
        if let Some(currency_list) = &dispute_list_constraints.currency {
            filter = filter.filter(dsl::dispute_currency.eq_any(currency_list.clone()));
        }
        if let Some(merchant_connector_id) = &dispute_list_constraints.merchant_connector_id {
            filter = filter.filter(dsl::merchant_connector_id.eq(merchant_connector_id.clone()))
        }
        if let Some(limit) = dispute_list_constraints.limit {
            filter = filter.limit(limit.into());
        }
        if let Some(offset) = dispute_list_constraints.offset {
            filter = filter.offset(offset.into());
        }

        logger::debug!(query = %diesel::debug_query::<diesel::pg::Pg, _>(&filter).to_string());

        generics::db_metrics::track_database_call::<<Self as HasTable>::Table, _, _>(
            filter.get_results_async(conn),
            generics::db_metrics::DatabaseOperation::Filter,
        )
        .await
        .change_context(errors::DatabaseError::NotFound)
        .attach_printable_lazy(|| "Error filtering records by predicate")
    }

    pub async fn get_dispute_status_with_count(
        conn: &PgPooledConn,
        merchant_id: &common_utils::id_type::MerchantId,
        profile_id_list: Option<Vec<common_utils::id_type::ProfileId>>,
        time_range: &common_utils::types::TimeRange,
    ) -> StorageResult<Vec<(common_enums::DisputeStatus, i64)>> {
        let mut query = <Self as HasTable>::table()
            .group_by(dsl::dispute_status)
            .select((dsl::dispute_status, diesel::dsl::count_star()))
            .filter(dsl::merchant_id.eq(merchant_id.to_owned()))
            .into_boxed();

        if let Some(profile_id) = profile_id_list {
            query = query.filter(dsl::profile_id.eq_any(profile_id));
        }

        query = query.filter(dsl::created_at.ge(time_range.start_time));

        query = match time_range.end_time {
            Some(ending_at) => query.filter(dsl::created_at.le(ending_at)),
            None => query,
        };

        logger::debug!(query = %diesel::debug_query::<diesel::pg::Pg,_>(&query).to_string());

        generics::db_metrics::track_database_call::<<Self as HasTable>::Table, _, _>(
            query.get_results_async::<(common_enums::DisputeStatus, i64)>(conn),
            generics::db_metrics::DatabaseOperation::Count,
        )
        .await
        .change_context(errors::DatabaseError::NotFound)
        .attach_printable_lazy(|| "Error filtering records by predicate")
    }
}
