pub use api_models::refunds::{RefundRequest, RefundResponse, RefundStatus};

use super::ConnectorCommon;
use crate::{
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

pub trait Refund: ConnectorCommon + RefundExecute + RefundSync {}
