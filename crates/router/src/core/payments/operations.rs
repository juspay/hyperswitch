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
    pii::Secret,
    routes::AppState,
    types::{
        self,
        api::{self, enums as api_enums},
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
        request_connector: Option<api_enums::Connector>,
    ) -> CustomResult<api::ConnectorCallType, errors::ApiErrorResponse>;
}

#[async_trait]
pub trait UpdateTracker<F, D, Req>: Send {
    async fn update_trackers<'b>(
        &'b self,
        db: &dyn StorageInterface,
        payment_id: &api::PaymentIdType,
        payment_data: D,
        customer: Option<storage::Customer>,
        storage_scheme: enums::MerchantStorageScheme,
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
        request_connector: Option<api_enums::Connector>,
    ) -> CustomResult<api::ConnectorCallType, errors::ApiErrorResponse> {
        helpers::get_connector_default(merchant_account, state, request_connector).await
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
        request_connector: Option<api_enums::Connector>,
    ) -> CustomResult<api::ConnectorCallType, errors::ApiErrorResponse> {
        helpers::get_connector_default(merchant_account, state, request_connector).await
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
        request_connector: Option<api_enums::Connector>,
    ) -> CustomResult<api::ConnectorCallType, errors::ApiErrorResponse> {
        helpers::get_connector_default(merchant_account, state, request_connector).await
    }
}
