pub mod payment_cancel;
pub mod payment_capture;
pub mod payment_complete_authorize;
pub mod payment_confirm;
pub mod payment_create;
pub mod payment_method_validate;
pub mod payment_response;
pub mod payment_session;
pub mod payment_start;
pub mod payment_status;
pub mod payment_update;

use api_models::enums::CancelTransaction;
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
use super::{helpers, CustomerDetails, PaymentData};
use crate::{
    core::errors::{self, CustomResult, RouterResult},
    db::StorageInterface,
    routes::AppState,
    services,
    types::{
        self, api, domain,
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

#[derive(Clone)]
pub struct ValidateResult<'a> {
    pub merchant_id: &'a str,
    pub payment_id: api::PaymentIdType,
    pub mandate_type: Option<api::MandateTransactionType>,
    pub storage_scheme: enums::MerchantStorageScheme,
    pub requeue: bool,
}

#[allow(clippy::type_complexity)]
pub trait ValidateRequest<F, R> {
    fn validate_request<'a, 'b>(
        &'b self,
        request: &R,
        merchant_account: &'a domain::MerchantAccount,
    ) -> RouterResult<(BoxedOperation<'b, F, R>, ValidateResult<'a>)>;
}

#[async_trait]
pub trait GetTracker<F, D, R>: Send {
    #[allow(clippy::too_many_arguments)]
    async fn get_trackers<'a>(
        &'a self,
        state: &'a AppState,
        payment_id: &api::PaymentIdType,
        request: &R,
        mandate_type: Option<api::MandateTransactionType>,
        merchant_account: &domain::MerchantAccount,
        mechant_key_store: &domain::MerchantKeyStore,
        auth_flow: services::AuthFlow,
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
        merchant_key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<(BoxedOperation<'a, F, R>, Option<domain::Customer>), errors::StorageError>;

    #[allow(clippy::too_many_arguments)]
    async fn make_pm_data<'a>(
        &'a self,
        state: &'a AppState,
        payment_data: &mut PaymentData<F>,
        storage_scheme: enums::MerchantStorageScheme,
    ) -> RouterResult<(BoxedOperation<'a, F, R>, Option<api::PaymentMethodData>)>;

    async fn add_task_to_process_tracker<'a>(
        &'a self,
        _db: &'a AppState,
        _payment_attempt: &storage::PaymentAttempt,
        _requeue: bool,
        _schedule_time: Option<time::PrimitiveDateTime>,
    ) -> CustomResult<(), errors::ApiErrorResponse> {
        Ok(())
    }

    async fn get_connector<'a>(
        &'a self,
        merchant_account: &domain::MerchantAccount,
        state: &AppState,
        request: &R,
        payment_intent: &storage::payment_intent::PaymentIntent,
        mechant_key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<api::ConnectorChoice, errors::ApiErrorResponse>;
}

#[async_trait]
#[allow(clippy::too_many_arguments)]
pub trait UpdateTracker<F, D, Req>: Send {
    async fn update_trackers<'b>(
        &'b self,
        db: &dyn StorageInterface,
        payment_data: D,
        customer: Option<domain::Customer>,
        storage_scheme: enums::MerchantStorageScheme,
        updated_customer: Option<storage::CustomerUpdate>,
        mechant_key_store: &domain::MerchantKeyStore,
        should_cancel_transaction: Option<CancelTransaction>,
    ) -> RouterResult<(BoxedOperation<'b, F, Req>, D)>
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
        response: types::RouterData<F, R, PaymentsResponseData>,
        storage_scheme: enums::MerchantStorageScheme,
    ) -> RouterResult<D>
    where
        F: 'b + Send;
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
        merchant_key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<
        (
            BoxedOperation<'a, F, api::PaymentsRetrieveRequest>,
            Option<domain::Customer>,
        ),
        errors::StorageError,
    > {
        Ok((
            Box::new(self),
            helpers::get_customer_from_details(
                db,
                payment_data.payment_intent.customer_id.clone(),
                &merchant_key_store.merchant_id,
                payment_data,
                merchant_key_store,
            )
            .await?,
        ))
    }

    async fn get_connector<'a>(
        &'a self,
        _merchant_account: &domain::MerchantAccount,
        state: &AppState,
        _request: &api::PaymentsRetrieveRequest,
        _payment_intent: &storage::payment_intent::PaymentIntent,
        _merchant_key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<api::ConnectorChoice, errors::ApiErrorResponse> {
        helpers::get_connector_default(state, None).await
    }

    #[instrument(skip_all)]
    async fn make_pm_data<'a>(
        &'a self,
        state: &'a AppState,
        payment_data: &mut PaymentData<F>,
        _storage_scheme: enums::MerchantStorageScheme,
    ) -> RouterResult<(
        BoxedOperation<'a, F, api::PaymentsRetrieveRequest>,
        Option<api::PaymentMethodData>,
    )> {
        helpers::make_pm_data(Box::new(self), state, payment_data).await
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
        merchant_key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<
        (
            BoxedOperation<'a, F, api::PaymentsCaptureRequest>,
            Option<domain::Customer>,
        ),
        errors::StorageError,
    > {
        Ok((
            Box::new(self),
            helpers::get_customer_from_details(
                db,
                payment_data.payment_intent.customer_id.clone(),
                &merchant_key_store.merchant_id,
                payment_data,
                merchant_key_store,
            )
            .await?,
        ))
    }
    #[instrument(skip_all)]
    async fn make_pm_data<'a>(
        &'a self,
        _state: &'a AppState,
        _payment_data: &mut PaymentData<F>,
        _storage_scheme: enums::MerchantStorageScheme,
    ) -> RouterResult<(
        BoxedOperation<'a, F, api::PaymentsCaptureRequest>,
        Option<api::PaymentMethodData>,
    )> {
        Ok((Box::new(self), None))
    }

    async fn get_connector<'a>(
        &'a self,
        _merchant_account: &domain::MerchantAccount,
        state: &AppState,
        _request: &api::PaymentsCaptureRequest,
        _payment_intent: &storage::payment_intent::PaymentIntent,
        _merchant_key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<api::ConnectorChoice, errors::ApiErrorResponse> {
        helpers::get_connector_default(state, None).await
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
        merchant_key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<
        (
            BoxedOperation<'a, F, api::PaymentsCancelRequest>,
            Option<domain::Customer>,
        ),
        errors::StorageError,
    > {
        Ok((
            Box::new(self),
            helpers::get_customer_from_details(
                db,
                payment_data.payment_intent.customer_id.clone(),
                &merchant_key_store.merchant_id,
                payment_data,
                merchant_key_store,
            )
            .await?,
        ))
    }

    #[instrument(skip_all)]
    async fn make_pm_data<'a>(
        &'a self,
        _state: &'a AppState,
        _payment_data: &mut PaymentData<F>,
        _storage_scheme: enums::MerchantStorageScheme,
    ) -> RouterResult<(
        BoxedOperation<'a, F, api::PaymentsCancelRequest>,
        Option<api::PaymentMethodData>,
    )> {
        Ok((Box::new(self), None))
    }

    async fn get_connector<'a>(
        &'a self,
        _merchant_account: &domain::MerchantAccount,
        state: &AppState,
        _request: &api::PaymentsCancelRequest,
        _payment_intent: &storage::payment_intent::PaymentIntent,
        _merchant_key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<api::ConnectorChoice, errors::ApiErrorResponse> {
        helpers::get_connector_default(state, None).await
    }
}
