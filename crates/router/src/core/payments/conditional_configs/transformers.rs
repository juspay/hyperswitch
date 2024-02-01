use api_models::{self, conditional_configs};
use diesel_models::enums as storage_enums;
use euclid::enums as dsl_enums;

use crate::types::transformers::ForeignFrom;
impl ForeignFrom<dsl_enums::AuthenticationType> for conditional_configs::AuthenticationType {
        /// Converts a value of type `dsl_enums::AuthenticationType` into a value of the current type.
    fn foreign_from(from: dsl_enums::AuthenticationType) -> Self {
        match from {
            dsl_enums::AuthenticationType::ThreeDs => Self::ThreeDs,
            dsl_enums::AuthenticationType::NoThreeDs => Self::NoThreeDs,
        }
    }
}

impl ForeignFrom<conditional_configs::AuthenticationType> for storage_enums::AuthenticationType {
        /// Converts an enum value from conditional_configs::AuthenticationType to Self
    fn foreign_from(from: conditional_configs::AuthenticationType) -> Self {
        match from {
            conditional_configs::AuthenticationType::ThreeDs => Self::ThreeDs,
            conditional_configs::AuthenticationType::NoThreeDs => Self::NoThreeDs,
        }
    }
}
