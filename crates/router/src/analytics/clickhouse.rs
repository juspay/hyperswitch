use std::sync::Arc;

use bigdecimal::{BigDecimal, FromPrimitive};
use common_utils::errors::ParsingError;
use error_stack::{IntoReport, Report, ResultExt};
use http::StatusCode;
use router_env::logger;
use time::PrimitiveDateTime;

use super::{
    payments::{filters::FilterRow, metrics::PaymentMetricRow},
    query::{Aggregate, ToSql},
    refunds::{filters::RefundFilterRow, metrics::RefundMetricRow},
    types::{AnalyticsCollection, AnalyticsDataSource, LoadRow, QueryExecutionError},
};
use crate::analytics::types::TableEngine;

pub type ClickhouseResult<T> = error_stack::Result<T, ClickhouseError>;

#[derive(Clone, Debug)]
pub struct ClickhouseClient {
    pub config: Arc<ClickhouseConfig>,
}

#[derive(Clone, Debug, serde::Deserialize)]
pub struct ClickhouseConfig {
    username: String,
    password: Option<String>,
    host: String,
    database_name: String,
}

impl Default for ClickhouseConfig {
    fn default() -> Self {
        Self {
            username: "default".to_string(),
            password: None,
            host: "http://localhost:8123".to_string(),
            database_name: "default".to_string(),
        }
    }
}

impl ClickhouseClient {
    async fn execute_query(&self, query: &str) -> ClickhouseResult<Vec<CkhRow>> {
        logger::debug!("Executing query: {query}");
        let client = reqwest::Client::new();
        let params = CkhQuery {
            user: self.config.username.clone(),
            password: self.config.password.clone(),
            date_time_output_format: String::from("iso"),
            output_format_json_quote_64bit_integers: 0,
            database: self.config.database_name.clone(),
        };
        let response = client
            .post(&self.config.host)
            .query(&params)
            // TODO: This also assumes the clickhouse table to be collapsing merge with sign_flag as the default value
            .body(format!("{query}\nFORMAT JSON"))
            .send()
            .await
            .into_report()
            .change_context(ClickhouseError::ConnectionError)?;

        logger::debug!(clickhouse_response=?response, query=?query, "Clickhouse response");
        if response.status() != StatusCode::OK {
            response.text().await.map_or_else(
                |er| {
                    Err(ClickhouseError::ResponseError)
                        .into_report()
                        .attach_printable_lazy(|| format!("Error: {er:?}"))
                },
                |t| Err(ClickhouseError::ResponseNotOK(t)).into_report(),
            )
        } else {
            Ok(response
                .json::<CkhOutput<CkhRow>>() // TODO: Add prom metrics for clickhouse operations
                .await
                .into_report()
                .change_context(ClickhouseError::ResponseError)?
                .data)
        }
    }
}

#[async_trait::async_trait]
impl AnalyticsDataSource for ClickhouseClient {
    type Row = CkhRow;

    async fn load_results<T>(
        &self,
        query: &str,
    ) -> common_utils::errors::CustomResult<Vec<T>, QueryExecutionError>
    where
        Self: LoadRow<T>,
    {
        self.execute_query(query)
            .await
            .change_context(QueryExecutionError::DatabaseError)?
            .into_iter()
            .map(Self::load_row)
            .collect::<Result<Vec<_>, _>>()
            .change_context(QueryExecutionError::RowExtractionFailure)
    }

    fn get_table_engine(table: AnalyticsCollection) -> TableEngine {
        match table {
            AnalyticsCollection::Payment | AnalyticsCollection::Refund => {
                TableEngine::CollapsingMergeTree { sign: "sign_flag" }
            }
        }
    }
}

impl<T, E> LoadRow<T> for ClickhouseClient
where
    Self::Row: TryInto<T, Error = Report<E>>,
{
    fn load_row(row: Self::Row) -> common_utils::errors::CustomResult<T, QueryExecutionError> {
        row.try_into()
            .change_context(QueryExecutionError::RowExtractionFailure)
    }
}

impl super::payments::filters::PaymentFilterAnalytics for ClickhouseClient {}
impl super::payments::metrics::PaymentMetricAnalytics for ClickhouseClient {}
impl super::refunds::metrics::RefundMetricAnalytics for ClickhouseClient {}
impl super::refunds::filters::RefundFilterAnalytics for ClickhouseClient {}

#[derive(Debug, serde::Serialize)]
struct CkhQuery {
    user: String,
    password: Option<String>,
    date_time_output_format: String,
    output_format_json_quote_64bit_integers: u8,
    database: String,
}

#[derive(Debug, serde::Deserialize)]
struct CkhOutput<T> {
    data: Vec<T>,
}

#[derive(Debug, serde::Deserialize)]
pub struct CkhRow {
    pub currency: Option<String>,
    pub status: Option<String>,
    pub connector: Option<String>,
    pub authentication_type: Option<String>,
    pub payment_method: Option<String>,
    pub platform: Option<String>,
    pub browser_name: Option<String>,
    pub source: Option<String>,
    pub component: Option<String>,
    pub refund_type: Option<String>,
    pub refund_status: Option<String>,
    pub total: Option<i64>,
    pub count: Option<i64>,
    #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
    pub start_bucket: Option<PrimitiveDateTime>,
    #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
    pub end_bucket: Option<PrimitiveDateTime>,
    pub time_bucket: Option<String>,
}

impl TryInto<PaymentMetricRow> for CkhRow {
    type Error = Report<ParsingError>;

    fn try_into(self) -> Result<PaymentMetricRow, Self::Error> {
        Ok(PaymentMetricRow {
            currency: self.currency.map(|i| i.parse()).transpose()?,
            status: self.status.map(|i| i.parse()).transpose()?,
            connector: self.connector,
            authentication_type: self.authentication_type.map(|i| i.parse()).transpose()?,
            payment_method: self.payment_method,
            total: self.total.and_then(BigDecimal::from_i64),
            count: self.count,
            start_bucket: self.start_bucket,
            end_bucket: self.end_bucket,
        })
    }
}

impl TryInto<FilterRow> for CkhRow {
    type Error = Report<ParsingError>;

    fn try_into(self) -> Result<FilterRow, Self::Error> {
        Ok(FilterRow {
            currency: self.currency.map(|i| i.parse()).transpose()?,
            status: self.status.map(|i| i.parse()).transpose()?,
            connector: self.connector,
            authentication_type: self.authentication_type.map(|i| i.parse()).transpose()?,
            payment_method: self.payment_method,
        })
    }
}

impl TryInto<RefundMetricRow> for CkhRow {
    type Error = Report<ParsingError>;
    fn try_into(self) -> Result<RefundMetricRow, Self::Error> {
        Ok(RefundMetricRow {
            currency: self.currency.map(|i| i.parse()).transpose()?,
            refund_status: self.refund_status.map(|i| i.parse()).transpose()?,
            connector: self.connector,
            refund_type: self.refund_type.map(|i| i.parse()).transpose()?,
            total: self.total.and_then(BigDecimal::from_i64),
            count: self.count,
            start_bucket: self.start_bucket,
            end_bucket: self.end_bucket,
        })
    }
}

impl TryInto<RefundFilterRow> for CkhRow {
    type Error = Report<ParsingError>;
    fn try_into(self) -> Result<RefundFilterRow, Self::Error> {
        Ok(RefundFilterRow {
            currency: self.currency.map(|i| i.parse()).transpose()?,
            refund_status: self.refund_status.map(|i| i.parse()).transpose()?,
            connector: self.connector,
            refund_type: self.refund_type.map(|i| i.parse()).transpose()?,
        })
    }
}

impl ToSql<ClickhouseClient> for PrimitiveDateTime {
    fn to_sql(&self, _table_engine: &TableEngine) -> error_stack::Result<String, ParsingError> {
        let format =
            time::format_description::parse("[year]-[month]-[day] [hour]:[minute]:[second]")
                .into_report()
                // TODO: Add a datatime variant in the parsing error
                .change_context(ParsingError::UnknownError)
                .attach_printable("Failed to parse format description")?;
        self.format(&format)
            .into_report()
            .change_context(ParsingError::EncodeError(
                "failed to encode to clickhouse date-time format",
            ))
            .attach_printable("Failed to format date time")
    }
}

impl ToSql<ClickhouseClient> for AnalyticsCollection {
    fn to_sql(&self, _table_engine: &TableEngine) -> error_stack::Result<String, ParsingError> {
        match self {
            Self::Payment => Ok("payment_attempt_dist".to_string()),
            Self::Refund => Ok("refund_dist".to_string()),
        }
    }
}

// This assumes that the underlying table is CollapsingMergeTree with the sign set to sign_flag
// Please don't use this for normal merge tables, or if the sign_flag is a different column
// TODO: Make this more generic where the ToSQL method can be derived independently for each table
// and not just the source type
// https://github.com/juspay/hyperswitch-cloud/issues/429
impl<T> ToSql<ClickhouseClient> for Aggregate<T>
where
    T: ToSql<ClickhouseClient>,
{
    fn to_sql(&self, table_engine: &TableEngine) -> error_stack::Result<String, ParsingError> {
        Ok(match self {
            Self::Count { field: _, alias } => {
                let query = match table_engine {
                    TableEngine::CollapsingMergeTree { sign } => format!("sum({sign})"),
                    TableEngine::BasicTree => "count(*)".to_string(),
                };
                format!(
                    "{query}{}",
                    alias.map_or_else(|| "".to_owned(), |alias| format!(" as {}", alias))
                )
            }
            Self::Sum { field, alias } => {
                let query = match table_engine {
                    TableEngine::CollapsingMergeTree { sign } => format!(
                        "sum({sign} * {})",
                        field
                            .to_sql(table_engine)
                            .attach_printable("Failed to sum aggregate")?
                    ),
                    TableEngine::BasicTree => format!(
                        "sum({})",
                        field
                            .to_sql(table_engine)
                            .attach_printable("Failed to sum aggregate")?
                    ),
                };
                format!(
                    "{query}{}",
                    alias.map_or_else(|| "".to_owned(), |alias| format!(" as {}", alias))
                )
            }
            Self::Min { field, alias } => {
                format!(
                    "min({}){}",
                    field
                        .to_sql(table_engine)
                        .attach_printable("Failed to min aggregate")?,
                    alias.map_or_else(|| "".to_owned(), |alias| format!(" as {}", alias))
                )
            }
            Self::Max { field, alias } => {
                format!(
                    "max({}){}",
                    field
                        .to_sql(table_engine)
                        .attach_printable("Failed to max aggregate")?,
                    alias.map_or_else(|| "".to_owned(), |alias| format!(" as {}", alias))
                )
            }
        })
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ClickhouseError {
    #[error("Clickhouse connection error")]
    ConnectionError,
    #[error("Clickhouse NON-200 response content: '{0}'")]
    ResponseNotOK(String),
    #[error("Clickhouse response error")]
    ResponseError,
}
