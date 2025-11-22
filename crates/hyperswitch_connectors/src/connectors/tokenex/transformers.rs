use common_utils::types::StringMinorUnit;
use error_stack::ResultExt;
use hyperswitch_domain_models::{
    router_data::{ConnectorAuthType, ErrorResponse, RouterData},
    router_flow_types::{ExternalVaultInsertFlow, ExternalVaultRetrieveFlow},
    router_request_types::VaultRequestData,
    router_response_types::{VaultIdType, VaultResponseData},
    types::VaultRouterData,
    vault::{PaymentMethodCustomVaultingData, PaymentMethodVaultingData},
};
use hyperswitch_interfaces::errors;
use masking::Secret;
use serde::{Deserialize, Serialize};

use crate::types::ResponseRouterData;

pub struct TokenexRouterData<T> {
    pub amount: StringMinorUnit, // The type of amount that a connector accepts, for example, String, i64, f64, etc.
    pub router_data: T,
}

impl<T> From<(StringMinorUnit, T)> for TokenexRouterData<T> {
    fn from((amount, item): (StringMinorUnit, T)) -> Self {
        Self {
            amount,
            router_data: item,
        }
    }
}

#[derive(Default, Debug, Serialize, PartialEq)]
pub struct TokenexInsertRequest {
    data: cards::CardNumber, //Currently only card number is tokenized. Data can be stringified and can be tokenized
}

impl<F> TryFrom<&VaultRouterData<F>> for TokenexInsertRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &VaultRouterData<F>) -> Result<Self, Self::Error> {
        match item.request.payment_method_vaulting_data.clone() {
            Some(PaymentMethodCustomVaultingData::CardData(req_card)) => Ok(Self {
                data: req_card.card_number.clone().ok_or(
                    errors::ConnectorError::MissingRequiredField {
                        field_name: "card_number",
                    },
                )?,
            }),
            _ => Err(errors::ConnectorError::NotImplemented(
                "Payment method apart from card".to_string(),
            )
            .into()),
        }
    }
}
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
#[serde(rename_all = "camelCase")]
pub struct TokenexInsertResponse {
    token: Option<String>,
    first_six: Option<String>,
    last_four: Option<String>,
    success: bool,
    error: String,
    message: Option<String>,
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
        match resp.success && resp.error.is_empty() {
            true => {
                let token = resp
                    .token
                    .clone()
                    .ok_or(errors::ConnectorError::ResponseDeserializationFailed)
                    .attach_printable("Token is missing in tokenex response")?;
                Ok(Self {
                    status: common_enums::AttemptStatus::Started,
                    response: Ok(VaultResponseData::ExternalVaultInsertResponse {
                        connector_vault_id: VaultIdType::SingleVaultId(token.clone()),
                        //fingerprint is not provided by tokenex, using token as fingerprint
                        fingerprint_id: token.clone(),
                    }),
                    ..item.data
                })
            }
            false => {
                let (code, message) = resp.error.split_once(':').unwrap_or(("", ""));

                let response = Err(ErrorResponse {
                    code: code.to_string(),
                    message: message.to_string(),
                    reason: resp.message,
                    status_code: item.http_code,
                    attempt_status: None,
                    connector_transaction_id: None,
                    network_decline_code: None,
                    network_advice_code: None,
                    network_error_message: None,
                    connector_metadata: None,
                });

                Ok(Self {
                    response,
                    ..item.data
                })
            }
        }
    }
}
#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TokenexRetrieveRequest {
    token: Secret<String>, //Currently only card number is tokenized. Data can be stringified and can be tokenized
    cache_cvv: bool,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TokenexRetrieveResponse {
    value: Option<cards::CardNumber>,
    success: bool,
    error: String,
    message: Option<String>,
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

        match resp.success && resp.error.is_empty() {
            true => {
                let data = resp
                    .value
                    .clone()
                    .ok_or(errors::ConnectorError::ResponseDeserializationFailed)
                    .attach_printable("Card number is missing in tokenex response")?;
                Ok(Self {
                    status: common_enums::AttemptStatus::Started,
                    response: Ok(VaultResponseData::ExternalVaultRetrieveResponse {
                        vault_data: PaymentMethodVaultingData::CardNumber(data),
                    }),
                    ..item.data
                })
            }
            false => {
                let (code, message) = resp.error.split_once(':').unwrap_or(("", ""));

                let response = Err(ErrorResponse {
                    code: code.to_string(),
                    message: message.to_string(),
                    reason: resp.message,
                    status_code: item.http_code,
                    attempt_status: None,
                    connector_transaction_id: None,
                    network_decline_code: None,
                    network_advice_code: None,
                    network_error_message: None,
                    connector_metadata: None,
                });

                Ok(Self {
                    response,
                    ..item.data
                })
            }
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct TokenexErrorResponse {
    pub error: String,
    pub message: String,
}
