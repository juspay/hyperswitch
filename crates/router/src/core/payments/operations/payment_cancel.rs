use std::marker::PhantomData;

use api_models::payments::PaymentsCancelRequest;
use async_trait::async_trait;
use error_stack::ResultExt;
use router_derive;
use router_env::{instrument, tracing};

use super::{
    BoxedOperation, DeriveFlow, Domain, GetTracker, Operation, UpdateTracker, ValidateRequest,
};
use crate::{
    core::{
        errors::{self, RouterResult, StorageErrorExt},
        payments::{
            self, connector_specific_call_connector, helpers, operations, CustomerDetails,
            PaymentAddress, PaymentData,
        },
    },
    db::StorageInterface,
    routes::AppState,
    services,
    types::{
        api::{self, PaymentIdTypeExt},
        storage::{self, enums, Customer},
        transformers::ForeignInto,
    },
    utils::OptionExt,
};

#[derive(Debug, Clone, Copy)]
// #[operation(ops = "all", flow = "cancel")]
pub struct PaymentCancel;

#[async_trait]
impl Operation<PaymentsCancelRequest> for &PaymentCancel {
    fn to_validate_request(
        &self,
    ) -> RouterResult<&(dyn ValidateRequest<PaymentsCancelRequest> + Send + Sync)> {
        Ok(*self)
    }
    fn to_get_tracker(
        &self,
    ) -> RouterResult<&(dyn GetTracker<PaymentData, PaymentsCancelRequest> + Send + Sync)> {
        Ok(*self)
    }
    fn to_domain(&self) -> RouterResult<&(dyn Domain<PaymentsCancelRequest>)> {
        Ok(*self)
    }
    fn to_update_tracker(
        &self,
    ) -> RouterResult<&(dyn UpdateTracker<PaymentData, PaymentsCancelRequest> + Send + Sync)> {
        Ok(*self)
    }
    async fn calling_connector(
        &self,
        state: &AppState,
        merchant_account: &storage::MerchantAccount,
        payment_data: PaymentData,
        customer: &Option<Customer>,
        call_connector_action: payments::CallConnectorAction,
        connector_details: api::ConnectorCallType,
        validate_result: operations::ValidateResult<'_>,
    ) -> RouterResult<PaymentData> {
        self.call_connector(
            state,
            merchant_account,
            payment_data,
            customer,
            call_connector_action,
            connector_details,
            validate_result,
        )
        .await
    }
}
#[automatically_derived]
#[async_trait]
impl Operation<PaymentsCancelRequest> for PaymentCancel {
    fn to_validate_request(
        &self,
    ) -> RouterResult<&(dyn ValidateRequest<PaymentsCancelRequest> + Send + Sync)> {
        Ok(self)
    }
    fn to_get_tracker(
        &self,
    ) -> RouterResult<&(dyn GetTracker<PaymentData, PaymentsCancelRequest> + Send + Sync)> {
        Ok(self)
    }
    fn to_domain(&self) -> RouterResult<&dyn Domain<PaymentsCancelRequest>> {
        Ok(self)
    }
    fn to_update_tracker(
        &self,
    ) -> RouterResult<&(dyn UpdateTracker<PaymentData, PaymentsCancelRequest> + Send + Sync)> {
        Ok(self)
    }

    async fn calling_connector(
        &self,
        state: &AppState,
        merchant_account: &storage::MerchantAccount,
        payment_data: PaymentData,
        customer: &Option<Customer>,
        call_connector_action: payments::CallConnectorAction,
        connector_details: api::ConnectorCallType,
        validate_result: operations::ValidateResult<'_>,
    ) -> RouterResult<PaymentData> {
        self.call_connector(
            state,
            merchant_account,
            payment_data,
            customer,
            call_connector_action,
            connector_details,
            validate_result,
        )
        .await
    }
}

#[async_trait]
impl GetTracker<PaymentData, api::PaymentsCancelRequest> for PaymentCancel {
    #[instrument(skip_all)]
    async fn get_trackers<'a>(
        &'a self,
        state: &'a AppState,
        payment_id: &api::PaymentIdType,
        request: &api::PaymentsCancelRequest,
        _mandate_type: Option<api::MandateTxnType>,
        merchant_account: &storage::MerchantAccount,
    ) -> RouterResult<(
        BoxedOperation<'a, api::PaymentsCancelRequest>,
        PaymentData,
        Option<CustomerDetails>,
    )> {
        let db = &*state.store;
        let merchant_id = &merchant_account.merchant_id;
        let storage_scheme = merchant_account.storage_scheme;
        let payment_id = payment_id
            .get_payment_intent_id()
            .change_context(errors::ApiErrorResponse::PaymentNotFound)?;

        let payment_intent = db
            .find_payment_intent_by_payment_id_merchant_id(&payment_id, merchant_id, storage_scheme)
            .await
            .map_err(|error| {
                error.to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)
            })?;

        let mut payment_attempt = db
            .find_payment_attempt_by_payment_id_merchant_id(
                &payment_id,
                merchant_id,
                storage_scheme,
            )
            .await
            .map_err(|error| {
                error.to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)
            })?;

        let shipping_address = helpers::get_address_for_payment_request(
            db,
            None,
            payment_intent.shipping_address_id.as_deref(),
            merchant_id,
            &payment_intent.customer_id,
        )
        .await?;
        let billing_address = helpers::get_address_for_payment_request(
            db,
            None,
            payment_intent.billing_address_id.as_deref(),
            merchant_id,
            &payment_intent.customer_id,
        )
        .await?;

        let connector_response = db
            .find_connector_response_by_payment_id_merchant_id_attempt_id(
                &payment_attempt.payment_id,
                &payment_attempt.merchant_id,
                &payment_attempt.attempt_id,
                storage_scheme,
            )
            .await
            .map_err(|error| {
                error.to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)
            })?;
        let currency = payment_attempt.currency.get_required_value("currency")?;
        let amount = payment_attempt.amount.into();

        payment_attempt.cancellation_reason = request.cancellation_reason.clone();

        match payment_intent.status {
            status if status != enums::IntentStatus::RequiresCapture => {
                Err(errors::ApiErrorResponse::InvalidRequestData {
                    message: "You cannot cancel the payment that has not been authorized"
                        .to_string(),
                }
                .into())
            }
            _ => Ok((
                Box::new(self),
                PaymentData {
                    payment_intent,
                    payment_attempt,
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
                    confirm: None,
                    payment_method_data: None,
                    force_sync: None,
                    refunds: vec![],
                    connector_response,
                    sessions_token: vec![],
                    card_cvc: None,
                },
                None,
            )),
        }
    }
}

#[async_trait]
impl UpdateTracker<PaymentData, api::PaymentsCancelRequest> for PaymentCancel {
    #[instrument(skip_all)]
    async fn update_trackers<'b>(
        &'b self,
        db: &dyn StorageInterface,
        _payment_id: &api::PaymentIdType,
        mut payment_data: PaymentData,
        _customer: Option<Customer>,
        storage_scheme: enums::MerchantStorageScheme,
    ) -> RouterResult<(BoxedOperation<'b, api::PaymentsCancelRequest>, PaymentData)> {
        let cancellation_reason = payment_data.payment_attempt.cancellation_reason.clone();
        payment_data.payment_attempt = db
            .update_payment_attempt(
                payment_data.payment_attempt,
                storage::PaymentAttemptUpdate::VoidUpdate {
                    status: enums::AttemptStatus::VoidInitiated,
                    cancellation_reason,
                },
                storage_scheme,
            )
            .await
            .map_err(|err| err.to_not_found_response(errors::ApiErrorResponse::PaymentNotFound))?;

        Ok((Box::new(self), payment_data))
    }
}

impl ValidateRequest<api::PaymentsCancelRequest> for PaymentCancel {
    #[instrument(skip_all)]
    fn validate_request<'a, 'b>(
        &'b self,
        request: &api::PaymentsCancelRequest,
        merchant_account: &'a storage::MerchantAccount,
    ) -> RouterResult<(
        BoxedOperation<'b, api::PaymentsCancelRequest>,
        operations::ValidateResult<'a>,
    )> {
        Ok((
            Box::new(self),
            operations::ValidateResult {
                merchant_id: &merchant_account.merchant_id,
                payment_id: api::PaymentIdType::PaymentIntentId(request.payment_id.to_owned()),
                mandate_type: None,
                storage_scheme: merchant_account.storage_scheme,
            },
        ))
    }
}

impl<FData: Send> DeriveFlow<api::Void, FData> for PaymentCancel
where
    PaymentData: payments::flows::ConstructFlowSpecificData<
        api::Void,
        FData,
        crate::types::PaymentsResponseData,
    >,
    crate::types::RouterData<api::Void, FData, crate::types::PaymentsResponseData>:
        payments::flows::Feature<api::Void, FData>,
    (dyn api::Connector + 'static):
        services::api::ConnectorIntegration<api::Void, FData, crate::types::PaymentsResponseData>,
    operations::payment_response::PaymentResponse: operations::EndOperation<api::Void, FData>,
{
    fn should_call_connector(&self, payment_data: &PaymentData) -> bool {
        matches!(
            payment_data.payment_intent.status,
            storage_models::enums::IntentStatus::RequiresCapture
        )
    }
}
