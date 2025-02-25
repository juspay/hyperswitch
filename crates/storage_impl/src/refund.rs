use async_bb8_diesel::AsyncRunQueryDsl;
// use diesel::{associations::HasTable, BoolExpressionMethods, ExpressionMethods, QueryDsl};
use diesel::{associations::HasTable, ExpressionMethods, QueryDsl};
use error_stack::{report, ResultExt};

// #[cfg(not(feature = "kv_store"))]
// mod storage {

// use hyperswitch_domain_models::refunds as domain;
use common_utils::errors::CustomResult;
use router_env::{instrument, logger, tracing};
use diesel_models::{refund as storage, errors as diesel_errors, enums, query::generics::db_metrics, schema::refund::dsl};
use sample::refund::RefundInterface;

use crate::{connection, errors, DatabaseStore, RouterStore};
use crate::redis::kv_store::KvStorePartition;

impl KvStorePartition for storage::Refund {}

#[async_trait::async_trait]
impl<T: DatabaseStore> RefundInterface for RouterStore<T> {
    type Error = errors::StorageError;

    #[instrument(skip_all)]
    async fn find_refund_by_internal_reference_id_merchant_id(
        &self,
        internal_reference_id: &str,
        merchant_id: &common_utils::id_type::MerchantId,
        _storage_scheme: enums::MerchantStorageScheme,
    ) -> CustomResult<storage::Refund, errors::StorageError> {
        let conn = connection::pg_connection_read(self).await?;
        storage::Refund::find_by_internal_reference_id_merchant_id(
            &conn,
            internal_reference_id,
            merchant_id,
        )
        .await
        .map_err(|error| report!(errors::StorageError::from(error)))
    }

    #[instrument(skip_all)]
    async fn insert_refund(
        &self,
        new: storage::RefundNew,
        _storage_scheme: enums::MerchantStorageScheme,
    ) -> CustomResult<storage::Refund, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        new.insert(&conn)
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))
    }

    #[instrument(skip_all)]
    async fn find_refund_by_merchant_id_connector_transaction_id(
        &self,
        merchant_id: &common_utils::id_type::MerchantId,
        connector_transaction_id: &str,
        _storage_scheme: enums::MerchantStorageScheme,
    ) -> CustomResult<Vec<storage::Refund>, errors::StorageError> {
        let conn = connection::pg_connection_read(self).await?;
        storage::Refund::find_by_merchant_id_connector_transaction_id(
            &conn,
            merchant_id,
            connector_transaction_id,
        )
        .await
        .map_err(|error| report!(errors::StorageError::from(error)))
    }

    #[instrument(skip_all)]
    async fn update_refund(
        &self,
        this: storage::Refund,
        refund: storage::RefundUpdate,
        _storage_scheme: enums::MerchantStorageScheme,
    ) -> CustomResult<storage::Refund, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        this.update(&conn, refund)
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))
    }

    #[instrument(skip_all)]
    async fn find_refund_by_merchant_id_refund_id(
        &self,
        merchant_id: &common_utils::id_type::MerchantId,
        refund_id: &str,
        _storage_scheme: enums::MerchantStorageScheme,
    ) -> CustomResult<storage::Refund, errors::StorageError> {
        let conn = connection::pg_connection_read(self).await?;
        storage::Refund::find_by_merchant_id_refund_id(&conn, merchant_id, refund_id)
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))
    }

    #[instrument(skip_all)]
    async fn find_refund_by_merchant_id_connector_refund_id_connector(
        &self,
        merchant_id: &common_utils::id_type::MerchantId,
        connector_refund_id: &str,
        connector: &str,
        _storage_scheme: enums::MerchantStorageScheme,
    ) -> CustomResult<storage::Refund, errors::StorageError> {
        let conn = connection::pg_connection_read(self).await?;
        storage::Refund::find_by_merchant_id_connector_refund_id_connector(
            &conn,
            merchant_id,
            connector_refund_id,
            connector,
        )
        .await
        .map_err(|error| report!(errors::StorageError::from(error)))
    }

    #[instrument(skip_all)]
    async fn find_refund_by_payment_id_merchant_id(
        &self,
        payment_id: &common_utils::id_type::PaymentId,
        merchant_id: &common_utils::id_type::MerchantId,
        _storage_scheme: enums::MerchantStorageScheme,
    ) -> CustomResult<Vec<storage::Refund>, errors::StorageError> {
        let conn = connection::pg_connection_read(self).await?;
        storage::Refund::find_by_payment_id_merchant_id(&conn, payment_id, merchant_id)
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))
    }

    // TODO(jarnura): use of RefundDbExt
    // #[cfg(feature = "olap")]
    // #[instrument(skip_all)]
    // async fn filter_refund_by_constraints(
    //     &self,
    //     merchant_id: &common_utils::id_type::MerchantId,
    //     refund_details: &refunds::RefundListConstraints,
    //     _storage_scheme: enums::MerchantStorageScheme,
    //     limit: i64,
    //     offset: i64,
    // ) -> CustomResult<Vec<diesel_models::refund::Refund>, errors::StorageError> {
    //     let conn = connection::pg_connection_read(self).await?;
    //     <diesel_models::refund::Refund as storage::RefundDbExt>::filter_by_constraints(
    //         &conn,
    //         merchant_id,
    //         refund_details,
    //         limit,
    //         offset,
    //     )
    //     .await
    //     .map_err(|error| report!(errors::StorageError::from(error)))
    // }

    // #[cfg(feature = "olap")]
    // #[instrument(skip_all)]
    // async fn filter_refund_by_meta_constraints(
    //     &self,
    //     merchant_id: &common_utils::id_type::MerchantId,
    //     refund_details: &api_models::payments::TimeRange,
    //     _storage_scheme: enums::MerchantStorageScheme,
    // ) -> CustomResult<api_models::refunds::RefundListMetaData, errors::StorageError> {
    //     let conn = connection::pg_connection_read(self).await?;
    //     <diesel_models::refund::Refund as storage::RefundDbExt>::filter_by_meta_constraints(
    //         &conn,
    //         merchant_id,
    //         refund_details,
    //     )
    //     .await
    //     .map_err(|error|report!(errors::StorageError::from(error)))
    // }

    // TODO(jarnura): use of RefundDbExt
    // #[cfg(feature = "olap")]
    // #[instrument(skip_all)]
    // async fn get_refund_status_with_count(
    //     &self,
    //     merchant_id: &common_utils::id_type::MerchantId,
    //     profile_id_list: Option<Vec<common_utils::id_type::ProfileId>>,
    //     time_range: &common_utils::types::TimeRange,
    //     _storage_scheme: enums::MerchantStorageScheme,
    // ) -> CustomResult<Vec<(common_enums::enums::RefundStatus, i64)>, errors::StorageError> {
    //     let conn = connection::pg_connection_read(self).await?;
    //     <diesel_models::refund::Refund as storage::RefundDbExt>::get_refund_status_with_count(&conn, merchant_id,profile_id_list, time_range)
    //     .await
    //     .map_err(|error|report!(errors::StorageError::from(error)))
    // }

    // TODO(jarnura): use of RefundDbExt
    // #[cfg(feature = "olap")]
    // #[instrument(skip_all)]
    // async fn get_total_count_of_refunds(
    //     &self,
    //     merchant_id: &common_utils::id_type::MerchantId,
    //     refund_details: &refunds::RefundListConstraints,
    //     _storage_scheme: enums::MerchantStorageScheme,
    // ) -> CustomResult<i64, errors::StorageError> {
    //     let conn = connection::pg_connection_read(self).await?;
    //     <diesel_models::refund::Refund as storage::RefundDbExt>::get_refunds_count(
    //         &conn,
    //         merchant_id,
    //         refund_details,
    //     )
    //     .await
    //     .map_err(|error| report!(errors::StorageError::from(error)))
    // }
}
// }





#[async_trait::async_trait]
pub trait RefundDbExt: Sized {
    // TODO(jarnura): use of api models
    // async fn filter_by_constraints(
    //     conn: &connection::PgPooledConn,
    //     merchant_id: &common_utils::id_type::MerchantId,
    //     refund_list_details: &domain::RefundListConstraints,
    //     limit: i64,
    //     offset: i64,
    // ) -> CustomResult<Vec<Self>, diesel_errors::DatabaseError>;

    // TODO(jarnura): use of api models
    // async fn filter_by_meta_constraints(
    //     conn: &connection::PgPooledConn,
    //     merchant_id: &common_utils::id_type::MerchantId,
    //     refund_list_details: &common_utils::types::TimeRange,
    // ) -> CustomResult<api_models::refunds::RefundListMetaData, diesel_errors::DatabaseError>;

    // TODO(jarnura): use of api models
    // async fn get_refunds_count(
    //     conn: &connection::PgPooledConn,
    //     merchant_id: &common_utils::id_type::MerchantId,
    //     refund_list_details: &domain::RefundListConstraints,
    // ) -> CustomResult<i64, diesel_errors::DatabaseError>;

    async fn get_refund_status_with_count(
        conn: &connection::PgPooledConn,
        merchant_id: &common_utils::id_type::MerchantId,
        profile_id_list: Option<Vec<common_utils::id_type::ProfileId>>,
        time_range: &common_utils::types::TimeRange,
    ) -> CustomResult<Vec<(enums::enums::RefundStatus, i64)>, diesel_errors::DatabaseError>;
}

#[async_trait::async_trait]
impl RefundDbExt for storage::Refund {
    // TODO(jarnura): use of api models
    // async fn filter_by_constraints(
    //     conn: &connection::PgPooledConn,
    //     merchant_id: &common_utils::id_type::MerchantId,
    //     refund_list_details: &refunds::RefundListConstraints,
    //     limit: i64,
    //     offset: i64,
    // ) -> CustomResult<Vec<Self>, diesel_errors::DatabaseError> {
    //     let mut filter = <Self as HasTable>::table()
    //         .filter(dsl::merchant_id.eq(merchant_id.to_owned()))
    //         .order(dsl::modified_at.desc())
    //         .into_boxed();
    //     let mut search_by_pay_or_ref_id = false;

    //     if let (Some(pid), Some(ref_id)) = (
    //         &refund_list_details.payment_id,
    //         &refund_list_details.refund_id,
    //     ) {
    //         search_by_pay_or_ref_id = true;
    //         filter = filter
    //             .filter(
    //                 dsl::payment_id
    //                     .eq(pid.to_owned())
    //                     .or(dsl::refund_id.eq(ref_id.to_owned())),
    //             )
    //             .limit(limit)
    //             .offset(offset);
    //     };

    //     if !search_by_pay_or_ref_id {
    //         match &refund_list_details.payment_id {
    //             Some(pid) => {
    //                 filter = filter.filter(dsl::payment_id.eq(pid.to_owned()));
    //             }
    //             None => {
    //                 filter = filter.limit(limit).offset(offset);
    //             }
    //         };
    //     }
    //     if !search_by_pay_or_ref_id {
    //         match &refund_list_details.refund_id {
    //             Some(ref_id) => {
    //                 filter = filter.filter(dsl::refund_id.eq(ref_id.to_owned()));
    //             }
    //             None => {
    //                 filter = filter.limit(limit).offset(offset);
    //             }
    //         };
    //     }
    //     match &refund_list_details.profile_id {
    //         Some(profile_id) => {
    //             filter = filter
    //                 .filter(dsl::profile_id.eq_any(profile_id.to_owned()))
    //                 .limit(limit)
    //                 .offset(offset);
    //         }
    //         None => {
    //             filter = filter.limit(limit).offset(offset);
    //         }
    //     };

    //     if let Some(time_range) = refund_list_details.time_range {
    //         filter = filter.filter(dsl::created_at.ge(time_range.start_time));

    //         if let Some(end_time) = time_range.end_time {
    //             filter = filter.filter(dsl::created_at.le(end_time));
    //         }
    //     }

    //     filter = match refund_list_details.amount_filter {
    //         Some(AmountFilter {
    //             start_amount: Some(start),
    //             end_amount: Some(end),
    //         }) => filter.filter(dsl::refund_amount.between(start, end)),
    //         Some(AmountFilter {
    //             start_amount: Some(start),
    //             end_amount: None,
    //         }) => filter.filter(dsl::refund_amount.ge(start)),
    //         Some(AmountFilter {
    //             start_amount: None,
    //             end_amount: Some(end),
    //         }) => filter.filter(dsl::refund_amount.le(end)),
    //         _ => filter,
    //     };

    //     if let Some(connector) = refund_list_details.connector.clone() {
    //         filter = filter.filter(dsl::connector.eq_any(connector));
    //     }

    //     if let Some(merchant_connector_id) = refund_list_details.merchant_connector_id.clone() {
    //         filter = filter.filter(dsl::merchant_connector_id.eq_any(merchant_connector_id));
    //     }

    //     if let Some(filter_currency) = &refund_list_details.currency {
    //         filter = filter.filter(dsl::currency.eq_any(filter_currency.clone()));
    //     }

    //     if let Some(filter_refund_status) = &refund_list_details.refund_status {
    //         filter = filter.filter(dsl::refund_status.eq_any(filter_refund_status.clone()));
    //     }

    //     logger::debug!(query = %diesel::debug_query::<diesel::pg::Pg, _>(&filter).to_string());

    //     db_metrics::track_database_call::<<Self as HasTable>::Table, _, _>(
    //         filter.get_results_async(conn),
    //         db_metrics::DatabaseOperation::Filter,
    //     )
    //     .await
    //     .change_context(diesel_errors::DatabaseError::NotFound)
    //     .attach_printable_lazy(|| "Error filtering records by predicate")
    // }

    // async fn filter_by_meta_constraints(
    //     conn: &connection::PgPooledConn,
    //     merchant_id: &common_utils::id_type::MerchantId,
    //     refund_list_details: &common_utils::types::TimeRange,
    // ) -> CustomResult<api_models::refunds::RefundListMetaData, diesel_errors::DatabaseError> {
    //     let start_time = refund_list_details.start_time;

    //     let end_time = refund_list_details
    //         .end_time
    //         .unwrap_or_else(common_utils::date_time::now);

    //     let filter = <Self as HasTable>::table()
    //         .filter(dsl::merchant_id.eq(merchant_id.to_owned()))
    //         .order(dsl::modified_at.desc())
    //         .filter(dsl::created_at.ge(start_time))
    //         .filter(dsl::created_at.le(end_time));

    //     let filter_connector: Vec<String> = filter
    //         .clone()
    //         .select(dsl::connector)
    //         .distinct()
    //         .order_by(dsl::connector.asc())
    //         .get_results_async(conn)
    //         .await
    //         .change_context(diesel_errors::DatabaseError::Others)
    //         .attach_printable("Error filtering records by connector")?;

    //     let filter_currency: Vec<Currency> = filter
    //         .clone()
    //         .select(dsl::currency)
    //         .distinct()
    //         .order_by(dsl::currency.asc())
    //         .get_results_async(conn)
    //         .await
    //         .change_context(diesel_errors::DatabaseError::Others)
    //         .attach_printable("Error filtering records by currency")?;

    //     let filter_status: Vec<enums::RefundStatus> = filter
    //         .select(dsl::refund_status)
    //         .distinct()
    //         .order_by(dsl::refund_status.asc())
    //         .get_results_async(conn)
    //         .await
    //         .change_context(diesel_errors::DatabaseError::Others)
    //         .attach_printable("Error filtering records by refund status")?;

    //     let meta = api_models::refunds::RefundListMetaData {
    //         connector: filter_connector,
    //         currency: filter_currency,
    //         refund_status: filter_status,
    //     };

    //     Ok(meta)
    // }

    // TODO(jarnura): use of api models
    // async fn get_refunds_count(
    //     conn: &connection::PgPooledConn,
    //     merchant_id: &common_utils::id_type::MerchantId,
    //     refund_list_details: &refunds::RefundListConstraints,
    // ) -> CustomResult<i64, diesel_errors::DatabaseError> {
    //     let mut filter = <Self as HasTable>::table()
    //         .count()
    //         .filter(dsl::merchant_id.eq(merchant_id.to_owned()))
    //         .into_boxed();

    //     let mut search_by_pay_or_ref_id = false;

    //     if let (Some(pid), Some(ref_id)) = (
    //         &refund_list_details.payment_id,
    //         &refund_list_details.refund_id,
    //     ) {
    //         search_by_pay_or_ref_id = true;
    //         filter = filter.filter(
    //             dsl::payment_id
    //                 .eq(pid.to_owned())
    //                 .or(dsl::refund_id.eq(ref_id.to_owned())),
    //         );
    //     };

    //     if !search_by_pay_or_ref_id {
    //         if let Some(pay_id) = &refund_list_details.payment_id {
    //             filter = filter.filter(dsl::payment_id.eq(pay_id.to_owned()));
    //         }
    //     }

    //     if !search_by_pay_or_ref_id {
    //         if let Some(ref_id) = &refund_list_details.refund_id {
    //             filter = filter.filter(dsl::refund_id.eq(ref_id.to_owned()));
    //         }
    //     }
    //     if let Some(profile_id) = &refund_list_details.profile_id {
    //         filter = filter.filter(dsl::profile_id.eq_any(profile_id.to_owned()));
    //     }

    //     if let Some(time_range) = refund_list_details.time_range {
    //         filter = filter.filter(dsl::created_at.ge(time_range.start_time));

    //         if let Some(end_time) = time_range.end_time {
    //             filter = filter.filter(dsl::created_at.le(end_time));
    //         }
    //     }

    //     filter = match refund_list_details.amount_filter {
    //         Some(AmountFilter {
    //             start_amount: Some(start),
    //             end_amount: Some(end),
    //         }) => filter.filter(dsl::refund_amount.between(start, end)),
    //         Some(AmountFilter {
    //             start_amount: Some(start),
    //             end_amount: None,
    //         }) => filter.filter(dsl::refund_amount.ge(start)),
    //         Some(AmountFilter {
    //             start_amount: None,
    //             end_amount: Some(end),
    //         }) => filter.filter(dsl::refund_amount.le(end)),
    //         _ => filter,
    //     };

    //     if let Some(connector) = refund_list_details.connector.clone() {
    //         filter = filter.filter(dsl::connector.eq_any(connector));
    //     }

    //     if let Some(merchant_connector_id) = refund_list_details.merchant_connector_id.clone() {
    //         filter = filter.filter(dsl::merchant_connector_id.eq_any(merchant_connector_id))
    //     }

    //     if let Some(filter_currency) = &refund_list_details.currency {
    //         filter = filter.filter(dsl::currency.eq_any(filter_currency.clone()));
    //     }

    //     if let Some(filter_refund_status) = &refund_list_details.refund_status {
    //         filter = filter.filter(dsl::refund_status.eq_any(filter_refund_status.clone()));
    //     }

    //     logger::debug!(query = %diesel::debug_query::<diesel::pg::Pg, _>(&filter).to_string());

    //     filter
    //         .get_result_async::<i64>(conn)
    //         .await
    //         .change_context(diesel_errors::DatabaseError::NotFound)
    //         .attach_printable_lazy(|| "Error filtering count of refunds")
    // }

    async fn get_refund_status_with_count(
        conn: &connection::PgPooledConn,
        merchant_id: &common_utils::id_type::MerchantId,
        profile_id_list: Option<Vec<common_utils::id_type::ProfileId>>,
        time_range: &common_utils::types::TimeRange,
    ) -> CustomResult<Vec<(enums::RefundStatus, i64)>, diesel_errors::DatabaseError> {
        let mut query = <Self as HasTable>::table()
            .group_by(dsl::refund_status)
            .select((dsl::refund_status, diesel::dsl::count_star()))
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

        logger::debug!(filter = %diesel::debug_query::<diesel::pg::Pg,_>(&query).to_string());

        db_metrics::track_database_call::<<Self as HasTable>::Table, _, _>(
            query.get_results_async::<(enums::RefundStatus, i64)>(conn),
            db_metrics::DatabaseOperation::Count,
        )
        .await
        .change_context(diesel_errors::DatabaseError::NotFound)
        .attach_printable_lazy(|| "Error filtering status count of refunds")
    }
}

