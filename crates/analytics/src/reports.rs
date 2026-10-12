//! Report generation that used to run in the reporting Lambdas, read from the analytics Postgres
//! pool.
//!
//! Every column is rendered to text by Postgres, so the CSV carries standard values: JSON columns
//! as JSON, booleans as `true`/`false`, timestamps as ISO 8601 UTC with milliseconds (the format
//! API responses use) and amounts as exact decimals.

pub mod payments;

use common_utils::errors::CustomResult;
use error_stack::ResultExt;
use sqlx::{postgres::PgRow, Row};

use crate::{enums::AuthInfo, errors::AnalyticsError, AnalyticsProvider};

/// The format of every timestamp in a report, its file name aside: ISO 8601 UTC with
/// milliseconds, as API responses render them.
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

/// Formats a timestamp as the report's CSV cells do.
pub fn format_timestamp(value: time::PrimitiveDateTime) -> String {
    value
        .format(TIMESTAMP_FORMAT)
        .unwrap_or_else(|_| value.to_string())
}

/// One CSV column: its header and the SQL expression rendering it as text. `NULL` becomes an
/// empty field.
#[derive(Clone, Copy, Debug)]
struct ReportColumn {
    header: &'static str,
    expression: &'static str,
}

impl ReportColumn {
    const fn new(header: &'static str, expression: &'static str) -> Self {
        Self { header, expression }
    }
}

/// The rows a report may include, taken from the requester's auth. Mirrors the filters the
/// analytics query builder applies for the same `AuthInfo`.
#[derive(Clone, Debug)]
pub struct ReportScope {
    org_id: String,
    merchant_ids: Option<Vec<String>>,
    processor_merchant_ids: Option<Vec<String>>,
    profile_ids: Option<Vec<String>>,
}

impl From<&AuthInfo> for ReportScope {
    fn from(auth: &AuthInfo) -> Self {
        let ids = |ids: &[common_utils::id_type::MerchantId]| {
            ids.iter()
                .map(|id| id.get_string_repr().to_owned())
                .collect::<Vec<_>>()
        };

        match auth {
            AuthInfo::OrgLevel { org_id } => Self {
                org_id: org_id.get_string_repr().to_owned(),
                merchant_ids: None,
                processor_merchant_ids: None,
                profile_ids: None,
            },
            AuthInfo::MerchantLevel {
                org_id,
                merchant_ids,
                processor_merchant_ids,
            } => Self {
                org_id: org_id.get_string_repr().to_owned(),
                merchant_ids: Some(ids(merchant_ids)),
                processor_merchant_ids: processor_merchant_ids.as_deref().map(ids),
                profile_ids: None,
            },
            AuthInfo::ProfileLevel {
                org_id,
                merchant_id,
                profile_ids,
                processor_merchant_id,
            } => Self {
                org_id: org_id.get_string_repr().to_owned(),
                merchant_ids: Some(vec![merchant_id.get_string_repr().to_owned()]),
                processor_merchant_ids: processor_merchant_id
                    .as_ref()
                    .map(|id| vec![id.get_string_repr().to_owned()]),
                profile_ids: Some(
                    profile_ids
                        .iter()
                        .map(|id| id.get_string_repr().to_owned())
                        .collect(),
                ),
            },
        }
    }
}

/// One page of report rows, already encoded as CSV.
#[derive(Debug)]
pub struct ReportPage<C> {
    pub csv: Vec<u8>,
    pub row_count: usize,
    /// Where the next page starts; `None` once the scan is exhausted.
    pub next_cursor: Option<C>,
}

/// Every report reads through the Postgres pool; ClickHouse-only deployments cannot serve them.
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

/// Whether reports can be generated with this provider, so a request fails up front instead of
/// in the background.
pub fn is_supported(provider: &AnalyticsProvider) -> bool {
    report_pool(provider).is_ok()
}

/// RFC 4180: every field quoted, quotes doubled, CRLF line endings.
fn push_csv_record<I, S>(out: &mut Vec<u8>, fields: I)
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    for (index, field) in fields.into_iter().enumerate() {
        if index > 0 {
            out.push(b',');
        }
        out.push(b'"');
        for byte in field.as_ref().bytes() {
            if byte == b'"' {
                out.push(b'"');
            }
            out.push(byte);
        }
        out.push(b'"');
    }
    out.extend_from_slice(b"\r\n");
}

fn csv_header(columns: &[ReportColumn]) -> Vec<u8> {
    let mut out = Vec::new();
    push_csv_record(&mut out, columns.iter().map(|column| column.header));
    out
}

fn render_rows(columns: &[ReportColumn], rows: &[PgRow]) -> CustomResult<Vec<u8>, AnalyticsError> {
    let mut out = Vec::new();
    for row in rows {
        let fields = (0..columns.len())
            .map(|index| {
                row.try_get::<Option<String>, _>(index)
                    .map(Option::unwrap_or_default)
            })
            .collect::<Result<Vec<_>, _>>()
            .change_context(AnalyticsError::UnknownError)
            .attach_printable("Failed to decode a report row")?;
        push_csv_record(&mut out, fields);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn csv_record_quotes_every_field() {
        let mut out = Vec::new();
        push_csv_record(&mut out, ["", "a\"b", "x\ny", "true", "1.50"]);
        assert_eq!(out, b"\"\",\"a\"\"b\",\"x\ny\",\"true\",\"1.50\"\r\n");
    }

    #[test]
    fn timestamps_use_the_api_format() {
        assert_eq!(
            format_timestamp(time::macros::datetime!(2026-10-12 09:10:07.215205)),
            "2026-10-12T09:10:07.215Z"
        );
    }
}
