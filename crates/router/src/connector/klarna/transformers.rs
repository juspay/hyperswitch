use error_stack::{report, IntoReport, ResultExt};
use serde::{Deserialize, Serialize};
use url::Url;

use crate::{
    core::errors,
    services,
    types::{self, storage::enums},
};

#[derive(Default, Debug, Serialize)]
pub struct KlarnaPaymentsRequest {
    order_lines: Vec<OrderLines>,
    order_amount: i64,
    purchase_country: String,
    purchase_currency: enums::Currency,
}

#[derive(Default, Debug, Deserialize)]
pub struct KlarnaPaymentsResponse {
    order_id: String,
    redirection_url: String,
}
#[derive(Serialize)]
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
                order_lines: vec![OrderLines {
                    name: order_details.product_name,
                    quantity: order_details.quantity,
                    unit_price: request.amount,
                    total_amount: request.amount,
                }],
            }),
            None => Err(report!(errors::ConnectorError::MissingRequiredField {
                field_name: "product_name".to_string()
            })),
        }
    }
}

impl TryFrom<types::PaymentsSessionResponseRouterData<KlarnaSessionResponse>>
    for types::PaymentsSessionRouterData
{
    type Error = error_stack::Report<errors::ParsingError>;
    fn try_from(
        item: types::PaymentsSessionResponseRouterData<KlarnaSessionResponse>,
    ) -> Result<Self, Self::Error> {
        let response = &item.response;
        Ok(Self {
            response: Ok(types::PaymentsResponseData::SessionResponse {
                session_token: types::api::SessionToken::Klarna {
                    session_token: response.client_token.clone(),
                    session_id: response.session_id.clone(),
                },
            }),
            ..item.data
        })
    }
}

impl TryFrom<&types::PaymentsAuthorizeRouterData> for KlarnaPaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsAuthorizeRouterData) -> Result<Self, Self::Error> {
        let request = &item.request;
        match request.order_details.clone() {
            Some(order_details) => Ok(Self {
                purchase_country: "US".to_string(),
                purchase_currency: request.currency,
                order_amount: request.amount,
                order_lines: vec![OrderLines {
                    name: order_details.product_name,
                    quantity: order_details.quantity,
                    unit_price: request.amount,
                    total_amount: request.amount,
                }],
            }),
            None => Err(report!(errors::ConnectorError::MissingRequiredField {
                field_name: "product_name".to_string()
            })),
        }
    }
}

impl TryFrom<types::PaymentsResponseRouterData<KlarnaPaymentsResponse>>
    for types::PaymentsAuthorizeRouterData
{
    type Error = error_stack::Report<errors::ParsingError>;
    fn try_from(
        item: types::PaymentsResponseRouterData<KlarnaPaymentsResponse>,
    ) -> Result<Self, Self::Error> {
        let response = &item.response;
        let url = Url::parse(&response.redirection_url)
            .into_report()
            .change_context(errors::ParsingError)
            .attach_printable("Could not parse the redirection data")?;
        let redirection_data = services::RedirectForm {
            url: url.to_string(),
            method: services::Method::Get,
            form_fields: std::collections::HashMap::from_iter(
                url.query_pairs()
                    .map(|(k, v)| (k.to_string(), v.to_string())),
            ),
        };
        Ok(Self {
            response: Ok(types::PaymentsResponseData::TransactionResponse {
                resource_id: types::ResponseId::ConnectorTransactionId(item.response.order_id),
                redirect: true,
                redirection_data: Some(redirection_data),
                mandate_reference: None,
            }),
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

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
#[allow(dead_code)]
pub enum KlarnaSessionIntent {
    Buy,
    Tokenize,
    BuyAndTokenize,
}

pub struct KlarnaAuthType {
    pub basic_token: String,
}

impl TryFrom<&types::ConnectorAuthType> for KlarnaAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &types::ConnectorAuthType) -> Result<Self, Self::Error> {
        if let types::ConnectorAuthType::HeaderKey { api_key } = auth_type {
            Ok(Self {
                basic_token: api_key.to_string(),
            })
        } else {
            Err(errors::ConnectorError::FailedToObtainAuthType.into())
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum KlarnaPaymentStatus {
    Succeeded,
    Failed,
    #[default]
    Processing,
}

impl From<KlarnaPaymentStatus> for enums::AttemptStatus {
    fn from(item: KlarnaPaymentStatus) -> Self {
        match item {
            KlarnaPaymentStatus::Succeeded => Self::Charged,
            KlarnaPaymentStatus::Failed => Self::Failure,
            KlarnaPaymentStatus::Processing => Self::Authorizing,
        }
    }
}

#[derive(Deserialize)]
pub struct KlarnaErrorResponse {
    pub error_code: String,
    pub error_messages: Vec<String>,
}
