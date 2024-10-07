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
        payment_methods::cards::create_encrypted_data,
        payments::{self, helpers, operations, CustomerDetails},
    },
    routes::{app::ReqState, SessionState},
    services,
    types::{
        api, domain,
        storage::{self, enums},
        transformers::ForeignFrom,
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

#[async_trait::async_trait]
pub trait PaymentsCreateIntentBridge {
    async fn create_domain_model_from_request(
        &self,
        state: &SessionState,
        key_store: &domain::MerchantKeyStore,
        payment_id: &common_utils::id_type::GlobalPaymentId,
        merchant_account: &domain::MerchantAccount,
        profile: &domain::Profile,
    ) -> RouterResult<hyperswitch_domain_models::payments::PaymentIntent>;

    fn get_request_incremental_authorization_value(
        &self,
    ) -> RouterResult<common_enums::RequestIncrementalAuthorization>;
}

#[async_trait::async_trait]
impl PaymentsCreateIntentBridge for PaymentsCreateIntentRequest {
    async fn create_domain_model_from_request(
        &self,
        state: &SessionState,
        key_store: &domain::MerchantKeyStore,
        payment_id: &common_utils::id_type::GlobalPaymentId,
        merchant_account: &domain::MerchantAccount,
        profile: &domain::Profile,
    ) -> RouterResult<hyperswitch_domain_models::payments::PaymentIntent> {
        let session_expiry =
            common_utils::date_time::now().saturating_add(time::Duration::seconds(
                self.session_expiry.map(i64::from).unwrap_or(
                    profile
                        .session_expiry
                        .unwrap_or(common_utils::consts::DEFAULT_SESSION_EXPIRY),
                ),
            ));
        let client_secret = common_utils::types::ClientSecret::new(
            payment_id.clone(),
            common_utils::generate_time_ordered_id_without_prefix(),
        );

        // Derivation of directly supplied Billing Address data in our Payment Create Request
        // Encrypting our Billing Address Details to be stored in Payment Intent
        let billing_address = self
            .billing
            .clone()
            .async_map(|billing_details| create_encrypted_data(state, key_store, billing_details))
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
        let shipping_address = self
            .shipping
            .clone()
            .async_map(|shipping_details| create_encrypted_data(state, key_store, shipping_details))
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
        let order_details = self.order_details.clone().map(|order_details| {
            order_details
                .into_iter()
                .map(|order_detail| masking::Secret::new(order_detail))
                .collect()
        });
        Ok(hyperswitch_domain_models::payments::PaymentIntent {
            id: payment_id.clone(),
            merchant_id: merchant_account.get_id().clone(),
            // Intent status would be RequiresPaymentMethod because we are creating a new payment intent
            status: common_enums::IntentStatus::RequiresPaymentMethod,
            amount_details: hyperswitch_domain_models::payments::AmountDetails::foreign_from(
                self.amount_details.clone(),
            ),
            amount_captured: None,
            customer_id: self.customer_id.clone(),
            description: self.description.clone(),
            return_url: self.return_url.clone(),
            metadata: self.metadata.clone(),
            statement_descriptor: self.statement_descriptor.clone(),
            created_at: common_utils::date_time::now(),
            modified_at: common_utils::date_time::now(),
            last_synced: None,
            setup_future_usage: self.setup_future_usage.unwrap_or_default(),
            client_secret,
            active_attempt: None,
            order_details,
            allowed_payment_method_types: self
                .get_allowed_payment_method_types_as_value()
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Error getting allowed payment method types as value")?,
            connector_metadata: self
                .get_connector_metadata_as_value()
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Error getting connector metadata as value")?,
            feature_metadata: self
                .get_feature_metadata_as_value()
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Error getting feature metadata as value")?,
            // Attempt count is 0 in create intent as no attempt is made yet
            attempt_count: 0,
            profile_id: profile.get_id().clone(),
            payment_link_id: None,
            frm_merchant_decision: None,
            updated_by: merchant_account.storage_scheme.to_string(),
            request_incremental_authorization: self
                .get_request_incremental_authorization_value()?,
            // Authorization count is 0 in create intent as no authorization is made yet
            authorization_count: Some(0),
            session_expiry,
            request_external_three_ds_authentication: self
                .request_external_three_ds_authentication
                .clone()
                .unwrap_or_default(),
            frm_metadata: self.frm_metadata.clone(),
            customer_details: None,
            merchant_reference_id: self.merchant_reference_id.clone(),
            billing_address,
            shipping_address,
            capture_method: self.capture_method.unwrap_or_default(),
            authentication_type: self.authentication_type.unwrap_or_default(),
            prerouting_algorithm: None,
            organization_id: merchant_account.organization_id.clone(),
            enable_payment_link: self.payment_link_enabled.clone().unwrap_or_default(),
            apply_mit_exemption: self.apply_mit_exemption.clone().unwrap_or_default(),
            customer_present: self.customer_present.clone().unwrap_or_default(),
            payment_link_config: self
                .payment_link_config
                .clone()
                .map(ForeignFrom::foreign_from),
            routing_algorithm_id: self.routing_algorithm_id.clone(),
        })
    }

    fn get_request_incremental_authorization_value(
        &self,
    ) -> RouterResult<common_enums::RequestIncrementalAuthorization> {
        self.request_incremental_authorization
            .map(|request_incremental_authorization| {
                if request_incremental_authorization == common_enums::RequestIncrementalAuthorization::True {
                    if self.capture_method == Some(common_enums::CaptureMethod::Automatic) {
                        Err(errors::ApiErrorResponse::InvalidRequestData { message: "incremental authorization is not supported when capture_method is automatic".to_owned() })?
                    }
                    Ok(common_enums::RequestIncrementalAuthorization::True)
                } else {
                    Ok(common_enums::RequestIncrementalAuthorization::False)
                }
            })
            .unwrap_or(Ok(common_enums::RequestIncrementalAuthorization::default()))
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
        _auth_flow: services::AuthFlow,
        _header_payload: &api::HeaderPayload,
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

        let profile_id = profile.get_id().clone();

        let profile = db
            .find_business_profile_by_profile_id(&(state).into(), key_store, &profile_id)
            .await
            .to_not_found_response(errors::ApiErrorResponse::ProfileNotFound {
                id: profile_id.get_string_repr().to_owned(),
            })?;

        let payment_intent_domain = request
            .create_domain_model_from_request(
                state,
                key_store,
                payment_id,
                merchant_account,
                &profile,
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
            customer_details: None,
            payment_data,
            mandate_type: None,
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
        state: &'b SessionState,
        _req_state: ReqState,
        payment_data: payments::PaymentIntentData<F>,
        _customer: Option<domain::Customer>,
        storage_scheme: enums::MerchantStorageScheme,
        _updated_customer: Option<storage::CustomerUpdate>,
        key_store: &domain::MerchantKeyStore,
        _frm_suggestion: Option<FrmSuggestion>,
        _header_payload: api::HeaderPayload,
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
        cell_id: &common_utils::id_type::CellId,
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
        _request: Option<CustomerDetails>,
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
