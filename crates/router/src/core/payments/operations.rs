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
    routes::AppState,
    services,
    types::{
        self, api, domain,
        storage::{self, enums},
        PaymentsResponseData,
    },
};

pub type BoxedOperation<'a, F, T, Ctx> = Box<dyn Operation<F, T, Ctx> + Send + Sync + 'a>;

pub trait Operation<F: Clone, T, Ctx: PaymentMethodRetrieve>: Send + std::fmt::Debug {
        /// This method attempts to retrieve a reference to an object that implements the ValidateRequest trait for the specified types F, T, and Ctx. If the object is found, it returns a RouterResult containing a reference to the object. If the object is not found, it returns an error with an internal server error response, along with a printable message indicating that the validate request interface was not found for the specified types.
    fn to_validate_request(&self) -> RouterResult<&(dyn ValidateRequest<F, T, Ctx> + Send + Sync)> {
        Err(report!(errors::ApiErrorResponse::InternalServerError))
            .attach_printable_lazy(|| format!("validate request interface not found for {self:?}"))
    }
        /// This method returns a reference to a `GetTracker` trait object that is both `Send` and `Sync`. It returns an `InternalServerError` ApiErrorResponse if the tracker interface is not found for the specified parameters.
    fn to_get_tracker(
        &self,
    ) -> RouterResult<&(dyn GetTracker<F, PaymentData<F>, T, Ctx> + Send + Sync)> {
        Err(report!(errors::ApiErrorResponse::InternalServerError))
            .attach_printable_lazy(|| format!("get tracker interface not found for {self:?}"))
    }
        /// This method attempts to convert the current object into a reference to a trait object representing a domain, and returns a `RouterResult` containing the result. If successful, it will return a reference to the domain. If not, it will return an `ApiErrorResponse::InternalServerError` error with a printable message indicating that the domain interface could not be found for the current object.
    fn to_domain(&self) -> RouterResult<&dyn Domain<F, T, Ctx>> {
        Err(report!(errors::ApiErrorResponse::InternalServerError))
            .attach_printable_lazy(|| format!("domain interface not found for {self:?}"))
    }
        /// This method returns an error result with an internal server error response if the update tracker interface is not found for the given context.
    fn to_update_tracker(
        &self,
    ) -> RouterResult<&(dyn UpdateTracker<F, PaymentData<F>, T, Ctx> + Send + Sync)> {
        Err(report!(errors::ApiErrorResponse::InternalServerError))
            .attach_printable_lazy(|| format!("update tracker interface not found for {self:?}"))
    }
        /// Retrieves the post update tracker for the current instance.
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
pub trait ValidateRequest<F, R, Ctx: PaymentMethodRetrieve> {
    fn validate_request<'a, 'b>(
        &'b self,
        request: &R,
        merchant_account: &'a domain::MerchantAccount,
    ) -> RouterResult<(BoxedOperation<'b, F, R, Ctx>, ValidateResult<'a>)>;
}

pub struct GetTrackerResponse<'a, F: Clone, R, Ctx> {
    pub operation: BoxedOperation<'a, F, R, Ctx>,
    pub customer_details: Option<CustomerDetails>,
    pub payment_data: PaymentData<F>,
    pub business_profile: storage::business_profile::BusinessProfile,
}

#[async_trait]
pub trait GetTracker<F: Clone, D, R, Ctx: PaymentMethodRetrieve>: Send {
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
    )>;

        /// Adds a task to the process tracker for handling payment attempts. 
    /// 
    /// # Arguments
    /// 
    /// * `_db` - The reference to the application state.
    /// * `_payment_attempt` - The reference to the payment attempt that needs to be added to the process tracker.
    /// * `_requeue` - A boolean indicating whether to requeue the task or not.
    /// * `_schedule_time` - An optional primitive date time for scheduling the task.
    /// 
    /// # Returns
    /// 
    /// Returns a `CustomResult<(), errors::ApiErrorResponse>` indicating the result of the operation.
    /// 
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

        /// Asynchronously populates the payment data for a given merchant account.
    ///
    /// # Arguments
    ///
    /// * `state` - The application state
    /// * `payment_data` - Mutable reference to the payment data to be populated
    /// * `merchant_account` - The merchant account for which the payment data is being populated
    ///
    /// # Returns
    ///
    /// A `CustomResult` indicating the success or failure of the operation, with an optional `ApiErrorResponse` in case of failure.
    ///
    async fn populate_payment_data<'a>(
        &'a self,
        _state: &AppState,
        _payment_data: &mut PaymentData<F>,
        _merchant_account: &domain::MerchantAccount,
    ) -> CustomResult<(), errors::ApiErrorResponse> {
        Ok(())
    }
}

#[async_trait]
#[allow(clippy::too_many_arguments)]
pub trait UpdateTracker<F, D, Req, Ctx: PaymentMethodRetrieve>: Send {
    async fn update_trackers<'b>(
        &'b self,
        db: &'b AppState,
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
        /// Asynchronously retrieves existing customer details from the database based on the provided payment data, or creates new customer details if none are found. If a request for customer details is provided, it will be ignored. Returns a tuple containing a boxed operation and an optional customer object, or a storage error if the operation fails.
    async fn get_or_create_customer_details<'a>(
        &'a self,
        db: &dyn StorageInterface,
        payment_data: &mut PaymentData<F>,
        _request: Option<CustomerDetails>,
        merchant_key_store: &domain::MerchantKeyStore,
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
            )
            .await?,
        ))
    }

        /// Asynchronously retrieves a connector choice based on the provided merchant account, application state, payment retrieval request, payment intent, and merchant key store.
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
        /// Asynchronously creates payment method data using the provided state, payment data, storage scheme, merchant key store, and customer information.
    async fn make_pm_data<'a>(
        &'a self,
        state: &'a AppState,
        payment_data: &mut PaymentData<F>,
        _storage_scheme: enums::MerchantStorageScheme,
        merchant_key_store: &domain::MerchantKeyStore,
        customer: &Option<domain::Customer>,
    ) -> RouterResult<(
        BoxedOperation<'a, F, api::PaymentsRetrieveRequest, Ctx>,
        Option<api::PaymentMethodData>,
    )> {
        helpers::make_pm_data(
            Box::new(self),
            state,
            payment_data,
            merchant_key_store,
            customer,
        )
        .await
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
        /// This method either retrieves customer details from the database or creates new customer details if they do not exist. It takes a storage interface, mutable payment data, an optional customer details request, and a merchant key store as input parameters. It returns a custom result containing a boxed operation and an optional customer, or a storage error if the operation fails.
    async fn get_or_create_customer_details<'a>(
        &'a self,
        db: &dyn StorageInterface,
        payment_data: &mut PaymentData<F>,
        _request: Option<CustomerDetails>,
        merchant_key_store: &domain::MerchantKeyStore,
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
            )
            .await?,
        ))
    }
    #[instrument(skip_all)]
        /// Asynchronously creates payment method data using the provided state, payment data, storage scheme, merchant key store, and customer. Returns a tuple containing a boxed operation and an optional payment method data.
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
    )> {
        Ok((Box::new(self), None))
    }

        /// Asynchronously retrieves a connector choice based on the given merchant account, application state, payments capture request, payment intent, and merchant key store. Returns a Result containing the selected connector choice or an API error response.
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
        /// Asynchronously gets or creates customer details from the database using the provided payment data, request, and merchant key store. Returns a tuple containing a boxed operation and an optional customer.
    async fn get_or_create_customer_details<'a>(
        &'a self,
        db: &dyn StorageInterface,
        payment_data: &mut PaymentData<F>,
        _request: Option<CustomerDetails>,
        merchant_key_store: &domain::MerchantKeyStore,
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
            )
            .await?,
        ))
    }

    #[instrument(skip_all)]
        /// Asynchronously creates payment method data for a given state, payment data, storage scheme, merchant key store, and customer. 
    /// Returns a tuple containing a boxed operation and an optional payment method data.
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
    )> {
        Ok((Box::new(self), None))
    }

        /// Asynchronously retrieves a connector choice based on the provided merchant account, application state, request, payment intent, and merchant key store.
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
        /// Asynchronously retrieves or creates customer details from the database using the provided payment data, request, and merchant key store. Returns a custom result containing a boxed operation and an optional customer, or a storage error if the operation fails.
    async fn get_or_create_customer_details<'a>(
        &'a self,
        _db: &dyn StorageInterface,
        _payment_data: &mut PaymentData<F>,
        _request: Option<CustomerDetails>,
        _merchant_key_store: &domain::MerchantKeyStore,
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
        /// Asynchronously creates payment method data using the provided state, payment data, storage scheme, merchant key store, and customer. 
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
    )> {
        Ok((Box::new(self), None))
    }

        /// Asynchronously retrieves a connector choice for processing a payment rejection request
    ///
    /// # Arguments
    ///
    /// * `_merchant_account` - The merchant account associated with the request
    /// * `state` - The application state
    /// * `_request` - The payment rejection request
    /// * `_payment_intent` - The payment intent being rejected
    /// * `_merchant_key_store` - The merchant key store
    ///
    /// # Returns
    ///
    /// A custom result containing the chosen connector or an API error response
    ///
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
}
