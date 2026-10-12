//! The v1 payment report, ported from `weekly_payment_report_generator_step_1`: its headers,
//! column order and rows, with standard values.

use super::{major_units, sql_timestamp, ReportDefinition};

/// Every attempt, with its payment's details when it is the payment's active attempt. The Lambda
/// joined on `active_attempt_id` alone; also matching the merchant and payment keeps the join on
/// the intent's primary key.
pub(crate) const V1_PAYMENTS: ReportDefinition = ReportDefinition {
    name: "v1_payments",
    webhook_type: "payment_report",
    title: "Payments",
    from: "payment_attempt AS pa LEFT JOIN payment_intent AS pi \
           ON pi.merchant_id = pa.merchant_id \
           AND pi.payment_id = pa.payment_id \
           AND pi.active_attempt_id = pa.attempt_id",
    table: "pa",
    organization_scoped: true,
    // JSON columns all go through `jsonb` so they share one formatting.
    columns: &[
        ("payment_id", "pi.payment_id"),
        ("attempt_id", "pa.attempt_id"),
        ("status", "pa.status::text"),
        ("amount", major_units!("pi.amount", "pi.currency")),
        ("currency", "pi.currency::text"),
        ("connector", "pa.connector"),
        ("connector_transaction_id", "pa.connector_transaction_id"),
        (
            "amount_to_capture",
            major_units!("pa.amount_to_capture", "pi.currency"),
        ),
        ("customer_id", "pi.customer_id"),
        ("created_at", sql_timestamp!("pa.created_at")),
        ("order_details", "to_jsonb(pi.order_details)::text"),
        ("error_message", "pa.error_message"),
        ("capture_method", "pa.capture_method::text"),
        ("authentication_type", "pa.authentication_type::text"),
        ("mandate_id", "pa.mandate_id"),
        ("payment_method", "pa.payment_method"),
        ("payment_method_type", "pa.payment_method_type"),
        ("metadata", "pi.metadata::text"),
        ("setup_future_usage", "pi.setup_future_usage::text"),
        ("statement_descriptor_name", "pi.statement_descriptor_name"),
        ("description", "pi.description"),
        ("off_session", "pi.off_session::text"),
        ("business_country", "pi.business_country::text"),
        ("business_label", "pi.business_label"),
        ("business_sub_label", "pa.business_sub_label"),
        (
            "allowed_payment_method_types",
            "pi.allowed_payment_method_types::jsonb::text",
        ),
        ("payment_method_data", "pa.payment_method_data::text"),
        (
            "card_network",
            "pa.payment_method_data->'card'->>'card_network'",
        ),
        ("fingerprint_id", "pi.fingerprint_id"),
        ("modified_at", sql_timestamp!("pa.modified_at")),
        ("error_code", "pa.error_code"),
        ("payment_method_id", "pa.payment_method_id"),
        (
            "card_holder_name",
            "pa.payment_method_data->'card'->>'card_holder_name'",
        ),
        (
            "merchant_order_reference_id",
            "pi.merchant_order_reference_id",
        ),
        ("profile_id", "pi.profile_id"),
    ],
};
