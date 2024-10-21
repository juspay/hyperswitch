use api_models::payments::Address;
use base64::Engine;
use common_utils::{errors::CustomResult, ext_traits::OptionExt, pii, types::MinorUnit};
use diesel_models::enums;
use error_stack::ResultExt;
use hyperswitch_connectors::utils::RouterData;
use masking::{ExposeInterface, PeekInterface, Secret};
use serde::{Deserialize, Serialize};

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
        MinorUnit,
        T,
    )> for WorldpayRouterData<T>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        (_currency_unit, _currency, minor_amount, item): (
            &types::api::CurrencyUnit,
            types::storage::enums::Currency,
            MinorUnit,
            T,
        ),
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            amount: minor_amount.get_amount_as_i64(),
            router_data: item,
        })
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct WorldpayConnectorMetadataObject {
    pub merchant_name: Option<Secret<String>>,
}

impl TryFrom<&Option<pii::SecretSerdeValue>> for WorldpayConnectorMetadataObject {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(meta_data: &Option<pii::SecretSerdeValue>) -> Result<Self, Self::Error> {
        let metadata: Self = utils::to_connector_meta_from_secret::<Self>(meta_data.clone())
            .change_context(errors::ConnectorError::InvalidConnectorConfig {
                config: "metadata",
            })?;
        Ok(metadata)
    }
}

fn fetch_payment_instrument(
    payment_method: domain::PaymentMethodData,
    billing_address: Option<&Address>,
    auth_type: enums::AuthenticationType,
) -> CustomResult<PaymentInstrument, errors::ConnectorError> {
    match payment_method {
        domain::PaymentMethodData::Card(card) => {
            if auth_type == enums::AuthenticationType::ThreeDs {
                return Err(errors::ConnectorError::NotImplemented(
                    "ThreeDS flow through worldpay".to_string(),
                )
                .into());
            }
            Ok(PaymentInstrument::Card(CardPayment {
                payment_type: PaymentType::Plain,
                expiry_date: ExpiryDate {
                    month: utils::CardData::get_expiry_month_as_i8(&card)?,
                    year: utils::CardData::get_expiry_year_as_i32(&card)?,
                },
                card_number: card.card_number,
                cvc: card.card_cvc,
                card_holder_name: card.nick_name,
                billing_address: if let Some(address) =
                    billing_address.and_then(|addr| addr.address.clone())
                {
                    Some(BillingAddress {
                        address1: address.line1,
                        address2: address.line2,
                        address3: address.line3,
                        city: address.city,
                        state: address.state,
                        postal_code: address.zip.get_required_value("zip").change_context(
                            errors::ConnectorError::MissingRequiredField { field_name: "zip" },
                        )?,
                        country_code: address
                            .country
                            .get_required_value("country_code")
                            .change_context(errors::ConnectorError::MissingRequiredField {
                                field_name: "country_code",
                            })?,
                    })
                } else {
                    None
                },
            }))
        }
        domain::PaymentMethodData::Wallet(wallet) => match wallet {
            domain::WalletData::GooglePay(data) => {
                Ok(PaymentInstrument::Googlepay(WalletPayment {
                    payment_type: PaymentType::Encrypted,
                    wallet_token: Secret::new(data.tokenization_data.token),
                    ..WalletPayment::default()
                }))
            }
            domain::WalletData::ApplePay(data) => Ok(PaymentInstrument::Applepay(WalletPayment {
                payment_type: PaymentType::Encrypted,
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
            | domain::WalletData::Paze(_)
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
        | domain::PaymentMethodData::NetworkToken(_)
        | domain::PaymentMethodData::CardDetailsForNetworkTransactionId(_) => {
            Err(errors::ConnectorError::NotImplemented(
                utils::get_unimplemented_payment_method_error_message("worldpay"),
            )
            .into())
        }
    }
}

impl TryFrom<(enums::PaymentMethod, enums::PaymentMethodType)> for PaymentMethod {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        src: (enums::PaymentMethod, enums::PaymentMethodType),
    ) -> Result<Self, Self::Error> {
        match (src.0, src.1) {
            (enums::PaymentMethod::Card, _) => Ok(Self::Card),
            (enums::PaymentMethod::Wallet, enums::PaymentMethodType::ApplePay) => {
                Ok(Self::ApplePay)
            }
            (enums::PaymentMethod::Wallet, enums::PaymentMethodType::GooglePay) => {
                Ok(Self::GooglePay)
            }
            _ => Err(errors::ConnectorError::NotImplemented(
                utils::get_unimplemented_payment_method_error_message("worldpay"),
            )
            .into()),
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
        let worldpay_connector_metadata_object: WorldpayConnectorMetadataObject =
            WorldpayConnectorMetadataObject::try_from(&item.router_data.connector_meta_data)?;
        let merchant_name = worldpay_connector_metadata_object.merchant_name.ok_or(
            errors::ConnectorError::InvalidConnectorConfig {
                config: "metadata.merchant_name",
            },
        )?;
        Ok(Self {
            instruction: Instruction {
                settlement: item
                    .router_data
                    .request
                    .capture_method
                    .map(|capture_method| AutoSettlement {
                        auto: capture_method == enums::CaptureMethod::Automatic,
                    }),
                method: item
                    .router_data
                    .request
                    .payment_method_type
                    .map(|pmt| PaymentMethod::try_from((item.router_data.payment_method, pmt)))
                    .transpose()?
                    .get_required_value("payment_method")
                    .change_context(errors::ConnectorError::MissingRequiredField {
                        field_name: "payment_method",
                    })?,
                payment_instrument: fetch_payment_instrument(
                    item.router_data.request.payment_method_data.clone(),
                    item.router_data.get_optional_billing(),
                    item.router_data.auth_type,
                )?,
                narrative: InstructionNarrative {
                    line1: merchant_name.expose(),
                },
                value: PaymentValue {
                    amount: item.amount,
                    currency: item.router_data.request.currency,
                },
                debt_repayment: None,
            },
            merchant: Merchant {
                entity: entity_id.clone(),
                ..Default::default()
            },
            transaction_reference: item.router_data.connector_request_reference_id.clone(),
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
            PaymentOutcome::SentForSettlement => Self::CaptureInitiated,
            PaymentOutcome::ThreeDsDeviceDataRequired => Self::DeviceDataCollectionPending,
            PaymentOutcome::ThreeDsAuthenticationFailed => Self::AuthenticationFailed,
            PaymentOutcome::ThreeDsChallenged => Self::AuthenticationPending,
            PaymentOutcome::SentForCancellation => Self::VoidInitiated,
            PaymentOutcome::SentForPartialRefund | PaymentOutcome::SentForRefund => {
                Self::AutoRefunded
            }
            PaymentOutcome::Refused | PaymentOutcome::FraudHighRisk => Self::Failure,
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
            EventType::Refused | EventType::SettlementFailed => Self::Failure,
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
            EventType::Refunded | EventType::SentForRefund => Self::Success,
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
            | EventType::Unknown => Self::Pending,
        }
    }
}

impl
    ForeignTryFrom<(
        types::PaymentsResponseRouterData<WorldpayPaymentsResponse>,
        Option<String>,
    )> for types::PaymentsAuthorizeRouterData
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn foreign_try_from(
        item: (
            types::PaymentsResponseRouterData<WorldpayPaymentsResponse>,
            Option<String>,
        ),
    ) -> Result<Self, Self::Error> {
        let (router_data, optional_correlation_id) = item;
        let description = match router_data.response.other_fields {
            WorldpayPaymentResponseFields::AuthorizedResponse(ref res) => res.description.clone(),
            WorldpayPaymentResponseFields::DDCResponse(_)
            | WorldpayPaymentResponseFields::FraudHighRisk(_)
            | WorldpayPaymentResponseFields::RefusedResponse(_) => None,
        };
        Ok(Self {
            status: enums::AttemptStatus::from(router_data.response.outcome.clone()),
            description,
            response: Ok(PaymentsResponseData::TransactionResponse {
                resource_id: types::ResponseId::foreign_try_from((
                    router_data.response,
                    optional_correlation_id.clone(),
                ))?,
                redirection_data: None,
                mandate_reference: None,
                connector_metadata: None,
                network_txn_id: None,
                connector_response_reference_id: optional_correlation_id,
                incremental_authorization_allowed: None,
                charge_id: None,
            }),
            ..router_data.data
        })
    }
}

impl TryFrom<(&types::PaymentsCaptureRouterData, MinorUnit)> for WorldpayPartialRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(req: (&types::PaymentsCaptureRouterData, MinorUnit)) -> Result<Self, Self::Error> {
        let (item, amount) = req;
        Ok(Self {
            reference: item.payment_id.clone().replace("_", "-"),
            value: PaymentValue {
                amount: amount.get_amount_as_i64(),
                currency: item.request.currency,
            },
        })
    }
}

impl<F> TryFrom<(&types::RefundsRouterData<F>, MinorUnit)> for WorldpayPartialRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(req: (&types::RefundsRouterData<F>, MinorUnit)) -> Result<Self, Self::Error> {
        let (item, amount) = req;
        Ok(Self {
            reference: item.request.refund_id.clone().replace("_", "-"),
            value: PaymentValue {
                amount: amount.get_amount_as_i64(),
                currency: item.request.currency,
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

impl ForeignTryFrom<(WorldpayPaymentsResponse, Option<String>)> for ResponseIdStr {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn foreign_try_from(
        item: (WorldpayPaymentsResponse, Option<String>),
    ) -> Result<Self, Self::Error> {
        get_resource_id(item.0, item.1, |id| Self { id })
    }
}

impl ForeignTryFrom<(WorldpayPaymentsResponse, Option<String>)> for types::ResponseId {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn foreign_try_from(
        item: (WorldpayPaymentsResponse, Option<String>),
    ) -> Result<Self, Self::Error> {
        get_resource_id(item.0, item.1, Self::ConnectorTransactionId)
    }
}
