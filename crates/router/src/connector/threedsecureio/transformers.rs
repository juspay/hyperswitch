use api_models::payments::{DeviceChannel, SDKEphemPubKey, ThreeDSCompInd};
use common_utils::date_time;
use error_stack::{report, IntoReport, ResultExt};
use iso_currency::Currency;
use isocountry;
use masking::{ExposeInterface, PeekInterface, Secret};
use serde::{Deserialize, Serialize};

use crate::{
    connector::utils::{to_connector_meta, AddressDetailsData, CardData, SELECTED_PAYMENT_METHOD},
    consts::NO_ERROR_MESSAGE,
    core::errors,
    types::{
        self,
        api::{self, MessageCategory},
        authentication::ChallengeParams,
        storage::enums,
        transformers::ForeignTryFrom,
    },
    utils::OptionExt,
};

//TODO: Fill the struct with respective fields
pub struct ThreedsecureioRouterData<T> {
    pub amount: i64, // The type of amount that a connector accepts, for example, String, i64, f64, etc.
    pub router_data: T,
}

impl<T>
    TryFrom<(
        &types::api::CurrencyUnit,
        types::storage::enums::Currency,
        i64,
        T,
    )> for ThreedsecureioRouterData<T>
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

impl<T> TryFrom<(i64, T)> for ThreedsecureioRouterData<T> {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(router_data: (i64, T)) -> Result<Self, Self::Error> {
        //Todo :  use utils to convert the amount to the type of amount that a connector accepts
        Ok(Self {
            amount: router_data.0,
            router_data: router_data.1,
        })
    }
}

impl
    TryFrom<
        types::ResponseRouterData<
            api::Authentication,
            ThreedsecureioAuthenticationResponse,
            types::authentication::ConnectorAuthenticationRequestData,
            types::authentication::AuthenticationResponseData,
        >,
    > for types::authentication::ConnectorAuthenticationRouterData
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<
            api::Authentication,
            ThreedsecureioAuthenticationResponse,
            types::authentication::ConnectorAuthenticationRequestData,
            types::authentication::AuthenticationResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        let response = match item.response {
            ThreedsecureioAuthenticationResponse::Success(response) => {
                let creq = serde_json::json!({
                    "threeDSServerTransID": response.three_dsserver_trans_id,
                    "acsTransID": response.acs_trans_id,
                    "messageVersion": response.message_version,
                    "messageType": "CReq",
                    "challengeWindowSize": "01",
                });
                println!("creq authn {}", creq);
                let creq_str = serde_json::to_string(&creq)
                    .into_report()
                    .change_context(errors::ConnectorError::ResponseDeserializationFailed)
                    .attach_printable("error while constructing creq_str")?;
                let creq_base64 = base64::Engine::encode(&crate::consts::BASE64_ENGINE, creq_str)
                    .trim_end_matches('=')
                    .to_owned();
                println!("creq_base64 authn {}", creq_base64);
                Ok(
                    types::authentication::AuthenticationResponseData::AuthNResponse {
                        trans_status: response.trans_status.clone().into(),
                        authn_flow_type: if response.trans_status == ThreedsecureioTransStatus::C {
                            types::authentication::AuthNFlowType::Challenge(Box::new(
                                ChallengeParams {
                                    acs_url: response.acs_url,
                                    challenge_request: Some(creq_base64),
                                    acs_reference_number: Some(
                                        response.acs_reference_number.clone(),
                                    ),
                                    acs_trans_id: Some(response.acs_trans_id.clone()),
                                    three_dsserver_trans_id: Some(response.three_dsserver_trans_id),
                                    acs_signed_content: response.acs_signed_content,
                                },
                            ))
                        } else {
                            types::authentication::AuthNFlowType::Frictionless
                        },
                        cavv: response.authentication_value,
                    },
                )
            }
            ThreedsecureioAuthenticationResponse::Error(err_response) => match *err_response {
                ThreedsecureioErrorResponseWrapper::ErrorResponse(resp) => {
                    Err(types::ErrorResponse {
                        code: resp.error_code,
                        message: resp
                            .error_description
                            .clone()
                            .unwrap_or(NO_ERROR_MESSAGE.to_owned()),
                        reason: resp.error_description,
                        status_code: item.http_code,
                        attempt_status: None,
                        connector_transaction_id: None,
                    })
                }
                ThreedsecureioErrorResponseWrapper::ErrorString(error) => {
                    Err(types::ErrorResponse {
                        code: error.clone(),
                        message: error.clone(),
                        reason: Some(error),
                        status_code: item.http_code,
                        attempt_status: None,
                        connector_transaction_id: None,
                    })
                }
            },
        };
        Ok(Self {
            response,
            ..item.data.clone()
        })
    }
}

//TODO: Fill the struct with respective fields
// Auth Struct
pub struct ThreedsecureioAuthType {
    pub(super) api_key: Secret<String>,
}

impl TryFrom<&types::ConnectorAuthType> for ThreedsecureioAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &types::ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            types::ConnectorAuthType::HeaderKey { api_key } => Ok(Self {
                api_key: api_key.to_owned(),
            }),
            _ => Err(errors::ConnectorError::FailedToObtainAuthType.into()),
        }
    }
}
// PaymentsResponse
//TODO: Append the remaining status flags
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ThreedsecureioPaymentStatus {
    Succeeded,
    Failed,
    #[default]
    Processing,
}

impl From<ThreedsecureioPaymentStatus> for enums::AttemptStatus {
    fn from(item: ThreedsecureioPaymentStatus) -> Self {
        match item {
            ThreedsecureioPaymentStatus::Succeeded => Self::Charged,
            ThreedsecureioPaymentStatus::Failed => Self::Failure,
            ThreedsecureioPaymentStatus::Processing => Self::Authorizing,
        }
    }
}

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ThreedsecureioPaymentsResponse {
    status: ThreedsecureioPaymentStatus,
    id: String,
}

impl<F, T>
    TryFrom<
        types::ResponseRouterData<
            F,
            ThreedsecureioPaymentsResponse,
            T,
            types::PaymentsResponseData,
        >,
    > for types::RouterData<F, T, types::PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<
            F,
            ThreedsecureioPaymentsResponse,
            T,
            types::PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            status: enums::AttemptStatus::from(item.response.status),
            response: Ok(types::PaymentsResponseData::TransactionResponse {
                resource_id: types::ResponseId::ConnectorTransactionId(item.response.id),
                redirection_data: None,
                mandate_reference: None,
                connector_metadata: None,
                network_txn_id: None,
                connector_response_reference_id: None,
                incremental_authorization_allowed: None,
            }),
            ..item.data
        })
    }
}

//TODO: Fill the struct with respective fields
// REFUND :
// Type definition for RefundRequest
#[derive(Default, Debug, Serialize)]
pub struct ThreedsecureioRefundRequest {
    pub amount: i64,
}

impl<F> TryFrom<&ThreedsecureioRouterData<&types::RefundsRouterData<F>>>
    for ThreedsecureioRefundRequest
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &ThreedsecureioRouterData<&types::RefundsRouterData<F>>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            amount: item.amount.to_owned(),
        })
    }
}

// Type definition for Refund Response

#[allow(dead_code)]
#[derive(Debug, Serialize, Default, Deserialize, Clone)]
pub enum RefundStatus {
    Succeeded,
    Failed,
    #[default]
    Processing,
}

impl From<RefundStatus> for enums::RefundStatus {
    fn from(item: RefundStatus) -> Self {
        match item {
            RefundStatus::Succeeded => Self::Success,
            RefundStatus::Failed => Self::Failure,
            RefundStatus::Processing => Self::Pending,
            //TODO: Review mapping
        }
    }
}

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct RefundResponse {
    id: String,
    status: RefundStatus,
}

impl TryFrom<types::RefundsResponseRouterData<api::Execute, RefundResponse>>
    for types::RefundsRouterData<api::Execute>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::RefundsResponseRouterData<api::Execute, RefundResponse>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(types::RefundsResponseData {
                connector_refund_id: item.response.id.to_string(),
                refund_status: enums::RefundStatus::from(item.response.status),
            }),
            ..item.data
        })
    }
}

impl TryFrom<types::RefundsResponseRouterData<api::RSync, RefundResponse>>
    for types::RefundsRouterData<api::RSync>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::RefundsResponseRouterData<api::RSync, RefundResponse>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(types::RefundsResponseData {
                connector_refund_id: item.response.id.to_string(),
                refund_status: enums::RefundStatus::from(item.response.status),
            }),
            ..item.data
        })
    }
}

fn get_card_details(
    payment_method_data: api_models::payments::PaymentMethodData,
) -> Result<api_models::payments::Card, errors::ConnectorError> {
    match payment_method_data {
        api_models::payments::PaymentMethodData::Card(details) => Ok(details),
        _ => Err(errors::ConnectorError::NotSupported {
            message: SELECTED_PAYMENT_METHOD.to_string(),
            connector: "threedsecureio",
        })?,
    }
}

impl TryFrom<&ThreedsecureioRouterData<&types::authentication::ConnectorAuthenticationRouterData>>
    for ThreedsecureioAuthenticationRequest
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &ThreedsecureioRouterData<&types::authentication::ConnectorAuthenticationRouterData>,
    ) -> Result<Self, Self::Error> {
        let request = &item.router_data.request;
        //browser_details are mandatory for Browser flows
        let browser_details = match request.browser_details.clone() {
            Some(details) => Ok::<Option<types::BrowserInformation>, Self::Error>(Some(details)),
            None => {
                if request.device_channel == DeviceChannel::BRW {
                    Err(errors::ConnectorError::MissingRequiredField {
                        field_name: "browser_info",
                    })?
                } else {
                    Ok(None)
                }
            }
        }?;
        let card_details = get_card_details(request.payment_method_data.clone())?;
        let currency = request
            .currency
            .map(|currency| currency.to_string())
            .ok_or(errors::ConnectorError::RequestEncodingFailed)
            .into_report()
            .attach_printable("missing field currency")?;
        let purchase_currency: Currency = iso_currency::Currency::from_code(&currency)
            .ok_or(errors::ConnectorError::RequestEncodingFailed)
            .into_report()
            .attach_printable("error while parsing Currency")?;
        let billing_address = request.billing_address.address.clone().ok_or(
            errors::ConnectorError::MissingRequiredField {
                field_name: "billing_address.address",
            },
        )?;
        let billing_state = billing_address.clone().to_state_code()?;
        let billing_country = isocountry::CountryCode::for_alpha2(
            &billing_address
                .country
                .ok_or(errors::ConnectorError::MissingRequiredField {
                    field_name: "billing_address.address.country",
                })?
                .to_string(),
        )
        .into_report()
        .change_context(errors::ConnectorError::RequestEncodingFailed)
        .attach_printable("Error parsing billing_address.address.country")?;
        let connector_meta_data: ThreeDSecureIoMetaData = item
            .router_data
            .connector_meta_data
            .clone()
            .parse_value("ThreeDSecureIoMetaData")
            .change_context(errors::ConnectorError::RequestEncodingFailed)?;

        let authentication_data = &request.authentication_data.0;
        let sdk_information = match request.device_channel {
            DeviceChannel::APP => Some(item.router_data.request.sdk_information.clone().ok_or(
                errors::ConnectorError::MissingRequiredField {
                    field_name: "sdk_information",
                },
            )?),
            DeviceChannel::BRW => None,
        };
        let acquirer_details = authentication_data
            .acquirer_details
            .clone()
            .get_required_value("acquirer_details")
            .change_context(errors::ConnectorError::MissingRequiredField {
                field_name: "acquirer_details",
            })?;
        let meta: ThreeDSecureIoConnectorMetaData =
            to_connector_meta(request.authentication_data.1.connector_metadata.clone())?;
        Ok(Self {
            ds_start_protocol_version: meta.ds_start_protocol_version.clone(),
            ds_end_protocol_version: meta.ds_end_protocol_version.clone(),
            acs_start_protocol_version: meta.acs_start_protocol_version.clone(),
            acs_end_protocol_version: meta.acs_end_protocol_version.clone(),
            three_dsserver_trans_id: authentication_data.threeds_server_transaction_id.clone(),
            acct_number: card_details.card_number.clone(),
            notification_url: request
                .return_url
                .clone()
                .ok_or(errors::ConnectorError::RequestEncodingFailed)
                .into_report()
                .attach_printable("missing return_url")?,
            three_dscomp_ind: request.threeds_method_comp_ind.clone(),
            three_dsrequestor_url: request.three_ds_requestor_url.clone(),
            acquirer_bin: acquirer_details.acquirer_bin,
            acquirer_merchant_id: acquirer_details.acquirer_merchant_id,
            card_expiry_date: card_details.get_expiry_date_as_yymm()?.expose(),
            bill_addr_city: billing_address
                .city
                .clone()
                .ok_or(errors::ConnectorError::MissingRequiredField {
                    field_name: "billing_address.address.city",
                })?
                .to_string(),
            bill_addr_country: billing_country.numeric_id().to_string(),
            bill_addr_line1: billing_address
                .line1
                .clone()
                .ok_or(errors::ConnectorError::MissingRequiredField {
                    field_name: "billing_address.address.line1",
                })?
                .expose()
                .to_string(),
            bill_addr_post_code: billing_address
                .zip
                .clone()
                .ok_or(errors::ConnectorError::MissingRequiredField {
                    field_name: "billing_address.address.zip",
                })?
                .expose()
                .to_string(),
            bill_addr_state: billing_state.peek().to_string(),
            three_dsrequestor_authentication_ind: "01".to_string(),
            device_channel: match item.router_data.request.device_channel.clone() {
                DeviceChannel::APP => "01",
                DeviceChannel::BRW => "02",
            }
            .to_string(),
            message_category: match item.router_data.request.message_category.clone() {
                MessageCategory::Payment => "01",
                MessageCategory::NonPayment => "02",
            }
            .to_string(),
            browser_javascript_enabled: browser_details
                .clone()
                .and_then(|details| details.java_script_enabled),
            browser_accept_header: browser_details
                .clone()
                .and_then(|details| details.accept_header.clone()),
            browser_ip: browser_details
                .clone()
                .and_then(|details| details.ip_address.map(|ip| ip.to_string())),
            browser_java_enabled: browser_details
                .clone()
                .and_then(|details| details.java_enabled),
            browser_language: browser_details
                .clone()
                .and_then(|details| details.language.clone()),
            browser_color_depth: browser_details
                .clone()
                .and_then(|details| details.color_depth.map(|a| a.to_string())),
            browser_screen_height: browser_details
                .clone()
                .and_then(|details| details.screen_height.map(|a| a.to_string())),
            browser_screen_width: browser_details
                .clone()
                .and_then(|details| details.screen_width.map(|a| a.to_string())),
            browser_tz: browser_details
                .clone()
                .and_then(|details| details.time_zone.map(|a| a.to_string())),
            browser_user_agent: browser_details
                .clone()
                .and_then(|details| details.user_agent.map(|a| a.to_string())),
            mcc: connector_meta_data.mcc,
            merchant_country_code: connector_meta_data.merchant_country_code,
            merchant_name: connector_meta_data.merchant_name,
            message_type: "AReq".to_string(),
            message_version: authentication_data.message_version.clone(),
            purchase_amount: item.amount.to_string(),
            purchase_currency: purchase_currency.numeric().to_string(),
            trans_type: "01".to_string(),
            purchase_exponent: purchase_currency
                .exponent()
                .ok_or(errors::ConnectorError::RequestEncodingFailed)
                .into_report()
                .attach_printable("missing purchase_exponent")?
                .to_string(),
            purchase_date: date_time::DateTime::<date_time::YYYYMMDDHHmmss>::from(date_time::now())
                .to_string(),
            sdk_app_id: sdk_information.clone().map(|sdk_info| sdk_info.sdk_app_id),
            sdk_enc_data: sdk_information
                .clone()
                .map(|sdk_info| sdk_info.sdk_enc_data),
            sdk_ephem_pub_key: sdk_information
                .clone()
                .map(|sdk_info| sdk_info.sdk_ephem_pub_key),
            sdk_reference_number: sdk_information
                .clone()
                .map(|sdk_info| sdk_info.sdk_reference_number),
            sdk_trans_id: sdk_information
                .clone()
                .map(|sdk_info| sdk_info.sdk_trans_id),
            sdk_max_timeout: sdk_information
                .clone()
                .map(|sdk_info| sdk_info.sdk_max_timeout),
            device_render_options: match request.device_channel {
                DeviceChannel::APP => Some(DeviceRenderOptions {
                    sdk_interface: "01".to_string(),
                    sdk_ui_type: vec!["01".to_string()],
                }),
                DeviceChannel::BRW => None,
            },
            cardholder_name: card_details.card_holder_name,
            email: request.email.clone(),
        })
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThreedsecureioErrorResponse {
    pub error_code: String,
    pub error_component: Option<String>,
    pub error_description: Option<String>,
    pub error_detail: Option<String>,
    pub error_message_type: Option<String>,
    pub message_type: Option<String>,
    pub message_version: Option<String>,
    pub three_dsserver_trans_id: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum ThreedsecureioErrorResponseWrapper {
    ErrorResponse(ThreedsecureioErrorResponse),
    ErrorString(String),
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ThreedsecureioAuthenticationResponse {
    Success(Box<ThreedsecureioAuthenticationSuccessResponse>),
    Error(Box<ThreedsecureioErrorResponseWrapper>),
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ThreedsecureioAuthenticationSuccessResponse {
    #[serde(rename = "acsChallengeMandated")]
    pub acs_challenge_mandated: Option<String>,
    #[serde(rename = "acsOperatorID")]
    pub acs_operator_id: Option<String>,
    #[serde(rename = "acsReferenceNumber")]
    pub acs_reference_number: String,
    #[serde(rename = "acsTransID")]
    pub acs_trans_id: String,
    #[serde(rename = "acsURL")]
    pub acs_url: Option<url::Url>,
    #[serde(rename = "authenticationType")]
    pub authentication_type: Option<String>,
    #[serde(rename = "dsReferenceNumber")]
    pub ds_reference_number: String,
    #[serde(rename = "dsTransID")]
    pub ds_trans_id: String,
    #[serde(rename = "messageType")]
    pub message_type: Option<String>,
    #[serde(rename = "messageVersion")]
    pub message_version: String,
    #[serde(rename = "threeDSServerTransID")]
    pub three_dsserver_trans_id: String,
    #[serde(rename = "transStatus")]
    pub trans_status: ThreedsecureioTransStatus,
    #[serde(rename = "acsSignedContent")]
    pub acs_signed_content: Option<String>,
    #[serde(rename = "authenticationValue")]
    pub authentication_value: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThreedsecureioAuthenticationRequest {
    pub ds_start_protocol_version: String,
    pub ds_end_protocol_version: String,
    pub acs_start_protocol_version: String,
    pub acs_end_protocol_version: String,
    pub three_dsserver_trans_id: String,
    pub acct_number: cards::CardNumber,
    pub notification_url: String,
    pub three_dscomp_ind: ThreeDSCompInd,
    pub three_dsrequestor_url: String,
    pub acquirer_bin: String,
    pub acquirer_merchant_id: String,
    pub card_expiry_date: String,
    pub bill_addr_city: String,
    pub bill_addr_country: String,
    pub bill_addr_line1: String,
    pub bill_addr_post_code: String,
    pub bill_addr_state: String,
    pub email: Option<common_utils::pii::Email>,
    pub three_dsrequestor_authentication_ind: String,
    pub cardholder_name: Option<Secret<String>>,
    pub device_channel: String,
    pub browser_javascript_enabled: Option<bool>,
    pub browser_accept_header: Option<String>,
    pub browser_ip: Option<String>,
    pub browser_java_enabled: Option<bool>,
    pub browser_language: Option<String>,
    pub browser_color_depth: Option<String>,
    pub browser_screen_height: Option<String>,
    pub browser_screen_width: Option<String>,
    pub browser_tz: Option<String>,
    pub browser_user_agent: Option<String>,
    pub sdk_app_id: Option<String>,
    pub sdk_enc_data: Option<String>,
    pub sdk_ephem_pub_key: Option<SDKEphemPubKey>,
    pub sdk_reference_number: Option<String>,
    pub sdk_trans_id: Option<String>,
    pub mcc: String,
    pub merchant_country_code: String,
    pub merchant_name: String,
    pub message_category: String,
    pub message_type: String,
    pub message_version: String,
    pub purchase_amount: String,
    pub purchase_currency: String,
    pub purchase_exponent: String,
    pub purchase_date: String,
    pub trans_type: String,
    pub sdk_max_timeout: Option<String>,
    pub device_render_options: Option<DeviceRenderOptions>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ThreeDSecureIoMetaData {
    pub mcc: String,
    pub merchant_country_code: String,
    pub merchant_name: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ThreeDSecureIoConnectorMetaData {
    pub ds_start_protocol_version: String,
    pub ds_end_protocol_version: String,
    pub acs_start_protocol_version: String,
    pub acs_end_protocol_version: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceRenderOptions {
    pub sdk_interface: String,
    pub sdk_ui_type: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThreedsecureioPreAuthenticationRequest {
    acct_number: cards::CardNumber,
    ds: Option<DirectoryServer>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThreedsecureioPostAuthenticationRequest {
    pub three_ds_server_trans_id: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ThreedsecureioPostAuthenticationResponse {
    pub authentication_value: Option<String>,
    pub trans_status: ThreedsecureioTransStatus,
    pub eci: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub enum ThreedsecureioTransStatus {
    /// Authentication/ Account Verification Successful
    Y,
    /// Not Authenticated /Account Not Verified; Transaction denied
    N,
    /// Authentication/ Account Verification Could Not Be Performed; Technical or other problem, as indicated in ARes or RReq
    U,
    /// Attempts Processing Performed; Not Authenticated/Verified , but a proof of attempted authentication/verification is provided
    A,
    /// Authentication/ Account Verification Rejected; Issuer is rejecting authentication/verification and request that authorisation not be attempted.
    R,
    C,
}

impl From<ThreedsecureioTransStatus> for api_models::payments::TransStatus {
    fn from(value: ThreedsecureioTransStatus) -> Self {
        match value {
            ThreedsecureioTransStatus::Y => Self::Y,
            ThreedsecureioTransStatus::N => Self::N,
            ThreedsecureioTransStatus::U => Self::U,
            ThreedsecureioTransStatus::A => Self::A,
            ThreedsecureioTransStatus::R => Self::R,
            ThreedsecureioTransStatus::C => Self::C,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DirectoryServer {
    Standin,
    Visa,
    Mastercard,
    Jcb,
    Upi,
    Amex,
    Protectbuy,
    Sbn,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum ThreedsecureioPreAuthenticationResponse {
    Success(Box<ThreedsecureioPreAuthenticationResponseData>),
    Failure(Box<ThreedsecureioErrorResponse>),
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ThreedsecureioPreAuthenticationResponseData {
    pub ds_start_protocol_version: String,
    pub ds_end_protocol_version: String,
    pub acs_start_protocol_version: String,
    pub acs_end_protocol_version: String,
    #[serde(rename = "threeDSMethodURL")]
    pub threeds_method_url: Option<String>,
    #[serde(rename = "threeDSServerTransID")]
    pub threeds_server_trans_id: String,
    pub scheme: Option<String>,
    pub message_type: Option<String>,
}

impl TryFrom<&ThreedsecureioRouterData<&types::authentication::PreAuthNRouterData>>
    for ThreedsecureioPreAuthenticationRequest
{
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(
        value: &ThreedsecureioRouterData<&types::authentication::PreAuthNRouterData>,
    ) -> Result<Self, Self::Error> {
        let router_data = value.router_data;
        Ok(Self {
            acct_number: router_data.request.card_holder_account_number.clone(),
            ds: None,
        })
    }
}

impl ForeignTryFrom<String> for (i64, i64, i64) {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn foreign_try_from(value: String) -> Result<Self, Self::Error> {
        let mut split_version = value.split('.');
        let version_string = {
            let major_version = split_version.next().ok_or(report!(
                errors::ConnectorError::ResponseDeserializationFailed
            ))?;
            let minor_version = split_version.next().ok_or(report!(
                errors::ConnectorError::ResponseDeserializationFailed
            ))?;
            let patch_version = split_version.next().ok_or(report!(
                errors::ConnectorError::ResponseDeserializationFailed
            ))?;
            (major_version, minor_version, patch_version)
        };
        let int_representation = {
            let major_version = version_string
                .0
                .parse()
                .into_report()
                .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
            let minor_version = version_string
                .1
                .parse()
                .into_report()
                .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
            let patch_version = version_string
                .2
                .parse()
                .into_report()
                .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
            (major_version, minor_version, patch_version)
        };
        Ok(int_representation)
    }
}
