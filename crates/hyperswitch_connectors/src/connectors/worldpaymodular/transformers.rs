pub mod request;
pub mod response;
use api_models::payments::{MandateIds, MandateReferenceId};
use base64::Engine;
use common_enums::{enums, Currency, PaymentChannel};
use common_utils::{
    consts::BASE64_ENGINE, errors::CustomResult, ext_traits::OptionExt, types::MinorUnit,
};
use error_stack::ResultExt;
use hyperswitch_domain_models::{
    address::Address,
    payment_method_data::{PaymentMethodData, WalletData},
    router_data::{ConnectorAuthType, RouterData},
    router_flow_types::*,
    router_request_types::*,
    router_response_types::*,
    types::*,
};
use hyperswitch_interfaces::{api::*, errors::ConnectorError};
use masking::{PeekInterface, Secret};
pub use request::*;
pub use response::*;
use serde::Serialize;

use crate::{
    types::PaymentsResponseRouterData,
    utils::{
        get_unimplemented_payment_method_error_message, ApplePay,
        PaymentsAuthorizeRequestData as _, RouterData as _,
    },
};

#[derive(Debug, Serialize)]
pub struct WorldpaymodularRouterData<T> {
    amount: MinorUnit,
    router_data: T,
}
impl<T> TryFrom<(&CurrencyUnit, Currency, MinorUnit, T)> for WorldpaymodularRouterData<T> {
    type Error = error_stack::Report<ConnectorError>;
    fn try_from(
        (_currency_unit, _currency, minor_amount, item): (&CurrencyUnit, Currency, MinorUnit, T),
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            amount: minor_amount,
            router_data: item,
        })
    }
}
fn fetch_payment_instrument(
    payment_method: PaymentMethodData,
    billing_address: Option<&Address>,
    connector_mandate_id: Option<MandateIds>,
    base_url: &str,
) -> CustomResult<PaymentInstrument, ConnectorError> {
    let billing_address =
        if let Some(address) = billing_address.and_then(|addr| addr.address.clone()) {
            Some(BillingAddress {
                address1: address.line1,
                address2: address.line2,
                address3: address.line3,
                city: address.city,
                state: address.state,
                postal_code: address
                    .zip
                    .get_required_value("zip")
                    .change_context(ConnectorError::MissingRequiredField { field_name: "zip" })?,
                country_code: address
                    .country
                    .get_required_value("country_code")
                    .change_context(ConnectorError::MissingRequiredField {
                        field_name: "country_code",
                    })?,
            })
        } else {
            None
        };
    match payment_method {
        PaymentMethodData::MandatePayment => {
            let mandate_id = connector_mandate_id
                .as_ref()
                .and_then(|mandate_ids| {
                    mandate_ids
                        .mandate_reference_id
                        .as_ref()
                        .and_then(|mandate_ref_id| match mandate_ref_id {
                            MandateReferenceId::ConnectorMandateId(id) => {
                                id.get_connector_mandate_id()
                            }
                            _ => None,
                        })
                })
                .ok_or(ConnectorError::MissingConnectorMandateID)?;
            Ok(PaymentInstrument::CardToken(CardToken {
                payment_type: PaymentType::CardToken,
                href: format!("{base_url}tokens/{mandate_id}"),
            }))
        }
        PaymentMethodData::Wallet(WalletData::GooglePay(data)) => {
            Ok(PaymentInstrument::Googlepay(WalletPayment {
                payment_type: PaymentType::Googlepay,
                wallet_token: data
                    .tokenization_data
                    .get_encrypted_google_pay_token()
                    .change_context(ConnectorError::MissingRequiredField {
                        field_name: "gpay wallet_token",
                    })?
                    .into(),
                billing_address,
            }))
        }
        PaymentMethodData::Wallet(WalletData::ApplePay(data)) => {
            Ok(PaymentInstrument::Applepay(WalletPayment {
                payment_type: PaymentType::Applepay,
                wallet_token: data.get_applepay_decoded_payment_data()?,
                billing_address,
            }))
        }
        _ => Err(
            ConnectorError::NotImplemented(get_unimplemented_payment_method_error_message(
                "worldpaymodular",
            ))
            .into(),
        ),
    }
}

impl
    TryFrom<(
        &WorldpaymodularRouterData<
            &RouterData<Authorize, PaymentsAuthorizeData, PaymentsResponseData>,
        >,
        &Secret<String>,
        &str,
    )> for WorldpaymodularPaymentsRequest
{
    type Error = error_stack::Report<ConnectorError>;

    fn try_from(
        req: (
            &WorldpaymodularRouterData<
                &RouterData<Authorize, PaymentsAuthorizeData, PaymentsResponseData>,
            >,
            &Secret<String>,
            &str,
        ),
    ) -> Result<Self, Self::Error> {
        let (item, entity_id, base_url) = req;

        let customer_agreement = if item.router_data.request.is_cit_mandate_payment() {
            Some(WMCustomerAcceptance::CardOnFile(WMCustomerAgreement {
                stored_card_usage: WMStoredCardUsage::First,
            }))
        } else if item.router_data.request.is_mit_payment() {
            Some(WMCustomerAcceptance::Subscription)
        } else {
            None
        };
        let channel = if item.router_data.request.is_mit_payment() {
            None
        } else {
            match item.router_data.request.payment_channel {
                Some(PaymentChannel::MailOrder) => Some(Channel::Moto),
                _ => Some(Channel::Ecom),
            }
        };
        Ok(Self {
            instruction: Instruction {
                request_auto_settlement: RequestAutoSettlement {
                    enabled: item.router_data.request.capture_method
                        == Some(enums::CaptureMethod::Automatic),
                },
                value: PaymentValue {
                    amount: item.amount,
                    currency: item.router_data.request.currency,
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
                    item.router_data.get_optional_billing(),
                    item.router_data.request.mandate_id.clone(),
                    base_url,
                )?,
                debt_repayment: None,
                customer_agreement,
            },
            merchant: Merchant {
                entity: entity_id.clone(),
                ..Default::default()
            },
            transaction_reference: item.router_data.connector_request_reference_id.clone(),
            channel: channel,
            customer: None,
        })
    }
}

pub struct WorldpaymodularAuthType {
    pub(super) api_key: Secret<String>,
    pub(super) entity_id: Secret<String>,
}

impl TryFrom<&ConnectorAuthType> for WorldpaymodularAuthType {
    type Error = error_stack::Report<ConnectorError>;
    fn try_from(auth_type: &ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            ConnectorAuthType::SignatureKey {
                api_key,
                key1,
                api_secret,
            } => {
                let auth_key = format!("{}:{}", key1.peek(), api_key.peek());
                let auth_header = format!("Basic {}", BASE64_ENGINE.encode(auth_key));
                Ok(Self {
                    api_key: Secret::new(auth_header),
                    entity_id: api_secret.clone(),
                })
            }
            _ => Err(ConnectorError::FailedToObtainAuthType)?,
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

impl From<&EventType> for enums::AttemptStatus {
    fn from(value: &EventType) -> Self {
        match value {
            EventType::SentForAuthorization => Self::Authorizing,
            EventType::SentForSettlement => Self::CaptureInitiated,
            EventType::Settled => Self::Charged,
            EventType::Authorized => Self::Authorized,
            EventType::SettlementRejected | EventType::Refused | EventType::SettlementFailed => {
                Self::Failure
            }
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
            EventType::SentForRefund => Self::Pending,
            EventType::Refunded => Self::Success,
            EventType::RefundFailed => Self::Failure,
            EventType::Authorized
            | EventType::Cancelled
            | EventType::Settled
            | EventType::Refused
            | EventType::Error
            | EventType::SentForSettlement
            | EventType::SentForAuthorization
            | EventType::SettlementFailed
            | EventType::Expired
            | EventType::SettlementRejected
            | EventType::Unknown => Self::Pending,
        }
    }
}

impl TryFrom<PaymentsResponseRouterData<WorldpaymodularPaymentsResponse>>
    for PaymentsAuthorizeRouterData
{
    type Error = error_stack::Report<ConnectorError>;
    fn try_from(
        item: PaymentsResponseRouterData<WorldpaymodularPaymentsResponse>,
    ) -> Result<Self, Self::Error> {
        let mandate = item.response.links.get_mandate_id();
        Ok(Self {
            status: enums::AttemptStatus::from(item.response.outcome),
            description: item.response.description,
            response: Ok(PaymentsResponseData::TransactionResponse {
                resource_id: item.response.links.get_resource_id()?,
                redirection_data: Box::new(None),
                mandate_reference: Box::new(mandate),
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

impl TryFrom<(&PaymentsCaptureRouterData, MinorUnit)> for WorldpaymodularPartialRequest {
    type Error = error_stack::Report<ConnectorError>;
    fn try_from(req: (&PaymentsCaptureRouterData, MinorUnit)) -> Result<Self, Self::Error> {
        let (item, amount) = req;
        Ok(Self {
            reference: item.payment_id.clone().replace("_", "-"),
            value: PaymentValue {
                amount: amount,
                currency: item.request.currency,
            },
        })
    }
}

impl<F> TryFrom<(&RefundsRouterData<F>, MinorUnit)> for WorldpaymodularPartialRequest {
    type Error = error_stack::Report<ConnectorError>;
    fn try_from(req: (&RefundsRouterData<F>, MinorUnit)) -> Result<Self, Self::Error> {
        let (item, amount) = req;
        Ok(Self {
            reference: item.request.refund_id.clone().replace("_", "-"),
            value: PaymentValue {
                amount: amount,
                currency: item.request.currency,
            },
        })
    }
}

impl TryFrom<WorldpaymodularWebhookEventType> for WorldpaymodularEventResponse {
    type Error = error_stack::Report<ConnectorError>;
    fn try_from(event: WorldpaymodularWebhookEventType) -> Result<Self, Self::Error> {
        Ok(Self {
            last_event: event.event_details.event_type,
            links: None,
        })
    }
}
