use common_enums::{enums,Currency};
use common_utils::{errors::CustomResult, types::{StringMajorUnit,StringMinorUnit}, request::Method,};
use hyperswitch_domain_models::{
    payment_method_data::PaymentMethodData,
    router_data::{ConnectorAuthType, RouterData},
    router_flow_types::refunds::{Execute, RSync},
    router_request_types::{PaymentsAuthorizeData,ResponseId,PaymentsPreProcessingData},
    router_response_types::{PaymentsResponseData, RefundsResponseData, RedirectForm},
    types::{PaymentsAuthorizeRouterData, RefundsRouterData, PaymentsPreProcessingRouterData},
};
use hyperswitch_interfaces::errors;
use masking::Secret;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use cards::CardNumber;
use url::Url;
use std::collections::HashMap;
use masking::ExposeInterface;

use crate::{
    types::{RefundsResponseRouterData, ResponseRouterData},
    utils::{RouterData as _,missing_field_err}
};

//TODO: Fill the struct with respective fields
pub struct NexixpayRouterData<T> {
    pub amount: StringMinorUnit, // The type of amount that a connector accepts, for example, String, i64, f64, etc.
    pub router_data: T,
}

impl<T> From<(StringMinorUnit, T)> for NexixpayRouterData<T> {
    fn from((amount, item): (StringMinorUnit, T)) -> Self {
        //Todo :  use utils to convert the amount to the type of amount that a connector accepts
        Self {
            amount,
            router_data: item,
        }
    }
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, )]
#[serde(rename_all = "camelCase")]
pub struct NexixpayPaymentsRequest {
    order: Order,
    card: NexixpayCard,
    recurrence: Option<Recurrence>,
    exemptions: Option<String>,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, )]
#[serde(rename_all = "camelCase")]
pub struct NexixpayPreProcessingRequest {
    operation_id: String,
    three_d_s_auth_response: String,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, )]
#[serde(rename_all = "camelCase")]
pub struct Order {
    order_id: String,
    amount: StringMinorUnit,
    currency: Currency,
    description: Option<String>,
    custom_field: Option<String>,
    customer_info: CustomerInfo,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, )]
#[serde(rename_all = "camelCase")]
pub struct CustomerInfo {
    card_holder_name: Option<Secret<String>>,
    billing_address: Address,
    shipping_address: Address,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, )]
#[serde(rename_all = "camelCase")]
pub struct Address {
    name: Option<Secret<String>>,
    street: Option<Secret<String>>,
    additional_info: Option<String>,
    city: Option<String>,
    post_code: Option<Secret<String>>,
    province: Option<String>,
    country: Option<enums::CountryAlpha2>,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, )]
#[serde(rename_all = "camelCase")]
pub struct NexixpayCard {
    pan: CardNumber,
    expiry_date: Secret<String>,
    cvv: Secret<String>,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, )]
#[serde(rename_all = "camelCase")]
struct Recurrence {
    action: String,
}

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Clone, Serialize, Deserialize, )]
#[serde(rename_all = "camelCase")]
pub struct NexixpayPaymentsResponse  {
    operation: Operation,
    three_ds_auth_request: String,
    three_ds_auth_url: String,
    three_ds_enrollment_status: String,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, )]
#[serde(rename_all = "camelCase")]
pub struct ThreeDSAuthResult {
    authentication_value: String,   
    status: String,              
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, )]
#[serde(rename_all = "camelCase")]
pub struct NexixpayPreProcessingResponse{
    operation: Operation,
    three_d_s_auth_result: ThreeDSAuthResult,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, )]
#[serde(rename_all = "camelCase")]
pub struct Operation {
    additional_data: AdditionalData,
    customer_info: CustomerInfo,
    operation_amount: String,
    operation_currency: String,
    operation_id: String,
    operation_result: NexixpayPaymentStatus,
    operation_time: String,
    operation_type: String,
    order_id: String,
    payment_circuit: String,
    payment_end_to_end_id: String,
    payment_instrument_info: String,
    payment_method: String,
    warnings: Option<Vec<String>>,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, )]
#[serde(rename_all = "camelCase")]
pub struct AdditionalData {
    masked_pan: String,
    card_id: String,
    card_id4: Option<String>,
    card_expiry_date: Option<String>,
}

pub struct NexixpayLinks {
    href: Option<Url>,
    rel: String,
}

impl TryFrom<&PaymentsPreProcessingRouterData>for NexixpayPreProcessingRequest{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &PaymentsPreProcessingRouterData,
    ) -> Result<Self, Self::Error> {
        let redirect_response = item.request.redirect_response.clone().ok_or(
            errors::ConnectorError::MissingRequiredField {
                field_name: "redirect_response",
            },
        )?;
        let operation_id = item.connector_request_reference_id;
        let redirect_payload = redirect_response
                    .payload
                    .ok_or(errors::ConnectorError::MissingConnectorRedirectionPayload {
                        field_name: "request.redirect_response.payload",
                    })?
                    .expose().to_string();
        Ok(Self {
            operation_id,
            three_d_s_auth_response: redirect_payload,
        })
    }
}

impl<F> TryFrom<ResponseRouterData<F, NexixpayPreProcessingResponse, PaymentsPreProcessingData, PaymentsResponseData>>
    for RouterData<F, PaymentsPreProcessingData, PaymentsResponseData>
    {
        type Error = error_stack::Report<errors::ConnectorError>;
        fn try_from(
            item: ResponseRouterData<F, NexixpayPreProcessingResponse, PaymentsPreProcessingData, PaymentsResponseData>,
        ) -> Result<Self, Self::Error> {
            let complete_authorise_url = item.data.request.complete_authorize_url.clone().ok_or_else(missing_field_err("complete_authorise_url"))?;
            Ok(Self {
                status: common_enums::AttemptStatus::from(item.response.operation.operation_result),
                response: Ok(PaymentsResponseData::TransactionResponse {
                    resource_id: ResponseId::NoResponseId,
                    redirection_data,
                    mandate_reference: None,
                    connector_metadata: Some(serde_json::json!({
                        "three_ds_data": three_ds_data
                    })),
                    network_txn_id: None,
                    connector_response_reference_id,
                    incremental_authorization_allowed: None,
                    charge_id: None,
                }),
                ..item.data
            })
        }
    }

impl TryFrom<&NexixpayRouterData<&PaymentsAuthorizeRouterData>> for NexixpayPaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &NexixpayRouterData<&PaymentsAuthorizeRouterData>,
    ) -> Result<Self, Self::Error> {
        match item.router_data.request.payment_method_data.clone() {
            PaymentMethodData::Card(req_card) => {
                let card = NexixpayCard {
                    pan: req_card.card_number,
                    expiry_date: req_card.card_exp_month,
                    cvv: req_card.card_cvc,
                };
                let billing_address=Address{
                    name: item.router_data.get_optional_billing_full_name().clone(),
                    street: item.router_data.get_optional_billing_line1(),
                    additional_info: None,
                    city: item.router_data.get_optional_billing_city(),
                    post_code: item.router_data.get_optional_billing_zip(),
                    province: None,
                    country: item.router_data.get_optional_billing_country(),
                };
                let customer_info = CustomerInfo{
                    card_holder_name: item.router_data.get_optional_billing_full_name().clone(),
                    billing_address:billing_address.clone(),
                    shipping_address: billing_address
                };
                let order = Order{
                    order_id: Uuid::new_v4().to_string(),
                    amount: item.amount.clone(),
                    currency: item.router_data.request.currency,
                    description: None,
                    custom_field: None,
                    customer_info,
                };
                Ok(Self {
                    order,
                    card,
                    recurrence: None,
                    exemptions: None
                })
            }
            _ => Err(errors::ConnectorError::NotImplemented("Payment methods".to_string()).into()),
        }
    }
}

//TODO: Fill the struct with respective fields
// Auth Struct
pub struct NexixpayAuthType {
    pub(super) api_key: Secret<String>,
}

impl TryFrom<&ConnectorAuthType> for NexixpayAuthType {
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
#[derive(Debug, Clone, Default, Serialize, Deserialize, )]
#[serde(rename_all = "lowercase")]
pub enum NexixpayPaymentStatus {
    Succeeded,
    Failed,
    #[default]
    Processing,
}

impl From<NexixpayPaymentStatus> for common_enums::AttemptStatus {
    fn from(item: NexixpayPaymentStatus) -> Self {
        match item {
            NexixpayPaymentStatus::Succeeded => Self::Charged,
            NexixpayPaymentStatus::Failed => Self::Failure,
            NexixpayPaymentStatus::Processing => Self::Authorizing,
        }
    }
}


fn get_redirect_url(
    link_vec: Vec<NexixpayLinks>,
) -> CustomResult<Option<Url>, errors::ConnectorError> {
    let mut link: Option<Url> = None;
    for item2 in link_vec.iter() {
        if item2.rel == "payer-action" {
            link.clone_from(&item2.href)
        }
    }
    Ok(link)
}

impl<F> TryFrom<ResponseRouterData<F, NexixpayPaymentsResponse, PaymentsAuthorizeData, PaymentsResponseData>>
    for RouterData<F, PaymentsAuthorizeData, PaymentsResponseData>
    {
        type Error = error_stack::Report<errors::ConnectorError>;
        fn try_from(
            item: ResponseRouterData<F, NexixpayPaymentsResponse, PaymentsAuthorizeData, PaymentsResponseData>,
        ) -> Result<Self, Self::Error> {
            let complete_authorise_url = item.data.request.complete_authorize_url.clone().ok_or_else(missing_field_err("complete_authorise_url"))?;
            Ok(Self {
                status: common_enums::AttemptStatus::from(item.response.operation.operation_result),
                response: Ok(PaymentsResponseData::TransactionResponse {
                    resource_id: ResponseId::ConnectorTransactionId(item.response.operation.operation_id.clone()),
                    redirection_data: Some(nexixpay_threeds_link((
                        item.response.three_ds_auth_url,
                        item.response.three_ds_auth_request,
                        complete_authorise_url,
                    ))?),
                    mandate_reference: None,
                    connector_metadata: None,
                    network_txn_id: None,
                    connector_response_reference_id: Some(item.response.operation.operation_id),
                    incremental_authorization_allowed: None,
                    charge_id: None,
                }),
                ..item.data
            })
        }
    }
    
    fn nexixpay_threeds_link(
        (three_ds_auth_url, three_ds_request, return_url): (String, String, String),
    ) -> CustomResult<RedirectForm, errors::ConnectorError> {
        let mut form_fields = HashMap::<String, String>::new();
        // paypal requires return url to be passed as a field along with payer_action_url
        form_fields.insert(String::from("three_ds_auth_url"), three_ds_auth_url.clone());
        form_fields.insert(String::from("three_ds_request"), three_ds_request);
        form_fields.insert(String::from("return_url"), return_url);
    
        Ok(RedirectForm::Form {
            endpoint: three_ds_auth_url,
            method: Method::Post,
            form_fields,
        })
    }

//TODO: Fill the struct with respective fields
// REFUND :
// Type definition for RefundRequest
#[derive(Default, Debug, Clone, Serialize, Deserialize, )]
pub struct NexixpayRefundRequest {
    pub amount: StringMinorUnit,
}

impl<F> TryFrom<&NexixpayRouterData<&RefundsRouterData<F>>> for NexixpayRefundRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &NexixpayRouterData<&RefundsRouterData<F>>) -> Result<Self, Self::Error> {
        Ok(Self {
            amount: item.amount.to_owned(),
        })
    }
}

// Type definition for Refund Response

#[allow(dead_code)]
#[derive(Default, Debug, Clone, Serialize, Deserialize, )]
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
#[derive(Default, Debug, Clone, Serialize, Deserialize, )]
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
#[derive(Default, Debug, Clone, Serialize, Deserialize, )]
pub struct NexixpayErrorResponse {
    pub status_code: u16,
    pub code: String,
    pub message: String,
    pub reason: Option<String>,
}
