use common_enums::{enums, Currency};
use common_utils::types::{FloatMajorUnit, StringMinorUnit};
use hyperswitch_domain_models::{
    payment_method_data::PaymentMethodData,
    router_data::{ConnectorAuthType, RouterData},
    router_flow_types::refunds::{Execute, RSync},
    router_request_types::{unified_authentication_service::{UasPreAuthenticationRequestData, UasAuthenticationResponseData, UasPostAuthenticationRequestData}, ResponseId},
    router_response_types::{PaymentsResponseData, RefundsResponseData},
    types::{PaymentsAuthorizeRouterData, RefundsRouterData},
};
use hyperswitch_interfaces::errors;
use masking::Secret;
use serde::{self, Deserialize, Serialize};

use crate::{
    types::{RefundsResponseRouterData, ResponseRouterData},
    utils::PaymentsAuthorizeRequestData,
};

//TODO: Fill the struct with respective fields
pub struct UnifiedAuthenticationServiceRouterData<T> {
    pub amount: StringMinorUnit, // The type of amount that a connector accepts, for example, String, i64, f64, etc.
    pub router_data: T,
}

impl<T> From<(StringMinorUnit, T)> for UnifiedAuthenticationServiceRouterData<T> {
    fn from((amount, item): (StringMinorUnit, T)) -> Self {
        //Todo :  use utils to convert the amount to the type of amount that a connector accepts
        Self {
            amount,
            router_data: item,
        }
    }
}


#[derive(Serialize, Deserialize, Debug)]
pub struct ServiceDetails {
    pub merchant_transaction_id: Option<String>,
    pub correlation_id: Option<String>,
    pub x_src_flow_id: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct TransactionDetails {
    pub amount: f64,
    pub currency: Currency,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct AuthCreds {
    pub auth_type: String,
    pub api_key: Secret<String>,
}


/******** just to fix error **********/
#[derive(Serialize, Deserialize, Debug)]
pub struct AuthenticationInfo{
    pub a: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CustomerDetails{
    pub a: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PaymentDetails{
    pub a: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct DeviceDetails{
    pub a: String,
}
/************************************/

#[derive(Serialize, Deserialize, Debug)]
pub struct UnifiedAuthenticationServicePreAuthenticationRequest {
    pub authenticate_by: String,
    pub session_id: String,
    pub source_authentication_id: String,
    pub authentication_info: Option<AuthenticationInfo>,
    pub service_details: Option<ServiceDetails>,
    pub customer_details: Option<CustomerDetails>,
    pub pmt_details: Option<PaymentDetails>,
    pub transaction_details: Option<TransactionDetails>,
    pub auth_creds: AuthCreds
}

#[derive(Serialize, Deserialize, Debug)]
pub struct UnifiedAuthenticationServicePreAuthenticationResponse {
    pub status: String,
    pub device_details: Option<DeviceDetails>,
    pub authenticate_by: String,
    pub eligibility: Option<String>,
}

/***************************************************************************/
#[derive(Serialize, Deserialize, Debug)]
pub struct VerificationData {
    #[serde(rename = "type")]
    pub verification_type: String,
    pub entity: u32,
    pub method: u32,
    pub results: u32,
    pub timestamp: u64,
    pub data: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct UnifiedAuthenticationServicePostAuthenticatioRequest{
    pub authenticate_by: String,
    pub source_authentication_id: String,
    pub auth_creds: AuthCreds,
    pub consumer_account_details: Option<PaymentDetails>,
    pub verification_data: Option<VerificationData>,
    pub cryptogram_type: Option<String>,
    pub x_src_flow_id: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct TokenDetails {
    pub payment_token: String,
    pub payment_account_reference: String,
    pub token_expiration_month: String,
    pub token_expiration_year: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct DynamicDataDetails {
    pub dynamic_data_value: Option<String>,
    #[serde(rename = "type")]
    pub dynamic_data_type: String,
    pub ds_trans_id:Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct AuthenticationDetails {
    pub eci: Option<String>,
    pub token_details: TokenDetails,
    pub dynamic_data_details: Option<DynamicDataDetails>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct UnifiedAuthenticationServicePostAuthenticatioResponse {
    pub device_details: Option<serde_json::Value>,
    pub authentication_metadata: Option<serde_json::Value>,
    pub authentication_details: AuthenticationDetails,
}
/***************************************************************************/

#[derive(Serialize, Deserialize, Debug)]
pub struct UnifiedAuthenticationServiceConfirmationRequest {
    pub authenticate_by: String,
    pub source_authentication_id: String,
    pub x_src_flow_id: Option<String>,
    pub transaction_amount: Option<FloatMajorUnit>,
    pub transaction_currency:  Option<String>,
    pub checkout_event_type:  Option<String>,
    pub checkout_event_status: Option<String>,
    pub confirmation_status: Option<String>,
    pub confirmation_reason: Option<String>,
    pub confirmation_timestamp: Option<String>,
    pub network_authorization_code: Option<String>,
    pub network_transaction_identifier: Option<String>,
    pub correlation_id: Option<String>,
    pub merchant_transaction_id: Option<String>,
    pub auth_creds: Option<AuthCreds>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct UnifiedAuthenticationServiceConfirmationResponse {
    pub status: String
}

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Serialize, PartialEq)]
pub struct UnifiedAuthenticationServicePaymentsRequest {
    amount: StringMinorUnit,
    card: UnifiedAuthenticationServiceCard,
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
pub struct UnifiedAuthenticationServiceCard {
    number: cards::CardNumber,
    expiry_month: Secret<String>,
    expiry_year: Secret<String>,
    cvc: Secret<String>,
    complete: bool,
}

impl TryFrom<&UnifiedAuthenticationServiceRouterData<&PaymentsAuthorizeRouterData>>
    for UnifiedAuthenticationServicePaymentsRequest
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &UnifiedAuthenticationServiceRouterData<&PaymentsAuthorizeRouterData>,
    ) -> Result<Self, Self::Error> {
        match item.router_data.request.payment_method_data.clone() {
            PaymentMethodData::Card(req_card) => {
                let card = UnifiedAuthenticationServiceCard {
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
pub struct UnifiedAuthenticationServiceAuthType {
    pub(super) api_key: Secret<String>,
}

impl TryFrom<&ConnectorAuthType> for UnifiedAuthenticationServiceAuthType {
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
pub enum UnifiedAuthenticationServicePaymentStatus {
    Succeeded,
    Failed,
    #[default]
    Processing,
}

impl From<UnifiedAuthenticationServicePaymentStatus> for common_enums::AttemptStatus {
    fn from(item: UnifiedAuthenticationServicePaymentStatus) -> Self {
        match item {
            UnifiedAuthenticationServicePaymentStatus::Succeeded => Self::Charged,
            UnifiedAuthenticationServicePaymentStatus::Failed => Self::Failure,
            UnifiedAuthenticationServicePaymentStatus::Processing => Self::Authorizing,
        }
    }
}

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UnifiedAuthenticationServicePaymentsResponse {
    status: UnifiedAuthenticationServicePaymentStatus,
    id: String,
}

impl<F, T>
    TryFrom<
        ResponseRouterData<
            F,
            UnifiedAuthenticationServicePaymentsResponse,
            T,
            PaymentsResponseData,
        >,
    > for RouterData<F, T, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<
            F,
            UnifiedAuthenticationServicePaymentsResponse,
            T,
            PaymentsResponseData,
        >,
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
                charge_id: None,
            }),
            ..item.data
        })
    }
}

//TODO: Fill the struct with respective fields
// REFUND :
// Type definition for RefundRequest
#[derive(Default, Debug, Serialize)]
pub struct UnifiedAuthenticationServiceRefundRequest {
    pub amount: StringMinorUnit,
}

impl<F> TryFrom<&UnifiedAuthenticationServiceRouterData<&RefundsRouterData<F>>>
    for UnifiedAuthenticationServiceRefundRequest
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &UnifiedAuthenticationServiceRouterData<&RefundsRouterData<F>>,
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

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
pub struct UnifiedAuthenticationServiceErrorResponse {
    pub status_code: u16,
    pub code: String,
    pub message: String,
    pub reason: Option<String>,
}


impl TryFrom<&UnifiedAuthenticationServiceRouterData<&UasPreAuthenticationRequestData>> 
    for  UnifiedAuthenticationServicePreAuthenticationRequest {
        type Error = error_stack::Report<errors::ConnectorError>;

        fn try_from(
            value: &UnifiedAuthenticationServiceRouterData<&types::authentication::PreAuthNRouterData>,
        ) -> Result<Self, Self::Error> {
            let router_data = value.router_data;
            Ok(Self { 
                authenticate_by: (), 
                session_id: (), 
                source_authentication_id: (), 
                authentication_info: (), 
                service_details: (), 
                customer_details: (), 
                pmt_details: (), 
                transaction_details: (), 
                auth_creds: () 
            })
        }
    }

