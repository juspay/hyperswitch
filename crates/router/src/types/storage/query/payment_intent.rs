use async_bb8_diesel::AsyncRunQueryDsl;
use diesel::{associations::HasTable, BoolExpressionMethods, ExpressionMethods, QueryDsl};
use error_stack::{IntoReport, ResultExt};
use router_env::tracing::{self, instrument};

#[cfg(not(feature = "kv_store"))]
use super::generics::{self, ExecuteQuery};
#[cfg(feature = "kv_store")]
use super::generics::{self, RawQuery, RawSqlQuery};
use crate::{
    connection::PgPooledConn,
    core::errors::{self, CustomResult},
    schema::payment_intent::dsl,
    types::{
        api,
        storage::{
            PaymentIntent, PaymentIntentNew, PaymentIntentUpdate, PaymentIntentUpdateInternal,
        },
    },
};

impl PaymentIntentNew {
    #[cfg(not(feature = "kv_store"))]
    #[instrument(skip(conn))]
    pub async fn insert_diesel(
        self,
        conn: &PgPooledConn,
    ) -> CustomResult<PaymentIntent, errors::StorageError> {
        generics::generic_insert::<_, _, PaymentIntent, _>(conn, self, ExecuteQuery::new()).await
    }

    #[cfg(feature = "kv_store")]
    #[instrument(skip(conn))]
    pub async fn insert_diesel(
        self,
        conn: &PgPooledConn,
    ) -> CustomResult<RawSqlQuery, errors::StorageError> {
        generics::generic_insert::<_, _, PaymentIntent, _>(conn, self, RawQuery).await
    }
}

impl PaymentIntent {
    #[cfg(not(feature = "kv_store"))]
    #[instrument(skip(conn))]
    pub async fn update(
        self,
        conn: &PgPooledConn,
        payment_intent: PaymentIntentUpdate,
    ) -> CustomResult<Self, errors::StorageError> {
        match generics::generic_update_by_id::<<Self as HasTable>::Table, _, _, Self, _>(
            conn,
            self.id,
            PaymentIntentUpdateInternal::from(payment_intent),
            ExecuteQuery::new(),
        )
        .await
        {
            Err(error) => match error.current_context() {
                errors::StorageError::DatabaseError(errors::DatabaseError::NoFieldsToUpdate) => {
                    Ok(self)
                }
                _ => Err(error),
            },
            result => result,
        }
    }

    #[cfg(feature = "kv_store")]
    #[instrument(skip(conn))]
    pub async fn update(
        self,
        conn: &PgPooledConn,
        payment_intent: PaymentIntentUpdate,
    ) -> CustomResult<RawSqlQuery, errors::StorageError> {
        generics::generic_update_by_id::<<Self as HasTable>::Table, _, _, Self, _>(
            conn,
            self.id,
            PaymentIntentUpdateInternal::from(payment_intent),
            RawQuery,
        )
        .await
    }

    #[instrument(skip(conn))]
    pub async fn find_by_payment_id_merchant_id(
        conn: &PgPooledConn,
        payment_id: &str,
        merchant_id: &str,
    ) -> CustomResult<Self, errors::StorageError> {
        generics::generic_find_one::<<Self as HasTable>::Table, _, _>(
            conn,
            dsl::merchant_id
                .eq(merchant_id.to_owned())
                .and(dsl::payment_id.eq(payment_id.to_owned())),
        )
        .await
    }

    #[instrument(skip(conn))]
    pub async fn find_optional_by_payment_id_merchant_id(
        conn: &PgPooledConn,
        payment_id: &str,
        merchant_id: &str,
    ) -> CustomResult<Option<Self>, errors::StorageError> {
        generics::generic_find_one_optional::<<Self as HasTable>::Table, _, _>(
            conn,
            dsl::merchant_id
                .eq(merchant_id.to_owned())
                .and(dsl::payment_id.eq(payment_id.to_owned())),
        )
        .await
    }

    #[instrument(skip(conn))]
    pub async fn filter_by_constraints(
        conn: &PgPooledConn,
        merchant_id: &str,
        pc: &api::PaymentListConstraints,
    ) -> CustomResult<Vec<Self>, errors::StorageError> {
        let customer_id = &pc.customer_id;
        let starting_after = &pc.starting_after;
        let ending_before = &pc.ending_before;

        //TODO: Replace this with Boxable Expression and pass it into generic filter
        // when https://github.com/rust-lang/rust/issues/52662 becomes stable

        let mut filter = <Self as HasTable>::table()
            .filter(dsl::merchant_id.eq(merchant_id.to_owned()))
            .order_by(dsl::id)
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
            .change_context(errors::StorageError::DatabaseError(
                errors::DatabaseError::NotFound,
            ))
            .attach_printable_lazy(|| "Error filtering records by predicate")
    }
}

#[cfg(feature = "kv_store")]
impl crate::utils::storage_partitioning::KvStorePartition for PaymentIntent {}
