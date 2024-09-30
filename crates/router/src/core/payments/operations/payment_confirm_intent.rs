use async_trait::async_trait;
use router_env::{instrument, tracing};
use tracing_futures::Instrument;

use super::{Domain, GetTracker, Operation, UpdateTracker, ValidateRequest};
use hyperswitch_domain_models::payments::PaymentConfirmData;

use api_models::payments::{HeaderPayload, PaymentsConfirmIntentRequest};

use hyperswitch_domain_models::{
    merchant_account::MerchantAccount, merchant_key_store::MerchantKeyStore,
};

use crate::{
    core::{
        authentication,
        errors::{self, CustomResult, RouterResult, StorageErrorExt},
        payments::{
            self, helpers, operations, populate_surcharge_details, CustomerDetails, PaymentAddress,
            PaymentData,
        },
        utils as core_utils,
    },
    routes::{app::ReqState, SessionState},
    services,
    types::{
        self,
        api::{self, ConnectorCallType, PaymentIdTypeExt},
        domain::{self},
        storage::{self, enums as storage_enums},
    },
    utils::{self, OptionExt},
};

use super::GetTrackerResponse;

#[derive(Debug, Clone, Copy)]
pub struct PaymentsIntentConfirm;

type BoxedConfirmOperation<'b, F> =
    super::BoxedOperation<'b, F, PaymentsConfirmIntentRequest, PaymentConfirmData<F>>;

// TODO: change the macro to include changes for v2
// TODO: PaymentData in the macro should be an input
impl<F: Send + Clone> Operation<F, PaymentsConfirmIntentRequest> for &PaymentsIntentConfirm {
    type Data = PaymentConfirmData<F>;
    fn to_validate_request(
        &self,
    ) -> RouterResult<
        &(dyn ValidateRequest<F, PaymentsConfirmIntentRequest, Self::Data> + Send + Sync),
    > {
        Ok(*self)
    }
    fn to_get_tracker(
        &self,
    ) -> RouterResult<&(dyn GetTracker<F, Self::Data, PaymentsConfirmIntentRequest> + Send + Sync)>
    {
        Ok(*self)
    }
    fn to_domain(
        &self,
    ) -> RouterResult<&(dyn Domain<F, PaymentsConfirmIntentRequest, Self::Data>)> {
        Ok(*self)
    }
    fn to_update_tracker(
        &self,
    ) -> RouterResult<&(dyn UpdateTracker<F, Self::Data, PaymentsConfirmIntentRequest> + Send + Sync)>
    {
        Ok(*self)
    }
}
#[automatically_derived]
impl<F: Send + Clone> Operation<F, PaymentsConfirmIntentRequest> for PaymentsIntentConfirm {
    type Data = PaymentConfirmData<F>;
    fn to_validate_request(
        &self,
    ) -> RouterResult<
        &(dyn ValidateRequest<F, PaymentsConfirmIntentRequest, Self::Data> + Send + Sync),
    > {
        Ok(self)
    }
    fn to_get_tracker(
        &self,
    ) -> RouterResult<&(dyn GetTracker<F, Self::Data, PaymentsConfirmIntentRequest> + Send + Sync)>
    {
        Ok(self)
    }
    fn to_domain(&self) -> RouterResult<&dyn Domain<F, PaymentsConfirmIntentRequest, Self::Data>> {
        Ok(self)
    }
    fn to_update_tracker(
        &self,
    ) -> RouterResult<&(dyn UpdateTracker<F, Self::Data, PaymentsConfirmIntentRequest> + Send + Sync)>
    {
        Ok(self)
    }
}

#[async_trait]
impl<F: Send + Clone> GetTracker<F, PaymentConfirmData<F>, PaymentsConfirmIntentRequest>
    for PaymentsIntentConfirm
{
    #[instrument(skip_all)]
    async fn get_trackers<'a>(
        &'a self,
        state: &'a SessionState,
        payment_id: &common_utils::id_type::GlobalPaymentId,
        request: &PaymentsConfirmIntentRequest,
        merchant_account: &MerchantAccount,
        key_store: &MerchantKeyStore,
        auth_flow: services::AuthFlow,
        header_payload: &HeaderPayload,
    ) -> RouterResult<GetTrackerResponse<'a, F, PaymentsConfirmIntentRequest, PaymentConfirmData<F>>>
    {
        todo!()
    }
}

#[async_trait]
impl<F: Clone + Send> Domain<F, PaymentsConfirmIntentRequest, PaymentConfirmData<F>>
    for PaymentsIntentConfirm
{
    #[instrument(skip_all)]
    async fn get_or_create_customer_details<'a>(
        &'a self,
        state: &SessionState,
        payment_data: &mut PaymentConfirmData<F>,
        request: Option<CustomerDetails>,
        key_store: &MerchantKeyStore,
        storage_scheme: common_enums::enums::MerchantStorageScheme,
    ) -> CustomResult<(BoxedConfirmOperation<'a, F>, Option<domain::Customer>), errors::StorageError>
    {
        // TODO: Modify this trait function appropriately for v2
        // This should always return a customer, the result can be an enum
        // The enum will have two variants
        // CustomerDetails - Actual customer details of recurring customer
        // GuestCustomer - Details of a guest customer

        Ok((Box::new(self), None))
    }

    #[instrument(skip_all)]
    async fn make_pm_data<'a>(
        &'a self,
        state: &'a SessionState,
        payment_data: &mut PaymentConfirmData<F>,
        storage_scheme: storage_enums::MerchantStorageScheme,
        key_store: &MerchantKeyStore,
        customer: &Option<domain::Customer>,
        business_profile: &domain::Profile,
    ) -> RouterResult<(
        BoxedConfirmOperation<'a, F>,
        Option<domain::PaymentMethodData>,
        Option<String>,
    )> {
        Ok((Box::new(self), None, None))
    }

    async fn get_connector<'a>(
        &'a self,
        _merchant_account: &domain::MerchantAccount,
        state: &SessionState,
        request: &PaymentsConfirmIntentRequest,
        _payment_intent: &storage::PaymentIntent,
        _key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<api::ConnectorChoice, errors::ApiErrorResponse> {
        todo!()
    }
}
