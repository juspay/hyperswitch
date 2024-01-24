use std::{fmt::Display, str::FromStr};

use api_models::analytics::refunds::RefundType;
use common_utils::errors::{CustomResult, ParsingError};
use diesel_models::enums::{
    AttemptStatus, AuthenticationType, Currency, PaymentMethod, RefundStatus,
};
use error_stack::{IntoReport, ResultExt};
use masking::PeekInterface;
use sqlx::{
    postgres::{PgArgumentBuffer, PgPoolOptions, PgRow, PgTypeInfo, PgValueRef},
    Decode, Encode,
    Error::ColumnNotFound,
    FromRow, Pool, Postgres, Row,
};
use storage_impl::config::Database;
use time::PrimitiveDateTime;

use super::{
    health_check::HealthCheck,
    query::{Aggregate, ToSql, Window},
    types::{
        AnalyticsCollection, AnalyticsDataSource, DBEnumWrapper, LoadRow, QueryExecutionError,
        TableEngine,
    },
};

#[derive(Debug, Clone)]
pub struct SqlxClient {
    pool: Pool<Postgres>,
}

impl Default for SqlxClient {
    fn default() -> Self {
        let database_url = format!(
            "postgres://{}:{}@{}:{}/{}",
            "db_user", "db_pass", "localhost", 5432, "hyperswitch_db"
        );
        Self {
            #[allow(clippy::expect_used)]
            pool: PgPoolOptions::new()
                .connect_lazy(&database_url)
                .expect("SQLX Pool Creation failed"),
        }
    }
}

impl SqlxClient {
    pub async fn from_conf(conf: &Database) -> Self {
        let password = &conf.password.peek();
        let database_url = format!(
            "postgres://{}:{}@{}:{}/{}",
            conf.username, password, conf.host, conf.port, conf.dbname
        );
        #[allow(clippy::expect_used)]
        let pool = PgPoolOptions::new()
            .max_connections(conf.pool_size)
            .acquire_timeout(std::time::Duration::from_secs(conf.connection_timeout))
            .connect_lazy(&database_url)
            .expect("SQLX Pool Creation failed");
        Self { pool }
    }
}

pub trait DbType {
    fn name() -> &'static str;
}

macro_rules! db_type {
    ($a: ident, $str: tt) => {
        impl DbType for $a {
            fn name() -> &'static str {
                stringify!($str)
            }
        }
    };
    ($a:ident) => {
        impl DbType for $a {
            fn name() -> &'static str {
                stringify!($a)
            }
        }
    };
}

db_type!(Currency);
db_type!(AuthenticationType);
db_type!(AttemptStatus);
db_type!(PaymentMethod, TEXT);
db_type!(RefundStatus);
db_type!(RefundType);

impl<'q, Type> Encode<'q, Postgres> for DBEnumWrapper<Type>
where
    Type: DbType + FromStr + Display,
{
    fn encode_by_ref(&self, buf: &mut PgArgumentBuffer) -> sqlx::encode::IsNull {
        self.0.to_string().encode(buf)
    }
    fn size_hint(&self) -> usize {
        self.0.to_string().size_hint()
    }
}

impl<'r, Type> Decode<'r, Postgres> for DBEnumWrapper<Type>
where
    Type: DbType + FromStr + Display,
{
    fn decode(
        value: PgValueRef<'r>,
    ) -> Result<Self, Box<dyn std::error::Error + 'static + Send + Sync>> {
        let str_value = <&'r str as Decode<'r, Postgres>>::decode(value)?;
        Type::from_str(str_value).map(DBEnumWrapper).or(Err(format!(
            "invalid value {:?} for enum {}",
            str_value,
            Type::name()
        )
        .into()))
    }
}

impl<Type> sqlx::Type<Postgres> for DBEnumWrapper<Type>
where
    Type: DbType + FromStr + Display,
{
    fn type_info() -> PgTypeInfo {
        PgTypeInfo::with_name(Type::name())
    }
}

impl<T> LoadRow<T> for SqlxClient
where
    for<'a> T: FromRow<'a, PgRow>,
{
    fn load_row(row: PgRow) -> CustomResult<T, QueryExecutionError> {
        T::from_row(&row)
            .into_report()
            .change_context(QueryExecutionError::RowExtractionFailure)
    }
}

impl super::payments::filters::PaymentFilterAnalytics for SqlxClient {}
impl super::payments::metrics::PaymentMetricAnalytics for SqlxClient {}
impl super::payments::distribution::PaymentDistributionAnalytics for SqlxClient {}
impl super::refunds::metrics::RefundMetricAnalytics for SqlxClient {}
impl super::refunds::filters::RefundFilterAnalytics for SqlxClient {}

#[async_trait::async_trait]
impl AnalyticsDataSource for SqlxClient {
    type Row = PgRow;

    async fn load_results<T>(&self, query: &str) -> CustomResult<Vec<T>, QueryExecutionError>
    where
        Self: LoadRow<T>,
    {
        sqlx::query(&format!("{query};"))
            .fetch_all(&self.pool)
            .await
            .into_report()
            .change_context(QueryExecutionError::DatabaseError)
            .attach_printable_lazy(|| format!("Failed to run query {query}"))?
            .into_iter()
            .map(Self::load_row)
            .collect::<Result<Vec<_>, _>>()
            .change_context(QueryExecutionError::RowExtractionFailure)
    }
}
#[async_trait::async_trait]
impl HealthCheck for SqlxClient {
    async fn deep_health_check(&self) -> CustomResult<(), QueryExecutionError> {
        sqlx::query("SELECT 1")
            .fetch_all(&self.pool)
            .await
            .map(|_| ())
            .into_report()
            .change_context(QueryExecutionError::DatabaseError)
    }
}

impl<'a> FromRow<'a, PgRow> for super::refunds::metrics::RefundMetricRow {
    fn from_row(row: &'a PgRow) -> sqlx::Result<Self> {
        let currency: Option<DBEnumWrapper<Currency>> =
            row.try_get("currency").or_else(|e| match e {
                ColumnNotFound(_) => Ok(Default::default()),
                e => Err(e),
            })?;
        let refund_status: Option<DBEnumWrapper<RefundStatus>> =
            row.try_get("refund_status").or_else(|e| match e {
                ColumnNotFound(_) => Ok(Default::default()),
                e => Err(e),
            })?;
        let connector: Option<String> = row.try_get("connector").or_else(|e| match e {
            ColumnNotFound(_) => Ok(Default::default()),
            e => Err(e),
        })?;
        let refund_type: Option<DBEnumWrapper<RefundType>> =
            row.try_get("refund_type").or_else(|e| match e {
                ColumnNotFound(_) => Ok(Default::default()),
                e => Err(e),
            })?;
        let total: Option<bigdecimal::BigDecimal> = row.try_get("total").or_else(|e| match e {
            ColumnNotFound(_) => Ok(Default::default()),
            e => Err(e),
        })?;
        let count: Option<i64> = row.try_get("count").or_else(|e| match e {
            ColumnNotFound(_) => Ok(Default::default()),
            e => Err(e),
        })?;
        // Removing millisecond precision to get accurate diffs against clickhouse
        let start_bucket: Option<PrimitiveDateTime> = row
            .try_get::<Option<PrimitiveDateTime>, _>("start_bucket")?
            .and_then(|dt| dt.replace_millisecond(0).ok());
        let end_bucket: Option<PrimitiveDateTime> = row
            .try_get::<Option<PrimitiveDateTime>, _>("end_bucket")?
            .and_then(|dt| dt.replace_millisecond(0).ok());
        Ok(Self {
            currency,
            refund_status,
            connector,
            refund_type,
            total,
            count,
            start_bucket,
            end_bucket,
        })
    }
}

impl<'a> FromRow<'a, PgRow> for super::payments::metrics::PaymentMetricRow {
    fn from_row(row: &'a PgRow) -> sqlx::Result<Self> {
        let currency: Option<DBEnumWrapper<Currency>> =
            row.try_get("currency").or_else(|e| match e {
                ColumnNotFound(_) => Ok(Default::default()),
                e => Err(e),
            })?;
        let status: Option<DBEnumWrapper<AttemptStatus>> =
            row.try_get("status").or_else(|e| match e {
                ColumnNotFound(_) => Ok(Default::default()),
                e => Err(e),
            })?;
        let connector: Option<String> = row.try_get("connector").or_else(|e| match e {
            ColumnNotFound(_) => Ok(Default::default()),
            e => Err(e),
        })?;
        let authentication_type: Option<DBEnumWrapper<AuthenticationType>> =
            row.try_get("authentication_type").or_else(|e| match e {
                ColumnNotFound(_) => Ok(Default::default()),
                e => Err(e),
            })?;
        let payment_method: Option<String> =
            row.try_get("payment_method").or_else(|e| match e {
                ColumnNotFound(_) => Ok(Default::default()),
                e => Err(e),
            })?;
        let payment_method_type: Option<String> =
            row.try_get("payment_method_type").or_else(|e| match e {
                ColumnNotFound(_) => Ok(Default::default()),
                e => Err(e),
            })?;
        let total: Option<bigdecimal::BigDecimal> = row.try_get("total").or_else(|e| match e {
            ColumnNotFound(_) => Ok(Default::default()),
            e => Err(e),
        })?;
        let count: Option<i64> = row.try_get("count").or_else(|e| match e {
            ColumnNotFound(_) => Ok(Default::default()),
            e => Err(e),
        })?;
        // Removing millisecond precision to get accurate diffs against clickhouse
        let start_bucket: Option<PrimitiveDateTime> = row
            .try_get::<Option<PrimitiveDateTime>, _>("start_bucket")?
            .and_then(|dt| dt.replace_millisecond(0).ok());
        let end_bucket: Option<PrimitiveDateTime> = row
            .try_get::<Option<PrimitiveDateTime>, _>("end_bucket")?
            .and_then(|dt| dt.replace_millisecond(0).ok());
        Ok(Self {
            currency,
            status,
            connector,
            authentication_type,
            payment_method,
            payment_method_type,
            total,
            count,
            start_bucket,
            end_bucket,
        })
    }
}

impl<'a> FromRow<'a, PgRow> for super::payments::distribution::PaymentDistributionRow {
    fn from_row(row: &'a PgRow) -> sqlx::Result<Self> {
        let currency: Option<DBEnumWrapper<Currency>> =
            row.try_get("currency").or_else(|e| match e {
                ColumnNotFound(_) => Ok(Default::default()),
                e => Err(e),
            })?;
        let status: Option<DBEnumWrapper<AttemptStatus>> =
            row.try_get("status").or_else(|e| match e {
                ColumnNotFound(_) => Ok(Default::default()),
                e => Err(e),
            })?;
        let connector: Option<String> = row.try_get("connector").or_else(|e| match e {
            ColumnNotFound(_) => Ok(Default::default()),
            e => Err(e),
        })?;
        let authentication_type: Option<DBEnumWrapper<AuthenticationType>> =
            row.try_get("authentication_type").or_else(|e| match e {
                ColumnNotFound(_) => Ok(Default::default()),
                e => Err(e),
            })?;
        let payment_method: Option<String> =
            row.try_get("payment_method").or_else(|e| match e {
                ColumnNotFound(_) => Ok(Default::default()),
                e => Err(e),
            })?;
        let payment_method_type: Option<String> =
            row.try_get("payment_method_type").or_else(|e| match e {
                ColumnNotFound(_) => Ok(Default::default()),
                e => Err(e),
            })?;
        let total: Option<bigdecimal::BigDecimal> = row.try_get("total").or_else(|e| match e {
            ColumnNotFound(_) => Ok(Default::default()),
            e => Err(e),
        })?;
        let count: Option<i64> = row.try_get("count").or_else(|e| match e {
            ColumnNotFound(_) => Ok(Default::default()),
            e => Err(e),
        })?;
        let error_message: Option<String> = row.try_get("error_message").or_else(|e| match e {
            ColumnNotFound(_) => Ok(Default::default()),
            e => Err(e),
        })?;
        // Removing millisecond precision to get accurate diffs against clickhouse
        let start_bucket: Option<PrimitiveDateTime> = row
            .try_get::<Option<PrimitiveDateTime>, _>("start_bucket")?
            .and_then(|dt| dt.replace_millisecond(0).ok());
        let end_bucket: Option<PrimitiveDateTime> = row
            .try_get::<Option<PrimitiveDateTime>, _>("end_bucket")?
            .and_then(|dt| dt.replace_millisecond(0).ok());
        Ok(Self {
            currency,
            status,
            connector,
            authentication_type,
            payment_method,
            payment_method_type,
            total,
            count,
            error_message,
            start_bucket,
            end_bucket,
        })
    }
}

impl<'a> FromRow<'a, PgRow> for super::payments::filters::FilterRow {
    fn from_row(row: &'a PgRow) -> sqlx::Result<Self> {
        let currency: Option<DBEnumWrapper<Currency>> =
            row.try_get("currency").or_else(|e| match e {
                ColumnNotFound(_) => Ok(Default::default()),
                e => Err(e),
            })?;
        let status: Option<DBEnumWrapper<AttemptStatus>> =
            row.try_get("status").or_else(|e| match e {
                ColumnNotFound(_) => Ok(Default::default()),
                e => Err(e),
            })?;
        let connector: Option<String> = row.try_get("connector").or_else(|e| match e {
            ColumnNotFound(_) => Ok(Default::default()),
            e => Err(e),
        })?;
        let authentication_type: Option<DBEnumWrapper<AuthenticationType>> =
            row.try_get("authentication_type").or_else(|e| match e {
                ColumnNotFound(_) => Ok(Default::default()),
                e => Err(e),
            })?;
        let payment_method: Option<String> =
            row.try_get("payment_method").or_else(|e| match e {
                ColumnNotFound(_) => Ok(Default::default()),
                e => Err(e),
            })?;
        let payment_method_type: Option<String> =
            row.try_get("payment_method_type").or_else(|e| match e {
                ColumnNotFound(_) => Ok(Default::default()),
                e => Err(e),
            })?;
        Ok(Self {
            currency,
            status,
            connector,
            authentication_type,
            payment_method,
            payment_method_type,
        })
    }
}

impl<'a> FromRow<'a, PgRow> for super::refunds::filters::RefundFilterRow {
    fn from_row(row: &'a PgRow) -> sqlx::Result<Self> {
        let currency: Option<DBEnumWrapper<Currency>> =
            row.try_get("currency").or_else(|e| match e {
                ColumnNotFound(_) => Ok(Default::default()),
                e => Err(e),
            })?;
        let refund_status: Option<DBEnumWrapper<RefundStatus>> =
            row.try_get("refund_status").or_else(|e| match e {
                ColumnNotFound(_) => Ok(Default::default()),
                e => Err(e),
            })?;
        let connector: Option<String> = row.try_get("connector").or_else(|e| match e {
            ColumnNotFound(_) => Ok(Default::default()),
            e => Err(e),
        })?;
        let refund_type: Option<DBEnumWrapper<RefundType>> =
            row.try_get("refund_type").or_else(|e| match e {
                ColumnNotFound(_) => Ok(Default::default()),
                e => Err(e),
            })?;
        Ok(Self {
            currency,
            refund_status,
            connector,
            refund_type,
        })
    }
}

impl ToSql<SqlxClient> for PrimitiveDateTime {
    fn to_sql(&self, _table_engine: &TableEngine) -> error_stack::Result<String, ParsingError> {
        Ok(self.to_string())
    }
}

impl ToSql<SqlxClient> for AnalyticsCollection {
    fn to_sql(&self, _table_engine: &TableEngine) -> error_stack::Result<String, ParsingError> {
        match self {
            Self::Payment => Ok("payment_attempt".to_string()),
            Self::Refund => Ok("refund".to_string()),
            Self::SdkEvents => Err(error_stack::report!(ParsingError::UnknownError)
                .attach_printable("SdkEvents table is not implemented for Sqlx"))?,
            Self::ApiEvents => Err(error_stack::report!(ParsingError::UnknownError)
                .attach_printable("ApiEvents table is not implemented for Sqlx"))?,
            Self::PaymentIntent => Ok("payment_intent".to_string()),
            Self::ConnectorEvents => Err(error_stack::report!(ParsingError::UnknownError)
                .attach_printable("ConnectorEvents table is not implemented for Sqlx"))?,
            Self::OutgoingWebhookEvent => Err(error_stack::report!(ParsingError::UnknownError)
                .attach_printable("OutgoingWebhookEvents table is not implemented for Sqlx"))?,
        }
    }
}

impl<T> ToSql<SqlxClient> for Aggregate<T>
where
    T: ToSql<SqlxClient>,
{
    fn to_sql(&self, table_engine: &TableEngine) -> error_stack::Result<String, ParsingError> {
        Ok(match self {
            Self::Count { field: _, alias } => {
                format!(
                    "count(*){}",
                    alias.map_or_else(|| "".to_owned(), |alias| format!(" as {}", alias))
                )
            }
            Self::Sum { field, alias } => {
                format!(
                    "sum({}){}",
                    field
                        .to_sql(table_engine)
                        .attach_printable("Failed to sum aggregate")?,
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

impl<T> ToSql<SqlxClient> for Window<T>
where
    T: ToSql<SqlxClient>,
{
    fn to_sql(&self, table_engine: &TableEngine) -> error_stack::Result<String, ParsingError> {
        Ok(match self {
            Self::Sum {
                field,
                partition_by,
                order_by,
                alias,
            } => {
                format!(
                    "sum({}) over ({}{}){}",
                    field
                        .to_sql(table_engine)
                        .attach_printable("Failed to sum window")?,
                    partition_by.as_ref().map_or_else(
                        || "".to_owned(),
                        |partition_by| format!("partition by {}", partition_by.to_owned())
                    ),
                    order_by.as_ref().map_or_else(
                        || "".to_owned(),
                        |(order_column, order)| format!(
                            " order by {} {}",
                            order_column.to_owned(),
                            order.to_string()
                        )
                    ),
                    alias.map_or_else(|| "".to_owned(), |alias| format!(" as {}", alias))
                )
            }
            Self::RowNumber {
                field: _,
                partition_by,
                order_by,
                alias,
            } => {
                format!(
                    "row_number() over ({}{}){}",
                    partition_by.as_ref().map_or_else(
                        || "".to_owned(),
                        |partition_by| format!("partition by {}", partition_by.to_owned())
                    ),
                    order_by.as_ref().map_or_else(
                        || "".to_owned(),
                        |(order_column, order)| format!(
                            " order by {} {}",
                            order_column.to_owned(),
                            order.to_string()
                        )
                    ),
                    alias.map_or_else(|| "".to_owned(), |alias| format!(" as {}", alias))
                )
            }
        })
    }
}
