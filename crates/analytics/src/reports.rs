//! Reports replacing the reporting Lambdas.
//!
//! A report is a [`ReportDefinition`]: where its rows come from and the SQL rendering each column
//! as text. [`ReportKind::query`] builds any definition's query, which the store runs on the read
//! replica. Postgres renders the values, so the CSV carries
//! standard ones: JSON as JSON, booleans as `true`/`false`, timestamps as ISO 8601 UTC with
//! milliseconds (the format API responses use) and amounts as exact decimals.

mod definitions;

use std::fmt::Write as _;

use api_models::analytics::{GenerateReportRequest, ReportColumns};
use common_utils::errors::CustomResult;

use crate::{enums::AuthInfo, errors::AnalyticsError, AnalyticsFlow};

/// Where a report's rows come from and the columns written for each.
#[derive(Debug)]
pub struct ReportDefinition {
    /// Names the report in file keys and file names, e.g. `v1_payments`.
    pub name: &'static str,
    /// `data.report_type` in the report webhook, e.g. `payment_report`.
    pub webhook_type: &'static str,
    /// Names the report in the email's subject and link, e.g. `Refunds`.
    pub title: &'static str,
    /// The report, as the email's body names it: "the `refund` report".
    pub noun: &'static str,
    /// The rows, as the email's body describes them: "an overview of `refunds made by your
    /// customers` via Hyperswitch".
    pub covers: &'static str,
    /// The `FROM` clause, joins included.
    from: &'static str,
    /// The alias whose `organization_id`, `merchant_id`, `processor_merchant_id`, `profile_id`
    /// and `created_at` scope the rows.
    table: &'static str,
    /// Whether `table` has an `organization_id` column; `relay` does not.
    organization_scoped: bool,
    columns: &'static [Column],
}

/// One CSV column, rendered to text by Postgres. `NULL` becomes an empty field.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Column {
    /// A column cast to text, headed by its name: `ref.refund_id` heads `refund_id`.
    Text(&'static str),
    /// A timestamp column, as ISO 8601 UTC with milliseconds.
    Time(&'static str),
    /// Any other expression, with its header.
    Expr(&'static str, &'static str),
}

impl Column {
    fn header(self) -> &'static str {
        match self {
            Self::Text(column) | Self::Time(column) => {
                column.rsplit_once('.').map_or(column, |(_, name)| name)
            }
            Self::Expr(header, _) => header,
        }
    }

    fn sql(self) -> String {
        match self {
            Self::Text(column) => format!("{column}::text"),
            // TIMESTAMP_FORMAT for `to_char`, so CSV cells match the webhook and email.
            Self::Time(column) => format!("to_char({column}, 'YYYY-MM-DD\"T\"HH24:MI:SS.MS\"Z\"')"),
            Self::Expr(_, expression) => expression.to_owned(),
        }
    }
}

/// The reports that can be generated.
#[derive(
    Clone,
    Copy,
    Debug,
    PartialEq,
    Eq,
    Hash,
    serde::Serialize,
    serde::Deserialize,
    strum::Display,
    strum::EnumIter,
)]
#[strum(serialize_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum ReportKind {
    V1Payments,
    Refunds,
    Disputes,
    Payouts,
    Relay,
    Authentications,
    V2Payments,
    RevenueRecovery,
}

impl ReportKind {
    pub fn definition(self) -> &'static ReportDefinition {
        match self {
            Self::V1Payments => &definitions::V1_PAYMENTS,
            Self::Refunds => &definitions::REFUNDS,
            Self::Disputes => &definitions::DISPUTES,
            Self::Payouts => &definitions::PAYOUTS,
            Self::Relay => &definitions::RELAY,
            Self::Authentications => &definitions::AUTHENTICATIONS,
            Self::V2Payments => &definitions::V2_PAYMENTS,
            Self::RevenueRecovery => &definitions::REVENUE_RECOVERY,
        }
    }

    /// The report `request` asks for from `self`'s routes: on v2 the payment routes serve the
    /// report its `report_type` names, as the payment Lambda did.
    pub fn requested_by(self, request: &api_models::analytics::ReportRequest) -> Self {
        #[cfg(feature = "v2")]
        if self == Self::V1Payments {
            use api_models::analytics::ReportType;
            return match request.report_type {
                ReportType::V2Payments => Self::V2Payments,
                ReportType::RevenueRecovery => Self::RevenueRecovery,
            };
        }
        let _ = request;
        self
    }

    /// The flow the report's API requests are logged under.
    pub fn flow(self) -> AnalyticsFlow {
        match self {
            Self::V1Payments | Self::V2Payments | Self::RevenueRecovery => {
                AnalyticsFlow::GeneratePaymentReport
            }
            Self::Refunds => AnalyticsFlow::GenerateRefundReport,
            Self::Disputes => AnalyticsFlow::GenerateDisputeReport,
            Self::Payouts => AnalyticsFlow::GeneratePayoutReport,
            Self::Relay => AnalyticsFlow::GenerateRelayReport,
            Self::Authentications => AnalyticsFlow::GenerateAuthenticationReport,
        }
    }

    /// The columns `requested` selects, in its order and each once; every column when it names
    /// none. Rejects an empty list, names the report does not define, and choices for a report
    /// whose columns cannot be chosen.
    fn columns(
        self,
        requested: Option<&ReportColumns>,
    ) -> CustomResult<Vec<Column>, AnalyticsError> {
        let invalid =
            |reason: String| error_stack::report!(AnalyticsError::InvalidReportColumns(reason));
        let definition = self.definition();
        let names: Vec<&str> = match (self, requested) {
            (_, None) => return Ok(definition.columns.to_vec()),
            (Self::V1Payments, Some(ReportColumns::Payment(columns))) => {
                columns.iter().map(AsRef::as_ref).collect()
            }
            (_, Some(_)) => {
                return Err(invalid(format!(
                    "columns cannot be chosen for the {} report",
                    definition.name
                )))
            }
        };
        if names.is_empty() {
            return Err(invalid("at least one column must be requested".to_owned()));
        }

        let mut selected = Vec::with_capacity(names.len());
        for name in names {
            let column = definition
                .columns
                .iter()
                .copied()
                .find(|column| column.header() == name)
                .ok_or_else(|| {
                    invalid(format!(
                        "`{name}` is not a column of the {} report",
                        definition.name
                    ))
                })?;
            if !selected.contains(&column) {
                selected.push(column);
            }
        }
        Ok(selected)
    }

    /// Checks the request's column choice, so a bad one is rejected when the report is requested.
    pub fn validate_columns(
        self,
        request: &GenerateReportRequest,
    ) -> CustomResult<(), AnalyticsError> {
        self.columns(request.request.columns.as_ref()).map(|_| ())
    }
}

/// The format of every timestamp in a report: ISO 8601 UTC with milliseconds.
const TIMESTAMP_FORMAT: &[time::format_description::FormatItem<'static>] = time::macros::format_description!(
    "[year]-[month]-[day]T[hour]:[minute]:[second].[subsecond digits:3]Z"
);

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
            .map(|column| column.sql())
            .collect::<Vec<_>>()
            .join(", ");
        let mut query = ReportQuery {
            sql: format!(
                "SELECT ARRAY[{cells}]::text[] AS cells FROM {} WHERE TRUE",
                definition.from
            ),
            binds: Vec::new(),
            header: csv_record(columns.iter().map(|column| column.header())),
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
        use api_models::analytics::payments::PaymentReportColumn as PaymentColumn;

        let headers = |requested: Option<ReportColumns>| {
            ReportKind::V1Payments
                .columns(requested.as_ref())
                .map(|columns| columns.iter().map(|c| c.header()).collect::<Vec<_>>())
        };

        assert_eq!(
            headers(None).ok().map(|all| all.len()),
            Some(definitions::V1_PAYMENTS.columns.len())
        );
        assert_eq!(
            headers(Some(ReportColumns::Payment(vec![
                PaymentColumn::Amount,
                PaymentColumn::PaymentId,
                PaymentColumn::Amount,
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

        use api_models::analytics::{
            payments::PaymentReportColumn as PaymentColumn, ReportRequest,
        };
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
                    PaymentColumn::PaymentId,
                    PaymentColumn::Amount,
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
            .starts_with("SELECT ARRAY[pi.payment_id::text, (CASE WHEN"));
        assert!(query.sql.ends_with(
            " AND pa.organization_id = $1 AND pa.merchant_id = ANY($2) \
             AND pa.profile_id = ANY($3) AND pa.created_at >= $4 AND pa.created_at <= $5 \
             ORDER BY pa.created_at"
        ));
        assert_eq!(query.binds.len(), 5);
        assert_eq!(query.header, b"\"payment_id\",\"amount\"\r\n");
    }

    #[test]
    fn every_report_has_unique_headers() {
        use strum::IntoEnumIterator;

        for report in ReportKind::iter() {
            let headers = report
                .definition()
                .columns
                .iter()
                .map(|c| c.header())
                .collect::<Vec<_>>();
            let unique = headers.iter().collect::<std::collections::HashSet<_>>();
            assert_eq!(unique.len(), headers.len(), "{report:?}");
        }
        // The Lambdas' column counts.
        let counts = ReportKind::iter()
            .map(|report| report.definition().columns.len())
            .collect::<Vec<_>>();
        assert_eq!(counts, [35, 23, 16, 33, 13, 35, 31, 27]);
    }

    #[test]
    fn v2_payment_report_headers_match_lambda() {
        // BASE_CSV_COLUMNS and V2_PAYMENTS_EXTRA_CSV_COLUMNS from
        // weekly_payment_report_generator_step_1.
        let base = concat!(
            r#""id","attempt_id","status","amount","currency","connector","connector_payment_id","#,
            r#""amount_to_capture","customer_id","created_at","order_details","error_message","#,
            r#""capture_method","authentication_type","payment_method_type_v2","#,
            r#""payment_method_subtype","metadata","setup_future_usage","statement_descriptor","#,
            r#""description","payment_method_data","card_network","modified_at","error_code","#,
            r#""payment_method_id","merchant_reference_id","profile_id""#,
        );
        let header = |definition: &ReportDefinition| {
            String::from_utf8_lossy(&csv_record(definition.columns.iter().map(|c| c.header())))
                .into_owned()
        };
        assert_eq!(
            header(&definitions::REVENUE_RECOVERY),
            format!("{base}\r\n")
        );
        assert_eq!(
            header(&definitions::V2_PAYMENTS),
            format!(
                "{base},\"mandate_id\",\"allowed_payment_method_types\",\"fingerprint_id\",\"card_holder_name\"\r\n"
            )
        );
    }

    #[test]
    fn columns_are_chosen_only_for_payments() {
        let choice = ReportColumns::Payment(vec![
            api_models::analytics::payments::PaymentReportColumn::Amount,
        ]);
        assert!(ReportKind::V1Payments.columns(Some(&choice)).is_ok());
        assert!(ReportKind::Refunds.columns(Some(&choice)).is_err());
        assert_eq!(
            ReportKind::Refunds.columns(None).map(|c| c.len()).ok(),
            Some(definitions::REFUNDS.columns.len())
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
        let header = csv_record(definitions::V1_PAYMENTS.columns.iter().map(|c| c.header()));
        assert_eq!(String::from_utf8_lossy(&header), expected);
    }
}
