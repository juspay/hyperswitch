use api_models::routing::{
    MerchantRoutingAlgorithm, RoutingAlgorithm as Algorithm, RoutingAlgorithmKind,
    RoutingDictionaryRecord,
};
#[cfg(all(feature = "v1", feature = "dynamic_routing"))]
use api_models::{
    open_router::{OpenRouterDecideGatewayRequest, PaymentInfo, RankingAlgorithm},
    routing::RoutableConnectorChoice,
};
use common_utils::ext_traits::ValueExt;
use diesel_models::{
    enums as storage_enums,
    routing_algorithm::{RoutingAlgorithm, RoutingProfileMetadata},
};
#[cfg(all(feature = "v1", feature = "dynamic_routing"))]
use hyperswitch_domain_models::payments::payment_attempt::PaymentAttempt;

use crate::{
    core::{errors, routing},
    types::transformers::{ForeignFrom, ForeignInto, ForeignTryFrom},
};

impl ForeignFrom<RoutingProfileMetadata> for RoutingDictionaryRecord {
    fn foreign_from(value: RoutingProfileMetadata) -> Self {
        Self {
            id: value.algorithm_id,

            profile_id: value.profile_id,
            name: value.name,
            kind: value.kind.foreign_into(),
            description: value.description.unwrap_or_default(),
            created_at: value.created_at.assume_utc().unix_timestamp(),
            modified_at: value.modified_at.assume_utc().unix_timestamp(),
            algorithm_for: Some(value.algorithm_for),
        }
    }
}

impl ForeignFrom<RoutingAlgorithm> for RoutingDictionaryRecord {
    fn foreign_from(value: RoutingAlgorithm) -> Self {
        Self {
            id: value.algorithm_id,

            profile_id: value.profile_id,
            name: value.name,
            kind: value.kind.foreign_into(),
            description: value.description.unwrap_or_default(),
            created_at: value.created_at.assume_utc().unix_timestamp(),
            modified_at: value.modified_at.assume_utc().unix_timestamp(),
            algorithm_for: Some(value.algorithm_for),
        }
    }
}

impl ForeignTryFrom<RoutingAlgorithm> for MerchantRoutingAlgorithm {
    type Error = error_stack::Report<errors::ParsingError>;

    fn foreign_try_from(value: RoutingAlgorithm) -> Result<Self, Self::Error> {
        Ok(Self {
            id: value.algorithm_id,
            name: value.name,

            profile_id: value.profile_id,
            description: value.description.unwrap_or_default(),
            algorithm: value
                .algorithm_data
                .parse_value::<Algorithm>("RoutingAlgorithm")?,
            created_at: value.created_at.assume_utc().unix_timestamp(),
            modified_at: value.modified_at.assume_utc().unix_timestamp(),
            algorithm_for: value.algorithm_for,
        })
    }
}

impl ForeignFrom<storage_enums::RoutingAlgorithmKind> for RoutingAlgorithmKind {
    fn foreign_from(value: storage_enums::RoutingAlgorithmKind) -> Self {
        match value {
            storage_enums::RoutingAlgorithmKind::Single => Self::Single,
            storage_enums::RoutingAlgorithmKind::Priority => Self::Priority,
            storage_enums::RoutingAlgorithmKind::VolumeSplit => Self::VolumeSplit,
            storage_enums::RoutingAlgorithmKind::Advanced => Self::Advanced,
            storage_enums::RoutingAlgorithmKind::Dynamic => Self::Dynamic,
        }
    }
}

impl ForeignFrom<RoutingAlgorithmKind> for storage_enums::RoutingAlgorithmKind {
    fn foreign_from(value: RoutingAlgorithmKind) -> Self {
        match value {
            RoutingAlgorithmKind::Single => Self::Single,
            RoutingAlgorithmKind::Priority => Self::Priority,
            RoutingAlgorithmKind::VolumeSplit => Self::VolumeSplit,
            RoutingAlgorithmKind::Advanced => Self::Advanced,
            RoutingAlgorithmKind::Dynamic => Self::Dynamic,
        }
    }
}

impl From<&routing::TransactionData<'_>> for storage_enums::TransactionType {
    fn from(value: &routing::TransactionData<'_>) -> Self {
        match value {
            routing::TransactionData::Payment(_) => Self::Payment,
            #[cfg(feature = "payouts")]
            routing::TransactionData::Payout(_) => Self::Payout,
        }
    }
}

#[cfg(all(feature = "v1", feature = "dynamic_routing"))]
pub trait OpenRouterDecideGatewayRequestExt {
    fn construct_sr_request(
        attempt: &PaymentAttempt,
        eligible_gateway_list: Vec<RoutableConnectorChoice>,
        ranking_algorithm: Option<RankingAlgorithm>,
        is_elimination_enabled: bool,
    ) -> Self
    where
        Self: Sized;
}

#[cfg(all(feature = "v1", feature = "dynamic_routing"))]
impl OpenRouterDecideGatewayRequestExt for OpenRouterDecideGatewayRequest {
    fn construct_sr_request(
        attempt: &PaymentAttempt,
        eligible_gateway_list: Vec<RoutableConnectorChoice>,
        ranking_algorithm: Option<RankingAlgorithm>,
        is_elimination_enabled: bool,
    ) -> Self {
        Self {
            payment_info: PaymentInfo {
                payment_id: attempt.payment_id.clone(),
                amount: attempt.net_amount.get_order_amount(),
                currency: attempt.currency.unwrap_or(storage_enums::Currency::USD),
                payment_type: "ORDER_PAYMENT".to_string(),
                // payment_method_type: attempt.payment_method_type.clone().unwrap(),
                payment_method_type: "UPI".into(), // TODO: once open-router makes this field string, we can send from attempt
                payment_method: attempt.payment_method.unwrap_or_default(),
            },
            merchant_id: attempt.profile_id.clone(),
            eligible_gateway_list: Some(
                eligible_gateway_list
                    .into_iter()
                    .map(|connector| connector.to_string())
                    .collect(),
            ),
            ranking_algorithm,
            elimination_enabled: Some(is_elimination_enabled),
        }
    }
}
