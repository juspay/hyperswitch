use common_utils::errors::CustomResult;
use error_stack::{IntoReport, ResultExt};
use masking::PeekInterface;
use serde::{Deserialize, Serialize};
use storage_models::enums;
use uuid::Uuid;

use super::{requests::*, response::*};
use crate::{
    core::errors,
    types::{self, api},
};

fn to_int(
    val: masking::Secret<String, masking::WithType>,
) -> CustomResult<i32, errors::ConnectorError> {
    val.peek()
        .parse()
        .into_report()
        .change_context(errors::ConnectorError::RequestEncodingFailed)
}

fn fetch_payment_instrument(
    payment_method: api::PaymentMethod,
) -> CustomResult<PaymentInstrument, errors::ConnectorError> {
    match payment_method {
        api::PaymentMethod::Card(card) => Ok(PaymentInstrument::Card(CardPayment::new(
            CardExpiryDate::new(to_int(card.card_exp_month)?, to_int(card.card_exp_year)?),
            card.card_number.peek().to_string(),
        ))),
        api::PaymentMethod::Wallet(wallet) => match wallet.issuer_name {
            api_models::enums::WalletIssuer::ApplePay => Ok(PaymentInstrument::Applepay(
                WalletPayment::new(PaymentType::Applepay, wallet.token),
            )),
            api_models::enums::WalletIssuer::GooglePay => Ok(PaymentInstrument::Googlepay(
                WalletPayment::new(PaymentType::Googlepay, wallet.token),
            )),
            _ => Err(errors::ConnectorError::NotImplemented("Wallet Type".to_string()).into()),
        },
        _ => {
            Err(errors::ConnectorError::NotImplemented("Current Payment Method".to_string()).into())
        }
    }
}

impl TryFrom<&types::PaymentsAuthorizeRouterData> for PaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsAuthorizeRouterData) -> Result<Self, Self::Error> {
        Ok(Self::new(
            Instruction::new(
                PaymentValue::new(item.request.amount, item.request.currency.to_string()),
                InstructionNarrative::new(item.merchant_id.clone()),
                fetch_payment_instrument(item.request.payment_method_data.clone())?,
            ),
            Merchant::new(item.merchant_id.clone()),
            Uuid::new_v4().to_string(),
        ))
    }
}

pub struct WorldpayAuthType {
    pub(super) api_key: String,
}

impl TryFrom<&types::ConnectorAuthType> for WorldpayAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &types::ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            types::ConnectorAuthType::HeaderKey { api_key } => Ok(Self {
                api_key: api_key.to_string(),
            }),
            _ => Err(errors::ConnectorError::FailedToObtainAuthType)?,
        }
    }
}

impl From<Outcome> for enums::AttemptStatus {
    fn from(item: Outcome) -> Self {
        match item {
            Outcome::Authorized => Self::Authorized,
            Outcome::Refused => Self::Failure,
        }
    }
}

impl TryFrom<types::PaymentsResponseRouterData<PaymentsResponse>>
    for types::PaymentsAuthorizeRouterData
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::PaymentsResponseRouterData<PaymentsResponse>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            status: match item.response.outcome {
                Some(outcome) => enums::AttemptStatus::from(outcome),
                None => Err(errors::ConnectorError::MissingRequiredField {
                    field_name: "outcome".to_string(),
                })?,
            },
            description: item.response.description,
            response: Ok(types::PaymentsResponseData::TransactionResponse {
                resource_id: types::ResponseId::try_from(item.response._links)?,
                redirection_data: None,
                redirect: false,
                mandate_reference: None,
            }),
            ..item.data
        })
    }
}

impl<F> TryFrom<&types::RefundsRouterData<F>> for WorldpayRefundRequest {
    type Error = error_stack::Report<errors::ParsingError>;
    fn try_from(item: &types::RefundsRouterData<F>) -> Result<Self, Self::Error> {
        Ok(Self {
            reference: item.request.connector_transaction_id.clone(),
            value: Box::new(PaymentValue {
                amount: item.request.amount,
                currency: item.request.currency.to_string(),
            }),
        })
    }
}

#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct WorldpayErrorResponse {
    pub error_name: String,
    pub message: String,
}
