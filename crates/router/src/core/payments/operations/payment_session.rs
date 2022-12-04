use std::marker::PhantomData;

use async_trait::async_trait;
use error_stack::{report, ResultExt};
use router_derive::PaymentOperation;
use router_env::{instrument, tracing};

use super::{BoxedOperation, Domain, GetTracker, Operation, UpdateTracker, ValidateRequest};
use crate::{
    core::{
        errors::{self, RouterResult, StorageErrorExt},
        payments::{self, helpers, PaymentData},
    },
    db::StorageInterface,
    routes::AppState,
    types::{
        api,
        storage::{self, enums},
        Connector,
    },
    utils::OptionExt,
};

#[derive(Debug, Clone, Copy, PaymentOperation)]
#[operation(ops = "all", flow = "session")]
pub struct PaymentSession;

#[async_trait]
impl<F: Send + Clone> GetTracker<F, PaymentData<F>, api::PaymentsSessionRequest>
    for PaymentSession
{
    #[instrument(skip_all)]
    async fn get_trackers<'a>(
        &'a self,
        state: &'a AppState,
        payment_id: &api::PaymentIdType,
        merchant_id: &str,
        _connector: Connector,
        request: &api::PaymentsSessionRequest,
        _mandate_type: Option<api::MandateTxnType>,
    ) -> RouterResult<(
        BoxedOperation<'a, F, api::PaymentsSessionRequest>,
        PaymentData<F>,
        Option<payments::CustomerDetails>,
    )> {
        let payment_id = payment_id
            .get_payment_intent_id()
            .change_context(errors::ApiErrorResponse::PaymentNotFound)?;

        let db = &*state.store;

        let mut payment_attempt = db
            .find_payment_attempt_by_payment_id_merchant_id(&payment_id, merchant_id)
            .await
            .map_err(|error| {
                error.to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)
            })?;

        let mut payment_intent = db
            .find_payment_intent_by_payment_id_merchant_id(&payment_id, merchant_id)
            .await
            .map_err(|error| {
                error.to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)
            })?;

        let currency = payment_intent.currency.get_required_value("currency")?;

        payment_attempt.payment_method = Some(enums::PaymentMethodType::Wallet);

        let amount = payment_intent.amount;

        if let Some(ref payment_intent_client_secret) = payment_intent.client_secret {
            if request.client_secret.ne(payment_intent_client_secret) {
                return Err(report!(errors::ApiErrorResponse::ClientSecretInvalid));
            }
        }

        let shipping_address = helpers::get_address_for_payment_request(
            db,
            None,
            payment_intent.shipping_address_id.as_deref(),
        )
        .await?;
        let billing_address = helpers::get_address_for_payment_request(
            db,
            None,
            payment_intent.billing_address_id.as_deref(),
        )
        .await?;

        payment_intent.shipping_address_id = shipping_address.clone().map(|x| x.address_id);
        payment_intent.billing_address_id = billing_address.clone().map(|x| x.address_id);

        let db = db as &dyn StorageInterface;
        let connector_response = db
            .find_connector_response_by_payment_id_merchant_id_txn_id(
                &payment_intent.payment_id,
                &payment_intent.merchant_id,
                &payment_attempt.txn_id,
            )
            .await
            .map_err(|error| {
                error
                    .change_context(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable("Database error when finding connector response")
            })?;

        Ok((
            Box::new(self),
            PaymentData {
                flow: PhantomData,
                payment_intent,
                payment_attempt,
                currency,
                amount,
                mandate_id: None,
                token: None,
                setup_mandate: None,
                address: payments::PaymentAddress {
                    shipping: shipping_address.as_ref().map(|a| a.into()),
                    billing: billing_address.as_ref().map(|a| a.into()),
                },
                confirm: None,
                payment_method_data: None,
                force_sync: None,
                refunds: vec![],
                connector_response,
            },
            None,
        ))
    }
}

#[async_trait]
impl<F: Clone> UpdateTracker<F, PaymentData<F>, api::PaymentsSessionRequest> for PaymentSession {
    #[instrument(skip_all)]
    async fn update_trackers<'b>(
        &'b self,
        _db: &dyn StorageInterface,
        _payment_id: &api::PaymentIdType,
        payment_data: PaymentData<F>,
        _customer: Option<storage::Customer>,
    ) -> RouterResult<(
        BoxedOperation<'b, F, api::PaymentsSessionRequest>,
        PaymentData<F>,
    )>
    where
        F: 'b + Send,
    {
        Ok((Box::new(self), payment_data))
    }
}

impl<F: Send + Clone> ValidateRequest<F, api::PaymentsSessionRequest> for PaymentSession {
    #[instrument(skip_all)]
    fn validate_request<'a, 'b>(
        &'b self,
        request: &api::PaymentsSessionRequest,
        merchant_account: &'a storage::MerchantAccount,
    ) -> RouterResult<(
        BoxedOperation<'b, F, api::PaymentsSessionRequest>,
        &'a str,
        api::PaymentIdType,
        Option<api::MandateTxnType>,
    )> {
        //paymentid is already generated and should be sent in the request
        let given_payment_id = request
            .payment_id
            .get_payment_intent_id()
            .change_context(errors::ApiErrorResponse::PaymentNotFound)?;

        Ok((
            Box::new(self),
            &merchant_account.merchant_id,
            api::PaymentIdType::PaymentIntentId(given_payment_id),
            None,
        ))
    }
}

#[async_trait]
impl<F: Clone + Send, Op: Send + Sync + Operation<F, api::PaymentsSessionRequest>>
    Domain<F, api::PaymentsSessionRequest> for Op
where
    for<'a> &'a Op: Operation<F, api::PaymentsSessionRequest>,
{
    #[instrument(skip_all)]
    async fn get_or_create_customer_details<'a>(
        &'a self,
        db: &dyn StorageInterface,
        payment_data: &mut PaymentData<F>,
        request: Option<payments::CustomerDetails>,
        merchant_id: &str,
    ) -> errors::CustomResult<
        (
            BoxedOperation<'a, F, api::PaymentsSessionRequest>,
            Option<api::CustomerResponse>,
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
        _state: &'a AppState,
        _payment_method: Option<enums::PaymentMethodType>,
        _txn_id: &str,
        _payment_attempt: &storage::PaymentAttempt,
        _request: &Option<api::PaymentMethod>,
        _token: &Option<String>,
    ) -> RouterResult<(
        BoxedOperation<'a, F, api::PaymentsSessionRequest>,
        Option<api::PaymentMethod>,
    )> {
        //No payment method data for this operation
        Ok((Box::new(self), None))
    }
}
