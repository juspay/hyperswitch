use std::{marker::PhantomData, ops::Deref};

use api_models::enums::FrmSuggestion;
use async_trait::async_trait;
use common_utils::ext_traits::AsyncExt;
use error_stack::ResultExt;
use hyperswitch_domain_models::payment_method_data::ApiPaymentMethodDataMandateIds;
use router_env::{instrument, tracing};

use super::{BoxedOperation, Domain, GetTracker, Operation, UpdateTracker, ValidateRequest};
use crate::{
    core::{
        configs::dimension_state,
        errors::{self, CustomResult, RouterResult, StorageErrorExt},
        payments::{
            self, helpers, multi_currency_processing, operations, types::MultipleCaptureData,
            CustomerDetails,
        },
    },
    events::audit_events::{AuditEvent, AuditEventType},
    routes::{app::ReqState, SessionState},
    services,
    types::{
        self as core_types,
        api::{self, PaymentIdTypeExt},
        domain,
        storage::{self, enums, payment_attempt::PaymentAttemptExt},
    },
    utils::OptionExt,
};

#[derive(Debug, Clone, Copy, router_derive::PaymentOperation)]
#[operation(operations = "all", flow = "capture")]
pub struct PaymentCapture;

type PaymentCaptureOperation<'b, F> =
    BoxedOperation<'b, F, api::PaymentsCaptureRequest, payments::PaymentData<F>>;

#[async_trait]
impl<F: Send + Clone + Sync> GetTracker<F, payments::PaymentData<F>, api::PaymentsCaptureRequest>
    for PaymentCapture
{
    #[instrument(skip_all)]
    async fn get_trackers<'a>(
        &'a self,
        state: &'a SessionState,
        payment_id: &api::PaymentIdType,
        request: &api::PaymentsCaptureRequest,
        platform: &domain::Platform,
        _auth_flow: services::AuthFlow,
        _flow_kind: operations::PaymentFlowKind,
        _header_payload: &hyperswitch_domain_models::payments::HeaderPayload,
        payment_method_fetch_data: operations::PaymentMethodFetchData,
        _dimensions: &dimension_state::DimensionsWithProcessorAndProviderMerchantId,
        _payment_pre_fetched_info: Option<operations::PaymentPreFetchedInformation>,
        profile_id_from_auth_layer: Option<&common_utils::id_type::ProfileId>,
    ) -> RouterResult<
        operations::GetTrackerResponse<
            'a,
            F,
            api::PaymentsCaptureRequest,
            payments::PaymentData<F>,
        >,
    > {
        let db = &*state.store;

        let processor_merchant_id = platform.get_processor().get_processor_merchant_id();
        let storage_scheme = platform.get_processor().get_storage_scheme();
        let (payment_intent, mut payment_attempt, currency, amount);

        let payment_id = payment_id
            .get_payment_intent_id()
            .change_context(errors::ApiErrorResponse::PaymentNotFound)?;

        payment_intent = helpers::find_payment_intent(
            db,
            &payment_id,
            &processor_merchant_id,
            platform.get_processor().get_key_store(),
            storage_scheme,
            profile_id_from_auth_layer,
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)?;

        payment_attempt = db
            .find_payment_attempt_by_payment_id_processor_merchant_id_attempt_id(
                &payment_intent.payment_id,
                &processor_merchant_id,
                payment_intent.active_attempt.get_id().as_str(),
                storage_scheme,
                platform.get_processor().get_key_store(),
            )
            .await
            .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)?;

        payment_attempt
            .amount_to_capture
            .update_value(request.amount_to_capture);

        let capture_method = payment_attempt
            .get_effective_capture_method()
            .get_required_value("capture_method")?;

        helpers::validate_status_with_capture_method(payment_intent.status, capture_method)?;

        if !*payment_attempt
            .is_overcapture_enabled
            .unwrap_or_default()
            .deref()
        {
            helpers::validate_amount_to_capture(
                payment_attempt.amount_capturable.get_amount_as_i64(),
                request
                    .amount_to_capture
                    .map(|capture_amount| capture_amount.get_amount_as_i64()),
            )?;
        }

        helpers::validate_capture_method(capture_method)?;

        let payment_feature_metadata = payment_intent
            .get_optional_feature_metadata()
            .change_context(errors::ApiErrorResponse::InternalServerError)?;
        let capture_connector = payment_attempt
            .connector
            .as_deref()
            .get_required_value("connector")?;
        let multiple_capture_input = if capture_method == enums::CaptureMethod::ManualMultiple {
            let amount_to_capture = request
                .amount_to_capture
                .get_required_value("amount_to_capture")?;

            helpers::validate_amount_to_capture(
                payment_attempt.amount_capturable.get_amount_as_i64(),
                Some(amount_to_capture.get_amount_as_i64()),
            )?;

            let previous_captures = db
                .find_all_captures_by_merchant_id_payment_id_authorized_attempt_id(
                    &payment_attempt.merchant_id,
                    &payment_attempt.payment_id,
                    &payment_attempt.attempt_id,
                    storage_scheme.inner(),
                )
                .await
                .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)?;

            Some((amount_to_capture, previous_captures))
        } else {
            None
        };
        // The route holds the payment lock while loading capture history and validating MCP.
        let previous_captures = multiple_capture_input
            .as_ref()
            .map(|(_, captures)| captures.as_slice())
            .unwrap_or_default();
        let capture_feature_metadata =
            multi_currency_processing::validate_multi_currency_processing_capture_settlement(
                capture_connector,
                payment_feature_metadata.as_ref(),
                request.feature_metadata.as_ref(),
                previous_captures,
            )?;

        let multiple_capture_data =
            if let Some((amount_to_capture, previous_captures)) = multiple_capture_input {
                let capture_new = payment_attempt.make_new_capture(
                    amount_to_capture,
                    enums::CaptureStatus::Started,
                    capture_feature_metadata.clone(),
                )?;
                let capture = db
                    .insert_capture(capture_new, storage_scheme.inner())
                    .await
                    .to_not_found_response(errors::ApiErrorResponse::DuplicatePayment {
                        payment_id: payment_id.clone(),
                    })?;

                Some(MultipleCaptureData::new_for_create(
                    previous_captures,
                    capture,
                ))
            } else {
                None
            };

        currency = payment_attempt.currency.get_required_value("currency")?;

        amount = payment_attempt.get_total_amount().into();

        let shipping_address = helpers::get_address_by_id(
            state,
            payment_intent.shipping_address_id.clone(),
            platform.get_processor().get_key_store(),
            &payment_intent.payment_id,
            &processor_merchant_id,
            platform.get_processor().get_account().storage_scheme,
        )
        .await?;

        let billing_address = helpers::get_address_by_id(
            state,
            payment_intent.billing_address_id.clone(),
            platform.get_processor().get_key_store(),
            &payment_intent.payment_id,
            &processor_merchant_id,
            platform.get_processor().get_account().storage_scheme,
        )
        .await?;

        let payment_method_billing = helpers::get_address_by_id(
            state,
            payment_attempt.payment_method_billing_address_id.clone(),
            platform.get_processor().get_key_store(),
            &payment_intent.payment_id,
            &processor_merchant_id,
            platform.get_processor().get_account().storage_scheme,
        )
        .await?;

        let creds_identifier = request
            .merchant_connector_details
            .as_ref()
            .map(|mcd| mcd.creds_identifier.to_owned());
        request
            .merchant_connector_details
            .to_owned()
            .async_map(|mcd| async {
                helpers::insert_merchant_connector_creds_to_config(
                    db,
                    platform.get_processor(),
                    mcd,
                )
                .await
            })
            .await
            .transpose()?;

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

        let payment_method_data = payment_method_fetch_data
            .payment_method_with_raw_data
            .clone()
            .and_then(|pm| pm.raw_payment_method_data)
            .or(request
                .payment_method_data
                .clone()
                .map(domain::PaymentMethodData::from));

        let payment_data = payments::PaymentData {
            capture_feature_metadata,
            flow: PhantomData,
            previous_db_records: (payment_attempt.clone(), payment_intent.clone()),
            payment_intent,
            payment_attempt,
            currency,
            force_sync: None,
            all_keys_required: None,
            amount,
            mandate_id: request
                .payment_method_data
                .as_ref()
                .and_then(ApiPaymentMethodDataMandateIds::get_mandate_ids),
            mandate_connector: None,
            setup_mandate: None,
            customer_acceptance: None,
            token: None,
            token_data: None,
            address: core_types::PaymentAddress::new(
                shipping_address.as_ref().map(From::from),
                billing_address.as_ref().map(From::from),
                payment_method_billing.as_ref().map(From::from),
                business_profile.use_billing_as_payment_method_billing,
            ),
            confirm: None,
            payment_method_data,
            payment_method_token: None,
            payment_method_info: None,
            refunds: vec![],
            disputes: vec![],
            attempts: None,
            sessions_token: vec![],
            card_cvc: None,
            creds_identifier,
            pm_token: None,
            connector_customer_id: None,
            recurring_mandate_payment_data: None,
            multiple_capture_data,
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
            client_session_id: None,
            vault_session_details: None,
            external_vault_pmd: None,
            update_request_fields: None,
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
impl<F: Clone + Sync> UpdateTracker<F, payments::PaymentData<F>, api::PaymentsCaptureRequest>
    for PaymentCapture
{
    #[instrument(skip_all)]
    async fn update_trackers<'b>(
        &'b self,
        db: &'b SessionState,
        req_state: ReqState,
        processor: &domain::Processor,
        mut payment_data: payments::PaymentData<F>,
        _frm_suggestion: Option<FrmSuggestion>,
        _header_payload: hyperswitch_domain_models::payments::HeaderPayload,
        _dimensions: &dimension_state::DimensionsWithProcessorAndProviderMerchantId,
    ) -> RouterResult<(PaymentCaptureOperation<'b, F>, payments::PaymentData<F>)>
    where
        F: 'b + Send,
    {
        let storage_scheme = processor.get_storage_scheme();
        let merchant_key_store = processor.get_key_store();

        payment_data.payment_attempt = if payment_data.multiple_capture_data.is_some()
            || payment_data.payment_attempt.amount_to_capture.is_some()
        {
            let multiple_capture_count = payment_data
                .multiple_capture_data
                .as_ref()
                .map(|multiple_capture_data| multiple_capture_data.get_captures_count())
                .transpose()?;
            let amount_to_capture = payment_data.payment_attempt.amount_to_capture;
            db.store
                .update_payment_attempt_with_attempt_id(
                    payment_data.payment_attempt,
                    storage::PaymentAttemptUpdate::CaptureUpdate {
                        amount_to_capture,
                        multiple_capture_count,
                        updated_by: storage_scheme.inner().to_string(),
                    },
                    storage_scheme,
                    merchant_key_store,
                )
                .await
                .to_not_found_response(errors::ApiErrorResponse::InternalServerError)?
        } else {
            payment_data.payment_attempt
        };
        let capture_amount = payment_data.payment_attempt.amount_to_capture;
        let multiple_capture_count = payment_data.payment_attempt.multiple_capture_count;
        req_state
            .event_context
            .event(AuditEvent::new(AuditEventType::PaymentCapture {
                capture_amount,
                multiple_capture_count,
            }))
            .with(payment_data.to_event())
            .emit();
        Ok((Box::new(self), payment_data))
    }
}

impl<F: Send + Clone + Sync>
    ValidateRequest<F, api::PaymentsCaptureRequest, payments::PaymentData<F>> for PaymentCapture
{
    #[instrument(skip_all)]
    fn validate_request<'a, 'b>(
        &'b self,
        request: &api::PaymentsCaptureRequest,
        processor: &'a domain::Processor,
    ) -> RouterResult<(PaymentCaptureOperation<'b, F>, operations::ValidateResult)> {
        let request_merchant_id = request.merchant_id.as_ref();
        helpers::validate_merchant_id(processor.get_account().get_id(), request_merchant_id)
            .change_context(errors::ApiErrorResponse::InvalidDataFormat {
                field_name: "merchant_id".into(),
                expected_format: "merchant_id from merchant account".to_string(),
            })?;

        Ok((
            Box::new(self),
            operations::ValidateResult {
                merchant_id: processor.get_account().get_id().to_owned(),
                payment_id: api::PaymentIdType::PaymentIntentId(request.payment_id.to_owned()),
                storage_scheme: processor.get_account().storage_scheme,
                requeue: false,
            },
        ))
    }
}

#[async_trait]
impl<F: Clone + Send + Sync> Domain<F, api::PaymentsCaptureRequest, payments::PaymentData<F>>
    for PaymentCapture
{
    #[instrument(skip_all)]
    #[cfg(feature = "v1")]
    async fn get_or_create_customer_details<'a>(
        &'a self,
        _state: &SessionState,
        _payment_data: &mut payments::PaymentData<F>,
        _request: Option<CustomerDetails>,
        _provider: &domain::Provider,
        _initiator: Option<&domain::Initiator>,
        _dimensions: &dimension_state::DimensionsWithProcessorAndProviderMerchantIdAndProfileId,
        _mandate_type: Option<api::MandateTransactionType>,
    ) -> CustomResult<
        (PaymentCaptureOperation<'a, F>, Option<domain::Customer>),
        errors::StorageError,
    > {
        // We don't need to fetch customer here.
        // Customer details have already been populated in the payment_intent during Confirm
        // The customer returned from this method is only used for updating connector_customer_id
        // which does not happen in case of Capture
        Ok((Box::new(self), None))
    }

    #[instrument(skip_all)]
    #[cfg(feature = "v2")]
    async fn get_customer_details<'a>(
        &'a self,
        _state: &SessionState,
        _payment_data: &mut payments::PaymentData<F>,
        _merchant_key_store: &domain::MerchantKeyStore,
        _storage_scheme: enums::MerchantStorageScheme,
    ) -> CustomResult<
        (PaymentCaptureOperation<'a, F>, Option<domain::Customer>),
        errors::StorageError,
    > {
        todo!()
    }

    #[instrument(skip_all)]
    async fn make_pm_data<'a>(
        &'a self,
        state: &'a SessionState,
        payment_data: &mut payments::PaymentData<F>,
        _storage_scheme: enums::MerchantStorageScheme,
        platform: &domain::Platform,
        _business_profile: &domain::Profile,
        _should_retry_with_pan: bool,
    ) -> RouterResult<(
        PaymentCaptureOperation<'a, F>,
        Option<domain::PaymentMethodData>,
        Option<String>,
    )> {
        let payment_method_data = operations::get_or_fetch_payment_method_data_for_operation(
            state,
            platform,
            &payment_data.payment_attempt,
            payment_data.payment_method_data.clone(),
        )
        .await;

        Ok((Box::new(self), payment_method_data, None))
    }

    async fn get_connector<'a>(
        &'a self,
        _processor: &domain::Processor,
        state: &SessionState,
        _request: &api::PaymentsCaptureRequest,
        _payment_intent: &storage::PaymentIntent,
    ) -> CustomResult<api::ConnectorChoice, errors::ApiErrorResponse> {
        helpers::get_connector_default(state, None).await
    }

    #[instrument(skip_all)]
    async fn add_task_to_process_tracker<'a>(
        &'a self,
        state: &'a SessionState,
        payment_attempt: &storage::PaymentAttempt,
        requeue: bool,
        schedule_time: Option<time::PrimitiveDateTime>,
    ) -> CustomResult<(), errors::ApiErrorResponse> {
        helpers::add_domain_task_to_pt(self, state, payment_attempt, requeue, schedule_time).await
    }

    #[instrument(skip_all)]
    async fn guard_payment_against_blocklist<'a>(
        &'a self,
        _state: &SessionState,
        _processor: &domain::Processor,
        _dimensions: &dimension_state::DimensionsWithProcessorAndProviderMerchantId,
        _payment_data: &mut payments::PaymentData<F>,
        _business_profile: &domain::Profile,
    ) -> CustomResult<bool, errors::ApiErrorResponse> {
        Ok(false)
    }
}
