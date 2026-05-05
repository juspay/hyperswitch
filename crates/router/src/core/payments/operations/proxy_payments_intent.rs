use api_models::payments::ProxyPaymentsRequest;
use async_trait::async_trait;
use common_enums::enums;
use common_utils::{ext_traits::Encode, types::keymanager::ToEncryptable};
use error_stack::ResultExt;
use hyperswitch_domain_models::{
    payment_method_data::{PaymentMethodData, RecurringDetails as DomainRecurringDetails},
    payments::PaymentConfirmData,
};
use hyperswitch_interfaces::api::ConnectorSpecifications;
use hyperswitch_masking::PeekInterface;
use router_env::{instrument, tracing};

use super::{Domain, GetTracker, Operation, PostUpdateTracker, UpdateTracker, ValidateRequest};
use crate::{
    core::{
        configs::dimension_state,
        errors::{self, CustomResult, RouterResult, StorageErrorExt},
        payments::{
            operations::{self, ValidateStatusForOperation},
            OperationSessionGetters, OperationSessionSetters,
        },
    },
    routes::{app::ReqState, SessionState},
    types::{
        self,
        api::{self, ConnectorCallType},
        domain::{self, types as domain_types},
        storage::{self, enums as storage_enums},
    },
    utils::OptionExt,
};

#[derive(Debug, Clone, Copy)]
pub struct PaymentProxyIntent;

impl ValidateStatusForOperation for PaymentProxyIntent {
    /// Validate if the current operation can be performed on the current status of the payment intent
    fn validate_status_for_operation(
        &self,
        intent_status: common_enums::IntentStatus,
    ) -> Result<(), errors::ApiErrorResponse> {
        match intent_status {
            //Failed state is included here so that in PCR, retries can be done for failed payments, otherwise for a failed attempt it was asking for new payment_intent
            common_enums::IntentStatus::RequiresPaymentMethod
            | common_enums::IntentStatus::Failed
            | common_enums::IntentStatus::Processing
            | common_enums::IntentStatus::PartiallyCapturedAndProcessing => Ok(()),
            //Failed state is included here so that in PCR, retries can be done for failed payments, otherwise for a failed attempt it was asking for new payment_intent
            common_enums::IntentStatus::RequiresPaymentMethod
            | common_enums::IntentStatus::Failed => Ok(()),
            common_enums::IntentStatus::Conflicted
            | common_enums::IntentStatus::Succeeded
            | common_enums::IntentStatus::Cancelled
            | common_enums::IntentStatus::CancelledPostCapture
            | common_enums::IntentStatus::RequiresCustomerAction
            | common_enums::IntentStatus::RequiresMerchantAction
            | common_enums::IntentStatus::RequiresCapture
            | common_enums::IntentStatus::PartiallyAuthorizedAndRequiresCapture
            | common_enums::IntentStatus::PartiallyCaptured
            | common_enums::IntentStatus::RequiresConfirmation
            | common_enums::IntentStatus::PartiallyCapturedAndCapturable
            | common_enums::IntentStatus::PartiallyCapturedAndProcessing
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
    super::BoxedOperation<'b, F, ProxyPaymentsRequest, PaymentConfirmData<F>>;

impl<F: Send + Clone + Sync> Operation<F, ProxyPaymentsRequest> for &PaymentProxyIntent {
    type Data = PaymentConfirmData<F>;
    fn to_validate_request(
        &self,
    ) -> RouterResult<&(dyn ValidateRequest<F, ProxyPaymentsRequest, Self::Data> + Send + Sync)>
    {
        Ok(*self)
    }
    fn to_get_tracker(
        &self,
    ) -> RouterResult<&(dyn GetTracker<F, Self::Data, ProxyPaymentsRequest> + Send + Sync)> {
        Ok(*self)
    }
    fn to_domain(&self) -> RouterResult<&(dyn Domain<F, ProxyPaymentsRequest, Self::Data>)> {
        Ok(*self)
    }
    fn to_update_tracker(
        &self,
    ) -> RouterResult<&(dyn UpdateTracker<F, Self::Data, ProxyPaymentsRequest> + Send + Sync)> {
        Ok(*self)
    }
}

#[automatically_derived]
impl<F: Send + Clone + Sync> Operation<F, ProxyPaymentsRequest> for PaymentProxyIntent {
    type Data = PaymentConfirmData<F>;
    fn to_validate_request(
        &self,
    ) -> RouterResult<&(dyn ValidateRequest<F, ProxyPaymentsRequest, Self::Data> + Send + Sync)>
    {
        Ok(self)
    }
    fn to_get_tracker(
        &self,
    ) -> RouterResult<&(dyn GetTracker<F, Self::Data, ProxyPaymentsRequest> + Send + Sync)> {
        Ok(self)
    }
    fn to_domain(&self) -> RouterResult<&dyn Domain<F, ProxyPaymentsRequest, Self::Data>> {
        Ok(self)
    }
    fn to_update_tracker(
        &self,
    ) -> RouterResult<&(dyn UpdateTracker<F, Self::Data, ProxyPaymentsRequest> + Send + Sync)> {
        Ok(self)
    }
}

impl<F: Send + Clone + Sync> ValidateRequest<F, ProxyPaymentsRequest, PaymentConfirmData<F>>
    for PaymentProxyIntent
{
    #[instrument(skip_all)]
    fn validate_request<'a, 'b>(
        &'b self,
        _request: &ProxyPaymentsRequest,
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
impl<F: Send + Clone + Sync> GetTracker<F, PaymentConfirmData<F>, ProxyPaymentsRequest>
    for PaymentProxyIntent
{
    #[instrument(skip_all)]
    async fn get_trackers<'a>(
        &'a self,
        state: &'a SessionState,
        payment_id: &common_utils::id_type::GlobalPaymentId,
        request: &ProxyPaymentsRequest,
        platform: &domain::Platform,
        _profile: &domain::Profile,
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

        self.validate_status_for_operation(payment_intent.status)?;

        let cell_id = state.conf.cell_information.id.clone();

        let batch_encrypted_data = domain_types::crypto_operation(
            key_manager_state,
            common_utils::type_name!(hyperswitch_domain_models::payments::payment_attempt::PaymentAttempt),
            domain_types::CryptoOperation::BatchEncrypt(
                hyperswitch_domain_models::payments::payment_attempt::FromRequestEncryptablePaymentAttempt::to_encryptable(
                    hyperswitch_domain_models::payments::payment_attempt::FromRequestEncryptablePaymentAttempt {
                        payment_method_billing_address: None,
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
        let active_attempt_id = payment_intent.active_attempt_id.clone();

        let payment_attempt = match active_attempt_id {
            Some(ref active_attempt_id) => db
                .find_payment_attempt_by_id(
                    platform.get_processor().get_key_store(),
                    active_attempt_id,
                    storage_scheme,
                )
                .await
                .change_context(errors::ApiErrorResponse::PaymentNotFound)
                .attach_printable("Could not find payment attempt")?,

            None => {
                let payment_attempt_domain_model: hyperswitch_domain_models::payments::payment_attempt::PaymentAttempt =
                hyperswitch_domain_models::payments::payment_attempt::PaymentAttempt::proxy_create_domain_model(
                    &payment_intent,
                    cell_id,
                    storage_scheme,
                    request,
                    encrypted_data,
                    platform.get_initiator(),
                )
                .await?;
                db.insert_payment_attempt(
                    platform.get_processor().get_key_store(),
                    payment_attempt_domain_model,
                    storage_scheme,
                )
                .await
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Could not insert payment attempt")?
            }
        };

        let (mandate_data_input, payment_method_data) = match &request.recurring_details {
            api_models::mandates::RecurringDetails::ProcessorPaymentToken(token) => {
                let mandate_ids = api_models::payments::MandateIds {
                    mandate_id: None,
                    mandate_reference_id: Some(
                        api_models::payments::MandateReferenceId::ConnectorMandateId(
                            api_models::payments::ConnectorMandateReferenceId::new(
                                Some(token.processor_payment_token.clone()),
                                None,
                                None,
                                None,
                                None,
                                None,
                            ),
                        ),
                    ),
                };
                (Some(mandate_ids), Some(PaymentMethodData::MandatePayment))
            }
            api_models::mandates::RecurringDetails::CardWithLimitedData(_)
            | api_models::mandates::RecurringDetails::NetworkTransactionIdAndCardDetails(_)
            | api_models::mandates::RecurringDetails::NetworkTransactionIdAndNetworkTokenDetails(_)
            | api_models::mandates::RecurringDetails::NetworkTransactionIdAndDecryptedWalletTokenDetails(_) => {
                let (mandate_reference_id, pmd) =
                    DomainRecurringDetails::from(request.recurring_details.clone())
                        .get_mandate_reference_id_and_payment_method_data_for_proxy_flow()
                        .ok_or(errors::ApiErrorResponse::InternalServerError)
                        .attach_printable("Failed to get mandate reference id for proxy flow")?;
                let mandate_ids = api_models::payments::MandateIds {
                    mandate_id: None,
                    mandate_reference_id: Some(mandate_reference_id),
                };
                (Some(mandate_ids), Some(pmd))
            }
            api_models::mandates::RecurringDetails::MandateId(_)
            | api_models::mandates::RecurringDetails::PaymentMethodId(_) => {
                return Err(error_stack::Report::new(
                    errors::ApiErrorResponse::NotSupported {
                        message: "Recurring flow via Proxy not supported for MandateId or PaymentMethodId".to_string(),
                    },
                ))
            }
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

        let payment_data = PaymentConfirmData {
            flow: std::marker::PhantomData,
            payment_intent,
            payment_attempt,
            payment_method_data,
            payment_address,
            mandate_data: mandate_data_input,
            payment_method: None,
            merchant_connector_details: None,
            external_vault_pmd: None,
            webhook_url: None,
            recurring_details: Some(request.recurring_details.clone()),
        };

        let get_trackers_response = operations::GetTrackerResponse { payment_data };

        Ok(get_trackers_response)
    }
}

#[async_trait]
impl<F: Clone + Send + Sync> Domain<F, ProxyPaymentsRequest, PaymentConfirmData<F>>
    for PaymentProxyIntent
{
    async fn get_customer_details<'a>(
        &'a self,
        _state: &SessionState,
        _payment_data: &mut PaymentConfirmData<F>,
        _merchant_key_store: &domain::MerchantKeyStore,
        _storage_scheme: storage_enums::MerchantStorageScheme,
    ) -> CustomResult<(BoxedConfirmOperation<'a, F>, Option<domain::Customer>), errors::StorageError>
    {
        Ok((Box::new(self), None))
    }

    #[instrument(skip_all)]
    async fn make_pm_data<'a>(
        &'a self,
        _state: &'a SessionState,
        _payment_data: &mut PaymentConfirmData<F>,
        _storage_scheme: storage_enums::MerchantStorageScheme,
        _platform: &domain::Platform,
        _business_profile: &domain::Profile,
        _should_retry_with_pan: bool,
    ) -> RouterResult<(
        BoxedConfirmOperation<'a, F>,
        Option<PaymentMethodData>,
        Option<String>,
    )> {
        Ok((Box::new(self), None, None))
    }
    #[instrument(skip_all)]
    async fn populate_payment_data<'a>(
        &'a self,
        _state: &SessionState,
        payment_data: &mut PaymentConfirmData<F>,
        _processor: &domain::Processor,
        _business_profile: &domain::Profile,
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

    async fn perform_routing<'a>(
        &'a self,
        platform: &domain::Platform,
        business_profile: &domain::Profile,
        state: &SessionState,
        payment_data: &mut PaymentConfirmData<F>,
    ) -> CustomResult<ConnectorCallType, errors::ApiErrorResponse> {
        let connector_name = payment_data.get_payment_attempt_connector().map(|s| s.to_owned());

        if let Some(connector_name) = connector_name {
            let merchant_connector_id = payment_data.get_merchant_connector_id_in_attempt();

            // If ProcessorPaymentToken is used, allow it only when CardWithLimitedData is disabled
            let is_processor_payment_token_flow = matches!(
                payment_data.get_recurring_details(),
                Some(api_models::mandates::RecurringDetails::ProcessorPaymentToken(_))
            );

            if is_processor_payment_token_flow {
                let dimensions = dimension_state::Dimensions::new()
                    .with_processor_merchant_id(
                        platform.get_processor().get_processor_merchant_id(),
                    )
                    .with_provider_merchant_id(
                        platform.get_provider().get_provider_merchant_id(),
                    )
                    .with_profile_id(business_profile.get_id().clone());

                let is_mit_with_limited_card_data_enabled = dimensions
                    .get_should_enable_mit_with_limited_card_data(
                        state.store.as_ref(),
                        state.superposition_service.as_ref(),
                        None,
                    )
                    .await;

                if is_mit_with_limited_card_data_enabled {
                    return Err(error_stack::Report::new(
                        errors::ApiErrorResponse::NotSupported {
                            message: "ProcessorPaymentToken cannot be used when MIT with limited card data is enabled. Use CardWithLimitedData instead.".to_string(),
                        },
                    ));
                }

                // CardWithLimitedData is not enabled — allow ProcessorPaymentToken as fallback
                let connector_data = api::ConnectorData::get_connector_by_name(
                    &state.conf.connectors,
                    &connector_name,
                    api::GetToken::Connector,
                    merchant_connector_id,
                )?;
                payment_data.set_connector_in_payment_attempt(Some(connector_name));
                return Ok(ConnectorCallType::PreDetermined(connector_data.into()));
            }

            let routable_connector = connector_name
                .parse::<euclid::enums::RoutableConnectors>()
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Failed to parse connector name as RoutableConnectors")?;
            let routable_choice = api_models::routing::RoutableConnectorChoice {
                choice_kind: api_models::routing::RoutableChoiceKind::FullStruct,
                connector: routable_connector,
                merchant_connector_id: merchant_connector_id.clone(),
            };
            let straight_through =
                api_models::routing::StraightThroughAlgorithm::Single(Box::new(routable_choice));
            let val = straight_through
                .encode_to_value()
                .change_context(errors::ApiErrorResponse::InternalServerError)?;
            let connector_choice = api::ConnectorChoice::StraightThrough(val);
            let _connector =
                crate::core::payments::set_eligible_connector_for_proxy_in_payment_data(
                    state,
                    &business_profile,
                    platform.get_processor().get_key_store(),
                    payment_data,
                    connector_choice,
                )
                .await?;
            let connector_data = api::ConnectorData::get_connector_by_name(
                &state.conf.connectors,
                &connector_name,
                api::GetToken::Connector,
                merchant_connector_id,
            )?;

            Ok(ConnectorCallType::PreDetermined(connector_data.into()))
        } else {
            Err(error_stack::Report::new(
                errors::ApiErrorResponse::InternalServerError,
            ))
        }
    }
}
#[async_trait]
impl<F: Clone + Sync> UpdateTracker<F, PaymentConfirmData<F>, ProxyPaymentsRequest>
    for PaymentProxyIntent
{
    #[instrument(skip_all)]
    async fn update_trackers<'b>(
        &'b self,
        state: &'b SessionState,
        _req_state: ReqState,
        processor: &domain::Processor,
        mut payment_data: PaymentConfirmData<F>,
        _frm_suggestion: Option<api_models::enums::FrmSuggestion>,
        _header_payload: hyperswitch_domain_models::payments::HeaderPayload,
        _dimensions: &dimension_state::DimensionsWithProcessorAndProviderMerchantId,
    ) -> RouterResult<(BoxedConfirmOperation<'b, F>, PaymentConfirmData<F>)>
    where
        F: 'b + Send,
    {
        let db = &*state.store;
        let storage_scheme = processor.get_account().storage_scheme;
        let key_store = processor.get_key_store();

        let intent_status = common_enums::IntentStatus::Processing;
        let attempt_status = common_enums::AttemptStatus::Pending;

        let connector = payment_data
            .payment_attempt
            .connector
            .clone()
            .get_required_value("connector")
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Connector is none when constructing response")?;

        let merchant_connector_id = Some(
            payment_data
                .payment_attempt
                .merchant_connector_id
                .clone()
                .get_required_value("merchant_connector_id")
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Merchant connector id is none when constructing response")?,
        );
        let active_attempt_id = payment_data.payment_intent.active_attempt_id.clone();
        let payment_intent_update =
            hyperswitch_domain_models::payments::payment_intent::PaymentIntentUpdate::ConfirmIntent {
                status: intent_status,
                updated_by: storage_scheme.to_string(),
                active_attempt_id,
            };

        let authentication_type = payment_data
            .payment_intent
            .authentication_type
            .unwrap_or_default();

        let connector_request_reference_id = payment_data
            .payment_attempt
            .connector_request_reference_id
            .clone();

        let connector_response_reference_id = payment_data
            .payment_attempt
            .connector_response_reference_id
            .clone();

        let payment_attempt_update = hyperswitch_domain_models::payments::payment_attempt::PaymentAttemptUpdate::ConfirmIntent {
            status: attempt_status,
            updated_by: storage_scheme.to_string(),
            connector,
            merchant_connector_id,
            authentication_type,
            connector_request_reference_id,
            connector_response_reference_id,
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

        Ok((Box::new(self), payment_data))
    }
}

#[cfg(feature = "v2")]
#[async_trait]
impl<F: Clone> PostUpdateTracker<F, PaymentConfirmData<F>, types::PaymentsAuthorizeData>
    for PaymentProxyIntent
{
    async fn update_tracker<'b>(
        &'b self,
        state: &'b SessionState,
        processor: &domain::Processor,
        _initiator: Option<&domain::Initiator>,
        mut payment_data: PaymentConfirmData<F>,
        response: types::RouterData<F, types::PaymentsAuthorizeData, types::PaymentsResponseData>,
    ) -> RouterResult<PaymentConfirmData<F>>
    where
        F: 'b + Send + Sync,
        types::RouterData<F, types::PaymentsAuthorizeData, types::PaymentsResponseData>:
            hyperswitch_domain_models::router_data::TrackerPostUpdateObjects<
                F,
                types::PaymentsAuthorizeData,
                PaymentConfirmData<F>,
            >,
    {
        use hyperswitch_domain_models::router_data::TrackerPostUpdateObjects;

        let db = &*state.store;

        let response_router_data = response;

        let payment_intent_update = response_router_data
            .get_payment_intent_update(&payment_data, processor.get_account().storage_scheme);
        let payment_attempt_update = response_router_data
            .get_payment_attempt_update(&payment_data, processor.get_account().storage_scheme);

        let updated_payment_intent = db
            .update_payment_intent(
                payment_data.payment_intent,
                payment_intent_update,
                processor.get_key_store(),
                processor.get_account().storage_scheme,
            )
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Unable to update payment intent")?;

        let updated_payment_attempt = db
            .update_payment_attempt(
                processor.get_key_store(),
                payment_data.payment_attempt,
                payment_attempt_update,
                processor.get_account().storage_scheme,
            )
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Unable to update payment attempt")?;

        payment_data.payment_intent = updated_payment_intent;
        payment_data.payment_attempt = updated_payment_attempt;

        Ok(payment_data)
    }
}
