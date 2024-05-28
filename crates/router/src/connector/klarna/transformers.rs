use api_models::payments;
use common_utils::pii;
use error_stack::{report, ResultExt};
use masking::{ExposeInterface, Secret};
use serde::{Deserialize, Serialize};

use crate::{
    connector::utils::{self, RouterData},
    core::errors,
    types::{self, api, storage::enums},
};

#[derive(Debug, Serialize)]
pub struct KlarnaRouterData<T> {
    amount: i64,
    router_data: T,
}

impl<T> TryFrom<(&api::CurrencyUnit, enums::Currency, i64, T)> for KlarnaRouterData<T> {
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(
        (_currency_unit, _currency, amount, router_data): (
            &api::CurrencyUnit,
            enums::Currency,
            i64,
            T,
        ),
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            amount,
            router_data,
        })
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct KlarnaConnectorMetadataObject {
    pub region_based_endpoint: Option<KlarnaEndpoint>,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum KlarnaEndpoint {
    Europe,
    NorthAmerica,
    Oceania,
}

impl From<KlarnaEndpoint> for &'static str {
    fn from(endpoint: KlarnaEndpoint) -> Self {
        match endpoint {
            KlarnaEndpoint::Europe => "",
            KlarnaEndpoint::NorthAmerica => "-na",
            KlarnaEndpoint::Oceania => "-oc",
        }
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
    order_lines: Vec<OrderLines>,
    order_amount: i64,
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
    order_amount: i64,
    order_lines: Vec<OrderLines>,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct KlarnaSessionResponse {
    pub client_token: Secret<String>,
    pub session_id: Secret<String>,
}

impl TryFrom<&types::PaymentsSessionRouterData> for KlarnaSessionRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsSessionRouterData) -> Result<Self, Self::Error> {
        let request = &item.request;
        match request.order_details.clone() {
            Some(order_details) => Ok(Self {
                intent: KlarnaSessionIntent::Buy,
                purchase_country: request.country.ok_or(
                    errors::ConnectorError::MissingRequiredField {
                        field_name: "purchase_country",
                    },
                )?,
                purchase_currency: request.currency,
                order_amount: request.amount,
                order_lines: order_details
                    .iter()
                    .map(|data| OrderLines {
                        name: data.product_name.clone(),
                        quantity: data.quantity,
                        unit_price: data.amount,
                        total_amount: i64::from(data.quantity) * (data.amount),
                    })
                    .collect(),
            }),
            None => Err(report!(errors::ConnectorError::MissingRequiredField {
                field_name: "product_name",
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
                        session_id: response.session_id.clone().expose(),
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
                order_amount: request.amount,
                order_lines: order_details
                    .iter()
                    .map(|data| OrderLines {
                        name: data.product_name.clone(),
                        quantity: data.quantity,
                        unit_price: data.amount,
                        total_amount: i64::from(data.quantity) * (data.amount),
                    })
                    .collect(),
                merchant_reference1: Some(item.router_data.connector_request_reference_id.clone()),
            }),
            None => Err(report!(errors::ConnectorError::MissingRequiredField {
                field_name: "product_name"
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
            status: item.response.fraud_status.into(),
            ..item.data
        })
    }
}

#[derive(Debug, Serialize)]
pub struct OrderLines {
    name: String,
    quantity: u16,
    unit_price: i64,
    total_amount: i64,
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
    pub basic_token: Secret<String>,
}

impl TryFrom<&types::ConnectorAuthType> for KlarnaAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &types::ConnectorAuthType) -> Result<Self, Self::Error> {
        if let types::ConnectorAuthType::HeaderKey { api_key } = auth_type {
            Ok(Self {
                basic_token: api_key.to_owned(),
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
}

impl From<KlarnaFraudStatus> for enums::AttemptStatus {
    fn from(item: KlarnaFraudStatus) -> Self {
        match item {
            KlarnaFraudStatus::Accepted => Self::Authorized,
            KlarnaFraudStatus::Pending => Self::Authorizing,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct KlarnaCaptureRequest {
    captured_amount: i64,
    reference: Option<String>,
}

impl TryFrom<&KlarnaRouterData<&types::PaymentsCaptureRouterData>> for KlarnaCaptureRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &KlarnaRouterData<&types::PaymentsCaptureRouterData>,
    ) -> Result<Self, Self::Error> {
        let reference = Some(
            item.router_data
                .request
                .connector_transaction_id
                .to_string(),
        );
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

impl<F, T>
    TryFrom<types::ResponseRouterData<F, KlarnaCaptureResponse, T, types::PaymentsResponseData>>
    for types::RouterData<F, T, types::PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<F, KlarnaCaptureResponse, T, types::PaymentsResponseData>,
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

        Ok(Self {
            response: Ok(types::PaymentsResponseData::TransactionResponse {
                resource_id: types::ResponseId::NoResponseId,
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
    refunded_amount: i64,
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
        Ok(Self {
            response: Ok(types::RefundsResponseData {
                connector_refund_id: item.response.refund_id,
                refund_status: enums::RefundStatus::Pending,
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
        Ok(Self {
            response: Ok(types::RefundsResponseData {
                connector_refund_id: item.response.refund_id,
                refund_status: enums::RefundStatus::Success,
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
