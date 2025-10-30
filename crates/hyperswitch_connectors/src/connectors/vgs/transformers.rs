use common_utils::{
    ext_traits::{Encode, StringExt},
    types::StringMinorUnit,
};
use error_stack::ResultExt;
use hyperswitch_domain_models::{
    router_data::{AccessToken, ConnectorAuthType, RouterData},
    router_flow_types::{ExternalVaultInsertFlow, ExternalVaultRetrieveFlow},
    router_request_types::VaultRequestData,
    router_response_types::{VaultIdType, VaultResponseData},
    types::{RefreshTokenRouterData, VaultRouterData},
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

const VGS_FORMAT: &str = "UUID";
const VGS_CLASSIFIER: &str = "data";

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
                        classifiers: vec![VGS_CLASSIFIER.to_string()],
                        format: VGS_FORMAT.to_string(),
                    }],
                })
            }
            Some(PaymentMethodVaultingData::NetworkToken(network_token_data)) => {
                let mut data = vec![
                    VgsTokenRequestItem {
                        value: Secret::new(network_token_data.network_token.get_card_no()),
                        classifiers: vec!["network_token".to_string()],
                        format: VGS_FORMAT.to_string(),
                    },
                    VgsTokenRequestItem {
                        value: network_token_data.network_token_exp_month,
                        classifiers: vec!["expiry_month".to_string()],
                        format: VGS_FORMAT.to_string(),
                    },
                    VgsTokenRequestItem {
                        value: network_token_data.network_token_exp_year,
                        classifiers: vec!["expiry_year".to_string()],
                        format: VGS_FORMAT.to_string(),
                    },
                ];

                if let Some(tavv) = network_token_data.tavv {
                    data.push(VgsTokenRequestItem {
                        value: tavv,
                        classifiers: vec!["tavv".to_string()],
                        format: VGS_FORMAT.to_string(),
                    });
                }

                Ok(Self { data })
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
    pub(super) vault_id: Secret<String>,
}

impl TryFrom<&ConnectorAuthType> for VgsAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            ConnectorAuthType::SignatureKey {
                api_key,
                key1,
                api_secret,
            } => Ok(Self {
                username: api_key.to_owned(),
                password: key1.to_owned(),
                vault_id: api_secret.to_owned(),
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
    created_at: Option<String>,
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

fn get_token_from_response(
    response: &Vec<VgsTokenResponseItem>,
    alias_classifier: &str,
) -> Result<String, error_stack::Report<errors::ConnectorError>> {
    for token_data in response {
        for response_classifier in &token_data.classifiers {
            if response_classifier.as_str() == alias_classifier {
                for alias in &token_data.aliases {
                    if matches!(alias.format.as_str(), "UUID") {
                        return Ok(alias.alias.clone());
                    }
                }
            }
        }
    }
    router_env::logger::error!("missing alias for the given classifier: `{alias_classifier}`");
    Err(errors::ConnectorError::MissingRequiredField {
        field_name: "alias",
    }
    .into())
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
        match item.data.request.payment_method_vaulting_data.clone() {
            Some(PaymentMethodVaultingData::NetworkToken(_)) => {
                let mut multi_tokens: std::collections::HashMap<String, String> =
                    std::collections::HashMap::new();

                multi_tokens.insert(
                    "network_token".to_string(),
                    get_token_from_response(&item.response.data, "network_token")?,
                );
                multi_tokens.insert(
                    "tavv".to_string(),
                    get_token_from_response(&item.response.data, "tavv")?,
                );
                multi_tokens.insert(
                    "token_expiry_month".to_string(),
                    get_token_from_response(&item.response.data, "token_expiry_month")?,
                );
                multi_tokens.insert(
                    "token_expiry_year".to_string(),
                    get_token_from_response(&item.response.data, "token_expiry_year")?,
                );
                Ok(Self {
                    status: common_enums::AttemptStatus::Started,
                    response: Ok(VaultResponseData::ExternalVaultInsertResponse {
                        connector_vault_id: VaultIdType::MultiVauldIds(multi_tokens),
                        fingerprint_id: get_token_from_response(
                            &item.response.data,
                            "network_token",
                        )?,
                    }),
                    ..item.data
                })
            }
            _ => {
                let vgs_alias = get_token_from_response(&item.response.data, "data")?;

                Ok(Self {
                    status: common_enums::AttemptStatus::Started,
                    response: Ok(VaultResponseData::ExternalVaultInsertResponse {
                        connector_vault_id: VaultIdType::SingleVaultId(vgs_alias.clone()),
                        fingerprint_id: vgs_alias.clone(),
                    }),
                    ..item.data
                })
            }
        }
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
            .first()
            .ok_or(errors::ConnectorError::ResponseHandlingFailed)?;

        let card_detail: api_models::payment_methods::CardDetail = token_response_item
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

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct VgsAuthUpdateRequest {
    grant_type: String,
    client_id: Secret<String>,
    client_secret: Secret<String>,
}
impl TryFrom<&RefreshTokenRouterData> for VgsAuthUpdateRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &RefreshTokenRouterData) -> Result<Self, Self::Error> {
        let auth = VgsAuthType::try_from(&item.connector_auth_type)
            .change_context(errors::ConnectorError::FailedToObtainAuthType)?;

        Ok(Self {
            grant_type: "client_credentials".to_string(),
            client_id: auth.username,
            client_secret: auth.password,
        })
    }
}

#[derive(Default, Debug, Clone, Deserialize, PartialEq, Serialize)]
pub struct VgsAuthUpdateResponse {
    pub access_token: Secret<String>,
    pub token_type: String,
    pub expires_in: i64,
}

impl<F, T> TryFrom<ResponseRouterData<F, VgsAuthUpdateResponse, T, AccessToken>>
    for RouterData<F, T, AccessToken>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, VgsAuthUpdateResponse, T, AccessToken>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(AccessToken {
                token: item.response.access_token,
                expires: item.response.expires_in,
            }),
            ..item.data
        })
    }
}

#[derive(Deserialize, Debug, Serialize)]
pub struct VgsAccessTokenErrorResponse {
    pub error: String,
    pub error_description: String,
}
