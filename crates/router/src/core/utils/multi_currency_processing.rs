use api_models::payments::{
    CaptureFeatureMetadata, FeatureMetadata, MultiCurrencyProcessingAmountData,
};
use common_utils::{ext_traits::ValueExt, types::MinorUnit};
use diesel_models::{capture::Capture, enums::CaptureStatus};
use error_stack::{report, ResultExt};
use hyperswitch_interfaces::api::ConnectorSpecifications;

use crate::core::errors::{ApiErrorResponse, RouterResult};

pub(crate) fn is_supported(connector: &str) -> RouterResult<bool> {
    Ok(
        crate::types::api::ConnectorData::convert_connector(connector)?
            .is_multi_currency_processing_supported(),
    )
}

/// Resolve the merchant settlement values without deriving them from customer money.
pub(crate) fn resolve_operation_multi_currency_processing(
    connector: &str,
    payment_feature_metadata: Option<&FeatureMetadata>,
    operation_multi_currency_processing: Option<&MultiCurrencyProcessingAmountData>,
    customer_amount: MinorUnit,
    payment_amount: MinorUnit,
) -> RouterResult<Option<MultiCurrencyProcessingAmountData>> {
    if !is_supported(connector)? {
        return Ok(None);
    }
    let Some(payment_settlement) =
        payment_feature_metadata.and_then(|metadata| metadata.multi_currency_processing.as_ref())
    else {
        if operation_multi_currency_processing.is_some() {
            return Err(report!(ApiErrorResponse::InvalidRequestData {
                message: "feature_metadata.multi_currency_processing requires a payment with multi-currency processing".to_string(),
            }));
        }
        return Ok(None);
    };

    let settlement = match operation_multi_currency_processing {
        Some(multi_currency_processing) => multi_currency_processing.clone(),
        None if customer_amount == payment_amount => MultiCurrencyProcessingAmountData {
            amount: payment_settlement.amount,
            currency: payment_settlement.currency,
        },
        None => {
            return Err(report!(ApiErrorResponse::InvalidRequestData {
                message: "Partial multi-currency processing operations require feature_metadata.multi_currency_processing.amount and currency"
                    .to_string(),
            }));
        }
    };
    if settlement.currency != payment_settlement.currency {
        return Err(report!(ApiErrorResponse::InvalidRequestData {
            message: "multi-currency processing settlement currency must match the payment's settlement currency"
                .to_string(),
        }));
    }
    if settlement.amount.get_amount_as_i64() <= 0 || settlement.amount > payment_settlement.amount {
        return Err(report!(ApiErrorResponse::InvalidRequestData {
            message: "multi-currency processing settlement amount must be positive and must not exceed the authorized settlement amount".to_string(),
        }));
    }
    Ok(Some(settlement))
}

/// Validate before inserting the current capture; previous captures exclude it.
pub(crate) fn validate_capture_settlement(
    connector: &str,
    payment_feature_metadata: Option<&FeatureMetadata>,
    current_settlement: Option<&MultiCurrencyProcessingAmountData>,
    previous_captures: &[Capture],
) -> RouterResult<()> {
    if !is_supported(connector)? {
        return Ok(());
    }
    let Some(payment_settlement) =
        payment_feature_metadata.and_then(|metadata| metadata.multi_currency_processing.as_ref())
    else {
        return Ok(());
    };
    let Some(current_settlement) = current_settlement else {
        return Err(report!(ApiErrorResponse::InternalServerError)
            .attach_printable("Missing resolved multi-currency processing for capture"));
    };

    let mut remaining = payment_settlement.amount.get_amount_as_i64();
    for capture in previous_captures
        .iter()
        .filter(|capture| capture.status != CaptureStatus::Failed)
    {
        // Started captures reserve settlement money until their outcome is known.
        let previous_settlement = capture
            .feature_metadata
            .as_ref()
            .map(|metadata| {
                metadata
                    .clone()
                    .parse_value::<CaptureFeatureMetadata>("CaptureFeatureMetadata")
            })
            .transpose()
            .change_context(ApiErrorResponse::InternalServerError)?
            .and_then(|metadata| metadata.multi_currency_processing)
            .ok_or_else(|| {
                report!(ApiErrorResponse::InternalServerError).attach_printable(
                    "Missing multi-currency processing feature metadata for a previous capture",
                )
            })?;
        if previous_settlement.currency != payment_settlement.currency
            || previous_settlement.amount.get_amount_as_i64() <= 0
        {
            return Err(
                report!(ApiErrorResponse::InternalServerError).attach_printable(
                    "Invalid multi-currency processing feature metadata for a previous capture",
                ),
            );
        }
        remaining = remaining
            .checked_sub(previous_settlement.amount.get_amount_as_i64())
            .ok_or_else(|| report!(ApiErrorResponse::InternalServerError))?;
    }
    if current_settlement.amount.get_amount_as_i64() > remaining {
        return Err(report!(ApiErrorResponse::InvalidRequestData {
            message: "Cumulative multi-currency processing settlement captures must not exceed the authorized settlement amount".to_string(),
        }));
    }
    Ok(())
}
