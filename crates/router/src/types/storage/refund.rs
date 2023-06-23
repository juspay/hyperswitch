use async_bb8_diesel::AsyncRunQueryDsl;
use common_utils::errors::CustomResult;
use diesel::{associations::HasTable, ExpressionMethods, QueryDsl};
use error_stack::{IntoReport, ResultExt};
pub use storage_models::refund::{
    Refund, RefundCoreWorkflow, RefundNew, RefundUpdate, RefundUpdateInternal,
};
use storage_models::{
    enums::{Currency, RefundStatus},
    errors,
    schema::refund::dsl,
};

use crate::{connection::PgPooledConn, logger, types::transformers::ForeignInto};

#[cfg(feature = "kv_store")]
impl crate::utils::storage_partitioning::KvStorePartition for Refund {}

#[async_trait::async_trait]
pub trait RefundDbExt: Sized {
    async fn filter_by_constraints(
        conn: &PgPooledConn,
        merchant_id: &str,
        refund_list_details: &api_models::refunds::RefundListRequest,
        limit: i64,
        offset: i64,
    ) -> CustomResult<Vec<Self>, errors::DatabaseError>;

    async fn filter_by_meta_constraints(
        conn: &PgPooledConn,
        merchant_id: &str,
        refund_list_details: &api_models::refunds::RefundListRequest,
        limit: i64,
        offset: i64,
    ) -> CustomResult<api_models::refunds::RefundListMetaData, errors::DatabaseError>;
}

#[async_trait::async_trait]
impl RefundDbExt for Refund {
    async fn filter_by_constraints(
        conn: &PgPooledConn,
        merchant_id: &str,
        refund_list_details: &api_models::refunds::RefundListRequest,
        limit: i64,
        offset: i64,
    ) -> CustomResult<Vec<Self>, errors::DatabaseError> {
        let mut filter = <Self as HasTable>::table()
            .filter(dsl::merchant_id.eq(merchant_id.to_owned()))
            .order(dsl::modified_at.desc())
            .into_boxed();

        match &refund_list_details.payment_id {
            Some(pid) => {
                filter = filter.filter(dsl::payment_id.eq(pid.to_owned()));
            }
            None => {
                filter = filter.limit(limit).offset(offset);
            }
        };

        if let Some(time_range) = refund_list_details.time_range {
            filter = filter.filter(dsl::created_at.ge(time_range.start_time));

            if let Some(end_time) = time_range.end_time {
                filter = filter.filter(dsl::created_at.le(end_time));
            }
        }

        if let Some(connector) = refund_list_details.clone().connector {
            filter = filter.filter(dsl::connector.eq_any(connector));
        }

        if let Some(filter_currency) = refund_list_details.clone().currency {
            let currency: Vec<Currency> = filter_currency
                .iter()
                .map(|currency| (*currency).foreign_into())
                .collect();
            filter = filter.filter(dsl::currency.eq_any(currency));
        }

        if let Some(filter_refund_status) = refund_list_details.clone().refund_status {
            let refund_status: Vec<RefundStatus> = filter_refund_status
                .iter()
                .map(|refund_status| (*refund_status).foreign_into())
                .collect();
            filter = filter.filter(dsl::refund_status.eq_any(refund_status));
        }

        logger::debug!(query = %diesel::debug_query::<diesel::pg::Pg, _>(&filter).to_string());

        filter
            .get_results_async(conn)
            .await
            .into_report()
            .change_context(errors::DatabaseError::NotFound)
            .attach_printable_lazy(|| "Error filtering records by predicate")
    }

    async fn filter_by_meta_constraints(
        conn: &PgPooledConn,
        merchant_id: &str,
        refund_list_details: &api_models::refunds::RefundListRequest,
        limit: i64,
        offset: i64,
    ) -> CustomResult<api_models::refunds::RefundListMetaData, errors::DatabaseError> {
        let start_time = refund_list_details
            .time_range
            .map(|t| t.start_time)
            .unwrap_or(common_utils::date_time::now() - time::Duration::days(30));

        let end_time = refund_list_details
            .time_range
            .and_then(|t| t.end_time)
            .unwrap_or_else(common_utils::date_time::now);

        let filter = <Self as HasTable>::table()
            .filter(dsl::merchant_id.eq(merchant_id.to_owned()))
            .order(dsl::modified_at.desc())
            .limit(limit)
            .offset(offset)
            .filter(dsl::created_at.ge(start_time))
            .filter(dsl::created_at.le(end_time));

        let filter_connector: Vec<String> = filter
            .clone()
            .select(dsl::connector)
            .distinct()
            .order_by(dsl::connector.asc())
            .get_results_async(conn)
            .await
            .into_report()
            .change_context(errors::DatabaseError::Others)
            .attach_printable_lazy(|| "Error filtering records by connector")?;

        let filter_currency: Vec<Currency> = filter
            .clone()
            .select(dsl::currency)
            .distinct()
            .order_by(dsl::currency.asc())
            .get_results_async(conn)
            .await
            .into_report()
            .change_context(errors::DatabaseError::Others)
            .attach_printable_lazy(|| "Error filtering records by currency")?;

        let filter_status: Vec<RefundStatus> = filter
            .select(dsl::refund_status)
            .distinct()
            .order_by(dsl::refund_status.asc())
            .get_results_async(conn)
            .await
            .into_report()
            .change_context(errors::DatabaseError::Others)
            .attach_printable_lazy(|| "Error filtering records by refund status")?;

        let meta = api_models::refunds::RefundListMetaData {
            connector: filter_connector,
            currency: filter_currency
                .into_iter()
                .map(|curr| curr.foreign_into())
                .collect(),
            status: filter_status
                .into_iter()
                .map(|curr| curr.foreign_into())
                .collect(),
        };

        Ok(meta)
    }
}
