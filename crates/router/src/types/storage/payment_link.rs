use async_bb8_diesel::AsyncRunQueryDsl;
use diesel::{associations::HasTable, ExpressionMethods, QueryDsl};
pub use diesel_models::{
    payment_link::{PaymentLink, PaymentLinkNew},
    schema::payment_link::dsl,
};
use error_stack::{IntoReport, ResultExt};

use crate::{
    connection::PgPooledConn,
    core::errors::{self, CustomResult},
    logger,
};
#[async_trait::async_trait]

pub trait PaymentLinkDbExt: Sized {
    async fn filter_by_constraints(
        conn: &PgPooledConn,
        merchant_id: &str,
        payment_link_list_constraints: api_models::payments::PaymentLinkListConstraints,
    ) -> CustomResult<Vec<Self>, errors::DatabaseError>;
}

#[async_trait::async_trait]
impl PaymentLinkDbExt for PaymentLink {
        /// Asynchronously filters payment links based on the specified constraints for a given merchant ID.
    async fn filter_by_constraints(
        conn: &PgPooledConn,
        merchant_id: &str,
        payment_link_list_constraints: api_models::payments::PaymentLinkListConstraints,
    ) -> CustomResult<Vec<Self>, errors::DatabaseError> {
        let mut filter = <Self as HasTable>::table()
            .filter(dsl::merchant_id.eq(merchant_id.to_owned()))
            .order(dsl::created_at.desc())
            .into_boxed();

        if let Some(created_time) = payment_link_list_constraints.created {
            filter = filter.filter(dsl::created_at.eq(created_time));
        }
        if let Some(created_time_lt) = payment_link_list_constraints.created_lt {
            filter = filter.filter(dsl::created_at.lt(created_time_lt));
        }
        if let Some(created_time_gt) = payment_link_list_constraints.created_gt {
            filter = filter.filter(dsl::created_at.gt(created_time_gt));
        }
        if let Some(created_time_lte) = payment_link_list_constraints.created_lte {
            filter = filter.filter(dsl::created_at.le(created_time_lte));
        }
        if let Some(created_time_gte) = payment_link_list_constraints.created_gte {
            filter = filter.filter(dsl::created_at.ge(created_time_gte));
        }
        if let Some(limit) = payment_link_list_constraints.limit {
            filter = filter.limit(limit);
        }

        logger::debug!(query = %diesel::debug_query::<diesel::pg::Pg, _>(&filter).to_string());

        filter
            .get_results_async(conn)
            .await
            .into_report()
            // The query built here returns an empty Vec when no records are found, and if any error does occur,
            // it would be an internal database error, due to which we are raising a DatabaseError::Unknown error
            .change_context(errors::DatabaseError::Others)
            .attach_printable("Error filtering payment link by specified constraints")
    }
}
