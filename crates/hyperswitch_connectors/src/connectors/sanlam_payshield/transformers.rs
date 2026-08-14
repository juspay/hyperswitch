use api_models::payments::{additional_info::BankDebitAdditionalData, AdditionalPaymentData};
use common_enums::FraudCheckStatus;
use common_utils::pii::SecretSerdeValue;
use error_stack::ResultExt;
use hyperswitch_domain_models::{
    router_data::ConnectorAuthType, router_request_types::ResponseId,
    router_response_types::fraud_check::FraudCheckResponseData,
};
use hyperswitch_interfaces::errors::ConnectorError;
use hyperswitch_masking::Secret;
use serde::{Deserialize, Serialize};

use crate::{
    types::{FrmCheckoutRouterData, ResponseRouterData},
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
    amount_in_cents: String,
    currency: String,
    payment_method_type: PaymentMethodType,
    created_at: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum PaymentMethodType {
    EftDebitOrder,
}

impl TryFrom<&FrmCheckoutRouterData> for SanlamPayshieldCheckoutRequest {
    type Error = error_stack::Report<ConnectorError>;

    fn try_from(data: &FrmCheckoutRouterData) -> Result<Self, Self::Error> {
        let connector_id = data
            .request
            .gateway_mca_id
            .as_ref()
            .ok_or(ConnectorError::MissingRequiredField {
                field_name: "gateway",
            })?
            .get_string_repr()
            .to_owned();
        let currency = data
            .request
            .currency
            .ok_or(ConnectorError::MissingRequiredField {
                field_name: "currency",
            })?;
        let payment_method_type = match data.request.payment_method_data.as_ref() {
            Some(AdditionalPaymentData::BankDebit {
                details: Some(BankDebitAdditionalData::EftDebitOrder { .. }),
            }) => Ok(PaymentMethodType::EftDebitOrder),

            _ => Err(ConnectorError::NotImplemented(
                get_unimplemented_payment_method_error_message("sanlam_paysheild"),
            )),
        }?;

        Ok(Self {
            request_id: data.connector_request_reference_id.clone(),
            profile_id: data.request.profile_id.get_string_repr().to_owned(),
            connector_id,
            connector_type: ConnectorType::Payin,
            transaction: Transaction {
                payment_id: data.payment_id.clone(),
                amount_in_cents: data.request.amount.to_string(),
                currency: currency.to_string(),
                payment_method_type,
                created_at: get_current_time()?,
            },
            metadata: data.frm_metadata.clone(),
        })
    }
}

fn get_current_time() -> Result<String, error_stack::Report<ConnectorError>> {
    let format = time::macros::format_description!(
        "[year]-[month]-[day]T[hour]:[minute]:[second][offset_hour sign:mandatory]:[offset_minute]"
    );

    let time = time::OffsetDateTime::now_utc()
        .to_offset(time::macros::offset!(+2))
        .format(&format)
        .change_context(ConnectorError::RequestEncodingFailed)?;

    Ok(time)
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

impl
    TryFrom<
        ResponseRouterData<
            hyperswitch_domain_models::router_flow_types::Checkout,
            SanlamPayshieldCheckoutResponse,
            hyperswitch_domain_models::router_request_types::fraud_check::FraudCheckCheckoutData,
            FraudCheckResponseData,
        >,
    > for FrmCheckoutRouterData
{
    type Error = error_stack::Report<ConnectorError>;

    fn try_from(
        item: ResponseRouterData<
            hyperswitch_domain_models::router_flow_types::Checkout,
            SanlamPayshieldCheckoutResponse,
            hyperswitch_domain_models::router_request_types::fraud_check::FraudCheckCheckoutData,
            FraudCheckResponseData,
        >,
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
