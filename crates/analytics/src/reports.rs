//! Reports generated from the analytics Postgres pool, replacing the reporting Lambdas.
//!
//! A report is a [`ReportDefinition`]: where its rows come from and the SQL rendering each column
//! as text. [`write_csv`] runs any definition. Postgres renders the values, so the CSV carries
//! standard ones: JSON as JSON, booleans as `true`/`false`, timestamps as ISO 8601 UTC with
//! milliseconds (the format API responses use) and amounts as exact decimals.

pub mod payments;

use common_utils::errors::CustomResult;
use error_stack::ResultExt;
use futures::TryStreamExt;
use sqlx::{Postgres, QueryBuilder, Row};

use crate::{enums::AuthInfo, errors::AnalyticsError, AnalyticsProvider};

/// Where a report's rows come from and the columns written for each.
#[derive(Debug)]
pub struct ReportDefinition {
    /// Names the report in file keys and file names, e.g. `v1_payments`.
    pub name: &'static str,
    /// `data.report_type` in the report webhook, e.g. `payment_report`.
    pub webhook_type: &'static str,
    /// What the report covers, as the email words it, e.g. `Payments`.
    pub title: &'static str,
    /// The `FROM` clause, joins included.
    from: &'static str,
    /// The alias whose `organization_id`, `merchant_id`, `processor_merchant_id`, `profile_id`
    /// and `created_at` scope the rows.
    table: &'static str,
    /// Whether `table` has an `organization_id` column; `relay` does not.
    organization_scoped: bool,
    /// Each column's header and the SQL rendering it as text. `NULL` becomes an empty field.
    columns: &'static [(&'static str, &'static str)],
}

/// The reports that can be generated.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReportKind {
    V1Payments,
}

impl ReportKind {
    pub fn definition(self) -> &'static ReportDefinition {
        match self {
            Self::V1Payments => &payments::V1_PAYMENTS,
        }
    }
}

/// The format of every timestamp in a report: ISO 8601 UTC with milliseconds.
const TIMESTAMP_FORMAT: &[time::format_description::FormatItem<'static>] = time::macros::format_description!(
    "[year]-[month]-[day]T[hour]:[minute]:[second].[subsecond digits:3]Z"
);

/// `TIMESTAMP_FORMAT` for Postgres's `to_char`, so CSV cells match the webhook and email.
macro_rules! sql_timestamp {
    ($column:literal) => {
        concat!(
            "to_char(",
            $column,
            ", 'YYYY-MM-DD\"T\"HH24:MI:SS.MS\"Z\"')"
        )
    };
}
pub(crate) use sql_timestamp;

/// An amount in major units as an exact decimal carrying its currency's places (`65.00`, `4.074`,
/// `2537`). The exponent table is the Lambdas' own.
macro_rules! major_units {
    ($amount:literal, $currency:literal) => {
        concat!(
            "(CASE WHEN ", $currency, " IN ('BHD','IQD','JOD','KWD','LYD','OMR','TND') THEN round(",
            $amount,
            "::numeric / 1000, 3) WHEN ", $currency, " IN ('BIF','CLP','DJF','GNF','JPY','KMF','KRW','MGA','PYG','RWF','UGX','VND','VUV','XAF','XOF','XPF') THEN ",
            $amount,
            "::numeric ELSE round(",
            $amount,
            "::numeric / 100, 2) END)::text"
        )
    };
}
pub(crate) use major_units;

/// Formats a timestamp as the report's CSV cells do.
pub fn format_timestamp(value: time::PrimitiveDateTime) -> String {
    value
        .format(TIMESTAMP_FORMAT)
        .unwrap_or_else(|_| value.to_string())
}

/// Whether reports can be generated with this provider, so a request fails up front instead of
/// in the background. Reports read the Postgres pool; ClickHouse-only deployments cannot serve them.
pub fn is_supported(provider: &AnalyticsProvider) -> bool {
    report_pool(provider).is_ok()
}

fn report_pool(provider: &AnalyticsProvider) -> CustomResult<&sqlx::PgPool, AnalyticsError> {
    match provider {
        AnalyticsProvider::Sqlx(client)
        | AnalyticsProvider::CombinedCkh(client, _)
        | AnalyticsProvider::CombinedSqlx(client, _) => Ok(client.pool()),
        AnalyticsProvider::Clickhouse(_) => Err(error_stack::report!(
            AnalyticsError::NotImplemented("report generation without a Postgres analytics pool")
        )),
    }
}

/// Receives a report's CSV as [`write_csv`] produces it.
#[async_trait::async_trait]
pub trait CsvSink: Send {
    async fn write(&mut self, bytes: &[u8]) -> CustomResult<(), AnalyticsError>;
}

/// Streams the rows `auth` may see, created within `[start_time, end_time]` (inclusive at both
/// ends, as the Lambda was), to `sink` as CSV records, header first. Returns the number of rows.
pub async fn write_csv(
    provider: &AnalyticsProvider,
    report: &ReportDefinition,
    auth: &AuthInfo,
    start_time: time::PrimitiveDateTime,
    end_time: time::PrimitiveDateTime,
    sink: &mut dyn CsvSink,
) -> CustomResult<usize, AnalyticsError> {
    let table = report.table;
    let mut query = QueryBuilder::<Postgres>::new("SELECT ");
    let mut columns = query.separated(", ");
    for (_, expression) in report.columns {
        columns.push(*expression);
    }
    query.push(format_args!(" FROM {} WHERE TRUE", report.from));
    push_scope(&mut query, report, auth);
    query.push(format_args!(" AND {table}.created_at >= "));
    query.push_bind(start_time);
    query.push(format_args!(" AND {table}.created_at <= "));
    query.push_bind(end_time);
    query.push(format_args!(" ORDER BY {table}.created_at"));

    sink.write(&csv_record(
        report.columns.iter().map(|(header, _)| *header),
    ))
    .await?;

    let mut rows = query.build().fetch(report_pool(provider)?);
    let mut row_count = 0;
    while let Some(row) = rows
        .try_next()
        .await
        .change_context(AnalyticsError::UnknownError)
        .attach_printable_lazy(|| format!("Failed to read the {} report", report.name))?
    {
        let fields = (0..report.columns.len())
            .map(|index| row.try_get::<Option<String>, _>(index))
            .collect::<Result<Vec<_>, _>>()
            .change_context(AnalyticsError::UnknownError)
            .attach_printable("Failed to decode a report row")?;
        sink.write(&csv_record(
            fields.into_iter().map(Option::unwrap_or_default),
        ))
        .await?;
        row_count += 1;
    }
    Ok(row_count)
}

/// The same scope the analytics query builder applies for `auth`. The Lambda ignored
/// `processor_merchant_id`, so a connected merchant received its platform's other rows.
fn push_scope(query: &mut QueryBuilder<'_, Postgres>, report: &ReportDefinition, auth: &AuthInfo) {
    let table = report.table;
    let ids = |ids: &[common_utils::id_type::MerchantId]| {
        ids.iter()
            .map(|id| id.get_string_repr().to_owned())
            .collect::<Vec<_>>()
    };
    let (org_id, filters) = match auth {
        AuthInfo::OrgLevel { org_id } => (org_id, Vec::new()),
        AuthInfo::MerchantLevel {
            org_id,
            merchant_ids,
            processor_merchant_ids,
        } => (
            org_id,
            [
                Some(("merchant_id", ids(merchant_ids))),
                processor_merchant_ids
                    .as_deref()
                    .map(|processor_ids| ("processor_merchant_id", ids(processor_ids))),
            ]
            .into_iter()
            .flatten()
            .collect(),
        ),
        AuthInfo::ProfileLevel {
            org_id,
            merchant_id,
            profile_ids,
            processor_merchant_id,
        } => (
            org_id,
            [
                Some(("merchant_id", ids(std::slice::from_ref(merchant_id)))),
                Some((
                    "profile_id",
                    profile_ids
                        .iter()
                        .map(|id| id.get_string_repr().to_owned())
                        .collect(),
                )),
                processor_merchant_id
                    .as_ref()
                    .map(|id| ("processor_merchant_id", ids(std::slice::from_ref(id)))),
            ]
            .into_iter()
            .flatten()
            .collect(),
        ),
    };

    if report.organization_scoped {
        query.push(format_args!(" AND {table}.organization_id = "));
        query.push_bind(org_id.get_string_repr().to_owned());
    }
    for (column, values) in filters {
        query.push(format_args!(" AND {table}.{column} = ANY("));
        query.push_bind(values);
        query.push(")");
    }
}

/// One RFC 4180 record: every field quoted, quotes doubled, CRLF line ending.
fn csv_record<S: AsRef<str>>(fields: impl IntoIterator<Item = S>) -> Vec<u8> {
    let mut out = Vec::new();
    for (index, field) in fields.into_iter().enumerate() {
        if index > 0 {
            out.push(b',');
        }
        out.push(b'"');
        out.extend_from_slice(field.as_ref().replace('"', "\"\"").as_bytes());
        out.push(b'"');
    }
    out.extend_from_slice(b"\r\n");
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn csv_record_quotes_every_field() {
        assert_eq!(
            csv_record(["", "a\"b", "x\ny", "true", "1.50"]),
            b"\"\",\"a\"\"b\",\"x\ny\",\"true\",\"1.50\"\r\n"
        );
    }

    #[test]
    fn timestamps_use_the_api_format() {
        assert_eq!(
            format_timestamp(time::macros::datetime!(2026-10-12 09:10:07.215205)),
            "2026-10-12T09:10:07.215Z"
        );
    }

    #[test]
    fn payment_report_header_matches_lambda() {
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
        let header = csv_record(payments::V1_PAYMENTS.columns.iter().map(|(h, _)| *h));
        assert_eq!(String::from_utf8_lossy(&header), expected);
    }
}
