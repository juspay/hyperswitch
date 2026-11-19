//! Multi-currency processing authentication and settlement validation.

use diesel_models::enums;
use error_stack::report;
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

pub(crate) fn is_mcp_supported(connector: &str) -> RouterResult<bool> {
    Ok(api::ConnectorData::convert_connector(connector)?.is_multi_currency_processing_supported())
}

/// Resolve operation MCP at dispatch boundaries, including automatic captures and refund retries.
pub(crate) fn resolve_operation_multi_currency_processing<'a>(
    connector: &str,
    payment_feature_metadata: Option<&api_models::payments::FeatureMetadata>,
    requested_operation_mcp: Option<&'a api_models::payments::MultiCurrencyProcessingAmountData>,
) -> RouterResult<Option<&'a api_models::payments::MultiCurrencyProcessingAmountData>> {
    let payment_mcp =
        payment_feature_metadata.and_then(|metadata| metadata.multi_currency_processing.as_ref());
    resolve_operation_mcp(connector, payment_mcp, requested_operation_mcp)
}

/// Require and validate explicit MCP data for every capture or refund of an MCP payment.
fn resolve_operation_mcp<'a>(
    connector: &str,
    payment_mcp: Option<&api_models::payments::MultiCurrencyProcessingData>,
    requested_operation_mcp: Option<&'a api_models::payments::MultiCurrencyProcessingAmountData>,
) -> RouterResult<Option<&'a api_models::payments::MultiCurrencyProcessingAmountData>> {
    let supported = is_mcp_supported(connector)?;
    match (supported, payment_mcp, requested_operation_mcp) {
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
            validate_operation_settlement(payment_mcp, requested_operation_mcp)?;
            Ok(Some(requested_operation_mcp))
        }
    }
}

fn validate_operation_settlement(
    payment_mcp: &api_models::payments::MultiCurrencyProcessingData,
    operation_mcp: &api_models::payments::MultiCurrencyProcessingAmountData,
) -> RouterResult<()> {
    if operation_mcp.currency != payment_mcp.currency {
        Err(report!(errors::ApiErrorResponse::InvalidRequestData {
            message: "multi-currency processing settlement currency must match the payment's settlement currency"
                .to_string(),
        }))
    } else if operation_mcp.amount.get_amount_as_i64() <= 0
        || operation_mcp.amount > payment_mcp.amount
    {
        Err(report!(errors::ApiErrorResponse::InvalidRequestData {
            message: "multi-currency processing settlement amount must be positive and must not exceed the authorized settlement amount".to_string(),
        }))
    } else {
        Ok(())
    }
}

/// Reject missing or invalid settlement legs with an MCP validation error.
fn validate_stored_settlement(
    stored: Option<&api_models::payments::MultiCurrencyProcessingAmountData>,
    currency: enums::Currency,
) -> RouterResult<&api_models::payments::MultiCurrencyProcessingAmountData> {
    let stored = stored.ok_or_else(|| {
        report!(errors::ApiErrorResponse::InvalidRequestData {
            message: "Missing multi-currency processing metadata on a reserved operation"
                .to_string(),
        })
    })?;
    if stored.currency != currency || stored.amount.get_amount_as_i64() <= 0 {
        Err(report!(errors::ApiErrorResponse::InvalidRequestData {
            message: "Invalid multi-currency processing currency or amount on a reserved operation"
                .to_string(),
        }))
    } else {
        Ok(stored)
    }
}

/// Status selection and the settlement ceiling belong to each operation's policy.
fn remaining_settlement_balance<'a>(
    ceiling: common_utils::types::MinorUnit,
    currency: enums::Currency,
    mut reservations: impl Iterator<
        Item = Option<&'a api_models::payments::MultiCurrencyProcessingAmountData>,
    >,
) -> RouterResult<i64> {
    reservations.try_fold(ceiling.get_amount_as_i64(), |remaining, stored| {
        let reserved = validate_stored_settlement(stored, currency)?;
        remaining.checked_sub(reserved.amount.get_amount_as_i64())
            .filter(|balance| *balance >= 0)
            .ok_or_else(|| {
                report!(errors::ApiErrorResponse::InvalidRequestData {
                    message: "Stored multi-currency processing reservations exceed the settlement ceiling or overflow its balance".to_string(),
                })
            })
    })
}

/// Validate capture settlement and history, then construct its persistence metadata.
pub(crate) fn validate_multi_currency_processing_capture_settlement(
    connector: &str,
    payment_feature_metadata: Option<&api_models::payments::FeatureMetadata>,
    requested_metadata: Option<&api_models::payments::CaptureFeatureMetadata>,
    previous_captures: &[diesel_models::capture::Capture],
) -> RouterResult<Option<api_models::payments::CaptureFeatureMetadata>> {
    let payment_mcp =
        payment_feature_metadata.and_then(|metadata| metadata.multi_currency_processing.as_ref());
    let requested_mcp =
        requested_metadata.and_then(|metadata| metadata.multi_currency_processing.as_ref());
    let settlement = resolve_operation_mcp(connector, payment_mcp, requested_mcp)?;
    // No resolved operation MCP means the resolver found no active MCP processing.
    match (payment_mcp, settlement) {
        (_, None) | (None, _) => Ok(()),
        (Some(payment_mcp), Some(current_capture_mcp)) => {
            // For the first capture, empty history leaves the full payment MCP amount available.
            let reservations = previous_captures
                .iter()
                .filter(|capture| match capture.status {
                    enums::CaptureStatus::Charged
                    | enums::CaptureStatus::Started
                    | enums::CaptureStatus::Pending => true,
                    enums::CaptureStatus::Failed => false,
                })
                .map(|capture| {
                    capture
                        .feature_metadata
                        .as_ref()
                        .and_then(|metadata| metadata.multi_currency_processing.as_ref())
                });
            let remaining_mcp_amount = remaining_settlement_balance(
                payment_mcp.amount,
                payment_mcp.currency,
                reservations,
            )?;

            if current_capture_mcp.amount.get_amount_as_i64() > remaining_mcp_amount {
                Err(report!(errors::ApiErrorResponse::InvalidRequestData {
                    message: "Cumulative multi-currency processing settlement captures must not exceed the authorized MCP amount".to_string(),
                }))
            } else {
                Ok(())
            }
        }
    }?;
    Ok(settlement
        .cloned()
        .map(
            |multi_currency_processing| api_models::payments::CaptureFeatureMetadata {
                multi_currency_processing: Some(multi_currency_processing),
            },
        )
        .or_else(|| requested_metadata.cloned()))
}

/// Validate refund settlement and history, then construct its persistence metadata.
pub(crate) fn validate_multi_currency_processing_refund_settlement(
    connector: &str,
    payment_feature_metadata: Option<&api_models::payments::FeatureMetadata>,
    requested_metadata: Option<api_models::refunds::RefundFeatureMetadata>,
    all_refunds: &[diesel_models::refund::Refund],
) -> RouterResult<Option<api_models::refunds::RefundFeatureMetadata>> {
    let payment_mcp =
        payment_feature_metadata.and_then(|metadata| metadata.multi_currency_processing.as_ref());
    let requested_mcp = requested_metadata
        .as_ref()
        .and_then(|metadata| metadata.multi_currency_processing.as_ref());
    let settlement = resolve_operation_mcp(connector, payment_mcp, requested_mcp)?;
    // No resolved operation MCP means the resolver found no active MCP processing.
    match (payment_mcp, settlement) {
        (_, None) | (None, _) => Ok(()),
        (Some(payment_mcp), Some(current_refund_mcp)) => {
            // For the first refund, empty history leaves the full payment MCP amount available.
            let reservations = all_refunds
                .iter()
                .filter(|refund| match refund.refund_status {
                    enums::RefundStatus::Success
                    | enums::RefundStatus::Pending
                    | enums::RefundStatus::ManualReview => true,
                    enums::RefundStatus::Failure | enums::RefundStatus::TransactionFailure => false,
                })
                .map(|refund| {
                    refund
                        .feature_metadata
                        .as_ref()
                        .and_then(|metadata| metadata.multi_currency_processing.as_ref())
                });
            let remaining_mcp_amount = remaining_settlement_balance(
                payment_mcp.amount,
                payment_mcp.currency,
                reservations,
            )?;

            if current_refund_mcp.amount.get_amount_as_i64() > remaining_mcp_amount {
                Err(report!(errors::ApiErrorResponse::InvalidRequestData {
                    message:
                        "Cumulative multi-currency processing settlement refunds must not exceed the authorized MCP amount"
                            .to_string(),
                }))
            } else {
                Ok(())
            }
        }
    }?;
    Ok(settlement
        .cloned()
        .map(
            |multi_currency_processing| api_models::refunds::RefundFeatureMetadata {
                multi_currency_processing: Some(multi_currency_processing),
            },
        )
        .or(requested_metadata))
}
