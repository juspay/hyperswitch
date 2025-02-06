#[cfg(feature = "v1")]
pub mod payment_approve;
#[cfg(feature = "v1")]
pub mod payment_cancel;
#[cfg(feature = "v1")]
pub mod payment_capture;
#[cfg(feature = "v1")]
pub mod payment_complete_authorize;
#[cfg(feature = "v1")]
pub mod payment_confirm;
#[cfg(feature = "v1")]
pub mod payment_create;
#[cfg(feature = "v1")]
pub mod payment_post_session_tokens;
#[cfg(feature = "v1")]
pub mod payment_reject;
pub mod payment_response;
#[cfg(feature = "v1")]
pub mod payment_session;
#[cfg(feature = "v2")]
pub mod payment_session_intent;
#[cfg(feature = "v1")]
pub mod payment_start;
#[cfg(feature = "v1")]
pub mod payment_status;
#[cfg(feature = "v1")]
pub mod payment_update;
#[cfg(feature = "v1")]
pub mod payments_incremental_authorization;
#[cfg(feature = "v1")]
pub mod tax_calculation;

#[cfg(feature = "v2")]
pub mod payment_confirm_intent;
#[cfg(feature = "v2")]
pub mod payment_create_intent;
#[cfg(feature = "v2")]
pub mod payment_get_intent;
#[cfg(feature = "v2")]
pub mod payment_update_intent;

#[cfg(feature = "v2")]
pub mod payment_get;

#[cfg(feature = "v2")]
pub mod payment_capture_v2;

use api_models::enums::FrmSuggestion;
#[cfg(all(feature = "v1", feature = "dynamic_routing"))]
use api_models::routing::RoutableConnectorChoice;
use async_trait::async_trait;
use error_stack::{report, ResultExt};
use router_env::{instrument, tracing};

#[cfg(feature = "v2")]
pub use self::payment_get::PaymentGet;
#[cfg(feature = "v2")]
pub use self::payment_get_intent::PaymentGetIntent;
pub use self::payment_response::PaymentResponse;
#[cfg(feature = "v2")]
pub use self::payment_update_intent::PaymentUpdateIntent;
#[cfg(feature = "v1")]
pub use self::{
    payment_approve::PaymentApprove, payment_cancel::PaymentCancel,
    payment_capture::PaymentCapture, payment_confirm::PaymentConfirm,
    payment_create::PaymentCreate, payment_post_session_tokens::PaymentPostSessionTokens,
    payment_reject::PaymentReject, payment_session::PaymentSession, payment_start::PaymentStart,
    payment_status::PaymentStatus, payment_update::PaymentUpdate,
    payments_incremental_authorization::PaymentIncrementalAuthorization,
    tax_calculation::PaymentSessionUpdate,
};
#[cfg(feature = "v2")]
pub use self::{
    payment_confirm_intent::PaymentIntentConfirm, payment_create_intent::PaymentIntentCreate,
    payment_session_intent::PaymentSessionIntent,
};
use super::{helpers, CustomerDetails, OperationSessionGetters, OperationSessionSetters};
use crate::{
    core::errors::{self, CustomResult, RouterResult},
    routes::{app::ReqState, SessionState},
    services,
    types::{
        self,
        api::{self, ConnectorCallType},
        domain,
        storage::{self, enums},
        PaymentsResponseData,
    },
};

pub type BoxedOperation<'a, F, T, D> = Box<dyn Operation<F, T, Data = D> + Send + Sync + 'a>;

pub trait Operation<F: Clone, T>: Send + std::fmt::Debug {
    type Data;
    fn to_validate_request(
        &self,
    ) -> RouterResult<&(dyn ValidateRequest<F, T, Self::Data> + Send + Sync)> {
        Err(report!(errors::ApiErrorResponse::InternalServerError))
            .attach_printable_lazy(|| format!("validate request interface not found for {self:?}"))
    }

    fn to_get_tracker(&self) -> RouterResult<&(dyn GetTracker<F, Self::Data, T> + Send + Sync)> {
        Err(report!(errors::ApiErrorResponse::InternalServerError))
            .attach_printable_lazy(|| format!("get tracker interface not found for {self:?}"))
    }

    fn to_domain(&self) -> RouterResult<&dyn Domain<F, T, Self::Data>> {
        Err(report!(errors::ApiErrorResponse::InternalServerError))
            .attach_printable_lazy(|| format!("domain interface not found for {self:?}"))
    }

    fn to_update_tracker(
        &self,
    ) -> RouterResult<&(dyn UpdateTracker<F, Self::Data, T> + Send + Sync)> {
        Err(report!(errors::ApiErrorResponse::InternalServerError))
            .attach_printable_lazy(|| format!("update tracker interface not found for {self:?}"))
    }

    fn to_post_update_tracker(
        &self,
    ) -> RouterResult<&(dyn PostUpdateTracker<F, Self::Data, T> + Send + Sync)> {
        Err(report!(errors::ApiErrorResponse::InternalServerError)).attach_printable_lazy(|| {
            format!("post connector update tracker not found for {self:?}")
        })
    }
}

#[cfg(feature = "v1")]
#[derive(Clone)]
pub struct ValidateResult {
    pub merchant_id: common_utils::id_type::MerchantId,
    pub payment_id: api::PaymentIdType,
    pub storage_scheme: enums::MerchantStorageScheme,
    pub requeue: bool,
}

#[cfg(feature = "v2")]
#[derive(Clone)]
pub struct ValidateResult {
    pub merchant_id: common_utils::id_type::MerchantId,
    pub storage_scheme: enums::MerchantStorageScheme,
    pub requeue: bool,
}

#[cfg(feature = "v1")]
#[allow(clippy::type_complexity)]
pub trait ValidateRequest<F, R, D> {
    fn validate_request<'b>(
        &'b self,
        request: &R,
        merchant_account: &domain::MerchantAccount,
    ) -> RouterResult<(BoxedOperation<'b, F, R, D>, ValidateResult)>;
}

#[cfg(feature = "v2")]
pub trait ValidateRequest<F, R, D> {
    fn validate_request(
        &self,
        request: &R,
        merchant_account: &domain::MerchantAccount,
    ) -> RouterResult<ValidateResult>;
}

#[cfg(feature = "v2")]
pub struct GetTrackerResponse<D> {
    pub payment_data: D,
}

#[cfg(feature = "v1")]
pub struct GetTrackerResponse<'a, F: Clone, R, D> {
    pub operation: BoxedOperation<'a, F, R, D>,
    pub customer_details: Option<CustomerDetails>,
    pub payment_data: D,
    pub business_profile: domain::Profile,
    pub mandate_type: Option<api::MandateTransactionType>,
}

/// This trait is used to fetch / create all the tracker related information for a payment
/// This functions returns the session data that is used by subsequent functions
#[async_trait]
pub trait GetTracker<F: Clone, D, R>: Send {
    #[cfg(feature = "v1")]
    #[allow(clippy::too_many_arguments)]
    async fn get_trackers<'a>(
        &'a self,
        state: &'a SessionState,
        payment_id: &api::PaymentIdType,
        request: &R,
        merchant_account: &domain::MerchantAccount,
        mechant_key_store: &domain::MerchantKeyStore,
        auth_flow: services::AuthFlow,
        header_payload: &hyperswitch_domain_models::payments::HeaderPayload,
        platform_merchant_account: Option<&domain::MerchantAccount>,
    ) -> RouterResult<GetTrackerResponse<'a, F, R, D>>;

    #[cfg(feature = "v2")]
    #[allow(clippy::too_many_arguments)]
    async fn get_trackers<'a>(
        &'a self,
        state: &'a SessionState,
        payment_id: &common_utils::id_type::GlobalPaymentId,
        request: &R,
        merchant_account: &domain::MerchantAccount,
        profile: &domain::Profile,
        mechant_key_store: &domain::MerchantKeyStore,
        header_payload: &hyperswitch_domain_models::payments::HeaderPayload,
        platform_merchant_account: Option<&domain::MerchantAccount>,
    ) -> RouterResult<GetTrackerResponse<D>>;
}

#[async_trait]
pub trait Domain<F: Clone, R, D>: Send + Sync {
    #[cfg(feature = "v1")]
    /// This will fetch customer details, (this operation is flow specific)
    async fn get_or_create_customer_details<'a>(
        &'a self,
        state: &SessionState,
        payment_data: &mut D,
        request: Option<CustomerDetails>,
        merchant_key_store: &domain::MerchantKeyStore,
        storage_scheme: enums::MerchantStorageScheme,
    ) -> CustomResult<(BoxedOperation<'a, F, R, D>, Option<domain::Customer>), errors::StorageError>;

    #[cfg(feature = "v2")]
    /// This will fetch customer details, (this operation is flow specific)
    async fn get_customer_details<'a>(
        &'a self,
        state: &SessionState,
        payment_data: &mut D,
        merchant_key_store: &domain::MerchantKeyStore,
        storage_scheme: enums::MerchantStorageScheme,
    ) -> CustomResult<(BoxedOperation<'a, F, R, D>, Option<domain::Customer>), errors::StorageError>;

    #[cfg(feature = "v2")]
    /// This will run the decision manager for the payment
    async fn run_decision_manager<'a>(
        &'a self,
        state: &SessionState,
        payment_data: &mut D,
        business_profile: &domain::Profile,
    ) -> CustomResult<(), errors::ApiErrorResponse> {
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    async fn make_pm_data<'a>(
        &'a self,
        state: &'a SessionState,
        payment_data: &mut D,
        storage_scheme: enums::MerchantStorageScheme,
        merchant_key_store: &domain::MerchantKeyStore,
        customer: &Option<domain::Customer>,
        business_profile: &domain::Profile,
    ) -> RouterResult<(
        BoxedOperation<'a, F, R, D>,
        Option<domain::PaymentMethodData>,
        Option<String>,
    )>;

    async fn add_task_to_process_tracker<'a>(
        &'a self,
        _db: &'a SessionState,
        _payment_attempt: &storage::PaymentAttempt,
        _requeue: bool,
        _schedule_time: Option<time::PrimitiveDateTime>,
    ) -> CustomResult<(), errors::ApiErrorResponse> {
        Ok(())
    }

    #[cfg(feature = "v1")]
    async fn get_connector<'a>(
        &'a self,
        merchant_account: &domain::MerchantAccount,
        state: &SessionState,
        request: &R,
        payment_intent: &storage::PaymentIntent,
        mechant_key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<api::ConnectorChoice, errors::ApiErrorResponse>;

    #[cfg(feature = "v2")]
    async fn perform_routing<'a>(
        &'a self,
        merchant_account: &domain::MerchantAccount,
        business_profile: &domain::Profile,
        state: &SessionState,
        // TODO: do not take the whole payment data here
        payment_data: &mut D,
        mechant_key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<ConnectorCallType, errors::ApiErrorResponse>;

    async fn populate_payment_data<'a>(
        &'a self,
        _state: &SessionState,
        _payment_data: &mut D,
        _merchant_account: &domain::MerchantAccount,
    ) -> CustomResult<(), errors::ApiErrorResponse> {
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    async fn call_external_three_ds_authentication_if_eligible<'a>(
        &'a self,
        _state: &SessionState,
        _payment_data: &mut D,
        _should_continue_confirm_transaction: &mut bool,
        _connector_call_type: &ConnectorCallType,
        _merchant_account: &domain::Profile,
        _key_store: &domain::MerchantKeyStore,
        _mandate_type: Option<api_models::payments::MandateTransactionType>,
    ) -> CustomResult<(), errors::ApiErrorResponse> {
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    async fn call_unified_authentication_service_if_eligible<'a>(
        &'a self,
        _state: &SessionState,
        _payment_data: &mut D,
        _should_continue_confirm_transaction: &mut bool,
        _connector_call_type: &ConnectorCallType,
        _merchant_account: &domain::Profile,
        _key_store: &domain::MerchantKeyStore,
        _mandate_type: Option<api_models::payments::MandateTransactionType>,
    ) -> CustomResult<(), errors::ApiErrorResponse> {
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    async fn payments_dynamic_tax_calculation<'a>(
        &'a self,
        _state: &SessionState,
        _payment_data: &mut D,
        _connector_call_type: &ConnectorCallType,
        _business_profile: &domain::Profile,
        _key_store: &domain::MerchantKeyStore,
        _merchant_account: &domain::MerchantAccount,
    ) -> CustomResult<(), errors::ApiErrorResponse> {
        Ok(())
    }

    #[instrument(skip_all)]
    async fn guard_payment_against_blocklist<'a>(
        &'a self,
        _state: &SessionState,
        _merchant_account: &domain::MerchantAccount,
        _key_store: &domain::MerchantKeyStore,
        _payment_data: &mut D,
    ) -> CustomResult<bool, errors::ApiErrorResponse> {
        Ok(false)
    }

    async fn store_extended_card_info_temporarily<'a>(
        &'a self,
        _state: &SessionState,
        _payment_id: &common_utils::id_type::PaymentId,
        _business_profile: &domain::Profile,
        _payment_method_data: Option<&domain::PaymentMethodData>,
    ) -> CustomResult<(), errors::ApiErrorResponse> {
        Ok(())
    }

    // #[cfg(feature = "v2")]
    // async fn call_connector<'a, RouterDataReq>(
    //     &'a self,
    //     _state: &SessionState,
    //     _req_state: ReqState,
    //     _merchant_account: &domain::MerchantAccount,
    //     _key_store: &domain::MerchantKeyStore,
    //     _business_profile: &domain::Profile,
    //     _payment_method_data: Option<&domain::PaymentMethodData>,
    //     _connector: api::ConnectorData,
    //     _customer: &Option<domain::Customer>,
    //     _payment_data: &mut D,
    //     _call_connector_action: common_enums::CallConnectorAction,
    // ) -> CustomResult<
    //     hyperswitch_domain_models::router_data::RouterData<F, RouterDataReq, PaymentsResponseData>,
    //     errors::ApiErrorResponse,
    // > {
    //     // TODO: raise an error here
    //     todo!();
    // }
}

#[async_trait]
#[allow(clippy::too_many_arguments)]
pub trait UpdateTracker<F, D, Req>: Send {
    /// Update the tracker information with the new data from request or calculated by the operations performed after get trackers
    /// This will persist the SessionData ( PaymentData ) in the database
    ///
    /// In case we are calling a processor / connector, we persist all the data in the database and then call the connector
    async fn update_trackers<'b>(
        &'b self,
        db: &'b SessionState,
        req_state: ReqState,
        payment_data: D,
        customer: Option<domain::Customer>,
        storage_scheme: enums::MerchantStorageScheme,
        updated_customer: Option<storage::CustomerUpdate>,
        mechant_key_store: &domain::MerchantKeyStore,
        frm_suggestion: Option<FrmSuggestion>,
        header_payload: hyperswitch_domain_models::payments::HeaderPayload,
    ) -> RouterResult<(BoxedOperation<'b, F, Req, D>, D)>
    where
        F: 'b + Send;
}

#[cfg(feature = "v2")]
#[async_trait]
#[allow(clippy::too_many_arguments)]
pub trait CallConnector<F, D, RouterDReq: Send>: Send {
    async fn call_connector<'b>(
        &'b self,
        db: &'b SessionState,
        req_state: ReqState,
        payment_data: D,
        key_store: &domain::MerchantKeyStore,
        call_connector_action: common_enums::CallConnectorAction,
        connector_data: api::ConnectorData,
        storage_scheme: enums::MerchantStorageScheme,
    ) -> RouterResult<types::RouterData<F, RouterDReq, PaymentsResponseData>>
    where
        F: 'b + Send + Sync,
        D: super::flows::ConstructFlowSpecificData<F, RouterDReq, PaymentsResponseData>,
        types::RouterData<F, RouterDReq, PaymentsResponseData>:
            super::flows::Feature<F, RouterDReq> + Send;
}

#[async_trait]
#[allow(clippy::too_many_arguments)]
pub trait PostUpdateTracker<F, D, R: Send>: Send {
    /// Update the tracker information with the response from the connector
    /// The response from routerdata is used to update paymentdata and also persist this in the database
    #[cfg(feature = "v1")]
    async fn update_tracker<'b>(
        &'b self,
        db: &'b SessionState,
        payment_data: D,
        response: types::RouterData<F, R, PaymentsResponseData>,
        key_store: &domain::MerchantKeyStore,
        storage_scheme: enums::MerchantStorageScheme,
        locale: &Option<String>,
        #[cfg(feature = "dynamic_routing")] routable_connector: Vec<RoutableConnectorChoice>,
        #[cfg(feature = "dynamic_routing")] business_profile: &domain::Profile,
    ) -> RouterResult<D>
    where
        F: 'b + Send + Sync;

    #[cfg(feature = "v2")]
    async fn update_tracker<'b>(
        &'b self,
        db: &'b SessionState,
        payment_data: D,
        response: types::RouterData<F, R, PaymentsResponseData>,
        key_store: &domain::MerchantKeyStore,
        storage_scheme: enums::MerchantStorageScheme,
    ) -> RouterResult<D>
    where
        F: 'b + Send + Sync,
        types::RouterData<F, R, PaymentsResponseData>:
            hyperswitch_domain_models::router_data::TrackerPostUpdateObjects<F, R, D>;

    async fn save_pm_and_mandate<'b>(
        &self,
        _state: &SessionState,
        _resp: &types::RouterData<F, R, PaymentsResponseData>,
        _merchant_account: &domain::MerchantAccount,
        _key_store: &domain::MerchantKeyStore,
        _payment_data: &mut D,
        _business_profile: &domain::Profile,
    ) -> CustomResult<(), errors::ApiErrorResponse>
    where
        F: 'b + Clone + Send + Sync,
    {
        Ok(())
    }
}

#[cfg(feature = "v1")]
#[async_trait]
impl<
        D,
        F: Clone + Send,
        Op: Send + Sync + Operation<F, api::PaymentsRetrieveRequest, Data = D>,
    > Domain<F, api::PaymentsRetrieveRequest, D> for Op
where
    for<'a> &'a Op: Operation<F, api::PaymentsRetrieveRequest, Data = D>,
    D: OperationSessionGetters<F> + OperationSessionSetters<F> + Send,
{
    #[instrument(skip_all)]
    #[cfg(feature = "v1")]
    async fn get_or_create_customer_details<'a>(
        &'a self,
        state: &SessionState,
        payment_data: &mut D,
        _request: Option<CustomerDetails>,
        merchant_key_store: &domain::MerchantKeyStore,
        storage_scheme: enums::MerchantStorageScheme,
    ) -> CustomResult<
        (
            BoxedOperation<'a, F, api::PaymentsRetrieveRequest, D>,
            Option<domain::Customer>,
        ),
        errors::StorageError,
    > {
        let db = &*state.store;
        let customer = match payment_data.get_payment_intent().customer_id.as_ref() {
            None => None,
            Some(customer_id) => {
                // This function is to retrieve customer details. If the customer is deleted, it returns
                // customer details that contains the fields as Redacted
                db.find_customer_optional_with_redacted_customer_details_by_customer_id_merchant_id(
                    &state.into(),
                    customer_id,
                    &merchant_key_store.merchant_id,
                    merchant_key_store,
                    storage_scheme,
                )
                .await?
            }
        };

        if let Some(email) = customer.as_ref().and_then(|inner| inner.email.clone()) {
            payment_data.set_email_if_not_present(email.into());
        }

        Ok((Box::new(self), customer))
    }

    async fn get_connector<'a>(
        &'a self,
        _merchant_account: &domain::MerchantAccount,
        state: &SessionState,
        _request: &api::PaymentsRetrieveRequest,
        _payment_intent: &storage::PaymentIntent,
        _merchant_key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<api::ConnectorChoice, errors::ApiErrorResponse> {
        helpers::get_connector_default(state, None).await
    }

    #[instrument(skip_all)]
    async fn make_pm_data<'a>(
        &'a self,
        _state: &'a SessionState,
        _payment_data: &mut D,
        _storage_scheme: enums::MerchantStorageScheme,
        _merchant_key_store: &domain::MerchantKeyStore,
        _customer: &Option<domain::Customer>,
        _business_profile: &domain::Profile,
    ) -> RouterResult<(
        BoxedOperation<'a, F, api::PaymentsRetrieveRequest, D>,
        Option<domain::PaymentMethodData>,
        Option<String>,
    )> {
        Ok((Box::new(self), None, None))
    }

    #[instrument(skip_all)]
    async fn guard_payment_against_blocklist<'a>(
        &'a self,
        _state: &SessionState,
        _merchant_account: &domain::MerchantAccount,
        _key_store: &domain::MerchantKeyStore,
        _payment_data: &mut D,
    ) -> CustomResult<bool, errors::ApiErrorResponse> {
        Ok(false)
    }
}

#[cfg(feature = "v1")]
#[async_trait]
impl<D, F: Clone + Send, Op: Send + Sync + Operation<F, api::PaymentsCaptureRequest, Data = D>>
    Domain<F, api::PaymentsCaptureRequest, D> for Op
where
    for<'a> &'a Op: Operation<F, api::PaymentsCaptureRequest, Data = D>,
    D: OperationSessionGetters<F> + OperationSessionSetters<F> + Send,
{
    #[instrument(skip_all)]
    #[cfg(feature = "v1")]
    async fn get_or_create_customer_details<'a>(
        &'a self,
        state: &SessionState,
        payment_data: &mut D,
        _request: Option<CustomerDetails>,
        merchant_key_store: &domain::MerchantKeyStore,
        storage_scheme: enums::MerchantStorageScheme,
    ) -> CustomResult<
        (
            BoxedOperation<'a, F, api::PaymentsCaptureRequest, D>,
            Option<domain::Customer>,
        ),
        errors::StorageError,
    > {
        let db = &*state.store;

        let customer = match payment_data.get_payment_intent().customer_id.as_ref() {
            None => None,
            Some(customer_id) => {
                db.find_customer_optional_by_customer_id_merchant_id(
                    &state.into(),
                    customer_id,
                    &merchant_key_store.merchant_id,
                    merchant_key_store,
                    storage_scheme,
                )
                .await?
            }
        };

        if let Some(email) = customer.as_ref().and_then(|inner| inner.email.clone()) {
            payment_data.set_email_if_not_present(email.into());
        }

        Ok((Box::new(self), customer))
    }

    #[instrument(skip_all)]
    #[cfg(feature = "v2")]
    async fn get_customer_details<'a>(
        &'a self,
        _state: &SessionState,
        _payment_data: &mut D,
        _merchant_key_store: &domain::MerchantKeyStore,
        _storage_scheme: enums::MerchantStorageScheme,
    ) -> CustomResult<
        (
            BoxedOperation<'a, F, api::PaymentsCaptureRequest, D>,
            Option<domain::Customer>,
        ),
        errors::StorageError,
    > {
        todo!()
    }

    #[instrument(skip_all)]
    async fn make_pm_data<'a>(
        &'a self,
        _state: &'a SessionState,
        _payment_data: &mut D,
        _storage_scheme: enums::MerchantStorageScheme,
        _merchant_key_store: &domain::MerchantKeyStore,
        _customer: &Option<domain::Customer>,
        _business_profile: &domain::Profile,
    ) -> RouterResult<(
        BoxedOperation<'a, F, api::PaymentsCaptureRequest, D>,
        Option<domain::PaymentMethodData>,
        Option<String>,
    )> {
        Ok((Box::new(self), None, None))
    }

    async fn get_connector<'a>(
        &'a self,
        _merchant_account: &domain::MerchantAccount,
        state: &SessionState,
        _request: &api::PaymentsCaptureRequest,
        _payment_intent: &storage::PaymentIntent,
        _merchant_key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<api::ConnectorChoice, errors::ApiErrorResponse> {
        helpers::get_connector_default(state, None).await
    }

    #[instrument(skip_all)]
    async fn guard_payment_against_blocklist<'a>(
        &'a self,
        _state: &SessionState,
        _merchant_account: &domain::MerchantAccount,
        _key_store: &domain::MerchantKeyStore,
        _payment_data: &mut D,
    ) -> CustomResult<bool, errors::ApiErrorResponse> {
        Ok(false)
    }
}

#[cfg(feature = "v1")]
#[async_trait]
impl<D, F: Clone + Send, Op: Send + Sync + Operation<F, api::PaymentsCancelRequest, Data = D>>
    Domain<F, api::PaymentsCancelRequest, D> for Op
where
    for<'a> &'a Op: Operation<F, api::PaymentsCancelRequest, Data = D>,
    D: OperationSessionGetters<F> + OperationSessionSetters<F> + Send,
{
    #[instrument(skip_all)]
    #[cfg(feature = "v1")]
    async fn get_or_create_customer_details<'a>(
        &'a self,
        state: &SessionState,
        payment_data: &mut D,
        _request: Option<CustomerDetails>,
        merchant_key_store: &domain::MerchantKeyStore,
        storage_scheme: enums::MerchantStorageScheme,
    ) -> CustomResult<
        (
            BoxedOperation<'a, F, api::PaymentsCancelRequest, D>,
            Option<domain::Customer>,
        ),
        errors::StorageError,
    > {
        let db = &*state.store;

        let customer = match payment_data.get_payment_intent().customer_id.as_ref() {
            None => None,
            Some(customer_id) => {
                db.find_customer_optional_by_customer_id_merchant_id(
                    &state.into(),
                    customer_id,
                    &merchant_key_store.merchant_id,
                    merchant_key_store,
                    storage_scheme,
                )
                .await?
            }
        };

        if let Some(email) = customer.as_ref().and_then(|inner| inner.email.clone()) {
            payment_data.set_email_if_not_present(email.into());
        }

        Ok((Box::new(self), customer))
    }

    #[instrument(skip_all)]
    #[cfg(feature = "v2")]
    async fn get_customer_details<'a>(
        &'a self,
        _state: &SessionState,
        _payment_data: &mut D,
        _merchant_key_store: &domain::MerchantKeyStore,
        storage_scheme: enums::MerchantStorageScheme,
    ) -> CustomResult<
        (
            BoxedOperation<'a, F, api::PaymentsCancelRequest, D>,
            Option<domain::Customer>,
        ),
        errors::StorageError,
    > {
        todo!()
    }

    #[instrument(skip_all)]
    async fn make_pm_data<'a>(
        &'a self,
        _state: &'a SessionState,
        _payment_data: &mut D,
        _storage_scheme: enums::MerchantStorageScheme,
        _merchant_key_store: &domain::MerchantKeyStore,
        _customer: &Option<domain::Customer>,
        _business_profile: &domain::Profile,
    ) -> RouterResult<(
        BoxedOperation<'a, F, api::PaymentsCancelRequest, D>,
        Option<domain::PaymentMethodData>,
        Option<String>,
    )> {
        Ok((Box::new(self), None, None))
    }

    async fn get_connector<'a>(
        &'a self,
        _merchant_account: &domain::MerchantAccount,
        state: &SessionState,
        _request: &api::PaymentsCancelRequest,
        _payment_intent: &storage::PaymentIntent,
        _merchant_key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<api::ConnectorChoice, errors::ApiErrorResponse> {
        helpers::get_connector_default(state, None).await
    }

    #[instrument(skip_all)]
    async fn guard_payment_against_blocklist<'a>(
        &'a self,
        _state: &SessionState,
        _merchant_account: &domain::MerchantAccount,
        _key_store: &domain::MerchantKeyStore,
        _payment_data: &mut D,
    ) -> CustomResult<bool, errors::ApiErrorResponse> {
        Ok(false)
    }
}

#[cfg(feature = "v1")]
#[async_trait]
impl<D, F: Clone + Send, Op: Send + Sync + Operation<F, api::PaymentsRejectRequest, Data = D>>
    Domain<F, api::PaymentsRejectRequest, D> for Op
where
    for<'a> &'a Op: Operation<F, api::PaymentsRejectRequest, Data = D>,
{
    #[cfg(feature = "v1")]
    #[instrument(skip_all)]
    async fn get_or_create_customer_details<'a>(
        &'a self,
        _state: &SessionState,
        _payment_data: &mut D,
        _request: Option<CustomerDetails>,
        _merchant_key_store: &domain::MerchantKeyStore,
        _storage_scheme: enums::MerchantStorageScheme,
    ) -> CustomResult<
        (
            BoxedOperation<'a, F, api::PaymentsRejectRequest, D>,
            Option<domain::Customer>,
        ),
        errors::StorageError,
    > {
        Ok((Box::new(self), None))
    }

    #[cfg(feature = "v2")]
    #[instrument(skip_all)]
    async fn get_customer_details<'a>(
        &'a self,
        _state: &SessionState,
        _payment_data: &mut D,
        _merchant_key_store: &domain::MerchantKeyStore,
        _storage_scheme: enums::MerchantStorageScheme,
    ) -> CustomResult<
        (
            BoxedOperation<'a, F, api::PaymentsRejectRequest, D>,
            Option<domain::Customer>,
        ),
        errors::StorageError,
    > {
        Ok((Box::new(self), None))
    }

    #[instrument(skip_all)]
    async fn make_pm_data<'a>(
        &'a self,
        _state: &'a SessionState,
        _payment_data: &mut D,
        _storage_scheme: enums::MerchantStorageScheme,
        _merchant_key_store: &domain::MerchantKeyStore,
        _customer: &Option<domain::Customer>,
        _business_profile: &domain::Profile,
    ) -> RouterResult<(
        BoxedOperation<'a, F, api::PaymentsRejectRequest, D>,
        Option<domain::PaymentMethodData>,
        Option<String>,
    )> {
        Ok((Box::new(self), None, None))
    }

    async fn get_connector<'a>(
        &'a self,
        _merchant_account: &domain::MerchantAccount,
        state: &SessionState,
        _request: &api::PaymentsRejectRequest,
        _payment_intent: &storage::PaymentIntent,
        _merchant_key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<api::ConnectorChoice, errors::ApiErrorResponse> {
        helpers::get_connector_default(state, None).await
    }

    #[instrument(skip_all)]
    async fn guard_payment_against_blocklist<'a>(
        &'a self,
        _state: &SessionState,
        _merchant_account: &domain::MerchantAccount,
        _key_store: &domain::MerchantKeyStore,
        _payment_data: &mut D,
    ) -> CustomResult<bool, errors::ApiErrorResponse> {
        Ok(false)
    }
}

/// Validate if a particular operation can be performed for the given intent status
pub trait ValidateStatusForOperation {
    fn validate_status_for_operation(
        &self,
        intent_status: common_enums::IntentStatus,
    ) -> Result<(), errors::ApiErrorResponse>;
}

/// Should the connector be called for this operation
pub trait ShouldCallConnector {
    fn should_call_connector(
        &self,
        intent_status: common_enums::IntentStatus,
        force_sync: Option<bool>,
    ) -> bool;
}
