use api_models::{self, routing as routing_types};
use diesel_models::enums as storage_enums;
use euclid::{enums as dsl_enums, frontend::ast as dsl_ast};

use crate::types::transformers::ForeignFrom;

impl ForeignFrom<routing_types::RoutableConnectorChoice> for dsl_ast::ConnectorChoice {
        /// Converts a `RoutableConnectorChoice` from a foreign type into Self.
    fn foreign_from(from: routing_types::RoutableConnectorChoice) -> Self {
        Self {
            // #[cfg(feature = "backwards_compatibility")]
            // choice_kind: from.choice_kind.foreign_into(),
            connector: from.connector,
            #[cfg(not(feature = "connector_choice_mca_id"))]
            sub_label: from.sub_label,
        }
    }
}

impl ForeignFrom<storage_enums::CaptureMethod> for Option<dsl_enums::CaptureMethod> {
        /// Converts a value from the storage_enums::CaptureMethod enum to the equivalent value in the current enum.
    fn foreign_from(value: storage_enums::CaptureMethod) -> Self {
        match value {
            storage_enums::CaptureMethod::Automatic => Some(dsl_enums::CaptureMethod::Automatic),
            storage_enums::CaptureMethod::Manual => Some(dsl_enums::CaptureMethod::Manual),
            _ => None,
        }
    }
}

impl ForeignFrom<api_models::payments::AcceptanceType> for dsl_enums::MandateAcceptanceType {
        /// Converts a value of type `api_models::payments::AcceptanceType` to an instance of the current type.
    fn foreign_from(from: api_models::payments::AcceptanceType) -> Self {
        match from {
            api_models::payments::AcceptanceType::Online => Self::Online,
            api_models::payments::AcceptanceType::Offline => Self::Offline,
        }
    }
}

impl ForeignFrom<api_models::payments::MandateType> for dsl_enums::MandateType {
        /// Converts a `MandateType` from the `api_models::payments` module into an instance of the current type.
    fn foreign_from(from: api_models::payments::MandateType) -> Self {
        match from {
            api_models::payments::MandateType::MultiUse(_) => Self::MultiUse,
            api_models::payments::MandateType::SingleUse(_) => Self::SingleUse,
        }
    }
}

impl ForeignFrom<storage_enums::MandateDataType> for dsl_enums::MandateType {
        /// Converts a value of type `storage_enums::MandateDataType` to an instance of the current enum type.
    fn foreign_from(from: storage_enums::MandateDataType) -> Self {
        match from {
            storage_enums::MandateDataType::MultiUse(_) => Self::MultiUse,
            storage_enums::MandateDataType::SingleUse(_) => Self::SingleUse,
        }
    }
}
