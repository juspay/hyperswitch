//! The v1 payment report, ported from `weekly_payment_report_generator_step_1`.
//!
//! Headers and column order are the Lambda's. Values are standard rather than Python-rendered:
//! JSON columns are JSON, amounts are exact decimals, booleans are `true`/`false` and timestamps
//! are ISO 8601 UTC. Also on purpose:
//! - rows are read in keyset pages instead of stopping at the Lambda's `LIMIT 50000`;
//! - the scope adds `processor_merchant_id`, which the Lambda ignored, so a connected merchant no
//!   longer receives its platform's other payments.

use common_utils::errors::CustomResult;
use error_stack::ResultExt;
use sqlx::Row;

use super::{sql_timestamp, ReportColumn as Column, ReportPage, ReportScope};
use crate::{errors::AnalyticsError, AnalyticsProvider};

/// An amount in major units as an exact decimal carrying the currency's places (`65.00`, `4.074`,
/// `2537`). The exponent table is the Lambda's own.
macro_rules! major_units {
    ($amount:literal) => {
        concat!(
            "(CASE WHEN pi.currency IN ('BHD','IQD','JOD','KWD','LYD','OMR','TND') THEN round(",
            $amount,
            "::numeric / 1000, 3) WHEN pi.currency IN ('BIF','CLP','DJF','GNF','JPY','KMF','KRW','MGA','PYG','RWF','UGX','VND','VUV','XAF','XOF','XPF') THEN ",
            $amount,
            "::numeric ELSE round(",
            $amount,
            "::numeric / 100, 2) END)::text"
        )
    };
}

/// `V1_PAYMENTS_CSV_COLUMNS` / `V1_PAYMENTS_SQL_COLUMNS`, each rendered as text. JSON columns all
/// go through `jsonb` so they share one formatting.
const V1_COLUMNS: [Column; 35] = [
    Column::new("payment_id", "pi.payment_id"),
    Column::new("attempt_id", "pa.attempt_id"),
    Column::new("status", "pa.status::text"),
    Column::new("amount", major_units!("pi.amount")),
    Column::new("currency", "pi.currency::text"),
    Column::new("connector", "pa.connector"),
    Column::new("connector_transaction_id", "pa.connector_transaction_id"),
    Column::new("amount_to_capture", major_units!("pa.amount_to_capture")),
    Column::new("customer_id", "pi.customer_id"),
    Column::new("created_at", sql_timestamp!("pa.created_at")),
    Column::new("order_details", "to_jsonb(pi.order_details)::text"),
    Column::new("error_message", "pa.error_message"),
    Column::new("capture_method", "pa.capture_method::text"),
    Column::new("authentication_type", "pa.authentication_type::text"),
    Column::new("mandate_id", "pa.mandate_id"),
    Column::new("payment_method", "pa.payment_method"),
    Column::new("payment_method_type", "pa.payment_method_type"),
    Column::new("metadata", "pi.metadata::text"),
    Column::new("setup_future_usage", "pi.setup_future_usage::text"),
    Column::new("statement_descriptor_name", "pi.statement_descriptor_name"),
    Column::new("description", "pi.description"),
    Column::new("off_session", "pi.off_session::text"),
    Column::new("business_country", "pi.business_country::text"),
    Column::new("business_label", "pi.business_label"),
    Column::new("business_sub_label", "pa.business_sub_label"),
    Column::new(
        "allowed_payment_method_types",
        "pi.allowed_payment_method_types::jsonb::text",
    ),
    Column::new("payment_method_data", "pa.payment_method_data::text"),
    Column::new(
        "card_network",
        "pa.payment_method_data->'card'->>'card_network'",
    ),
    Column::new("fingerprint_id", "pi.fingerprint_id"),
    Column::new("modified_at", sql_timestamp!("pa.modified_at")),
    Column::new("error_code", "pa.error_code"),
    Column::new("payment_method_id", "pa.payment_method_id"),
    Column::new(
        "card_holder_name",
        "pa.payment_method_data->'card'->>'card_holder_name'",
    ),
    Column::new(
        "merchant_order_reference_id",
        "pi.merchant_order_reference_id",
    ),
    Column::new("profile_id", "pi.profile_id"),
];

/// The last row of a page. `attempt_id` is unique only within a merchant, so the merchant breaks
/// ties at org scope.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct PaymentReportCursor {
    created_at: time::PrimitiveDateTime,
    merchant_id: String,
    attempt_id: String,
}

/// The CSV header row.
pub fn csv_header() -> Vec<u8> {
    super::csv_header(&V1_COLUMNS)
}

/// Reads the page of attempts after `cursor` created within `[start_time, end_time]`, inclusive at
/// both ends as the Lambda was.
pub async fn fetch_page(
    provider: &AnalyticsProvider,
    scope: &ReportScope,
    start_time: time::PrimitiveDateTime,
    end_time: time::PrimitiveDateTime,
    cursor: Option<&PaymentReportCursor>,
    page_size: usize,
) -> CustomResult<ReportPage<PaymentReportCursor>, AnalyticsError> {
    let pool = super::report_pool(provider)?;
    let limit = i64::try_from(page_size)
        .change_context(AnalyticsError::UnknownError)
        .attach_printable("Payment report page size does not fit a SQL LIMIT")?;

    let mut query = sqlx::QueryBuilder::<sqlx::Postgres>::new("SELECT ");
    for column in V1_COLUMNS.iter() {
        query.push(column.expression).push(", ");
    }
    // The Lambda joined on `active_attempt_id` alone. Also matching the merchant and payment keeps
    // the join on the intent's primary key and away from other merchants' attempts.
    query.push(
        "pa.created_at, pa.merchant_id, pa.attempt_id \
         FROM payment_attempt AS pa \
         LEFT JOIN payment_intent AS pi \
         ON pi.merchant_id = pa.merchant_id \
         AND pi.payment_id = pa.payment_id \
         AND pi.active_attempt_id = pa.attempt_id \
         WHERE pa.organization_id = ",
    );
    query.push_bind(scope.org_id.clone());

    if let Some(merchant_ids) = &scope.merchant_ids {
        query.push(" AND pa.merchant_id = ANY(");
        query.push_bind(merchant_ids.clone());
        query.push(")");
    }
    if let Some(processor_merchant_ids) = &scope.processor_merchant_ids {
        query.push(" AND pa.processor_merchant_id = ANY(");
        query.push_bind(processor_merchant_ids.clone());
        query.push(")");
    }
    if let Some(profile_ids) = &scope.profile_ids {
        query.push(" AND pa.profile_id = ANY(");
        query.push_bind(profile_ids.clone());
        query.push(")");
    }

    query.push(" AND pa.created_at >= ");
    query.push_bind(start_time);
    query.push(" AND pa.created_at <= ");
    query.push_bind(end_time);

    if let Some(cursor) = cursor {
        query.push(" AND (pa.created_at, pa.merchant_id, pa.attempt_id) > (");
        query.push_bind(cursor.created_at);
        query.push(", ");
        query.push_bind(cursor.merchant_id.clone());
        query.push(", ");
        query.push_bind(cursor.attempt_id.clone());
        query.push(")");
    }

    query.push(" ORDER BY pa.created_at, pa.merchant_id, pa.attempt_id LIMIT ");
    query.push_bind(limit);

    let rows = query
        .build()
        .fetch_all(pool)
        .await
        .change_context(AnalyticsError::UnknownError)
        .attach_printable("Failed to read a payment report page")?;

    let next_cursor = match rows.last() {
        Some(last) if rows.len() >= page_size => Some(
            read_cursor(last)
                .change_context(AnalyticsError::UnknownError)
                .attach_printable("Failed to read the payment report cursor")?,
        ),
        _ => None,
    };

    Ok(ReportPage {
        csv: super::render_rows(&V1_COLUMNS, &rows)?,
        row_count: rows.len(),
        next_cursor,
    })
}

/// The cursor columns are selected after the report columns.
fn read_cursor(row: &sqlx::postgres::PgRow) -> Result<PaymentReportCursor, sqlx::Error> {
    let base = V1_COLUMNS.len();
    Ok(PaymentReportCursor {
        created_at: row.try_get(base)?,
        merchant_id: row.try_get(base + 1)?,
        attempt_id: row.try_get(base + 2)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn header_matches_lambda() {
        // V1_PAYMENTS_CSV_COLUMNS from weekly_payment_report_generator_step_1.
        let expected = concat!(
            r#""payment_id","attempt_id","status","amount","currency","connector","#,
            r#""connector_transaction_id","amount_to_capture","customer_id","created_at","#,
            r#""order_details","error_message","capture_method","authentication_type","#,
            r#""mandate_id","payment_method","payment_method_type","metadata","#,
            r#""setup_future_usage","statement_descriptor_name","description","off_session","#,
            r#""business_country","business_label","business_sub_label","#,
            r#""allowed_payment_method_types","payment_method_data","card_network","#,
            r#""fingerprint_id","modified_at","error_code","payment_method_id","#,
            r#""card_holder_name","merchant_order_reference_id","profile_id""#,
            "\r\n"
        );
        assert_eq!(String::from_utf8_lossy(&csv_header()), expected);
    }
}
