use std::marker::PhantomData;

use api_models::enums::FrmSuggestion;
use async_trait::async_trait;
use error_stack::{report, ResultExt};
use router_derive::PaymentOperation;
use router_env::{instrument, tracing};

use super::{BoxedOperation, Domain, GetTracker, Operation, UpdateTracker, ValidateRequest};
use crate::{
    core::{
        errors::{self, CustomResult, RouterResult, StorageErrorExt},
        mandate::helpers as m_helpers,
        payments::{self, helpers, operations, CustomerDetails, PaymentAddress, PaymentData},
        utils as core_utils,
    },
    db::StorageInterface,
    routes::{app::ReqState, AppState},
    services,
    types::{
        api::{self, PaymentIdTypeExt},
        domain,
        storage::{self, enums as storage_enums},
    },
    utils::{self, OptionExt},
};

#[derive(Debug, Clone, Copy, PaymentOperation)]
#[operation(operations = "all", flow = "authorize")]
pub struct CompleteAuthorize;

#[async_trait]
impl<F: Send + Clone> GetTracker<F, PaymentData<F>, api::PaymentsRequest> for CompleteAuthorize {
    #[instrument(skip_all)]
    async fn get_trackers<'a>(
        &'a self,
        state: &'a AppState,
        payment_id: &api::PaymentIdType,
        request: &api::PaymentsRequest,
        merchant_account: &domain::MerchantAccount,
        key_store: &domain::MerchantKeyStore,
        _auth_flow: services::AuthFlow,
        _payment_confirm_source: Option<common_enums::PaymentSource>,
    ) -> RouterResult<operations::GetTrackerResponse<'a, F, api::PaymentsRequest>> {
        let db = &*state.store;
        let merchant_id = &merchant_account.merchant_id;
        let storage_scheme = merchant_account.storage_scheme;
        let (mut payment_intent, mut payment_attempt, currency, amount);

        let payment_id = payment_id
            .get_payment_intent_id()
            .change_context(errors::ApiErrorResponse::PaymentNotFound)?;

        payment_intent = db
            .find_payment_intent_by_payment_id_merchant_id(&payment_id, merchant_id, storage_scheme)
            .await
            .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)?;
        payment_intent.setup_future_usage = request
            .setup_future_usage
            .or(payment_intent.setup_future_usage);

        helpers::authenticate_client_secret(request.client_secret.as_ref(), &payment_intent)?;

        helpers::validate_payment_status_against_not_allowed_statuses(
            &payment_intent.status,
            &[
                storage_enums::IntentStatus::Failed,
                storage_enums::IntentStatus::Succeeded,
            ],
            "confirm",
        )?;

        let browser_info = request
            .browser_info
            .clone()
            .as_ref()
            .map(utils::Encode::encode_to_value)
            .transpose()
            .change_context(errors::ApiErrorResponse::InvalidDataValue {
                field_name: "browser_info",
            })?;

        let recurring_details = request.recurring_details.clone();
        let customer_acceptance = request.customer_acceptance.clone().map(From::from);

        payment_attempt = db
            .find_payment_attempt_by_payment_id_merchant_id_attempt_id(
                &payment_intent.payment_id,
                merchant_id,
                &payment_intent.active_attempt.get_id(),
                storage_scheme,
            )
            .await
            .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)?;

        let mandate_type = m_helpers::get_mandate_type(
            request.mandate_data.clone(),
            request.off_session,
            payment_intent.setup_future_usage,
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
            mandate_type.to_owned(),
            merchant_account,
            key_store,
            payment_attempt.payment_method_id.clone(),
        )
        .await?;
        let token = token.or_else(|| payment_attempt.payment_token.clone());

        if let Some(payment_method) = payment_method {
            let should_validate_pm_or_token_given =
                //this validation should happen if data was stored in the vault
                helpers::should_store_payment_method_data_in_vault(
                    &state.conf.temp_locker_enable_config,
                    payment_attempt.connector.clone(),
                    payment_method,
                );
            if should_validate_pm_or_token_given {
                helpers::validate_pm_or_token_given(
                    &request.payment_method,
                    &request
                        .payment_method_data
                        .as_ref()
                        .and_then(|pmd| pmd.payment_method_data.clone()),
                    &request.payment_method_type,
                    &mandate_type,
                    &token,
                )?;
            }
        }

        let token_data = if let Some((token, payment_method)) = token
            .as_ref()
            .zip(payment_method.or(payment_attempt.payment_method))
        {
            Some(
                helpers::retrieve_payment_token_data(state, token.clone(), Some(payment_method))
                    .await?,
            )
        } else {
            None
        };

        payment_attempt.payment_method = payment_method.or(payment_attempt.payment_method);
        payment_attempt.browser_info = browser_info;
        payment_attempt.payment_method_type =
            payment_method_type.or(payment_attempt.payment_method_type);
        payment_attempt.payment_experience = request
            .payment_experience
            .or(payment_attempt.payment_experience);
        currency = payment_attempt.currency.get_required_value("currency")?;
        amount = payment_attempt.get_total_amount().into();

        helpers::validate_customer_id_mandatory_cases(
            request.setup_future_usage.is_some(),
            &payment_intent
                .customer_id
                .clone()
                .or_else(|| request.customer_id.clone()),
        )?;

        let shipping_address = helpers::create_or_update_address_for_payment_by_request(
            db,
            request.shipping.as_ref(),
            payment_intent.shipping_address_id.clone().as_deref(),
            merchant_id.as_ref(),
            payment_intent.customer_id.as_ref(),
            key_store,
            payment_id.as_ref(),
            storage_scheme,
        )
        .await?;

        payment_intent.shipping_address_id = shipping_address
            .as_ref()
            .map(|shipping_address| shipping_address.address_id.clone());

        let billing_address = helpers::get_address_by_id(
            db,
            payment_intent.billing_address_id.clone(),
            key_store,
            &payment_intent.payment_id,
            merchant_id,
            merchant_account.storage_scheme,
        )
        .await?;

        let payment_method_billing = helpers::get_address_by_id(
            db,
            payment_attempt.payment_method_billing_address_id.clone(),
            key_store,
            &payment_intent.payment_id,
            merchant_id,
            merchant_account.storage_scheme,
        )
        .await?;

        let redirect_response = request
            .feature_metadata
            .as_ref()
            .and_then(|fm| fm.redirect_response.clone());

        payment_intent.shipping_address_id = shipping_address.clone().map(|i| i.address_id);
        payment_intent.billing_address_id = billing_address.clone().map(|i| i.address_id);
        payment_intent.return_url = request
            .return_url
            .as_ref()
            .map(|a| a.to_string())
            .or(payment_intent.return_url);

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
            .find_business_profile_by_profile_id(profile_id)
            .await
            .to_not_found_response(errors::ApiErrorResponse::BusinessProfileNotFound {
                id: profile_id.to_string(),
            })?;

        let payment_data = PaymentData {
            flow: PhantomData,
            payment_intent,
            payment_attempt,
            currency,
            amount,
            email: request.email.clone(),
            mandate_id: None,
            mandate_connector,
            setup_mandate,
            customer_acceptance,
            token,
            token_data,
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
                .and_then(|pmd| pmd.payment_method_data.clone()),
            payment_method_info,
            force_sync: None,
            refunds: vec![],
            disputes: vec![],
            attempts: None,
            sessions_token: vec![],
            card_cvc: request.card_cvc.clone(),
            creds_identifier: None,
            pm_token: None,
            connector_customer_id: None,
            recurring_mandate_payment_data,
            ephemeral_key: None,
            multiple_capture_data: None,
            redirect_response,
            surcharge_details: None,
            frm_message: None,
            payment_link_data: None,
            incremental_authorization_details: None,
            authorizations: vec![],
            authentication: None,
            recurring_details,
            poll_config: None,
        };

        let customer_details = Some(CustomerDetails {
            customer_id: request.customer_id.clone(),
            name: request.name.clone(),
            email: request.email.clone(),
            phone: request.phone.clone(),
            phone_country_code: request.phone_country_code.clone(),
        });

        let get_trackers_response = operations::GetTrackerResponse {
            operation: Box::new(self),
            customer_details,
            payment_data,
            business_profile,
            mandate_type,
        };

        Ok(get_trackers_response)
    }
}

#[async_trait]
impl<F: Clone + Send> Domain<F, api::PaymentsRequest> for CompleteAuthorize {
    #[instrument(skip_all)]
    async fn get_or_create_customer_details<'a>(
        &'a self,
        db: &dyn StorageInterface,
        payment_data: &mut PaymentData<F>,
        request: Option<CustomerDetails>,
        key_store: &domain::MerchantKeyStore,
        storage_scheme: common_enums::enums::MerchantStorageScheme,
    ) -> CustomResult<
        (
            BoxedOperation<'a, F, api::PaymentsRequest>,
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
        storage_scheme: storage_enums::MerchantStorageScheme,
        merchant_key_store: &domain::MerchantKeyStore,
        customer: &Option<domain::Customer>,
    ) -> RouterResult<(
        BoxedOperation<'a, F, api::PaymentsRequest>,
        Option<api::PaymentMethodData>,
        Option<String>,
    )> {
        let (op, payment_method_data, pm_id) = helpers::make_pm_data(
            Box::new(self),
            state,
            payment_data,
            merchant_key_store,
            customer,
            storage_scheme,
        )
        .await?;
        Ok((op, payment_method_data, pm_id))
    }

    #[instrument(skip_all)]
    async fn add_task_to_process_tracker<'a>(
        &'a self,
        _state: &'a AppState,
        _payment_attempt: &storage::PaymentAttempt,
        _requeue: bool,
        _schedule_time: Option<time::PrimitiveDateTime>,
    ) -> CustomResult<(), errors::ApiErrorResponse> {
        Ok(())
    }

    async fn get_connector<'a>(
        &'a self,
        _merchant_account: &domain::MerchantAccount,
        state: &AppState,
        request: &api::PaymentsRequest,
        _payment_intent: &storage::PaymentIntent,
        _key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<api::ConnectorChoice, errors::ApiErrorResponse> {
        // Use a new connector in the confirm call or use the same one which was passed when
        // creating the payment or if none is passed then use the routing algorithm
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
impl<F: Clone> UpdateTracker<F, PaymentData<F>, api::PaymentsRequest> for CompleteAuthorize {
    #[instrument(skip_all)]
    async fn update_trackers<'b>(
        &'b self,
        state: &'b AppState,
        _req_state: ReqState,
        mut payment_data: PaymentData<F>,
        _customer: Option<domain::Customer>,
        storage_scheme: storage_enums::MerchantStorageScheme,
        _updated_customer: Option<storage::CustomerUpdate>,
        _merchant_key_store: &domain::MerchantKeyStore,
        _frm_suggestion: Option<FrmSuggestion>,
        _header_payload: api::HeaderPayload,
    ) -> RouterResult<(BoxedOperation<'b, F, api::PaymentsRequest>, PaymentData<F>)>
    where
        F: 'b + Send,
    {
        let payment_intent_update = hyperswitch_domain_models::payments::payment_intent::PaymentIntentUpdate::CompleteAuthorizeUpdate {
            shipping_address_id: payment_data.payment_intent.shipping_address_id.clone()
        };

        let db = &*state.store;
        let payment_intent = payment_data.payment_intent.clone();

        let updated_payment_intent = db
            .update_payment_intent(payment_intent, payment_intent_update, storage_scheme)
            .await
            .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)?;

        payment_data.payment_intent = updated_payment_intent;
        Ok((Box::new(self), payment_data))
    }
}

impl<F: Send + Clone> ValidateRequest<F, api::PaymentsRequest> for CompleteAuthorize {
    #[instrument(skip_all)]
    fn validate_request<'a, 'b>(
        &'b self,
        request: &api::PaymentsRequest,
        merchant_account: &'a domain::MerchantAccount,
    ) -> RouterResult<(
        BoxedOperation<'b, F, api::PaymentsRequest>,
        operations::ValidateResult<'a>,
    )> {
        let payment_id = request
            .payment_id
            .clone()
            .ok_or(report!(errors::ApiErrorResponse::PaymentNotFound))?;

        let request_merchant_id = request.merchant_id.as_deref();
        helpers::validate_merchant_id(&merchant_account.merchant_id, request_merchant_id)
            .change_context(errors::ApiErrorResponse::InvalidDataFormat {
                field_name: "merchant_id".to_string(),
                expected_format: "merchant_id from merchant account".to_string(),
            })?;

        helpers::validate_payment_method_fields_present(request)?;

        let _mandate_type =
            helpers::validate_mandate(request, payments::is_operation_confirm(self))?;

        helpers::validate_recurring_details_and_token(
            &request.recurring_details,
            &request.payment_token,
            &request.mandate_id,
        )?;

        Ok((
            Box::new(self),
            operations::ValidateResult {
                merchant_id: &merchant_account.merchant_id,
                payment_id: payment_id.and_then(|id| core_utils::validate_id(id, "payment_id"))?,
                storage_scheme: merchant_account.storage_scheme,
                requeue: matches!(
                    request.retry_action,
                    Some(api_models::enums::RetryAction::Requeue)
                ),
            },
        ))
    }
}
