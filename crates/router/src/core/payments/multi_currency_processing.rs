//! Multi-currency processing authentication and settlement validation.

use diesel_models::enums;
use error_stack::report;
#[cfg(feature = "v1")]
use hyperswitch_domain_models::payments::PaymentIntent;
use hyperswitch_interfaces::api::ConnectorSpecifications;
use router_env::logger;

use crate::{
    core::errors::{self, RouterResult},
    types::api,
};
#[cfg(feature = "v1")]
use crate::{routes::metrics, services};

#[cfg(feature = "v1")]
pub fn validate_merchant_only_mcp_fields(
    auth_flow: services::AuthFlow,
    request: &api::PaymentsRequest,
    operation: &'static str,
) -> Result<(), errors::ApiErrorResponse> {
    if auth_flow == services::AuthFlow::Client
        && request
            .feature_metadata
            .as_ref()
            .and_then(|metadata| metadata.multi_currency_processing.as_ref())
            .is_some()
    {
        logger::error!(
            error_status_code = 400,
            error_status_class = "4xx",
            error_category = "invalid_integration",
            operation,
            "Invalid integration: MCP data supplied with client authentication"
        );
        metrics::CLIENT_MCP_UPDATE_REJECTED
            .add(1, router_env::metric_attributes!(("operation", operation)));
        return Err(errors::ApiErrorResponse::InvalidRequestData {
            message: "feature_metadata.multi_currency_processing can only be updated with merchant authentication".to_string(),
        });
    }

    Ok(())
}

#[cfg(feature = "v1")]
pub fn validate_and_update_settlement_conclusion(
    auth_flow: services::AuthFlow,
    request: &api::PaymentsRequest,
    payment_intent: &mut PaymentIntent,
    operation: &'static str,
) -> Result<(), errors::ApiErrorResponse> {
    match (auth_flow, request.settlement_conclusion_applied) {
        (services::AuthFlow::Client, Some(_)) => {
            logger::error!(
                error_status_code = 400,
                error_status_class = "4xx",
                error_category = "invalid_integration",
                operation,
                "Invalid integration: settlement_conclusion_applied supplied with client authentication"
            );
            metrics::CLIENT_SETTLEMENT_CONCLUSION_UPDATE_REJECTED
                .add(1, router_env::metric_attributes!(("operation", operation)));
            Err(errors::ApiErrorResponse::InvalidRequestData {
                message:
                    "settlement_conclusion_applied can only be updated with merchant authentication"
                        .to_string(),
            })
        }
        (services::AuthFlow::Merchant, Some(settlement_conclusion_applied)) => {
            payment_intent.settlement_conclusion_applied = Some(settlement_conclusion_applied);
            Ok(())
        }
        (services::AuthFlow::Client | services::AuthFlow::Merchant, None) => Ok(()),
    }
}

pub(crate) fn is_mcp_supported(connector: &str) -> RouterResult<bool> {
    Ok(api::ConnectorData::convert_connector(connector)?.is_multi_currency_processing_supported())
}

/// Require and validate explicit MCP data for every capture or refund of an MCP payment.
pub(crate) fn resolve_operation_multi_currency_processing(
    connector: &str,
    payment_feature_metadata: Option<&api_models::payments::FeatureMetadata>,
    requested_operation_mcp: Option<&api_models::payments::MultiCurrencyProcessingAmountData>,
) -> RouterResult<Option<api_models::payments::MultiCurrencyProcessingAmountData>> {
    let payment_mcp =
        payment_feature_metadata.and_then(|metadata| metadata.multi_currency_processing.as_ref());

    match (
        is_mcp_supported(connector)?,
        payment_mcp,
        requested_operation_mcp,
    ) {
        (false, _, _) | (true, None, None) => Ok(None),
        (true, None, Some(_)) => {
            Err(report!(errors::ApiErrorResponse::InvalidRequestData {
                message: "feature_metadata.multi_currency_processing requires a payment with multi-currency processing".to_string(),
            }))
        }
        (true, Some(_), None) => {
            Err(report!(errors::ApiErrorResponse::InvalidRequestData {
                message: "Captures and refunds of multi-currency processing payments require feature_metadata.multi_currency_processing.amount and currency"
                    .to_string(),
            }))
        }
        (true, Some(payment_mcp), Some(requested_operation_mcp)) => {
            let payment_mcp_currency = payment_mcp.currency;
            let operation_mcp_currency = requested_operation_mcp.currency;

            if operation_mcp_currency != payment_mcp_currency {
                Err(report!(errors::ApiErrorResponse::InvalidRequestData {
                    message: "multi-currency processing settlement currency must match the payment's settlement currency"
                        .to_string(),
                }))
            } else if requested_operation_mcp.amount.get_amount_as_i64() <= 0
                || requested_operation_mcp.amount > payment_mcp.amount
            {
                Err(report!(errors::ApiErrorResponse::InvalidRequestData {
                    message: "multi-currency processing settlement amount must be positive and must not exceed the authorized settlement amount".to_string(),
                }))
            } else {
                Ok(Some(requested_operation_mcp.clone()))
            }
        }
    }
}

/// Read and validate MCP data from a previous capture's stored feature metadata.
/// The caller includes successful and pending captures before calling this helper.
fn get_previous_capture(
    capture: &diesel_models::capture::Capture,
    payment_mcp_currency: enums::Currency,
) -> RouterResult<api_models::payments::MultiCurrencyProcessingAmountData> {
    let previous_capture_mcp = capture
        .feature_metadata
        .as_ref()
        .and_then(|metadata| metadata.multi_currency_processing.clone())
        .ok_or_else(|| {
            logger::error!("Missing MCP feature metadata for a previous capture");
            report!(errors::ApiErrorResponse::InternalServerError).attach_printable(
                "Missing multi-currency processing feature metadata for a previous capture",
            )
        })?;

    if previous_capture_mcp.currency != payment_mcp_currency
        || previous_capture_mcp.amount.get_amount_as_i64() <= 0
    {
        logger::error!("Invalid MCP currency or amount in a previous capture's feature metadata");
        Err(
            report!(errors::ApiErrorResponse::InternalServerError).attach_printable(
                "Invalid multi-currency processing feature metadata for a previous capture",
            ),
        )
    } else {
        Ok(previous_capture_mcp)
    }
}

/// Read and resolve MCP data from a previous refund's stored feature metadata.
/// The caller excludes failed refunds before calling this helper.
fn get_previous_mcp(
    connector: &str,
    payment_feature_metadata: Option<&api_models::payments::FeatureMetadata>,
    refund: &diesel_models::refund::Refund,
) -> RouterResult<api_models::payments::MultiCurrencyProcessingAmountData> {
    resolve_operation_multi_currency_processing(
        connector,
        payment_feature_metadata,
        refund
            .feature_metadata
            .as_ref()
            .and_then(|metadata| metadata.multi_currency_processing.as_ref()),
    )?
    .ok_or_else(|| {
        logger::error!(connector, "No MCP data resolved for a previous refund");
        report!(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("No MCP data resolved for a previous refund")
    })
}

/// Validate capture MCP currency and cumulative amounts before inserting the current capture.
pub(crate) fn validate_multi_currency_processing_capture_settlement(
    connector: &str,
    payment_feature_metadata: Option<&api_models::payments::FeatureMetadata>,
    current_capture_mcp: Option<&api_models::payments::MultiCurrencyProcessingAmountData>,
    previous_captures: &[diesel_models::capture::Capture],
) -> RouterResult<()> {
    let payment_mcp =
        payment_feature_metadata.and_then(|metadata| metadata.multi_currency_processing.as_ref());

    match (
        is_mcp_supported(connector)?,
        payment_mcp,
        current_capture_mcp,
    ) {
        (false, _, _) | (true, None, _) => Ok(()),
        (true, Some(_), None) => {
            logger::error!("Missing resolved MCP data for the current capture");
            Err(report!(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Missing resolved multi-currency processing for capture"))
        }
        (true, Some(payment_mcp), Some(current_capture_mcp)) => {
            // For the first capture, empty history leaves the full payment MCP amount available.
            let remaining_mcp_amount = previous_captures
                .iter()
                .filter(|capture| match capture.status {
                    enums::CaptureStatus::Charged
                    | enums::CaptureStatus::Started
                    | enums::CaptureStatus::Pending => true,
                    enums::CaptureStatus::Failed => false,
                })
                .try_fold(
                    payment_mcp.amount.get_amount_as_i64(),
                    |remaining_mcp_amount, capture| {
                        let previous_capture_mcp =
                            get_previous_capture(capture, payment_mcp.currency)?;

                        Ok::<_, error_stack::Report<errors::ApiErrorResponse>>(
                            remaining_mcp_amount - previous_capture_mcp.amount.get_amount_as_i64(),
                        )
                    },
                )?;

            if current_capture_mcp.currency != payment_mcp.currency {
                Err(report!(errors::ApiErrorResponse::InvalidRequestData {
                    message: "multi-currency processing settlement currency must match the payment's settlement currency"
                        .to_string(),
                }))
            } else if current_capture_mcp.amount.get_amount_as_i64() > remaining_mcp_amount {
                Err(report!(errors::ApiErrorResponse::InvalidRequestData {
                    message: "Cumulative multi-currency processing settlement captures must not exceed the authorized MCP amount".to_string(),
                }))
            } else {
                Ok(())
            }
        }
    }
}

/// Resolve the current refund MCP and validate cumulative refund MCP amounts.
pub(crate) fn validate_multi_currency_processing_refund_settlement(
    connector: &str,
    payment_feature_metadata: Option<&api_models::payments::FeatureMetadata>,
    refund_feature_metadata: Option<&api_models::refunds::RefundFeatureMetadata>,
    all_refunds: &[diesel_models::refund::Refund],
) -> RouterResult<Option<api_models::payments::MultiCurrencyProcessingAmountData>> {
    let current_refund_mcp = resolve_operation_multi_currency_processing(
        connector,
        payment_feature_metadata,
        refund_feature_metadata.and_then(|metadata| metadata.multi_currency_processing.as_ref()),
    )?;
    let payment_mcp =
        payment_feature_metadata.and_then(|metadata| metadata.multi_currency_processing.as_ref());

    match (current_refund_mcp, payment_mcp) {
        (Some(current_refund_mcp), Some(payment_mcp)) => {
            // For the first refund, empty history leaves the full payment MCP amount available.
            let remaining_mcp_amount = all_refunds
                .iter()
                .filter(|refund| match refund.refund_status {
                    enums::RefundStatus::Success
                    | enums::RefundStatus::Pending
                    | enums::RefundStatus::ManualReview => true,
                    enums::RefundStatus::Failure | enums::RefundStatus::TransactionFailure => false,
                })
                .try_fold(
                    payment_mcp.amount.get_amount_as_i64(),
                    |remaining_mcp_amount, refund| {
                        let previous_refund_mcp =
                            get_previous_mcp(connector, payment_feature_metadata, refund)?;

                        Ok::<_, error_stack::Report<errors::ApiErrorResponse>>(
                            remaining_mcp_amount - previous_refund_mcp.amount.get_amount_as_i64(),
                        )
                    },
                )?;

            if current_refund_mcp.amount.get_amount_as_i64() > remaining_mcp_amount {
                Err(report!(errors::ApiErrorResponse::InvalidRequestData {
                    message:
                        "Cumulative multi-currency processing settlement refunds must not exceed the authorized MCP amount"
                            .to_string(),
                }))
            } else {
                Ok(Some(current_refund_mcp))
            }
        }
        _ => Ok(None),
    }
}
