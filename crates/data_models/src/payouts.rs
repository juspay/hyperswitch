pub mod payout_attempt;
#[allow(clippy::module_inception)]
pub mod payouts;

use common_enums as storage_enums;
use common_utils::consts;
use time::PrimitiveDateTime;

pub enum PayoutFetchConstraints {
    Single { payout_id: String },
    List(Box<PayoutListParams>),
}

pub struct PayoutListParams {
    pub offset: u32,
    pub starting_at: Option<PrimitiveDateTime>,
    pub ending_at: Option<PrimitiveDateTime>,
    pub connector: Option<Vec<api_models::enums::PayoutConnectors>>,
    pub currency: Option<Vec<storage_enums::Currency>>,
    pub status: Option<Vec<storage_enums::PayoutStatus>>,
    pub payout_method: Option<Vec<common_enums::PayoutType>>,
    pub profile_id: Option<String>,
    pub customer_id: Option<String>,
    pub starting_after_id: Option<String>,
    pub ending_before_id: Option<String>,
    pub entity_type: Option<common_enums::PayoutEntityType>,
    pub limit: Option<u32>,
}

impl From<api_models::payouts::PayoutListConstraints> for PayoutFetchConstraints {
    fn from(value: api_models::payouts::PayoutListConstraints) -> Self {
        Self::List(Box::new(PayoutListParams {
            offset: 0,
            starting_at: value.created_gte.or(value.created_gt).or(value.created),
            ending_at: value.created_lte.or(value.created_lt).or(value.created),
            connector: None,
            currency: None,
            status: None,
            payout_method: None,
            profile_id: None,
            customer_id: value.customer_id,
            starting_after_id: value.starting_after,
            ending_before_id: value.ending_before,
            entity_type: None,
            limit: Some(std::cmp::min(
                value.limit,
                consts::PAYOUTS_LIST_MAX_LIMIT_GET,
            )),
        }))
    }
}

impl From<api_models::payments::TimeRange> for PayoutFetchConstraints {
    fn from(value: api_models::payments::TimeRange) -> Self {
        Self::List(Box::new(PayoutListParams {
            offset: 0,
            starting_at: Some(value.start_time),
            ending_at: value.end_time,
            connector: None,
            currency: None,
            status: None,
            payout_method: None,
            profile_id: None,
            customer_id: None,
            starting_after_id: None,
            ending_before_id: None,
            entity_type: None,
            limit: None,
        }))
    }
}

impl From<api_models::payouts::PayoutListFilterConstraints> for PayoutFetchConstraints {
    fn from(value: api_models::payouts::PayoutListFilterConstraints) -> Self {
        if let Some(payout_id) = value.payout_id {
            Self::Single { payout_id }
        } else {
            Self::List(Box::new(PayoutListParams {
                offset: value.offset.unwrap_or_default(),
                starting_at: value.time_range.map(|t| t.start_time),
                ending_at: value.time_range.and_then(|t| t.end_time),
                connector: value.connector,
                currency: value.currency,
                status: value.status,
                payout_method: value.payout_method,
                profile_id: value.profile_id,
                customer_id: value.customer_id,
                starting_after_id: None,
                ending_before_id: None,
                entity_type: value.entity_type,
                limit: Some(std::cmp::min(
                    value.limit,
                    consts::PAYOUTS_LIST_MAX_LIMIT_GET,
                )),
            }))
        }
    }
}
