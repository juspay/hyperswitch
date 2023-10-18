use std::marker::PhantomData;

use api_models::enums::FrmSuggestion;
use async_trait::async_trait;
use common_utils::ext_traits::AsyncExt;
use error_stack::ResultExt;
use router_derive::PaymentOperation;
use router_env::{instrument, tracing};

use super::{BoxedOperation, Domain, GetTracker, Operation, UpdateTracker, ValidateRequest};
use crate::{
    core::{
        errors::{self, CustomResult, RouterResult, StorageErrorExt},
        payment_methods::PaymentMethodRetrieve,
        payments::{
            helpers, operations, types as payment_types, CustomerDetails, PaymentAddress,
            PaymentData,
        },
    },
    db::StorageInterface,
    routes::AppState,
    services,
    types::{
        api, domain,
        storage::{self, enums},
    },
    utils::OptionExt,
};

#[derive(Debug, Clone, Copy, PaymentOperation)]
#[operation(ops = "all", flow = "sync")]
pub struct PaymentStatus;

impl<F: Send + Clone, Ctx: PaymentMethodRetrieve> Operation<F, api::PaymentsRequest, Ctx>
    for PaymentStatus
{
    fn to_domain(&self) -> RouterResult<&dyn Domain<F, api::PaymentsRequest, Ctx>> {
        Ok(self)
    }
    fn to_update_tracker(
        &self,
    ) -> RouterResult<
        &(dyn UpdateTracker<F, PaymentData<F>, api::PaymentsRequest, Ctx> + Send + Sync),
    > {
        Ok(self)
    }
}
impl<F: Send + Clone, Ctx: PaymentMethodRetrieve> Operation<F, api::PaymentsRequest, Ctx>
    for &PaymentStatus
{
    fn to_domain(&self) -> RouterResult<&dyn Domain<F, api::PaymentsRequest, Ctx>> {
        Ok(*self)
    }
    fn to_update_tracker(
        &self,
    ) -> RouterResult<
        &(dyn UpdateTracker<F, PaymentData<F>, api::PaymentsRequest, Ctx> + Send + Sync),
    > {
        Ok(*self)
    }
}

#[async_trait]
impl<F: Clone + Send, Ctx: PaymentMethodRetrieve> Domain<F, api::PaymentsRequest, Ctx>
    for PaymentStatus
{
    #[instrument(skip_all)]
    async fn get_or_create_customer_details<'a>(
        &'a self,
        db: &dyn StorageInterface,
        payment_data: &mut PaymentData<F>,
        request: Option<CustomerDetails>,
        key_store: &domain::MerchantKeyStore,
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
        )
        .await
    }

    #[instrument(skip_all)]
    async fn make_pm_data<'a>(
        &'a self,
        state: &'a AppState,
        payment_data: &mut PaymentData<F>,
        _storage_scheme: enums::MerchantStorageScheme,
    ) -> RouterResult<(
        BoxedOperation<'a, F, api::PaymentsRequest, Ctx>,
        Option<api::PaymentMethodData>,
    )> {
        helpers::make_pm_data(Box::new(self), state, payment_data).await
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
}

#[async_trait]
impl<F: Clone, Ctx: PaymentMethodRetrieve>
    UpdateTracker<F, PaymentData<F>, api::PaymentsRequest, Ctx> for PaymentStatus
{
    async fn update_trackers<'b>(
        &'b self,
        _db: &dyn StorageInterface,
        payment_data: PaymentData<F>,
        _customer: Option<domain::Customer>,
        _storage_scheme: enums::MerchantStorageScheme,
        _updated_customer: Option<storage::CustomerUpdate>,
        _key_store: &domain::MerchantKeyStore,
        _frm_suggestion: Option<FrmSuggestion>,
        _header_payload: api::HeaderPayload,
    ) -> RouterResult<(
        BoxedOperation<'b, F, api::PaymentsRequest, Ctx>,
        PaymentData<F>,
    )>
    where
        F: 'b + Send,
    {
        Ok((Box::new(self), payment_data))
    }
}

#[async_trait]
impl<F: Clone, Ctx: PaymentMethodRetrieve>
    UpdateTracker<F, PaymentData<F>, api::PaymentsRetrieveRequest, Ctx> for PaymentStatus
{
    async fn update_trackers<'b>(
        &'b self,
        _db: &dyn StorageInterface,
        payment_data: PaymentData<F>,
        _customer: Option<domain::Customer>,
        _storage_scheme: enums::MerchantStorageScheme,
        _updated_customer: Option<storage::CustomerUpdate>,
        _key_store: &domain::MerchantKeyStore,
        _frm_suggestion: Option<FrmSuggestion>,
        _header_payload: api::HeaderPayload,
    ) -> RouterResult<(
        BoxedOperation<'b, F, api::PaymentsRetrieveRequest, Ctx>,
        PaymentData<F>,
    )>
    where
        F: 'b + Send,
    {
        Ok((Box::new(self), payment_data))
    }
}

#[async_trait]
impl<F: Send + Clone, Ctx: PaymentMethodRetrieve>
    GetTracker<F, PaymentData<F>, api::PaymentsRetrieveRequest, Ctx> for PaymentStatus
{
    #[instrument(skip_all)]
    async fn get_trackers<'a>(
        &'a self,
        state: &'a AppState,
        payment_id: &api::PaymentIdType,
        request: &api::PaymentsRetrieveRequest,
        _mandate_type: Option<api::MandateTransactionType>,
        merchant_account: &domain::MerchantAccount,
        key_store: &domain::MerchantKeyStore,
        _auth_flow: services::AuthFlow,
    ) -> RouterResult<(
        BoxedOperation<'a, F, api::PaymentsRetrieveRequest, Ctx>,
        PaymentData<F>,
        Option<CustomerDetails>,
    )> {
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
    Ctx: PaymentMethodRetrieve,
    Op: Operation<F, api::PaymentsRetrieveRequest, Ctx> + 'a + Send + Sync,
>(
    payment_id: &api::PaymentIdType,
    merchant_account: &domain::MerchantAccount,
    mechant_key_store: &domain::MerchantKeyStore,
    db: &dyn StorageInterface,
    request: &api::PaymentsRetrieveRequest,
    operation: Op,
    storage_scheme: enums::MerchantStorageScheme,
) -> RouterResult<(
    BoxedOperation<'a, F, api::PaymentsRetrieveRequest, Ctx>,
    PaymentData<F>,
    Option<CustomerDetails>,
)> {
    let (payment_intent, payment_attempt, currency, amount);

    (payment_intent, payment_attempt) = get_payment_intent_payment_attempt(
        db,
        payment_id,
        &merchant_account.merchant_id,
        storage_scheme,
    )
    .await?;

    let intent_fulfillment_time = helpers::get_merchant_fullfillment_time(
        payment_intent.payment_link_id.clone(),
        merchant_account.intent_fulfillment_time,
        db,
    )
    .await?;
    helpers::authenticate_client_secret(
        request.client_secret.as_ref(),
        &payment_intent,
        intent_fulfillment_time,
    )?;

    let payment_id_str = payment_attempt.payment_id.clone();

    let mut connector_response = db
        .find_connector_response_by_payment_id_merchant_id_attempt_id(
            &payment_intent.payment_id,
            &payment_intent.merchant_id,
            &payment_attempt.attempt_id,
            storage_scheme,
        )
        .await
        .change_context(errors::ApiErrorResponse::PaymentNotFound)
        .attach_printable("Database error when finding connector response")?;

    connector_response.encoded_data = request.param.clone();
    currency = payment_attempt.currency.get_required_value("currency")?;
    amount = payment_attempt.amount.into();

    let shipping_address = helpers::get_address_by_id(
        db,
        payment_intent.shipping_address_id.clone(),
        mechant_key_store,
        payment_intent.payment_id.clone(),
        merchant_account.merchant_id.clone(),
        merchant_account.storage_scheme,
    )
    .await?;
    let billing_address = helpers::get_address_by_id(
        db,
        payment_intent.billing_address_id.clone(),
        mechant_key_store,
        payment_intent.payment_id.clone(),
        merchant_account.merchant_id.clone(),
        merchant_account.storage_scheme,
    )
    .await?;

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

    let contains_encoded_data = connector_response.encoded_data.is_some();

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
    Ok((
        Box::new(operation),
        PaymentData {
            flow: PhantomData,
            payment_intent,
            connector_response,
            currency,
            amount,
            email: None,
            mandate_id: payment_attempt.mandate_id.clone().map(|id| {
                api_models::payments::MandateIds {
                    mandate_id: id,
                    mandate_reference_id: None,
                }
            }),
            mandate_connector: None,
            setup_mandate: None,
            token: None,
            address: PaymentAddress {
                shipping: shipping_address.as_ref().map(|a| a.into()),
                billing: billing_address.as_ref().map(|a| a.into()),
            },
            confirm: Some(request.force_sync),
            payment_method_data: None,
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
        },
        None,
    ))
}

impl<F: Send + Clone, Ctx: PaymentMethodRetrieve>
    ValidateRequest<F, api::PaymentsRetrieveRequest, Ctx> for PaymentStatus
{
    fn validate_request<'a, 'b>(
        &'b self,
        request: &api::PaymentsRetrieveRequest,
        merchant_account: &'a domain::MerchantAccount,
    ) -> RouterResult<(
        BoxedOperation<'b, F, api::PaymentsRetrieveRequest, Ctx>,
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
                mandate_type: None,
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
