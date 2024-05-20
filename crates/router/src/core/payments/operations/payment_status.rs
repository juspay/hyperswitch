use std::marker::PhantomData;

use api_models::enums::FrmSuggestion;
use async_trait::async_trait;
use common_utils::ext_traits::AsyncExt;
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
    db::StorageInterface,
    routes::{app::ReqState, AppState},
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

impl<F: Send + Clone> Operation<F, api::PaymentsRequest> for PaymentStatus {
    fn to_domain(&self) -> RouterResult<&dyn Domain<F, api::PaymentsRequest>> {
        Ok(self)
    }
    fn to_update_tracker(
        &self,
    ) -> RouterResult<&(dyn UpdateTracker<F, PaymentData<F>, api::PaymentsRequest> + Send + Sync)>
    {
        Ok(self)
    }
}
impl<F: Send + Clone> Operation<F, api::PaymentsRequest> for &PaymentStatus {
    fn to_domain(&self) -> RouterResult<&dyn Domain<F, api::PaymentsRequest>> {
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
impl<F: Clone + Send> Domain<F, api::PaymentsRequest> for PaymentStatus {
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
        storage_scheme: enums::MerchantStorageScheme,
        merchant_key_store: &domain::MerchantKeyStore,
        customer: &Option<domain::Customer>,
    ) -> RouterResult<(
        BoxedOperation<'a, F, api::PaymentsRequest>,
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
        state: &'a AppState,
        payment_attempt: &storage::PaymentAttempt,
        requeue: bool,
        schedule_time: Option<time::PrimitiveDateTime>,
    ) -> CustomResult<(), errors::ApiErrorResponse> {
        helpers::add_domain_task_to_pt(self, state, payment_attempt, requeue, schedule_time).await
    }

    async fn get_connector<'a>(
        &'a self,
        _merchant_account: &domain::MerchantAccount,
        state: &AppState,
        request: &api::PaymentsRequest,
        _payment_intent: &storage::PaymentIntent,
        _key_store: &domain::MerchantKeyStore,
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
impl<F: Clone> UpdateTracker<F, PaymentData<F>, api::PaymentsRequest> for PaymentStatus {
    async fn update_trackers<'b>(
        &'b self,
        _state: &'b AppState,
        _req_state: ReqState,
        payment_data: PaymentData<F>,
        _customer: Option<domain::Customer>,
        _storage_scheme: enums::MerchantStorageScheme,
        _updated_customer: Option<storage::CustomerUpdate>,
        _key_store: &domain::MerchantKeyStore,
        _frm_suggestion: Option<FrmSuggestion>,
        _header_payload: api::HeaderPayload,
    ) -> RouterResult<(BoxedOperation<'b, F, api::PaymentsRequest>, PaymentData<F>)>
    where
        F: 'b + Send,
    {
        Ok((Box::new(self), payment_data))
    }
}

#[async_trait]
impl<F: Clone> UpdateTracker<F, PaymentData<F>, api::PaymentsRetrieveRequest> for PaymentStatus {
    async fn update_trackers<'b>(
        &'b self,
        _state: &'b AppState,
        _req_state: ReqState,
        payment_data: PaymentData<F>,
        _customer: Option<domain::Customer>,
        _storage_scheme: enums::MerchantStorageScheme,
        _updated_customer: Option<storage::CustomerUpdate>,
        _key_store: &domain::MerchantKeyStore,
        _frm_suggestion: Option<FrmSuggestion>,
        _header_payload: api::HeaderPayload,
    ) -> RouterResult<(
        BoxedOperation<'b, F, api::PaymentsRetrieveRequest>,
        PaymentData<F>,
    )>
    where
        F: 'b + Send,
    {
        Ok((Box::new(self), payment_data))
    }
}

#[async_trait]
impl<F: Send + Clone> GetTracker<F, PaymentData<F>, api::PaymentsRetrieveRequest>
    for PaymentStatus
{
    #[instrument(skip_all)]
    async fn get_trackers<'a>(
        &'a self,
        state: &'a AppState,
        payment_id: &api::PaymentIdType,
        request: &api::PaymentsRetrieveRequest,
        merchant_account: &domain::MerchantAccount,
        key_store: &domain::MerchantKeyStore,
        _auth_flow: services::AuthFlow,
        _payment_confirm_source: Option<common_enums::PaymentSource>,
    ) -> RouterResult<operations::GetTrackerResponse<'a, F, api::PaymentsRetrieveRequest>> {
        get_tracker_for_sync(
            payment_id,
            merchant_account,
            key_store,
            &*state.store,
            request,
            self,
            merchant_account.storage_scheme,
        )
        .await
    }
}

async fn get_tracker_for_sync<
    'a,
    F: Send + Clone,
    Op: Operation<F, api::PaymentsRetrieveRequest> + 'a + Send + Sync,
>(
    payment_id: &api::PaymentIdType,
    merchant_account: &domain::MerchantAccount,
    mechant_key_store: &domain::MerchantKeyStore,
    db: &dyn StorageInterface,
    request: &api::PaymentsRetrieveRequest,
    operation: Op,
    storage_scheme: enums::MerchantStorageScheme,
) -> RouterResult<operations::GetTrackerResponse<'a, F, api::PaymentsRetrieveRequest>> {
    let (payment_intent, mut payment_attempt, currency, amount);

    (payment_intent, payment_attempt) = get_payment_intent_payment_attempt(
        db,
        payment_id,
        &merchant_account.merchant_id,
        storage_scheme,
    )
    .await?;

    helpers::authenticate_client_secret(request.client_secret.as_ref(), &payment_intent)?;

    let payment_id_str = payment_attempt.payment_id.clone();

    currency = payment_attempt.currency.get_required_value("currency")?;
    amount = payment_attempt.get_total_amount().into();

    let shipping_address = helpers::get_address_by_id(
        db,
        payment_intent.shipping_address_id.clone(),
        mechant_key_store,
        &payment_intent.payment_id.clone(),
        &merchant_account.merchant_id,
        merchant_account.storage_scheme,
    )
    .await?;
    let billing_address = helpers::get_address_by_id(
        db,
        payment_intent.billing_address_id.clone(),
        mechant_key_store,
        &payment_intent.payment_id.clone(),
        &merchant_account.merchant_id,
        merchant_account.storage_scheme,
    )
    .await?;

    let payment_method_billing = helpers::get_address_by_id(
        db,
        payment_attempt.payment_method_billing_address_id.clone(),
        mechant_key_store,
        &payment_intent.payment_id.clone(),
        &merchant_account.merchant_id,
        merchant_account.storage_scheme,
    )
    .await?;

    payment_attempt.encoded_data.clone_from(&request.param);

    let attempts = match request.expand_attempts {
        Some(true) => {
            Some(db
                .find_attempts_by_merchant_id_payment_id(&merchant_account.merchant_id, &payment_id_str, storage_scheme)
                .await
                .change_context(errors::ApiErrorResponse::PaymentNotFound)
                .attach_printable_lazy(|| {
                    format!("Error while retrieving attempt list for, merchant_id: {}, payment_id: {payment_id_str}",&merchant_account.merchant_id)
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
                    format!("Error while retrieving capture list for, merchant_id: {}, payment_id: {payment_id_str}", merchant_account.merchant_id)
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
            &payment_id_str,
            &merchant_account.merchant_id,
            storage_scheme,
        )
        .await
        .change_context(errors::ApiErrorResponse::PaymentNotFound)
        .attach_printable_lazy(|| {
            format!(
                "Failed while getting refund list for, payment_id: {}, merchant_id: {}",
                &payment_id_str, merchant_account.merchant_id
            )
        })?;

    let authorizations = db
        .find_all_authorizations_by_merchant_id_payment_id(
            &merchant_account.merchant_id,
            &payment_id_str,
        )
        .await
        .change_context(errors::ApiErrorResponse::PaymentNotFound)
        .attach_printable_lazy(|| {
            format!(
                "Failed while getting authorizations list for, payment_id: {}, merchant_id: {}",
                &payment_id_str, merchant_account.merchant_id
            )
        })?;

    let disputes = db
        .find_disputes_by_merchant_id_payment_id(&merchant_account.merchant_id, &payment_id_str)
        .await
        .change_context(errors::ApiErrorResponse::PaymentNotFound)
        .attach_printable_lazy(|| {
            format!("Error while retrieving dispute list for, merchant_id: {}, payment_id: {payment_id_str}", &merchant_account.merchant_id)
        })?;

    let frm_response = db
        .find_fraud_check_by_payment_id(payment_id_str.to_string(), merchant_account.merchant_id.clone())
        .await
        .change_context(errors::ApiErrorResponse::PaymentNotFound)
        .attach_printable_lazy(|| {
            format!("Error while retrieving frm_response, merchant_id: {}, payment_id: {payment_id_str}", &merchant_account.merchant_id)
        });

    let contains_encoded_data = payment_attempt.encoded_data.is_some();

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
                &merchant_account.merchant_id,
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
        .find_business_profile_by_profile_id(profile_id)
        .await
        .to_not_found_response(errors::ApiErrorResponse::BusinessProfileNotFound {
            id: profile_id.to_string(),
        })?;

    let payment_method_info =
        if let Some(ref payment_method_id) = payment_attempt.payment_method_id.clone() {
            match db
                .find_payment_method(payment_method_id, storage_scheme)
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
                    merchant_id,
                    authentication_id.clone(),
                )
                .await
                .to_not_found_response(errors::ApiErrorResponse::InternalServerError)
                .attach_printable_lazy(|| format!("Error while fetching authentication record with authentication_id {authentication_id}"))
        }).await
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
                && (helpers::check_force_psync_precondition(&payment_attempt.status)
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
        payment_link_data: None,
        surcharge_details: None,
        frm_message: frm_response.ok(),
        incremental_authorization_details: None,
        authorizations,
        authentication,
        recurring_details: None,
        poll_config: None,
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

impl<F: Send + Clone> ValidateRequest<F, api::PaymentsRetrieveRequest> for PaymentStatus {
    fn validate_request<'a, 'b>(
        &'b self,
        request: &api::PaymentsRetrieveRequest,
        merchant_account: &'a domain::MerchantAccount,
    ) -> RouterResult<(
        BoxedOperation<'b, F, api::PaymentsRetrieveRequest>,
        operations::ValidateResult<'a>,
    )> {
        let request_merchant_id = request.merchant_id.as_deref();
        helpers::validate_merchant_id(&merchant_account.merchant_id, request_merchant_id)
            .change_context(errors::ApiErrorResponse::InvalidDataFormat {
                field_name: "merchant_id".to_string(),
                expected_format: "merchant_id from merchant account".to_string(),
            })?;

        Ok((
            Box::new(self),
            operations::ValidateResult {
                merchant_id: &merchant_account.merchant_id,
                payment_id: request.resource_id.clone(),
                storage_scheme: merchant_account.storage_scheme,
                requeue: false,
            },
        ))
    }
}

#[inline]
pub async fn get_payment_intent_payment_attempt(
    db: &dyn StorageInterface,
    payment_id: &api::PaymentIdType,
    merchant_id: &str,
    storage_scheme: enums::MerchantStorageScheme,
) -> RouterResult<(storage::PaymentIntent, storage::PaymentAttempt)> {
    let get_pi_pa = || async {
        let (pi, pa);
        match payment_id {
            api_models::payments::PaymentIdType::PaymentIntentId(ref id) => {
                pi = db
                    .find_payment_intent_by_payment_id_merchant_id(id, merchant_id, storage_scheme)
                    .await?;
                pa = db
                    .find_payment_attempt_by_payment_id_merchant_id_attempt_id(
                        pi.payment_id.as_str(),
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
                        pa.payment_id.as_str(),
                        merchant_id,
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
                        pa.payment_id.as_str(),
                        merchant_id,
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
                        pa.payment_id.as_str(),
                        merchant_id,
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
}
