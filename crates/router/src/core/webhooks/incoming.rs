use std::{str::FromStr, time::Instant};

use actix_web::FromRequest;
#[cfg(feature = "payouts")]
use api_models::payouts as payout_models;
use api_models::{
    enums::Connector,
    webhooks::{self, WebhookResponseTracker},
};
pub use common_enums::{connector_enums::InvoiceStatus, enums::ProcessTrackerRunner};
use common_utils::{
    errors::ReportSwitchExt,
    events::ApiEventsType,
    ext_traits::{AsyncExt, ByteSliceExt},
    types::{AmountConvertor, StringMinorUnitForConnector},
};
use diesel_models::{refund as diesel_refund, ConnectorMandateReferenceId};
use error_stack::{report, ResultExt};
#[cfg(feature = "payouts")]
use hyperswitch_domain_models::payouts::payout_attempt::PayoutAttempt;
use hyperswitch_domain_models::{
    mandates::CommonMandateReference,
    payments::{payment_attempt::PaymentAttempt, HeaderPayload},
    router_request_types::VerifyWebhookSourceRequestData,
    router_response_types::{VerifyWebhookSourceResponseData, VerifyWebhookStatus},
};
use hyperswitch_interfaces::webhooks::{IncomingWebhookFlowError, IncomingWebhookRequestDetails};
use masking::{ExposeInterface, PeekInterface};
use router_env::{instrument, tracing, RequestId};
use unified_connector_service_client::payments as payments_grpc;

use super::{types, utils, MERCHANT_ID};
use crate::{
    consts,
    core::{
        api_locking,
        errors::{self, ConnectorErrorExt, CustomResult, RouterResponse, StorageErrorExt},
        metrics, payment_methods,
        payment_methods::cards,
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
    platform: domain::Platform,
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
            platform.clone(),
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
        merchant_id: platform.get_processor().get_account().get_id().clone(),
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
        Some(platform.get_processor().get_account().get_id().clone()),
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
    platform: domain::Platform,
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
            platform.get_processor().get_account().get_id().clone()
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
        fetch_optional_mca_and_connector(&state, &platform, connector_name_or_mca_id).await?;

    // Determine webhook processing path (Direct vs UCS vs Shadow UCS) and handle event type extraction
    let execution_path =
        unified_connector_service::should_call_unified_connector_service_for_webhooks(
            &state,
            &platform,
            &connector_name,
        )
        .await?;

    let webhook_processing_result = match execution_path {
        common_enums::ExecutionPath::UnifiedConnectorService => {
            logger::info!(
                connector = connector_name,
                "Using Unified Connector Service for webhook processing",
            );
            process_ucs_webhook_transform(
                &state,
                &platform,
                &connector_name,
                &body,
                &request_details,
                merchant_connector_account.as_ref(),
            )
            .await
        }
        common_enums::ExecutionPath::ShadowUnifiedConnectorService => {
            logger::info!(
                connector = connector_name,
                "Using Shadow Unified Connector Service for webhook processing",
            );
            process_shadow_ucs_webhook_transform(
                &state,
                &platform,
                &connector,
                &connector_name,
                &body,
                &request_details,
                merchant_connector_account.as_ref(),
            )
            .await
        }
        common_enums::ExecutionPath::Direct => {
            logger::info!(
                connector = connector_name,
                "Using Direct connector processing for webhook",
            );
            // DIRECT PATH: Need to decode body first
            let decoded_body = connector
                .decode_webhook_body(
                    &request_details,
                    platform.get_processor().get_account().get_id(),
                    merchant_connector_account
                        .clone()
                        .and_then(|mca| mca.connector_webhook_details.clone()),
                    &connector_name,
                )
                .await
                .switch()
                .attach_printable("There was an error in incoming webhook body decoding")?;

            process_non_ucs_webhook(
                &state,
                &platform,
                &connector,
                &connector_name,
                decoded_body.into(),
                &request_details,
            )
            .await
        }
    };

    let mut webhook_processing_result = match webhook_processing_result {
        Ok(result) => result,
        Err(error) => {
            let error_result = handle_incoming_webhook_error(
                error,
                &connector,
                connector_name.as_str(),
                &request_details,
                platform.get_processor().get_account().get_id(),
            );
            match error_result {
                Ok((response, webhook_tracker, serialized_request)) => {
                    return Ok((response, webhook_tracker, serialized_request));
                }
                Err(e) => return Err(e),
            }
        }
    };

    // if it is a setup webhook event, return ok status
    if webhook_processing_result.event_type == webhooks::IncomingWebhookEvent::SetupWebhook {
        return Ok((
            services::ApplicationResponse::StatusOk,
            WebhookResponseTracker::NoEffect,
            serde_json::Value::default(),
        ));
    }

    // Update request_details with the appropriate body (decoded for non-UCS, raw for UCS)
    let final_request_details = match &webhook_processing_result.decoded_body {
        Some(decoded_body) => &IncomingWebhookRequestDetails {
            method: request_details.method.clone(),
            uri: request_details.uri.clone(),
            headers: request_details.headers,
            query_params: request_details.query_params.clone(),
            body: decoded_body,
        },
        None => &request_details, // Use original request details for UCS
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
        platform.get_processor().get_account().get_id(),
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
                &platform,
                &connector,
                &connector_name,
                webhook_processing_result.event_type,
                webhook_processing_result.source_verified,
                &webhook_processing_result.transform_data,
                &mut webhook_processing_result.shadow_ucs_data,
                final_request_details,
                is_relay_webhook,
            )
            .await;

            match business_logic_result {
                Ok(response) => {
                    // Extract event object for serialization
                    event_object = extract_webhook_event_object(
                        &webhook_processing_result.transform_data,
                        &connector,
                        final_request_details,
                    )?;
                    response
                }
                Err(error) => {
                    let error_result = handle_incoming_webhook_error(
                        error,
                        &connector,
                        connector_name.as_str(),
                        final_request_details,
                        platform.get_processor().get_account().get_id(),
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
                    platform.get_processor().get_account().get_id().clone()
                )),
            );
            WebhookResponseTracker::NoEffect
        }
    };

    // Generate response
    let response = connector
        .get_webhook_api_response(final_request_details, None)
        .switch()
        .attach_printable("Could not get incoming webhook api response from connector")?;

    let serialized_request = event_object
        .masked_serialize()
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Could not convert webhook effect to string")?;

    Ok((response, webhook_effect, serialized_request))
}

/// Process UCS webhook transformation using the high-level UCS abstraction
async fn process_ucs_webhook_transform<'a>(
    state: &'a SessionState,
    platform: &'a domain::Platform,
    connector_name: &'a str,
    body: &'a actix_web::web::Bytes,
    request_details: &'a IncomingWebhookRequestDetails<'a>,
    merchant_connector_account: Option<&'a domain::MerchantConnectorAccount>,
) -> errors::RouterResult<WebhookProcessingResult<'a>> {
    // Use the new UCS abstraction which provides clean separation
    let (event_type, source_verified, transform_data) =
        unified_connector_service::call_unified_connector_service_for_webhook(
            state,
            platform,
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
        shadow_ucs_data: None,
    })
}

/// Shadow UCS data container
#[allow(dead_code)]
pub struct ShadowUcsData<'a> {
    ucs_source_verified: bool,
    ucs_event_type: webhooks::IncomingWebhookEvent,
    ucs_transform_data: Box<unified_connector_service::WebhookTransformData>,
    request_details: &'a IncomingWebhookRequestDetails<'a>,
    webhook_details: Option<api::IncomingWebhookDetails>,
}

/// Result type for webhook processing path determination
pub struct WebhookProcessingResult<'a> {
    event_type: webhooks::IncomingWebhookEvent,
    source_verified: bool,
    transform_data: Option<Box<unified_connector_service::WebhookTransformData>>,
    decoded_body: Option<actix_web::web::Bytes>,
    shadow_ucs_data: Option<ShadowUcsData<'a>>,
}

/// Process shadow UCS webhook transformation with dual execution (UCS + Direct)
async fn process_shadow_ucs_webhook_transform<'a>(
    state: &'a SessionState,
    platform: &'a domain::Platform,
    connector: &'a ConnectorEnum,
    connector_name: &'a str,
    body: &'a actix_web::web::Bytes,
    request_details: &'a IncomingWebhookRequestDetails<'a>,
    merchant_connector_account: Option<&'a domain::MerchantConnectorAccount>,
) -> errors::RouterResult<WebhookProcessingResult<'a>> {
    // Execute UCS path
    let ucs_result = unified_connector_service::call_unified_connector_service_for_webhook(
        state,
        platform,
        connector_name,
        body,
        request_details,
        merchant_connector_account,
    )
    .await;

    // Execute Direct path for comparison
    let direct_result = async {
        // Decode body for direct processing
        let decoded_body = connector
            .decode_webhook_body(
                request_details,
                platform.get_processor().get_account().get_id(),
                merchant_connector_account.and_then(|mca| mca.connector_webhook_details.clone()),
                connector_name,
            )
            .await
            .switch()
            .attach_printable(
                "There was an error in incoming webhook body decoding for shadow processing",
            )?;

        // Reuse existing process_non_ucs_webhook function
        process_non_ucs_webhook(
            state,
            platform,
            connector,
            connector_name,
            decoded_body.into(),
            request_details,
        )
        .await
    }
    .await;

    // Handle results and comparison
    match (ucs_result, direct_result) {
        (
            Ok((ucs_event_type, ucs_source_verified, ucs_transform_data)),
            Ok(mut direct_processing_result),
        ) => {
            // Log both calls succeeded
            logger::info!(
                connector = connector_name,
                merchant_id = ?platform.get_processor().get_account().get_id(),
                "Shadow UCS: Both UCS and Direct calls succeeded"
            );

            // Create ShadowUcsData with UCS results
            let shadow_ucs_data = ShadowUcsData {
                ucs_source_verified,
                ucs_event_type,
                ucs_transform_data: Box::new(ucs_transform_data),
                request_details,
                webhook_details: None,
            };

            // Return Direct result as primary with UCS data in shadow_ucs_data
            direct_processing_result.shadow_ucs_data = Some(shadow_ucs_data);
            Ok(direct_processing_result)
        }
        (Ok((_, _, _)), Err(direct_error)) => {
            // Log UCS call succeeded, direct call failed
            logger::info!(
                connector = connector_name,
                merchant_id = ?platform.get_processor().get_account().get_id(),
                direct_error = ?direct_error,
                "Shadow UCS: UCS call succeeded, Direct call failed"
            );

            // Return direct_result error as required
            Err(direct_error)
        }
        (Err(ucs_error), Ok(direct_processing_result)) => {
            // Log the UCS error and direct result succeeded
            logger::info!(
                connector = connector_name,
                merchant_id = ?platform.get_processor().get_account().get_id(),
                ucs_error = ?ucs_error,
                "Shadow UCS: UCS call failed, Direct call succeeded"
            );

            // In shadow mode, if UCS fails, fall back to Direct result
            Ok(direct_processing_result)
        }
        (Err(ucs_error), Err(direct_error)) => {
            // Log both the errors and both call failed
            logger::error!(
                connector = connector_name,
                merchant_id = ?platform.get_processor().get_account().get_id(),
                ucs_error = ?ucs_error,
                direct_error = ?direct_error,
                "Shadow UCS: Both UCS and Direct calls failed"
            );

            // Return direct_result error as required
            Err(direct_error)
        }
    }
}

/// Process non-UCS webhook using traditional connector processing
async fn process_non_ucs_webhook<'a>(
    state: &'a SessionState,
    platform: &'a domain::Platform,
    connector: &'a ConnectorEnum,
    connector_name: &'a str,
    decoded_body: actix_web::web::Bytes,
    request_details: &'a IncomingWebhookRequestDetails<'a>,
) -> errors::RouterResult<WebhookProcessingResult<'a>> {
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
            shadow_ucs_data: None,
        }),
        None => {
            metrics::WEBHOOK_EVENT_TYPE_IDENTIFICATION_FAILURE_COUNT.add(
                1,
                router_env::metric_attributes!(
                    (
                        MERCHANT_ID,
                        platform.get_processor().get_account().get_id().clone()
                    ),
                    ("connector", connector_name.to_string())
                ),
            );
            Err(errors::ApiErrorResponse::WebhookProcessingFailure)
                .attach_printable("Failed to identify event type in incoming webhook body")
        }
    }
}

/// Extract resource object from UCS WebhookResponseContent
fn get_ucs_webhook_resource_object(
    webhook_response_content: &payments_grpc::WebhookResponseContent,
) -> errors::RouterResult<Box<dyn masking::ErasedMaskSerialize>> {
    let resource_object = match &webhook_response_content.content {
        Some(payments_grpc::webhook_response_content::Content::IncompleteTransformation(
            incomplete_transformation_response,
        )) => {
            // Deserialize resource object
            serde_json::from_slice::<serde_json::Value>(
                &incomplete_transformation_response.resource_object,
            )
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Failed to deserialize resource object from UCS webhook response")?
        }
        _ => {
            // Convert UCS webhook content to appropriate format
            serde_json::to_value(webhook_response_content)
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Failed to serialize UCS webhook content")?
        }
    };
    Ok(Box::new(resource_object))
}

/// Extract webhook event object based on transform data availability
fn extract_webhook_event_object(
    webhook_transform_data: &Option<Box<unified_connector_service::WebhookTransformData>>,
    connector: &ConnectorEnum,
    request_details: &IncomingWebhookRequestDetails<'_>,
) -> errors::RouterResult<Box<dyn masking::ErasedMaskSerialize>> {
    match webhook_transform_data {
        Some(webhook_transform_data) => webhook_transform_data
            .webhook_content
            .as_ref()
            .map(|webhook_response_content| {
                get_ucs_webhook_resource_object(webhook_response_content)
            })
            .unwrap_or_else(|| {
                connector
                    .get_webhook_resource_object(request_details)
                    .switch()
                    .attach_printable("Could not find resource object in incoming webhook body")
            }),
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
    platform: &domain::Platform,
    connector: &ConnectorEnum,
    connector_name: &str,
    event_type: webhooks::IncomingWebhookEvent,
    source_verified_via_ucs: bool,
    webhook_transform_data: &Option<Box<unified_connector_service::WebhookTransformData>>,
    shadow_ucs_data: &mut Option<ShadowUcsData<'_>>,
    request_details: &IncomingWebhookRequestDetails<'_>,
    is_relay_webhook: bool,
) -> errors::RouterResult<WebhookResponseTracker> {
    let object_ref_id = connector
        .get_webhook_object_reference_id(request_details)
        .switch()
        .attach_printable("Could not find object reference id in incoming webhook body")?;
    let connector_enum = Connector::from_str(connector_name)
        .change_context(errors::ApiErrorResponse::InvalidDataValue {
            field_name: "connector",
        })
        .attach_printable_lazy(|| format!("unable to parse connector name {connector_name:?}"))?;
    let connectors_with_source_verification_call = &state.conf.webhook_source_verification_call;

    let merchant_connector_account = match Box::pin(helper_utils::get_mca_from_object_reference_id(
        state,
        object_ref_id.clone(),
        platform,
        connector_name,
    ))
    .await
    {
        Ok(mca) => mca,
        Err(error) => {
            let result = handle_incoming_webhook_error(
                error,
                connector,
                connector_name,
                request_details,
                platform.get_processor().get_account().get_id(),
            );
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
                platform,
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
                    platform.get_processor().get_account().get_id(),
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
                platform.get_processor().get_account().get_id().clone()
            )),
        );
    } else if connector.is_webhook_source_verification_mandatory() {
        // if webhook consumption is mandatory for connector, fail webhook
        // so that merchant can retrigger it after updating merchant_secret
        return Err(errors::ApiErrorResponse::WebhookAuthenticationFailed.into());
    }

    logger::info!(source_verified=?source_verified);

    let event_object: Box<dyn masking::ErasedMaskSerialize> = match webhook_transform_data {
        Some(webhook_transform_data) => {
            // Extract resource_object from UCS webhook content
            webhook_transform_data
                .webhook_content
                .as_ref()
                .map(|webhook_response_content| {
                    get_ucs_webhook_resource_object(webhook_response_content)
                })
                .unwrap_or_else(|| {
                    // Fall back to connector extraction
                    connector
                        .get_webhook_resource_object(request_details)
                        .switch()
                        .attach_printable("Could not find resource object in incoming webhook body")
                })?
        }
        None => {
            // Use traditional connector extraction
            connector
                .get_webhook_resource_object(request_details)
                .switch()
                .attach_printable("Could not find resource object in incoming webhook body")?
        }
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

    // Create shadow_event_object and shadow_webhook_details using shadow UCS data
    let shadow_event_object: Option<Box<dyn masking::ErasedMaskSerialize>> =
        shadow_ucs_data.as_ref().and_then(|shadow_data| {
            // Create shadow event object using UCS transform data and shadow request details
            let shadow_event_result = shadow_data
                .ucs_transform_data
                .webhook_content
                .as_ref()
                .map(|webhook_response_content| {
                    get_ucs_webhook_resource_object(webhook_response_content)
                })
                .unwrap_or_else(|| {
                    // Fall back to connector extraction using shadow request details
                    connector
                        .get_webhook_resource_object(shadow_data.request_details)
                        .switch()
                        .attach_printable(
                            "Could not find resource object in shadow incoming webhook body",
                        )
                });

            match shadow_event_result {
                Ok(shadow_obj) => Some(shadow_obj),
                Err(error) => {
                    logger::warn!(
                        connector = connector_name,
                        merchant_id = ?platform.get_processor().get_account().get_id(),
                        error = ?error,
                        "Failed to create shadow event object from UCS transform data"
                    );
                    None
                }
            }
        });

    let shadow_webhook_details: Option<api::IncomingWebhookDetails> =
        shadow_event_object.as_ref().and_then(|shadow_obj| {
            let resource_object_result = serde_json::to_vec(shadow_obj)
                .change_context(errors::ParsingError::EncodeError("byte-vec"))
                .attach_printable("Unable to convert shadow webhook payload to a value")
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable(
                    "There was an issue when encoding the shadow incoming webhook body to bytes",
                );

            match resource_object_result {
                Ok(resource_object) => Some(api::IncomingWebhookDetails {
                    object_reference_id: object_ref_id.clone(),
                    resource_object,
                }),
                Err(error) => {
                    logger::warn!(
                        connector = connector_name,
                        merchant_id = ?platform.get_processor().get_account().get_id(),
                        error = ?error,
                        "Failed to serialize shadow webhook payload to bytes"
                    );
                    None
                }
            }
        });

    // Assign shadow_webhook_details to shadow_ucs_data
    if let Some(shadow_data) = shadow_ucs_data.as_mut() {
        shadow_data.webhook_details = shadow_webhook_details;
    }

    let profile_id = &merchant_connector_account.profile_id;

    let business_profile = state
        .store
        .find_business_profile_by_profile_id(platform.get_processor().get_key_store(), profile_id)
        .await
        .to_not_found_response(errors::ApiErrorResponse::ProfileNotFound {
            id: profile_id.get_string_repr().to_owned(),
        })?;

    // If the incoming webhook is a relay webhook, then we need to trigger the relay webhook flow
    let result_response = if is_relay_webhook {
        let relay_webhook_response = Box::pin(relay_incoming_webhook_flow(
            state.clone(),
            platform.clone(),
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
                platform.clone(),
                business_profile,
                webhook_details,
                source_verified,
                connector,
                request_details,
                event_type,
                webhook_transform_data,
                shadow_ucs_data,
            ))
            .await
            .attach_printable("Incoming webhook flow for payments failed"),

            api::WebhookFlow::Refund => Box::pin(refunds_incoming_webhook_flow(
                state.clone(),
                platform.clone(),
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
                platform.clone(),
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
                platform.clone(),
                business_profile,
                webhook_details,
                source_verified,
            ))
            .await
            .attach_printable("Incoming bank-transfer webhook flow failed"),

            api::WebhookFlow::ReturnResponse => Ok(WebhookResponseTracker::NoEffect),

            api::WebhookFlow::Mandate => Box::pin(mandates_incoming_webhook_flow(
                state.clone(),
                platform.clone(),
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
                    platform.clone(),
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
                platform.clone(),
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
                platform.clone(),
                business_profile,
                webhook_details,
                event_type,
                source_verified,
                request_details,
                connector,
            ))
            .await
            .attach_printable("Incoming webhook flow for payouts failed"),

            api::WebhookFlow::Subscription => {
                Box::pin(subscriptions::webhooks::incoming_webhook_flow(
                    state.clone().into(),
                    platform.clone(),
                    business_profile,
                    webhook_details,
                    source_verified,
                    connector,
                    request_details,
                    event_type,
                    merchant_connector_account,
                ))
                .await
                .attach_printable("Incoming webhook flow for subscription failed")
            }

            _ => Err(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Unsupported Flow Type received in incoming webhooks"),
        }
    };

    match result_response {
        Ok(response) => Ok(response),
        Err(error) => {
            let result = handle_incoming_webhook_error(
                error,
                connector,
                connector_name,
                request_details,
                platform.get_processor().get_account().get_id(),
            );
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
    merchant_id: &common_utils::id_type::MerchantId,
) -> errors::RouterResult<(
    services::ApplicationResponse<serde_json::Value>,
    WebhookResponseTracker,
    serde_json::Value,
)> {
    logger::error!(?error, "Incoming webhook flow failed");

    // fetch the connector enum from the connector name
    let connector_enum = Connector::from_str(connector_name)
        .change_context(errors::ApiErrorResponse::InvalidDataValue {
            field_name: "connector",
        })
        .attach_printable_lazy(|| format!("unable to parse connector name {connector_name:?}"))?;

    // get the error response from the connector
    if connector_enum.should_acknowledge_webhook_for_resource_not_found_errors() {
        metrics::WEBHOOK_FLOW_FAILED_BUT_ACKNOWLEDGED.add(
            1,
            router_env::metric_attributes!(
                ("connector", connector_name.to_string()),
                ("merchant_id", merchant_id.get_string_repr().to_string())
            ),
        );
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

    let platform =
        network_tokenization_incoming::fetch_merchant_account_for_network_token_webhooks(
            state,
            &merchant_id,
        )
        .await?;
    let payment_method =
        network_tokenization_incoming::fetch_payment_method_for_network_token_webhooks(
            state,
            platform.get_processor().get_account(),
            platform.get_processor().get_key_store(),
            &payment_method_id,
        )
        .await?;

    let response_data = response.get_response_data();

    let webhook_resp_tracker = response_data
        .update_payment_method(state, &payment_method, &platform)
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
    platform: domain::Platform,
    business_profile: domain::Profile,
    webhook_details: api::IncomingWebhookDetails,
    source_verified: bool,
    connector: &ConnectorEnum,
    request_details: &IncomingWebhookRequestDetails<'_>,
    event_type: webhooks::IncomingWebhookEvent,
    webhook_transform_data: &Option<Box<unified_connector_service::WebhookTransformData>>,
    shadow_ucs_data: &Option<ShadowUcsData<'_>>,
) -> CustomResult<WebhookResponseTracker, errors::ApiErrorResponse> {
    let consume_or_trigger_flow = if source_verified {
        // Determine the appropriate action based on UCS availability
        let resource_object = webhook_details.resource_object;

        match webhook_transform_data.as_ref() {
            Some(transform_data) => {
                match transform_data.webhook_transformation_status {
                    unified_connector_service::WebhookTransformationStatus::Complete => {
                        // Consume response from UCS
                        payments::CallConnectorAction::UCSConsumeResponse(resource_object)
                    }
                    unified_connector_service::WebhookTransformationStatus::Incomplete => {
                        // Make a second call to UCS
                        payments::CallConnectorAction::UCSHandleResponse(resource_object)
                    }
                }
            }
            None => payments::CallConnectorAction::HandleResponse(resource_object),
        }
    } else {
        payments::CallConnectorAction::Trigger
    };

    // Determine shadow UCS call connector action based on shadow UCS data
    let shadow_ucs_call_connector_action = shadow_ucs_data.as_ref().and_then(|shadow_data| {
        if shadow_data.ucs_source_verified {
            // Proceed with the match case of webhook_transformation_status
            shadow_data.webhook_details.as_ref().map(|webhook_details| {
                match shadow_data.ucs_transform_data.webhook_transformation_status {
                    unified_connector_service::WebhookTransformationStatus::Complete => {
                        payments::CallConnectorAction::UCSConsumeResponse(
                            webhook_details.resource_object.clone(),
                        )
                    }
                    unified_connector_service::WebhookTransformationStatus::Incomplete => {
                        payments::CallConnectorAction::UCSHandleResponse(
                            webhook_details.resource_object.clone(),
                        )
                    }
                }
            })
        } else {
            // If ucs_source_verified is not true, use Trigger action
            Some(payments::CallConnectorAction::Trigger)
        }
    });

    let payments_response = match webhook_details.object_reference_id {
        webhooks::ObjectReferenceId::PaymentId(ref id) => {
            let payment_id = get_payment_id(
                state.store.as_ref(),
                id,
                platform.get_processor().get_account().get_id(),
                platform.get_processor().get_account().storage_scheme,
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
                    platform.get_processor().get_account().get_id().to_owned(),
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
                platform.clone(),
                None,
                payments::operations::PaymentStatus,
                api::PaymentsRetrieveRequest {
                    resource_id: id.clone(),
                    merchant_id: Some(platform.get_processor().get_account().get_id().clone()),
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
                shadow_ucs_call_connector_action,
                None,
                HeaderPayload::default(),
            ))
            .await;
            // When mandate details are present in successful webhooks, and consuming webhooks are skipped during payment sync if the payment status is already updated to charged, this function is used to update the connector mandate details.
            if should_update_connector_mandate_details(source_verified, event_type) {
                update_connector_mandate_details(
                    &state,
                    &platform,
                    webhook_details.object_reference_id.clone(),
                    connector,
                    request_details,
                )
                .await?
            };

            lock_action
                .free_lock_action(
                    &state,
                    platform.get_processor().get_account().get_id().to_owned(),
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
                            platform.get_processor().get_account().get_id().clone()
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
            let payment_method_id_opt = payments_response.payment_method_id.clone();

            match payment_method_id_opt {
                Some(payment_method_id) => {
                    if let Err(e) = update_additional_payment_method_data(
                        &state,
                        &platform,
                        connector,
                        request_details,
                        payment_method_id,
                    )
                    .await
                    {
                        logger::warn!(?e, "Failed to update additional payment method data");
                    }
                }
                None => {
                    logger::warn!("Failed to find payment_method_id in payments response");
                }
            }

            let status = payments_response.status;

            let event_type: Option<enums::EventType> = payments_response.status.into();

            // If event is NOT an UnsupportedEvent, trigger Outgoing Webhook
            if let Some(outgoing_event_type) = event_type {
                let primary_object_created_at = payments_response.created;
                Box::pin(super::create_event_and_trigger_outgoing_webhook(
                    state,
                    platform,
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
#[allow(clippy::too_many_arguments)]
async fn payouts_incoming_webhook_flow(
    state: SessionState,
    platform: domain::Platform,
    business_profile: domain::Profile,
    webhook_details: api::IncomingWebhookDetails,
    event_type: webhooks::IncomingWebhookEvent,
    source_verified: bool,
    request_details: &IncomingWebhookRequestDetails<'_>,
    connector: &ConnectorEnum,
) -> CustomResult<WebhookResponseTracker, errors::ApiErrorResponse> {
    metrics::INCOMING_PAYOUT_WEBHOOK_METRIC.add(1, &[]);
    let db = &*state.store;
    //find payout_attempt by object_reference_id
    let payout_attempt = match webhook_details.object_reference_id {
        webhooks::ObjectReferenceId::PayoutId(payout_id_type) => match payout_id_type {
            webhooks::PayoutIdType::PayoutAttemptId(id) => db
                .find_payout_attempt_by_merchant_id_payout_attempt_id(
                    platform.get_processor().get_account().get_id(),
                    &id,
                    platform.get_processor().get_account().storage_scheme,
                )
                .await
                .change_context(errors::ApiErrorResponse::WebhookResourceNotFound)
                .attach_printable("Failed to fetch the payout attempt")?,
            webhooks::PayoutIdType::ConnectorPayoutId(id) => db
                .find_payout_attempt_by_merchant_id_connector_payout_id(
                    platform.get_processor().get_account().get_id(),
                    &id,
                    platform.get_processor().get_account().storage_scheme,
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
            platform.get_processor().get_account().get_id(),
            &payout_attempt.payout_id,
            platform.get_processor().get_account().storage_scheme,
        )
        .await
        .change_context(errors::ApiErrorResponse::WebhookResourceNotFound)
        .attach_printable("Failed to fetch the payout")?;
    let action_req =
        payout_models::PayoutRequest::PayoutActionRequest(payout_models::PayoutActionRequest {
            payout_id: payouts.payout_id.clone(),
        });

    let mut payout_data = Box::pin(payouts::make_payout_data(
        &state,
        &platform,
        None,
        &action_req,
        common_utils::consts::DEFAULT_LOCALE,
    ))
    .await?;

    let payout_webhook_action = get_payout_webhook_action(
        payout_attempt.status.is_non_terminal_status(),
        source_verified,
    );
    match payout_webhook_action {
        PaoyoutWebhookAction::UpdateStatus => {
            payout_incoming_webhook_update_status(
                state,
                platform,
                business_profile,
                event_type,
                request_details,
                connector,
                payout_attempt,
                &mut payout_data,
            )
            .await
        }
        PaoyoutWebhookAction::RetrieveStatus => {
            payout_incoming_webhook_retrieve_status(
                state,
                platform,
                payout_attempt,
                &mut payout_data,
            )
            .await
        }
        PaoyoutWebhookAction::NoAction => Ok(WebhookResponseTracker::Payout {
            payout_id: payout_data.payout_attempt.payout_id,
            status: payout_data.payout_attempt.status,
        }),
    }
}

enum PaoyoutWebhookAction {
    UpdateStatus,
    RetrieveStatus,
    NoAction,
}

// only update if the payout is in non-terminal status
// if source verified, update the payout attempt and trigger outgoing webhook
// if not source verified, do a payout retrieve call and update the status
fn get_payout_webhook_action(
    is_non_terminal_status: bool,
    is_source_verified: bool,
) -> PaoyoutWebhookAction {
    match (is_non_terminal_status, is_source_verified) {
        (true, true) => PaoyoutWebhookAction::UpdateStatus,
        (true, false) => PaoyoutWebhookAction::RetrieveStatus,
        (false, true) | (false, false) => PaoyoutWebhookAction::NoAction,
    }
}

#[cfg(feature = "payouts")]
#[instrument(skip_all)]
#[allow(clippy::too_many_arguments)]
async fn payout_incoming_webhook_update_status(
    state: SessionState,
    platform: domain::Platform,
    business_profile: domain::Profile,
    event_type: webhooks::IncomingWebhookEvent,
    request_details: &IncomingWebhookRequestDetails<'_>,
    connector: &ConnectorEnum,
    payout_attempt: PayoutAttempt,
    payout_data: &mut payouts::PayoutData,
) -> CustomResult<WebhookResponseTracker, errors::ApiErrorResponse> {
    let db = &*state.store;
    let status = common_enums::PayoutStatus::foreign_try_from(event_type)
        .change_context(errors::ApiErrorResponse::WebhookProcessingFailure)
        .attach_printable("failed payout status mapping from event type")?;

    let payout_webhook_details = connector
        .get_payout_webhook_details(request_details)
        .switch()
        .attach_printable("Failed to get error object for payouts")?;

    // if status is failure then update the error_code and error_message as well
    let payout_attempt_update = if status.is_payout_failure() {
        PayoutAttemptUpdate::StatusUpdate {
            connector_payout_id: payout_attempt.connector_payout_id.clone(),
            status,
            error_message: payout_webhook_details.error_message,
            error_code: payout_webhook_details.error_code,
            is_eligible: payout_attempt.is_eligible,
            unified_code: None,
            unified_message: None,
            payout_connector_metadata: payout_attempt.payout_connector_metadata.clone(),
        }
    } else {
        PayoutAttemptUpdate::StatusUpdate {
            connector_payout_id: payout_attempt.connector_payout_id.clone(),
            status,
            error_message: None,
            error_code: None,
            is_eligible: payout_attempt.is_eligible,
            unified_code: None,
            unified_message: None,
            payout_connector_metadata: payout_attempt.payout_connector_metadata.clone(),
        }
    };

    let updated_payout_attempt = db
        .update_payout_attempt(
            &payout_attempt,
            payout_attempt_update,
            &payout_data.payouts,
            platform.get_processor().get_account().storage_scheme,
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
            payouts::response_handler(&state, &platform, payout_data).await?;

        Box::pin(super::create_event_and_trigger_outgoing_webhook(
            state,
            platform,
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
        payout_id: payout_data.payout_attempt.payout_id.clone(),
        status: payout_data.payout_attempt.status,
    })
}

#[cfg(feature = "payouts")]
#[instrument(skip_all)]
#[allow(clippy::too_many_arguments)]
async fn payout_incoming_webhook_retrieve_status(
    state: SessionState,
    platform: domain::Platform,
    payout_attempt: PayoutAttempt,
    payout_data: &mut payouts::PayoutData,
) -> CustomResult<WebhookResponseTracker, errors::ApiErrorResponse> {
    metrics::INCOMING_PAYOUT_WEBHOOK_SIGNATURE_FAILURE_METRIC.add(1, &[]);
    // Form connector data
    let connector_data = match &payout_attempt.connector {
        Some(connector) => ConnectorData::get_payout_connector_by_name(
            &state.conf.connectors,
            connector,
            GetToken::Connector,
            payout_attempt.merchant_connector_id.clone(),
        )
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to get the connector data")?,
        _ => Err(errors::ApplicationError::InvalidConfigurationValueError(
            "Connector not found in payout_attempt - should not reach here.".to_string(),
        ))
        .change_context(errors::ApiErrorResponse::MissingRequiredField {
            field_name: "connector",
        })
        .attach_printable("Connector not found for payout fulfillment")?,
    };

    Box::pin(payouts::create_payout_retrieve(
        &state,
        &platform,
        &connector_data,
        payout_data,
    ))
    .await
    .attach_printable("Payout retrieval failed for given Payout request")?;

    Ok(WebhookResponseTracker::Payout {
        payout_id: payout_data.payout_attempt.payout_id.clone(),
        status: payout_data.payout_attempt.status,
    })
}

async fn relay_refunds_incoming_webhook_flow(
    state: SessionState,
    platform: domain::Platform,
    business_profile: domain::Profile,
    webhook_details: api::IncomingWebhookDetails,
    event_type: webhooks::IncomingWebhookEvent,
    source_verified: bool,
) -> CustomResult<WebhookResponseTracker, errors::ApiErrorResponse> {
    let db = &*state.store;

    let relay_record = match webhook_details.object_reference_id {
        webhooks::ObjectReferenceId::RefundId(refund_id_type) => match refund_id_type {
            webhooks::RefundIdType::RefundId(refund_id) => {
                let relay_id = common_utils::id_type::RelayId::from_str(&refund_id)
                    .change_context(errors::ValidationError::IncorrectValueProvided {
                        field_name: "relay_id",
                    })
                    .change_context(errors::ApiErrorResponse::InternalServerError)?;

                db.find_relay_by_id(platform.get_processor().get_key_store(), &relay_id)
                    .await
                    .to_not_found_response(errors::ApiErrorResponse::WebhookResourceNotFound)
                    .attach_printable("Failed to fetch the relay record")?
            }
            webhooks::RefundIdType::ConnectorRefundId(connector_refund_id) => db
                .find_relay_by_profile_id_connector_reference_id(
                    platform.get_processor().get_key_store(),
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
            platform.get_processor().get_key_store(),
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
            platform,
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
    platform: domain::Platform,
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
                    platform.get_processor().get_account().get_id(),
                    &id,
                    platform.get_processor().get_account().storage_scheme,
                )
                .await
                .change_context(errors::ApiErrorResponse::WebhookResourceNotFound)
                .attach_printable("Failed to fetch the refund")?,
            webhooks::RefundIdType::ConnectorRefundId(id) => db
                .find_refund_by_merchant_id_connector_refund_id_connector(
                    platform.get_processor().get_account().get_id(),
                    &id,
                    connector_name,
                    platform.get_processor().get_account().storage_scheme,
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
            updated_by: platform
                .get_processor()
                .get_account()
                .storage_scheme
                .to_string(),
            processor_refund_data: None,
        };
        db.update_refund(
            refund.to_owned(),
            refund_update,
            platform.get_processor().get_account().storage_scheme,
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::WebhookResourceNotFound)
        .attach_printable_lazy(|| format!("Failed while updating refund: refund_id: {refund_id}"))?
    } else {
        Box::pin(refunds::refund_retrieve_core_with_refund_id(
            state.clone(),
            platform.clone(),
            None,
            api_models::refunds::RefundsRetrieveRequest {
                refund_id: refund_id.to_owned(),
                force_sync: Some(true),
                merchant_connector_details: None,
                all_keys_required: None,
            },
        ))
        .await
        .attach_printable_lazy(|| format!("Failed while updating refund: refund_id: {refund_id}"))?
        .0
    };
    let event_type: Option<enums::EventType> = updated_refund.refund_status.into();

    // If event is NOT an UnsupportedEvent, trigger Outgoing Webhook
    if let Some(outgoing_event_type) = event_type {
        let refund_response: api_models::refunds::RefundResponse =
            updated_refund.clone().foreign_into();
        Box::pin(super::create_event_and_trigger_outgoing_webhook(
            state,
            platform,
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
    platform: domain::Platform,
    business_profile: domain::Profile,
    webhook_details: api::IncomingWebhookDetails,
    event_type: webhooks::IncomingWebhookEvent,
    source_verified: bool,
) -> CustomResult<WebhookResponseTracker, errors::ApiErrorResponse> {
    let flow_type: api::WebhookFlow = event_type.into();

    let result_response = match flow_type {
        webhooks::WebhookFlow::Refund => Box::pin(relay_refunds_incoming_webhook_flow(
            state,
            platform,
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
        | webhooks::WebhookFlow::Setup
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
    platform: &domain::Platform,
) -> CustomResult<PaymentAttempt, errors::ApiErrorResponse> {
    let db = &*state.store;
    match object_reference_id {
        api::ObjectReferenceId::PaymentId(api::PaymentIdType::ConnectorTransactionId(ref id)) => db
            .find_payment_attempt_by_merchant_id_connector_txn_id(
                platform.get_processor().get_account().get_id(),
                id,
                platform.get_processor().get_account().storage_scheme,
            )
            .await
            .to_not_found_response(errors::ApiErrorResponse::WebhookResourceNotFound),
        api::ObjectReferenceId::PaymentId(api::PaymentIdType::PaymentAttemptId(ref id)) => db
            .find_payment_attempt_by_attempt_id_merchant_id(
                id,
                platform.get_processor().get_account().get_id(),
                platform.get_processor().get_account().storage_scheme,
            )
            .await
            .to_not_found_response(errors::ApiErrorResponse::WebhookResourceNotFound),
        api::ObjectReferenceId::PaymentId(api::PaymentIdType::PreprocessingId(ref id)) => db
            .find_payment_attempt_by_preprocessing_id_merchant_id(
                id,
                platform.get_processor().get_account().get_id(),
                platform.get_processor().get_account().storage_scheme,
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
    platform: domain::Platform,
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
                            platform.get_processor().get_account().get_id(),
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
                            platform.get_processor().get_account().get_id().clone(),
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
                    platform.get_processor().get_key_store().key.get_inner(),
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
                    merchant_id: Some(platform.get_processor().get_account().get_id().clone()),
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
                    platform.clone(),
                    None,
                    payments::PaymentConfirm,
                    payment_confirm_req,
                    services::api::AuthFlow::Merchant,
                    payments::CallConnectorAction::Trigger,
                    None,
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
                            platform.get_processor().get_account().get_id(),
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
                                platform,
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
    platform: domain::Platform,
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
                    platform.get_processor().get_account().get_id(),
                    mandate_id.as_str(),
                    platform.get_processor().get_account().storage_scheme,
                )
                .await
                .to_not_found_response(errors::ApiErrorResponse::MandateNotFound)?,
            webhooks::ObjectReferenceId::MandateId(
                webhooks::MandateIdType::ConnectorMandateId(connector_mandate_id),
            ) => db
                .find_mandate_by_merchant_id_connector_mandate_id(
                    platform.get_processor().get_account().get_id(),
                    connector_mandate_id.as_str(),
                    platform.get_processor().get_account().storage_scheme,
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
                platform.get_processor().get_account().get_id(),
                &mandate_id,
                storage::MandateUpdate::StatusUpdate { mandate_status },
                mandate,
                platform.get_processor().get_account().storage_scheme,
            )
            .await
            .to_not_found_response(errors::ApiErrorResponse::MandateNotFound)?;
        let mandates_response = Box::new(
            api::mandates::MandateResponse::from_db_mandate(
                &state,
                platform.get_processor().get_key_store().clone(),
                updated_mandate.clone(),
                platform.get_processor().get_account(),
            )
            .await?,
        );
        let event_type: Option<enums::EventType> = updated_mandate.mandate_status.into();
        if let Some(outgoing_event_type) = event_type {
            Box::pin(super::create_event_and_trigger_outgoing_webhook(
                state,
                platform,
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
    platform: domain::Platform,
    source_verified: bool,
    event_type: webhooks::IncomingWebhookEvent,
    object_ref_id: api::ObjectReferenceId,
    business_profile: domain::Profile,
) -> CustomResult<WebhookResponseTracker, errors::ApiErrorResponse> {
    if source_verified {
        let payment_attempt =
            get_payment_attempt_from_object_reference_id(&state, object_ref_id, &platform).await?;
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
                    platform.clone(),
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
                    platform.clone(),
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
                        platform,
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
    platform: domain::Platform,
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
            &platform,
        )
        .await?;
        let option_dispute = db
            .find_by_merchant_id_payment_id_connector_dispute_id(
                platform.get_processor().get_account().get_id(),
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
            platform.get_processor().get_account().get_id(),
            &platform.get_processor().get_account().organization_id,
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
            platform,
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
    platform: domain::Platform,
    business_profile: domain::Profile,
    webhook_details: api::IncomingWebhookDetails,
    source_verified: bool,
) -> CustomResult<WebhookResponseTracker, errors::ApiErrorResponse> {
    let response = if source_verified {
        let payment_attempt = get_payment_attempt_from_object_reference_id(
            &state,
            webhook_details.object_reference_id,
            &platform,
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
            platform.to_owned(),
            None,
            payments::PaymentConfirm,
            request,
            services::api::AuthFlow::Merchant,
            payments::CallConnectorAction::Trigger,
            None,
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
                    platform,
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
    platform: &domain::Platform,
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
            platform.get_processor().get_account().get_id(),
            connector_name,
            merchant_connector_account.connector_webhook_details.clone(),
        )
        .await
        .change_context(errors::ConnectorError::WebhookSourceVerificationFailed)?;

    let router_data = construct_webhook_router_data(
        state,
        connector_name,
        merchant_connector_account,
        platform,
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
    platform: &domain::Platform,
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
                platform.get_processor().get_account().get_id(),
                &common_utils::id_type::MerchantConnectorAccountId::wrap(
                    connector_name_or_mca_id.to_owned(),
                )
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable(
                    "Error while converting MerchanConnectorAccountId from string
                    ",
                )?,
                platform.get_processor().get_key_store(),
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

async fn update_additional_payment_method_data(
    state: &SessionState,
    platform: &domain::Platform,
    connector: &ConnectorEnum,
    request_details: &IncomingWebhookRequestDetails<'_>,
    payment_method_id: String,
) -> CustomResult<(), errors::ApiErrorResponse> {
    let payment_method_update = connector
        .get_additional_payment_method_data(request_details)
        .change_context(errors::ApiErrorResponse::InternalServerError)?
        .ok_or(errors::ApiErrorResponse::InternalServerError)?;

    let db = state.store.as_ref();

    let pm = db
        .find_payment_method(
            platform.get_processor().get_key_store(),
            payment_method_id.as_str(),
            platform.get_processor().get_account().storage_scheme,
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::PaymentMethodNotFound)?;

    if pm.locker_id.is_some() {
        return Err(report!(errors::ApiErrorResponse::NotSupported {
            message: "Cannot proceed to update Payment Method when locker_id is already present"
                .into(),
        }));
    }

    Box::pin(cards::update_customer_payment_method(
        state.clone(),
        platform.clone(),
        payment_method_update,
        &payment_method_id,
        Some(pm),
    ))
    .await?;
    Ok(())
}

async fn update_connector_mandate_details(
    state: &SessionState,
    platform: &domain::Platform,
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
            get_payment_attempt_from_object_reference_id(state, object_ref_id, platform).await?;
        if let Some(ref payment_method_id) = payment_attempt.payment_method_id {
            let payment_method_info = state
                .store
                .find_payment_method(
                    platform.get_processor().get_key_store(),
                    payment_method_id,
                    platform.get_processor().get_account().storage_scheme,
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

                    if mandate_details.payments.as_ref().is_none_or(|payments| {
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
                                tokenization: None, // We need to handle tokenization field update in webhooks as well
                                updated_by: platform
                                    .get_processor()
                                    .get_account()
                                    .storage_scheme
                                    .to_string(),
                            };

                        state
                            .store
                            .update_payment_attempt_with_attempt_id(
                                payment_attempt.clone(),
                                attempt_update,
                                platform.get_processor().get_account().storage_scheme,
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
                last_modified_by: None,
            };

            state
                .store
                .update_payment_method(
                    platform.get_processor().get_key_store(),
                    payment_method_info,
                    pm_update,
                    platform.get_processor().get_account().storage_scheme,
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
