use common_enums::{enums,Currency};
use common_utils::{errors::CustomResult, types::StringMinorUnit, request::Method,};
use hyperswitch_domain_models::{
    payment_method_data::PaymentMethodData,
    router_data::{ConnectorAuthType, RouterData},
    router_flow_types::refunds::{Execute, RSync},
    router_request_types::{PaymentsAuthorizeData,ResponseId,PaymentsPreProcessingData, CompleteAuthorizeData, PaymentsSyncData, PaymentsCaptureData},
    router_response_types::{PaymentsResponseData, RefundsResponseData, RedirectForm},
    types::{PaymentsAuthorizeRouterData, RefundsRouterData, PaymentsPreProcessingRouterData, PaymentsCompleteAuthorizeRouterData, PaymentsCaptureRouterData},
};
use hyperswitch_interfaces::errors;
use masking::Secret;
use serde::{Deserialize, Serialize};
use cards::CardNumber;
use url::Url;
use std::collections::HashMap;
use masking::ExposeInterface;
use error_stack::ResultExt;

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

#[derive(Debug, Clone, Serialize, Deserialize, )]
#[serde(rename_all = "camelCase")]
struct PaymentRequest {
    operation_id: String,
    order: Order, 
}

#[derive(Debug, Clone, Serialize, Deserialize, )]
#[serde(rename_all = "camelCase")]
pub struct NexixpayPaymentsRequest {
    order: Order,
    card: NexixpayCard,
    recurrence: Option<Recurrence>,
    exemptions: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, )]
#[serde(rename_all = "camelCase")]
pub struct NexixpayCompleteAuthorizeRequest {
    order: Order,
    card: PaymentMethodData,
    operation_id: String
}

#[derive(Debug, Clone, Serialize, Deserialize, )]
#[serde(rename_all = "camelCase")]
pub struct NexixpayCompleteAuthorizeResponse {
    order: Order,
    card: PaymentMethodData,
    operation_id: String
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, )]
#[serde(rename_all = "camelCase")]
pub struct NexixpayPreProcessingRequest {
    operation_id: String,
    three_d_s_auth_response: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, )]
#[serde(rename_all = "camelCase")]
pub struct Order {
    order_id: String,
    amount: StringMinorUnit,
    currency: Currency,
    description: Option<String>,
    custom_field: Option<String>,
    customer_info: Option<CustomerInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize, )]
#[serde(rename_all = "camelCase")]
pub struct CustomerInfo {
    card_holder_name: Option<Secret<String>>,
    billing_address: Address,
    shipping_address: Address,
}

#[derive(Debug, Clone, Serialize, Deserialize, )]
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

#[derive(Debug, Clone, Serialize, Deserialize, )]
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
#[derive(Debug, Clone, Serialize, Deserialize, )]
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

#[derive( Debug, Clone, Serialize, Deserialize, )]
#[serde(rename_all = "camelCase")]
pub struct NexixpayPreProcessingResponse{
    operation: Operation,
    three_d_s_auth_result: ThreeDSAuthResult,
}

#[derive(Debug, Clone, Serialize, Deserialize, )]
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

#[derive(Default, Debug, Clone, Serialize, Deserialize, )]
#[serde(rename_all = "PascalCase")]
pub struct RedirectPayload {
    pa_res: String,
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
        let operation_id = item.connector_request_reference_id.clone();
        let redirect_payload = redirect_response
                    .payload
                    .ok_or(errors::ConnectorError::MissingConnectorRedirectionPayload {
                        field_name: "request.redirect_response.payload",
                    })?
                    .expose();
        let customer_details_encrypted =
                serde_json::from_value::<RedirectPayload>(redirect_payload).change_context(
                    errors::ConnectorError::MissingConnectorRedirectionPayload {
                        field_name: "redirection_payload",
                    },
                )?;
        Ok(Self {
            operation_id,
            three_d_s_auth_response: customer_details_encrypted.pa_res,
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
           // let complete_authorise_url = item.data.request.complete_authorize_url.clone().ok_or_else(missing_field_err("complete_authorise_url"))?;
            let three_ds_data = item.response.three_d_s_auth_result;
            Ok(Self {
                status: common_enums::AttemptStatus::from(item.response.operation.operation_result),
                response: Ok(PaymentsResponseData::TransactionResponse {
                    resource_id: ResponseId::NoResponseId,
                    redirection_data: None,
                    mandate_reference: None,
                    connector_metadata: Some(serde_json::json!({
                        "three_ds_data": three_ds_data
                    })),
                    network_txn_id: None,
                    connector_response_reference_id: Some(item.response.operation.operation_id),
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
                    order_id:item.router_data.connector_request_reference_id.clone(),
                    amount: item.amount.clone(),
                    currency: item.router_data.request.currency,
                    description: None,
                    custom_field: None,
                    customer_info: Some(customer_info),
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

//TODO: Append the remaining status flags
#[derive(Debug, Clone, Serialize, Deserialize, )]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum NexixpayPaymentStatus {
    Authorized,
    Executed,
    Declined,
    DeniedByRisk,
    ThreedsValidated,
    ThreedsFailed, 
    Pending,
    Canceled,
    Voided,
    Refunded,
    Failed,
    StatusNotReceived
}

#[derive(Debug, Clone, Serialize, Deserialize, )]
#[serde(rename_all = "camelCase")]
pub struct NexixpayTransactionResponse {
    order_id: String,
    operation_id: String,
    operation_result: NexixpayPaymentStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, )]
#[serde(rename_all = "camelCase")]
pub struct NexixpayPaymentsCaptureRequest {
    amount: StringMinorUnit,
    currency: Currency,
}

#[derive(Debug, Clone, Serialize, Deserialize, )]
#[serde(rename_all = "camelCase")]
pub struct NexixpayPaymentsCaptureResponse  {
    operation_id: String,
}

impl From<NexixpayPaymentStatus> for common_enums::AttemptStatus {
    fn from(item: NexixpayPaymentStatus) -> Self {
        match item {
            NexixpayPaymentStatus::Declined
            | NexixpayPaymentStatus::DeniedByRisk
            | NexixpayPaymentStatus::ThreedsFailed
            | NexixpayPaymentStatus::Failed => Self::Failure,
            NexixpayPaymentStatus::Authorized 
            | NexixpayPaymentStatus::ThreedsValidated=> Self::Authorized,
            NexixpayPaymentStatus::Executed=> Self::Charged,
            NexixpayPaymentStatus::Pending
            | NexixpayPaymentStatus::StatusNotReceived=> Self::Pending,
            NexixpayPaymentStatus::Canceled
            | NexixpayPaymentStatus::Voided => Self::Voided,
            NexixpayPaymentStatus::Refunded => Self::AutoRefunded,
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

impl<F> TryFrom<ResponseRouterData<F, NexixpayPaymentsResponse, CompleteAuthorizeData, PaymentsResponseData>>
    for RouterData<F, CompleteAuthorizeData, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, NexixpayPaymentsResponse, CompleteAuthorizeData, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            status: common_enums::AttemptStatus::from(item.response.operation.operation_result),
            response: Ok(PaymentsResponseData::TransactionResponse {
                resource_id: ResponseId::ConnectorTransactionId(item.response.operation.operation_id),
                redirection_data: None,
                mandate_reference: None,
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


impl TryFrom<&NexixpayRouterData<&PaymentsCompleteAuthorizeRouterData>> for NexixpayCompleteAuthorizeRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &NexixpayRouterData<&PaymentsCompleteAuthorizeRouterData>,
    ) -> Result<Self, Self::Error> {
        let payment_method_data: PaymentMethodData = item.router_data.request.payment_method_data.clone().ok_or(
            errors::ConnectorError::MissingRequiredField {
                field_name: "payment_method_data",
            },
        )?;
        let operation_id = item.router_data.request.connector_transaction_id.clone().ok_or(
            errors::ConnectorError::MissingRequiredField {
                field_name: "connector_transaction_id",
            },
        )?;
        let order_id = item.router_data.connector_request_reference_id.clone();
        let amount = item.amount.clone();
        let order_data = Order{
            order_id,
            amount,
            currency: item.router_data.request.currency,
            description: None,
            custom_field: None,
            customer_info: None,
        };
        Ok(Self {
            operation_id: operation_id,
            order: order_data,
            card: payment_method_data
        })
    }
}

impl<F>
    TryFrom<
        ResponseRouterData<
            F,
            NexixpayTransactionResponse,
            PaymentsSyncData,
            PaymentsResponseData,
        >,
    > for RouterData<F, PaymentsSyncData, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<
            F,
            NexixpayTransactionResponse,
            PaymentsSyncData,
            PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            status: item.data.status,
            response: Ok(PaymentsResponseData::TransactionResponse {
                resource_id: ResponseId::ConnectorTransactionId(
                    item.response.operation_id.clone(),
                ),
                redirection_data: None,
                mandate_reference: None,
                connector_metadata: None,
                network_txn_id: None,
                connector_response_reference_id: Some(item.response.operation_id.clone()),
                incremental_authorization_allowed: None,
                charge_id: None,
            }),
            ..item.data
        })
    }
}

impl TryFrom<&NexixpayRouterData<&PaymentsCaptureRouterData>>
    for NexixpayPaymentsCaptureRequest
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &NexixpayRouterData<&PaymentsCaptureRouterData>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            amount: item.amount.clone(),
            currency: item.router_data.request.currency,
        })
    }
}

impl<F>
    TryFrom<
        ResponseRouterData<
            F,
            NexixpayPaymentsCaptureResponse,
            PaymentsCaptureData,
            PaymentsResponseData,
        >,
    > for RouterData<F, PaymentsCaptureData, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<
            F,
            NexixpayPaymentsCaptureResponse,
            PaymentsCaptureData,
            PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            status: item.data.status,
            response: Ok(PaymentsResponseData::TransactionResponse {
                resource_id: ResponseId::ConnectorTransactionId(
                    item.response.operation_id.clone(),
                ),
                redirection_data: None,
                mandate_reference: None,
                connector_metadata: None,
                network_txn_id: None,
                connector_response_reference_id: Some(item.response.operation_id.clone()),
                incremental_authorization_allowed: None,
                charge_id: None,
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
