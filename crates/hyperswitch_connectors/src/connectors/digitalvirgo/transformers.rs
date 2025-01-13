use common_enums::enums;
use common_utils::types::FloatMajorUnit;
use error_stack::ResultExt;
use hyperswitch_domain_models::{
    payment_method_data::{MobilePaymentData, PaymentMethodData},
    router_data::{ConnectorAuthType, RouterData},
    router_flow_types::refunds::{Execute, RSync},
    router_request_types::ResponseId,
    router_response_types::{PaymentsResponseData, RefundsResponseData},
    types::{PaymentsAuthorizeRouterData, PaymentsCompleteAuthorizeRouterData, RefundsRouterData},
};
use hyperswitch_interfaces::errors;
use masking::{ExposeInterface, Secret};
use serde::{Deserialize, Serialize};

use crate::{
    types::{RefundsResponseRouterData, ResponseRouterData},
    utils::{PaymentsAuthorizeRequestData, PaymentsCompleteAuthorizeRequestData},
};

pub struct DigitalvirgoRouterData<T> {
    pub amount: FloatMajorUnit,
    pub surcharge_amount: Option<FloatMajorUnit>,
    pub router_data: T,
}

impl<T> From<(FloatMajorUnit, Option<FloatMajorUnit>, T)> for DigitalvirgoRouterData<T> {
    fn from((amount, surcharge_amount, item): (FloatMajorUnit, Option<FloatMajorUnit>, T)) -> Self {
        Self {
            amount,
            surcharge_amount,
            router_data: item,
        }
    }
}

#[derive(Default, Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DigitalvirgoPaymentsRequest {
    amount: FloatMajorUnit,
    amount_surcharge: Option<FloatMajorUnit>,
    client_uid: Option<String>,
    msisdn: String,
    product_name: String,
    description: Option<String>,
    partner_transaction_id: String,
}

impl TryFrom<&DigitalvirgoRouterData<&PaymentsAuthorizeRouterData>>
    for DigitalvirgoPaymentsRequest
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &DigitalvirgoRouterData<&PaymentsAuthorizeRouterData>,
    ) -> Result<Self, Self::Error> {
        match item.router_data.request.payment_method_data.clone() {
            PaymentMethodData::MobilePayment(mobile_payment_data) => match mobile_payment_data {
                MobilePaymentData::DirectCarrierBilling { msisdn, client_uid } => {
                    let order_details = item.router_data.request.get_order_details()?;
                    let product_name = order_details
                        .first()
                        .map(|order| order.product_name.to_owned())
                        .ok_or(errors::ConnectorError::MissingRequiredField {
                            field_name: "product_name",
                        })?;

                    Ok(Self {
                        amount: item.amount.to_owned(),
                        amount_surcharge: item.surcharge_amount.to_owned(),
                        client_uid,
                        msisdn,
                        product_name,
                        description: item.router_data.description.to_owned(),
                        partner_transaction_id: item
                            .router_data
                            .connector_request_reference_id
                            .to_owned(),
                    })
                }
            },
            _ => Err(errors::ConnectorError::NotImplemented("Payment method".to_string()).into()),
        }
    }
}

pub struct DigitalvirgoAuthType {
    pub(super) username: Secret<String>,
    pub(super) password: Secret<String>,
}

impl TryFrom<&ConnectorAuthType> for DigitalvirgoAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            ConnectorAuthType::BodyKey { api_key, key1 } => Ok(Self {
                username: key1.to_owned(),
                password: api_key.to_owned(),
            }),
            _ => Err(errors::ConnectorError::FailedToObtainAuthType.into()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DigitalvirgoPaymentStatus {
    Ok,
    ConfirmPayment,
}

impl From<DigitalvirgoPaymentStatus> for common_enums::AttemptStatus {
    fn from(item: DigitalvirgoPaymentStatus) -> Self {
        match item {
            DigitalvirgoPaymentStatus::Ok => Self::Charged,
            DigitalvirgoPaymentStatus::ConfirmPayment => Self::AuthenticationPending,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DigitalvirgoPaymentsResponse {
    state: DigitalvirgoPaymentStatus,
    transaction_id: String,
    consent: Option<DigitalvirgoConsentStatus>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DigitalvirgoConsentStatus {
    required: Option<bool>,
}

impl<F, T> TryFrom<ResponseRouterData<F, DigitalvirgoPaymentsResponse, T, PaymentsResponseData>>
    for RouterData<F, T, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, DigitalvirgoPaymentsResponse, T, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        // show if consent is required in next action
        let connector_metadata = item
            .response
            .consent
            .and_then(|consent_status| {
                consent_status.required.map(|consent_required| {
                    if consent_required {
                        serde_json::json!({
                            "consent_data_required": "consent_required",
                        })
                    } else {
                        serde_json::json!({
                            "consent_data_required": "consent_not_required",
                        })
                    }
                })
            })
            .or(Some(serde_json::json!({
                "consent_data_required": "consent_not_required",
            })));
        Ok(Self {
            status: common_enums::AttemptStatus::from(item.response.state),
            response: Ok(PaymentsResponseData::TransactionResponse {
                resource_id: ResponseId::ConnectorTransactionId(item.response.transaction_id),
                redirection_data: Box::new(None),
                mandate_reference: Box::new(None),
                connector_metadata,
                network_txn_id: None,
                connector_response_reference_id: None,
                incremental_authorization_allowed: None,
                charges: None,
            }),
            ..item.data
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DigitalvirgoPaymentSyncStatus {
    Accepted,
    Payed,
    Pending,
    Cancelled,
    Rejected,
    Locked,
}

impl From<DigitalvirgoPaymentSyncStatus> for common_enums::AttemptStatus {
    fn from(item: DigitalvirgoPaymentSyncStatus) -> Self {
        match item {
            DigitalvirgoPaymentSyncStatus::Accepted => Self::AuthenticationPending,
            DigitalvirgoPaymentSyncStatus::Payed => Self::Charged,
            DigitalvirgoPaymentSyncStatus::Pending | DigitalvirgoPaymentSyncStatus::Locked => {
                Self::Pending
            }
            DigitalvirgoPaymentSyncStatus::Cancelled => Self::Voided,
            DigitalvirgoPaymentSyncStatus::Rejected => Self::Failure,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DigitalvirgoPaymentSyncResponse {
    payment_status: DigitalvirgoPaymentSyncStatus,
    transaction_id: String,
}

impl<F, T> TryFrom<ResponseRouterData<F, DigitalvirgoPaymentSyncResponse, T, PaymentsResponseData>>
    for RouterData<F, T, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, DigitalvirgoPaymentSyncResponse, T, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            status: common_enums::AttemptStatus::from(item.response.payment_status),
            response: Ok(PaymentsResponseData::TransactionResponse {
                resource_id: ResponseId::ConnectorTransactionId(item.response.transaction_id),
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
pub struct DigitalvirgoConfirmRequest {
    transaction_id: String,
    token: Secret<String>,
}

#[derive(Default, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DigitalvirgoRedirectResponseData {
    otp: Secret<String>,
}

impl TryFrom<&PaymentsCompleteAuthorizeRouterData> for DigitalvirgoConfirmRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &PaymentsCompleteAuthorizeRouterData) -> Result<Self, Self::Error> {
        let payload_data = item.request.get_redirect_response_payload()?.expose();

        let otp_data: DigitalvirgoRedirectResponseData = serde_json::from_value(payload_data)
            .change_context(errors::ConnectorError::MissingConnectorRedirectionPayload {
                field_name: "otp for transaction",
            })?;

        Ok(Self {
            transaction_id: item
                .request
                .connector_transaction_id
                .clone()
                .ok_or(errors::ConnectorError::MissingConnectorTransactionID)?,
            token: otp_data.otp,
        })
    }
}

#[derive(Default, Debug, Serialize)]
pub struct DigitalvirgoRefundRequest {
    pub amount: FloatMajorUnit,
}

impl<F> TryFrom<&DigitalvirgoRouterData<&RefundsRouterData<F>>> for DigitalvirgoRefundRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &DigitalvirgoRouterData<&RefundsRouterData<F>>) -> Result<Self, Self::Error> {
        Ok(Self {
            amount: item.amount.to_owned(),
        })
    }
}

#[allow(dead_code)]
#[derive(Debug, Serialize, Default, Deserialize, Clone)]
pub enum RefundStatus {
    Succeeded,
    Failed,
    #[default]
    Processing,
}

impl From<RefundStatus> for enums::RefundStatus {
    fn from(item: RefundStatus) -> Self {
        match item {
            RefundStatus::Succeeded => Self::Success,
            RefundStatus::Failed => Self::Failure,
            RefundStatus::Processing => Self::Pending,
        }
    }
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct RefundResponse {
    id: String,
    status: RefundStatus,
}

impl TryFrom<RefundsResponseRouterData<Execute, RefundResponse>> for RefundsRouterData<Execute> {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: RefundsResponseRouterData<Execute, RefundResponse>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(RefundsResponseData {
                connector_refund_id: item.response.id.to_string(),
                refund_status: enums::RefundStatus::from(item.response.status),
            }),
            ..item.data
        })
    }
}

impl TryFrom<RefundsResponseRouterData<RSync, RefundResponse>> for RefundsRouterData<RSync> {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: RefundsResponseRouterData<RSync, RefundResponse>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(RefundsResponseData {
                connector_refund_id: item.response.id.to_string(),
                refund_status: enums::RefundStatus::from(item.response.status),
            }),
            ..item.data
        })
    }
}

#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
pub struct DigitalvirgoErrorResponse {
    pub cause: Option<String>,
    pub operation_error: Option<String>,
    pub description: Option<String>,
}
