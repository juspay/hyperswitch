use std::marker::PhantomData;

use api_models::enums::FrmSuggestion;
use async_trait::async_trait;
use error_stack::ResultExt;
use masking::ExposeInterface;
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
#[operation(operations = "all", flow = "update_metadata")]
pub struct PaymentUpdateMetadata;

type PaymentUpdateMetadataOperation<'b, F> =
    BoxedOperation<'b, F, api::PaymentsUpdateMetadataRequest, PaymentData<F>>;

#[async_trait]
impl<F: Send + Clone + Sync> GetTracker<F, PaymentData<F>, api::PaymentsUpdateMetadataRequest>
    for PaymentUpdateMetadata
{
    #[instrument(skip_all)]
    async fn get_trackers<'a>(
        &'a self,
        state: &'a SessionState,
        payment_id: &api::PaymentIdType,
        request: &api::PaymentsUpdateMetadataRequest,
        platform: &domain::Platform,
        _auth_flow: services::AuthFlow,
        _header_payload: &hyperswitch_domain_models::payments::HeaderPayload,
    ) -> RouterResult<
        operations::GetTrackerResponse<'a, F, api::PaymentsUpdateMetadataRequest, PaymentData<F>>,
    > {
        let payment_id = payment_id
            .get_payment_intent_id()
            .change_context(errors::ApiErrorResponse::PaymentNotFound)?;

        let db = &*state.store;
        let merchant_id = platform.get_processor().get_account().get_id();
        let storage_scheme = platform.get_processor().get_account().storage_scheme;
        let mut payment_intent = db
            .find_payment_intent_by_payment_id_merchant_id(
                &payment_id,
                merchant_id,
                platform.get_processor().get_key_store(),
                storage_scheme,
            )
            .await
            .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)?;

        helpers::validate_payment_status_against_allowed_statuses(
            payment_intent.status,
            &[
                storage_enums::IntentStatus::Succeeded,
                storage_enums::IntentStatus::Failed,
                storage_enums::IntentStatus::PartiallyCaptured,
                storage_enums::IntentStatus::PartiallyCapturedAndCapturable,
                storage_enums::IntentStatus::RequiresCapture,
            ],
            "update_metadata",
        )?;

        let payment_attempt = db
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
            .find_business_profile_by_profile_id(
                platform.get_processor().get_key_store(),
                profile_id,
            )
            .await
            .to_not_found_response(errors::ApiErrorResponse::ProfileNotFound {
                id: profile_id.get_string_repr().to_owned(),
            })?;

        let merged_metadata = payment_intent
            .merge_metadata(request.metadata.clone().expose())
            .change_context(errors::ApiErrorResponse::InvalidRequestData {
                message: "Metadata should be an object and contain at least 1 key".to_owned(),
            })?;

        payment_intent.metadata = Some(merged_metadata);

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
            address: payments::PaymentAddress::new(None, None, None, None),
            confirm: None,
            payment_method_data: None,
            payment_method_token: None,
            payment_method_info: None,
            force_sync: None,
            all_keys_required: None,
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
            card_testing_guard_data: None,
            vault_operation: None,
            threeds_method_comp_ind: None,
            whole_connector_response: None,
            is_manual_retry_enabled: None,
            is_l2_l3_enabled: false,
            external_authentication_data: None,
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
impl<F: Clone + Send + Sync> Domain<F, api::PaymentsUpdateMetadataRequest, PaymentData<F>>
    for PaymentUpdateMetadata
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
            PaymentUpdateMetadataOperation<'a, F>,
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
        _should_retry_with_pan: bool,
    ) -> RouterResult<(
        PaymentUpdateMetadataOperation<'a, F>,
        Option<domain::PaymentMethodData>,
        Option<String>,
    )> {
        Ok((Box::new(self), None, None))
    }

    async fn get_connector<'a>(
        &'a self,
        _platform: &domain::Platform,
        state: &SessionState,
        _request: &api::PaymentsUpdateMetadataRequest,
        _payment_intent: &storage::PaymentIntent,
    ) -> errors::CustomResult<api::ConnectorChoice, errors::ApiErrorResponse> {
        helpers::get_connector_default(state, None).await
    }

    #[instrument(skip_all)]
    async fn guard_payment_against_blocklist<'a>(
        &'a self,
        _state: &SessionState,
        _platform: &domain::Platform,
        _payment_data: &mut PaymentData<F>,
    ) -> errors::CustomResult<bool, errors::ApiErrorResponse> {
        Ok(false)
    }
}

#[async_trait]
impl<F: Clone + Sync> UpdateTracker<F, PaymentData<F>, api::PaymentsUpdateMetadataRequest>
    for PaymentUpdateMetadata
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
    ) -> RouterResult<(PaymentUpdateMetadataOperation<'b, F>, PaymentData<F>)>
    where
        F: 'b + Send,
    {
        Ok((Box::new(self), payment_data))
    }
}

impl<F: Send + Clone + Sync> ValidateRequest<F, api::PaymentsUpdateMetadataRequest, PaymentData<F>>
    for PaymentUpdateMetadata
{
    #[instrument(skip_all)]
    fn validate_request<'a, 'b>(
        &'b self,
        request: &api::PaymentsUpdateMetadataRequest,
        platform: &'a domain::Platform,
    ) -> RouterResult<(
        PaymentUpdateMetadataOperation<'b, F>,
        operations::ValidateResult,
    )> {
        //payment id is already generated and should be sent in the request
        let given_payment_id = request.payment_id.clone();

        Ok((
            Box::new(self),
            operations::ValidateResult {
                merchant_id: platform.get_processor().get_account().get_id().to_owned(),
                payment_id: api::PaymentIdType::PaymentIntentId(given_payment_id),
                storage_scheme: platform.get_processor().get_account().storage_scheme,
                requeue: false,
            },
        ))
    }
}
