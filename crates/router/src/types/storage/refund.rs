use async_bb8_diesel::AsyncRunQueryDsl;
use common_utils::errors::CustomResult;
use diesel::{associations::HasTable, ExpressionMethods, QueryDsl};
use error_stack::{IntoReport, ResultExt};
pub use storage_models::refund::{
    Refund, RefundCoreWorkflow, RefundNew, RefundUpdate, RefundUpdateInternal,
};
use storage_models::{errors, metrics::database_metric, schema::refund::dsl};

use crate::{connection::PgPooledConn, logger};

#[cfg(feature = "kv_store")]
impl crate::utils::storage_partitioning::KvStorePartition for Refund {}

#[async_trait::async_trait]
pub trait RefundDbExt: Sized {
    async fn filter_by_constraints(
        conn: &PgPooledConn,
        merchant_id: &str,
        refund_list_details: &api_models::refunds::RefundListRequest,
        limit: i64,
    ) -> CustomResult<Vec<Self>, errors::DatabaseError>;
}

#[async_trait::async_trait]
impl RefundDbExt for Refund {
    async fn filter_by_constraints(
        conn: &PgPooledConn,
        merchant_id: &str,
        refund_list_details: &api_models::refunds::RefundListRequest,
        limit: i64,
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
                filter = filter.limit(limit);
            }
        };

        if let Some(created) = refund_list_details.created {
            filter = filter.filter(dsl::created_at.eq(created));
        }
        if let Some(created_lt) = refund_list_details.created_lt {
            filter = filter.filter(dsl::created_at.lt(created_lt));
        }
        if let Some(created_gt) = refund_list_details.created_gt {
            filter = filter.filter(dsl::created_at.gt(created_gt));
        }
        if let Some(created_lte) = refund_list_details.created_lte {
            filter = filter.filter(dsl::created_at.le(created_lte));
        }
        if let Some(created_gte) = refund_list_details.created_gte {
            filter = filter.filter(dsl::created_at.gt(created_gte));
        }

        logger::debug!(query = %diesel::debug_query::<diesel::pg::Pg, _>(&filter).to_string());

        let table_name = std::any::type_name::<Self>();

        database_metric::time_database_call(
            database_metric::DatabaseCallType::Read,
            || filter.get_results_async(conn),
            table_name,
        )
        .await
        .into_report()
        .change_context(errors::DatabaseError::NotFound)
        .attach_printable_lazy(|| "Error filtering records by predicate")
    }
}
