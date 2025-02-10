use std::marker::PhantomData;

use api_models::{enums::FrmSuggestion, payments::PaymentsIncrementalAuthorizationRequest};
use async_trait::async_trait;
use common_utils::errors::CustomResult;
use diesel_models::authorization::AuthorizationNew;
use error_stack::{report, ResultExt};
use router_env::{instrument, tracing};

use super::{BoxedOperation, Domain, GetTracker, Operation, UpdateTracker, ValidateRequest};
use crate::{
    core::{
        errors::{self, RouterResult, StorageErrorExt},
        payments::{
            self, helpers, operations, CustomerDetails, IncrementalAuthorizationDetails,
            PaymentAddress,
        },
        utils::ValidatePlatformMerchant,
    },
    routes::{app::ReqState, SessionState},
    services,
    types::{
        api::{self, PaymentIdTypeExt},
        domain,
        storage::{self, enums},
    },
    utils::OptionExt,
};

#[derive(Debug, Clone, Copy, router_derive::PaymentOperation)]
#[operation(operations = "all", flow = "incremental_authorization")]
pub struct PaymentIncrementalAuthorization;

type PaymentIncrementalAuthorizationOperation<'b, F> =
    BoxedOperation<'b, F, PaymentsIncrementalAuthorizationRequest, payments::PaymentData<F>>;

#[async_trait]
impl<F: Send + Clone + Sync>
    GetTracker<F, payments::PaymentData<F>, PaymentsIncrementalAuthorizationRequest>
    for PaymentIncrementalAuthorization
{
    #[instrument(skip_all)]
    async fn get_trackers<'a>(
        &'a self,
        state: &'a SessionState,
        payment_id: &api::PaymentIdType,
        request: &PaymentsIncrementalAuthorizationRequest,
        merchant_account: &domain::MerchantAccount,
        key_store: &domain::MerchantKeyStore,
        _auth_flow: services::AuthFlow,
        _header_payload: &hyperswitch_domain_models::payments::HeaderPayload,
        platform_merchant_account: Option<&domain::MerchantAccount>,
    ) -> RouterResult<
        operations::GetTrackerResponse<
            'a,
            F,
            PaymentsIncrementalAuthorizationRequest,
            payments::PaymentData<F>,
        >,
    > {
        let db = &*state.store;
        let key_manager_state = &state.into();

        let merchant_id = merchant_account.get_id();
        let storage_scheme = merchant_account.storage_scheme;
        let payment_id = payment_id
            .get_payment_intent_id()
            .change_context(errors::ApiErrorResponse::PaymentNotFound)?;

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

        payment_intent
            .validate_platform_merchant(platform_merchant_account.map(|ma| ma.get_id()))?;

        helpers::validate_payment_status_against_allowed_statuses(
            payment_intent.status,
            &[enums::IntentStatus::RequiresCapture],
            "increment authorization",
        )?;

        if payment_intent.incremental_authorization_allowed != Some(true) {
            Err(errors::ApiErrorResponse::PreconditionFailed {
                message:
                    "You cannot increment authorization this payment because it is not allowed for incremental_authorization".to_owned(),
            })?
        }

        let attempt_id = payment_intent.active_attempt.get_id().clone();
        let payment_attempt = db
            .find_payment_attempt_by_payment_id_merchant_id_attempt_id(
                &payment_intent.payment_id,
                merchant_id,
                attempt_id.clone().as_str(),
                storage_scheme,
            )
            .await
            .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)?;

        // Incremental authorization should be performed on an amount greater than the original authorized amount (in this case, greater than the net_amount which is sent for authorization)
        // request.amount is the total amount that should be authorized in incremental authorization which should be greater than the original authorized amount
        if payment_attempt.get_total_amount() > request.amount {
            Err(errors::ApiErrorResponse::PreconditionFailed {
                message: "Amount should be greater than original authorized amount".to_owned(),
            })?
        }

        let currency = payment_attempt.currency.get_required_value("currency")?;
        let amount = payment_attempt.get_total_amount();

        let profile_id = payment_intent
            .profile_id
            .as_ref()
            .get_required_value("profile_id")
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("'profile_id' not set in payment intent")?;

        let business_profile = state
            .store
            .find_business_profile_by_profile_id(key_manager_state, key_store, profile_id)
            .await
            .to_not_found_response(errors::ApiErrorResponse::ProfileNotFound {
                id: profile_id.get_string_repr().to_owned(),
            })?;

        let payment_data = payments::PaymentData {
            flow: PhantomData,
            payment_intent,
            payment_attempt,
            currency,
            amount: amount.into(),
            email: None,
            mandate_id: None,
            mandate_connector: None,
            setup_mandate: None,
            customer_acceptance: None,
            token: None,
            token_data: None,
            address: PaymentAddress::new(None, None, None, None),
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
            incremental_authorization_details: Some(IncrementalAuthorizationDetails {
                additional_amount: request.amount - amount,
                total_amount: request.amount,
                reason: request.reason.clone(),
                authorization_id: None,
            }),
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
impl<F: Clone + Sync>
    UpdateTracker<F, payments::PaymentData<F>, PaymentsIncrementalAuthorizationRequest>
    for PaymentIncrementalAuthorization
{
    #[instrument(skip_all)]
    async fn update_trackers<'b>(
        &'b self,
        state: &'b SessionState,
        _req_state: ReqState,
        mut payment_data: payments::PaymentData<F>,
        _customer: Option<domain::Customer>,
        storage_scheme: enums::MerchantStorageScheme,
        _updated_customer: Option<storage::CustomerUpdate>,
        key_store: &domain::MerchantKeyStore,
        _frm_suggestion: Option<FrmSuggestion>,
        _header_payload: hyperswitch_domain_models::payments::HeaderPayload,
    ) -> RouterResult<(
        PaymentIncrementalAuthorizationOperation<'b, F>,
        payments::PaymentData<F>,
    )>
    where
        F: 'b + Send,
    {
        let new_authorization_count = payment_data
            .payment_intent
            .authorization_count
            .map(|count| count + 1)
            .unwrap_or(1);
        // Create new authorization record
        let authorization_new = AuthorizationNew {
            authorization_id: format!(
                "{}_{}",
                common_utils::generate_id_with_default_len("auth"),
                new_authorization_count
            ),
            merchant_id: payment_data.payment_intent.merchant_id.clone(),
            payment_id: payment_data.payment_intent.payment_id.clone(),
            amount: payment_data
                .incremental_authorization_details
                .clone()
                .map(|details| details.total_amount)
                .ok_or(
                    report!(errors::ApiErrorResponse::InternalServerError).attach_printable(
                        "missing incremental_authorization_details in payment_data",
                    ),
                )?,
            status: common_enums::AuthorizationStatus::Processing,
            error_code: None,
            error_message: None,
            connector_authorization_id: None,
            previously_authorized_amount: payment_data.payment_attempt.get_total_amount(),
        };
        let authorization = state
            .store
            .insert_authorization(authorization_new.clone())
            .await
            .to_duplicate_response(errors::ApiErrorResponse::GenericDuplicateError {
                message: format!(
                    "Authorization with authorization_id {} already exists",
                    authorization_new.authorization_id
                ),
            })
            .attach_printable("failed while inserting new authorization")?;
        // Update authorization_count in payment_intent
        payment_data.payment_intent = state
            .store
            .update_payment_intent(
                &state.into(),
                payment_data.payment_intent.clone(),
                storage::PaymentIntentUpdate::AuthorizationCountUpdate {
                    authorization_count: new_authorization_count,
                },
                key_store,
                storage_scheme,
            )
            .await
            .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)
            .attach_printable("Failed to update authorization_count in Payment Intent")?;
        match &payment_data.incremental_authorization_details {
            Some(details) => {
                payment_data.incremental_authorization_details =
                    Some(IncrementalAuthorizationDetails {
                        authorization_id: Some(authorization.authorization_id),
                        ..details.clone()
                    });
            }
            None => Err(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("missing incremental_authorization_details in payment_data")?,
        }
        Ok((Box::new(self), payment_data))
    }
}

impl<F: Send + Clone + Sync>
    ValidateRequest<F, PaymentsIncrementalAuthorizationRequest, payments::PaymentData<F>>
    for PaymentIncrementalAuthorization
{
    #[instrument(skip_all)]
    fn validate_request<'a, 'b>(
        &'b self,
        request: &PaymentsIncrementalAuthorizationRequest,
        merchant_account: &'a domain::MerchantAccount,
    ) -> RouterResult<(
        PaymentIncrementalAuthorizationOperation<'b, F>,
        operations::ValidateResult,
    )> {
        Ok((
            Box::new(self),
            operations::ValidateResult {
                merchant_id: merchant_account.get_id().to_owned(),
                payment_id: api::PaymentIdType::PaymentIntentId(request.payment_id.to_owned()),
                storage_scheme: merchant_account.storage_scheme,
                requeue: false,
            },
        ))
    }
}

#[async_trait]
impl<F: Clone + Send + Sync>
    Domain<F, PaymentsIncrementalAuthorizationRequest, payments::PaymentData<F>>
    for PaymentIncrementalAuthorization
{
    #[instrument(skip_all)]
    async fn get_or_create_customer_details<'a>(
        &'a self,
        _state: &SessionState,
        _payment_data: &mut payments::PaymentData<F>,
        _request: Option<CustomerDetails>,
        _merchant_key_store: &domain::MerchantKeyStore,
        _storage_scheme: enums::MerchantStorageScheme,
    ) -> CustomResult<
        (
            BoxedOperation<
                'a,
                F,
                PaymentsIncrementalAuthorizationRequest,
                payments::PaymentData<F>,
            >,
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
        _payment_data: &mut payments::PaymentData<F>,
        _storage_scheme: enums::MerchantStorageScheme,
        _merchant_key_store: &domain::MerchantKeyStore,
        _customer: &Option<domain::Customer>,
        _business_profile: &domain::Profile,
    ) -> RouterResult<(
        PaymentIncrementalAuthorizationOperation<'a, F>,
        Option<domain::PaymentMethodData>,
        Option<String>,
    )> {
        Ok((Box::new(self), None, None))
    }

    async fn get_connector<'a>(
        &'a self,
        _merchant_account: &domain::MerchantAccount,
        state: &SessionState,
        _request: &PaymentsIncrementalAuthorizationRequest,
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
        _payment_data: &mut payments::PaymentData<F>,
    ) -> CustomResult<bool, errors::ApiErrorResponse> {
        Ok(false)
    }
}
