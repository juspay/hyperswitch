//! Report generation that used to run in the reporting Lambdas, read from the analytics Postgres
//! pool and rendered byte-compatible with the CSVs those Lambdas produced.

pub mod payments;
pub mod python_compat;

use common_utils::errors::CustomResult;
use error_stack::ResultExt;
use sqlx::{postgres::PgRow, Row};

use crate::{enums::AuthInfo, errors::AnalyticsError, AnalyticsProvider};

/// How a selected column is decoded and then rendered as Python's `str()` would.
#[derive(Clone, Copy, Debug)]
enum ColumnKind {
    Text,
    Float,
    Bool,
    Timestamp,
    /// The column's JSON text; rendered as the `repr` of the parsed value.
    Json,
}

/// One CSV column: its header and the SQL expression producing it.
#[derive(Clone, Copy, Debug)]
struct ReportColumn {
    header: &'static str,
    expression: &'static str,
    kind: ColumnKind,
}

impl ReportColumn {
    const fn new(header: &'static str, expression: &'static str, kind: ColumnKind) -> Self {
        Self {
            header,
            expression,
            kind,
        }
    }

    fn render(&self, row: &PgRow, index: usize) -> Result<String, sqlx::Error> {
        Ok(match self.kind {
            ColumnKind::Text => row.try_get::<Option<String>, _>(index)?.unwrap_or_default(),
            ColumnKind::Float => row
                .try_get::<Option<f64>, _>(index)?
                .map(python_compat::float_repr)
                .unwrap_or_default(),
            ColumnKind::Bool => row
                .try_get::<Option<bool>, _>(index)?
                .map(|value| python_compat::bool_str(value).to_owned())
                .unwrap_or_default(),
            ColumnKind::Timestamp => row
                .try_get::<Option<time::PrimitiveDateTime>, _>(index)?
                .map(|value| python_compat::datetime_str(value, ' '))
                .unwrap_or_default(),
            ColumnKind::Json => {
                python_compat::json_field(row.try_get::<Option<String>, _>(index)?.as_deref())
            }
        })
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

/// `csv.writer(quoting=csv.QUOTE_ALL)`: every field quoted, quotes doubled, CRLF line endings.
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
        let fields = columns
            .iter()
            .enumerate()
            .map(|(index, column)| column.render(row, index))
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
    fn csv_record_matches_python_quote_all() {
        // From CPython: csv.writer(quoting=csv.QUOTE_ALL).writerow([None, 'a"b', 'x\ny', True, 1.5])
        let mut out = Vec::new();
        push_csv_record(&mut out, ["", "a\"b", "x\ny", "True", "1.5"]);
        assert_eq!(out, b"\"\",\"a\"\"b\",\"x\ny\",\"True\",\"1.5\"\r\n");
    }
}
