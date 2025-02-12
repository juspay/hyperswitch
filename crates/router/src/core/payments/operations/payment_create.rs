use std::marker::PhantomData;

use api_models::{
    enums::FrmSuggestion, mandates::RecurringDetails, payment_methods::PaymentMethodsData,
    payments::GetAddressFromPaymentMethodData,
};
use async_trait::async_trait;
use common_utils::{
    ext_traits::{AsyncExt, Encode, ValueExt},
    type_name,
    types::{
        keymanager::{Identifier, KeyManagerState, ToEncryptable},
        MinorUnit,
    },
};
use diesel_models::{
    ephemeral_key,
    payment_attempt::ConnectorMandateReferenceId as DieselConnectorMandateReferenceId,
};
use error_stack::{self, ResultExt};
use hyperswitch_domain_models::{
    mandates::{MandateData, MandateDetails},
    payments::{
        payment_attempt::PaymentAttempt, payment_intent::CustomerData,
        FromRequestEncryptablePaymentIntent,
    },
};
use masking::{ExposeInterface, PeekInterface, Secret};
use router_derive::PaymentOperation;
use router_env::{instrument, logger, tracing};
use time::PrimitiveDateTime;

use super::{BoxedOperation, Domain, GetTracker, Operation, UpdateTracker, ValidateRequest};
use crate::{
    consts,
    core::{
        errors::{self, CustomResult, RouterResult, StorageErrorExt},
        mandate::helpers as m_helpers,
        payment_link,
        payment_methods::cards::create_encrypted_data,
        payments::{self, helpers, operations, CustomerDetails, PaymentAddress, PaymentData},
        utils as core_utils,
    },
    db::StorageInterface,
    events::audit_events::{AuditEvent, AuditEventType},
    routes::{app::ReqState, SessionState},
    services,
    types::{
        self,
        api::{self, ConnectorCallType, PaymentIdTypeExt},
        domain,
        storage::{
            self,
            enums::{self, IntentStatus},
        },
        transformers::{ForeignFrom, ForeignTryFrom},
    },
    utils::{self, OptionExt},
};

#[derive(Debug, Clone, Copy, PaymentOperation)]
#[operation(operations = "all", flow = "authorize")]
pub struct PaymentCreate;

type PaymentCreateOperation<'a, F> = BoxedOperation<'a, F, api::PaymentsRequest, PaymentData<F>>;

/// The `get_trackers` function for `PaymentsCreate` is an entrypoint for new payments
/// This will create all the entities required for a new payment from the request
#[async_trait]
impl<F: Send + Clone + Sync> GetTracker<F, PaymentData<F>, api::PaymentsRequest> for PaymentCreate {
    #[instrument(skip_all)]
    async fn get_trackers<'a>(
        &'a self,
        state: &'a SessionState,
        payment_id: &api::PaymentIdType,
        request: &api::PaymentsRequest,
        merchant_account: &domain::MerchantAccount,
        merchant_key_store: &domain::MerchantKeyStore,
        _auth_flow: services::AuthFlow,
        header_payload: &hyperswitch_domain_models::payments::HeaderPayload,
        platform_merchant_account: Option<&domain::MerchantAccount>,
    ) -> RouterResult<operations::GetTrackerResponse<'a, F, api::PaymentsRequest, PaymentData<F>>>
    {
        let db = &*state.store;
        let key_manager_state = &state.into();
        let ephemeral_key = Self::get_ephemeral_key(request, state, merchant_account).await;
        let merchant_id = merchant_account.get_id();
        let storage_scheme = merchant_account.storage_scheme;

        let money @ (amount, currency) = payments_create_request_validation(request)?;

        let payment_id = payment_id
            .get_payment_intent_id()
            .change_context(errors::ApiErrorResponse::PaymentNotFound)?;

        #[cfg(feature = "v1")]
        helpers::validate_business_details(
            request.business_country,
            request.business_label.as_ref(),
            merchant_account,
        )?;

        // If profile id is not passed, get it from the business_country and business_label
        #[cfg(feature = "v1")]
        let profile_id = core_utils::get_profile_id_from_business_details(
            key_manager_state,
            merchant_key_store,
            request.business_country,
            request.business_label.as_ref(),
            merchant_account,
            request.profile_id.as_ref(),
            &*state.store,
            true,
        )
        .await?;

        // Profile id will be mandatory in v2 in the request / headers
        #[cfg(feature = "v2")]
        let profile_id = request
            .profile_id
            .clone()
            .get_required_value("profile_id")
            .attach_printable("Profile id is a mandatory parameter")?;

        // TODO: eliminate a redundant db call to fetch the business profile
        // Validate whether profile_id passed in request is valid and is linked to the merchant
        let business_profile = if let Some(business_profile) =
            core_utils::validate_and_get_business_profile(
                db,
                key_manager_state,
                merchant_key_store,
                Some(&profile_id),
                merchant_id,
            )
            .await?
        {
            business_profile
        } else {
            db.find_business_profile_by_profile_id(
                key_manager_state,
                merchant_key_store,
                &profile_id,
            )
            .await
            .to_not_found_response(errors::ApiErrorResponse::ProfileNotFound {
                id: profile_id.get_string_repr().to_owned(),
            })?
        };
        let customer_acceptance = request.customer_acceptance.clone().map(From::from);

        let recurring_details = request.recurring_details.clone();

        let mandate_type = m_helpers::get_mandate_type(
            request.mandate_data.clone(),
            request.off_session,
            request.setup_future_usage,
            request.customer_acceptance.clone(),
            request.payment_token.clone(),
            request.payment_method,
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
        } = helpers::get_token_pm_type_mandate_details(
            state,
            request,
            mandate_type,
            merchant_account,
            merchant_key_store,
            None,
            None,
        )
        .await?;

        helpers::validate_allowed_payment_method_types_request(
            state,
            &profile_id,
            merchant_account,
            merchant_key_store,
            request.allowed_payment_method_types.clone(),
        )
        .await?;

        let customer_details = helpers::get_customer_details_from_request(request);

        let shipping_address = helpers::create_or_find_address_for_payment_by_request(
            state,
            request.shipping.as_ref(),
            None,
            merchant_id,
            customer_details.customer_id.as_ref(),
            merchant_key_store,
            &payment_id,
            merchant_account.storage_scheme,
        )
        .await?;

        let billing_address = helpers::create_or_find_address_for_payment_by_request(
            state,
            request.billing.as_ref(),
            None,
            merchant_id,
            customer_details.customer_id.as_ref(),
            merchant_key_store,
            &payment_id,
            merchant_account.storage_scheme,
        )
        .await?;

        let payment_method_billing_address =
            helpers::create_or_find_address_for_payment_by_request(
                state,
                request
                    .payment_method_data
                    .as_ref()
                    .and_then(|pmd| pmd.billing.as_ref()),
                None,
                merchant_id,
                customer_details.customer_id.as_ref(),
                merchant_key_store,
                &payment_id,
                merchant_account.storage_scheme,
            )
            .await?;

        let browser_info = request
            .browser_info
            .clone()
            .as_ref()
            .map(Encode::encode_to_value)
            .transpose()
            .change_context(errors::ApiErrorResponse::InvalidDataValue {
                field_name: "browser_info",
            })?;

        let attempt_id = if core_utils::is_merchant_enabled_for_payment_id_as_connector_request_id(
            &state.conf,
            merchant_id,
        ) {
            payment_id.get_string_repr().to_string()
        } else {
            payment_id.get_attempt_id(1)
        };

        let session_expiry =
            common_utils::date_time::now().saturating_add(time::Duration::seconds(
                request.session_expiry.map(i64::from).unwrap_or(
                    business_profile
                        .session_expiry
                        .unwrap_or(consts::DEFAULT_SESSION_EXPIRY),
                ),
            ));

        let payment_link_data = match request.payment_link {
            Some(true) => {
                let merchant_name = merchant_account
                    .merchant_name
                    .clone()
                    .map(|name| name.into_inner().peek().to_owned())
                    .unwrap_or_default();

                let default_domain_name = state.base_url.clone();

                let (payment_link_config, domain_name) =
                    payment_link::get_payment_link_config_based_on_priority(
                        request.payment_link_config.clone(),
                        business_profile.payment_link_config.clone(),
                        merchant_name,
                        default_domain_name,
                        request.payment_link_config_id.clone(),
                    )?;

                create_payment_link(
                    request,
                    payment_link_config,
                    merchant_id,
                    payment_id.clone(),
                    db,
                    amount,
                    request.description.clone(),
                    profile_id.clone(),
                    domain_name,
                    session_expiry,
                    header_payload.locale.clone(),
                )
                .await?
            }
            _ => None,
        };

        let payment_intent_new = Self::make_payment_intent(
            state,
            &payment_id,
            merchant_account,
            merchant_key_store,
            money,
            request,
            shipping_address
                .as_ref()
                .map(|address| address.address_id.clone()),
            payment_link_data.clone(),
            billing_address
                .as_ref()
                .map(|address| address.address_id.clone()),
            attempt_id,
            profile_id.clone(),
            session_expiry,
            platform_merchant_account,
        )
        .await?;

        let (payment_attempt_new, additional_payment_data) = Self::make_payment_attempt(
            &payment_id,
            merchant_id,
            &merchant_account.organization_id,
            money,
            payment_method,
            payment_method_type,
            request,
            browser_info,
            state,
            payment_method_billing_address
                .as_ref()
                .map(|address| address.address_id.clone()),
            &payment_method_info,
            merchant_key_store,
            profile_id,
            &customer_acceptance,
        )
        .await?;

        let payment_intent = db
            .insert_payment_intent(
                key_manager_state,
                payment_intent_new,
                merchant_key_store,
                storage_scheme,
            )
            .await
            .to_duplicate_response(errors::ApiErrorResponse::DuplicatePayment {
                payment_id: payment_id.clone(),
            })?;

        if let Some(order_details) = &request.order_details {
            helpers::validate_order_details_amount(
                order_details.to_owned(),
                payment_intent.amount,
                false,
            )?;
        }

        #[cfg(feature = "v1")]
        let mut payment_attempt = db
            .insert_payment_attempt(payment_attempt_new, storage_scheme)
            .await
            .to_duplicate_response(errors::ApiErrorResponse::DuplicatePayment {
                payment_id: payment_id.clone(),
            })?;

        #[cfg(feature = "v2")]
        let payment_attempt = db
            .insert_payment_attempt(
                key_manager_state,
                merchant_key_store,
                payment_attempt_new,
                storage_scheme,
            )
            .await
            .to_duplicate_response(errors::ApiErrorResponse::DuplicatePayment {
                payment_id: payment_id.clone(),
            })?;

        let mandate_details_present = payment_attempt.mandate_details.is_some();

        helpers::validate_mandate_data_and_future_usage(
            request.setup_future_usage,
            mandate_details_present,
        )?;
        // connector mandate reference update history
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
                    .find_mandate_by_merchant_id_mandate_id(merchant_id, mandate_id, storage_scheme)
                    .await
                    .to_not_found_response(errors::ApiErrorResponse::MandateNotFound);
                Some(mandate.and_then(|mandate_obj| {
                    match (
                        mandate_obj.network_transaction_id,
                        mandate_obj.connector_mandate_ids,
                    ) {
                        (_, Some(connector_mandate_id)) => connector_mandate_id
                        .parse_value("ConnectorMandateId")
                        .change_context(errors::ApiErrorResponse::MandateNotFound)
                        .map(|connector_id: api_models::payments::ConnectorMandateReferenceId| {
                            api_models::payments::MandateIds {
                                mandate_id: Some(mandate_obj.mandate_id),
                                mandate_reference_id: Some(api_models::payments::MandateReferenceId::ConnectorMandateId(
                                api_models::payments::ConnectorMandateReferenceId::new(
                                    connector_id.get_connector_mandate_id(),
                                    connector_id.get_payment_method_id(),
                                    None,
                                    None,
                                    connector_id.get_connector_mandate_request_reference_id(),
                                )
                                ))
                            }
                         }),
                        (Some(network_tx_id), _) => Ok(api_models::payments::MandateIds {
                            mandate_id: Some(mandate_obj.mandate_id),
                            mandate_reference_id: Some(
                                api_models::payments::MandateReferenceId::NetworkMandateId(
                                    network_tx_id,
                                ),
                            ),
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

        let mandate_id = if mandate_id.is_none() {
            request
                .recurring_details
                .as_ref()
                .and_then(|recurring_details| match recurring_details {
                    RecurringDetails::ProcessorPaymentToken(token) => {
                        Some(api_models::payments::MandateIds {
                            mandate_id: None,
                            mandate_reference_id: Some(
                                api_models::payments::MandateReferenceId::ConnectorMandateId(
                                    api_models::payments::ConnectorMandateReferenceId::new(
                                        Some(token.processor_payment_token.clone()),
                                        None,
                                        None,
                                        None,
                                        None,
                                    ),
                                ),
                            ),
                        })
                    }
                    _ => None,
                })
        } else {
            mandate_id
        };
        let operation = payments::if_not_create_change_operation::<_, F>(
            payment_intent.status,
            request.confirm,
            self,
        );

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
        let setup_mandate = mandate_data.map(MandateData::from);

        let surcharge_details = request.surcharge_details.map(|request_surcharge_details| {
            payments::types::SurchargeDetails::from((&request_surcharge_details, &payment_attempt))
        });

        let payment_method_data_after_card_bin_call = request
            .payment_method_data
            .as_ref()
            .and_then(|payment_method_data_from_request| {
                payment_method_data_from_request
                    .payment_method_data
                    .as_ref()
            })
            .zip(additional_payment_data)
            .map(|(payment_method_data, additional_payment_data)| {
                payment_method_data.apply_additional_payment_data(additional_payment_data)
            })
            .transpose()
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Card cobadge check failed due to an invalid card network regex")?;

        let additional_pm_data_from_locker = if let Some(ref pm) = payment_method_info {
            let card_detail_from_locker: Option<api::CardDetailFromLocker> = pm
                .payment_method_data
                .clone()
                .map(|x| x.into_inner().expose())
                .and_then(|v| {
                    v.parse_value("PaymentMethodsData")
                        .map_err(|err| {
                            router_env::logger::info!(
                                "PaymentMethodsData deserialization failed: {:?}",
                                err
                            )
                        })
                        .ok()
                })
                .and_then(|pmd| match pmd {
                    PaymentMethodsData::Card(crd) => Some(api::CardDetailFromLocker::from(crd)),
                    _ => None,
                });

            card_detail_from_locker.map(|card_details| {
                let additional_data = card_details.into();
                api_models::payments::AdditionalPaymentData::Card(Box::new(additional_data))
            })
        } else {
            None
        };
        // Only set `payment_attempt.payment_method_data` if `additional_pm_data_from_locker` is not None
        if let Some(additional_pm_data) = additional_pm_data_from_locker.as_ref() {
            payment_attempt.payment_method_data = Some(
                Encode::encode_to_value(additional_pm_data)
                    .change_context(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable("Failed to encode additional pm data")?,
            );
        }
        let amount = payment_attempt.get_total_amount().into();

        payment_attempt.connector_mandate_detail =
            Some(DieselConnectorMandateReferenceId::foreign_from(
                api_models::payments::ConnectorMandateReferenceId::new(
                    None,
                    None,
                    None, // update_history
                    None, // mandate_metadata
                    Some(common_utils::generate_id_with_len(
                        consts::CONNECTOR_MANDATE_REQUEST_REFERENCE_ID_LENGTH,
                    )), // connector_mandate_request_reference_id
                ),
            ));

        let address = PaymentAddress::new(
            shipping_address.as_ref().map(From::from),
            billing_address.as_ref().map(From::from),
            payment_method_billing_address.as_ref().map(From::from),
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

        let unified_address =
            address.unify_with_payment_method_data_billing(payment_method_data_billing);

        let payment_data = PaymentData {
            flow: PhantomData,
            payment_intent,
            payment_attempt,
            currency,
            amount,
            email: request.email.clone(),
            mandate_id: mandate_id.clone(),
            mandate_connector,
            setup_mandate,
            customer_acceptance,
            token,
            address: unified_address,
            token_data: None,
            confirm: request.confirm,
            payment_method_data: payment_method_data_after_card_bin_call.map(Into::into),
            payment_method_info,
            refunds: vec![],
            disputes: vec![],
            attempts: None,
            force_sync: None,
            sessions_token: vec![],
            card_cvc: request.card_cvc.clone(),
            creds_identifier,
            pm_token: None,
            connector_customer_id: None,
            recurring_mandate_payment_data,
            ephemeral_key,
            multiple_capture_data: None,
            redirect_response: None,
            surcharge_details,
            frm_message: None,
            payment_link_data,
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
            operation,
            customer_details: Some(customer_details),
            payment_data,
            business_profile,
            mandate_type,
        };

        Ok(get_trackers_response)
    }
}

#[async_trait]
impl<F: Clone + Send + Sync> Domain<F, api::PaymentsRequest, PaymentData<F>> for PaymentCreate {
    #[instrument(skip_all)]
    async fn get_or_create_customer_details<'a>(
        &'a self,
        state: &SessionState,
        payment_data: &mut PaymentData<F>,
        request: Option<CustomerDetails>,
        key_store: &domain::MerchantKeyStore,
        storage_scheme: enums::MerchantStorageScheme,
    ) -> CustomResult<(PaymentCreateOperation<'a, F>, Option<domain::Customer>), errors::StorageError>
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
        storage_scheme: enums::MerchantStorageScheme,
        merchant_key_store: &domain::MerchantKeyStore,
        customer: &Option<domain::Customer>,
        business_profile: &domain::Profile,
    ) -> RouterResult<(
        PaymentCreateOperation<'a, F>,
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
        _payment_attempt: &PaymentAttempt,
        _requeue: bool,
        _schedule_time: Option<PrimitiveDateTime>,
    ) -> CustomResult<(), errors::ApiErrorResponse> {
        Ok(())
    }

    async fn get_connector<'a>(
        &'a self,
        _merchant_account: &domain::MerchantAccount,
        state: &SessionState,
        request: &api::PaymentsRequest,
        _payment_intent: &storage::PaymentIntent,
        _merchant_key_store: &domain::MerchantKeyStore,
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
impl<F: Clone + Sync> UpdateTracker<F, PaymentData<F>, api::PaymentsRequest> for PaymentCreate {
    #[instrument(skip_all)]
    async fn update_trackers<'b>(
        &'b self,
        state: &'b SessionState,
        req_state: ReqState,
        mut payment_data: PaymentData<F>,
        customer: Option<domain::Customer>,
        storage_scheme: enums::MerchantStorageScheme,
        _updated_customer: Option<storage::CustomerUpdate>,
        key_store: &domain::MerchantKeyStore,
        _frm_suggestion: Option<FrmSuggestion>,
        _header_payload: hyperswitch_domain_models::payments::HeaderPayload,
    ) -> RouterResult<(PaymentCreateOperation<'b, F>, PaymentData<F>)>
    where
        F: 'b + Send,
    {
        let status = match payment_data.payment_intent.status {
            IntentStatus::RequiresPaymentMethod => match payment_data.payment_method_data {
                Some(_) => Some(IntentStatus::RequiresConfirmation),
                _ => None,
            },
            IntentStatus::RequiresConfirmation => {
                if let Some(true) = payment_data.confirm {
                    //TODO: do this later, request validation should happen before
                    Some(IntentStatus::Processing)
                } else {
                    None
                }
            }
            _ => None,
        };

        let payment_token = payment_data.token.clone();
        let connector = payment_data.payment_attempt.connector.clone();
        let straight_through_algorithm = payment_data
            .payment_attempt
            .straight_through_algorithm
            .clone();
        let authorized_amount = payment_data.payment_attempt.get_total_amount();
        let merchant_connector_id = payment_data.payment_attempt.merchant_connector_id.clone();

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
                storage::PaymentAttemptUpdate::UpdateTrackers {
                    payment_token,
                    connector,
                    straight_through_algorithm,
                    amount_capturable: match payment_data.confirm.unwrap_or(true) {
                        true => Some(authorized_amount),
                        false => None,
                    },
                    surcharge_amount,
                    tax_amount,
                    updated_by: storage_scheme.to_string(),
                    merchant_connector_id,
                },
                storage_scheme,
            )
            .await
            .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)?;

        let customer_id = payment_data.payment_intent.customer_id.clone();

        let raw_customer_details = customer
            .map(|customer| CustomerData::foreign_try_from(customer.clone()))
            .transpose()?;
        let key_manager_state = state.into();
        // Updation of Customer Details for the cases where both customer_id and specific customer
        // details are provided in Payment Create Request
        let customer_details = raw_customer_details
            .clone()
            .async_map(|customer_details| {
                create_encrypted_data(&key_manager_state, key_store, customer_details)
            })
            .await
            .transpose()
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Unable to encrypt customer details")?;

        payment_data.payment_intent = state
            .store
            .update_payment_intent(
                &state.into(),
                payment_data.payment_intent,
                storage::PaymentIntentUpdate::PaymentCreateUpdate {
                    return_url: None,
                    status,
                    customer_id,
                    shipping_address_id: None,
                    billing_address_id: None,
                    customer_details,
                    updated_by: storage_scheme.to_string(),
                },
                key_store,
                storage_scheme,
            )
            .await
            .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)?;
        req_state
            .event_context
            .event(AuditEvent::new(AuditEventType::PaymentCreate))
            .with(payment_data.to_event())
            .emit();

        // payment_data.mandate_id = response.and_then(|router_data| router_data.request.mandate_id);
        Ok((
            payments::is_confirm(self, payment_data.confirm),
            payment_data,
        ))
    }
}

impl<F: Send + Clone + Sync> ValidateRequest<F, api::PaymentsRequest, PaymentData<F>>
    for PaymentCreate
{
    #[instrument(skip_all)]
    fn validate_request<'a, 'b>(
        &'b self,
        request: &api::PaymentsRequest,
        merchant_account: &'a domain::MerchantAccount,
    ) -> RouterResult<(PaymentCreateOperation<'b, F>, operations::ValidateResult)> {
        helpers::validate_customer_information(request)?;

        if let Some(amount) = request.amount {
            helpers::validate_max_amount(amount)?;
        }
        if let Some(session_expiry) = &request.session_expiry {
            helpers::validate_session_expiry(session_expiry.to_owned())?;
        }

        if let Some(payment_link) = &request.payment_link {
            if *payment_link {
                helpers::validate_payment_link_request(request.confirm)?;
            }
        };

        let payment_id = request.payment_id.clone().ok_or(error_stack::report!(
            errors::ApiErrorResponse::PaymentNotFound
        ))?;

        let request_merchant_id = request.merchant_id.as_ref();
        helpers::validate_merchant_id(merchant_account.get_id(), request_merchant_id)
            .change_context(errors::ApiErrorResponse::MerchantAccountNotFound)?;

        helpers::validate_request_amount_and_amount_to_capture(
            request.amount,
            request.amount_to_capture,
            request.surcharge_details,
        )
        .change_context(errors::ApiErrorResponse::InvalidDataFormat {
            field_name: "amount_to_capture".to_string(),
            expected_format: "amount_to_capture lesser than amount".to_string(),
        })?;

        helpers::validate_amount_to_capture_and_capture_method(None, request)?;
        helpers::validate_card_data(
            request
                .payment_method_data
                .as_ref()
                .and_then(|pmd| pmd.payment_method_data.clone()),
        )?;

        helpers::validate_payment_method_fields_present(request)?;

        let mandate_type =
            helpers::validate_mandate(request, payments::is_operation_confirm(self))?;

        helpers::validate_recurring_details_and_token(
            &request.recurring_details,
            &request.payment_token,
            &request.mandate_id,
        )?;

        if request.confirm.unwrap_or(false) {
            helpers::validate_pm_or_token_given(
                &request.payment_method,
                &request
                    .payment_method_data
                    .as_ref()
                    .and_then(|pmd| pmd.payment_method_data.clone()),
                &request.payment_method_type,
                &mandate_type,
                &request.payment_token,
                &request.ctp_service_details,
            )?;

            helpers::validate_customer_id_mandatory_cases(
                request.setup_future_usage.is_some(),
                request.customer_id.as_ref().or(request
                    .customer
                    .as_ref()
                    .map(|customer| customer.id.clone())
                    .as_ref()),
            )?;
        }

        if request.split_payments.is_some() {
            let amount = request.amount.get_required_value("amount")?;
            helpers::validate_platform_request_for_marketplace(
                amount,
                request.split_payments.clone(),
            )?;
        };

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

impl PaymentCreate {
    #[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
    #[instrument(skip_all)]
    #[allow(clippy::too_many_arguments)]
    pub async fn make_payment_attempt(
        payment_id: &common_utils::id_type::PaymentId,
        merchant_id: &common_utils::id_type::MerchantId,
        organization_id: &common_utils::id_type::OrganizationId,
        money: (api::Amount, enums::Currency),
        payment_method: Option<enums::PaymentMethod>,
        payment_method_type: Option<enums::PaymentMethodType>,
        request: &api::PaymentsRequest,
        browser_info: Option<serde_json::Value>,
        state: &SessionState,
        payment_method_billing_address_id: Option<String>,
        payment_method_info: &Option<domain::PaymentMethod>,
        _key_store: &domain::MerchantKeyStore,
        profile_id: common_utils::id_type::ProfileId,
        customer_acceptance: &Option<payments::CustomerAcceptance>,
    ) -> RouterResult<(
        storage::PaymentAttemptNew,
        Option<api_models::payments::AdditionalPaymentData>,
    )> {
        todo!()
    }

    #[cfg(all(
        any(feature = "v1", feature = "v2"),
        not(feature = "payment_methods_v2")
    ))]
    #[instrument(skip_all)]
    #[allow(clippy::too_many_arguments)]
    pub async fn make_payment_attempt(
        payment_id: &common_utils::id_type::PaymentId,
        merchant_id: &common_utils::id_type::MerchantId,
        organization_id: &common_utils::id_type::OrganizationId,
        money: (api::Amount, enums::Currency),
        payment_method: Option<enums::PaymentMethod>,
        payment_method_type: Option<enums::PaymentMethodType>,
        request: &api::PaymentsRequest,
        browser_info: Option<serde_json::Value>,
        state: &SessionState,
        payment_method_billing_address_id: Option<String>,
        payment_method_info: &Option<domain::PaymentMethod>,
        _key_store: &domain::MerchantKeyStore,
        profile_id: common_utils::id_type::ProfileId,
        customer_acceptance: &Option<payments::CustomerAcceptance>,
    ) -> RouterResult<(
        storage::PaymentAttemptNew,
        Option<api_models::payments::AdditionalPaymentData>,
    )> {
        let payment_method_data =
            request
                .payment_method_data
                .as_ref()
                .and_then(|payment_method_data_request| {
                    payment_method_data_request.payment_method_data.as_ref()
                });

        let created_at @ modified_at @ last_synced = Some(common_utils::date_time::now());
        let status = helpers::payment_attempt_status_fsm(payment_method_data, request.confirm);
        let (amount, currency) = (money.0, Some(money.1));

        let mut additional_pm_data = request
            .payment_method_data
            .as_ref()
            .and_then(|payment_method_data_request| {
                payment_method_data_request.payment_method_data.clone()
            })
            .async_map(|payment_method_data| async {
                helpers::get_additional_payment_data(
                    &payment_method_data.into(),
                    &*state.store,
                    &profile_id,
                )
                .await
            })
            .await
            .transpose()?
            .flatten();

        if additional_pm_data.is_none() {
            // If recurring payment is made using payment_method_id, then fetch payment_method_data from retrieved payment_method object
            additional_pm_data = payment_method_info.as_ref().and_then(|pm_info| {
                pm_info
                    .payment_method_data
                    .clone()
                    .map(|x| x.into_inner().expose())
                    .and_then(|v| {
                        serde_json::from_value::<PaymentMethodsData>(v)
                            .map_err(|err| {
                                logger::error!(
                                    "Unable to deserialize payment methods data: {:?}",
                                    err
                                )
                            })
                            .ok()
                    })
                    .and_then(|pmd| match pmd {
                        PaymentMethodsData::Card(card) => {
                            Some(api_models::payments::AdditionalPaymentData::Card(Box::new(
                                api::CardDetailFromLocker::from(card).into(),
                            )))
                        }
                        PaymentMethodsData::WalletDetails(wallet) => match payment_method_type {
                            Some(enums::PaymentMethodType::GooglePay) => {
                                Some(api_models::payments::AdditionalPaymentData::Wallet {
                                    apple_pay: None,
                                    google_pay: Some(wallet.into()),
                                    samsung_pay: None,
                                })
                            }
                            Some(enums::PaymentMethodType::SamsungPay) => {
                                Some(api_models::payments::AdditionalPaymentData::Wallet {
                                    apple_pay: None,
                                    google_pay: None,
                                    samsung_pay: Some(wallet.into()),
                                })
                            }
                            _ => None,
                        },
                        _ => None,
                    })
            });
        };

        let additional_pm_data_value = additional_pm_data
            .as_ref()
            .map(Encode::encode_to_value)
            .transpose()
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Failed to encode additional pm data")?;
        let attempt_id = if core_utils::is_merchant_enabled_for_payment_id_as_connector_request_id(
            &state.conf,
            merchant_id,
        ) {
            payment_id.get_string_repr().to_owned()
        } else {
            payment_id.get_attempt_id(1)
        };

        if request.mandate_data.as_ref().is_some_and(|mandate_data| {
            mandate_data.update_mandate_id.is_some() && mandate_data.mandate_type.is_some()
        }) {
            Err(errors::ApiErrorResponse::InvalidRequestData {message:"Only one field out of 'mandate_type' and 'update_mandate_id' was expected, found both".to_string()})?
        }

        let mandate_data = if let Some(update_id) = request
            .mandate_data
            .as_ref()
            .and_then(|inner| inner.update_mandate_id.clone())
        {
            let mandate_details = MandateDetails {
                update_mandate_id: Some(update_id),
            };
            Some(mandate_details)
        } else {
            None
        };

        let payment_method_type = Option::<enums::PaymentMethodType>::foreign_from((
            payment_method_type,
            additional_pm_data.as_ref(),
        ));

        Ok((
            storage::PaymentAttemptNew {
                payment_id: payment_id.to_owned(),
                merchant_id: merchant_id.to_owned(),
                attempt_id,
                status,
                currency,
                payment_method,
                capture_method: request.capture_method,
                capture_on: request.capture_on,
                confirm: request.confirm.unwrap_or(false),
                created_at,
                modified_at,
                last_synced,
                authentication_type: request.authentication_type,
                browser_info,
                payment_experience: request.payment_experience,
                payment_method_type,
                payment_method_data: additional_pm_data_value,
                amount_to_capture: request.amount_to_capture,
                payment_token: request.payment_token.clone(),
                mandate_id: request.mandate_id.clone(),
                business_sub_label: request.business_sub_label.clone(),
                mandate_details: request
                    .mandate_data
                    .as_ref()
                    .and_then(|inner| inner.mandate_type.clone().map(Into::into)),
                external_three_ds_authentication_attempted: None,
                mandate_data,
                payment_method_billing_address_id,
                net_amount: hyperswitch_domain_models::payments::payment_attempt::NetAmount::from_payments_request(
                    request,
                    MinorUnit::from(amount),
                ),
                save_to_locker: None,
                connector: None,
                error_message: None,
                offer_amount: None,
                payment_method_id: payment_method_info
                    .as_ref()
                    .map(|pm_info| pm_info.get_id().clone()),
                cancellation_reason: None,
                error_code: None,
                connector_metadata: None,
                straight_through_algorithm: None,
                preprocessing_step_id: None,
                error_reason: None,
                connector_response_reference_id: None,
                multiple_capture_count: None,
                amount_capturable: MinorUnit::new(i64::default()),
                updated_by: String::default(),
                authentication_data: None,
                encoded_data: None,
                merchant_connector_id: None,
                unified_code: None,
                unified_message: None,
                fingerprint_id: None,
                authentication_connector: None,
                authentication_id: None,
                client_source: None,
                client_version: None,
                customer_acceptance: customer_acceptance
                    .clone()
                    .map(|customer_acceptance| customer_acceptance.encode_to_value())
                    .transpose()
                    .change_context(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable("Failed to serialize customer_acceptance")?
                    .map(Secret::new),
                organization_id: organization_id.clone(),
                profile_id,
                connector_mandate_detail: None,
                card_discovery: None,
            },
            additional_pm_data,

        ))
    }

    #[instrument(skip_all)]
    #[allow(clippy::too_many_arguments)]
    async fn make_payment_intent(
        state: &SessionState,
        payment_id: &common_utils::id_type::PaymentId,
        merchant_account: &domain::MerchantAccount,
        key_store: &domain::MerchantKeyStore,
        money: (api::Amount, enums::Currency),
        request: &api::PaymentsRequest,
        shipping_address_id: Option<String>,
        payment_link_data: Option<api_models::payments::PaymentLinkResponse>,
        billing_address_id: Option<String>,
        active_attempt_id: String,
        profile_id: common_utils::id_type::ProfileId,
        session_expiry: PrimitiveDateTime,
        platform_merchant_account: Option<&domain::MerchantAccount>,
    ) -> RouterResult<storage::PaymentIntent> {
        let created_at @ modified_at @ last_synced = common_utils::date_time::now();

        let status = helpers::payment_intent_status_fsm(
            request
                .payment_method_data
                .as_ref()
                .and_then(|request_payment_method_data| {
                    request_payment_method_data.payment_method_data.as_ref()
                }),
            request.confirm,
        );
        let client_secret = payment_id.generate_client_secret();
        let (amount, currency) = (money.0, Some(money.1));

        let order_details = request
            .get_order_details_as_value()
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Failed to convert order details to value")?;

        let allowed_payment_method_types = request
            .get_allowed_payment_method_types_as_value()
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Error converting allowed_payment_types to Value")?;

        let connector_metadata = request
            .get_connector_metadata_as_value()
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Error converting connector_metadata to Value")?;

        let feature_metadata = request
            .get_feature_metadata_as_value()
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Error converting feature_metadata to Value")?;

        let payment_link_id = payment_link_data.map(|pl_data| pl_data.payment_link_id);

        let request_incremental_authorization =
            core_utils::get_request_incremental_authorization_value(
                request.request_incremental_authorization,
                request.capture_method,
            )?;

        let split_payments = request.split_payments.clone();

        // Derivation of directly supplied Customer data in our Payment Create Request
        let raw_customer_details = if request.customer_id.is_none()
            && (request.name.is_some()
                || request.email.is_some()
                || request.phone.is_some()
                || request.phone_country_code.is_some())
        {
            Some(CustomerData {
                name: request.name.clone(),
                phone: request.phone.clone(),
                email: request.email.clone(),
                phone_country_code: request.phone_country_code.clone(),
            })
        } else {
            None
        };
        let is_payment_processor_token_flow = request.recurring_details.as_ref().and_then(
            |recurring_details| match recurring_details {
                RecurringDetails::ProcessorPaymentToken(_) => Some(true),
                _ => None,
            },
        );

        let key = key_store.key.get_inner().peek();
        let identifier = Identifier::Merchant(key_store.merchant_id.clone());
        let key_manager_state: KeyManagerState = state.into();

        let shipping_details_encoded = request
            .shipping
            .clone()
            .map(|shipping| Encode::encode_to_value(&shipping).map(Secret::new))
            .transpose()
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Unable to encode billing details to serde_json::Value")?;

        let billing_details_encoded = request
            .billing
            .clone()
            .map(|billing| Encode::encode_to_value(&billing).map(Secret::new))
            .transpose()
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Unable to encode billing details to serde_json::Value")?;

        let customer_details_encoded = raw_customer_details
            .map(|customer| Encode::encode_to_value(&customer).map(Secret::new))
            .transpose()
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Unable to encode shipping details to serde_json::Value")?;

        let encrypted_data = domain::types::crypto_operation(
            &key_manager_state,
            type_name!(storage::PaymentIntent),
            domain::types::CryptoOperation::BatchEncrypt(
                FromRequestEncryptablePaymentIntent::to_encryptable(
                    FromRequestEncryptablePaymentIntent {
                        shipping_details: shipping_details_encoded,
                        billing_details: billing_details_encoded,
                        customer_details: customer_details_encoded,
                    },
                ),
            ),
            identifier.clone(),
            key,
        )
        .await
        .and_then(|val| val.try_into_batchoperation())
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Unable to encrypt data")?;

        let encrypted_data = FromRequestEncryptablePaymentIntent::from_encryptable(encrypted_data)
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Unable to encrypt the payment intent data")?;

        let skip_external_tax_calculation = request.skip_external_tax_calculation;

        let tax_details = request
            .order_tax_amount
            .map(|tax_amount| diesel_models::TaxDetails {
                default: Some(diesel_models::DefaultTax {
                    order_tax_amount: tax_amount,
                }),
                payment_method_type: None,
            });

        Ok(storage::PaymentIntent {
            payment_id: payment_id.to_owned(),
            merchant_id: merchant_account.get_id().to_owned(),
            status,
            amount: MinorUnit::from(amount),
            currency,
            description: request.description.clone(),
            created_at,
            modified_at,
            last_synced: Some(last_synced),
            client_secret: Some(client_secret),
            setup_future_usage: request.setup_future_usage,
            off_session: request.off_session,
            return_url: request.return_url.as_ref().map(|a| a.to_string()),
            shipping_address_id,
            billing_address_id,
            statement_descriptor_name: request.statement_descriptor_name.clone(),
            statement_descriptor_suffix: request.statement_descriptor_suffix.clone(),
            metadata: request.metadata.clone(),
            business_country: request.business_country,
            business_label: request.business_label.clone(),
            active_attempt: hyperswitch_domain_models::RemoteStorageObject::ForeignID(
                active_attempt_id,
            ),
            order_details,
            amount_captured: None,
            customer_id: request.get_customer_id().cloned(),
            connector_id: None,
            allowed_payment_method_types,
            connector_metadata,
            feature_metadata,
            attempt_count: 1,
            profile_id: Some(profile_id),
            merchant_decision: None,
            payment_link_id,
            payment_confirm_source: None,
            surcharge_applicable: None,
            updated_by: merchant_account.storage_scheme.to_string(),
            request_incremental_authorization,
            incremental_authorization_allowed: None,
            authorization_count: None,
            fingerprint_id: None,
            session_expiry: Some(session_expiry),
            request_external_three_ds_authentication: request
                .request_external_three_ds_authentication,
            split_payments,
            frm_metadata: request.frm_metadata.clone(),
            billing_details: encrypted_data.billing_details,
            customer_details: encrypted_data.customer_details,
            merchant_order_reference_id: request.merchant_order_reference_id.clone(),
            shipping_details: encrypted_data.shipping_details,
            is_payment_processor_token_flow,
            organization_id: merchant_account.organization_id.clone(),
            shipping_cost: request.shipping_cost,
            tax_details,
            skip_external_tax_calculation,
            psd2_sca_exemption_type: request.psd2_sca_exemption_type,
            platform_merchant_id: platform_merchant_account
                .map(|platform_merchant_account| platform_merchant_account.get_id().to_owned()),
        })
    }

    #[instrument(skip_all)]
    pub async fn get_ephemeral_key(
        request: &api::PaymentsRequest,
        state: &SessionState,
        merchant_account: &domain::MerchantAccount,
    ) -> Option<ephemeral_key::EphemeralKey> {
        match request.get_customer_id() {
            Some(customer_id) => helpers::make_ephemeral_key(
                state.clone(),
                customer_id.clone(),
                merchant_account.get_id().to_owned().clone(),
            )
            .await
            .ok()
            .and_then(|ek| {
                if let services::ApplicationResponse::Json(ek) = ek {
                    Some(ek)
                } else {
                    None
                }
            }),
            None => None,
        }
    }
}

#[instrument(skip_all)]
pub fn payments_create_request_validation(
    req: &api::PaymentsRequest,
) -> RouterResult<(api::Amount, enums::Currency)> {
    let currency = req.currency.get_required_value("currency")?;
    let amount = req.amount.get_required_value("amount")?;
    Ok((amount, currency))
}

#[allow(clippy::too_many_arguments)]
async fn create_payment_link(
    request: &api::PaymentsRequest,
    payment_link_config: api_models::admin::PaymentLinkConfig,
    merchant_id: &common_utils::id_type::MerchantId,
    payment_id: common_utils::id_type::PaymentId,
    db: &dyn StorageInterface,
    amount: api::Amount,
    description: Option<String>,
    profile_id: common_utils::id_type::ProfileId,
    domain_name: String,
    session_expiry: PrimitiveDateTime,
    locale: Option<String>,
) -> RouterResult<Option<api_models::payments::PaymentLinkResponse>> {
    let created_at @ last_modified_at = Some(common_utils::date_time::now());
    let payment_link_id = utils::generate_id(consts::ID_LENGTH, "plink");
    let locale_str = locale.unwrap_or("en".to_owned());
    let open_payment_link = format!(
        "{}/payment_link/{}/{}?locale={}",
        domain_name,
        merchant_id.get_string_repr(),
        payment_id.get_string_repr(),
        locale_str.clone(),
    );

    let secure_link = payment_link_config.allowed_domains.as_ref().map(|_| {
        format!(
            "{}/payment_link/s/{}/{}?locale={}",
            domain_name,
            merchant_id.get_string_repr(),
            payment_id.get_string_repr(),
            locale_str,
        )
    });

    let payment_link_config_encoded_value = payment_link_config.encode_to_value().change_context(
        errors::ApiErrorResponse::InvalidDataValue {
            field_name: "payment_link_config",
        },
    )?;

    let payment_link_req = storage::PaymentLinkNew {
        payment_link_id: payment_link_id.clone(),
        payment_id: payment_id.clone(),
        merchant_id: merchant_id.clone(),
        link_to_pay: open_payment_link.clone(),
        amount: MinorUnit::from(amount),
        currency: request.currency,
        created_at,
        last_modified_at,
        fulfilment_time: Some(session_expiry),
        custom_merchant_name: Some(payment_link_config.seller_name),
        description,
        payment_link_config: Some(payment_link_config_encoded_value),
        profile_id: Some(profile_id),
        secure_link,
    };
    let payment_link_db = db
        .insert_payment_link(payment_link_req)
        .await
        .to_duplicate_response(errors::ApiErrorResponse::GenericDuplicateError {
            message: "payment link already exists!".to_string(),
        })?;

    Ok(Some(api_models::payments::PaymentLinkResponse {
        link: payment_link_db.link_to_pay.clone(),
        secure_link: payment_link_db.secure_link,
        payment_link_id: payment_link_db.payment_link_id,
    }))
}
