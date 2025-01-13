use std::collections::HashMap;

use cards::CardNumber;
use common_enums::{enums, AttemptStatus, CaptureMethod, Currency, RefundStatus};
use common_utils::{
    errors::CustomResult, ext_traits::ValueExt, request::Method, types::StringMinorUnit,
};
use error_stack::ResultExt;
use hyperswitch_domain_models::{
    payment_method_data::PaymentMethodData,
    router_data::{ConnectorAuthType, RouterData},
    router_flow_types::refunds::{Execute, RSync},
    router_request_types::{
        CompleteAuthorizeData, PaymentsAuthorizeData, PaymentsCancelData, PaymentsCaptureData,
        PaymentsPreProcessingData, PaymentsSyncData, ResponseId,
    },
    router_response_types::{
        MandateReference, PaymentsResponseData, RedirectForm, RefundsResponseData,
    },
    types::{
        PaymentsAuthorizeRouterData, PaymentsCancelRouterData, PaymentsCaptureRouterData,
        PaymentsCompleteAuthorizeRouterData, PaymentsPreProcessingRouterData, RefundsRouterData,
    },
};
use hyperswitch_interfaces::errors;
use masking::{ExposeInterface, Secret};
use serde::{Deserialize, Serialize};

use crate::{
    types::{RefundsResponseRouterData, ResponseRouterData},
    utils::{
        get_unimplemented_payment_method_error_message, to_connector_meta,
        to_connector_meta_from_secret, CardData, PaymentsAuthorizeRequestData,
        PaymentsCompleteAuthorizeRequestData, PaymentsPreProcessingRequestData, RouterData as _,
    },
};

pub struct NexixpayRouterData<T> {
    pub amount: StringMinorUnit,
    pub router_data: T,
}

impl<T> From<(StringMinorUnit, T)> for NexixpayRouterData<T> {
    fn from((amount, item): (StringMinorUnit, T)) -> Self {
        Self {
            amount,
            router_data: item,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum NexixpayRecurringAction {
    NoRecurring,
    SubsequentPayment,
    ContractCreation,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ContractType {
    MitUnscheduled,
    MitScheduled,
    Cit,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecurrenceRequest {
    action: NexixpayRecurringAction,
    contract_id: Secret<String>,
    contract_type: ContractType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NexixpayNonMandatePaymentRequest {
    card: NexixpayCard,
    recurrence: Option<RecurrenceRequest>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NexixpayMandatePaymentRequest {
    contract_id: Secret<String>,
    capture_type: Option<NexixpayCaptureType>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(untagged)]
pub enum NexixpayPaymentsRequestData {
    NexixpayNonMandatePaymentRequest(Box<NexixpayNonMandatePaymentRequest>),
    NexixpayMandatePaymentRequest(Box<NexixpayMandatePaymentRequest>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NexixpayPaymentsRequest {
    order: Order,
    #[serde(flatten)]
    payment_data: NexixpayPaymentsRequestData,
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
    recurrence: Option<RecurrenceRequest>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OperationData {
    operation_id: String,
    operation_currency: Currency,
    operation_result: NexixpayPaymentStatus,
    operation_type: NexixpayOperationType,
    order_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NexixpayCompleteAuthorizeResponse {
    operation: OperationData,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NexixpayPreProcessingRequest {
    operation_id: Option<String>,
    three_d_s_auth_response: Option<Secret<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Order {
    order_id: String,
    amount: StringMinorUnit,
    currency: Currency,
    description: Option<String>,
    customer_info: CustomerInfo,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CustomerInfo {
    card_holder_name: Secret<String>,
    billing_address: BillingAddress,
    shipping_address: Option<ShippingAddress>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BillingAddress {
    name: Secret<String>,
    street: Secret<String>,
    city: String,
    post_code: Secret<String>,
    country: enums::CountryAlpha2,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShippingAddress {
    name: Option<Secret<String>>,
    street: Option<Secret<String>>,
    city: Option<String>,
    post_code: Option<Secret<String>>,
    country: Option<enums::CountryAlpha2>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NexixpayCard {
    pan: CardNumber,
    expiry_date: Secret<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Recurrence {
    action: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PaymentsResponse {
    operation: Operation,
    three_d_s_auth_request: String,
    three_d_s_auth_url: Secret<url::Url>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NexixpayMandateResponse {
    operation: Operation,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(untagged)]
pub enum NexixpayPaymentsResponse {
    PaymentResponse(Box<PaymentsResponse>),
    MandateResponse(Box<NexixpayMandateResponse>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThreeDSAuthResult {
    authentication_value: Option<Secret<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub enum NexixpayPaymentIntent {
    Capture,
    Cancel,
    Authorize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NexixpayRedirectionRequest {
    pub three_d_s_auth_url: String,
    pub three_ds_request: String,
    pub return_url: String,
    pub transaction_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NexixpayConnectorMetaData {
    pub three_d_s_auth_result: Option<ThreeDSAuthResult>,
    pub three_d_s_auth_response: Option<Secret<String>>,
    pub authorization_operation_id: Option<String>,
    pub capture_operation_id: Option<String>,
    pub cancel_operation_id: Option<String>,
    pub psync_flow: NexixpayPaymentIntent,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateNexixpayConnectorMetaData {
    pub three_d_s_auth_result: Option<ThreeDSAuthResult>,
    pub three_d_s_auth_response: Option<Secret<String>>,
    pub authorization_operation_id: Option<String>,
    pub capture_operation_id: Option<String>,
    pub cancel_operation_id: Option<String>,
    pub psync_flow: Option<NexixpayPaymentIntent>,
    pub meta_data: serde_json::Value,
    pub is_auto_capture: bool,
}

fn update_nexi_meta_data(
    update_request: UpdateNexixpayConnectorMetaData,
) -> CustomResult<serde_json::Value, errors::ConnectorError> {
    let nexixpay_meta_data =
        serde_json::from_value::<NexixpayConnectorMetaData>(update_request.meta_data)
            .change_context(errors::ConnectorError::ParsingFailed)?;

    Ok(serde_json::json!(NexixpayConnectorMetaData {
        three_d_s_auth_result: nexixpay_meta_data
            .three_d_s_auth_result
            .or(update_request.three_d_s_auth_result),
        three_d_s_auth_response: nexixpay_meta_data
            .three_d_s_auth_response
            .or(update_request.three_d_s_auth_response),
        authorization_operation_id: nexixpay_meta_data
            .authorization_operation_id
            .clone()
            .or(update_request.authorization_operation_id.clone()),
        capture_operation_id: {
            nexixpay_meta_data
                .capture_operation_id
                .or(if update_request.is_auto_capture {
                    nexixpay_meta_data
                        .authorization_operation_id
                        .or(update_request.authorization_operation_id.clone())
                } else {
                    update_request.capture_operation_id
                })
        },
        cancel_operation_id: nexixpay_meta_data
            .cancel_operation_id
            .or(update_request.cancel_operation_id),
        psync_flow: update_request
            .psync_flow
            .unwrap_or(nexixpay_meta_data.psync_flow)
    }))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThreeDSAuthData {
    three_d_s_auth_response: Option<Secret<String>>,
    authentication_value: Option<Secret<String>>,
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
    payment_method: String,
    warnings: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdditionalData {
    masked_pan: String,
    card_id: Secret<String>,
    card_id4: Option<Secret<String>>,
    card_expiry_date: Option<Secret<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedirectPayload {
    #[serde(rename = "PaRes")]
    pa_res: Option<Secret<String>>,

    #[serde(rename = "paymentId")]
    payment_id: Option<String>,
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
        Ok(Self {
            operation_id: customer_details_encrypted.payment_id,
            three_d_s_auth_response: customer_details_encrypted.pa_res,
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
        let three_ds_data = item.response.three_d_s_auth_result;
        let customer_details_encrypted: RedirectPayload = item
            .data
            .request
            .redirect_response
            .as_ref()
            .and_then(|res| res.payload.to_owned())
            .ok_or(errors::ConnectorError::MissingConnectorRedirectionPayload {
                field_name: "request.redirect_response.payload",
            })?
            .expose()
            .parse_value("RedirectPayload")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        let is_auto_capture = item.data.request.is_auto_capture()?;
        let meta_data = to_connector_meta_from_secret(item.data.request.metadata.clone())?;
        let connector_metadata = Some(update_nexi_meta_data(UpdateNexixpayConnectorMetaData {
            three_d_s_auth_result: Some(three_ds_data),
            three_d_s_auth_response: customer_details_encrypted.pa_res,
            authorization_operation_id: None,
            capture_operation_id: None,
            cancel_operation_id: None,
            psync_flow: None,
            meta_data,
            is_auto_capture,
        })?);

        Ok(Self {
            status: AttemptStatus::from(item.response.operation.operation_result),
            response: Ok(PaymentsResponseData::TransactionResponse {
                resource_id: ResponseId::ConnectorTransactionId(
                    item.response.operation.order_id.clone(),
                ),
                redirection_data: Box::new(None),
                mandate_reference: Box::new(None),
                connector_metadata,
                network_txn_id: None,
                connector_response_reference_id: Some(item.response.operation.order_id),
                incremental_authorization_allowed: None,
                charges: None,
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
        let billing_address_street = format!(
            "{}, {}",
            item.router_data.get_billing_line1()?.expose(),
            item.router_data.get_billing_line2()?.expose()
        );

        let billing_address = BillingAddress {
            name: item.router_data.get_billing_full_name()?,
            street: Secret::new(billing_address_street),
            city: item.router_data.get_billing_city()?,
            post_code: item.router_data.get_billing_zip()?,
            country: item.router_data.get_billing_country()?,
        };
        let shipping_address_street = match (
            item.router_data.get_optional_shipping_line1(),
            item.router_data.get_optional_shipping_line2(),
        ) {
            (Some(line1), Some(line2)) => Some(Secret::new(format!(
                "{}, {}",
                line1.expose(),
                line2.expose()
            ))),
            (Some(line1), None) => Some(Secret::new(line1.expose())),
            (None, Some(line2)) => Some(Secret::new(line2.expose())),
            (None, None) => None,
        };

        let shipping_address = item
            .router_data
            .get_optional_billing()
            .map(|_| ShippingAddress {
                name: item.router_data.get_optional_shipping_full_name(),
                street: shipping_address_street,
                city: item.router_data.get_optional_shipping_city(),
                post_code: item.router_data.get_optional_shipping_zip(),
                country: item.router_data.get_optional_shipping_country(),
            });
        let customer_info = CustomerInfo {
            card_holder_name: item.router_data.get_billing_full_name()?,
            billing_address: billing_address.clone(),
            shipping_address: shipping_address.clone(),
        };
        let order = Order {
            order_id: item.router_data.connector_request_reference_id.clone(),
            amount: item.amount.clone(),
            currency: item.router_data.request.currency,
            description: item.router_data.description.clone(),
            customer_info,
        };
        let payment_data = NexixpayPaymentsRequestData::try_from(item)?;
        Ok(Self {
            order,
            payment_data,
        })
    }
}

impl TryFrom<&NexixpayRouterData<&PaymentsAuthorizeRouterData>> for NexixpayPaymentsRequestData {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &NexixpayRouterData<&PaymentsAuthorizeRouterData>,
    ) -> Result<Self, Self::Error> {
        match item
            .router_data
            .request
            .mandate_id
            .clone()
            .and_then(|mandate_id| mandate_id.mandate_reference_id)
        {
            None => {
                let recurrence_request_obj = if item.router_data.request.is_mandate_payment() {
                    let contract_id = item
                        .router_data
                        .connector_mandate_request_reference_id
                        .clone()
                        .ok_or_else(|| errors::ConnectorError::MissingRequiredField {
                            field_name: "connector_mandate_request_reference_id",
                        })?;
                    Some(RecurrenceRequest {
                        action: NexixpayRecurringAction::ContractCreation,
                        contract_id: Secret::new(contract_id),
                        contract_type: ContractType::MitUnscheduled,
                    })
                } else {
                    None
                };

                match item.router_data.request.payment_method_data {
                    PaymentMethodData::Card(ref req_card) => {
                        if item.router_data.is_three_ds() {
                            Ok(Self::NexixpayNonMandatePaymentRequest(Box::new(
                                NexixpayNonMandatePaymentRequest {
                                    card: NexixpayCard {
                                        pan: req_card.card_number.clone(),
                                        expiry_date: req_card.get_expiry_date_as_mmyy()?,
                                    },
                                    recurrence: recurrence_request_obj,
                                },
                            )))
                        } else {
                            Err(errors::ConnectorError::NotSupported {
                                message: "No threeds is not supported".to_string(),
                                connector: "nexixpay",
                            }
                            .into())
                        }
                    }
                    PaymentMethodData::CardRedirect(_)
                    | PaymentMethodData::Wallet(_)
                    | PaymentMethodData::PayLater(_)
                    | PaymentMethodData::BankRedirect(_)
                    | PaymentMethodData::BankDebit(_)
                    | PaymentMethodData::BankTransfer(_)
                    | PaymentMethodData::Crypto(_)
                    | PaymentMethodData::MandatePayment
                    | PaymentMethodData::Reward
                    | PaymentMethodData::RealTimePayment(_)
                    | PaymentMethodData::Upi(_)
                    | PaymentMethodData::MobilePayment(_)
                    | PaymentMethodData::Voucher(_)
                    | PaymentMethodData::GiftCard(_)
                    | PaymentMethodData::OpenBanking(_)
                    | PaymentMethodData::CardToken(_)
                    | PaymentMethodData::CardDetailsForNetworkTransactionId(_)
                    | PaymentMethodData::NetworkToken(_) => {
                        Err(errors::ConnectorError::NotImplemented(
                            get_unimplemented_payment_method_error_message("nexixpay"),
                        ))?
                    }
                }
            }
            Some(api_models::payments::MandateReferenceId::ConnectorMandateId(mandate_data)) => {
                let contract_id = Secret::new(
                    mandate_data
                        .get_connector_mandate_request_reference_id()
                        .ok_or(errors::ConnectorError::MissingConnectorMandateID)?,
                );
                let capture_type =
                    get_nexixpay_capture_type(item.router_data.request.capture_method)?;
                Ok(Self::NexixpayMandatePaymentRequest(Box::new(
                    NexixpayMandatePaymentRequest {
                        contract_id,
                        capture_type,
                    },
                )))
            }
            Some(api_models::payments::MandateReferenceId::NetworkTokenWithNTI(_))
            | Some(api_models::payments::MandateReferenceId::NetworkMandateId(_)) => {
                Err(errors::ConnectorError::NotImplemented(
                    get_unimplemented_payment_method_error_message("nexixpay"),
                )
                .into())
            }
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
    Executed,
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
pub struct NexixpayPaymentsCancelRequest {
    description: Option<String>,
    amount: StringMinorUnit,
    currency: Currency,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NexixpayOperationResponse {
    operation_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NexixpayRefundRequest {
    pub amount: StringMinorUnit,
    pub currency: Currency,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RefundResponse {
    operation_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NexixpayErrorBody {
    pub code: Option<String>,
    pub description: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
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
            NexixpayPaymentStatus::Authorized => Self::Authorized,
            NexixpayPaymentStatus::ThreedsValidated => Self::AuthenticationSuccessful,
            NexixpayPaymentStatus::Executed => Self::Charged,
            NexixpayPaymentStatus::Pending => Self::AuthenticationPending, // this is being used in authorization calls only.
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
        Some(CaptureMethod::Automatic) | Some(CaptureMethod::SequentialAutomatic) | None => {
            Ok(Some(NexixpayCaptureType::Implicit))
        }
        Some(item) => Err(errors::ConnectorError::FlowNotSupported {
            flow: item.to_string(),
            connector: "Nexixpay".to_string(),
        }
        .into()),
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
        match item.response {
            NexixpayPaymentsResponse::PaymentResponse(ref response_body) => {
                let complete_authorize_url = item.data.request.get_complete_authorize_url()?;
                let operation_id: String = response_body.operation.operation_id.clone();
                let redirection_form = nexixpay_threeds_link(NexixpayRedirectionRequest {
                    three_d_s_auth_url: response_body
                        .three_d_s_auth_url
                        .clone()
                        .expose()
                        .to_string(),
                    three_ds_request: response_body.three_d_s_auth_request.clone(),
                    return_url: complete_authorize_url.clone(),
                    transaction_id: operation_id.clone(),
                })?;
                let is_auto_capture = item.data.request.is_auto_capture()?;
                let connector_metadata = Some(serde_json::json!(NexixpayConnectorMetaData {
                    three_d_s_auth_result: None,
                    three_d_s_auth_response: None,
                    authorization_operation_id: Some(operation_id.clone()),
                    cancel_operation_id: None,
                    capture_operation_id: {
                        if is_auto_capture {
                            Some(operation_id)
                        } else {
                            None
                        }
                    },
                    psync_flow: NexixpayPaymentIntent::Authorize
                }));
                Ok(Self {
                    status: AttemptStatus::from(response_body.operation.operation_result.clone()),
                    response: Ok(PaymentsResponseData::TransactionResponse {
                        resource_id: ResponseId::ConnectorTransactionId(
                            response_body.operation.order_id.clone(),
                        ),
                        redirection_data: Box::new(Some(redirection_form.clone())),
                        mandate_reference: Box::new(Some(MandateReference {
                            connector_mandate_id: item
                                .data
                                .connector_mandate_request_reference_id
                                .clone(),
                            payment_method_id: None,
                            mandate_metadata: None,
                            connector_mandate_request_reference_id: None,
                        })),
                        connector_metadata,
                        network_txn_id: None,
                        connector_response_reference_id: Some(
                            response_body.operation.order_id.clone(),
                        ),
                        incremental_authorization_allowed: None,
                        charges: None,
                    }),
                    ..item.data
                })
            }
            NexixpayPaymentsResponse::MandateResponse(ref mandate_response) => Ok(Self {
                status: AttemptStatus::from(mandate_response.operation.operation_result.clone()),
                response: Ok(PaymentsResponseData::TransactionResponse {
                    resource_id: ResponseId::ConnectorTransactionId(
                        mandate_response.operation.order_id.clone(),
                    ),
                    redirection_data: Box::new(None),
                    mandate_reference: Box::new(None),
                    connector_metadata: None,
                    network_txn_id: None,
                    connector_response_reference_id: Some(
                        mandate_response.operation.order_id.clone(),
                    ),
                    incremental_authorization_allowed: None,
                    charges: None,
                }),
                ..item.data
            }),
        }
    }
}

fn nexixpay_threeds_link(
    request: NexixpayRedirectionRequest,
) -> CustomResult<RedirectForm, errors::ConnectorError> {
    let mut form_fields = HashMap::<String, String>::new();
    form_fields.insert(String::from("ThreeDsRequest"), request.three_ds_request);
    form_fields.insert(String::from("ReturnUrl"), request.return_url);
    form_fields.insert(String::from("transactionId"), request.transaction_id);

    Ok(RedirectForm::Form {
        endpoint: request.three_d_s_auth_url,
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
            NexixpayRefundResultStatus::Voided
            | NexixpayRefundResultStatus::Refunded
            | NexixpayRefundResultStatus::Executed => Self::Success,
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
                refund_status: RefundStatus::Pending, // Refund call do not return status in their response.
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
        let is_auto_capture = item.data.request.is_auto_capture()?;
        let meta_data = to_connector_meta(item.data.request.connector_meta.clone())?;
        let connector_metadata = Some(update_nexi_meta_data(UpdateNexixpayConnectorMetaData {
            three_d_s_auth_result: None,
            three_d_s_auth_response: None,
            authorization_operation_id: Some(item.response.operation.operation_id.clone()),
            capture_operation_id: None,
            cancel_operation_id: None,
            psync_flow: Some(NexixpayPaymentIntent::Authorize),
            meta_data,
            is_auto_capture,
        })?);
        Ok(Self {
            status: AttemptStatus::from(item.response.operation.operation_result),
            response: Ok(PaymentsResponseData::TransactionResponse {
                resource_id: ResponseId::ConnectorTransactionId(
                    item.response.operation.order_id.clone(),
                ),
                redirection_data: Box::new(None),
                mandate_reference: Box::new(Some(MandateReference {
                    connector_mandate_id: item.data.connector_mandate_request_reference_id.clone(),
                    payment_method_id: None,
                    mandate_metadata: None,
                    connector_mandate_request_reference_id: None,
                })),
                connector_metadata,
                network_txn_id: None,
                connector_response_reference_id: Some(item.response.operation.order_id),
                incremental_authorization_allowed: None,
                charges: None,
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
        let capture_type = get_nexixpay_capture_type(item.router_data.request.capture_method)?;

        let order_id = item.router_data.connector_request_reference_id.clone();
        let amount = item.amount.clone();
        let billing_address_street = format!(
            "{}, {}",
            item.router_data.get_billing_line1()?.expose(),
            item.router_data.get_billing_line2()?.expose()
        );

        let billing_address = BillingAddress {
            name: item.router_data.get_billing_full_name()?,
            street: Secret::new(billing_address_street),
            city: item.router_data.get_billing_city()?,
            post_code: item.router_data.get_billing_zip()?,
            country: item.router_data.get_billing_country()?,
        };
        let shipping_address_street = match (
            item.router_data.get_optional_shipping_line1(),
            item.router_data.get_optional_shipping_line2(),
        ) {
            (Some(line1), Some(line2)) => Some(Secret::new(format!(
                "{}, {}",
                line1.expose(),
                line2.expose()
            ))),
            (Some(line1), None) => Some(Secret::new(line1.expose())),
            (None, Some(line2)) => Some(Secret::new(line2.expose())),
            (None, None) => None,
        };

        let shipping_address = item
            .router_data
            .get_optional_billing()
            .map(|_| ShippingAddress {
                name: item.router_data.get_optional_shipping_full_name(),
                street: shipping_address_street,
                city: item.router_data.get_optional_shipping_city(),
                post_code: item.router_data.get_optional_shipping_zip(),
                country: item.router_data.get_optional_shipping_country(),
            });
        let customer_info = CustomerInfo {
            card_holder_name: item.router_data.get_billing_full_name()?,
            billing_address: billing_address.clone(),
            shipping_address: shipping_address.clone(),
        };
        let order_data = Order {
            order_id,
            amount,
            currency: item.router_data.request.currency,
            description: item.router_data.description.clone(),
            customer_info,
        };
        let connector_metadata =
            to_connector_meta(item.router_data.request.connector_meta.clone())?;
        let nexixpay_meta_data =
            serde_json::from_value::<NexixpayConnectorMetaData>(connector_metadata)
                .change_context(errors::ConnectorError::ParsingFailed)?;
        let operation_id = nexixpay_meta_data.authorization_operation_id.ok_or(
            errors::ConnectorError::MissingRequiredField {
                field_name: "authorization_operation_id",
            },
        )?;
        let authentication_value = nexixpay_meta_data
            .three_d_s_auth_result
            .and_then(|data| data.authentication_value);
        let three_d_s_auth_data = ThreeDSAuthData {
            three_d_s_auth_response: nexixpay_meta_data.three_d_s_auth_response,
            authentication_value,
        };
        let card: Result<NexixpayCard, error_stack::Report<errors::ConnectorError>> =
            match payment_method_data {
                PaymentMethodData::Card(req_card) => Ok(NexixpayCard {
                    pan: req_card.card_number.clone(),
                    expiry_date: req_card.get_expiry_date_as_mmyy()?,
                }),
                PaymentMethodData::CardRedirect(_)
                | PaymentMethodData::Wallet(_)
                | PaymentMethodData::PayLater(_)
                | PaymentMethodData::BankRedirect(_)
                | PaymentMethodData::BankDebit(_)
                | PaymentMethodData::BankTransfer(_)
                | PaymentMethodData::Crypto(_)
                | PaymentMethodData::MandatePayment
                | PaymentMethodData::Reward
                | PaymentMethodData::RealTimePayment(_)
                | PaymentMethodData::MobilePayment(_)
                | PaymentMethodData::Upi(_)
                | PaymentMethodData::Voucher(_)
                | PaymentMethodData::GiftCard(_)
                | PaymentMethodData::OpenBanking(_)
                | PaymentMethodData::CardToken(_)
                | PaymentMethodData::NetworkToken(_)
                | PaymentMethodData::CardDetailsForNetworkTransactionId(_) => {
                    Err(errors::ConnectorError::NotImplemented(
                        get_unimplemented_payment_method_error_message("nexixpay"),
                    )
                    .into())
                }
            };
        let contract_id = Secret::new(
            item.router_data
                .connector_mandate_request_reference_id
                .clone()
                .ok_or_else(|| errors::ConnectorError::MissingRequiredField {
                    field_name: "connector_mandate_request_reference_id",
                })?,
        );
        Ok(Self {
            order: order_data,
            card: card?,
            operation_id,
            capture_type,
            three_d_s_auth_data,
            recurrence: Some(RecurrenceRequest {
                action: NexixpayRecurringAction::ContractCreation,
                contract_id,
                contract_type: ContractType::MitUnscheduled,
            }),
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
                resource_id: ResponseId::ConnectorTransactionId(item.response.order_id.clone()),
                redirection_data: Box::new(None),
                mandate_reference: Box::new(Some(MandateReference {
                    connector_mandate_id: item.data.connector_mandate_request_reference_id.clone(),
                    payment_method_id: None,
                    mandate_metadata: None,
                    connector_mandate_request_reference_id: None,
                })),
                connector_metadata: item.data.request.connector_meta.clone(),
                network_txn_id: None,
                connector_response_reference_id: Some(item.response.order_id.clone()),
                incremental_authorization_allowed: None,
                charges: None,
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

impl<F>
    TryFrom<
        ResponseRouterData<F, NexixpayOperationResponse, PaymentsCaptureData, PaymentsResponseData>,
    > for RouterData<F, PaymentsCaptureData, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<
            F,
            NexixpayOperationResponse,
            PaymentsCaptureData,
            PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        let meta_data = to_connector_meta(item.data.request.connector_meta.clone())?;
        let connector_metadata = Some(update_nexi_meta_data(UpdateNexixpayConnectorMetaData {
            three_d_s_auth_result: None,
            three_d_s_auth_response: None,
            authorization_operation_id: None,
            capture_operation_id: Some(item.response.operation_id.clone()),
            cancel_operation_id: None,
            psync_flow: Some(NexixpayPaymentIntent::Capture),
            meta_data,
            is_auto_capture: false,
        })?);
        Ok(Self {
            status: AttemptStatus::Pending, // Capture call do not return status in their response.
            response: Ok(PaymentsResponseData::TransactionResponse {
                resource_id: ResponseId::ConnectorTransactionId(
                    item.data.request.connector_transaction_id.clone(),
                ),
                redirection_data: Box::new(None),
                mandate_reference: Box::new(None),
                connector_metadata,
                network_txn_id: None,
                connector_response_reference_id: Some(
                    item.data.request.connector_transaction_id.clone(),
                ),
                incremental_authorization_allowed: None,
                charges: None,
            }),
            ..item.data
        })
    }
}

impl TryFrom<NexixpayRouterData<&PaymentsCancelRouterData>> for NexixpayPaymentsCancelRequest {
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

impl<F>
    TryFrom<
        ResponseRouterData<F, NexixpayOperationResponse, PaymentsCancelData, PaymentsResponseData>,
    > for RouterData<F, PaymentsCancelData, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<
            F,
            NexixpayOperationResponse,
            PaymentsCancelData,
            PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        let meta_data = to_connector_meta(item.data.request.connector_meta.clone())?;
        let connector_metadata = Some(update_nexi_meta_data(UpdateNexixpayConnectorMetaData {
            three_d_s_auth_result: None,
            three_d_s_auth_response: None,
            authorization_operation_id: None,
            capture_operation_id: None,
            cancel_operation_id: Some(item.response.operation_id.clone()),
            psync_flow: Some(NexixpayPaymentIntent::Cancel),
            meta_data,
            is_auto_capture: false,
        })?);
        Ok(Self {
            status: AttemptStatus::Pending, // Cancel call do not return status in their response.
            response: Ok(PaymentsResponseData::TransactionResponse {
                resource_id: ResponseId::ConnectorTransactionId(
                    item.data.request.connector_transaction_id.clone(),
                ),
                redirection_data: Box::new(None),
                mandate_reference: Box::new(None),
                connector_metadata,
                network_txn_id: None,
                connector_response_reference_id: Some(
                    item.data.request.connector_transaction_id.clone(),
                ),
                incremental_authorization_allowed: None,
                charges: None,
            }),
            ..item.data
        })
    }
}
