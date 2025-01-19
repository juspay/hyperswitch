use common_enums::enums;
use common_utils::types::FloatMajorUnit;
use hyperswitch_domain_models::{
    router_data::{ConnectorAuthType, RouterData},
    router_request_types::unified_authentication_service::{
        DynamicData, PostAuthenticationDetails, PreAuthenticationDetails, TokenDetails,
        UasAuthenticationResponseData,
    },
    types::{UasPostAuthenticationRouterData, UasPreAuthenticationRouterData},
};
use hyperswitch_interfaces::errors;
use masking::Secret;
use serde::{Deserialize, Serialize};
use time::PrimitiveDateTime;

use crate::types::ResponseRouterData;

const CTP_MASTERCARD: &str = "ctp_mastercard";

//TODO: Fill the struct with respective fields
pub struct UnifiedAuthenticationServiceRouterData<T> {
    pub amount: FloatMajorUnit, // The type of amount that a connector accepts, for example, String, i64, f64, etc.
    pub router_data: T,
}

impl<T> From<(FloatMajorUnit, T)> for UnifiedAuthenticationServiceRouterData<T> {
    fn from((amount, item): (FloatMajorUnit, T)) -> Self {
        //Todo :  use utils to convert the amount to the type of amount that a connector accepts
        Self {
            amount,
            router_data: item,
        }
    }
}

#[derive(Debug, Serialize, PartialEq)]
pub struct UnifiedAuthenticationServicePreAuthenticateRequest {
    pub authenticate_by: String,
    pub session_id: String,
    pub source_authentication_id: String,
    pub authentication_info: Option<AuthenticationInfo>,
    pub service_details: Option<CtpServiceDetails>,
    pub customer_details: Option<CustomerDetails>,
    pub pmt_details: Option<PaymentDetails>,
    pub auth_creds: UnifiedAuthenticationServiceAuthType,
    pub transaction_details: Option<TransactionDetails>,
}

#[derive(Default, Debug, Serialize, PartialEq)]
pub struct PaymentDetails {
    pub pan: cards::CardNumber,
    pub digital_card_id: Option<String>,
    pub payment_data_type: Option<String>,
    pub encrypted_src_card_details: Option<String>,
    pub card_expiry_date: Secret<String>,
    pub cardholder_name: Option<Secret<String>>,
    pub card_token_number: Secret<String>,
    pub account_type: Option<common_enums::CardNetwork>,
}

#[derive(Default, Debug, Serialize, PartialEq)]
pub struct TransactionDetails {
    pub amount: FloatMajorUnit,
    pub currency: enums::Currency,
    pub date: Option<PrimitiveDateTime>,
    pub pan_source: Option<String>,
    pub protection_type: Option<String>,
    pub entry_mode: Option<String>,
    pub transaction_type: Option<String>,
    pub otp_value: Option<String>,
    pub three_ds_data: Option<ThreeDSData>,
}

#[derive(Default, Debug, Serialize, PartialEq)]
pub struct ThreeDSData {
    pub browser: BrowserInfo,
    pub acquirer: Acquirer,
}

#[derive(Default, Debug, Serialize, PartialEq)]
pub struct Acquirer {
    pub merchant_id: String,
    pub bin: u32,
}

#[derive(Default, Debug, Serialize, PartialEq)]
pub struct BrowserInfo {
    pub accept_header: String,
    pub screen_width: u32,
    pub screen_height: u32,
    pub java_enabled: bool,
    pub javascript_enabled: bool,
    pub language: String,
    pub user_agent: String,
    pub color_depth: u32,
    pub ip: String,
    pub tz: i32,
    pub time_zone: i8,
    pub challenge_window_size: String,
}

#[derive(Default, Debug, Serialize, PartialEq)]
pub struct AuthenticationInfo {
    pub authentication_type: Option<String>,
    pub authentication_reasons: Option<Vec<String>>,
    pub consent_received: bool,
    pub is_authenticated: bool,
    pub locale: Option<String>,
    pub supported_card_brands: Option<String>,
}

#[derive(Default, Debug, Serialize, PartialEq)]
pub struct CtpServiceDetails {
    pub service_session_ids: Option<ServiceSessionIds>,
    pub merchant_details: Option<MerchantDetails>,
}

#[derive(Default, Debug, Serialize, PartialEq)]
pub struct ServiceSessionIds {
    pub client_id: Option<String>,
    pub service_id: Option<String>,
    pub correlation_id: Option<String>,
    pub client_reference_id: Option<String>,
    pub merchant_transaction_id: Option<String>,
    pub x_src_flow_id: Option<String>,
}

#[derive(Default, Debug, Serialize, PartialEq)]
pub struct MerchantDetails {
    pub merchant_id: String,
    pub merchant_name: String,
    pub mcc: String,
    pub country_code: String,
    pub name: String,
    pub requestor_id: String,
    pub requestor_name: String,
    pub configuration_id: String,
    pub merchant_country: String,
    pub merchant_category_code: u32,
}

#[derive(Default, Debug, Serialize, PartialEq)]
pub struct Address {
    pub city: String,
    pub country: String,
    pub line1: Secret<String>,
    pub line2: Secret<String>,
    pub line3: Option<Secret<String>>,
    pub post_code: Secret<String>,
    pub state: Secret<String>,
}

#[derive(Default, Debug, Serialize, PartialEq)]
pub struct CustomerDetails {
    pub name: Secret<String>,
    pub email: Option<Secret<String>>,
    pub phone_number: Option<Secret<String>>,
    pub customer_id: String,
    #[serde(rename = "type")]
    pub customer_type: Option<String>,
    pub billing_address: Address,
    pub shipping_address: Address,
    pub wallet_account_id: Secret<String>,
    pub email_hash: Secret<String>,
    pub country_code: String,
    pub national_identifier: String,
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
pub struct UnifiedAuthenticationServiceCard {
    number: cards::CardNumber,
    expiry_month: Secret<String>,
    expiry_year: Secret<String>,
    cvc: Secret<String>,
    complete: bool,
}

impl TryFrom<&UnifiedAuthenticationServiceRouterData<&UasPreAuthenticationRouterData>>
    for UnifiedAuthenticationServicePreAuthenticateRequest
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &UnifiedAuthenticationServiceRouterData<&UasPreAuthenticationRouterData>,
    ) -> Result<Self, Self::Error> {
        let auth_type =
            UnifiedAuthenticationServiceAuthType::try_from(&item.router_data.connector_auth_type)?;
        let authentication_id = item.router_data.authentication_id.clone().ok_or(
            errors::ConnectorError::MissingRequiredField {
                field_name: "authentication_id",
            },
        )?;
        Ok(Self {
            authenticate_by: item.router_data.connector.clone(),
            session_id: authentication_id.clone(),
            source_authentication_id: authentication_id,
            authentication_info: None,
            service_details: Some(CtpServiceDetails {
                service_session_ids: item.router_data.request.service_details.clone().map(
                    |service_details| ServiceSessionIds {
                        client_id: None,
                        service_id: None,
                        correlation_id: service_details
                            .service_session_ids
                            .clone()
                            .and_then(|service_session_ids| service_session_ids.correlation_id),
                        client_reference_id: None,
                        merchant_transaction_id: service_details
                            .service_session_ids
                            .clone()
                            .and_then(|service_session_ids| {
                                service_session_ids.merchant_transaction_id
                            }),
                        x_src_flow_id: service_details
                            .service_session_ids
                            .clone()
                            .and_then(|service_session_ids| service_session_ids.x_src_flow_id),
                    },
                ),
                merchant_details: None,
            }),
            customer_details: None,
            pmt_details: None,
            auth_creds: auth_type,
            transaction_details: Some(TransactionDetails {
                amount: item.amount,
                currency: item
                    .router_data
                    .request
                    .transaction_details
                    .clone()
                    .ok_or(errors::ConnectorError::MissingRequiredField {
                        field_name: "transaction_details",
                    })?
                    .currency
                    .ok_or(errors::ConnectorError::MissingRequiredField {
                        field_name: "currency",
                    })?,
                date: None,
                pan_source: None,
                protection_type: None,
                entry_mode: None,
                transaction_type: None,
                otp_value: None,
                three_ds_data: None,
            }),
        })
    }
}

impl TryFrom<&UasPreAuthenticationRouterData>
    for UnifiedAuthenticationServicePreAuthenticateRequest
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &UasPreAuthenticationRouterData) -> Result<Self, Self::Error> {
        let auth_type = UnifiedAuthenticationServiceAuthType::try_from(&item.connector_auth_type)?;
        let authentication_id =
            item.authentication_id
                .clone()
                .ok_or(errors::ConnectorError::MissingRequiredField {
                    field_name: "authentication_id",
                })?;
        Ok(Self {
            authenticate_by: item.connector.clone(),
            session_id: authentication_id.clone(),
            source_authentication_id: authentication_id,
            authentication_info: None,
            service_details: None,
            customer_details: None,
            pmt_details: item
                .request
                .pmt_details
                .clone()
                .map(|details| PaymentDetails {
                    pan: details.pan,
                    digital_card_id: details.digital_card_id,
                    payment_data_type: details.payment_data_type,
                    encrypted_src_card_details: details.encrypted_src_card_details,
                    card_expiry_date: details.card_expiry_date,
                    cardholder_name: details.cardholder_name,
                    card_token_number: details.card_token_number,
                    account_type: details.account_type,
                }),
            auth_creds: auth_type,
            transaction_details: None,
        })
    }
}

//TODO: Fill the struct with respective fields
// Auth Struct
#[derive(Debug, Serialize, PartialEq)]
#[serde(tag = "auth_type")]
pub enum UnifiedAuthenticationServiceAuthType {
    HeaderKey {
        api_key: Secret<String>,
    },
    CertificateAuth {
        certificate: Secret<String>,
        private_key: Secret<String>,
    },
}

impl TryFrom<&ConnectorAuthType> for UnifiedAuthenticationServiceAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            ConnectorAuthType::HeaderKey { api_key } => {
                Ok(Self::HeaderKey {
                    api_key: api_key.clone(),
                })
            }
            ConnectorAuthType::CertificateAuth {
                certificate,
                private_key,
            } => Ok(Self::CertificateAuth {
                certificate: certificate.clone(),
                private_key: private_key.clone(),
            }),
            _ => Err(errors::ConnectorError::FailedToObtainAuthType.into()),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum UnifiedAuthenticationServicePreAuthenticateStatus {
    ACKSUCCESS,
    ACKFAILURE,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UnifiedAuthenticationServicePreAuthenticateResponse {
    status: UnifiedAuthenticationServicePreAuthenticateStatus,
    pub eligibility: Option<Eligibility>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "eligibility")]
pub enum Eligibility {
    None,
    TokenEligibilityResponse {
        token_eligibility_response: Box<TokenEligibilityResponse>,
    },
    ThreeDsEligibilityResponse {
        three_ds_eligibility_response: Box<ThreeDsEligibilityResponse>,
    },
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TokenEligibilityResponse {
    pub network_request_id: Option<String>,
    pub network_client_id: Option<String>,
    pub nonce: Option<String>,
    pub payment_method_details: Option<PaymentMethodDetails>,
    pub network_pan_enrollment_id: Option<String>,
    pub ignore_00_field: Option<String>,
    pub token_details: Option<TokenDetails>,
    pub network_provisioned_token_id: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PaymentMethodDetails {
    pub ignore_01_field: String,
    pub cvv2_printed_ind: String,
    pub last4: String,
    pub exp_date_printed_ind: String,
    pub payment_account_reference: String,
    pub exp_year: String,
    pub exp_month: String,
    pub verification_results: VerificationResults,
    pub enabled_services: EnabledServices,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct VerificationResults {
    pub address_verification_code: String,
    pub cvv2_verification_code: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct EnabledServices {
    pub merchant_presented_qr: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ThreeDsEligibilityResponse {
    pub three_ds_server_trans_id: String,
    pub scheme_id: Option<String>,
    pub acs_protocol_versions: Option<Vec<AcsProtocolVersion>>,
    pub ds_protocol_versions: Option<Vec<String>>,
    pub three_ds_method_data_form: ThreeDsMethodDataForm,
    pub three_ds_method_data: Option<ThreeDsMethodData>,
    pub error_details: Option<String>,
    pub is_card_found_in_2x_ranges: bool,
    pub directory_server_id: Option<String>,
}

impl ThreeDsEligibilityResponse {
    pub fn get_max_acs_protocol_version_if_available(&self) -> Option<AcsProtocolVersion> {
        let max_acs_version =
            self.acs_protocol_versions
                .as_ref()
                .and_then(|acs_protocol_versions| {
                    acs_protocol_versions
                        .iter()
                        .max_by_key(|acs_protocol_versions| acs_protocol_versions.version.clone())
                });
        max_acs_version.cloned()
    }
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct AcsProtocolVersion {
    pub version: common_utils::types::SemanticVersion,
    pub acs_info_ind: Vec<String>,
    pub three_ds_method_url: Option<String>,
    pub supported_msg_ext: Option<Vec<SupportedMsgExt>>,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct SupportedMsgExt {
    pub id: String,
    pub version: String,
}

#[derive(Clone, Serialize, Deserialize, Debug, Default)]
pub struct ThreeDsMethodDataForm {
    pub three_ds_method_data: Option<String>,
}

#[derive(Default, Clone, Serialize, Deserialize, Debug)]
pub struct ThreeDsMethodData {
    pub three_ds_method_notification_url: String,
    pub server_transaction_id: String,
}

impl<F, T>
    TryFrom<
        ResponseRouterData<
            F,
            UnifiedAuthenticationServicePreAuthenticateResponse,
            T,
            UasAuthenticationResponseData,
        >,
    > for RouterData<F, T, UasAuthenticationResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<
            F,
            UnifiedAuthenticationServicePreAuthenticateResponse,
            T,
            UasAuthenticationResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        let three_ds_eligibility_response = if let Some(Eligibility::ThreeDsEligibilityResponse {
            three_ds_eligibility_response,
        }) = item.response.eligibility
        {
            Some(three_ds_eligibility_response)
        } else {
            None
        };
        let max_acs_protocol_version = three_ds_eligibility_response
            .as_ref()
            .and_then(|response| response.get_max_acs_protocol_version_if_available());
        let maximum_supported_3ds_version = max_acs_protocol_version
            .as_ref()
            .map(|acs_protocol_version| acs_protocol_version.version.clone());
        let three_ds_method_data =
            three_ds_eligibility_response
                .as_ref()
                .and_then(|three_ds_eligibility_response| {
                    three_ds_eligibility_response
                        .three_ds_method_data_form
                        .three_ds_method_data
                        .clone()
                });
        let three_ds_method_url = max_acs_protocol_version
            .and_then(|acs_protocol_version| acs_protocol_version.three_ds_method_url);
        Ok(Self {
            response: Ok(UasAuthenticationResponseData::PreAuthentication {
                authentication_details: PreAuthenticationDetails {
                    threeds_server_transaction_id: three_ds_eligibility_response
                        .as_ref()
                        .map(|response| response.three_ds_server_trans_id.clone()),
                    maximum_supported_3ds_version: maximum_supported_3ds_version.clone(),
                    connector_authentication_id: three_ds_eligibility_response
                        .as_ref()
                        .map(|response| response.three_ds_server_trans_id.clone()),
                    three_ds_method_data,
                    three_ds_method_url,
                    message_version: maximum_supported_3ds_version,
                    connector_metadata: None,
                    directory_server_id: three_ds_eligibility_response
                        .and_then(|response| response.directory_server_id),
                },
            }),
            ..item.data
        })
    }
}

#[derive(Debug, Serialize, PartialEq)]
pub struct UnifiedAuthenticationServicePostAuthenticateRequest {
    pub authenticate_by: String,
    pub source_authentication_id: String,
    pub auth_creds: UnifiedAuthenticationServiceAuthType,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UnifiedAuthenticationServicePostAuthenticateResponse {
    pub authentication_details: AuthenticationDetails,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AuthenticationDetails {
    pub eci: Option<String>,
    pub token_details: UasTokenDetails,
    pub dynamic_data_details: Option<UasDynamicData>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UasTokenDetails {
    pub payment_token: cards::CardNumber,
    pub payment_account_reference: String,
    pub token_expiration_month: Secret<String>,
    pub token_expiration_year: Secret<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UasDynamicData {
    pub dynamic_data_value: Option<Secret<String>>,
    pub dynamic_data_type: Option<String>,
    pub ds_trans_id: Option<String>,
}

impl TryFrom<&UasPostAuthenticationRouterData>
    for UnifiedAuthenticationServicePostAuthenticateRequest
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &UasPostAuthenticationRouterData) -> Result<Self, Self::Error> {
        let auth_type = UnifiedAuthenticationServiceAuthType::try_from(&item.connector_auth_type)?;
        Ok(Self {
            authenticate_by: CTP_MASTERCARD.to_owned(),
            source_authentication_id: item.authentication_id.clone().ok_or(
                errors::ConnectorError::MissingRequiredField {
                    field_name: "authentication_id",
                },
            )?,
            auth_creds: auth_type,
        })
    }
}

impl<F, T>
    TryFrom<
        ResponseRouterData<
            F,
            UnifiedAuthenticationServicePostAuthenticateResponse,
            T,
            UasAuthenticationResponseData,
        >,
    > for RouterData<F, T, UasAuthenticationResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<
            F,
            UnifiedAuthenticationServicePostAuthenticateResponse,
            T,
            UasAuthenticationResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(UasAuthenticationResponseData::PostAuthentication {
                authentication_details: PostAuthenticationDetails {
                    eci: item.response.authentication_details.eci,
                    token_details: Some(TokenDetails {
                        payment_token: item
                            .response
                            .authentication_details
                            .token_details
                            .payment_token,
                        payment_account_reference: item
                            .response
                            .authentication_details
                            .token_details
                            .payment_account_reference,
                        token_expiration_month: item
                            .response
                            .authentication_details
                            .token_details
                            .token_expiration_month,
                        token_expiration_year: item
                            .response
                            .authentication_details
                            .token_details
                            .token_expiration_year,
                    }),
                    dynamic_data_details: item
                        .response
                        .authentication_details
                        .dynamic_data_details
                        .map(|dynamic_data| DynamicData {
                            dynamic_data_value: dynamic_data.dynamic_data_value,
                            dynamic_data_type: dynamic_data.dynamic_data_type,
                            ds_trans_id: dynamic_data.ds_trans_id,
                        }),
                    trans_status: None,
                },
            }),
            ..item.data
        })
    }
}

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
pub struct UnifiedAuthenticationServiceErrorResponse {
    pub error: String,
}
