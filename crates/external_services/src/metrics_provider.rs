mod error;
pub mod grafana;
mod query;
mod response;

use common_utils::errors::CustomResult;

pub use self::{
    error::MetricsError,
    query::MetricRange,
    response::{MetricPoint, MetricSeries},
};

pub type MetricsResult<T> = CustomResult<T, MetricsError>;

#[async_trait::async_trait]
pub trait MetricsProvider: Send + Sync + std::fmt::Debug {
    type Request: Send + Sync;
    type Response: Send;
    type Error: error_stack::Context;

    fn validate_request(request: &Self::Request) -> CustomResult<(), Self::Error>;

    async fn fetch(
        &self,
        request: Self::Request,
        range: MetricRange,
    ) -> CustomResult<Self::Response, Self::Error>;
}
