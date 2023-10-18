use api_models::payments;
use error_stack::report;
use masking::Secret;
use serde::{Deserialize, Serialize};

use crate::{
    core::errors,
    types::{self, storage::enums},
};

#[derive(Debug, Serialize)]
pub struct KlarnaRouterData<T> {
    amount: i64,
    router_data: T,
}

impl<T>
    TryFrom<(
        &types::api::CurrencyUnit,
        types::storage::enums::Currency,
        i64,
        T,
    )> for KlarnaRouterData<T>
{
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(
        (_currency_unit, _currency, amount, router_data): (
            &types::api::CurrencyUnit,
            types::storage::enums::Currency,
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

#[derive(Default, Debug, Serialize)]
pub struct KlarnaPaymentsRequest {
    order_lines: Vec<OrderLines>,
    order_amount: i64,
    purchase_country: String,
    purchase_currency: enums::Currency,
    merchant_reference1: String,
}

#[derive(Default, Debug, Deserialize)]
pub struct KlarnaPaymentsResponse {
    order_id: String,
    fraud_status: KlarnaFraudStatus,
}

#[derive(Debug, Serialize)]
pub struct KlarnaSessionRequest {
    intent: KlarnaSessionIntent,
    purchase_country: String,
    purchase_currency: enums::Currency,
    locale: String,
    order_amount: i64,
    order_lines: Vec<OrderLines>,
}

#[derive(Deserialize)]
pub struct KlarnaSessionResponse {
    pub client_token: String,
    pub session_id: String,
}

impl TryFrom<&types::PaymentsSessionRouterData> for KlarnaSessionRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsSessionRouterData) -> Result<Self, Self::Error> {
        let request = &item.request;
        match request.order_details.clone() {
            Some(order_details) => Ok(Self {
                intent: KlarnaSessionIntent::Buy,
                purchase_country: "US".to_string(),
                purchase_currency: request.currency,
                order_amount: request.amount,
                locale: "en-US".to_string(),
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
                session_token: types::api::SessionToken::Klarna(Box::new(
                    payments::KlarnaSessionTokenResponse {
                        session_token: response.client_token.clone(),
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
                purchase_country: "US".to_string(),
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
                merchant_reference1: item.router_data.connector_request_reference_id.clone(),
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

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum KlarnaFraudStatus {
    Accepted,
    #[default]
    Pending,
}

impl From<KlarnaFraudStatus> for enums::AttemptStatus {
    fn from(item: KlarnaFraudStatus) -> Self {
        match item {
            KlarnaFraudStatus::Accepted => Self::Charged,
            KlarnaFraudStatus::Pending => Self::Authorizing,
        }
    }
}

#[derive(Deserialize)]
pub struct KlarnaErrorResponse {
    pub error_code: String,
    pub error_messages: Option<Vec<String>>,
    pub error_message: Option<String>,
}
