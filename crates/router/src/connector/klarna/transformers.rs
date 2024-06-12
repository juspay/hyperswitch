use api_models::payments;
use common_utils::{pii, types::MinorUnit};
use error_stack::{report, ResultExt};
use masking::{ExposeInterface, Secret};
use serde::{Deserialize, Serialize};

use crate::{
    connector::utils::{self, PaymentsAuthorizeRequestData, RouterData},
    core::errors,
    types::{self, api, storage::enums, transformers::ForeignFrom},
};

#[derive(Debug, Serialize)]
pub struct KlarnaRouterData<T> {
    amount: MinorUnit,
    router_data: T,
}

impl<T> From<(MinorUnit, T)> for KlarnaRouterData<T> {
    fn from((amount, router_data): (MinorUnit, T)) -> Self {
        Self {
            amount,
            router_data,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct KlarnaConnectorMetadataObject {
    pub klarna_region: Option<KlarnaEndpoint>,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum KlarnaEndpoint {
    Europe,
    NorthAmerica,
    Oceania,
}

impl From<KlarnaEndpoint> for String {
    fn from(endpoint: KlarnaEndpoint) -> Self {
        Self::from(match endpoint {
            KlarnaEndpoint::Europe => "",
            KlarnaEndpoint::NorthAmerica => "-na",
            KlarnaEndpoint::Oceania => "-oc",
        })
    }
}

impl TryFrom<&Option<pii::SecretSerdeValue>> for KlarnaConnectorMetadataObject {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(meta_data: &Option<pii::SecretSerdeValue>) -> Result<Self, Self::Error> {
        let metadata: Self = utils::to_connector_meta_from_secret::<Self>(meta_data.clone())
            .change_context(errors::ConnectorError::InvalidConnectorConfig {
                config: "metadata",
            })?;
        Ok(metadata)
    }
}

#[derive(Default, Debug, Serialize)]
pub struct KlarnaPaymentsRequest {
    auto_capture: bool,
    order_lines: Vec<OrderLines>,
    order_amount: MinorUnit,
    purchase_country: enums::CountryAlpha2,
    purchase_currency: enums::Currency,
    merchant_reference1: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct KlarnaPaymentsResponse {
    order_id: String,
    fraud_status: KlarnaFraudStatus,
}

#[derive(Debug, Serialize)]
pub struct KlarnaSessionRequest {
    intent: KlarnaSessionIntent,
    purchase_country: enums::CountryAlpha2,
    purchase_currency: enums::Currency,
    order_amount: MinorUnit,
    order_lines: Vec<OrderLines>,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct KlarnaSessionResponse {
    pub client_token: Secret<String>,
    pub session_id: String,
}

impl TryFrom<&KlarnaRouterData<&types::PaymentsSessionRouterData>> for KlarnaSessionRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &KlarnaRouterData<&types::PaymentsSessionRouterData>,
    ) -> Result<Self, Self::Error> {
        let request = &item.router_data.request;
        match request.order_details.clone() {
            Some(order_details) => Ok(Self {
                intent: KlarnaSessionIntent::Buy,
                purchase_country: request.country.ok_or(
                    errors::ConnectorError::MissingRequiredField {
                        field_name: "billing.address.country",
                    },
                )?,
                purchase_currency: request.currency,
                order_amount: item.amount,
                order_lines: order_details
                    .iter()
                    .map(|data| OrderLines {
                        name: data.product_name.clone(),
                        quantity: data.quantity,
                        unit_price: data.minor_amount,
                        total_amount: MinorUnit::new(
                            i64::from(data.quantity) * (data.minor_amount.get_amount_as_i64()),
                        ),
                    })
                    .collect(),
            }),
            None => Err(report!(errors::ConnectorError::MissingRequiredField {
                field_name: "order_details",
            })),
        }
    }
}

impl TryFrom<types::PaymentsSessionResponseRouterData<KlarnaSessionResponse>>
    for types::PaymentsSessionRouterData
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::PaymentsSessionResponseRouterData<KlarnaSessionResponse>,
    ) -> Result<Self, Self::Error> {
        let response = &item.response;
        Ok(Self {
            response: Ok(types::PaymentsResponseData::SessionResponse {
                session_token: api::SessionToken::Klarna(Box::new(
                    payments::KlarnaSessionTokenResponse {
                        session_token: response.client_token.clone().expose(),
                        session_id: response.session_id.clone(),
                    },
                )),
            }),
            ..item.data
        })
    }
}

impl TryFrom<&KlarnaRouterData<&types::PaymentsAuthorizeRouterData>> for KlarnaPaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(
        item: &KlarnaRouterData<&types::PaymentsAuthorizeRouterData>,
    ) -> Result<Self, Self::Error> {
        let request = &item.router_data.request;
        match request.order_details.clone() {
            Some(order_details) => Ok(Self {
                purchase_country: item.router_data.get_billing_country()?,
                purchase_currency: request.currency,
                order_amount: item.amount,
                order_lines: order_details
                    .iter()
                    .map(|data| OrderLines {
                        name: data.product_name.clone(),
                        quantity: data.quantity,
                        unit_price: data.minor_amount,
                        total_amount: MinorUnit::new(
                            i64::from(data.quantity) * (data.minor_amount.get_amount_as_i64()),
                        ),
                    })
                    .collect(),
                merchant_reference1: Some(item.router_data.connector_request_reference_id.clone()),
                auto_capture: request.is_auto_capture()?,
            }),
            None => Err(report!(errors::ConnectorError::MissingRequiredField {
                field_name: "order_details"
            })),
        }
    }
}

impl TryFrom<types::PaymentsResponseRouterData<KlarnaPaymentsResponse>>
    for types::PaymentsAuthorizeRouterData
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::PaymentsResponseRouterData<KlarnaPaymentsResponse>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(types::PaymentsResponseData::TransactionResponse {
                resource_id: types::ResponseId::ConnectorTransactionId(
                    item.response.order_id.clone(),
                ),
                redirection_data: None,
                mandate_reference: None,
                connector_metadata: None,
                network_txn_id: None,
                connector_response_reference_id: Some(item.response.order_id.clone()),
                incremental_authorization_allowed: None,
                charge_id: None,
            }),
            status: enums::AttemptStatus::foreign_from((
                item.response.fraud_status,
                item.data.request.is_auto_capture()?,
            )),
            ..item.data
        })
    }
}

#[derive(Debug, Serialize)]
pub struct OrderLines {
    name: String,
    quantity: u16,
    unit_price: MinorUnit,
    total_amount: MinorUnit,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
#[allow(dead_code)]
pub enum KlarnaSessionIntent {
    Buy,
    Tokenize,
    BuyAndTokenize,
}

pub struct KlarnaAuthType {
    pub username: Secret<String>,
    pub password: Secret<String>,
}

impl TryFrom<&types::ConnectorAuthType> for KlarnaAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &types::ConnectorAuthType) -> Result<Self, Self::Error> {
        if let types::ConnectorAuthType::BodyKey { api_key, key1 } = auth_type {
            Ok(Self {
                username: key1.to_owned(),
                password: api_key.to_owned(),
            })
        } else {
            Err(errors::ConnectorError::FailedToObtainAuthType.into())
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum KlarnaFraudStatus {
    Accepted,
    Pending,
    Rejected,
}

impl ForeignFrom<(KlarnaFraudStatus, bool)> for enums::AttemptStatus {
    fn foreign_from((klarna_status, is_auto_capture): (KlarnaFraudStatus, bool)) -> Self {
        match klarna_status {
            KlarnaFraudStatus::Accepted => {
                if is_auto_capture {
                    Self::Charged
                } else {
                    Self::Authorized
                }
            }
            KlarnaFraudStatus::Pending => Self::Pending,
            KlarnaFraudStatus::Rejected => Self::Failure,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct KlarnaPsyncResponse {
    pub order_id: String,
    pub status: KlarnaPaymentStatus,
    pub klarna_reference: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum KlarnaPaymentStatus {
    Authorized,
    PartCaptured,
    Captured,
    Cancelled,
    Expired,
    Closed,
}

impl From<KlarnaPaymentStatus> for enums::AttemptStatus {
    fn from(item: KlarnaPaymentStatus) -> Self {
        match item {
            KlarnaPaymentStatus::Authorized => Self::Authorized,
            KlarnaPaymentStatus::PartCaptured => Self::PartialCharged,
            KlarnaPaymentStatus::Captured => Self::Charged,
            KlarnaPaymentStatus::Cancelled => Self::Voided,
            KlarnaPaymentStatus::Expired | KlarnaPaymentStatus::Closed => Self::Failure,
        }
    }
}

impl<F, T>
    TryFrom<types::ResponseRouterData<F, KlarnaPsyncResponse, T, types::PaymentsResponseData>>
    for types::RouterData<F, T, types::PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<F, KlarnaPsyncResponse, T, types::PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            status: enums::AttemptStatus::from(item.response.status),
            response: Ok(types::PaymentsResponseData::TransactionResponse {
                resource_id: types::ResponseId::ConnectorTransactionId(
                    item.response.order_id.clone(),
                ),
                redirection_data: None,
                mandate_reference: None,
                connector_metadata: None,
                network_txn_id: None,
                connector_response_reference_id: item
                    .response
                    .klarna_reference
                    .or(Some(item.response.order_id)),
                incremental_authorization_allowed: None,
                charge_id: None,
            }),
            ..item.data
        })
    }
}

#[derive(Debug, Serialize)]
pub struct KlarnaCaptureRequest {
    captured_amount: MinorUnit,
    reference: Option<String>,
}

impl TryFrom<&KlarnaRouterData<&types::PaymentsCaptureRouterData>> for KlarnaCaptureRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &KlarnaRouterData<&types::PaymentsCaptureRouterData>,
    ) -> Result<Self, Self::Error> {
        let reference = Some(item.router_data.connector_request_reference_id.clone());
        Ok(Self {
            reference,
            captured_amount: item.amount.to_owned(),
        })
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct KlarnaMeta {
    capture_id: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct KlarnaCaptureResponse {
    pub capture_id: Option<String>,
}

impl<F>
    TryFrom<
        types::ResponseRouterData<
            F,
            KlarnaCaptureResponse,
            types::PaymentsCaptureData,
            types::PaymentsResponseData,
        >,
    > for types::RouterData<F, types::PaymentsCaptureData, types::PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<
            F,
            KlarnaCaptureResponse,
            types::PaymentsCaptureData,
            types::PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        let connector_meta = serde_json::json!(KlarnaMeta {
            capture_id: item.response.capture_id,
        });

        // https://docs.klarna.com/api/ordermanagement/#operation/captureOrder
        // If 201 status code, then order is captured, other status codes are handled by the error handler
        let status = if item.http_code == 201 {
            enums::AttemptStatus::Charged
        } else {
            item.data.status
        };
        let resource_id = item.data.request.connector_transaction_id.clone();

        Ok(Self {
            response: Ok(types::PaymentsResponseData::TransactionResponse {
                resource_id: types::ResponseId::ConnectorTransactionId(resource_id),
                redirection_data: None,
                mandate_reference: None,
                connector_metadata: Some(connector_meta),
                network_txn_id: None,
                connector_response_reference_id: None,
                incremental_authorization_allowed: None,
                charge_id: None,
            }),
            status,
            ..item.data
        })
    }
}

#[derive(Default, Debug, Serialize)]
pub struct KlarnaRefundRequest {
    refunded_amount: MinorUnit,
    reference: Option<String>,
}

impl<F> TryFrom<&KlarnaRouterData<&types::RefundsRouterData<F>>> for KlarnaRefundRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &KlarnaRouterData<&types::RefundsRouterData<F>>,
    ) -> Result<Self, Self::Error> {
        let request = &item.router_data.request;
        Ok(Self {
            refunded_amount: item.amount,
            reference: Some(request.refund_id.clone()),
        })
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct KlarnaRefundResponse {
    pub refund_id: String,
}

impl TryFrom<types::RefundsResponseRouterData<api::Execute, KlarnaRefundResponse>>
    for types::RefundsRouterData<api::Execute>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::RefundsResponseRouterData<api::Execute, KlarnaRefundResponse>,
    ) -> Result<Self, Self::Error> {
        // https://docs.klarna.com/api/ordermanagement/#operation/refundOrder
        // If 201 status code, then Refund is Successful, other status codes are handled by the error handler
        let status = if item.http_code == 201 {
            enums::RefundStatus::Pending
        } else {
            enums::RefundStatus::Failure
        };
        Ok(Self {
            response: Ok(types::RefundsResponseData {
                connector_refund_id: item.response.refund_id,
                refund_status: status,
            }),
            ..item.data
        })
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct KlarnaRefundSyncResponse {
    pub refund_id: String,
}

impl TryFrom<types::RefundsResponseRouterData<api::RSync, KlarnaRefundSyncResponse>>
    for types::RefundsRouterData<api::RSync>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::RefundsResponseRouterData<api::RSync, KlarnaRefundSyncResponse>,
    ) -> Result<Self, Self::Error> {
        // https://docs.klarna.com/api/ordermanagement/#operation/get
        // If 200 status code, then Refund is Successful, other status codes are handled by the error handler
        let status = if item.http_code == 200 {
            enums::RefundStatus::Success
        } else {
            enums::RefundStatus::Failure
        };
        Ok(Self {
            response: Ok(types::RefundsResponseData {
                connector_refund_id: item.response.refund_id,
                refund_status: status,
            }),
            ..item.data
        })
    }
}

#[derive(Deserialize, Serialize, Debug)]
pub struct KlarnaErrorResponse {
    pub error_code: String,
    pub error_messages: Option<Vec<String>>,
    pub error_message: Option<String>,
}
