use diesel_models::enums as storage_enums;
use euclid::enums as dsl_enums;

use crate::types::transformers::ForeignFrom;
impl ForeignFrom<dsl_enums::AuthenticationType> for common_types::payments::AuthenticationType {
    fn foreign_from(from: dsl_enums::AuthenticationType) -> Self {
        match from {
            dsl_enums::AuthenticationType::ThreeDs => Self::ThreeDs,
            dsl_enums::AuthenticationType::NoThreeDs => Self::NoThreeDs,
        }
    }
}

impl ForeignFrom<common_types::payments::AuthenticationType> for storage_enums::AuthenticationType {
    fn foreign_from(from: common_types::payments::AuthenticationType) -> Self {
        match from {
            common_types::payments::AuthenticationType::ThreeDs => Self::ThreeDs,
            common_types::payments::AuthenticationType::NoThreeDs => Self::NoThreeDs,
        }
    }
}
