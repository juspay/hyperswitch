pub mod admin;
pub mod api_keys;
pub mod authentication;
pub mod configs;
#[cfg(feature = "olap")]
pub mod connector_onboarding;
pub mod customers;
pub mod disputes;
pub mod enums;
pub mod ephemeral_key;
pub mod files;
#[cfg(feature = "frm")]
pub mod fraud_check;
pub mod mandates;
pub mod payment_link;
pub mod payment_methods;
pub mod payments;
#[cfg(feature = "payouts")]
pub mod payouts;
pub mod poll;
pub mod refunds;
pub mod routing;
#[cfg(feature = "olap")]
pub mod verify_connector;
#[cfg(feature = "olap")]
pub mod webhook_events;
pub mod webhooks;

pub mod authentication_v2;
pub mod connector_mapping;
pub mod disputes_v2;
pub mod feature_matrix;
pub mod files_v2;
#[cfg(feature = "frm")]
pub mod fraud_check_v2;
pub mod payments_v2;
#[cfg(feature = "payouts")]
pub mod payouts_v2;
pub mod refunds_v2;

use std::{fmt::Debug, str::FromStr};

use api_models::routing::{self as api_routing, RoutableConnectorChoice};
use common_enums::RoutableConnectors;
use error_stack::ResultExt;
pub use hyperswitch_domain_models::router_flow_types::{
    access_token_auth::{AccessTokenAuth, AccessTokenAuthentication},
    mandate_revoke::MandateRevoke,
    unified_authentication_service::*,
    webhooks::VerifyWebhookSource,
};
pub use hyperswitch_interfaces::{
    api::{
        authentication::{
            ConnectorAuthentication, ConnectorPostAuthentication, ConnectorPreAuthentication,
            ConnectorPreAuthenticationVersionCall, ExternalAuthentication,
        },
        authentication_v2::{
            ConnectorAuthenticationV2, ConnectorPostAuthenticationV2, ConnectorPreAuthenticationV2,
            ConnectorPreAuthenticationVersionCallV2, ExternalAuthenticationV2,
        },
        fraud_check::FraudCheck,
        revenue_recovery::{
            BillingConnectorInvoiceSyncIntegration, BillingConnectorPaymentsSyncIntegration,
            RevenueRecovery, RevenueRecoveryRecordBack,
        },
        revenue_recovery_v2::RevenueRecoveryV2,
        BoxedConnector, Connector, ConnectorAccessToken, ConnectorAccessTokenV2,
        ConnectorAuthenticationToken, ConnectorAuthenticationTokenV2, ConnectorCommon,
        ConnectorCommonExt, ConnectorMandateRevoke, ConnectorMandateRevokeV2,
        ConnectorTransactionId, ConnectorVerifyWebhookSource, ConnectorVerifyWebhookSourceV2,
        CurrencyUnit,
    },
    connector_integration_v2::{BoxedConnectorV2, ConnectorV2},
};
use rustc_hash::FxHashMap;

#[cfg(feature = "frm")]
pub use self::fraud_check::*;
#[cfg(feature = "payouts")]
pub use self::payouts::*;
pub use self::{
    admin::*, api_keys::*, authentication::*, configs::*, connector_mapping::*, customers::*,
    disputes::*, files::*, payment_link::*, payment_methods::*, payments::*, poll::*, refunds::*,
    refunds_v2::*, webhooks::*,
};
use super::transformers::ForeignTryFrom;
use crate::{
    connector, consts,
    core::{
        errors::{self, CustomResult},
        payments::types as payments_types,
    },
    services::connector_integration_interface::ConnectorEnum,
    types::{self, api::enums as api_enums},
};
#[derive(Clone)]
pub enum ConnectorCallType {
    PreDetermined(ConnectorRoutingData),
    Retryable(Vec<ConnectorRoutingData>),
    SessionMultiple(SessionConnectorDatas),
    #[cfg(feature = "v2")]
    Skip,
}

impl From<ConnectorData> for ConnectorRoutingData {
    fn from(connector_data: ConnectorData) -> Self {
        Self {
            connector_data,
            network: None,
            action_type: None,
        }
    }
}

#[derive(Clone, Debug)]
pub struct SessionConnectorData {
    pub payment_method_sub_type: api_enums::PaymentMethodType,
    pub payment_method_type: api_enums::PaymentMethod,
    pub connector: ConnectorData,
    pub business_sub_label: Option<String>,
}

impl SessionConnectorData {
    pub fn new(
        payment_method_sub_type: api_enums::PaymentMethodType,
        connector: ConnectorData,
        business_sub_label: Option<String>,
        payment_method_type: api_enums::PaymentMethod,
    ) -> Self {
        Self {
            payment_method_sub_type,
            connector,
            business_sub_label,
            payment_method_type,
        }
    }
}

common_utils::create_list_wrapper!(
    SessionConnectorDatas,
    SessionConnectorData,
    impl_functions: {
        pub fn apply_filter_for_session_routing(&self) -> Self {
            let routing_enabled_pmts = &consts::ROUTING_ENABLED_PAYMENT_METHOD_TYPES;
            let routing_enabled_pms = &consts::ROUTING_ENABLED_PAYMENT_METHODS;
            self
                .iter()
                .filter(|connector_data| {
                    routing_enabled_pmts.contains(&connector_data.payment_method_sub_type)
                        || routing_enabled_pms.contains(&connector_data.payment_method_type)
                })
                .cloned()
                .collect()
        }
        pub fn filter_and_validate_for_session_flow(self, routing_results: &FxHashMap<api_enums::PaymentMethodType, Vec<routing::SessionRoutingChoice>>) -> Result<Self, errors::ApiErrorResponse> {
            let mut final_list = Self::new(Vec::new());
            let routing_enabled_pmts = &consts::ROUTING_ENABLED_PAYMENT_METHOD_TYPES;
            for connector_data in self {
                if !routing_enabled_pmts.contains(&connector_data.payment_method_sub_type) {
                    final_list.push(connector_data);
                } else if let Some(choice) = routing_results.get(&connector_data.payment_method_sub_type) {
                    let routing_choice = choice
                        .first()
                        .ok_or(errors::ApiErrorResponse::InternalServerError)?;
                    if connector_data.connector.connector_name == routing_choice.connector.connector_name
                        && connector_data.connector.merchant_connector_id
                            == routing_choice.connector.merchant_connector_id
                    {
                        final_list.push(connector_data);
                    }
                }
            }
            Ok(final_list)
        }
    }
);

pub fn convert_connector_data_to_routable_connectors(
    connectors: &[ConnectorRoutingData],
) -> CustomResult<Vec<RoutableConnectorChoice>, common_utils::errors::ValidationError> {
    connectors
        .iter()
        .map(|connectors_routing_data| {
            RoutableConnectorChoice::foreign_try_from(
                connectors_routing_data.connector_data.clone(),
            )
        })
        .collect()
}

impl ForeignTryFrom<ConnectorData> for RoutableConnectorChoice {
    type Error = error_stack::Report<common_utils::errors::ValidationError>;
    fn foreign_try_from(from: ConnectorData) -> Result<Self, Self::Error> {
        match RoutableConnectors::foreign_try_from(from.connector_name) {
            Ok(connector) => Ok(Self {
                choice_kind: api_routing::RoutableChoiceKind::FullStruct,
                connector,
                merchant_connector_id: from.merchant_connector_id,
            }),
            Err(e) => Err(common_utils::errors::ValidationError::InvalidValue {
                message: format!("This is not a routable connector: {e:?}"),
            })?,
        }
    }
}

/// Session Surcharge type
pub enum SessionSurchargeDetails {
    /// Surcharge is calculated by hyperswitch
    Calculated(payments_types::SurchargeMetadata),
    /// Surcharge is sent by merchant
    PreDetermined(payments_types::SurchargeDetails),
}

impl SessionSurchargeDetails {
    pub fn fetch_surcharge_details(
        &self,
        payment_method: enums::PaymentMethod,
        payment_method_type: enums::PaymentMethodType,
        card_network: Option<&enums::CardNetwork>,
    ) -> Option<payments_types::SurchargeDetails> {
        match self {
            Self::Calculated(surcharge_metadata) => surcharge_metadata
                .get_surcharge_details(payments_types::SurchargeKey::PaymentMethodData(
                    payment_method,
                    payment_method_type,
                    card_network.cloned(),
                ))
                .cloned(),
            Self::PreDetermined(surcharge_details) => Some(surcharge_details.clone()),
        }
    }
}

pub enum ConnectorChoice {
    SessionMultiple(SessionConnectorDatas),
    StraightThrough(serde_json::Value),
    Decide,
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_convert_connector_parsing_success() {
        let result = enums::Connector::from_str("aci");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), enums::Connector::Aci);

        let result = enums::Connector::from_str("shift4");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), enums::Connector::Shift4);

        let result = enums::Connector::from_str("authorizedotnet");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), enums::Connector::Authorizedotnet);
    }

    #[test]
    fn test_convert_connector_parsing_fail_for_unknown_type() {
        let result = enums::Connector::from_str("unknowntype");
        assert!(result.is_err());

        let result = enums::Connector::from_str("randomstring");
        assert!(result.is_err());
    }

    #[test]
    fn test_convert_connector_parsing_fail_for_camel_case() {
        let result = enums::Connector::from_str("Paypal");
        assert!(result.is_err());

        let result = enums::Connector::from_str("Authorizedotnet");
        assert!(result.is_err());

        let result = enums::Connector::from_str("Opennode");
        assert!(result.is_err());
    }
}

#[derive(Clone)]
pub struct TaxCalculateConnectorData {
    pub connector: ConnectorEnum,
    pub connector_name: enums::TaxConnectors,
}

impl TaxCalculateConnectorData {
    pub fn get_connector_by_name(name: &str) -> CustomResult<Self, errors::ApiErrorResponse> {
        let connector_name = enums::TaxConnectors::from_str(name)
            .change_context(errors::ApiErrorResponse::IncorrectConnectorNameGiven)
            .attach_printable_lazy(|| format!("unable to parse connector: {name}"))?;
        let connector = Self::convert_connector(connector_name)?;
        Ok(Self {
            connector,
            connector_name,
        })
    }

    fn convert_connector(
        connector_name: enums::TaxConnectors,
    ) -> CustomResult<ConnectorEnum, errors::ApiErrorResponse> {
        match connector_name {
            enums::TaxConnectors::Taxjar => {
                Ok(ConnectorEnum::Old(Box::new(connector::Taxjar::new())))
            }
        }
    }
}
