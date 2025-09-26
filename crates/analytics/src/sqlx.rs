use std::{fmt::Display, str::FromStr};

use api_models::{
    analytics::{frm::FrmTransactionType, refunds::RefundType},
    enums::{DisputeStage, DisputeStatus},
};
use common_enums::{
    AuthenticationConnectors, AuthenticationStatus, DecoupledAuthenticationType, TransactionStatus,
};
use common_utils::{
    errors::{CustomResult, ParsingError},
    DbConnectionParams,
};
use diesel_models::enums::{
    AttemptStatus, AuthenticationType, Currency, FraudCheckStatus, IntentStatus, PaymentMethod,
    RefundStatus, RoutingApproach,
};
use error_stack::ResultExt;
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
    pub async fn from_conf(conf: &Database, schema: &str) -> Self {
        let database_url = conf.get_database_url(schema);
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
db_type!(IntentStatus);
db_type!(PaymentMethod, TEXT);
db_type!(RefundStatus);
db_type!(RefundType);
db_type!(FraudCheckStatus);
db_type!(FrmTransactionType);
db_type!(DisputeStage);
db_type!(DisputeStatus);
db_type!(AuthenticationStatus);
db_type!(TransactionStatus);
db_type!(AuthenticationConnectors);
db_type!(DecoupledAuthenticationType);
db_type!(RoutingApproach);

impl<'q, Type> Encode<'q, Postgres> for DBEnumWrapper<Type>
where
    Type: DbType + FromStr + Display,
{
    fn encode_by_ref(
        &self,
        buf: &mut PgArgumentBuffer,
    ) -> Result<sqlx::encode::IsNull, Box<dyn std::error::Error + Send + Sync + 'static>> {
        <String as Encode<'q, Postgres>>::encode(self.0.to_string(), buf)
    }
    fn size_hint(&self) -> usize {
        <String as Encode<'q, Postgres>>::size_hint(&self.0.to_string())
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
        T::from_row(&row).change_context(QueryExecutionError::RowExtractionFailure)
    }
}

impl super::payments::filters::PaymentFilterAnalytics for SqlxClient {}
impl super::payments::metrics::PaymentMetricAnalytics for SqlxClient {}
impl super::payments::distribution::PaymentDistributionAnalytics for SqlxClient {}
impl super::payment_intents::filters::PaymentIntentFilterAnalytics for SqlxClient {}
impl super::payment_intents::metrics::PaymentIntentMetricAnalytics for SqlxClient {}
impl super::refunds::metrics::RefundMetricAnalytics for SqlxClient {}
impl super::refunds::filters::RefundFilterAnalytics for SqlxClient {}
impl super::refunds::distribution::RefundDistributionAnalytics for SqlxClient {}
impl super::disputes::filters::DisputeFilterAnalytics for SqlxClient {}
impl super::disputes::metrics::DisputeMetricAnalytics for SqlxClient {}
impl super::frm::metrics::FrmMetricAnalytics for SqlxClient {}
impl super::frm::filters::FrmFilterAnalytics for SqlxClient {}
impl super::auth_events::metrics::AuthEventMetricAnalytics for SqlxClient {}
impl super::auth_events::filters::AuthEventFilterAnalytics for SqlxClient {}

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
            .change_context(QueryExecutionError::DatabaseError)
    }
}

impl<'a> FromRow<'a, PgRow> for super::auth_events::metrics::AuthEventMetricRow {
    fn from_row(row: &'a PgRow) -> sqlx::Result<Self> {
        let authentication_status: Option<DBEnumWrapper<AuthenticationStatus>> =
            row.try_get("authentication_status").or_else(|e| match e {
                ColumnNotFound(_) => Ok(Default::default()),
                e => Err(e),
            })?;
        let trans_status: Option<DBEnumWrapper<TransactionStatus>> =
            row.try_get("trans_status").or_else(|e| match e {
                ColumnNotFound(_) => Ok(Default::default()),
                e => Err(e),
            })?;
        let authentication_type: Option<DBEnumWrapper<DecoupledAuthenticationType>> =
            row.try_get("authentication_type").or_else(|e| match e {
                ColumnNotFound(_) => Ok(Default::default()),
                e => Err(e),
            })?;
        let error_message: Option<String> = row.try_get("error_message").or_else(|e| match e {
            ColumnNotFound(_) => Ok(Default::default()),
            e => Err(e),
        })?;
        let authentication_connector: Option<DBEnumWrapper<AuthenticationConnectors>> = row
            .try_get("authentication_connector")
            .or_else(|e| match e {
                ColumnNotFound(_) => Ok(Default::default()),
                e => Err(e),
            })?;
        let message_version: Option<String> =
            row.try_get("message_version").or_else(|e| match e {
                ColumnNotFound(_) => Ok(Default::default()),
                e => Err(e),
            })?;

        let platform: Option<String> = row.try_get("platform").or_else(|e| match e {
            ColumnNotFound(_) => Ok(Default::default()),
            e => Err(e),
        })?;
        let acs_reference_number: Option<String> =
            row.try_get("acs_reference_number").or_else(|e| match e {
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
        let mcc: Option<String> = row.try_get("mcc").or_else(|e| match e {
            ColumnNotFound(_) => Ok(Default::default()),
            e => Err(e),
        })?;
        let currency: Option<DBEnumWrapper<Currency>> =
            row.try_get("currency").or_else(|e| match e {
                ColumnNotFound(_) => Ok(Default::default()),
                e => Err(e),
            })?;
        let merchant_country: Option<String> =
            row.try_get("merchant_country").or_else(|e| match e {
                ColumnNotFound(_) => Ok(Default::default()),
                e => Err(e),
            })?;
        let billing_country: Option<String> =
            row.try_get("billing_country").or_else(|e| match e {
                ColumnNotFound(_) => Ok(Default::default()),
                e => Err(e),
            })?;
        let shipping_country: Option<String> =
            row.try_get("shipping_country").or_else(|e| match e {
                ColumnNotFound(_) => Ok(Default::default()),
                e => Err(e),
            })?;
        let issuer_country: Option<String> =
            row.try_get("issuer_country").or_else(|e| match e {
                ColumnNotFound(_) => Ok(Default::default()),
                e => Err(e),
            })?;
        let earliest_supported_version: Option<String> = row
            .try_get("earliest_supported_version")
            .or_else(|e| match e {
                ColumnNotFound(_) => Ok(Default::default()),
                e => Err(e),
            })?;
        let latest_supported_version: Option<String> = row
            .try_get("latest_supported_version")
            .or_else(|e| match e {
                ColumnNotFound(_) => Ok(Default::default()),
                e => Err(e),
            })?;
        let whitelist_decision: Option<bool> =
            row.try_get("whitelist_decision").or_else(|e| match e {
                ColumnNotFound(_) => Ok(Default::default()),
                e => Err(e),
            })?;
        let device_manufacturer: Option<String> =
            row.try_get("device_manufacturer").or_else(|e| match e {
                ColumnNotFound(_) => Ok(Default::default()),
                e => Err(e),
            })?;
        let device_type: Option<String> = row.try_get("device_type").or_else(|e| match e {
            ColumnNotFound(_) => Ok(Default::default()),
            e => Err(e),
        })?;
        let device_brand: Option<String> = row.try_get("device_brand").or_else(|e| match e {
            ColumnNotFound(_) => Ok(Default::default()),
            e => Err(e),
        })?;
        let device_os: Option<String> = row.try_get("device_os").or_else(|e| match e {
            ColumnNotFound(_) => Ok(Default::default()),
            e => Err(e),
        })?;
        let device_display: Option<String> =
            row.try_get("device_display").or_else(|e| match e {
                ColumnNotFound(_) => Ok(Default::default()),
                e => Err(e),
            })?;
        let browser_name: Option<String> = row.try_get("browser_name").or_else(|e| match e {
            ColumnNotFound(_) => Ok(Default::default()),
            e => Err(e),
        })?;
        let browser_version: Option<String> =
            row.try_get("browser_version").or_else(|e| match e {
                ColumnNotFound(_) => Ok(Default::default()),
                e => Err(e),
            })?;
        let issuer_id: Option<String> = row.try_get("issuer_id").or_else(|e| match e {
            ColumnNotFound(_) => Ok(Default::default()),
            e => Err(e),
        })?;
        let scheme_name: Option<String> = row.try_get("scheme_name").or_else(|e| match e {
            ColumnNotFound(_) => Ok(Default::default()),
            e => Err(e),
        })?;
        let exemption_requested: Option<bool> =
            row.try_get("exemption_requested").or_else(|e| match e {
                ColumnNotFound(_) => Ok(Default::default()),
                e => Err(e),
            })?;
        let exemption_accepted: Option<bool> =
            row.try_get("exemption_accepted").or_else(|e| match e {
                ColumnNotFound(_) => Ok(Default::default()),
                e => Err(e),
            })?;

        Ok(Self {
            authentication_status,
            trans_status,
            authentication_type,
            error_message,
            authentication_connector,
            message_version,
            acs_reference_number,
            platform,
            count,
            start_bucket,
            end_bucket,
            mcc,
            currency,
            merchant_country,
            billing_country,
            shipping_country,
            issuer_country,
            earliest_supported_version,
            latest_supported_version,
            whitelist_decision,
            device_manufacturer,
            device_type,
            device_brand,
            device_os,
            device_display,
            browser_name,
            browser_version,
            issuer_id,
            scheme_name,
            exemption_requested,
            exemption_accepted,
        })
    }
}

impl<'a> FromRow<'a, PgRow> for super::auth_events::filters::AuthEventFilterRow {
    fn from_row(row: &'a PgRow) -> sqlx::Result<Self> {
        let authentication_status: Option<DBEnumWrapper<AuthenticationStatus>> =
            row.try_get("authentication_status").or_else(|e| match e {
                ColumnNotFound(_) => Ok(Default::default()),
                e => Err(e),
            })?;
        let trans_status: Option<DBEnumWrapper<TransactionStatus>> =
            row.try_get("trans_status").or_else(|e| match e {
                ColumnNotFound(_) => Ok(Default::default()),
                e => Err(e),
            })?;
        let authentication_type: Option<DBEnumWrapper<DecoupledAuthenticationType>> =
            row.try_get("authentication_type").or_else(|e| match e {
                ColumnNotFound(_) => Ok(Default::default()),
                e => Err(e),
            })?;
        let error_message: Option<String> = row.try_get("error_message").or_else(|e| match e {
            ColumnNotFound(_) => Ok(Default::default()),
            e => Err(e),
        })?;
        let authentication_connector: Option<DBEnumWrapper<AuthenticationConnectors>> = row
            .try_get("authentication_connector")
            .or_else(|e| match e {
                ColumnNotFound(_) => Ok(Default::default()),
                e => Err(e),
            })?;
        let message_version: Option<String> =
            row.try_get("message_version").or_else(|e| match e {
                ColumnNotFound(_) => Ok(Default::default()),
                e => Err(e),
            })?;
        let acs_reference_number: Option<String> =
            row.try_get("acs_reference_number").or_else(|e| match e {
                ColumnNotFound(_) => Ok(Default::default()),
                e => Err(e),
            })?;
        let platform: Option<String> = row.try_get("platform").or_else(|e| match e {
            ColumnNotFound(_) => Ok(Default::default()),
            e => Err(e),
        })?;
        let mcc: Option<String> = row.try_get("mcc").or_else(|e| match e {
            ColumnNotFound(_) => Ok(Default::default()),
            e => Err(e),
        })?;
        let currency: Option<DBEnumWrapper<Currency>> =
            row.try_get("currency").or_else(|e| match e {
                ColumnNotFound(_) => Ok(Default::default()),
                e => Err(e),
            })?;
        let merchant_country: Option<String> =
            row.try_get("merchant_country").or_else(|e| match e {
                ColumnNotFound(_) => Ok(Default::default()),
                e => Err(e),
            })?;
        let billing_country: Option<String> =
            row.try_get("billing_country").or_else(|e| match e {
                ColumnNotFound(_) => Ok(Default::default()),
                e => Err(e),
            })?;
        let shipping_country: Option<String> =
            row.try_get("shipping_country").or_else(|e| match e {
                ColumnNotFound(_) => Ok(Default::default()),
                e => Err(e),
            })?;
        let issuer_country: Option<String> =
            row.try_get("issuer_country").or_else(|e| match e {
                ColumnNotFound(_) => Ok(Default::default()),
                e => Err(e),
            })?;
        let earliest_supported_version: Option<String> = row
            .try_get("earliest_supported_version")
            .or_else(|e| match e {
                ColumnNotFound(_) => Ok(Default::default()),
                e => Err(e),
            })?;
        let latest_supported_version: Option<String> = row
            .try_get("latest_supported_version")
            .or_else(|e| match e {
                ColumnNotFound(_) => Ok(Default::default()),
                e => Err(e),
            })?;
        let whitelist_decision: Option<bool> =
            row.try_get("whitelist_decision").or_else(|e| match e {
                ColumnNotFound(_) => Ok(Default::default()),
                e => Err(e),
            })?;
        let device_manufacturer: Option<String> =
            row.try_get("device_manufacturer").or_else(|e| match e {
                ColumnNotFound(_) => Ok(Default::default()),
                e => Err(e),
            })?;
        let device_type: Option<String> = row.try_get("device_type").or_else(|e| match e {
            ColumnNotFound(_) => Ok(Default::default()),
            e => Err(e),
        })?;
        let device_brand: Option<String> = row.try_get("device_brand").or_else(|e| match e {
            ColumnNotFound(_) => Ok(Default::default()),
            e => Err(e),
        })?;
        let device_os: Option<String> = row.try_get("device_os").or_else(|e| match e {
            ColumnNotFound(_) => Ok(Default::default()),
            e => Err(e),
        })?;
        let device_display: Option<String> =
            row.try_get("device_display").or_else(|e| match e {
                ColumnNotFound(_) => Ok(Default::default()),
                e => Err(e),
            })?;
        let browser_name: Option<String> = row.try_get("browser_name").or_else(|e| match e {
            ColumnNotFound(_) => Ok(Default::default()),
            e => Err(e),
        })?;
        let browser_version: Option<String> =
            row.try_get("browser_version").or_else(|e| match e {
                ColumnNotFound(_) => Ok(Default::default()),
                e => Err(e),
            })?;
        let issuer_id: Option<String> = row.try_get("issuer_id").or_else(|e| match e {
            ColumnNotFound(_) => Ok(Default::default()),
            e => Err(e),
        })?;
        let scheme_name: Option<String> = row.try_get("scheme_name").or_else(|e| match e {
            ColumnNotFound(_) => Ok(Default::default()),
            e => Err(e),
        })?;
        let exemption_requested: Option<bool> =
            row.try_get("exemption_requested").or_else(|e| match e {
                ColumnNotFound(_) => Ok(Default::default()),
                e => Err(e),
            })?;
        let exemption_accepted: Option<bool> =
            row.try_get("exemption_accepted").or_else(|e| match e {
                ColumnNotFound(_) => Ok(Default::default()),
                e => Err(e),
            })?;

        Ok(Self {
            authentication_status,
            trans_status,
            authentication_type,
            error_message,
            authentication_connector,
            message_version,
            platform,
            acs_reference_number,
            mcc,
            currency,
            merchant_country,
            billing_country,
            shipping_country,
            issuer_country,
            earliest_supported_version,
            latest_supported_version,
            whitelist_decision,
            device_manufacturer,
            device_type,
            device_brand,
            device_os,
            device_display,
            browser_name,
            browser_version,
            issuer_id,
            scheme_name,
            exemption_requested,
            exemption_accepted,
        })
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
        let profile_id: Option<String> = row.try_get("profile_id").or_else(|e| match e {
            ColumnNotFound(_) => Ok(Default::default()),
            e => Err(e),
        })?;
        let refund_reason: Option<String> = row.try_get("refund_reason").or_else(|e| match e {
            ColumnNotFound(_) => Ok(Default::default()),
            e => Err(e),
        })?;
        let refund_error_message: Option<String> =
            row.try_get("refund_error_message").or_else(|e| match e {
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
            profile_id,
            refund_reason,
            refund_error_message,
            total,
            count,
            start_bucket,
            end_bucket,
        })
    }
}

impl<'a> FromRow<'a, PgRow> for super::frm::metrics::FrmMetricRow {
    fn from_row(row: &'a PgRow) -> sqlx::Result<Self> {
        let frm_name: Option<String> = row.try_get("frm_name").or_else(|e| match e {
            ColumnNotFound(_) => Ok(Default::default()),
            e => Err(e),
        })?;
        let frm_status: Option<DBEnumWrapper<FraudCheckStatus>> =
            row.try_get("frm_status").or_else(|e| match e {
                ColumnNotFound(_) => Ok(Default::default()),
                e => Err(e),
            })?;
        let frm_transaction_type: Option<DBEnumWrapper<FrmTransactionType>> =
            row.try_get("frm_transaction_type").or_else(|e| match e {
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
            frm_name,
            frm_status,
            frm_transaction_type,
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
        let client_source: Option<String> = row.try_get("client_source").or_else(|e| match e {
            ColumnNotFound(_) => Ok(Default::default()),
            e => Err(e),
        })?;
        let client_version: Option<String> =
            row.try_get("client_version").or_else(|e| match e {
                ColumnNotFound(_) => Ok(Default::default()),
                e => Err(e),
            })?;
        let profile_id: Option<String> = row.try_get("profile_id").or_else(|e| match e {
            ColumnNotFound(_) => Ok(Default::default()),
            e => Err(e),
        })?;
        let card_network: Option<String> = row.try_get("card_network").or_else(|e| match e {
            ColumnNotFound(_) => Ok(Default::default()),
            e => Err(e),
        })?;
        let merchant_id: Option<String> = row.try_get("merchant_id").or_else(|e| match e {
            ColumnNotFound(_) => Ok(Default::default()),
            e => Err(e),
        })?;
        let card_last_4: Option<String> = row.try_get("card_last_4").or_else(|e| match e {
            ColumnNotFound(_) => Ok(Default::default()),
            e => Err(e),
        })?;
        let card_issuer: Option<String> = row.try_get("card_issuer").or_else(|e| match e {
            ColumnNotFound(_) => Ok(Default::default()),
            e => Err(e),
        })?;
        let error_reason: Option<String> = row.try_get("error_reason").or_else(|e| match e {
            ColumnNotFound(_) => Ok(Default::default()),
            e => Err(e),
        })?;
        let first_attempt: Option<bool> = row.try_get("first_attempt").or_else(|e| match e {
            ColumnNotFound(_) => Ok(Default::default()),
            e => Err(e),
        })?;
        let routing_approach: Option<DBEnumWrapper<RoutingApproach>> =
            row.try_get("routing_approach").or_else(|e| match e {
                ColumnNotFound(_) => Ok(Default::default()),
                e => Err(e),
            })?;
        let signature_network: Option<String> =
            row.try_get("signature_network").or_else(|e| match e {
                ColumnNotFound(_) => Ok(Default::default()),
                e => Err(e),
            })?;
        let is_issuer_regulated: Option<bool> =
            row.try_get("is_issuer_regulated").or_else(|e| match e {
                ColumnNotFound(_) => Ok(Default::default()),
                e => Err(e),
            })?;
        let is_debit_routed: Option<bool> =
            row.try_get("is_debit_routed").or_else(|e| match e {
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
            client_source,
            client_version,
            profile_id,
            card_network,
            merchant_id,
            card_last_4,
            card_issuer,
            error_reason,
            first_attempt,
            routing_approach,
            signature_network,
            is_issuer_regulated,
            is_debit_routed,
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
        let client_source: Option<String> = row.try_get("client_source").or_else(|e| match e {
            ColumnNotFound(_) => Ok(Default::default()),
            e => Err(e),
        })?;
        let client_version: Option<String> =
            row.try_get("client_version").or_else(|e| match e {
                ColumnNotFound(_) => Ok(Default::default()),
                e => Err(e),
            })?;
        let profile_id: Option<String> = row.try_get("profile_id").or_else(|e| match e {
            ColumnNotFound(_) => Ok(Default::default()),
            e => Err(e),
        })?;
        let card_network: Option<String> = row.try_get("card_network").or_else(|e| match e {
            ColumnNotFound(_) => Ok(Default::default()),
            e => Err(e),
        })?;
        let merchant_id: Option<String> = row.try_get("merchant_id").or_else(|e| match e {
            ColumnNotFound(_) => Ok(Default::default()),
            e => Err(e),
        })?;
        let card_last_4: Option<String> = row.try_get("card_last_4").or_else(|e| match e {
            ColumnNotFound(_) => Ok(Default::default()),
            e => Err(e),
        })?;
        let card_issuer: Option<String> = row.try_get("card_issuer").or_else(|e| match e {
            ColumnNotFound(_) => Ok(Default::default()),
            e => Err(e),
        })?;
        let error_reason: Option<String> = row.try_get("error_reason").or_else(|e| match e {
            ColumnNotFound(_) => Ok(Default::default()),
            e => Err(e),
        })?;
        let routing_approach: Option<DBEnumWrapper<RoutingApproach>> =
            row.try_get("routing_approach").or_else(|e| match e {
                ColumnNotFound(_) => Ok(Default::default()),
                e => Err(e),
            })?;

        let signature_network: Option<String> =
            row.try_get("signature_network").or_else(|e| match e {
                ColumnNotFound(_) => Ok(Default::default()),
                e => Err(e),
            })?;
        let is_issuer_regulated: Option<bool> =
            row.try_get("is_issuer_regulated").or_else(|e| match e {
                ColumnNotFound(_) => Ok(Default::default()),
                e => Err(e),
            })?;
        let is_debit_routed: Option<bool> =
            row.try_get("is_debit_routed").or_else(|e| match e {
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
        let first_attempt: Option<bool> = row.try_get("first_attempt").or_else(|e| match e {
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
            client_source,
            client_version,
            profile_id,
            card_network,
            merchant_id,
            card_last_4,
            card_issuer,
            error_reason,
            first_attempt,
            total,
            count,
            error_message,
            routing_approach,
            signature_network,
            is_issuer_regulated,
            is_debit_routed,
            start_bucket,
            end_bucket,
        })
    }
}

impl<'a> FromRow<'a, PgRow> for super::payments::filters::PaymentFilterRow {
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
        let client_source: Option<String> = row.try_get("client_source").or_else(|e| match e {
            ColumnNotFound(_) => Ok(Default::default()),
            e => Err(e),
        })?;
        let client_version: Option<String> =
            row.try_get("client_version").or_else(|e| match e {
                ColumnNotFound(_) => Ok(Default::default()),
                e => Err(e),
            })?;
        let profile_id: Option<String> = row.try_get("profile_id").or_else(|e| match e {
            ColumnNotFound(_) => Ok(Default::default()),
            e => Err(e),
        })?;
        let card_network: Option<String> = row.try_get("card_network").or_else(|e| match e {
            ColumnNotFound(_) => Ok(Default::default()),
            e => Err(e),
        })?;
        let merchant_id: Option<String> = row.try_get("merchant_id").or_else(|e| match e {
            ColumnNotFound(_) => Ok(Default::default()),
            e => Err(e),
        })?;
        let card_last_4: Option<String> = row.try_get("card_last_4").or_else(|e| match e {
            ColumnNotFound(_) => Ok(Default::default()),
            e => Err(e),
        })?;
        let card_issuer: Option<String> = row.try_get("card_issuer").or_else(|e| match e {
            ColumnNotFound(_) => Ok(Default::default()),
            e => Err(e),
        })?;
        let error_reason: Option<String> = row.try_get("error_reason").or_else(|e| match e {
            ColumnNotFound(_) => Ok(Default::default()),
            e => Err(e),
        })?;
        let first_attempt: Option<bool> = row.try_get("first_attempt").or_else(|e| match e {
            ColumnNotFound(_) => Ok(Default::default()),
            e => Err(e),
        })?;
        let routing_approach: Option<DBEnumWrapper<RoutingApproach>> =
            row.try_get("routing_approach").or_else(|e| match e {
                ColumnNotFound(_) => Ok(Default::default()),
                e => Err(e),
            })?;
        let signature_network: Option<String> =
            row.try_get("signature_network").or_else(|e| match e {
                ColumnNotFound(_) => Ok(Default::default()),
                e => Err(e),
            })?;
        let is_issuer_regulated: Option<bool> =
            row.try_get("is_issuer_regulated").or_else(|e| match e {
                ColumnNotFound(_) => Ok(Default::default()),
                e => Err(e),
            })?;
        let is_debit_routed: Option<bool> =
            row.try_get("is_debit_routed").or_else(|e| match e {
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
            client_source,
            client_version,
            profile_id,
            card_network,
            merchant_id,
            card_last_4,
            card_issuer,
            error_reason,
            first_attempt,
            routing_approach,
            signature_network,
            is_issuer_regulated,
            is_debit_routed,
        })
    }
}

impl<'a> FromRow<'a, PgRow> for super::payment_intents::metrics::PaymentIntentMetricRow {
    fn from_row(row: &'a PgRow) -> sqlx::Result<Self> {
        let status: Option<DBEnumWrapper<IntentStatus>> =
            row.try_get("status").or_else(|e| match e {
                ColumnNotFound(_) => Ok(Default::default()),
                e => Err(e),
            })?;
        let currency: Option<DBEnumWrapper<Currency>> =
            row.try_get("currency").or_else(|e| match e {
                ColumnNotFound(_) => Ok(Default::default()),
                e => Err(e),
            })?;
        let profile_id: Option<String> = row.try_get("profile_id").or_else(|e| match e {
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
        let card_network: Option<String> = row.try_get("card_network").or_else(|e| match e {
            ColumnNotFound(_) => Ok(Default::default()),
            e => Err(e),
        })?;
        let merchant_id: Option<String> = row.try_get("merchant_id").or_else(|e| match e {
            ColumnNotFound(_) => Ok(Default::default()),
            e => Err(e),
        })?;
        let card_last_4: Option<String> = row.try_get("card_last_4").or_else(|e| match e {
            ColumnNotFound(_) => Ok(Default::default()),
            e => Err(e),
        })?;
        let card_issuer: Option<String> = row.try_get("card_issuer").or_else(|e| match e {
            ColumnNotFound(_) => Ok(Default::default()),
            e => Err(e),
        })?;
        let error_reason: Option<String> = row.try_get("error_reason").or_else(|e| match e {
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
        let first_attempt: Option<i64> = row.try_get("first_attempt").or_else(|e| match e {
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
            status,
            currency,
            profile_id,
            connector,
            authentication_type,
            payment_method,
            payment_method_type,
            card_network,
            merchant_id,
            card_last_4,
            card_issuer,
            error_reason,
            first_attempt,
            total,
            count,
            start_bucket,
            end_bucket,
        })
    }
}

impl<'a> FromRow<'a, PgRow> for super::payment_intents::filters::PaymentIntentFilterRow {
    fn from_row(row: &'a PgRow) -> sqlx::Result<Self> {
        let status: Option<DBEnumWrapper<IntentStatus>> =
            row.try_get("status").or_else(|e| match e {
                ColumnNotFound(_) => Ok(Default::default()),
                e => Err(e),
            })?;
        let currency: Option<DBEnumWrapper<Currency>> =
            row.try_get("currency").or_else(|e| match e {
                ColumnNotFound(_) => Ok(Default::default()),
                e => Err(e),
            })?;
        let profile_id: Option<String> = row.try_get("profile_id").or_else(|e| match e {
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
        let card_network: Option<String> = row.try_get("card_network").or_else(|e| match e {
            ColumnNotFound(_) => Ok(Default::default()),
            e => Err(e),
        })?;
        let merchant_id: Option<String> = row.try_get("merchant_id").or_else(|e| match e {
            ColumnNotFound(_) => Ok(Default::default()),
            e => Err(e),
        })?;
        let card_last_4: Option<String> = row.try_get("card_last_4").or_else(|e| match e {
            ColumnNotFound(_) => Ok(Default::default()),
            e => Err(e),
        })?;
        let card_issuer: Option<String> = row.try_get("card_issuer").or_else(|e| match e {
            ColumnNotFound(_) => Ok(Default::default()),
            e => Err(e),
        })?;
        let error_reason: Option<String> = row.try_get("error_reason").or_else(|e| match e {
            ColumnNotFound(_) => Ok(Default::default()),
            e => Err(e),
        })?;
        let customer_id: Option<String> = row.try_get("customer_id").or_else(|e| match e {
            ColumnNotFound(_) => Ok(Default::default()),
            e => Err(e),
        })?;
        Ok(Self {
            status,
            currency,
            profile_id,
            connector,
            authentication_type,
            payment_method,
            payment_method_type,
            card_network,
            merchant_id,
            card_last_4,
            card_issuer,
            error_reason,
            customer_id,
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
        let profile_id: Option<String> = row.try_get("profile_id").or_else(|e| match e {
            ColumnNotFound(_) => Ok(Default::default()),
            e => Err(e),
        })?;
        let refund_reason: Option<String> = row.try_get("refund_reason").or_else(|e| match e {
            ColumnNotFound(_) => Ok(Default::default()),
            e => Err(e),
        })?;
        let refund_error_message: Option<String> =
            row.try_get("refund_error_message").or_else(|e| match e {
                ColumnNotFound(_) => Ok(Default::default()),
                e => Err(e),
            })?;
        Ok(Self {
            currency,
            refund_status,
            connector,
            refund_type,
            profile_id,
            refund_reason,
            refund_error_message,
        })
    }
}

impl<'a> FromRow<'a, PgRow> for super::refunds::distribution::RefundDistributionRow {
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
        let profile_id: Option<String> = row.try_get("profile_id").or_else(|e| match e {
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
        let refund_reason: Option<String> = row.try_get("refund_reason").or_else(|e| match e {
            ColumnNotFound(_) => Ok(Default::default()),
            e => Err(e),
        })?;
        let refund_error_message: Option<String> =
            row.try_get("refund_error_message").or_else(|e| match e {
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
            profile_id,
            total,
            count,
            refund_reason,
            refund_error_message,
            start_bucket,
            end_bucket,
        })
    }
}

impl<'a> FromRow<'a, PgRow> for super::frm::filters::FrmFilterRow {
    fn from_row(row: &'a PgRow) -> sqlx::Result<Self> {
        let frm_name: Option<String> = row.try_get("frm_name").or_else(|e| match e {
            ColumnNotFound(_) => Ok(Default::default()),
            e => Err(e),
        })?;
        let frm_status: Option<DBEnumWrapper<FraudCheckStatus>> =
            row.try_get("frm_status").or_else(|e| match e {
                ColumnNotFound(_) => Ok(Default::default()),
                e => Err(e),
            })?;
        let frm_transaction_type: Option<DBEnumWrapper<FrmTransactionType>> =
            row.try_get("frm_transaction_type").or_else(|e| match e {
                ColumnNotFound(_) => Ok(Default::default()),
                e => Err(e),
            })?;
        Ok(Self {
            frm_name,
            frm_status,
            frm_transaction_type,
        })
    }
}

impl<'a> FromRow<'a, PgRow> for super::disputes::filters::DisputeFilterRow {
    fn from_row(row: &'a PgRow) -> sqlx::Result<Self> {
        let dispute_stage: Option<String> = row.try_get("dispute_stage").or_else(|e| match e {
            ColumnNotFound(_) => Ok(Default::default()),
            e => Err(e),
        })?;
        let dispute_status: Option<String> =
            row.try_get("dispute_status").or_else(|e| match e {
                ColumnNotFound(_) => Ok(Default::default()),
                e => Err(e),
            })?;
        let connector: Option<String> = row.try_get("connector").or_else(|e| match e {
            ColumnNotFound(_) => Ok(Default::default()),
            e => Err(e),
        })?;
        let connector_status: Option<String> =
            row.try_get("connector_status").or_else(|e| match e {
                ColumnNotFound(_) => Ok(Default::default()),
                e => Err(e),
            })?;
        let currency: Option<DBEnumWrapper<Currency>> =
            row.try_get("currency").or_else(|e| match e {
                ColumnNotFound(_) => Ok(Default::default()),
                e => Err(e),
            })?;
        Ok(Self {
            dispute_stage,
            dispute_status,
            connector,
            connector_status,
            currency,
        })
    }
}
impl<'a> FromRow<'a, PgRow> for super::disputes::metrics::DisputeMetricRow {
    fn from_row(row: &'a PgRow) -> sqlx::Result<Self> {
        let dispute_stage: Option<DBEnumWrapper<DisputeStage>> =
            row.try_get("dispute_stage").or_else(|e| match e {
                ColumnNotFound(_) => Ok(Default::default()),
                e => Err(e),
            })?;
        let dispute_status: Option<DBEnumWrapper<DisputeStatus>> =
            row.try_get("dispute_status").or_else(|e| match e {
                ColumnNotFound(_) => Ok(Default::default()),
                e => Err(e),
            })?;
        let connector: Option<String> = row.try_get("connector").or_else(|e| match e {
            ColumnNotFound(_) => Ok(Default::default()),
            e => Err(e),
        })?;
        let currency: Option<DBEnumWrapper<Currency>> =
            row.try_get("currency").or_else(|e| match e {
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
            dispute_stage,
            dispute_status,
            connector,
            currency,
            total,
            count,
            start_bucket,
            end_bucket,
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
            Self::PaymentSessionized => Err(error_stack::report!(ParsingError::UnknownError)
                .attach_printable("PaymentSessionized table is not implemented for Sqlx"))?,
            Self::Refund => Ok("refund".to_string()),
            Self::RefundSessionized => Err(error_stack::report!(ParsingError::UnknownError)
                .attach_printable("RefundSessionized table is not implemented for Sqlx"))?,
            Self::SdkEvents => Err(error_stack::report!(ParsingError::UnknownError)
                .attach_printable("SdkEventsAudit table is not implemented for Sqlx"))?,
            Self::SdkEventsAnalytics => Err(error_stack::report!(ParsingError::UnknownError)
                .attach_printable("SdkEvents table is not implemented for Sqlx"))?,
            Self::ApiEvents => Err(error_stack::report!(ParsingError::UnknownError)
                .attach_printable("ApiEvents table is not implemented for Sqlx"))?,
            Self::FraudCheck => Ok("fraud_check".to_string()),
            Self::PaymentIntent => Ok("payment_intent".to_string()),
            Self::PaymentIntentSessionized => Err(error_stack::report!(
                ParsingError::UnknownError
            )
            .attach_printable("PaymentIntentSessionized table is not implemented for Sqlx"))?,
            Self::ConnectorEvents => Err(error_stack::report!(ParsingError::UnknownError)
                .attach_printable("ConnectorEvents table is not implemented for Sqlx"))?,
            Self::ApiEventsAnalytics => Err(error_stack::report!(ParsingError::UnknownError)
                .attach_printable("ApiEvents table is not implemented for Sqlx"))?,
            Self::ActivePaymentsAnalytics => Err(error_stack::report!(ParsingError::UnknownError)
                .attach_printable("ActivePaymentsAnalytics table is not implemented for Sqlx"))?,
            Self::OutgoingWebhookEvent => Err(error_stack::report!(ParsingError::UnknownError)
                .attach_printable("OutgoingWebhookEvents table is not implemented for Sqlx"))?,
            Self::Dispute => Ok("dispute".to_string()),
            Self::DisputeSessionized => Err(error_stack::report!(ParsingError::UnknownError)
                .attach_printable("DisputeSessionized table is not implemented for Sqlx"))?,
            Self::Authentications => Err(error_stack::report!(ParsingError::UnknownError)
                .attach_printable("Authentications table is not implemented for Sqlx"))?,
            Self::RoutingEvents => Err(error_stack::report!(ParsingError::UnknownError)
                .attach_printable("RoutingEvents table is not implemented for Sqlx"))?,
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
                    alias.map_or_else(|| "".to_owned(), |alias| format!(" as {alias}"))
                )
            }
            Self::Sum { field, alias } => {
                format!(
                    "sum({}){}",
                    field
                        .to_sql(table_engine)
                        .attach_printable("Failed to sum aggregate")?,
                    alias.map_or_else(|| "".to_owned(), |alias| format!(" as {alias}"))
                )
            }
            Self::Min { field, alias } => {
                format!(
                    "min({}){}",
                    field
                        .to_sql(table_engine)
                        .attach_printable("Failed to min aggregate")?,
                    alias.map_or_else(|| "".to_owned(), |alias| format!(" as {alias}"))
                )
            }
            Self::Max { field, alias } => {
                format!(
                    "max({}){}",
                    field
                        .to_sql(table_engine)
                        .attach_printable("Failed to max aggregate")?,
                    alias.map_or_else(|| "".to_owned(), |alias| format!(" as {alias}"))
                )
            }
            Self::Percentile {
                field,
                alias,
                percentile,
            } => {
                format!(
                    "percentile_cont(0.{}) within group (order by {} asc){}",
                    percentile.map_or_else(|| "50".to_owned(), |percentile| percentile.to_string()),
                    field
                        .to_sql(table_engine)
                        .attach_printable("Failed to percentile aggregate")?,
                    alias.map_or_else(|| "".to_owned(), |alias| format!(" as {alias}"))
                )
            }
            Self::DistinctCount { field, alias } => {
                format!(
                    "count(distinct {}){}",
                    field
                        .to_sql(table_engine)
                        .attach_printable("Failed to distinct count aggregate")?,
                    alias.map_or_else(|| "".to_owned(), |alias| format!(" as {alias}"))
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
                            order
                        )
                    ),
                    alias.map_or_else(|| "".to_owned(), |alias| format!(" as {alias}"))
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
                            order
                        )
                    ),
                    alias.map_or_else(|| "".to_owned(), |alias| format!(" as {alias}"))
                )
            }
        })
    }
}
