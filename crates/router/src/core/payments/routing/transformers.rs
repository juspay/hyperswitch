use api_models::{self, enums as api_enums, routing as routing_types};
use diesel_models::enums as storage_enums;
use euclid::{enums as dsl_enums, frontend::ast as dsl_ast};

use crate::types::transformers::{ForeignFrom, ForeignInto};

impl ForeignFrom<routing_types::RoutableConnectorChoice> for dsl_ast::ConnectorChoice {
    fn foreign_from(from: routing_types::RoutableConnectorChoice) -> Self {
        Self {
            // #[cfg(feature = "backwards_compatibility")]
            // choice_kind: from.choice_kind.foreign_into(),
            connector: from.connector.foreign_into(),
            #[cfg(not(feature = "connector_choice_mca_id"))]
            sub_label: from.sub_label,
        }
    }
}

impl ForeignFrom<storage_enums::CaptureMethod> for Option<dsl_enums::CaptureMethod> {
    fn foreign_from(value: storage_enums::CaptureMethod) -> Self {
        match value {
            storage_enums::CaptureMethod::Automatic => Some(dsl_enums::CaptureMethod::Automatic),
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

impl ForeignFrom<api_enums::RoutableConnectors> for dsl_enums::Connector {
    fn foreign_from(from: api_enums::RoutableConnectors) -> Self {
        match from {
            #[cfg(feature = "dummy_connector")]
            api_enums::RoutableConnectors::DummyConnector1 => Self::DummyConnector1,
            #[cfg(feature = "dummy_connector")]
            api_enums::RoutableConnectors::DummyConnector2 => Self::DummyConnector2,
            #[cfg(feature = "dummy_connector")]
            api_enums::RoutableConnectors::DummyConnector3 => Self::DummyConnector3,
            #[cfg(feature = "dummy_connector")]
            api_enums::RoutableConnectors::DummyConnector4 => Self::DummyConnector4,
            #[cfg(feature = "dummy_connector")]
            api_enums::RoutableConnectors::DummyConnector5 => Self::DummyConnector5,
            #[cfg(feature = "dummy_connector")]
            api_enums::RoutableConnectors::DummyConnector6 => Self::DummyConnector6,
            #[cfg(feature = "dummy_connector")]
            api_enums::RoutableConnectors::DummyConnector7 => Self::DummyConnector7,
            api_enums::RoutableConnectors::Aci => Self::Aci,
            api_enums::RoutableConnectors::Adyen => Self::Adyen,
            api_enums::RoutableConnectors::Airwallex => Self::Airwallex,
            api_enums::RoutableConnectors::Authorizedotnet => Self::Authorizedotnet,
            api_enums::RoutableConnectors::Bitpay => Self::Bitpay,
            api_enums::RoutableConnectors::Bambora => Self::Bambora,
            api_enums::RoutableConnectors::Bluesnap => Self::Bluesnap,
            api_enums::RoutableConnectors::Boku => Self::Boku,
            api_enums::RoutableConnectors::Braintree => Self::Braintree,
            api_enums::RoutableConnectors::Cashtocode => Self::Cashtocode,
            api_enums::RoutableConnectors::Checkout => Self::Checkout,
            api_enums::RoutableConnectors::Coinbase => Self::Coinbase,
            api_enums::RoutableConnectors::Cryptopay => Self::Cryptopay,
            api_enums::RoutableConnectors::Cybersource => Self::Cybersource,
            api_enums::RoutableConnectors::Dlocal => Self::Dlocal,
            api_enums::RoutableConnectors::Fiserv => Self::Fiserv,
            api_enums::RoutableConnectors::Forte => Self::Forte,
            api_enums::RoutableConnectors::Globalpay => Self::Globalpay,
            api_enums::RoutableConnectors::Globepay => Self::Globepay,
            api_enums::RoutableConnectors::Gocardless => Self::Gocardless,
            api_enums::RoutableConnectors::Helcim => Self::Helcim,
            api_enums::RoutableConnectors::Iatapay => Self::Iatapay,
            api_enums::RoutableConnectors::Klarna => Self::Klarna,
            api_enums::RoutableConnectors::Mollie => Self::Mollie,
            api_enums::RoutableConnectors::Multisafepay => Self::Multisafepay,
            api_enums::RoutableConnectors::Nexinets => Self::Nexinets,
            api_enums::RoutableConnectors::Nmi => Self::Nmi,
            api_enums::RoutableConnectors::Noon => Self::Noon,
            api_enums::RoutableConnectors::Nuvei => Self::Nuvei,
            api_enums::RoutableConnectors::Opennode => Self::Opennode,
            api_enums::RoutableConnectors::Payme => Self::Payme,
            api_enums::RoutableConnectors::Paypal => Self::Paypal,
            api_enums::RoutableConnectors::Payu => Self::Payu,
            api_enums::RoutableConnectors::Powertranz => Self::Powertranz,
            api_enums::RoutableConnectors::Rapyd => Self::Rapyd,
            api_enums::RoutableConnectors::Shift4 => Self::Shift4,
            api_enums::RoutableConnectors::Square => Self::Square,
            api_enums::RoutableConnectors::Stax => Self::Stax,
            api_enums::RoutableConnectors::Stripe => Self::Stripe,
            api_enums::RoutableConnectors::Trustpay => Self::Trustpay,
            api_enums::RoutableConnectors::Tsys => Self::Tsys,
            api_enums::RoutableConnectors::Volt => Self::Volt,
            api_enums::RoutableConnectors::Wise => Self::Wise,
            api_enums::RoutableConnectors::Worldline => Self::Worldline,
            api_enums::RoutableConnectors::Worldpay => Self::Worldpay,
            api_enums::RoutableConnectors::Zen => Self::Zen,
        }
    }
}
