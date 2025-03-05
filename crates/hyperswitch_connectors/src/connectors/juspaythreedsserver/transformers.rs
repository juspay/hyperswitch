use common_enums::enums;
use common_utils::types::StringMinorUnit;
use hyperswitch_domain_models::{
    payment_method_data::PaymentMethodData,
    router_data::{ConnectorAuthType, RouterData},
    router_flow_types::refunds::{Execute, RSync},
    router_request_types::ResponseId,
    router_response_types::{PaymentsResponseData, RefundsResponseData},
    types::{PaymentsAuthorizeRouterData, RefundsRouterData},
};
use hyperswitch_interfaces::errors;
use masking::Secret;
use serde::{Deserialize, Serialize};

use crate::{
    types::{RefundsResponseRouterData, ResponseRouterData},
    utils::PaymentsAuthorizeRequestData,
};

//TODO: Fill the struct with respective fields
pub struct JuspaythreedsserverRouterData<T> {
    pub amount: StringMinorUnit, // The type of amount that a connector accepts, for example, String, i64, f64, etc.
    pub router_data: T,
}

impl<T> From<(StringMinorUnit, T)> for JuspaythreedsserverRouterData<T> {
    fn from((amount, item): (StringMinorUnit, T)) -> Self {
        //Todo :  use utils to convert the amount to the type of amount that a connector accepts
        Self {
            amount,
            router_data: item,
        }
    }
}

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Serialize, PartialEq)]
pub struct JuspaythreedsserverPaymentsRequest {
    amount: StringMinorUnit,
    card: JuspaythreedsserverCard,
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
pub struct JuspaythreedsserverCard {
    number: cards::CardNumber,
    expiry_month: Secret<String>,
    expiry_year: Secret<String>,
    cvc: Secret<String>,
    complete: bool,
}

impl TryFrom<&JuspaythreedsserverRouterData<&PaymentsAuthorizeRouterData>>
    for JuspaythreedsserverPaymentsRequest
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &JuspaythreedsserverRouterData<&PaymentsAuthorizeRouterData>,
    ) -> Result<Self, Self::Error> {
        match item.router_data.request.payment_method_data.clone() {
            PaymentMethodData::Card(req_card) => {
                let card = JuspaythreedsserverCard {
                    number: req_card.card_number,
                    expiry_month: req_card.card_exp_month,
                    expiry_year: req_card.card_exp_year,
                    cvc: req_card.card_cvc,
                    complete: item.router_data.request.is_auto_capture()?,
                };
                Ok(Self {
                    amount: item.amount.clone(),
                    card,
                })
            }
            _ => Err(errors::ConnectorError::NotImplemented("Payment method".to_string()).into()),
        }
    }
}

//TODO: Fill the struct with respective fields
// Auth Struct
pub struct JuspaythreedsserverAuthType {
    pub(super) api_key: Secret<String>,
}

impl TryFrom<&ConnectorAuthType> for JuspaythreedsserverAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            ConnectorAuthType::HeaderKey { api_key } => Ok(Self {
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
pub enum JuspaythreedsserverPaymentStatus {
    Succeeded,
    Failed,
    #[default]
    Processing,
}

impl From<JuspaythreedsserverPaymentStatus> for common_enums::AttemptStatus {
    fn from(item: JuspaythreedsserverPaymentStatus) -> Self {
        match item {
            JuspaythreedsserverPaymentStatus::Succeeded => Self::Charged,
            JuspaythreedsserverPaymentStatus::Failed => Self::Failure,
            JuspaythreedsserverPaymentStatus::Processing => Self::Authorizing,
        }
    }
}

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct JuspaythreedsserverPaymentsResponse {
    status: JuspaythreedsserverPaymentStatus,
    id: String,
}

impl<F, T>
    TryFrom<ResponseRouterData<F, JuspaythreedsserverPaymentsResponse, T, PaymentsResponseData>>
    for RouterData<F, T, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, JuspaythreedsserverPaymentsResponse, T, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            status: common_enums::AttemptStatus::from(item.response.status),
            response: Ok(PaymentsResponseData::TransactionResponse {
                resource_id: ResponseId::ConnectorTransactionId(item.response.id),
                redirection_data: Box::new(None),
                mandate_reference: Box::new(None),
                connector_metadata: None,
                network_txn_id: None,
                connector_response_reference_id: None,
                incremental_authorization_allowed: None,
                charges: None,
            }),
            ..item.data
        })
    }
}

//TODO: Fill the struct with respective fields
// REFUND :
// Type definition for RefundRequest
#[derive(Default, Debug, Serialize)]
pub struct JuspaythreedsserverRefundRequest {
    pub amount: StringMinorUnit,
}

impl<F> TryFrom<&JuspaythreedsserverRouterData<&RefundsRouterData<F>>>
    for JuspaythreedsserverRefundRequest
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &JuspaythreedsserverRouterData<&RefundsRouterData<F>>,
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

impl TryFrom<RefundsResponseRouterData<Execute, RefundResponse>> for RefundsRouterData<Execute> {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: RefundsResponseRouterData<Execute, RefundResponse>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(RefundsResponseData {
                connector_refund_id: item.response.id.to_string(),
                refund_status: enums::RefundStatus::from(item.response.status),
            }),
            ..item.data
        })
    }
}

impl TryFrom<RefundsResponseRouterData<RSync, RefundResponse>> for RefundsRouterData<RSync> {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: RefundsResponseRouterData<RSync, RefundResponse>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(RefundsResponseData {
                connector_refund_id: item.response.id.to_string(),
                refund_status: enums::RefundStatus::from(item.response.status),
            }),
            ..item.data
        })
    }
}

#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
pub struct JuspaythreedsserverErrorResponse {
    pub status_code: u16,
    pub code: String,
    pub message: String,
    pub reason: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct VersionReq {
    pub card_number: cards::CardNumber,
    pub scheme_id: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct VersionRes {
    pub three_ds_server_trans_id: String,
    pub card_ranges: Vec<CardRange>,
    pub error_details: Option<ErrorDetails>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CardRange {
    pub scheme_id: Option<String>,
    pub three_ds_method_url: Option<String>,
    pub acs_end_protocol_version: Option<String>,
    pub acs_info_ind: Option<Vec<String>>,
    pub acs_start_protocol_version: Option<String>,
    pub action_ind: Option<String>,
    pub ds_start_protocol_version: Option<String>,
    pub ds_end_protocol_version: Option<String>,
    pub start_range: String,
    pub end_range: String,
}


/// Authentication
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AReq {
    pub three_ds_server_trans_id: String,
    pub device_channel: String,
    pub message_category: String,
    pub preferred_protocol_version: Option<String>,
    pub enforce_preferred_protocol_version: bool,
    pub three_ds_comp_ind: Option<String>,
    pub three_ds_requestor: ThreeDSRequestor,
    pub cardholder_account: CardholderAccount,
    pub cardholder: Cardholder,
    pub purchase: Purchase,
    pub acquirer: Acquirer,
    pub merchant: Merchant,
    pub message_extension: Option<Vec<MessageExtension>>,
    pub sdk_information: Option<SdkInformation>,
    pub browser_information: Option<BrowserInformation>,
    pub device_render_options: Option<DeviceRenderOptions>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ARes {
    pub three_ds_server_trans_id: Option<String>,
    pub acs_url: Option<String>,
    pub trans_status: Option<ThreedsServerTransStatus>,
    pub acs_challenge_mandated: Option<String>,
    pub authentication_request: Option<AuthenticationRequest>,
    pub authentication_response: Option<AuthenticationResponse>,
    pub purchase_date: Option<String>,
    pub challenge_request: Option<ChallengeRequest>,
    pub base64_encoded_challenge_request: Option<String>,
    pub error_details: Option<ErrorDetails>,
    pub authentication_value: Secret<String>,
    pub eci: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AuthenticationRequest {
    pub message_type: Option<String>,
    pub three_ds_comp_ind: Option<String>,
    pub three_ds_requestor_authentication_info: Option<ThreeDSRequestorAuthenticationInfo>,
    pub three_ds_requestor_challenge_ind: Option<String>,
    pub three_ds_requestor_id: Option<String>,
    pub three_ds_requestor_name: Option<String>,
    pub three_ds_requestor_authentication_ind: Option<String>,
    pub three_ds_requestor_prior_authentication_info: Option<ThreeDSRequestorPriorAuthenticationInfo>,
    pub three_ds_requestor_url: Option<String>,
    pub three_ds_server_ref_number: Option<String>,
    pub three_ds_server_operator_id: Option<String>,
    pub three_ds_server_trans_id: Option<String>,
    pub three_ds_server_url: Option<String>,
    pub acct_type: Option<String>,
    pub acquirer_bin: Option<String>,
    pub acquirer_merchant_id: Option<String>,
    pub addr_match: Option<String>,
    pub browser_accept_header: Option<String>,
    pub browser_ip: Option<String>,
    pub browser_java_enabled: Option<bool>,
    pub browser_language: Option<String>,
    pub browser_color_depth: Option<String>,
    pub browser_screen_height: Option<String>,
    pub browser_screen_width: Option<String>,
    pub browser_tz: Option<String>,
    pub browser_user_agent: Option<String>,
    pub card_expiry_date: String,
    pub acct_info: Option<AcctInfo>,
    pub acct_number: cards::CardNumber,
    pub bill_addr_city: Option<String>,
    pub bill_addr_country: Option<String>,
    pub bill_addr_line1: Option<String>,
    pub bill_addr_line2: Option<String>,
    pub bill_addr_line3: Option<String>,
    pub bill_addr_post_code: Option<String>,
    pub bill_addr_state: Option<String>,
    pub email: Option<String>,
    pub home_phone: Option<Phone>,
    pub mobile_phone: Option<Phone>,
    pub cardholder_name: Option<String>,
    pub ship_addr_city: Option<String>,
    pub ship_addr_country: Option<String>,
    pub ship_addr_line1: Option<String>,
    pub ship_addr_line2: Option<String>,
    pub ship_addr_line3: Option<String>,
    pub ship_addr_post_code: Option<String>,
    pub ship_addr_state: Option<String>,
    pub work_phone: Option<Phone>,
    pub device_channel: Option<String>,
    pub device_render_options: Option<DeviceRenderOptions>,
    pub pay_token_ind: Option<bool>,
    pub mcc: Option<String>,
    pub merchant_country_code: Option<String>,
    pub merchant_risk_indicator: Option<MerchantRiskIndicator>,
    pub message_category: Option<String>,
    pub message_extension: Option<Vec<MessageExtension>>,
    pub message_version: Option<String>,
    pub purchase_amount: String,
    pub purchase_exponent: Option<String>,
    pub purchase_currency: Option<String>,
    pub purchase_date: Option<String>,
    pub recurring_expiry: Option<String>,
    pub recurring_frequency: Option<String>,
    pub sdk_app_id: Option<String>,
    pub sdk_ephem_pub_key: Option<SdkEphemPubKey>,
    pub sdk_reference_number: Option<String>,
    pub sdk_trans_id: Option<String>,
    pub sdk_max_timeout: Option<String>,
    pub trans_type: String,
    pub notification_url: Option<String>,
    pub broad_info: Option<BroadInfo>,
    pub sdk_enc_data: Option<String>,
}


#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AuthenticationResponse {
    pub message_type: String,
    pub three_ds_server_trans_id: String,
    pub acs_trans_id: String,
    pub acs_reference_number: String,
    pub acs_operator_id: Option<String>,
    pub acs_rendering_type: Option<AcsRenderingType>,
    pub acs_url: Option<String>,
    pub acs_signed_content: Option<String>,
    pub authentication_type: Option<String>,
    pub acs_challenge_mandated: Option<String>,
    pub authentication_value: Secret<String>,
    pub ds_reference_number: String,
    pub ds_trans_id: String,
    pub eci: Option<String>,
    pub message_version: String,
    pub sdk_trans_id: Option<String>,
    pub trans_status: Option<ThreedsServerTransStatus>,
    pub trans_status_reason: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ChallengeRequest {
    pub message_type: String,
    pub three_ds_server_trans_id: String,
    pub acs_trans_id: String,
    pub challenge_window_size: String,
    pub message_version: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DeviceRenderOptions {
    pub sdk_interface: String,
    pub sdk_ui_type: Vec<String>,
    pub sdk_authentication_type: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SdkEphemPubKey {
    pub kty: String,
    pub crv: String,
    pub x: String,
    pub y: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AcsRenderingType {
    pub acs_interface: String,
    pub acs_ui_template: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct BroadInfo {
    pub message: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AcctInfo {
    pub ch_acc_age_ind: String,
    pub ch_acc_date: String,
    pub ch_acc_change_ind: String,
    pub ch_acc_change: String,
    pub ch_acc_pw_change_ind: String,
    pub ch_acc_pw_change: String,
    pub ship_address_usage_ind: String,
    pub ship_address_usage: String,
    pub txn_activity_day: String,
    pub txn_activity_year: String,
    pub provision_attempts_day: String,
    pub nb_purchase_account: String,
    pub suspicious_acc_activity: String,
    pub ship_name_indicator: String,
    pub payment_acc_ind: String,
    pub payment_acc_age: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ThreeDSRequestorAuthenticationInfo {
    pub three_ds_req_auth_method: String,
    pub three_ds_req_auth_timestamp: String,
    pub three_ds_req_auth_data: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ThreeDSRequestorPriorAuthenticationInfo {
    pub three_ds_req_prior_ref: String,
    pub three_ds_req_prior_auth_method: String,
    pub three_ds_req_prior_auth_timestamp: String,
    pub three_ds_req_prior_auth_data: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MessageExtensionData {
    pub ch_acc_req_id: String,
    pub auth_pay_cred_status: String,
    pub auth_pay_process_req_ind: String,
    pub daf_advice: String,
    pub version: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MessageExtension {
    pub name: String,
    pub id: String,
    pub criticality_indicator: bool,
    #[serde(rename = "data")]
    pub data_field: MessageExtensionData,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MerchantRiskIndicator {
    pub ship_indicator: String,
    pub delivery_timeframe: String,
    pub delivery_email_address: String,
    pub reorder_items_ind: String,
    pub pre_order_purchase_ind: String,
    pub pre_order_date: String,
    pub gift_card_amount: String,
    pub gift_card_curr: String,
    pub gift_card_count: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Phone {
    pub cc: String,
    pub subscriber: String,
}


#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ResultsRequest {
    pub three_ds_server_trans_id: String,
    pub acs_trans_id: Option<String>,
    pub acs_rendering_type: Option<AcsRenderingType>,
    pub authentication_method: Option<String>,
    pub authentication_type: Option<String>,
    pub authentication_value: Secret<String>,
    pub ds_trans_id: Option<String>,
    pub eci: Option<String>,
    pub interaction_counter: Option<String>,
    pub message_category: Option<String>,
    pub message_type: Option<String>,
    pub message_version: Option<String>,
    pub trans_status: ThreedsServerTransStatus,
    pub challenge_cancel: Option<String>,
    pub trans_status_reason: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub enum ThreedsServerTransStatus {
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

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ResultsResponse {
    pub three_ds_server_trans_id: String,
    pub acs_trans_id: Option<String>,
    pub ds_trans_id: Option<String>,
    pub message_type: Option<String>,
    pub message_version: Option<String>,
    pub results_status: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ErrorDetails {
    pub three_ds_server_trans_id: Option<String>,
    pub acs_trans_id: Option<String>,
    pub ds_trans_id: Option<String>,
    pub sdk_trans_id: Option<String>,
    pub error_code: String,
    pub error_component: String,
    pub error_description: String,
    pub error_detail: Option<String>,
}


#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ThreeDSRequestor {
    pub three_ds_requestor_authentication_ind: String,
    pub three_ds_requestor_authentication_info: ThreeDSRequestorAuthenticationInfo,
    pub three_ds_requestor_challenge_ind: String,
}


#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CardholderAccount {
    pub acct_type: String,
    pub card_expiry_date: String,
    pub acct_info: Option<AcctInfo>,
    pub scheme_id: Option<String>,
    pub acct_number: cards::CardNumber,
    pub pay_token_ind: Option<bool>,
    pub card_security_code: Secret<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Cardholder {
    pub addr_match: Option<String>,
    pub bill_addr_city: Option<String>,
    pub bill_addr_country: Option<String>,
    pub bill_addr_line1: Option<String>,
    pub bill_addr_line2: Option<String>,
    pub bill_addr_line3: Option<String>,
    pub bill_addr_post_code: Option<String>,
    pub bill_addr_state: Option<String>,
    pub email: Secret<String>,
    pub home_phone: Phone,
    pub mobile_phone: Phone,
    pub work_phone: Phone,
    pub cardholder_name: Secret<String>,
    pub ship_addr_city: Option<String>,
    pub ship_addr_country: Option<String>,
    pub ship_addr_line1: Option<String>,
    pub ship_addr_line2: Option<String>,
    pub ship_addr_line3: Option<String>,
    pub ship_addr_post_code: Option<String>,
    pub ship_addr_state: Option<String>,
}


#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Purchase {
    pub purchase_instal_data: Option<i32>,
    pub merchant_risk_indicator: Option<MerchantRiskIndicator>,
    pub purchase_amount: i32,
    pub purchase_currency: String,
    pub purchase_exponent: i32,
    pub purchase_date: String,
    pub recurring_expiry: Option<String>,
    pub recurring_frequency: Option<String>,
    pub trans_type: String,
    pub recurring_amount: Option<i32>,
    pub recurring_currency: Option<String>,
    pub recurring_exponent: Option<i32>,
    pub recurring_date: Option<String>,
    pub amount_ind: Option<String>,
    pub frequency_ind: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Acquirer {
    pub acquirer_bin: String,
    pub acquirer_merchant_id: String,
}


#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct BrowserInformation {
    pub browser_accept_header: String,
    pub browser_ip: String,
    pub browser_language: String,
    pub browser_color_depth: String,
    pub browser_screen_height: i32,
    pub browser_screen_width: i32,
    pub browser_tz: i32,
    pub browser_user_agent: String,
    pub challenge_window_size: String,
    pub browser_java_enabled: bool,
    pub browser_javascript_enabled: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SdkInformation {
    pub sdk_app_id: String,
    pub sdk_enc_data: String,
    pub sdk_ephem_pub_key: SdkEphemPubKey,
    pub sdk_max_timeout: Option<i32>,
    pub sdk_reference_number: String,
    pub sdk_trans_id: String,
    pub sdk_type: Option<String>,
    pub default_sdk_type: Option<DefaultSdkType>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DefaultSdkType {
    pub sdk_variant: String,
    pub wrapped_ind: String,
}


#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Merchant {
    pub mcc: String,
    pub merchant_country_code: String,
    pub three_ds_requestor_id: String,
    pub three_ds_requestor_name: String,
    pub merchant_name: String,
    pub notification_url: String,
    pub results_response_notification_url: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Jpp3DssFinalRequest {
    pub three_ds_server_trans_id: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Jpp3DssFinalResponse {
    pub three_ds_server_trans_id: Option<String>,
    pub trans_status: Option<ThreedsServerTransStatus>,
    pub authentication_value: Secret<String>,
    pub eci: Option<String>,
    pub results_request: Option<ResultsRequest>,
    pub results_response: Option<ResultsResponse>,
    pub error_details: Option<ErrorDetails>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Jpp3DssChallengeReqType {
    pub md: Option<String>,
    pub pa_req: Option<String>,
    pub term_url: Option<String>,
    pub c_req: Option<String>,
    pub creq: Option<String>,
}
