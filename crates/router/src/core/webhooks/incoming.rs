use std::{str::FromStr, time::Instant};

use actix_web::FromRequest;
#[cfg(feature = "payouts")]
use api_models::payouts as payout_models;
use api_models::webhooks::{self, WebhookResponseTracker};
use common_utils::{
    errors::ReportSwitchExt,
    events::ApiEventsType,
    ext_traits::{AsyncExt, ByteSliceExt},
    types::{AmountConvertor, StringMinorUnitForConnector},
};
use diesel_models::{refund as diesel_refund, ConnectorMandateReferenceId};
use error_stack::{report, ResultExt};
use hyperswitch_domain_models::{
    mandates::CommonMandateReference,
    payments::{payment_attempt::PaymentAttempt, HeaderPayload},
    router_request_types::VerifyWebhookSourceRequestData,
    router_response_types::{VerifyWebhookSourceResponseData, VerifyWebhookStatus},
};
use hyperswitch_interfaces::webhooks::{IncomingWebhookFlowError, IncomingWebhookRequestDetails};
use masking::{ExposeInterface, PeekInterface};
use router_env::{instrument, tracing, tracing_actix_web::RequestId};

use super::{types, utils, MERCHANT_ID};
use crate::{
    consts,
    core::{
        api_locking,
        errors::{self, ConnectorErrorExt, CustomResult, RouterResponse, StorageErrorExt},
        metrics, payment_methods,
        payments::{self, tokenization},
        refunds, relay, unified_connector_service, utils as core_utils,
        webhooks::{network_tokenization_incoming, utils::construct_webhook_router_data},
    },
    db::StorageInterface,
    events::api_logs::ApiEvent,
    logger,
    routes::{
        app::{ReqState, SessionStateInfo},
        lock_utils, SessionState,
    },
    services::{
        self, authentication as auth, connector_integration_interface::ConnectorEnum,
        ConnectorValidation,
    },
    types::{
        api::{
            self, mandates::MandateResponseExt, ConnectorCommon, ConnectorData, GetToken,
            IncomingWebhook,
        },
        domain,
        storage::{self, enums},
        transformers::{ForeignFrom, ForeignInto, ForeignTryFrom},
    },
    utils::{self as helper_utils, ext_traits::OptionExt, generate_id},
};
#[cfg(feature = "payouts")]
use crate::{core::payouts, types::storage::PayoutAttemptUpdate};

#[allow(clippy::too_many_arguments)]
pub async fn incoming_webhooks_wrapper<W: types::OutgoingWebhookType>(
    flow: &impl router_env::types::FlowMetric,
    state: SessionState,
    req_state: ReqState,
    req: &actix_web::HttpRequest,
    merchant_context: domain::MerchantContext,
    connector_name_or_mca_id: &str,
    body: actix_web::web::Bytes,
    is_relay_webhook: bool,
) -> RouterResponse<serde_json::Value> {
    let start_instant = Instant::now();
    let (application_response, webhooks_response_tracker, serialized_req) =
        Box::pin(incoming_webhooks_core::<W>(
            state.clone(),
            req_state,
            req,
            merchant_context.clone(),
            connector_name_or_mca_id,
            body.clone(),
            is_relay_webhook,
        ))
        .await?;

    logger::info!(incoming_webhook_payload = ?serialized_req);

    let request_duration = Instant::now()
        .saturating_duration_since(start_instant)
        .as_millis();

    let request_id = RequestId::extract(req)
        .await
        .attach_printable("Unable to extract request id from request")
        .change_context(errors::ApiErrorResponse::InternalServerError)?;
    let auth_type = auth::AuthenticationType::WebhookAuth {
        merchant_id: merchant_context.get_merchant_account().get_id().clone(),
    };
    let status_code = 200;
    let api_event = ApiEventsType::Webhooks {
        connector: connector_name_or_mca_id.to_string(),
        payment_id: webhooks_response_tracker.get_payment_id(),
    };
    let response_value = serde_json::to_value(&webhooks_response_tracker)
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Could not convert webhook effect to string")?;
    let infra = state.infra_components.clone();
    let api_event = ApiEvent::new(
        state.tenant.tenant_id.clone(),
        Some(merchant_context.get_merchant_account().get_id().clone()),
        flow,
        &request_id,
        request_duration,
        status_code,
        serialized_req,
        Some(response_value),
        None,
        auth_type,
        None,
        api_event,
        req,
        req.method(),
        infra,
    );
    state.event_handler().log_event(&api_event);
    Ok(application_response)
}

#[cfg(feature = "v1")]
pub async fn network_token_incoming_webhooks_wrapper<W: types::OutgoingWebhookType>(
    flow: &impl router_env::types::FlowMetric,
    state: SessionState,
    req: &actix_web::HttpRequest,
    body: actix_web::web::Bytes,
) -> RouterResponse<serde_json::Value> {
    let start_instant = Instant::now();

    let request_details: IncomingWebhookRequestDetails<'_> = IncomingWebhookRequestDetails {
        method: req.method().clone(),
        uri: req.uri().clone(),
        headers: req.headers(),
        query_params: req.query_string().to_string(),
        body: &body,
    };

    let (application_response, webhooks_response_tracker, serialized_req, merchant_id) = Box::pin(
        network_token_incoming_webhooks_core::<W>(&state, request_details),
    )
    .await?;

    logger::info!(incoming_webhook_payload = ?serialized_req);

    let request_duration = Instant::now()
        .saturating_duration_since(start_instant)
        .as_millis();

    let request_id = RequestId::extract(req)
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Unable to extract request id from request")?;
    let auth_type = auth::AuthenticationType::NoAuth;
    let status_code = 200;
    let api_event = ApiEventsType::NetworkTokenWebhook {
        payment_method_id: webhooks_response_tracker.get_payment_method_id(),
    };
    let response_value = serde_json::to_value(&webhooks_response_tracker)
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Could not convert webhook effect to string")?;
    let infra = state.infra_components.clone();
    let api_event = ApiEvent::new(
        state.tenant.tenant_id.clone(),
        Some(merchant_id),
        flow,
        &request_id,
        request_duration,
        status_code,
        serialized_req,
        Some(response_value),
        None,
        auth_type,
        None,
        api_event,
        req,
        req.method(),
        infra,
    );
    state.event_handler().log_event(&api_event);
    Ok(application_response)
}

#[allow(clippy::too_many_arguments)]
#[instrument(skip_all)]
async fn incoming_webhooks_core<W: types::OutgoingWebhookType>(
    state: SessionState,
    req_state: ReqState,
    req: &actix_web::HttpRequest,
    merchant_context: domain::MerchantContext,
    connector_name_or_mca_id: &str,
    body: actix_web::web::Bytes,
    is_relay_webhook: bool,
) -> errors::RouterResult<(
    services::ApplicationResponse<serde_json::Value>,
    WebhookResponseTracker,
    serde_json::Value,
)> {
    // Initial setup and metrics
    metrics::WEBHOOK_INCOMING_COUNT.add(
        1,
        router_env::metric_attributes!((
            MERCHANT_ID,
            merchant_context.get_merchant_account().get_id().clone()
        )),
    );

    let request_details = IncomingWebhookRequestDetails {
        method: req.method().clone(),
        uri: req.uri().clone(),
        headers: req.headers(),
        query_params: req.query_string().to_string(),
        body: &body,
    };

    // Fetch the merchant connector account to get the webhooks source secret
    let (merchant_connector_account, connector, connector_name) =
        fetch_optional_mca_and_connector(&state, &merchant_context, connector_name_or_mca_id)
            .await?;

    // Determine webhook processing path (UCS vs non-UCS) and handle event type extraction
    let webhook_processing_result =
        if unified_connector_service::should_call_unified_connector_service_for_webhooks(
            &state,
            &merchant_context,
            &connector_name,
        )
        .await?
        {
            logger::info!(
                connector = connector_name,
                "Using Unified Connector Service for webhook processing",
            );
            process_ucs_webhook_transform(
                &state,
                &merchant_context,
                &connector_name,
                &body,
                &request_details,
                merchant_connector_account.as_ref(),
            )
            .await?
        } else {
            // NON-UCS PATH: Need to decode body first
            let decoded_body = connector
                .decode_webhook_body(
                    &request_details,
                    merchant_context.get_merchant_account().get_id(),
                    merchant_connector_account
                        .and_then(|mca| mca.connector_webhook_details.clone()),
                    &connector_name,
                )
                .await
                .switch()
                .attach_printable("There was an error in incoming webhook body decoding")?;

            process_non_ucs_webhook(
                &state,
                &merchant_context,
                &connector,
                &connector_name,
                decoded_body.into(),
                &request_details,
            )
            .await?
        };

    // Update request_details with the appropriate body (decoded for non-UCS, raw for UCS)
    let final_request_details = match &webhook_processing_result.decoded_body {
        Some(decoded_body) => IncomingWebhookRequestDetails {
            method: request_details.method.clone(),
            uri: request_details.uri.clone(),
            headers: request_details.headers,
            query_params: request_details.query_params.clone(),
            body: decoded_body,
        },
        None => request_details, // Use original request details for UCS
    };

    logger::info!(event_type=?webhook_processing_result.event_type);

    // Check if webhook should be processed further
    let is_webhook_event_supported = !matches!(
        webhook_processing_result.event_type,
        webhooks::IncomingWebhookEvent::EventNotSupported
    );
    let is_webhook_event_enabled = !utils::is_webhook_event_disabled(
        &*state.clone().store,
        connector_name.as_str(),
        merchant_context.get_merchant_account().get_id(),
        &webhook_processing_result.event_type,
    )
    .await;
    let flow_type: api::WebhookFlow = webhook_processing_result.event_type.into();
    let process_webhook_further = is_webhook_event_enabled
        && is_webhook_event_supported
        && !matches!(flow_type, api::WebhookFlow::ReturnResponse);
    logger::info!(process_webhook=?process_webhook_further);
    let mut event_object: Box<dyn masking::ErasedMaskSerialize> = Box::new(serde_json::Value::Null);

    let webhook_effect = match process_webhook_further {
        true => {
            let business_logic_result = process_webhook_business_logic(
                &state,
                req_state,
                &merchant_context,
                &connector,
                &connector_name,
                webhook_processing_result.event_type,
                webhook_processing_result.source_verified,
                &webhook_processing_result.transform_data,
                &final_request_details,
                is_relay_webhook,
            )
            .await;

            match business_logic_result {
                Ok(response) => {
                    // Extract event object for serialization
                    event_object = extract_webhook_event_object(
                        &webhook_processing_result.transform_data,
                        &connector,
                        &final_request_details,
                    )?;
                    response
                }
                Err(error) => {
                    let error_result = handle_incoming_webhook_error(
                        error,
                        &connector,
                        connector_name.as_str(),
                        &final_request_details,
                    );
                    match error_result {
                        Ok((_, webhook_tracker, _)) => webhook_tracker,
                        Err(e) => return Err(e),
                    }
                }
            }
        }
        false => {
            metrics::WEBHOOK_INCOMING_FILTERED_COUNT.add(
                1,
                router_env::metric_attributes!((
                    MERCHANT_ID,
                    merchant_context.get_merchant_account().get_id().clone()
                )),
            );
            WebhookResponseTracker::NoEffect
        }
    };

    // Generate response
    let response = connector
        .get_webhook_api_response(&final_request_details, None)
        .switch()
        .attach_printable("Could not get incoming webhook api response from connector")?;

    let serialized_request = event_object
        .masked_serialize()
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Could not convert webhook effect to string")?;

    Ok((response, webhook_effect, serialized_request))
}

/// Process UCS webhook transformation using the high-level UCS abstraction
async fn process_ucs_webhook_transform(
    state: &SessionState,
    merchant_context: &domain::MerchantContext,
    connector_name: &str,
    body: &actix_web::web::Bytes,
    request_details: &IncomingWebhookRequestDetails<'_>,
    merchant_connector_account: Option<&domain::MerchantConnectorAccount>,
) -> errors::RouterResult<WebhookProcessingResult> {
    // Use the new UCS abstraction which provides clean separation
    let (event_type, source_verified, transform_data) =
        unified_connector_service::call_unified_connector_service_for_webhook(
            state,
            merchant_context,
            connector_name,
            body,
            request_details,
            merchant_connector_account,
        )
        .await?;
    Ok(WebhookProcessingResult {
        event_type,
        source_verified,
        transform_data: Some(Box::new(transform_data)),
        decoded_body: None, // UCS path uses raw body
    })
}
/// Result type for webhook processing path determination
pub struct WebhookProcessingResult {
    event_type: webhooks::IncomingWebhookEvent,
    source_verified: bool,
    transform_data: Option<Box<unified_connector_service::WebhookTransformData>>,
    decoded_body: Option<actix_web::web::Bytes>,
}

/// Process non-UCS webhook using traditional connector processing
async fn process_non_ucs_webhook(
    state: &SessionState,
    merchant_context: &domain::MerchantContext,
    connector: &ConnectorEnum,
    connector_name: &str,
    decoded_body: actix_web::web::Bytes,
    request_details: &IncomingWebhookRequestDetails<'_>,
) -> errors::RouterResult<WebhookProcessingResult> {
    // Create request_details with decoded body for connector processing
    let updated_request_details = IncomingWebhookRequestDetails {
        method: request_details.method.clone(),
        uri: request_details.uri.clone(),
        headers: request_details.headers,
        query_params: request_details.query_params.clone(),
        body: &decoded_body,
    };

    match connector
        .get_webhook_event_type(&updated_request_details)
        .allow_webhook_event_type_not_found(
            state
                .clone()
                .conf
                .webhooks
                .ignore_error
                .event_type
                .unwrap_or(true),
        )
        .switch()
        .attach_printable("Could not find event type in incoming webhook body")?
    {
        Some(event_type) => Ok(WebhookProcessingResult {
            event_type,
            source_verified: false,
            transform_data: None,
            decoded_body: Some(decoded_body),
        }),
        None => {
            metrics::WEBHOOK_EVENT_TYPE_IDENTIFICATION_FAILURE_COUNT.add(
                1,
                router_env::metric_attributes!(
                    (
                        MERCHANT_ID,
                        merchant_context.get_merchant_account().get_id().clone()
                    ),
                    ("connector", connector_name.to_string())
                ),
            );
            Err(errors::ApiErrorResponse::WebhookProcessingFailure)
                .attach_printable("Failed to identify event type in incoming webhook body")
        }
    }
}

/// Extract webhook event object based on transform data availability
fn extract_webhook_event_object(
    transform_data: &Option<Box<unified_connector_service::WebhookTransformData>>,
    connector: &ConnectorEnum,
    request_details: &IncomingWebhookRequestDetails<'_>,
) -> errors::RouterResult<Box<dyn masking::ErasedMaskSerialize>> {
    match transform_data {
        Some(transform_data) => match &transform_data.webhook_content {
            Some(webhook_content) => {
                let serialized_value = serde_json::to_value(webhook_content)
                    .change_context(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable("Failed to serialize UCS webhook content")?;
                Ok(Box::new(serialized_value))
            }
            None => connector
                .get_webhook_resource_object(request_details)
                .switch()
                .attach_printable("Could not find resource object in incoming webhook body"),
        },
        None => connector
            .get_webhook_resource_object(request_details)
            .switch()
            .attach_printable("Could not find resource object in incoming webhook body"),
    }
}

/// Process the main webhook business logic after event type determination
#[allow(clippy::too_many_arguments)]
async fn process_webhook_business_logic(
    state: &SessionState,
    req_state: ReqState,
    merchant_context: &domain::MerchantContext,
    connector: &ConnectorEnum,
    connector_name: &str,
    event_type: webhooks::IncomingWebhookEvent,
    source_verified_via_ucs: bool,
    webhook_transform_data: &Option<Box<unified_connector_service::WebhookTransformData>>,
    request_details: &IncomingWebhookRequestDetails<'_>,
    is_relay_webhook: bool,
) -> errors::RouterResult<WebhookResponseTracker> {
    let object_ref_id = connector
        .get_webhook_object_reference_id(request_details)
        .switch()
        .attach_printable("Could not find object reference id in incoming webhook body")?;
    let connector_enum = api_models::enums::Connector::from_str(connector_name)
        .change_context(errors::ApiErrorResponse::InvalidDataValue {
            field_name: "connector",
        })
        .attach_printable_lazy(|| format!("unable to parse connector name {connector_name:?}"))?;
    let connectors_with_source_verification_call = &state.conf.webhook_source_verification_call;

    let merchant_connector_account = match Box::pin(helper_utils::get_mca_from_object_reference_id(
        state,
        object_ref_id.clone(),
        merchant_context,
        connector_name,
    ))
    .await
    {
        Ok(mca) => mca,
        Err(error) => {
            let result =
                handle_incoming_webhook_error(error, connector, connector_name, request_details);
            match result {
                Ok((_, webhook_tracker, _)) => return Ok(webhook_tracker),
                Err(e) => return Err(e),
            }
        }
    };

    let source_verified = if source_verified_via_ucs {
        // If UCS handled verification, use that result
        source_verified_via_ucs
    } else {
        // Fall back to traditional source verification
        if connectors_with_source_verification_call
            .connectors_with_webhook_source_verification_call
            .contains(&connector_enum)
        {
            verify_webhook_source_verification_call(
                connector.clone(),
                state,
                merchant_context,
                merchant_connector_account.clone(),
                connector_name,
                request_details,
            )
            .await
            .or_else(|error| match error.current_context() {
                errors::ConnectorError::WebhookSourceVerificationFailed => {
                    logger::error!(?error, "Source Verification Failed");
                    Ok(false)
                }
                _ => Err(error),
            })
            .switch()
            .attach_printable("There was an issue in incoming webhook source verification")?
        } else {
            connector
                .clone()
                .verify_webhook_source(
                    request_details,
                    merchant_context.get_merchant_account().get_id(),
                    merchant_connector_account.connector_webhook_details.clone(),
                    merchant_connector_account.connector_account_details.clone(),
                    connector_name,
                )
                .await
                .or_else(|error| match error.current_context() {
                    errors::ConnectorError::WebhookSourceVerificationFailed => {
                        logger::error!(?error, "Source Verification Failed");
                        Ok(false)
                    }
                    _ => Err(error),
                })
                .switch()
                .attach_printable("There was an issue in incoming webhook source verification")?
        }
    };

    if source_verified {
        metrics::WEBHOOK_SOURCE_VERIFIED_COUNT.add(
            1,
            router_env::metric_attributes!((
                MERCHANT_ID,
                merchant_context.get_merchant_account().get_id().clone()
            )),
        );
    } else if connector.is_webhook_source_verification_mandatory() {
        // if webhook consumption is mandatory for connector, fail webhook
        // so that merchant can retrigger it after updating merchant_secret
        return Err(errors::ApiErrorResponse::WebhookAuthenticationFailed.into());
    }

    logger::info!(source_verified=?source_verified);

    let event_object: Box<dyn masking::ErasedMaskSerialize> =
        if let Some(transform_data) = webhook_transform_data {
            // Use UCS transform data if available
            if let Some(webhook_content) = &transform_data.webhook_content {
                // Convert UCS webhook content to appropriate format
                Box::new(
                    serde_json::to_value(webhook_content)
                        .change_context(errors::ApiErrorResponse::InternalServerError)
                        .attach_printable("Failed to serialize UCS webhook content")?,
                )
            } else {
                // Fall back to connector extraction
                connector
                    .get_webhook_resource_object(request_details)
                    .switch()
                    .attach_printable("Could not find resource object in incoming webhook body")?
            }
        } else {
            // Use traditional connector extraction
            connector
                .get_webhook_resource_object(request_details)
                .switch()
                .attach_printable("Could not find resource object in incoming webhook body")?
        };

    let webhook_details = api::IncomingWebhookDetails {
        object_reference_id: object_ref_id.clone(),
        resource_object: serde_json::to_vec(&event_object)
            .change_context(errors::ParsingError::EncodeError("byte-vec"))
            .attach_printable("Unable to convert webhook payload to a value")
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable(
                "There was an issue when encoding the incoming webhook body to bytes",
            )?,
    };

    let profile_id = &merchant_connector_account.profile_id;
    let key_manager_state = &(state).into();

    let business_profile = state
        .store
        .find_business_profile_by_profile_id(
            key_manager_state,
            merchant_context.get_merchant_key_store(),
            profile_id,
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::ProfileNotFound {
            id: profile_id.get_string_repr().to_owned(),
        })?;

    // If the incoming webhook is a relay webhook, then we need to trigger the relay webhook flow
    let result_response = if is_relay_webhook {
        let relay_webhook_response = Box::pin(relay_incoming_webhook_flow(
            state.clone(),
            merchant_context.clone(),
            business_profile,
            webhook_details,
            event_type,
            source_verified,
        ))
        .await
        .attach_printable("Incoming webhook flow for relay failed");

        // Using early return ensures unsupported webhooks are acknowledged to the connector
        if let Some(errors::ApiErrorResponse::NotSupported { .. }) = relay_webhook_response
            .as_ref()
            .err()
            .map(|a| a.current_context())
        {
            logger::error!(
                webhook_payload =? request_details.body,
                "Failed while identifying the event type",
            );

            let _response = connector
                    .get_webhook_api_response(request_details, None)
                    .switch()
                    .attach_printable(
                        "Failed while early return in case of not supported event type in relay webhooks",
                    )?;

            return Ok(WebhookResponseTracker::NoEffect);
        };

        relay_webhook_response
    } else {
        let flow_type: api::WebhookFlow = event_type.into();
        match flow_type {
            api::WebhookFlow::Payment => Box::pin(payments_incoming_webhook_flow(
                state.clone(),
                req_state,
                merchant_context.clone(),
                business_profile,
                webhook_details,
                source_verified,
                connector,
                request_details,
                event_type,
                webhook_transform_data,
            ))
            .await
            .attach_printable("Incoming webhook flow for payments failed"),

            api::WebhookFlow::Refund => Box::pin(refunds_incoming_webhook_flow(
                state.clone(),
                merchant_context.clone(),
                business_profile,
                webhook_details,
                connector_name,
                source_verified,
                event_type,
            ))
            .await
            .attach_printable("Incoming webhook flow for refunds failed"),

            api::WebhookFlow::Dispute => Box::pin(disputes_incoming_webhook_flow(
                state.clone(),
                merchant_context.clone(),
                business_profile,
                webhook_details,
                source_verified,
                connector,
                request_details,
                event_type,
            ))
            .await
            .attach_printable("Incoming webhook flow for disputes failed"),

            api::WebhookFlow::BankTransfer => Box::pin(bank_transfer_webhook_flow(
                state.clone(),
                req_state,
                merchant_context.clone(),
                business_profile,
                webhook_details,
                source_verified,
            ))
            .await
            .attach_printable("Incoming bank-transfer webhook flow failed"),

            api::WebhookFlow::ReturnResponse => Ok(WebhookResponseTracker::NoEffect),

            api::WebhookFlow::Mandate => Box::pin(mandates_incoming_webhook_flow(
                state.clone(),
                merchant_context.clone(),
                business_profile,
                webhook_details,
                source_verified,
                event_type,
            ))
            .await
            .attach_printable("Incoming webhook flow for mandates failed"),

            api::WebhookFlow::ExternalAuthentication => {
                Box::pin(external_authentication_incoming_webhook_flow(
                    state.clone(),
                    req_state,
                    merchant_context.clone(),
                    source_verified,
                    event_type,
                    request_details,
                    connector,
                    object_ref_id,
                    business_profile,
                    merchant_connector_account,
                ))
                .await
                .attach_printable("Incoming webhook flow for external authentication failed")
            }
            api::WebhookFlow::FraudCheck => Box::pin(frm_incoming_webhook_flow(
                state.clone(),
                req_state,
                merchant_context.clone(),
                source_verified,
                event_type,
                object_ref_id,
                business_profile,
            ))
            .await
            .attach_printable("Incoming webhook flow for fraud check failed"),

            #[cfg(feature = "payouts")]
            api::WebhookFlow::Payout => Box::pin(payouts_incoming_webhook_flow(
                state.clone(),
                merchant_context.clone(),
                business_profile,
                webhook_details,
                event_type,
                source_verified,
            ))
            .await
            .attach_printable("Incoming webhook flow for payouts failed"),

            _ => Err(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Unsupported Flow Type received in incoming webhooks"),
        }
    };

    match result_response {
        Ok(response) => Ok(response),
        Err(error) => {
            let result =
                handle_incoming_webhook_error(error, connector, connector_name, request_details);
            match result {
                Ok((_, webhook_tracker, _)) => Ok(webhook_tracker),
                Err(e) => Err(e),
            }
        }
    }
}

fn handle_incoming_webhook_error(
    error: error_stack::Report<errors::ApiErrorResponse>,
    connector: &ConnectorEnum,
    connector_name: &str,
    request_details: &IncomingWebhookRequestDetails<'_>,
) -> errors::RouterResult<(
    services::ApplicationResponse<serde_json::Value>,
    WebhookResponseTracker,
    serde_json::Value,
)> {
    logger::error!(?error, "Incoming webhook flow failed");

    // fetch the connector enum from the connector name
    let connector_enum = api_models::connector_enums::Connector::from_str(connector_name)
        .change_context(errors::ApiErrorResponse::InvalidDataValue {
            field_name: "connector",
        })
        .attach_printable_lazy(|| format!("unable to parse connector name {connector_name:?}"))?;

    // get the error response from the connector
    if connector_enum.should_acknowledge_webhook_for_resource_not_found_errors() {
        let response = connector
            .get_webhook_api_response(
                request_details,
                Some(IncomingWebhookFlowError::from(error.current_context())),
            )
            .switch()
            .attach_printable("Failed to get incoming webhook api response from connector")?;
        Ok((
            response,
            WebhookResponseTracker::NoEffect,
            serde_json::Value::Null,
        ))
    } else {
        Err(error)
    }
}

#[instrument(skip_all)]
#[cfg(feature = "v1")]
async fn network_token_incoming_webhooks_core<W: types::OutgoingWebhookType>(
    state: &SessionState,
    request_details: IncomingWebhookRequestDetails<'_>,
) -> errors::RouterResult<(
    services::ApplicationResponse<serde_json::Value>,
    WebhookResponseTracker,
    serde_json::Value,
    common_utils::id_type::MerchantId,
)> {
    let serialized_request =
        network_tokenization_incoming::get_network_token_resource_object(&request_details)
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Network Token Requestor Webhook deserialization failed")?
            .masked_serialize()
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Could not convert webhook effect to string")?;

    let network_tokenization_service = &state
        .conf
        .network_tokenization_service
        .as_ref()
        .ok_or(errors::NetworkTokenizationError::NetworkTokenizationServiceNotConfigured)
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Network Tokenization Service not configured")?;

    //source verification
    network_tokenization_incoming::Authorization::new(request_details.headers.get("Authorization"))
        .verify_webhook_source(network_tokenization_service.get_inner())
        .await?;

    let response: network_tokenization_incoming::NetworkTokenWebhookResponse = request_details
        .body
        .parse_struct("NetworkTokenWebhookResponse")
        .change_context(errors::ApiErrorResponse::WebhookUnprocessableEntity)?;

    let (merchant_id, payment_method_id, _customer_id) = response
        .fetch_merchant_id_payment_method_id_customer_id_from_callback_mapper(state)
        .await?;

    metrics::WEBHOOK_SOURCE_VERIFIED_COUNT.add(
        1,
        router_env::metric_attributes!((MERCHANT_ID, merchant_id.clone())),
    );

    let merchant_context =
        network_tokenization_incoming::fetch_merchant_account_for_network_token_webhooks(
            state,
            &merchant_id,
        )
        .await?;
    let payment_method =
        network_tokenization_incoming::fetch_payment_method_for_network_token_webhooks(
            state,
            merchant_context.get_merchant_account(),
            merchant_context.get_merchant_key_store(),
            &payment_method_id,
        )
        .await?;

    let response_data = response.get_response_data();

    let webhook_resp_tracker = response_data
        .update_payment_method(state, &payment_method, &merchant_context)
        .await?;

    Ok((
        services::ApplicationResponse::StatusOk,
        webhook_resp_tracker,
        serialized_request,
        merchant_id.clone(),
    ))
}

#[allow(clippy::too_many_arguments)]
#[instrument(skip_all)]
async fn payments_incoming_webhook_flow(
    state: SessionState,
    req_state: ReqState,
    merchant_context: domain::MerchantContext,
    business_profile: domain::Profile,
    webhook_details: api::IncomingWebhookDetails,
    source_verified: bool,
    connector: &ConnectorEnum,
    request_details: &IncomingWebhookRequestDetails<'_>,
    event_type: webhooks::IncomingWebhookEvent,
    webhook_transform_data: &Option<Box<unified_connector_service::WebhookTransformData>>,
) -> CustomResult<WebhookResponseTracker, errors::ApiErrorResponse> {
    let consume_or_trigger_flow = if source_verified {
        // Determine the appropriate action based on UCS availability
        let resource_object = webhook_details.resource_object;

        match webhook_transform_data.as_ref() {
            Some(transform_data) => {
                // Serialize the transform data to pass to UCS handler
                let transform_data_bytes = serde_json::to_vec(transform_data.as_ref())
                    .change_context(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable("Failed to serialize UCS webhook transform data")?;
                payments::CallConnectorAction::UCSHandleResponse(transform_data_bytes)
            }
            None => payments::CallConnectorAction::HandleResponse(resource_object),
        }
    } else {
        payments::CallConnectorAction::Trigger
    };
    let payments_response = match webhook_details.object_reference_id {
        webhooks::ObjectReferenceId::PaymentId(ref id) => {
            let payment_id = get_payment_id(
                state.store.as_ref(),
                id,
                merchant_context.get_merchant_account().get_id(),
                merchant_context.get_merchant_account().storage_scheme,
            )
            .await?;

            let lock_action = api_locking::LockAction::Hold {
                input: api_locking::LockingInput {
                    unique_locking_key: payment_id.get_string_repr().to_owned(),
                    api_identifier: lock_utils::ApiIdentifier::Payments,
                    override_lock_retries: None,
                },
            };

            lock_action
                .clone()
                .perform_locking_action(
                    &state,
                    merchant_context.get_merchant_account().get_id().to_owned(),
                )
                .await?;

            let response = Box::pin(payments::payments_core::<
                api::PSync,
                api::PaymentsResponse,
                _,
                _,
                _,
                payments::PaymentData<api::PSync>,
            >(
                state.clone(),
                req_state,
                merchant_context.clone(),
                None,
                payments::operations::PaymentStatus,
                api::PaymentsRetrieveRequest {
                    resource_id: id.clone(),
                    merchant_id: Some(merchant_context.get_merchant_account().get_id().clone()),
                    force_sync: true,
                    connector: None,
                    param: None,
                    merchant_connector_details: None,
                    client_secret: None,
                    expand_attempts: None,
                    expand_captures: None,
                    all_keys_required: None,
                },
                services::AuthFlow::Merchant,
                consume_or_trigger_flow.clone(),
                None,
                HeaderPayload::default(),
            ))
            .await;
            // When mandate details are present in successful webhooks, and consuming webhooks are skipped during payment sync if the payment status is already updated to charged, this function is used to update the connector mandate details.
            if should_update_connector_mandate_details(source_verified, event_type) {
                update_connector_mandate_details(
                    &state,
                    &merchant_context,
                    webhook_details.object_reference_id.clone(),
                    connector,
                    request_details,
                )
                .await?
            };
            lock_action
                .free_lock_action(
                    &state,
                    merchant_context.get_merchant_account().get_id().to_owned(),
                )
                .await?;

            match response {
                Ok(value) => value,
                Err(err)
                    if matches!(
                        err.current_context(),
                        &errors::ApiErrorResponse::PaymentNotFound
                    ) && state
                        .conf
                        .webhooks
                        .ignore_error
                        .payment_not_found
                        .unwrap_or(true) =>
                {
                    metrics::WEBHOOK_PAYMENT_NOT_FOUND.add(
                        1,
                        router_env::metric_attributes!((
                            "merchant_id",
                            merchant_context.get_merchant_account().get_id().clone()
                        )),
                    );
                    return Ok(WebhookResponseTracker::NoEffect);
                }
                error @ Err(_) => error?,
            }
        }
        _ => Err(errors::ApiErrorResponse::WebhookProcessingFailure).attach_printable(
            "Did not get payment id as object reference id in webhook payments flow",
        )?,
    };

    match payments_response {
        services::ApplicationResponse::JsonWithHeaders((payments_response, _)) => {
            let payment_id = payments_response.payment_id.clone();

            let status = payments_response.status;

            let event_type: Option<enums::EventType> = payments_response.status.into();

            // If event is NOT an UnsupportedEvent, trigger Outgoing Webhook
            if let Some(outgoing_event_type) = event_type {
                let primary_object_created_at = payments_response.created;
                Box::pin(super::create_event_and_trigger_outgoing_webhook(
                    state,
                    merchant_context,
                    business_profile,
                    outgoing_event_type,
                    enums::EventClass::Payments,
                    payment_id.get_string_repr().to_owned(),
                    enums::EventObjectType::PaymentDetails,
                    api::OutgoingWebhookContent::PaymentDetails(Box::new(payments_response)),
                    primary_object_created_at,
                ))
                .await?;
            };

            let response = WebhookResponseTracker::Payment { payment_id, status };

            Ok(response)
        }

        _ => Err(errors::ApiErrorResponse::WebhookProcessingFailure)
            .attach_printable("received non-json response from payments core")?,
    }
}

#[cfg(feature = "payouts")]
#[instrument(skip_all)]
async fn payouts_incoming_webhook_flow(
    state: SessionState,
    merchant_context: domain::MerchantContext,
    business_profile: domain::Profile,
    webhook_details: api::IncomingWebhookDetails,
    event_type: webhooks::IncomingWebhookEvent,
    source_verified: bool,
) -> CustomResult<WebhookResponseTracker, errors::ApiErrorResponse> {
    metrics::INCOMING_PAYOUT_WEBHOOK_METRIC.add(1, &[]);
    if source_verified {
        let db = &*state.store;
        //find payout_attempt by object_reference_id
        let payout_attempt = match webhook_details.object_reference_id {
            webhooks::ObjectReferenceId::PayoutId(payout_id_type) => match payout_id_type {
                webhooks::PayoutIdType::PayoutAttemptId(id) => db
                    .find_payout_attempt_by_merchant_id_payout_attempt_id(
                        merchant_context.get_merchant_account().get_id(),
                        &id,
                        merchant_context.get_merchant_account().storage_scheme,
                    )
                    .await
                    .change_context(errors::ApiErrorResponse::WebhookResourceNotFound)
                    .attach_printable("Failed to fetch the payout attempt")?,
                webhooks::PayoutIdType::ConnectorPayoutId(id) => db
                    .find_payout_attempt_by_merchant_id_connector_payout_id(
                        merchant_context.get_merchant_account().get_id(),
                        &id,
                        merchant_context.get_merchant_account().storage_scheme,
                    )
                    .await
                    .change_context(errors::ApiErrorResponse::WebhookResourceNotFound)
                    .attach_printable("Failed to fetch the payout attempt")?,
            },
            _ => Err(errors::ApiErrorResponse::WebhookProcessingFailure)
                .attach_printable("received a non-payout id when processing payout webhooks")?,
        };

        let payouts = db
            .find_payout_by_merchant_id_payout_id(
                merchant_context.get_merchant_account().get_id(),
                &payout_attempt.payout_id,
                merchant_context.get_merchant_account().storage_scheme,
            )
            .await
            .change_context(errors::ApiErrorResponse::WebhookResourceNotFound)
            .attach_printable("Failed to fetch the payout")?;

        let payout_attempt_update = PayoutAttemptUpdate::StatusUpdate {
            connector_payout_id: payout_attempt.connector_payout_id.clone(),
            status: common_enums::PayoutStatus::foreign_try_from(event_type)
                .change_context(errors::ApiErrorResponse::WebhookProcessingFailure)
                .attach_printable("failed payout status mapping from event type")?,
            error_message: None,
            error_code: None,
            is_eligible: payout_attempt.is_eligible,
            unified_code: None,
            unified_message: None,
        };

        let action_req =
            payout_models::PayoutRequest::PayoutActionRequest(payout_models::PayoutActionRequest {
                payout_id: payouts.payout_id.clone(),
            });

        let mut payout_data = Box::pin(payouts::make_payout_data(
            &state,
            &merchant_context,
            None,
            &action_req,
            common_utils::consts::DEFAULT_LOCALE,
        ))
        .await?;

        let updated_payout_attempt = db
            .update_payout_attempt(
                &payout_attempt,
                payout_attempt_update,
                &payout_data.payouts,
                merchant_context.get_merchant_account().storage_scheme,
            )
            .await
            .change_context(errors::ApiErrorResponse::WebhookResourceNotFound)
            .attach_printable_lazy(|| {
                format!(
                    "Failed while updating payout attempt: payout_attempt_id: {}",
                    payout_attempt.payout_attempt_id
                )
            })?;
        payout_data.payout_attempt = updated_payout_attempt;

        let event_type: Option<enums::EventType> = payout_data.payout_attempt.status.into();

        // If event is NOT an UnsupportedEvent, trigger Outgoing Webhook
        if let Some(outgoing_event_type) = event_type {
            let payout_create_response =
                payouts::response_handler(&state, &merchant_context, &payout_data).await?;

            Box::pin(super::create_event_and_trigger_outgoing_webhook(
                state,
                merchant_context,
                business_profile,
                outgoing_event_type,
                enums::EventClass::Payouts,
                payout_data
                    .payout_attempt
                    .payout_id
                    .get_string_repr()
                    .to_string(),
                enums::EventObjectType::PayoutDetails,
                api::OutgoingWebhookContent::PayoutDetails(Box::new(payout_create_response)),
                Some(payout_data.payout_attempt.created_at),
            ))
            .await?;
        }

        Ok(WebhookResponseTracker::Payout {
            payout_id: payout_data.payout_attempt.payout_id,
            status: payout_data.payout_attempt.status,
        })
    } else {
        metrics::INCOMING_PAYOUT_WEBHOOK_SIGNATURE_FAILURE_METRIC.add(1, &[]);
        Err(report!(
            errors::ApiErrorResponse::WebhookAuthenticationFailed
        ))
    }
}

async fn relay_refunds_incoming_webhook_flow(
    state: SessionState,
    merchant_context: domain::MerchantContext,
    business_profile: domain::Profile,
    webhook_details: api::IncomingWebhookDetails,
    event_type: webhooks::IncomingWebhookEvent,
    source_verified: bool,
) -> CustomResult<WebhookResponseTracker, errors::ApiErrorResponse> {
    let db = &*state.store;
    let key_manager_state = &(&state).into();

    let relay_record = match webhook_details.object_reference_id {
        webhooks::ObjectReferenceId::RefundId(refund_id_type) => match refund_id_type {
            webhooks::RefundIdType::RefundId(refund_id) => {
                let relay_id = common_utils::id_type::RelayId::from_str(&refund_id)
                    .change_context(errors::ValidationError::IncorrectValueProvided {
                        field_name: "relay_id",
                    })
                    .change_context(errors::ApiErrorResponse::InternalServerError)?;

                db.find_relay_by_id(
                    key_manager_state,
                    merchant_context.get_merchant_key_store(),
                    &relay_id,
                )
                .await
                .to_not_found_response(errors::ApiErrorResponse::WebhookResourceNotFound)
                .attach_printable("Failed to fetch the relay record")?
            }
            webhooks::RefundIdType::ConnectorRefundId(connector_refund_id) => db
                .find_relay_by_profile_id_connector_reference_id(
                    key_manager_state,
                    merchant_context.get_merchant_key_store(),
                    business_profile.get_id(),
                    &connector_refund_id,
                )
                .await
                .to_not_found_response(errors::ApiErrorResponse::WebhookResourceNotFound)
                .attach_printable("Failed to fetch the relay record")?,
        },
        _ => Err(errors::ApiErrorResponse::WebhookProcessingFailure)
            .attach_printable("received a non-refund id when processing relay refund webhooks")?,
    };

    // if source_verified then update relay status else trigger relay force sync
    let relay_response = if source_verified {
        let relay_update = hyperswitch_domain_models::relay::RelayUpdate::StatusUpdate {
            connector_reference_id: None,
            status: common_enums::RelayStatus::foreign_try_from(event_type)
                .change_context(errors::ApiErrorResponse::WebhookProcessingFailure)
                .attach_printable("failed relay refund status mapping from event type")?,
        };
        db.update_relay(
            key_manager_state,
            merchant_context.get_merchant_key_store(),
            relay_record,
            relay_update,
        )
        .await
        .map(api_models::relay::RelayResponse::from)
        .to_not_found_response(errors::ApiErrorResponse::WebhookResourceNotFound)
        .attach_printable("Failed to update relay")?
    } else {
        let relay_retrieve_request = api_models::relay::RelayRetrieveRequest {
            force_sync: true,
            id: relay_record.id,
        };
        let relay_force_sync_response = Box::pin(relay::relay_retrieve(
            state,
            merchant_context,
            Some(business_profile.get_id().clone()),
            relay_retrieve_request,
        ))
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to force sync relay")?;

        if let hyperswitch_domain_models::api::ApplicationResponse::Json(response) =
            relay_force_sync_response
        {
            response
        } else {
            Err(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Unexpected response from force sync relay")?
        }
    };

    Ok(WebhookResponseTracker::Relay {
        relay_id: relay_response.id,
        status: relay_response.status,
    })
}

#[allow(clippy::too_many_arguments)]
#[instrument(skip_all)]
async fn refunds_incoming_webhook_flow(
    state: SessionState,
    merchant_context: domain::MerchantContext,
    business_profile: domain::Profile,
    webhook_details: api::IncomingWebhookDetails,
    connector_name: &str,
    source_verified: bool,
    event_type: webhooks::IncomingWebhookEvent,
) -> CustomResult<WebhookResponseTracker, errors::ApiErrorResponse> {
    let db = &*state.store;
    //find refund by connector refund id
    let refund = match webhook_details.object_reference_id {
        webhooks::ObjectReferenceId::RefundId(refund_id_type) => match refund_id_type {
            webhooks::RefundIdType::RefundId(id) => db
                .find_refund_by_merchant_id_refund_id(
                    merchant_context.get_merchant_account().get_id(),
                    &id,
                    merchant_context.get_merchant_account().storage_scheme,
                )
                .await
                .change_context(errors::ApiErrorResponse::WebhookResourceNotFound)
                .attach_printable("Failed to fetch the refund")?,
            webhooks::RefundIdType::ConnectorRefundId(id) => db
                .find_refund_by_merchant_id_connector_refund_id_connector(
                    merchant_context.get_merchant_account().get_id(),
                    &id,
                    connector_name,
                    merchant_context.get_merchant_account().storage_scheme,
                )
                .await
                .change_context(errors::ApiErrorResponse::WebhookResourceNotFound)
                .attach_printable("Failed to fetch the refund")?,
        },
        _ => Err(errors::ApiErrorResponse::WebhookProcessingFailure)
            .attach_printable("received a non-refund id when processing refund webhooks")?,
    };
    let refund_id = refund.refund_id.to_owned();
    //if source verified then update refund status else trigger refund sync
    let updated_refund = if source_verified {
        let refund_update = diesel_refund::RefundUpdate::StatusUpdate {
            connector_refund_id: None,
            sent_to_gateway: true,
            refund_status: common_enums::RefundStatus::foreign_try_from(event_type)
                .change_context(errors::ApiErrorResponse::WebhookProcessingFailure)
                .attach_printable("failed refund status mapping from event type")?,
            updated_by: merchant_context
                .get_merchant_account()
                .storage_scheme
                .to_string(),
            processor_refund_data: None,
        };
        db.update_refund(
            refund.to_owned(),
            refund_update,
            merchant_context.get_merchant_account().storage_scheme,
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::WebhookResourceNotFound)
        .attach_printable_lazy(|| format!("Failed while updating refund: refund_id: {refund_id}"))?
    } else {
        Box::pin(refunds::refund_retrieve_core_with_refund_id(
            state.clone(),
            merchant_context.clone(),
            None,
            api_models::refunds::RefundsRetrieveRequest {
                refund_id: refund_id.to_owned(),
                force_sync: Some(true),
                merchant_connector_details: None,
            },
        ))
        .await
        .attach_printable_lazy(|| format!("Failed while updating refund: refund_id: {refund_id}"))?
    };
    let event_type: Option<enums::EventType> = updated_refund.refund_status.into();

    // If event is NOT an UnsupportedEvent, trigger Outgoing Webhook
    if let Some(outgoing_event_type) = event_type {
        let refund_response: api_models::refunds::RefundResponse =
            updated_refund.clone().foreign_into();
        Box::pin(super::create_event_and_trigger_outgoing_webhook(
            state,
            merchant_context,
            business_profile,
            outgoing_event_type,
            enums::EventClass::Refunds,
            refund_id,
            enums::EventObjectType::RefundDetails,
            api::OutgoingWebhookContent::RefundDetails(Box::new(refund_response)),
            Some(updated_refund.created_at),
        ))
        .await?;
    }

    Ok(WebhookResponseTracker::Refund {
        payment_id: updated_refund.payment_id,
        refund_id: updated_refund.refund_id,
        status: updated_refund.refund_status,
    })
}

async fn relay_incoming_webhook_flow(
    state: SessionState,
    merchant_context: domain::MerchantContext,
    business_profile: domain::Profile,
    webhook_details: api::IncomingWebhookDetails,
    event_type: webhooks::IncomingWebhookEvent,
    source_verified: bool,
) -> CustomResult<WebhookResponseTracker, errors::ApiErrorResponse> {
    let flow_type: api::WebhookFlow = event_type.into();

    let result_response = match flow_type {
        webhooks::WebhookFlow::Refund => Box::pin(relay_refunds_incoming_webhook_flow(
            state,
            merchant_context,
            business_profile,
            webhook_details,
            event_type,
            source_verified,
        ))
        .await
        .attach_printable("Incoming webhook flow for relay refund failed")?,
        webhooks::WebhookFlow::Payment
        | webhooks::WebhookFlow::Payout
        | webhooks::WebhookFlow::Dispute
        | webhooks::WebhookFlow::Subscription
        | webhooks::WebhookFlow::ReturnResponse
        | webhooks::WebhookFlow::BankTransfer
        | webhooks::WebhookFlow::Mandate
        | webhooks::WebhookFlow::ExternalAuthentication
        | webhooks::WebhookFlow::FraudCheck => Err(errors::ApiErrorResponse::NotSupported {
            message: "Relay webhook flow types not supported".to_string(),
        })?,
    };
    Ok(result_response)
}

pub async fn get_payment_attempt_from_object_reference_id(
    state: &SessionState,
    object_reference_id: webhooks::ObjectReferenceId,
    merchant_context: &domain::MerchantContext,
) -> CustomResult<PaymentAttempt, errors::ApiErrorResponse> {
    let db = &*state.store;
    match object_reference_id {
        api::ObjectReferenceId::PaymentId(api::PaymentIdType::ConnectorTransactionId(ref id)) => db
            .find_payment_attempt_by_merchant_id_connector_txn_id(
                merchant_context.get_merchant_account().get_id(),
                id,
                merchant_context.get_merchant_account().storage_scheme,
            )
            .await
            .to_not_found_response(errors::ApiErrorResponse::WebhookResourceNotFound),
        api::ObjectReferenceId::PaymentId(api::PaymentIdType::PaymentAttemptId(ref id)) => db
            .find_payment_attempt_by_attempt_id_merchant_id(
                id,
                merchant_context.get_merchant_account().get_id(),
                merchant_context.get_merchant_account().storage_scheme,
            )
            .await
            .to_not_found_response(errors::ApiErrorResponse::WebhookResourceNotFound),
        api::ObjectReferenceId::PaymentId(api::PaymentIdType::PreprocessingId(ref id)) => db
            .find_payment_attempt_by_preprocessing_id_merchant_id(
                id,
                merchant_context.get_merchant_account().get_id(),
                merchant_context.get_merchant_account().storage_scheme,
            )
            .await
            .to_not_found_response(errors::ApiErrorResponse::WebhookResourceNotFound),
        _ => Err(errors::ApiErrorResponse::WebhookProcessingFailure)
            .attach_printable("received a non-payment id for retrieving payment")?,
    }
}

#[allow(clippy::too_many_arguments)]
pub async fn get_or_update_dispute_object(
    state: SessionState,
    option_dispute: Option<diesel_models::dispute::Dispute>,
    dispute_details: api::disputes::DisputePayload,
    merchant_id: &common_utils::id_type::MerchantId,
    organization_id: &common_utils::id_type::OrganizationId,
    payment_attempt: &PaymentAttempt,
    dispute_status: common_enums::enums::DisputeStatus,
    business_profile: &domain::Profile,
    connector_name: &str,
) -> CustomResult<diesel_models::dispute::Dispute, errors::ApiErrorResponse> {
    let db = &*state.store;
    match option_dispute {
        None => {
            metrics::INCOMING_DISPUTE_WEBHOOK_NEW_RECORD_METRIC.add(1, &[]);
            let dispute_id = generate_id(consts::ID_LENGTH, "dp");
            let new_dispute = diesel_models::dispute::DisputeNew {
                dispute_id,
                amount: dispute_details.amount.clone(),
                currency: dispute_details.currency.to_string(),
                dispute_stage: dispute_details.dispute_stage,
                dispute_status,
                payment_id: payment_attempt.payment_id.to_owned(),
                connector: connector_name.to_owned(),
                attempt_id: payment_attempt.attempt_id.to_owned(),
                merchant_id: merchant_id.to_owned(),
                connector_status: dispute_details.connector_status,
                connector_dispute_id: dispute_details.connector_dispute_id,
                connector_reason: dispute_details.connector_reason,
                connector_reason_code: dispute_details.connector_reason_code,
                challenge_required_by: dispute_details.challenge_required_by,
                connector_created_at: dispute_details.created_at,
                connector_updated_at: dispute_details.updated_at,
                profile_id: Some(business_profile.get_id().to_owned()),
                evidence: None,
                merchant_connector_id: payment_attempt.merchant_connector_id.clone(),
                dispute_amount: StringMinorUnitForConnector::convert_back(
                    &StringMinorUnitForConnector,
                    dispute_details.amount,
                    dispute_details.currency,
                )
                .change_context(
                    errors::ApiErrorResponse::AmountConversionFailed {
                        amount_type: "MinorUnit",
                    },
                )?,
                organization_id: organization_id.clone(),
                dispute_currency: Some(dispute_details.currency),
            };
            state
                .store
                .insert_dispute(new_dispute.clone())
                .await
                .to_not_found_response(errors::ApiErrorResponse::WebhookResourceNotFound)
        }
        Some(dispute) => {
            logger::info!("Dispute Already exists, Updating the dispute details");
            metrics::INCOMING_DISPUTE_WEBHOOK_UPDATE_RECORD_METRIC.add(1, &[]);
            core_utils::validate_dispute_stage_and_dispute_status(
                dispute.dispute_stage,
                dispute.dispute_status,
                dispute_details.dispute_stage,
                dispute_status,
            )
            .change_context(errors::ApiErrorResponse::WebhookProcessingFailure)
            .attach_printable("dispute stage and status validation failed")?;
            let update_dispute = diesel_models::dispute::DisputeUpdate::Update {
                dispute_stage: dispute_details.dispute_stage,
                dispute_status,
                connector_status: dispute_details.connector_status,
                connector_reason: dispute_details.connector_reason,
                connector_reason_code: dispute_details.connector_reason_code,
                challenge_required_by: dispute_details.challenge_required_by,
                connector_updated_at: dispute_details.updated_at,
            };
            db.update_dispute(dispute, update_dispute)
                .await
                .to_not_found_response(errors::ApiErrorResponse::WebhookResourceNotFound)
        }
    }
}

#[allow(clippy::too_many_arguments)]
#[instrument(skip_all)]
async fn external_authentication_incoming_webhook_flow(
    state: SessionState,
    req_state: ReqState,
    merchant_context: domain::MerchantContext,
    source_verified: bool,
    event_type: webhooks::IncomingWebhookEvent,
    request_details: &IncomingWebhookRequestDetails<'_>,
    connector: &ConnectorEnum,
    object_ref_id: api::ObjectReferenceId,
    business_profile: domain::Profile,
    merchant_connector_account: domain::MerchantConnectorAccount,
) -> CustomResult<WebhookResponseTracker, errors::ApiErrorResponse> {
    if source_verified {
        let authentication_details = connector
            .get_external_authentication_details(request_details)
            .switch()?;
        let trans_status = authentication_details.trans_status;
        let authentication_update = storage::AuthenticationUpdate::PostAuthenticationUpdate {
            authentication_status: common_enums::AuthenticationStatus::foreign_from(
                trans_status.clone(),
            ),
            trans_status,
            eci: authentication_details.eci,
            challenge_cancel: authentication_details.challenge_cancel,
            challenge_code_reason: authentication_details.challenge_code_reason,
        };
        let authentication =
            if let webhooks::ObjectReferenceId::ExternalAuthenticationID(authentication_id_type) =
                object_ref_id
            {
                match authentication_id_type {
                    webhooks::AuthenticationIdType::AuthenticationId(authentication_id) => state
                        .store
                        .find_authentication_by_merchant_id_authentication_id(
                            merchant_context.get_merchant_account().get_id(),
                            &authentication_id,
                        )
                        .await
                        .to_not_found_response(errors::ApiErrorResponse::AuthenticationNotFound {
                            id: authentication_id.get_string_repr().to_string(),
                        })
                        .attach_printable("Error while fetching authentication record"),
                    webhooks::AuthenticationIdType::ConnectorAuthenticationId(
                        connector_authentication_id,
                    ) => state
                        .store
                        .find_authentication_by_merchant_id_connector_authentication_id(
                            merchant_context.get_merchant_account().get_id().clone(),
                            connector_authentication_id.clone(),
                        )
                        .await
                        .to_not_found_response(errors::ApiErrorResponse::AuthenticationNotFound {
                            id: connector_authentication_id,
                        })
                        .attach_printable("Error while fetching authentication record"),
                }
            } else {
                Err(errors::ApiErrorResponse::WebhookProcessingFailure).attach_printable(
                    "received a non-external-authentication id for retrieving authentication",
                )
            }?;
        let updated_authentication = state
            .store
            .update_authentication_by_merchant_id_authentication_id(
                authentication,
                authentication_update,
            )
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Error while updating authentication")?;

        authentication_details
            .authentication_value
            .async_map(|auth_val| {
                payment_methods::vault::create_tokenize(
                    &state,
                    auth_val.expose(),
                    None,
                    updated_authentication
                        .authentication_id
                        .clone()
                        .get_string_repr()
                        .to_string(),
                    merchant_context.get_merchant_key_store().key.get_inner(),
                )
            })
            .await
            .transpose()?;
        // Check if it's a payment authentication flow, payment_id would be there only for payment authentication flows
        if let Some(payment_id) = updated_authentication.payment_id {
            let is_pull_mechanism_enabled = helper_utils::check_if_pull_mechanism_for_external_3ds_enabled_from_connector_metadata(merchant_connector_account.metadata.map(|metadata| metadata.expose()));
            // Merchant doesn't have pull mechanism enabled and if it's challenge flow, we have to authorize whenever we receive a ARes webhook
            if !is_pull_mechanism_enabled
                && updated_authentication.authentication_type
                    == Some(common_enums::DecoupledAuthenticationType::Challenge)
                && event_type == webhooks::IncomingWebhookEvent::ExternalAuthenticationARes
            {
                let payment_confirm_req = api::PaymentsRequest {
                    payment_id: Some(api_models::payments::PaymentIdType::PaymentIntentId(
                        payment_id,
                    )),
                    merchant_id: Some(merchant_context.get_merchant_account().get_id().clone()),
                    ..Default::default()
                };
                let payments_response = Box::pin(payments::payments_core::<
                    api::Authorize,
                    api::PaymentsResponse,
                    _,
                    _,
                    _,
                    payments::PaymentData<api::Authorize>,
                >(
                    state.clone(),
                    req_state,
                    merchant_context.clone(),
                    None,
                    payments::PaymentConfirm,
                    payment_confirm_req,
                    services::api::AuthFlow::Merchant,
                    payments::CallConnectorAction::Trigger,
                    None,
                    HeaderPayload::with_source(enums::PaymentSource::ExternalAuthenticator),
                ))
                .await?;
                match payments_response {
                    services::ApplicationResponse::JsonWithHeaders((payments_response, _)) => {
                        let payment_id = payments_response.payment_id.clone();

                        let status = payments_response.status;
                        let event_type: Option<enums::EventType> = payments_response.status.into();
                        // Set poll_id as completed in redis to allow the fetch status of poll through retrieve_poll_status api from client
                        let poll_id = core_utils::get_poll_id(
                            merchant_context.get_merchant_account().get_id(),
                            core_utils::get_external_authentication_request_poll_id(&payment_id),
                        );
                        let redis_conn = state
                            .store
                            .get_redis_conn()
                            .change_context(errors::ApiErrorResponse::InternalServerError)
                            .attach_printable("Failed to get redis connection")?;
                        redis_conn
                            .set_key_without_modifying_ttl(
                                &poll_id.into(),
                                api_models::poll::PollStatus::Completed.to_string(),
                            )
                            .await
                            .change_context(errors::ApiErrorResponse::InternalServerError)
                            .attach_printable("Failed to add poll_id in redis")?;
                        // If event is NOT an UnsupportedEvent, trigger Outgoing Webhook
                        if let Some(outgoing_event_type) = event_type {
                            let primary_object_created_at = payments_response.created;
                            Box::pin(super::create_event_and_trigger_outgoing_webhook(
                                state,
                                merchant_context,
                                business_profile,
                                outgoing_event_type,
                                enums::EventClass::Payments,
                                payment_id.get_string_repr().to_owned(),
                                enums::EventObjectType::PaymentDetails,
                                api::OutgoingWebhookContent::PaymentDetails(Box::new(
                                    payments_response,
                                )),
                                primary_object_created_at,
                            ))
                            .await?;
                        };
                        let response = WebhookResponseTracker::Payment { payment_id, status };
                        Ok(response)
                    }
                    _ => Err(errors::ApiErrorResponse::WebhookProcessingFailure).attach_printable(
                        "Did not get payment id as object reference id in webhook payments flow",
                    )?,
                }
            } else {
                Ok(WebhookResponseTracker::NoEffect)
            }
        } else {
            Ok(WebhookResponseTracker::NoEffect)
        }
    } else {
        logger::error!(
            "Webhook source verification failed for external authentication webhook flow"
        );
        Err(report!(
            errors::ApiErrorResponse::WebhookAuthenticationFailed
        ))
    }
}

#[instrument(skip_all)]
async fn mandates_incoming_webhook_flow(
    state: SessionState,
    merchant_context: domain::MerchantContext,
    business_profile: domain::Profile,
    webhook_details: api::IncomingWebhookDetails,
    source_verified: bool,
    event_type: webhooks::IncomingWebhookEvent,
) -> CustomResult<WebhookResponseTracker, errors::ApiErrorResponse> {
    if source_verified {
        let db = &*state.store;
        let mandate = match webhook_details.object_reference_id {
            webhooks::ObjectReferenceId::MandateId(webhooks::MandateIdType::MandateId(
                mandate_id,
            )) => db
                .find_mandate_by_merchant_id_mandate_id(
                    merchant_context.get_merchant_account().get_id(),
                    mandate_id.as_str(),
                    merchant_context.get_merchant_account().storage_scheme,
                )
                .await
                .to_not_found_response(errors::ApiErrorResponse::MandateNotFound)?,
            webhooks::ObjectReferenceId::MandateId(
                webhooks::MandateIdType::ConnectorMandateId(connector_mandate_id),
            ) => db
                .find_mandate_by_merchant_id_connector_mandate_id(
                    merchant_context.get_merchant_account().get_id(),
                    connector_mandate_id.as_str(),
                    merchant_context.get_merchant_account().storage_scheme,
                )
                .await
                .to_not_found_response(errors::ApiErrorResponse::MandateNotFound)?,
            _ => Err(errors::ApiErrorResponse::WebhookProcessingFailure)
                .attach_printable("received a non-mandate id for retrieving mandate")?,
        };
        let mandate_status = common_enums::MandateStatus::foreign_try_from(event_type)
            .change_context(errors::ApiErrorResponse::WebhookProcessingFailure)
            .attach_printable("event type to mandate status mapping failed")?;
        let mandate_id = mandate.mandate_id.clone();
        let updated_mandate = db
            .update_mandate_by_merchant_id_mandate_id(
                merchant_context.get_merchant_account().get_id(),
                &mandate_id,
                storage::MandateUpdate::StatusUpdate { mandate_status },
                mandate,
                merchant_context.get_merchant_account().storage_scheme,
            )
            .await
            .to_not_found_response(errors::ApiErrorResponse::MandateNotFound)?;
        let mandates_response = Box::new(
            api::mandates::MandateResponse::from_db_mandate(
                &state,
                merchant_context.get_merchant_key_store().clone(),
                updated_mandate.clone(),
                merchant_context.get_merchant_account(),
            )
            .await?,
        );
        let event_type: Option<enums::EventType> = updated_mandate.mandate_status.into();
        if let Some(outgoing_event_type) = event_type {
            Box::pin(super::create_event_and_trigger_outgoing_webhook(
                state,
                merchant_context,
                business_profile,
                outgoing_event_type,
                enums::EventClass::Mandates,
                updated_mandate.mandate_id.clone(),
                enums::EventObjectType::MandateDetails,
                api::OutgoingWebhookContent::MandateDetails(mandates_response),
                Some(updated_mandate.created_at),
            ))
            .await?;
        }
        Ok(WebhookResponseTracker::Mandate {
            mandate_id: updated_mandate.mandate_id,
            status: updated_mandate.mandate_status,
        })
    } else {
        logger::error!("Webhook source verification failed for mandates webhook flow");
        Err(report!(
            errors::ApiErrorResponse::WebhookAuthenticationFailed
        ))
    }
}

#[allow(clippy::too_many_arguments)]
#[instrument(skip_all)]
async fn frm_incoming_webhook_flow(
    state: SessionState,
    req_state: ReqState,
    merchant_context: domain::MerchantContext,
    source_verified: bool,
    event_type: webhooks::IncomingWebhookEvent,
    object_ref_id: api::ObjectReferenceId,
    business_profile: domain::Profile,
) -> CustomResult<WebhookResponseTracker, errors::ApiErrorResponse> {
    if source_verified {
        let payment_attempt =
            get_payment_attempt_from_object_reference_id(&state, object_ref_id, &merchant_context)
                .await?;
        let payment_response = match event_type {
            webhooks::IncomingWebhookEvent::FrmApproved => {
                Box::pin(payments::payments_core::<
                    api::Capture,
                    api::PaymentsResponse,
                    _,
                    _,
                    _,
                    payments::PaymentData<api::Capture>,
                >(
                    state.clone(),
                    req_state,
                    merchant_context.clone(),
                    None,
                    payments::PaymentApprove,
                    api::PaymentsCaptureRequest {
                        payment_id: payment_attempt.payment_id,
                        amount_to_capture: payment_attempt.amount_to_capture,
                        ..Default::default()
                    },
                    services::api::AuthFlow::Merchant,
                    payments::CallConnectorAction::Trigger,
                    None,
                    HeaderPayload::default(),
                ))
                .await?
            }
            webhooks::IncomingWebhookEvent::FrmRejected => {
                Box::pin(payments::payments_core::<
                    api::Void,
                    api::PaymentsResponse,
                    _,
                    _,
                    _,
                    payments::PaymentData<api::Void>,
                >(
                    state.clone(),
                    req_state,
                    merchant_context.clone(),
                    None,
                    payments::PaymentReject,
                    api::PaymentsCancelRequest {
                        payment_id: payment_attempt.payment_id.clone(),
                        cancellation_reason: Some(
                            "Rejected by merchant based on FRM decision".to_string(),
                        ),
                        ..Default::default()
                    },
                    services::api::AuthFlow::Merchant,
                    payments::CallConnectorAction::Trigger,
                    None,
                    HeaderPayload::default(),
                ))
                .await?
            }
            _ => Err(errors::ApiErrorResponse::EventNotFound)?,
        };
        match payment_response {
            services::ApplicationResponse::JsonWithHeaders((payments_response, _)) => {
                let payment_id = payments_response.payment_id.clone();
                let status = payments_response.status;
                let event_type: Option<enums::EventType> = payments_response.status.into();
                if let Some(outgoing_event_type) = event_type {
                    let primary_object_created_at = payments_response.created;
                    Box::pin(super::create_event_and_trigger_outgoing_webhook(
                        state,
                        merchant_context,
                        business_profile,
                        outgoing_event_type,
                        enums::EventClass::Payments,
                        payment_id.get_string_repr().to_owned(),
                        enums::EventObjectType::PaymentDetails,
                        api::OutgoingWebhookContent::PaymentDetails(Box::new(payments_response)),
                        primary_object_created_at,
                    ))
                    .await?;
                };
                let response = WebhookResponseTracker::Payment { payment_id, status };
                Ok(response)
            }
            _ => Err(errors::ApiErrorResponse::WebhookProcessingFailure).attach_printable(
                "Did not get payment id as object reference id in webhook payments flow",
            )?,
        }
    } else {
        logger::error!("Webhook source verification failed for frm webhooks flow");
        Err(report!(
            errors::ApiErrorResponse::WebhookAuthenticationFailed
        ))
    }
}

#[allow(clippy::too_many_arguments)]
#[instrument(skip_all)]
async fn disputes_incoming_webhook_flow(
    state: SessionState,
    merchant_context: domain::MerchantContext,
    business_profile: domain::Profile,
    webhook_details: api::IncomingWebhookDetails,
    source_verified: bool,
    connector: &ConnectorEnum,
    request_details: &IncomingWebhookRequestDetails<'_>,
    event_type: webhooks::IncomingWebhookEvent,
) -> CustomResult<WebhookResponseTracker, errors::ApiErrorResponse> {
    metrics::INCOMING_DISPUTE_WEBHOOK_METRIC.add(1, &[]);
    if source_verified {
        let db = &*state.store;
        let dispute_details = connector.get_dispute_details(request_details).switch()?;
        let payment_attempt = get_payment_attempt_from_object_reference_id(
            &state,
            webhook_details.object_reference_id,
            &merchant_context,
        )
        .await?;
        let option_dispute = db
            .find_by_merchant_id_payment_id_connector_dispute_id(
                merchant_context.get_merchant_account().get_id(),
                &payment_attempt.payment_id,
                &dispute_details.connector_dispute_id,
            )
            .await
            .to_not_found_response(errors::ApiErrorResponse::WebhookResourceNotFound)?;
        let dispute_status = common_enums::DisputeStatus::foreign_try_from(event_type)
            .change_context(errors::ApiErrorResponse::WebhookProcessingFailure)
            .attach_printable("event type to dispute status mapping failed")?;

        let dispute_object = get_or_update_dispute_object(
            state.clone(),
            option_dispute,
            dispute_details,
            merchant_context.get_merchant_account().get_id(),
            &merchant_context.get_merchant_account().organization_id,
            &payment_attempt,
            dispute_status,
            &business_profile,
            connector.id(),
        )
        .await?;
        let disputes_response = Box::new(dispute_object.clone().foreign_into());
        let event_type: enums::EventType = dispute_object.dispute_status.into();

        Box::pin(super::create_event_and_trigger_outgoing_webhook(
            state,
            merchant_context,
            business_profile,
            event_type,
            enums::EventClass::Disputes,
            dispute_object.dispute_id.clone(),
            enums::EventObjectType::DisputeDetails,
            api::OutgoingWebhookContent::DisputeDetails(disputes_response),
            Some(dispute_object.created_at),
        ))
        .await?;
        metrics::INCOMING_DISPUTE_WEBHOOK_MERCHANT_NOTIFIED_METRIC.add(1, &[]);
        Ok(WebhookResponseTracker::Dispute {
            dispute_id: dispute_object.dispute_id,
            payment_id: dispute_object.payment_id,
            status: dispute_object.dispute_status,
        })
    } else {
        metrics::INCOMING_DISPUTE_WEBHOOK_SIGNATURE_FAILURE_METRIC.add(1, &[]);
        Err(report!(
            errors::ApiErrorResponse::WebhookAuthenticationFailed
        ))
    }
}

#[instrument(skip_all)]
async fn bank_transfer_webhook_flow(
    state: SessionState,
    req_state: ReqState,
    merchant_context: domain::MerchantContext,
    business_profile: domain::Profile,
    webhook_details: api::IncomingWebhookDetails,
    source_verified: bool,
) -> CustomResult<WebhookResponseTracker, errors::ApiErrorResponse> {
    let response = if source_verified {
        let payment_attempt = get_payment_attempt_from_object_reference_id(
            &state,
            webhook_details.object_reference_id,
            &merchant_context,
        )
        .await?;
        let payment_id = payment_attempt.payment_id;
        let request = api::PaymentsRequest {
            payment_id: Some(api_models::payments::PaymentIdType::PaymentIntentId(
                payment_id,
            )),
            payment_token: payment_attempt.payment_token,
            ..Default::default()
        };
        Box::pin(payments::payments_core::<
            api::Authorize,
            api::PaymentsResponse,
            _,
            _,
            _,
            payments::PaymentData<api::Authorize>,
        >(
            state.clone(),
            req_state,
            merchant_context.to_owned(),
            None,
            payments::PaymentConfirm,
            request,
            services::api::AuthFlow::Merchant,
            payments::CallConnectorAction::Trigger,
            None,
            HeaderPayload::with_source(common_enums::PaymentSource::Webhook),
        ))
        .await
    } else {
        Err(report!(
            errors::ApiErrorResponse::WebhookAuthenticationFailed
        ))
    };

    match response? {
        services::ApplicationResponse::JsonWithHeaders((payments_response, _)) => {
            let payment_id = payments_response.payment_id.clone();

            let event_type: Option<enums::EventType> = payments_response.status.into();
            let status = payments_response.status;

            // If event is NOT an UnsupportedEvent, trigger Outgoing Webhook
            if let Some(outgoing_event_type) = event_type {
                let primary_object_created_at = payments_response.created;
                Box::pin(super::create_event_and_trigger_outgoing_webhook(
                    state,
                    merchant_context,
                    business_profile,
                    outgoing_event_type,
                    enums::EventClass::Payments,
                    payment_id.get_string_repr().to_owned(),
                    enums::EventObjectType::PaymentDetails,
                    api::OutgoingWebhookContent::PaymentDetails(Box::new(payments_response)),
                    primary_object_created_at,
                ))
                .await?;
            }

            Ok(WebhookResponseTracker::Payment { payment_id, status })
        }

        _ => Err(errors::ApiErrorResponse::WebhookProcessingFailure)
            .attach_printable("received non-json response from payments core")?,
    }
}

async fn get_payment_id(
    db: &dyn StorageInterface,
    payment_id: &api::PaymentIdType,
    merchant_id: &common_utils::id_type::MerchantId,
    storage_scheme: enums::MerchantStorageScheme,
) -> errors::RouterResult<common_utils::id_type::PaymentId> {
    let pay_id = || async {
        match payment_id {
            api_models::payments::PaymentIdType::PaymentIntentId(ref id) => Ok(id.to_owned()),
            api_models::payments::PaymentIdType::ConnectorTransactionId(ref id) => db
                .find_payment_attempt_by_merchant_id_connector_txn_id(
                    merchant_id,
                    id,
                    storage_scheme,
                )
                .await
                .map(|p| p.payment_id),
            api_models::payments::PaymentIdType::PaymentAttemptId(ref id) => db
                .find_payment_attempt_by_attempt_id_merchant_id(id, merchant_id, storage_scheme)
                .await
                .map(|p| p.payment_id),
            api_models::payments::PaymentIdType::PreprocessingId(ref id) => db
                .find_payment_attempt_by_preprocessing_id_merchant_id(
                    id,
                    merchant_id,
                    storage_scheme,
                )
                .await
                .map(|p| p.payment_id),
        }
    };

    pay_id()
        .await
        .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)
}

#[inline]
async fn verify_webhook_source_verification_call(
    connector: ConnectorEnum,
    state: &SessionState,
    merchant_context: &domain::MerchantContext,
    merchant_connector_account: domain::MerchantConnectorAccount,
    connector_name: &str,
    request_details: &IncomingWebhookRequestDetails<'_>,
) -> CustomResult<bool, errors::ConnectorError> {
    let connector_data = ConnectorData::get_connector_by_name(
        &state.conf.connectors,
        connector_name,
        GetToken::Connector,
        None,
    )
    .change_context(errors::ConnectorError::WebhookSourceVerificationFailed)
    .attach_printable("invalid connector name received in payment attempt")?;
    let connector_integration: services::BoxedWebhookSourceVerificationConnectorIntegrationInterface<
        hyperswitch_domain_models::router_flow_types::VerifyWebhookSource,
        VerifyWebhookSourceRequestData,
        VerifyWebhookSourceResponseData,
    > = connector_data.connector.get_connector_integration();
    let connector_webhook_secrets = connector
        .get_webhook_source_verification_merchant_secret(
            merchant_context.get_merchant_account().get_id(),
            connector_name,
            merchant_connector_account.connector_webhook_details.clone(),
        )
        .await
        .change_context(errors::ConnectorError::WebhookSourceVerificationFailed)?;

    let router_data = construct_webhook_router_data(
        state,
        connector_name,
        merchant_connector_account,
        merchant_context,
        &connector_webhook_secrets,
        request_details,
    )
    .await
    .change_context(errors::ConnectorError::WebhookSourceVerificationFailed)
    .attach_printable("Failed while constructing webhook router data")?;

    let response = services::execute_connector_processing_step(
        state,
        connector_integration,
        &router_data,
        payments::CallConnectorAction::Trigger,
        None,
        None,
    )
    .await?;

    let verification_result = response
        .response
        .map(|response| response.verify_webhook_status);
    match verification_result {
        Ok(VerifyWebhookStatus::SourceVerified) => Ok(true),
        _ => Ok(false),
    }
}

pub fn get_connector_by_connector_name(
    state: &SessionState,
    connector_name: &str,
    merchant_connector_id: Option<common_utils::id_type::MerchantConnectorAccountId>,
) -> CustomResult<(ConnectorEnum, String), errors::ApiErrorResponse> {
    let authentication_connector =
        api_models::enums::convert_authentication_connector(connector_name);
    #[cfg(feature = "frm")]
    {
        let frm_connector = api_models::enums::convert_frm_connector(connector_name);
        if frm_connector.is_some() {
            let frm_connector_data =
                api::FraudCheckConnectorData::get_connector_by_name(connector_name)?;
            return Ok((
                frm_connector_data.connector,
                frm_connector_data.connector_name.to_string(),
            ));
        }
    }

    let (connector, connector_name) = if authentication_connector.is_some() {
        let authentication_connector_data =
            api::AuthenticationConnectorData::get_connector_by_name(connector_name)?;
        (
            authentication_connector_data.connector,
            authentication_connector_data.connector_name.to_string(),
        )
    } else {
        let connector_data = ConnectorData::get_connector_by_name(
            &state.conf.connectors,
            connector_name,
            GetToken::Connector,
            merchant_connector_id,
        )
        .change_context(errors::ApiErrorResponse::InvalidRequestData {
            message: "invalid connector name received".to_string(),
        })
        .attach_printable("Failed construction of ConnectorData")?;
        (
            connector_data.connector,
            connector_data.connector_name.to_string(),
        )
    };
    Ok((connector, connector_name))
}

/// This function fetches the merchant connector account ( if the url used is /{merchant_connector_id})
/// if merchant connector id is not passed in the request, then this will return None for mca
async fn fetch_optional_mca_and_connector(
    state: &SessionState,
    merchant_context: &domain::MerchantContext,
    connector_name_or_mca_id: &str,
) -> CustomResult<
    (
        Option<domain::MerchantConnectorAccount>,
        ConnectorEnum,
        String,
    ),
    errors::ApiErrorResponse,
> {
    let db = &state.store;
    if connector_name_or_mca_id.starts_with("mca_") {
        #[cfg(feature = "v1")]
        let mca = db
            .find_by_merchant_connector_account_merchant_id_merchant_connector_id(
                &state.into(),
                merchant_context.get_merchant_account().get_id(),
                &common_utils::id_type::MerchantConnectorAccountId::wrap(
                    connector_name_or_mca_id.to_owned(),
                )
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable(
                    "Error while converting MerchanConnectorAccountId from string
                    ",
                )?,
                merchant_context.get_merchant_key_store(),
            )
            .await
            .to_not_found_response(errors::ApiErrorResponse::MerchantConnectorAccountNotFound {
                id: connector_name_or_mca_id.to_string(),
            })
            .attach_printable(
                "error while fetching merchant_connector_account from connector_id",
            )?;

        #[cfg(feature = "v2")]
        let mca: domain::MerchantConnectorAccount = {
            let _ = merchant_account;
            let _ = key_store;
            let _ = db;
            todo!()
        };

        let (connector, connector_name) =
            get_connector_by_connector_name(state, &mca.connector_name, Some(mca.get_id()))?;

        Ok((Some(mca), connector, connector_name))
    } else {
        // Merchant connector account is already being queried, it is safe to set connector id as None
        let (connector, connector_name) =
            get_connector_by_connector_name(state, connector_name_or_mca_id, None)?;
        Ok((None, connector, connector_name))
    }
}

fn should_update_connector_mandate_details(
    source_verified: bool,
    event_type: webhooks::IncomingWebhookEvent,
) -> bool {
    source_verified && event_type == webhooks::IncomingWebhookEvent::PaymentIntentSuccess
}

async fn update_connector_mandate_details(
    state: &SessionState,
    merchant_context: &domain::MerchantContext,
    object_ref_id: api::ObjectReferenceId,
    connector: &ConnectorEnum,
    request_details: &IncomingWebhookRequestDetails<'_>,
) -> CustomResult<(), errors::ApiErrorResponse> {
    let webhook_connector_mandate_details = connector
        .get_mandate_details(request_details)
        .switch()
        .attach_printable("Could not find connector mandate details in incoming webhook body")?;

    let webhook_connector_network_transaction_id = connector
        .get_network_txn_id(request_details)
        .switch()
        .attach_printable(
            "Could not find connector network transaction id in incoming webhook body",
        )?;

    // Either one OR both of the fields are present
    if webhook_connector_mandate_details.is_some()
        || webhook_connector_network_transaction_id.is_some()
    {
        let payment_attempt =
            get_payment_attempt_from_object_reference_id(state, object_ref_id, merchant_context)
                .await?;
        if let Some(ref payment_method_id) = payment_attempt.payment_method_id {
            let key_manager_state = &state.into();
            let payment_method_info = state
                .store
                .find_payment_method(
                    key_manager_state,
                    merchant_context.get_merchant_key_store(),
                    payment_method_id,
                    merchant_context.get_merchant_account().storage_scheme,
                )
                .await
                .to_not_found_response(errors::ApiErrorResponse::PaymentMethodNotFound)?;

            // Update connector's mandate details
            let updated_connector_mandate_details =
                if let Some(webhook_mandate_details) = webhook_connector_mandate_details {
                    let mandate_details = payment_method_info
                        .get_common_mandate_reference()
                        .change_context(errors::ApiErrorResponse::InternalServerError)
                        .attach_printable("Failed to deserialize to Payment Mandate Reference")?;

                    let merchant_connector_account_id = payment_attempt
                        .merchant_connector_id
                        .clone()
                        .get_required_value("merchant_connector_id")?;

                    if mandate_details.payments.as_ref().map_or(true, |payments| {
                        !payments.0.contains_key(&merchant_connector_account_id)
                    }) {
                        // Update the payment attempt to maintain consistency across tables.
                        let (mandate_metadata, connector_mandate_request_reference_id) =
                            payment_attempt
                                .connector_mandate_detail
                                .as_ref()
                                .map(|details| {
                                    (
                                        details.mandate_metadata.clone(),
                                        details.connector_mandate_request_reference_id.clone(),
                                    )
                                })
                                .unwrap_or((None, None));

                        let connector_mandate_reference_id = ConnectorMandateReferenceId {
                            connector_mandate_id: Some(
                                webhook_mandate_details
                                    .connector_mandate_id
                                    .peek()
                                    .to_string(),
                            ),
                            payment_method_id: Some(payment_method_id.to_string()),
                            mandate_metadata,
                            connector_mandate_request_reference_id,
                        };

                        let attempt_update =
                            storage::PaymentAttemptUpdate::ConnectorMandateDetailUpdate {
                                connector_mandate_detail: Some(connector_mandate_reference_id),
                                updated_by: merchant_context
                                    .get_merchant_account()
                                    .storage_scheme
                                    .to_string(),
                            };

                        state
                            .store
                            .update_payment_attempt_with_attempt_id(
                                payment_attempt.clone(),
                                attempt_update,
                                merchant_context.get_merchant_account().storage_scheme,
                            )
                            .await
                            .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)?;

                        insert_mandate_details(
                            &payment_attempt,
                            &webhook_mandate_details,
                            Some(mandate_details),
                        )?
                    } else {
                        logger::info!(
                        "Skipping connector mandate details update since they are already present."
                    );
                        None
                    }
                } else {
                    None
                };

            let connector_mandate_details_value = updated_connector_mandate_details
                .map(|common_mandate| {
                    common_mandate.get_mandate_details_value().map_err(|err| {
                        router_env::logger::error!(
                            "Failed to get get_mandate_details_value : {:?}",
                            err
                        );
                        errors::ApiErrorResponse::MandateUpdateFailed
                    })
                })
                .transpose()?;

            let pm_update = diesel_models::PaymentMethodUpdate::ConnectorNetworkTransactionIdAndMandateDetailsUpdate {
                connector_mandate_details: connector_mandate_details_value.map(masking::Secret::new),
                network_transaction_id: webhook_connector_network_transaction_id
                    .map(|webhook_network_transaction_id| webhook_network_transaction_id.get_id().clone()),
            };

            state
                .store
                .update_payment_method(
                    key_manager_state,
                    merchant_context.get_merchant_key_store(),
                    payment_method_info,
                    pm_update,
                    merchant_context.get_merchant_account().storage_scheme,
                )
                .await
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Failed to update payment method in db")?;
        }
    }
    Ok(())
}

fn insert_mandate_details(
    payment_attempt: &PaymentAttempt,
    webhook_mandate_details: &hyperswitch_domain_models::router_flow_types::ConnectorMandateDetails,
    payment_method_mandate_details: Option<CommonMandateReference>,
) -> CustomResult<Option<CommonMandateReference>, errors::ApiErrorResponse> {
    let (mandate_metadata, connector_mandate_request_reference_id) = payment_attempt
        .connector_mandate_detail
        .clone()
        .map(|mandate_reference| {
            (
                mandate_reference.mandate_metadata,
                mandate_reference.connector_mandate_request_reference_id,
            )
        })
        .unwrap_or((None, None));
    let connector_mandate_details = tokenization::update_connector_mandate_details(
        payment_method_mandate_details,
        payment_attempt.payment_method_type,
        Some(
            payment_attempt
                .net_amount
                .get_total_amount()
                .get_amount_as_i64(),
        ),
        payment_attempt.currency,
        payment_attempt.merchant_connector_id.clone(),
        Some(
            webhook_mandate_details
                .connector_mandate_id
                .peek()
                .to_string(),
        ),
        mandate_metadata,
        connector_mandate_request_reference_id,
    )?;
    Ok(connector_mandate_details)
}
