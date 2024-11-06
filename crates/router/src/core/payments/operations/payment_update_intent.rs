use std::marker::PhantomData;

use api_models::{enums::FrmSuggestion, payments::PaymentsUpdateIntentRequest};
use async_trait::async_trait;
use common_utils::{
    errors::CustomResult,
    ext_traits::{AsyncExt, ValueExt},
    types::MinorUnit,
};
use error_stack::ResultExt;
use hyperswitch_domain_models::payments::{
    payment_intent::PaymentIntentUpdate, AmountDetails, PaymentIntent,
};
use masking::Secret;
use router_env::{instrument, tracing};

use super::{BoxedOperation, Domain, GetTracker, Operation, UpdateTracker, ValidateRequest};
use crate::{
    core::{
        errors::{self, RouterResult},
        payment_methods::cards::create_encrypted_data,
        payments::{self, helpers, operations},
    },
    db::errors::StorageErrorExt,
    routes::{app::ReqState, SessionState},
    types::{
        api, domain,
        storage::{self, enums},
    },
};

#[derive(Debug, Clone, Copy)]
pub struct PaymentUpdateIntent;

impl<F: Send + Clone> Operation<F, PaymentsUpdateIntentRequest> for &PaymentUpdateIntent {
    type Data = payments::PaymentUpdateData<F>;
    fn to_validate_request(
        &self,
    ) -> RouterResult<
        &(dyn ValidateRequest<F, PaymentsUpdateIntentRequest, Self::Data> + Send + Sync),
    > {
        Ok(*self)
    }
    fn to_get_tracker(
        &self,
    ) -> RouterResult<&(dyn GetTracker<F, Self::Data, PaymentsUpdateIntentRequest> + Send + Sync)>
    {
        Ok(*self)
    }
    fn to_domain(&self) -> RouterResult<&(dyn Domain<F, PaymentsUpdateIntentRequest, Self::Data>)> {
        Ok(*self)
    }
    fn to_update_tracker(
        &self,
    ) -> RouterResult<&(dyn UpdateTracker<F, Self::Data, PaymentsUpdateIntentRequest> + Send + Sync)>
    {
        Ok(*self)
    }
}

impl<F: Send + Clone> Operation<F, PaymentsUpdateIntentRequest> for PaymentUpdateIntent {
    type Data = payments::PaymentUpdateData<F>;
    fn to_validate_request(
        &self,
    ) -> RouterResult<
        &(dyn ValidateRequest<F, PaymentsUpdateIntentRequest, Self::Data> + Send + Sync),
    > {
        Ok(self)
    }
    fn to_get_tracker(
        &self,
    ) -> RouterResult<&(dyn GetTracker<F, Self::Data, PaymentsUpdateIntentRequest> + Send + Sync)>
    {
        Ok(self)
    }
    fn to_domain(&self) -> RouterResult<&dyn Domain<F, PaymentsUpdateIntentRequest, Self::Data>> {
        Ok(self)
    }
    fn to_update_tracker(
        &self,
    ) -> RouterResult<&(dyn UpdateTracker<F, Self::Data, PaymentsUpdateIntentRequest> + Send + Sync)>
    {
        Ok(self)
    }
}

type PaymentsUpdateIntentOperation<'b, F> =
    BoxedOperation<'b, F, PaymentsUpdateIntentRequest, payments::PaymentUpdateData<F>>;

#[async_trait]
impl<F: Send + Clone> GetTracker<F, payments::PaymentUpdateData<F>, PaymentsUpdateIntentRequest>
    for PaymentUpdateIntent
{
    #[instrument(skip_all)]
    async fn get_trackers<'a>(
        &'a self,
        state: &'a SessionState,
        payment_id: &common_utils::id_type::GlobalPaymentId,
        request: &PaymentsUpdateIntentRequest,
        merchant_account: &domain::MerchantAccount,
        profile: &domain::Profile,
        key_store: &domain::MerchantKeyStore,
        _header_payload: &hyperswitch_domain_models::payments::HeaderPayload,
    ) -> RouterResult<
        operations::GetTrackerResponse<
            'a,
            F,
            PaymentsUpdateIntentRequest,
            payments::PaymentUpdateData<F>,
        >,
    > {
        let db = &*state.store;
        let key_manager_state = &state.into();
        let storage_scheme = merchant_account.storage_scheme;
        let payment_intent = db
            .find_payment_intent_by_id(key_manager_state, payment_id, key_store, storage_scheme)
            .await
            .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)?;

        // TODO: Use Batch Encryption
        let billing_address = request
            .billing
            .clone()
            .async_map(|billing_details| {
                create_encrypted_data(key_manager_state, key_store, billing_details)
            })
            .await
            .transpose()
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Unable to encrypt billing details")?
            .map(|encrypted_value| {
                encrypted_value.deserialize_inner_value(|value| value.parse_value("Address"))
            })
            .transpose()
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Unable to deserialize decrypted value to Address")?;

        let shipping_address = request
            .shipping
            .clone()
            .async_map(|shipping_details| {
                create_encrypted_data(key_manager_state, key_store, shipping_details)
            })
            .await
            .transpose()
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Unable to encrypt shipping details")?
            .map(|encrypted_value| {
                encrypted_value.deserialize_inner_value(|value| value.parse_value("Address"))
            })
            .transpose()
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Unable to deserialize decrypted value to Address")?;

        let order_details = request
            .order_details
            .clone()
            .map(|order_details| order_details.into_iter().map(Secret::new).collect());

        // TODO: This should most likely be created_time + session_expiry rather than now + session_expiry
        let session_expiry = request.session_expiry.map(|expiry| {
            common_utils::date_time::now()
                .saturating_add(time::Duration::seconds(i64::from(expiry)))
        });

        let payment_intent_update = PaymentIntentUpdate::UpdateIntent {
            amount: request
                .amount_details
                .as_ref()
                .map(|details| MinorUnit::from(details.order_amount())),
            currency: request
                .amount_details
                .as_ref()
                .map(|details| details.currency()),
            merchant_reference_id: request.merchant_reference_id.clone(),
            routing_algorithm_id: request.routing_algorithm_id.clone(),
            capture_method: request.capture_method.clone(),
            authentication_type: request.authentication_type.clone(),
            billing_address,
            shipping_address,
            customer_id: request.customer_id.clone(),
            customer_present: request.customer_present.clone(),
            description: request.description.clone(),
            return_url: request.return_url.clone(),
            setup_future_usage: request.setup_future_usage.clone(),
            apply_mit_exemption: request.apply_mit_exemption.clone(),
            statement_descriptor: request.statement_descriptor.clone(),
            order_details,
            allowed_payment_method_types: request.allowed_payment_method_types.clone(),
            metadata: request.metadata.clone(),
            connector_metadata: request.connector_metadata.clone(),
            feature_metadata: request.feature_metadata.clone(),
            payment_link_enabled: request.payment_link_enabled.clone(),
            payment_link_config: request.payment_link_config.clone(),
            request_incremental_authorization: request.request_incremental_authorization.clone(),
            session_expiry: session_expiry,
            // TODO: Does frm_metadata need more processing?
            frm_metadata: request.frm_metadata.clone(),
            request_external_three_ds_authentication: request
                .request_external_three_ds_authentication
                .clone(),
            updated_by: storage_scheme.to_string(),
        };

        let payment_data = payments::PaymentUpdateData {
            flow: PhantomData,
            payment_intent,
            payment_intent_update,
        };

        let get_trackers_response = operations::GetTrackerResponse {
            operation: Box::new(self),
            payment_data,
        };

        Ok(get_trackers_response)
    }
}

#[async_trait]
impl<F: Clone> UpdateTracker<F, payments::PaymentUpdateData<F>, PaymentsUpdateIntentRequest>
    for PaymentUpdateIntent
{
    #[instrument(skip_all)]
    async fn update_trackers<'b>(
        &'b self,
        state: &'b SessionState,
        _req_state: ReqState,
        payment_data: payments::PaymentUpdateData<F>,
        _customer: Option<domain::Customer>,
        storage_scheme: enums::MerchantStorageScheme,
        _updated_customer: Option<storage::CustomerUpdate>,
        key_store: &domain::MerchantKeyStore,
        _frm_suggestion: Option<FrmSuggestion>,
        _header_payload: hyperswitch_domain_models::payments::HeaderPayload,
    ) -> RouterResult<(
        PaymentsUpdateIntentOperation<'b, F>,
        payments::PaymentUpdateData<F>,
    )>
    where
        F: 'b + Send,
    {
        let db = &*state.store;
        let key_manager_state = &state.into();

        let new_payment_intent = db
            .update_payment_intent(
                key_manager_state,
                payment_data.payment_intent,
                payment_data.payment_intent_update.clone(),
                key_store,
                storage_scheme,
            )
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Could not update Intent")?;

        let payment_data = payments::PaymentUpdateData {
            flow: PhantomData,
            payment_intent: new_payment_intent,
            payment_intent_update: payment_data.payment_intent_update,
        };

        Ok((Box::new(self), payment_data))
    }
}

impl<F: Send + Clone>
    ValidateRequest<F, PaymentsUpdateIntentRequest, payments::PaymentUpdateData<F>>
    for PaymentUpdateIntent
{
    #[instrument(skip_all)]
    fn validate_request<'a, 'b>(
        &'b self,
        _request: &PaymentsUpdateIntentRequest,
        merchant_account: &'a domain::MerchantAccount,
    ) -> RouterResult<(
        PaymentsUpdateIntentOperation<'b, F>,
        operations::ValidateResult,
    )> {
        Ok((
            Box::new(self),
            operations::ValidateResult {
                merchant_id: merchant_account.get_id().to_owned(),
                storage_scheme: merchant_account.storage_scheme,
                requeue: false,
            },
        ))
    }
}

#[async_trait]
impl<F: Clone + Send> Domain<F, PaymentsUpdateIntentRequest, payments::PaymentUpdateData<F>>
    for PaymentUpdateIntent
{
    #[instrument(skip_all)]
    async fn get_customer_details<'a>(
        &'a self,
        state: &SessionState,
        payment_data: &mut payments::PaymentUpdateData<F>,
        merchant_key_store: &domain::MerchantKeyStore,
        storage_scheme: enums::MerchantStorageScheme,
    ) -> CustomResult<
        (
            BoxedOperation<'a, F, PaymentsUpdateIntentRequest, payments::PaymentUpdateData<F>>,
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
        _payment_data: &mut payments::PaymentUpdateData<F>,
        _storage_scheme: enums::MerchantStorageScheme,
        _merchant_key_store: &domain::MerchantKeyStore,
        _customer: &Option<domain::Customer>,
        _business_profile: &domain::Profile,
    ) -> RouterResult<(
        PaymentsUpdateIntentOperation<'a, F>,
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
        payment_data: &mut payments::PaymentUpdateData<F>,
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
        _payment_data: &mut payments::PaymentUpdateData<F>,
    ) -> CustomResult<bool, errors::ApiErrorResponse> {
        Ok(false)
    }
}
