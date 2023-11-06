use std::marker::PhantomData;

use api_models::enums::FrmSuggestion;
use async_trait::async_trait;
use common_utils::ext_traits::{AsyncExt, Encode, ValueExt};
use data_models::{mandates::MandateData, payments::payment_attempt::PaymentAttempt};
use diesel_models::ephemeral_key;
use error_stack::{self, ResultExt};
use router_derive::PaymentOperation;
use router_env::{instrument, tracing};

use super::{BoxedOperation, Domain, GetTracker, Operation, UpdateTracker, ValidateRequest};
use crate::{
    consts,
    core::{
        errors::{self, CustomResult, RouterResult, StorageErrorExt},
        payment_methods::PaymentMethodRetrieve,
        payments::{self, helpers, operations, CustomerDetails, PaymentAddress, PaymentData},
        utils::{self as core_utils},
    },
    db::StorageInterface,
    routes::AppState,
    services,
    types::{
        self,
        api::{self, PaymentIdTypeExt},
        domain,
        storage::{
            self,
            enums::{self, IntentStatus},
        },
    },
    utils::{self, OptionExt},
};

#[derive(Debug, Clone, Copy, PaymentOperation)]
#[operation(ops = "all", flow = "authorize")]
pub struct PaymentCreate;

/// The `get_trackers` function for `PaymentsCreate` is an entrypoint for new payments
/// This will create all the entities required for a new payment from the request
#[async_trait]
impl<F: Send + Clone, Ctx: PaymentMethodRetrieve>
    GetTracker<F, PaymentData<F>, api::PaymentsRequest, Ctx> for PaymentCreate
{
    #[instrument(skip_all)]
    async fn get_trackers<'a>(
        &'a self,
        state: &'a AppState,
        payment_id: &api::PaymentIdType,
        request: &api::PaymentsRequest,
        mandate_type: Option<api::MandateTransactionType>,
        merchant_account: &domain::MerchantAccount,
        merchant_key_store: &domain::MerchantKeyStore,
        _auth_flow: services::AuthFlow,
    ) -> RouterResult<(
        BoxedOperation<'a, F, api::PaymentsRequest, Ctx>,
        PaymentData<F>,
        Option<CustomerDetails>,
    )> {
        let db = &*state.store;
        let ephemeral_key = Self::get_ephemeral_key(request, state, merchant_account).await;
        let merchant_id = &merchant_account.merchant_id;
        let storage_scheme = merchant_account.storage_scheme;
        let (payment_intent, payment_attempt, connector_response);

        let money @ (amount, currency) = payments_create_request_validation(request)?;

        let payment_id = payment_id
            .get_payment_intent_id()
            .change_context(errors::ApiErrorResponse::PaymentNotFound)?;

        let payment_link_data = if let Some(payment_link_object) = &request.payment_link_object {
            create_payment_link(
                request,
                payment_link_object.clone(),
                merchant_id.clone(),
                payment_id.clone(),
                db,
                state,
                amount,
            )
            .await?
        } else {
            None
        };

        helpers::validate_business_details(
            request.business_country,
            request.business_label.as_ref(),
            merchant_account,
        )?;

        // Validate whether profile_id passed in request is valid and is linked to the merchant
        core_utils::validate_and_get_business_profile(db, request.profile_id.as_ref(), merchant_id)
            .await?;

        let (
            token,
            payment_method,
            payment_method_type,
            setup_mandate,
            recurring_mandate_payment_data,
            mandate_connector,
        ) = helpers::get_token_pm_type_mandate_details(
            state,
            request,
            mandate_type,
            merchant_account,
        )
        .await?;

        let customer_details = helpers::get_customer_details_from_request(request);

        let shipping_address = helpers::create_or_find_address_for_payment_by_request(
            db,
            request.shipping.as_ref(),
            None,
            merchant_id,
            customer_details.customer_id.as_ref(),
            merchant_key_store,
            &payment_id,
            merchant_account.storage_scheme,
        )
        .await?;

        let billing_address = helpers::create_or_find_address_for_payment_by_request(
            db,
            request.billing.as_ref(),
            None,
            merchant_id,
            customer_details.customer_id.as_ref(),
            merchant_key_store,
            &payment_id,
            merchant_account.storage_scheme,
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

        let attempt_id = if core_utils::is_merchant_enabled_for_payment_id_as_connector_request_id(
            &state.conf,
            merchant_id,
        ) {
            payment_id.to_string()
        } else {
            utils::get_payment_attempt_id(payment_id.clone(), 1)
        };

        let payment_intent_new = Self::make_payment_intent(
            &payment_id,
            merchant_account,
            money,
            request,
            shipping_address.clone().map(|x| x.address_id),
            payment_link_data.clone(),
            billing_address.clone().map(|x| x.address_id),
            attempt_id,
            state,
        )
        .await?;

        let payment_attempt_new = Self::make_payment_attempt(
            &payment_id,
            merchant_id,
            money,
            payment_method,
            payment_method_type,
            request,
            browser_info,
            state,
        )
        .await?;

        payment_intent = db
            .insert_payment_intent(payment_intent_new, storage_scheme)
            .await
            .to_duplicate_response(errors::ApiErrorResponse::DuplicatePayment {
                payment_id: payment_id.clone(),
            })?;

        payment_attempt = db
            .insert_payment_attempt(payment_attempt_new, storage_scheme)
            .await
            .to_duplicate_response(errors::ApiErrorResponse::DuplicatePayment {
                payment_id: payment_id.clone(),
            })?;

        connector_response = db
            .insert_connector_response(
                Self::make_connector_response(&payment_attempt),
                storage_scheme,
            )
            .await
            .to_duplicate_response(errors::ApiErrorResponse::DuplicatePayment {
                payment_id: payment_id.clone(),
            })?;

        let mandate_id = request
            .mandate_id
            .as_ref()
            .async_and_then(|mandate_id| async {
                let mandate = db
                    .find_mandate_by_merchant_id_mandate_id(merchant_id, mandate_id)
                    .await
                    .to_not_found_response(errors::ApiErrorResponse::MandateNotFound);
                Some(mandate.and_then(|mandate_obj| {
                    match (
                        mandate_obj.network_transaction_id,
                        mandate_obj.connector_mandate_ids,
                    ) {
                        (Some(network_tx_id), _) => Ok(api_models::payments::MandateIds {
                            mandate_id: mandate_obj.mandate_id,
                            mandate_reference_id: Some(
                                api_models::payments::MandateReferenceId::NetworkMandateId(
                                    network_tx_id,
                                ),
                            ),
                        }),
                        (_, Some(connector_mandate_id)) => connector_mandate_id
                        .parse_value("ConnectorMandateId")
                        .change_context(errors::ApiErrorResponse::MandateNotFound)
                        .map(|connector_id: api_models::payments::ConnectorMandateReferenceId| {
                            api_models::payments::MandateIds {
                                mandate_id: mandate_obj.mandate_id,
                                mandate_reference_id: Some(api_models::payments::MandateReferenceId::ConnectorMandateId(
                                    api_models::payments::ConnectorMandateReferenceId {
                                        connector_mandate_id: connector_id.connector_mandate_id,
                                        payment_method_id: connector_id.payment_method_id,
                                    },
                                ))
                            }
                         }),
                        (_, _) => Ok(api_models::payments::MandateIds {
                            mandate_id: mandate_obj.mandate_id,
                            mandate_reference_id: None,
                        }),
                    }
                }))
            })
            .await
            .transpose()?;

        let operation = payments::if_not_create_change_operation::<_, F, Ctx>(
            payment_intent.status,
            request.confirm,
            self,
        );
        let creds_identifier = request
            .merchant_connector_details
            .as_ref()
            .map(|mcd| mcd.creds_identifier.to_owned());
        request
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
            .await
            .transpose()?;

        // The operation merges mandate data from both request and payment_attempt
        let setup_mandate: Option<MandateData> = setup_mandate.map(Into::into);

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
                mandate_connector,
                setup_mandate,
                token,
                address: PaymentAddress {
                    shipping: shipping_address.as_ref().map(|a| a.into()),
                    billing: billing_address.as_ref().map(|a| a.into()),
                },
                confirm: request.confirm,
                payment_method_data: request.payment_method_data.clone(),
                refunds: vec![],
                disputes: vec![],
                attempts: None,
                force_sync: None,
                connector_response,
                sessions_token: vec![],
                card_cvc: request.card_cvc.clone(),
                creds_identifier,
                pm_token: None,
                connector_customer_id: None,
                recurring_mandate_payment_data,
                ephemeral_key,
                multiple_capture_data: None,
                redirect_response: None,
                surcharge_details: None,
                frm_message: None,
                payment_link_data,
            },
            Some(customer_details),
        ))
    }
}

#[async_trait]
impl<F: Clone + Send, Ctx: PaymentMethodRetrieve> Domain<F, api::PaymentsRequest, Ctx>
    for PaymentCreate
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
        _storage_scheme: enums::MerchantStorageScheme,
    ) -> RouterResult<(
        BoxedOperation<'a, F, api::PaymentsRequest, Ctx>,
        Option<api::PaymentMethodData>,
    )> {
        helpers::make_pm_data(Box::new(self), state, payment_data).await
    }

    #[instrument(skip_all)]
    async fn add_task_to_process_tracker<'a>(
        &'a self,
        _state: &'a AppState,
        _payment_attempt: &PaymentAttempt,
        _requeue: bool,
        _schedule_time: Option<time::PrimitiveDateTime>,
    ) -> CustomResult<(), errors::ApiErrorResponse> {
        Ok(())
    }

    async fn get_connector<'a>(
        &'a self,
        _merchant_account: &domain::MerchantAccount,
        state: &AppState,
        request: &api::PaymentsRequest,
        _payment_intent: &storage::PaymentIntent,
        _merchant_key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<api::ConnectorChoice, errors::ApiErrorResponse> {
        helpers::get_connector_default(state, request.routing.clone()).await
    }
}

#[async_trait]
impl<F: Clone, Ctx: PaymentMethodRetrieve>
    UpdateTracker<F, PaymentData<F>, api::PaymentsRequest, Ctx> for PaymentCreate
{
    #[instrument(skip_all)]
    async fn update_trackers<'b>(
        &'b self,
        db: &dyn StorageInterface,
        mut payment_data: PaymentData<F>,
        _customer: Option<domain::Customer>,
        storage_scheme: enums::MerchantStorageScheme,
        _updated_customer: Option<storage::CustomerUpdate>,
        _merchant_key_store: &domain::MerchantKeyStore,
        _frm_suggestion: Option<FrmSuggestion>,
        _header_payload: api::HeaderPayload,
    ) -> RouterResult<(
        BoxedOperation<'b, F, api::PaymentsRequest, Ctx>,
        PaymentData<F>,
    )>
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
                    //TODO: do this later, request validation should happen before
                    Some(IntentStatus::Processing)
                } else {
                    None
                }
            }
            _ => None,
        };

        let payment_token = payment_data.token.clone();
        let connector = payment_data.payment_attempt.connector.clone();
        let straight_through_algorithm = payment_data
            .payment_attempt
            .straight_through_algorithm
            .clone();
        let authorized_amount = payment_data.payment_attempt.amount;
        let merchant_connector_id = payment_data.payment_attempt.merchant_connector_id.clone();

        payment_data.payment_attempt = db
            .update_payment_attempt_with_attempt_id(
                payment_data.payment_attempt,
                storage::PaymentAttemptUpdate::UpdateTrackers {
                    payment_token,
                    connector,
                    straight_through_algorithm,
                    amount_capturable: match payment_data.confirm.unwrap_or(true) {
                        true => Some(authorized_amount),
                        false => None,
                    },
                    updated_by: storage_scheme.to_string(),
                    merchant_connector_id,
                },
                storage_scheme,
            )
            .await
            .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)?;

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
                    updated_by: storage_scheme.to_string(),
                },
                storage_scheme,
            )
            .await
            .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)?;

        // payment_data.mandate_id = response.and_then(|router_data| router_data.request.mandate_id);
        Ok((
            payments::is_confirm(self, payment_data.confirm),
            payment_data,
        ))
    }
}

impl<F: Send + Clone, Ctx: PaymentMethodRetrieve> ValidateRequest<F, api::PaymentsRequest, Ctx>
    for PaymentCreate
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

        if let Some(payment_link_object) = &request.payment_link_object {
            helpers::validate_payment_link_request(
                payment_link_object,
                request.confirm,
                request.order_details.clone(),
            )?;
        }

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

        helpers::validate_card_data(request.payment_method_data.clone())?;

        helpers::validate_payment_method_fields_present(request)?;

        let payment_id = core_utils::get_or_generate_id("payment_id", &given_payment_id, "pay")?;

        let mandate_type =
            helpers::validate_mandate(request, payments::is_operation_confirm(self))?;

        if request.confirm.unwrap_or(false) {
            helpers::validate_pm_or_token_given(
                &request.payment_method,
                &request.payment_method_data,
                &request.payment_method_type,
                &mandate_type,
                &request.payment_token,
            )?;

            helpers::validate_customer_id_mandatory_cases(
                request.setup_future_usage.is_some(),
                &request
                    .customer
                    .clone()
                    .map(|customer| customer.id)
                    .or(request.customer_id.clone()),
            )?;
        }

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

impl PaymentCreate {
    #[instrument(skip_all)]
    #[allow(clippy::too_many_arguments)]
    pub async fn make_payment_attempt(
        payment_id: &str,
        merchant_id: &str,
        money: (api::Amount, enums::Currency),
        payment_method: Option<enums::PaymentMethod>,
        payment_method_type: Option<enums::PaymentMethodType>,
        request: &api::PaymentsRequest,
        browser_info: Option<serde_json::Value>,
        state: &AppState,
    ) -> RouterResult<storage::PaymentAttemptNew> {
        let created_at @ modified_at @ last_synced = Some(common_utils::date_time::now());
        let status =
            helpers::payment_attempt_status_fsm(&request.payment_method_data, request.confirm);
        let (amount, currency) = (money.0, Some(money.1));

        let additional_pm_data = request
            .payment_method_data
            .as_ref()
            .async_map(|payment_method_data| async {
                helpers::get_additional_payment_data(payment_method_data, &*state.store).await
            })
            .await
            .as_ref()
            .map(Encode::<api_models::payments::AdditionalPaymentData>::encode_to_value)
            .transpose()
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Failed to encode additional pm data")?;
        let attempt_id = if core_utils::is_merchant_enabled_for_payment_id_as_connector_request_id(
            &state.conf,
            merchant_id,
        ) {
            payment_id.to_string()
        } else {
            utils::get_payment_attempt_id(payment_id, 1)
        };

        Ok(storage::PaymentAttemptNew {
            payment_id: payment_id.to_string(),
            merchant_id: merchant_id.to_string(),
            attempt_id,
            status,
            currency,
            amount: amount.into(),
            payment_method,
            capture_method: request.capture_method,
            capture_on: request.capture_on,
            confirm: request.confirm.unwrap_or(false),
            created_at,
            modified_at,
            last_synced,
            authentication_type: request.authentication_type,
            browser_info,
            payment_experience: request.payment_experience,
            payment_method_type,
            payment_method_data: additional_pm_data,
            amount_to_capture: request.amount_to_capture,
            payment_token: request.payment_token.clone(),
            mandate_id: request.mandate_id.clone(),
            business_sub_label: request.business_sub_label.clone(),
            mandate_details: request
                .mandate_data
                .as_ref()
                .and_then(|inner| inner.mandate_type.clone().map(Into::into)),
            ..storage::PaymentAttemptNew::default()
        })
    }

    #[instrument(skip_all)]
    #[allow(clippy::too_many_arguments)]
    async fn make_payment_intent(
        payment_id: &str,
        merchant_account: &types::domain::MerchantAccount,
        money: (api::Amount, enums::Currency),
        request: &api::PaymentsRequest,
        shipping_address_id: Option<String>,
        payment_link_data: Option<api_models::payments::PaymentLinkResponse>,
        billing_address_id: Option<String>,
        active_attempt_id: String,
        state: &AppState,
    ) -> RouterResult<storage::PaymentIntentNew> {
        let created_at @ modified_at @ last_synced = Some(common_utils::date_time::now());
        let status =
            helpers::payment_intent_status_fsm(&request.payment_method_data, request.confirm);
        let client_secret =
            crate::utils::generate_id(consts::ID_LENGTH, format!("{payment_id}_secret").as_str());
        let (amount, currency) = (money.0, Some(money.1));

        let order_details = request
            .get_order_details_as_value()
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Failed to convert order details to value")?;

        // If profile id is not passed, get it from the business_country and business_label
        let profile_id = core_utils::get_profile_id_from_business_details(
            request.business_country,
            request.business_label.as_ref(),
            merchant_account,
            request.profile_id.as_ref(),
            &*state.store,
            true,
        )
        .await?;

        let allowed_payment_method_types = request
            .get_allowed_payment_method_types_as_value()
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Error converting allowed_payment_types to Value")?;

        let connector_metadata = request
            .get_connector_metadata_as_value()
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Error converting connector_metadata to Value")?;

        let feature_metadata = request
            .get_feature_metadata_as_value()
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Error converting feature_metadata to Value")?;

        let payment_link_id = payment_link_data.map(|pl_data| pl_data.payment_link_id);

        Ok(storage::PaymentIntentNew {
            payment_id: payment_id.to_string(),
            merchant_id: merchant_account.merchant_id.to_string(),
            status,
            amount: amount.into(),
            currency,
            description: request.description.clone(),
            created_at,
            modified_at,
            last_synced,
            client_secret: Some(client_secret),
            setup_future_usage: request.setup_future_usage,
            off_session: request.off_session,
            return_url: request.return_url.as_ref().map(|a| a.to_string()),
            shipping_address_id,
            billing_address_id,
            statement_descriptor_name: request.statement_descriptor_name.clone(),
            statement_descriptor_suffix: request.statement_descriptor_suffix.clone(),
            metadata: request.metadata.clone(),
            business_country: request.business_country,
            business_label: request.business_label.clone(),
            active_attempt: data_models::RemoteStorageObject::ForeignID(active_attempt_id),
            order_details,
            amount_captured: None,
            customer_id: None,
            connector_id: None,
            allowed_payment_method_types,
            connector_metadata,
            feature_metadata,
            attempt_count: 1,
            profile_id: Some(profile_id),
            merchant_decision: None,
            payment_link_id,
            payment_confirm_source: None,
            surcharge_applicable: None,
            updated_by: merchant_account.storage_scheme.to_string(),
        })
    }

    #[instrument(skip_all)]
    pub fn make_connector_response(
        payment_attempt: &PaymentAttempt,
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
            updated_by: payment_attempt.updated_by.clone(),
        }
    }

    #[instrument(skip_all)]
    pub async fn get_ephemeral_key(
        request: &api::PaymentsRequest,
        state: &AppState,
        merchant_account: &domain::MerchantAccount,
    ) -> Option<ephemeral_key::EphemeralKey> {
        match request.customer_id.clone() {
            Some(customer_id) => helpers::make_ephemeral_key(
                state.clone(),
                customer_id,
                merchant_account.merchant_id.clone(),
            )
            .await
            .ok()
            .and_then(|ek| {
                if let services::ApplicationResponse::Json(ek) = ek {
                    Some(ek)
                } else {
                    None
                }
            }),
            None => None,
        }
    }
}

#[instrument(skip_all)]
pub fn payments_create_request_validation(
    req: &api::PaymentsRequest,
) -> RouterResult<(api::Amount, enums::Currency)> {
    let currency = req.currency.get_required_value("currency")?;
    let amount = req.amount.get_required_value("amount")?;
    Ok((amount, currency))
}

async fn create_payment_link(
    request: &api::PaymentsRequest,
    payment_link_object: api_models::payments::PaymentLinkObject,
    merchant_id: String,
    payment_id: String,
    db: &dyn StorageInterface,
    state: &AppState,
    amount: api::Amount,
) -> RouterResult<Option<api_models::payments::PaymentLinkResponse>> {
    let created_at @ last_modified_at = Some(common_utils::date_time::now());
    let domain = if let Some(domain_name) = payment_link_object.merchant_custom_domain_name {
        format!("https://{domain_name}")
    } else {
        state.conf.server.base_url.clone()
    };

    let payment_link_id = utils::generate_id(consts::ID_LENGTH, "plink");
    let payment_link = format!(
        "{}/payment_link/{}/{}",
        domain,
        merchant_id.clone(),
        payment_id.clone()
    );
    let payment_link_req = storage::PaymentLinkNew {
        payment_link_id: payment_link_id.clone(),
        payment_id: payment_id.clone(),
        merchant_id: merchant_id.clone(),
        link_to_pay: payment_link.clone(),
        amount: amount.into(),
        currency: request.currency,
        created_at,
        last_modified_at,
        fulfilment_time: payment_link_object.link_expiry,
    };
    let payment_link_db = db
        .insert_payment_link(payment_link_req)
        .await
        .to_duplicate_response(errors::ApiErrorResponse::GenericDuplicateError {
            message: "payment link already exists!".to_string(),
        })?;

    Ok(Some(api_models::payments::PaymentLinkResponse {
        link: payment_link_db.link_to_pay,
        payment_link_id: payment_link_db.payment_link_id,
    }))
}
