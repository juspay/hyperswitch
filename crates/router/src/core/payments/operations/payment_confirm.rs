use std::marker::PhantomData;

use api_models::payments::PaymentsRequest;
use async_trait::async_trait;
use error_stack::{report, ResultExt};
use router_env::{instrument, tracing};

use super::{
    BoxedOperation, DeriveFlow, Domain, GetTracker, Operation, UpdateTracker, ValidateRequest,
};
use crate::{
    core::{
        errors::{self, CustomResult, RouterResult, StorageErrorExt},
        payment_methods::vault,
        payments::{self, helpers, operations, CustomerDetails, PaymentAddress, PaymentData},
        utils as core_utils,
    },
    db::StorageInterface,
    routes::AppState,
    services,
    types::{
        self,
        api::{self, PaymentIdTypeExt},
        storage::{self, enums},
        transformers::ForeignInto,
    },
    utils::{self, OptionExt},
};

#[derive(Debug, Clone, Copy)]
pub struct PaymentConfirm;

#[async_trait]
impl Operation<PaymentsRequest> for &PaymentConfirm {
    fn to_validate_request(
        &self,
    ) -> RouterResult<&(dyn ValidateRequest<PaymentsRequest> + Send + Sync)> {
        Ok(*self)
    }
    fn to_get_tracker(
        &self,
    ) -> RouterResult<&(dyn GetTracker<PaymentData, PaymentsRequest> + Send + Sync)> {
        Ok(*self)
    }
    fn to_domain(&self) -> RouterResult<&(dyn Domain<PaymentsRequest>)> {
        Ok(*self)
    }
    fn to_update_tracker(
        &self,
    ) -> RouterResult<&(dyn UpdateTracker<PaymentData, PaymentsRequest> + Send + Sync)> {
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
impl Operation<PaymentsRequest> for PaymentConfirm {
    fn to_validate_request(
        &self,
    ) -> RouterResult<&(dyn ValidateRequest<PaymentsRequest> + Send + Sync)> {
        Ok(self)
    }
    fn to_get_tracker(
        &self,
    ) -> RouterResult<&(dyn GetTracker<PaymentData, PaymentsRequest> + Send + Sync)> {
        Ok(self)
    }
    fn to_domain(&self) -> RouterResult<&dyn Domain<PaymentsRequest>> {
        Ok(self)
    }
    fn to_update_tracker(
        &self,
    ) -> RouterResult<&(dyn UpdateTracker<PaymentData, PaymentsRequest> + Send + Sync)> {
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
impl GetTracker<PaymentData, api::PaymentsRequest> for PaymentConfirm {
    #[instrument(skip_all)]
    async fn get_trackers<'a>(
        &'a self,
        state: &'a AppState,
        payment_id: &api::PaymentIdType,
        request: &api::PaymentsRequest,
        mandate_type: Option<api::MandateTxnType>,
        merchant_account: &storage::MerchantAccount,
    ) -> RouterResult<(
        BoxedOperation<'a, api::PaymentsRequest>,
        PaymentData,
        Option<CustomerDetails>,
    )> {
        let db = &*state.store;
        let merchant_id = &merchant_account.merchant_id;
        let storage_scheme = merchant_account.storage_scheme;
        let (mut payment_intent, mut payment_attempt, currency, amount, connector_response);

        let payment_id = payment_id
            .get_payment_intent_id()
            .change_context(errors::ApiErrorResponse::PaymentNotFound)?;

        let (token, payment_method_type, setup_mandate) =
            helpers::get_token_pm_type_mandate_details(
                state,
                request,
                mandate_type.clone(),
                merchant_account,
            )
            .await?;

        payment_intent = db
            .find_payment_intent_by_payment_id_merchant_id(&payment_id, merchant_id, storage_scheme)
            .await
            .map_err(|error| {
                error.to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)
            })?;

        helpers::authenticate_client_secret(
            request.client_secret.as_ref(),
            payment_intent.client_secret.as_ref(),
        )?;

        let browser_info = request
            .browser_info
            .clone()
            .map(|x| utils::Encode::<types::BrowserInformation>::encode_to_value(&x))
            .transpose()
            .change_context(errors::ApiErrorResponse::InvalidDataValue {
                field_name: "browser_info",
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

        let token = token.or_else(|| payment_attempt.payment_token.clone());

        helpers::validate_pm_or_token_given(
            &request.payment_method,
            &request.payment_method_data,
            &mandate_type,
            &token,
        )?;

        payment_attempt.payment_method = payment_method_type.or(payment_attempt.payment_method);
        payment_attempt.browser_info = browser_info;
        currency = payment_attempt.currency.get_required_value("currency")?;
        amount = payment_attempt.amount.into();

        helpers::validate_customer_id_mandatory_cases(
            request.shipping.is_some(),
            request.billing.is_some(),
            request.setup_future_usage.is_some(),
            &payment_intent
                .customer_id
                .clone()
                .or_else(|| request.customer_id.clone()),
        )?;

        let shipping_address = helpers::get_address_for_payment_request(
            db,
            request.shipping.as_ref(),
            payment_intent.shipping_address_id.as_deref(),
            merchant_id,
            &payment_intent.customer_id,
        )
        .await?;
        let billing_address = helpers::get_address_for_payment_request(
            db,
            request.billing.as_ref(),
            payment_intent.billing_address_id.as_deref(),
            merchant_id,
            &payment_intent.customer_id,
        )
        .await?;

        connector_response = db
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

        payment_intent.shipping_address_id = shipping_address.clone().map(|i| i.address_id);
        payment_intent.billing_address_id = billing_address.clone().map(|i| i.address_id);
        payment_intent.return_url = request.return_url.clone();

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
                    payment_attempt,
                    currency,
                    connector_response,
                    amount,
                    email: request.email.clone(),
                    mandate_id: None,
                    setup_mandate,
                    token,
                    address: PaymentAddress {
                        shipping: shipping_address.as_ref().map(|a| a.foreign_into()),
                        billing: billing_address.as_ref().map(|a| a.foreign_into()),
                    },
                    confirm: request.confirm,
                    payment_method_data: request.payment_method_data.clone(),
                    force_sync: None,
                    refunds: vec![],
                    sessions_token: vec![],
                    card_cvc: request.card_cvc.clone(),
                },
                Some(CustomerDetails {
                    customer_id: request.customer_id.clone(),
                    name: request.name.clone(),
                    email: request.email.clone(),
                    phone: request.phone.clone(),
                    phone_country_code: request.phone_country_code.clone(),
                }),
            )),
        }
    }
}

#[async_trait]
impl Domain<api::PaymentsRequest> for PaymentConfirm {
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
        let (op, payment_method_data) =
            helpers::make_pm_data(Box::new(self), state, payment_data).await?;

        utils::when(payment_method_data.is_none(), || {
            Err(errors::ApiErrorResponse::PaymentMethodNotFound)
        })?;

        Ok((op, payment_method_data))
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
        request: &api::PaymentsRequest,
    ) -> CustomResult<api::ConnectorCallType, errors::ApiErrorResponse> {
        helpers::get_connector_default(state, request.connector).await
    }
}

#[async_trait]
impl UpdateTracker<PaymentData, api::PaymentsRequest> for PaymentConfirm {
    #[instrument(skip_all)]
    async fn update_trackers<'b>(
        &'b self,
        db: &dyn StorageInterface,
        _payment_id: &api::PaymentIdType,
        mut payment_data: PaymentData,
        customer: Option<storage::Customer>,
        storage_scheme: enums::MerchantStorageScheme,
    ) -> RouterResult<(BoxedOperation<'b, api::PaymentsRequest>, PaymentData)> {
        let payment_method = payment_data.payment_attempt.payment_method;
        let browser_info = payment_data.payment_attempt.browser_info.clone();

        let (intent_status, attempt_status) = match payment_data.payment_attempt.authentication_type
        {
            Some(enums::AuthenticationType::NoThreeDs) => (
                enums::IntentStatus::Processing,
                enums::AttemptStatus::Pending,
            ),
            _ => (
                enums::IntentStatus::RequiresCustomerAction,
                enums::AttemptStatus::AuthenticationPending,
            ),
        };

        let connector = payment_data.payment_attempt.connector.clone();
        let payment_token = payment_data.token.clone();

        payment_data.payment_attempt = db
            .update_payment_attempt(
                payment_data.payment_attempt,
                storage::PaymentAttemptUpdate::ConfirmUpdate {
                    amount: payment_data.amount.into(),
                    currency: payment_data.currency,
                    status: attempt_status,
                    payment_method,
                    authentication_type: None,
                    browser_info,
                    connector,
                    payment_token,
                },
                storage_scheme,
            )
            .await
            .map_err(|error| {
                error.to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)
            })?;

        let (shipping_address, billing_address) = (
            payment_data.payment_intent.shipping_address_id.clone(),
            payment_data.payment_intent.billing_address_id.clone(),
        );

        let customer_id = customer.map(|c| c.customer_id);
        let return_url = payment_data.payment_intent.return_url.clone();

        payment_data.payment_intent = db
            .update_payment_intent(
                payment_data.payment_intent,
                storage::PaymentIntentUpdate::Update {
                    amount: payment_data.amount.into(),
                    currency: payment_data.currency,
                    status: intent_status,
                    customer_id,
                    shipping_address_id: shipping_address,
                    billing_address_id: billing_address,
                    return_url,
                },
                storage_scheme,
            )
            .await
            .map_err(|error| {
                error.to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)
            })?;

        Ok((Box::new(self), payment_data))
    }
}

impl ValidateRequest<api::PaymentsRequest> for PaymentConfirm {
    #[instrument(skip_all)]
    fn validate_request<'a, 'b>(
        &'b self,
        request: &api::PaymentsRequest,
        merchant_account: &'a storage::MerchantAccount,
    ) -> RouterResult<(
        BoxedOperation<'b, api::PaymentsRequest>,
        operations::ValidateResult<'a>,
    )> {
        let given_payment_id = match &request.payment_id {
            Some(id_type) => Some(
                id_type
                    .get_payment_intent_id()
                    .change_context(errors::ApiErrorResponse::PaymentNotFound)?,
            ),
            None => None,
        };

        let request_merchant_id = request.merchant_id.as_deref();
        helpers::validate_merchant_id(&merchant_account.merchant_id, request_merchant_id)
            .change_context(errors::ApiErrorResponse::InvalidDataFormat {
                field_name: "merchant_id".to_string(),
                expected_format: "merchant_id from merchant account".to_string(),
            })?;

        helpers::validate_payment_method_fields_present(request)?;

        let mandate_type = helpers::validate_mandate(request)?;
        let payment_id = core_utils::get_or_generate_id("payment_id", &given_payment_id, "pay")?;

        Ok((
            Box::new(self),
            operations::ValidateResult {
                merchant_id: &merchant_account.merchant_id,
                payment_id: api::PaymentIdType::PaymentIntentId(payment_id),
                mandate_type,
                storage_scheme: merchant_account.storage_scheme,
            },
        ))
    }
}

#[async_trait]
impl<FData> DeriveFlow<api::Authorize, FData> for PaymentConfirm
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
        true
    }

    async fn call_connector(
        &self,
        state: &AppState,
        merchant_account: &storage::MerchantAccount,
        payment_data: PaymentData,
        customer: &Option<storage::Customer>,
        call_connector_action: crate::core::payments::CallConnectorAction,
        connector_details: api::ConnectorCallType,
        validate_result: operations::ValidateResult<'_>,
    ) -> RouterResult<PaymentData> {
        let payment_data = if payment_data.payment_attempt.amount == 0 {
            payments::connector_specific_call_connector::<_, api::Verify, _>(
                &self,
                state,
                merchant_account,
                payment_data,
                customer,
                call_connector_action,
                connector_details,
                validate_result,
            )
            .await
        } else {
            payments::connector_specific_call_connector::<_, api::Authorize, _>(
                &self,
                state,
                merchant_account,
                payment_data,
                customer,
                call_connector_action,
                connector_details,
                validate_result,
            )
            .await
        }?;
        vault::Vault::delete_locker_payment_method_by_lookup_key(state, &payment_data.token).await;
        Ok(payment_data)
    }
}
