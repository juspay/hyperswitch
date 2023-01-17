pub mod payment_cancel;
pub mod payment_capture;
pub mod payment_confirm;
pub mod payment_create;
pub mod payment_method_validate;
pub mod payment_response;
pub mod payment_session;
pub mod payment_start;
pub mod payment_status;
pub mod payment_update;

use std::fmt;

use async_trait::async_trait;
use error_stack::{report, ResultExt};
use router_env::{instrument, tracing};

pub use self::{
    payment_cancel::PaymentCancel, payment_capture::PaymentCapture,
    payment_confirm::PaymentConfirm, payment_create::PaymentCreate,
    payment_method_validate::PaymentMethodValidate, payment_response::PaymentResponse,
    payment_session::PaymentSession, payment_start::PaymentStart, payment_status::PaymentStatus,
    payment_update::PaymentUpdate,
};
use super::{helpers, CallConnectorAction, CustomerDetails, PaymentData};
use crate::{
    core::errors::{self, CustomResult, RouterResult},
    db::StorageInterface,
    routes::AppState,
    types::{
        self, api,
        storage::{self, enums},
        PaymentsResponseData,
    },
};

pub type BoxedOperation<'a, T> = Box<dyn Operation<T> + Send + Sync + 'a>;
pub type BoxedEndOperation<'a, F, T> = Box<dyn EndOperation<F, T> + Send + Sync + 'a>;

pub trait Operation<T>: Send + std::fmt::Debug {
    fn to_validate_request(&self) -> RouterResult<&(dyn ValidateRequest<T> + Send + Sync)> {
        Err(report!(errors::ApiErrorResponse::InternalServerError))
            .attach_printable_lazy(|| format!("validate request interface not found for {self:?}"))
    }
    fn to_get_tracker(&self) -> RouterResult<&(dyn GetTracker<PaymentData, T> + Send + Sync)> {
        Err(report!(errors::ApiErrorResponse::InternalServerError))
            .attach_printable_lazy(|| format!("get tracker interface not found for {self:?}"))
    }
    fn to_domain(&self) -> RouterResult<&dyn Domain<T>> {
        Err(report!(errors::ApiErrorResponse::InternalServerError))
            .attach_printable_lazy(|| format!("domain interface not found for {self:?}"))
    }
    fn to_update_tracker(
        &self,
    ) -> RouterResult<&(dyn UpdateTracker<PaymentData, T> + Send + Sync)> {
        Err(report!(errors::ApiErrorResponse::InternalServerError))
            .attach_printable_lazy(|| format!("update tracker interface not found for {self:?}"))
    }
}

pub trait EndOperation<F: Clone, T>: Send + std::fmt::Debug {
    fn to_post_update_tracker(
        &self,
    ) -> RouterResult<&(dyn PostUpdateTracker<F, PaymentData, T> + Send + Sync)> {
        Err(report!(errors::ApiErrorResponse::InternalServerError)).attach_printable_lazy(|| {
            format!("post connector update tracker not found for {self:?}")
        })
    }
}

#[async_trait::async_trait]
pub trait DeriveFlow<F, FData>: fmt::Debug + Send + Sync {
    async fn call_connector(
        &self,
        db: &AppState,
        merchant_account: &storage::MerchantAccount,
        payment_id: &api::PaymentIdType,
        connector: api::ConnectorData,
        payment_data: PaymentData,
        customer: &Option<storage::Customer>,
        call_connector_action: CallConnectorAction,
    ) -> RouterResult<PaymentData> {
        Ok(payment_data)
    }
}

pub struct ValidateResult<'a> {
    pub merchant_id: &'a str,
    pub payment_id: api::PaymentIdType,
    pub mandate_type: Option<api::MandateTxnType>,
    pub storage_scheme: enums::MerchantStorageScheme,
}

#[allow(clippy::type_complexity)]
pub trait ValidateRequest<R> {
    fn validate_request<'a, 'b>(
        &'b self,
        request: &R,
        merchant_account: &'a storage::MerchantAccount,
    ) -> RouterResult<(BoxedOperation<'b, R>, ValidateResult<'a>)>;
}

#[async_trait]
pub trait GetTracker<D, R>: Send {
    #[allow(clippy::too_many_arguments)]
    async fn get_trackers<'a>(
        &'a self,
        state: &'a AppState,
        payment_id: &api::PaymentIdType,
        request: &R,
        mandate_type: Option<api::MandateTxnType>,
        merchant_account: &storage::MerchantAccount,
    ) -> RouterResult<(BoxedOperation<'a, R>, D, Option<CustomerDetails>)>;
}

#[async_trait]
pub trait Domain<R>: Send + Sync {
    /// This will fetch customer details, (this operation is flow specific)
    async fn get_or_create_customer_details<'a>(
        &'a self,
        db: &dyn StorageInterface,
        payment_data: &mut PaymentData,
        request: Option<CustomerDetails>,
        merchant_id: &str,
    ) -> CustomResult<(BoxedOperation<'a, R>, Option<storage::Customer>), errors::StorageError>;

    #[allow(clippy::too_many_arguments)]
    async fn make_pm_data<'a>(
        &'a self,
        state: &'a AppState,
        payment_data: &mut PaymentData,
        storage_scheme: enums::MerchantStorageScheme,
    ) -> RouterResult<(BoxedOperation<'a, R>, Option<api::PaymentMethod>)>;

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
        request: &R,
    ) -> CustomResult<api::ConnectorCallType, errors::ApiErrorResponse>;
}

#[async_trait]
pub trait UpdateTracker<D, Req>: Send {
    async fn update_trackers<'b>(
        &'b self,
        db: &dyn StorageInterface,
        payment_id: &api::PaymentIdType,
        payment_data: D,
        customer: Option<storage::Customer>,
        storage_scheme: enums::MerchantStorageScheme,
    ) -> RouterResult<(BoxedOperation<'b, Req>, D)>;
}

#[async_trait]
pub trait PostUpdateTracker<F, D, R>: Send {
    async fn update_tracker<'b>(
        &'b self,
        db: &dyn StorageInterface,
        payment_id: &api::PaymentIdType,
        payment_data: D,
        response: types::RouterData<F, R, PaymentsResponseData>,
        storage_scheme: enums::MerchantStorageScheme,
    ) -> RouterResult<D>
    where
        F: 'b + Send;
}

#[async_trait]
impl<Op: Send + Sync + Operation<api::PaymentsRetrieveRequest>> Domain<api::PaymentsRetrieveRequest>
    for Op
where
    for<'a> &'a Op: Operation<api::PaymentsRetrieveRequest>,
{
    #[instrument(skip_all)]
    async fn get_or_create_customer_details<'a>(
        &'a self,
        db: &dyn StorageInterface,
        payment_data: &mut PaymentData,
        _request: Option<CustomerDetails>,
        merchant_id: &str,
    ) -> CustomResult<
        (
            BoxedOperation<'a, api::PaymentsRetrieveRequest>,
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
        _merchant_account: &storage::MerchantAccount,
        state: &AppState,
        _request: &api::PaymentsRetrieveRequest,
    ) -> CustomResult<api::ConnectorCallType, errors::ApiErrorResponse> {
        helpers::get_connector_default(state, None).await
    }

    #[instrument(skip_all)]
    async fn make_pm_data<'a>(
        &'a self,
        state: &'a AppState,
        payment_data: &mut PaymentData,
        _storage_scheme: enums::MerchantStorageScheme,
    ) -> RouterResult<(
        BoxedOperation<'a, api::PaymentsRetrieveRequest>,
        Option<api::PaymentMethod>,
    )> {
        helpers::make_pm_data(Box::new(self), state, payment_data).await
    }
}

#[async_trait]
impl<Op: Send + Sync + Operation<api::PaymentsCaptureRequest>> Domain<api::PaymentsCaptureRequest>
    for Op
where
    for<'a> &'a Op: Operation<api::PaymentsCaptureRequest>,
{
    #[instrument(skip_all)]
    async fn get_or_create_customer_details<'a>(
        &'a self,
        db: &dyn StorageInterface,
        payment_data: &mut PaymentData,
        _request: Option<CustomerDetails>,
        merchant_id: &str,
    ) -> CustomResult<
        (
            BoxedOperation<'a, api::PaymentsCaptureRequest>,
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
        _payment_data: &mut PaymentData,
        _storage_scheme: enums::MerchantStorageScheme,
    ) -> RouterResult<(
        BoxedOperation<'a, api::PaymentsCaptureRequest>,
        Option<api::PaymentMethod>,
    )> {
        Ok((Box::new(self), None))
    }

    async fn get_connector<'a>(
        &'a self,
        _merchant_account: &storage::MerchantAccount,
        state: &AppState,
        _request: &api::PaymentsCaptureRequest,
    ) -> CustomResult<api::ConnectorCallType, errors::ApiErrorResponse> {
        helpers::get_connector_default(state, None).await
    }
}

#[async_trait]
impl<Op: Send + Sync + Operation<api::PaymentsCancelRequest>> Domain<api::PaymentsCancelRequest>
    for Op
where
    for<'a> &'a Op: Operation<api::PaymentsCancelRequest>,
{
    #[instrument(skip_all)]
    async fn get_or_create_customer_details<'a>(
        &'a self,
        db: &dyn StorageInterface,
        payment_data: &mut PaymentData,
        _request: Option<CustomerDetails>,
        merchant_id: &str,
    ) -> CustomResult<
        (
            BoxedOperation<'a, api::PaymentsCancelRequest>,
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
        _payment_data: &mut PaymentData,
        _storage_scheme: enums::MerchantStorageScheme,
    ) -> RouterResult<(
        BoxedOperation<'a, api::PaymentsCancelRequest>,
        Option<api::PaymentMethod>,
    )> {
        Ok((Box::new(self), None))
    }

    async fn get_connector<'a>(
        &'a self,
        _merchant_account: &storage::MerchantAccount,
        state: &AppState,
        _request: &api::PaymentsCancelRequest,
    ) -> CustomResult<api::ConnectorCallType, errors::ApiErrorResponse> {
        helpers::get_connector_default(state, None).await
    }
}
