pub mod payment_approve;
pub mod payment_cancel;
pub mod payment_capture;
pub mod payment_complete_authorize;
pub mod payment_confirm;
pub mod payment_create;
pub mod payment_reject;
pub mod payment_response;
pub mod payment_session;
pub mod payment_start;
pub mod payment_status;
pub mod payment_update;
pub mod payments_incremental_authorization;

use api_models::enums::FrmSuggestion;
use async_trait::async_trait;
use error_stack::{report, ResultExt};
use router_env::{instrument, tracing};

pub use self::{
    payment_approve::PaymentApprove, payment_cancel::PaymentCancel,
    payment_capture::PaymentCapture, payment_confirm::PaymentConfirm,
    payment_create::PaymentCreate, payment_reject::PaymentReject,
    payment_response::PaymentResponse, payment_session::PaymentSession,
    payment_start::PaymentStart, payment_status::PaymentStatus, payment_update::PaymentUpdate,
    payments_incremental_authorization::PaymentIncrementalAuthorization,
};
use super::{helpers, CustomerDetails, PaymentData};
use crate::{
    core::{
        errors::{self, CustomResult, RouterResult},
        payment_methods::PaymentMethodRetrieve,
    },
    db::StorageInterface,
    routes::{app::ReqState, AppState},
    services,
    types::{
        self,
        api::{self, ConnectorCallType},
        domain,
        storage::{self, enums},
        PaymentsResponseData,
    },
};

pub type BoxedOperation<'a, F, T, Ctx> = Box<dyn Operation<F, T, Ctx> + Send + Sync + 'a>;

pub trait Operation<F: Clone, T, Ctx: PaymentMethodRetrieve>: Send + std::fmt::Debug {
    fn to_validate_request(&self) -> RouterResult<&(dyn ValidateRequest<F, T, Ctx> + Send + Sync)> {
        Err(report!(errors::ApiErrorResponse::InternalServerError))
            .attach_printable_lazy(|| format!("validate request interface not found for {self:?}"))
    }
    fn to_get_tracker(
        &self,
    ) -> RouterResult<&(dyn GetTracker<F, PaymentData<F>, T, Ctx> + Send + Sync)> {
        Err(report!(errors::ApiErrorResponse::InternalServerError))
            .attach_printable_lazy(|| format!("get tracker interface not found for {self:?}"))
    }
    fn to_domain(&self) -> RouterResult<&dyn Domain<F, T, Ctx>> {
        Err(report!(errors::ApiErrorResponse::InternalServerError))
            .attach_printable_lazy(|| format!("domain interface not found for {self:?}"))
    }
    fn to_update_tracker(
        &self,
    ) -> RouterResult<&(dyn UpdateTracker<F, PaymentData<F>, T, Ctx> + Send + Sync)> {
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
    pub storage_scheme: enums::MerchantStorageScheme,
    pub requeue: bool,
}

#[allow(clippy::type_complexity)]
pub trait ValidateRequest<F, R, Ctx: PaymentMethodRetrieve> {
    fn validate_request<'a, 'b>(
        &'b self,
        request: &R,
        merchant_account: &'a domain::MerchantAccount,
    ) -> RouterResult<(BoxedOperation<'b, F, R, Ctx>, ValidateResult<'a>)>;
}
pub trait ValidateRequestFlow<F, R, Ctx: PaymentMethodRetrieve> {
    fn validate_request_for_flow<'a, 'b>(
        &'b self,
        request: &R,
        merchant_account: &'a domain::MerchantAccount,
    ) -> RouterResult<()> {
        Ok(())
    }
}

pub struct GetTrackerResponse<'a, F: Clone, R, Ctx> {
    pub operation: BoxedOperation<'a, F, R, Ctx>,
    pub customer_details: Option<CustomerDetails>,
    pub payment_data: PaymentData<F>,
    pub business_profile: storage::business_profile::BusinessProfile,
    pub mandate_type: Option<api::MandateTransactionType>,
}

#[async_trait]
pub trait GetTracker<F: Clone, D, R, Ctx: PaymentMethodRetrieve>: Send {
    #[allow(clippy::too_many_arguments)]
    async fn get_trackers<'a>(
        &'a self,
        state: &'a AppState,
        payment_id: &api::PaymentIdType,
        request: &R,
        merchant_account: &domain::MerchantAccount,
        mechant_key_store: &domain::MerchantKeyStore,
        auth_flow: services::AuthFlow,
        payment_confirm_source: Option<enums::PaymentSource>,
    ) -> RouterResult<GetTrackerResponse<'a, F, R, Ctx>>;
}

#[async_trait]
pub trait Domain<F: Clone, R, Ctx: PaymentMethodRetrieve>: Send + Sync {
    /// This will fetch customer details, (this operation is flow specific)
    async fn get_or_create_customer_details<'a>(
        &'a self,
        db: &dyn StorageInterface,
        payment_data: &mut PaymentData<F>,
        request: Option<CustomerDetails>,
        merchant_key_store: &domain::MerchantKeyStore,
        storage_scheme: enums::MerchantStorageScheme,
    ) -> CustomResult<(BoxedOperation<'a, F, R, Ctx>, Option<domain::Customer>), errors::StorageError>;

    #[allow(clippy::too_many_arguments)]
    async fn make_pm_data<'a>(
        &'a self,
        state: &'a AppState,
        payment_data: &mut PaymentData<F>,
        storage_scheme: enums::MerchantStorageScheme,
        merchant_key_store: &domain::MerchantKeyStore,
        customer: &Option<domain::Customer>,
    ) -> RouterResult<(
        BoxedOperation<'a, F, R, Ctx>,
        Option<api::PaymentMethodData>,
        Option<String>,
    )>;

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
        payment_intent: &storage::PaymentIntent,
        mechant_key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<api::ConnectorChoice, errors::ApiErrorResponse>;

    async fn populate_payment_data<'a>(
        &'a self,
        _state: &AppState,
        _payment_data: &mut PaymentData<F>,
        _merchant_account: &domain::MerchantAccount,
    ) -> CustomResult<(), errors::ApiErrorResponse> {
        Ok(())
    }

    async fn call_external_three_ds_authentication_if_eligible<'a>(
        &'a self,
        _state: &AppState,
        _payment_data: &mut PaymentData<F>,
        _should_continue_confirm_transaction: &mut bool,
        _connector_call_type: &ConnectorCallType,
        _merchant_account: &storage::BusinessProfile,
        _key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<(), errors::ApiErrorResponse> {
        Ok(())
    }

    #[instrument(skip_all)]
    async fn guard_payment_against_blocklist<'a>(
        &'a self,
        _state: &AppState,
        _merchant_account: &domain::MerchantAccount,
        _payment_data: &mut PaymentData<F>,
    ) -> CustomResult<bool, errors::ApiErrorResponse> {
        Ok(false)
    }
}

#[async_trait]
#[allow(clippy::too_many_arguments)]
pub trait UpdateTracker<F, D, Req, Ctx: PaymentMethodRetrieve>: Send {
    async fn update_trackers<'b>(
        &'b self,
        db: &'b AppState,
        req_state: ReqState,
        payment_data: D,
        customer: Option<domain::Customer>,
        storage_scheme: enums::MerchantStorageScheme,
        updated_customer: Option<storage::CustomerUpdate>,
        mechant_key_store: &domain::MerchantKeyStore,
        frm_suggestion: Option<FrmSuggestion>,
        header_payload: api::HeaderPayload,
    ) -> RouterResult<(BoxedOperation<'b, F, Req, Ctx>, D)>
    where
        F: 'b + Send;
}

#[async_trait]
pub trait PostUpdateTracker<F, D, R>: Send {
    async fn update_tracker<'b>(
        &'b self,
        db: &'b AppState,
        payment_id: &api::PaymentIdType,
        payment_data: D,
        response: types::RouterData<F, R, PaymentsResponseData>,
        storage_scheme: enums::MerchantStorageScheme,
    ) -> RouterResult<D>
    where
        F: 'b + Send;
}

#[async_trait]
impl<
        F: Clone + Send,
        Ctx: PaymentMethodRetrieve,
        Op: Send + Sync + Operation<F, api::PaymentsRetrieveRequest, Ctx>,
    > Domain<F, api::PaymentsRetrieveRequest, Ctx> for Op
where
    for<'a> &'a Op: Operation<F, api::PaymentsRetrieveRequest, Ctx>,
{
    #[instrument(skip_all)]
    async fn get_or_create_customer_details<'a>(
        &'a self,
        db: &dyn StorageInterface,
        payment_data: &mut PaymentData<F>,
        _request: Option<CustomerDetails>,
        merchant_key_store: &domain::MerchantKeyStore,
        storage_scheme: enums::MerchantStorageScheme,
    ) -> CustomResult<
        (
            BoxedOperation<'a, F, api::PaymentsRetrieveRequest, Ctx>,
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
                storage_scheme,
            )
            .await?,
        ))
    }

    async fn get_connector<'a>(
        &'a self,
        _merchant_account: &domain::MerchantAccount,
        state: &AppState,
        _request: &api::PaymentsRetrieveRequest,
        _payment_intent: &storage::PaymentIntent,
        _merchant_key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<api::ConnectorChoice, errors::ApiErrorResponse> {
        helpers::get_connector_default(state, None).await
    }

    #[instrument(skip_all)]
    async fn make_pm_data<'a>(
        &'a self,
        state: &'a AppState,
        payment_data: &mut PaymentData<F>,
        storage_scheme: enums::MerchantStorageScheme,
        merchant_key_store: &domain::MerchantKeyStore,
        customer: &Option<domain::Customer>,
    ) -> RouterResult<(
        BoxedOperation<'a, F, api::PaymentsRetrieveRequest, Ctx>,
        Option<api::PaymentMethodData>,
        Option<String>,
    )> {
        helpers::make_pm_data(
            Box::new(self),
            state,
            payment_data,
            merchant_key_store,
            customer,
            storage_scheme,
        )
        .await
    }

    #[instrument(skip_all)]
    async fn guard_payment_against_blocklist<'a>(
        &'a self,
        _state: &AppState,
        _merchant_account: &domain::MerchantAccount,
        _payment_data: &mut PaymentData<F>,
    ) -> CustomResult<bool, errors::ApiErrorResponse> {
        Ok(false)
    }
}

#[async_trait]
impl<
        F: Clone + Send,
        Ctx: PaymentMethodRetrieve,
        Op: Send + Sync + Operation<F, api::PaymentsCaptureRequest, Ctx>,
    > Domain<F, api::PaymentsCaptureRequest, Ctx> for Op
where
    for<'a> &'a Op: Operation<F, api::PaymentsCaptureRequest, Ctx>,
{
    #[instrument(skip_all)]
    async fn get_or_create_customer_details<'a>(
        &'a self,
        db: &dyn StorageInterface,
        payment_data: &mut PaymentData<F>,
        _request: Option<CustomerDetails>,
        merchant_key_store: &domain::MerchantKeyStore,
        storage_scheme: enums::MerchantStorageScheme,
    ) -> CustomResult<
        (
            BoxedOperation<'a, F, api::PaymentsCaptureRequest, Ctx>,
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
                storage_scheme,
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
        _merchant_key_store: &domain::MerchantKeyStore,
        _customer: &Option<domain::Customer>,
    ) -> RouterResult<(
        BoxedOperation<'a, F, api::PaymentsCaptureRequest, Ctx>,
        Option<api::PaymentMethodData>,
        Option<String>,
    )> {
        Ok((Box::new(self), None, None))
    }

    async fn get_connector<'a>(
        &'a self,
        _merchant_account: &domain::MerchantAccount,
        state: &AppState,
        _request: &api::PaymentsCaptureRequest,
        _payment_intent: &storage::PaymentIntent,
        _merchant_key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<api::ConnectorChoice, errors::ApiErrorResponse> {
        helpers::get_connector_default(state, None).await
    }

    #[instrument(skip_all)]
    async fn guard_payment_against_blocklist<'a>(
        &'a self,
        _state: &AppState,
        _merchant_account: &domain::MerchantAccount,
        _payment_data: &mut PaymentData<F>,
    ) -> CustomResult<bool, errors::ApiErrorResponse> {
        Ok(false)
    }
}

#[async_trait]
impl<
        F: Clone + Send,
        Ctx: PaymentMethodRetrieve,
        Op: Send + Sync + Operation<F, api::PaymentsCancelRequest, Ctx>,
    > Domain<F, api::PaymentsCancelRequest, Ctx> for Op
where
    for<'a> &'a Op: Operation<F, api::PaymentsCancelRequest, Ctx>,
{
    #[instrument(skip_all)]
    async fn get_or_create_customer_details<'a>(
        &'a self,
        db: &dyn StorageInterface,
        payment_data: &mut PaymentData<F>,
        _request: Option<CustomerDetails>,
        merchant_key_store: &domain::MerchantKeyStore,
        storage_scheme: enums::MerchantStorageScheme,
    ) -> CustomResult<
        (
            BoxedOperation<'a, F, api::PaymentsCancelRequest, Ctx>,
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
                storage_scheme,
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
        _merchant_key_store: &domain::MerchantKeyStore,
        _customer: &Option<domain::Customer>,
    ) -> RouterResult<(
        BoxedOperation<'a, F, api::PaymentsCancelRequest, Ctx>,
        Option<api::PaymentMethodData>,
        Option<String>,
    )> {
        Ok((Box::new(self), None, None))
    }

    async fn get_connector<'a>(
        &'a self,
        _merchant_account: &domain::MerchantAccount,
        state: &AppState,
        _request: &api::PaymentsCancelRequest,
        _payment_intent: &storage::PaymentIntent,
        _merchant_key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<api::ConnectorChoice, errors::ApiErrorResponse> {
        helpers::get_connector_default(state, None).await
    }

    #[instrument(skip_all)]
    async fn guard_payment_against_blocklist<'a>(
        &'a self,
        _state: &AppState,
        _merchant_account: &domain::MerchantAccount,
        _payment_data: &mut PaymentData<F>,
    ) -> CustomResult<bool, errors::ApiErrorResponse> {
        Ok(false)
    }
}

#[async_trait]
impl<
        F: Clone + Send,
        Ctx: PaymentMethodRetrieve,
        Op: Send + Sync + Operation<F, api::PaymentsRejectRequest, Ctx>,
    > Domain<F, api::PaymentsRejectRequest, Ctx> for Op
where
    for<'a> &'a Op: Operation<F, api::PaymentsRejectRequest, Ctx>,
{
    #[instrument(skip_all)]
    async fn get_or_create_customer_details<'a>(
        &'a self,
        _db: &dyn StorageInterface,
        _payment_data: &mut PaymentData<F>,
        _request: Option<CustomerDetails>,
        _merchant_key_store: &domain::MerchantKeyStore,
        _storage_scheme: enums::MerchantStorageScheme,
    ) -> CustomResult<
        (
            BoxedOperation<'a, F, api::PaymentsRejectRequest, Ctx>,
            Option<domain::Customer>,
        ),
        errors::StorageError,
    > {
        Ok((Box::new(self), None))
    }

    #[instrument(skip_all)]
    async fn make_pm_data<'a>(
        &'a self,
        _state: &'a AppState,
        _payment_data: &mut PaymentData<F>,
        _storage_scheme: enums::MerchantStorageScheme,
        _merchant_key_store: &domain::MerchantKeyStore,
        _customer: &Option<domain::Customer>,
    ) -> RouterResult<(
        BoxedOperation<'a, F, api::PaymentsRejectRequest, Ctx>,
        Option<api::PaymentMethodData>,
        Option<String>,
    )> {
        Ok((Box::new(self), None, None))
    }

    async fn get_connector<'a>(
        &'a self,
        _merchant_account: &domain::MerchantAccount,
        state: &AppState,
        _request: &api::PaymentsRejectRequest,
        _payment_intent: &storage::PaymentIntent,
        _merchant_key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<api::ConnectorChoice, errors::ApiErrorResponse> {
        helpers::get_connector_default(state, None).await
    }

    #[instrument(skip_all)]
    async fn guard_payment_against_blocklist<'a>(
        &'a self,
        _state: &AppState,
        _merchant_account: &domain::MerchantAccount,
        _payment_data: &mut PaymentData<F>,
    ) -> CustomResult<bool, errors::ApiErrorResponse> {
        Ok(false)
    }
}
