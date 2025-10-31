use common_enums::enums;
use common_utils::{pii::Email, request::Method, types::FloatMajorUnit};
use hyperswitch_domain_models::{
    payment_method_data::PaymentMethodData,
    router_data::{AccessToken, ConnectorAuthType, RouterData},
    router_flow_types::refunds::{Execute, RSync},
    router_request_types::ResponseId,
    router_response_types::{PaymentsResponseData, RedirectForm, RefundsResponseData},
    types::{PaymentsAuthorizeRouterData, RefundsRouterData},
};
use hyperswitch_interfaces::errors;
use masking::Secret;
use serde::{Deserialize, Serialize};

use crate::{
    types::{self, RefundsResponseRouterData, ResponseRouterData},
    utils::{
        self, AddressDetailsData, BrowserInformationData, PaymentsAuthorizeRequestData,
        RouterData as _,
    },
};

pub struct FlexitiRouterData<T> {
    pub amount: FloatMajorUnit, // The type of amount that a connector accepts, for example, String, i64, f64, etc.
    pub router_data: T,
}

impl<T> From<(FloatMajorUnit, T)> for FlexitiRouterData<T> {
    fn from((amount, item): (FloatMajorUnit, T)) -> Self {
        Self {
            amount,
            router_data: item,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct FlexitiPaymentsRequest {
    merchant_order_id: Option<String>,
    lang: String,
    flow: FlexitiFlow,
    amount_requested: FloatMajorUnit,
    email: Option<Email>,
    fname: Secret<String>,
    billing_information: BillingInformation,
    shipping_information: ShippingInformation,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FlexitiFlow {
    #[serde(rename = "apply/buy")]
    ApplyAndBuy,
    Apply,
    Buy,
}
#[derive(Debug, Serialize)]
pub struct BillingInformation {
    first_name: Secret<String>,
    last_name: Secret<String>,
    address_1: Secret<String>,
    address_2: Secret<String>,
    city: Secret<String>,
    postal_code: Secret<String>,
    province: Secret<String>,
}

#[derive(Debug, Serialize)]
pub struct ShippingInformation {
    first_name: Secret<String>,
    last_name: Secret<String>,
    address_1: Secret<String>,
    address_2: Secret<String>,
    city: Secret<String>,
    postal_code: Secret<String>,
    province: Secret<String>,
}

impl TryFrom<&FlexitiRouterData<&PaymentsAuthorizeRouterData>> for FlexitiPaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &FlexitiRouterData<&PaymentsAuthorizeRouterData>,
    ) -> Result<Self, Self::Error> {
        match item.router_data.request.payment_method_data.clone() {
            PaymentMethodData::PayLater(pay_later_data) => match pay_later_data {
                hyperswitch_domain_models::payment_method_data::PayLaterData::FlexitiRedirect {  } =>  {
                    let shipping_address = item.router_data.get_shipping_address()?;
                    let shipping_information = ShippingInformation {
                        first_name: shipping_address.get_first_name()?.to_owned(),
                        last_name: shipping_address.get_last_name()?.to_owned(),
                        address_1: shipping_address.get_line1()?.to_owned(),
                        address_2: shipping_address.get_line2()?.to_owned(),
                        city: shipping_address.get_city()?.to_owned().into(),
                        postal_code: shipping_address.get_zip()?.to_owned(),
                        province: shipping_address.to_state_code()?,
                    };
                    let billing_information = BillingInformation {
                        first_name: item.router_data.get_billing_first_name()?,
                        last_name: item.router_data.get_billing_last_name()?,
                        address_1: item.router_data.get_billing_line1()?,
                        address_2: item.router_data.get_billing_line2()?,
                        city: item.router_data.get_billing_city()?.into(),
                        postal_code: item.router_data.get_billing_zip()?,
                        province: item.router_data.get_billing_state_code()?,
                    };
                    Ok(Self {
                        merchant_order_id: item.router_data.request.merchant_order_reference_id.to_owned(),
                        lang: item.router_data.request.get_browser_info()?.get_language()?,
                        flow: FlexitiFlow::ApplyAndBuy,
                        amount_requested: item.amount.to_owned(),
                        email: item.router_data.get_optional_billing_email(),
                        fname: item.router_data.get_billing_first_name()?,
                        billing_information,
                        shipping_information,
                    })
                },
                hyperswitch_domain_models::payment_method_data::PayLaterData::KlarnaRedirect {  } |
                hyperswitch_domain_models::payment_method_data::PayLaterData::KlarnaSdk { .. } |
                hyperswitch_domain_models::payment_method_data::PayLaterData::AffirmRedirect {  }  |
                hyperswitch_domain_models::payment_method_data::PayLaterData::BreadpayRedirect {  }  |
                hyperswitch_domain_models::payment_method_data::PayLaterData::AfterpayClearpayRedirect {  }  |
                hyperswitch_domain_models::payment_method_data::PayLaterData::PayBrightRedirect {  }  |
                hyperswitch_domain_models::payment_method_data::PayLaterData::WalleyRedirect {  }  |
                hyperswitch_domain_models::payment_method_data::PayLaterData::AlmaRedirect {  }  |
                hyperswitch_domain_models::payment_method_data::PayLaterData::AtomeRedirect {  } |
                hyperswitch_domain_models::payment_method_data::PayLaterData::PayjustnowRedirect {  } => {
                                Err(errors::ConnectorError::NotImplemented(
                                utils::get_unimplemented_payment_method_error_message("flexiti"),
                            ))
                            }?,
            }
            _ => Err(errors::ConnectorError::NotImplemented("Payment method".to_string()).into()),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct FlexitiAccessTokenRequest {
    client_id: Secret<String>,
    client_secret: Secret<String>,
    grant_type: FlexitiGranttype,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FlexitiGranttype {
    Password,
    RefreshToken,
    ClientCredentials,
}

impl TryFrom<&types::RefreshTokenRouterData> for FlexitiAccessTokenRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::RefreshTokenRouterData) -> Result<Self, Self::Error> {
        let auth_details = FlexitiAuthType::try_from(&item.connector_auth_type)?;

        Ok(Self {
            client_id: auth_details.client_id,
            client_secret: auth_details.client_secret,
            grant_type: FlexitiGranttype::ClientCredentials,
        })
    }
}

impl<F, T> TryFrom<ResponseRouterData<F, FlexitiAccessTokenResponse, T, AccessToken>>
    for RouterData<F, T, AccessToken>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, FlexitiAccessTokenResponse, T, AccessToken>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(AccessToken {
                token: item.response.access_token,
                expires: item.response.expires_in,
            }),
            ..item.data
        })
    }
}

// Auth Struct
pub struct FlexitiAuthType {
    pub(super) client_id: Secret<String>,
    pub(super) client_secret: Secret<String>,
}

impl TryFrom<&ConnectorAuthType> for FlexitiAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            ConnectorAuthType::BodyKey { api_key, key1 } => Ok(Self {
                client_id: api_key.to_owned(),
                client_secret: key1.to_owned(),
            }),
            _ => Err(errors::ConnectorError::FailedToObtainAuthType.into()),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FlexitiAccessTokenResponse {
    access_token: Secret<String>,
    expires_in: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlexitiPaymentsResponse {
    redirection_url: url::Url,
    online_order_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlexitiSyncResponse {
    transaction_id: String,
    purchase: FlexitiPurchase,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlexitiPurchase {
    status: FlexitiPurchaseStatus,
}

// Since this is an alpha integration, we don't have access to all the status mapping. This needs to be updated.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FlexitiPurchaseStatus {
    Success,
    Failed,
}

// Since this is an alpha integration, we don't have access to all the status mapping. This needs to be updated.
impl From<FlexitiPurchaseStatus> for common_enums::AttemptStatus {
    fn from(item: FlexitiPurchaseStatus) -> Self {
        match item {
            FlexitiPurchaseStatus::Success => Self::Authorized,
            FlexitiPurchaseStatus::Failed => Self::Failure,
        }
    }
}

impl<F, T> TryFrom<ResponseRouterData<F, FlexitiSyncResponse, T, PaymentsResponseData>>
    for RouterData<F, T, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, FlexitiSyncResponse, T, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            status: common_enums::AttemptStatus::from(item.response.purchase.status),
            response: Ok(PaymentsResponseData::TransactionResponse {
                resource_id: ResponseId::ConnectorTransactionId(
                    item.response.transaction_id.to_owned(),
                ),
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

impl<F, T> TryFrom<ResponseRouterData<F, FlexitiPaymentsResponse, T, PaymentsResponseData>>
    for RouterData<F, T, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, FlexitiPaymentsResponse, T, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            status: common_enums::AttemptStatus::AuthenticationPending,
            response: Ok(PaymentsResponseData::TransactionResponse {
                resource_id: ResponseId::ConnectorTransactionId(
                    item.response.online_order_id.to_owned(),
                ),
                redirection_data: Box::new(Some(RedirectForm::from((
                    item.response.redirection_url,
                    Method::Get,
                )))),
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
pub struct FlexitiRefundRequest {
    pub amount: FloatMajorUnit,
}

impl<F> TryFrom<&FlexitiRouterData<&RefundsRouterData<F>>> for FlexitiRefundRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &FlexitiRouterData<&RefundsRouterData<F>>) -> Result<Self, Self::Error> {
        Ok(Self {
            amount: item.amount.to_owned(),
        })
    }
}

// Type definition for Refund Response

#[allow(dead_code)]
#[derive(Debug, Copy, Serialize, Default, Deserialize, Clone)]
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

#[derive(Debug, Serialize, Deserialize)]
pub struct FlexitiErrorResponse {
    pub message: String,
    pub error: String,
}
