use error_stack::ResultExt;
use masking::Secret;
use serde::{Deserialize, Serialize};

use super::netcetera_types;
use crate::{
    connector::utils::{self, CardData},
    core::errors,
    types::{self, api},
    utils::OptionExt,
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
                    message: error_response.error_details.error_detail,
                    reason: Some(error_response.error_details.error_description),
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

impl
    TryFrom<
        types::ResponseRouterData<
            api::Authentication,
            NetceteraAuthenticationResponse,
            types::authentication::ConnectorAuthenticationRequestData,
            types::authentication::AuthenticationResponseData,
        >,
    > for types::authentication::ConnectorAuthenticationRouterData
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<
            api::Authentication,
            NetceteraAuthenticationResponse,
            types::authentication::ConnectorAuthenticationRequestData,
            types::authentication::AuthenticationResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        let response = match item.response {
            NetceteraAuthenticationResponse::Success(response) => {
                let authn_flow_type = match response.acs_challenge_mandated {
                    Some(ACSChallengeMandatedIndicator::Y) => {
                        types::authentication::AuthNFlowType::Challenge(Box::new(
                            types::authentication::ChallengeParams {
                                acs_url: response.authentication_response.acs_url.clone(),
                                challenge_request: response.encoded_challenge_request,
                                acs_reference_number: response
                                    .authentication_response
                                    .acs_reference_number,
                                acs_trans_id: response.authentication_response.acs_trans_id,
                                three_dsserver_trans_id: Some(response.three_ds_server_trans_id),
                                acs_signed_content: None,
                            },
                        ))
                    }
                    _ => types::authentication::AuthNFlowType::Frictionless,
                };
                Ok(
                    types::authentication::AuthenticationResponseData::AuthNResponse {
                        authn_flow_type,
                        authentication_value: response.authentication_value,
                        trans_status: response.trans_status,
                    },
                )
            }
            NetceteraAuthenticationResponse::Error(error_response) => Err(types::ErrorResponse {
                code: error_response.error_details.error_code,
                message: error_response.error_details.error_detail,
                reason: Some(error_response.error_details.error_description),
                status_code: item.http_code,
                attempt_status: None,
                connector_transaction_id: None,
            }),
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
    /// Universally unique identifier for the transaction assigned by the 3DS Server.
    #[serde(rename = "threeDSServerTransID")]
    pub three_ds_server_trans_id: Option<String>,

    /// Universally Unique identifier for the transaction assigned by the ACS.
    #[serde(rename = "acsTransID")]
    pub acs_trans_id: Option<String>,

    /// Universally unique identifier for the transaction assigned by the DS.
    #[serde(rename = "dsTransID")]
    pub ds_trans_id: Option<String>,

    /// Code indicating the type of problem identified.
    pub error_code: String,

    /// Code indicating the 3-D Secure component that identified the error.
    pub error_component: String,

    /// Text describing the problem identified.
    pub error_description: String,

    /// Additional detail regarding the problem identified.
    pub error_detail: String,

    /// Universally unique identifier for the transaction assigned by the 3DS SDK.
    #[serde(rename = "sdkTransID")]
    pub sdk_trans_id: Option<String>,

    /// The Message Type that was identified as erroneous.
    pub error_message_type: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NetceteraMetaData {
    pub mcc: String,
    pub merchant_country_code: String,
    pub merchant_name: String,
    pub endpoint_prefix: String,
    pub three_ds_requestor_name: String,
    pub three_ds_requestor_id: String,
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

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
#[serde_with::skip_serializing_none]
pub struct NetceteraAuthenticationRequest {
    /// Specifies the preferred version of 3D Secure protocol to be utilized while executing 3D Secure authentication.
    /// 3DS Server initiates an authentication request with the preferred version and if this version is not supported by
    /// other 3D Secure components, it falls back to the next supported version(s) and continues authentication.
    ///
    /// If the preferred version is enforced by setting  #enforcePreferredProtocolVersion flag, but this version
    /// is not supported by one of the 3D Secure components, 3DS Server does not initiate an authentication and provides
    /// corresponding error message to the customer.
    ///
    /// The accepted values are:
    /// - 2.1.0 -> prefer authentication with 2.1.0 version,
    /// - 2.2.0 -> prefer authentication with 2.2.0 version,
    /// - 2.3.1 -> prefer authentication with 2.3.1 version,
    /// - latest -> prefer authentication with the latest version, the 3DS Server is certified for. 2.3.1 at this moment.
    pub preferred_protocol_version: Option<common_utils::types::SemanticVersion>,
    /// Boolean flag that enforces preferred 3D Secure protocol version to be used in 3D Secure authentication.
    /// The value should be set true to enforce preferred version. If value is false or not provided,
    /// 3DS Server can fall back to next supported 3DS protocol version while initiating 3D Secure authentication.
    ///
    /// For application initiated transactions (deviceChannel = '01'), the preferred protocol version must be enforced.
    pub enforce_preferred_protocol_version: Option<bool>,
    pub device_channel: netcetera_types::NetceteraDeviceChannel,
    /// Identifies the category of the message for a specific use case. The accepted values are:
    ///
    /// - 01 -> PA
    /// - 02 -> NPA
    /// - 80 - 99 -> PS Specific Values (80 -> MasterCard Identity Check Insights;
    ///                                85 -> MasterCard Identity Check, Production Validation PA;
    ///                                86 -> MasterCard Identity Check, Production Validation NPA)
    pub message_category: netcetera_types::NetceteraMessageCategory,
    #[serde(rename = "threeDSCompInd")]
    pub three_ds_comp_ind: Option<netcetera_types::ThreeDSMethodCompletionIndicator>,
    /**
     * Contains the 3DS Server Transaction ID used during the previous execution of the 3DS method. Accepted value
     * length is 36 characters. Accepted value is a Canonical format as defined in IETF RFC 4122. May utilise any of the
     * specified versions if the output meets specified requirements.
     *
     * This field is required if the 3DS Requestor reuses previous 3DS Method execution with deviceChannel = 02 (BRW).
     * Available for supporting EMV 3DS 2.3.1 and later versions.
     */
    #[serde(rename = "threeDSMethodId")]
    pub three_ds_method_id: Option<String>,
    #[serde(rename = "threeDSRequestor")]
    pub three_ds_requestor: Option<netcetera_types::ThreeDSRequestor>,
    #[serde(rename = "threeDSServerTransID")]
    pub three_ds_server_trans_id: String,
    #[serde(rename = "threeDSRequestorURL")]
    pub three_ds_requestor_url: Option<String>,
    pub cardholder_account: netcetera_types::CardholderAccount,
    pub cardholder: Option<netcetera_types::Cardholder>,
    pub purchase: Option<netcetera_types::Purchase>,
    pub acquirer: Option<netcetera_types::AcquirerData>,
    pub merchant: Option<netcetera_types::MerchantData>,
    pub broad_info: Option<String>,
    pub device_render_options: Option<String>,
    pub message_extension: Option<Vec<netcetera_types::MessageExtensionAttribute>>,
    pub challenge_message_extension: Option<Vec<netcetera_types::MessageExtensionAttribute>>,
    pub browser_information: Option<netcetera_types::Browser>,
    #[serde(rename = "threeRIInd")]
    pub three_ri_ind: Option<String>,
    pub sdk_information: Option<netcetera_types::Sdk>,
    pub device: Option<String>,
    pub multi_transaction: Option<String>,
    pub device_id: Option<String>,
    pub user_id: Option<String>,
    pub payee_origin: Option<url::Url>,
}

impl TryFrom<&NetceteraRouterData<&types::authentication::ConnectorAuthenticationRouterData>>
    for NetceteraAuthenticationRequest
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &NetceteraRouterData<&types::authentication::ConnectorAuthenticationRouterData>,
    ) -> Result<Self, Self::Error> {
        let now = common_utils::date_time::now();
        let three_ds_req_auth_timestamp = common_utils::date_time::format_date(
            now,
            common_utils::date_time::DateFormat::YYYYMMDDHHmm,
        )
        .change_context(errors::ConnectorError::RequestEncodingFailedWithReason(
            "Failed to format Date".to_string(),
        ))?;
        let request = item.router_data.request.clone();
        let pre_authn_data = request.pre_authentication_data.clone();
        let three_ds_requestor = netcetera_types::ThreeDSRequestor {
            three_ds_requestor_authentication_ind:
                netcetera_types::ThreeDSRequestorAuthenticationIndicator::Payment,
            three_ds_requestor_authentication_info: Some(
                netcetera_types::SingleOrListElement::new_single(
                    netcetera_types::ThreeDSRequestorAuthenticationInformation {
                        three_ds_req_auth_method: "01".to_string(),
                        three_ds_req_auth_timestamp,
                        three_ds_req_auth_data: None,
                    },
                ),
            ),
            three_ds_requestor_challenge_ind: None,
            three_ds_requestor_prior_authentication_info: None,
            three_ds_requestor_dec_req_ind: None,
            three_ds_requestor_dec_max_time: None,
            app_ip: None,
            three_ds_requestor_spc_support: None,
            spc_incomp_ind: None,
        };
        let card = utils::get_card_details(request.payment_method_data, "netcetera")?;
        let cardholder_account = netcetera_types::CardholderAccount {
            acct_type: None,
            card_expiry_date: Some(card.get_expiry_date_as_yymm()?),
            acct_info: None,
            acct_number: card.card_number,
            scheme_id: None,
            acct_id: None,
            pay_token_ind: None,
            pay_token_info: None,
            card_security_code: Some(card.card_cvc),
        };
        let currency = request
            .currency
            .get_required_value("currency")
            .change_context(errors::ConnectorError::MissingRequiredField {
                field_name: "currency",
            })?;
        let purchase = netcetera_types::Purchase {
            purchase_instal_data: None,
            merchant_risk_indicator: None,
            purchase_amount: request.amount,
            purchase_currency: currency.iso_4217().to_string(),
            purchase_exponent: currency.number_of_digits_after_decimal_point(),
            purchase_date: Some(
                common_utils::date_time::format_date(
                    now,
                    common_utils::date_time::DateFormat::YYYYMMDDHHmmss,
                )
                .change_context(
                    errors::ConnectorError::RequestEncodingFailedWithReason(
                        "Failed to format Date".to_string(),
                    ),
                )?,
            ),
            recurring_expiry: Some("20240401".to_string()),
            recurring_frequency: Some(1),
            trans_type: None,
            recurring_amount: None,
            recurring_currency: None,
            recurring_exponent: None,
            recurring_date: None,
            amount_ind: None,
            frequency_ind: None,
        };
        let acquirer_details = netcetera_types::AcquirerData {
            acquirer_bin: request.pre_authentication_data.acquirer_bin,
            acquirer_merchant_id: request.pre_authentication_data.acquirer_merchant_id,
            acquirer_country_code: None,
        };
        let connector_meta_data: NetceteraMetaData = item
            .router_data
            .connector_meta_data
            .clone()
            .parse_value("NetceteraMetaData")
            .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        let merchant_data = netcetera_types::MerchantData {
            merchant_configuration_id: None,
            mcc: Some(connector_meta_data.mcc),
            merchant_country_code: Some(connector_meta_data.merchant_country_code),
            merchant_name: Some(connector_meta_data.merchant_name),
            notification_url: request.return_url.clone(),
            three_ds_requestor_id: Some(connector_meta_data.three_ds_requestor_id),
            three_ds_requestor_name: Some(connector_meta_data.three_ds_requestor_name),
            white_list_status: None,
            trust_list_status: None,
            seller_info: None,
            results_response_notification_url: request.return_url,
        };
        let browser_information = request.browser_details.map(netcetera_types::Browser::from);
        let sdk_information = request.sdk_information.map(netcetera_types::Sdk::from);
        Ok(Self {
            preferred_protocol_version: Some(pre_authn_data.message_version),
            enforce_preferred_protocol_version: None,
            device_channel: netcetera_types::NetceteraDeviceChannel::from(request.device_channel),
            message_category: netcetera_types::NetceteraMessageCategory::from(
                request.message_category,
            ),
            three_ds_comp_ind: Some(netcetera_types::ThreeDSMethodCompletionIndicator::from(
                request.threeds_method_comp_ind,
            )),
            three_ds_method_id: None,
            three_ds_requestor: Some(three_ds_requestor),
            three_ds_server_trans_id: pre_authn_data.threeds_server_transaction_id,
            three_ds_requestor_url: Some(request.three_ds_requestor_url),
            cardholder_account,
            cardholder: Some(netcetera_types::Cardholder::from((
                request.billing_address,
                request.shipping_address,
            ))),
            purchase: Some(purchase),
            acquirer: Some(acquirer_details),
            merchant: Some(merchant_data),
            broad_info: None,
            device_render_options: None,
            message_extension: None,
            challenge_message_extension: None,
            browser_information,
            three_ri_ind: None,
            sdk_information,
            device: None,
            multi_transaction: None,
            device_id: None,
            user_id: None,
            payee_origin: None,
        })
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum NetceteraAuthenticationResponse {
    Error(NetceteraAuthenticationFailureResponse),
    Success(NetceteraAuthenticationSuccessResponse),
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NetceteraAuthenticationSuccessResponse {
    #[serde(rename = "threeDSServerTransID")]
    pub three_ds_server_trans_id: String,
    pub trans_status: common_enums::TransactionStatus,
    pub authentication_value: Option<String>,
    pub eci: Option<String>,
    pub acs_challenge_mandated: Option<ACSChallengeMandatedIndicator>,
    pub authentication_response: AuthenticationResponse,
    #[serde(rename = "base64EncodedChallengeRequest")]
    pub encoded_challenge_request: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NetceteraAuthenticationFailureResponse {
    pub error_details: NetceteraErrorDetails,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthenticationResponse {
    #[serde(rename = "acsURL")]
    pub acs_url: Option<url::Url>,
    pub acs_reference_number: Option<String>,
    #[serde(rename = "acsTransID")]
    pub acs_trans_id: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub enum ACSChallengeMandatedIndicator {
    /// Challenge is mandated
    Y,
    /// Challenge is not mandated
    N,
}
