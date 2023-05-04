pub use api_models::payouts::{PayoutCreateRequest, PayoutCreateResponse};

use super::ConnectorCommon;
use crate::{services::api, types};

#[derive(Debug, Clone)]
pub struct Payout;

pub trait PayoutCreate:
    api::ConnectorIntegration<Payout, types::PayoutsData, types::PayoutsResponseData>
{
}

pub trait Payouts: ConnectorCommon + PayoutCreate {}
