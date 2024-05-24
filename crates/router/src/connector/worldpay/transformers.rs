use base64::Engine;
use common_utils::errors::CustomResult;
use diesel_models::enums;
use masking::{PeekInterface, Secret};
use serde::Serialize;

use super::{requests::*, response::*};
use crate::{
    connector::utils,
    consts,
    core::errors,
    types::{
        self, domain, transformers::ForeignTryFrom, PaymentsAuthorizeData, PaymentsResponseData,
    },
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
    payment_method: domain::PaymentMethodData,
) -> CustomResult<PaymentInstrument, errors::ConnectorError> {
    match payment_method {
        domain::PaymentMethodData::Card(card) => Ok(PaymentInstrument::Card(CardPayment {
            card_expiry_date: CardExpiryDate {
                month: utils::CardData::get_expiry_month_as_i8(&card)?,
                year: utils::CardData::get_expiry_year_as_i32(&card)?,
            },
            card_number: card.card_number,
            ..CardPayment::default()
        })),
        domain::PaymentMethodData::Wallet(wallet) => match wallet {
            domain::WalletData::GooglePay(data) => {
                Ok(PaymentInstrument::Googlepay(WalletPayment {
                    payment_type: PaymentType::Googlepay,
                    wallet_token: Secret::new(data.tokenization_data.token),
                    ..WalletPayment::default()
                }))
            }
            domain::WalletData::ApplePay(data) => Ok(PaymentInstrument::Applepay(WalletPayment {
                payment_type: PaymentType::Applepay,
                wallet_token: Secret::new(data.payment_data),
                ..WalletPayment::default()
            })),
            domain::WalletData::AliPayQr(_)
            | domain::WalletData::AliPayRedirect(_)
            | domain::WalletData::AliPayHkRedirect(_)
            | domain::WalletData::MomoRedirect(_)
            | domain::WalletData::KakaoPayRedirect(_)
            | domain::WalletData::GoPayRedirect(_)
            | domain::WalletData::GcashRedirect(_)
            | domain::WalletData::ApplePayRedirect(_)
            | domain::WalletData::ApplePayThirdPartySdk(_)
            | domain::WalletData::DanaRedirect {}
            | domain::WalletData::GooglePayRedirect(_)
            | domain::WalletData::GooglePayThirdPartySdk(_)
            | domain::WalletData::MbWayRedirect(_)
            | domain::WalletData::MobilePayRedirect(_)
            | domain::WalletData::PaypalRedirect(_)
            | domain::WalletData::PaypalSdk(_)
            | domain::WalletData::SamsungPay(_)
            | domain::WalletData::TwintRedirect {}
            | domain::WalletData::VippsRedirect {}
            | domain::WalletData::TouchNGoRedirect(_)
            | domain::WalletData::WeChatPayRedirect(_)
            | domain::WalletData::CashappQr(_)
            | domain::WalletData::SwishQr(_)
            | domain::WalletData::WeChatPayQr(_) => Err(errors::ConnectorError::NotImplemented(
                utils::get_unimplemented_payment_method_error_message("worldpay"),
            )
            .into()),
        },
        domain::PaymentMethodData::PayLater(_)
        | domain::PaymentMethodData::BankRedirect(_)
        | domain::PaymentMethodData::BankDebit(_)
        | domain::PaymentMethodData::BankTransfer(_)
        | domain::PaymentMethodData::Crypto(_)
        | domain::PaymentMethodData::MandatePayment
        | domain::PaymentMethodData::Reward
        | domain::PaymentMethodData::Upi(_)
        | domain::PaymentMethodData::Voucher(_)
        | domain::PaymentMethodData::CardRedirect(_)
        | domain::PaymentMethodData::GiftCard(_)
        | domain::PaymentMethodData::CardToken(_) => Err(errors::ConnectorError::NotImplemented(
            utils::get_unimplemented_payment_method_error_message("worldpay"),
        )
        .into()),
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
            response: Ok(PaymentsResponseData::TransactionResponse {
                resource_id: types::ResponseId::foreign_try_from(item.response.links)?,
                redirection_data: None,
                mandate_reference: None,
                connector_metadata: None,
                network_txn_id: None,
                connector_response_reference_id: None,
                incremental_authorization_allowed: None,
                charge_id: None,
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
