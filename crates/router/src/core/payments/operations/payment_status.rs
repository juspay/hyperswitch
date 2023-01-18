use api_models::payments::PaymentsRetrieveRequest;
use async_trait::async_trait;
use error_stack::ResultExt;
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
    routes::AppState,
    services,
    types::{
        self, api,
        storage::{self, enums},
        transformers::ForeignInto,
    },
    utils::OptionExt,
};

#[derive(Debug, Clone, Copy)]
// #[operation(ops = "all", flow = "sync")]
pub struct PaymentStatus;

#[async_trait]
impl Operation<PaymentsRetrieveRequest> for &PaymentStatus {
    fn to_validate_request(
        &self,
    ) -> RouterResult<&(dyn ValidateRequest<PaymentsRetrieveRequest> + Send + Sync)> {
        Ok(*self)
    }
    fn to_get_tracker(
        &self,
    ) -> RouterResult<&(dyn GetTracker<PaymentData, PaymentsRetrieveRequest> + Send + Sync)> {
        Ok(*self)
    }
    fn to_domain(&self) -> RouterResult<&(dyn Domain<PaymentsRetrieveRequest>)> {
        Ok(*self)
    }
    fn to_update_tracker(
        &self,
    ) -> RouterResult<&(dyn UpdateTracker<PaymentData, PaymentsRetrieveRequest> + Send + Sync)>
    {
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
impl Operation<PaymentsRetrieveRequest> for PaymentStatus {
    fn to_validate_request(
        &self,
    ) -> RouterResult<&(dyn ValidateRequest<PaymentsRetrieveRequest> + Send + Sync)> {
        Ok(self)
    }
    fn to_get_tracker(
        &self,
    ) -> RouterResult<&(dyn GetTracker<PaymentData, PaymentsRetrieveRequest> + Send + Sync)> {
        Ok(self)
    }
    fn to_domain(&self) -> RouterResult<&dyn Domain<PaymentsRetrieveRequest>> {
        Ok(self)
    }
    fn to_update_tracker(
        &self,
    ) -> RouterResult<&(dyn UpdateTracker<PaymentData, PaymentsRetrieveRequest> + Send + Sync)>
    {
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
impl Operation<api::PaymentsRequest> for PaymentStatus {
    fn to_domain(&self) -> RouterResult<&dyn Domain<api::PaymentsRequest>> {
        Ok(self)
    }
    fn to_update_tracker(
        &self,
    ) -> RouterResult<&(dyn UpdateTracker<PaymentData, api::PaymentsRequest> + Send + Sync)> {
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
impl Operation<api::PaymentsRequest> for &PaymentStatus {
    fn to_domain(&self) -> RouterResult<&dyn Domain<api::PaymentsRequest>> {
        Ok(*self)
    }
    fn to_update_tracker(
        &self,
    ) -> RouterResult<&(dyn UpdateTracker<PaymentData, api::PaymentsRequest> + Send + Sync)> {
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
impl Domain<api::PaymentsRequest> for PaymentStatus {
    #[instrument(skip_all)]
    async fn get_or_create_customer_details<'a>(
        &'a self,
        db: &dyn StorageInterface,
        payment_data: &mut PaymentData,
        request: Option<CustomerDetails>,
        merchant_id: &str,
    ) -> CustomResult<
        (
            BoxedOperation<'a, api::PaymentsRequest>,
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
        BoxedOperation<'a, api::PaymentsRequest>,
        Option<api::PaymentMethod>,
    )> {
        helpers::make_pm_data(Box::new(self), state, payment_data).await
    }

    #[instrument(skip_all)]
    async fn add_task_to_process_tracker<'a>(
        &'a self,
        state: &'a AppState,
        payment_attempt: &storage::PaymentAttempt,
    ) -> CustomResult<(), errors::ApiErrorResponse> {
        helpers::add_domain_task_to_pt(self, state, payment_attempt).await
    }

    async fn get_connector<'a>(
        &'a self,
        _merchant_account: &storage::MerchantAccount,
        state: &AppState,
        _request: &api::PaymentsRequest,
    ) -> CustomResult<api::ConnectorCallType, errors::ApiErrorResponse> {
        helpers::get_connector_default(state, None).await
    }
}

#[async_trait]
impl UpdateTracker<PaymentData, api::PaymentsRequest> for PaymentStatus {
    async fn update_trackers<'b>(
        &'b self,
        _db: &dyn StorageInterface,
        _payment_id: &api::PaymentIdType,
        payment_data: PaymentData,
        _customer: Option<storage::Customer>,
        _storage_scheme: enums::MerchantStorageScheme,
    ) -> RouterResult<(BoxedOperation<'b, api::PaymentsRequest>, PaymentData)> {
        Ok((Box::new(self), payment_data))
    }
}

#[async_trait]
impl UpdateTracker<PaymentData, PaymentsRetrieveRequest> for PaymentStatus {
    async fn update_trackers<'b>(
        &'b self,
        _db: &dyn StorageInterface,
        _payment_id: &api::PaymentIdType,
        payment_data: PaymentData,
        _customer: Option<storage::Customer>,
        _storage_scheme: enums::MerchantStorageScheme,
    ) -> RouterResult<(BoxedOperation<'b, PaymentsRetrieveRequest>, PaymentData)> {
        Ok((Box::new(self), payment_data))
    }
}

#[async_trait]
impl GetTracker<PaymentData, PaymentsRetrieveRequest> for PaymentStatus {
    #[instrument(skip_all)]
    async fn get_trackers<'a>(
        &'a self,
        state: &'a AppState,
        payment_id: &api::PaymentIdType,
        request: &PaymentsRetrieveRequest,
        _mandate_type: Option<api::MandateTxnType>,
        merchant_account: &storage::MerchantAccount,
    ) -> RouterResult<(
        BoxedOperation<'a, PaymentsRetrieveRequest>,
        PaymentData,
        Option<CustomerDetails>,
    )> {
        get_tracker_for_sync(
            payment_id,
            &merchant_account.merchant_id,
            &*state.store,
            request,
            self,
            merchant_account.storage_scheme,
        )
        .await
    }
}

async fn get_tracker_for_sync<'a, Op: Operation<PaymentsRetrieveRequest> + 'a + Send + Sync>(
    payment_id: &api::PaymentIdType,
    merchant_id: &str,
    db: &dyn StorageInterface,
    request: &PaymentsRetrieveRequest,
    operation: Op,
    storage_scheme: enums::MerchantStorageScheme,
) -> RouterResult<(
    BoxedOperation<'a, PaymentsRetrieveRequest>,
    PaymentData,
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

    Ok((
        Box::new(operation),
        PaymentData {
            payment_intent,
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
            force_sync: Some(
                request.force_sync && helpers::can_call_connector(&payment_attempt.status),
            ),
            payment_attempt,
            refunds,
            sessions_token: vec![],
            card_cvc: None,
        },
        None,
    ))
}

impl ValidateRequest<PaymentsRetrieveRequest> for PaymentStatus {
    fn validate_request<'a, 'b>(
        &'b self,
        request: &PaymentsRetrieveRequest,
        merchant_account: &'a storage::MerchantAccount,
    ) -> RouterResult<(
        BoxedOperation<'b, PaymentsRetrieveRequest>,
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

impl<FData> DeriveFlow<api::PSync, FData> for PaymentStatus
where
    PaymentData: payments::flows::ConstructFlowSpecificData<
        api::PSync,
        FData,
        crate::types::PaymentsResponseData,
    >,
    types::RouterData<api::PSync, FData, crate::types::PaymentsResponseData>:
        payments::flows::Feature<api::PSync, FData>,
    (dyn api::Connector + 'static):
        services::api::ConnectorIntegration<api::PSync, FData, types::PaymentsResponseData>,
    operations::payment_response::PaymentResponse: operations::EndOperation<api::PSync, FData>,
    FData: Send,
{
    fn should_call_connector(&self, payment_data: &PaymentData) -> bool {
        matches!(
            payment_data.payment_intent.status,
            storage_models::enums::IntentStatus::Failed
                | storage_models::enums::IntentStatus::Processing
                | storage_models::enums::IntentStatus::Succeeded
                | storage_models::enums::IntentStatus::RequiresCustomerAction
        ) && payment_data.force_sync.unwrap_or(false)
    }
}
