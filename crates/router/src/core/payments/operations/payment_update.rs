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
        payments::{self, helpers, operations, CustomerDetails, PaymentAddress, PaymentData},
        utils as core_utils,
    },
    db::StorageInterface,
    routes::AppState,
    types::{
        api::{self, PaymentIdTypeExt},
        storage::{self, enums as storage_enums},
        transformers::ForeignInto,
    },
    utils::OptionExt,
};
#[derive(Debug, Clone, Copy, PaymentOperation)]
#[operation(ops = "all", flow = "authorize")]
pub struct PaymentUpdate;

#[async_trait]
impl<F: Send + Clone> GetTracker<F, PaymentData<F>, api::PaymentsRequest> for PaymentUpdate {
    #[instrument(skip_all)]
    async fn get_trackers<'a>(
        &'a self,
        state: &'a AppState,
        payment_id: &api::PaymentIdType,
        request: &api::PaymentsRequest,
        mandate_type: Option<api::MandateTxnType>,
        merchant_account: &storage::MerchantAccount,
    ) -> RouterResult<(
        BoxedOperation<'a, F, api::PaymentsRequest>,
        PaymentData<F>,
        Option<CustomerDetails>,
    )> {
        let (mut payment_intent, mut payment_attempt, currency): (_, _, storage_enums::Currency);

        let payment_id = payment_id
            .get_payment_intent_id()
            .change_context(errors::ApiErrorResponse::PaymentNotFound)?;
        let merchant_id = &merchant_account.merchant_id;
        let storage_scheme = merchant_account.storage_scheme;

        let db = &*state.store;

        payment_intent = db
            .find_payment_intent_by_payment_id_merchant_id(&payment_id, merchant_id, storage_scheme)
            .await
            .map_err(|error| {
                error.to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)
            })?;

        payment_intent.setup_future_usage = request
            .setup_future_usage
            .map(ForeignInto::foreign_into)
            .or(payment_intent.setup_future_usage);

        helpers::validate_payment_status_against_not_allowed_statuses(
            &payment_intent.status,
            &[
                storage_enums::IntentStatus::Failed,
                storage_enums::IntentStatus::Succeeded,
                storage_enums::IntentStatus::RequiresCapture,
            ],
            "update",
        )?;

        helpers::authenticate_client_secret(
            request.client_secret.as_ref(),
            payment_intent.client_secret.as_ref(),
        )?;

        let (token, payment_method_type, setup_mandate) =
            helpers::get_token_pm_type_mandate_details(
                state,
                request,
                mandate_type.clone(),
                merchant_account,
            )
            .await?;

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

        currency = match request.currency {
            Some(cur) => cur.foreign_into(),
            None => payment_attempt.currency.get_required_value("currency")?,
        };

        payment_attempt.payment_method = payment_method_type.or(payment_attempt.payment_method);

        let amount = request
            .amount
            .unwrap_or_else(|| payment_attempt.amount.into());

        if request.confirm.unwrap_or(false) {
            helpers::validate_customer_id_mandatory_cases(
                request.shipping.is_some(),
                request.billing.is_some(),
                request.setup_future_usage.is_some(),
                &payment_intent
                    .customer_id
                    .clone()
                    .or_else(|| request.customer_id.clone()),
            )?;
        }

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

        payment_intent.shipping_address_id = shipping_address.clone().map(|x| x.address_id);
        payment_intent.billing_address_id = billing_address.clone().map(|x| x.address_id);
        payment_intent.return_url = request.return_url.clone();

        let token = token.or_else(|| payment_attempt.payment_token.clone());

        if request.confirm.unwrap_or(false) {
            helpers::validate_pm_or_token_given(
                &request.payment_method,
                &request.payment_method_data,
                &mandate_type,
                &token,
            )?;
        }

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

        let mandate_id = request
            .mandate_id
            .as_ref()
            .async_and_then(|mandate_id| async {
                let mandate = db
                    .find_mandate_by_merchant_id_mandate_id(merchant_id, mandate_id)
                    .await
                    .change_context(errors::ApiErrorResponse::MandateNotFound);
                Some(mandate.map(|mandate_obj| api_models::payments::MandateIds {
                    mandate_id: mandate_obj.mandate_id,
                    connector_mandate_id: mandate_obj.connector_mandate_id,
                }))
            })
            .await
            .transpose()?;
        let next_operation: BoxedOperation<'a, F, api::PaymentsRequest> =
            if request.confirm.unwrap_or(false) {
                Box::new(operations::PaymentConfirm)
            } else {
                Box::new(self)
            };

        payment_intent.status = match request.payment_method_data.as_ref() {
            Some(_) => {
                if request.confirm.unwrap_or(false) {
                    payment_intent.status
                } else {
                    storage_enums::IntentStatus::RequiresConfirmation
                }
            }
            None => storage_enums::IntentStatus::RequiresPaymentMethod,
        };

        Ok((
            next_operation,
            PaymentData {
                flow: PhantomData,
                payment_intent,
                payment_attempt,
                currency,
                amount,
                email: request.email.clone(),
                mandate_id,
                token,
                setup_mandate,
                address: PaymentAddress {
                    shipping: shipping_address.as_ref().map(|a| a.foreign_into()),
                    billing: billing_address.as_ref().map(|a| a.foreign_into()),
                },
                confirm: request.confirm,
                payment_method_data: request.payment_method_data.clone(),
                force_sync: None,
                refunds: vec![],
                connector_response,
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
        ))
    }
}

#[async_trait]
impl<F: Clone + Send> Domain<F, api::PaymentsRequest> for PaymentUpdate {
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
        payment_data: &mut PaymentData<F>,
        _storage_scheme: storage_enums::MerchantStorageScheme,
    ) -> RouterResult<(
        BoxedOperation<'a, F, api::PaymentsRequest>,
        Option<api::PaymentMethod>,
    )> {
        helpers::make_pm_data(Box::new(self), state, payment_data).await
    }

    #[instrument(skip_all)]
    async fn add_task_to_process_tracker<'a>(
        &'a self,
        _state: &'a AppState,
        _payment_attempt: &storage::PaymentAttempt,
    ) -> CustomResult<(), errors::ApiErrorResponse> {
        Ok(())
    }

    async fn get_connector<'a>(
        &'a self,
        _merchant_account: &storage::MerchantAccount,
        state: &AppState,
        _request: &api::PaymentsRequest,
        previously_used_connector: Option<&String>,
    ) -> CustomResult<api::ConnectorCallType, errors::ApiErrorResponse> {
        helpers::get_connector_default(state, previously_used_connector).await
    }
}

#[async_trait]
impl<F: Clone> UpdateTracker<F, PaymentData<F>, api::PaymentsRequest> for PaymentUpdate {
    #[instrument(skip_all)]
    async fn update_trackers<'b>(
        &'b self,
        db: &dyn StorageInterface,
        _payment_id: &api::PaymentIdType,
        mut payment_data: PaymentData<F>,
        customer: Option<storage::Customer>,
        storage_scheme: storage_enums::MerchantStorageScheme,
    ) -> RouterResult<(BoxedOperation<'b, F, api::PaymentsRequest>, PaymentData<F>)>
    where
        F: 'b + Send,
    {
        let is_payment_method_unavailable =
            payment_data.payment_attempt.payment_method_id.is_none()
                && payment_data.payment_intent.status
                    == storage_enums::IntentStatus::RequiresPaymentMethod;

        let payment_method = payment_data.payment_attempt.payment_method;

        let get_attempt_status = || {
            if is_payment_method_unavailable {
                storage_enums::AttemptStatus::PaymentMethodAwaited
            } else {
                storage_enums::AttemptStatus::ConfirmationAwaited
            }
        };

        payment_data.payment_attempt = db
            .update_payment_attempt(
                payment_data.payment_attempt,
                storage::PaymentAttemptUpdate::Update {
                    amount: payment_data.amount.into(),
                    currency: payment_data.currency,
                    status: get_attempt_status(),
                    authentication_type: None,
                    payment_method,
                    payment_token: payment_data.token.clone(),
                },
                storage_scheme,
            )
            .await
            .map_err(|error| {
                error.to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)
            })?;

        let customer_id = customer.map(|c| c.customer_id);

        let intent_status = {
            let current_intent_status = payment_data.payment_intent.status;
            if is_payment_method_unavailable {
                storage_enums::IntentStatus::RequiresPaymentMethod
            } else if !payment_data.confirm.unwrap_or(true)
                || current_intent_status == storage_enums::IntentStatus::RequiresCustomerAction
            {
                storage_enums::IntentStatus::RequiresConfirmation
            } else {
                payment_data.payment_intent.status
            }
        };

        let (shipping_address, billing_address) = (
            payment_data.payment_intent.shipping_address_id.clone(),
            payment_data.payment_intent.billing_address_id.clone(),
        );

        let return_url = payment_data.payment_intent.return_url.clone();
        let setup_future_usage = payment_data.payment_intent.setup_future_usage;

        payment_data.payment_intent = db
            .update_payment_intent(
                payment_data.payment_intent,
                storage::PaymentIntentUpdate::Update {
                    amount: payment_data.amount.into(),
                    currency: payment_data.currency,
                    setup_future_usage,
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

        payment_data.mandate_id = payment_data.mandate_id.clone();

        Ok((
            payments::is_confirm(self, payment_data.confirm),
            payment_data,
        ))
    }
}

impl<F: Send + Clone> ValidateRequest<F, api::PaymentsRequest> for PaymentUpdate {
    #[instrument(skip_all)]
    fn validate_request<'a, 'b>(
        &'b self,
        request: &api::PaymentsRequest,
        merchant_account: &'a storage::MerchantAccount,
    ) -> RouterResult<(
        BoxedOperation<'b, F, api::PaymentsRequest>,
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

        helpers::validate_request_amount_and_amount_to_capture(
            request.amount,
            request.amount_to_capture,
        )
        .change_context(errors::ApiErrorResponse::InvalidDataFormat {
            field_name: "amount_to_capture".to_string(),
            expected_format: "amount_to_capture lesser than or equal to amount".to_string(),
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
