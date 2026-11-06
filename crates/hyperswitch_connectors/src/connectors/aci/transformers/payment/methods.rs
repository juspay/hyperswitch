//! One adapter per payment method ACI accepts.
//!
//! An adapter renders the method's instrument fields ([`PaymentDetails`]) and declares which of
//! the payment's processing rules apply to it. [`resolve`] is the only place hyperswitch payment
//! method data is mapped onto an adapter.

mod apple_pay;
mod bank_redirect;
mod card;
mod google_pay;
mod mandate;
mod network_token;
mod pay_later;
mod samsung_pay;
mod wallet_redirect;

use hyperswitch_domain_models::payment_method_data::{PaymentMethodData, WalletData};
use hyperswitch_interfaces::errors;

pub(in crate::connectors::aci::transformers) use self::card::card_details;
use self::{
    apple_pay::ApplePay,
    bank_redirect::BankRedirect,
    card::Card,
    google_pay::GooglePay,
    mandate::Mandate,
    network_token::NetworkToken,
    pay_later::PayLater,
    samsung_pay::SamsungPay,
    wallet_redirect::{AliPay, MbWay},
};
use super::{AciPayment, PaymentItem, PaymentPurpose};
use crate::{
    connectors::aci::transformers::{
        request::{AciPaymentsRequest, PaymentBrand, PaymentDetails},
        Error,
    },
    utils,
};

/// A payment method, as ACI takes it.
pub(super) trait PaymentMethodAdapter: Sized {
    /// Whether the method can carry a card-on-file agreement. One that cannot is sent as a
    /// one-off whatever the request says: no `standingInstruction.*` block, no legacy
    /// `recurringType`, and no shopper-authentication stripping.
    const CARD_ON_FILE: bool;

    /// Whether the agreement is also stated in the legacy top-level `recurringType`. Every
    /// card-on-file method does except a raw network token (a preserved inconsistency: its
    /// decrypted wallet cousins do).
    const LEGACY_RECURRING_TYPE: bool = Self::CARD_ON_FILE;

    /// Whether ACI's own 3DS2 applies: `customParameters[3DS2_enrolled]` and the MPGS browser
    /// rules. Card only.
    const CONNECTOR_THREE_DS: bool = false;

    /// The instrument fields for `payment`.
    fn instrument(self, payment: &AciPayment<'_>) -> Result<PaymentDetails, Error>;

    /// The request for `payment` made with this method. The transaction (credentials, capture
    /// method) is validated before the instrument, so a request faulty in both reports the
    /// transaction's error.
    fn request(self, payment: AciPayment<'_>) -> Result<AciPaymentsRequest, Error> {
        let txn_details = payment.transaction_details()?;
        let instrument = self.instrument(&payment)?;
        payment.into_request::<Self>(txn_details, instrument)
    }
}

/// The request for `item`, made with whichever adapter its payment method data maps to.
pub(super) fn resolve(
    item: PaymentItem<'_>,
    purpose: PaymentPurpose,
) -> Result<AciPaymentsRequest, Error> {
    fn with<M: PaymentMethodAdapter>(
        method: M,
        item: PaymentItem<'_>,
        purpose: PaymentPurpose,
    ) -> Result<AciPaymentsRequest, Error> {
        method.request(AciPayment::new::<M>(item, purpose))
    }

    match &item.router_data.request.payment_method_data {
        PaymentMethodData::Card(card) => with(Card::WithCvc(card), item, purpose),
        PaymentMethodData::CardWithOptionalCVC(card) => {
            with(Card::OptionalCvc(card), item, purpose)
        }
        PaymentMethodData::NetworkToken(token) => with(NetworkToken(token), item, purpose),
        PaymentMethodData::Wallet(wallet) => match wallet {
            WalletData::ApplePay(wallet) => with(ApplePay(wallet), item, purpose),
            WalletData::GooglePay(wallet) => with(GooglePay(wallet), item, purpose),
            WalletData::SamsungPay(wallet) => with(SamsungPay(wallet), item, purpose),
            WalletData::MbWayRedirect(_) => with(MbWay, item, purpose),
            WalletData::AliPayRedirect { .. } => with(AliPay, item, purpose),
            WalletData::AliPayHkRedirect(_)
            | WalletData::AmazonPayRedirect(_)
            | WalletData::Paysera(_)
            | WalletData::Skrill(_)
            | WalletData::Neteller(_)
            | WalletData::MomoRedirect(_)
            | WalletData::KakaoPayRedirect(_)
            | WalletData::GoPayRedirect(_)
            | WalletData::GcashRedirect(_)
            | WalletData::AmazonPay(_)
            | WalletData::ApplePayThirdPartySdk(_)
            | WalletData::DanaRedirect { .. }
            | WalletData::BluecodeRedirect {}
            | WalletData::GooglePayThirdPartySdk(_)
            | WalletData::MobilePayRedirect(_)
            | WalletData::PaypalRedirect(_)
            | WalletData::PaypalSdk(_)
            | WalletData::Paze(_)
            | WalletData::TwintRedirect { .. }
            | WalletData::VippsRedirect { .. }
            | WalletData::WeroRedirect { .. }
            | WalletData::TouchNGoRedirect(_)
            | WalletData::WeChatPayRedirect(_)
            | WalletData::WeChatPayQr(_)
            | WalletData::CashappQr(_)
            | WalletData::SwishQr(_)
            | WalletData::AliPayQr(_)
            | WalletData::ApplePayRedirect(_)
            | WalletData::GooglePayRedirect(_)
            | WalletData::Mifinity(_)
            | WalletData::RevolutPay(_) => Err(unsupported_variant()),
        },
        PaymentMethodData::PayLater(pay_later) => with(PayLater(pay_later), item, purpose),
        PaymentMethodData::BankRedirect(bank_redirect) => {
            with(BankRedirect(bank_redirect), item, purpose)
        }
        PaymentMethodData::MandatePayment => {
            // The registration is addressed by the URL, built from this id.
            item.router_data.request.mandate_id.as_ref().ok_or(
                errors::ConnectorError::MissingRequiredField {
                    field_name: "mandate_id".into(),
                },
            )?;
            with(Mandate, item, purpose)
        }
        PaymentMethodData::Crypto(_)
        | PaymentMethodData::BankDebit(_)
        | PaymentMethodData::BankTransfer(_)
        | PaymentMethodData::Reward
        | PaymentMethodData::RealTimePayment(_)
        | PaymentMethodData::MobilePayment(_)
        | PaymentMethodData::GiftCard(_)
        | PaymentMethodData::CardRedirect(_)
        | PaymentMethodData::Upi(_)
        | PaymentMethodData::Voucher(_)
        | PaymentMethodData::OpenBanking(_)
        | PaymentMethodData::CardToken(_)
        | PaymentMethodData::CardDetailsForNetworkTransactionId(_)
        | PaymentMethodData::CardWithNetworkTokenDetails(_)
        | PaymentMethodData::CardWithLimitedDetails(_)
        | PaymentMethodData::DecryptedWalletTokenDetailsForNetworkTransactionId(_)
        | PaymentMethodData::NetworkTokenDetailsForNetworkTransactionId(_) => {
            Err(unsupported_payment_method())
        }
    }
}

/// A payment method ACI is not integrated for.
fn unsupported_payment_method() -> error_stack::Report<errors::ConnectorError> {
    errors::ConnectorError::NotImplemented(utils::get_unimplemented_payment_method_error_message(
        "Aci",
    ))
    .into()
}

/// A variant of a supported family (a wallet or bank redirect) ACI is not integrated for.
fn unsupported_variant() -> error_stack::Report<errors::ConnectorError> {
    errors::ConnectorError::NotImplemented("Payment method".to_string()).into()
}

impl PaymentBrand {
    /// The brand for a card network. An unsupported network is an error, and so is an absent one.
    pub(in crate::connectors::aci::transformers) fn for_card_network(
        card_network: Option<&common_enums::CardNetwork>,
    ) -> Result<Self, Error> {
        use common_enums::CardNetwork;

        match card_network {
            Some(CardNetwork::Visa) => Ok(Self::Visa),
            Some(CardNetwork::Mastercard) => Ok(Self::Mastercard),
            Some(CardNetwork::AmericanExpress) => Ok(Self::AmericanExpress),
            Some(CardNetwork::JCB) => Ok(Self::Jcb),
            Some(CardNetwork::DinersClub) => Ok(Self::DinersClub),
            Some(CardNetwork::Discover) => Ok(Self::Discover),
            Some(CardNetwork::UnionPay) => Ok(Self::UnionPay),
            Some(CardNetwork::Maestro) => Ok(Self::Maestro),
            Some(unsupported_network) => Err(errors::ConnectorError::NotSupported {
                message: format!("Card network {unsupported_network} is not supported by ACI"),
                connector: "ACI".into(),
            })?,
            None => Err(errors::ConnectorError::MissingRequiredField {
                field_name: "card.card_network".into(),
            }
            .into()),
        }
    }

    /// The card brand a wallet names its network by, as Apple Pay and Google Pay spell it.
    fn for_wallet_network(network: &str) -> Option<Self> {
        match network.to_lowercase().as_str() {
            "visa" => Some(Self::Visa),
            "mastercard" => Some(Self::Mastercard),
            "amex" | "americanexpress" | "american express" => Some(Self::AmericanExpress),
            "jcb" => Some(Self::Jcb),
            "diners" | "dinersclub" | "diners club" => Some(Self::DinersClub),
            "discover" => Some(Self::Discover),
            "unionpay" | "union pay" => Some(Self::UnionPay),
            "maestro" => Some(Self::Maestro),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// [PT-3979] Every card network the brand mapper knows is accepted — mandate setup used to
    /// allow-list Visa/MC/Amex and synthetically reject Diners with IR-19 before the gateway
    /// ever saw the request.
    #[test]
    fn card_networks_map_to_their_brands() {
        use common_enums::CardNetwork;

        for (network, brand) in [
            (CardNetwork::Visa, "VISA"),
            (CardNetwork::Mastercard, "MASTER"),
            (CardNetwork::AmericanExpress, "AMEX"),
            (CardNetwork::JCB, "JCB"),
            (CardNetwork::DinersClub, "DINERS"),
            (CardNetwork::Discover, "DISCOVER"),
            (CardNetwork::UnionPay, "UNIONPAY"),
            (CardNetwork::Maestro, "MAESTRO"),
        ] {
            let mapped = PaymentBrand::for_card_network(Some(&network))
                .unwrap_or_else(|_| panic!("{network:?} maps to an ACI payment brand"));
            assert_eq!(
                serde_json::to_value(mapped).expect("brand serializes"),
                serde_json::json!(brand)
            );
        }
        assert!(PaymentBrand::for_card_network(Some(&CardNetwork::Interac)).is_err());
        assert!(PaymentBrand::for_card_network(None).is_err());
    }

    #[test]
    fn wallet_networks_are_matched_case_insensitively() {
        assert_eq!(
            PaymentBrand::for_wallet_network("MasterCard"),
            Some(PaymentBrand::Mastercard)
        );
        assert_eq!(
            PaymentBrand::for_wallet_network("American Express"),
            Some(PaymentBrand::AmericanExpress)
        );
        assert_eq!(PaymentBrand::for_wallet_network("elo"), None);
    }
}
