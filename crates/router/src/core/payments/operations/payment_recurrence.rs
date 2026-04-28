use std::marker::PhantomData;

use api_models::{enums::FrmSuggestion, payments::GetAddressFromPaymentMethodData};
use async_trait::async_trait;
use common_utils::ext_traits::{AsyncExt, ValueExt};
use error_stack::{report, ResultExt};
use futures::FutureExt;
use hyperswitch_masking::ExposeInterface;
use router_derive::PaymentOperation;
use router_env::{instrument, tracing};
use tracing_futures::Instrument;

use super::{BoxedOperation, Domain, GetTracker, Operation, UpdateTracker, ValidateRequest};
#[cfg(feature = "v1")]
use crate::events::audit_events::{AuditEvent, AuditEventType};
use crate::{
    core::{
        configs::dimension_state,
        errors::{self, CustomResult, RouterResult, StorageErrorExt},
        mandate::helpers as m_helpers,
        payment_methods::transformers as pm_transformers,
        payments::{helpers, operations, CustomerDetails, PaymentAddress, PaymentData},
    },
    routes::{app::ReqState, SessionState},
    services,
    types::{
        api::{self, PaymentIdTypeExt},
        domain::{self},
        storage::{self, enums as storage_enums},
    },
    utils::{self, OptionExt},
};

#[derive(Debug, Clone, Copy, PaymentOperation)]
#[operation(operations = "all", flow = "authorize")]
pub struct PaymentRecurrence;

type PaymentRecurrenceOperation<'b, F> =
    BoxedOperation<'b, F, api::PaymentsRequest, PaymentData<F>>;

#[async_trait]
impl<F: Send + Clone + Sync> GetTracker<F, PaymentData<F>, api::PaymentsRequest>
    for PaymentRecurrence
{
    #[instrument(skip_all)]
    async fn get_trackers<'a>(
        &'a self,
        state: &'a SessionState,
        payment_id: &api::PaymentIdType,
        request: &api::PaymentsRequest,
        platform: &domain::Platform,
        _auth_flow: services::AuthFlow,
        _header_payload: &hyperswitch_domain_models::payments::HeaderPayload,
        payment_method_with_raw_data: Option<pm_transformers::PaymentMethodWithRawData>,
        dimensions: &dimension_state::DimensionsWithProcessorAndProviderMerchantId,
    ) -> RouterResult<operations::GetTrackerResponse<'a, F, api::PaymentsRequest, PaymentData<F>>>
    {
        let processor_merchant_id = platform.get_processor().get_account().get_id();
        let storage_scheme = platform.get_processor().get_account().storage_scheme;
        let (currency, amount);

        let payment_id = payment_id
            .get_payment_intent_id()
            .change_context(errors::ApiErrorResponse::PaymentNotFound)?;

        let store = &*state.store;
        let m_merchant_id = processor_merchant_id.clone();

        let payment_intent = store
            .find_payment_intent_by_payment_id_processor_merchant_id(
                &payment_id,
                &m_merchant_id,
                platform.get_processor().get_key_store(),
                storage_scheme,
            )
            .await
            .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)?;

        helpers::validate_payment_status_against_allowed_statuses(
            payment_intent.status,
            &[storage_enums::IntentStatus::RequiresCustomerAction],
            "recurrence",
        )?;

        let customer_details =
            helpers::get_customer_details_from_request_or_pm_table(request, None, None)?;

        let store = state.store.clone();
        let m_payment_id = payment_intent.payment_id.clone();
        let m_merchant_id = processor_merchant_id.clone();
        let attempt_id = payment_intent.active_attempt.get_id();
        let merchant_key_store = platform.get_processor().get_key_store().clone();

        let payment_attempt = store
            .find_payment_attempt_by_payment_id_processor_merchant_id_attempt_id(
                &m_payment_id,
                &m_merchant_id,
                attempt_id.as_str(),
                storage_scheme,
                &merchant_key_store,
            )
            .await
            .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)?;

        let profile_id = payment_intent
            .profile_id
            .clone()
            .get_required_value("profile_id")
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("'profile_id' not set in payment intent")?;
        let key_store_clone = platform.get_processor().get_key_store().clone();
        let business_profile = store
            .find_business_profile_by_profile_id(&key_store_clone, &profile_id)
            .map(|business_profile_result| {
                business_profile_result.to_not_found_response(
                    errors::ApiErrorResponse::ProfileNotFound {
                        id: profile_id.get_string_repr().to_owned(),
                    },
                )
            })
            .await?;

        let customer_acceptance = request.customer_acceptance.clone().or(payment_attempt
            .customer_acceptance
            .clone()
            .map(|customer_acceptance| {
                customer_acceptance
                    .expose()
                    .parse_value("CustomerAcceptance")
                    .change_context(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable("Failed while deserializing customer_acceptance")
            })
            .transpose()?);

        currency = payment_attempt.currency.get_required_value("currency")?;
        amount = payment_attempt.get_total_amount().into();

        let creds_identifier = request
            .merchant_connector_details
            .as_ref()
            .map(|mcd| mcd.creds_identifier.to_owned());

        let n_request_payment_method_data = request
            .payment_method_data
            .as_ref()
            .and_then(|pmd| pmd.payment_method_data.clone())
            .map(domain::PaymentMethodData::from);

        let store = state.clone().store;
        let superposition_service = state.clone().superposition_service;
        let profile_id = payment_intent
            .profile_id
            .clone()
            .get_required_value("profile_id")
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("'profile_id' not set in payment intent")?;
        let mandate_dimensions = dimensions.with_profile_id(profile_id.clone());

        let additional_pm_data_fut = tokio::spawn(
            async move {
                Ok(n_request_payment_method_data
                    .async_map(|payment_method_data| async move {
                        helpers::get_additional_payment_data(
                            &payment_method_data,
                            store.as_ref(),
                            superposition_service.as_ref(),
                            &mandate_dimensions,
                            None,
                            None,
                        )
                        .await
                    })
                    .await)
            }
            .in_current_span(),
        );

        let session_state = state.clone();
        let m_payment_intent_billing_address_id = payment_intent.billing_address_id.clone();
        let m_key_store = platform.get_processor().get_key_store().clone();
        let m_payment_intent_payment_id = payment_intent.payment_id.clone();
        let m_merchant_id = processor_merchant_id.clone();
        let m_storage_scheme = platform.get_processor().get_account().storage_scheme;

        let payment_method_billing_future = tokio::spawn(
            async move {
                helpers::get_address_by_id(
                    &session_state,
                    m_payment_intent_billing_address_id,
                    &m_key_store,
                    &m_payment_intent_payment_id,
                    &m_merchant_id,
                    m_storage_scheme,
                )
                .await
            }
            .in_current_span(),
        );

        let mandate_type = m_helpers::get_mandate_type(
            request.mandate_data.clone(),
            request.off_session,
            payment_intent.setup_future_usage,
            request.customer_acceptance.clone(),
            request.payment_token.clone(),
            payment_attempt.payment_method.or(request.payment_method),
        )
        .change_context(errors::ApiErrorResponse::MandateValidationFailed {
            reason: "Expected one out of recurring_details and mandate_data but got both".into(),
        })?;

        let m_state = state.clone();
        let m_mandate_type = mandate_type;
        let m_platform = platform.clone();
        let m_request = request.clone();

        let payment_intent_customer_id = payment_intent.customer_id.clone();

        let m_pm_wrapper = payment_method_with_raw_data.clone();
        let mandate_dimensions = dimensions.clone();
        let mandate_details_fut = tokio::spawn(
            async move {
                Box::pin(helpers::get_token_pm_type_mandate_details(
                    &m_state,
                    &m_request,
                    m_mandate_type,
                    &m_platform,
                    None,
                    payment_intent_customer_id.as_ref(),
                    m_pm_wrapper.map(|pm| pm.payment_method.0),
                    &mandate_dimensions,
                ))
                .await
            }
            .in_current_span(),
        );

        let (mandate_details, additional_pm_info, payment_method_billing) = tokio::try_join!(
            utils::flatten_join_error(mandate_details_fut),
            utils::flatten_join_error(additional_pm_data_fut),
            utils::flatten_join_error(payment_method_billing_future),
        )?;

        let setup_mandate = mandate_details.mandate_data.map(|mut sm| {
            sm.mandate_type = payment_attempt.mandate_details.clone().or(sm.mandate_type);
            sm.update_mandate_id = payment_attempt
                .mandate_data
                .clone()
                .and_then(|mandate| mandate.update_mandate_id)
                .or(sm.update_mandate_id);
            sm
        });

        let payment_method_data_from_request =
            request
                .payment_method_data
                .as_ref()
                .and_then(|request_payment_method_data| {
                    request_payment_method_data.payment_method_data.clone()
                });

        let additional_pm_data = additional_pm_info.transpose()?.flatten();
        let payment_method_data = payment_method_with_raw_data
            .clone()
            .and_then(|pm| pm.raw_payment_method_data)
            .or(payment_method_data_from_request.map(Into::into))
            .map(|payment_method_data| {
                if let Some(additional_pm_data) = additional_pm_data {
                    payment_method_data.apply_additional_payment_data(additional_pm_data)
                } else {
                    payment_method_data
                }
            });

        let shipping_address = helpers::get_address_by_id(
            state,
            payment_intent.shipping_address_id.clone(),
            &platform.get_processor().get_key_store().clone(),
            &payment_intent.payment_id.clone(),
            &processor_merchant_id.clone(),
            m_storage_scheme,
        )
        .await?;

        let billing_address = helpers::get_address_by_id(
            state,
            payment_intent.billing_address_id.clone(),
            &platform.get_processor().get_key_store().clone(),
            &payment_intent.payment_id.clone(),
            &processor_merchant_id.clone(),
            m_storage_scheme,
        )
        .await?;

        let address = PaymentAddress::new(
            shipping_address.as_ref().map(From::from),
            billing_address.as_ref().map(From::from),
            payment_method_billing.as_ref().map(From::from),
            business_profile.use_billing_as_payment_method_billing,
        );

        let payment_method_data_billing = request
            .payment_method_data
            .as_ref()
            .and_then(|pmd| pmd.payment_method_data.as_ref())
            .and_then(|payment_method_data_billing| {
                payment_method_data_billing.get_billing_address()
            })
            .map(From::from);
        let pm_pmd_billing = payment_method_with_raw_data.as_ref().and_then(|pm| {
            pm.payment_method
                .0
                .payment_method_billing_address
                .clone()
                .and_then(|decrypted_data| {
                    let exposed = decrypted_data.into_inner().expose();
                    match exposed.parse_value::<
                        hyperswitch_domain_models::address::Address,
                    >("payment method billing address") {
                        Ok(address) => Some(address),
                        Err(err) => {
                            router_env::logger::error!(error = ?err, "Failed to parse payment method billing address");
                            None
                        }
                    }
                })
        });

        let pmd_address = payment_method_data_billing.or(pm_pmd_billing);

        let unified_address = address.unify_with_payment_method_data_billing(pmd_address);

        let payment_data = PaymentData {
            flow: PhantomData,
            payment_intent,
            payment_attempt,
            currency,
            amount,
            mandate_id: None,
            mandate_connector: None,
            setup_mandate,
            customer_acceptance,
            token: None,
            address: unified_address,
            token_data: None,
            confirm: request.confirm,
            payment_method_data,
            payment_method_token: None,
            payment_method_info: None,
            force_sync: None,
            all_keys_required: None,
            refunds: vec![],
            disputes: vec![],
            attempts: None,
            sessions_token: vec![],
            card_cvc: None,
            creds_identifier,
            pm_token: None,
            connector_customer_id: None,
            recurring_mandate_payment_data: None,
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
            is_manual_retry_enabled: business_profile.is_manual_retry_enabled,
            is_l2_l3_enabled: business_profile.is_l2_l3_enabled,
            external_authentication_data: None,
            client_session_id: None,
            vault_session_details: None,
        };

        let get_trackers_response = operations::GetTrackerResponse {
            operation: Box::new(self),
            customer_details: Some(customer_details),
            payment_data,
            business_profile,
            mandate_type,
        };

        Ok(get_trackers_response)
    }
}

#[async_trait]
impl<F: Clone + Send + Sync> Domain<F, api::PaymentsRequest, PaymentData<F>> for PaymentRecurrence {
    #[instrument(skip_all)]
    async fn get_or_create_customer_details<'a>(
        &'a self,
        state: &SessionState,
        payment_data: &mut PaymentData<F>,
        request: Option<CustomerDetails>,
        provider: &domain::Provider,
        _initiator: Option<&domain::Initiator>,
        _dimensions: &dimension_state::DimensionsWithProcessorAndProviderMerchantIdAndProfileId,
        _mandate_type: Option<api::MandateTransactionType>,
    ) -> CustomResult<
        (PaymentRecurrenceOperation<'a, F>, Option<domain::Customer>),
        errors::StorageError,
    > {
        match provider.get_account().merchant_account_type {
            common_enums::MerchantAccountType::Standard
            | common_enums::MerchantAccountType::Platform => {
                let customer = helpers::get_customer_if_exists(
                    state,
                    request.as_ref().and_then(|r| r.customer_id.as_ref()),
                    payment_data.payment_intent.customer_id.as_ref(),
                    provider,
                )
                .await?
                .map(|cust| {
                    payment_data
                        .payment_intent
                        .customer_id
                        .as_ref()
                        .is_some_and(|existing_id| existing_id != &cust.customer_id)
                        .then_some(errors::StorageError::ValueNotFound(
                            "Customer id mismatch between payment intent and request".to_string(),
                        ))
                        .map_or(Ok(()), Err)?;
                    Ok(cust)
                })
                .transpose()
                .map_err(|e: errors::StorageError| report!(e))?;

                Ok((Box::new(self), customer))
            }
            common_enums::MerchantAccountType::Connected => {
                Err(errors::StorageError::ValueNotFound(
                    "Connected merchant cannot be a provider".to_string(),
                )
                .into())
            }
        }
    }

    #[instrument(skip_all)]
    async fn add_task_to_process_tracker<'a>(
        &'a self,
        _state: &'a SessionState,
        _payment_attempt: &storage::PaymentAttempt,
        _requeue: bool,
        _schedule_time: Option<time::PrimitiveDateTime>,
    ) -> CustomResult<(), errors::ApiErrorResponse> {
        Ok(())
    }

    async fn get_connector<'a>(
        &'a self,
        _processor: &domain::Processor,
        state: &SessionState,
        request: &api::PaymentsRequest,
        _payment_intent: &storage::PaymentIntent,
    ) -> CustomResult<api::ConnectorChoice, errors::ApiErrorResponse> {
        helpers::get_connector_default(state, request.routing.clone()).await
    }

    #[instrument(skip_all)]
    async fn make_pm_data<'a>(
        &'a self,
        state: &'a SessionState,
        payment_data: &mut PaymentData<F>,
        storage_scheme: storage_enums::MerchantStorageScheme,
        platform: &domain::Platform,
        business_profile: &domain::Profile,
        should_retry_with_pan: bool,
    ) -> RouterResult<(
        PaymentRecurrenceOperation<'a, F>,
        Option<domain::PaymentMethodData>,
        Option<String>,
    )> {
        let (op, payment_method_data, pm_id) = Box::pin(helpers::make_pm_data(
            Box::new(self),
            state,
            payment_data,
            platform,
            storage_scheme,
            business_profile,
            should_retry_with_pan,
        ))
        .await?;
        utils::when(payment_method_data.is_none(), || {
            Err(errors::ApiErrorResponse::PaymentMethodNotFound)
        })?;

        Ok((op, payment_method_data, pm_id))
    }
}

#[cfg(feature = "v1")]
#[async_trait]
impl<F: Clone + Sync> UpdateTracker<F, PaymentData<F>, api::PaymentsRequest> for PaymentRecurrence {
    #[instrument(skip_all)]
    async fn update_trackers<'b>(
        &'b self,
        state: &'b SessionState,
        req_state: ReqState,
        processor: &domain::Processor,
        mut payment_data: PaymentData<F>,
        frm_suggestion: Option<FrmSuggestion>,
        _header_payload: hyperswitch_domain_models::payments::HeaderPayload,
        _dimensions: &dimension_state::DimensionsWithProcessorAndProviderMerchantId,
    ) -> RouterResult<(
        BoxedOperation<'b, F, api::PaymentsRequest, PaymentData<F>>,
        PaymentData<F>,
    )>
    where
        F: 'b + Send,
    {
        let storage_scheme = processor.get_account().storage_scheme;
        let key_store = processor.get_key_store();
        let frm_message = payment_data.frm_message.clone();

        let default_status_result = (
            storage_enums::IntentStatus::Processing,
            storage_enums::AttemptStatus::Pending,
            (None, None),
        );
        let status_handler_for_frm_results = |frm_suggestion: FrmSuggestion| match frm_suggestion {
            FrmSuggestion::FrmCancelTransaction => (
                storage_enums::IntentStatus::Failed,
                storage_enums::AttemptStatus::Failure,
                frm_message.map_or((None, None), |fraud_check| {
                    (
                        Some(Some(fraud_check.frm_status.to_string())),
                        Some(fraud_check.frm_reason.map(|reason| reason.to_string())),
                    )
                }),
            ),
            FrmSuggestion::FrmManualReview => (
                storage_enums::IntentStatus::RequiresMerchantAction,
                storage_enums::AttemptStatus::Unresolved,
                (None, None),
            ),
            FrmSuggestion::FrmAuthorizeTransaction => (
                storage_enums::IntentStatus::RequiresCapture,
                storage_enums::AttemptStatus::Authorized,
                (None, None),
            ),
        };

        let status_handler_for_authentication_results =
            |authentication: &hyperswitch_domain_models::authentication::Authentication| {
                if authentication.authentication_status.is_failed() {
                    (
                        storage_enums::IntentStatus::Failed,
                        storage_enums::AttemptStatus::Failure,
                        (
                            Some(Some("EXTERNAL_AUTHENTICATION_FAILURE".to_string())),
                            Some(Some("external authentication failure".to_string())),
                        ),
                    )
                } else if authentication.is_separate_authn_required() {
                    (
                        storage_enums::IntentStatus::RequiresCustomerAction,
                        storage_enums::AttemptStatus::AuthenticationPending,
                        (None, None),
                    )
                } else {
                    default_status_result.clone()
                }
            };

        let (intent_status, attempt_status, (error_code, error_message)) =
            match (frm_suggestion, payment_data.authentication.as_ref()) {
                (Some(frm_suggestion), _) => status_handler_for_frm_results(frm_suggestion),
                (_, Some(authentication_details)) => status_handler_for_authentication_results(
                    &authentication_details.authentication,
                ),
                _ => default_status_result,
            };
        let m_payment_data_payment_attempt = payment_data.payment_attempt.clone();
        let m_error_code = error_code.clone();
        let m_error_message = error_message.clone();
        let m_error_reason = error_message.clone();
        let m_db = state.clone().store;
        let cloned_key_store = key_store.clone();
        let payment_attempt_fut = tokio::spawn(
            async move {
                m_db.update_payment_attempt_with_attempt_id(
                    m_payment_data_payment_attempt,
                    storage::PaymentAttemptUpdate::RecurrenceUpdate {
                        status: attempt_status,
                        error_code: m_error_code.flatten(),
                        error_message: m_error_message.flatten(),
                        error_reason: m_error_reason.flatten(),
                        updated_by: storage_scheme.to_string(),
                        connector_mandate_detail: payment_data
                            .payment_attempt
                            .connector_mandate_detail
                            .clone(),
                    },
                    storage_scheme,
                    &cloned_key_store,
                )
                .map(|x| x.to_not_found_response(errors::ApiErrorResponse::PaymentNotFound))
                .await
            }
            .in_current_span(),
        );

        let m_payment_data_payment_intent = payment_data.payment_intent.clone();
        let m_db = state.clone().store;
        let m_storage_scheme = storage_scheme.to_string();
        let m_key_store = key_store.clone();
        let payment_intent_fut = tokio::spawn(
            async move {
                m_db.update_payment_intent(
                    m_payment_data_payment_intent,
                    storage::PaymentIntentUpdate::RecurrenceUpdate {
                        status: intent_status,
                        updated_by: m_storage_scheme,
                    },
                    &m_key_store,
                    storage_scheme,
                )
                .map(|x| x.to_not_found_response(errors::ApiErrorResponse::PaymentNotFound))
                .await
            }
            .in_current_span(),
        );

        let (payment_intent, payment_attempt) = tokio::try_join!(
            utils::flatten_join_error(payment_intent_fut),
            utils::flatten_join_error(payment_attempt_fut),
        )?;

        payment_data.payment_intent = payment_intent;
        payment_data.payment_attempt = payment_attempt;

        req_state
            .event_context
            .event(AuditEvent::new(AuditEventType::PaymentRecurrence))
            .with(payment_data.to_event())
            .emit();
        Ok((Box::new(self), payment_data))
    }
}

#[async_trait]
impl<F: Send + Clone + Sync> ValidateRequest<F, api::PaymentsRequest, PaymentData<F>>
    for PaymentRecurrence
{
    #[instrument(skip_all)]
    fn validate_request<'a, 'b>(
        &'b self,
        request: &api::PaymentsRequest,
        processor: &'a domain::Processor,
    ) -> RouterResult<(
        PaymentRecurrenceOperation<'b, F>,
        operations::ValidateResult,
    )> {
        let payment_id = request
            .payment_id
            .clone()
            .ok_or(report!(errors::ApiErrorResponse::PaymentNotFound))?;

        Ok((
            Box::new(self),
            operations::ValidateResult {
                merchant_id: processor.get_account().get_id().to_owned(),
                payment_id,
                storage_scheme: processor.get_account().storage_scheme,
                requeue: matches!(
                    request.retry_action,
                    Some(api_models::enums::RetryAction::Requeue)
                ),
            },
        ))
    }
}
