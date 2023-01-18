use std::marker::PhantomData;

use api_models::payments::PaymentsStartRequest;
use async_trait::async_trait;
use error_stack::{report, ResultExt};
use router_env::{instrument, tracing};

use super::{
    BoxedOperation, DeriveFlow, Domain, GetTracker, Operation, UpdateTracker, ValidateRequest,
};
use crate::{
    core::{
        errors::{self, CustomResult, RouterResult, StorageErrorExt},
        payments::{self, helpers, operations, CustomerDetails, PaymentAddress, PaymentData},
    },
    db::StorageInterface,
    pii,
    pii::Secret,
    routes::AppState,
    services,
    types::{
        self,
        api::{self, PaymentIdTypeExt},
        storage::{self, enums},
        transformers::ForeignInto,
    },
    utils::OptionExt,
};

#[derive(Debug, Clone, Copy)]
// #[operation(ops = "all", flow = "start")]
pub struct PaymentStart;
#[async_trait]
impl Operation<PaymentsStartRequest> for &PaymentStart {
    fn to_validate_request(
        &self,
    ) -> RouterResult<&(dyn ValidateRequest<PaymentsStartRequest> + Send + Sync)> {
        Ok(*self)
    }
    fn to_get_tracker(
        &self,
    ) -> RouterResult<&(dyn GetTracker<PaymentData, PaymentsStartRequest> + Send + Sync)> {
        Ok(*self)
    }
    fn to_domain(&self) -> RouterResult<&(dyn Domain<PaymentsStartRequest>)> {
        Ok(*self)
    }
    fn to_update_tracker(
        &self,
    ) -> RouterResult<&(dyn UpdateTracker<PaymentData, PaymentsStartRequest> + Send + Sync)> {
        Ok(*self)
    }

    async fn calling_connector(
        &self,
        state: &AppState,
        merchant_account: &storage::MerchantAccount,
        payment_data: PaymentData,
        customer: &Option<storage_models::customers::Customer>,
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
impl Operation<PaymentsStartRequest> for PaymentStart {
    fn to_validate_request(
        &self,
    ) -> RouterResult<&(dyn ValidateRequest<PaymentsStartRequest> + Send + Sync)> {
        Ok(self)
    }
    fn to_get_tracker(
        &self,
    ) -> RouterResult<&(dyn GetTracker<PaymentData, PaymentsStartRequest> + Send + Sync)> {
        Ok(self)
    }
    fn to_domain(&self) -> RouterResult<&dyn Domain<PaymentsStartRequest>> {
        Ok(self)
    }
    fn to_update_tracker(
        &self,
    ) -> RouterResult<&(dyn UpdateTracker<PaymentData, PaymentsStartRequest> + Send + Sync)> {
        Ok(self)
    }

    async fn calling_connector(
        &self,
        state: &AppState,
        merchant_account: &storage::MerchantAccount,
        payment_data: PaymentData,
        customer: &Option<storage_models::customers::Customer>,
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
impl GetTracker<PaymentData, api::PaymentsStartRequest> for PaymentStart {
    #[instrument(skip_all)]
    async fn get_trackers<'a>(
        &'a self,
        state: &'a AppState,
        payment_id: &api::PaymentIdType,
        _request: &api::PaymentsStartRequest,
        _mandate_type: Option<api::MandateTxnType>,
        merchant_account: &storage::MerchantAccount,
    ) -> RouterResult<(
        BoxedOperation<'a, api::PaymentsStartRequest>,
        PaymentData,
        Option<CustomerDetails>,
    )> {
        let (mut payment_intent, payment_attempt, currency, amount);
        let db = &*state.store;

        let merchant_id = &merchant_account.merchant_id;
        let storage_scheme = merchant_account.storage_scheme;
        let payment_id = payment_id
            .get_payment_intent_id()
            .change_context(errors::ApiErrorResponse::PaymentNotFound)?;

        payment_intent = db
            .find_payment_intent_by_payment_id_merchant_id(&payment_id, merchant_id, storage_scheme)
            .await
            .map_err(|error| {
                error.to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)
            })?;

        payment_attempt = db
            .find_payment_attempt_by_payment_id_merchant_id(
                &payment_id,
                merchant_id,
                storage_scheme,
            )
            .await
            .map_err(|error| {
                error.to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)
            })?;

        currency = payment_attempt.currency.get_required_value("currency")?;
        amount = payment_attempt.amount.into();

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

        payment_intent.shipping_address_id = shipping_address.clone().map(|i| i.address_id);
        payment_intent.billing_address_id = billing_address.clone().map(|i| i.address_id);

        let customer_details = CustomerDetails {
            customer_id: payment_intent.customer_id.clone(),
            ..CustomerDetails::default()
        };

        let connector_response = db
            .find_connector_response_by_payment_id_merchant_id_attempt_id(
                &payment_intent.payment_id,
                &payment_intent.merchant_id,
                &payment_attempt.attempt_id,
                storage_scheme,
            )
            .await
            .map_err(|error| {
                error
                    .change_context(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable("Database error when finding connector response")
            })?;

        match payment_intent.status {
            enums::IntentStatus::Succeeded | enums::IntentStatus::Failed => {
                Err(report!(errors::ApiErrorResponse::PreconditionFailed {
                    message: "You cannot confirm this Payment because it has already succeeded \
                              after being previously confirmed."
                        .into()
                }))
            }
            _ => Ok((
                Box::new(self),
                PaymentData {
                    payment_intent,
                    currency,
                    amount,
                    email: None::<Secret<String, pii::Email>>,
                    mandate_id: None,
                    connector_response,
                    setup_mandate: None,
                    token: None,
                    address: PaymentAddress {
                        shipping: shipping_address.as_ref().map(|a| a.foreign_into()),
                        billing: billing_address.as_ref().map(|a| a.foreign_into()),
                    },
                    confirm: Some(payment_attempt.confirm),
                    payment_attempt,
                    payment_method_data: None,
                    force_sync: None,
                    refunds: vec![],
                    sessions_token: vec![],
                    card_cvc: None,
                },
                Some(customer_details),
            )),
        }
    }
}

#[async_trait]
impl UpdateTracker<PaymentData, api::PaymentsStartRequest> for PaymentStart {
    #[instrument(skip_all)]
    async fn update_trackers<'b>(
        &'b self,
        _db: &dyn StorageInterface,
        _payment_id: &api::PaymentIdType,
        payment_data: PaymentData,
        _customer: Option<storage::Customer>,
        _storage_scheme: enums::MerchantStorageScheme,
    ) -> RouterResult<(BoxedOperation<'b, api::PaymentsStartRequest>, PaymentData)> {
        Ok((Box::new(self), payment_data))
    }
}

impl ValidateRequest<api::PaymentsStartRequest> for PaymentStart {
    #[instrument(skip_all)]
    fn validate_request<'a, 'b>(
        &'b self,
        request: &api::PaymentsStartRequest,
        merchant_account: &'a storage::MerchantAccount,
    ) -> RouterResult<(
        BoxedOperation<'b, api::PaymentsStartRequest>,
        operations::ValidateResult<'a>,
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
            operations::ValidateResult {
                merchant_id: &merchant_account.merchant_id,
                payment_id: api::PaymentIdType::PaymentIntentId(payment_id),
                mandate_type: None,
                storage_scheme: merchant_account.storage_scheme,
            },
        ))
    }
}

#[async_trait]
impl<Op: Send + Sync + Operation<api::PaymentsStartRequest>> Domain<api::PaymentsStartRequest>
    for Op
where
    for<'a> &'a Op: Operation<api::PaymentsStartRequest>,
{
    #[instrument(skip_all)]
    async fn get_or_create_customer_details<'a>(
        &'a self,
        db: &dyn StorageInterface,
        payment_data: &mut PaymentData,
        request: Option<CustomerDetails>,
        merchant_id: &str,
    ) -> CustomResult<
        (
            BoxedOperation<'a, api::PaymentsStartRequest>,
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
        payment_data: &mut PaymentData,
        _storage_scheme: enums::MerchantStorageScheme,
    ) -> RouterResult<(
        BoxedOperation<'a, api::PaymentsStartRequest>,
        Option<api::PaymentMethod>,
    )> {
        helpers::make_pm_data(Box::new(self), state, payment_data).await
    }

    async fn get_connector<'a>(
        &'a self,
        _merchant_account: &storage::MerchantAccount,
        state: &AppState,
        _request: &api::PaymentsStartRequest,
    ) -> CustomResult<api::ConnectorCallType, errors::ApiErrorResponse> {
        helpers::get_connector_default(state, None).await
    }
}

impl<FData> DeriveFlow<api::Authorize, FData> for PaymentStart
where
    PaymentData: payments::flows::ConstructFlowSpecificData<
        api::Authorize,
        FData,
        crate::types::PaymentsResponseData,
    >,
    types::RouterData<api::Authorize, FData, crate::types::PaymentsResponseData>:
        payments::flows::Feature<api::Authorize, FData>,
    (dyn api::Connector + 'static):
        services::api::ConnectorIntegration<api::Authorize, FData, types::PaymentsResponseData>,
    operations::payment_response::PaymentResponse: operations::EndOperation<api::Authorize, FData>,
    FData: Send,
{
    fn should_call_connector(&self, payment_data: &PaymentData) -> bool {
        !matches!(
            payment_data.payment_intent.status,
            storage_models::enums::IntentStatus::Failed
                | storage_models::enums::IntentStatus::Succeeded
        ) && payment_data
            .connector_response
            .authentication_data
            .is_none()
    }
}
