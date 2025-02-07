use std::marker::PhantomData;

use api_models::enums::FrmSuggestion;
use async_trait::async_trait;
use common_utils::{ext_traits::AsyncExt, types::keymanager::KeyManagerState};
use error_stack::ResultExt;
use router_derive::PaymentOperation;
use router_env::{instrument, logger, tracing};

use super::{BoxedOperation, Domain, GetTracker, Operation, UpdateTracker, ValidateRequest};
use crate::{
    core::{
        errors::{self, CustomResult, RouterResult, StorageErrorExt},
        payments::{
            helpers, operations, types as payment_types, CustomerDetails, PaymentAddress,
            PaymentData,
        },
    },
    events::audit_events::{AuditEvent, AuditEventType},
    routes::{app::ReqState, SessionState},
    services,
    types::{
        api, domain,
        storage::{self, enums},
    },
    utils::OptionExt,
};

#[derive(Debug, Clone, Copy, PaymentOperation)]
#[operation(operations = "all", flow = "sync")]
pub struct PaymentStatus;

type PaymentStatusOperation<'b, F, R> = BoxedOperation<'b, F, R, PaymentData<F>>;

impl<F: Send + Clone + Sync> Operation<F, api::PaymentsRequest> for PaymentStatus {
    type Data = PaymentData<F>;
    fn to_domain(&self) -> RouterResult<&dyn Domain<F, api::PaymentsRequest, PaymentData<F>>> {
        Ok(self)
    }
    fn to_update_tracker(
        &self,
    ) -> RouterResult<&(dyn UpdateTracker<F, PaymentData<F>, api::PaymentsRequest> + Send + Sync)>
    {
        Ok(self)
    }
}
impl<F: Send + Clone + Sync> Operation<F, api::PaymentsRequest> for &PaymentStatus {
    type Data = PaymentData<F>;
    fn to_domain(&self) -> RouterResult<&dyn Domain<F, api::PaymentsRequest, PaymentData<F>>> {
        Ok(*self)
    }
    fn to_update_tracker(
        &self,
    ) -> RouterResult<&(dyn UpdateTracker<F, PaymentData<F>, api::PaymentsRequest> + Send + Sync)>
    {
        Ok(*self)
    }
}

#[async_trait]
impl<F: Clone + Send + Sync> Domain<F, api::PaymentsRequest, PaymentData<F>> for PaymentStatus {
    #[instrument(skip_all)]
    async fn get_or_create_customer_details<'a>(
        &'a self,
        state: &SessionState,
        payment_data: &mut PaymentData<F>,
        request: Option<CustomerDetails>,
        key_store: &domain::MerchantKeyStore,
        storage_scheme: enums::MerchantStorageScheme,
    ) -> CustomResult<
        (
            PaymentStatusOperation<'a, F, api::PaymentsRequest>,
            Option<domain::Customer>,
        ),
        errors::StorageError,
    > {
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

    #[instrument(skip_all)]
    async fn make_pm_data<'a>(
        &'a self,
        _state: &'a SessionState,
        _payment_data: &mut PaymentData<F>,
        _storage_scheme: enums::MerchantStorageScheme,
        _merchant_key_store: &domain::MerchantKeyStore,
        _customer: &Option<domain::Customer>,
        _business_profile: &domain::Profile,
    ) -> RouterResult<(
        PaymentStatusOperation<'a, F, api::PaymentsRequest>,
        Option<domain::PaymentMethodData>,
        Option<String>,
    )> {
        Ok((Box::new(self), None, None))
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
impl<F: Clone + Sync> UpdateTracker<F, PaymentData<F>, api::PaymentsRequest> for PaymentStatus {
    async fn update_trackers<'b>(
        &'b self,
        _state: &'b SessionState,
        req_state: ReqState,
        payment_data: PaymentData<F>,
        _customer: Option<domain::Customer>,
        _storage_scheme: enums::MerchantStorageScheme,
        _updated_customer: Option<storage::CustomerUpdate>,
        _key_store: &domain::MerchantKeyStore,
        _frm_suggestion: Option<FrmSuggestion>,
        _header_payload: hyperswitch_domain_models::payments::HeaderPayload,
    ) -> RouterResult<(
        PaymentStatusOperation<'b, F, api::PaymentsRequest>,
        PaymentData<F>,
    )>
    where
        F: 'b + Send,
    {
        req_state
            .event_context
            .event(AuditEvent::new(AuditEventType::PaymentStatus))
            .with(payment_data.to_event())
            .emit();

        Ok((Box::new(self), payment_data))
    }
}

#[async_trait]
impl<F: Clone + Sync> UpdateTracker<F, PaymentData<F>, api::PaymentsRetrieveRequest>
    for PaymentStatus
{
    async fn update_trackers<'b>(
        &'b self,
        _state: &'b SessionState,
        req_state: ReqState,
        payment_data: PaymentData<F>,
        _customer: Option<domain::Customer>,
        _storage_scheme: enums::MerchantStorageScheme,
        _updated_customer: Option<storage::CustomerUpdate>,
        _key_store: &domain::MerchantKeyStore,
        _frm_suggestion: Option<FrmSuggestion>,
        _header_payload: hyperswitch_domain_models::payments::HeaderPayload,
    ) -> RouterResult<(
        PaymentStatusOperation<'b, F, api::PaymentsRetrieveRequest>,
        PaymentData<F>,
    )>
    where
        F: 'b + Send,
    {
        req_state
            .event_context
            .event(AuditEvent::new(AuditEventType::PaymentStatus))
            .with(payment_data.to_event())
            .emit();

        Ok((Box::new(self), payment_data))
    }
}

#[async_trait]
impl<F: Send + Clone + Sync> GetTracker<F, PaymentData<F>, api::PaymentsRetrieveRequest>
    for PaymentStatus
{
    #[instrument(skip_all)]
    async fn get_trackers<'a>(
        &'a self,
        state: &'a SessionState,
        payment_id: &api::PaymentIdType,
        request: &api::PaymentsRetrieveRequest,
        merchant_account: &domain::MerchantAccount,
        key_store: &domain::MerchantKeyStore,
        _auth_flow: services::AuthFlow,
        _header_payload: &hyperswitch_domain_models::payments::HeaderPayload,
        platform_merchant_account: Option<&domain::MerchantAccount>,
    ) -> RouterResult<
        operations::GetTrackerResponse<'a, F, api::PaymentsRetrieveRequest, PaymentData<F>>,
    > {
        get_tracker_for_sync(
            payment_id,
            merchant_account,
            key_store,
            state,
            request,
            self,
            merchant_account.storage_scheme,
            platform_merchant_account,
        )
        .await
    }
}

#[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
async fn get_tracker_for_sync<
    'a,
    F: Send + Clone,
    Op: Operation<F, api::PaymentsRetrieveRequest, Data = PaymentData<F>> + 'a + Send + Sync,
>(
    _payment_id: &api::PaymentIdType,
    _merchant_account: &domain::MerchantAccount,
    _key_store: &domain::MerchantKeyStore,
    _state: &SessionState,
    _request: &api::PaymentsRetrieveRequest,
    _operation: Op,
    _storage_scheme: enums::MerchantStorageScheme,
) -> RouterResult<operations::GetTrackerResponse<'a, F, api::PaymentsRetrieveRequest, PaymentData<F>>>
{
    todo!()
}

#[cfg(all(
    any(feature = "v2", feature = "v1"),
    not(feature = "payment_methods_v2")
))]
#[allow(clippy::too_many_arguments)]
async fn get_tracker_for_sync<
    'a,
    F: Send + Clone,
    Op: Operation<F, api::PaymentsRetrieveRequest, Data = PaymentData<F>> + 'a + Send + Sync,
>(
    payment_id: &api::PaymentIdType,
    merchant_account: &domain::MerchantAccount,
    key_store: &domain::MerchantKeyStore,
    state: &SessionState,
    request: &api::PaymentsRetrieveRequest,
    operation: Op,
    storage_scheme: enums::MerchantStorageScheme,
    platform_merchant_account: Option<&domain::MerchantAccount>,
) -> RouterResult<operations::GetTrackerResponse<'a, F, api::PaymentsRetrieveRequest, PaymentData<F>>>
{
    let (payment_intent, mut payment_attempt, currency, amount);

    (payment_intent, payment_attempt) = get_payment_intent_payment_attempt(
        state,
        payment_id,
        merchant_account.get_id(),
        key_store,
        storage_scheme,
        platform_merchant_account,
    )
    .await?;

    helpers::authenticate_client_secret(request.client_secret.as_ref(), &payment_intent)?;

    let payment_id = payment_attempt.payment_id.clone();

    currency = payment_attempt.currency.get_required_value("currency")?;
    amount = payment_attempt.get_total_amount().into();

    let shipping_address = helpers::get_address_by_id(
        state,
        payment_intent.shipping_address_id.clone(),
        key_store,
        &payment_intent.payment_id.clone(),
        merchant_account.get_id(),
        merchant_account.storage_scheme,
    )
    .await?;
    let billing_address = helpers::get_address_by_id(
        state,
        payment_intent.billing_address_id.clone(),
        key_store,
        &payment_intent.payment_id.clone(),
        merchant_account.get_id(),
        merchant_account.storage_scheme,
    )
    .await?;

    let payment_method_billing = helpers::get_address_by_id(
        state,
        payment_attempt.payment_method_billing_address_id.clone(),
        key_store,
        &payment_intent.payment_id.clone(),
        merchant_account.get_id(),
        merchant_account.storage_scheme,
    )
    .await?;

    payment_attempt.encoded_data.clone_from(&request.param);
    let db = &*state.store;
    let key_manager_state = &state.into();
    let attempts = match request.expand_attempts {
        Some(true) => {
            Some(db
                .find_attempts_by_merchant_id_payment_id(merchant_account.get_id(), &payment_id, storage_scheme)
                .await
                .change_context(errors::ApiErrorResponse::PaymentNotFound)
                .attach_printable_lazy(|| {
                    format!("Error while retrieving attempt list for, merchant_id: {:?}, payment_id: {payment_id:?}",merchant_account.get_id())
                })?)
        },
        _ => None,
    };

    let multiple_capture_data = if payment_attempt.multiple_capture_count > Some(0) {
        let captures = db
            .find_all_captures_by_merchant_id_payment_id_authorized_attempt_id(
                &payment_attempt.merchant_id,
                &payment_attempt.payment_id,
                &payment_attempt.attempt_id,
                storage_scheme,
            )
            .await
            .change_context(errors::ApiErrorResponse::PaymentNotFound)
                .attach_printable_lazy(|| {
                    format!("Error while retrieving capture list for, merchant_id: {:?}, payment_id: {payment_id:?}", merchant_account.get_id())
                })?;
        Some(payment_types::MultipleCaptureData::new_for_sync(
            captures,
            request.expand_captures,
        )?)
    } else {
        None
    };

    let refunds = db
        .find_refund_by_payment_id_merchant_id(
            &payment_id,
            merchant_account.get_id(),
            storage_scheme,
        )
        .await
        .change_context(errors::ApiErrorResponse::PaymentNotFound)
        .attach_printable_lazy(|| {
            format!(
                "Failed while getting refund list for, payment_id: {:?}, merchant_id: {:?}",
                &payment_id,
                merchant_account.get_id()
            )
        })?;

    let authorizations = db
        .find_all_authorizations_by_merchant_id_payment_id(merchant_account.get_id(), &payment_id)
        .await
        .change_context(errors::ApiErrorResponse::PaymentNotFound)
        .attach_printable_lazy(|| {
            format!(
                "Failed while getting authorizations list for, payment_id: {:?}, merchant_id: {:?}",
                &payment_id,
                merchant_account.get_id()
            )
        })?;

    let disputes = db
        .find_disputes_by_merchant_id_payment_id(merchant_account.get_id(), &payment_id)
        .await
        .change_context(errors::ApiErrorResponse::PaymentNotFound)
        .attach_printable_lazy(|| {
            format!("Error while retrieving dispute list for, merchant_id: {:?}, payment_id: {payment_id:?}", merchant_account.get_id())
        })?;

    let frm_response = if cfg!(feature = "frm") {
        db.find_fraud_check_by_payment_id(payment_id.to_owned(), merchant_account.get_id().clone())
            .await
            .change_context(errors::ApiErrorResponse::PaymentNotFound)
            .attach_printable_lazy(|| {
                format!("Error while retrieving frm_response, merchant_id: {:?}, payment_id: {payment_id:?}", merchant_account.get_id())
            })
            .ok()
    } else {
        None
    };

    let contains_encoded_data = payment_attempt.encoded_data.is_some();

    let creds_identifier = request
        .merchant_connector_details
        .as_ref()
        .map(|mcd| mcd.creds_identifier.to_owned());
    request
        .merchant_connector_details
        .to_owned()
        .async_map(|mcd| async {
            helpers::insert_merchant_connector_creds_to_config(db, merchant_account.get_id(), mcd)
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
        .find_business_profile_by_profile_id(key_manager_state, key_store, profile_id)
        .await
        .to_not_found_response(errors::ApiErrorResponse::ProfileNotFound {
            id: profile_id.get_string_repr().to_owned(),
        })?;

    let payment_method_info =
        if let Some(ref payment_method_id) = payment_attempt.payment_method_id.clone() {
            match db
                .find_payment_method(
                    &(state.into()),
                    key_store,
                    payment_method_id,
                    storage_scheme,
                )
                .await
            {
                Ok(payment_method) => Some(payment_method),
                Err(error) => {
                    if error.current_context().is_db_not_found() {
                        logger::info!("Payment Method not found in db {:?}", error);
                        None
                    } else {
                        Err(error)
                            .change_context(errors::ApiErrorResponse::InternalServerError)
                            .attach_printable("Error retrieving payment method from db")?
                    }
                }
            }
        } else {
            None
        };

    let merchant_id = payment_intent.merchant_id.clone();
    let authentication = payment_attempt.authentication_id.clone().async_map(|authentication_id| async move {
            db.find_authentication_by_merchant_id_authentication_id(
                    &merchant_id,
                    authentication_id.clone(),
                )
                .await
                .to_not_found_response(errors::ApiErrorResponse::InternalServerError)
                .attach_printable_lazy(|| format!("Error while fetching authentication record with authentication_id {authentication_id}"))
        }).await
        .transpose()?;

    let payment_link_data = payment_intent
        .payment_link_id
        .as_ref()
        .async_map(|id| crate::core::payments::get_payment_link_response_from_id(state, id))
        .await
        .transpose()?;

    let payment_data = PaymentData {
        flow: PhantomData,
        payment_intent,
        currency,
        amount,
        email: None,
        mandate_id: payment_attempt
            .mandate_id
            .clone()
            .map(|id| api_models::payments::MandateIds {
                mandate_id: Some(id),
                mandate_reference_id: None,
            }),
        mandate_connector: None,
        setup_mandate: None,
        customer_acceptance: None,
        token: None,
        address: PaymentAddress::new(
            shipping_address.as_ref().map(From::from),
            billing_address.as_ref().map(From::from),
            payment_method_billing.as_ref().map(From::from),
            business_profile.use_billing_as_payment_method_billing,
        ),
        token_data: None,
        confirm: Some(request.force_sync),
        payment_method_data: None,
        payment_method_info,
        force_sync: Some(
            request.force_sync
                && (helpers::check_force_psync_precondition(payment_attempt.status)
                    || contains_encoded_data),
        ),
        payment_attempt,
        refunds,
        disputes,
        attempts,
        sessions_token: vec![],
        card_cvc: None,
        creds_identifier,
        pm_token: None,
        connector_customer_id: None,
        recurring_mandate_payment_data: None,
        ephemeral_key: None,
        multiple_capture_data,
        redirect_response: None,
        payment_link_data,
        surcharge_details: None,
        frm_message: frm_response,
        incremental_authorization_details: None,
        authorizations,
        authentication,
        recurring_details: None,
        poll_config: None,
        tax_data: None,
        session_id: None,
        service_details: None,
    };

    let get_trackers_response = operations::GetTrackerResponse {
        operation: Box::new(operation),
        customer_details: None,
        payment_data,
        business_profile,
        mandate_type: None,
    };

    Ok(get_trackers_response)
}

impl<F: Send + Clone + Sync> ValidateRequest<F, api::PaymentsRetrieveRequest, PaymentData<F>>
    for PaymentStatus
{
    fn validate_request<'b>(
        &'b self,
        request: &api::PaymentsRetrieveRequest,
        merchant_account: &domain::MerchantAccount,
    ) -> RouterResult<(
        PaymentStatusOperation<'b, F, api::PaymentsRetrieveRequest>,
        operations::ValidateResult,
    )> {
        let request_merchant_id = request.merchant_id.as_ref();
        helpers::validate_merchant_id(merchant_account.get_id(), request_merchant_id)
            .change_context(errors::ApiErrorResponse::InvalidDataFormat {
                field_name: "merchant_id".to_string(),
                expected_format: "merchant_id from merchant account".to_string(),
            })?;

        Ok((
            Box::new(self),
            operations::ValidateResult {
                merchant_id: merchant_account.get_id().to_owned(),
                payment_id: request.resource_id.clone(),
                storage_scheme: merchant_account.storage_scheme,
                requeue: false,
            },
        ))
    }
}

pub async fn get_payment_intent_payment_attempt(
    state: &SessionState,
    payment_id: &api::PaymentIdType,
    merchant_id: &common_utils::id_type::MerchantId,
    key_store: &domain::MerchantKeyStore,
    storage_scheme: enums::MerchantStorageScheme,
    _platform_merchant_account: Option<&domain::MerchantAccount>,
) -> RouterResult<(storage::PaymentIntent, storage::PaymentAttempt)> {
    let key_manager_state: KeyManagerState = state.into();
    let db = &*state.store;
    let get_pi_pa = || async {
        let (pi, pa);
        match payment_id {
            api_models::payments::PaymentIdType::PaymentIntentId(ref id) => {
                pi = db
                    .find_payment_intent_by_payment_id_merchant_id(
                        &key_manager_state,
                        id,
                        merchant_id,
                        key_store,
                        storage_scheme,
                    )
                    .await?;
                pa = db
                    .find_payment_attempt_by_payment_id_merchant_id_attempt_id(
                        &pi.payment_id,
                        merchant_id,
                        pi.active_attempt.get_id().as_str(),
                        storage_scheme,
                    )
                    .await?;
            }
            api_models::payments::PaymentIdType::ConnectorTransactionId(ref id) => {
                pa = db
                    .find_payment_attempt_by_merchant_id_connector_txn_id(
                        merchant_id,
                        id,
                        storage_scheme,
                    )
                    .await?;
                pi = db
                    .find_payment_intent_by_payment_id_merchant_id(
                        &key_manager_state,
                        &pa.payment_id,
                        merchant_id,
                        key_store,
                        storage_scheme,
                    )
                    .await?;
            }
            api_models::payments::PaymentIdType::PaymentAttemptId(ref id) => {
                pa = db
                    .find_payment_attempt_by_attempt_id_merchant_id(id, merchant_id, storage_scheme)
                    .await?;
                pi = db
                    .find_payment_intent_by_payment_id_merchant_id(
                        &key_manager_state,
                        &pa.payment_id,
                        merchant_id,
                        key_store,
                        storage_scheme,
                    )
                    .await?;
            }
            api_models::payments::PaymentIdType::PreprocessingId(ref id) => {
                pa = db
                    .find_payment_attempt_by_preprocessing_id_merchant_id(
                        id,
                        merchant_id,
                        storage_scheme,
                    )
                    .await?;

                pi = db
                    .find_payment_intent_by_payment_id_merchant_id(
                        &key_manager_state,
                        &pa.payment_id,
                        merchant_id,
                        key_store,
                        storage_scheme,
                    )
                    .await?;
            }
        }
        error_stack::Result::<_, errors::DataStorageError>::Ok((pi, pa))
    };

    get_pi_pa()
        .await
        .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)

    // TODO (#7195): Add platform merchant account validation once client_secret auth is solved
}
