pub use api_models::refunds::{
    RefundRequest, RefundResponse, RefundStatus, RefundType, RefundUpdateRequest,
    RefundsRetrieveRequest,
};

use super::ConnectorCommon;
use crate::{
    services::api,
    types::{self, storage::enums as storage_enums, transformers::ForeignFrom},
};

impl ForeignFrom<storage_enums::RefundStatus> for RefundStatus {
        /// Converts a refund status from the storage_enums module to a corresponding status in the current module.
    fn foreign_from(status: storage_enums::RefundStatus) -> Self {
        match status {
            storage_enums::RefundStatus::Failure
            | storage_enums::RefundStatus::TransactionFailure => Self::Failed,
            storage_enums::RefundStatus::ManualReview => Self::Review,
            storage_enums::RefundStatus::Pending => Self::Pending,
            storage_enums::RefundStatus::Success => Self::Succeeded,
        }
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
