use async_bb8_diesel::AsyncRunQueryDsl;
use common_utils::errors::CustomResult;
use diesel::{associations::HasTable, ExpressionMethods, QueryDsl};
pub use diesel_models::refund::{
    Refund, RefundCoreWorkflow, RefundNew, RefundUpdate, RefundUpdateInternal,
};
use diesel_models::{
    enums::{Currency, RefundStatus},
    errors,
    query::generics::db_metrics,
    schema::refund::dsl,
};
use error_stack::{IntoReport, ResultExt};

use crate::{connection::PgPooledConn, logger};

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
        refund_list_details: &api_models::payments::TimeRange,
    ) -> CustomResult<api_models::refunds::RefundListMetaData, errors::DatabaseError>;

    async fn get_refunds_count(
        conn: &PgPooledConn,
        merchant_id: &str,
        refund_list_details: &api_models::refunds::RefundListRequest,
    ) -> CustomResult<i64, errors::DatabaseError>;
}

#[async_trait::async_trait]
impl RefundDbExt for Refund {
        /// Asynchronously filter refund records by various constraints such as merchant ID, payment ID, refund ID, profile ID, time range, connector, currency, and refund status. Returns a vector of filtered refund records or a database error.
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
        let mut search_by_pay_or_ref_id = false;

        if let (Some(pid), Some(ref_id)) = (
            &refund_list_details.payment_id,
            &refund_list_details.refund_id,
        ) {
            search_by_pay_or_ref_id = true;
            filter = filter
                .filter(dsl::payment_id.eq(pid.to_owned()))
                .or_filter(dsl::refund_id.eq(ref_id.to_owned()))
                .limit(limit)
                .offset(offset);
        };

        if !search_by_pay_or_ref_id {
            match &refund_list_details.payment_id {
                Some(pid) => {
                    filter = filter.filter(dsl::payment_id.eq(pid.to_owned()));
                }
                None => {
                    filter = filter.limit(limit).offset(offset);
                }
            };
        }
        if !search_by_pay_or_ref_id {
            match &refund_list_details.refund_id {
                Some(ref_id) => {
                    filter = filter.filter(dsl::refund_id.eq(ref_id.to_owned()));
                }
                None => {
                    filter = filter.limit(limit).offset(offset);
                }
            };
        }
        match &refund_list_details.profile_id {
            Some(profile_id) => {
                filter = filter
                    .filter(dsl::profile_id.eq(profile_id.to_owned()))
                    .limit(limit)
                    .offset(offset);
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

        if let Some(filter_currency) = &refund_list_details.currency {
            filter = filter.filter(dsl::currency.eq_any(filter_currency.clone()));
        }

        if let Some(filter_refund_status) = &refund_list_details.refund_status {
            filter = filter.filter(dsl::refund_status.eq_any(filter_refund_status.clone()));
        }

        logger::debug!(query = %diesel::debug_query::<diesel::pg::Pg, _>(&filter).to_string());

        db_metrics::track_database_call::<<Self as HasTable>::Table, _, _>(
            filter.get_results_async(conn),
            db_metrics::DatabaseOperation::Filter,
        )
        .await
        .into_report()
        .change_context(errors::DatabaseError::NotFound)
        .attach_printable_lazy(|| "Error filtering records by predicate")
    }

        /// Retrieves and filters refund list metadata based on the provided constraints.
    /// 
    /// # Arguments
    /// 
    /// * `conn` - A reference to a pooled PostgreSQL connection.
    /// * `merchant_id` - A string slice representing the merchant ID.
    /// * `refund_list_details` - A reference to the time range details for filtering refunds.
    /// 
    /// # Returns
    /// 
    /// A `CustomResult` containing the filtered refund list metadata or a `DatabaseError` if an error occurs.
    /// 
    async fn filter_by_meta_constraints(
        conn: &PgPooledConn,
        merchant_id: &str,
        refund_list_details: &api_models::payments::TimeRange,
    ) -> CustomResult<api_models::refunds::RefundListMetaData, errors::DatabaseError> {
        let start_time = refund_list_details.start_time;

        let end_time = refund_list_details
            .end_time
            .unwrap_or_else(common_utils::date_time::now);

        let filter = <Self as HasTable>::table()
            .filter(dsl::merchant_id.eq(merchant_id.to_owned()))
            .order(dsl::modified_at.desc())
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
            .attach_printable("Error filtering records by connector")?;

        let filter_currency: Vec<Currency> = filter
            .clone()
            .select(dsl::currency)
            .distinct()
            .order_by(dsl::currency.asc())
            .get_results_async(conn)
            .await
            .into_report()
            .change_context(errors::DatabaseError::Others)
            .attach_printable("Error filtering records by currency")?;

        let filter_status: Vec<RefundStatus> = filter
            .select(dsl::refund_status)
            .distinct()
            .order_by(dsl::refund_status.asc())
            .get_results_async(conn)
            .await
            .into_report()
            .change_context(errors::DatabaseError::Others)
            .attach_printable("Error filtering records by refund status")?;

        let meta = api_models::refunds::RefundListMetaData {
            connector: filter_connector,
            currency: filter_currency,
            refund_status: filter_status,
        };

        Ok(meta)
    }

        /// Asynchronously retrieves the count of refunds based on the provided criteria. 
    /// 
    /// # Arguments
    /// * `conn` - A reference to a pooled Postgres connection
    /// * `merchant_id` - A string slice representing the merchant ID
    /// * `refund_list_details` - A reference to the refund list request details
    /// 
    /// # Returns
    /// A `CustomResult` containing the count of refunds if successful, otherwise a `DatabaseError`
    /// 
    async fn get_refunds_count(
        conn: &PgPooledConn,
        merchant_id: &str,
        refund_list_details: &api_models::refunds::RefundListRequest,
    ) -> CustomResult<i64, errors::DatabaseError> {
        let mut filter = <Self as HasTable>::table()
            .count()
            .filter(dsl::merchant_id.eq(merchant_id.to_owned()))
            .into_boxed();

        let mut search_by_pay_or_ref_id = false;

        if let (Some(pid), Some(ref_id)) = (
            &refund_list_details.payment_id,
            &refund_list_details.refund_id,
        ) {
            search_by_pay_or_ref_id = true;
            filter = filter
                .filter(dsl::payment_id.eq(pid.to_owned()))
                .or_filter(dsl::refund_id.eq(ref_id.to_owned()));
        };

        if !search_by_pay_or_ref_id {
            if let Some(pay_id) = &refund_list_details.payment_id {
                filter = filter.filter(dsl::payment_id.eq(pay_id.to_owned()));
            }
        }

        if !search_by_pay_or_ref_id {
            if let Some(ref_id) = &refund_list_details.refund_id {
                filter = filter.filter(dsl::refund_id.eq(ref_id.to_owned()));
            }
        }
        if let Some(profile_id) = &refund_list_details.profile_id {
            filter = filter.filter(dsl::profile_id.eq(profile_id.to_owned()));
        }

        if let Some(time_range) = refund_list_details.time_range {
            filter = filter.filter(dsl::created_at.ge(time_range.start_time));

            if let Some(end_time) = time_range.end_time {
                filter = filter.filter(dsl::created_at.le(end_time));
            }
        }

        if let Some(connector) = refund_list_details.clone().connector {
            filter = filter.filter(dsl::connector.eq_any(connector));
        }

        if let Some(filter_currency) = &refund_list_details.currency {
            filter = filter.filter(dsl::currency.eq_any(filter_currency.clone()));
        }

        if let Some(filter_refund_status) = &refund_list_details.refund_status {
            filter = filter.filter(dsl::refund_status.eq_any(filter_refund_status.clone()));
        }

        logger::debug!(query = %diesel::debug_query::<diesel::pg::Pg, _>(&filter).to_string());

        filter
            .get_result_async::<i64>(conn)
            .await
            .into_report()
            .change_context(errors::DatabaseError::NotFound)
            .attach_printable_lazy(|| "Error filtering count of refunds")
    }
}
