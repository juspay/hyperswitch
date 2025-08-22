use common_utils::{
    ext_traits::{Encode, OptionExt, StringExt},
    types::StringMinorUnit,
};
use error_stack::ResultExt;
use hyperswitch_domain_models::{
    router_data::{ConnectorAuthType, RouterData},
    router_flow_types::{ExternalVaultInsertFlow, ExternalVaultRetrieveFlow},
    router_request_types::VaultRequestData,
    router_response_types::VaultResponseData,
    types::VaultRouterData,
    vault::PaymentMethodVaultingData,
};
use hyperswitch_interfaces::errors;
use masking::{ExposeInterface, Secret};
use serde::{Deserialize, Serialize};

use crate::types::ResponseRouterData;

pub struct VgsRouterData<T> {
    pub amount: StringMinorUnit, // The type of amount that a connector accepts, for example, String, i64, f64, etc.
    pub router_data: T,
}

impl<T> From<(StringMinorUnit, T)> for VgsRouterData<T> {
    fn from((amount, item): (StringMinorUnit, T)) -> Self {
        Self {
            amount,
            router_data: item,
        }
    }
}

#[derive(Default, Debug, Serialize, PartialEq)]
pub struct VgsTokenRequestItem {
    value: Secret<String>,
    classifiers: Vec<String>,
    format: String,
}

#[derive(Default, Debug, Serialize, PartialEq)]
pub struct VgsInsertRequest {
    data: Vec<VgsTokenRequestItem>,
}

impl<F> TryFrom<&VaultRouterData<F>> for VgsInsertRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &VaultRouterData<F>) -> Result<Self, Self::Error> {
        match item.request.payment_method_vaulting_data.clone() {
            Some(PaymentMethodVaultingData::Card(req_card)) => {
                let stringified_card = req_card
                    .encode_to_string_of_json()
                    .change_context(errors::ConnectorError::RequestEncodingFailed)?;

                Ok(Self {
                    data: vec![VgsTokenRequestItem {
                        value: Secret::new(stringified_card),
                        classifiers: vec!["data".to_string()],
                        format: "UUID".to_string(),
                    }],
                })
            }
            _ => Err(errors::ConnectorError::NotImplemented(
                "Payment method apart from card".to_string(),
            )
            .into()),
        }
    }
}

pub struct VgsAuthType {
    pub(super) username: Secret<String>,
    pub(super) password: Secret<String>,
}

impl TryFrom<&ConnectorAuthType> for VgsAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            ConnectorAuthType::BodyKey { api_key, key1 } => Ok(Self {
                username: api_key.to_owned(),
                password: key1.to_owned(),
            }),
            _ => Err(errors::ConnectorError::FailedToObtainAuthType.into()),
        }
    }
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VgsAliasItem {
    alias: String,
    format: String,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VgsTokenResponseItem {
    value: Secret<String>,
    classifiers: Vec<String>,
    aliases: Vec<VgsAliasItem>,
    created_at: String,
    storage: String,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VgsInsertResponse {
    data: Vec<VgsTokenResponseItem>,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VgsRetrieveResponse {
    data: Vec<VgsTokenResponseItem>,
}

impl
    TryFrom<
        ResponseRouterData<
            ExternalVaultInsertFlow,
            VgsInsertResponse,
            VaultRequestData,
            VaultResponseData,
        >,
    > for RouterData<ExternalVaultInsertFlow, VaultRequestData, VaultResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<
            ExternalVaultInsertFlow,
            VgsInsertResponse,
            VaultRequestData,
            VaultResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        let vgs_alias = item
            .response
            .data
            .get(0)
            .and_then(|val| val.aliases.get(0))
            .get_required_value("VgsAliasItem")
            .change_context(errors::ConnectorError::MissingRequiredField {
                field_name: "VgsAliasItem",
            })?;

        Ok(Self {
            status: common_enums::AttemptStatus::Failure,
            response: Ok(VaultResponseData::ExternalVaultInsertResponse {
                connector_vault_id: vgs_alias.alias.clone(),
                fingerprint_id: vgs_alias.alias.clone(),
            }),
            ..item.data
        })
    }
}

impl
    TryFrom<
        ResponseRouterData<
            ExternalVaultRetrieveFlow,
            VgsRetrieveResponse,
            VaultRequestData,
            VaultResponseData,
        >,
    > for RouterData<ExternalVaultRetrieveFlow, VaultRequestData, VaultResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<
            ExternalVaultRetrieveFlow,
            VgsRetrieveResponse,
            VaultRequestData,
            VaultResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        let token_response_item = item
            .response
            .data
            .get(0)
            .get_required_value("VgsTokenResponseItem")
            .change_context(errors::ConnectorError::MissingRequiredField {
                field_name: "VgsTokenResponseItem",
            })?;

        let card_detail: api_models::payment_methods::CardDetail = token_response_item
            .value
            .clone()
            .expose()
            .parse_struct("CardDetail")
            .change_context(errors::ConnectorError::ParsingFailed)?;

        Ok(Self {
            status: common_enums::AttemptStatus::Failure,
            response: Ok(VaultResponseData::ExternalVaultRetrieveResponse {
                vault_data: PaymentMethodVaultingData::Card(card_detail),
            }),
            ..item.data
        })
    }
}

#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
pub struct VgsErrorItem {
    pub status: u16,
    pub code: String,
    pub detail: Option<String>,
}

#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
pub struct VgsErrorResponse {
    pub errors: Vec<VgsErrorItem>,
    pub trace_id: String,
}
