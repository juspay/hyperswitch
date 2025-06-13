use hyperswitch_domain_models::consts;
use common_enums::connector_enums;
use rustc_hash::FxHashMap;
use crate::connector_integration_interface::ConnectorEnum;
use hyperswitch_domain_models::errors::api_error_response as errors;

/// Normal flow will call the connector and follow the flow specific operations (capture, authorize)
/// SessionTokenFromMetadata will avoid calling the connector instead create the session token ( for sdk )
#[derive(Clone, Eq, PartialEq, Debug)]
pub enum GetToken {
    /// Google Pay Metadata
    GpayMetadata,
    /// Samsung Pay Metadata
    SamsungPayMetadata,
    /// Apple Pay Metadata
    ApplePayMetadata,
    /// Paypal SDK Metadata
    PaypalSdkMetadata,
    /// Paze Metadata
    PazeMetadata,
    /// Connector Metadata
    Connector,
}

impl From<api_models::enums::PaymentMethodType> for GetToken {
    fn from(value: api_models::enums::PaymentMethodType) -> Self {
        match value {
            api_models::enums::PaymentMethodType::GooglePay => Self::GpayMetadata,
            api_models::enums::PaymentMethodType::ApplePay => Self::ApplePayMetadata,
            api_models::enums::PaymentMethodType::SamsungPay => Self::SamsungPayMetadata,
            api_models::enums::PaymentMethodType::Paypal => Self::PaypalSdkMetadata,
            api_models::enums::PaymentMethodType::Paze => Self::PazeMetadata,
            _ => Self::Connector,
        }
    }
}

/// This struct is used to hold the connector data and payment method type for a session
#[derive(Debug)]
pub struct SessionRoutingChoice {
    /// This struct is used to hold the connector data for a session
    pub connector: ConnectorData,
    /// This struct is used to hold the payment method type for a session
    pub payment_method_type: api_models::enums::PaymentMethodType,
}

/// Routing algorithm will output merchant connector identifier instead of connector name
/// In order to support backwards compatibility for older routing algorithms and merchant accounts
/// the support for connector name is retained
#[derive(Clone, Debug)]
pub struct ConnectorData {
    /// connector enum
    pub connector: ConnectorEnum,
    /// connector name
    pub connector_name: connector_enums::Connector,
    /// get_token enum
    pub get_token: GetToken,
    /// merchant connector id
    pub merchant_connector_id: Option<common_utils::id_type::MerchantConnectorAccountId>,
}

/// This struct is used to hold the connector data for a session
#[derive(Clone, Debug)]
pub struct SessionConnectorData {
    /// payment method sub type
    pub payment_method_sub_type: api_models::enums::PaymentMethodType,
    /// payment method type
    pub payment_method_type: api_models::enums::PaymentMethod,
    /// connector data
    pub connector: ConnectorData,
    /// business sub label
    pub business_sub_label: Option<String>,
}

/// This struct is used to hold the connector data for a session
impl SessionConnectorData {
    /// This function is used to create a new session connector data
    pub fn new(
        payment_method_sub_type: api_models::enums::PaymentMethodType,
        connector: ConnectorData,
        business_sub_label: Option<String>,
        payment_method_type: api_models::enums::PaymentMethod,
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
        /// This function is used to filter the connector data for session routing
        /// based on the payment method types that are enabled for routing.
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
        /// This function is used to filter the connector data for session routing
        /// based on the payment method types that are enabled for routing.
        pub fn filter_and_validate_for_session_flow(self, routing_results: &FxHashMap<api_models::enums::PaymentMethodType, Vec<SessionRoutingChoice>>) -> Result<Self, errors::ApiErrorResponse> {
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
