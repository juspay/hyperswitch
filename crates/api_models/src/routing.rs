use std::fmt::Debug;

use common_utils::errors::ParsingError;
use error_stack::IntoReport;
use euclid::{
    dssa::types::EuclidAnalysable,
    enums as euclid_enums,
    frontend::{
        ast,
        dir::{DirKeyKind, EuclidDirFilter},
    },
};
use serde::{Deserialize, Serialize};

use crate::enums::{self, RoutableConnectors};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data", rename_all = "snake_case")]
pub enum ConnectorSelection {
    Priority(Vec<RoutableConnectorChoice>),
    VolumeSplit(Vec<ConnectorVolumeSplit>),
}

impl ConnectorSelection {
    pub fn get_connector_list(&self) -> Vec<RoutableConnectorChoice> {
        match self {
            Self::Priority(list) => list.clone(),
            Self::VolumeSplit(splits) => {
                splits.iter().map(|split| split.connector.clone()).collect()
            }
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RoutingConfigRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub algorithm: Option<RoutingAlgorithm>,
    pub profile_id: Option<String>,
}

#[cfg(feature = "business_profile_routing")]
#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct RoutingRetrieveQuery {
    pub limit: Option<u16>,
    pub offset: Option<u8>,

    pub profile_id: Option<String>,
}

#[cfg(feature = "business_profile_routing")]
#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct RoutingRetrieveLinkQuery {
    pub profile_id: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RoutingRetrieveResponse {
    pub algorithm: Option<MerchantRoutingAlgorithm>,
}

#[derive(Debug, serde::Serialize)]
#[serde(untagged)]
pub enum LinkedRoutingConfigRetrieveResponse {
    MerchantAccountBased(RoutingRetrieveResponse),
    ProfileBased(Vec<RoutingDictionaryRecord>),
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MerchantRoutingAlgorithm {
    pub id: String,
    #[cfg(feature = "business_profile_routing")]
    pub profile_id: String,
    pub name: String,
    pub description: String,
    pub algorithm: RoutingAlgorithm,
    pub created_at: i64,
    pub modified_at: i64,
}

impl EuclidDirFilter for ConnectorSelection {
    const ALLOWED: &'static [DirKeyKind] = &[
        DirKeyKind::PaymentMethod,
        DirKeyKind::CardBin,
        DirKeyKind::CardType,
        DirKeyKind::CardNetwork,
        DirKeyKind::PayLaterType,
        DirKeyKind::WalletType,
        DirKeyKind::UpiType,
        DirKeyKind::BankRedirectType,
        DirKeyKind::BankDebitType,
        DirKeyKind::CryptoType,
        DirKeyKind::MetaData,
        DirKeyKind::PaymentAmount,
        DirKeyKind::PaymentCurrency,
        DirKeyKind::AuthenticationType,
        DirKeyKind::MandateAcceptanceType,
        DirKeyKind::MandateType,
        DirKeyKind::PaymentType,
        DirKeyKind::SetupFutureUsage,
        DirKeyKind::CaptureMethod,
        DirKeyKind::BillingCountry,
        DirKeyKind::BusinessCountry,
        DirKeyKind::BusinessLabel,
        DirKeyKind::MetaData,
        DirKeyKind::RewardType,
        DirKeyKind::VoucherType,
        DirKeyKind::CardRedirectType,
        DirKeyKind::BankTransferType,
    ];
}

impl EuclidAnalysable for ConnectorSelection {
    fn get_dir_value_for_analysis(
        &self,
        rule_name: String,
    ) -> Vec<(euclid::frontend::dir::DirValue, euclid::types::Metadata)> {
        self.get_connector_list()
            .into_iter()
            .map(|connector_choice| {
                let connector_name = connector_choice.connector.to_string();
                #[cfg(not(feature = "connector_choice_mca_id"))]
                let sub_label = connector_choice.sub_label.clone();
                #[cfg(feature = "connector_choice_mca_id")]
                let mca_id = connector_choice.merchant_connector_id.clone();

                (
                    euclid::frontend::dir::DirValue::Connector(Box::new(connector_choice.into())),
                    std::collections::HashMap::from_iter([(
                        "CONNECTOR_SELECTION".to_string(),
                        #[cfg(feature = "connector_choice_mca_id")]
                        serde_json::json!({
                            "rule_name": rule_name,
                            "connector_name": connector_name,
                            "mca_id": mca_id,
                        }),
                        #[cfg(not(feature = "connector_choice_mca_id"))]
                        serde_json ::json!({
                            "rule_name": rule_name,
                            "connector_name": connector_name,
                            "sub_label": sub_label,
                        }),
                    )]),
                )
            })
            .collect()
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ConnectorVolumeSplit {
    pub connector: RoutableConnectorChoice,
    pub split: u8,
}

#[cfg(feature = "connector_choice_bcompat")]
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub enum RoutableChoiceKind {
    OnlyConnector,
    FullStruct,
}

#[cfg(feature = "connector_choice_bcompat")]
#[derive(Debug, serde::Deserialize, serde::Serialize)]
#[serde(untagged)]
pub enum RoutableChoiceSerde {
    OnlyConnector(Box<RoutableConnectors>),
    FullStruct {
        connector: RoutableConnectors,
        #[cfg(feature = "connector_choice_mca_id")]
        merchant_connector_id: Option<String>,
        #[cfg(not(feature = "connector_choice_mca_id"))]
        sub_label: Option<String>,
    },
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[cfg_attr(
    feature = "connector_choice_bcompat",
    serde(from = "RoutableChoiceSerde"),
    serde(into = "RoutableChoiceSerde")
)]
#[cfg_attr(not(feature = "connector_choice_bcompat"), derive(PartialEq, Eq))]
pub struct RoutableConnectorChoice {
    #[cfg(feature = "connector_choice_bcompat")]
    pub choice_kind: RoutableChoiceKind,
    pub connector: RoutableConnectors,
    #[cfg(feature = "connector_choice_mca_id")]
    pub merchant_connector_id: Option<String>,
    #[cfg(not(feature = "connector_choice_mca_id"))]
    pub sub_label: Option<String>,
}

impl ToString for RoutableConnectorChoice {
    fn to_string(&self) -> String {
        #[cfg(feature = "connector_choice_mca_id")]
        let base = self.connector.to_string();

        #[cfg(not(feature = "connector_choice_mca_id"))]
        let base = {
            let mut sub_base = self.connector.to_string();
            if let Some(ref label) = self.sub_label {
                sub_base.push('_');
                sub_base.push_str(label);
            }

            sub_base
        };

        base
    }
}

#[cfg(feature = "connector_choice_bcompat")]
impl PartialEq for RoutableConnectorChoice {
    fn eq(&self, other: &Self) -> bool {
        #[cfg(not(feature = "connector_choice_mca_id"))]
        {
            self.connector.eq(&other.connector) && self.sub_label.eq(&other.sub_label)
        }

        #[cfg(feature = "connector_choice_mca_id")]
        {
            self.connector.eq(&other.connector)
                && self.merchant_connector_id.eq(&other.merchant_connector_id)
        }
    }
}

#[cfg(feature = "connector_choice_bcompat")]
impl Eq for RoutableConnectorChoice {}

#[cfg(feature = "connector_choice_bcompat")]
impl From<RoutableChoiceSerde> for RoutableConnectorChoice {
    fn from(value: RoutableChoiceSerde) -> Self {
        match value {
            RoutableChoiceSerde::OnlyConnector(connector) => Self {
                choice_kind: RoutableChoiceKind::OnlyConnector,
                connector: *connector,
                #[cfg(feature = "connector_choice_mca_id")]
                merchant_connector_id: None,
                #[cfg(not(feature = "connector_choice_mca_id"))]
                sub_label: None,
            },

            RoutableChoiceSerde::FullStruct {
                connector,
                #[cfg(feature = "connector_choice_mca_id")]
                merchant_connector_id,
                #[cfg(not(feature = "connector_choice_mca_id"))]
                sub_label,
            } => Self {
                choice_kind: RoutableChoiceKind::FullStruct,
                connector,
                #[cfg(feature = "connector_choice_mca_id")]
                merchant_connector_id,
                #[cfg(not(feature = "connector_choice_mca_id"))]
                sub_label,
            },
        }
    }
}

#[cfg(feature = "connector_choice_bcompat")]
impl From<RoutableConnectorChoice> for RoutableChoiceSerde {
    fn from(value: RoutableConnectorChoice) -> Self {
        match value.choice_kind {
            RoutableChoiceKind::OnlyConnector => Self::OnlyConnector(Box::new(value.connector)),
            RoutableChoiceKind::FullStruct => Self::FullStruct {
                connector: value.connector,
                #[cfg(feature = "connector_choice_mca_id")]
                merchant_connector_id: value.merchant_connector_id,
                #[cfg(not(feature = "connector_choice_mca_id"))]
                sub_label: value.sub_label,
            },
        }
    }
}

impl From<RoutableConnectorChoice> for ast::ConnectorChoice {
    fn from(value: RoutableConnectorChoice) -> Self {
        Self {
            connector: match value.connector {
                #[cfg(feature = "dummy_connector")]
                RoutableConnectors::DummyConnector1 => euclid_enums::Connector::DummyConnector1,
                #[cfg(feature = "dummy_connector")]
                RoutableConnectors::DummyConnector2 => euclid_enums::Connector::DummyConnector2,
                #[cfg(feature = "dummy_connector")]
                RoutableConnectors::DummyConnector3 => euclid_enums::Connector::DummyConnector3,
                #[cfg(feature = "dummy_connector")]
                RoutableConnectors::DummyConnector4 => euclid_enums::Connector::DummyConnector4,
                #[cfg(feature = "dummy_connector")]
                RoutableConnectors::DummyConnector5 => euclid_enums::Connector::DummyConnector5,
                #[cfg(feature = "dummy_connector")]
                RoutableConnectors::DummyConnector6 => euclid_enums::Connector::DummyConnector6,
                #[cfg(feature = "dummy_connector")]
                RoutableConnectors::DummyConnector7 => euclid_enums::Connector::DummyConnector7,
                RoutableConnectors::Aci => euclid_enums::Connector::Aci,
                RoutableConnectors::Adyen => euclid_enums::Connector::Adyen,
                RoutableConnectors::Airwallex => euclid_enums::Connector::Airwallex,
                RoutableConnectors::Authorizedotnet => euclid_enums::Connector::Authorizedotnet,
                RoutableConnectors::Bitpay => euclid_enums::Connector::Bitpay,
                RoutableConnectors::Bambora => euclid_enums::Connector::Bambora,
                RoutableConnectors::Bluesnap => euclid_enums::Connector::Bluesnap,
                RoutableConnectors::Boku => euclid_enums::Connector::Boku,
                RoutableConnectors::Braintree => euclid_enums::Connector::Braintree,
                RoutableConnectors::Cashtocode => euclid_enums::Connector::Cashtocode,
                RoutableConnectors::Checkout => euclid_enums::Connector::Checkout,
                RoutableConnectors::Coinbase => euclid_enums::Connector::Coinbase,
                RoutableConnectors::Cryptopay => euclid_enums::Connector::Cryptopay,
                RoutableConnectors::Cybersource => euclid_enums::Connector::Cybersource,
                RoutableConnectors::Dlocal => euclid_enums::Connector::Dlocal,
                RoutableConnectors::Fiserv => euclid_enums::Connector::Fiserv,
                RoutableConnectors::Forte => euclid_enums::Connector::Forte,
                RoutableConnectors::Globalpay => euclid_enums::Connector::Globalpay,
                RoutableConnectors::Globepay => euclid_enums::Connector::Globepay,
                RoutableConnectors::Gocardless => euclid_enums::Connector::Gocardless,
                RoutableConnectors::Helcim => euclid_enums::Connector::Helcim,
                RoutableConnectors::Iatapay => euclid_enums::Connector::Iatapay,
                RoutableConnectors::Klarna => euclid_enums::Connector::Klarna,
                RoutableConnectors::Mollie => euclid_enums::Connector::Mollie,
                RoutableConnectors::Multisafepay => euclid_enums::Connector::Multisafepay,
                RoutableConnectors::Nexinets => euclid_enums::Connector::Nexinets,
                RoutableConnectors::Nmi => euclid_enums::Connector::Nmi,
                RoutableConnectors::Noon => euclid_enums::Connector::Noon,
                RoutableConnectors::Nuvei => euclid_enums::Connector::Nuvei,
                RoutableConnectors::Opennode => euclid_enums::Connector::Opennode,
                RoutableConnectors::Payme => euclid_enums::Connector::Payme,
                RoutableConnectors::Paypal => euclid_enums::Connector::Paypal,
                RoutableConnectors::Payu => euclid_enums::Connector::Payu,
                RoutableConnectors::Powertranz => euclid_enums::Connector::Powertranz,
                RoutableConnectors::Rapyd => euclid_enums::Connector::Rapyd,
                RoutableConnectors::Shift4 => euclid_enums::Connector::Shift4,
                RoutableConnectors::Square => euclid_enums::Connector::Square,
                RoutableConnectors::Stax => euclid_enums::Connector::Stax,
                RoutableConnectors::Stripe => euclid_enums::Connector::Stripe,
                RoutableConnectors::Trustpay => euclid_enums::Connector::Trustpay,
                RoutableConnectors::Tsys => euclid_enums::Connector::Tsys,
                RoutableConnectors::Volt => euclid_enums::Connector::Volt,
                RoutableConnectors::Wise => euclid_enums::Connector::Wise,
                RoutableConnectors::Worldline => euclid_enums::Connector::Worldline,
                RoutableConnectors::Worldpay => euclid_enums::Connector::Worldpay,
                RoutableConnectors::Zen => euclid_enums::Connector::Zen,
            },

            #[cfg(not(feature = "connector_choice_mca_id"))]
            sub_label: value.sub_label,
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DetailedConnectorChoice {
    pub connector: RoutableConnectors,
    pub business_label: Option<String>,
    pub business_country: Option<enums::CountryAlpha2>,
    pub business_sub_label: Option<String>,
}

impl DetailedConnectorChoice {
    pub fn get_connector_label(&self) -> Option<String> {
        self.business_country
            .as_ref()
            .zip(self.business_label.as_ref())
            .map(|(business_country, business_label)| {
                let mut base_label = format!(
                    "{}_{:?}_{}",
                    self.connector, business_country, business_label
                );

                if let Some(ref sub_label) = self.business_sub_label {
                    base_label.push('_');
                    base_label.push_str(sub_label);
                }

                base_label
            })
    }
}

#[derive(Debug, Copy, Clone, serde::Serialize, serde::Deserialize, strum::Display)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum RoutingAlgorithmKind {
    Single,
    Priority,
    VolumeSplit,
    Advanced,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(
    tag = "type",
    content = "data",
    rename_all = "snake_case",
    try_from = "RoutingAlgorithmSerde"
)]
pub enum RoutingAlgorithm {
    Single(Box<RoutableConnectorChoice>),
    Priority(Vec<RoutableConnectorChoice>),
    VolumeSplit(Vec<ConnectorVolumeSplit>),
    Advanced(euclid::frontend::ast::Program<ConnectorSelection>),
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type", content = "data", rename_all = "snake_case")]
pub enum RoutingAlgorithmSerde {
    Single(Box<RoutableConnectorChoice>),
    Priority(Vec<RoutableConnectorChoice>),
    VolumeSplit(Vec<ConnectorVolumeSplit>),
    Advanced(euclid::frontend::ast::Program<ConnectorSelection>),
}

impl TryFrom<RoutingAlgorithmSerde> for RoutingAlgorithm {
    type Error = error_stack::Report<ParsingError>;

    fn try_from(value: RoutingAlgorithmSerde) -> Result<Self, Self::Error> {
        match &value {
            RoutingAlgorithmSerde::Priority(i) if i.is_empty() => {
                Err(ParsingError::StructParseFailure(
                    "Connectors list can't be empty for Priority Algorithm",
                ))
                .into_report()?
            }
            RoutingAlgorithmSerde::VolumeSplit(i) if i.is_empty() => {
                Err(ParsingError::StructParseFailure(
                    "Connectors list can't be empty for Volume split Algorithm",
                ))
                .into_report()?
            }
            _ => {}
        };
        Ok(match value {
            RoutingAlgorithmSerde::Single(i) => Self::Single(i),
            RoutingAlgorithmSerde::Priority(i) => Self::Priority(i),
            RoutingAlgorithmSerde::VolumeSplit(i) => Self::VolumeSplit(i),
            RoutingAlgorithmSerde::Advanced(i) => Self::Advanced(i),
        })
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(
    tag = "type",
    content = "data",
    rename_all = "snake_case",
    try_from = "StraightThroughAlgorithmSerde",
    into = "StraightThroughAlgorithmSerde"
)]
pub enum StraightThroughAlgorithm {
    Single(Box<RoutableConnectorChoice>),
    Priority(Vec<RoutableConnectorChoice>),
    VolumeSplit(Vec<ConnectorVolumeSplit>),
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type", content = "data", rename_all = "snake_case")]
pub enum StraightThroughAlgorithmInner {
    Single(Box<RoutableConnectorChoice>),
    Priority(Vec<RoutableConnectorChoice>),
    VolumeSplit(Vec<ConnectorVolumeSplit>),
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(untagged)]
pub enum StraightThroughAlgorithmSerde {
    Direct(StraightThroughAlgorithmInner),
    Nested {
        algorithm: StraightThroughAlgorithmInner,
    },
}

impl TryFrom<StraightThroughAlgorithmSerde> for StraightThroughAlgorithm {
    type Error = error_stack::Report<ParsingError>;

    fn try_from(value: StraightThroughAlgorithmSerde) -> Result<Self, Self::Error> {
        let inner = match value {
            StraightThroughAlgorithmSerde::Direct(algorithm) => algorithm,
            StraightThroughAlgorithmSerde::Nested { algorithm } => algorithm,
        };

        match &inner {
            StraightThroughAlgorithmInner::Priority(i) if i.is_empty() => {
                Err(ParsingError::StructParseFailure(
                    "Connectors list can't be empty for Priority Algorithm",
                ))
                .into_report()?
            }
            StraightThroughAlgorithmInner::VolumeSplit(i) if i.is_empty() => {
                Err(ParsingError::StructParseFailure(
                    "Connectors list can't be empty for Volume split Algorithm",
                ))
                .into_report()?
            }
            _ => {}
        };

        Ok(match inner {
            StraightThroughAlgorithmInner::Single(single) => Self::Single(single),
            StraightThroughAlgorithmInner::Priority(plist) => Self::Priority(plist),
            StraightThroughAlgorithmInner::VolumeSplit(vsplit) => Self::VolumeSplit(vsplit),
        })
    }
}

impl From<StraightThroughAlgorithm> for StraightThroughAlgorithmSerde {
    fn from(value: StraightThroughAlgorithm) -> Self {
        let inner = match value {
            StraightThroughAlgorithm::Single(conn) => StraightThroughAlgorithmInner::Single(conn),
            StraightThroughAlgorithm::Priority(plist) => {
                StraightThroughAlgorithmInner::Priority(plist)
            }
            StraightThroughAlgorithm::VolumeSplit(vsplit) => {
                StraightThroughAlgorithmInner::VolumeSplit(vsplit)
            }
        };

        Self::Nested { algorithm: inner }
    }
}

impl From<StraightThroughAlgorithm> for RoutingAlgorithm {
    fn from(value: StraightThroughAlgorithm) -> Self {
        match value {
            StraightThroughAlgorithm::Single(conn) => Self::Single(conn),
            StraightThroughAlgorithm::Priority(conns) => Self::Priority(conns),
            StraightThroughAlgorithm::VolumeSplit(splits) => Self::VolumeSplit(splits),
        }
    }
}

impl RoutingAlgorithm {
    pub fn get_kind(&self) -> RoutingAlgorithmKind {
        match self {
            Self::Single(_) => RoutingAlgorithmKind::Single,
            Self::Priority(_) => RoutingAlgorithmKind::Priority,
            Self::VolumeSplit(_) => RoutingAlgorithmKind::VolumeSplit,
            Self::Advanced(_) => RoutingAlgorithmKind::Advanced,
        }
    }
}

#[derive(Debug, Default, Clone, serde::Serialize, serde::Deserialize)]
pub struct RoutingAlgorithmRef {
    pub algorithm_id: Option<String>,
    pub timestamp: i64,
    pub config_algo_id: Option<String>,
    pub surcharge_config_algo_id: Option<String>,
}

impl RoutingAlgorithmRef {
    pub fn update_algorithm_id(&mut self, new_id: String) {
        self.algorithm_id = Some(new_id);
        self.timestamp = common_utils::date_time::now_unix_timestamp();
    }

    pub fn update_conditional_config_id(&mut self, ids: String) {
        self.config_algo_id = Some(ids);
        self.timestamp = common_utils::date_time::now_unix_timestamp();
    }

    pub fn update_surcharge_config_id(&mut self, ids: String) {
        self.surcharge_config_algo_id = Some(ids);
        self.timestamp = common_utils::date_time::now_unix_timestamp();
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]

pub struct RoutingDictionaryRecord {
    pub id: String,
    #[cfg(feature = "business_profile_routing")]
    pub profile_id: String,
    pub name: String,
    pub kind: RoutingAlgorithmKind,
    pub description: String,
    pub created_at: i64,
    pub modified_at: i64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RoutingDictionary {
    pub merchant_id: String,
    pub active_id: Option<String>,
    pub records: Vec<RoutingDictionaryRecord>,
}

#[derive(serde::Serialize, serde::Deserialize, Debug)]
#[serde(untagged)]
pub enum RoutingKind {
    Config(RoutingDictionary),
    RoutingAlgorithm(Vec<RoutingDictionaryRecord>),
}
