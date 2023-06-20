use async_bb8_diesel::AsyncRunQueryDsl;
use diesel::{associations::HasTable, ExpressionMethods, QueryDsl};
use error_stack::{IntoReport, ResultExt};
use router_env::{instrument, tracing};
pub use storage_models::{
    errors,
    payment_intent::{
        PaymentIntent, PaymentIntentNew, PaymentIntentUpdate, PaymentIntentUpdateInternal,
    },
    schema::payment_intent::dsl,
};

use crate::{connection::PgPooledConn, core::errors::CustomResult, types::api};

#[async_trait::async_trait]
pub trait PaymentIntentDbExt: Sized {
    async fn filter_by_constraints(
        conn: &PgPooledConn,
        merchant_id: &str,
        pc: &api::PaymentListConstraints,
    ) -> CustomResult<Vec<Self>, errors::DatabaseError>;
}

#[async_trait::async_trait]
impl PaymentIntentDbExt for PaymentIntent {
    #[instrument(skip(conn))]
    async fn filter_by_constraints(
        conn: &PgPooledConn,
        merchant_id: &str,
        pc: &api::PaymentListConstraints,
    ) -> CustomResult<Vec<Self>, errors::DatabaseError> {
        let customer_id = &pc.customer_id;
        let starting_after = &pc.starting_after;
        let ending_before = &pc.ending_before;

        //[#350]: Replace this with Boxable Expression and pass it into generic filter
        // when https://github.com/rust-lang/rust/issues/52662 becomes stable
        let mut filter = <Self as HasTable>::table()
            .filter(dsl::merchant_id.eq(merchant_id.to_owned()))
            .order(dsl::modified_at.desc())
            .into_boxed();

        if let Some(customer_id) = customer_id {
            filter = filter.filter(dsl::customer_id.eq(customer_id.to_owned()));
        }
        if let Some(created) = pc.created {
            filter = filter.filter(dsl::created_at.eq(created));
        }
        if let Some(created_lt) = pc.created_lt {
            filter = filter.filter(dsl::created_at.lt(created_lt));
        }
        if let Some(created_gt) = pc.created_gt {
            filter = filter.filter(dsl::created_at.gt(created_gt));
        }
        if let Some(created_lte) = pc.created_lte {
            filter = filter.filter(dsl::created_at.le(created_lte));
        }
        if let Some(created_gte) = pc.created_gte {
            filter = filter.filter(dsl::created_at.gt(created_gte));
        }
        if let Some(starting_after) = starting_after {
            let id = Self::find_by_payment_id_merchant_id(conn, starting_after, merchant_id)
                .await?
                .id;
            filter = filter.filter(dsl::id.gt(id));
        }
        if let Some(ending_before) = ending_before {
            let id = Self::find_by_payment_id_merchant_id(conn, ending_before, merchant_id)
                .await?
                .id;
            filter = filter.filter(dsl::id.lt(id.to_owned()));
        }

        filter = filter.limit(pc.limit);

        crate::logger::debug!(query = %diesel::debug_query::<diesel::pg::Pg, _>(&filter).to_string());

        filter
            .get_results_async(conn)
            .await
            .into_report()
            .change_context(errors::DatabaseError::NotFound)
            .attach_printable_lazy(|| "Error filtering records by predicate")
    }
}
