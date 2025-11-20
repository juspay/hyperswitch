use std::marker::PhantomData;

use api_models::{enums::FrmSuggestion, payments::PaymentsCreateIntentRequest};
use async_trait::async_trait;
use common_utils::{
    errors::CustomResult,
    ext_traits::Encode,
    types::{authentication, keymanager::ToEncryptable},
};
use error_stack::ResultExt;
use masking::PeekInterface;
use router_env::{instrument, tracing};

use super::{BoxedOperation, Domain, GetTracker, Operation, UpdateTracker, ValidateRequest};
use crate::{
    core::{
        errors::{self, RouterResult, StorageErrorExt},
        payments::{self, helpers, operations},
    },
    routes::{app::ReqState, SessionState},
    types::{
        api,
        domain::{self, types as domain_types},
        storage::{self, enums},
    },
};

#[derive(Debug, Clone, Copy)]
pub struct PaymentIntentCreate;

impl<F: Send + Clone + Sync> Operation<F, PaymentsCreateIntentRequest> for &PaymentIntentCreate {
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

impl<F: Send + Clone + Sync> Operation<F, PaymentsCreateIntentRequest> for PaymentIntentCreate {
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
impl<F: Send + Clone + Sync>
    GetTracker<F, payments::PaymentIntentData<F>, PaymentsCreateIntentRequest>
    for PaymentIntentCreate
{
    #[instrument(skip_all)]
    async fn get_trackers<'a>(
        &'a self,
        state: &'a SessionState,
        payment_id: &common_utils::id_type::GlobalPaymentId,
        request: &PaymentsCreateIntentRequest,
        platform: &domain::Platform,
        profile: &domain::Profile,
        _header_payload: &hyperswitch_domain_models::payments::HeaderPayload,
    ) -> RouterResult<operations::GetTrackerResponse<payments::PaymentIntentData<F>>> {
        let db = &*state.store;
        if let Some(routing_algorithm_id) = request.routing_algorithm_id.as_ref() {
            helpers::validate_routing_id_with_profile_id(
                db,
                routing_algorithm_id,
                profile.get_id(),
            )
            .await?;
        }
        let key_manager_state = &state.into();

        let storage_scheme = platform.get_processor().get_account().storage_scheme;

        let batch_encrypted_data = domain_types::crypto_operation(
            key_manager_state,
            common_utils::type_name!(hyperswitch_domain_models::payments::PaymentIntent),
            domain_types::CryptoOperation::BatchEncrypt(
                hyperswitch_domain_models::payments::FromRequestEncryptablePaymentIntent::to_encryptable(
                    hyperswitch_domain_models::payments::FromRequestEncryptablePaymentIntent {
                        shipping_address: request.shipping.clone().map(|address| address.encode_to_value()).transpose().change_context(errors::ApiErrorResponse::InternalServerError).attach_printable("Failed to encode shipping address")?.map(masking::Secret::new),
                        billing_address: request.billing.clone().map(|address| address.encode_to_value()).transpose().change_context(errors::ApiErrorResponse::InternalServerError).attach_printable("Failed to encode billing address")?.map(masking::Secret::new),
                        customer_details: None,
                    },
                ),
            ),
            common_utils::types::keymanager::Identifier::Merchant(platform.get_processor().get_account().get_id().to_owned()),
            platform.get_processor().get_key_store().key.peek(),
        )
        .await
        .and_then(|val| val.try_into_batchoperation())
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed while encrypting payment intent details".to_string())?;

        let encrypted_data =
             hyperswitch_domain_models::payments::FromRequestEncryptablePaymentIntent::from_encryptable(batch_encrypted_data)
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Failed while encrypting payment intent details")?;

        let payment_intent_domain =
            hyperswitch_domain_models::payments::PaymentIntent::create_domain_model_from_request(
                payment_id,
                platform,
                profile,
                request.clone(),
                encrypted_data,
            )
            .await?;

        let payment_intent = db
            .insert_payment_intent(
                payment_intent_domain,
                platform.get_processor().get_key_store(),
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

        let client_secret = helpers::create_client_secret(
            state,
            platform.get_processor().get_account().get_id(),
            authentication::ResourceId::Payment(payment_id.clone()),
        )
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Unable to create client secret")?;

        let payment_data = payments::PaymentIntentData {
            flow: PhantomData,
            payment_intent,
            client_secret: Some(client_secret.secret),
            sessions_token: vec![],
            vault_session_details: None,
            connector_customer_id: None,
        };

        let get_trackers_response = operations::GetTrackerResponse { payment_data };

        Ok(get_trackers_response)
    }
}

#[async_trait]
impl<F: Clone + Sync> UpdateTracker<F, payments::PaymentIntentData<F>, PaymentsCreateIntentRequest>
    for PaymentIntentCreate
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
    for PaymentIntentCreate
{
    #[instrument(skip_all)]
    fn validate_request<'a, 'b>(
        &'b self,
        _request: &PaymentsCreateIntentRequest,
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
impl<F: Clone + Send + Sync> Domain<F, PaymentsCreateIntentRequest, payments::PaymentIntentData<F>>
    for PaymentIntentCreate
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
                .find_customer_by_global_id(&id, merchant_key_store, storage_scheme)
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
        _should_retry_with_pan: bool,
    ) -> RouterResult<(
        PaymentsCreateIntentOperation<'a, F>,
        Option<domain::PaymentMethodData>,
        Option<String>,
    )> {
        Ok((Box::new(self), None, None))
    }

    #[instrument(skip_all)]
    async fn perform_routing<'a>(
        &'a self,
        platform: &domain::Platform,
        business_profile: &domain::Profile,
        state: &SessionState,
        // TODO: do not take the whole payment data here
        payment_data: &mut payments::PaymentIntentData<F>,
    ) -> CustomResult<api::ConnectorCallType, errors::ApiErrorResponse> {
        Ok(api::ConnectorCallType::Skip)
    }

    #[instrument(skip_all)]
    async fn guard_payment_against_blocklist<'a>(
        &'a self,
        _state: &SessionState,
        _platform: &domain::Platform,
        _payment_data: &mut payments::PaymentIntentData<F>,
    ) -> CustomResult<bool, errors::ApiErrorResponse> {
        Ok(false)
    }
}
