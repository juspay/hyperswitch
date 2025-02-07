use std::marker::PhantomData;

use api_models::{
    enums::FrmSuggestion, mandates::RecurringDetails, payments::RequestSurchargeDetails,
};
use async_trait::async_trait;
use common_utils::{
    ext_traits::{AsyncExt, Encode, ValueExt},
    pii::Email,
    types::keymanager::KeyManagerState,
};
use error_stack::{report, ResultExt};
use hyperswitch_domain_models::payments::payment_intent::{
    CustomerData, PaymentIntentUpdateFields,
};
use router_derive::PaymentOperation;
use router_env::{instrument, tracing};

use super::{BoxedOperation, Domain, GetTracker, Operation, UpdateTracker, ValidateRequest};
use crate::{
    core::{
        errors::{self, CustomResult, RouterResult, StorageErrorExt},
        mandate::helpers as m_helpers,
        payment_methods::cards::create_encrypted_data,
        payments::{self, helpers, operations, CustomerDetails, PaymentAddress, PaymentData},
        utils as core_utils,
    },
    events::audit_events::{AuditEvent, AuditEventType},
    routes::{app::ReqState, SessionState},
    services,
    types::{
        self,
        api::{self, ConnectorCallType, PaymentIdTypeExt},
        domain,
        storage::{self, enums as storage_enums, payment_attempt::PaymentAttemptExt},
        transformers::ForeignTryFrom,
    },
    utils::OptionExt,
};

#[derive(Debug, Clone, Copy, PaymentOperation)]
#[operation(operations = "all", flow = "authorize")]
pub struct PaymentUpdate;

type PaymentUpdateOperation<'a, F> = BoxedOperation<'a, F, api::PaymentsRequest, PaymentData<F>>;

#[async_trait]
impl<F: Send + Clone + Sync> GetTracker<F, PaymentData<F>, api::PaymentsRequest> for PaymentUpdate {
    #[instrument(skip_all)]
    async fn get_trackers<'a>(
        &'a self,
        state: &'a SessionState,
        payment_id: &api::PaymentIdType,
        request: &api::PaymentsRequest,
        merchant_account: &domain::MerchantAccount,
        key_store: &domain::MerchantKeyStore,
        auth_flow: services::AuthFlow,
        _header_payload: &hyperswitch_domain_models::payments::HeaderPayload,
        _platform_merchant_account: Option<&domain::MerchantAccount>,
    ) -> RouterResult<operations::GetTrackerResponse<'a, F, api::PaymentsRequest, PaymentData<F>>>
    {
        let (mut payment_intent, mut payment_attempt, currency): (_, _, storage_enums::Currency);

        let payment_id = payment_id
            .get_payment_intent_id()
            .change_context(errors::ApiErrorResponse::PaymentNotFound)?;
        let merchant_id = merchant_account.get_id();
        let storage_scheme = merchant_account.storage_scheme;

        let db = &*state.store;
        let key_manager_state = &state.into();

        payment_intent = db
            .find_payment_intent_by_payment_id_merchant_id(
                key_manager_state,
                &payment_id,
                merchant_id,
                key_store,
                storage_scheme,
            )
            .await
            .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)?;

        // TODO (#7195): Add platform merchant account validation once publishable key auth is solved

        if let Some(order_details) = &request.order_details {
            helpers::validate_order_details_amount(
                order_details.to_owned(),
                payment_intent.amount,
                false,
            )?;
        }

        payment_intent.setup_future_usage = request
            .setup_future_usage
            .or(payment_intent.setup_future_usage);

        helpers::validate_customer_access(&payment_intent, auth_flow, request)?;

        helpers::validate_card_data(
            request
                .payment_method_data
                .as_ref()
                .and_then(|pmd| pmd.payment_method_data.clone()),
        )?;

        helpers::validate_payment_status_against_allowed_statuses(
            payment_intent.status,
            &[
                storage_enums::IntentStatus::RequiresPaymentMethod,
                storage_enums::IntentStatus::RequiresConfirmation,
            ],
            "update",
        )?;

        helpers::authenticate_client_secret(request.client_secret.as_ref(), &payment_intent)?;

        payment_intent.order_details = request
            .get_order_details_as_value()
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Failed to convert order details to value")?
            .or(payment_intent.order_details);

        payment_attempt = db
            .find_payment_attempt_by_payment_id_merchant_id_attempt_id(
                &payment_intent.payment_id,
                merchant_id,
                payment_intent.active_attempt.get_id().as_str(),
                storage_scheme,
            )
            .await
            .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)?;

        let customer_acceptance = request.customer_acceptance.clone().map(From::from);
        let recurring_details = request.recurring_details.clone();

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

        let m_helpers::MandateGenericData {
            token,
            payment_method,
            payment_method_type,
            mandate_data,
            recurring_mandate_payment_data,
            mandate_connector,
            payment_method_info,
        } = Box::pin(helpers::get_token_pm_type_mandate_details(
            state,
            request,
            mandate_type.to_owned(),
            merchant_account,
            key_store,
            None,
            payment_intent.customer_id.as_ref(),
        ))
        .await?;
        helpers::validate_amount_to_capture_and_capture_method(Some(&payment_attempt), request)?;

        helpers::validate_request_amount_and_amount_to_capture(
            request.amount,
            request.amount_to_capture,
            request
                .surcharge_details
                .or(payment_attempt.get_surcharge_details()),
        )
        .change_context(errors::ApiErrorResponse::InvalidDataFormat {
            field_name: "amount_to_capture".to_string(),
            expected_format: "amount_to_capture lesser than or equal to amount".to_string(),
        })?;

        currency = request
            .currency
            .or(payment_attempt.currency)
            .get_required_value("currency")?;

        payment_attempt.payment_method = payment_method.or(payment_attempt.payment_method);
        payment_attempt.payment_method_type =
            payment_method_type.or(payment_attempt.payment_method_type);
        let customer_details = helpers::get_customer_details_from_request(request);

        let amount = request
            .amount
            .unwrap_or_else(|| payment_attempt.net_amount.get_order_amount().into());

        if request.confirm.unwrap_or(false) {
            helpers::validate_customer_id_mandatory_cases(
                request.setup_future_usage.is_some(),
                payment_intent
                    .customer_id
                    .as_ref()
                    .or(customer_details.customer_id.as_ref()),
            )?;
        }

        let shipping_address = helpers::create_or_update_address_for_payment_by_request(
            state,
            request.shipping.as_ref(),
            payment_intent.shipping_address_id.as_deref(),
            merchant_id,
            payment_intent
                .customer_id
                .as_ref()
                .or(customer_details.customer_id.as_ref()),
            key_store,
            &payment_intent.payment_id,
            merchant_account.storage_scheme,
        )
        .await?;
        let billing_address = helpers::create_or_update_address_for_payment_by_request(
            state,
            request.billing.as_ref(),
            payment_intent.billing_address_id.as_deref(),
            merchant_id,
            payment_intent
                .customer_id
                .as_ref()
                .or(customer_details.customer_id.as_ref()),
            key_store,
            &payment_intent.payment_id,
            merchant_account.storage_scheme,
        )
        .await?;

        let payment_method_billing = helpers::create_or_update_address_for_payment_by_request(
            state,
            request
                .payment_method_data
                .as_ref()
                .and_then(|pmd| pmd.billing.as_ref()),
            payment_attempt.payment_method_billing_address_id.as_deref(),
            merchant_id,
            payment_intent
                .customer_id
                .as_ref()
                .or(customer_details.customer_id.as_ref()),
            key_store,
            &payment_intent.payment_id,
            merchant_account.storage_scheme,
        )
        .await?;

        payment_intent.shipping_address_id = shipping_address.clone().map(|x| x.address_id);
        payment_intent.billing_address_id = billing_address.clone().map(|x| x.address_id);
        payment_attempt.payment_method_billing_address_id = payment_method_billing
            .as_ref()
            .map(|payment_method_billing| payment_method_billing.address_id.clone());

        payment_intent.allowed_payment_method_types = request
            .get_allowed_payment_method_types_as_value()
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Error converting allowed_payment_types to Value")?
            .or(payment_intent.allowed_payment_method_types);

        payment_intent.connector_metadata = request
            .get_connector_metadata_as_value()
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Error converting connector_metadata to Value")?
            .or(payment_intent.connector_metadata);

        payment_intent.feature_metadata = request
            .get_feature_metadata_as_value()
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Error converting feature_metadata to Value")?
            .or(payment_intent.feature_metadata);
        payment_intent.metadata = request.metadata.clone().or(payment_intent.metadata);
        payment_intent.frm_metadata = request.frm_metadata.clone().or(payment_intent.frm_metadata);
        payment_intent.psd2_sca_exemption_type = request
            .psd2_sca_exemption_type
            .or(payment_intent.psd2_sca_exemption_type);
        Self::populate_payment_intent_with_request(&mut payment_intent, request);

        let token = token.or_else(|| payment_attempt.payment_token.clone());

        if request.confirm.unwrap_or(false) {
            helpers::validate_pm_or_token_given(
                &request.payment_method,
                &request
                    .payment_method_data
                    .as_ref()
                    .and_then(|pmd| pmd.payment_method_data.clone()),
                &request.payment_method_type,
                &mandate_type,
                &token,
                &request.ctp_service_details,
            )?;
        }

        let token_data = if let Some(token) = token.clone() {
            Some(helpers::retrieve_payment_token_data(state, token, payment_method).await?)
        } else {
            None
        };

        let mandate_id = request
            .mandate_id
            .as_ref()
            .or_else(|| {
            request.recurring_details
                .as_ref()
                .and_then(|recurring_details| match recurring_details {
                    RecurringDetails::MandateId(id) => Some(id),
                    _ => None,
                })
        })
            .async_and_then(|mandate_id| async {
                let mandate = db
                    .find_mandate_by_merchant_id_mandate_id(merchant_id, mandate_id, merchant_account.storage_scheme)
                    .await
                    .change_context(errors::ApiErrorResponse::MandateNotFound);
                Some(mandate.and_then(|mandate_obj| {
                    match (
                        mandate_obj.network_transaction_id,
                        mandate_obj.connector_mandate_ids,
                    ) {
                        (Some(network_tx_id), _) => Ok(api_models::payments::MandateIds {
                            mandate_id: Some(mandate_obj.mandate_id),
                            mandate_reference_id: Some(
                                api_models::payments::MandateReferenceId::NetworkMandateId(
                                    network_tx_id,
                                ),
                            ),
                        }),
                        (_, Some(connector_mandate_id)) => connector_mandate_id
                        .parse_value("ConnectorMandateId")
                        .change_context(errors::ApiErrorResponse::MandateNotFound)
                        .map(|connector_id: api_models::payments::ConnectorMandateReferenceId| {
                            api_models::payments::MandateIds {
                                mandate_id: Some(mandate_obj.mandate_id),
                                mandate_reference_id: Some(api_models::payments::MandateReferenceId::ConnectorMandateId(
                                    api_models::payments::ConnectorMandateReferenceId::new(
                                        connector_id.get_connector_mandate_id(),        // connector_mandate_id
                                        connector_id.get_payment_method_id(),           // payment_method_id
                                        None,                                     // update_history
                                        connector_id.get_mandate_metadata(),            // mandate_metadata
                                        connector_id.get_connector_mandate_request_reference_id()  // connector_mandate_request_reference_id
                                    )
                                ))
                            }
                         }),
                        (_, _) => Ok(api_models::payments::MandateIds {
                            mandate_id: Some(mandate_obj.mandate_id),
                            mandate_reference_id: None,
                        }),
                    }
                }))
            })
            .await
            .transpose()?;
        let (next_operation, amount): (PaymentUpdateOperation<'a, F>, _) =
            if request.confirm.unwrap_or(false) {
                let amount = {
                    let amount = request
                        .amount
                        .map(Into::into)
                        .unwrap_or(payment_attempt.net_amount.get_order_amount());
                    payment_attempt.net_amount.set_order_amount(amount);
                    payment_intent.amount = amount;
                    let surcharge_amount = request
                        .surcharge_details
                        .as_ref()
                        .map(RequestSurchargeDetails::get_total_surcharge_amount)
                        .or(payment_attempt.get_total_surcharge_amount());
                    amount + surcharge_amount.unwrap_or_default()
                };
                (Box::new(operations::PaymentConfirm), amount.into())
            } else {
                (Box::new(self), amount)
            };

        payment_intent.status = if request
            .payment_method_data
            .as_ref()
            .is_some_and(|payment_method_data| payment_method_data.payment_method_data.is_some())
        {
            if request.confirm.unwrap_or(false) {
                payment_intent.status
            } else {
                storage_enums::IntentStatus::RequiresConfirmation
            }
        } else {
            storage_enums::IntentStatus::RequiresPaymentMethod
        };

        payment_intent.request_external_three_ds_authentication = request
            .request_external_three_ds_authentication
            .or(payment_intent.request_external_three_ds_authentication);

        payment_intent.merchant_order_reference_id = request
            .merchant_order_reference_id
            .clone()
            .or(payment_intent.merchant_order_reference_id);

        Self::populate_payment_attempt_with_request(&mut payment_attempt, request);

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
                    merchant_account.get_id(),
                    mcd,
                )
                .await
            })
            .await
            .transpose()?;

        // The operation merges mandate data from both request and payment_attempt
        let setup_mandate = mandate_data.map(Into::into);
        let mandate_details_present =
            payment_attempt.mandate_details.is_some() || request.mandate_data.is_some();
        helpers::validate_mandate_data_and_future_usage(
            payment_intent.setup_future_usage,
            mandate_details_present,
        )?;
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

        let surcharge_details = request.surcharge_details.map(|request_surcharge_details| {
            payments::types::SurchargeDetails::from((&request_surcharge_details, &payment_attempt))
        });

        let payment_data = PaymentData {
            flow: PhantomData,
            payment_intent,
            payment_attempt,
            currency,
            amount,
            email: request.email.clone(),
            mandate_id,
            mandate_connector,
            token,
            token_data,
            setup_mandate,
            customer_acceptance,
            address: PaymentAddress::new(
                shipping_address.as_ref().map(From::from),
                billing_address.as_ref().map(From::from),
                payment_method_billing.as_ref().map(From::from),
                business_profile.use_billing_as_payment_method_billing,
            ),
            confirm: request.confirm,
            payment_method_data: request
                .payment_method_data
                .as_ref()
                .and_then(|pmd| pmd.payment_method_data.clone().map(Into::into)),
            payment_method_info,
            force_sync: None,
            refunds: vec![],
            disputes: vec![],
            attempts: None,
            sessions_token: vec![],
            card_cvc: request.card_cvc.clone(),
            creds_identifier,
            pm_token: None,
            connector_customer_id: None,
            recurring_mandate_payment_data,
            ephemeral_key: None,
            multiple_capture_data: None,
            redirect_response: None,
            surcharge_details,
            frm_message: None,
            payment_link_data: None,
            incremental_authorization_details: None,
            authorizations: vec![],
            authentication: None,
            recurring_details,
            poll_config: None,
            tax_data: None,
            session_id: None,
            service_details: None,
        };

        let get_trackers_response = operations::GetTrackerResponse {
            operation: next_operation,
            customer_details: Some(customer_details),
            payment_data,
            business_profile,
            mandate_type,
        };

        Ok(get_trackers_response)
    }
}

#[async_trait]
impl<F: Clone + Send + Sync> Domain<F, api::PaymentsRequest, PaymentData<F>> for PaymentUpdate {
    #[instrument(skip_all)]
    async fn get_or_create_customer_details<'a>(
        &'a self,
        state: &SessionState,
        payment_data: &mut PaymentData<F>,
        request: Option<CustomerDetails>,
        key_store: &domain::MerchantKeyStore,
        storage_scheme: common_enums::enums::MerchantStorageScheme,
    ) -> CustomResult<(PaymentUpdateOperation<'a, F>, Option<domain::Customer>), errors::StorageError>
    {
        helpers::create_customer_if_not_exist(
            state,
            Box::new(self),
            payment_data,
            request,
            &key_store.merchant_id,
            key_store,
            storage_scheme,
        )
        .await
    }

    async fn payments_dynamic_tax_calculation<'a>(
        &'a self,
        state: &SessionState,
        payment_data: &mut PaymentData<F>,
        _connector_call_type: &ConnectorCallType,
        business_profile: &domain::Profile,
        key_store: &domain::MerchantKeyStore,
        merchant_account: &domain::MerchantAccount,
    ) -> CustomResult<(), errors::ApiErrorResponse> {
        let is_tax_connector_enabled = business_profile.get_is_tax_connector_enabled();
        let skip_external_tax_calculation = payment_data
            .payment_intent
            .skip_external_tax_calculation
            .unwrap_or(false);
        if is_tax_connector_enabled && !skip_external_tax_calculation {
            let db = state.store.as_ref();
            let key_manager_state: &KeyManagerState = &state.into();

            let merchant_connector_id = business_profile
                .tax_connector_id
                .as_ref()
                .get_required_value("business_profile.tax_connector_id")?;

            #[cfg(feature = "v1")]
            let mca = db
                .find_by_merchant_connector_account_merchant_id_merchant_connector_id(
                    key_manager_state,
                    &business_profile.merchant_id,
                    merchant_connector_id,
                    key_store,
                )
                .await
                .to_not_found_response(
                    errors::ApiErrorResponse::MerchantConnectorAccountNotFound {
                        id: merchant_connector_id.get_string_repr().to_string(),
                    },
                )?;

            #[cfg(feature = "v2")]
            let mca = db
                .find_merchant_connector_account_by_id(
                    key_manager_state,
                    merchant_connector_id,
                    key_store,
                )
                .await
                .to_not_found_response(
                    errors::ApiErrorResponse::MerchantConnectorAccountNotFound {
                        id: merchant_connector_id.get_string_repr().to_string(),
                    },
                )?;

            let connector_data =
                api::TaxCalculateConnectorData::get_connector_by_name(&mca.connector_name)?;

            let router_data = core_utils::construct_payments_dynamic_tax_calculation_router_data(
                state,
                merchant_account,
                key_store,
                payment_data,
                &mca,
            )
            .await?;
            let connector_integration: services::BoxedPaymentConnectorIntegrationInterface<
                api::CalculateTax,
                types::PaymentsTaxCalculationData,
                types::TaxCalculationResponseData,
            > = connector_data.connector.get_connector_integration();

            let response = services::execute_connector_processing_step(
                state,
                connector_integration,
                &router_data,
                payments::CallConnectorAction::Trigger,
                None,
            )
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Tax connector Response Failed")?;

            let tax_response = response.response.map_err(|err| {
                errors::ApiErrorResponse::ExternalConnectorError {
                    code: err.code,
                    message: err.message,
                    connector: connector_data.connector_name.clone().to_string(),
                    status_code: err.status_code,
                    reason: err.reason,
                }
            })?;

            payment_data.payment_intent.tax_details = Some(diesel_models::TaxDetails {
                default: Some(diesel_models::DefaultTax {
                    order_tax_amount: tax_response.order_tax_amount,
                }),
                payment_method_type: None,
            });

            Ok(())
        } else {
            Ok(())
        }
    }

    #[instrument(skip_all)]
    async fn make_pm_data<'a>(
        &'a self,
        state: &'a SessionState,
        payment_data: &mut PaymentData<F>,
        storage_scheme: storage_enums::MerchantStorageScheme,
        merchant_key_store: &domain::MerchantKeyStore,
        customer: &Option<domain::Customer>,
        business_profile: &domain::Profile,
    ) -> RouterResult<(
        PaymentUpdateOperation<'a, F>,
        Option<domain::PaymentMethodData>,
        Option<String>,
    )> {
        Box::pin(helpers::make_pm_data(
            Box::new(self),
            state,
            payment_data,
            merchant_key_store,
            customer,
            storage_scheme,
            business_profile,
        ))
        .await
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
        _merchant_account: &domain::MerchantAccount,
        state: &SessionState,
        request: &api::PaymentsRequest,
        _payment_intent: &storage::PaymentIntent,
        _key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<api::ConnectorChoice, errors::ApiErrorResponse> {
        helpers::get_connector_default(state, request.routing.clone()).await
    }

    #[instrument(skip_all)]
    async fn guard_payment_against_blocklist<'a>(
        &'a self,
        _state: &SessionState,
        _merchant_account: &domain::MerchantAccount,
        _key_store: &domain::MerchantKeyStore,
        _payment_data: &mut PaymentData<F>,
    ) -> CustomResult<bool, errors::ApiErrorResponse> {
        Ok(false)
    }
}

#[async_trait]
impl<F: Clone + Sync> UpdateTracker<F, PaymentData<F>, api::PaymentsRequest> for PaymentUpdate {
    #[cfg(all(feature = "v2", feature = "customer_v2"))]
    #[instrument(skip_all)]
    async fn update_trackers<'b>(
        &'b self,
        _state: &'b SessionState,
        req_state: ReqState,
        mut _payment_data: PaymentData<F>,
        _customer: Option<domain::Customer>,
        _storage_scheme: storage_enums::MerchantStorageScheme,
        _updated_customer: Option<storage::CustomerUpdate>,
        _key_store: &domain::MerchantKeyStore,
        _frm_suggestion: Option<FrmSuggestion>,
        _header_payload: hyperswitch_domain_models::payments::HeaderPayload,
    ) -> RouterResult<(PaymentUpdateOperation<'b, F>, PaymentData<F>)>
    where
        F: 'b + Send,
    {
        todo!()
    }

    #[cfg(all(any(feature = "v1", feature = "v2"), not(feature = "customer_v2")))]
    #[instrument(skip_all)]
    async fn update_trackers<'b>(
        &'b self,
        state: &'b SessionState,
        req_state: ReqState,
        mut payment_data: PaymentData<F>,
        customer: Option<domain::Customer>,
        storage_scheme: storage_enums::MerchantStorageScheme,
        _updated_customer: Option<storage::CustomerUpdate>,
        key_store: &domain::MerchantKeyStore,
        _frm_suggestion: Option<FrmSuggestion>,
        _header_payload: hyperswitch_domain_models::payments::HeaderPayload,
    ) -> RouterResult<(PaymentUpdateOperation<'b, F>, PaymentData<F>)>
    where
        F: 'b + Send,
    {
        let is_payment_method_unavailable =
            payment_data.payment_attempt.payment_method_id.is_none()
                && payment_data.payment_intent.status
                    == storage_enums::IntentStatus::RequiresPaymentMethod;

        let payment_method = payment_data.payment_attempt.payment_method;

        let get_attempt_status = || {
            if is_payment_method_unavailable {
                storage_enums::AttemptStatus::PaymentMethodAwaited
            } else {
                storage_enums::AttemptStatus::ConfirmationAwaited
            }
        };
        let profile_id = payment_data
            .payment_intent
            .profile_id
            .as_ref()
            .get_required_value("profile_id")
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("'profile_id' not set in payment intent")?;

        let additional_pm_data = payment_data
            .payment_method_data
            .as_ref()
            .async_map(|payment_method_data| async {
                helpers::get_additional_payment_data(payment_method_data, &*state.store, profile_id)
                    .await
            })
            .await
            .transpose()?
            .flatten();

        let encoded_pm_data = additional_pm_data
            .as_ref()
            .map(Encode::encode_to_value)
            .transpose()
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Failed to encode additional pm data")?;

        let business_sub_label = payment_data.payment_attempt.business_sub_label.clone();

        let payment_method_type = payment_data.payment_attempt.payment_method_type;
        let payment_experience = payment_data.payment_attempt.payment_experience;
        let amount_to_capture = payment_data.payment_attempt.amount_to_capture;
        let capture_method = payment_data.payment_attempt.capture_method;
        let payment_method_billing_address_id = payment_data
            .payment_attempt
            .payment_method_billing_address_id
            .clone();

        let surcharge_amount = payment_data
            .surcharge_details
            .as_ref()
            .map(|surcharge_details| surcharge_details.surcharge_amount);
        let tax_amount = payment_data
            .surcharge_details
            .as_ref()
            .map(|surcharge_details| surcharge_details.tax_on_surcharge_amount);
        payment_data.payment_attempt = state
            .store
            .update_payment_attempt_with_attempt_id(
                payment_data.payment_attempt,
                storage::PaymentAttemptUpdate::Update {
                    currency: payment_data.currency,
                    status: get_attempt_status(),
                    authentication_type: None,
                    payment_method,
                    payment_token: payment_data.token.clone(),
                    payment_method_data: encoded_pm_data,
                    payment_experience,
                    payment_method_type,
                    business_sub_label,
                    amount_to_capture,
                    capture_method,
                    fingerprint_id: None,
                    payment_method_billing_address_id,
                    updated_by: storage_scheme.to_string(),
                    net_amount:
                        hyperswitch_domain_models::payments::payment_attempt::NetAmount::new(
                            payment_data.amount.into(),
                            None,
                            None,
                            surcharge_amount,
                            tax_amount,
                        ),
                },
                storage_scheme,
            )
            .await
            .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)?;

        let customer_id = customer.clone().map(|c| c.customer_id);

        let intent_status = {
            let current_intent_status = payment_data.payment_intent.status;
            if is_payment_method_unavailable {
                storage_enums::IntentStatus::RequiresPaymentMethod
            } else if !payment_data.confirm.unwrap_or(true)
                || current_intent_status == storage_enums::IntentStatus::RequiresCustomerAction
            {
                storage_enums::IntentStatus::RequiresConfirmation
            } else {
                payment_data.payment_intent.status
            }
        };

        let (shipping_address, billing_address) = (
            payment_data.payment_intent.shipping_address_id.clone(),
            payment_data.payment_intent.billing_address_id.clone(),
        );

        let customer_details = payment_data.payment_intent.customer_details.clone();

        let return_url = payment_data.payment_intent.return_url.clone();
        let setup_future_usage = payment_data.payment_intent.setup_future_usage;
        let business_label = payment_data.payment_intent.business_label.clone();
        let business_country = payment_data.payment_intent.business_country;
        let description = payment_data.payment_intent.description.clone();
        let statement_descriptor_name = payment_data
            .payment_intent
            .statement_descriptor_name
            .clone();
        let statement_descriptor_suffix = payment_data
            .payment_intent
            .statement_descriptor_suffix
            .clone();
        let key_manager_state = state.into();
        let billing_details = payment_data
            .address
            .get_payment_billing()
            .async_map(|billing_details| {
                create_encrypted_data(&key_manager_state, key_store, billing_details)
            })
            .await
            .transpose()
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Unable to encrypt billing details")?;

        let shipping_details = payment_data
            .address
            .get_shipping()
            .async_map(|shipping_details| {
                create_encrypted_data(&key_manager_state, key_store, shipping_details)
            })
            .await
            .transpose()
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Unable to encrypt shipping details")?;

        let order_details = payment_data.payment_intent.order_details.clone();
        let metadata = payment_data.payment_intent.metadata.clone();
        let frm_metadata = payment_data.payment_intent.frm_metadata.clone();
        let session_expiry = payment_data.payment_intent.session_expiry;
        let merchant_order_reference_id = payment_data
            .payment_intent
            .merchant_order_reference_id
            .clone();
        payment_data.payment_intent = state
            .store
            .update_payment_intent(
                &state.into(),
                payment_data.payment_intent.clone(),
                storage::PaymentIntentUpdate::Update(Box::new(PaymentIntentUpdateFields {
                    amount: payment_data.amount.into(),
                    currency: payment_data.currency,
                    setup_future_usage,
                    status: intent_status,
                    customer_id: customer_id.clone(),
                    shipping_address_id: shipping_address,
                    billing_address_id: billing_address,
                    return_url,
                    business_country,
                    business_label,
                    description,
                    statement_descriptor_name,
                    statement_descriptor_suffix,
                    order_details,
                    metadata,
                    payment_confirm_source: None,
                    updated_by: storage_scheme.to_string(),
                    fingerprint_id: None,
                    session_expiry,
                    request_external_three_ds_authentication: payment_data
                        .payment_intent
                        .request_external_three_ds_authentication,
                    frm_metadata,
                    customer_details,
                    merchant_order_reference_id,
                    billing_details,
                    shipping_details,
                    is_payment_processor_token_flow: None,
                    tax_details: None,
                })),
                key_store,
                storage_scheme,
            )
            .await
            .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)?;
        let amount = payment_data.amount;
        req_state
            .event_context
            .event(AuditEvent::new(AuditEventType::PaymentUpdate { amount }))
            .with(payment_data.to_event())
            .emit();

        Ok((
            payments::is_confirm(self, payment_data.confirm),
            payment_data,
        ))
    }
}

impl ForeignTryFrom<domain::Customer> for CustomerData {
    type Error = errors::ApiErrorResponse;
    fn foreign_try_from(value: domain::Customer) -> Result<Self, Self::Error> {
        Ok(Self {
            name: value.name.map(|name| name.into_inner()),
            email: value.email.map(Email::from),
            phone: value.phone.map(|ph| ph.into_inner()),
            phone_country_code: value.phone_country_code,
        })
    }
}

impl<F: Send + Clone + Sync> ValidateRequest<F, api::PaymentsRequest, PaymentData<F>>
    for PaymentUpdate
{
    #[instrument(skip_all)]
    fn validate_request<'a, 'b>(
        &'b self,
        request: &api::PaymentsRequest,
        merchant_account: &'a domain::MerchantAccount,
    ) -> RouterResult<(PaymentUpdateOperation<'b, F>, operations::ValidateResult)> {
        helpers::validate_customer_information(request)?;

        if let Some(amount) = request.amount {
            helpers::validate_max_amount(amount)?;
        }
        if let Some(session_expiry) = &request.session_expiry {
            helpers::validate_session_expiry(session_expiry.to_owned())?;
        }
        let payment_id = request
            .payment_id
            .clone()
            .ok_or(report!(errors::ApiErrorResponse::PaymentNotFound))?;

        let request_merchant_id = request.merchant_id.as_ref();
        helpers::validate_merchant_id(merchant_account.get_id(), request_merchant_id)
            .change_context(errors::ApiErrorResponse::InvalidDataFormat {
                field_name: "merchant_id".to_string(),
                expected_format: "merchant_id from merchant account".to_string(),
            })?;

        helpers::validate_request_amount_and_amount_to_capture(
            request.amount,
            request.amount_to_capture,
            request.surcharge_details,
        )
        .change_context(errors::ApiErrorResponse::InvalidDataFormat {
            field_name: "amount_to_capture".to_string(),
            expected_format: "amount_to_capture lesser than or equal to amount".to_string(),
        })?;

        helpers::validate_payment_method_fields_present(request)?;

        let _mandate_type = helpers::validate_mandate(request, false)?;

        helpers::validate_recurring_details_and_token(
            &request.recurring_details,
            &request.payment_token,
            &request.mandate_id,
        )?;

        let _request_straight_through: Option<api::routing::StraightThroughAlgorithm> = request
            .routing
            .clone()
            .map(|val| val.parse_value("RoutingAlgorithm"))
            .transpose()
            .change_context(errors::ApiErrorResponse::InvalidRequestData {
                message: "Invalid straight through routing rules format".to_string(),
            })
            .attach_printable("Invalid straight through routing rules format")?;

        Ok((
            Box::new(self),
            operations::ValidateResult {
                merchant_id: merchant_account.get_id().to_owned(),
                payment_id,
                storage_scheme: merchant_account.storage_scheme,
                requeue: matches!(
                    request.retry_action,
                    Some(api_models::enums::RetryAction::Requeue)
                ),
            },
        ))
    }
}

impl PaymentUpdate {
    fn populate_payment_attempt_with_request(
        payment_attempt: &mut storage::PaymentAttempt,
        request: &api::PaymentsRequest,
    ) {
        request
            .business_sub_label
            .clone()
            .map(|bsl| payment_attempt.business_sub_label.replace(bsl));
        request
            .payment_method_type
            .map(|pmt| payment_attempt.payment_method_type.replace(pmt));
        request
            .payment_experience
            .map(|experience| payment_attempt.payment_experience.replace(experience));
        payment_attempt.amount_to_capture = request
            .amount_to_capture
            .or(payment_attempt.amount_to_capture);
        request
            .capture_method
            .map(|i| payment_attempt.capture_method.replace(i));
    }
    fn populate_payment_intent_with_request(
        payment_intent: &mut storage::PaymentIntent,
        request: &api::PaymentsRequest,
    ) {
        request
            .return_url
            .clone()
            .map(|i| payment_intent.return_url.replace(i.to_string()));

        payment_intent.business_country = request.business_country;

        payment_intent
            .business_label
            .clone_from(&request.business_label);

        request
            .description
            .clone()
            .map(|i| payment_intent.description.replace(i));

        request
            .statement_descriptor_name
            .clone()
            .map(|i| payment_intent.statement_descriptor_name.replace(i));

        request
            .statement_descriptor_suffix
            .clone()
            .map(|i| payment_intent.statement_descriptor_suffix.replace(i));

        request
            .client_secret
            .clone()
            .map(|i| payment_intent.client_secret.replace(i));
    }
}
