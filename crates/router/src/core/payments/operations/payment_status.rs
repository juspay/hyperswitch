use std::marker::PhantomData;

use async_trait::async_trait;
use error_stack::ResultExt;
use masking::Secret;
use router_derive::PaymentOperation;
use router_env::{instrument, tracing};

use super::{BoxedOperation, Domain, GetTracker, Operation, UpdateTracker, ValidateRequest};
use crate::{
    core::{
        errors::{self, ApiErrorResponse, CustomResult, RouterResult, StorageErrorExt},
        payments::{helpers, operations, CustomerDetails, PaymentAddress, PaymentData},
    },
    db::StorageInterface,
    routes::AppState,
    types::{
        api::{self, enums as api_enums},
        storage::{self, enums},
        transformers::ForeignInto,
    },
    utils::{self, OptionExt},
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
        merchant_id: &str,
    ) -> CustomResult<
        (
            BoxedOperation<'a, F, api::PaymentsRequest>,
            Option<storage::Customer>,
        ),
        errors::StorageError,
    > {
        helpers::create_customer_if_not_exist(
            Box::new(self),
            db,
            payment_data,
            request,
            merchant_id,
        )
        .await
    }

    #[instrument(skip_all)]
    async fn make_pm_data<'a>(
        &'a self,
        state: &'a AppState,
        payment_method: Option<enums::PaymentMethodType>,
        txn_id: &str,
        payment_attempt: &storage::PaymentAttempt,
        request: &Option<api::PaymentMethod>,
        token: &Option<String>,
        card_cvc: Option<Secret<String>>,
        _storage_scheme: enums::MerchantStorageScheme,
    ) -> RouterResult<(
        BoxedOperation<'a, F, api::PaymentsRequest>,
        Option<api::PaymentMethod>,
        Option<String>,
    )> {
        helpers::make_pm_data(
            Box::new(self),
            state,
            payment_method,
            txn_id,
            payment_attempt,
            request,
            token,
            card_cvc,
        )
        .await
    }

    #[instrument(skip_all)]
    async fn add_task_to_process_tracker<'a>(
        &'a self,
        state: &'a AppState,
        payment_attempt: &storage::PaymentAttempt,
    ) -> CustomResult<(), ApiErrorResponse> {
        helpers::add_domain_task_to_pt(self, state, payment_attempt).await
    }

    async fn get_connector<'a>(
        &'a self,
        merchant_account: &storage::MerchantAccount,
        state: &AppState,
        request_connector: Option<api_enums::Connector>,
    ) -> CustomResult<api::ConnectorCallType, ApiErrorResponse> {
        helpers::get_connector_default(merchant_account, state, request_connector).await
    }
}

#[async_trait]
impl<F: Clone> UpdateTracker<F, PaymentData<F>, api::PaymentsRequest> for PaymentStatus {
    async fn update_trackers<'b>(
        &'b self,
        _db: &dyn StorageInterface,
        _payment_id: &api::PaymentIdType,
        payment_data: PaymentData<F>,
        _customer: Option<storage::Customer>,
        _storage_scheme: enums::MerchantStorageScheme,
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
        _payment_id: &api::PaymentIdType,
        payment_data: PaymentData<F>,
        _customer: Option<storage::Customer>,
        _storage_scheme: enums::MerchantStorageScheme,
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
        merchant_id: &str,
        request: &api::PaymentsRetrieveRequest,
        _mandate_type: Option<api::MandateTxnType>,
        storage_scheme: enums::MerchantStorageScheme,
    ) -> RouterResult<(
        BoxedOperation<'a, F, api::PaymentsRetrieveRequest>,
        PaymentData<F>,
        Option<CustomerDetails>,
    )> {
        get_tracker_for_sync(
            payment_id,
            merchant_id,
            &*state.store,
            request,
            self,
            storage_scheme,
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

    payment_attempt = match payment_id {
        api::PaymentIdType::PaymentIntentId(ref id) => {
            db.find_payment_attempt_by_payment_id_merchant_id(id, merchant_id, storage_scheme)
        }
        api::PaymentIdType::ConnectorTransactionId(ref id) => {
            db.find_payment_attempt_by_merchant_id_connector_txn_id(merchant_id, id, storage_scheme)
        }
        api::PaymentIdType::PaymentAttemptId(ref id) => {
            db.find_payment_attempt_by_merchant_id_attempt_id(merchant_id, id, storage_scheme)
        }
    }
    .await
    .map_err(|error| error.to_not_found_response(errors::ApiErrorResponse::PaymentNotFound))?;

    let payment_id_str = payment_attempt.payment_id.clone();

    payment_intent = db
        .find_payment_intent_by_payment_id_merchant_id(&payment_id_str, merchant_id, storage_scheme)
        .await
        .map_err(|error| error.to_not_found_response(errors::ApiErrorResponse::PaymentNotFound))?;

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

    let shipping_address =
        helpers::get_address_by_id(db, payment_intent.shipping_address_id.clone()).await?;
    let billing_address =
        helpers::get_address_by_id(db, payment_intent.billing_address_id.clone()).await?;

    utils::when(
        request.force_sync && !helpers::can_call_connector(payment_intent.status),
        || {
            Err(ApiErrorResponse::InvalidRequestData {
                message: format!(
                    "cannot perform force_sync as status: {}",
                    payment_intent.status
                ),
            })
        },
    )?;

    let refunds = db
        .find_refund_by_payment_id_merchant_id(&payment_id_str, merchant_id, storage_scheme)
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)?;

    Ok((
        Box::new(operation),
        PaymentData {
            flow: PhantomData,
            payment_intent,
            payment_attempt,
            connector_response,
            currency,
            amount,
            email: None,
            mandate_id: None,
            setup_mandate: None,
            token: None,
            address: PaymentAddress {
                shipping: shipping_address.as_ref().map(|a| a.foreign_into()),
                billing: billing_address.as_ref().map(|a| a.foreign_into()),
            },
            confirm: Some(request.force_sync),
            payment_method_data: None,
            force_sync: Some(request.force_sync),
            refunds,
            sessions_token: vec![],
            card_cvc: None,
        },
        None,
    ))
}

impl<F: Send + Clone> ValidateRequest<F, api::PaymentsRetrieveRequest> for PaymentStatus {
    fn validate_request<'a, 'b>(
        &'b self,
        request: &api::PaymentsRetrieveRequest,
        merchant_account: &'a storage::MerchantAccount,
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
            },
        ))
    }
}
