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

use crate::{types::ResponseRouterData, utils::RouterData as _};

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

                if let Some(payment_account_reference) =
                    network_token_data.payment_account_reference
                {
                    data.push(VgsTokenRequestItem {
                        value: Secret::new(payment_account_reference),
                        classifiers: vec!["payment_account_reference".to_string()],
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
    pub(super) _vault_id: Secret<String>,
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
                _vault_id: api_secret.to_owned(),
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
    classifier: &str,
) -> Result<String, error_stack::Report<errors::ConnectorError>> {
    for token_data in response {
        for response_classifier in &token_data.classifiers {
            if matches!(response_classifier.as_str(), classifier) {
                for alias in &token_data.aliases {
                    if matches!(alias.format.as_str(), "UUID") {
                        return Ok(alias.alias.clone());
                    }
                }
            }
        }
    }
    router_env::logger::error!("missing alias for the given classifier: `{classifier}`");
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
            Some(PaymentMethodVaultingData::NetworkToken(network_token_data)) => Ok(Self {
                status: common_enums::AttemptStatus::Started,
                response: Ok(VaultResponseData::ExternalVaultMultiTokenResponse {
                    payment_token: Secret::new(get_token_from_response(
                        &item.response.data,
                        "network_token",
                    )?),
                    payment_account_reference: Secret::new(get_token_from_response(
                        &item.response.data,
                        "payment_account_reference",
                    )?),
                    token_expiration_month: Secret::new(get_token_from_response(
                        &item.response.data,
                        "expiry_month",
                    )?),
                    token_expiration_year: Secret::new(get_token_from_response(
                        &item.response.data,
                        "expiry_year",
                    )?),
                }),
                ..item.data
            }),
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
                        connector_vault_id: vgs_alias.alias.clone(),
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
