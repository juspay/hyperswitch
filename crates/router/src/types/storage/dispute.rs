use async_bb8_diesel::AsyncRunQueryDsl;
use common_utils::errors::CustomResult;
use diesel::{associations::HasTable, ExpressionMethods, QueryDsl};
pub use diesel_models::dispute::{Dispute, DisputeNew, DisputeUpdate};
use diesel_models::{errors, query::generics::db_metrics, schema::dispute::dsl};
use error_stack::ResultExt;

use crate::{connection::PgPooledConn, logger};

#[async_trait::async_trait]
pub trait DisputeDbExt: Sized {
    async fn filter_by_constraints(
        conn: &PgPooledConn,
        merchant_id: &common_utils::id_type::MerchantId,
        dispute_list_constraints: api_models::disputes::DisputeListConstraints,
    ) -> CustomResult<Vec<Self>, errors::DatabaseError>;
}

#[async_trait::async_trait]
impl DisputeDbExt for Dispute {
    async fn filter_by_constraints(
        conn: &PgPooledConn,
        merchant_id: &common_utils::id_type::MerchantId,
        dispute_list_constraints: api_models::disputes::DisputeListConstraints,
    ) -> CustomResult<Vec<Self>, errors::DatabaseError> {
        let mut filter = <Self as HasTable>::table()
            .filter(dsl::merchant_id.eq(merchant_id.to_owned()))
            .order(dsl::modified_at.desc())
            .into_boxed();

        if let Some(profile_id) = dispute_list_constraints.profile_id {
            filter = filter.filter(dsl::profile_id.eq(profile_id));
        }
        if let Some(connector) = dispute_list_constraints.connector {
            filter = filter.filter(dsl::connector.eq_any(connector));
        }
        if let Some(reason) = dispute_list_constraints.reason {
            filter = filter.filter(dsl::connector_reason.eq(reason));
        }
        if let Some(dispute_stage) = dispute_list_constraints.dispute_stage {
            filter = filter.filter(dsl::dispute_stage.eq_any(dispute_stage));
        }
        if let Some(dispute_status) = dispute_list_constraints.dispute_status {
            filter = filter.filter(dsl::dispute_status.eq_any(dispute_status));
        }
        if let Some(limit) = dispute_list_constraints.limit {
            filter = filter.limit(limit.into());
        }

        logger::debug!(query = %diesel::debug_query::<diesel::pg::Pg, _>(&filter).to_string());

        db_metrics::track_database_call::<<Self as HasTable>::Table, _, _>(
            filter.get_results_async(conn),
            db_metrics::DatabaseOperation::Filter,
        )
        .await
        .change_context(errors::DatabaseError::NotFound)
        .attach_printable_lazy(|| "Error filtering records by predicate")
    }
}
