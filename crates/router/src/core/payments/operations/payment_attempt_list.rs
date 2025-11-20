use std::marker::PhantomData;

#[cfg(feature = "v2")]
use api_models::{enums::FrmSuggestion, payments::PaymentAttemptListRequest};
use async_trait::async_trait;
use common_utils::errors::CustomResult;
use router_env::{instrument, tracing};

use super::{BoxedOperation, Domain, GetTracker, Operation, UpdateTracker, ValidateRequest};
use crate::{
    core::{
        errors::{self, RouterResult},
        payments::{self, operations},
    },
    db::errors::StorageErrorExt,
    routes::{app::ReqState, SessionState},
    types::{
        api, domain,
        storage::{self, enums},
    },
};

#[derive(Debug, Clone, Copy)]
pub struct PaymentGetListAttempts;

impl<F: Send + Clone + Sync> Operation<F, PaymentAttemptListRequest> for &PaymentGetListAttempts {
    type Data = payments::PaymentAttemptListData<F>;
    fn to_validate_request(
        &self,
    ) -> RouterResult<&(dyn ValidateRequest<F, PaymentAttemptListRequest, Self::Data> + Send + Sync)>
    {
        Ok(*self)
    }
    fn to_get_tracker(
        &self,
    ) -> RouterResult<&(dyn GetTracker<F, Self::Data, PaymentAttemptListRequest> + Send + Sync)>
    {
        Ok(*self)
    }
    fn to_domain(&self) -> RouterResult<&(dyn Domain<F, PaymentAttemptListRequest, Self::Data>)> {
        Ok(*self)
    }
    fn to_update_tracker(
        &self,
    ) -> RouterResult<&(dyn UpdateTracker<F, Self::Data, PaymentAttemptListRequest> + Send + Sync)>
    {
        Ok(*self)
    }
}

impl<F: Send + Clone + Sync> Operation<F, PaymentAttemptListRequest> for PaymentGetListAttempts {
    type Data = payments::PaymentAttemptListData<F>;
    fn to_validate_request(
        &self,
    ) -> RouterResult<&(dyn ValidateRequest<F, PaymentAttemptListRequest, Self::Data> + Send + Sync)>
    {
        Ok(self)
    }
    fn to_get_tracker(
        &self,
    ) -> RouterResult<&(dyn GetTracker<F, Self::Data, PaymentAttemptListRequest> + Send + Sync)>
    {
        Ok(self)
    }
    fn to_domain(&self) -> RouterResult<&(dyn Domain<F, PaymentAttemptListRequest, Self::Data>)> {
        Ok(self)
    }
    fn to_update_tracker(
        &self,
    ) -> RouterResult<&(dyn UpdateTracker<F, Self::Data, PaymentAttemptListRequest> + Send + Sync)>
    {
        Ok(self)
    }
}

type PaymentAttemptsListOperation<'b, F> =
    BoxedOperation<'b, F, PaymentAttemptListRequest, payments::PaymentAttemptListData<F>>;

#[async_trait]
impl<F: Send + Clone + Sync>
    GetTracker<F, payments::PaymentAttemptListData<F>, PaymentAttemptListRequest>
    for PaymentGetListAttempts
{
    #[instrument(skip_all)]
    async fn get_trackers<'a>(
        &'a self,
        state: &'a SessionState,
        _payment_id: &common_utils::id_type::GlobalPaymentId,
        request: &PaymentAttemptListRequest,
        platform: &domain::Platform,
        _profile: &domain::Profile,
        _header_payload: &hyperswitch_domain_models::payments::HeaderPayload,
    ) -> RouterResult<operations::GetTrackerResponse<payments::PaymentAttemptListData<F>>> {
        let db = &*state.store;
        let storage_scheme = platform.get_processor().get_account().storage_scheme;
        let payment_attempt_list = db
            .find_payment_attempts_by_payment_intent_id(
                &request.payment_intent_id,
                platform.get_processor().get_key_store(),
                storage_scheme,
            )
            .await
            .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)?;

        let payment_data = payments::PaymentAttemptListData {
            flow: PhantomData,
            payment_attempt_list,
        };

        let get_trackers_response = operations::GetTrackerResponse { payment_data };

        Ok(get_trackers_response)
    }
}

#[async_trait]
impl<F: Clone + Sync>
    UpdateTracker<F, payments::PaymentAttemptListData<F>, PaymentAttemptListRequest>
    for PaymentGetListAttempts
{
    #[instrument(skip_all)]
    async fn update_trackers<'b>(
        &'b self,
        _state: &'b SessionState,
        _req_state: ReqState,
        payment_data: payments::PaymentAttemptListData<F>,
        _customer: Option<domain::Customer>,
        _storage_scheme: enums::MerchantStorageScheme,
        _updated_customer: Option<storage::CustomerUpdate>,
        _key_store: &domain::MerchantKeyStore,
        _frm_suggestion: Option<FrmSuggestion>,
        _header_payload: hyperswitch_domain_models::payments::HeaderPayload,
    ) -> RouterResult<(
        PaymentAttemptsListOperation<'b, F>,
        payments::PaymentAttemptListData<F>,
    )>
    where
        F: 'b + Send,
    {
        Ok((Box::new(self), payment_data))
    }
}

impl<F: Send + Clone + Sync>
    ValidateRequest<F, PaymentAttemptListRequest, payments::PaymentAttemptListData<F>>
    for PaymentGetListAttempts
{
    #[instrument(skip_all)]
    fn validate_request<'a, 'b>(
        &'b self,
        _request: &PaymentAttemptListRequest,
        platform: &'a domain::Platform,
    ) -> RouterResult<operations::ValidateResult> {
        Ok(operations::ValidateResult {
            merchant_id: platform.get_processor().get_account().get_id().to_owned(),
            storage_scheme: platform.get_processor().get_account().storage_scheme,
            requeue: false,
        })
    }
}

#[async_trait]
impl<F: Clone + Send + Sync>
    Domain<F, PaymentAttemptListRequest, payments::PaymentAttemptListData<F>>
    for PaymentGetListAttempts
{
    #[instrument(skip_all)]
    async fn get_customer_details<'a>(
        &'a self,
        _state: &SessionState,
        _payment_data: &mut payments::PaymentAttemptListData<F>,
        _merchant_key_store: &domain::MerchantKeyStore,
        _storage_scheme: enums::MerchantStorageScheme,
    ) -> CustomResult<
        (
            BoxedOperation<'a, F, PaymentAttemptListRequest, payments::PaymentAttemptListData<F>>,
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
        _payment_data: &mut payments::PaymentAttemptListData<F>,
        _storage_scheme: enums::MerchantStorageScheme,
        _merchant_key_store: &domain::MerchantKeyStore,
        _customer: &Option<domain::Customer>,
        _business_profile: &domain::Profile,
        _should_retry_with_pan: bool,
    ) -> RouterResult<(
        PaymentAttemptsListOperation<'a, F>,
        Option<domain::PaymentMethodData>,
        Option<String>,
    )> {
        Ok((Box::new(self), None, None))
    }

    #[instrument(skip_all)]
    async fn perform_routing<'a>(
        &'a self,
        _platform: &domain::Platform,
        _business_profile: &domain::Profile,
        _state: &SessionState,
        _payment_data: &mut payments::PaymentAttemptListData<F>,
    ) -> CustomResult<api::ConnectorCallType, errors::ApiErrorResponse> {
        Ok(api::ConnectorCallType::Skip)
    }

    #[instrument(skip_all)]
    async fn guard_payment_against_blocklist<'a>(
        &'a self,
        _state: &SessionState,
        _platform: &domain::Platform,
        _payment_data: &mut payments::PaymentAttemptListData<F>,
    ) -> CustomResult<bool, errors::ApiErrorResponse> {
        Ok(false)
    }
}
