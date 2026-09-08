use async_bb8_diesel::AsyncRunQueryDsl;
use diesel::{associations::HasTable, BoolExpressionMethods, ExpressionMethods, QueryDsl};
pub use diesel_models::{
    payment_link::{PaymentLink, PaymentLinkNew},
    query::generics::db_metrics,
    schema::payment_link::dsl,
};
use error_stack::ResultExt;

use crate::{
    connection::DatabaseConnectionWithContext,
    core::errors::{self, CustomResult},
    logger,
};

#[async_trait::async_trait]
pub trait PaymentLinkDbExt: Sized {
    async fn filter_by_constraints(
        conn: &DatabaseConnectionWithContext<'_>,
        processor_merchant_id: &common_utils::id_type::MerchantId,
        payment_link_list_constraints: &api_models::payments::PaymentLinkListConstraints,
        profile_id: Option<common_utils::id_type::ProfileId>,
    ) -> CustomResult<Vec<Self>, errors::DatabaseError>;

    async fn get_total_count_of_payment_links(
        conn: &PgPooledConn,
        processor_merchant_id: &common_utils::id_type::MerchantId,
        payment_link_list_constraints: &api_models::payments::PaymentLinkListConstraints,
        profile_id: Option<common_utils::id_type::ProfileId>,
    ) -> CustomResult<i64, errors::DatabaseError>;
}

#[async_trait::async_trait]
impl PaymentLinkDbExt for PaymentLink {
    async fn filter_by_constraints(
        conn: &DatabaseConnectionWithContext<'_>,
        processor_merchant_id: &common_utils::id_type::MerchantId,
        payment_link_list_constraints: &api_models::payments::PaymentLinkListConstraints,
        profile_id: Option<common_utils::id_type::ProfileId>,
    ) -> CustomResult<Vec<Self>, errors::DatabaseError> {
        let mut filter = diesel_models::list::into_boxed_list(
            <Self as HasTable>::table()
                .filter(
                    dsl::processor_merchant_id
                        .eq(processor_merchant_id.to_owned())
                        .or(dsl::processor_merchant_id
                            .is_null()
                            .and(dsl::merchant_id.eq(processor_merchant_id.to_owned()))),
                )
                .order(dsl::created_at.desc()),
        );

        filter = match profile_id {
            Some(pid) => filter.filter(dsl::profile_id.eq(pid)),
            None => filter,
        };

        filter = match payment_link_list_constraints.time_range {
            Some(tr) => {
                let f = filter.filter(dsl::created_at.ge(tr.start_time));
                match tr.end_time {
                    Some(end_time) => f.filter(dsl::created_at.le(end_time)),
                    None => f,
                }
            }
            None => filter,
        };

        let filter = diesel_models::list::apply_pagination(
            filter,
            payment_link_list_constraints.limit,
            payment_link_list_constraints.offset,
        );

        logger::debug!(query = %diesel::debug_query::<diesel::pg::Pg, _>(&filter).to_string());

        db_metrics::track_database_call::<<Self as HasTable>::Table, _, _>(
            filter.get_results_async(conn.raw_connection()),
            db_metrics::DatabaseOperation::Filter,
        )
        .await
        .change_context(errors::DatabaseError::Others)
        .attach_printable("Error filtering payment link by specified constraints")
    }

    async fn get_total_count_of_payment_links(
        conn: &PgPooledConn,
        processor_merchant_id: &common_utils::id_type::MerchantId,
        payment_link_list_constraints: &api_models::payments::PaymentLinkListConstraints,
        profile_id: Option<common_utils::id_type::ProfileId>,
    ) -> CustomResult<i64, errors::DatabaseError> {
        let mut filter = diesel_models::list::into_boxed_list(
            <Self as HasTable>::table().count().filter(
                dsl::processor_merchant_id
                    .eq(processor_merchant_id.to_owned())
                    .or(dsl::processor_merchant_id
                        .is_null()
                        .and(dsl::merchant_id.eq(processor_merchant_id.to_owned()))),
            ),
        );

        filter = match profile_id {
            Some(pid) => filter.filter(dsl::profile_id.eq(pid)),
            None => filter,
        };

        filter = match payment_link_list_constraints.time_range {
            Some(tr) => {
                let f = filter.filter(dsl::created_at.ge(tr.start_time));
                match tr.end_time {
                    Some(end_time) => f.filter(dsl::created_at.le(end_time)),
                    None => f,
                }
            }
            None => filter,
        };

        db_metrics::track_database_call::<<Self as HasTable>::Table, _, _>(
            filter.get_result_async(conn.raw_connection()),
            db_metrics::DatabaseOperation::Count,
        )
        .await
        .change_context(errors::DatabaseError::Others)
        .attach_printable("Error counting payment links by specified constraints")
    }
}
