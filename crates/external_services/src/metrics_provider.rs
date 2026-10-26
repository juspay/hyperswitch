mod error;
pub mod grafana;

pub use self::{
    error::MetricsError,
    grafana::cloudwatch::{MetricPoint, MetricRange, MetricSeries},
};
use common_utils::errors::CustomResult;
use serde::{de::DeserializeOwned, Serialize};

pub type MetricsResult<T> = CustomResult<T, MetricsError>;

#[async_trait::async_trait]
pub trait MetricsProvider: Send + Sync + std::fmt::Debug {
    type MetricsQuery: Send + Sync;
    type MetricsRequest: Serialize + Send + 'static;
    type MetricsResponse: DeserializeOwned + Send;
    type MetricsError: error_stack::Context;

    fn build_request(
        query: Self::MetricsQuery,
        range: MetricRange,
    ) -> CustomResult<Self::MetricsRequest, Self::MetricsError>;

    fn handle_response(
        &self,
        response: Self::MetricsResponse,
        status: reqwest::StatusCode,
    ) -> CustomResult<Vec<MetricSeries>, Self::MetricsError>;

    async fn fetch_metrics(
        &self,
        request: Self::MetricsRequest,
    ) -> CustomResult<Vec<MetricSeries>, Self::MetricsError>;
}
