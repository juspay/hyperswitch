use std::marker::PhantomData;

use async_trait::async_trait;
use common_utils::ext_traits::{AsyncExt, Encode};
use error_stack::{self, ResultExt};
use router_derive::PaymentOperation;
use router_env::{instrument, tracing};
use uuid::Uuid;

use super::{BoxedOperation, Domain, GetTracker, Operation, UpdateTracker, ValidateRequest};
use crate::{
    consts,
    core::{
        errors::{self, CustomResult, RouterResult, StorageErrorExt},
        payments::{self, helpers, operations, CustomerDetails, PaymentAddress, PaymentData},
        utils as core_utils,
    },
    db::StorageInterface,
    routes::AppState,
    types::{
        self,
        api::{self, PaymentIdTypeExt},
        storage::{
            self,
            enums::{self, IntentStatus},
        },
        transformers::ForeignInto,
    },
    utils::OptionExt,
};
#[derive(Debug, Clone, Copy, PaymentOperation)]
#[operation(ops = "all", flow = "authorize")]
pub struct PaymentCreate;

#[async_trait]
impl<F: Send + Clone> GetTracker<F, PaymentData<F>, api::PaymentsRequest> for PaymentCreate {
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
        let db = &*state.store;

        let merchant_id = &merchant_account.merchant_id;
        let storage_scheme = merchant_account.storage_scheme;

        let (payment_intent, payment_attempt, connector_response);

        let money @ (amount, currency) = payments_create_request_validation(request)?;

        let payment_id = payment_id
            .get_payment_intent_id()
            .change_context(errors::ApiErrorResponse::PaymentNotFound)?;

        let (token, payment_method_type, setup_mandate) =
            helpers::get_token_pm_type_mandate_details(
                state,
                request,
                mandate_type,
                merchant_account,
            )
            .await?;

        let shipping_address = helpers::get_address_for_payment_request(
            db,
            request.shipping.as_ref(),
            None,
            merchant_id,
            &request.customer_id,
        )
        .await?;

        let billing_address = helpers::get_address_for_payment_request(
            db,
            request.billing.as_ref(),
            None,
            merchant_id,
            &request.customer_id,
        )
        .await?;

        let browser_info = request
            .browser_info
            .clone()
            .map(|x| {
                common_utils::ext_traits::Encode::<types::BrowserInformation>::encode_to_value(&x)
            })
            .transpose()
            .change_context(errors::ApiErrorResponse::InvalidDataValue {
                field_name: "browser_info",
            })?;

        payment_attempt = db
            .insert_payment_attempt(
                Self::make_payment_attempt(
                    &payment_id,
                    merchant_id,
                    money,
                    payment_method_type,
                    request,
                    browser_info,
                ),
                storage_scheme,
            )
            .await
            .map_err(|err| {
                err.to_duplicate_response(errors::ApiErrorResponse::DuplicatePayment {
                    payment_id: payment_id.clone(),
                })
            })?;

        payment_intent = db
            .insert_payment_intent(
                Self::make_payment_intent(
                    &payment_id,
                    merchant_id,
                    money,
                    request,
                    shipping_address.clone().map(|x| x.address_id),
                    billing_address.clone().map(|x| x.address_id),
                )?,
                storage_scheme,
            )
            .await
            .map_err(|err| {
                err.to_duplicate_response(errors::ApiErrorResponse::DuplicatePayment {
                    payment_id: payment_id.clone(),
                })
            })?;
        connector_response = db
            .insert_connector_response(
                Self::make_connector_response(&payment_attempt),
                storage_scheme,
            )
            .await
            .map_err(|err| {
                err.to_duplicate_response(errors::ApiErrorResponse::DuplicatePayment {
                    payment_id: payment_id.clone(),
                })
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

        let operation = payments::if_not_create_change_operation::<_, F>(
            payment_intent.status,
            request.confirm,
            self,
        );

        Ok((
            operation,
            PaymentData {
                flow: PhantomData,
                payment_intent,
                payment_attempt,
                currency,
                amount,
                email: request.email.clone(),
                mandate_id,
                setup_mandate,
                token,
                address: PaymentAddress {
                    shipping: shipping_address.as_ref().map(|a| a.foreign_into()),
                    billing: billing_address.as_ref().map(|a| a.foreign_into()),
                },
                confirm: request.confirm,
                payment_method_data: request.payment_method_data.clone(),
                refunds: vec![],
                force_sync: None,
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
impl<F: Clone + Send> Domain<F, api::PaymentsRequest> for PaymentCreate {
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
        _storage_scheme: enums::MerchantStorageScheme,
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
        request: &api::PaymentsRequest,
        _previously_used_connector: Option<&String>,
    ) -> CustomResult<api::ConnectorCallType, errors::ApiErrorResponse> {
        let request_connector = request.connector.map(|connector| connector.to_string());
        helpers::get_connector_default(state, request_connector.as_ref()).await
    }
}

#[async_trait]
impl<F: Clone> UpdateTracker<F, PaymentData<F>, api::PaymentsRequest> for PaymentCreate {
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
        let status = match payment_data.payment_intent.status {
            IntentStatus::RequiresPaymentMethod => match payment_data.payment_method_data {
                Some(_) => Some(IntentStatus::RequiresConfirmation),
                _ => None,
            },
            IntentStatus::RequiresConfirmation => {
                if let Some(true) = payment_data.confirm {
                    Some(IntentStatus::Processing)
                } else {
                    None
                }
            }
            _ => None,
        };

        let payment_token = payment_data.token.clone();
        let connector = payment_data.payment_attempt.connector.clone();

        payment_data.payment_attempt = db
            .update_payment_attempt(
                payment_data.payment_attempt,
                storage::PaymentAttemptUpdate::UpdateTrackers {
                    payment_token,
                    connector,
                },
                storage_scheme,
            )
            .await
            .map_err(|error| {
                error.to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)
            })?;

        let customer_id = payment_data.payment_intent.customer_id.clone();
        payment_data.payment_intent = db
            .update_payment_intent(
                payment_data.payment_intent,
                storage::PaymentIntentUpdate::ReturnUrlUpdate {
                    return_url: None,
                    status,
                    customer_id,
                    shipping_address_id: None,
                    billing_address_id: None,
                },
                storage_scheme,
            )
            .await
            .map_err(|error| {
                error.to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)
            })?;

        // payment_data.mandate_id = response.and_then(|router_data| router_data.request.mandate_id);

        Ok((
            payments::is_confirm(self, payment_data.confirm),
            payment_data,
        ))
    }
}

impl<F: Send + Clone> ValidateRequest<F, api::PaymentsRequest> for PaymentCreate {
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
            .change_context(errors::ApiErrorResponse::MerchantAccountNotFound)?;

        helpers::validate_request_amount_and_amount_to_capture(
            request.amount,
            request.amount_to_capture,
        )
        .change_context(errors::ApiErrorResponse::InvalidDataFormat {
            field_name: "amount_to_capture".to_string(),
            expected_format: "amount_to_capture lesser than amount".to_string(),
        })?;

        helpers::validate_payment_method_fields_present(request)?;

        let payment_id = core_utils::get_or_generate_id("payment_id", &given_payment_id, "pay")?;

        let mandate_type = helpers::validate_mandate(request)?;

        if request.confirm.unwrap_or(false) {
            helpers::validate_pm_or_token_given(
                &request.payment_method,
                &request.payment_method_data,
                &mandate_type,
                &request.payment_token,
            )?;

            helpers::validate_customer_id_mandatory_cases(
                request.shipping.is_some(),
                request.billing.is_some(),
                request.setup_future_usage.is_some(),
                &request.customer_id,
            )?;
        }

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

impl PaymentCreate {
    #[instrument(skip_all)]
    fn make_payment_attempt(
        payment_id: &str,
        merchant_id: &str,
        money: (api::Amount, enums::Currency),
        payment_method: Option<enums::PaymentMethodType>,
        request: &api::PaymentsRequest,
        browser_info: Option<serde_json::Value>,
    ) -> storage::PaymentAttemptNew {
        let created_at @ modified_at @ last_synced = Some(common_utils::date_time::now());
        let status =
            helpers::payment_attempt_status_fsm(&request.payment_method_data, request.confirm);
        let (amount, currency) = (money.0, Some(money.1));
        storage::PaymentAttemptNew {
            payment_id: payment_id.to_string(),
            merchant_id: merchant_id.to_string(),
            attempt_id: Uuid::new_v4().simple().to_string(),
            status,
            amount: amount.into(),
            currency,
            payment_method,
            capture_method: request.capture_method.map(ForeignInto::foreign_into),
            capture_on: request.capture_on,
            confirm: request.confirm.unwrap_or(false),
            created_at,
            modified_at,
            last_synced,
            authentication_type: request.authentication_type.map(ForeignInto::foreign_into),
            browser_info,
            payment_experience: request.payment_experience.map(ForeignInto::foreign_into),
            payment_issuer: request.payment_issuer.map(ForeignInto::foreign_into),
            ..storage::PaymentAttemptNew::default()
        }
    }

    #[instrument(skip_all)]
    fn make_payment_intent(
        payment_id: &str,
        merchant_id: &str,
        money: (api::Amount, enums::Currency),
        request: &api::PaymentsRequest,
        shipping_address_id: Option<String>,
        billing_address_id: Option<String>,
    ) -> RouterResult<storage::PaymentIntentNew> {
        let created_at @ modified_at @ last_synced = Some(common_utils::date_time::now());
        let status =
            helpers::payment_intent_status_fsm(&request.payment_method_data, request.confirm);
        let client_secret =
            crate::utils::generate_id(consts::ID_LENGTH, format!("{payment_id}_secret").as_str());
        let (amount, currency) = (money.0, Some(money.1));
        let metadata = request
            .metadata
            .as_ref()
            .map(Encode::<api_models::payments::Metadata>::encode_to_value)
            .transpose()
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Encoding Metadata to value failed")?;
        Ok(storage::PaymentIntentNew {
            payment_id: payment_id.to_string(),
            merchant_id: merchant_id.to_string(),
            status,
            amount: amount.into(),
            currency,
            description: request.description.clone(),
            created_at,
            modified_at,
            last_synced,
            client_secret: Some(client_secret),
            setup_future_usage: request.setup_future_usage.map(ForeignInto::foreign_into),
            off_session: request.off_session,
            return_url: request.return_url.clone(),
            shipping_address_id,
            billing_address_id,
            statement_descriptor_name: request.statement_descriptor_name.clone(),
            statement_descriptor_suffix: request.statement_descriptor_suffix.clone(),
            metadata,
            ..storage::PaymentIntentNew::default()
        })
    }

    #[instrument(skip_all)]
    pub fn make_connector_response(
        payment_attempt: &storage::PaymentAttempt,
    ) -> storage::ConnectorResponseNew {
        storage::ConnectorResponseNew {
            payment_id: payment_attempt.payment_id.clone(),
            merchant_id: payment_attempt.merchant_id.clone(),
            attempt_id: payment_attempt.attempt_id.clone(),
            created_at: payment_attempt.created_at,
            modified_at: payment_attempt.modified_at,
            connector_name: payment_attempt.connector.clone(),
            connector_transaction_id: None,
            authentication_data: None,
            encoded_data: None,
        }
    }
}

#[instrument(skip_all)]
pub fn payments_create_request_validation(
    req: &api::PaymentsRequest,
) -> RouterResult<(api::Amount, enums::Currency)> {
    let currency = req
        .currency
        .map(ForeignInto::foreign_into)
        .get_required_value("currency")?;
    let amount = req.amount.get_required_value("amount")?;
    Ok((amount, currency))
}
