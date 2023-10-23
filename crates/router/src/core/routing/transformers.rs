use api_models::routing::{
    MerchantRoutingAlgorithm, RoutingAlgorithm as Algorithm, RoutingAlgorithmKind,
    RoutingDictionaryRecord,
};
use common_utils::ext_traits::ValueExt;
use diesel_models::{
    enums as storage_enums,
    routing_algorithm::{RoutingAlgorithm, RoutingProfileMetadata},
};

use crate::{
    core::errors,
    types::transformers::{ForeignFrom, ForeignInto, ForeignTryFrom},
};

impl ForeignFrom<RoutingProfileMetadata> for RoutingDictionaryRecord {
    fn foreign_from(value: RoutingProfileMetadata) -> Self {
        Self {
            id: value.algorithm_id,
            #[cfg(feature = "business_profile_routing")]
            profile_id: value.profile_id,
            name: value.name,

            kind: value.kind.foreign_into(),
            description: value.description.unwrap_or_default(),
            created_at: value.created_at.assume_utc().unix_timestamp(),
            modified_at: value.modified_at.assume_utc().unix_timestamp(),
        }
    }
}

impl ForeignFrom<RoutingAlgorithm> for RoutingDictionaryRecord {
    fn foreign_from(value: RoutingAlgorithm) -> Self {
        Self {
            id: value.algorithm_id,
            #[cfg(feature = "business_profile_routing")]
            profile_id: value.profile_id,
            name: value.name,
            kind: value.kind.foreign_into(),
            description: value.description.unwrap_or_default(),
            created_at: value.created_at.assume_utc().unix_timestamp(),
            modified_at: value.modified_at.assume_utc().unix_timestamp(),
        }
    }
}

impl ForeignTryFrom<RoutingAlgorithm> for MerchantRoutingAlgorithm {
    type Error = error_stack::Report<errors::ParsingError>;

    fn foreign_try_from(value: RoutingAlgorithm) -> Result<Self, Self::Error> {
        Ok(Self {
            id: value.algorithm_id,
            name: value.name,
            #[cfg(feature = "business_profile_routing")]
            profile_id: value.profile_id,
            description: value.description.unwrap_or_default(),
            algorithm: value
                .algorithm_data
                .parse_value::<Algorithm>("RoutingAlgorithm")?,
            created_at: value.created_at.assume_utc().unix_timestamp(),
            modified_at: value.modified_at.assume_utc().unix_timestamp(),
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
        }
    }
}
