#[cfg(feature = "v1")]
use common_utils::id_type;
#[cfg(feature = "v1")]
use diesel_models::{PaymentAttempt, PaymentIntent};
#[cfg(feature = "v1")]
use error_stack::report;
#[cfg(feature = "v1")]
use router_env::{instrument, tracing};
#[cfg(feature = "v1")]
use time::PrimitiveDateTime;

use super::MockDb;
#[cfg(feature = "v1")]
use crate::connection;
use crate::{core::errors, services::Store};

/// Storage access for building analytics reports through the process tracker.
#[async_trait::async_trait]
pub trait PaymentReportInterface {
    /// Fetches payment attempts created within the given time range along with the payment
    /// intent they are the active attempt of, scoped to the given organization and the
    /// optional merchant and profile filters.
    #[cfg(feature = "v1")]
    async fn find_payment_report_rows(
        &self,
        organization_id: &id_type::OrganizationId,
        merchant_ids: Option<Vec<id_type::MerchantId>>,
        profile_ids: Option<Vec<id_type::ProfileId>>,
        time_lower_limit: PrimitiveDateTime,
        time_upper_limit: PrimitiveDateTime,
        limit: i64,
    ) -> errors::CustomResult<Vec<(PaymentAttempt, Option<PaymentIntent>)>, errors::StorageError>;
}

#[async_trait::async_trait]
impl PaymentReportInterface for Store {
    #[cfg(feature = "v1")]
    #[instrument(skip_all)]
    async fn find_payment_report_rows(
        &self,
        organization_id: &id_type::OrganizationId,
        merchant_ids: Option<Vec<id_type::MerchantId>>,
        profile_ids: Option<Vec<id_type::ProfileId>>,
        time_lower_limit: PrimitiveDateTime,
        time_upper_limit: PrimitiveDateTime,
        limit: i64,
    ) -> errors::CustomResult<Vec<(PaymentAttempt, Option<PaymentIntent>)>, errors::StorageError>
    {
        let conn = connection::pg_connection_read(self).await?;
        PaymentAttempt::find_for_payment_report(
            &conn,
            organization_id,
            merchant_ids,
            profile_ids,
            time_lower_limit,
            time_upper_limit,
            limit,
        )
        .await
        .map_err(|error| report!(errors::StorageError::from(error)))
    }
}

#[async_trait::async_trait]
impl PaymentReportInterface for MockDb {
    #[cfg(feature = "v1")]
    async fn find_payment_report_rows(
        &self,
        _organization_id: &id_type::OrganizationId,
        _merchant_ids: Option<Vec<id_type::MerchantId>>,
        _profile_ids: Option<Vec<id_type::ProfileId>>,
        _time_lower_limit: PrimitiveDateTime,
        _time_upper_limit: PrimitiveDateTime,
        _limit: i64,
    ) -> errors::CustomResult<Vec<(PaymentAttempt, Option<PaymentIntent>)>, errors::StorageError>
    {
        Err(errors::StorageError::MockDbError)?
    }
}
