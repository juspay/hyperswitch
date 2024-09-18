use std::collections::HashMap;

use cards::CardNumber;
use common_enums::{enums, AttemptStatus, CaptureMethod, Currency, RefundStatus};
use common_utils::{errors::CustomResult, request::Method, types::StringMinorUnit};
use error_stack::ResultExt;
use hyperswitch_domain_models::{
    payment_method_data::PaymentMethodData,
    router_data::{ConnectorAuthType, RouterData},
    router_flow_types::refunds::{Execute, RSync},
    router_request_types::{
        CompleteAuthorizeData, PaymentsAuthorizeData, PaymentsPreProcessingData, PaymentsSyncData,
        ResponseId,
    },
    router_response_types::{PaymentsResponseData, RedirectForm, RefundsResponseData},
    types::{
        PaymentsAuthorizeRouterData, PaymentsCancelRouterData, PaymentsCaptureRouterData,
        PaymentsCompleteAuthorizeRouterData, PaymentsPreProcessingRouterData, RefundsRouterData,
    },
};
use hyperswitch_interfaces::errors;
use masking::{ExposeInterface, Secret};
use serde::{Deserialize, Serialize};
use url::Url;

use crate::{
    types::{RefundsResponseRouterData, ResponseRouterData},
    utils::{missing_field_err, CardData, RouterData as _},
};

pub struct NexixpayRouterData<T> {
    pub amount: StringMinorUnit,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NexixpayPaymentsRequest {
    order: Order,
    card: NexixpayCard,
    recurrence: Option<Recurrence>,
    exemptions: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum NexixpayCaptureType {
    Implicit,
    Explicit,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NexixpayCompleteAuthorizeRequest {
    order: Order,
    card: NexixpayCard,
    operation_id: String,
    capture_type: Option<NexixpayCaptureType>,
    three_d_s_auth_data: ThreeDSAuthData,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OperationData {
    operation_id: String,
    operation_currency: Currency,
    operation_result: NexixpayPaymentStatus,
    operation_type: NexixpayOperationType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NexixpayCompleteAuthorizeResponse {
    operation: OperationData,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NexixpayPreProcessingRequest {
    operation_id: String,
    three_d_s_auth_response: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Order {
    order_id: String,
    amount: StringMinorUnit,
    currency: Currency,
    description: Option<String>,
    custom_field: Option<String>,
    customer_info: Option<CustomerInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CustomerInfo {
    card_holder_name: Option<Secret<String>>,
    billing_address: Address,
    shipping_address: Address,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NexixpayCard {
    pan: CardNumber,
    expiry_date: Secret<String>,
    cvv: Option<Secret<String>>,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Recurrence {
    action: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NexixpayPaymentsResponse {
    operation: Operation,
    three_d_s_auth_request: String,
    three_d_s_auth_url: Secret<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThreeDSAuthResult {
    authentication_value: Secret<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThreeDSCompleteAuthRequestData {
    three_d_s_auth_result: ThreeDSAuthResult,
    three_d_s_auth_response: String,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct ThreeDSAuthData {
    three_d_s_auth_response: String,
    authentication_value: Secret<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NexixpayPreProcessingResponse {
    operation: Operation,
    three_d_s_auth_result: ThreeDSAuthResult,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Operation {
    additional_data: AdditionalData,
    customer_info: CustomerInfo,
    operation_amount: String,
    operation_currency: Currency,
    operation_id: String,
    operation_result: NexixpayPaymentStatus,
    operation_time: String,
    operation_type: NexixpayOperationType,
    order_id: String,
    payment_circuit: String,
    payment_end_to_end_id: String,
    payment_instrument_info: String,
    payment_method: String,
    warnings: Option<Vec<String>>,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdditionalData {
    masked_pan: String,
    card_id: Secret<String>,
    card_id4: Option<Secret<String>>,
    card_expiry_date: Option<Secret<String>>,
}

pub struct NexixpayLinks {
    _href: Option<Url>,
    _rel: String,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct RedirectPayload {
    #[serde(rename = "PaRes")]
    pa_res: String,

    #[serde(rename = "paymentId")]
    payment_id: String,
}

impl TryFrom<&PaymentsPreProcessingRouterData> for NexixpayPreProcessingRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &PaymentsPreProcessingRouterData) -> Result<Self, Self::Error> {
        let redirect_response = item.request.redirect_response.clone().ok_or(
            errors::ConnectorError::MissingRequiredField {
                field_name: "redirect_response",
            },
        )?;
        let redirect_payload = redirect_response
            .payload
            .ok_or(errors::ConnectorError::MissingConnectorRedirectionPayload {
                field_name: "request.redirect_response.payload",
            })?
            .expose();
        let customer_details_encrypted: RedirectPayload =
            serde_json::from_value::<RedirectPayload>(redirect_payload.clone()).change_context(
                errors::ConnectorError::MissingConnectorRedirectionPayload {
                    field_name: "redirection_payload",
                },
            )?;
        let operation_id = customer_details_encrypted.payment_id;
        let pares = customer_details_encrypted.pa_res;
        Ok(Self {
            operation_id: operation_id.clone(),
            three_d_s_auth_response: pares.clone(),
        })
    }
}

impl<F>
    TryFrom<
        ResponseRouterData<
            F,
            NexixpayPreProcessingResponse,
            PaymentsPreProcessingData,
            PaymentsResponseData,
        >,
    > for RouterData<F, PaymentsPreProcessingData, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<
            F,
            NexixpayPreProcessingResponse,
            PaymentsPreProcessingData,
            PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        let three_ds_data: ThreeDSAuthResult = item.response.three_d_s_auth_result;

        let redirect_response = item.data.request.redirect_response.clone().ok_or(
            errors::ConnectorError::MissingRequiredField {
                field_name: "redirect_response",
            },
        )?;
        let redirect_payload = redirect_response
            .payload
            .ok_or(errors::ConnectorError::MissingConnectorRedirectionPayload {
                field_name: "request.redirect_response.payload",
            })?
            .expose();
        let customer_details_encrypted = serde_json::from_value::<RedirectPayload>(
            redirect_payload,
        )
        .change_context(errors::ConnectorError::MissingConnectorRedirectionPayload {
            field_name: "redirection_payload",
        })?;
        Ok(Self {
            status: AttemptStatus::from(item.response.operation.operation_result),
            response: Ok(PaymentsResponseData::TransactionResponse {
                resource_id: ResponseId::NoResponseId,
                redirection_data: None,
                mandate_reference: None,
                connector_metadata: Some(serde_json::json!({
                    "threeDSAuthResult": three_ds_data,
                    "threeDSAuthResponse": customer_details_encrypted.pa_res,
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
            PaymentMethodData::Card(ref req_card) => {
                let card = NexixpayCard {
                    pan: req_card.card_number.clone(),
                    expiry_date: req_card.get_expiry_date_as_mmyy()?,
                    cvv: Some(req_card.card_cvc.clone()),
                };
                let billing_address = Address {
                    name: item.router_data.get_optional_billing_full_name().clone(),
                    street: item.router_data.get_optional_billing_line1(),
                    additional_info: None,
                    city: item.router_data.get_optional_billing_city(),
                    post_code: item.router_data.get_optional_billing_zip(),
                    province: None,
                    country: item.router_data.get_optional_billing_country(),
                };
                let customer_info = CustomerInfo {
                    card_holder_name: item.router_data.get_optional_billing_full_name().clone(),
                    billing_address: billing_address.clone(),
                    shipping_address: billing_address,
                };
                let order = Order {
                    order_id: item.router_data.connector_request_reference_id.clone(),
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
                    exemptions: None,
                })
            }
            _ => Err(errors::ConnectorError::NotImplemented("Payment methods".to_string()).into()),
        }
    }
}

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
#[derive(Debug, Clone, Serialize, Deserialize)]
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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum NexixpayOperationType {
    Authorization,
    Capture,
    Void,
    Refund,
    CardVerification,
    Noshow,
    Incremental,
    DelayCharge,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum NexixpayRefundOperationType {
    Refund,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum NexixpayRefundResultStatus {
    Pending,
    Voided,
    Refunded,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NexixpayTransactionResponse {
    order_id: String,
    operation_id: String,
    operation_result: NexixpayPaymentStatus,
    operation_type: NexixpayOperationType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NexixpayRSyncResponse {
    order_id: String,
    operation_id: String,
    operation_result: NexixpayRefundResultStatus,
    operation_type: NexixpayRefundOperationType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NexixpayPaymentsCaptureRequest {
    amount: StringMinorUnit,
    currency: Currency,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NexixpayPaymentsCancleRequest {
    description: Option<String>,
    amount: StringMinorUnit,
    currency: Currency,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NexixpayOperationResponse {
    operation_id: String,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct NexixpayRefundRequest {
    pub amount: StringMinorUnit,
    pub currency: Currency,
}

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RefundResponse {
    operation_id: String,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NexixpayErrorBody {
    pub code: Option<String>,
    pub description: Option<String>,
}
#[derive(Default, Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NexixpayErrorResponse {
    pub errors: Vec<NexixpayErrorBody>,
}

impl From<NexixpayPaymentStatus> for AttemptStatus {
    fn from(item: NexixpayPaymentStatus) -> Self {
        match item {
            NexixpayPaymentStatus::Declined
            | NexixpayPaymentStatus::DeniedByRisk
            | NexixpayPaymentStatus::ThreedsFailed
            | NexixpayPaymentStatus::Failed => Self::Failure,
            NexixpayPaymentStatus::Authorized | NexixpayPaymentStatus::ThreedsValidated => {
                Self::Authorized
            }
            NexixpayPaymentStatus::Executed => Self::Charged,
            NexixpayPaymentStatus::Pending => Self::AuthenticationPending,
            NexixpayPaymentStatus::Canceled | NexixpayPaymentStatus::Voided => Self::Voided,
            NexixpayPaymentStatus::Refunded => Self::AutoRefunded,
        }
    }
}

fn get_nexixpay_capture_type(
    item: Option<CaptureMethod>,
) -> CustomResult<Option<NexixpayCaptureType>, errors::ConnectorError> {
    match item {
        Some(CaptureMethod::Manual) => Ok(Some(NexixpayCaptureType::Explicit)),
        Some(CaptureMethod::Automatic) => Ok(Some(NexixpayCaptureType::Implicit)),
        Some(item) => Err(errors::ConnectorError::FlowNotSupported {
            flow: item.to_string(),
            connector: "Nexixpay".to_string(),
        }
        .into()),
        None => Ok(None),
    }
}

impl<F>
    TryFrom<
        ResponseRouterData<
            F,
            NexixpayPaymentsResponse,
            PaymentsAuthorizeData,
            PaymentsResponseData,
        >,
    > for RouterData<F, PaymentsAuthorizeData, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<
            F,
            NexixpayPaymentsResponse,
            PaymentsAuthorizeData,
            PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        let complete_authorise_url = item
            .data
            .request
            .complete_authorize_url
            .clone()
            .ok_or_else(missing_field_err("complete_authorise_url"))?;
        let operation_id: String = item.response.operation.operation_id;
        let redirection_form = nexixpay_threeds_link((
            item.response.three_d_s_auth_url.expose().clone(),
            item.response.three_d_s_auth_request.clone(),
            complete_authorise_url.clone(),
            operation_id.clone(),
        ))?;
        Ok(Self {
            status: AttemptStatus::from(item.response.operation.operation_result),
            response: Ok(PaymentsResponseData::TransactionResponse {
                resource_id: ResponseId::ConnectorTransactionId(operation_id.clone()),
                redirection_data: Some(redirection_form.clone()),
                mandate_reference: None,
                connector_metadata: None,
                network_txn_id: None,
                connector_response_reference_id: Some(operation_id.clone()),
                incremental_authorization_allowed: None,
                charge_id: None,
            }),
            ..item.data
        })
    }
}

fn nexixpay_threeds_link(
    (three_d_s_auth_url, three_ds_request, return_url, transaction_id): (
        String,
        String,
        String,
        String,
    ),
) -> CustomResult<RedirectForm, errors::ConnectorError> {
    let mut form_fields = HashMap::<String, String>::new();
    form_fields.insert(String::from("ThreeDsRequest"), three_ds_request);
    form_fields.insert(String::from("ReturnUrl"), return_url);
    form_fields.insert(String::from("transactionId"), transaction_id);

    Ok(RedirectForm::Form {
        endpoint: three_d_s_auth_url,
        method: Method::Post,
        form_fields,
    })
}

impl<F> TryFrom<&NexixpayRouterData<&RefundsRouterData<F>>> for NexixpayRefundRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &NexixpayRouterData<&RefundsRouterData<F>>) -> Result<Self, Self::Error> {
        Ok(Self {
            amount: item.amount.to_owned(),
            currency: item.router_data.request.currency,
        })
    }
}

impl From<NexixpayRefundResultStatus> for RefundStatus {
    fn from(item: NexixpayRefundResultStatus) -> Self {
        match item {
            NexixpayRefundResultStatus::Voided | NexixpayRefundResultStatus::Refunded => {
                Self::Success
            }
            NexixpayRefundResultStatus::Pending => Self::Pending,
            NexixpayRefundResultStatus::Failed => Self::Failure,
        }
    }
}

impl TryFrom<RefundsResponseRouterData<Execute, RefundResponse>> for RefundsRouterData<Execute> {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: RefundsResponseRouterData<Execute, RefundResponse>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(RefundsResponseData {
                connector_refund_id: item.response.operation_id,
                refund_status: RefundStatus::from(NexixpayRefundResultStatus::Pending),
            }),
            ..item.data
        })
    }
}

impl TryFrom<RefundsResponseRouterData<RSync, NexixpayRSyncResponse>> for RefundsRouterData<RSync> {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: RefundsResponseRouterData<RSync, NexixpayRSyncResponse>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(RefundsResponseData {
                connector_refund_id: item.response.operation_id,
                refund_status: RefundStatus::from(item.response.operation_result),
            }),
            ..item.data
        })
    }
}

impl<F>
    TryFrom<
        ResponseRouterData<
            F,
            NexixpayCompleteAuthorizeResponse,
            CompleteAuthorizeData,
            PaymentsResponseData,
        >,
    > for RouterData<F, CompleteAuthorizeData, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<
            F,
            NexixpayCompleteAuthorizeResponse,
            CompleteAuthorizeData,
            PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            status: AttemptStatus::from(item.response.operation.operation_result),
            response: Ok(PaymentsResponseData::TransactionResponse {
                resource_id: ResponseId::ConnectorTransactionId(
                    item.response.operation.operation_id,
                ),
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

impl TryFrom<&NexixpayRouterData<&PaymentsCompleteAuthorizeRouterData>>
    for NexixpayCompleteAuthorizeRequest
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &NexixpayRouterData<&PaymentsCompleteAuthorizeRouterData>,
    ) -> Result<Self, Self::Error> {
        let payment_method_data: PaymentMethodData =
            item.router_data.request.payment_method_data.clone().ok_or(
                errors::ConnectorError::MissingRequiredField {
                    field_name: "payment_method_data",
                },
            )?;
        let operation_id = item
            .router_data
            .request
            .connector_transaction_id
            .clone()
            .ok_or(errors::ConnectorError::MissingRequiredField {
                field_name: "connector_transaction_id",
            })?;
        let capture_type = get_nexixpay_capture_type(item.router_data.request.capture_method)?;

        let order_id = item.router_data.connector_request_reference_id.clone();
        let amount = item.amount.clone();
        let billing_address = Address {
            name: item.router_data.get_optional_billing_full_name().clone(),
            street: item.router_data.get_optional_billing_line1(),
            additional_info: None,
            city: item.router_data.get_optional_billing_city(),
            post_code: item.router_data.get_optional_billing_zip(),
            province: None,
            country: item.router_data.get_optional_billing_country(),
        };
        let customer_info = CustomerInfo {
            card_holder_name: None,
            billing_address: billing_address.clone(),
            shipping_address: billing_address,
        };
        let order_data = Order {
            order_id,
            amount,
            currency: item.router_data.request.currency,
            description: None,
            custom_field: None,
            customer_info: Some(customer_info),
        };
        let connector_metadata = item.router_data.request.connector_meta.clone().ok_or(
            errors::ConnectorError::MissingRequiredField {
                field_name: "connector_meta",
            },
        )?;
        let three_d_s_complete_auth_request_data =
            serde_json::from_value::<ThreeDSCompleteAuthRequestData>(connector_metadata)
                .change_context(errors::ConnectorError::ParsingFailed)?;
        let three_d_s_auth_data = ThreeDSAuthData {
            three_d_s_auth_response: three_d_s_complete_auth_request_data.three_d_s_auth_response,
            authentication_value: three_d_s_complete_auth_request_data
                .three_d_s_auth_result
                .authentication_value,
        };
        let card: Result<NexixpayCard, error_stack::Report<errors::ConnectorError>> =
            match payment_method_data {
                PaymentMethodData::Card(req_card) => Ok(NexixpayCard {
                    pan: req_card.clone().card_number,
                    expiry_date: req_card.clone().get_expiry_date_as_mmyy()?,
                    cvv: None,
                }),
                _ => Err(
                    errors::ConnectorError::NotImplemented("Payment methods".to_string()).into(),
                ),
            };
        Ok(Self {
            order: order_data,
            card: card?.clone(),
            operation_id,
            capture_type,
            three_d_s_auth_data,
        })
    }
}

impl<F>
    TryFrom<
        ResponseRouterData<F, NexixpayTransactionResponse, PaymentsSyncData, PaymentsResponseData>,
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
            status: AttemptStatus::from(item.response.operation_result),
            response: Ok(PaymentsResponseData::TransactionResponse {
                resource_id: ResponseId::ConnectorTransactionId(item.response.operation_id.clone()),
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

impl TryFrom<&NexixpayRouterData<&PaymentsCaptureRouterData>> for NexixpayPaymentsCaptureRequest {
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

impl<F, T> TryFrom<ResponseRouterData<F, NexixpayOperationResponse, T, PaymentsResponseData>>
    for RouterData<F, T, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, NexixpayOperationResponse, T, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            status: item.data.status,
            response: Ok(PaymentsResponseData::TransactionResponse {
                resource_id: ResponseId::ConnectorTransactionId(item.response.operation_id.clone()),
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

impl TryFrom<NexixpayRouterData<&PaymentsCancelRouterData>> for NexixpayPaymentsCancleRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: NexixpayRouterData<&PaymentsCancelRouterData>) -> Result<Self, Self::Error> {
        let description = item.router_data.request.cancellation_reason.clone();
        let currency = item.router_data.request.currency.ok_or(
            errors::ConnectorError::MissingRequiredField {
                field_name: "currency",
            },
        )?;
        Ok(Self {
            amount: item.amount,
            currency,
            description,
        })
    }
}
