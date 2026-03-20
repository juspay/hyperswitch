use common_utils::{
    ext_traits::{Encode, StringExt},
    types::StringMinorUnit,
};
use error_stack::ResultExt;
use hyperswitch_domain_models::{
    router_data::{AccessToken, ConnectorAuthType, RouterData},
    router_flow_types::{ExternalVaultInsertFlow, ExternalVaultRetrieveFlow},
    router_request_types::VaultRequestData,
    router_response_types::{MultiVaultIdType, VaultIdType, VaultResponseData},
    types::{RefreshTokenRouterData, VaultRouterData},
    vault::{PaymentMethodCustomVaultingData, PaymentMethodVaultingData},
};
use hyperswitch_interfaces::errors;
use masking::{ExposeInterface, PeekInterface, Secret};
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

#[derive(Debug, Serialize)]
pub struct VgsTokenRequestItem {
    value: Secret<String>,
    classifiers: Vec<String>,
    format: String,
    storage: VgsStorageType,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum VgsStorageType {
    Persistent,
    Volatile,
}

#[derive(Debug, Serialize)]
pub struct VgsInsertRequest {
    data: Vec<VgsTokenRequestItem>,
}

impl<F> TryFrom<&VaultRouterData<F>> for VgsInsertRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &VaultRouterData<F>) -> Result<Self, Self::Error> {
        match item.request.payment_method_vaulting_data.clone() {
            Some(PaymentMethodCustomVaultingData::CardData(req_card)) => {
                if item.request.should_generate_multiple_tokens == Some(true) {
                    let mut data: Vec<VgsTokenRequestItem> = Vec::new();

                    if let Some(card_number) = req_card.card_number {
                        data.push(VgsTokenRequestItem {
                            value: Secret::new(card_number.get_card_no()),
                            classifiers: vec!["card_number".to_string()],
                            format: VGS_FORMAT.to_string(),
                            storage: VgsStorageType::Volatile,
                        });
                    }

                    if let Some(card_exp_month) = req_card.card_exp_month {
                        data.push(VgsTokenRequestItem {
                            value: card_exp_month,
                            classifiers: vec!["card_expiry_month".to_string()],
                            format: VGS_FORMAT.to_string(),
                            storage: VgsStorageType::Volatile,
                        });
                    }

                    if let Some(card_exp_year) = req_card.card_exp_year {
                        data.push(VgsTokenRequestItem {
                            value: card_exp_year,
                            classifiers: vec!["card_expiry_year".to_string()],
                            format: VGS_FORMAT.to_string(),
                            storage: VgsStorageType::Volatile,
                        });
                    }

                    if let Some(card_cvc) = req_card.card_cvc {
                        data.push(VgsTokenRequestItem {
                            value: card_cvc,
                            classifiers: vec!["card_cvc".to_string()],
                            format: VGS_FORMAT.to_string(),
                            storage: VgsStorageType::Volatile,
                        });
                    }

                    Ok(Self { data })
                } else {
                    let stringified_card = req_card
                        .encode_to_string_of_json()
                        .change_context(errors::ConnectorError::RequestEncodingFailed)?;

                    Ok(Self {
                        data: vec![VgsTokenRequestItem {
                            value: Secret::new(stringified_card),
                            classifiers: vec![VGS_CLASSIFIER.to_string()],
                            format: VGS_FORMAT.to_string(),
                            storage: VgsStorageType::Persistent,
                        }],
                    })
                }
            }
            Some(PaymentMethodCustomVaultingData::NetworkTokenData(network_token_data)) => {
                let mut data: Vec<VgsTokenRequestItem> = Vec::new();

                if let Some(network_token) = network_token_data.network_token {
                    data.push(VgsTokenRequestItem {
                        value: Secret::new(network_token.get_card_no()),
                        classifiers: vec!["payment_token".to_string()],
                        format: VGS_FORMAT.to_string(),
                        storage: VgsStorageType::Volatile,
                    });
                }

                if let Some(network_token_exp_month) = network_token_data.network_token_exp_month {
                    data.push(VgsTokenRequestItem {
                        value: network_token_exp_month,
                        classifiers: vec!["token_expiry_month".to_string()],
                        format: VGS_FORMAT.to_string(),
                        storage: VgsStorageType::Volatile,
                    });
                }

                if let Some(network_token_exp_year) = network_token_data.network_token_exp_year {
                    data.push(VgsTokenRequestItem {
                        value: network_token_exp_year,
                        classifiers: vec!["token_expiry_year".to_string()],
                        format: VGS_FORMAT.to_string(),
                        storage: VgsStorageType::Volatile,
                    });
                }

                if let Some(cryptogram) = network_token_data.cryptogram {
                    data.push(VgsTokenRequestItem {
                        value: cryptogram,
                        classifiers: vec!["token_cryptogram".to_string()],
                        format: VGS_FORMAT.to_string(),
                        storage: VgsStorageType::Volatile,
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
    value: &str,
) -> Option<Secret<String>> {
    for token_data in response {
        if token_data.value.peek() == value {
            for alias in &token_data.aliases {
                if matches!(alias.format.as_str(), VGS_FORMAT) {
                    return Some(Secret::new(alias.alias.clone()));
                }
            }
        }
    }

    None
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
            Some(PaymentMethodCustomVaultingData::NetworkTokenData(network_token_data)) => {
                let multi_tokens = MultiVaultIdType::NetworkToken {
                    tokenized_network_token: network_token_data.network_token.clone().and_then(
                        |network_token| {
                            get_token_from_response(&item.response.data, network_token.peek())
                        },
                    ),
                    tokenized_network_token_exp_year: network_token_data
                        .network_token_exp_year
                        .clone()
                        .and_then(|network_token_exp_year| {
                            get_token_from_response(
                                &item.response.data,
                                network_token_exp_year.peek(),
                            )
                        }),
                    tokenized_network_token_exp_month: network_token_data
                        .network_token_exp_month
                        .clone()
                        .and_then(|network_token_exp_month| {
                            get_token_from_response(
                                &item.response.data,
                                network_token_exp_month.peek(),
                            )
                        }),
                    tokenized_cryptogram: network_token_data.cryptogram.clone().and_then(
                        |cryptogram| {
                            get_token_from_response(&item.response.data, cryptogram.peek())
                        },
                    ),
                };

                Ok(Self {
                    status: common_enums::AttemptStatus::Started,
                    response: Ok(VaultResponseData::ExternalVaultInsertResponse {
                        connector_vault_id: VaultIdType::MultiVauldIds(multi_tokens),
                        fingerprint_id: network_token_data
                            .network_token
                            .clone()
                            .and_then(|network_token| {
                                get_token_from_response(&item.response.data, network_token.peek())
                            })
                            .unwrap_or(Secret::new("default".to_string()))
                            .expose(),
                    }),
                    ..item.data
                })
            }
            Some(PaymentMethodCustomVaultingData::CardData(card_data)) => {
                if item.data.request.should_generate_multiple_tokens == Some(true) {
                    let multi_tokens = MultiVaultIdType::Card {
                        tokenized_card_number: card_data.card_number.clone().and_then(
                            |card_number| {
                                get_token_from_response(&item.response.data, card_number.peek())
                            },
                        ),
                        tokenized_card_expiry_month: card_data.card_exp_month.clone().and_then(
                            |card_exp_month| {
                                get_token_from_response(&item.response.data, card_exp_month.peek())
                            },
                        ),
                        tokenized_card_expiry_year: card_data.card_exp_year.clone().and_then(
                            |card_exp_year| {
                                get_token_from_response(&item.response.data, card_exp_year.peek())
                            },
                        ),
                        tokenized_card_cvc: card_data.card_cvc.clone().and_then(|card_cvc| {
                            get_token_from_response(&item.response.data, card_cvc.peek())
                        }),
                    };

                    Ok(Self {
                        status: common_enums::AttemptStatus::Started,
                        response: Ok(VaultResponseData::ExternalVaultInsertResponse {
                            connector_vault_id: VaultIdType::MultiVauldIds(multi_tokens),
                            fingerprint_id: card_data
                                .card_number
                                .clone()
                                .and_then(|card_number| {
                                    get_token_from_response(&item.response.data, card_number.peek())
                                })
                                .unwrap_or(Secret::new("default".to_string()))
                                .expose(),
                        }),
                        ..item.data
                    })
                } else {
                    let vgs_alias = item
                        .response
                        .data
                        .first()
                        .and_then(|val| val.aliases.first())
                        .ok_or(errors::ConnectorError::ResponseHandlingFailed)?;

                    Ok(Self {
                        status: common_enums::AttemptStatus::Started,
                        response: Ok(VaultResponseData::ExternalVaultInsertResponse {
                            connector_vault_id: VaultIdType::SingleVaultId(vgs_alias.alias.clone()),
                            fingerprint_id: vgs_alias.alias.clone(),
                        }),
                        ..item.data
                    })
                }
            }
            _ => {
                let vgs_alias = item
                    .response
                    .data
                    .first()
                    .and_then(|val| val.aliases.first())
                    .ok_or(errors::ConnectorError::ResponseHandlingFailed)?;

                Ok(Self {
                    status: common_enums::AttemptStatus::Started,
                    response: Ok(VaultResponseData::ExternalVaultInsertResponse {
                        connector_vault_id: VaultIdType::SingleVaultId(vgs_alias.alias.clone()),
                        fingerprint_id: vgs_alias.alias.clone(),
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
