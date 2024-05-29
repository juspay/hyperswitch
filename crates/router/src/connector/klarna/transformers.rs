use api_models::payments;
use common_utils::pii;
use error_stack::{report, ResultExt};
use masking::{ExposeInterface, Secret};
use serde::{Deserialize, Serialize};

use crate::{
    connector::utils::{self, PaymentsAuthorizeRequestData, RouterData},
    core::errors,
    types::{self, storage::enums, transformers::ForeignFrom},
};

#[derive(Debug, Serialize)]
pub struct KlarnaRouterData<T> {
    amount: i64,
    router_data: T,
}

impl<T> TryFrom<(&types::api::CurrencyUnit, enums::Currency, i64, T)> for KlarnaRouterData<T> {
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(
        (_currency_unit, _currency, amount, router_data): (
            &types::api::CurrencyUnit,
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
                        unit_price: data.amount,
                        total_amount: i64::from(data.quantity) * (data.amount),
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
                session_token: types::api::SessionToken::Klarna(Box::new(
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
                        unit_price: data.amount,
                        total_amount: i64::from(data.quantity) * (data.amount),
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
            KlarnaFraudStatus::Pending => Self::Authorizing,
            KlarnaFraudStatus::Rejected => Self::Failure,
        }
    }
}

#[derive(Deserialize, Serialize, Debug)]
pub struct KlarnaErrorResponse {
    pub error_code: String,
    pub error_messages: Option<Vec<String>>,
    pub error_message: Option<String>,
}
