use diesel_models::refund as diesel_refund;
use error_stack::report;
#[cfg(feature = "v1")]
use hyperswitch_domain_models::router_response_types::SupportedPaymentMethodsExt;
#[cfg(feature = "v1")]
use hyperswitch_interfaces::{self, api::ConnectorSpecifications};
use router_env::{instrument, tracing};
use time::PrimitiveDateTime;

use crate::{
    core::errors::{self, CustomResult, RouterResult},
    types::{
        self,
        api::enums as api_enums,
        storage::{self, enums},
    },
    utils::{self, OptionExt},
};

// Limit constraints for refunds list flow
pub const LOWER_LIMIT: i64 = 1;
pub const UPPER_LIMIT: i64 = 100;
pub const DEFAULT_LIMIT: i64 = 10;

#[derive(Debug, thiserror::Error)]
pub enum RefundValidationError {
    #[error("The payment attempt was not successful")]
    UnsuccessfulPaymentAttempt,
    #[error("The refund amount exceeds the amount captured")]
    RefundAmountExceedsPaymentAmount,
    #[error("The order has expired")]
    OrderExpired,
    #[error("The maximum refund count for this payment attempt")]
    MaxRefundCountReached,
    #[error("There is already another refund request for this payment attempt")]
    DuplicateRefund,
}

#[instrument(skip_all)]
pub fn validate_success_transaction(
    transaction: &storage::PaymentAttempt,
) -> CustomResult<(), RefundValidationError> {
    if transaction.status != enums::AttemptStatus::Charged {
        Err(report!(RefundValidationError::UnsuccessfulPaymentAttempt))?
    }

    Ok(())
}

#[instrument(skip_all)]
pub fn validate_refund_amount(
    amount_captured: i64,
    all_refunds: &[diesel_refund::Refund],
    refund_amount: i64,
) -> CustomResult<(), RefundValidationError> {
    let total_refunded_amount: i64 = all_refunds
        .iter()
        .filter_map(|refund| {
            if refund.refund_status != enums::RefundStatus::Failure
                && refund.refund_status != enums::RefundStatus::TransactionFailure
            {
                Some(refund.refund_amount.get_amount_as_i64())
            } else {
                None
            }
        })
        .sum();

    utils::when(
        refund_amount > (amount_captured - total_refunded_amount),
        || {
            Err(report!(
                RefundValidationError::RefundAmountExceedsPaymentAmount
            ))
        },
    )
}

#[instrument(skip_all)]
pub fn validate_payment_order_age(
    created_at: &PrimitiveDateTime,
    refund_max_age: i64,
) -> CustomResult<(), RefundValidationError> {
    let current_time = common_utils::date_time::now();

    utils::when(
        (current_time - *created_at).whole_days() > refund_max_age,
        || Err(report!(RefundValidationError::OrderExpired)),
    )
}

#[instrument(skip_all)]
pub fn validate_maximum_refund_against_payment_attempt(
    all_refunds: &[diesel_refund::Refund],
    refund_max_attempts: usize,
) -> CustomResult<(), RefundValidationError> {
    utils::when(all_refunds.len() > refund_max_attempts, || {
        Err(report!(RefundValidationError::MaxRefundCountReached))
    })
}

pub fn validate_refund_list(limit: Option<i64>) -> CustomResult<i64, errors::ApiErrorResponse> {
    match limit {
        Some(limit_val) => {
            if !(LOWER_LIMIT..=UPPER_LIMIT).contains(&limit_val) {
                Err(errors::ApiErrorResponse::InvalidRequestData {
                    message: "limit should be in between 1 and 100".to_string(),
                }
                .into())
            } else {
                Ok(limit_val)
            }
        }
        None => Ok(DEFAULT_LIMIT),
    }
}

#[cfg(feature = "v1")]
pub fn validate_for_valid_refunds(
    payment_attempt: &hyperswitch_domain_models::payments::payment_attempt::PaymentAttempt,
    connector_enum: hyperswitch_interfaces::connector_integration_interface::ConnectorEnum,
    connector: api_models::enums::Connector,
) -> RouterResult<()> {
    let payment_method = payment_attempt
        .payment_method
        .as_ref()
        .get_required_value("payment_method")?;

    let payment_method_type = payment_attempt
        .payment_method_type
        .get_required_value("payment_method_type")?;

    let supported_payment_methods = connector_enum.get_supported_payment_methods();

    let is_refund_supported = supported_payment_methods.is_none_or(|supported_payment_method| {
        supported_payment_method.is_refund_supported(payment_method, &payment_method_type)
    });

    if !is_refund_supported {
        Err(errors::ApiErrorResponse::InvalidRequestData {
                message: format!("Refunds are currently not supported for {payment_method_type} transactions via {connector}"),
            }
            .into())
    } else {
        Ok(())
    }
}

#[cfg(feature = "v2")]
pub fn validate_for_valid_refunds(
    payment_attempt: &hyperswitch_domain_models::payments::payment_attempt::PaymentAttempt,
    connector: api_models::enums::Connector,
) -> RouterResult<()> {
    let payment_method_type = payment_attempt.payment_method_type;

    match payment_method_type {
        diesel_models::enums::PaymentMethod::PayLater
        | diesel_models::enums::PaymentMethod::Wallet => {
            let payment_method_subtype = payment_attempt.payment_method_subtype;

            utils::when(
                matches!(
                    (connector, payment_method_subtype),
                    (
                        api_models::enums::Connector::Braintree,
                        Some(diesel_models::enums::PaymentMethodType::Paypal),
                    )
                ),
                || {
                    Err(errors::ApiErrorResponse::RefundNotPossible {
                        connector: connector.to_string(),
                    }
                    .into())
                },
            )
        }
        _ => Ok(()),
    }
}

pub fn validate_stripe_charge_refund(
    charge_type_option: Option<api_enums::PaymentChargeType>,
    split_refund_request: &Option<common_types::refunds::SplitRefund>,
) -> RouterResult<types::ChargeRefundsOptions> {
    let charge_type = charge_type_option.ok_or_else(|| {
        report!(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Missing `charge_type` in PaymentAttempt.")
    })?;

    let refund_request = match split_refund_request {
        Some(common_types::refunds::SplitRefund::StripeSplitRefund(stripe_split_refund)) => {
            stripe_split_refund
        }
        _ => Err(errors::ApiErrorResponse::MissingRequiredField {
            field_name: "stripe_split_refund",
        })?,
    };

    let options = match charge_type {
        api_enums::PaymentChargeType::Stripe(api_enums::StripeChargeType::Direct) => {
            types::ChargeRefundsOptions::Direct(types::DirectChargeRefund {
                revert_platform_fee: refund_request
                    .revert_platform_fee
                    .get_required_value("revert_platform_fee")?,
            })
        }
        api_enums::PaymentChargeType::Stripe(api_enums::StripeChargeType::Destination) => {
            types::ChargeRefundsOptions::Destination(types::DestinationChargeRefund {
                revert_platform_fee: refund_request
                    .revert_platform_fee
                    .get_required_value("revert_platform_fee")?,
                revert_transfer: refund_request
                    .revert_transfer
                    .get_required_value("revert_transfer")?,
            })
        }
    };

    Ok(options)
}

pub fn validate_adyen_charge_refund(
    adyen_split_payment_response: &common_types::domain::AdyenSplitData,
    adyen_split_refund_request: &common_types::domain::AdyenSplitData,
) -> RouterResult<()> {
    if adyen_split_refund_request.store != adyen_split_payment_response.store {
        return Err(report!(errors::ApiErrorResponse::InvalidDataValue {
            field_name: "split_payments.adyen_split_payment.store",
        }));
    };

    for refund_split_item in adyen_split_refund_request.split_items.iter() {
        let refund_split_reference = refund_split_item.reference.clone();
        let matching_payment_split_item = adyen_split_payment_response
            .split_items
            .iter()
            .find(|payment_split_item| refund_split_reference == payment_split_item.reference);

        if let Some(payment_split_item) = matching_payment_split_item {
            if let Some((refund_account, payment_account)) = refund_split_item
                .account
                .as_ref()
                .zip(payment_split_item.account.as_ref())
            {
                if !refund_account.eq(payment_account) {
                    return Err(report!(errors::ApiErrorResponse::InvalidRequestData {
                        message: format!(
                            "Invalid refund account for split item, reference: {refund_split_reference}",

                        ),
                    }));
                }
            }

            if refund_split_item.split_type != payment_split_item.split_type {
                return Err(report!(errors::ApiErrorResponse::InvalidRequestData {
                    message: format!(
                        "Invalid refund split_type for split item, reference: {refund_split_reference}",

                    ),
                }));
            }
        } else {
            return Err(report!(errors::ApiErrorResponse::InvalidRequestData {
                message: format!(
                    "No matching payment split item found for reference: {refund_split_reference}",
                ),
            }));
        }
    }
    Ok(())
}
pub fn validate_xendit_charge_refund(
    xendit_split_payment_response: &common_types::payments::XenditChargeResponseData,
    xendit_split_refund_request: &common_types::domain::XenditSplitSubMerchantData,
) -> RouterResult<Option<String>> {
    match xendit_split_payment_response {
        common_types::payments::XenditChargeResponseData::MultipleSplits(
            payment_sub_merchant_data,
        ) => {
            if payment_sub_merchant_data.for_user_id
                != Some(xendit_split_refund_request.for_user_id.clone())
            {
                return Err(errors::ApiErrorResponse::InvalidDataValue {
                    field_name: "xendit_split_refund.for_user_id does not match xendit_split_payment.for_user_id",
                }.into());
            }
            Ok(Some(xendit_split_refund_request.for_user_id.clone()))
        }
        common_types::payments::XenditChargeResponseData::SingleSplit(
            payment_sub_merchant_data,
        ) => {
            if payment_sub_merchant_data.for_user_id != xendit_split_refund_request.for_user_id {
                return Err(errors::ApiErrorResponse::InvalidDataValue {
                    field_name: "xendit_split_refund.for_user_id does not match xendit_split_payment.for_user_id",
                }.into());
            }
            Ok(Some(xendit_split_refund_request.for_user_id.clone()))
        }
    }
}

#[cfg(all(test, feature = "v1"))]
mod refund_amount_validation_tests {
    use common_utils::types::{ConnectorTransactionId, MinorUnit};
    use diesel_models::refund as diesel_refund;

    use super::*;

    /// Builds a `Refund` row with the given amount and status; all other fields are irrelevant to
    /// `validate_refund_amount` and are filled with placeholders.
    fn refund(amount: i64, status: enums::RefundStatus) -> diesel_refund::Refund {
        let now = common_utils::date_time::now();
        diesel_refund::Refund {
            internal_reference_id: "ref_internal".to_string(),
            refund_id: "ref_1".to_string(),
            payment_id: common_utils::id_type::PaymentId::default(),
            merchant_id: common_utils::id_type::MerchantId::default(),
            connector_transaction_id: ConnectorTransactionId::from("txn_1".to_string()),
            connector: "stripe".to_string(),
            connector_refund_id: None,
            external_reference_id: None,
            refund_type: enums::RefundType::InstantRefund,
            total_amount: MinorUnit::new(amount),
            currency: enums::Currency::USD,
            refund_amount: MinorUnit::new(amount),
            refund_status: status,
            sent_to_gateway: false,
            refund_error_message: None,
            metadata: None,
            refund_arn: None,
            created_at: now,
            modified_at: now,
            description: None,
            attempt_id: "att_1".to_string(),
            refund_reason: None,
            refund_error_code: None,
            profile_id: None,
            updated_by: String::new(),
            merchant_connector_id: None,
            charges: None,
            organization_id: common_utils::id_type::OrganizationId::default(),
            connector_refund_data: None,
            connector_transaction_data: None,
            split_refunds: None,
            unified_code: None,
            unified_message: None,
            processor_refund_data: None,
            processor_transaction_data: None,
            issuer_error_code: None,
            issuer_error_message: None,
            processor_merchant_id: None,
            created_by: None,
        }
    }

    #[test]
    fn full_refund_at_boundary_is_allowed() {
        // captured 100, nothing refunded yet, refund exactly 100 -> allowed
        assert!(validate_refund_amount(100, &[], 100).is_ok());
    }

    #[test]
    fn refund_one_over_captured_is_rejected() {
        // captured 100, refund 101 -> rejected
        assert!(validate_refund_amount(100, &[], 101).is_err());
    }

    #[test]
    fn partial_refunds_summing_to_captured_are_allowed() {
        // captured 100, already refunded 70 (Success), refund remaining 30 -> allowed
        let existing = [refund(70, enums::RefundStatus::Success)];
        assert!(validate_refund_amount(100, &existing, 30).is_ok());
    }

    #[test]
    fn partial_refund_exceeding_remaining_is_rejected() {
        // captured 100, already refunded 70, refund 31 (>30 remaining) -> rejected
        let existing = [refund(70, enums::RefundStatus::Success)];
        assert!(validate_refund_amount(100, &existing, 31).is_err());
    }

    #[test]
    fn pending_refunds_count_toward_the_total() {
        // A not-yet-settled (Pending) refund must still reserve balance, otherwise concurrent
        // in-flight refunds could both pass. captured 100, pending 100, refund 1 -> rejected.
        let existing = [refund(100, enums::RefundStatus::Pending)];
        assert!(validate_refund_amount(100, &existing, 1).is_err());
    }

    #[test]
    fn failed_refunds_are_excluded_from_the_total() {
        // Failed and TransactionFailure refunds free up balance again.
        let existing = [
            refund(100, enums::RefundStatus::Failure),
            refund(100, enums::RefundStatus::TransactionFailure),
        ];
        assert!(validate_refund_amount(100, &existing, 100).is_ok());
    }

    #[test]
    fn mixed_statuses_only_non_failed_reserve_balance() {
        // captured 100: 40 Success + 100 Failure(excluded) + 30 Pending => reserved 70,
        // remaining 30. Refund 30 -> allowed, 31 -> rejected.
        let existing = [
            refund(40, enums::RefundStatus::Success),
            refund(100, enums::RefundStatus::Failure),
            refund(30, enums::RefundStatus::Pending),
        ];
        assert!(validate_refund_amount(100, &existing, 30).is_ok());
        assert!(validate_refund_amount(100, &existing, 31).is_err());
    }
}
