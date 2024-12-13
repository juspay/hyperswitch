use common_enums::enums;
use common_utils::types::StringMajorUnit;
use hyperswitch_domain_models::{
    payment_method_data::{AmazonPayWalletData, PaymentMethodData, WalletData as WalletDataPaymentMethod},
    router_data::{ConnectorAuthType, RouterData},
    router_flow_types::refunds::{Execute, RSync},
    router_request_types::ResponseId,
    router_response_types::{PaymentsResponseData, RefundsResponseData},
    types::{PaymentsAuthorizeRouterData, RefundsRouterData, PaymentsSessionRouterData},
};
use hyperswitch_interfaces::errors;
use masking::Secret;
use serde::{Deserialize, Serialize};

use crate::{
    types::{RefundsResponseRouterData, ResponseRouterData},
    utils,
    utils::PaymentsAuthorizeRequestData,
};

use crates::api_models::PaymentsRequest;

//TODO: Fill the struct with respective fields
pub struct AmazonpayRouterData<T> {
    pub amount: StringMajorUnit, // The type of amount that a connector accepts, for example, String, i64, f64, etc.
    pub router_data: T,
}

impl<T> From<(StringMajorUnit, T)> for AmazonpayRouterData<T> {
    fn from((amount, item): (StringMajorUnit, T)) -> Self {
        Self {
            amount,
            router_data: item,
        }
    }
}

#[derive(Default, Debug, Serialize, PartialEq)]
pub struct AmazonpayFinalizeRequest {
    charge_amount: AmazonpayChargeAmount,
    total_order_amount: Option<AmazonpayTotalOrderAmount>,
    shipping_address: AmazonpayShippingAddress,
    payment_intent: AmazonpayPaymentIntent
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AmazonpayChargeAmount {
    amount: StringMajorUnit,
    currency_code: common_enums::Currency,
}

#[derive(Default, Debug, Serialize, PartialEq)]
pub struct AmazonpayTotalOrderAmount {
    amount: StringMajorUnit,
    currency_code: common_enums::Currency,
    can_handle_pending_authorization: Option<bool>,
    supplementary_data: Option<String>
}

#[derive(Default, Debug, Serialize, PartialEq)]
pub struct AmazonpayShippingAddress {
    name: Option<String>,
    address_line_1: Option<String>,
    address_line_2: Option<String>,
    address_line_3: Option<String>,
    city: Option<String>,
    country: Option<String>,
    district: Option<String>,
    state_or_region: Option<String>,
    postal_code: Option<String>,
    country_code: Option<common_enums::CountryAlpha2>,
    phone_number: Option<String>
}

#[derive(Default, Debug, Serialize, PartialEq)]
pub enum AmazonpayPaymentIntent {
    Authorize,
    AuthorizeWithCapture,
    #[default]
    Confirm
}


impl TryFrom<&AmazonpayRouterData<&PaymentsSessionRouterData>> for AmazonpayFinalizeRequests {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &AmazonpayRouterData<&PaymentsSessionRouterData>,
    ) -> Result<Self, Self::Error> {
        // let charge_amount = AmazonpayChargeAmount {
        //     amount: item.amount.clone(),
        //     currency_code: common_enums::Currency::USD
        // };
        // let total_order_amount = Some(AmazonpayTotalOrderAmount {
        //     amount: 
        // })
        // let address_line_1 = Some(item.router_data.address);
        // let shipping_address = AmazonpayShippingAddress {
        //     name: Some(item.router_data.connector_customer),
        //     address_line_1: Some(item.router_data.address.)
        // };
        
    }
} 


//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Serialize, PartialEq)]
pub struct AmazonpayPaymentsRequest {
    charge_amount: AmazonpayChargeAmount,
    charge_permission_id: AmazonPayWalletData,
    capture_now: Option<bool>
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
pub struct AmazonpayCard {
    number: cards::CardNumber,
    expiry_month: Secret<String>,
    expiry_year: Secret<String>,
    cvc: Secret<String>,
    complete: bool,
}

impl TryFrom<&AmazonpayRouterData<&PaymentsAuthorizeRouterData>> for AmazonpayPaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &AmazonpayRouterData<&PaymentsAuthorizeRouterData>,
    ) -> Result<Self, Self::Error> {
        match item.router_data.request.payment_method_data.clone() {
            PaymentMethodData::Wallet(ref wallet_data) => match wallet_data {
                WalletDataPaymentMethod::AmazonPay(ref req_wallet) => {
                    let charge_amount = AmazonpayChargeAmount {
                        amount: item.amount.clone(),
                        currency_code: common_enums::Currency::USD,
                    };
                    let charge_permission_id = AmazonPayWalletData {
                        charge_permission_id: item.router_data.connector_request_reference_id.clone(),
                    };
                    let capture_now: Option<bool> = Some(item.router_data.request.is_auto_capture()?);
                    Ok(Self {
                        charge_amount,
                        charge_permission_id,
                        capture_now
                    })
                }
                WalletDataPaymentMethod::AliPayQr(_)
                | WalletDataPaymentMethod::AliPayRedirect(_)
                | WalletDataPaymentMethod::AliPayHkRedirect(_)
                | WalletDataPaymentMethod::MomoRedirect(_)
                | WalletDataPaymentMethod::KakaoPayRedirect(_)
                | WalletDataPaymentMethod::GoPayRedirect(_)
                | WalletDataPaymentMethod::GcashRedirect(_)
                | WalletDataPaymentMethod::ApplePay(_)
                | WalletDataPaymentMethod::ApplePayRedirect(_)
                | WalletDataPaymentMethod::ApplePayThirdPartySdk(_)
                | WalletDataPaymentMethod::DanaRedirect {}
                | WalletDataPaymentMethod::GooglePay(_)
                | WalletDataPaymentMethod::GooglePayRedirect(_)
                | WalletDataPaymentMethod::GooglePayThirdPartySdk(_)
                | WalletDataPaymentMethod::MbWayRedirect(_)
                | WalletDataPaymentMethod::MobilePayRedirect(_)
                | WalletDataPaymentMethod::PaypalRedirect(_)
                | WalletDataPaymentMethod::PaypalSdk(_)
                | WalletDataPaymentMethod::Paze(_)
                | WalletDataPaymentMethod::SamsungPay(_)
                | WalletDataPaymentMethod::TwintRedirect {}
                | WalletDataPaymentMethod::VippsRedirect {}
                | WalletDataPaymentMethod::TouchNGoRedirect(_)
                | WalletDataPaymentMethod::WeChatPayRedirect(_)
                | WalletDataPaymentMethod::WeChatPayQr(_)
                | WalletDataPaymentMethod::CashappQr(_)
                | WalletDataPaymentMethod::SwishQr(_)
                | WalletDataPaymentMethod::Mifinity(_) => {
                    Err(errors::ConnectorError::NotImplemented(
                        utils::get_unimplemented_payment_method_error_message("amazonpay"),
                    )
                    .into())
                }
            }
            _ => Err(errors::ConnectorError::NotImplemented("Payment method".to_string()).into()),
        }
    }
}

//TODO: Fill the struct with respective fields
// Auth Struct
pub struct AmazonpayAuthType {
    pub(super) public_key: Secret<String>,
    pub(super) private_key: Secret<String>,
}

impl TryFrom<&ConnectorAuthType> for AmazonpayAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            ConnectorAuthType::CertificateAuth { 
                certificate,
                private_key, 
            } => Ok(Self {
                public_key: certificate.to_owned(),
                private_key: private_key.to_owned()
            }),
            _ => Err(errors::ConnectorError::FailedToObtainAuthType.into()),
        }
    }
}
// PaymentsResponse
//TODO: Append the remaining status flags
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub enum AmazonpayPaymentStatus {
    #[default]
    AuthorizationInitiated,
    Authorized,
    Canceled,
    Captured,
    CaptureInitiated,
    Declined
}

impl From<AmazonpayPaymentStatus> for common_enums::AttemptStatus {
    fn from(item: AmazonpayPaymentStatus) -> Self {
        match item {
            AmazonpayPaymentStatus::AuthorizationInitiated => Self::Pending,
            AmazonpayPaymentStatus::Authorized => Self::Authorized,
            AmazonpayPaymentStatus::Canceled => Self::Voided,
            AmazonpayPaymentStatus::Captured => Self::Charged,
            AmazonpayPaymentStatus::CaptureInitiated => Self::CaptureInitiated,
            AmazonpayPaymentStatus::Declined => Self::AuthorizationFailed,  // handle CaptureFailed
        }
    }
}

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AmazonpayPaymentsResponse {
    charge_id: String,
    charge_amount: AmazonpayChargeAmount,
    charge_permission_id: AmazonPayWalletData,
    capture_amount: Option<AmazonpayCaptureAmount>,
    refunded_amount: AmazonpayRefundAmount,
    soft_descriptor: Option<String>,
    provider_metadata: AmazonpayProviderMetadata,
    converted_amount: Option<AmazonpayConvertedAmount>,
    conversion_rate: Option<f64>,
    channel: Option<String>,  // not sure
    charge_initiator: Option<String>,
    status_details: AmazonpayStatusDetails,
    creation_timestamp: String,
    expiration_timestamp: String,
    release_environment: AmazonpayReleaseEnvironment,
    merchant_metadata: AmazonpayMerchantMetadata,
    platform_id: Option<String>,
    web_checkout_details: Option<String>,  // not sure
    disburement_details: Option<String>,  // not sure
    payment_method: Option<String>  // not sure
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AmazonpayCaptureAmount {
    amount: StringMajorUnit,
    currency_code: common_enums::Currency
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AmazonpayRefundAmount {
    amount: StringMajorUnit,
    currency_code: common_enums::Currency
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AmazonpayProviderMetadata {
    provider_reference_id: Option<String>
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AmazonpayConvertedAmount {
    amount: StringMajorUnit,
    currency_code: common_enums::Currency
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AmazonpayStatusDetails {
    state: AmazonpayPaymentStatus,
    reason_code: Option<String>,
    reason_description: Option<String>,
    last_updated_timestamp: String,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AmazonpayReleaseEnvironment {
    #[default]
    Sandbox,
    Live
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AmazonpayMerchantMetadata {
    merchant_reference_id: Option<String>,
    merchant_store_name: Option<String>,
    note_to_buyer: Option<String>,
    custom_information: Option<String>,
}

impl<F, T> TryFrom<ResponseRouterData<F, AmazonpayPaymentsResponse, T, PaymentsResponseData>>
    for RouterData<F, T, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, AmazonpayPaymentsResponse, T, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            status: common_enums::AttemptStatus::from(item.response.status_details.state),
            response: Ok(PaymentsResponseData::TransactionResponse {
                resource_id: ResponseId::ConnectorTransactionId(item.response.charge_id),
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
pub struct AmazonpayRefundRequest {
    pub amount: StringMajorUnit,
}

impl<F> TryFrom<&AmazonpayRouterData<&RefundsRouterData<F>>> for AmazonpayRefundRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &AmazonpayRouterData<&RefundsRouterData<F>>) -> Result<Self, Self::Error> {
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
pub struct AmazonpayErrorResponse {
    pub status_code: u16,
    pub code: String,
    pub message: String,
    pub reason: Option<String>,
}
