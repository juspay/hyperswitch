use std::marker::PhantomData;

use api_models::{enums::FrmSuggestion, payments::GetAddressFromPaymentMethodData};
use async_trait::async_trait;
use common_utils::ext_traits::{AsyncExt, Encode, ValueExt};
use error_stack::{report, ResultExt};
use futures::FutureExt;
#[cfg(feature = "v1")]
use hyperswitch_domain_models::payments::payment_intent::PaymentIntentUpdateFields;
use hyperswitch_masking::ExposeInterface;
use router_derive::PaymentOperation;
use router_env::{instrument, tracing};
use tracing_futures::Instrument;

use super::{BoxedOperation, Domain, GetTracker, Operation, UpdateTracker, ValidateRequest};
#[cfg(feature = "v1")]
use crate::{
    core::payment_methods::cards::create_encrypted_data,
    events::audit_events::{AuditEvent, AuditEventType},
};
use crate::{
    core::{
        configs::dimension_state::DimensionsWithMerchantIdAndProfileId,
        errors::{self, CustomResult, RouterResult, StorageErrorExt},
        mandate::helpers as m_helpers,
        payment_methods::transformers as pm_transformers,
        payments::{
            helpers, operations, CustomerDetails, OperationSessionGetters, PaymentAddress,
            PaymentData,
        },
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
        #[cfg(feature = "pm_modular")] payment_method_with_raw_data: Option<
            pm_transformers::PaymentMethodWithRawData,
        >,
    ) -> RouterResult<operations::GetTrackerResponse<'a, F, api::PaymentsRequest, PaymentData<F>>>
    {
        #[cfg(not(feature = "pm_modular"))]
        let payment_method_with_raw_data: Option<
            pm_transformers::PaymentMethodWithRawData,
        > = None;

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

        let customer_details = helpers::get_customer_details_from_request(request);

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
        let profile_id = payment_intent
            .profile_id
            .clone()
            .get_required_value("profile_id")
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("'profile_id' not set in payment intent")?;

        let additional_pm_data_fut = tokio::spawn(
            async move {
                Ok(n_request_payment_method_data
                    .async_map(|payment_method_data| async move {
                        helpers::get_additional_payment_data(
                            &payment_method_data,
                            store.as_ref(),
                            &profile_id,
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

        let m_merchant_id = processor_merchant_id.clone();
        let m_payment_intent_shipping_address_id = payment_intent.shipping_address_id.clone();
        let m_payment_intent_payment_id = payment_intent.payment_id.clone();
        let m_key_store = platform.get_processor().get_key_store().clone();
        let session_state = state.clone();

        let shipping_address = helpers::get_address_by_id(
            &session_state,
            m_payment_intent_shipping_address_id,
            &m_key_store,
            &m_payment_intent_payment_id,
            &m_merchant_id,
            m_storage_scheme,
        )
        .await?;

        let session_state = state.clone();
        let m_payment_intent_billing_address_id = payment_intent.billing_address_id.clone();
        let m_key_store = platform.get_processor().get_key_store().clone();
        let m_payment_intent_payment_id = payment_intent.payment_id.clone();
        let m_merchant_id = processor_merchant_id.clone();

        let billing_address = helpers::get_address_by_id(
            &session_state,
            m_payment_intent_billing_address_id,
            &m_key_store,
            &m_payment_intent_payment_id,
            &m_merchant_id,
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
                .map(|decrypted_data| decrypted_data.into_inner().expose())
                .and_then(|decrypted_value| {
                    decrypted_value
                        .parse_value::<hyperswitch_domain_models::address::Address>(
                            "payment method billing address",
                        )
                        .ok()
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
        _dimensions: DimensionsWithMerchantIdAndProfileId,
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
        header_payload: hyperswitch_domain_models::payments::HeaderPayload,
    ) -> RouterResult<(
        BoxedOperation<'b, F, api::PaymentsRequest, PaymentData<F>>,
        PaymentData<F>,
    )>
    where
        F: 'b + Send,
    {
        let storage_scheme = processor.get_account().storage_scheme;
        let key_store = processor.get_key_store();
        let payment_method = payment_data.payment_attempt.payment_method;
        let browser_info = payment_data.payment_attempt.browser_info.clone();
        let frm_message = payment_data.frm_message.clone();
        let capture_method = payment_data.payment_attempt.capture_method;

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

        let connector = payment_data.payment_attempt.connector.clone();
        let merchant_connector_id = payment_data.payment_attempt.merchant_connector_id.clone();
        let connector_request_reference_id = payment_data
            .payment_attempt
            .connector_request_reference_id
            .clone();

        let straight_through_algorithm = payment_data
            .payment_attempt
            .straight_through_algorithm
            .clone();
        let payment_token = payment_data.token.clone();
        let payment_method_type = payment_data.payment_attempt.payment_method_type;
        let profile_id = payment_data
            .payment_intent
            .profile_id
            .as_ref()
            .get_required_value("profile_id")
            .change_context(errors::ApiErrorResponse::InternalServerError)?;

        let payment_experience = payment_data.payment_attempt.payment_experience;
        let additional_pm_data = payment_data
            .payment_method_data
            .as_ref()
            .async_map(|payment_method_data| async {
                helpers::get_additional_payment_data(
                    payment_method_data,
                    &*state.store,
                    profile_id,
                    payment_data.payment_method_token.as_ref(),
                )
                .await
            })
            .await
            .transpose()?
            .flatten();

        let encoded_additional_pm_data = additional_pm_data
            .as_ref()
            .map(Encode::encode_to_value)
            .transpose()
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Failed to encode additional pm data")?;

        let customer_details = payment_data.payment_intent.customer_details.clone();
        let business_sub_label = payment_data.payment_attempt.business_sub_label.clone();
        let authentication_type = payment_data.payment_attempt.authentication_type;

        let (shipping_address_id, billing_address_id, payment_method_billing_address_id) = (
            payment_data.payment_intent.shipping_address_id.clone(),
            payment_data.payment_intent.billing_address_id.clone(),
            payment_data
                .payment_attempt
                .payment_method_billing_address_id
                .clone(),
        );

        let customer_id = payment_data.payment_intent.customer_id.clone();
        let return_url = payment_data.payment_intent.return_url.take();
        let setup_future_usage = payment_data.payment_intent.setup_future_usage;
        let business_label = payment_data.payment_intent.business_label.clone();
        let business_country = payment_data.payment_intent.business_country;
        let description = payment_data.payment_intent.description.take();
        let statement_descriptor_name =
            payment_data.payment_intent.statement_descriptor_name.take();
        let statement_descriptor_suffix = payment_data
            .payment_intent
            .statement_descriptor_suffix
            .take();
        let order_details = payment_data.payment_intent.order_details.clone();
        let metadata = payment_data.payment_intent.metadata.clone();
        let frm_metadata = payment_data.payment_intent.frm_metadata.clone();

        let client_source = header_payload
            .client_source
            .clone()
            .or(payment_data.payment_attempt.client_source.clone());
        let client_version = header_payload
            .client_version
            .clone()
            .or(payment_data.payment_attempt.client_version.clone());

        let m_payment_data_payment_attempt = payment_data.payment_attempt.clone();
        let m_payment_method_id =
            payment_data
                .payment_attempt
                .payment_method_id
                .clone()
                .or(payment_data
                    .payment_method_info
                    .as_ref()
                    .map(|payment_method| payment_method.payment_method_id.clone()));
        let m_browser_info = browser_info.clone();
        let m_connector = connector.clone();
        let m_capture_method = capture_method;
        let m_payment_token = payment_token.clone();
        let m_additional_pm_data = encoded_additional_pm_data
            .clone()
            .or(payment_data.payment_attempt.payment_method_data.clone());
        let m_business_sub_label = business_sub_label.clone();
        let m_straight_through_algorithm = straight_through_algorithm.clone();
        let m_error_code = error_code.clone();
        let m_error_message = error_message.clone();
        let m_fingerprint_id = payment_data.payment_attempt.fingerprint_id.clone();
        let m_db = state.clone().store;
        let surcharge_amount = payment_data
            .surcharge_details
            .as_ref()
            .map(|surcharge_details| surcharge_details.surcharge_amount);
        let tax_amount = payment_data
            .surcharge_details
            .as_ref()
            .map(|surcharge_details| surcharge_details.tax_on_surcharge_amount);

        let (
            external_three_ds_authentication_attempted,
            authentication_connector,
            authentication_id,
        ) = match payment_data.authentication.as_ref() {
            Some(authentication_store) => (
                Some(
                    authentication_store
                        .authentication
                        .is_separate_authn_required(),
                ),
                authentication_store
                    .authentication
                    .authentication_connector
                    .clone(),
                Some(
                    authentication_store
                        .authentication
                        .authentication_id
                        .clone(),
                ),
            ),
            None => (None, None, None),
        };

        let card_discovery = payment_data.get_card_discovery_for_card_payment_method();
        let installment_data = payment_data.get_installment_details().cloned();
        let is_stored_credential = helpers::is_stored_credential(
            &payment_data.recurring_details,
            &payment_data.pm_token,
            payment_data.mandate_id.is_some(),
            payment_data.payment_attempt.is_stored_credential,
        );
        let cloned_key_store = key_store.clone();
        let payment_attempt_fut = tokio::spawn(
            async move {
                m_db.update_payment_attempt_with_attempt_id(
                    m_payment_data_payment_attempt,
                    storage::PaymentAttemptUpdate::ConfirmUpdate {
                        currency: payment_data.currency,
                        status: attempt_status,
                        payment_method,
                        authentication_type,
                        capture_method: m_capture_method,
                        browser_info: m_browser_info,
                        connector: m_connector,
                        payment_token: m_payment_token,
                        payment_method_data: m_additional_pm_data,
                        payment_method_type,
                        payment_experience,
                        business_sub_label: m_business_sub_label,
                        straight_through_algorithm: m_straight_through_algorithm,
                        error_code: m_error_code,
                        error_message: m_error_message,
                        updated_by: storage_scheme.to_string(),
                        merchant_connector_id,
                        external_three_ds_authentication_attempted,
                        authentication_connector,
                        authentication_id,
                        payment_method_billing_address_id,
                        fingerprint_id: m_fingerprint_id,
                        payment_method_id: m_payment_method_id,
                        client_source,
                        client_version,
                        customer_acceptance: payment_data
                            .payment_attempt
                            .customer_acceptance
                            .clone(),
                        net_amount:
                            hyperswitch_domain_models::payments::payment_attempt::NetAmount::new(
                                payment_data.payment_attempt.net_amount.get_order_amount(),
                                payment_data.payment_intent.shipping_cost,
                                payment_data
                                    .payment_attempt
                                    .net_amount
                                    .get_order_tax_amount(),
                                surcharge_amount,
                                tax_amount,
                                payment_data
                                    .payment_attempt
                                    .net_amount
                                    .get_installment_interest(),
                            ),

                        connector_mandate_detail: payment_data
                            .payment_attempt
                            .connector_mandate_detail
                            .clone(),
                        card_discovery,
                        routing_approach: payment_data.payment_attempt.routing_approach.clone(),
                        connector_request_reference_id,
                        network_transaction_id: payment_data
                            .payment_attempt
                            .network_transaction_id
                            .clone(),
                        is_stored_credential,
                        request_extended_authorization: payment_data
                            .payment_attempt
                            .request_extended_authorization,
                        tokenization: payment_data.payment_attempt.get_tokenization_strategy(),
                        installment_data,
                    },
                    storage_scheme,
                    &cloned_key_store,
                )
                .map(|x| x.to_not_found_response(errors::ApiErrorResponse::PaymentNotFound))
                .await
            }
            .in_current_span(),
        );

        let billing_address = payment_data.address.get_payment_billing();
        let key_manager_state = state.into();
        let billing_details = billing_address
            .async_map(|billing_details| {
                create_encrypted_data(&key_manager_state, key_store, billing_details)
            })
            .await
            .transpose()
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Unable to encrypt billing details")?;

        let shipping_address = payment_data.address.get_shipping();
        let shipping_details = shipping_address
            .async_map(|shipping_details| {
                create_encrypted_data(&key_manager_state, key_store, shipping_details)
            })
            .await
            .transpose()
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Unable to encrypt shipping details")?;

        let m_payment_data_payment_intent = payment_data.payment_intent.clone();
        let m_customer_id = customer_id.clone();
        let m_shipping_address_id = shipping_address_id.clone();
        let m_billing_address_id = billing_address_id.clone();
        let m_return_url = return_url.clone();
        let m_business_label = business_label.clone();
        let m_description = description.clone();
        let m_statement_descriptor_name = statement_descriptor_name.clone();
        let m_statement_descriptor_suffix = statement_descriptor_suffix.clone();
        let m_order_details = order_details.clone();
        let m_metadata = metadata.clone();
        let m_frm_metadata = frm_metadata.clone();
        let m_db = state.clone().store;
        let m_storage_scheme = storage_scheme.to_string();
        let session_expiry = m_payment_data_payment_intent.session_expiry;
        let m_key_store = key_store.clone();
        let is_payment_processor_token_flow =
            payment_data.payment_intent.is_payment_processor_token_flow;
        let payment_intent_fut = tokio::spawn(
            async move {
                m_db.update_payment_intent(
                    m_payment_data_payment_intent,
                    storage::PaymentIntentUpdate::Update(Box::new(PaymentIntentUpdateFields {
                        amount: payment_data.payment_intent.amount,
                        currency: payment_data.currency,
                        setup_future_usage,
                        status: intent_status,
                        customer_id: m_customer_id,
                        shipping_address_id: m_shipping_address_id,
                        billing_address_id: m_billing_address_id,
                        return_url: m_return_url,
                        business_country,
                        business_label: m_business_label,
                        description: m_description,
                        statement_descriptor_name: m_statement_descriptor_name,
                        statement_descriptor_suffix: m_statement_descriptor_suffix,
                        order_details: m_order_details,
                        metadata: m_metadata,
                        payment_confirm_source: header_payload.payment_confirm_source,
                        updated_by: m_storage_scheme,
                        fingerprint_id: None,
                        session_expiry,
                        request_external_three_ds_authentication: None,
                        frm_metadata: m_frm_metadata,
                        customer_details,
                        merchant_order_reference_id: None,
                        billing_details,
                        shipping_details,
                        is_payment_processor_token_flow,
                        tax_details: None,
                        force_3ds_challenge: payment_data.payment_intent.force_3ds_challenge,
                        is_iframe_redirection_enabled: payment_data
                            .payment_intent
                            .is_iframe_redirection_enabled,
                        is_confirm_operation: true, // Indicates that this is a confirm operation
                        payment_channel: payment_data.payment_intent.payment_channel,
                        feature_metadata: payment_data
                            .payment_intent
                            .feature_metadata
                            .clone()
                            .map(hyperswitch_masking::Secret::new),
                        tax_status: payment_data.payment_intent.tax_status,
                        discount_amount: payment_data.payment_intent.discount_amount,
                        order_date: payment_data.payment_intent.order_date,
                        shipping_amount_tax: payment_data.payment_intent.shipping_amount_tax,
                        duty_amount: payment_data.payment_intent.duty_amount,
                        enable_partial_authorization: payment_data
                            .payment_intent
                            .enable_partial_authorization,
                        enable_overcapture: payment_data.payment_intent.enable_overcapture,
                        shipping_cost: None,
                        installment_options: payment_data
                            .payment_intent
                            .installment_options
                            .clone(),
                    })),
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
