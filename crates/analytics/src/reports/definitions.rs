//! The reports ported from the `weekly_*_report_generator_step_1` Lambdas: their headers, column
//! order and rows, with standard values. Amounts are written as the Lambdas did: payments and
//! payouts in major units, refunds and disputes as stored. The email wording is each Lambda's.

use super::{
    major_units,
    Column::{Expr, Text, Time},
    ReportDefinition,
};

/// Every attempt, with its payment's details when it is the payment's active attempt. The Lambda
/// joined on `active_attempt_id` alone; also matching the merchant and payment keeps the join on
/// the intent's primary key.
pub(super) const V1_PAYMENTS: ReportDefinition = ReportDefinition {
    name: "v1_payments",
    webhook_type: "payment_report",
    title: "Payments",
    noun: "payments",
    covers: "payments processed",
    from: "payment_attempt AS pa LEFT JOIN payment_intent AS pi \
           ON pi.merchant_id = pa.merchant_id \
           AND pi.payment_id = pa.payment_id \
           AND pi.active_attempt_id = pa.attempt_id",
    table: "pa",
    organization_scoped: true,
    columns: &[
        Text("pi.payment_id"),
        Text("pa.attempt_id"),
        Text("pa.status"),
        Expr("amount", major_units!("pi.amount", "pi.currency")),
        Text("pi.currency"),
        Text("pa.connector"),
        Text("pa.connector_transaction_id"),
        Expr(
            "amount_to_capture",
            major_units!("pa.amount_to_capture", "pi.currency"),
        ),
        Text("pi.customer_id"),
        Time("pa.created_at"),
        Expr("order_details", "to_jsonb(pi.order_details)::text"),
        Text("pa.error_message"),
        Text("pa.capture_method"),
        Text("pa.authentication_type"),
        Text("pa.mandate_id"),
        Text("pa.payment_method"),
        Text("pa.payment_method_type"),
        Text("pi.metadata"),
        Text("pi.setup_future_usage"),
        Text("pi.statement_descriptor_name"),
        Text("pi.description"),
        Text("pi.off_session"),
        Text("pi.business_country"),
        Text("pi.business_label"),
        Text("pa.business_sub_label"),
        Expr(
            "allowed_payment_method_types",
            "pi.allowed_payment_method_types::jsonb::text",
        ),
        Text("pa.payment_method_data"),
        Expr(
            "card_network",
            "pa.payment_method_data->'card'->>'card_network'",
        ),
        Text("pi.fingerprint_id"),
        Time("pa.modified_at"),
        Text("pa.error_code"),
        Text("pa.payment_method_id"),
        Expr(
            "card_holder_name",
            "pa.payment_method_data->'card'->>'card_holder_name'",
        ),
        Text("pi.merchant_order_reference_id"),
        Text("pi.profile_id"),
    ],
};

pub(super) const REFUNDS: ReportDefinition = ReportDefinition {
    name: "refunds",
    webhook_type: "refund_report",
    title: "Refunds",
    noun: "refund",
    covers: "refunds made by your customers",
    from: "refund AS ref",
    table: "ref",
    organization_scoped: true,
    columns: &[
        Text("ref.internal_reference_id"),
        Text("ref.refund_id"),
        Text("ref.payment_id"),
        Text("ref.merchant_id"),
        Text("ref.connector_transaction_id"),
        Text("ref.connector"),
        Text("ref.total_amount"),
        Text("ref.currency"),
        Text("ref.refund_amount"),
        Text("ref.refund_status"),
        Text("ref.connector_refund_id"),
        Text("ref.external_reference_id"),
        Text("ref.refund_reason"),
        Text("ref.refund_type"),
        Text("ref.sent_to_gateway"),
        Text("ref.refund_error_message"),
        Text("ref.metadata"),
        Time("ref.created_at"),
        Time("ref.modified_at"),
        Text("ref.description"),
        Text("ref.attempt_id"),
        Text("ref.refund_error_code"),
        Text("ref.profile_id"),
    ],
};

/// The Lambda filtered disputes by merchant alone, so profile reports carried the merchant's other
/// profiles and organization reports were empty; the shared scope fixes both.
pub(super) const DISPUTES: ReportDefinition = ReportDefinition {
    name: "disputes",
    webhook_type: "dispute_report",
    title: "Disputes",
    noun: "dispute",
    covers: "disputes made by your customers",
    from: "dispute",
    table: "dispute",
    organization_scoped: true,
    columns: &[
        // Dropped by the 2025-02-06-111828 migration removing the int `id` columns, which left the
        // Lambda's query failing; kept empty so the columns stay where they were.
        Expr("id", "NULL::text"),
        Text("dispute.dispute_id"),
        Text("dispute.payment_id"),
        Text("dispute.connector_dispute_id"),
        Text("dispute.connector"),
        Text("dispute.amount"),
        Text("dispute.currency"),
        Text("dispute.dispute_stage"),
        Text("dispute.dispute_status"),
        Text("dispute.connector_status"),
        Text("dispute.connector_reason"),
        Text("dispute.connector_reason_code"),
        Time("dispute.created_at"),
        Time("dispute.modified_at"),
        Time("dispute.challenge_required_by"),
        Text("dispute.evidence"),
    ],
};

/// One row per payout attempt. Matching the merchant as well keeps the join to one payout.
pub(super) const PAYOUTS: ReportDefinition = ReportDefinition {
    name: "payouts",
    webhook_type: "payout_report",
    title: "Payouts",
    noun: "payout",
    covers: "payouts made by your customers",
    from: "payouts AS p LEFT JOIN payout_attempt AS pa \
           ON pa.merchant_id = p.merchant_id AND pa.payout_id = p.payout_id",
    table: "p",
    organization_scoped: true,
    columns: &[
        Text("p.payout_id"),
        Text("pa.payout_attempt_id"),
        Text("p.payout_link_id"),
        Text("pa.merchant_order_reference_id"),
        Text("pa.connector_payout_id"),
        Text("pa.status"),
        Expr("amount", major_units!("p.amount", "p.destination_currency")),
        Expr("currency", "p.destination_currency::text"),
        Text("p.payout_type"),
        Text("p.confirm"),
        Text("p.attempt_count"),
        Text("pa.is_eligible"),
        Text("pa.connector"),
        Text("p.payout_method_id"),
        Text("p.profile_id"),
        Text("p.merchant_id"),
        Text("p.organization_id"),
        Text("p.customer_id"),
        Text("p.recurring"),
        Text("p.auto_fulfill"),
        Text("p.priority"),
        Text("p.description"),
        Text("pa.error_code"),
        Text("pa.error_message"),
        Text("pa.unified_code"),
        Text("pa.unified_message"),
        Text("pa.business_country"),
        Text("pa.business_label"),
        Text("p.entity_type"),
        Time("p.created_at"),
        Time("p.last_modified_at"),
        Text("pa.additional_payout_method_data"),
        Text("p.metadata"),
    ],
};

/// `relay` has no `organization_id`, so it is scoped by merchant and profile only.
pub(super) const RELAY: ReportDefinition = ReportDefinition {
    name: "relay",
    webhook_type: "relay_report",
    title: "Relay",
    noun: "relay",
    covers: "relay requests made by your customers",
    from: "relay",
    table: "relay",
    organization_scoped: false,
    columns: &[
        Text("relay.id"),
        Text("relay.profile_id"),
        Text("relay.connector_id"),
        Text("relay.relay_type"),
        Text("relay.connector_resource_id"),
        Text("relay.request_data"),
        Text("relay.status"),
        Text("relay.connector_reference_id"),
        Text("relay.error_code"),
        Text("relay.error_message"),
        Text("relay.response_data"),
        Time("relay.created_at"),
        Time("relay.modified_at"),
    ],
};

pub(super) const AUTHENTICATIONS: ReportDefinition = ReportDefinition {
    name: "authentication",
    webhook_type: "authentication_report",
    title: "Authentication",
    noun: "authentication",
    covers: "authentications made by your customers",
    from: "authentication",
    table: "authentication",
    organization_scoped: true,
    columns: &[
        Text("authentication.authentication_id"),
        Text("authentication.merchant_id"),
        Text("authentication.authentication_connector"),
        Text("authentication.connector_authentication_id"),
        Text("authentication.authentication_data"),
        Text("authentication.payment_method_id"),
        Text("authentication.authentication_type"),
        Text("authentication.authentication_status"),
        Text("authentication.authentication_lifecycle_status"),
        Time("authentication.created_at"),
        Time("authentication.modified_at"),
        Text("authentication.error_message"),
        Text("authentication.error_code"),
        Text("authentication.connector_metadata"),
        Text("authentication.maximum_supported_version"),
        Text("authentication.threeds_server_transaction_id"),
        Text("authentication.authentication_flow_type"),
        Text("authentication.message_version"),
        Text("authentication.eci"),
        Text("authentication.trans_status"),
        Text("authentication.acquirer_bin"),
        Text("authentication.acquirer_merchant_id"),
        Text("authentication.three_ds_method_url"),
        Text("authentication.acs_url"),
        Text("authentication.acs_reference_number"),
        Text("authentication.acs_trans_id"),
        // Dropped by 2024-05-08-111348_delete_unused_column_from_authentication, which left the
        // Lambda's query failing; kept empty so the columns stay where they were.
        Expr("three_dsserver_trans_id", "NULL::text"),
        Text("authentication.profile_id"),
        Text("authentication.payment_id"),
        Text("authentication.merchant_connector_id"),
        Text("authentication.directory_server_id"),
        Text("authentication.ds_trans_id"),
        Text("authentication.acquirer_country_code"),
        Text("authentication.service_details"),
        Text("authentication.organization_id"),
    ],
};

/// `v2_payments` and `revenue_recovery` read v2's tables: every attempt, with its payment, scoped
/// and ordered by the payment.
const V2_PAYMENTS_FROM: &str =
    "payment_attempt AS pa INNER JOIN payment_intent AS pi ON pi.id = pa.payment_id";

macro_rules! v2_payment_columns {
    ($($extra:expr),* $(,)?) => {
        &[
            Text("pi.id"),
            Expr("attempt_id", "pa.id::text"),
            Text("pa.status"),
            Expr("amount", major_units!("pi.amount", "pi.currency")),
            Text("pi.currency"),
            Text("pa.connector"),
            Text("pa.connector_payment_id"),
            Expr(
                "amount_to_capture",
                major_units!("pa.amount_to_capture", "pi.currency"),
            ),
            Text("pi.customer_id"),
            Time("pi.created_at"),
            Expr("order_details", "to_jsonb(pi.order_details)::text"),
            Text("pa.error_message"),
            Text("pi.capture_method"),
            Text("pa.authentication_type"),
            Text("pa.payment_method_type_v2"),
            Text("pa.payment_method_subtype"),
            Text("pi.metadata"),
            Text("pi.setup_future_usage"),
            Text("pi.statement_descriptor"),
            Text("pi.description"),
            Text("pa.payment_method_data"),
            Text("pa.card_network"),
            Time("pi.modified_at"),
            Text("pa.error_code"),
            Text("pa.payment_method_id"),
            Text("pi.merchant_reference_id"),
            Text("pi.profile_id"),
            $($extra,)*
        ]
    };
}

pub(super) const V2_PAYMENTS: ReportDefinition = ReportDefinition {
    name: "v2_payments",
    webhook_type: "payment_report",
    title: "Payments",
    noun: "payments",
    covers: "payments processed",
    from: V2_PAYMENTS_FROM,
    table: "pi",
    organization_scoped: true,
    columns: v2_payment_columns![
        // v2's payment_attempt has no mandate_id, which left the Lambda's query failing; kept empty
        // so the column stays where it was.
        Expr("mandate_id", "NULL::text"),
        Expr(
            "allowed_payment_method_types",
            "pi.allowed_payment_method_types::jsonb::text",
        ),
        Text("pa.fingerprint_id"),
        Expr(
            "card_holder_name",
            "pa.payment_method_data->'card'->>'card_holder_name'",
        ),
    ],
};

pub(super) const REVENUE_RECOVERY: ReportDefinition = ReportDefinition {
    name: "revenue_recovery",
    webhook_type: "payment_report",
    title: "Revenue Recovery",
    noun: "revenue recovery transactions",
    covers: "revenue recovery transactions processed",
    from: V2_PAYMENTS_FROM,
    table: "pi",
    organization_scoped: true,
    columns: v2_payment_columns![],
};
