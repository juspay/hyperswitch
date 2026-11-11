use common_types::payment_methods::PayViaSingleUseToken;
use error_stack::report;
use hyperswitch_domain_models::{
    payment_method_data::PaymentMethodData, router_data::PaymentMethodToken,
};
use hyperswitch_interfaces::{
    api::{ConnectorSpecifications, SingleUseTokenSupport},
    connector_integration_interface::ConnectorEnum,
};

use crate::core::errors::{ApiErrorResponse, PmResult};

/// Flows in which the payment can be made via a single-use token
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SingleUseTokenFlow {
    Authorize,
    SetupMandate,
}

/// Decision on whether the payment should be made via a single-use token created at the connector before authorization
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SingleUseTokenDecision {
    Default,
    PayViaSingleUseToken,
    PayDirectly,
}

impl SingleUseTokenDecision {
    pub fn is_connector_tokenization_enabled(
        self,
        is_connector_tokenization_enabled_by_default: bool,
    ) -> bool {
        match self {
            Self::Default => is_connector_tokenization_enabled_by_default,
            Self::PayViaSingleUseToken => true,
            Self::PayDirectly => false,
        }
    }
}

/// Decides whether the payment should be made via a single-use token from the `pay_via_single_use_token` config,
/// erroring if the connector support conflicts with the config
pub fn decide_single_use_token(
    connector: &ConnectorEnum,
    connector_name: &str,
    single_use_token_flow: Option<SingleUseTokenFlow>,
    pay_via_single_use_token: Option<PayViaSingleUseToken>,
    payment_method_data: Option<&PaymentMethodData>,
    payment_method_token: Option<&PaymentMethodToken>,
) -> PmResult<SingleUseTokenDecision> {
    let pay_via_single_use_token = pay_via_single_use_token.unwrap_or_default();

    // The connector single-use token support is checked only when `Enabled` / `Disabled` is configured, for the
    // flows that can be made via a single-use token, and when the payment method data is available (not for
    // saved payment methods)
    let single_use_token_support = match pay_via_single_use_token {
        PayViaSingleUseToken::Default => None,
        PayViaSingleUseToken::Enabled | PayViaSingleUseToken::Disabled => {
            single_use_token_flow.zip(payment_method_data).and_then(
                |(single_use_token_flow, payment_method_data)| match single_use_token_flow {
                    SingleUseTokenFlow::Authorize => connector
                        .get_single_use_token_support(payment_method_data, payment_method_token),
                    SingleUseTokenFlow::SetupMandate => connector
                        .get_single_use_token_support_for_setup_mandate(
                            payment_method_data,
                            payment_method_token,
                        ),
                },
            )
        }
    };

    match (pay_via_single_use_token, single_use_token_support) {
        // Not configured, or not implemented for the connector, follow the default behaviour
        (PayViaSingleUseToken::Default, _) | (_, None) => Ok(SingleUseTokenDecision::Default),
        (PayViaSingleUseToken::Enabled, Some(single_use_token_support)) => {
            if single_use_token_support == SingleUseTokenSupport::NotSupported {
                Err(report!(ApiErrorResponse::InvalidConnectorConfiguration {
                    config: format!(
                        "pay_via_single_use_token: tokenization is not supported for the payment method type with connector {connector_name}"
                    ),
                }))
            } else {
                Ok(SingleUseTokenDecision::PayViaSingleUseToken)
            }
        }
        (PayViaSingleUseToken::Disabled, Some(single_use_token_support)) => {
            if single_use_token_support == SingleUseTokenSupport::Required {
                Err(report!(ApiErrorResponse::InvalidConnectorConfiguration {
                    config: format!(
                        "pay_via_single_use_token: payment without tokenization is not supported for the payment method type with connector {connector_name}"
                    ),
                }))
            } else {
                Ok(SingleUseTokenDecision::PayDirectly)
            }
        }
    }
}
