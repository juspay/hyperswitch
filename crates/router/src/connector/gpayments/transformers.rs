use api_models::payments::DeviceChannel;
use base64::Engine;
use common_utils::{date_time, types::MinorUnit};
use error_stack::ResultExt;
use masking::{ExposeInterface, Secret};
use serde::Deserialize;
use serde_json::to_string;

use super::gpayments_types;
use crate::{
    connector::{
        gpayments::gpayments_types::{
            AuthStatus, BrowserInfoCollected, GpaymentsAuthenticationSuccessResponse,
        },
        utils,
        utils::{get_card_details, CardData},
    },
    consts::BASE64_ENGINE,
    core::errors,
    types::{self, api, api::MessageCategory, authentication::ChallengeParams},
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
            acct_number: router_data.request.card.card_number.clone(),
            merchant_id: metadata.merchant_id,
        })
    }
}

#[derive(Deserialize, PartialEq)]
pub struct GpaymentsMetaData {
    pub endpoint_prefix: String,
    pub merchant_id: common_utils::id_type::MerchantId,
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
            acct_number: router_data.request.card.card_number.clone(),
            card_scheme: None,
            challenge_window_size: Some(gpayments_types::ChallengeWindowSize::FullScreen),
            event_callback_url: "https://webhook.site/55e3db24-7c4e-4432-9941-d806f68d210b"
                .to_string(),
            merchant_id: metadata.merchant_id,
            skip_auto_browser_info_collect: Some(true),
            // should auto generate this id.
            three_ds_requestor_trans_id: uuid::Uuid::new_v4().hyphenated().to_string(),
        })
    }
}

impl TryFrom<&GpaymentsRouterData<&types::authentication::ConnectorAuthenticationRouterData>>
    for gpayments_types::GpaymentsAuthenticationRequest
{
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(
        item: &GpaymentsRouterData<&types::authentication::ConnectorAuthenticationRouterData>,
    ) -> Result<Self, Self::Error> {
        let request = &item.router_data.request;
        let browser_details = match request.browser_details.clone() {
            Some(details) => Ok::<Option<types::BrowserInformation>, Self::Error>(Some(details)),
            None => {
                if request.device_channel == DeviceChannel::Browser {
                    Err(errors::ConnectorError::MissingRequiredField {
                        field_name: "browser_info",
                    })?
                } else {
                    Ok(None)
                }
            }
        }?;
        let card_details = get_card_details(request.payment_method_data.clone(), "gpayments")?;

        let metadata = GpaymentsMetaData::try_from(&item.router_data.connector_meta_data)?;

        Ok(Self {
            acct_number: card_details.card_number.clone(),
            authentication_ind: "01".into(),
            card_expiry_date: card_details.get_expiry_date_as_yymm()?.expose(),
            merchant_id: metadata.merchant_id,
            message_category: match item.router_data.request.message_category.clone() {
                MessageCategory::Payment => "01".into(),
                MessageCategory::NonPayment => "02".into(),
            },
            notification_url: request
                .return_url
                .clone()
                .ok_or(errors::ConnectorError::RequestEncodingFailed)
                .attach_printable("missing return_url")?,
            three_ds_comp_ind: request.threeds_method_comp_ind.clone(),
            purchase_amount: item.amount.to_string(),
            purchase_date: date_time::DateTime::<date_time::YYYYMMDDHHmmss>::from(date_time::now())
                .to_string(),
            three_ds_server_trans_id: request
                .pre_authentication_data
                .threeds_server_transaction_id
                .clone(),
            browser_info_collected: BrowserInfoCollected {
                browser_javascript_enabled: browser_details
                    .as_ref()
                    .and_then(|details| details.java_script_enabled),
                browser_accept_header: browser_details
                    .as_ref()
                    .and_then(|details| details.accept_header.clone()),
                browser_ip: browser_details
                    .clone()
                    .and_then(|details| details.ip_address.map(|ip| Secret::new(ip.to_string()))),
                browser_java_enabled: browser_details
                    .as_ref()
                    .and_then(|details| details.java_enabled),
                browser_language: browser_details
                    .as_ref()
                    .and_then(|details| details.language.clone()),
                browser_color_depth: browser_details
                    .as_ref()
                    .and_then(|details| details.color_depth.map(|a| a.to_string())),
                browser_screen_height: browser_details
                    .as_ref()
                    .and_then(|details| details.screen_height.map(|a| a.to_string())),
                browser_screen_width: browser_details
                    .as_ref()
                    .and_then(|details| details.screen_width.map(|a| a.to_string())),
                browser_tz: browser_details
                    .as_ref()
                    .and_then(|details| details.time_zone.map(|a| a.to_string())),
                browser_user_agent: browser_details
                    .as_ref()
                    .and_then(|details| details.user_agent.clone().map(|a| a.to_string())),
            },
        })
    }
}
impl
    TryFrom<
        types::ResponseRouterData<
            api::Authentication,
            GpaymentsAuthenticationSuccessResponse,
            types::authentication::ConnectorAuthenticationRequestData,
            types::authentication::AuthenticationResponseData,
        >,
    > for types::authentication::ConnectorAuthenticationRouterData
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<
            api::Authentication,
            GpaymentsAuthenticationSuccessResponse,
            types::authentication::ConnectorAuthenticationRequestData,
            types::authentication::AuthenticationResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        let response_auth = item.response;
        let creq = serde_json::json!({
            "threeDSServerTransID": response_auth.three_ds_server_trans_id,
            "acsTransID": response_auth.acs_trans_id,
            "messageVersion": response_auth.message_version,
            "messageType": "CReq",
            "challengeWindowSize": "01",
        });
        let creq_str = to_string(&creq)
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)
            .attach_printable("error while constructing creq_str")?;
        let creq_base64 = Engine::encode(&BASE64_ENGINE, creq_str)
            .trim_end_matches('=')
            .to_owned();
        let response: Result<
            types::authentication::AuthenticationResponseData,
            types::ErrorResponse,
        > = Ok(
            types::authentication::AuthenticationResponseData::AuthNResponse {
                trans_status: response_auth.trans_status.clone().into(),
                authn_flow_type: if response_auth.trans_status == AuthStatus::C {
                    types::authentication::AuthNFlowType::Challenge(Box::new(ChallengeParams {
                        acs_url: response_auth.acs_url,
                        challenge_request: Some(creq_base64),
                        acs_reference_number: Some(response_auth.acs_reference_number.clone()),
                        acs_trans_id: Some(response_auth.acs_trans_id.clone()),
                        three_dsserver_trans_id: Some(response_auth.three_ds_server_trans_id),
                        acs_signed_content: None,
                    }))
                } else {
                    types::authentication::AuthNFlowType::Frictionless
                },
                authentication_value: response_auth.authentication_value,
                ds_trans_id: Some(response_auth.ds_trans_id),
                connector_metadata: None,
            },
        );
        Ok(Self {
            response,
            ..item.data.clone()
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
                to_string(&three_ds_method_data_json)
                    .change_context(errors::ConnectorError::ResponseDeserializationFailed)
                    .attach_printable("error while constructing three_ds_method_data_str")
                    .map(|three_ds_method_data_string| {
                        Engine::encode(&BASE64_ENGINE, three_ds_method_data_string)
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
