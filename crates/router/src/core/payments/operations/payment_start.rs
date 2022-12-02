use std::marker::PhantomData;

use async_trait::async_trait;
use error_stack::{report, ResultExt};
use router_derive::PaymentOperation;
use router_env::{instrument, tracing};

use super::{BoxedOperation, Domain, GetTracker, Operation, UpdateTracker, ValidateRequest};
use crate::{
    core::{
        errors::{self, CustomResult, RouterResult, StorageErrorExt},
        payments::{helpers, CustomerDetails, PaymentAddress, PaymentData},
    },
    db::Db,
    routes::AppState,
    types::{
        api,
        storage::{self, enums, Customer},
        Connector,
    },
    utils::OptionExt,
};

#[derive(Debug, Clone, Copy, PaymentOperation)]
#[operation(ops = "all", flow = "start")]
pub struct PaymentStart;

#[async_trait]
impl<F: Send + Clone> GetTracker<F, PaymentData<F>, api::PaymentsStartRequest> for PaymentStart {
    #[instrument(skip_all)]
    async fn get_trackers<'a>(
        &'a self,
        state: &'a AppState,
        payment_id: &api::PaymentIdType,
        merchant_id: &str,
        _connector: Connector,
        _request: &api::PaymentsStartRequest,
        _mandate_type: Option<api::MandateTxnType>,
    ) -> RouterResult<(
        BoxedOperation<'a, F, api::PaymentsStartRequest>,
        PaymentData<F>,
        Option<CustomerDetails>,
    )> {
        let (mut payment_intent, payment_attempt, currency, amount);
        let db = &state.store as &dyn Db;

        let payment_id = payment_id
            .get_payment_intent_id()
            .change_context(errors::ApiErrorResponse::PaymentNotFound)?;

        payment_intent = db
            .find_payment_intent_by_payment_id_merchant_id(&payment_id, merchant_id)
            .await
            .map_err(|error| {
                error.to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)
            })?;

        payment_attempt = db
            .find_payment_attempt_by_payment_id_merchant_id(&payment_id, merchant_id)
            .await
            .map_err(|error| {
                error.to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)
            })?;

        currency = payment_attempt.currency.get_required_value("currency")?;
        amount = payment_attempt.amount;

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

        payment_intent.shipping_address_id = shipping_address.clone().map(|i| i.address_id);
        payment_intent.billing_address_id = billing_address.clone().map(|i| i.address_id);

        //TODO: get customer from db?
        let customer_details = CustomerDetails {
            customer_id: payment_intent.customer_id.clone(),
            name: None,
            email: None,
            phone: None,
            phone_country_code: None,
        };

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

        match payment_intent.status {
            enums::IntentStatus::Succeeded | enums::IntentStatus::Failed => {
                Err(report!(errors::ValidateError)
                    .attach_printable("You cannot confirm this Payment because it has already succeeded after being previously confirmed.")
                    .change_context(errors::ApiErrorResponse::InvalidDataFormat { field_name: "payment_id".to_string(), expected_format: "payment_id of pending payment".to_string() }))
            }
            _ => Ok((
                Box::new(self),
                PaymentData {
                    flow: PhantomData,
                    payment_intent,
                    currency,
                    amount,
                    mandate_id: None,
                    connector_response,
                    setup_mandate: None,
                    token: None,
                    address: PaymentAddress {
                        shipping: shipping_address.as_ref().map(|a| a.into()),
                        billing: billing_address.as_ref().map(|a| a.into()),
                    },
                    confirm: Some(payment_attempt.confirm),
                     payment_attempt,
                    payment_method_data: None,
                    force_sync: None,
                    refunds: vec![]
                },
                Some(customer_details)
            )),
        }
    }
}

#[async_trait]
impl<F: Clone> UpdateTracker<F, PaymentData<F>, api::PaymentsStartRequest> for PaymentStart {
    #[instrument(skip_all)]
    async fn update_trackers<'b>(
        &'b self,
        _db: &dyn Db,
        _payment_id: &api::PaymentIdType,
        payment_data: PaymentData<F>,
        _customer: Option<Customer>,
    ) -> RouterResult<(
        BoxedOperation<'b, F, api::PaymentsStartRequest>,
        PaymentData<F>,
    )>
    where
        F: 'b + Send,
    {
        Ok((Box::new(self), payment_data))
    }
}

impl<F: Send + Clone> ValidateRequest<F, api::PaymentsStartRequest> for PaymentStart {
    #[instrument(skip_all)]
    fn validate_request<'a, 'b>(
        &'b self,
        request: &api::PaymentsStartRequest,
        merchant_account: &'a storage::MerchantAccount,
    ) -> RouterResult<(
        BoxedOperation<'b, F, api::PaymentsStartRequest>,
        &'a str,
        api::PaymentIdType,
        Option<api::MandateTxnType>,
    )> {
        let request_merchant_id = Some(&request.merchant_id[..]);
        helpers::validate_merchant_id(&merchant_account.merchant_id, request_merchant_id)
            .change_context(errors::ApiErrorResponse::InvalidDataFormat {
                field_name: "merchant_id".to_string(),
                expected_format: "merchant_id from merchant account".to_string(),
            })?;
        // let mandate_type = validate_mandate(request)?;
        let payment_id = request.payment_id.clone();

        Ok((
            Box::new(self),
            &merchant_account.merchant_id,
            api::PaymentIdType::PaymentIntentId(payment_id),
            None,
        ))
    }
}

#[async_trait]
impl<F: Clone + Send, Op: Send + Sync + Operation<F, api::PaymentsStartRequest>>
    Domain<F, api::PaymentsStartRequest> for Op
where
    for<'a> &'a Op: Operation<F, api::PaymentsStartRequest>,
{
    #[instrument(skip_all)]
    async fn get_or_create_customer_details<'a>(
        &'a self,
        db: &dyn Db,
        payment_data: &mut PaymentData<F>,
        request: Option<CustomerDetails>,
        merchant_id: &str,
    ) -> CustomResult<
        (
            BoxedOperation<'a, F, api::PaymentsStartRequest>,
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
        state: &'a AppState,
        payment_method: Option<enums::PaymentMethodType>,
        txn_id: &str,
        payment_attempt: &storage::PaymentAttempt,
        request: &Option<api::PaymentMethod>,
        token: &Option<String>,
    ) -> RouterResult<(
        BoxedOperation<'a, F, api::PaymentsStartRequest>,
        Option<api::PaymentMethod>,
    )> {
        helpers::make_pm_data(
            Box::new(self),
            state,
            payment_method,
            txn_id,
            payment_attempt,
            request,
            token,
        )
        .await
    }
}
