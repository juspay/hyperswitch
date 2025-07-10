use std::time::SystemTime;

use base64::Engine;
use common_enums::enums;
use common_utils::errors::CustomResult;
use hyperswitch_domain_models::{
    payment_method_data::PaymentMethodData,
    router_data::{ConnectorAuthType, RouterData},
    router_flow_types::refunds::{Execute, RSync},
    router_request_types::ResponseId,
    router_response_types::{PaymentsResponseData, RefundsResponseData},
    types::{PaymentsAuthorizeRouterData, PaymentsCaptureRouterData, RefundsRouterData},
};
use hyperswitch_interfaces::errors;
use masking::{ExposeInterface, Secret};
use ring::signature;
use serde::{Deserialize, Serialize};

use crate::{
    types::{RefundsResponseRouterData, ResponseRouterData},
    utils::{self, PaymentsAuthorizeRequestData},
};

pub struct MpgsRouterData<T> {
    pub amount: String,
    pub router_data: T,
}

use hyperswitch_interfaces::api;

impl<T> TryFrom<(&api::CurrencyUnit, enums::Currency, i64, T)> for MpgsRouterData<T> {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        (currency_unit, currency, amount, item): (&api::CurrencyUnit, enums::Currency, i64, T),
    ) -> Result<Self, Self::Error> {
        let amount =
            utils::get_amount_as_string(currency_unit, amount, currency).map_err(|_| {
                errors::ConnectorError::MissingRequiredField {
                    field_name: "amount",
                }
            })?;
        Ok(Self {
            amount,
            router_data: item,
        })
    }
}

#[derive(Debug, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum MpgsApiOperation {
    Pay,
    #[default]
    Authorize,
    Capture,
    Void,
    Refund,
    Verify,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MpgsPaymentsRequest {
    pub api_operation: MpgsApiOperation,
    pub order: MpgsOrder,
    pub source_of_funds: MpgsSourceOfFunds,
    pub transaction: MpgsTransaction,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MpgsOrder {
    pub amount: String,
    pub currency: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MpgsSourceOfFunds {
    pub r#type: String,
    pub provided: Option<MpgsProvidedSourceOfFunds>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MpgsProvidedSourceOfFunds {
    pub card: MpgsCard,
}

#[derive(Debug, Serialize)]
pub struct MpgsCard {
    pub number: cards::CardNumber,
    pub expiry: MpgsExpiry,
    #[serde(rename = "securityCode")]
    pub security_code: Option<Secret<String>>,
}

#[derive(Debug, Serialize)]
pub struct MpgsExpiry {
    pub month: Secret<String>,
    pub year: Secret<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MpgsTransaction {
    pub reference: String,
}

impl TryFrom<&MpgsRouterData<&PaymentsAuthorizeRouterData>> for MpgsPaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &MpgsRouterData<&PaymentsAuthorizeRouterData>) -> Result<Self, Self::Error> {
        let source_of_funds = match item.router_data.request.payment_method_data.clone() {
            PaymentMethodData::Card(card) => Some(MpgsProvidedSourceOfFunds {
                card: MpgsCard {
                    number: card.card_number,
                    expiry: MpgsExpiry {
                        month: card.card_exp_month,
                        year: card.card_exp_year,
                    },
                    security_code: Some(card.card_cvc),
                },
            }),
            _ => None,
        };

        Ok(Self {
            api_operation: if item.router_data.request.is_auto_capture()? {
                MpgsApiOperation::Pay
            } else {
                MpgsApiOperation::Authorize
            },
            order: MpgsOrder {
                amount: item.amount.to_owned(),
                currency: item.router_data.request.currency.to_string(),
            },
            source_of_funds: MpgsSourceOfFunds {
                r#type: "CARD".to_string(),
                provided: source_of_funds,
            },
            transaction: MpgsTransaction {
                reference: item.router_data.payment_id.clone(),
            },
        })
    }
}

impl TryFrom<&MpgsRouterData<&PaymentsCaptureRouterData>> for MpgsPaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &MpgsRouterData<&PaymentsCaptureRouterData>) -> Result<Self, Self::Error> {
        Ok(Self {
            api_operation: MpgsApiOperation::Capture,
            order: MpgsOrder {
                amount: item.amount.to_owned(),
                currency: item.router_data.request.currency.to_string(),
            },
            source_of_funds: MpgsSourceOfFunds {
                r#type: "CARD".to_string(),
                provided: None,
            },
            transaction: MpgsTransaction {
                reference: item.router_data.request.connector_transaction_id.clone(),
            },
        })
    }
}

pub struct MpgsAuthType {
    pub(super) consumer_key: Secret<String>,
    pub(super) private_key: Secret<String>,
}

impl TryFrom<&ConnectorAuthType> for MpgsAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &ConnectorAuthType) -> Result<Self, Self::Error> {
        if let ConnectorAuthType::BodyKey { api_key, key1 } = auth_type {
            Ok(Self {
                consumer_key: api_key.to_owned(),
                private_key: key1.to_owned(),
            })
        } else {
            Err(errors::ConnectorError::FailedToObtainAuthType.into())
        }
    }
}

pub fn generate_oauth_token(auth: &MpgsAuthType) -> CustomResult<String, errors::ConnectorError> {
    let timestamp = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map_err(|_| errors::ConnectorError::RequestEncodingFailed)?
        .as_secs();
    let nonce = uuid::Uuid::new_v4().to_string();
    let signature_method = "RSA-SHA256";
    let version = "1.0";

    let base_string = format!(
        "oauth_consumer_key={}&oauth_nonce={}&oauth_signature_method={}&oauth_timestamp={}&oauth_version={}",
        auth.consumer_key.clone().expose(),
        nonce,
        signature_method,
        timestamp,
        version
    );

    let key_pair =
        signature::RsaKeyPair::from_pkcs8(auth.private_key.clone().expose().as_bytes())
            .map_err(|_| errors::ConnectorError::RequestEncodingFailed)?;
    let mut signature = vec![0; key_pair.public().modulus_len()];
    let rng = ring::rand::SystemRandom::new();
    key_pair
        .sign(
            &signature::RSA_PKCS1_SHA256,
            &rng,
            base_string.as_bytes(),
            &mut signature,
        )
        .map_err(|_| errors::ConnectorError::RequestEncodingFailed)?;

    let signature_base64 = common_utils::consts::BASE64_ENGINE.encode(&signature);

    Ok(format!(
        "OAuth oauth_consumer_key=\"{}\",oauth_nonce=\"{}\",oauth_signature_method=\"{}\",oauth_timestamp=\"{}\",oauth_version=\"{}\",oauth_signature=\"{}\"",
        auth.consumer_key.clone().expose(),
        nonce,
        signature_method,
        timestamp,
        version,
        signature_base64
    ))
}
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum MpgsPaymentStatus {
    Success,
    Pending,
    Failure,
    Unknown,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum MpgsTransactionType {
    Authorization,
    Payment,
    Capture,
    Void,
    Refund,
    Verification,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum MpgsResponseGatewayCode {
    Approved,
    ApprovedPendingSettlement,
    Declined,
    DeclinedAvs,
    DeclinedCsc,
    DeclinedAvsCsc,
    AuthenticationRequired,
    Submitted,
    Pending,
}

impl From<MpgsPaymentsResponse> for common_enums::AttemptStatus {
    fn from(item: MpgsPaymentsResponse) -> Self {
        match item.result {
            MpgsPaymentStatus::Success => match item.response.gateway_code {
                MpgsResponseGatewayCode::Approved
                | MpgsResponseGatewayCode::ApprovedPendingSettlement => {
                    match item.transaction.r#type {
                        MpgsTransactionType::Authorization => Self::Authorized,
                        MpgsTransactionType::Payment | MpgsTransactionType::Capture => {
                            Self::Charged
                        }
                        MpgsTransactionType::Void => Self::Voided,
                        _ => Self::Pending,
                    }
                }
                MpgsResponseGatewayCode::AuthenticationRequired => Self::AuthenticationPending,
                _ => Self::Failure,
            },
            MpgsPaymentStatus::Pending => match item.response.gateway_code {
                MpgsResponseGatewayCode::Pending | MpgsResponseGatewayCode::Submitted => {
                    Self::Pending
                }
                _ => Self::Failure,
            },
            MpgsPaymentStatus::Failure | MpgsPaymentStatus::Unknown => Self::Failure,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MpgsPaymentsResponse {
    result: MpgsPaymentStatus,
    transaction: MpgsTransactionResponse,
    response: MpgsResponse,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MpgsTransactionResponse {
    pub id: String,
    pub r#type: MpgsTransactionType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MpgsResponse {
    pub gateway_code: MpgsResponseGatewayCode,
}

impl<F, T> TryFrom<ResponseRouterData<F, MpgsPaymentsResponse, T, PaymentsResponseData>>
    for RouterData<F, T, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, MpgsPaymentsResponse, T, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            status: common_enums::AttemptStatus::from(item.response.clone()),
            response: Ok(PaymentsResponseData::TransactionResponse {
                resource_id: ResponseId::ConnectorTransactionId(item.response.transaction.id),
                redirection_data: Box::new(None),
                mandate_reference: Box::new(None),
                connector_metadata: None,
                network_txn_id: None,
                connector_response_reference_id: None,
                incremental_authorization_allowed: None,
                charges: None,
            }),
            ..item.data
        })
    }
}

#[derive(Default, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MpgsRefundRequest {
    api_operation: MpgsApiOperation,
    transaction: MpgsRefundTransaction,
}

#[derive(Default, Debug, Serialize)]
pub struct MpgsRefundTransaction {
    amount: String,
    currency: String,
}

impl<F> TryFrom<&MpgsRouterData<&RefundsRouterData<F>>> for MpgsRefundRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &MpgsRouterData<&RefundsRouterData<F>>) -> Result<Self, Self::Error> {
        Ok(Self {
            api_operation: MpgsApiOperation::Refund,
            transaction: MpgsRefundTransaction {
                amount: item.amount.to_owned(),
                currency: item.router_data.request.currency.to_string(),
            },
        })
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MpgsRefundResponse {
    result: MpgsPaymentStatus,
    transaction: MpgsTransactionResponse,
}

impl From<MpgsPaymentStatus> for enums::RefundStatus {
    fn from(item: MpgsPaymentStatus) -> Self {
        match item {
            MpgsPaymentStatus::Success => Self::Success,
            MpgsPaymentStatus::Failure => Self::Failure,
            MpgsPaymentStatus::Pending | MpgsPaymentStatus::Unknown => Self::Pending,
        }
    }
}

impl TryFrom<RefundsResponseRouterData<Execute, MpgsRefundResponse>>
    for RefundsRouterData<Execute>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: RefundsResponseRouterData<Execute, MpgsRefundResponse>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(RefundsResponseData {
                connector_refund_id: item.response.transaction.id.to_string(),
                refund_status: enums::RefundStatus::from(item.response.result),
            }),
            ..item.data
        })
    }
}

impl TryFrom<RefundsResponseRouterData<RSync, MpgsRefundResponse>> for RefundsRouterData<RSync> {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: RefundsResponseRouterData<RSync, MpgsRefundResponse>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(RefundsResponseData {
                connector_refund_id: item.response.transaction.id.to_string(),
                refund_status: enums::RefundStatus::from(item.response.result),
            }),
            ..item.data
        })
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MpgsErrorResponse {
    pub error: MpgsError,
    pub result: MpgsPaymentStatus,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MpgsError {
    pub cause: String,
    pub explanation: String,
}
