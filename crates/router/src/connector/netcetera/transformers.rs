use error_stack::ResultExt;
use masking::Secret;
use serde::{Deserialize, Serialize};

use crate::{
    connector::utils,
    consts::NO_ERROR_MESSAGE,
    core::errors,
    types::{self, api},
};

//TODO: Fill the struct with respective fields
pub struct NetceteraRouterData<T> {
    pub amount: i64, // The type of amount that a connector accepts, for example, String, i64, f64, etc.
    pub router_data: T,
}

impl<T>
    TryFrom<(
        &types::api::CurrencyUnit,
        types::storage::enums::Currency,
        i64,
        T,
    )> for NetceteraRouterData<T>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        (_currency_unit, _currency, amount, item): (
            &types::api::CurrencyUnit,
            types::storage::enums::Currency,
            i64,
            T,
        ),
    ) -> Result<Self, Self::Error> {
        //Todo :  use utils to convert the amount to the type of amount that a connector accepts
        Ok(Self {
            amount,
            router_data: item,
        })
    }
}

impl<T> TryFrom<(i64, T)> for NetceteraRouterData<T> {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from((amount, router_data): (i64, T)) -> Result<Self, Self::Error> {
        Ok(Self {
            amount,
            router_data,
        })
    }
}

impl
    TryFrom<
        types::ResponseRouterData<
            api::PreAuthentication,
            NetceteraPreAuthenticationResponse,
            types::authentication::PreAuthNRequestData,
            types::authentication::AuthenticationResponseData,
        >,
    > for types::authentication::PreAuthNRouterData
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<
            api::PreAuthentication,
            NetceteraPreAuthenticationResponse,
            types::authentication::PreAuthNRequestData,
            types::authentication::AuthenticationResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        let response = match item.response {
            NetceteraPreAuthenticationResponse::Success(pre_authn_response) => {
                // if card is not enrolled for 3ds, card_range will be None
                let card_range = pre_authn_response.get_card_range_if_avalable();
                let maximum_supported_3ds_version = card_range
                    .as_ref()
                    .map(|card_range| card_range.highest_common_supported_version.clone())
                    .unwrap_or_else(|| {
                        // Version "0.0.0" will be less that "2.0.0", hence we will treat this card as not eligible for 3ds authentication
                        common_utils::types::SemanticVersion::new(0, 0, 0)
                    });
                let three_ds_method_data = card_range.as_ref().and_then(|card_range| {
                    card_range
                        .three_ds_method_data_form
                        .as_ref()
                        .map(|data| data.three_ds_method_data.clone())
                });
                let three_ds_method_url = card_range
                    .as_ref()
                    .and_then(|card_range| card_range.get_three_ds_method_url());
                Ok(
                    types::authentication::AuthenticationResponseData::PreAuthNResponse {
                        threeds_server_transaction_id: pre_authn_response
                            .three_ds_server_trans_id
                            .clone(),
                        maximum_supported_3ds_version: maximum_supported_3ds_version.clone(),
                        connector_authentication_id: pre_authn_response.three_ds_server_trans_id,
                        three_ds_method_data,
                        three_ds_method_url,
                        message_version: maximum_supported_3ds_version,
                        connector_metadata: None,
                    },
                )
            }
            NetceteraPreAuthenticationResponse::Failure(error_response) => {
                Err(types::ErrorResponse {
                    code: error_response.error_details.error_code,
                    message: error_response
                        .error_details
                        .error_description
                        .clone()
                        .unwrap_or(NO_ERROR_MESSAGE.to_owned()),
                    reason: error_response.error_details.error_description,
                    status_code: item.http_code,
                    attempt_status: None,
                    connector_transaction_id: None,
                })
            }
        };
        Ok(Self {
            response,
            ..item.data.clone()
        })
    }
}

pub struct NetceteraAuthType {
    pub(super) certificate: Secret<String>,
    pub(super) private_key: Secret<String>,
}

impl TryFrom<&types::ConnectorAuthType> for NetceteraAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &types::ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type.to_owned() {
            types::ConnectorAuthType::CertificateAuth {
                certificate,
                private_key,
            } => Ok(Self {
                certificate,
                private_key,
            }),
            _ => Err(errors::ConnectorError::FailedToObtainAuthType.into()),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NetceteraErrorResponse {
    pub three_ds_server_trans_id: Option<String>,
    pub error_details: NetceteraErrorDetails,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NetceteraErrorDetails {
    pub error_code: String,
    pub error_component: Option<String>,
    pub error_description: Option<String>,
    pub error_detail: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NetceteraMetaData {
    pub mcc: String,
    pub merchant_country_code: String,
    pub merchant_name: String,
    pub endpoint_prefix: String,
}

impl TryFrom<&Option<common_utils::pii::SecretSerdeValue>> for NetceteraMetaData {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        meta_data: &Option<common_utils::pii::SecretSerdeValue>,
    ) -> Result<Self, Self::Error> {
        let metadata: Self = utils::to_connector_meta_from_secret::<Self>(meta_data.clone())
            .change_context(errors::ConnectorError::InvalidConnectorConfig {
                config: "metadata",
            })?;
        Ok(metadata)
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NetceteraPreAuthenticationRequest {
    cardholder_account_number: cards::CardNumber,
    scheme_id: Option<SchemeId>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum SchemeId {
    Visa,
    Mastercard,
    #[serde(rename = "JCB")]
    Jcb,
    #[serde(rename = "American Express")]
    AmericanExpress,
    Diners,
    // For Cartes Bancaires and UnionPay, it is recommended to send the scheme ID
    #[serde(rename = "CB")]
    CartesBancaires,
    UnionPay,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum NetceteraPreAuthenticationResponse {
    Success(Box<NetceteraPreAuthenticationResponseData>),
    Failure(Box<NetceteraErrorResponse>),
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NetceteraPreAuthenticationResponseData {
    #[serde(rename = "threeDSServerTransID")]
    pub three_ds_server_trans_id: String,
    pub card_ranges: Vec<CardRange>,
}

impl NetceteraPreAuthenticationResponseData {
    pub fn get_card_range_if_avalable(&self) -> Option<CardRange> {
        let card_range = self
            .card_ranges
            .iter()
            .max_by_key(|card_range| &card_range.highest_common_supported_version);
        card_range.cloned()
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CardRange {
    pub scheme_id: SchemeId,
    pub directory_server_id: Option<String>,
    pub acs_protocol_versions: Vec<AcsProtocolVersion>,
    #[serde(rename = "threeDSMethodDataForm")]
    pub three_ds_method_data_form: Option<ThreeDSMethodDataForm>,
    pub highest_common_supported_version: common_utils::types::SemanticVersion,
}

impl CardRange {
    pub fn get_three_ds_method_url(&self) -> Option<String> {
        self.acs_protocol_versions
            .iter()
            .find(|acs_protocol_version| {
                acs_protocol_version.version == self.highest_common_supported_version
            })
            .and_then(|acs_version| acs_version.three_ds_method_url.clone())
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ThreeDSMethodDataForm {
    // base64 encoded value for 3ds method data collection
    #[serde(rename = "threeDSMethodData")]
    pub three_ds_method_data: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AcsProtocolVersion {
    pub version: common_utils::types::SemanticVersion,
    #[serde(rename = "threeDSMethodURL")]
    pub three_ds_method_url: Option<String>,
}

impl TryFrom<&NetceteraRouterData<&types::authentication::PreAuthNRouterData>>
    for NetceteraPreAuthenticationRequest
{
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(
        value: &NetceteraRouterData<&types::authentication::PreAuthNRouterData>,
    ) -> Result<Self, Self::Error> {
        let router_data = value.router_data;
        Ok(Self {
            cardholder_account_number: router_data.request.card_holder_account_number.clone(),
            scheme_id: None,
        })
    }
}
