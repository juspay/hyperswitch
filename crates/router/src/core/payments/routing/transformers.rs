use std::collections::HashMap;

use api_models::{self, routing as routing_types};
use diesel_models::enums as storage_enums;
use euclid::{enums as dsl_enums, frontend::ast as dsl_ast};
use kgraph_utils::types;

use crate::{
    configs::settings,
    types::transformers::{ForeignFrom, ForeignInto},
};

impl ForeignFrom<routing_types::RoutableConnectorChoice> for dsl_ast::ConnectorChoice {
    fn foreign_from(from: routing_types::RoutableConnectorChoice) -> Self {
        Self {
            connector: from.connector,
        }
    }
}

impl ForeignFrom<storage_enums::CaptureMethod> for Option<dsl_enums::CaptureMethod> {
    fn foreign_from(value: storage_enums::CaptureMethod) -> Self {
        match value {
            storage_enums::CaptureMethod::Automatic => Some(dsl_enums::CaptureMethod::Automatic),
            storage_enums::CaptureMethod::SequentialAutomatic => {
                Some(dsl_enums::CaptureMethod::SequentialAutomatic)
            }
            storage_enums::CaptureMethod::Manual => Some(dsl_enums::CaptureMethod::Manual),
            _ => None,
        }
    }
}

impl ForeignFrom<api_models::payments::AcceptanceType> for dsl_enums::MandateAcceptanceType {
    fn foreign_from(from: api_models::payments::AcceptanceType) -> Self {
        match from {
            api_models::payments::AcceptanceType::Online => Self::Online,
            api_models::payments::AcceptanceType::Offline => Self::Offline,
        }
    }
}

impl ForeignFrom<api_models::payments::MandateType> for dsl_enums::MandateType {
    fn foreign_from(from: api_models::payments::MandateType) -> Self {
        match from {
            api_models::payments::MandateType::MultiUse(_) => Self::MultiUse,
            api_models::payments::MandateType::SingleUse(_) => Self::SingleUse,
        }
    }
}

impl ForeignFrom<storage_enums::MandateDataType> for dsl_enums::MandateType {
    fn foreign_from(from: storage_enums::MandateDataType) -> Self {
        match from {
            storage_enums::MandateDataType::MultiUse(_) => Self::MultiUse,
            storage_enums::MandateDataType::SingleUse(_) => Self::SingleUse,
        }
    }
}

impl ForeignFrom<settings::PaymentMethodFilterKey> for types::PaymentMethodFilterKey {
    fn foreign_from(from: settings::PaymentMethodFilterKey) -> Self {
        match from {
            settings::PaymentMethodFilterKey::PaymentMethodType(pmt) => {
                Self::PaymentMethodType(pmt)
            }
            settings::PaymentMethodFilterKey::CardNetwork(cn) => Self::CardNetwork(cn),
        }
    }
}
impl ForeignFrom<settings::CurrencyCountryFlowFilter> for types::CurrencyCountryFlowFilter {
    fn foreign_from(from: settings::CurrencyCountryFlowFilter) -> Self {
        Self {
            currency: from.currency,
            country: from.country,
            not_available_flows: from.not_available_flows.map(ForeignInto::foreign_into),
        }
    }
}
impl ForeignFrom<settings::NotAvailableFlows> for types::NotAvailableFlows {
    fn foreign_from(from: settings::NotAvailableFlows) -> Self {
        Self {
            capture_method: from.capture_method,
        }
    }
}
impl ForeignFrom<settings::PaymentMethodFilters> for types::PaymentMethodFilters {
    fn foreign_from(from: settings::PaymentMethodFilters) -> Self {
        let iter_map = from
            .0
            .into_iter()
            .map(|(key, val)| (key.foreign_into(), val.foreign_into()))
            .collect::<HashMap<_, _>>();
        Self(iter_map)
    }
}
