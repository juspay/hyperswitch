use async_bb8_diesel::AsyncRunQueryDsl;
use common_utils::errors::CustomResult;
use diesel::{associations::HasTable, ExpressionMethods, QueryDsl};
pub use diesel_models::mandate::{
    Mandate, MandateNew, MandateUpdate, MandateUpdateInternal, SingleUseMandate,
};
use diesel_models::{errors, schema::mandate::dsl};
use error_stack::ResultExt;

use crate::{connection::PgPooledConn, logger};

#[async_trait::async_trait]
pub trait MandateDbExt: Sized {
    async fn filter_by_constraints(
        conn: &PgPooledConn,
        merchant_id: &str,
        mandate_list_constraints: api_models::mandates::MandateListConstraints,
    ) -> CustomResult<Vec<Self>, errors::DatabaseError>;
}

#[async_trait::async_trait]
impl MandateDbExt for Mandate {
    async fn filter_by_constraints(
        conn: &PgPooledConn,
        merchant_id: &str,
        mandate_list_constraints: api_models::mandates::MandateListConstraints,
    ) -> CustomResult<Vec<Self>, errors::DatabaseError> {
        let mut filter = <Self as HasTable>::table()
            .filter(dsl::merchant_id.eq(merchant_id.to_owned()))
            .order(dsl::created_at.desc())
            .into_boxed();

        if let Some(created_time) = mandate_list_constraints.created_time {
            filter = filter.filter(dsl::created_at.eq(created_time));
        }
        if let Some(created_time_lt) = mandate_list_constraints.created_time_lt {
            filter = filter.filter(dsl::created_at.lt(created_time_lt));
        }
        if let Some(created_time_gt) = mandate_list_constraints.created_time_gt {
            filter = filter.filter(dsl::created_at.gt(created_time_gt));
        }
        if let Some(created_time_lte) = mandate_list_constraints.created_time_lte {
            filter = filter.filter(dsl::created_at.le(created_time_lte));
        }
        if let Some(created_time_gte) = mandate_list_constraints.created_time_gte {
            filter = filter.filter(dsl::created_at.ge(created_time_gte));
        }
        if let Some(connector) = mandate_list_constraints.connector {
            filter = filter.filter(dsl::connector.eq(connector));
        }
        if let Some(mandate_status) = mandate_list_constraints.mandate_status {
            filter = filter.filter(dsl::mandate_status.eq(mandate_status));
        }
        if let Some(limit) = mandate_list_constraints.limit {
            filter = filter.limit(limit);
        }
        if let Some(offset) = mandate_list_constraints.offset {
            filter = filter.offset(offset);
        }

        logger::debug!(query = %diesel::debug_query::<diesel::pg::Pg, _>(&filter).to_string());

        filter
            .get_results_async(conn)
            .await
            // The query built here returns an empty Vec when no records are found, and if any error does occur,
            // it would be an internal database error, due to which we are raising a DatabaseError::Unknown error
            .change_context(errors::DatabaseError::Others)
            .attach_printable("Error filtering mandates by specified constraints")
    }
}
