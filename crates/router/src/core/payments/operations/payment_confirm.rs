use std::marker::PhantomData;

use async_trait::async_trait;
use error_stack::{report, ResultExt};
use router_derive::PaymentOperation;
use router_env::{instrument, tracing};

use super::{BoxedOperation, Domain, GetTracker, Operation, UpdateTracker, ValidateRequest};
use crate::{
    core::{
        errors::{self, RouterResult, StorageErrorExt},
        payments::{helpers, operations, CustomerDetails, PaymentAddress, PaymentData},
        utils as core_utils,
    },
    db::StorageInterface,
    routes::AppState,
    types::{
        self,
        api::{self, PaymentIdTypeExt},
        storage::{self, enums},
        transformers::ForeignInto,
    },
    utils::{self, OptionExt},
};

#[derive(Debug, Clone, Copy, PaymentOperation)]
#[operation(ops = "all", flow = "authorize")]
pub struct PaymentConfirm;

#[async_trait]
impl<F: Send + Clone> GetTracker<F, PaymentData<F>, api::PaymentsRequest> for PaymentConfirm {
    #[instrument(skip_all)]
    async fn get_trackers<'a>(
        &'a self,
        state: &'a AppState,
        payment_id: &api::PaymentIdType,
        merchant_id: &str,
        request: &api::PaymentsRequest,
        mandate_type: Option<api::MandateTxnType>,
        storage_scheme: enums::MerchantStorageScheme,
    ) -> RouterResult<(
        BoxedOperation<'a, F, api::PaymentsRequest>,
        PaymentData<F>,
        Option<CustomerDetails>,
    )> {
        let db = &*state.store;
        let (mut payment_intent, mut payment_attempt, currency, amount, connector_response);

        let payment_id = payment_id
            .get_payment_intent_id()
            .change_context(errors::ApiErrorResponse::PaymentNotFound)?;

        let (token, payment_method_type, setup_mandate) =
            helpers::get_token_pm_type_mandate_details(state, request, mandate_type, merchant_id)
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
        payment_attempt.payment_method = payment_method_type.or(payment_attempt.payment_method);

        payment_attempt.payment_method = payment_method_type.or(payment_attempt.payment_method);
        payment_attempt.browser_info = browser_info;
        currency = payment_attempt.currency.get_required_value("currency")?;
        amount = payment_attempt.amount.into();

        connector_response = db
            .find_connector_response_by_payment_id_merchant_id_txn_id(
                &payment_attempt.payment_id,
                &payment_attempt.merchant_id,
                &payment_attempt.txn_id,
                storage_scheme,
            )
            .await
            .map_err(|error| {
                error.to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)
            })?;

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

        payment_intent.shipping_address_id = shipping_address.clone().map(|i| i.address_id);
        payment_intent.billing_address_id = billing_address.clone().map(|i| i.address_id);

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
                    flow: PhantomData,
                    payment_intent,
                    payment_attempt,
                    currency,
                    connector_response,
                    amount,
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
impl<F: Clone> UpdateTracker<F, PaymentData<F>, api::PaymentsRequest> for PaymentConfirm {
    #[instrument(skip_all)]
    async fn update_trackers<'b>(
        &'b self,
        db: &dyn StorageInterface,
        _payment_id: &api::PaymentIdType,
        mut payment_data: PaymentData<F>,
        _customer: Option<storage::Customer>,
        storage_scheme: enums::MerchantStorageScheme,
    ) -> RouterResult<(BoxedOperation<'b, F, api::PaymentsRequest>, PaymentData<F>)>
    where
        F: 'b + Send,
    {
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
                enums::AttemptStatus::PendingVbv,
            ),
        };

        let connector = payment_data.payment_attempt.connector.clone();

        payment_data.payment_attempt = db
            .update_payment_attempt(
                payment_data.payment_attempt,
                storage::PaymentAttemptUpdate::ConfirmUpdate {
                    status: attempt_status,
                    payment_method,
                    browser_info,
                    connector,
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

        payment_data.payment_intent = db
            .update_payment_intent(
                payment_data.payment_intent,
                storage::PaymentIntentUpdate::MerchantStatusUpdate {
                    status: intent_status,
                    shipping_address_id: shipping_address,
                    billing_address_id: billing_address,
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

impl<F: Send + Clone> ValidateRequest<F, api::PaymentsRequest> for PaymentConfirm {
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
