use api_models::{enums::FrmSuggestion, payments::PaymentsRetrieveRequest};
use async_trait::async_trait;
use common_utils::ext_traits::AsyncExt;
use error_stack::ResultExt;
use hyperswitch_domain_models::payments::PaymentStatusData;
use router_env::{instrument, tracing};

use super::{Domain, GetTracker, Operation, UpdateTracker, ValidateRequest};
use crate::{
    core::{
        errors::{self, CustomResult, RouterResult, StorageErrorExt},
        payments::{
            helpers,
            operations::{self, ValidateStatusForOperation},
        },
        utils::ValidatePlatformMerchant,
    },
    routes::{app::ReqState, SessionState},
    types::{
        api::{self, ConnectorCallType},
        domain::{self},
        storage::{self, enums as storage_enums},
    },
    utils::OptionExt,
};

#[derive(Debug, Clone, Copy)]
pub struct PaymentGet;

impl ValidateStatusForOperation for PaymentGet {
    /// Validate if the current operation can be performed on the current status of the payment intent
    fn validate_status_for_operation(
        &self,
        _intent_status: common_enums::IntentStatus,
    ) -> Result<(), errors::ApiErrorResponse> {
        Ok(())
    }
}

type BoxedConfirmOperation<'b, F> =
    super::BoxedOperation<'b, F, PaymentsRetrieveRequest, PaymentStatusData<F>>;

// TODO: change the macro to include changes for v2
// TODO: PaymentData in the macro should be an input
impl<F: Send + Clone + Sync> Operation<F, PaymentsRetrieveRequest> for &PaymentGet {
    type Data = PaymentStatusData<F>;
    fn to_validate_request(
        &self,
    ) -> RouterResult<&(dyn ValidateRequest<F, PaymentsRetrieveRequest, Self::Data> + Send + Sync)>
    {
        Ok(*self)
    }
    fn to_get_tracker(
        &self,
    ) -> RouterResult<&(dyn GetTracker<F, Self::Data, PaymentsRetrieveRequest> + Send + Sync)> {
        Ok(*self)
    }
    fn to_domain(&self) -> RouterResult<&(dyn Domain<F, PaymentsRetrieveRequest, Self::Data>)> {
        Ok(*self)
    }
    fn to_update_tracker(
        &self,
    ) -> RouterResult<&(dyn UpdateTracker<F, Self::Data, PaymentsRetrieveRequest> + Send + Sync)>
    {
        Ok(*self)
    }
}
#[automatically_derived]
impl<F: Send + Clone + Sync> Operation<F, PaymentsRetrieveRequest> for PaymentGet {
    type Data = PaymentStatusData<F>;
    fn to_validate_request(
        &self,
    ) -> RouterResult<&(dyn ValidateRequest<F, PaymentsRetrieveRequest, Self::Data> + Send + Sync)>
    {
        Ok(self)
    }
    fn to_get_tracker(
        &self,
    ) -> RouterResult<&(dyn GetTracker<F, Self::Data, PaymentsRetrieveRequest> + Send + Sync)> {
        Ok(self)
    }
    fn to_domain(&self) -> RouterResult<&dyn Domain<F, PaymentsRetrieveRequest, Self::Data>> {
        Ok(self)
    }
    fn to_update_tracker(
        &self,
    ) -> RouterResult<&(dyn UpdateTracker<F, Self::Data, PaymentsRetrieveRequest> + Send + Sync)>
    {
        Ok(self)
    }
}

impl<F: Send + Clone + Sync> ValidateRequest<F, PaymentsRetrieveRequest, PaymentStatusData<F>>
    for PaymentGet
{
    #[instrument(skip_all)]
    fn validate_request<'a, 'b>(
        &'b self,
        _request: &PaymentsRetrieveRequest,
        merchant_account: &'a domain::MerchantAccount,
    ) -> RouterResult<operations::ValidateResult> {
        let validate_result = operations::ValidateResult {
            merchant_id: merchant_account.get_id().to_owned(),
            storage_scheme: merchant_account.storage_scheme,
            requeue: false,
        };

        Ok(validate_result)
    }
}

#[async_trait]
impl<F: Send + Clone + Sync> GetTracker<F, PaymentStatusData<F>, PaymentsRetrieveRequest>
    for PaymentGet
{
    #[instrument(skip_all)]
    async fn get_trackers<'a>(
        &'a self,
        state: &'a SessionState,
        payment_id: &common_utils::id_type::GlobalPaymentId,
        request: &PaymentsRetrieveRequest,
        merchant_account: &domain::MerchantAccount,
        _profile: &domain::Profile,
        key_store: &domain::MerchantKeyStore,
        _header_payload: &hyperswitch_domain_models::payments::HeaderPayload,
        platform_merchant_account: Option<&domain::MerchantAccount>,
    ) -> RouterResult<operations::GetTrackerResponse<PaymentStatusData<F>>> {
        let db = &*state.store;
        let key_manager_state = &state.into();

        let storage_scheme = merchant_account.storage_scheme;

        let payment_intent = db
            .find_payment_intent_by_id(key_manager_state, payment_id, key_store, storage_scheme)
            .await
            .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)?;

        payment_intent
            .validate_platform_merchant(platform_merchant_account.map(|ma| ma.get_id()))?;

        let payment_attempt = payment_intent
            .active_attempt_id
            .as_ref()
            .async_map(|active_attempt| async {
                db.find_payment_attempt_by_id(
                    key_manager_state,
                    key_store,
                    active_attempt,
                    storage_scheme,
                )
                .await
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Could not find payment attempt given the attempt id")
            })
            .await
            .transpose()?;

        let should_sync_with_connector =
            request.force_sync && payment_intent.status.should_force_sync_with_connector();

        // We need the address here to send it in the response
        let payment_address = hyperswitch_domain_models::payment_address::PaymentAddress::new(
            payment_intent
                .shipping_address
                .clone()
                .map(|address| address.into_inner()),
            payment_intent
                .billing_address
                .clone()
                .map(|address| address.into_inner()),
            payment_attempt
                .as_ref()
                .and_then(|payment_attempt| payment_attempt.payment_method_billing_address.as_ref())
                .cloned()
                .map(|address| address.into_inner()),
            Some(true),
        );

        let attempts = match request.expand_attempts {
            true => payment_intent
                .active_attempt_id
                .as_ref()
                .async_map(|active_attempt| async {
                    db.find_payment_attempts_by_payment_intent_id(
                        key_manager_state,
                        payment_id,
                        key_store,
                        storage_scheme,
                    )
                    .await
                    .change_context(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable("Could not find payment attempts for the given the intent id")
                })
                .await
                .transpose()?,
            false => None,
        };

        let payment_data = PaymentStatusData {
            flow: std::marker::PhantomData,
            payment_intent,
            payment_attempt,
            payment_address,
            attempts,
            should_sync_with_connector,
        };

        let get_trackers_response = operations::GetTrackerResponse { payment_data };

        Ok(get_trackers_response)
    }
}

#[async_trait]
impl<F: Clone + Send + Sync> Domain<F, PaymentsRetrieveRequest, PaymentStatusData<F>>
    for PaymentGet
{
    async fn get_customer_details<'a>(
        &'a self,
        state: &SessionState,
        payment_data: &mut PaymentStatusData<F>,
        merchant_key_store: &domain::MerchantKeyStore,
        storage_scheme: storage_enums::MerchantStorageScheme,
    ) -> CustomResult<(BoxedConfirmOperation<'a, F>, Option<domain::Customer>), errors::StorageError>
    {
        match payment_data.payment_intent.customer_id.clone() {
            Some(id) => {
                let customer = state
                    .store
                    .find_customer_by_global_id(
                        &state.into(),
                        &id,
                        &payment_data.payment_intent.merchant_id,
                        merchant_key_store,
                        storage_scheme,
                    )
                    .await?;

                Ok((Box::new(self), Some(customer)))
            }
            None => Ok((Box::new(self), None)),
        }
    }

    #[instrument(skip_all)]
    async fn make_pm_data<'a>(
        &'a self,
        _state: &'a SessionState,
        _payment_data: &mut PaymentStatusData<F>,
        _storage_scheme: storage_enums::MerchantStorageScheme,
        _key_store: &domain::MerchantKeyStore,
        _customer: &Option<domain::Customer>,
        _business_profile: &domain::Profile,
    ) -> RouterResult<(
        BoxedConfirmOperation<'a, F>,
        Option<domain::PaymentMethodData>,
        Option<String>,
    )> {
        Ok((Box::new(self), None, None))
    }

    #[instrument(skip_all)]
    async fn perform_routing<'a>(
        &'a self,
        _merchant_account: &domain::MerchantAccount,
        _business_profile: &domain::Profile,
        state: &SessionState,
        // TODO: do not take the whole payment data here
        payment_data: &mut PaymentStatusData<F>,
        _mechant_key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<ConnectorCallType, errors::ApiErrorResponse> {
        match &payment_data.payment_attempt {
            Some(payment_attempt) if payment_data.should_sync_with_connector => {
                let connector = payment_attempt
                    .connector
                    .as_ref()
                    .get_required_value("connector")
                    .change_context(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable("Connector is none when constructing response")?;

                let merchant_connector_id = payment_attempt
                    .merchant_connector_id
                    .as_ref()
                    .get_required_value("merchant_connector_id")
                    .change_context(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable("Merchant connector id is none when constructing response")?;

                let connector_data = api::ConnectorData::get_connector_by_name(
                    &state.conf.connectors,
                    connector,
                    api::GetToken::Connector,
                    Some(merchant_connector_id.to_owned()),
                )
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Invalid connector name received")?;

                Ok(ConnectorCallType::PreDetermined(connector_data))
            }
            None | Some(_) => Ok(ConnectorCallType::Skip),
        }
    }
}

#[async_trait]
impl<F: Clone + Sync> UpdateTracker<F, PaymentStatusData<F>, PaymentsRetrieveRequest>
    for PaymentGet
{
    #[instrument(skip_all)]
    async fn update_trackers<'b>(
        &'b self,
        _state: &'b SessionState,
        _req_state: ReqState,
        payment_data: PaymentStatusData<F>,
        _customer: Option<domain::Customer>,
        _storage_scheme: storage_enums::MerchantStorageScheme,
        _updated_customer: Option<storage::CustomerUpdate>,
        _key_store: &domain::MerchantKeyStore,
        _frm_suggestion: Option<FrmSuggestion>,
        _header_payload: hyperswitch_domain_models::payments::HeaderPayload,
    ) -> RouterResult<(BoxedConfirmOperation<'b, F>, PaymentStatusData<F>)>
    where
        F: 'b + Send,
    {
        Ok((Box::new(self), payment_data))
    }
}
