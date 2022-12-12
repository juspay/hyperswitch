mod payment_cancel;
mod payment_capture;
mod payment_confirm;
mod payment_create;
mod payment_method_validate;
mod payment_response;
mod payment_session;
mod payment_start;
mod payment_status;
mod payment_update;
use async_trait::async_trait;
use error_stack::{report, IntoReport, ResultExt};
pub use payment_cancel::PaymentCancel;
pub use payment_capture::PaymentCapture;
pub use payment_confirm::PaymentConfirm;
pub use payment_create::PaymentCreate;
pub use payment_method_validate::PaymentMethodValidate;
pub use payment_response::PaymentResponse;
pub use payment_session::PaymentSession;
pub use payment_start::PaymentStart;
pub use payment_status::PaymentStatus;
pub use payment_update::PaymentUpdate;
use router_env::{instrument, tracing};
use storage::Customer;

use super::{helpers, CustomerDetails, PaymentData};
use crate::{
    core::errors::{self, CustomResult, RouterResult},
    db::StorageInterface,
    pii::Secret,
    routes::AppState,
    scheduler::{metrics, workflows::payment_sync},
    types::{
        self, api,
        storage::{self, enums},
        PaymentsResponseData,
    },
};

pub type BoxedOperation<'a, F, T> = Box<dyn Operation<F, T> + Send + Sync + 'a>;

pub trait Operation<F: Clone, T>: Send + std::fmt::Debug {
    fn to_validate_request(&self) -> RouterResult<&(dyn ValidateRequest<F, T> + Send + Sync)> {
        Err(report!(errors::ApiErrorResponse::InternalServerError))
            .attach_printable_lazy(|| format!("validate request interface not found for {self:?}"))
    }
    fn to_get_tracker(
        &self,
    ) -> RouterResult<&(dyn GetTracker<F, PaymentData<F>, T> + Send + Sync)> {
        Err(report!(errors::ApiErrorResponse::InternalServerError))
            .attach_printable_lazy(|| format!("get tracker interface not found for {self:?}"))
    }
    fn to_domain(&self) -> RouterResult<&dyn Domain<F, T>> {
        Err(report!(errors::ApiErrorResponse::InternalServerError))
            .attach_printable_lazy(|| format!("domain interface not found for {self:?}"))
    }
    fn to_update_tracker(
        &self,
    ) -> RouterResult<&(dyn UpdateTracker<F, PaymentData<F>, T> + Send + Sync)> {
        Err(report!(errors::ApiErrorResponse::InternalServerError))
            .attach_printable_lazy(|| format!("update tracker interface not found for {self:?}"))
    }
    fn to_post_update_tracker(
        &self,
    ) -> RouterResult<&(dyn PostUpdateTracker<F, PaymentData<F>, T> + Send + Sync)> {
        Err(report!(errors::ApiErrorResponse::InternalServerError)).attach_printable_lazy(|| {
            format!("post connector update tracker not found for {self:?}")
        })
    }
}

pub struct ValidateResult<'a> {
    pub merchant_id: &'a str,
    pub payment_id: api::PaymentIdType,
    pub mandate_type: Option<api::MandateTxnType>,
    pub storage_scheme: enums::MerchantStorageScheme,
}

#[allow(clippy::type_complexity)]
pub trait ValidateRequest<F, R> {
    fn validate_request<'a, 'b>(
        &'b self,
        request: &R,
        merchant_account: &'a storage::MerchantAccount,
    ) -> RouterResult<(BoxedOperation<'b, F, R>, ValidateResult<'a>)>;
}

#[async_trait]
pub trait GetTracker<F, D, R>: Send {
    #[allow(clippy::too_many_arguments)]
    async fn get_trackers<'a>(
        &'a self,
        state: &'a AppState,
        payment_id: &api::PaymentIdType,
        merchant_id: &str,
        request: &R,
        mandate_type: Option<api::MandateTxnType>,
        storage_scheme: enums::MerchantStorageScheme,
    ) -> RouterResult<(BoxedOperation<'a, F, R>, D, Option<CustomerDetails>)>;
}

#[async_trait]
pub trait Domain<F: Clone, R>: Send + Sync {
    /// This will fetch customer details, (this operation is flow specific)
    async fn get_or_create_customer_details<'a>(
        &'a self,
        db: &dyn StorageInterface,
        payment_data: &mut PaymentData<F>,
        request: Option<CustomerDetails>,
        merchant_id: &str,
    ) -> CustomResult<(BoxedOperation<'a, F, R>, Option<storage::Customer>), errors::StorageError>;

    #[allow(clippy::too_many_arguments)]
    async fn make_pm_data<'a>(
        &'a self,
        state: &'a AppState,
        _payment_method: Option<enums::PaymentMethodType>,
        txn_id: &str,
        payment_attempt: &storage::PaymentAttempt,
        request: &Option<api::PaymentMethod>,
        token: &Option<String>,
        card_cvc: Option<Secret<String>>,
        storage_scheme: enums::MerchantStorageScheme,
    ) -> RouterResult<(
        BoxedOperation<'a, F, R>,
        Option<api::PaymentMethod>,
        Option<String>,
    )>;

    async fn add_task_to_process_tracker<'a>(
        &'a self,
        _db: &'a AppState,
        _payment_attempt: &storage::PaymentAttempt,
    ) -> CustomResult<(), errors::ApiErrorResponse> {
        Ok(())
    }

    async fn get_connector<'a>(
        &'a self,
        merchant_account: &storage::MerchantAccount,
        state: &AppState,
    ) -> CustomResult<api::ConnectorCallType, errors::ApiErrorResponse>;
}

#[async_trait]
pub trait UpdateTracker<F, D, R>: Send {
    async fn update_trackers<'b>(
        &'b self,
        db: &dyn StorageInterface,
        payment_id: &api::PaymentIdType,
        payment_data: D,
        customer: Option<Customer>,
        storage_scheme: enums::MerchantStorageScheme,
    ) -> RouterResult<(BoxedOperation<'b, F, R>, D)>
    where
        F: 'b + Send;
}

#[async_trait]
pub trait PostUpdateTracker<F, D, R>: Send {
    async fn update_tracker<'b>(
        &'b self,
        db: &dyn StorageInterface,
        payment_id: &api::PaymentIdType,
        payment_data: D,
        response: Option<types::RouterData<F, R, PaymentsResponseData>>,
        storage_scheme: enums::MerchantStorageScheme,
    ) -> RouterResult<D>
    where
        F: 'b + Send;
}

#[async_trait]
impl<F: Clone + Send, Op: Send + Sync + Operation<F, api::PaymentsRequest>>
    Domain<F, api::PaymentsRequest> for Op
where
    for<'a> &'a Op: Operation<F, api::PaymentsRequest> + std::fmt::Debug,
{
    #[instrument(skip_all)]
    async fn get_or_create_customer_details<'a>(
        &'a self,
        db: &dyn StorageInterface,
        payment_data: &mut PaymentData<F>,
        request: Option<CustomerDetails>,
        merchant_id: &str,
    ) -> CustomResult<
        (
            BoxedOperation<'a, F, api::PaymentsRequest>,
            Option<storage::Customer>,
        ),
        errors::StorageError,
    > {
        helpers::create_customer_if_not_exist(
            Box::new(self),
            db,
            payment_data,
            request,
            merchant_id,
        )
        .await
    }

    #[instrument(skip_all)]
    async fn make_pm_data<'a>(
        &'a self,
        state: &'a AppState,
        payment_method: Option<enums::PaymentMethodType>,
        txn_id: &str,
        payment_attempt: &storage::PaymentAttempt,
        request: &Option<api::PaymentMethod>,
        token: &Option<String>,
        card_cvc: Option<Secret<String>>,
        _storage_scheme: enums::MerchantStorageScheme,
    ) -> RouterResult<(
        BoxedOperation<'a, F, api::PaymentsRequest>,
        Option<api::PaymentMethod>,
        Option<String>,
    )> {
        helpers::make_pm_data(
            Box::new(self),
            state,
            payment_method,
            txn_id,
            payment_attempt,
            request,
            token,
            card_cvc,
        )
        .await
    }

    #[instrument(skip_all)]
    async fn add_task_to_process_tracker<'a>(
        &'a self,
        state: &'a AppState,
        payment_attempt: &storage::PaymentAttempt,
    ) -> CustomResult<(), errors::ApiErrorResponse> {
        if helpers::check_if_operation_confirm(self) {
            metrics::TASKS_ADDED_COUNT.add(&metrics::CONTEXT, 1, &[]); // Metrics

            let connector_name = payment_attempt
                .connector
                .clone()
                .ok_or(errors::ApiErrorResponse::InternalServerError)?;

            let schedule_time = payment_sync::get_sync_process_schedule_time(
                &*state.store,
                &connector_name,
                &payment_attempt.merchant_id,
                0,
            )
            .await
            .into_report()
            .change_context(errors::ApiErrorResponse::InternalServerError)?;

            match schedule_time {
                Some(stime) => super::add_process_sync_task(&*state.store, payment_attempt, stime)
                    .await
                    .into_report()
                    .change_context(errors::ApiErrorResponse::InternalServerError),
                None => Ok(()),
            }
        } else {
            Ok(())
        }
    }

    async fn get_connector<'a>(
        &'a self,
        merchant_account: &storage::MerchantAccount,
        state: &AppState,
    ) -> CustomResult<api::ConnectorCallType, errors::ApiErrorResponse> {
        helpers::get_connector_default(merchant_account, state).await
    }
}

#[async_trait]
impl<F: Clone + Send, Op: Send + Sync + Operation<F, api::PaymentsRetrieveRequest>>
    Domain<F, api::PaymentsRetrieveRequest> for Op
where
    for<'a> &'a Op: Operation<F, api::PaymentsRetrieveRequest>,
{
    #[instrument(skip_all)]
    async fn get_or_create_customer_details<'a>(
        &'a self,
        db: &dyn StorageInterface,
        payment_data: &mut PaymentData<F>,
        _request: Option<CustomerDetails>,
        merchant_id: &str,
    ) -> CustomResult<
        (
            BoxedOperation<'a, F, api::PaymentsRetrieveRequest>,
            Option<storage::Customer>,
        ),
        errors::StorageError,
    > {
        Ok((
            Box::new(self),
            helpers::get_customer_from_details(
                db,
                payment_data.payment_intent.customer_id.clone(),
                merchant_id,
            )
            .await?,
        ))
    }

    async fn get_connector<'a>(
        &'a self,
        merchant_account: &storage::MerchantAccount,
        state: &AppState,
    ) -> CustomResult<api::ConnectorCallType, errors::ApiErrorResponse> {
        helpers::get_connector_default(merchant_account, state).await
    }

    #[instrument(skip_all)]
    async fn make_pm_data<'a>(
        &'a self,
        state: &'a AppState,
        payment_method: Option<enums::PaymentMethodType>,
        txn_id: &str,
        payment_attempt: &storage::PaymentAttempt,
        request: &Option<api::PaymentMethod>,
        token: &Option<String>,
        card_cvc: Option<Secret<String>>,
        _storage_scheme: enums::MerchantStorageScheme,
    ) -> RouterResult<(
        BoxedOperation<'a, F, api::PaymentsRetrieveRequest>,
        Option<api::PaymentMethod>,
        Option<String>,
    )> {
        helpers::make_pm_data(
            Box::new(self),
            state,
            payment_method,
            txn_id,
            payment_attempt,
            request,
            token,
            card_cvc,
        )
        .await
    }
}

#[async_trait]
impl<F: Clone + Send, Op: Send + Sync + Operation<F, api::PaymentsCaptureRequest>>
    Domain<F, api::PaymentsCaptureRequest> for Op
where
    for<'a> &'a Op: Operation<F, api::PaymentsCaptureRequest>,
{
    #[instrument(skip_all)]
    async fn get_or_create_customer_details<'a>(
        &'a self,
        db: &dyn StorageInterface,
        payment_data: &mut PaymentData<F>,
        _request: Option<CustomerDetails>,
        merchant_id: &str,
    ) -> CustomResult<
        (
            BoxedOperation<'a, F, api::PaymentsCaptureRequest>,
            Option<storage::Customer>,
        ),
        errors::StorageError,
    > {
        Ok((
            Box::new(self),
            helpers::get_customer_from_details(
                db,
                payment_data.payment_intent.customer_id.clone(),
                merchant_id,
            )
            .await?,
        ))
    }
    #[instrument(skip_all)]
    async fn make_pm_data<'a>(
        &'a self,
        _state: &'a AppState,
        _payment_method: Option<enums::PaymentMethodType>,
        _txn_id: &str,
        _payment_attempt: &storage::PaymentAttempt,
        _request: &Option<api::PaymentMethod>,
        _token: &Option<String>,
        _card_cvc: Option<Secret<String>>,
        _storage_scheme: enums::MerchantStorageScheme,
    ) -> RouterResult<(
        BoxedOperation<'a, F, api::PaymentsCaptureRequest>,
        Option<api::PaymentMethod>,
        Option<String>,
    )> {
        Ok((Box::new(self), None, None))
    }

    async fn get_connector<'a>(
        &'a self,
        merchant_account: &storage::MerchantAccount,
        state: &AppState,
    ) -> CustomResult<api::ConnectorCallType, errors::ApiErrorResponse> {
        helpers::get_connector_default(merchant_account, state).await
    }
}

#[async_trait]
impl<F: Clone + Send, Op: Send + Sync + Operation<F, api::PaymentsCancelRequest>>
    Domain<F, api::PaymentsCancelRequest> for Op
where
    for<'a> &'a Op: Operation<F, api::PaymentsCancelRequest>,
{
    #[instrument(skip_all)]
    async fn get_or_create_customer_details<'a>(
        &'a self,
        db: &dyn StorageInterface,
        payment_data: &mut PaymentData<F>,
        _request: Option<CustomerDetails>,
        merchant_id: &str,
    ) -> CustomResult<
        (
            BoxedOperation<'a, F, api::PaymentsCancelRequest>,
            Option<storage::Customer>,
        ),
        errors::StorageError,
    > {
        Ok((
            Box::new(self),
            helpers::get_customer_from_details(
                db,
                payment_data.payment_intent.customer_id.clone(),
                merchant_id,
            )
            .await?,
        ))
    }

    #[instrument(skip_all)]
    async fn make_pm_data<'a>(
        &'a self,
        _state: &'a AppState,
        _payment_method: Option<enums::PaymentMethodType>,
        _txn_id: &str,
        _payment_attempt: &storage::PaymentAttempt,
        _request: &Option<api::PaymentMethod>,
        _token: &Option<String>,
        _card_cvc: Option<Secret<String>>,
        _storage_scheme: enums::MerchantStorageScheme,
    ) -> RouterResult<(
        BoxedOperation<'a, F, api::PaymentsCancelRequest>,
        Option<api::PaymentMethod>,
        Option<String>,
    )> {
        Ok((Box::new(self), None, None))
    }

    async fn get_connector<'a>(
        &'a self,
        merchant_account: &storage::MerchantAccount,
        state: &AppState,
    ) -> CustomResult<api::ConnectorCallType, errors::ApiErrorResponse> {
        helpers::get_connector_default(merchant_account, state).await
    }
}
