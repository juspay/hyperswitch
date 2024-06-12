use common_utils::types::MinorUnit;
use error_stack::ResultExt;
use masking::Secret;
use serde::{Deserialize, Serialize};

use super::gpayments_types;
use crate::{
    connector::utils,
    consts,
    core::errors,
    types::{self, api},
};
pub struct GpaymentsRouterData<T> {
    pub amount: MinorUnit, // The type of amount that a connector accepts, for example, String, i64, f64, etc.
    pub router_data: T,
}

impl<T> From<(MinorUnit, T)> for GpaymentsRouterData<T> {
    fn from((amount, item): (MinorUnit, T)) -> Self {
        Self {
            amount,
            router_data: item,
        }
    }
}

//TODO: Fill the struct with respective fields
// Auth Struct
pub struct GpaymentsAuthType {
    /// base64 encoded certificate
    pub certificate: Secret<String>,
    /// base64 encoded private_key
    pub private_key: Secret<String>,
}

impl TryFrom<&types::ConnectorAuthType> for GpaymentsAuthType {
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

impl TryFrom<&GpaymentsRouterData<&types::authentication::PreAuthNVersionCallRouterData>>
    for gpayments_types::GpaymentsPreAuthVersionCallRequest
{
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(
        value: &GpaymentsRouterData<&types::authentication::PreAuthNVersionCallRouterData>,
    ) -> Result<Self, Self::Error> {
        let router_data = value.router_data;
        let metadata = GpaymentsMetaData::try_from(&router_data.connector_meta_data)?;
        Ok(Self {
            acct_number: router_data.request.card_holder_account_number.clone(),
            merchant_id: metadata.merchant_id,
        })
    }
}

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
pub struct GpaymentsErrorResponse {
    pub status_code: u16,
    pub code: String,
    pub message: String,
    pub reason: Option<String>,
}

#[derive(Deserialize, PartialEq)]
pub struct GpaymentsMetaData {
    pub endpoint_prefix: String,
    pub merchant_id: String,
}

impl TryFrom<&Option<common_utils::pii::SecretSerdeValue>> for GpaymentsMetaData {
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

impl
    TryFrom<
        types::ResponseRouterData<
            api::PreAuthenticationVersionCall,
            gpayments_types::GpaymentsPreAuthVersionCallResponse,
            types::authentication::PreAuthNRequestData,
            types::authentication::AuthenticationResponseData,
        >,
    > for types::authentication::PreAuthNVersionCallRouterData
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<
            api::PreAuthenticationVersionCall,
            gpayments_types::GpaymentsPreAuthVersionCallResponse,
            types::authentication::PreAuthNRequestData,
            types::authentication::AuthenticationResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        let version_response = item.response;
        let response = Ok(
            types::authentication::AuthenticationResponseData::PreAuthVersionCallResponse {
                maximum_supported_3ds_version: version_response
                    .supported_message_versions
                    .and_then(|supported_version| supported_version.iter().max().cloned()) // if no version is returned for the card number, then
                    .unwrap_or(common_utils::types::SemanticVersion::new(0, 0, 0)),
            },
        );
        Ok(Self {
            response,
            ..item.data.clone()
        })
    }
}

impl TryFrom<&GpaymentsRouterData<&types::authentication::PreAuthNRouterData>>
    for gpayments_types::GpaymentsPreAuthenticationRequest
{
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(
        value: &GpaymentsRouterData<&types::authentication::PreAuthNRouterData>,
    ) -> Result<Self, Self::Error> {
        let router_data = value.router_data;
        let metadata = GpaymentsMetaData::try_from(&router_data.connector_meta_data)?;
        Ok(Self {
            acct_number: router_data.request.card_holder_account_number.clone(),
            card_scheme: None,
            challenge_window_size: Some(gpayments_types::ChallengeWindowSize::FullScreen),
            // This is a required field but we don't listen to event callbacks
            event_callback_url: "https://webhook.site/55e3db24-7c4e-4432-9941-d806f68d210b"
                .to_string(),
            merchant_id: metadata.merchant_id,
            // Since this feature is not in our favour, hard coded it to true
            skip_auto_browser_info_collect: Some(true),
            // should auto generate this id.
            three_ds_requestor_trans_id: uuid::Uuid::new_v4().hyphenated().to_string(),
        })
    }
}

impl
    TryFrom<
        types::ResponseRouterData<
            api::PreAuthentication,
            gpayments_types::GpaymentsPreAuthenticationResponse,
            types::authentication::PreAuthNRequestData,
            types::authentication::AuthenticationResponseData,
        >,
    > for types::authentication::PreAuthNRouterData
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<
            api::PreAuthentication,
            gpayments_types::GpaymentsPreAuthenticationResponse,
            types::authentication::PreAuthNRequestData,
            types::authentication::AuthenticationResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        let threeds_method_response = item.response;
        let three_ds_method_data = threeds_method_response
            .three_ds_method_url
            .as_ref()
            .map(|_| {
                let three_ds_method_data_json = serde_json::json!({
                    "threeDSServerTransID": threeds_method_response.three_ds_server_trans_id,
                    "threeDSMethodNotificationURL": "https://webhook.site/bd06863d-82c2-42ea-b35b-5ffd5ecece71"
                });
                serde_json::to_string(&three_ds_method_data_json)
                    .change_context(errors::ConnectorError::ResponseDeserializationFailed)
                    .attach_printable("error while constructing three_ds_method_data_str")
                    .map(|three_ds_method_data_string| {
                        base64::Engine::encode(&consts::BASE64_ENGINE, three_ds_method_data_string)
                    })
            })
            .transpose()?;
        let connector_metadata = Some(serde_json::json!(
            gpayments_types::GpaymentsConnectorMetaData {
                authentication_url: threeds_method_response.auth_url,
                three_ds_requestor_trans_id: None,
            }
        ));
        let response: Result<
            types::authentication::AuthenticationResponseData,
            types::ErrorResponse,
        > = Ok(
            types::authentication::AuthenticationResponseData::PreAuthThreeDsMethodCallResponse {
                threeds_server_transaction_id: threeds_method_response
                    .three_ds_server_trans_id
                    .clone(),
                three_ds_method_data,
                three_ds_method_url: threeds_method_response.three_ds_method_url,
                connector_metadata,
            },
        );
        Ok(Self {
            response,
            ..item.data.clone()
        })
    }
}
