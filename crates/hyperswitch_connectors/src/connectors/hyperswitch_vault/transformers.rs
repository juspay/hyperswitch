use common_utils::{ext_traits::OptionExt, pii::Email};
use error_stack::ResultExt;
use hyperswitch_domain_models::{
    router_data::{ConnectorAuthType, RouterData},
    router_flow_types::vault::ExternalVaultCreateFlow,
    router_response_types::{
        ConnectorCustomerResponseData, PaymentsResponseData, VaultResponseData,
    },
    types::{ConnectorCustomerRouterData, VaultRouterData},
};
use hyperswitch_interfaces::errors;
use hyperswitch_masking::Secret;
use serde::{Deserialize, Serialize};

use crate::types::ResponseRouterData;

#[derive(Default, Debug, Serialize)]
pub struct HyperswitchVaultCreateRequest {
    // Optional so the guest flow (no customer) can send `customer_id: null`.
    customer_id: Option<String>,
    storage_type: common_enums::StorageType,
}

impl TryFrom<&VaultRouterData<ExternalVaultCreateFlow>> for HyperswitchVaultCreateRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &VaultRouterData<ExternalVaultCreateFlow>) -> Result<Self, Self::Error> {
        Ok(Self {
            customer_id: item.request.connector_customer_id.clone(),
            storage_type: item.request.storage_type.unwrap_or_default(),
        })
    }
}

pub struct HyperswitchVaultAuthType {
    pub(super) api_key: Secret<String>,
    pub(super) profile_id: Secret<String>,
}

impl TryFrom<&ConnectorAuthType> for HyperswitchVaultAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            ConnectorAuthType::SignatureKey {
                api_key,
                api_secret,
                ..
            } => Ok(Self {
                api_key: api_key.to_owned(),
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

#[derive(Default, Debug, Serialize)]
pub struct HyperswitchVaultCustomerCreateRequest {
    name: Secret<String>,
    email: Option<Email>,
}

impl TryFrom<&ConnectorCustomerRouterData> for HyperswitchVaultCustomerCreateRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &ConnectorCustomerRouterData) -> Result<Self, Self::Error> {
        Ok(Self {
            name: item
                .request
                .name
                .clone()
                .get_required_value("customer name")
                .change_context(errors::ConnectorError::MissingRequiredField {
                    field_name: "customer name",
                })?,
            email: item.request.email.clone(),
        })
    }
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct HyperswitchVaultCustomerCreateResponse {
    id: String,
}

impl<F, T>
    TryFrom<ResponseRouterData<F, HyperswitchVaultCustomerCreateResponse, T, PaymentsResponseData>>
    for RouterData<F, T, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<
            F,
            HyperswitchVaultCustomerCreateResponse,
            T,
            PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(PaymentsResponseData::ConnectorCustomerResponse(
                ConnectorCustomerResponseData::new_with_customer_id(item.response.id),
            )),
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
