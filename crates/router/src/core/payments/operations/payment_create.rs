use std::marker::PhantomData;

use api_models::{
    enums::FrmSuggestion, mandates::RecurringDetails, payment_methods::PaymentMethodsData,
};
use async_trait::async_trait;
use common_utils::ext_traits::{AsyncExt, Encode, ValueExt};
use data_models::{
    mandates::{MandateData, MandateDetails},
    payments::payment_attempt::PaymentAttempt,
};
use diesel_models::{ephemeral_key, PaymentMethod};
use error_stack::{self, ResultExt};
use masking::{ExposeInterface, PeekInterface};
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
        payment_methods::PaymentMethodRetrieve,
        payments::{self, helpers, operations, CustomerDetails, PaymentAddress, PaymentData},
        utils as core_utils,
    },
    db::StorageInterface,
    routes::{app::ReqState, AppState},
    services,
    types::{
        self,
        api::{self, PaymentIdTypeExt},
        domain,
        storage::{
            self,
            enums::{self, IntentStatus},
        },
    },
    utils::{self, OptionExt},
};

#[derive(Debug, Clone, Copy, PaymentOperation)]
#[operation(operations = "all", flow = "authorize")]
pub struct PaymentCreate;

/// The `get_trackers` function for `PaymentsCreate` is an entrypoint for new payments
/// This will create all the entities required for a new payment from the request
#[async_trait]
impl<F: Send + Clone, Ctx: PaymentMethodRetrieve>
    GetTracker<F, PaymentData<F>, api::PaymentsRequest, Ctx> for PaymentCreate
{
    #[instrument(skip_all)]
    async fn get_trackers<'a>(
        &'a self,
        state: &'a AppState,
        payment_id: &api::PaymentIdType,
        request: &api::PaymentsRequest,
        merchant_account: &domain::MerchantAccount,
        merchant_key_store: &domain::MerchantKeyStore,
        _auth_flow: services::AuthFlow,
        _payment_confirm_source: Option<common_enums::PaymentSource>,
    ) -> RouterResult<operations::GetTrackerResponse<'a, F, api::PaymentsRequest, Ctx>> {
        let db = &*state.store;
        let ephemeral_key = Self::get_ephemeral_key(request, state, merchant_account).await;
        let merchant_id = &merchant_account.merchant_id;
        let storage_scheme = merchant_account.storage_scheme;
        let (payment_intent, payment_attempt);

        let money @ (amount, currency) = payments_create_request_validation(request)?;

        let payment_id = payment_id
            .get_payment_intent_id()
            .change_context(errors::ApiErrorResponse::PaymentNotFound)?;

        helpers::validate_business_details(
            request.business_country,
            request.business_label.as_ref(),
            merchant_account,
        )?;

        // If profile id is not passed, get it from the business_country and business_label
        let profile_id = core_utils::get_profile_id_from_business_details(
            request.business_country,
            request.business_label.as_ref(),
            merchant_account,
            request.profile_id.as_ref(),
            &*state.store,
            true,
        )
        .await?;

        // Validate whether profile_id passed in request is valid and is linked to the merchant
        let business_profile = if let Some(business_profile) =
            core_utils::validate_and_get_business_profile(db, Some(&profile_id), merchant_id)
                .await?
        {
            business_profile
        } else {
            db.find_business_profile_by_profile_id(&profile_id)
                .await
                .to_not_found_response(errors::ApiErrorResponse::BusinessProfileNotFound {
                    id: profile_id.to_string(),
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
            mandate_type.clone(),
            merchant_account,
            merchant_key_store,
            None,
        )
        .await?;

        let customer_details = helpers::get_customer_details_from_request(request);

        let shipping_address = helpers::create_or_find_address_for_payment_by_request(
            db,
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
            db,
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
                db,
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
            payment_id.to_string()
        } else {
            utils::get_payment_attempt_id(payment_id.clone(), 1)
        };

        let session_expiry =
            common_utils::date_time::now().saturating_add(time::Duration::seconds(
                request.session_expiry.map(i64::from).unwrap_or(
                    business_profile
                        .session_expiry
                        .unwrap_or(consts::DEFAULT_SESSION_EXPIRY),
                ),
            ));

        let payment_link_data = if let Some(payment_link_create) = request.payment_link {
            if payment_link_create {
                let merchant_name = merchant_account
                    .merchant_name
                    .clone()
                    .map(|merchant_name| merchant_name.into_inner().peek().to_owned())
                    .unwrap_or_default();

                let default_domain_name = state.conf.server.base_url.clone();

                let (payment_link_config, domain_name) =
                    payment_link::get_payment_link_config_based_on_priority(
                        request.payment_link_config.clone(),
                        business_profile.payment_link_config.clone(),
                        merchant_name,
                        default_domain_name,
                    )?;
                create_payment_link(
                    request,
                    payment_link_config,
                    merchant_id.clone(),
                    payment_id.clone(),
                    db,
                    amount,
                    request.description.clone(),
                    profile_id.clone(),
                    domain_name,
                    session_expiry,
                )
                .await?
            } else {
                None
            }
        } else {
            None
        };

        let payment_intent_new = Self::make_payment_intent(
            &payment_id,
            merchant_account,
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
        )
        .await?;

        let (payment_attempt_new, additional_payment_data) = Self::make_payment_attempt(
            &payment_id,
            merchant_id,
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
        )
        .await?;

        payment_intent = db
            .insert_payment_intent(payment_intent_new, storage_scheme)
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

        payment_attempt = db
            .insert_payment_attempt(payment_attempt_new, storage_scheme)
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
                                api_models::payments::ConnectorMandateReferenceId{
                                    connector_mandate_id: connector_id.connector_mandate_id,
                                    payment_method_id: connector_id.payment_method_id,
                                    update_history: None
                                }
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

        let operation = payments::if_not_create_change_operation::<_, F, Ctx>(
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
                    merchant_account.merchant_id.as_str(),
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
            .zip(additional_payment_data)
            .map(|(payment_method_data, additional_payment_data)| {
                payment_method_data
                    .payment_method_data
                    .apply_additional_payment_data(additional_payment_data)
            });

        let amount = payment_attempt.get_total_amount().into();

        let payment_data = PaymentData {
            flow: PhantomData,
            payment_intent,
            payment_attempt,
            currency,
            amount,
            email: request.email.clone(),
            mandate_id,
            mandate_connector,
            setup_mandate,
            customer_acceptance,
            token,
            address: PaymentAddress::new(
                shipping_address.as_ref().map(From::from),
                billing_address.as_ref().map(From::from),
                payment_method_billing_address.as_ref().map(From::from),
            ),
            token_data: None,
            confirm: request.confirm,
            payment_method_data: payment_method_data_after_card_bin_call,
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
            frm_metadata: request.frm_metadata.clone(),
            recurring_details,
            poll_config: None,
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
impl<F: Clone + Send, Ctx: PaymentMethodRetrieve> Domain<F, api::PaymentsRequest, Ctx>
    for PaymentCreate
{
    #[instrument(skip_all)]
    async fn get_or_create_customer_details<'a>(
        &'a self,
        db: &dyn StorageInterface,
        payment_data: &mut PaymentData<F>,
        request: Option<CustomerDetails>,
        key_store: &domain::MerchantKeyStore,
        storage_scheme: enums::MerchantStorageScheme,
    ) -> CustomResult<
        (
            BoxedOperation<'a, F, api::PaymentsRequest, Ctx>,
            Option<domain::Customer>,
        ),
        errors::StorageError,
    > {
        helpers::create_customer_if_not_exist(
            Box::new(self),
            db,
            payment_data,
            request,
            &key_store.merchant_id,
            key_store,
            storage_scheme,
        )
        .await
    }

    #[instrument(skip_all)]
    async fn make_pm_data<'a>(
        &'a self,
        state: &'a AppState,
        payment_data: &mut PaymentData<F>,
        storage_scheme: enums::MerchantStorageScheme,
        merchant_key_store: &domain::MerchantKeyStore,
        customer: &Option<domain::Customer>,
    ) -> RouterResult<(
        BoxedOperation<'a, F, api::PaymentsRequest, Ctx>,
        Option<api::PaymentMethodData>,
        Option<String>,
    )> {
        helpers::make_pm_data(
            Box::new(self),
            state,
            payment_data,
            merchant_key_store,
            customer,
            storage_scheme,
        )
        .await
    }

    #[instrument(skip_all)]
    async fn add_task_to_process_tracker<'a>(
        &'a self,
        _state: &'a AppState,
        _payment_attempt: &PaymentAttempt,
        _requeue: bool,
        _schedule_time: Option<PrimitiveDateTime>,
    ) -> CustomResult<(), errors::ApiErrorResponse> {
        Ok(())
    }

    async fn get_connector<'a>(
        &'a self,
        _merchant_account: &domain::MerchantAccount,
        state: &AppState,
        request: &api::PaymentsRequest,
        _payment_intent: &storage::PaymentIntent,
        _merchant_key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<api::ConnectorChoice, errors::ApiErrorResponse> {
        helpers::get_connector_default(state, request.routing.clone()).await
    }

    #[instrument(skip_all)]
    async fn guard_payment_against_blocklist<'a>(
        &'a self,
        _state: &AppState,
        _merchant_account: &domain::MerchantAccount,
        _payment_data: &mut PaymentData<F>,
    ) -> CustomResult<bool, errors::ApiErrorResponse> {
        Ok(false)
    }
}

#[async_trait]
impl<F: Clone, Ctx: PaymentMethodRetrieve>
    UpdateTracker<F, PaymentData<F>, api::PaymentsRequest, Ctx> for PaymentCreate
{
    #[instrument(skip_all)]
    async fn update_trackers<'b>(
        &'b self,
        state: &'b AppState,
        _req_state: ReqState,
        mut payment_data: PaymentData<F>,
        _customer: Option<domain::Customer>,
        storage_scheme: enums::MerchantStorageScheme,
        _updated_customer: Option<storage::CustomerUpdate>,
        _merchant_key_store: &domain::MerchantKeyStore,
        _frm_suggestion: Option<FrmSuggestion>,
        _header_payload: api::HeaderPayload,
    ) -> RouterResult<(
        BoxedOperation<'b, F, api::PaymentsRequest, Ctx>,
        PaymentData<F>,
    )>
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
        let authorized_amount = payment_data.payment_attempt.amount;
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

        payment_data.payment_intent = state
            .store
            .update_payment_intent(
                payment_data.payment_intent,
                storage::PaymentIntentUpdate::ReturnUrlUpdate {
                    return_url: None,
                    status,
                    customer_id,
                    shipping_address_id: None,
                    billing_address_id: None,
                    updated_by: storage_scheme.to_string(),
                },
                storage_scheme,
            )
            .await
            .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)?;

        // payment_data.mandate_id = response.and_then(|router_data| router_data.request.mandate_id);
        Ok((
            payments::is_confirm(self, payment_data.confirm),
            payment_data,
        ))
    }
}

impl<F: Send + Clone, Ctx: PaymentMethodRetrieve> ValidateRequest<F, api::PaymentsRequest, Ctx>
    for PaymentCreate
{
    #[instrument(skip_all)]
    fn validate_request<'a, 'b>(
        &'b self,
        request: &api::PaymentsRequest,
        merchant_account: &'a domain::MerchantAccount,
    ) -> RouterResult<(
        BoxedOperation<'b, F, api::PaymentsRequest, Ctx>,
        operations::ValidateResult<'a>,
    )> {
        helpers::validate_customer_details_in_request(request)?;
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

        let request_merchant_id = request.merchant_id.as_deref();
        helpers::validate_merchant_id(&merchant_account.merchant_id, request_merchant_id)
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
                .map(|pmd| pmd.payment_method_data.clone()),
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
                    .map(|pmd| pmd.payment_method_data.clone()),
                &request.payment_method_type,
                &mandate_type,
                &request.payment_token,
            )?;

            helpers::validate_customer_id_mandatory_cases(
                request.setup_future_usage.is_some(),
                &request
                    .customer
                    .clone()
                    .map(|customer| customer.id)
                    .or(request.customer_id.clone()),
            )?;
        }

        Ok((
            Box::new(self),
            operations::ValidateResult {
                merchant_id: &merchant_account.merchant_id,
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
    #[instrument(skip_all)]
    #[allow(clippy::too_many_arguments)]
    pub async fn make_payment_attempt(
        payment_id: &str,
        merchant_id: &str,
        money: (api::Amount, enums::Currency),
        payment_method: Option<enums::PaymentMethod>,
        payment_method_type: Option<enums::PaymentMethodType>,
        request: &api::PaymentsRequest,
        browser_info: Option<serde_json::Value>,
        state: &AppState,
        payment_method_billing_address_id: Option<String>,
        payment_method_info: &Option<PaymentMethod>,
        key_store: &domain::MerchantKeyStore,
        profile_id: String,
    ) -> RouterResult<(
        storage::PaymentAttemptNew,
        Option<api_models::payments::AdditionalPaymentData>,
    )> {
        let created_at @ modified_at @ last_synced = Some(common_utils::date_time::now());
        let status =
            helpers::payment_attempt_status_fsm(&request.payment_method_data, request.confirm);
        let (amount, currency) = (money.0, Some(money.1));

        let mut additional_pm_data = request
            .payment_method_data
            .as_ref()
            .async_map(|payment_method_data| async {
                helpers::get_additional_payment_data(
                    &payment_method_data.payment_method_data,
                    &*state.store,
                    &profile_id,
                )
                .await
            })
            .await;

        if additional_pm_data.is_none() {
            // If recurring payment is made using payment_method_id, then fetch payment_method_data from retrieved payment_method object
            additional_pm_data = payment_method_info
                .as_ref()
                .async_map(|pm_info| async {
                    domain::types::decrypt::<serde_json::Value, masking::WithType>(
                        pm_info.payment_method_data.clone(),
                        key_store.key.get_inner().peek(),
                    )
                    .await
                    .map_err(|err| logger::error!("Failed to decrypt card details: {:?}", err))
                    .ok()
                    .flatten()
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
                        PaymentMethodsData::Card(crd) => Some(api::CardDetailFromLocker::from(crd)),
                        _ => None,
                    })
                })
                .await
                .flatten()
                .map(|card| {
                    api_models::payments::AdditionalPaymentData::Card(Box::new(card.into()))
                })
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
            payment_id.to_string()
        } else {
            utils::get_payment_attempt_id(payment_id, 1)
        };
        let surcharge_amount = request
            .surcharge_details
            .map(|surcharge_details| surcharge_details.surcharge_amount);
        let tax_amount = request
            .surcharge_details
            .and_then(|surcharge_details| surcharge_details.tax_amount);

        if request.mandate_data.as_ref().map_or(false, |mandate_data| {
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

        Ok((
            storage::PaymentAttemptNew {
                payment_id: payment_id.to_string(),
                merchant_id: merchant_id.to_string(),
                attempt_id,
                status,
                currency,
                amount: amount.into(),
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
                surcharge_amount,
                tax_amount,
                mandate_details: request
                    .mandate_data
                    .as_ref()
                    .and_then(|inner| inner.mandate_type.clone().map(Into::into)),
                external_three_ds_authentication_attempted: None,
                mandate_data,
                payment_method_billing_address_id,
                net_amount: i64::default(),
                save_to_locker: None,
                connector: None,
                error_message: None,
                offer_amount: None,
                payment_method_id: payment_method_info
                    .as_ref()
                    .map(|pm_info| pm_info.payment_method_id.clone()),
                cancellation_reason: None,
                error_code: None,
                connector_metadata: None,
                straight_through_algorithm: None,
                preprocessing_step_id: None,
                error_reason: None,
                connector_response_reference_id: None,
                multiple_capture_count: None,
                amount_capturable: i64::default(),
                updated_by: String::default(),
                authentication_data: None,
                encoded_data: None,
                merchant_connector_id: None,
                unified_code: None,
                unified_message: None,
                fingerprint_id: None,
                authentication_connector: None,
                authentication_id: None,
            },
            additional_pm_data,
        ))
    }

    #[instrument(skip_all)]
    #[allow(clippy::too_many_arguments)]
    async fn make_payment_intent(
        payment_id: &str,
        merchant_account: &types::domain::MerchantAccount,
        money: (api::Amount, enums::Currency),
        request: &api::PaymentsRequest,
        shipping_address_id: Option<String>,
        payment_link_data: Option<api_models::payments::PaymentLinkResponse>,
        billing_address_id: Option<String>,
        active_attempt_id: String,
        profile_id: String,
        session_expiry: PrimitiveDateTime,
    ) -> RouterResult<storage::PaymentIntentNew> {
        let created_at @ modified_at @ last_synced = Some(common_utils::date_time::now());
        let status =
            helpers::payment_intent_status_fsm(&request.payment_method_data, request.confirm);
        let client_secret =
            crate::utils::generate_id(consts::ID_LENGTH, format!("{payment_id}_secret").as_str());
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

        Ok(storage::PaymentIntentNew {
            payment_id: payment_id.to_string(),
            merchant_id: merchant_account.merchant_id.to_string(),
            status,
            amount: amount.into(),
            currency,
            description: request.description.clone(),
            created_at,
            modified_at,
            last_synced,
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
            active_attempt: data_models::RemoteStorageObject::ForeignID(active_attempt_id),
            order_details,
            amount_captured: None,
            customer_id: None,
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
        })
    }

    #[instrument(skip_all)]
    pub async fn get_ephemeral_key(
        request: &api::PaymentsRequest,
        state: &AppState,
        merchant_account: &domain::MerchantAccount,
    ) -> Option<ephemeral_key::EphemeralKey> {
        match request.customer_id.clone() {
            Some(customer_id) => helpers::make_ephemeral_key(
                state.clone(),
                customer_id,
                merchant_account.merchant_id.clone(),
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
    merchant_id: String,
    payment_id: String,
    db: &dyn StorageInterface,
    amount: api::Amount,
    description: Option<String>,
    profile_id: String,
    domain_name: String,
    session_expiry: PrimitiveDateTime,
) -> RouterResult<Option<api_models::payments::PaymentLinkResponse>> {
    let created_at @ last_modified_at = Some(common_utils::date_time::now());
    let payment_link_id = utils::generate_id(consts::ID_LENGTH, "plink");
    let payment_link = format!(
        "{}/payment_link/{}/{}",
        domain_name,
        merchant_id.clone(),
        payment_id.clone()
    );

    let payment_link_config_encoded_value = payment_link_config.encode_to_value().change_context(
        errors::ApiErrorResponse::InvalidDataValue {
            field_name: "payment_link_config",
        },
    )?;

    let payment_link_req = storage::PaymentLinkNew {
        payment_link_id: payment_link_id.clone(),
        payment_id: payment_id.clone(),
        merchant_id: merchant_id.clone(),
        link_to_pay: payment_link.clone(),
        amount: amount.into(),
        currency: request.currency,
        created_at,
        last_modified_at,
        fulfilment_time: Some(session_expiry),
        custom_merchant_name: Some(payment_link_config.seller_name),
        description,
        payment_link_config: Some(payment_link_config_encoded_value),
        profile_id: Some(profile_id),
    };
    let payment_link_db = db
        .insert_payment_link(payment_link_req)
        .await
        .to_duplicate_response(errors::ApiErrorResponse::GenericDuplicateError {
            message: "payment link already exists!".to_string(),
        })?;

    Ok(Some(api_models::payments::PaymentLinkResponse {
        link: payment_link_db.link_to_pay,
        payment_link_id: payment_link_db.payment_link_id,
    }))
}
