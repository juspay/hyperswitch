use std::marker::PhantomData;

use api_models::{enums::FrmSuggestion, payments::PaymentsCreateIntentRequest};
use async_trait::async_trait;
use common_utils::{
    errors::CustomResult,
    ext_traits::{AsyncExt, ValueExt},
};
use error_stack::ResultExt;
use router_env::{instrument, tracing};

use super::{BoxedOperation, Domain, GetTracker, Operation, UpdateTracker, ValidateRequest};
use crate::{
    core::{
        errors::{self, RouterResult, StorageErrorExt},
        payments::{self, cards::create_encrypted_data, helpers, operations},
    },
    routes::{app::ReqState, SessionState},
    services,
    types::{
        api, domain,
        storage::{self, enums},
    },
};

#[derive(Debug, Clone, Copy)]
pub struct PaymentCreateIntent;

impl<F: Send + Clone> Operation<F, PaymentsCreateIntentRequest> for &PaymentCreateIntent {
    type Data = payments::PaymentIntentData<F>;
    fn to_validate_request(
        &self,
    ) -> RouterResult<
        &(dyn ValidateRequest<F, PaymentsCreateIntentRequest, Self::Data> + Send + Sync),
    > {
        Ok(*self)
    }
    fn to_get_tracker(
        &self,
    ) -> RouterResult<&(dyn GetTracker<F, Self::Data, PaymentsCreateIntentRequest> + Send + Sync)>
    {
        Ok(*self)
    }
    fn to_domain(&self) -> RouterResult<&(dyn Domain<F, PaymentsCreateIntentRequest, Self::Data>)> {
        Ok(*self)
    }
    fn to_update_tracker(
        &self,
    ) -> RouterResult<&(dyn UpdateTracker<F, Self::Data, PaymentsCreateIntentRequest> + Send + Sync)>
    {
        Ok(*self)
    }
}

impl<F: Send + Clone> Operation<F, PaymentsCreateIntentRequest> for PaymentCreateIntent {
    type Data = payments::PaymentIntentData<F>;
    fn to_validate_request(
        &self,
    ) -> RouterResult<
        &(dyn ValidateRequest<F, PaymentsCreateIntentRequest, Self::Data> + Send + Sync),
    > {
        Ok(self)
    }
    fn to_get_tracker(
        &self,
    ) -> RouterResult<&(dyn GetTracker<F, Self::Data, PaymentsCreateIntentRequest> + Send + Sync)>
    {
        Ok(self)
    }
    fn to_domain(&self) -> RouterResult<&dyn Domain<F, PaymentsCreateIntentRequest, Self::Data>> {
        Ok(self)
    }
    fn to_update_tracker(
        &self,
    ) -> RouterResult<&(dyn UpdateTracker<F, Self::Data, PaymentsCreateIntentRequest> + Send + Sync)>
    {
        Ok(self)
    }
}

type PaymentsCreateIntentOperation<'b, F> =
    BoxedOperation<'b, F, PaymentsCreateIntentRequest, payments::PaymentIntentData<F>>;

#[async_trait]
impl<F: Send + Clone> GetTracker<F, payments::PaymentIntentData<F>, PaymentsCreateIntentRequest>
    for PaymentCreateIntent
{
    #[instrument(skip_all)]
    async fn get_trackers<'a>(
        &'a self,
        state: &'a SessionState,
        payment_id: &common_utils::id_type::GlobalPaymentId,
        request: &PaymentsCreateIntentRequest,
        merchant_account: &domain::MerchantAccount,
        profile: &domain::Profile,
        key_store: &domain::MerchantKeyStore,
        _header_payload: &hyperswitch_domain_models::payments::HeaderPayload,
    ) -> RouterResult<
        operations::GetTrackerResponse<
            'a,
            F,
            PaymentsCreateIntentRequest,
            payments::PaymentIntentData<F>,
        >,
    > {
        let db = &*state.store;
        let key_manager_state = &state.into();

        let storage_scheme = merchant_account.storage_scheme;
        // Derivation of directly supplied Billing Address data in our Payment Create Request
        // Encrypting our Billing Address Details to be stored in Payment Intent
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

        // Derivation of directly supplied Shipping Address data in our Payment Create Request
        // Encrypting our Shipping Address Details to be stored in Payment Intent
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
        let payment_intent_domain =
            hyperswitch_domain_models::payments::PaymentIntent::create_domain_model_from_request(
                payment_id,
                merchant_account,
                profile,
                request.clone(),
                billing_address,
                shipping_address,
            )
            .await?;

        let payment_intent = db
            .insert_payment_intent(
                key_manager_state,
                payment_intent_domain,
                key_store,
                storage_scheme,
            )
            .await
            .to_duplicate_response(errors::ApiErrorResponse::GenericDuplicateError {
                message: format!(
                    "Payment Intent with payment_id {} already exists",
                    payment_id.get_string_repr()
                ),
            })
            .attach_printable("failed while inserting new payment intent")?;

        let payment_data = payments::PaymentIntentData {
            flow: PhantomData,
            payment_intent,
        };

        let get_trackers_response = operations::GetTrackerResponse {
            operation: Box::new(self),
            payment_data,
        };

        Ok(get_trackers_response)
    }
}

#[async_trait]
impl<F: Clone> UpdateTracker<F, payments::PaymentIntentData<F>, PaymentsCreateIntentRequest>
    for PaymentCreateIntent
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
        PaymentsCreateIntentOperation<'b, F>,
        payments::PaymentIntentData<F>,
    )>
    where
        F: 'b + Send,
    {
        Ok((Box::new(self), payment_data))
    }
}

impl<F: Send + Clone>
    ValidateRequest<F, PaymentsCreateIntentRequest, payments::PaymentIntentData<F>>
    for PaymentCreateIntent
{
    #[instrument(skip_all)]
    fn validate_request<'a, 'b>(
        &'b self,
        _request: &PaymentsCreateIntentRequest,
        merchant_account: &'a domain::MerchantAccount,
    ) -> RouterResult<(
        PaymentsCreateIntentOperation<'b, F>,
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
impl<F: Clone + Send> Domain<F, PaymentsCreateIntentRequest, payments::PaymentIntentData<F>>
    for PaymentCreateIntent
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
            BoxedOperation<'a, F, PaymentsCreateIntentRequest, payments::PaymentIntentData<F>>,
            Option<domain::Customer>,
        ),
        errors::StorageError,
    > {
        // validate customer_id if sent in the request
        if let Some(id) = payment_data.payment_intent.customer_id.clone() {
            state
                .store
                .find_customer_by_global_id(
                    &state.into(),
                    id.get_string_repr(),
                    &payment_data.payment_intent.merchant_id,
                    merchant_key_store,
                    storage_scheme,
                )
                .await?;
        }
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
        PaymentsCreateIntentOperation<'a, F>,
        Option<domain::PaymentMethodData>,
        Option<String>,
    )> {
        Ok((Box::new(self), None, None))
    }

    async fn get_connector<'a>(
        &'a self,
        _merchant_account: &domain::MerchantAccount,
        state: &SessionState,
        _request: &PaymentsCreateIntentRequest,
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
        _payment_data: &mut payments::PaymentIntentData<F>,
    ) -> CustomResult<bool, errors::ApiErrorResponse> {
        Ok(false)
    }
}
