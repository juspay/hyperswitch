use api_models::{enums::FrmSuggestion, payments::PaymentsConfirmIntentRequest};
use async_trait::async_trait;
use common_utils::{ext_traits::Encode, fp_utils::when, id_type, types::keymanager::ToEncryptable};
use error_stack::ResultExt;
use hyperswitch_domain_models::payments::PaymentConfirmData;
use hyperswitch_interfaces::api::ConnectorSpecifications;
use masking::{ExposeOptionInterface, PeekInterface};
use router_env::{instrument, tracing};

use super::{Domain, GetTracker, Operation, UpdateTracker, ValidateRequest};
use crate::{
    core::{
        admin,
        errors::{self, CustomResult, RouterResult, StorageErrorExt},
        payment_methods,
        payments::{
            self, call_decision_manager, helpers,
            operations::{self, ValidateStatusForOperation},
            populate_surcharge_details, CustomerDetails, OperationSessionSetters, PaymentAddress,
            PaymentData,
        },
        utils as core_utils,
    },
    routes::{app::ReqState, SessionState},
    services::{self, connector_integration_interface::ConnectorEnum},
    types::{
        self,
        api::{self, ConnectorCallType, PaymentIdTypeExt},
        domain::{self, types as domain_types},
        storage::{self, enums as storage_enums},
    },
    utils::{self, OptionExt},
};

#[derive(Debug, Clone, Copy)]
pub struct PaymentIntentConfirm;

impl ValidateStatusForOperation for PaymentIntentConfirm {
    /// Validate if the current operation can be performed on the current status of the payment intent
    fn validate_status_for_operation(
        &self,
        intent_status: common_enums::IntentStatus,
    ) -> Result<(), errors::ApiErrorResponse> {
        match intent_status {
            common_enums::IntentStatus::RequiresPaymentMethod => Ok(()),
            common_enums::IntentStatus::Succeeded
            | common_enums::IntentStatus::Conflicted
            | common_enums::IntentStatus::Failed
            | common_enums::IntentStatus::Cancelled
            | common_enums::IntentStatus::CancelledPostCapture
            | common_enums::IntentStatus::Processing
            | common_enums::IntentStatus::RequiresCustomerAction
            | common_enums::IntentStatus::RequiresMerchantAction
            | common_enums::IntentStatus::RequiresCapture
            | common_enums::IntentStatus::PartiallyAuthorizedAndRequiresCapture
            | common_enums::IntentStatus::PartiallyCaptured
            | common_enums::IntentStatus::RequiresConfirmation
            | common_enums::IntentStatus::PartiallyCapturedAndCapturable
            | common_enums::IntentStatus::Expired => {
                Err(errors::ApiErrorResponse::PaymentUnexpectedState {
                    current_flow: format!("{self:?}"),
                    field_name: "status".to_string(),
                    current_value: intent_status.to_string(),
                    states: ["requires_payment_method".to_string()].join(", "),
                })
            }
        }
    }
}

type BoxedConfirmOperation<'b, F> =
    super::BoxedOperation<'b, F, PaymentsConfirmIntentRequest, PaymentConfirmData<F>>;

// TODO: change the macro to include changes for v2
// TODO: PaymentData in the macro should be an input
impl<F: Send + Clone + Sync> Operation<F, PaymentsConfirmIntentRequest> for &PaymentIntentConfirm {
    type Data = PaymentConfirmData<F>;
    fn to_validate_request(
        &self,
    ) -> RouterResult<
        &(dyn ValidateRequest<F, PaymentsConfirmIntentRequest, Self::Data> + Send + Sync),
    > {
        Ok(*self)
    }
    fn to_get_tracker(
        &self,
    ) -> RouterResult<&(dyn GetTracker<F, Self::Data, PaymentsConfirmIntentRequest> + Send + Sync)>
    {
        Ok(*self)
    }
    fn to_domain(
        &self,
    ) -> RouterResult<&(dyn Domain<F, PaymentsConfirmIntentRequest, Self::Data>)> {
        Ok(*self)
    }
    fn to_update_tracker(
        &self,
    ) -> RouterResult<&(dyn UpdateTracker<F, Self::Data, PaymentsConfirmIntentRequest> + Send + Sync)>
    {
        Ok(*self)
    }
}
#[automatically_derived]
impl<F: Send + Clone + Sync> Operation<F, PaymentsConfirmIntentRequest> for PaymentIntentConfirm {
    type Data = PaymentConfirmData<F>;
    fn to_validate_request(
        &self,
    ) -> RouterResult<
        &(dyn ValidateRequest<F, PaymentsConfirmIntentRequest, Self::Data> + Send + Sync),
    > {
        Ok(self)
    }
    fn to_get_tracker(
        &self,
    ) -> RouterResult<&(dyn GetTracker<F, Self::Data, PaymentsConfirmIntentRequest> + Send + Sync)>
    {
        Ok(self)
    }
    fn to_domain(&self) -> RouterResult<&dyn Domain<F, PaymentsConfirmIntentRequest, Self::Data>> {
        Ok(self)
    }
    fn to_update_tracker(
        &self,
    ) -> RouterResult<&(dyn UpdateTracker<F, Self::Data, PaymentsConfirmIntentRequest> + Send + Sync)>
    {
        Ok(self)
    }
}

impl<F: Send + Clone + Sync> ValidateRequest<F, PaymentsConfirmIntentRequest, PaymentConfirmData<F>>
    for PaymentIntentConfirm
{
    #[instrument(skip_all)]
    fn validate_request<'a, 'b>(
        &'b self,
        request: &PaymentsConfirmIntentRequest,
        platform: &'a domain::Platform,
    ) -> RouterResult<operations::ValidateResult> {
        let validate_result = operations::ValidateResult {
            merchant_id: platform.get_processor().get_account().get_id().to_owned(),
            storage_scheme: platform.get_processor().get_account().storage_scheme,
            requeue: false,
        };

        Ok(validate_result)
    }
}

#[async_trait]
impl<F: Send + Clone + Sync> GetTracker<F, PaymentConfirmData<F>, PaymentsConfirmIntentRequest>
    for PaymentIntentConfirm
{
    #[instrument(skip_all)]
    async fn get_trackers<'a>(
        &'a self,
        state: &'a SessionState,
        payment_id: &id_type::GlobalPaymentId,
        request: &PaymentsConfirmIntentRequest,
        platform: &domain::Platform,
        profile: &domain::Profile,
        header_payload: &hyperswitch_domain_models::payments::HeaderPayload,
    ) -> RouterResult<operations::GetTrackerResponse<PaymentConfirmData<F>>> {
        let db = &*state.store;
        let key_manager_state = &state.into();

        let storage_scheme = platform.get_processor().get_account().storage_scheme;

        let payment_intent = db
            .find_payment_intent_by_id(
                payment_id,
                platform.get_processor().get_key_store(),
                storage_scheme,
            )
            .await
            .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)?;

        // TODO (#7195): Add platform merchant account validation once publishable key auth is solved

        self.validate_status_for_operation(payment_intent.status)?;

        let cell_id = state.conf.cell_information.id.clone();

        let batch_encrypted_data = domain_types::crypto_operation(
            key_manager_state,
            common_utils::type_name!(hyperswitch_domain_models::payments::payment_attempt::PaymentAttempt),
            domain_types::CryptoOperation::BatchEncrypt(
                hyperswitch_domain_models::payments::payment_attempt::FromRequestEncryptablePaymentAttempt::to_encryptable(
                    hyperswitch_domain_models::payments::payment_attempt::FromRequestEncryptablePaymentAttempt {
                        payment_method_billing_address: request.payment_method_data.billing.as_ref().map(|address| address.clone().encode_to_value()).transpose().change_context(errors::ApiErrorResponse::InternalServerError).attach_printable("Failed to encode payment_method_billing address")?.map(masking::Secret::new),
                    },
                ),
            ),
            common_utils::types::keymanager::Identifier::Merchant(platform.get_processor().get_account().get_id().to_owned()),
            platform.get_processor().get_key_store().key.peek(),
        )
        .await
        .and_then(|val| val.try_into_batchoperation())
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed while encrypting payment intent details".to_string())?;

        let encrypted_data =
             hyperswitch_domain_models::payments::payment_attempt::FromRequestEncryptablePaymentAttempt::from_encryptable(batch_encrypted_data)
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Failed while encrypting payment intent details")?;

        let payment_attempt_domain_model =
            hyperswitch_domain_models::payments::payment_attempt::PaymentAttempt::create_domain_model(
                &payment_intent,
                cell_id,
                storage_scheme,
                request,
                encrypted_data
            )
            .await?;

        let payment_attempt: hyperswitch_domain_models::payments::payment_attempt::PaymentAttempt =
            db.insert_payment_attempt(
                platform.get_processor().get_key_store(),
                payment_attempt_domain_model,
                storage_scheme,
            )
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Could not insert payment attempt")?;

        let payment_method_data = request
            .payment_method_data
            .payment_method_data
            .clone()
            .map(hyperswitch_domain_models::payment_method_data::PaymentMethodData::from);

        if request.payment_token.is_some() {
            when(
                !matches!(
                    payment_method_data,
                    Some(domain::payment_method_data::PaymentMethodData::CardToken(_))
                ),
                || {
                    Err(errors::ApiErrorResponse::InvalidDataValue {
                        field_name: "payment_method_data",
                    })
                    .attach_printable(
                        "payment_method_data should be card_token when a token is passed",
                    )
                },
            )?;
        };

        let payment_address = hyperswitch_domain_models::payment_address::PaymentAddress::new(
            payment_intent
                .shipping_address
                .clone()
                .map(|address| address.into_inner()),
            payment_intent
                .billing_address
                .clone()
                .map(|address| address.into_inner()),
            payment_attempt
                .payment_method_billing_address
                .clone()
                .map(|address| address.into_inner()),
            Some(true),
        );

        let merchant_connector_details = request.merchant_connector_details.clone();

        let payment_data = PaymentConfirmData {
            flow: std::marker::PhantomData,
            payment_intent,
            payment_attempt,
            payment_method_data,
            payment_address,
            mandate_data: None,
            payment_method: None,
            merchant_connector_details,
            external_vault_pmd: None,
            webhook_url: request
                .webhook_url
                .as_ref()
                .map(|url| url.get_string_repr().to_string()),
        };

        let get_trackers_response = operations::GetTrackerResponse { payment_data };

        Ok(get_trackers_response)
    }

    #[instrument(skip_all)]
    async fn get_trackers_for_split_payments<'a>(
        &'a self,
        state: &'a SessionState,
        payment_id: &id_type::GlobalPaymentId,
        request: &PaymentsConfirmIntentRequest,
        platform: &domain::Platform,
        profile: &domain::Profile,
        header_payload: &hyperswitch_domain_models::payments::HeaderPayload,
        split_amount_data: (
            api_models::payments::PaymentMethodData,
            common_utils::types::MinorUnit,
        ),
        attempts_group_id: &id_type::GlobalAttemptGroupId,
    ) -> RouterResult<operations::GetTrackerResponse<PaymentConfirmData<F>>> {
        let db = &*state.store;
        let key_manager_state = &state.into();

        let storage_scheme = platform.get_processor().get_account().storage_scheme;

        let payment_intent = db
            .find_payment_intent_by_id(
                payment_id,
                platform.get_processor().get_key_store(),
                storage_scheme,
            )
            .await
            .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)?;

        // TODO (#7195): Add platform merchant account validation once publishable key auth is solved

        self.validate_status_for_operation(payment_intent.status)?;

        let cell_id = state.conf.cell_information.id.clone();

        let batch_encrypted_data = domain_types::crypto_operation(
            key_manager_state,
            common_utils::type_name!(hyperswitch_domain_models::payments::payment_attempt::PaymentAttempt),
            domain_types::CryptoOperation::BatchEncrypt(
                hyperswitch_domain_models::payments::payment_attempt::FromRequestEncryptablePaymentAttempt::to_encryptable(
                    hyperswitch_domain_models::payments::payment_attempt::FromRequestEncryptablePaymentAttempt {
                        payment_method_billing_address: request.payment_method_data.billing.as_ref().map(|address| address.clone().encode_to_value()).transpose().change_context(errors::ApiErrorResponse::InternalServerError).attach_printable("Failed to encode payment_method_billing address")?.map(masking::Secret::new),
                    },
                ),
            ),
            common_utils::types::keymanager::Identifier::Merchant(platform.get_processor().get_account().get_id().to_owned()),
            platform.get_processor().get_key_store().key.peek(),
        )
        .await
        .and_then(|val| val.try_into_batchoperation())
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed while encrypting payment intent details".to_string())?;

        let encrypted_data =
             hyperswitch_domain_models::payments::payment_attempt::FromRequestEncryptablePaymentAttempt::from_encryptable(batch_encrypted_data)
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Failed while encrypting payment intent details")?;

        let payment_attempt_domain_model =
            crate::core::split_payments::create_domain_model_for_split_payment(
                &payment_intent,
                cell_id,
                storage_scheme,
                request,
                encrypted_data,
                split_amount_data.1,
                attempts_group_id,
            )
            .await?;

        let payment_attempt: hyperswitch_domain_models::payments::payment_attempt::PaymentAttempt =
            db.insert_payment_attempt(
                platform.get_processor().get_key_store(),
                payment_attempt_domain_model,
                storage_scheme,
            )
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Could not insert payment attempt")?;

        let payment_method_data =
            hyperswitch_domain_models::payment_method_data::PaymentMethodData::from(
                split_amount_data.0,
            );

        let payment_address = hyperswitch_domain_models::payment_address::PaymentAddress::new(
            payment_intent
                .shipping_address
                .clone()
                .map(|address| address.into_inner()),
            payment_intent
                .billing_address
                .clone()
                .map(|address| address.into_inner()),
            payment_attempt
                .payment_method_billing_address
                .clone()
                .map(|address| address.into_inner()),
            Some(true),
        );

        let merchant_connector_details = request.merchant_connector_details.clone();

        let payment_data = PaymentConfirmData {
            flow: std::marker::PhantomData,
            payment_intent,
            payment_attempt,
            payment_method_data: Some(payment_method_data),
            payment_address,
            mandate_data: None,
            payment_method: None,
            merchant_connector_details,
            external_vault_pmd: None,
            webhook_url: request
                .webhook_url
                .as_ref()
                .map(|url| url.get_string_repr().to_string()),
        };

        let get_trackers_response = operations::GetTrackerResponse { payment_data };

        Ok(get_trackers_response)
    }
}

#[async_trait]
impl<F: Clone + Send + Sync> Domain<F, PaymentsConfirmIntentRequest, PaymentConfirmData<F>>
    for PaymentIntentConfirm
{
    async fn get_customer_details<'a>(
        &'a self,
        state: &SessionState,
        payment_data: &mut PaymentConfirmData<F>,
        merchant_key_store: &domain::MerchantKeyStore,
        storage_scheme: storage_enums::MerchantStorageScheme,
    ) -> CustomResult<(BoxedConfirmOperation<'a, F>, Option<domain::Customer>), errors::StorageError>
    {
        match payment_data.payment_intent.customer_id.clone() {
            Some(id) => {
                let customer = state
                    .store
                    .find_customer_by_global_id(&id, merchant_key_store, storage_scheme)
                    .await?;

                Ok((Box::new(self), Some(customer)))
            }
            None => Ok((Box::new(self), None)),
        }
    }

    async fn run_decision_manager<'a>(
        &'a self,
        state: &SessionState,
        payment_data: &mut PaymentConfirmData<F>,
        business_profile: &domain::Profile,
    ) -> CustomResult<(), errors::ApiErrorResponse> {
        let authentication_type = payment_data.payment_intent.authentication_type;

        let authentication_type = match business_profile.three_ds_decision_manager_config.as_ref() {
            Some(three_ds_decision_manager_config) => call_decision_manager(
                state,
                three_ds_decision_manager_config.clone(),
                payment_data,
            )?,
            None => authentication_type,
        };

        if let Some(auth_type) = authentication_type {
            payment_data.payment_attempt.authentication_type = auth_type;
        }

        Ok(())
    }

    #[instrument(skip_all)]
    async fn make_pm_data<'a>(
        &'a self,
        state: &'a SessionState,
        payment_data: &mut PaymentConfirmData<F>,
        storage_scheme: storage_enums::MerchantStorageScheme,
        key_store: &domain::MerchantKeyStore,
        customer: &Option<domain::Customer>,
        business_profile: &domain::Profile,
        _should_retry_with_pan: bool,
    ) -> RouterResult<(
        BoxedConfirmOperation<'a, F>,
        Option<domain::PaymentMethodData>,
        Option<String>,
    )> {
        Ok((Box::new(self), None, None))
    }

    #[cfg(feature = "v2")]
    async fn perform_routing<'a>(
        &'a self,
        platform: &domain::Platform,
        business_profile: &domain::Profile,
        state: &SessionState,
        payment_data: &mut PaymentConfirmData<F>,
    ) -> CustomResult<ConnectorCallType, errors::ApiErrorResponse> {
        payments::connector_selection(state, platform, business_profile, payment_data, None).await
    }

    #[instrument(skip_all)]
    async fn populate_payment_data<'a>(
        &'a self,
        state: &SessionState,
        payment_data: &mut PaymentConfirmData<F>,
        _platform: &domain::Platform,
        business_profile: &domain::Profile,
        connector_data: &api::ConnectorData,
    ) -> CustomResult<(), errors::ApiErrorResponse> {
        let connector_request_reference_id = connector_data
            .connector
            .generate_connector_request_reference_id(
                &payment_data.payment_intent,
                &payment_data.payment_attempt,
            );
        payment_data.set_connector_request_reference_id(Some(connector_request_reference_id));
        Ok(())
    }

    #[cfg(feature = "v2")]
    async fn create_or_fetch_payment_method<'a>(
        &'a self,
        state: &SessionState,
        platform: &domain::Platform,
        business_profile: &domain::Profile,
        payment_data: &mut PaymentConfirmData<F>,
    ) -> CustomResult<(), errors::ApiErrorResponse> {
        let (payment_method, payment_method_data) = match (
            &payment_data.payment_attempt.payment_token,
            &payment_data.payment_method_data,
            &payment_data.payment_attempt.customer_acceptance,
        ) {
            (
                Some(payment_token),
                Some(domain::payment_method_data::PaymentMethodData::CardToken(card_token)),
                None,
            ) => {
                let (card_cvc, card_holder_name) = {
                    (
                        card_token
                            .card_cvc
                            .clone()
                            .ok_or(errors::ApiErrorResponse::InvalidDataValue {
                                field_name: "card_cvc",
                            })
                            .or(
                                payment_methods::vault::retrieve_and_delete_cvc_from_payment_token(
                                    state,
                                    payment_token,
                                    payment_data.payment_attempt.payment_method_type,
                                    platform.get_processor().get_key_store().key.get_inner(),
                                )
                                .await,
                            )
                            .attach_printable("card_cvc not provided")?,
                        card_token.card_holder_name.clone(),
                    )
                };

                let (payment_method, vault_data) =
                    payment_methods::vault::retrieve_payment_method_from_vault_using_payment_token(
                        state,
                        platform,
                        business_profile,
                        payment_token,
                        &payment_data.payment_attempt.payment_method_type,
                    )
                    .await
                    .change_context(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable("Failed to retrieve payment method from vault")?;

                match vault_data {
                    domain::vault::PaymentMethodVaultingData::Card(card_detail) => {
                        let pm_data_from_vault =
                            domain::payment_method_data::PaymentMethodData::Card(
                                domain::payment_method_data::Card::from((
                                    card_detail,
                                    card_cvc,
                                    card_holder_name,
                                )),
                            );

                        (Some(payment_method), Some(pm_data_from_vault))
                    }
                    _ => Err(errors::ApiErrorResponse::NotImplemented {
                        message: errors::NotImplementedMessage::Reason(
                            "Non-card Tokenization not implemented".to_string(),
                        ),
                    })?,
                }
            }

            (Some(_payment_token), _, _) => Err(errors::ApiErrorResponse::InvalidDataValue {
                field_name: "payment_method_data",
            })
            .attach_printable("payment_method_data should be card_token when a token is passed")?,

            (None, Some(domain::PaymentMethodData::Card(card)), Some(_customer_acceptance)) => {
                let customer_id = match &payment_data.payment_intent.customer_id {
                    Some(customer_id) => customer_id.clone(),
                    None => {
                        return Err(errors::ApiErrorResponse::InvalidDataValue {
                            field_name: "customer_id",
                        })
                        .attach_printable("customer_id not provided");
                    }
                };

                let pm_create_data =
                    api::PaymentMethodCreateData::Card(api::CardDetail::from(card.clone()));

                let req = api::PaymentMethodCreate {
                    payment_method_type: payment_data.payment_attempt.payment_method_type,
                    payment_method_subtype: payment_data.payment_attempt.payment_method_subtype,
                    metadata: None,
                    customer_id,
                    payment_method_data: pm_create_data,
                    billing: None,
                    psp_tokenization: None,
                    network_tokenization: None,
                };

                let (_pm_response, payment_method) =
                    Box::pin(payment_methods::create_payment_method_core(
                        state,
                        &state.get_req_state(),
                        req,
                        platform,
                        business_profile,
                    ))
                    .await?;

                // Don't modify payment_method_data in this case, only the payment_method and payment_method_id
                (Some(payment_method), None)
            }
            _ => (None, None), // Pass payment_data unmodified for any other case
        };

        if let Some(pm_data) = payment_method_data {
            payment_data.update_payment_method_data(pm_data);
        }
        if let Some(pm) = payment_method {
            payment_data.update_payment_method_and_pm_id(pm.get_id().clone(), pm);
        }

        Ok(())
    }

    #[cfg(feature = "v2")]
    async fn get_connector_from_request<'a>(
        &'a self,
        state: &SessionState,
        request: &PaymentsConfirmIntentRequest,
        payment_data: &mut PaymentConfirmData<F>,
    ) -> CustomResult<api::ConnectorData, errors::ApiErrorResponse> {
        let connector_data = helpers::get_connector_data_from_request(
            state,
            request.merchant_connector_details.clone(),
        )
        .await?;
        payment_data
            .set_connector_in_payment_attempt(Some(connector_data.connector_name.to_string()));
        Ok(connector_data)
    }

    async fn get_connector_tokenization_action<'a>(
        &'a self,
        state: &SessionState,
        payment_data: &PaymentConfirmData<F>,
    ) -> RouterResult<payments::TokenizationAction> {
        let connector = payment_data.payment_attempt.connector.to_owned();

        let is_connector_mandate_flow = payment_data
            .mandate_data
            .as_ref()
            .and_then(|mandate_details| mandate_details.mandate_reference_id.as_ref())
            .map(|mandate_reference| match mandate_reference {
                api_models::payments::MandateReferenceId::ConnectorMandateId(_) => true,
                api_models::payments::MandateReferenceId::NetworkMandateId(_)
                | api_models::payments::MandateReferenceId::NetworkTokenWithNTI(_) => false,
            })
            .unwrap_or(false);

        let tokenization_action = match connector {
            Some(_) if is_connector_mandate_flow => {
                payments::TokenizationAction::SkipConnectorTokenization
            }
            Some(connector) => {
                let payment_method = payment_data
                    .payment_attempt
                    .get_payment_method()
                    .ok_or_else(|| errors::ApiErrorResponse::InternalServerError)
                    .attach_printable("Payment method not found")?;
                let payment_method_type: Option<common_enums::PaymentMethodType> =
                    payment_data.payment_attempt.get_payment_method_type();

                let mandate_flow_enabled = payment_data.payment_intent.setup_future_usage;

                let is_connector_tokenization_enabled =
                    payments::is_payment_method_tokenization_enabled_for_connector(
                        state,
                        &connector,
                        payment_method,
                        payment_method_type,
                        mandate_flow_enabled,
                    )?;

                if is_connector_tokenization_enabled {
                    payments::TokenizationAction::TokenizeInConnector
                } else {
                    payments::TokenizationAction::SkipConnectorTokenization
                }
            }
            None => payments::TokenizationAction::SkipConnectorTokenization,
        };

        Ok(tokenization_action)
    }
}

#[async_trait]
impl<F: Clone + Sync> UpdateTracker<F, PaymentConfirmData<F>, PaymentsConfirmIntentRequest>
    for PaymentIntentConfirm
{
    #[instrument(skip_all)]
    async fn update_trackers<'b>(
        &'b self,
        state: &'b SessionState,
        req_state: ReqState,
        mut payment_data: PaymentConfirmData<F>,
        customer: Option<domain::Customer>,
        storage_scheme: storage_enums::MerchantStorageScheme,
        updated_customer: Option<storage::CustomerUpdate>,
        key_store: &domain::MerchantKeyStore,
        frm_suggestion: Option<FrmSuggestion>,
        header_payload: hyperswitch_domain_models::payments::HeaderPayload,
    ) -> RouterResult<(BoxedConfirmOperation<'b, F>, PaymentConfirmData<F>)>
    where
        F: 'b + Send,
    {
        let db = &*state.store;

        let intent_status = common_enums::IntentStatus::Processing;
        let attempt_status = common_enums::AttemptStatus::Pending;

        let connector = payment_data
            .payment_attempt
            .connector
            .clone()
            .get_required_value("connector")
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Connector is none when constructing response")?;

        // If `merchant_connector_details` are present in the payment request, `merchant_connector_id` will not be populated.
        let merchant_connector_id = match &payment_data.merchant_connector_details {
            Some(_details) => None,
            None => Some(
                payment_data
                    .payment_attempt
                    .merchant_connector_id
                    .clone()
                    .get_required_value("merchant_connector_id")
                    .change_context(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable("Merchant connector id is none when constructing response")?,
            ),
        };

        let payment_intent_update =
            hyperswitch_domain_models::payments::payment_intent::PaymentIntentUpdate::ConfirmIntent {
                status: intent_status,
                updated_by: storage_scheme.to_string(),
                active_attempt_id: Some(payment_data.payment_attempt.id.clone()),
            };

        let authentication_type = payment_data.payment_attempt.authentication_type;

        let connector_request_reference_id = payment_data
            .payment_attempt
            .connector_request_reference_id
            .clone();

        // Updates payment_attempt for cases where authorize flow is not performed.
        let connector_response_reference_id = payment_data
            .payment_attempt
            .connector_response_reference_id
            .clone();

        let payment_attempt_update = match &payment_data.payment_method {
            // In the case of a tokenized payment method, we update the payment attempt with the tokenized payment method details.
            Some(payment_method) => {
                hyperswitch_domain_models::payments::payment_attempt::PaymentAttemptUpdate::ConfirmIntentTokenized {
                    status: attempt_status,
                    updated_by: storage_scheme.to_string(),
                    connector,
                    merchant_connector_id: merchant_connector_id.ok_or_else( || {
                        error_stack::report!(errors::ApiErrorResponse::InternalServerError)
                            .attach_printable("Merchant connector id is none when constructing response")
                    })?,
                    authentication_type,
                    connector_request_reference_id,
                    payment_method_id : payment_method.get_id().clone()
                }
            }
            None => {
                hyperswitch_domain_models::payments::payment_attempt::PaymentAttemptUpdate::ConfirmIntent {
                    status: attempt_status,
                    updated_by: storage_scheme.to_string(),
                    connector,
                    merchant_connector_id,
                    authentication_type,
                    connector_request_reference_id,
                    connector_response_reference_id,
                }
            }
        };

        let updated_payment_intent = db
            .update_payment_intent(
                payment_data.payment_intent.clone(),
                payment_intent_update,
                key_store,
                storage_scheme,
            )
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Unable to update payment intent")?;

        payment_data.payment_intent = updated_payment_intent;

        let updated_payment_attempt = db
            .update_payment_attempt(
                key_store,
                payment_data.payment_attempt.clone(),
                payment_attempt_update,
                storage_scheme,
            )
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Unable to update payment attempt")?;

        payment_data.payment_attempt = updated_payment_attempt;

        if let Some((customer, updated_customer)) = customer.zip(updated_customer) {
            let customer_id = customer.get_id().clone();
            let customer_merchant_id = customer.merchant_id.clone();

            let _updated_customer = db
                .update_customer_by_global_id(
                    &customer_id,
                    customer,
                    updated_customer,
                    key_store,
                    storage_scheme,
                )
                .await
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Failed to update customer during `update_trackers`")?;
        }

        Ok((Box::new(self), payment_data))
    }
}
