use common_enums::{Currency, FraudCheckStatus};
use common_utils::{ext_traits::ValueExt, pii::SecretSerdeValue, types::MinorUnit};
use error_stack::ResultExt;
use hyperswitch_domain_models::{
    router_data::{ConnectorAuthType, RouterData},
    router_request_types::ResponseId,
    router_response_types::fraud_check::FraudCheckResponseData,
};
use hyperswitch_interfaces::errors::ConnectorError;
use hyperswitch_masking::Secret;
use serde::{Deserialize, Serialize};

use crate::{
    types::{FrmCheckoutRouterData, PoFrmRouterData, ResponseRouterData},
    utils::get_unimplemented_payment_method_error_message,
};

pub struct SanlamPayshieldAuthType {
    pub(super) api_key: Secret<String>,
}

impl TryFrom<&ConnectorAuthType> for SanlamPayshieldAuthType {
    type Error = error_stack::Report<ConnectorError>;

    fn try_from(auth_type: &ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            ConnectorAuthType::HeaderKey { api_key } => Ok(Self {
                api_key: api_key.to_owned(),
            }),
            _ => Err(ConnectorError::FailedToObtainAuthType.into()),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SanlamPayshieldFrmMetadata {
    pub profile_id: String,
    pub connector_id: Option<String>,
    pub created_at: time::PrimitiveDateTime,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SanlamPayshieldCheckoutRequest {
    request_id: String,
    profile_id: String,
    connector_id: String,
    connector_type: ConnectorType,
    transaction: Transaction,
    metadata: Option<SecretSerdeValue>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ConnectorType {
    Payin,
    Payout,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Transaction {
    payment_id: String,
    amount_in_cents: MinorUnit,
    currency: Currency,
    payment_method_type: PaymentMethodType,
    created_at: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PaymentMethodType {
    EftDebitOrder,
    Payshap,
    PayshapProxy,
}

impl TryFrom<&common_enums::PaymentMethodType> for PaymentMethodType {
    type Error = error_stack::Report<ConnectorError>;

    fn try_from(
        payment_method_type: &common_enums::PaymentMethodType,
    ) -> Result<Self, Self::Error> {
        match payment_method_type {
            common_enums::PaymentMethodType::EftDebitOrder => Ok(Self::EftDebitOrder),
            common_enums::PaymentMethodType::Payshap => Ok(Self::Payshap),
            common_enums::PaymentMethodType::PayshapProxy => Ok(Self::PayshapProxy),
            _ => Err(ConnectorError::NotImplemented(
                get_unimplemented_payment_method_error_message("sanlam_paysheild"),
            ))?,
        }
    }
}

impl TryFrom<&FrmCheckoutRouterData> for SanlamPayshieldCheckoutRequest {
    type Error = error_stack::Report<ConnectorError>;

    fn try_from(data: &FrmCheckoutRouterData) -> Result<Self, Self::Error> {
        let SanlamPayshieldFrmMetadata {
            profile_id,
            connector_id,
            created_at,
        } = data
            .request
            .gateway_metadata
            .clone()
            .ok_or(ConnectorError::MissingRequiredField {
                field_name: "gateway_metadata".into(),
            })?
            .parse_value("SanlamPayshieldFrmMetadata")
            .change_context(ConnectorError::RequestEncodingFailed)
            .attach_printable("Failed to parse SanlamPayshieldFrmMetadata")?;

        let connector_id = connector_id.ok_or(ConnectorError::MissingRequiredField {
            field_name: "connector_id".into(),
        })?;

        let currency = data
            .request
            .currency
            .ok_or(ConnectorError::MissingRequiredField {
                field_name: "currency".into(),
            })?;

        let payment_method_type = data
            .payment_method_type
            .as_ref()
            .map(PaymentMethodType::try_from)
            .transpose()?
            .ok_or(ConnectorError::MissingRequiredField {
                field_name: "payment_method_type".into(),
            })?;

        let created_at = created_at
            .assume_utc()
            .to_offset(time::macros::offset!(+2))
            .format(time::macros::format_description!(
                "[year]-[month]-[day]T[hour]:[minute]:[second][offset_hour sign:mandatory]:[offset_minute]"
            ))
            .change_context(ConnectorError::RequestEncodingFailed)?;

        Ok(Self {
            request_id: data.connector_request_reference_id.clone(),
            profile_id,
            connector_id,
            connector_type: ConnectorType::Payin,
            transaction: Transaction {
                payment_id: data.payment_id.clone(),
                amount_in_cents: data.request.amount,
                currency,
                payment_method_type,
                created_at: get_current_time()?,
            },
            metadata: data.frm_metadata.clone(),
        })
    }
}

impl TryFrom<&PoFrmRouterData> for SanlamPayshieldCheckoutRequest {
    type Error = error_stack::Report<ConnectorError>;

    fn try_from(data: &PoFrmRouterData) -> Result<Self, Self::Error> {
        let SanlamPayshieldFrmMetadata {
            profile_id,
            connector_id,
        } = data
            .request
            .gateway_metadata
            .clone()
            .ok_or(ConnectorError::MissingRequiredField {
                field_name: "gateway_metadata".into(),
            })?
            .parse_value("SanlamPayshieldFrmMetadata")
            .change_context(ConnectorError::RequestEncodingFailed)
            .attach_printable("Failed to parse SanlamPayshieldFrmMetadata")?;

        let connector_id = connector_id.ok_or(ConnectorError::MissingRequiredField {
            field_name: "connector_id".into(),
        })?;

        let payment_method_type = data
            .payment_method_type
            .as_ref()
            .map(PaymentMethodType::try_from)
            .transpose()?
            .ok_or(ConnectorError::MissingRequiredField {
                field_name: "payment_method_type".into(),
            })?;

        let payout_id = data
            .payout_id
            .clone()
            .ok_or(ConnectorError::MissingRequiredField {
                field_name: "payout_id".into(),
            })?;

        Ok(Self {
            request_id: data.connector_request_reference_id.clone(),
            profile_id,
            connector_id,
            connector_type: ConnectorType::Payout,
            transaction: Transaction {
                payment_id: payout_id,
                amount_in_cents: data.request.amount,
                currency: data.request.currency,
                payment_method_type,
                created_at,
            },
            metadata: data.frm_metadata.clone(),
        })
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SanlamPayshieldCheckoutResponse {
    request_id: String,
    decision: Decision,
    severity: i32,
    reason_codes: Option<Vec<String>>,
    reason: Option<String>,
    rule_config_version: Option<String>,
    evaluated_checks: Vec<String>,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Decision {
    Accept,
    Reject,
}

impl From<Decision> for FraudCheckStatus {
    fn from(decision: Decision) -> Self {
        match decision {
            Decision::Accept => Self::Legit,
            Decision::Reject => Self::Fraud,
        }
    }
}

impl<F, T>
    TryFrom<ResponseRouterData<F, SanlamPayshieldCheckoutResponse, T, FraudCheckResponseData>>
    for RouterData<F, T, FraudCheckResponseData>
{
    type Error = error_stack::Report<ConnectorError>;

    fn try_from(
        item: ResponseRouterData<F, SanlamPayshieldCheckoutResponse, T, FraudCheckResponseData>,
    ) -> Result<Self, Self::Error> {
        let connector_metadata = serde_json::json!({
            "reasonCodes": item.response.reason_codes,
            "evaluatedChecks": item.response.evaluated_checks,
        });

        Ok(Self {
            response: Ok(FraudCheckResponseData::TransactionResponse {
                resource_id: ResponseId::ConnectorTransactionId(item.response.request_id),
                status: item.response.decision.into(),
                connector_metadata: Some(connector_metadata),
                reason: item.response.reason.map(serde_json::Value::String),
                score: Some(item.response.severity),
            }),
            ..item.data
        })
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SanlamPayshieldErrorResponse {
    pub error_code: Option<i64>,
    pub error_message: Option<String>,
    #[serde(default)]
    pub inner_errors: Vec<Self>,
    pub message: Option<String>,
}

impl SanlamPayshieldErrorResponse {
    pub fn reason(&self) -> Option<String> {
        (!self.inner_errors.is_empty())
            .then(|| serde_json::to_string(&self.inner_errors).ok())
            .flatten()
    }
}
