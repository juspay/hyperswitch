//! Reports replacing the reporting Lambdas.
//!
//! A report is a [`ReportDefinition`]: where its rows come from and the SQL rendering each column
//! as text. [`ReportKind::query`] builds any definition's query, which the store runs on the read
//! replica. Postgres renders the values, so the CSV carries
//! standard ones: JSON as JSON, booleans as `true`/`false`, timestamps as ISO 8601 UTC with
//! milliseconds (the format API responses use) and amounts as exact decimals.

pub mod payments;

use std::fmt::Write as _;

use api_models::analytics::{GenerateReportRequest, ReportColumns};
use common_utils::errors::CustomResult;

use crate::{enums::AuthInfo, errors::AnalyticsError};

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

/// A value bound to one of a report query's `$n` placeholders.
#[derive(Clone, Debug)]
pub enum ReportBind {
    Text(String),
    Texts(Vec<String>),
    Timestamp(time::PrimitiveDateTime),
}

/// A report's query and the CSV header for it. Each row has one column, `cells`: the requested
/// columns as `text[]`, in header order.
#[derive(Clone, Debug)]
pub struct ReportQuery {
    pub sql: String,
    pub binds: Vec<ReportBind>,
    pub header: Vec<u8>,
}

impl ReportQuery {
    fn bind(&mut self, value: ReportBind) -> String {
        self.binds.push(value);
        format!("${}", self.binds.len())
    }
}

impl ReportKind {
    /// The query for the requested columns of the rows `request.auth` may see, created within
    /// its time range (inclusive at both ends, as the Lambda was), oldest first.
    pub fn query(
        self,
        request: &GenerateReportRequest,
    ) -> CustomResult<ReportQuery, AnalyticsError> {
        let definition = self.definition();
        let columns = self.columns(request.request.columns.as_ref())?;
        let table = definition.table;
        let time_range = request.request.time_range;

        let cells = columns
            .iter()
            .map(|(_, expression)| *expression)
            .collect::<Vec<_>>()
            .join(", ");
        let mut query = ReportQuery {
            sql: format!(
                "SELECT ARRAY[{cells}]::text[] AS cells FROM {} WHERE TRUE",
                definition.from
            ),
            binds: Vec::new(),
            header: csv_record(columns.iter().map(|(header, _)| *header)),
        };
        push_scope(&mut query, definition, &request.auth);
        let start = query.bind(ReportBind::Timestamp(time_range.start_time));
        let end = query.bind(ReportBind::Timestamp(
            time_range
                .end_time
                .unwrap_or_else(common_utils::date_time::now),
        ));
        let _ = write!(
            query.sql,
            " AND {table}.created_at >= {start} AND {table}.created_at <= {end} \
             ORDER BY {table}.created_at"
        );
        Ok(query)
    }
}

/// The same scope the analytics query builder applies for `auth`. The Lambda ignored
/// `processor_merchant_id`, so a connected merchant received its platform's other rows.
fn push_scope(query: &mut ReportQuery, report: &ReportDefinition, auth: &AuthInfo) {
    let strings = |ids: &[common_utils::id_type::MerchantId]| {
        ids.iter()
            .map(|id| id.get_string_repr().to_owned())
            .collect::<Vec<_>>()
    };
    let (org_id, merchant_ids, processor_merchant_ids, profile_ids) = match auth {
        AuthInfo::OrgLevel { org_id } => (org_id, None, None, None),
        AuthInfo::MerchantLevel {
            org_id,
            merchant_ids,
            processor_merchant_ids,
        } => (
            org_id,
            Some(strings(merchant_ids)),
            processor_merchant_ids.as_deref().map(strings),
            None,
        ),
        AuthInfo::ProfileLevel {
            org_id,
            merchant_id,
            profile_ids,
            processor_merchant_id,
        } => (
            org_id,
            Some(strings(std::slice::from_ref(merchant_id))),
            processor_merchant_id
                .as_ref()
                .map(|id| strings(std::slice::from_ref(id))),
            Some(
                profile_ids
                    .iter()
                    .map(|id| id.get_string_repr().to_owned())
                    .collect(),
            ),
        ),
    };

    let table = report.table;
    if report.organization_scoped {
        let org_id = query.bind(ReportBind::Text(org_id.get_string_repr().to_owned()));
        let _ = write!(query.sql, " AND {table}.organization_id = {org_id}");
    }
    for (column, values) in [
        ("merchant_id", merchant_ids),
        ("processor_merchant_id", processor_merchant_ids),
        ("profile_id", profile_ids),
    ] {
        if let Some(values) = values {
            let values = query.bind(ReportBind::Texts(values));
            let _ = write!(query.sql, " AND {table}.{column} = ANY({values})");
        }
    }
}

/// One RFC 4180 record: every field quoted, quotes doubled, CRLF line ending.
pub fn csv_record<S: AsRef<str>>(fields: impl IntoIterator<Item = S>) -> Vec<u8> {
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
    #[allow(clippy::unwrap_used)]
    fn query_binds_scope_and_range_for_a_profile() {
        use std::borrow::Cow;

        use api_models::analytics::{payments::PaymentReportColumn as Column, ReportRequest};
        use common_utils::id_type;

        let request = GenerateReportRequest {
            request: ReportRequest {
                time_range: common_utils::types::TimeRange {
                    start_time: time::macros::datetime!(2026-10-01 00:00:00),
                    end_time: Some(time::macros::datetime!(2026-10-07 00:00:00)),
                },
                emails: None,
                return_url: None,
                #[cfg(feature = "v2")]
                report_type: Default::default(),
                columns: Some(ReportColumns::Payment(vec![
                    Column::PaymentId,
                    Column::Amount,
                ])),
            },
            merchant_id: None,
            auth: AuthInfo::ProfileLevel {
                org_id: id_type::OrganizationId::try_from(Cow::from("org_1")).unwrap(),
                merchant_id: id_type::MerchantId::try_from(Cow::from("merchant_1")).unwrap(),
                profile_ids: vec![id_type::ProfileId::try_from(Cow::from("pro_1")).unwrap()],
                processor_merchant_id: None,
            },
            email: common_utils::pii::Email::try_from("a@example.com".to_owned()).unwrap(),
            payment_response_hash_key: None,
        };

        let query = ReportKind::V1Payments.query(&request).unwrap();
        assert!(query
            .sql
            .starts_with("SELECT ARRAY[pi.payment_id, (CASE WHEN"));
        assert!(query.sql.ends_with(
            " AND pa.organization_id = $1 AND pa.merchant_id = ANY($2) \
             AND pa.profile_id = ANY($3) AND pa.created_at >= $4 AND pa.created_at <= $5 \
             ORDER BY pa.created_at"
        ));
        assert_eq!(query.binds.len(), 5);
        assert_eq!(query.header, b"\"payment_id\",\"amount\"\r\n");
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
