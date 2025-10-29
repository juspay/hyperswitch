use api_models::webhooks::IncomingWebhookEvent;
use common_utils::{pii, types::FloatMajorUnit};
use hyperswitch_domain_models::{
    payment_method_data::{BankTransferData, PaymentMethodData},
    router_data::{ConnectorAuthType, RouterData},
    router_request_types::ResponseId,
    router_response_types::PaymentsResponseData,
    types::PaymentsAuthorizeRouterData,
};
use hyperswitch_interfaces::errors::ConnectorError;
use masking::Secret;
use serde::{Deserialize, Serialize};

use crate::{
    types::ResponseRouterData,
    utils::{get_unimplemented_payment_method_error_message, RouterData as _},
};

#[derive(Debug, Serialize)]
pub struct CheckbookPaymentsRequest {
    name: Secret<String>,
    recipient: pii::Email,
    amount: FloatMajorUnit,
    description: String,
}

impl TryFrom<(FloatMajorUnit, &PaymentsAuthorizeRouterData)> for CheckbookPaymentsRequest {
    type Error = error_stack::Report<ConnectorError>;
    fn try_from(
        (amount, item): (FloatMajorUnit, &PaymentsAuthorizeRouterData),
    ) -> Result<Self, Self::Error> {
        match item.request.payment_method_data.clone() {
            PaymentMethodData::BankTransfer(bank_transfer_data) => match *bank_transfer_data {
                BankTransferData::AchBankTransfer {} => Ok(Self {
                    name: item.get_billing_full_name()?,
                    recipient: item.get_billing_email()?,
                    amount,
                    description: item.get_description()?,
                }),
                _ => Err(ConnectorError::NotImplemented(
                    get_unimplemented_payment_method_error_message("Checkbook"),
                )
                .into()),
            },
            _ => Err(ConnectorError::NotImplemented(
                get_unimplemented_payment_method_error_message("Checkbook"),
            )
            .into()),
        }
    }
}

pub struct CheckbookAuthType {
    pub(super) publishable_key: Secret<String>,
    pub(super) secret_key: Secret<String>,
}

impl TryFrom<&ConnectorAuthType> for CheckbookAuthType {
    type Error = error_stack::Report<ConnectorError>;
    fn try_from(auth_type: &ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            ConnectorAuthType::BodyKey { key1, api_key } => Ok(Self {
                publishable_key: key1.to_owned(),
                secret_key: api_key.to_owned(),
            }),
            _ => Err(ConnectorError::FailedToObtainAuthType.into()),
        }
    }
}
// PaymentsResponse
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CheckbookPaymentStatus {
    Unpaid,
    InProcess,
    Paid,
    Mailed,
    Printed,
    Failed,
    Expired,
    Void,
    #[default]
    Processing,
}

impl From<CheckbookPaymentStatus> for common_enums::AttemptStatus {
    fn from(item: CheckbookPaymentStatus) -> Self {
        match item {
            CheckbookPaymentStatus::Paid
            | CheckbookPaymentStatus::Mailed
            | CheckbookPaymentStatus::Printed => Self::Charged,
            CheckbookPaymentStatus::Failed | CheckbookPaymentStatus::Expired => Self::Failure,
            CheckbookPaymentStatus::Unpaid => Self::AuthenticationPending,
            CheckbookPaymentStatus::InProcess | CheckbookPaymentStatus::Processing => Self::Pending,
            CheckbookPaymentStatus::Void => Self::Voided,
        }
    }
}

impl From<CheckbookPaymentStatus> for IncomingWebhookEvent {
    fn from(status: CheckbookPaymentStatus) -> Self {
        match status {
            CheckbookPaymentStatus::Mailed
            | CheckbookPaymentStatus::Printed
            | CheckbookPaymentStatus::Paid => Self::PaymentIntentSuccess,
            CheckbookPaymentStatus::Failed | CheckbookPaymentStatus::Expired => {
                Self::PaymentIntentFailure
            }
            CheckbookPaymentStatus::Unpaid
            | CheckbookPaymentStatus::InProcess
            | CheckbookPaymentStatus::Processing => Self::PaymentIntentProcessing,
            CheckbookPaymentStatus::Void => Self::PaymentIntentCancelled,
        }
    }
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CheckbookPaymentsResponse {
    pub status: CheckbookPaymentStatus,
    pub id: String,
    pub amount: Option<FloatMajorUnit>,
    pub description: Option<String>,
    pub name: Option<String>,
    pub recipient: Option<String>,
}

impl<F, T> TryFrom<ResponseRouterData<F, CheckbookPaymentsResponse, T, PaymentsResponseData>>
    for RouterData<F, T, PaymentsResponseData>
{
    type Error = error_stack::Report<ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, CheckbookPaymentsResponse, T, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            status: common_enums::AttemptStatus::from(item.response.status),
            response: Ok(PaymentsResponseData::TransactionResponse {
                resource_id: ResponseId::ConnectorTransactionId(item.response.id),
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

#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
pub struct CheckbookErrorResponse {
    pub status_code: u16,
    pub code: String,
    pub message: String,
    pub reason: Option<String>,
}
