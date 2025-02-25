use error_stack::{report, ResultExt};
use async_bb8_diesel::AsyncRunQueryDsl;
use diesel::{associations::HasTable, BoolExpressionMethods, ExpressionMethods, QueryDsl};

use common_utils::errors::CustomResult;
use diesel_models::{dispute as storage, errors as diesel_errors, query::generics::db_metrics, schema::dispute::dsl};
use hyperswitch_domain_models::disputes as domain;
use router_env::{instrument, logger, tracing};
use sample::dispute::DisputeInterface;

use crate::{connection, errors, DatabaseStore, RouterStore};

#[async_trait::async_trait]
impl<T: DatabaseStore> DisputeInterface for RouterStore<T> {
    type Error = errors::StorageError;

    #[instrument(skip_all)]
    async fn insert_dispute(
        &self,
        dispute: storage::DisputeNew,
    ) -> CustomResult<storage::Dispute, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        dispute
            .insert(&conn)
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))
    }

    #[instrument(skip_all)]
    async fn find_by_merchant_id_payment_id_connector_dispute_id(
        &self,
        merchant_id: &common_utils::id_type::MerchantId,
        payment_id: &common_utils::id_type::PaymentId,
        connector_dispute_id: &str,
    ) -> CustomResult<Option<storage::Dispute>, errors::StorageError> {
        let conn = connection::pg_connection_read(self).await?;
        storage::Dispute::find_by_merchant_id_payment_id_connector_dispute_id(
            &conn,
            merchant_id,
            payment_id,
            connector_dispute_id,
        )
        .await
        .map_err(|error| report!(errors::StorageError::from(error)))
    }

    #[instrument(skip_all)]
    async fn find_dispute_by_merchant_id_dispute_id(
        &self,
        merchant_id: &common_utils::id_type::MerchantId,
        dispute_id: &str,
    ) -> CustomResult<storage::Dispute, errors::StorageError> {
        let conn = connection::pg_connection_read(self).await?;
        storage::Dispute::find_by_merchant_id_dispute_id(&conn, merchant_id, dispute_id)
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))
    }

    #[instrument(skip_all)]
    async fn find_disputes_by_constraints(
        &self,
        merchant_id: &common_utils::id_type::MerchantId,
        dispute_constraints: &domain::DisputeListConstraints,
    ) -> CustomResult<Vec<storage::Dispute>, errors::StorageError> {
        let conn = connection::pg_connection_read(self).await?;
        storage::Dispute::filter_by_constraints(&conn, merchant_id, dispute_constraints)
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))
    }

    #[instrument(skip_all)]
    async fn find_disputes_by_merchant_id_payment_id(
        &self,
        merchant_id: &common_utils::id_type::MerchantId,
        payment_id: &common_utils::id_type::PaymentId,
    ) -> CustomResult<Vec<storage::Dispute>, errors::StorageError> {
        let conn = connection::pg_connection_read(self).await?;
        storage::Dispute::find_by_merchant_id_payment_id(&conn, merchant_id, payment_id)
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))
    }

    #[instrument(skip_all)]
    async fn update_dispute(
        &self,
        this: storage::Dispute,
        dispute: storage::DisputeUpdate,
    ) -> CustomResult<storage::Dispute, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        this.update(&conn, dispute)
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))
    }

    #[instrument(skip_all)]
    async fn get_dispute_status_with_count(
        &self,
        merchant_id: &common_utils::id_type::MerchantId,
        profile_id_list: Option<Vec<common_utils::id_type::ProfileId>>,
        time_range: &common_utils::types::TimeRange,
    ) -> CustomResult<Vec<(common_enums::DisputeStatus, i64)>, errors::StorageError> {
        let conn = connection::pg_connection_read(self).await?;
        storage::Dispute::get_dispute_status_with_count(
            &conn,
            merchant_id,
            profile_id_list,
            time_range,
        )
        .await
        .map_err(|error| report!(errors::StorageError::from(error)))
    }
}

#[async_trait::async_trait]
pub trait DisputeDbExt: Sized {
    async fn filter_by_constraints(
        conn: &connection::PgPooledConn,
        merchant_id: &common_utils::id_type::MerchantId,
        dispute_list_constraints: &domain::DisputeListConstraints,
    ) -> CustomResult<Vec<Self>, diesel_errors::DatabaseError>;

    async fn get_dispute_status_with_count(
        conn: &connection::PgPooledConn,
        merchant_id: &common_utils::id_type::MerchantId,
        profile_id_list: Option<Vec<common_utils::id_type::ProfileId>>,
        time_range: &common_utils::types::TimeRange,
    ) -> CustomResult<Vec<(common_enums::enums::DisputeStatus, i64)>, diesel_errors::DatabaseError>;
}

#[async_trait::async_trait]
impl DisputeDbExt for storage::Dispute {
    async fn filter_by_constraints(
        conn: &connection::PgPooledConn,
        merchant_id: &common_utils::id_type::MerchantId,
        dispute_list_constraints: &domain::DisputeListConstraints,
    ) -> CustomResult<Vec<Self>, diesel_errors::DatabaseError> {
        let mut filter = <Self as HasTable>::table()
            .filter(dsl::merchant_id.eq(merchant_id.to_owned()))
            .order(dsl::modified_at.desc())
            .into_boxed();

        let mut search_by_payment_or_dispute_id = false;

        if let (Some(payment_id), Some(dispute_id)) = (
            &dispute_list_constraints.payment_id,
            &dispute_list_constraints.dispute_id,
        ) {
            search_by_payment_or_dispute_id = true;
            filter = filter.filter(
                dsl::payment_id
                    .eq(payment_id.to_owned())
                    .or(dsl::dispute_id.eq(dispute_id.to_owned())),
            );
        };

        if !search_by_payment_or_dispute_id {
            if let Some(payment_id) = &dispute_list_constraints.payment_id {
                filter = filter.filter(dsl::payment_id.eq(payment_id.to_owned()));
            };
        }
        if !search_by_payment_or_dispute_id {
            if let Some(dispute_id) = &dispute_list_constraints.dispute_id {
                filter = filter.filter(dsl::dispute_id.eq(dispute_id.clone()));
            };
        }

        if let Some(time_range) = dispute_list_constraints.time_range {
            filter = filter.filter(dsl::created_at.ge(time_range.start_time));

            if let Some(end_time) = time_range.end_time {
                filter = filter.filter(dsl::created_at.le(end_time));
            }
        }

        if let Some(profile_id) = &dispute_list_constraints.profile_id {
            filter = filter.filter(dsl::profile_id.eq_any(profile_id.clone()));
        }
        if let Some(connector_list) = &dispute_list_constraints.connector {
            filter = filter.filter(dsl::connector.eq_any(connector_list.clone()));
        }

        if let Some(reason) = &dispute_list_constraints.reason {
            filter = filter.filter(dsl::connector_reason.eq(reason.clone()));
        }
        if let Some(dispute_stage) = &dispute_list_constraints.dispute_stage {
            filter = filter.filter(dsl::dispute_stage.eq_any(dispute_stage.clone()));
        }
        if let Some(dispute_status) = &dispute_list_constraints.dispute_status {
            filter = filter.filter(dsl::dispute_status.eq_any(dispute_status.clone()));
        }
        if let Some(currency_list) = &dispute_list_constraints.currency {
            filter = filter.filter(dsl::dispute_currency.eq_any(currency_list.clone()));
        }
        if let Some(merchant_connector_id) = &dispute_list_constraints.merchant_connector_id {
            filter = filter.filter(dsl::merchant_connector_id.eq(merchant_connector_id.clone()))
        }
        if let Some(limit) = dispute_list_constraints.limit {
            filter = filter.limit(limit.into());
        }
        if let Some(offset) = dispute_list_constraints.offset {
            filter = filter.offset(offset.into());
        }

        logger::debug!(query = %diesel::debug_query::<diesel::pg::Pg, _>(&filter).to_string());

        db_metrics::track_database_call::<<Self as HasTable>::Table, _, _>(
            filter.get_results_async(conn),
            db_metrics::DatabaseOperation::Filter,
        )
        .await
        .change_context(diesel_errors::DatabaseError::NotFound)
        .attach_printable_lazy(|| "Error filtering records by predicate")
    }

    async fn get_dispute_status_with_count(
        conn: &connection::PgPooledConn,
        merchant_id: &common_utils::id_type::MerchantId,
        profile_id_list: Option<Vec<common_utils::id_type::ProfileId>>,
        time_range: &common_utils::types::TimeRange,
    ) -> CustomResult<Vec<(common_enums::DisputeStatus, i64)>, diesel_errors::DatabaseError> {
        let mut query = <Self as HasTable>::table()
            .group_by(dsl::dispute_status)
            .select((dsl::dispute_status, diesel::dsl::count_star()))
            .filter(dsl::merchant_id.eq(merchant_id.to_owned()))
            .into_boxed();

        if let Some(profile_id) = profile_id_list {
            query = query.filter(dsl::profile_id.eq_any(profile_id));
        }

        query = query.filter(dsl::created_at.ge(time_range.start_time));

        query = match time_range.end_time {
            Some(ending_at) => query.filter(dsl::created_at.le(ending_at)),
            None => query,
        };

        logger::debug!(query = %diesel::debug_query::<diesel::pg::Pg,_>(&query).to_string());

        db_metrics::track_database_call::<<Self as HasTable>::Table, _, _>(
            query.get_results_async::<(common_enums::DisputeStatus, i64)>(conn),
            db_metrics::DatabaseOperation::Count,
        )
        .await
        .change_context(diesel_errors::DatabaseError::NotFound)
        .attach_printable_lazy(|| "Error filtering records by predicate")
    }
}
