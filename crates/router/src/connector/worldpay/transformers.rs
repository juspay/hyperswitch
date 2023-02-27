use base64::Engine;
use common_utils::errors::CustomResult;
use error_stack::ResultExt;
use storage_models::enums;

use super::{requests::*, response::*};
use crate::{
    consts,
    core::errors,
    types::{self, api},
};

fn fetch_payment_instrument(
    payment_method: api::PaymentMethodData,
) -> CustomResult<PaymentInstrument, errors::ConnectorError> {
    match payment_method {
        api::PaymentMethodData::Card(card) => Ok(PaymentInstrument::Card(CardPayment {
            card_expiry_date: CardExpiryDate {
                month: card.card_exp_month,
                year: card.card_exp_year,
            },
            card_number: card.card_number,
            ..CardPayment::default()
        })),
        api::PaymentMethodData::Wallet(wallet) => match wallet {
            api_models::payments::WalletData::GooglePay(data) => {
                Ok(PaymentInstrument::Googlepay(WalletPayment {
                    payment_type: PaymentType::Googlepay,
                    wallet_token: data.tokenization_data.token,
                    ..WalletPayment::default()
                }))
            }
            api_models::payments::WalletData::ApplePay(data) => {
                Ok(PaymentInstrument::Applepay(WalletPayment {
                    payment_type: PaymentType::Applepay,

                    wallet_token:
                        consts::BASE64_ENGINE.encode(
                            common_utils::ext_traits::Encode::<
                                api_models::payments::ApplepayPaymentData,
                            >::encode_to_string_of_json(
                                &data.payment_data
                            )
                            .change_context(errors::ConnectorError::RequestEncodingFailed)?,
                        ),
                    ..WalletPayment::default()
                }))
            }
            _ => Err(errors::ConnectorError::NotImplemented("Wallet Type".to_string()).into()),
        },
        _ => {
            Err(errors::ConnectorError::NotImplemented("Current Payment Method".to_string()).into())
        }
    }
}

impl TryFrom<&types::PaymentsAuthorizeRouterData> for WorldpayPaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsAuthorizeRouterData) -> Result<Self, Self::Error> {
        Ok(Self {
            instruction: Instruction {
                value: PaymentValue {
                    amount: item.request.amount,
                    currency: item.request.currency.to_string(),
                },
                narrative: InstructionNarrative {
                    line1: item.merchant_id.clone(),
                    ..Default::default()
                },
                payment_instrument: fetch_payment_instrument(
                    item.request.payment_method_data.clone(),
                )?,
                debt_repayment: None,
            },
            merchant: Merchant {
                entity: item.payment_id.clone(),
                ..Default::default()
            },
            transaction_reference: item.attempt_id.clone(),
            channel: None,
            customer: None,
        })
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

impl From<EventType> for enums::AttemptStatus {
    fn from(value: EventType) -> Self {
        match value {
            EventType::Authorized => Self::Authorized,
            EventType::CaptureFailed => Self::CaptureFailed,
            EventType::Refused => Self::Failure,
            EventType::Charged => Self::Charged,
            _ => Self::Pending,
        }
    }
}

impl From<EventType> for enums::RefundStatus {
    fn from(value: EventType) -> Self {
        match value {
            EventType::Refunded => Self::Success,
            EventType::RefundFailed => Self::Failure,
            _ => Self::Pending,
        }
    }
}

impl TryFrom<types::PaymentsResponseRouterData<WorldpayPaymentsResponse>>
    for types::PaymentsAuthorizeRouterData
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::PaymentsResponseRouterData<WorldpayPaymentsResponse>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            status: match item.response.outcome {
                Some(outcome) => enums::AttemptStatus::from(outcome),
                None => Err(errors::ConnectorError::MissingRequiredField {
                    field_name: "outcome",
                })?,
            },
            description: item.response.description,
            response: Ok(types::PaymentsResponseData::TransactionResponse {
                resource_id: types::ResponseId::try_from(item.response.links)?,
                redirection_data: None,
                mandate_reference: None,
                connector_metadata: None,
            }),
            ..item.data
        })
    }
}

impl<F> TryFrom<&types::RefundsRouterData<F>> for WorldpayRefundRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::RefundsRouterData<F>) -> Result<Self, Self::Error> {
        Ok(Self {
            reference: item.request.connector_transaction_id.clone(),
            value: PaymentValue {
                amount: item.request.amount,
                currency: item.request.currency.to_string(),
            },
        })
    }
}
