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
            expiry_date: ExpiryDate {
                month: utils::CardData::get_expiry_month_as_i8(&card)?,
                year: utils::CardData::get_expiry_year_as_i32(&card)?,
            },
            card_number: card.card_number,
            cvc: card.card_cvc,
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
            | domain::WalletData::WeChatPayQr(_)
            | domain::WalletData::Mifinity(_) => Err(errors::ConnectorError::NotImplemented(
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
        | domain::PaymentMethodData::RealTimePayment(_)
        | domain::PaymentMethodData::Upi(_)
        | domain::PaymentMethodData::Voucher(_)
        | domain::PaymentMethodData::CardRedirect(_)
        | domain::PaymentMethodData::GiftCard(_)
        | domain::PaymentMethodData::OpenBanking(_)
        | domain::PaymentMethodData::CardToken(_)
        | domain::PaymentMethodData::NetworkToken(_) => {
            Err(errors::ConnectorError::NotImplemented(
                utils::get_unimplemented_payment_method_error_message("worldpay"),
            )
            .into())
        }
    }
}

impl
    TryFrom<(
        &WorldpayRouterData<
            &types::RouterData<
                types::api::payments::Authorize,
                PaymentsAuthorizeData,
                PaymentsResponseData,
            >,
        >,
        &Secret<String>,
    )> for WorldpayPaymentsRequest
{
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(
        req: (
            &WorldpayRouterData<
                &types::RouterData<
                    types::api::payments::Authorize,
                    PaymentsAuthorizeData,
                    PaymentsResponseData,
                >,
            >,
            &Secret<String>,
        ),
    ) -> Result<Self, Self::Error> {
        let (item, entity_id) = req;
        Ok(Self {
            instruction: Instruction {
                request_auto_settlement: RequestAutoSettlement {
                    enabled: item.router_data.request.capture_method
                        == Some(enums::CaptureMethod::Automatic),
                },
                value: PaymentValue {
                    amount: item.amount,
                    currency: item.router_data.request.currency.to_string(),
                },
                narrative: InstructionNarrative {
                    line1: item
                        .router_data
                        .merchant_id
                        .get_string_repr()
                        .replace('_', "-"),
                    ..Default::default()
                },
                payment_instrument: fetch_payment_instrument(
                    item.router_data.request.payment_method_data.clone(),
                )?,
                debt_repayment: None,
            },
            merchant: Merchant {
                entity: entity_id.clone(),
                ..Default::default()
            },
            transaction_reference: item.router_data.connector_request_reference_id.clone(),
            channel: Channel::Ecom,
            customer: None,
        })
    }
}

pub struct WorldpayAuthType {
    pub(super) api_key: Secret<String>,
    pub(super) entity_id: Secret<String>,
}

impl TryFrom<&types::ConnectorAuthType> for WorldpayAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &types::ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            // TODO: Remove this later, kept purely for backwards compatibility
            types::ConnectorAuthType::BodyKey { api_key, key1 } => {
                let auth_key = format!("{}:{}", key1.peek(), api_key.peek());
                let auth_header = format!("Basic {}", consts::BASE64_ENGINE.encode(auth_key));
                Ok(Self {
                    api_key: Secret::new(auth_header),
                    entity_id: Secret::new("default".to_string()),
                })
            }
            types::ConnectorAuthType::SignatureKey {
                api_key,
                key1,
                api_secret,
            } => {
                let auth_key = format!("{}:{}", key1.peek(), api_key.peek());
                let auth_header = format!("Basic {}", consts::BASE64_ENGINE.encode(auth_key));
                Ok(Self {
                    api_key: Secret::new(auth_header),
                    entity_id: api_secret.clone(),
                })
            }
            _ => Err(errors::ConnectorError::FailedToObtainAuthType)?,
        }
    }
}

impl From<PaymentOutcome> for enums::AttemptStatus {
    fn from(item: PaymentOutcome) -> Self {
        match item {
            PaymentOutcome::Authorized => Self::Authorized,
            PaymentOutcome::Refused => Self::Failure,
            PaymentOutcome::SentForSettlement => Self::CaptureInitiated,
            PaymentOutcome::SentForRefund => Self::AutoRefunded,
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

impl TryFrom<&types::PaymentsCaptureRouterData> for WorldpayPartialRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsCaptureRouterData) -> Result<Self, Self::Error> {
        Ok(Self {
            reference: item.payment_id.clone().replace("_", "-"),
            value: PaymentValue {
                amount: item.request.amount_to_capture,
                currency: item.request.currency.to_string(),
            },
        })
    }
}

impl<F> TryFrom<&types::RefundsRouterData<F>> for WorldpayPartialRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::RefundsRouterData<F>) -> Result<Self, Self::Error> {
        Ok(Self {
            reference: item.request.refund_id.clone().replace("_", "-"),
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
