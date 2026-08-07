use common_enums::FraudCheckStatus;
use common_utils::pii::SecretSerdeValue;
use hyperswitch_domain_models::{
    router_data::ConnectorAuthType, router_request_types::ResponseId,
    router_response_types::fraud_check::FraudCheckResponseData,
};
use api_models::payments::{AdditionalPaymentData, additional_info::BankDebitAdditionalData};
use hyperswitch_interfaces::errors::ConnectorError;
use hyperswitch_masking::Secret;
use serde::{Deserialize, Serialize};
use error_stack::ResultExt;

use crate::{types::{FrmCheckoutRouterData, ResponseRouterData}, utils::get_unimplemented_payment_method_error_message};

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
    payment_method: PaymentMethod,
    created_at: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PaymentMethod {
    Eft,
}

impl TryFrom<&FrmCheckoutRouterData> for SanlamPayshieldCheckoutRequest {
    type Error = error_stack::Report<ConnectorError>;

    fn try_from(data: &FrmCheckoutRouterData) -> Result<Self, Self::Error> {
        let connector_id = data
            .request
            .gateway
            .clone()
            .ok_or(ConnectorError::MissingRequiredField {
                field_name: "gateway",
            })?;
        let currency = data
            .request
            .currency
            .ok_or(ConnectorError::MissingRequiredField {
                field_name: "currency",
            })?;
        let payment_method = match data.request.payment_method_data.as_ref() {
            Some(AdditionalPaymentData::BankDebit {
                details: Some(BankDebitAdditionalData::EftDebitOrder { .. }),
            }) => Ok(PaymentMethod::Eft),

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
                payment_method,
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
    rule_config_version: String,
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
        let connector_metadata = serde_json::to_value(&item.response).ok();
        let reason = serde_json::json!({
            "reason": item.response.reason,
            "reasonCodes": item.response.reason_codes,
            "evaluatedChecks": item.response.evaluated_checks,
        });

        Ok(Self {
            response: Ok(FraudCheckResponseData::TransactionResponse {
                resource_id: ResponseId::ConnectorTransactionId(item.response.request_id),
                status: item.response.decision.into(),
                connector_metadata,
                reason: Some(reason),
                score: Some(item.response.severity),
            }),
            ..item.data
        })
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SanlamPayshieldErrorResponse {
    pub error_code: i64,
    pub error_message: String,
    #[serde(default)]
    pub inner_errors: Vec<SanlamPayshieldErrorResponse>,
}

impl SanlamPayshieldErrorResponse {
    pub fn reason(&self) -> Option<String> {
        (!self.inner_errors.is_empty())
            .then(|| serde_json::to_string(&self.inner_errors).ok())
            .flatten()
    }
}

#[cfg(test)]
mod tests {
    use common_utils::pii::SecretSerdeValue;

    use super::{
        ConnectorType, Decision, PaymentMethod, SanlamPayshieldCheckoutRequest,
        SanlamPayshieldCheckoutResponse, SanlamPayshieldErrorResponse, Transaction,
    };

    #[test]
    fn serializes_checkout_request_with_opaque_metadata() {
        let request = SanlamPayshieldCheckoutRequest {
            request_id: "req_123456789".to_string(),
            profile_id: "profile_123456789".to_string(),
            connector_id: "connector_123456789".to_string(),
            connector_type: ConnectorType::Payin,
            transaction: Transaction {
                payment_id: "pay_123456789".to_string(),
                amount_in_cents: "10000".to_string(),
                currency: "ZAR".to_string(),
                payment_method: PaymentMethod::Eft,
                created_at: "2026-08-02T10:30:00+02:00".to_string(),
            },
            metadata: Some(SecretSerdeValue::new(serde_json::json!({
                "actionDate": "2026-08-02T10:30:00+02:00"
            }))),
        };

        assert_eq!(
            serde_json::to_value(request).expect("request must serialize"),
            serde_json::json!({
                "requestId": "req_123456789",
                "profileId": "profile_123456789",
                "connectorId": "connector_123456789",
                "connectorType": "Payin",
                "transaction": {
                    "paymentId": "pay_123456789",
                    "amountInCents": "10000",
                    "currency": "ZAR",
                    "paymentMethod": "EFT",
                    "createdAt": "2026-08-02T10:30:00+02:00"
                },
                "metadata": {
                    "actionDate": "2026-08-02T10:30:00+02:00"
                }
            })
        );
    }

    #[test]
    fn accepts_only_recognized_decisions() {
        let accepted: SanlamPayshieldCheckoutResponse = serde_json::from_value(serde_json::json!({
            "requestId": "req_123456789",
            "decision": "ACCEPT",
            "severity": 0,
            "reasonCodes": [],
            "reason": "accepted",
            "ruleConfigVersion": "dev-0.1.0",
            "evaluatedChecks": ["ACCOUNT_STATUS"]
        }))
        .expect("ACCEPT must deserialize");
        assert!(matches!(accepted.decision, Decision::Accept));

        let rejected: SanlamPayshieldCheckoutResponse = serde_json::from_value(serde_json::json!({
            "requestId": "req_123456789",
            "decision": "REJECT",
            "severity": 3,
            "reasonCodes": ["ACTION_DATE_PUBLIC_HOLIDAY"],
            "reason": "rejected",
            "ruleConfigVersion": "dev-0.1.0",
            "evaluatedChecks": ["ACTION_DATE"]
        }))
        .expect("REJECT must deserialize");
        assert!(matches!(rejected.decision, Decision::Reject));

        let invalid =
            serde_json::from_value::<SanlamPayshieldCheckoutResponse>(serde_json::json!({
                "requestId": "req_123456789",
                "decision": "REVIEW",
                "severity": 1,
                "reasonCodes": [],
                "reason": "manual review",
                "ruleConfigVersion": "dev-0.1.0",
                "evaluatedChecks": []
            }));
        assert!(invalid.is_err());
    }

    #[test]
    fn preserves_nested_error_details() {
        let error: SanlamPayshieldErrorResponse = serde_json::from_value(serde_json::json!({
            "errorCode": 4000,
            "errorMessage": "Invalid request payload.",
            "innerErrors": [{
                "errorCode": 4001,
                "errorMessage": "requestId is required.",
                "innerErrors": []
            }]
        }))
        .expect("error response must deserialize");

        let reason = error.reason().expect("nested errors must be retained");
        assert!(reason.contains("requestId is required."));
    }
}
