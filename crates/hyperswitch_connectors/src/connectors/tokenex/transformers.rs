use common_utils::{
    ext_traits::{Encode, StringExt},
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

pub struct TokenexRouterData<T> {
    pub amount: StringMinorUnit, // The type of amount that a connector accepts, for example, String, i64, f64, etc.
    pub router_data: T,
}

impl<T> From<(StringMinorUnit, T)> for TokenexRouterData<T> {
    fn from((amount, item): (StringMinorUnit, T)) -> Self {
        //Todo :  use utils to convert the amount to the type of amount that a connector accepts
        Self {
            amount,
            router_data: item,
        }
    }
}

#[derive(Default, Debug, Serialize, PartialEq)]
pub struct TokenexInsertRequest {
    data: Secret<String>,
}

impl<F> TryFrom<&VaultRouterData<F>> for TokenexInsertRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &VaultRouterData<F>) -> Result<Self, Self::Error> {
        match item.request.payment_method_vaulting_data.clone() {
            Some(PaymentMethodVaultingData::Card(req_card)) => {
                let stringified_card = req_card
                    .encode_to_string_of_json()
                    .change_context(errors::ConnectorError::RequestEncodingFailed)?;
                Ok(Self {
                    data: Secret::new(stringified_card),
                })
            }
            _ => Err(errors::ConnectorError::NotImplemented(
                "Payment method apart from card".to_string(),
            )
            .into()),
        }
    }
}

//TODO: Fill the struct with respective fields
// Auth Struct
pub struct TokenexAuthType {
    pub(super) api_key: Secret<String>,
    pub(super) tokenex_id: Secret<String>,
}

impl TryFrom<&ConnectorAuthType> for TokenexAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            ConnectorAuthType::BodyKey { api_key, key1 } => Ok(Self {
                api_key: api_key.to_owned(),
                tokenex_id: key1.to_owned(),
            }),
            _ => Err(errors::ConnectorError::FailedToObtainAuthType.into()),
        }
    }
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TokenexInsertResponse {
    token: String,
    first_six: String,
    success: bool,
}
impl
    TryFrom<
        ResponseRouterData<
            ExternalVaultInsertFlow,
            TokenexInsertResponse,
            VaultRequestData,
            VaultResponseData,
        >,
    > for RouterData<ExternalVaultInsertFlow, VaultRequestData, VaultResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<
            ExternalVaultInsertFlow,
            TokenexInsertResponse,
            VaultRequestData,
            VaultResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        let resp = item.response;

        Ok(Self {
            status: common_enums::AttemptStatus::Started,
            response: Ok(VaultResponseData::ExternalVaultInsertResponse {
                connector_vault_id: resp.token.clone(),
                //fingerprint is not provided by tokenex, using token as fingerprint
                fingerprint_id: resp.token.clone(),
            }),
            ..item.data
        })
    }
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TokenexRetrieveRequest {
    token: Secret<String>, //Currently only card number is tokenized. Data can be stringified and can be tokenized
    cache_cvv: bool,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TokenexRetrieveResponse {
    value: Secret<String>,
    success: bool,
}

impl<F> TryFrom<&VaultRouterData<F>> for TokenexRetrieveRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &VaultRouterData<F>) -> Result<Self, Self::Error> {
        let connector_vault_id = item.request.connector_vault_id.as_ref().ok_or(
            errors::ConnectorError::MissingRequiredField {
                field_name: "connector_vault_id",
            },
        )?;
        Ok(Self {
            token: Secret::new(connector_vault_id.clone()),
            cache_cvv: false, //since cvv is not stored at tokenex
        })
    }
}

impl
    TryFrom<
        ResponseRouterData<
            ExternalVaultRetrieveFlow,
            TokenexRetrieveResponse,
            VaultRequestData,
            VaultResponseData,
        >,
    > for RouterData<ExternalVaultRetrieveFlow, VaultRequestData, VaultResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<
            ExternalVaultRetrieveFlow,
            TokenexRetrieveResponse,
            VaultRequestData,
            VaultResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        let resp = item.response;

        let card_detail: api_models::payment_methods::CardDetail = resp
            .value
            .clone()
            .expose()
            .parse_struct("CardDetail")
            .change_context(errors::ConnectorError::ParsingFailed)?;

        Ok(Self {
            status: common_enums::AttemptStatus::Started,
            response: Ok(VaultResponseData::ExternalVaultRetrieveResponse {
                vault_data: PaymentMethodVaultingData::Card(card_detail),
            }),
            ..item.data
        })
    }
}

#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
pub struct TokenexErrorResponse {
    pub status_code: u16,
    pub code: String,
    pub message: String,
    pub reason: Option<String>,
    pub network_advice_code: Option<String>,
    pub network_decline_code: Option<String>,
    pub network_error_message: Option<String>,
}
