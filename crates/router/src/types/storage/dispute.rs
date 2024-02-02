use async_bb8_diesel::AsyncRunQueryDsl;
use common_utils::errors::CustomResult;
use diesel::{associations::HasTable, ExpressionMethods, QueryDsl};
pub use diesel_models::dispute::{Dispute, DisputeNew, DisputeUpdate};
use diesel_models::{errors, query::generics::db_metrics, schema::dispute::dsl};
use error_stack::{IntoReport, ResultExt};

use crate::{connection::PgPooledConn, logger};

#[async_trait::async_trait]
pub trait DisputeDbExt: Sized {
    async fn filter_by_constraints(
        conn: &PgPooledConn,
        merchant_id: &str,
        dispute_list_constraints: api_models::disputes::DisputeListConstraints,
    ) -> CustomResult<Vec<Self>, errors::DatabaseError>;
}

#[async_trait::async_trait]
impl DisputeDbExt for Dispute {
        /// Asynchronously filters dispute records based on the provided constraints and returns the filtered results.
    /// 
    /// # Arguments
    /// - `conn`: A reference to a pooled database connection
    /// - `merchant_id`: A string reference representing the merchant ID
    /// - `dispute_list_constraints`: An instance of `DisputeListConstraints` containing the constraints for filtering
    /// 
    /// # Returns
    /// A `CustomResult` containing a vector of filtered dispute records or a `DatabaseError` if an error occurs during the database operation
    /// 
    async fn filter_by_constraints(
        conn: &PgPooledConn,
        merchant_id: &str,
        dispute_list_constraints: api_models::disputes::DisputeListConstraints,
    ) -> CustomResult<Vec<Self>, errors::DatabaseError> {
        let mut filter = <Self as HasTable>::table()
            .filter(dsl::merchant_id.eq(merchant_id.to_owned()))
            .order(dsl::modified_at.desc())
            .into_boxed();

        if let Some(profile_id) = dispute_list_constraints.profile_id {
            filter = filter.filter(dsl::profile_id.eq(profile_id));
        }
        if let Some(received_time) = dispute_list_constraints.received_time {
            filter = filter.filter(dsl::created_at.eq(received_time));
        }
        if let Some(received_time_lt) = dispute_list_constraints.received_time_lt {
            filter = filter.filter(dsl::created_at.lt(received_time_lt));
        }
        if let Some(received_time_gt) = dispute_list_constraints.received_time_gt {
            filter = filter.filter(dsl::created_at.gt(received_time_gt));
        }
        if let Some(received_time_lte) = dispute_list_constraints.received_time_lte {
            filter = filter.filter(dsl::created_at.le(received_time_lte));
        }
        if let Some(received_time_gte) = dispute_list_constraints.received_time_gte {
            filter = filter.filter(dsl::created_at.ge(received_time_gte));
        }
        if let Some(connector) = dispute_list_constraints.connector {
            filter = filter.filter(dsl::connector.eq(connector));
        }
        if let Some(reason) = dispute_list_constraints.reason {
            filter = filter.filter(dsl::connector_reason.eq(reason));
        }
        if let Some(dispute_stage) = dispute_list_constraints.dispute_stage {
            filter = filter.filter(dsl::dispute_stage.eq(dispute_stage));
        }
        if let Some(dispute_status) = dispute_list_constraints.dispute_status {
            filter = filter.filter(dsl::dispute_status.eq(dispute_status));
        }
        if let Some(limit) = dispute_list_constraints.limit {
            filter = filter.limit(limit);
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
}
