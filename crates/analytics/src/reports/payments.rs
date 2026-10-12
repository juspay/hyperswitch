//! The v1 payment report, ported from `weekly_payment_report_generator_step_1`.
//!
//! Headers, column order and value rendering are unchanged. Two things differ on purpose:
//! - rows are read in keyset pages instead of stopping at the Lambda's `LIMIT 50000`;
//! - the scope adds `processor_merchant_id`, which the Lambda ignored, so a connected merchant no
//!   longer receives its platform's other payments.

use common_utils::errors::CustomResult;
use error_stack::ResultExt;
use sqlx::Row;

use super::{ColumnKind as Kind, ReportColumn as Column, ReportPage, ReportScope};
use crate::{errors::AnalyticsError, AnalyticsProvider};

/// Amounts are written in major units. The exponent table is the Lambda's own, kept verbatim so
/// the rendered floats match; it is not `Currency::number_of_digits_after_decimal_point`.
macro_rules! major_units {
    ($amount:literal) => {
        concat!(
            "CASE WHEN pi.currency IN ('BHD','IQD','JOD','KWD','LYD','OMR','TND') THEN ",
            $amount,
            "::float / 1000.0 WHEN pi.currency IN ('BIF','CLP','DJF','GNF','JPY','KMF','KRW','MGA','PYG','RWF','UGX','VND','VUV','XAF','XOF','XPF') THEN ",
            $amount,
            "::float ELSE ",
            $amount,
            "::float / 100.0 END"
        )
    };
}

/// `V1_PAYMENTS_CSV_COLUMNS` / `V1_PAYMENTS_SQL_COLUMNS`. Enum columns are cast to text, which is
/// how psycopg2 received them.
const V1_COLUMNS: [Column; 35] = [
    Column::new("payment_id", "pi.payment_id", Kind::Text),
    Column::new("attempt_id", "pa.attempt_id", Kind::Text),
    Column::new("status", "pa.status::text", Kind::Text),
    Column::new("amount", major_units!("pi.amount"), Kind::Float),
    Column::new("currency", "pi.currency::text", Kind::Text),
    Column::new("connector", "pa.connector", Kind::Text),
    Column::new(
        "connector_transaction_id",
        "pa.connector_transaction_id",
        Kind::Text,
    ),
    Column::new(
        "amount_to_capture",
        major_units!("pa.amount_to_capture"),
        Kind::Float,
    ),
    Column::new("customer_id", "pi.customer_id", Kind::Text),
    Column::new("created_at", "pa.created_at", Kind::Timestamp),
    // psycopg2 decodes `jsonb[]` into a list, which `array_to_json` reproduces.
    Column::new(
        "order_details",
        "array_to_json(pi.order_details)::text",
        Kind::Json,
    ),
    Column::new("error_message", "pa.error_message", Kind::Text),
    Column::new("capture_method", "pa.capture_method::text", Kind::Text),
    Column::new(
        "authentication_type",
        "pa.authentication_type::text",
        Kind::Text,
    ),
    Column::new("mandate_id", "pa.mandate_id", Kind::Text),
    Column::new("payment_method", "pa.payment_method", Kind::Text),
    Column::new("payment_method_type", "pa.payment_method_type", Kind::Text),
    Column::new("metadata", "pi.metadata::text", Kind::Json),
    Column::new(
        "setup_future_usage",
        "pi.setup_future_usage::text",
        Kind::Text,
    ),
    Column::new(
        "statement_descriptor_name",
        "pi.statement_descriptor_name",
        Kind::Text,
    ),
    Column::new("description", "pi.description", Kind::Text),
    Column::new("off_session", "pi.off_session", Kind::Bool),
    Column::new("business_country", "pi.business_country::text", Kind::Text),
    Column::new("business_label", "pi.business_label", Kind::Text),
    Column::new("business_sub_label", "pa.business_sub_label", Kind::Text),
    Column::new(
        "allowed_payment_method_types",
        "pi.allowed_payment_method_types::text",
        Kind::Json,
    ),
    Column::new(
        "payment_method_data",
        "pa.payment_method_data::text",
        Kind::Json,
    ),
    Column::new(
        "card_network",
        "pa.payment_method_data->'card'->>'card_network'",
        Kind::Text,
    ),
    Column::new("fingerprint_id", "pi.fingerprint_id", Kind::Text),
    Column::new("modified_at", "pa.modified_at", Kind::Timestamp),
    Column::new("error_code", "pa.error_code", Kind::Text),
    Column::new("payment_method_id", "pa.payment_method_id", Kind::Text),
    Column::new(
        "card_holder_name",
        "pa.payment_method_data->'card'->>'card_holder_name'",
        Kind::Text,
    ),
    Column::new(
        "merchant_order_reference_id",
        "pi.merchant_order_reference_id",
        Kind::Text,
    ),
    Column::new("profile_id", "pi.profile_id", Kind::Text),
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
        // V1_PAYMENTS_CSV_COLUMNS from weekly_payment_report_generator_step_1, written by
        // csv.writer(quoting=csv.QUOTE_ALL).
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
