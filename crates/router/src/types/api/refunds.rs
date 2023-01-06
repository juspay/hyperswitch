pub use api_models::refunds::{RefundRequest, RefundResponse, RefundStatus};

use crate::{
    core::errors,
    routes,
    services::api,
    types::{self, storage::enums as storage_enums, transformers::Foreign},
};

impl From<Foreign<storage_enums::RefundStatus>> for Foreign<RefundStatus> {
    fn from(status: Foreign<storage_enums::RefundStatus>) -> Self {
        match status.0 {
            storage_enums::RefundStatus::Failure
            | storage_enums::RefundStatus::TransactionFailure => RefundStatus::Failed,
            storage_enums::RefundStatus::ManualReview => RefundStatus::Review,
            storage_enums::RefundStatus::Pending => RefundStatus::Pending,
            storage_enums::RefundStatus::Success => RefundStatus::Succeeded,
        }
        .into()
    }
}

#[derive(Debug, Clone)]
pub struct Execute;
#[derive(Debug, Clone)]
pub struct RSync;

pub trait RefundExecute:
    api::ConnectorIntegration<Execute, types::RefundsData, types::RefundsResponseData>
{
}

pub trait RefundSync:
    api::ConnectorIntegration<RSync, types::RefundsData, types::RefundsResponseData>
{
}

#[async_trait::async_trait]
pub trait RefundCommon {
    async fn refund_execute_update_tracker<'a>(
        &'a self,
        _state: &'a routes::AppState,
        _connector: &'a types::api::ConnectorData,
        router_data: types::RefundsRouterData<Execute>,
        _payment_attempt: &'a storage_models::payment_attempt::PaymentAttempt,
    ) -> errors::RouterResult<types::RefundsRouterData<Execute>> {
        Ok(router_data)
    }
}

pub trait Refund: types::api::ConnectorCommon + RefundExecute + RefundSync + RefundCommon {}
