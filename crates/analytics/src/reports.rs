//! Reports generated from the analytics Postgres pool, replacing the reporting Lambdas.
//!
//! A report is a [`ReportDefinition`]: where its rows come from and the SQL rendering each column
//! as text. [`write_csv`] runs any definition. Postgres renders the values, so the CSV carries
//! standard ones: JSON as JSON, booleans as `true`/`false`, timestamps as ISO 8601 UTC with
//! milliseconds (the format API responses use) and amounts as exact decimals.

pub mod payments;

use api_models::analytics::{GenerateReportRequest, ReportColumns};
use common_utils::{errors::CustomResult, DbConnectionParams};
use error_stack::ResultExt;
use futures::TryStreamExt;
use sqlx::{Postgres, QueryBuilder, Row};
use storage_impl::config::Database;

use crate::{enums::AuthInfo, errors::AnalyticsError, ProcessTrackerReportConfig};

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

    /// The columns `requested` selects, in its order and each once; every column when it names
    /// none. Rejects an empty list and names the report does not define.
    pub fn columns(
        self,
        requested: Option<&ReportColumns>,
    ) -> CustomResult<Vec<&'static (&'static str, &'static str)>, AnalyticsError> {
        let definition = self.definition();
        let names: Vec<&str> = match (self, requested) {
            (_, None) => return Ok(definition.columns.iter().collect()),
            (Self::V1Payments, Some(ReportColumns::Payment(columns))) => {
                columns.iter().map(AsRef::as_ref).collect()
            }
        };
        if names.is_empty() {
            return Err(error_stack::report!(AnalyticsError::InvalidReportColumns(
                "at least one column must be requested".to_owned()
            )));
        }

        let mut selected = Vec::with_capacity(names.len());
        for name in names {
            let column = definition
                .columns
                .iter()
                .find(|(header, _)| *header == name)
                .ok_or_else(|| {
                    error_stack::report!(AnalyticsError::InvalidReportColumns(format!(
                        "`{name}` is not a column of the {} report",
                        definition.name
                    )))
                })?;
            if !selected.contains(&column) {
                selected.push(column);
            }
        }
        Ok(selected)
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

/// A tenant's pool on the database reports read: the router's `replica_database`, so point that at
/// a reader to keep reports off the primary. Built once at startup and connected lazily, so a
/// deployment that never runs a report opens no connections. Its `max_pool_size` caps how many
/// reports query at once: further reports wait for a connection.
#[derive(Clone, Debug)]
pub struct ReportDatabase {
    pool: sqlx::PgPool,
}

impl ReportDatabase {
    pub fn connect_lazy(database: &Database, schema: &str) -> CustomResult<Self, AnalyticsError> {
        let pool = sqlx::postgres::PgPoolOptions::new()
            .max_connections(database.max_pool_size.max(1))
            .min_connections(0)
            .acquire_timeout(std::time::Duration::from_secs(database.connection_timeout))
            .idle_timeout(std::time::Duration::from_secs(database.idle_timeout))
            .max_lifetime(std::time::Duration::from_secs(database.max_lifetime))
            .connect_lazy(&database.get_database_url(schema))
            .change_context(AnalyticsError::UnknownError)
            .attach_printable("Invalid report database configuration")?;
        Ok(Self { pool })
    }

    /// Connects and runs `SELECT 1`, so a misconfigured database shows at startup rather than on
    /// the first report.
    pub async fn check(&self) -> CustomResult<(), AnalyticsError> {
        sqlx::query("SELECT 1")
            .execute(&self.pool)
            .await
            .change_context(AnalyticsError::UnknownError)
            .attach_printable("The report database is unreachable")?;
        Ok(())
    }
}

/// Receives a report's CSV as [`write_csv`] produces it.
#[async_trait::async_trait]
pub trait CsvSink: Send {
    async fn write(&mut self, bytes: &[u8]) -> CustomResult<(), AnalyticsError>;
}

/// Rows read ahead of the upload; bounds memory when storage is slower than the database.
const ROW_BUFFER: usize = 1024;

/// Streams the requested columns of the rows `request.auth` may see, created within its time
/// range (inclusive at both ends, as the Lambda was), to `sink` as CSV records, header first.
/// Returns the number of rows.
///
/// The query runs on the tenant's `database`, in a read-only transaction bounded by
/// `settings.query_timeout_secs`.
pub async fn write_csv(
    database: &ReportDatabase,
    report: ReportKind,
    request: &GenerateReportRequest,
    settings: &ProcessTrackerReportConfig,
    sink: &mut dyn CsvSink,
) -> CustomResult<usize, AnalyticsError> {
    let columns = report.columns(request.request.columns.as_ref())?;
    sink.write(&csv_record(columns.iter().map(|(header, _)| *header)))
        .await?;

    // The query runs on its own task, so reading rows overlaps uploading them.
    let (sender, mut receiver) = tokio::sync::mpsc::channel(ROW_BUFFER);
    let reader = tokio::spawn(read_rows(
        database.pool.clone(),
        report,
        request.clone(),
        settings.clone(),
        sender,
    ));

    let mut row_count = 0;
    while let Some(record) = receiver.recv().await {
        // Returning drops the receiver, which ends the reader at its next row.
        sink.write(&record).await?;
        row_count += 1;
    }
    reader
        .await
        .change_context(AnalyticsError::UnknownError)
        .attach_printable("The report reader stopped unexpectedly")??;
    Ok(row_count)
}

async fn read_rows(
    pool: sqlx::PgPool,
    report: ReportKind,
    request: GenerateReportRequest,
    settings: ProcessTrackerReportConfig,
    sender: tokio::sync::mpsc::Sender<Vec<u8>>,
) -> CustomResult<(), AnalyticsError> {
    let definition = report.definition();
    let columns = report.columns(request.request.columns.as_ref())?;
    let time_range = request.request.time_range;
    let table = definition.table;

    let mut query = QueryBuilder::<Postgres>::new("SELECT ");
    let mut select = query.separated(", ");
    for (_, expression) in &columns {
        select.push(*expression);
    }
    query.push(format_args!(" FROM {} WHERE TRUE", definition.from));
    push_scope(&mut query, definition, &request.auth);
    query.push(format_args!(" AND {table}.created_at >= "));
    query.push_bind(time_range.start_time);
    query.push(format_args!(" AND {table}.created_at <= "));
    query.push_bind(
        time_range
            .end_time
            .unwrap_or_else(common_utils::date_time::now),
    );
    query.push(format_args!(" ORDER BY {table}.created_at"));

    let mut transaction = pool
        .begin()
        .await
        .change_context(AnalyticsError::UnknownError)?;
    let timeout_ms = settings.query_timeout_secs.saturating_mul(1000);
    let timeout = format!("SET LOCAL statement_timeout = {timeout_ms}");
    for setting in ["SET TRANSACTION READ ONLY", timeout.as_str()] {
        run_statement(&mut transaction, setting)
            .await
            .change_context(AnalyticsError::UnknownError)
            .attach_printable("Failed to prepare the report transaction")?;
    }

    let mut rows = stream_rows(&mut transaction, query.build());
    while let Some(row) = rows
        .try_next()
        .await
        .change_context(AnalyticsError::UnknownError)
        .attach_printable_lazy(|| format!("Failed to read the {} report", definition.name))?
    {
        let fields = (0..columns.len())
            .map(|index| row.try_get::<Option<String>, _>(index))
            .collect::<Result<Vec<_>, _>>()
            .change_context(AnalyticsError::UnknownError)
            .attach_printable("Failed to decode a report row")?;
        let record = csv_record(fields.into_iter().map(Option::unwrap_or_default));
        if sender.send(record).await.is_err() {
            // The writer gave up; the transaction rolls back when dropped.
            return Ok(());
        }
    }
    Ok(())
}

// These two pin the connection's lifetime. Inferred inside an async body, it trips the
// higher-ranked `Executor` check (rust-lang/rust#102211).
fn run_statement<'a>(
    connection: &'a mut sqlx::PgConnection,
    sql: &'a str,
) -> futures::future::BoxFuture<'a, Result<sqlx::postgres::PgQueryResult, sqlx::Error>> {
    Box::pin(sqlx::query(sql).execute(connection))
}

fn stream_rows<'a>(
    connection: &'a mut sqlx::PgConnection,
    query: sqlx::query::Query<'a, Postgres, sqlx::postgres::PgArguments>,
) -> futures::stream::BoxStream<'a, Result<sqlx::postgres::PgRow, sqlx::Error>> {
    query.fetch(connection)
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
    fn requested_columns_are_selected_in_order() {
        use api_models::analytics::payments::PaymentReportColumn as Column;

        let headers = |requested: Option<ReportColumns>| {
            ReportKind::V1Payments
                .columns(requested.as_ref())
                .map(|columns| columns.iter().map(|(h, _)| *h).collect::<Vec<_>>())
        };

        assert_eq!(
            headers(None).ok().map(|all| all.len()),
            Some(payments::V1_PAYMENTS.columns.len())
        );
        assert_eq!(
            headers(Some(ReportColumns::Payment(vec![
                Column::Amount,
                Column::PaymentId,
                Column::Amount,
            ])))
            .ok(),
            Some(vec!["amount", "payment_id"])
        );
        assert!(headers(Some(ReportColumns::Payment(Vec::new()))).is_err());
    }

    #[test]
    fn every_payment_report_column_can_be_requested() {
        use api_models::analytics::payments::PaymentReportColumn;
        use strum::IntoEnumIterator;

        let all = PaymentReportColumn::iter().collect::<Vec<_>>();
        let selected = ReportKind::V1Payments.columns(Some(&ReportColumns::Payment(all.clone())));
        assert_eq!(selected.map(|columns| columns.len()).ok(), Some(all.len()));
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
