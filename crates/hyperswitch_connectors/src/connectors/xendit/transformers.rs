use std::collections::HashMap;

use api_models::payments::QrCodeInformation;
use cards::CardNumber;
use common_enums::{enums, Currency};
use common_utils::{
    errors::CustomResult, ext_traits::Encode, pii, request::Method, types::FloatMajorUnit,
};
use error_stack::ResultExt;
use hyperswitch_domain_models::{
    payment_method_data::PaymentMethodData,
    router_data::{ConnectorAuthType, ErrorResponse},
    router_flow_types::refunds::{Execute, RSync},
    router_request_types::ResponseId,
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
    types::{
        PaymentsCaptureResponseRouterData, PaymentsPreprocessingResponseRouterData,
        PaymentsResponseRouterData, PaymentsSyncResponseRouterData, RefundsResponseRouterData,
    },
    utils::{
        get_unimplemented_payment_method_error_message, CardData, PaymentsAuthorizeRequestData,
        PaymentsSyncRequestData, QrImage, RouterData as OtherRouterData,
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
    pub currency: Currency,
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
pub struct CommonXenditPaymentsRequestData {
    pub amount: FloatMajorUnit,
    pub currency: Currency,
    pub capture_method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payment_method: Option<PaymentMethod>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payment_method_id: Option<Secret<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub channel_properties: Option<ChannelProperties>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum QrType {
    Dynamic,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct XenditQrisPaymentsRequestData {
    pub amount: FloatMajorUnit,
    pub external_id: String,
    #[serde(rename = "type")]
    pub qr_type: QrType,
    pub callback_url: String,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(untagged)]
pub enum XenditPaymentsRequest {
    CommonXenditPaymentsRequest(Box<CommonXenditPaymentsRequestData>),
    QrPaymentsRequest(XenditQrisPaymentsRequestData),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct XenditSplitRoute {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub flat_amount: Option<FloatMajorUnit>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub percent_amount: Option<i64>,
    pub currency: Currency,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cvv: Option<Secret<String>>,
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
    Active,
    Inactive,
}
#[derive(Debug, Deserialize, Serialize, Clone)]
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
    pub payment_method: Option<PaymentMethodInfo>,
    pub failure_code: Option<String>,
    pub reference_id: Option<Secret<String>>,
    pub amount: FloatMajorUnit,
    pub currency: Option<Currency>,
    pub qr_string: Option<String>,
    pub external_id: Option<String>,
}

fn map_payment_response_to_attempt_status(
    response: XenditPaymentResponse,
    is_auto_capture: bool,
) -> enums::AttemptStatus {
    match response.status {
        PaymentStatus::Failed | PaymentStatus::Inactive => enums::AttemptStatus::Failure,
        PaymentStatus::Succeeded | PaymentStatus::Verified => {
            if is_auto_capture {
                enums::AttemptStatus::Charged
            } else {
                enums::AttemptStatus::Authorized
            }
        }
        PaymentStatus::Pending => enums::AttemptStatus::Pending,
        PaymentStatus::RequiresAction | PaymentStatus::Active => {
            enums::AttemptStatus::AuthenticationPending
        }
        PaymentStatus::AwaitingCapture => enums::AttemptStatus::Authorized,
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct XenditCaptureResponse {
    pub id: String,
    pub status: PaymentStatus,
    pub actions: Option<Vec<Action>>,
    pub payment_method: PaymentMethodInfo,
    pub failure_code: Option<String>,
    pub reference_id: Secret<String>,
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
        match item.router_data.request.payment_method_type {
            Some(common_enums::PaymentMethodType::Qris) => {
                if let Some(common_enums::CaptureMethod::Manual) =
                    item.router_data.request.capture_method
                {
                    return Err(errors::ConnectorError::NotSupported {
                        message: "Manual Capture for QRIS payments".to_string(),
                        connector: "Xendit",
                    }
                    .into());
                }
                let qr_request = XenditQrisPaymentsRequestData {
                    amount: item.amount,
                    external_id: item.router_data.connector_request_reference_id.clone(),
                    qr_type: QrType::Dynamic,
                    callback_url: item.router_data.request.get_webhook_url()?,
                };
                Ok(Self::QrPaymentsRequest(qr_request))
            }
            _ => {
                let common_request = CommonXenditPaymentsRequestData::try_from(item)?;
                Ok(Self::CommonXenditPaymentsRequest(Box::new(common_request)))
            }
        }
    }
}

impl TryFrom<XenditRouterData<&PaymentsAuthorizeRouterData>> for CommonXenditPaymentsRequestData {
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
                            cvv: if card_data.card_cvc.clone().expose().is_empty() {
                                None
                            } else {
                                Some(card_data.card_cvc.clone())
                            },
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

pub fn get_qr_image(qr_data: String) -> CustomResult<serde_json::Value, errors::ConnectorError> {
    let image_data = QrImage::new_from_data(qr_data)
        .change_context(errors::ConnectorError::ResponseHandlingFailed)?;
    let image_data_url = url::Url::parse(image_data.data.clone().as_str())
        .change_context(errors::ConnectorError::ResponseHandlingFailed)?;
    let qr_code_info = QrCodeInformation::QrDataUrl {
        image_data_url,
        display_to_timestamp: None,
    };
    qr_code_info
        .encode_to_value()
        .change_context(errors::ConnectorError::ResponseHandlingFailed)
}

pub fn extract_resource_id_from_payment_response(
    payment_method_type: Option<common_enums::PaymentMethodType>,
    response: &XenditPaymentResponse,
) -> CustomResult<ResponseId, errors::ConnectorError> {
    if payment_method_type == Some(common_enums::PaymentMethodType::Qris) {
        let ext_id = response
            .external_id
            .clone()
            .ok_or_else(|| errors::ConnectorError::WebhookReferenceIdNotFound)?;
        Ok(ResponseId::ConnectorTransactionId(ext_id))
    } else {
        Ok(ResponseId::ConnectorTransactionId(response.id.clone()))
    }
}

impl TryFrom<PaymentsResponseRouterData<XenditPaymentResponse>> for PaymentsAuthorizeRouterData {
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(
        item: PaymentsResponseRouterData<XenditPaymentResponse>,
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
                network_advice_code: None,
                network_decline_code: None,
                network_error_message: None,
                connector_metadata: None,
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

            let connector_metadata = if let Some(qr_data) = item.response.qr_string.clone() {
                Some(get_qr_image(qr_data)?)
            } else {
                None
            };

            let resource_id = extract_resource_id_from_payment_response(
                item.data.request.payment_method_type,
                &item.response,
            )?;

            Ok(PaymentsResponseData::TransactionResponse {
                resource_id,
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
                        connector_mandate_id: item
                            .response
                            .payment_method
                            .map(|payment_method| payment_method.id.expose()),
                        payment_method_id: None,
                        mandate_metadata: None,
                        connector_mandate_request_reference_id: None,
                    })),
                    false => Box::new(None),
                },
                connector_metadata,
                network_txn_id: None,
                connector_response_reference_id: item
                    .response
                    .reference_id
                    .map(|reference_id| reference_id.expose())
                    .or(item.response.external_id.clone()),
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

impl TryFrom<PaymentsCaptureResponseRouterData<XenditCaptureResponse>>
    for PaymentsCaptureRouterData
{
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(
        item: PaymentsCaptureResponseRouterData<XenditCaptureResponse>,
    ) -> Result<Self, Self::Error> {
        let status = match item.response.status {
            PaymentStatus::Failed | PaymentStatus::Inactive => enums::AttemptStatus::Failure,
            PaymentStatus::Succeeded | PaymentStatus::Verified => enums::AttemptStatus::Charged,
            PaymentStatus::Pending => enums::AttemptStatus::Pending,
            PaymentStatus::RequiresAction | PaymentStatus::Active => {
                enums::AttemptStatus::AuthenticationPending
            }
            PaymentStatus::AwaitingCapture => enums::AttemptStatus::Authorized,
        };
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
                network_advice_code: None,
                network_decline_code: None,
                network_error_message: None,
                connector_metadata: None,
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

impl TryFrom<PaymentsPreprocessingResponseRouterData<XenditSplitResponse>>
    for PaymentsPreProcessingRouterData
{
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(
        item: PaymentsPreprocessingResponseRouterData<XenditSplitResponse>,
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
                            item.data.request.currency.unwrap_or(Currency::USD),
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
                        network_advice_code: None,
                        network_decline_code: None,
                        network_error_message: None,
                        connector_metadata: None,
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
                let status = match webhook_event {
                    XenditWebhookEvent::CommonEvent(event_data) => match event_data.event {
                        XenditEventType::PaymentSucceeded
                        | XenditEventType::CaptureSucceeded
                        | XenditEventType::QrPayment => enums::AttemptStatus::Charged,
                        XenditEventType::PaymentAwaitingCapture => enums::AttemptStatus::Authorized,
                        XenditEventType::PaymentFailed | XenditEventType::CaptureFailed => {
                            enums::AttemptStatus::Failure
                        }
                    },
                    XenditWebhookEvent::QrEvent(_) => enums::AttemptStatus::Charged,
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
                                item.request.currency.unwrap_or(Currency::USD),
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
    pub id: String,
    pub status: RefundStatus,
    pub amount: FloatMajorUnit,
    pub currency: String,
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
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct XenditCommonWebhookEvent {
    pub event: XenditEventType,
    pub data: EventDetails,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct XenditQRWebhookEvent {
    pub id: String,
    pub event: XenditEventType,
    pub amount: FloatMajorUnit,
    pub qr_code: QrData,
    pub status: QRPaymentStatus,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct EventDetails {
    pub id: String,
    pub payment_request_id: Option<String>,
    pub amount: FloatMajorUnit,
    pub currency: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
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
    #[serde(rename = "qr.payment")]
    QrPayment,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum QRPaymentStatus {
    Completed,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct QrData {
    pub id: String,
    pub external_id: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(untagged)]
pub enum XenditWebhookEvent {
    CommonEvent(XenditCommonWebhookEvent),
    QrEvent(XenditQRWebhookEvent),
}
