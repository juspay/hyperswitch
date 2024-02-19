use std::{fmt::Display, str::FromStr};

use common_utils::{
    errors::{CustomResult, ErrorSwitch, ParsingError},
    events::{ApiEventMetric, ApiEventsType},
    impl_misc_api_event_type,
};
use error_stack::{report, Report, ResultExt};

use super::query::QueryBuildingError;
use crate::errors::AnalyticsError;

#[derive(serde::Deserialize, Debug, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AnalyticsDomain {
    Payments,
    Refunds,
    SdkEvents,
    ApiEvents,
    Dispute,
}

#[derive(Debug, strum::AsRefStr, strum::Display, Clone, Copy)]
pub enum AnalyticsCollection {
    Payment,
    Refund,
    SdkEvents,
    ApiEvents,
    PaymentIntent,
    ConnectorEvents,
    OutgoingWebhookEvent,
    Dispute,
}

#[allow(dead_code)]
#[derive(Debug)]
pub enum TableEngine {
    CollapsingMergeTree { sign: &'static str },
    BasicTree,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, Eq, PartialEq)]
#[serde(transparent)]
pub struct DBEnumWrapper<T: FromStr + Display>(pub T);

impl<T: FromStr + Display> AsRef<T> for DBEnumWrapper<T> {
    fn as_ref(&self) -> &T {
        &self.0
    }
}

impl<T> FromStr for DBEnumWrapper<T>
where
    T: FromStr + Display,
{
    type Err = Report<ParsingError>;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        T::from_str(s)
            .map_err(|_er| report!(ParsingError::EnumParseFailure(std::any::type_name::<T>())))
            .map(DBEnumWrapper)
            .attach_printable_lazy(|| format!("raw_value: {s}"))
    }
}

// Analytics Framework

pub trait RefundAnalytics {}
pub trait SdkEventAnalytics {}

#[async_trait::async_trait]
pub trait AnalyticsDataSource
where
    Self: Sized + Sync + Send,
{
    type Row;
    async fn load_results<T>(&self, query: &str) -> CustomResult<Vec<T>, QueryExecutionError>
    where
        Self: LoadRow<T>;

    fn get_table_engine(_table: AnalyticsCollection) -> TableEngine {
        TableEngine::BasicTree
    }
}

pub trait LoadRow<T>
where
    Self: AnalyticsDataSource,
    T: Sized,
{
    fn load_row(row: Self::Row) -> CustomResult<T, QueryExecutionError>;
}

#[derive(thiserror::Error, Debug)]
pub enum MetricsError {
    #[error("Error building query")]
    QueryBuildingError,
    #[error("Error running Query")]
    QueryExecutionFailure,
    #[error("Error processing query results")]
    PostProcessingFailure,
    #[allow(dead_code)]
    #[error("Not Implemented")]
    NotImplemented,
}

#[derive(Debug, thiserror::Error)]
pub enum QueryExecutionError {
    #[error("Failed to extract domain rows")]
    RowExtractionFailure,
    #[error("Database error")]
    DatabaseError,
}

pub type MetricsResult<T> = CustomResult<T, MetricsError>;

impl ErrorSwitch<MetricsError> for QueryBuildingError {
    fn switch(&self) -> MetricsError {
        MetricsError::QueryBuildingError
    }
}

pub type FiltersResult<T> = CustomResult<T, FiltersError>;

#[derive(thiserror::Error, Debug)]
pub enum FiltersError {
    #[error("Error building query")]
    QueryBuildingError,
    #[error("Error running Query")]
    QueryExecutionFailure,
    #[allow(dead_code)]
    #[error("Not Implemented: {0}")]
    NotImplemented(&'static str),
}

impl ErrorSwitch<FiltersError> for QueryBuildingError {
    fn switch(&self) -> FiltersError {
        FiltersError::QueryBuildingError
    }
}

impl ErrorSwitch<AnalyticsError> for FiltersError {
    fn switch(&self) -> AnalyticsError {
        match self {
            Self::QueryBuildingError | Self::QueryExecutionFailure => AnalyticsError::UnknownError,
            Self::NotImplemented(a) => AnalyticsError::NotImplemented(a),
        }
    }
}

impl_misc_api_event_type!(AnalyticsDomain);
