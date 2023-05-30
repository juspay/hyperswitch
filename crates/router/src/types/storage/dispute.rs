use async_bb8_diesel::AsyncRunQueryDsl;
use common_utils::errors::CustomResult;
use diesel::{associations::HasTable, ExpressionMethods, QueryDsl};
use error_stack::{IntoReport, ResultExt};
pub use storage_models::dispute::{Dispute, DisputeNew, DisputeUpdate};
use storage_models::{errors, metrics::database_metric, schema::dispute::dsl};

use crate::{connection::PgPooledConn, logger, types::transformers::ForeignInto};

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
    async fn filter_by_constraints(
        conn: &PgPooledConn,
        merchant_id: &str,
        dispute_list_constraints: api_models::disputes::DisputeListConstraints,
    ) -> CustomResult<Vec<Self>, errors::DatabaseError> {
        let mut filter = <Self as HasTable>::table()
            .filter(dsl::merchant_id.eq(merchant_id.to_owned()))
            .order(dsl::modified_at.desc())
            .into_boxed();

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
            let storage_dispute_stage: storage_models::enums::DisputeStage =
                dispute_stage.foreign_into();
            filter = filter.filter(dsl::dispute_stage.eq(storage_dispute_stage));
        }
        if let Some(dispute_status) = dispute_list_constraints.dispute_status {
            let storage_dispute_status: storage_models::enums::DisputeStatus =
                dispute_status.foreign_into();
            filter = filter.filter(dsl::dispute_status.eq(storage_dispute_status));
        }
        if let Some(limit) = dispute_list_constraints.limit {
            filter = filter.limit(limit);
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
