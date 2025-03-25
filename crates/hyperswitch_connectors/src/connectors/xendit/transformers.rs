use std::collections::HashMap;

use cards::CardNumber;
use common_enums::enums;
use common_utils::{pii, request::Method, types::FloatMajorUnit};
use hyperswitch_domain_models::{
    payment_method_data::PaymentMethodData,
    router_data::{ConnectorAuthType, ErrorResponse, RouterData},
    router_flow_types::refunds::{Execute, RSync},
    router_request_types::{
        PaymentsAuthorizeData, PaymentsCaptureData, PaymentsPreProcessingData, ResponseId,
    },
    router_response_types::{
        MandateReference, PaymentsResponseData, RedirectForm, RefundsResponseData,
    },
    types::{
        PaymentsAuthorizeRouterData, PaymentsCaptureRouterData, PaymentsPreProcessingRouterData,
        PaymentsSyncRouterData, RefundsRouterData,
    },
};
use hyperswitch_interfaces::{
    consts::{NO_ERROR_CODE, NO_ERROR_MESSAGE},
    errors,
};
use masking::{ExposeInterface, PeekInterface, Secret};
use serde::{Deserialize, Serialize};

use crate::{
    types::{PaymentsSyncResponseRouterData, RefundsResponseRouterData, ResponseRouterData},
    utils::{
        get_unimplemented_payment_method_error_message, CardData, PaymentsAuthorizeRequestData,
        PaymentsSyncRequestData, RouterData as OtherRouterData,
    },
};

//TODO: Fill the struct with respective fields
pub struct XenditRouterData<T> {
    pub amount: FloatMajorUnit, // The type of amount that a connector accepts, for example, String, i64, f64, etc.
    pub router_data: T,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum PaymentMethodType {
    CARD,
}
#[derive(Serialize, Deserialize, Debug)]
pub struct ChannelProperties {
    pub success_return_url: String,
    pub failure_return_url: String,
    pub skip_three_d_secure: bool,
}
#[derive(Serialize, Deserialize, Debug)]
#[serde(untagged)]
pub enum PaymentMethod {
    Card(CardPaymentRequest),
}
#[derive(Serialize, Deserialize, Debug)]
pub struct CardPaymentRequest {
    #[serde(rename = "type")]
    pub payment_type: PaymentMethodType,
    pub card: CardInfo,
    pub reusability: TransactionType,
    pub reference_id: Secret<String>,
}
#[derive(Serialize, Deserialize, Debug)]
pub struct MandatePaymentRequest {
    pub amount: FloatMajorUnit,
    pub currency: common_enums::Currency,
    pub capture_method: String,
    pub payment_method_id: Secret<String>,
    pub channel_properties: ChannelProperties,
}
#[derive(Serialize, Deserialize, Debug)]
pub struct XenditRedirectionResponse {
    pub status: PaymentStatus,
}
#[derive(Serialize, Deserialize, Debug)]
pub struct XenditPaymentsCaptureRequest {
    pub capture_amount: FloatMajorUnit,
}
#[derive(Serialize, Deserialize, Debug)]
pub struct XenditPaymentsRequest {
    pub amount: FloatMajorUnit,
    pub currency: common_enums::Currency,
    pub capture_method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payment_method: Option<PaymentMethod>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payment_method_id: Option<Secret<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub channel_properties: Option<ChannelProperties>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct XenditSplitRoute {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub flat_amount: Option<FloatMajorUnit>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub percent_amount: Option<i64>,
    pub currency: enums::Currency,
    pub destination_account_id: String,
    pub reference_id: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct XenditSplitRequest {
    pub name: String,
    pub description: String,
    pub routes: Vec<XenditSplitRoute>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct XenditSplitRequestData {
    #[serde(flatten)]
    pub split_data: XenditSplitRequest,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct XenditSplitResponse {
    id: String,
    name: String,
    description: String,
    routes: Vec<XenditSplitRoute>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CardInfo {
    pub channel_properties: ChannelProperties,
    pub card_information: CardInformation,
}
#[derive(Serialize, Deserialize, Debug)]
pub struct CardInformation {
    pub card_number: CardNumber,
    pub expiry_month: Secret<String>,
    pub expiry_year: Secret<String>,
    pub cvv: Secret<String>,
    pub cardholder_name: Secret<String>,
    pub cardholder_email: pii::Email,
    pub cardholder_phone_number: Secret<String>,
}
pub mod auth_headers {
    pub const WITH_SPLIT_RULE: &str = "with-split-rule";
    pub const FOR_USER_ID: &str = "for-user-id";
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TransactionType {
    OneTimeUse,
    MultipleUse,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct XenditErrorResponse {
    pub error_code: String,
    pub message: String,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PaymentStatus {
    Pending,
    RequiresAction,
    Failed,
    Succeeded,
    AwaitingCapture,
    Verified,
}
#[derive(Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum XenditResponse {
    Payment(XenditPaymentResponse),
    Webhook(XenditWebhookEvent),
}
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct XenditPaymentResponse {
    pub id: String,
    pub status: PaymentStatus,
    pub actions: Option<Vec<Action>>,
    pub payment_method: PaymentMethodInfo,
    pub failure_code: Option<String>,
    pub reference_id: Secret<String>,
}

fn map_payment_response_to_attempt_status(
    response: XenditPaymentResponse,
    is_auto_capture: bool,
) -> enums::AttemptStatus {
    match response.status {
        PaymentStatus::Failed => enums::AttemptStatus::Failure,
        PaymentStatus::Succeeded | PaymentStatus::Verified => {
            if is_auto_capture {
                enums::AttemptStatus::Charged
            } else {
                enums::AttemptStatus::Authorized
            }
        }
        PaymentStatus::Pending => enums::AttemptStatus::Pending,
        PaymentStatus::RequiresAction => enums::AttemptStatus::AuthenticationPending,
        PaymentStatus::AwaitingCapture => enums::AttemptStatus::Authorized,
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum MethodType {
    Get,
    Post,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Action {
    pub method: MethodType,
    pub url: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentMethodInfo {
    pub id: Secret<String>,
}
impl TryFrom<XenditRouterData<&PaymentsAuthorizeRouterData>> for XenditPaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: XenditRouterData<&PaymentsAuthorizeRouterData>) -> Result<Self, Self::Error> {
        match item.router_data.request.payment_method_data.clone() {
            PaymentMethodData::Card(card_data) => Ok(Self {
                capture_method: match item.router_data.request.is_auto_capture()? {
                    true => "AUTOMATIC".to_string(),
                    false => "MANUAL".to_string(),
                },
                currency: item.router_data.request.currency,
                amount: item.amount,
                payment_method: Some(PaymentMethod::Card(CardPaymentRequest {
                    payment_type: PaymentMethodType::CARD,
                    reference_id: Secret::new(
                        item.router_data.connector_request_reference_id.clone(),
                    ),
                    card: CardInfo {
                        channel_properties: ChannelProperties {
                            success_return_url: item.router_data.request.get_router_return_url()?,
                            failure_return_url: item.router_data.request.get_router_return_url()?,
                            skip_three_d_secure: !item.router_data.is_three_ds(),
                        },
                        card_information: CardInformation {
                            card_number: card_data.card_number.clone(),
                            expiry_month: card_data.card_exp_month.clone(),
                            expiry_year: card_data.get_expiry_year_4_digit(),
                            cvv: card_data.card_cvc.clone(),
                            cardholder_name: card_data
                                .get_cardholder_name()
                                .or(item.router_data.get_billing_full_name())?,
                            cardholder_email: item
                                .router_data
                                .get_billing_email()
                                .or(item.router_data.request.get_email())?,
                            cardholder_phone_number: item.router_data.get_billing_phone_number()?,
                        },
                    },
                    reusability: match item.router_data.request.is_mandate_payment() {
                        true => TransactionType::MultipleUse,
                        false => TransactionType::OneTimeUse,
                    },
                })),
                payment_method_id: None,
                channel_properties: None,
            }),
            PaymentMethodData::MandatePayment => Ok(Self {
                channel_properties: Some(ChannelProperties {
                    success_return_url: item.router_data.request.get_router_return_url()?,
                    failure_return_url: item.router_data.request.get_router_return_url()?,
                    skip_three_d_secure: true,
                }),
                capture_method: match item.router_data.request.is_auto_capture()? {
                    true => "AUTOMATIC".to_string(),
                    false => "MANUAL".to_string(),
                },
                currency: item.router_data.request.currency,
                amount: item.amount,
                payment_method_id: Some(Secret::new(
                    item.router_data.request.get_connector_mandate_id()?,
                )),
                payment_method: None,
            }),
            _ => Err(errors::ConnectorError::NotImplemented(
                get_unimplemented_payment_method_error_message("xendit"),
            )
            .into()),
        }
    }
}
impl TryFrom<XenditRouterData<&PaymentsCaptureRouterData>> for XenditPaymentsCaptureRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: XenditRouterData<&PaymentsCaptureRouterData>) -> Result<Self, Self::Error> {
        Ok(Self {
            capture_amount: item.amount,
        })
    }
}
impl<F>
    TryFrom<
        ResponseRouterData<F, XenditPaymentResponse, PaymentsAuthorizeData, PaymentsResponseData>,
    > for RouterData<F, PaymentsAuthorizeData, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(
        item: ResponseRouterData<
            F,
            XenditPaymentResponse,
            PaymentsAuthorizeData,
            PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        let status = map_payment_response_to_attempt_status(
            item.response.clone(),
            item.data.request.is_auto_capture()?,
        );
        let response = if status == enums::AttemptStatus::Failure {
            Err(ErrorResponse {
                code: item
                    .response
                    .failure_code
                    .clone()
                    .unwrap_or_else(|| NO_ERROR_CODE.to_string()),
                message: item
                    .response
                    .failure_code
                    .clone()
                    .unwrap_or_else(|| NO_ERROR_MESSAGE.to_string()),
                reason: Some(
                    item.response
                        .failure_code
                        .unwrap_or_else(|| NO_ERROR_MESSAGE.to_string()),
                ),
                attempt_status: None,
                connector_transaction_id: Some(item.response.id.clone()),
                status_code: item.http_code,
                issuer_error_code: None,
                issuer_error_message: None,
            })
        } else {
            let charges = match item.data.request.split_payments.as_ref() {
                Some(common_types::payments::SplitPaymentsRequest::XenditSplitPayment(
                    common_types::payments::XenditSplitRequest::MultipleSplits(_),
                )) => item
                    .data
                    .response
                    .as_ref()
                    .ok()
                    .and_then(|response| match response {
                        PaymentsResponseData::TransactionResponse { charges, .. } => {
                            charges.clone()
                        }
                        _ => None,
                    }),
                Some(common_types::payments::SplitPaymentsRequest::XenditSplitPayment(
                    common_types::payments::XenditSplitRequest::SingleSplit(ref split_data),
                )) => {
                    let charges = common_types::domain::XenditSplitSubMerchantData {
                        for_user_id: split_data.for_user_id.clone(),
                    };
                    Some(
                        common_types::payments::ConnectorChargeResponseData::XenditSplitPayment(
                            common_types::payments::XenditChargeResponseData::SingleSplit(charges),
                        ),
                    )
                }
                _ => None,
            };

            Ok(PaymentsResponseData::TransactionResponse {
                resource_id: ResponseId::ConnectorTransactionId(item.response.id.clone()),
                redirection_data: match item.response.actions {
                    Some(actions) if !actions.is_empty() => {
                        actions.first().map_or(Box::new(None), |single_action| {
                            Box::new(Some(RedirectForm::Form {
                                endpoint: single_action.url.clone(),
                                method: match single_action.method {
                                    MethodType::Get => Method::Get,
                                    MethodType::Post => Method::Post,
                                },
                                form_fields: HashMap::new(),
                            }))
                        })
                    }
                    _ => Box::new(None),
                },
                mandate_reference: match item.data.request.is_mandate_payment() {
                    true => Box::new(Some(MandateReference {
                        connector_mandate_id: Some(item.response.payment_method.id.expose()),
                        payment_method_id: None,
                        mandate_metadata: None,
                        connector_mandate_request_reference_id: None,
                    })),
                    false => Box::new(None),
                },
                connector_metadata: None,
                network_txn_id: None,
                connector_response_reference_id: Some(
                    item.response.reference_id.peek().to_string(),
                ),
                incremental_authorization_allowed: None,
                charges,
            })
        };
        Ok(Self {
            status,
            response,
            ..item.data
        })
    }
}

impl<F>
    TryFrom<ResponseRouterData<F, XenditPaymentResponse, PaymentsCaptureData, PaymentsResponseData>>
    for RouterData<F, PaymentsCaptureData, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(
        item: ResponseRouterData<
            F,
            XenditPaymentResponse,
            PaymentsCaptureData,
            PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        let status = map_payment_response_to_attempt_status(item.response.clone(), true);
        let response = if status == enums::AttemptStatus::Failure {
            Err(ErrorResponse {
                code: item
                    .response
                    .failure_code
                    .clone()
                    .unwrap_or_else(|| NO_ERROR_CODE.to_string()),
                message: item
                    .response
                    .failure_code
                    .clone()
                    .unwrap_or_else(|| NO_ERROR_MESSAGE.to_string()),
                reason: Some(
                    item.response
                        .failure_code
                        .unwrap_or_else(|| NO_ERROR_MESSAGE.to_string()),
                ),
                attempt_status: None,
                connector_transaction_id: None,
                status_code: item.http_code,
                issuer_error_code: None,
                issuer_error_message: None,
            })
        } else {
            Ok(PaymentsResponseData::TransactionResponse {
                resource_id: ResponseId::NoResponseId,
                redirection_data: Box::new(None),
                mandate_reference: Box::new(None),
                connector_metadata: None,
                network_txn_id: None,
                connector_response_reference_id: Some(
                    item.response.reference_id.peek().to_string(),
                ),
                incremental_authorization_allowed: None,
                charges: None,
            })
        };
        Ok(Self {
            status,
            response,
            ..item.data
        })
    }
}

impl<F>
    TryFrom<
        ResponseRouterData<F, XenditSplitResponse, PaymentsPreProcessingData, PaymentsResponseData>,
    > for RouterData<F, PaymentsPreProcessingData, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(
        item: ResponseRouterData<
            F,
            XenditSplitResponse,
            PaymentsPreProcessingData,
            PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        let for_user_id = match item.data.request.split_payments {
            Some(common_types::payments::SplitPaymentsRequest::XenditSplitPayment(
                common_types::payments::XenditSplitRequest::MultipleSplits(ref split_data),
            )) => split_data.for_user_id.clone(),
            _ => None,
        };

        let routes: Vec<common_types::payments::XenditSplitRoute> = item
            .response
            .routes
            .iter()
            .map(|route| {
                let required_conversion_type = common_utils::types::FloatMajorUnitForConnector;
                route
                    .flat_amount
                    .map(|amount| {
                        common_utils::types::AmountConvertor::convert_back(
                            &required_conversion_type,
                            amount,
                            item.data.request.currency.unwrap_or(enums::Currency::USD),
                        )
                        .map_err(|_| {
                            errors::ConnectorError::RequestEncodingFailedWithReason(
                                "Failed to convert the amount into a major unit".to_owned(),
                            )
                        })
                    })
                    .transpose()
                    .map(|flat_amount| common_types::payments::XenditSplitRoute {
                        flat_amount,
                        percent_amount: route.percent_amount,
                        currency: route.currency,
                        destination_account_id: route.destination_account_id.clone(),
                        reference_id: route.reference_id.clone(),
                    })
            })
            .collect::<Result<Vec<_>, _>>()?;

        let charges = common_types::payments::XenditMultipleSplitResponse {
            split_rule_id: item.response.id,
            for_user_id,
            name: item.response.name,
            description: item.response.description,
            routes,
        };

        let response = PaymentsResponseData::TransactionResponse {
            resource_id: ResponseId::NoResponseId,
            redirection_data: Box::new(None),
            mandate_reference: Box::new(None),
            connector_metadata: None,
            network_txn_id: None,
            connector_response_reference_id: None,
            incremental_authorization_allowed: None,
            charges: Some(
                common_types::payments::ConnectorChargeResponseData::XenditSplitPayment(
                    common_types::payments::XenditChargeResponseData::MultipleSplits(charges),
                ),
            ),
        };

        Ok(Self {
            response: Ok(response),
            ..item.data
        })
    }
}

impl TryFrom<PaymentsSyncResponseRouterData<XenditResponse>> for PaymentsSyncRouterData {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: PaymentsSyncResponseRouterData<XenditResponse>) -> Result<Self, Self::Error> {
        match item.response {
            XenditResponse::Payment(payment_response) => {
                let status = map_payment_response_to_attempt_status(
                    payment_response.clone(),
                    item.data.request.is_auto_capture()?,
                );
                let response = if status == enums::AttemptStatus::Failure {
                    Err(ErrorResponse {
                        code: payment_response
                            .failure_code
                            .clone()
                            .unwrap_or_else(|| NO_ERROR_CODE.to_string()),
                        message: payment_response
                            .failure_code
                            .clone()
                            .unwrap_or_else(|| NO_ERROR_MESSAGE.to_string()),
                        reason: Some(
                            payment_response
                                .failure_code
                                .unwrap_or_else(|| NO_ERROR_MESSAGE.to_string()),
                        ),
                        attempt_status: None,
                        connector_transaction_id: Some(payment_response.id.clone()),
                        status_code: item.http_code,
                        issuer_error_code: None,
                        issuer_error_message: None,
                    })
                } else {
                    Ok(PaymentsResponseData::TransactionResponse {
                        resource_id: ResponseId::NoResponseId,
                        redirection_data: Box::new(None),
                        mandate_reference: Box::new(None),
                        connector_metadata: None,
                        network_txn_id: None,
                        connector_response_reference_id: None,
                        incremental_authorization_allowed: None,
                        charges: None,
                    })
                };
                Ok(Self {
                    status,
                    response,
                    ..item.data
                })
            }
            XenditResponse::Webhook(webhook_event) => {
                let status = match webhook_event.event {
                    XenditEventType::PaymentSucceeded | XenditEventType::CaptureSucceeded => {
                        enums::AttemptStatus::Charged
                    }
                    XenditEventType::PaymentAwaitingCapture => enums::AttemptStatus::Authorized,
                    XenditEventType::PaymentFailed | XenditEventType::CaptureFailed => {
                        enums::AttemptStatus::Failure
                    }
                };
                Ok(Self {
                    status,
                    ..item.data
                })
            }
        }
    }
}
impl<T> From<(FloatMajorUnit, T)> for XenditRouterData<T> {
    fn from((amount, item): (FloatMajorUnit, T)) -> Self {
        Self {
            amount,
            router_data: item,
        }
    }
}
pub struct XenditAuthType {
    pub(super) api_key: Secret<String>,
}

impl TryFrom<&ConnectorAuthType> for XenditAuthType {
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

impl TryFrom<&PaymentsPreProcessingRouterData> for XenditSplitRequestData {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &PaymentsPreProcessingRouterData) -> Result<Self, Self::Error> {
        if let Some(common_types::payments::SplitPaymentsRequest::XenditSplitPayment(
            common_types::payments::XenditSplitRequest::MultipleSplits(ref split_data),
        )) = item.request.split_payments.clone()
        {
            let routes: Vec<XenditSplitRoute> = split_data
                .routes
                .iter()
                .map(|route| {
                    let required_conversion_type = common_utils::types::FloatMajorUnitForConnector;
                    route
                        .flat_amount
                        .map(|amount| {
                            common_utils::types::AmountConvertor::convert(
                                &required_conversion_type,
                                amount,
                                item.request.currency.unwrap_or(enums::Currency::USD),
                            )
                            .map_err(|_| {
                                errors::ConnectorError::RequestEncodingFailedWithReason(
                                    "Failed to convert the amount into a major unit".to_owned(),
                                )
                            })
                        })
                        .transpose()
                        .map(|flat_amount| XenditSplitRoute {
                            flat_amount,
                            percent_amount: route.percent_amount,
                            currency: route.currency,
                            destination_account_id: route.destination_account_id.clone(),
                            reference_id: route.reference_id.clone(),
                        })
                })
                .collect::<Result<Vec<_>, _>>()?;

            let split_data = XenditSplitRequest {
                name: split_data.name.clone(),
                description: split_data.description.clone(),
                routes,
            };

            Ok(Self { split_data })
        } else {
            Err(errors::ConnectorError::NotImplemented(
                get_unimplemented_payment_method_error_message("Xendit"),
            )
            .into())
        }
    }
}

//TODO: Fill the struct with respective fields
// REFUND :
// Type definition for RefundRequest
#[derive(Default, Debug, Serialize)]
pub struct XenditRefundRequest {
    pub amount: FloatMajorUnit,
    pub payment_request_id: String,
    pub reason: String,
}

impl<F> TryFrom<&XenditRouterData<&RefundsRouterData<F>>> for XenditRefundRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &XenditRouterData<&RefundsRouterData<F>>) -> Result<Self, Self::Error> {
        Ok(Self {
            amount: item.amount.to_owned(),
            payment_request_id: item.router_data.request.connector_transaction_id.clone(),
            reason: "REQUESTED_BY_CUSTOMER".to_string(),
        })
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RefundStatus {
    RequiresAction,
    Succeeded,
    Failed,
    Pending,
    Cancelled,
}

impl From<RefundStatus> for enums::RefundStatus {
    fn from(item: RefundStatus) -> Self {
        match item {
            RefundStatus::Succeeded => Self::Success,
            RefundStatus::Failed | RefundStatus::Cancelled => Self::Failure,
            RefundStatus::Pending | RefundStatus::RequiresAction => Self::Pending,
        }
    }
}

//TODO: Fill the struct with respective fields
#[derive(Debug, Clone, Serialize, Deserialize)]
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
                connector_refund_id: item.response.id,
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

#[derive(Default, Debug, Serialize, Deserialize)]
pub struct XenditMetadata {
    pub for_user_id: String,
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct XenditWebhookEvent {
    pub event: XenditEventType,
    pub data: EventDetails,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EventDetails {
    pub id: String,
    pub payment_request_id: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum XenditEventType {
    #[serde(rename = "payment.succeeded")]
    PaymentSucceeded,
    #[serde(rename = "payment.awaiting_capture")]
    PaymentAwaitingCapture,
    #[serde(rename = "payment.failed")]
    PaymentFailed,
    #[serde(rename = "capture.succeeded")]
    CaptureSucceeded,
    #[serde(rename = "capture.failed")]
    CaptureFailed,
}
