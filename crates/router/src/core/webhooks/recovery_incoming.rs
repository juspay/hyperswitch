use api_models::webhooks;
use common_utils::ext_traits::AsyncExt;
use error_stack::{report, ResultExt};
use hyperswitch_domain_models::{revenue_recovery, router_flow_types::GetAdditionalRevenueRecoveryDetails, router_request_types::revenue_recovery::GetAdditionalRevenueRecoveryRequestData, router_response_types::revenue_recovery::GetAdditionalRevenueRecoveryResponseData, types::AdditionalRevenueRecoveryDetailsRouterData};
use router_env::{instrument, tracing};
use hyperswitch_interfaces::webhooks as interface_webhooks;
use crate::types;
use std::marker::PhantomData;
use common_utils::ext_traits::ValueExt;
use std::str::FromStr;

use crate::{
    core::{
        errors::{self, CustomResult},
        payments::{self,helpers},
    },
    routes::{app::ReqState, SessionState},
    services::{self, connector_integration_interface},
    types::{
        api::{self,ConnectorData,GetToken}, 
        domain
    },
};

#[allow(clippy::too_many_arguments)]
#[instrument(skip_all)]
#[cfg(feature = "revenue_recovery")]
pub async fn recovery_incoming_webhook_flow(
    state: SessionState,
    merchant_account: domain::MerchantAccount,
    business_profile: domain::Profile,
    key_store: domain::MerchantKeyStore,
    _webhook_details: api::IncomingWebhookDetails,
    source_verified: bool,
    connector_enum: &connector_integration_interface::ConnectorEnum,
    merchant_connector_account: &hyperswitch_domain_models::merchant_connector_account::MerchantConnectorAccount,
    connector_name : &str,
    request_details: &hyperswitch_interfaces::webhooks::IncomingWebhookRequestDetails<'_>,
    event_type: webhooks::IncomingWebhookEvent,
    req_state: ReqState,
    object_ref_id : &webhooks::ObjectReferenceId
) -> CustomResult<webhooks::WebhookResponseTracker, errors::RevenueRecoveryError> {

    // Source verification is necessary for revenue recovery webhooks flow since We don't have payment intent/attempt object created before in our system.
    common_utils::fp_utils::when(!source_verified, || {
        Err(report!(
            errors::RevenueRecoveryError::WebhookAuthenticationFailed
        ))
    })?;

    let connectors_with_additional_recovery_details_call = &state.conf.additional_revenue_recovery_details_call;

    let connector = api_models::enums::Connector::from_str(connector_name)
        .change_context(errors::RevenueRecoveryError::InvoiceWebhookProcessingFailed)
        .attach_printable_lazy(|| {
            format!("unable to parse connector name {connector_name:?}")
    })?;

    let recovery_details  = 
        if connectors_with_additional_recovery_details_call
        .connectors_with_additional_revenue_recovery_details_call
        .contains(&connector)
        {
        
            let additional_revenue_recovery_id = match object_ref_id {
                webhooks::ObjectReferenceId::AdditionalRevenueRecoveryId(
                    webhooks::AdditionalRevenueRecoveryIdType::AdditionalRevenueRecoveryCallId(ref id)
                ) => Some(id.as_str()),
                _ => None,
            };

            let additional_call_response = handle_additional_recovery_details_call(
                connector_enum,
                &state, 
                &merchant_account, 
                merchant_connector_account, 
                connector_name, 
                additional_revenue_recovery_id.unwrap_or("fake_id")
            ).await?;
            
            Some(additional_call_response)

        } else {
            None
        };
    
    let invoice_details = match recovery_details.clone() {
        Some(data)=> {
            RevenueRecoveryInvoice(
            revenue_recovery::RevenueRecoveryInvoiceData::from(data.clone())
            )
        },
        None => 
        {
            RevenueRecoveryInvoice(
            interface_webhooks::IncomingWebhook::get_revenue_recovery_invoice_details(
                connector_enum,
                request_details,
            )
            .change_context(errors::RevenueRecoveryError::InvoiceWebhookProcessingFailed)
            .attach_printable("Failed while getting revenue recovery invoice details")?,
            )
        }
    };

    // Fetch the intent using merchant reference id, if not found create new intent.
    let payment_intent = invoice_details
        .get_payment_intent(
            &state,
            &req_state,
            &merchant_account,
            &business_profile,
            &key_store,
        )
        .await
        .transpose()
        .async_unwrap_or_else(|| async {
            invoice_details
                .create_payment_intent(
                    &state,
                    &req_state,
                    &merchant_account,
                    &business_profile,
                    &key_store,
                )
                .await
        })
        .await?;


    let payment_attempt = match event_type.is_recovery_transaction_event() {
        true => {
            let invoice_transaction_details = match recovery_details.clone() {
                Some(data) => 
                {
                    RevenueRecoveryAttempt(
                    revenue_recovery::RevenueRecoveryAttemptData::from(data)
                    )
                }
                None =>  
                {
                    RevenueRecoveryAttempt(
                    interface_webhooks::IncomingWebhook::get_revenue_recovery_attempt_details(
                        connector_enum,
                        request_details,
                    )
                    .change_context(errors::RevenueRecoveryError::TransactionWebhookProcessingFailed)?,
                    )
                }
            };


            invoice_transaction_details
                .get_payment_attempt(
                    &state,
                    &req_state,
                    &merchant_account,
                    &business_profile,
                    &key_store,
                    payment_intent.payment_id.clone(),
                )
                .await?
        }
        false => None,
    };

    let attempt_triggered_by = payment_attempt
        .and_then(revenue_recovery::RecoveryPaymentAttempt::get_attempt_triggered_by);

    let action = revenue_recovery::RecoveryAction::get_action(event_type, attempt_triggered_by);

    match action {
        revenue_recovery::RecoveryAction::CancelInvoice => todo!(),
        revenue_recovery::RecoveryAction::ScheduleFailedPayment => {
            todo!()
        }
        revenue_recovery::RecoveryAction::SuccessPaymentExternal => {
            todo!()
        }
        revenue_recovery::RecoveryAction::PendingPayment => {
            router_env::logger::info!(
                "Pending transactions are not consumed by the revenue recovery webhooks"
            );
            Ok(webhooks::WebhookResponseTracker::NoEffect)
        }
        revenue_recovery::RecoveryAction::NoAction => {
            router_env::logger::info!(
                "No Recovery action is taken place for recovery event : {:?} and attempt triggered_by : {:?} ", event_type.clone(), attempt_triggered_by
            );
            Ok(webhooks::WebhookResponseTracker::NoEffect)
        }
        revenue_recovery::RecoveryAction::InvalidAction => {
            router_env::logger::error!(
                "Invalid Revenue recovery action state has been received, event : {:?}, triggered_by : {:?}", event_type, attempt_triggered_by
            );
            Ok(webhooks::WebhookResponseTracker::NoEffect)
        }
    }
}

pub struct RevenueRecoveryInvoice(revenue_recovery::RevenueRecoveryInvoiceData);
pub struct RevenueRecoveryAttempt(revenue_recovery::RevenueRecoveryAttemptData);

impl RevenueRecoveryInvoice {
    async fn get_payment_intent(
        &self,
        state: &SessionState,
        req_state: &ReqState,
        merchant_account: &domain::MerchantAccount,
        profile: &domain::Profile,
        key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<Option<revenue_recovery::RecoveryPaymentIntent>, errors::RevenueRecoveryError>
    {
        let payment_response = Box::pin(payments::payments_get_intent_using_merchant_reference(
            state.clone(),
            merchant_account.clone(),
            profile.clone(),
            key_store.clone(),
            req_state.clone(),
            &self.0.merchant_reference_id,
            hyperswitch_domain_models::payments::HeaderPayload::default(),
            None,
        ))
        .await;
        let response = match payment_response {
            Ok(services::ApplicationResponse::JsonWithHeaders((payments_response, _))) => {
                let payment_id = payments_response.id.clone();
                let status = payments_response.status;
                let feature_metadata = payments_response.feature_metadata;
                Ok(Some(revenue_recovery::RecoveryPaymentIntent {
                    payment_id,
                    status,
                    feature_metadata,
                }))
            }
            Err(err)
                if matches!(
                    err.current_context(),
                    &errors::ApiErrorResponse::PaymentNotFound
                ) =>
            {
                Ok(None)
            }
            Ok(_) => Err(errors::RevenueRecoveryError::PaymentIntentFetchFailed)
                .attach_printable("Unexpected response from payment intent core"),
            error @ Err(_) => {
                router_env::logger::error!(?error);
                Err(errors::RevenueRecoveryError::PaymentIntentFetchFailed)
                    .attach_printable("failed to fetch payment intent recovery webhook flow")
            }
        }?;
        Ok(response)
    }
    async fn create_payment_intent(
        &self,
        state: &SessionState,
        req_state: &ReqState,
        merchant_account: &domain::MerchantAccount,
        profile: &domain::Profile,
        key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<revenue_recovery::RecoveryPaymentIntent, errors::RevenueRecoveryError> {
        let payload = api_models::payments::PaymentsCreateIntentRequest::from(&self.0);
        let global_payment_id =
            common_utils::id_type::GlobalPaymentId::generate(&state.conf.cell_information.id);

        let create_intent_response = Box::pin(payments::payments_intent_core::<
            hyperswitch_domain_models::router_flow_types::payments::PaymentCreateIntent,
            api_models::payments::PaymentsIntentResponse,
            _,
            _,
            hyperswitch_domain_models::payments::PaymentIntentData<
                hyperswitch_domain_models::router_flow_types::payments::PaymentCreateIntent,
            >,
        >(
            state.clone(),
            req_state.clone(),
            merchant_account.clone(),
            profile.clone(),
            key_store.clone(),
            payments::operations::PaymentIntentCreate,
            payload,
            global_payment_id,
            hyperswitch_domain_models::payments::HeaderPayload::default(),
            None,
        ))
        .await
        .change_context(errors::RevenueRecoveryError::PaymentIntentCreateFailed)?;

        let response = create_intent_response
            .get_json_body()
            .change_context(errors::RevenueRecoveryError::PaymentIntentCreateFailed)
            .attach_printable("expected json response")?;

        Ok(revenue_recovery::RecoveryPaymentIntent {
            payment_id: response.id,
            status: response.status,
            feature_metadata: response.feature_metadata,
        })
    }
}

impl RevenueRecoveryAttempt {
    async fn get_payment_attempt(
        &self,
        state: &SessionState,
        req_state: &ReqState,
        merchant_account: &domain::MerchantAccount,
        profile: &domain::Profile,
        key_store: &domain::MerchantKeyStore,
        payment_id: common_utils::id_type::GlobalPaymentId,
    ) -> CustomResult<Option<revenue_recovery::RecoveryPaymentAttempt>, errors::RevenueRecoveryError>
    {
        let attempt_response = Box::pin(payments::payments_core::<
            hyperswitch_domain_models::router_flow_types::payments::PSync,
            api_models::payments::PaymentsResponse,
            _,
            _,
            _,
            hyperswitch_domain_models::payments::PaymentStatusData<
                hyperswitch_domain_models::router_flow_types::payments::PSync,
            >,
        >(
            state.clone(),
            req_state.clone(),
            merchant_account.clone(),
            profile.clone(),
            key_store.clone(),
            payments::operations::PaymentGet,
            api_models::payments::PaymentsRetrieveRequest {
                force_sync: false,
                expand_attempts: true,
                param: None,
            },
            payment_id.clone(),
            payments::CallConnectorAction::Avoid,
            hyperswitch_domain_models::payments::HeaderPayload::default(),
        ))
        .await;
        let response = match attempt_response {
            Ok(services::ApplicationResponse::JsonWithHeaders((payments_response, _))) => {
                let final_attempt =
                    self.0
                        .connector_transaction_id
                        .as_ref()
                        .and_then(|transaction_id| {
                            payments_response
                                .find_attempt_in_attempts_list_using_connector_transaction_id(
                                    transaction_id,
                                )
                        });
                let payment_attempt =
                    final_attempt.map(|attempt_res| revenue_recovery::RecoveryPaymentAttempt {
                        attempt_id: attempt_res.id.to_owned(),
                        attempt_status: attempt_res.status.to_owned(),
                        feature_metadata: attempt_res.feature_metadata.to_owned(),
                    });
                Ok(payment_attempt)
            }
            Ok(_) => Err(errors::RevenueRecoveryError::PaymentIntentFetchFailed)
                .attach_printable("Unexpected response from payment intent core"),
            error @ Err(_) => {
                router_env::logger::error!(?error);
                Err(errors::RevenueRecoveryError::PaymentIntentFetchFailed)
                    .attach_printable("failed to fetch payment intent recovery webhook flow")
            }
        }?;
        Ok(response)
    }
    async fn record_payment_attempt(
        &self,
        _state: &SessionState,
        _req_state: &ReqState,
        _merchant_account: &domain::MerchantAccount,
        _profile: &domain::Profile,
        _key_store: &domain::MerchantKeyStore,
        _payment_id: common_utils::id_type::GlobalPaymentId,
    ) -> CustomResult<revenue_recovery::RecoveryPaymentAttempt, errors::RevenueRecoveryError> {
        todo!()
    }
}

async fn handle_additional_recovery_details_call(
    connector: &connector_integration_interface::ConnectorEnum,
    state: &SessionState,
    merchant_account: &domain::MerchantAccount,
    merchant_connector_account: &hyperswitch_domain_models::merchant_connector_account::MerchantConnectorAccount,
    connector_name: &str,
    id : &str 
) -> CustomResult<GetAdditionalRevenueRecoveryResponseData, errors::RevenueRecoveryError> {

    let connector_data = ConnectorData::get_connector_by_name(
        &state.conf.connectors,
        connector_name,
        GetToken::Connector,
        None,
    )
    .change_context(errors::RevenueRecoveryError::AdditionalRevenueRecoveryCallFailed)
    .attach_printable("invalid connector name received in payment attempt")?;


    let connector_integration: services::BoxedGetAdditionalRecoveryRecoveryDetailsIntegrationInterface<
    GetAdditionalRevenueRecoveryDetails,
    GetAdditionalRevenueRecoveryRequestData,
    GetAdditionalRevenueRecoveryResponseData
    > = connector_data.connector.get_connector_integration();

    let router_data = construct_router_data_for_additional_call(
        state,
        connector_name,
        merchant_connector_account,
        merchant_account,
        id,
    )
    .await
    .change_context(errors::RevenueRecoveryError::AdditionalRevenueRecoveryCallFailed)
    .attach_printable("Failed while constructing additional recovery details call router data")?;

    let response = services::execute_connector_processing_step(
        state,
        connector_integration,
        &router_data,
        payments::CallConnectorAction::Trigger,
        None,
    )
    .await
    .change_context(errors::RevenueRecoveryError::AdditionalRevenueRecoveryCallFailed)
    .attach_printable("Failed while calling the API")?;

    let additional_recovery_details = match response.response {
        Ok(response) => Ok(response),
        error @ Err(_) => {
            router_env::logger::error!(?error);
            Err(errors::RevenueRecoveryError::AdditionalRevenueRecoveryCallFailed)
                .attach_printable("failed to fetch payment intent recovery webhook flow")
        }
    }?;
    Ok(additional_recovery_details)
}

const IRRELEVANT_ATTEMPT_ID_IN_ADDITIONAL_REVENUE_RECOVERY_CALL_FLOW: &str =
    "irrelevant_attempt_id_in_additional_revenue_recovery_flow";

const IRRELEVANT_CONNECTOR_REQUEST_REFERENCE_ID_IN_ADDITIONAL_REVENUE_RECOVERY_CALL: &str =
"irrelevant_connector_request_reference_id_in_additional_revenue_recovery_flow";

async fn construct_router_data_for_additional_call(
    state: &SessionState,
    connector_name: &str,
    merchant_connector_account: &hyperswitch_domain_models::merchant_connector_account::MerchantConnectorAccount,
    merchant_account: &domain::MerchantAccount,
    additional_revenue_recovery_id: &str,
) -> CustomResult<AdditionalRevenueRecoveryDetailsRouterData, errors::RevenueRecoveryError>{

    let auth_type: types::ConnectorAuthType =
        helpers::MerchantConnectorAccountType::DbVal(Box::new(merchant_connector_account.clone()))
            .get_connector_account_details()
            .parse_value("ConnectorAuthType")
            .change_context(errors::RevenueRecoveryError::AdditionalRevenueRecoveryCallFailed)?;
    

    let router_data = types::RouterData {
        flow: PhantomData,
        merchant_id: merchant_account.get_id().clone(),
        connector: connector_name.to_string(),
        customer_id: None,
        tenant_id: state.tenant.tenant_id.clone(),
        payment_id: common_utils::id_type::PaymentId::get_irrelevant_id("additional revenue recovery details call flow")
            .get_string_repr()
            .to_owned(),
        attempt_id: IRRELEVANT_ATTEMPT_ID_IN_ADDITIONAL_REVENUE_RECOVERY_CALL_FLOW.to_string(),
        status: diesel_models::enums::AttemptStatus::default(),
        payment_method: diesel_models::enums::PaymentMethod::default(),
        connector_auth_type: auth_type,
        description: None,
        address: hyperswitch_domain_models::payment_address::PaymentAddress::default(),
        auth_type: diesel_models::enums::AuthenticationType::default(),
        connector_meta_data: None,
        connector_wallets_details: None,
        amount_captured: None,
        minor_amount_captured: None,
        request : GetAdditionalRevenueRecoveryRequestData{
            additional_revenue_recovery_id : additional_revenue_recovery_id.to_string()
        },
        response: Err(types::ErrorResponse::default()),
        access_token: None,
        session_token: None,
        reference_id: None,
        payment_method_token: None,
        connector_customer: None,
        recurring_mandate_payment_data: None,
        preprocessing_id: None,
        connector_request_reference_id:
            IRRELEVANT_CONNECTOR_REQUEST_REFERENCE_ID_IN_ADDITIONAL_REVENUE_RECOVERY_CALL.to_string(),
        #[cfg(feature = "payouts")]
        payout_method_data: None,
        #[cfg(feature = "payouts")]
        quote_id: None,
        test_mode: None,
        payment_method_balance: None,
        payment_method_status: None,
        connector_api_version: None,
        connector_http_status_code: None,
        external_latency: None,
        apple_pay_flow: None,
        frm_metadata: None,
        refund_id: None,
        dispute_id: None,
        connector_response: None,
        integrity_check: Ok(()),
        additional_merchant_data: None,
        header_payload: None,
        connector_mandate_request_reference_id: None,
        authentication_id: None,
        psd2_sca_exemption_type: None,
    };
    Ok(router_data)
}    