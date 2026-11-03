//! ACI (OPPWA) request and response mapping.
//!
//! - [`request`]: the wire format of every request body
//! - `payment`: the payment request as domain decisions — channel, card-on-file leg, shopper
//!   presence — and one adapter per payment method
//! - `transaction`: requests on an existing payment (capture, reversal, refund)
//! - `response` / `flow` / `cof`: the payment response and what each flow records from it
//! - `acquirer`: one strategy per acquirer behind ACI
//! - `metadata`, `result`, `three_ds`, `standalone_three_ds`, `webhook`

use common_utils::types::StringMajorUnit;
use hyperswitch_domain_models::router_data::ConnectorAuthType;
use hyperswitch_interfaces::errors;
use hyperswitch_masking::Secret;
use serde::Serialize;

mod acquirer;
mod cof;
mod flow;
mod metadata;
mod payment;
pub mod request;
mod response;
mod result;
mod standalone_three_ds;
mod three_ds;
pub mod transaction;
mod webhook;

pub use payment::setup_mandate_request;
pub use request::AciPaymentsRequest;
pub use response::AciPaymentsResponse;
pub use result::AciErrorResponse;
pub use standalone_three_ds::AciStandaloneThreeDsRequest;
pub use transaction::{
    capture::{AciCaptureRequest, AciCaptureResponse},
    refund::{AciRefundRequest, AciRefundResponse},
    reversal::AciCancelRequest,
    Endpoint,
};
pub use webhook::{AciPaymentWebhookPayload, AciWebhookBody, AciWebhookNotification};

type Error = error_stack::Report<errors::ConnectorError>;

#[derive(Debug, Serialize)]
pub struct AciRouterData<T> {
    pub amount: StringMajorUnit,
    pub router_data: T,
}

impl<T> From<(StringMajorUnit, T)> for AciRouterData<T> {
    fn from((amount, item): (StringMajorUnit, T)) -> Self {
        Self {
            amount,
            router_data: item,
        }
    }
}

pub struct AciAuthType {
    pub api_key: Secret<String>,
    pub entity_id: Secret<String>,
}

impl TryFrom<&ConnectorAuthType> for AciAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &ConnectorAuthType) -> Result<Self, Self::Error> {
        if let ConnectorAuthType::BodyKey { api_key, key1 } = item {
            Ok(Self {
                api_key: api_key.to_owned(),
                entity_id: key1.to_owned(),
            })
        } else {
            Err(errors::ConnectorError::FailedToObtainAuthType)?
        }
    }
}
