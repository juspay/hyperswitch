use base64::Engine;
use common_utils::errors::CustomResult;
use diesel_models::enums;
use error_stack::{IntoReport, ResultExt};
use masking::{PeekInterface, Secret};
use serde::Serialize;

use super::{requests::*, response::*};
use crate::{
    connector::utils,
    consts,
    core::errors,
    types::{self, api, PaymentsAuthorizeData, PaymentsResponseData},
};

#[derive(Debug, Serialize)]
pub struct WorldpayRouterData<T> {
    amount: i64,
    router_data: T,
}
impl<T>
    TryFrom<(
        &types::api::CurrencyUnit,
        types::storage::enums::Currency,
        i64,
        T,
    )> for WorldpayRouterData<T>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        (_currency_unit, _currency, amount, item): (
            &types::api::CurrencyUnit,
            types::storage::enums::Currency,
            i64,
            T,
        ),
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            amount,
            router_data: item,
        })
    }
}
fn fetch_payment_instrument(
    payment_method: api::PaymentMethodData,
) -> CustomResult<PaymentInstrument, errors::ConnectorError> {
    match payment_method {
        api::PaymentMethodData::Card(card) => Ok(PaymentInstrument::Card(CardPayment {
            card_expiry_date: CardExpiryDate {
                month: card
                    .card_exp_month
                    .peek()
                    .clone()
                    .parse::<i8>()
                    .into_report()
                    .change_context(errors::ConnectorError::ResponseDeserializationFailed)?,
                year: card
                    .card_exp_year
                    .peek()
                    .clone()
                    .parse::<i32>()
                    .into_report()
                    .change_context(errors::ConnectorError::ResponseDeserializationFailed)?,
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
                    wallet_token: data.payment_data,
                    ..WalletPayment::default()
                }))
            }
            api_models::payments::WalletData::AliPayQr(_)
            | api_models::payments::WalletData::AliPayRedirect(_)
            | api_models::payments::WalletData::AliPayHkRedirect(_)
            | api_models::payments::WalletData::MomoRedirect(_)
            | api_models::payments::WalletData::KakaoPayRedirect(_)
            | api_models::payments::WalletData::GoPayRedirect(_)
            | api_models::payments::WalletData::GcashRedirect(_)
            | api_models::payments::WalletData::ApplePayRedirect(_)
            | api_models::payments::WalletData::ApplePayThirdPartySdk(_)
            | api_models::payments::WalletData::DanaRedirect {}
            | api_models::payments::WalletData::GooglePayRedirect(_)
            | api_models::payments::WalletData::GooglePayThirdPartySdk(_)
            | api_models::payments::WalletData::MbWayRedirect(_)
            | api_models::payments::WalletData::MobilePayRedirect(_)
            | api_models::payments::WalletData::PaypalRedirect(_)
            | api_models::payments::WalletData::PaypalSdk(_)
            | api_models::payments::WalletData::SamsungPay(_)
            | api_models::payments::WalletData::TwintRedirect {}
            | api_models::payments::WalletData::VippsRedirect {}
            | api_models::payments::WalletData::TouchNGoRedirect(_)
            | api_models::payments::WalletData::WeChatPayRedirect(_)
            | api_models::payments::WalletData::CashappQr(_)
            | api_models::payments::WalletData::SwishQr(_)
            | api_models::payments::WalletData::WeChatPayQr(_) => {
                Err(errors::ConnectorError::NotImplemented(
                    utils::get_unimplemented_payment_method_error_message("worldpay"),
                )
                .into())
            }
        },
        api_models::payments::PaymentMethodData::PayLater(_)
        | api_models::payments::PaymentMethodData::BankRedirect(_)
        | api_models::payments::PaymentMethodData::BankDebit(_)
        | api_models::payments::PaymentMethodData::BankTransfer(_)
        | api_models::payments::PaymentMethodData::Crypto(_)
        | api_models::payments::PaymentMethodData::MandatePayment
        | api_models::payments::PaymentMethodData::Reward
        | api_models::payments::PaymentMethodData::Upi(_)
        | api_models::payments::PaymentMethodData::Voucher(_)
        | api_models::payments::PaymentMethodData::CardRedirect(_)
        | api_models::payments::PaymentMethodData::GiftCard(_) => {
            Err(errors::ConnectorError::NotImplemented(
                utils::get_unimplemented_payment_method_error_message("worldpay"),
            )
            .into())
        }
    }
}

impl
    TryFrom<
        &WorldpayRouterData<
            &types::RouterData<
                types::api::payments::Authorize,
                PaymentsAuthorizeData,
                PaymentsResponseData,
            >,
        >,
    > for WorldpayPaymentsRequest
{
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(
        item: &WorldpayRouterData<
            &types::RouterData<
                types::api::payments::Authorize,
                PaymentsAuthorizeData,
                PaymentsResponseData,
            >,
        >,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            instruction: Instruction {
                value: PaymentValue {
                    amount: item.amount,
                    currency: item.router_data.request.currency.to_string(),
                },
                narrative: InstructionNarrative {
                    line1: item.router_data.merchant_id.clone().replace('_', "-"),
                    ..Default::default()
                },
                payment_instrument: fetch_payment_instrument(
                    item.router_data.request.payment_method_data.clone(),
                )?,
                debt_repayment: None,
            },
            merchant: Merchant {
                entity: item
                    .router_data
                    .connector_request_reference_id
                    .clone()
                    .replace('_', "-"),
                ..Default::default()
            },
            transaction_reference: item.router_data.connector_request_reference_id.clone(),
            channel: None,
            customer: None,
        })
    }
}

pub struct WorldpayAuthType {
    pub(super) api_key: Secret<String>,
}

impl TryFrom<&types::ConnectorAuthType> for WorldpayAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &types::ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            types::ConnectorAuthType::BodyKey { api_key, key1 } => {
                let auth_key = format!("{}:{}", key1.peek(), api_key.peek());
                let auth_header = format!("Basic {}", consts::BASE64_ENGINE.encode(auth_key));
                Ok(Self {
                    api_key: Secret::new(auth_header),
                })
            }
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
            EventType::Charged | EventType::SentForSettlement => Self::Charged,
            EventType::Cancelled
            | EventType::SentForRefund
            | EventType::RefundFailed
            | EventType::Refunded
            | EventType::Error
            | EventType::Expired
            | EventType::Unknown => Self::Pending,
        }
    }
}

impl From<EventType> for enums::RefundStatus {
    fn from(value: EventType) -> Self {
        match value {
            EventType::Refunded => Self::Success,
            EventType::RefundFailed => Self::Failure,
            EventType::Authorized
            | EventType::Cancelled
            | EventType::Charged
            | EventType::SentForRefund
            | EventType::Refused
            | EventType::Error
            | EventType::SentForSettlement
            | EventType::Expired
            | EventType::CaptureFailed
            | EventType::Unknown => Self::Pending,
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
                network_txn_id: None,
                connector_response_reference_id: None,
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
                amount: item.request.refund_amount,
                currency: item.request.currency.to_string(),
            },
        })
    }
}

impl TryFrom<WorldpayWebhookEventType> for WorldpayEventResponse {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(event: WorldpayWebhookEventType) -> Result<Self, Self::Error> {
        Ok(Self {
            last_event: event.event_details.event_type,
            links: None,
        })
    }
}
