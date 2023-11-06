use std::marker::PhantomData;

use api_models::{enums::FrmSuggestion, payment_methods};
use async_trait::async_trait;
use common_utils::ext_traits::{AsyncExt, Encode};
use error_stack::ResultExt;
use futures::FutureExt;
use router_derive::PaymentOperation;
use router_env::{instrument, tracing};

use super::{BoxedOperation, Domain, GetTracker, Operation, UpdateTracker, ValidateRequest};
use crate::{
    core::{
        errors::{self, CustomResult, RouterResult, StorageErrorExt},
        payment_methods::PaymentMethodRetrieve,
        payments::{self, helpers, operations, CustomerDetails, PaymentAddress, PaymentData},
    },
    db::StorageInterface,
    routes::AppState,
    services,
    types::{
        self,
        api::{self, PaymentIdTypeExt},
        domain,
        storage::{self, enums as storage_enums},
    },
    utils::{self, OptionExt},
};

#[derive(Debug, Clone, Copy, PaymentOperation)]
#[operation(ops = "all", flow = "authorize")]
pub struct PaymentConfirm;
#[async_trait]
impl<F: Send + Clone, Ctx: PaymentMethodRetrieve>
    GetTracker<F, PaymentData<F>, api::PaymentsRequest, Ctx> for PaymentConfirm
{
    #[instrument(skip_all)]
    async fn get_trackers<'a>(
        &'a self,
        state: &'a AppState,
        payment_id: &api::PaymentIdType,
        request: &api::PaymentsRequest,
        mandate_type: Option<api::MandateTransactionType>,
        merchant_account: &domain::MerchantAccount,
        key_store: &domain::MerchantKeyStore,
        auth_flow: services::AuthFlow,
    ) -> RouterResult<(
        BoxedOperation<'a, F, api::PaymentsRequest, Ctx>,
        PaymentData<F>,
        Option<CustomerDetails>,
    )> {
        let db = &*state.store;
        let merchant_id = &merchant_account.merchant_id;
        let storage_scheme = merchant_account.storage_scheme;
        let (currency, amount);

        let payment_id = payment_id
            .get_payment_intent_id()
            .change_context(errors::ApiErrorResponse::PaymentNotFound)?;

        // Stage 1

        let payment_intent_fut = db
            .find_payment_intent_by_payment_id_merchant_id(&payment_id, merchant_id, storage_scheme)
            .map(|x| x.change_context(errors::ApiErrorResponse::PaymentNotFound));

        let mandate_details_fut = helpers::get_token_pm_type_mandate_details(
            state,
            request,
            mandate_type.clone(),
            merchant_account,
        );

        let (mut payment_intent, mandate_details) =
            futures::try_join!(payment_intent_fut, mandate_details_fut)?;

        helpers::validate_customer_access(&payment_intent, auth_flow, request)?;

        helpers::validate_payment_status_against_not_allowed_statuses(
            &payment_intent.status,
            &[
                storage_enums::IntentStatus::Cancelled,
                storage_enums::IntentStatus::Succeeded,
                storage_enums::IntentStatus::Processing,
                storage_enums::IntentStatus::RequiresCapture,
                storage_enums::IntentStatus::RequiresMerchantAction,
            ],
            "confirm",
        )?;

        let intent_fulfillment_time = helpers::get_merchant_fullfillment_time(
            payment_intent.payment_link_id.clone(),
            merchant_account.intent_fulfillment_time,
            db,
        )
        .await?;

        helpers::authenticate_client_secret(
            request.client_secret.as_ref(),
            &payment_intent,
            intent_fulfillment_time,
        )?;

        let customer_details = helpers::get_customer_details_from_request(request);

        // Stage 2

        let attempt_id = payment_intent.active_attempt.get_id();
        let payment_attempt_fut = db
            .find_payment_attempt_by_payment_id_merchant_id_attempt_id(
                payment_intent.payment_id.as_str(),
                merchant_id,
                attempt_id.as_str(),
                storage_scheme,
            )
            .map(|x| x.to_not_found_response(errors::ApiErrorResponse::PaymentNotFound));

        let shipping_address_fut = helpers::create_or_find_address_for_payment_by_request(
            db,
            request.shipping.as_ref(),
            payment_intent.shipping_address_id.as_deref(),
            merchant_id,
            payment_intent
                .customer_id
                .as_ref()
                .or(customer_details.customer_id.as_ref()),
            key_store,
            &payment_intent.payment_id,
            merchant_account.storage_scheme,
        );

        let billing_address_fut = helpers::create_or_find_address_for_payment_by_request(
            db,
            request.billing.as_ref(),
            payment_intent.billing_address_id.as_deref(),
            merchant_id,
            payment_intent
                .customer_id
                .as_ref()
                .or(customer_details.customer_id.as_ref()),
            key_store,
            &payment_intent.payment_id,
            merchant_account.storage_scheme,
        );

        let config_update_fut = request
            .merchant_connector_details
            .to_owned()
            .async_map(|mcd| async {
                helpers::insert_merchant_connector_creds_to_config(
                    db,
                    merchant_account.merchant_id.as_str(),
                    mcd,
                )
                .await
            })
            .map(|x| x.transpose());

        let (mut payment_attempt, shipping_address, billing_address, connector_response) =
            match payment_intent.status {
                api_models::enums::IntentStatus::RequiresCustomerAction
                | api_models::enums::IntentStatus::RequiresMerchantAction
                | api_models::enums::IntentStatus::RequiresPaymentMethod
                | api_models::enums::IntentStatus::RequiresConfirmation => {
                    let attempt_type = helpers::AttemptType::SameOld;

                    let attempt_id = payment_intent.active_attempt.get_id();
                    let connector_response_fut = attempt_type.get_connector_response(
                        db,
                        &payment_intent.payment_id,
                        &payment_intent.merchant_id,
                        attempt_id.as_str(),
                        storage_scheme,
                    );

                    let (payment_attempt, shipping_address, billing_address, connector_response, _) =
                        futures::try_join!(
                            payment_attempt_fut,
                            shipping_address_fut,
                            billing_address_fut,
                            connector_response_fut,
                            config_update_fut
                        )?;

                    (
                        payment_attempt,
                        shipping_address,
                        billing_address,
                        connector_response,
                    )
                }
                _ => {
                    let (mut payment_attempt, shipping_address, billing_address, _) = futures::try_join!(
                        payment_attempt_fut,
                        shipping_address_fut,
                        billing_address_fut,
                        config_update_fut
                    )?;

                    let attempt_type = helpers::get_attempt_type(
                        &payment_intent,
                        &payment_attempt,
                        request,
                        "confirm",
                    )?;

                    (payment_intent, payment_attempt) = attempt_type
                        .modify_payment_intent_and_payment_attempt(
                            // 3
                            request,
                            payment_intent,
                            payment_attempt,
                            db,
                            storage_scheme,
                        )
                        .await?;

                    let connector_response = attempt_type
                        .get_or_insert_connector_response(&payment_attempt, db, storage_scheme)
                        .await?;

                    (
                        payment_attempt,
                        shipping_address,
                        billing_address,
                        connector_response,
                    )
                }
            };

        payment_intent.order_details = request
            .get_order_details_as_value()
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Failed to convert order details to value")?
            .or(payment_intent.order_details);

        payment_intent.setup_future_usage = request
            .setup_future_usage
            .or(payment_intent.setup_future_usage);

        let (
            token,
            payment_method,
            payment_method_type,
            mut setup_mandate,
            recurring_mandate_payment_data,
            mandate_connector,
        ) = mandate_details;

        let browser_info = request
            .browser_info
            .clone()
            .or(payment_attempt.browser_info)
            .map(|x| utils::Encode::<types::BrowserInformation>::encode_to_value(&x))
            .transpose()
            .change_context(errors::ApiErrorResponse::InvalidDataValue {
                field_name: "browser_info",
            })?;

        helpers::validate_card_data(request.payment_method_data.clone())?;

        let token = token.or_else(|| payment_attempt.payment_token.clone());

        helpers::validate_pm_or_token_given(
            &request.payment_method,
            &request.payment_method_data,
            &request.payment_method_type,
            &mandate_type,
            &token,
        )?;

        payment_attempt.payment_method = payment_method.or(payment_attempt.payment_method);
        payment_attempt.browser_info = browser_info;
        payment_attempt.payment_method_type =
            payment_method_type.or(payment_attempt.payment_method_type);

        payment_attempt.payment_experience = request
            .payment_experience
            .or(payment_attempt.payment_experience);

        payment_attempt.capture_method = request.capture_method.or(payment_attempt.capture_method);

        currency = payment_attempt.currency.get_required_value("currency")?;
        amount = payment_attempt.amount.into();

        helpers::validate_customer_id_mandatory_cases(
            request.setup_future_usage.is_some(),
            &payment_intent
                .customer_id
                .clone()
                .or_else(|| customer_details.customer_id.clone()),
        )?;

        let creds_identifier = request
            .merchant_connector_details
            .as_ref()
            .map(|mcd| mcd.creds_identifier.to_owned());

        payment_intent.shipping_address_id = shipping_address.clone().map(|i| i.address_id);
        payment_intent.billing_address_id = billing_address.clone().map(|i| i.address_id);
        payment_intent.return_url = request
            .return_url
            .as_ref()
            .map(|a| a.to_string())
            .or(payment_intent.return_url);

        payment_intent.allowed_payment_method_types = request
            .get_allowed_payment_method_types_as_value()
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Error converting allowed_payment_types to Value")?
            .or(payment_intent.allowed_payment_method_types);

        payment_intent.connector_metadata = request
            .get_connector_metadata_as_value()
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Error converting connector_metadata to Value")?
            .or(payment_intent.connector_metadata);

        payment_intent.feature_metadata = request
            .get_feature_metadata_as_value()
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Error converting feature_metadata to Value")?
            .or(payment_intent.feature_metadata);
        payment_intent.metadata = request.metadata.clone().or(payment_intent.metadata);
        payment_attempt.business_sub_label = request
            .business_sub_label
            .clone()
            .or(payment_attempt.business_sub_label);

        // The operation merges mandate data from both request and payment_attempt
        setup_mandate = setup_mandate.map(|mut sm| {
            sm.mandate_type = payment_attempt.mandate_details.clone().or(sm.mandate_type);
            sm
        });

        // populate payment_data.surcharge_details from request
        let surcharge_details = request.surcharge_details.map(|surcharge_details| {
            payment_methods::SurchargeDetailsResponse {
                surcharge: payment_methods::Surcharge::Fixed(surcharge_details.surcharge_amount),
                tax_on_surcharge: None,
                surcharge_amount: surcharge_details.surcharge_amount,
                tax_on_surcharge_amount: surcharge_details.tax_amount.unwrap_or(0),
                final_amount: payment_attempt.amount
                    + surcharge_details.surcharge_amount
                    + surcharge_details.tax_amount.unwrap_or(0),
            }
        });

        Ok((
            Box::new(self),
            PaymentData {
                flow: PhantomData,
                payment_intent,
                payment_attempt,
                currency,
                connector_response,
                amount,
                email: request.email.clone(),
                mandate_id: None,
                mandate_connector,
                setup_mandate,
                token,
                address: PaymentAddress {
                    shipping: shipping_address.as_ref().map(|a| a.into()),
                    billing: billing_address.as_ref().map(|a| a.into()),
                },
                confirm: request.confirm,
                payment_method_data: request.payment_method_data.clone(),
                force_sync: None,
                refunds: vec![],
                disputes: vec![],
                attempts: None,
                sessions_token: vec![],
                card_cvc: request.card_cvc.clone(),
                creds_identifier,
                pm_token: None,
                connector_customer_id: None,
                recurring_mandate_payment_data,
                ephemeral_key: None,
                multiple_capture_data: None,
                redirect_response: None,
                surcharge_details,
                frm_message: None,
                payment_link_data: None,
            },
            Some(customer_details),
        ))
    }
}

#[async_trait]
impl<F: Clone + Send, Ctx: PaymentMethodRetrieve> Domain<F, api::PaymentsRequest, Ctx>
    for PaymentConfirm
{
    #[instrument(skip_all)]
    async fn get_or_create_customer_details<'a>(
        &'a self,
        db: &dyn StorageInterface,
        payment_data: &mut PaymentData<F>,
        request: Option<CustomerDetails>,
        key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<
        (
            BoxedOperation<'a, F, api::PaymentsRequest, Ctx>,
            Option<domain::Customer>,
        ),
        errors::StorageError,
    > {
        helpers::create_customer_if_not_exist(
            Box::new(self),
            db,
            payment_data,
            request,
            &key_store.merchant_id,
            key_store,
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
        BoxedOperation<'a, F, api::PaymentsRequest, Ctx>,
        Option<api::PaymentMethodData>,
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
        requeue: bool,
        schedule_time: Option<time::PrimitiveDateTime>,
    ) -> CustomResult<(), errors::ApiErrorResponse> {
        helpers::add_domain_task_to_pt(self, state, payment_attempt, requeue, schedule_time).await
    }

    async fn get_connector<'a>(
        &'a self,
        _merchant_account: &domain::MerchantAccount,
        state: &AppState,
        request: &api::PaymentsRequest,
        _payment_intent: &storage::PaymentIntent,
        _key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<api::ConnectorChoice, errors::ApiErrorResponse> {
        // Use a new connector in the confirm call or use the same one which was passed when
        // creating the payment or if none is passed then use the routing algorithm
        helpers::get_connector_default(state, request.routing.clone()).await
    }
}

#[async_trait]
impl<F: Clone, Ctx: PaymentMethodRetrieve>
    UpdateTracker<F, PaymentData<F>, api::PaymentsRequest, Ctx> for PaymentConfirm
{
    #[instrument(skip_all)]
    async fn update_trackers<'b>(
        &'b self,
        db: &dyn StorageInterface,
        mut payment_data: PaymentData<F>,
        customer: Option<domain::Customer>,
        storage_scheme: storage_enums::MerchantStorageScheme,
        updated_customer: Option<storage::CustomerUpdate>,
        key_store: &domain::MerchantKeyStore,
        frm_suggestion: Option<FrmSuggestion>,
        header_payload: api::HeaderPayload,
    ) -> RouterResult<(
        BoxedOperation<'b, F, api::PaymentsRequest, Ctx>,
        PaymentData<F>,
    )>
    where
        F: 'b + Send,
    {
        let payment_method = payment_data.payment_attempt.payment_method;
        let browser_info = payment_data.payment_attempt.browser_info.clone();
        let frm_message = payment_data.frm_message.clone();

        let (intent_status, attempt_status, (error_code, error_message)) = match frm_suggestion {
            Some(FrmSuggestion::FrmCancelTransaction) => (
                storage_enums::IntentStatus::Failed,
                storage_enums::AttemptStatus::Failure,
                frm_message.map_or((None, None), |fraud_check| {
                    (
                        Some(Some(fraud_check.frm_status.to_string())),
                        Some(fraud_check.frm_reason.map(|reason| reason.to_string())),
                    )
                }),
            ),
            Some(FrmSuggestion::FrmManualReview) => (
                storage_enums::IntentStatus::RequiresMerchantAction,
                storage_enums::AttemptStatus::Unresolved,
                (None, None),
            ),
            _ => (
                storage_enums::IntentStatus::Processing,
                storage_enums::AttemptStatus::Pending,
                (None, None),
            ),
        };

        let connector = payment_data.payment_attempt.connector.clone();
        let merchant_connector_id = payment_data.payment_attempt.merchant_connector_id.clone();

        let straight_through_algorithm = payment_data
            .payment_attempt
            .straight_through_algorithm
            .clone();
        let payment_token = payment_data.token.clone();
        let payment_method_type = payment_data.payment_attempt.payment_method_type;
        let payment_experience = payment_data.payment_attempt.payment_experience;
        let additional_pm_data = payment_data
            .payment_method_data
            .as_ref()
            .async_map(|payment_method_data| async {
                helpers::get_additional_payment_data(payment_method_data, db).await
            })
            .await
            .as_ref()
            .map(Encode::<api_models::payments::AdditionalPaymentData>::encode_to_value)
            .transpose()
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Failed to encode additional pm data")?;

        let business_sub_label = payment_data.payment_attempt.business_sub_label.clone();
        let authentication_type = payment_data.payment_attempt.authentication_type;

        let (shipping_address, billing_address) = (
            payment_data.payment_intent.shipping_address_id.clone(),
            payment_data.payment_intent.billing_address_id.clone(),
        );

        let customer_id = customer.clone().map(|c| c.customer_id);
        let return_url = payment_data.payment_intent.return_url.take();
        let setup_future_usage = payment_data.payment_intent.setup_future_usage;
        let business_label = payment_data.payment_intent.business_label.clone();
        let business_country = payment_data.payment_intent.business_country;
        let description = payment_data.payment_intent.description.take();
        let statement_descriptor_name =
            payment_data.payment_intent.statement_descriptor_name.take();
        let statement_descriptor_suffix = payment_data
            .payment_intent
            .statement_descriptor_suffix
            .take();
        let order_details = payment_data.payment_intent.order_details.clone();
        let metadata = payment_data.payment_intent.metadata.clone();
        let surcharge_amount = payment_data
            .surcharge_details
            .as_ref()
            .map(|surcharge_details| surcharge_details.surcharge_amount);
        let tax_amount = payment_data
            .surcharge_details
            .as_ref()
            .map(|surcharge_details| surcharge_details.tax_on_surcharge_amount);
        let authorized_amount = payment_data
            .surcharge_details
            .as_ref()
            .map(|surcharge_details| surcharge_details.final_amount)
            .unwrap_or(payment_data.payment_attempt.amount);
        let payment_attempt_fut = db
            .update_payment_attempt_with_attempt_id(
                payment_data.payment_attempt,
                storage::PaymentAttemptUpdate::ConfirmUpdate {
                    amount: payment_data.amount.into(),
                    currency: payment_data.currency,
                    status: attempt_status,
                    payment_method,
                    authentication_type,
                    browser_info,
                    connector,
                    payment_token,
                    payment_method_data: additional_pm_data,
                    payment_method_type,
                    payment_experience,
                    business_sub_label,
                    straight_through_algorithm,
                    error_code,
                    error_message,
                    amount_capturable: Some(authorized_amount),
                    surcharge_amount,
                    tax_amount,
                    updated_by: storage_scheme.to_string(),
                    merchant_connector_id,
                },
                storage_scheme,
            )
            .map(|x| x.to_not_found_response(errors::ApiErrorResponse::PaymentNotFound));

        let payment_intent_fut = db
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
                    business_country,
                    business_label,
                    description,
                    statement_descriptor_name,
                    statement_descriptor_suffix,
                    order_details,
                    metadata,
                    payment_confirm_source: header_payload.payment_confirm_source,
                    updated_by: storage_scheme.to_string(),
                },
                storage_scheme,
            )
            .map(|x| x.to_not_found_response(errors::ApiErrorResponse::PaymentNotFound));

        let customer_fut = Box::pin(async {
            if let Some((updated_customer, customer)) = updated_customer.zip(customer) {
                db.update_customer_by_customer_id_merchant_id(
                    customer.customer_id.to_owned(),
                    customer.merchant_id.to_owned(),
                    updated_customer,
                    key_store,
                )
                .await
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Failed to update CustomerConnector in customer")?;
            };
            Ok::<_, error_stack::Report<errors::ApiErrorResponse>>(())
        });

        let (payment_intent, payment_attempt, _) =
            futures::try_join!(payment_intent_fut, payment_attempt_fut, customer_fut)?;
        payment_data.payment_intent = payment_intent;
        payment_data.payment_attempt = payment_attempt;

        Ok((Box::new(self), payment_data))
    }
}

impl<F: Send + Clone, Ctx: PaymentMethodRetrieve> ValidateRequest<F, api::PaymentsRequest, Ctx>
    for PaymentConfirm
{
    #[instrument(skip_all)]
    fn validate_request<'a, 'b>(
        &'b self,
        request: &api::PaymentsRequest,
        merchant_account: &'a domain::MerchantAccount,
    ) -> RouterResult<(
        BoxedOperation<'b, F, api::PaymentsRequest, Ctx>,
        operations::ValidateResult<'a>,
    )> {
        helpers::validate_customer_details_in_request(request)?;
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

        let mandate_type =
            helpers::validate_mandate(request, payments::is_operation_confirm(self))?;
        let payment_id =
            crate::core::utils::get_or_generate_id("payment_id", &given_payment_id, "pay")?;

        Ok((
            Box::new(self),
            operations::ValidateResult {
                merchant_id: &merchant_account.merchant_id,
                payment_id: api::PaymentIdType::PaymentIntentId(payment_id),
                mandate_type,
                storage_scheme: merchant_account.storage_scheme,
                requeue: matches!(
                    request.retry_action,
                    Some(api_models::enums::RetryAction::Requeue)
                ),
            },
        ))
    }
}
