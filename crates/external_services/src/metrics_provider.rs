pub mod grafana;

use common_utils::errors::CustomResult;
use time::PrimitiveDateTime;

pub type MetricsResult<T> = CustomResult<T, MetricsError>;

#[async_trait::async_trait]
pub trait MetricsProvider: Send + Sync + std::fmt::Debug {
    type Query;

    async fn query(
        &self,
        query: &Self::Query,
        range: MetricRange,
    ) -> MetricsResult<Vec<MetricSeries>>;
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MetricRange {
    pub from: PrimitiveDateTime,
    pub to: PrimitiveDateTime,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MetricSeries {
    pub points: Vec<MetricPoint>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MetricPoint {
    pub timestamp: PrimitiveDateTime,
    pub value: f64,
}

#[derive(Debug, thiserror::Error)]
pub enum MetricsError {
    #[error("failed to serialize the metric query")]
    RequestSerialization,
    #[error("failed to send the metric request")]
    Transport,
    #[error("the metric request timed out")]
    Timeout,
    #[error("metric provider responded with HTTP status {status}")]
    HttpStatus { status: u16 },
    #[error("metric provider refused the query: {reason}")]
    QueryRefused { reason: String },
    #[error("could not interpret the metric response")]
    MalformedResponse,
}
