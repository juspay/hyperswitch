use common_enums::enums;
use hyperswitch_domain_models::{
    payment_method_data::PaymentMethodData,
    router_data::{ConnectorAuthType, RouterData},
    router_flow_types::vault::ExternalVaultCreateFlow,
    router_request_types::ResponseId,
    router_response_types::VaultResponseData,
    types::VaultRouterData,
};
use hyperswitch_interfaces::errors;
use masking::Secret;
use serde::{Deserialize, Serialize};

use crate::{
    types::{RefundsResponseRouterData, ResponseRouterData},
    utils,
};

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Serialize)]
pub struct HyperswitchVaultCreateRequest {
    customer_id: String,
}

impl TryFrom<&VaultRouterData<ExternalVaultCreateFlow>> for HyperswitchVaultCreateRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &VaultRouterData<ExternalVaultCreateFlow>) -> Result<Self, Self::Error> {
        let customer_id = item
            .connector_customer
            .clone()
            .ok_or_else(utils::missing_field_err("connector_customer"))?;
        Ok(Self { customer_id })
    }
}

pub struct HyperswitchVaultAuthType {
    pub(super) api_key: Secret<String>,
    pub(super) publishable_key: Secret<String>,
    pub(super) profile_id: Secret<String>,
}

impl TryFrom<&ConnectorAuthType> for HyperswitchVaultAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            ConnectorAuthType::SignatureKey {
                api_key,
                key1,
                api_secret,
            } => Ok(Self {
                api_key: api_key.to_owned(),
                publishable_key: key1.to_owned(),
                profile_id: api_secret.to_owned(),
            }),
            _ => Err(errors::ConnectorError::FailedToObtainAuthType.into()),
        }
    }
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct HyperswitchVaultCreateResponse {
    id: Secret<String>,
    client_secret: Secret<String>,
}

impl<F, T> TryFrom<ResponseRouterData<F, HyperswitchVaultCreateResponse, T, VaultResponseData>>
    for RouterData<F, T, VaultResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, HyperswitchVaultCreateResponse, T, VaultResponseData>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(VaultResponseData::ExternalVaultCreateResponse {
                session_id: item.response.id,
                client_secret: item.response.client_secret,
            }),
            ..item.data
        })
    }
}

#[derive(Default, Debug, Serialize, Deserialize)]
pub struct HyperswitchVaultErrorResponse {
    pub error: HyperswitchVaultErrorDetails,
}

#[derive(Default, Debug, Serialize, Deserialize)]
pub struct HyperswitchVaultErrorDetails {
    #[serde(alias = "type")]
    pub error_type: String,
    pub message: Option<String>,
    pub code: String,
}
