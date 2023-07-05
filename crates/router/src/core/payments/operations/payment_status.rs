use std::marker::PhantomData;

use async_trait::async_trait;
use common_utils::ext_traits::AsyncExt;
use error_stack::ResultExt;
use router_derive::PaymentOperation;
use router_env::{instrument, tracing};

use super::{BoxedOperation, Domain, GetTracker, Operation, UpdateTracker, ValidateRequest};
use crate::{
    core::{
        errors::{self, CustomResult, RouterResult, StorageErrorExt},
        payments::{helpers, operations, CustomerDetails, PaymentAddress, PaymentData},
    },
    db::StorageInterface,
    routes::AppState,
    types::{
        api, domain,
        storage::{self, enums},
    },
    utils::OptionExt,
};

#[derive(Debug, Clone, Copy, PaymentOperation)]
#[operation(ops = "all", flow = "sync")]
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
        BoxedOperation<'a, F, api::PaymentsRequest>,
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
        _payment_intent: &storage::payment_intent::PaymentIntent,
        _key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<api::ConnectorChoice, errors::ApiErrorResponse> {
        helpers::get_connector_default(state, request.routing.clone()).await
    }
}

#[async_trait]
impl<F: Clone> UpdateTracker<F, PaymentData<F>, api::PaymentsRequest> for PaymentStatus {
    async fn update_trackers<'b>(
        &'b self,
        _db: &dyn StorageInterface,
        payment_data: PaymentData<F>,
        _customer: Option<domain::Customer>,
        _storage_scheme: enums::MerchantStorageScheme,
        _updated_customer: Option<storage::CustomerUpdate>,
        _key_store: &domain::MerchantKeyStore,
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
        _db: &dyn StorageInterface,
        payment_data: PaymentData<F>,
        _customer: Option<domain::Customer>,
        _storage_scheme: enums::MerchantStorageScheme,
        _updated_customer: Option<storage::CustomerUpdate>,
        _key_store: &domain::MerchantKeyStore,
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
        _mandate_type: Option<api::MandateTransactionType>,
        merchant_account: &domain::MerchantAccount,
        key_store: &domain::MerchantKeyStore,
    ) -> RouterResult<(
        BoxedOperation<'a, F, api::PaymentsRetrieveRequest>,
        PaymentData<F>,
        Option<CustomerDetails>,
    )> {
        get_tracker_for_sync(
            payment_id,
            &merchant_account.merchant_id,
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
    merchant_id: &str,
    mechant_key_store: &domain::MerchantKeyStore,
    db: &dyn StorageInterface,
    request: &api::PaymentsRetrieveRequest,
    operation: Op,
    storage_scheme: enums::MerchantStorageScheme,
) -> RouterResult<(
    BoxedOperation<'a, F, api::PaymentsRetrieveRequest>,
    PaymentData<F>,
    Option<CustomerDetails>,
)> {
    let (payment_intent, payment_attempt, currency, amount);

    (payment_intent, payment_attempt) =
        get_payment_intent_payment_attempt(db, payment_id, merchant_id, storage_scheme).await?;

    let payment_id_str = payment_attempt.payment_id.clone();

    let mut connector_response = db
        .find_connector_response_by_payment_id_merchant_id_attempt_id(
            &payment_intent.payment_id,
            &payment_intent.merchant_id,
            &payment_attempt.attempt_id,
            storage_scheme,
        )
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Database error when finding connector response")?;

    connector_response.encoded_data = request.param.clone();
    currency = payment_attempt.currency.get_required_value("currency")?;
    amount = payment_attempt.amount.into();

    let shipping_address = helpers::get_address_by_id(
        db,
        payment_intent.shipping_address_id.clone(),
        mechant_key_store,
    )
    .await?;
    let billing_address = helpers::get_address_by_id(
        db,
        payment_intent.billing_address_id.clone(),
        mechant_key_store,
    )
    .await?;

    let refunds = db
        .find_refund_by_payment_id_merchant_id(&payment_id_str, merchant_id, storage_scheme)
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable_lazy(|| {
            format!(
                "Failed while getting refund list for, payment_id: {}, merchant_id: {}",
                &payment_id_str, merchant_id
            )
        })?;

    let disputes = db
        .find_disputes_by_merchant_id_payment_id(merchant_id, &payment_id_str)
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable_lazy(|| {
            format!("Error while retrieving dispute list for, merchant_id: {merchant_id}, payment_id: {payment_id_str}")
        })?;

    let contains_encoded_data = connector_response.encoded_data.is_some();

    let creds_identifier = request
        .merchant_connector_details
        .as_ref()
        .map(|mcd| mcd.creds_identifier.to_owned());
    request
        .merchant_connector_details
        .to_owned()
        .async_map(|mcd| async {
            helpers::insert_merchant_connector_creds_to_config(db, merchant_id, mcd).await
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
                    && (helpers::check_force_psync_precondition(
                        &payment_attempt.status,
                        &payment_attempt.connector_transaction_id,
                    ) || contains_encoded_data),
            ),
            payment_attempt,
            refunds,
            disputes,
            sessions_token: vec![],
            card_cvc: None,
            creds_identifier,
            pm_token: None,
            connector_customer_id: None,
            ephemeral_key: None,
            redirect_response: None,
        },
        None,
    ))
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
    (|| async {
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
                        pi.active_attempt_id.as_str(),
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
        Ok((pi, pa))
    })()
    .await
    .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)
}
