use std::marker::PhantomData;

use api_models::enums::FrmSuggestion;
use async_trait::async_trait;
use common_utils::types::keymanager::KeyManagerState;
use error_stack::ResultExt;
use masking::PeekInterface;
use router_derive::PaymentOperation;
use router_env::{instrument, tracing};

use super::{BoxedOperation, Domain, GetTracker, Operation, UpdateTracker, ValidateRequest};
use crate::{
    core::{
        errors::{self, RouterResult, StorageErrorExt},
        payments::{self, helpers, operations, PaymentData},
    },
    routes::{app::ReqState, SessionState},
    services,
    types::{
        api::{self, PaymentIdTypeExt},
        domain,
        storage::{self, enums as storage_enums},
    },
    utils::OptionExt,
};

#[derive(Debug, Clone, Copy, PaymentOperation)]
#[operation(operations = "all", flow = "post_session_tokens")]
pub struct PaymentPostSessionTokens;

type PaymentPostSessionTokensOperation<'b, F> =
    BoxedOperation<'b, F, api::PaymentsPostSessionTokensRequest, PaymentData<F>>;

#[async_trait]
impl<F: Send + Clone + Sync> GetTracker<F, PaymentData<F>, api::PaymentsPostSessionTokensRequest>
    for PaymentPostSessionTokens
{
    #[instrument(skip_all)]
    async fn get_trackers<'a>(
        &'a self,
        state: &'a SessionState,
        payment_id: &api::PaymentIdType,
        request: &api::PaymentsPostSessionTokensRequest,
        merchant_account: &domain::MerchantAccount,
        key_store: &domain::MerchantKeyStore,
        _auth_flow: services::AuthFlow,
        _header_payload: &hyperswitch_domain_models::payments::HeaderPayload,
        _platform_merchant_account: Option<&domain::MerchantAccount>,
    ) -> RouterResult<
        operations::GetTrackerResponse<
            'a,
            F,
            api::PaymentsPostSessionTokensRequest,
            PaymentData<F>,
        >,
    > {
        let payment_id = payment_id
            .get_payment_intent_id()
            .change_context(errors::ApiErrorResponse::PaymentNotFound)?;

        let db = &*state.store;
        let key_manager_state: &KeyManagerState = &state.into();
        let merchant_id = merchant_account.get_id();
        let storage_scheme = merchant_account.storage_scheme;
        let payment_intent = db
            .find_payment_intent_by_payment_id_merchant_id(
                &state.into(),
                &payment_id,
                merchant_id,
                key_store,
                storage_scheme,
            )
            .await
            .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)?;

        // TODO (#7195): Add platform merchant account validation once publishable key auth is solved

        helpers::authenticate_client_secret(Some(request.client_secret.peek()), &payment_intent)?;

        let mut payment_attempt = db
            .find_payment_attempt_by_payment_id_merchant_id_attempt_id(
                &payment_intent.payment_id,
                merchant_id,
                payment_intent.active_attempt.get_id().as_str(),
                storage_scheme,
            )
            .await
            .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)?;
        let currency = payment_intent.currency.get_required_value("currency")?;
        let amount = payment_attempt.get_total_amount().into();
        let profile_id = payment_intent
            .profile_id
            .as_ref()
            .get_required_value("profile_id")
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("'profile_id' not set in payment intent")?;

        let business_profile = db
            .find_business_profile_by_profile_id(key_manager_state, key_store, profile_id)
            .await
            .to_not_found_response(errors::ApiErrorResponse::ProfileNotFound {
                id: profile_id.get_string_repr().to_owned(),
            })?;
        payment_attempt.payment_method = Some(request.payment_method);
        payment_attempt.payment_method_type = Some(request.payment_method_type);
        let shipping_address = helpers::get_address_by_id(
            state,
            payment_intent.shipping_address_id.clone(),
            key_store,
            &payment_intent.payment_id,
            merchant_id,
            merchant_account.storage_scheme,
        )
        .await?;

        let billing_address = helpers::get_address_by_id(
            state,
            payment_intent.billing_address_id.clone(),
            key_store,
            &payment_intent.payment_id,
            merchant_id,
            merchant_account.storage_scheme,
        )
        .await?;

        let payment_data = PaymentData {
            flow: PhantomData,
            payment_intent,
            payment_attempt,
            currency,
            amount,
            email: None,
            mandate_id: None,
            mandate_connector: None,
            customer_acceptance: None,
            token: None,
            token_data: None,
            setup_mandate: None,
            address: payments::PaymentAddress::new(
                shipping_address.as_ref().map(From::from),
                billing_address.as_ref().map(From::from),
                None,
                business_profile.use_billing_as_payment_method_billing,
            ),
            confirm: None,
            payment_method_data: None,
            payment_method_info: None,
            force_sync: None,
            refunds: vec![],
            disputes: vec![],
            attempts: None,
            sessions_token: vec![],
            card_cvc: None,
            creds_identifier: None,
            pm_token: None,
            connector_customer_id: None,
            recurring_mandate_payment_data: None,
            ephemeral_key: None,
            multiple_capture_data: None,
            redirect_response: None,
            surcharge_details: None,
            frm_message: None,
            payment_link_data: None,
            incremental_authorization_details: None,
            authorizations: vec![],
            authentication: None,
            recurring_details: None,
            poll_config: None,
            tax_data: None,
            session_id: None,
            service_details: None,
        };
        let get_trackers_response = operations::GetTrackerResponse {
            operation: Box::new(self),
            customer_details: None,
            payment_data,
            business_profile,
            mandate_type: None,
        };

        Ok(get_trackers_response)
    }
}

#[async_trait]
impl<F: Clone + Send + Sync> Domain<F, api::PaymentsPostSessionTokensRequest, PaymentData<F>>
    for PaymentPostSessionTokens
{
    #[instrument(skip_all)]
    async fn get_or_create_customer_details<'a>(
        &'a self,
        _state: &SessionState,
        _payment_data: &mut PaymentData<F>,
        _request: Option<payments::CustomerDetails>,
        _merchant_key_store: &domain::MerchantKeyStore,
        _storage_scheme: storage_enums::MerchantStorageScheme,
    ) -> errors::CustomResult<
        (
            PaymentPostSessionTokensOperation<'a, F>,
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
        _payment_data: &mut PaymentData<F>,
        _storage_scheme: storage_enums::MerchantStorageScheme,
        _merchant_key_store: &domain::MerchantKeyStore,
        _customer: &Option<domain::Customer>,
        _business_profile: &domain::Profile,
    ) -> RouterResult<(
        PaymentPostSessionTokensOperation<'a, F>,
        Option<domain::PaymentMethodData>,
        Option<String>,
    )> {
        Ok((Box::new(self), None, None))
    }

    async fn get_connector<'a>(
        &'a self,
        _merchant_account: &domain::MerchantAccount,
        state: &SessionState,
        _request: &api::PaymentsPostSessionTokensRequest,
        _payment_intent: &storage::PaymentIntent,
        _merchant_key_store: &domain::MerchantKeyStore,
    ) -> errors::CustomResult<api::ConnectorChoice, errors::ApiErrorResponse> {
        helpers::get_connector_default(state, None).await
    }

    #[instrument(skip_all)]
    async fn guard_payment_against_blocklist<'a>(
        &'a self,
        _state: &SessionState,
        _merchant_account: &domain::MerchantAccount,
        _key_store: &domain::MerchantKeyStore,
        _payment_data: &mut PaymentData<F>,
    ) -> errors::CustomResult<bool, errors::ApiErrorResponse> {
        Ok(false)
    }
}

#[async_trait]
impl<F: Clone + Sync> UpdateTracker<F, PaymentData<F>, api::PaymentsPostSessionTokensRequest>
    for PaymentPostSessionTokens
{
    #[instrument(skip_all)]
    async fn update_trackers<'b>(
        &'b self,
        _state: &'b SessionState,
        _req_state: ReqState,
        payment_data: PaymentData<F>,
        _customer: Option<domain::Customer>,
        _storage_scheme: storage_enums::MerchantStorageScheme,
        _updated_customer: Option<storage::CustomerUpdate>,
        _key_store: &domain::MerchantKeyStore,
        _frm_suggestion: Option<FrmSuggestion>,
        _header_payload: hyperswitch_domain_models::payments::HeaderPayload,
    ) -> RouterResult<(PaymentPostSessionTokensOperation<'b, F>, PaymentData<F>)>
    where
        F: 'b + Send,
    {
        Ok((Box::new(self), payment_data))
    }
}

impl<F: Send + Clone + Sync>
    ValidateRequest<F, api::PaymentsPostSessionTokensRequest, PaymentData<F>>
    for PaymentPostSessionTokens
{
    #[instrument(skip_all)]
    fn validate_request<'a, 'b>(
        &'b self,
        request: &api::PaymentsPostSessionTokensRequest,
        merchant_account: &'a domain::MerchantAccount,
    ) -> RouterResult<(
        PaymentPostSessionTokensOperation<'b, F>,
        operations::ValidateResult,
    )> {
        //payment id is already generated and should be sent in the request
        let given_payment_id = request.payment_id.clone();

        Ok((
            Box::new(self),
            operations::ValidateResult {
                merchant_id: merchant_account.get_id().to_owned(),
                payment_id: api::PaymentIdType::PaymentIntentId(given_payment_id),
                storage_scheme: merchant_account.storage_scheme,
                requeue: false,
            },
        ))
    }
}
