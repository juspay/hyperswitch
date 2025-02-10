use std::marker::PhantomData;

use api_models::{enums::FrmSuggestion, payments::PaymentsGetIntentRequest};
use async_trait::async_trait;
use common_utils::errors::CustomResult;
use router_env::{instrument, tracing};

use super::{BoxedOperation, Domain, GetTracker, Operation, UpdateTracker, ValidateRequest};
use crate::{
    core::{
        errors::{self, RouterResult},
        payments::{self, helpers, operations},
        utils::ValidatePlatformMerchant,
    },
    db::errors::StorageErrorExt,
    routes::{app::ReqState, SessionState},
    types::{
        api, domain,
        storage::{self, enums},
    },
};

#[derive(Debug, Clone, Copy)]
pub struct PaymentGetIntent;

impl<F: Send + Clone + Sync> Operation<F, PaymentsGetIntentRequest> for &PaymentGetIntent {
    type Data = payments::PaymentIntentData<F>;
    fn to_validate_request(
        &self,
    ) -> RouterResult<&(dyn ValidateRequest<F, PaymentsGetIntentRequest, Self::Data> + Send + Sync)>
    {
        Ok(*self)
    }
    fn to_get_tracker(
        &self,
    ) -> RouterResult<&(dyn GetTracker<F, Self::Data, PaymentsGetIntentRequest> + Send + Sync)>
    {
        Ok(*self)
    }
    fn to_domain(&self) -> RouterResult<&(dyn Domain<F, PaymentsGetIntentRequest, Self::Data>)> {
        Ok(*self)
    }
    fn to_update_tracker(
        &self,
    ) -> RouterResult<&(dyn UpdateTracker<F, Self::Data, PaymentsGetIntentRequest> + Send + Sync)>
    {
        Ok(*self)
    }
}

impl<F: Send + Clone + Sync> Operation<F, PaymentsGetIntentRequest> for PaymentGetIntent {
    type Data = payments::PaymentIntentData<F>;
    fn to_validate_request(
        &self,
    ) -> RouterResult<&(dyn ValidateRequest<F, PaymentsGetIntentRequest, Self::Data> + Send + Sync)>
    {
        Ok(self)
    }
    fn to_get_tracker(
        &self,
    ) -> RouterResult<&(dyn GetTracker<F, Self::Data, PaymentsGetIntentRequest> + Send + Sync)>
    {
        Ok(self)
    }
    fn to_domain(&self) -> RouterResult<&dyn Domain<F, PaymentsGetIntentRequest, Self::Data>> {
        Ok(self)
    }
    fn to_update_tracker(
        &self,
    ) -> RouterResult<&(dyn UpdateTracker<F, Self::Data, PaymentsGetIntentRequest> + Send + Sync)>
    {
        Ok(self)
    }
}

type PaymentsGetIntentOperation<'b, F> =
    BoxedOperation<'b, F, PaymentsGetIntentRequest, payments::PaymentIntentData<F>>;

#[async_trait]
impl<F: Send + Clone + Sync> GetTracker<F, payments::PaymentIntentData<F>, PaymentsGetIntentRequest>
    for PaymentGetIntent
{
    #[instrument(skip_all)]
    async fn get_trackers<'a>(
        &'a self,
        state: &'a SessionState,
        _payment_id: &common_utils::id_type::GlobalPaymentId,
        request: &PaymentsGetIntentRequest,
        merchant_account: &domain::MerchantAccount,
        _profile: &domain::Profile,
        key_store: &domain::MerchantKeyStore,
        _header_payload: &hyperswitch_domain_models::payments::HeaderPayload,
        platform_merchant_account: Option<&domain::MerchantAccount>,
    ) -> RouterResult<operations::GetTrackerResponse<payments::PaymentIntentData<F>>> {
        let db = &*state.store;
        let key_manager_state = &state.into();
        let storage_scheme = merchant_account.storage_scheme;
        let payment_intent = db
            .find_payment_intent_by_id(key_manager_state, &request.id, key_store, storage_scheme)
            .await
            .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)?;

        payment_intent
            .validate_platform_merchant(platform_merchant_account.map(|ma| ma.get_id()))?;

        let payment_data = payments::PaymentIntentData {
            flow: PhantomData,
            payment_intent,
            sessions_token: vec![],
        };

        let get_trackers_response = operations::GetTrackerResponse { payment_data };

        Ok(get_trackers_response)
    }
}

#[async_trait]
impl<F: Clone + Sync> UpdateTracker<F, payments::PaymentIntentData<F>, PaymentsGetIntentRequest>
    for PaymentGetIntent
{
    #[instrument(skip_all)]
    async fn update_trackers<'b>(
        &'b self,
        _state: &'b SessionState,
        _req_state: ReqState,
        payment_data: payments::PaymentIntentData<F>,
        _customer: Option<domain::Customer>,
        _storage_scheme: enums::MerchantStorageScheme,
        _updated_customer: Option<storage::CustomerUpdate>,
        _key_store: &domain::MerchantKeyStore,
        _frm_suggestion: Option<FrmSuggestion>,
        _header_payload: hyperswitch_domain_models::payments::HeaderPayload,
    ) -> RouterResult<(
        PaymentsGetIntentOperation<'b, F>,
        payments::PaymentIntentData<F>,
    )>
    where
        F: 'b + Send,
    {
        Ok((Box::new(self), payment_data))
    }
}

impl<F: Send + Clone + Sync>
    ValidateRequest<F, PaymentsGetIntentRequest, payments::PaymentIntentData<F>>
    for PaymentGetIntent
{
    #[instrument(skip_all)]
    fn validate_request<'a, 'b>(
        &'b self,
        _request: &PaymentsGetIntentRequest,
        merchant_account: &'a domain::MerchantAccount,
    ) -> RouterResult<operations::ValidateResult> {
        Ok(operations::ValidateResult {
            merchant_id: merchant_account.get_id().to_owned(),
            storage_scheme: merchant_account.storage_scheme,
            requeue: false,
        })
    }
}

#[async_trait]
impl<F: Clone + Send + Sync> Domain<F, PaymentsGetIntentRequest, payments::PaymentIntentData<F>>
    for PaymentGetIntent
{
    #[instrument(skip_all)]
    async fn get_customer_details<'a>(
        &'a self,
        state: &SessionState,
        payment_data: &mut payments::PaymentIntentData<F>,
        merchant_key_store: &domain::MerchantKeyStore,
        storage_scheme: enums::MerchantStorageScheme,
    ) -> CustomResult<
        (
            BoxedOperation<'a, F, PaymentsGetIntentRequest, payments::PaymentIntentData<F>>,
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
        _payment_data: &mut payments::PaymentIntentData<F>,
        _storage_scheme: enums::MerchantStorageScheme,
        _merchant_key_store: &domain::MerchantKeyStore,
        _customer: &Option<domain::Customer>,
        _business_profile: &domain::Profile,
    ) -> RouterResult<(
        PaymentsGetIntentOperation<'a, F>,
        Option<domain::PaymentMethodData>,
        Option<String>,
    )> {
        Ok((Box::new(self), None, None))
    }

    #[instrument(skip_all)]
    async fn perform_routing<'a>(
        &'a self,
        merchant_account: &domain::MerchantAccount,
        business_profile: &domain::Profile,
        state: &SessionState,
        // TODO: do not take the whole payment data here
        payment_data: &mut payments::PaymentIntentData<F>,
        mechant_key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<api::ConnectorCallType, errors::ApiErrorResponse> {
        Ok(api::ConnectorCallType::Skip)
    }

    #[instrument(skip_all)]
    async fn guard_payment_against_blocklist<'a>(
        &'a self,
        _state: &SessionState,
        _merchant_account: &domain::MerchantAccount,
        _key_store: &domain::MerchantKeyStore,
        _payment_data: &mut payments::PaymentIntentData<F>,
    ) -> CustomResult<bool, errors::ApiErrorResponse> {
        Ok(false)
    }
}
