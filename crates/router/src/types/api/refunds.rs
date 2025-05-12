#[cfg(all(any(feature = "v1", feature = "v2"), not(feature = "refunds_v2")))]
pub use api_models::refunds::RefundRequest;
#[cfg(all(feature = "v2", feature = "refunds_v2"))]
pub use api_models::refunds::RefundsCreateRequest;
pub use api_models::refunds::{
    RefundResponse, RefundStatus, RefundType, RefundUpdateRequest, RefundsRetrieveBody,
    RefundsRetrieveRequest,
};
pub use hyperswitch_domain_models::router_flow_types::refunds::{Execute, RSync};
pub use hyperswitch_interfaces::api::refunds::{Refund, RefundExecute, RefundSync};

use crate::types::{storage::enums as storage_enums, transformers::ForeignFrom};

impl ForeignFrom<storage_enums::RefundStatus> for RefundStatus {
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
