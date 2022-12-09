use serde::{Deserialize, Serialize};

use crate::{
    core::errors,
    types::{self, storage::enums},
};

#[derive(Default, Debug, Serialize)]
pub struct KlarnaPaymentsRequest {}

#[derive(Serialize)]
pub struct KlarnaSessionRequest {
    intent: KlarnaSessionIntent,
    purchase_country: String,
    purchase_currency: enums::Currency,
    locale: String,
    order_amount: i32,
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
        Ok(Self {
            intent: KlarnaSessionIntent::Buy,
            purchase_country: "US".to_string(),
            purchase_currency: request.currency,
            order_amount: request.amount,
            locale: "en-US".to_string(),
            order_lines: vec![OrderLines {
                name: "Battery Power Pack".to_string(),
                quantity: 1,
                unit_price: request.amount,
                total_amount: request.amount,
            }],
        })
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
        Ok(types::RouterData {
            response: Ok(types::PaymentsResponseData::SessionResponse(
                types::PaymentsSessionResponse {
                    session_id: Some(response.session_id.clone()),
                    session_token: response.client_token.clone(),
                },
            )),
            ..item.data
        })
    }
}

#[derive(Serialize)]
pub struct OrderLines {
    name: String,
    quantity: u64,
    unit_price: i32,
    total_amount: i32,
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
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
            KlarnaPaymentStatus::Succeeded => enums::AttemptStatus::Charged,
            KlarnaPaymentStatus::Failed => enums::AttemptStatus::Failure,
            KlarnaPaymentStatus::Processing => enums::AttemptStatus::Authorizing,
        }
    }
}

#[derive(Deserialize)]
pub struct KlarnaErrorResponse {
    pub error_code: String,
    pub error_messages: Vec<String>,
}
