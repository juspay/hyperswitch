pub mod fiservcommercehub;

use bytes::Bytes;
use common_utils::request::Request;
use hyperswitch_domain_models::connector_endpoints::Connectors;
use hyperswitch_interfaces::{
    errors::ConnectorError,
    relay::{ConnectorRelayIntegration, UnreferencedRefundResponse, UnreferencedRefundRouterData},
};

macro_rules! define_relay_connectors {
    (
        $(
            $name:literal => $variant:ident($connector:path)
        ),* $(,)?
    ) => {
        pub enum RelayConnectors {
            $(
                $variant($connector),
            )*
        }

        impl RelayConnectors {
            pub fn from_connector_name(
                name: &str,
            ) -> error_stack::Result<Self, ConnectorError> {
                match name {
                    $(
                        $name => Ok(Self::$variant($connector)),
                    )*
                    _ => Err(ConnectorError::FlowNotSupported {
                        flow: "UnreferencedRefund".to_string(),
                        connector: name.to_string(),
                    }
                    .into()),
                }
            }
        }

        impl ConnectorRelayIntegration for RelayConnectors {
            fn base_url<'a>(
                &self,
                connectors: &'a Connectors,
            ) -> &'a str {
                match self {
                    $(
                        Self::$variant(c) => c.base_url(connectors),
                    )*
                }
            }

            fn supports_access_token(&self) -> bool {
                match self {
                    $(
                        Self::$variant(c) => c.supports_access_token(),
                    )*
                }
            }

            fn build_relay_request(
                &self,
                router_data: &UnreferencedRefundRouterData<'_>,
            ) -> error_stack::Result<Request, ConnectorError> {
                match self {
                    $(
                        Self::$variant(c) => {
                            c.build_relay_request(router_data)
                        }
                    )*
                }
            }

            fn handle_relay_success_response(
                &self,
                response: Bytes,
            ) -> error_stack::Result<
                UnreferencedRefundResponse,
                ConnectorError,
            > {
                match self {
                    $(
                        Self::$variant(c) => {
                            c.handle_relay_success_response(response)
                        }
                    )*
                }
            }

            fn get_relay_error_response(
                &self,
                response: Bytes,
                status_code: u16,
            ) -> error_stack::Result<
                UnreferencedRefundResponse,
                ConnectorError,
            > {
                match self {
                    $(
                        Self::$variant(c) => {
                            c.get_relay_error_response(
                                response,
                                status_code,
                            )
                        }
                    )*
                }
            }
        }
    };
}

define_relay_connectors! {
    "fiservcommercehub" =>
        Fiservcommercehub(
            fiservcommercehub::Fiservcommercehub
        ),
}
