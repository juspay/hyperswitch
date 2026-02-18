#[cfg(feature = "v2")]
use std::str::FromStr;

use api_models::{
    mandates,
    payment_methods::{self},
    payments::{
        additional_info as payment_additional_types, AdditionalNetworkTokenInfo, ExtendedCardInfo,
    },
};
use common_enums::{enums as api_enums, GooglePayCardFundingSource};
use common_utils::{
    ext_traits::{OptionExt, StringExt},
    id_type,
    new_type::{
        MaskedBankAccount, MaskedIban, MaskedRoutingNumber, MaskedSortCode, MaskedUpiVpaId,
    },
    payout_method_utils,
    pii::{self, Email},
};
use masking::{ExposeInterface, PeekInterface, Secret};
use serde::{Deserialize, Serialize};
use time::Date;

use crate::router_data::PaymentMethodToken;

// We need to derive Serialize and Deserialize because some parts of payment method data are being
// stored in the database as serde_json::Value
#[derive(PartialEq, Clone, Debug, Serialize, Deserialize)]
pub enum PaymentMethodData {
    Card(Card),
    CardDetailsForNetworkTransactionId(CardDetailsForNetworkTransactionId),
    CardWithLimitedDetails(CardWithLimitedDetails),
    NetworkTokenDetailsForNetworkTransactionId(NetworkTokenDetailsForNetworkTransactionId),
    DecryptedWalletTokenDetailsForNetworkTransactionId(
        DecryptedWalletTokenDetailsForNetworkTransactionId,
    ),
    CardRedirect(CardRedirectData),
    Wallet(WalletData),
    PayLater(PayLaterData),
    BankRedirect(BankRedirectData),
    BankDebit(BankDebitData),
    BankTransfer(Box<BankTransferData>),
    Crypto(CryptoData),
    MandatePayment,
    Reward,
    RealTimePayment(Box<RealTimePaymentData>),
    Upi(UpiData),
    Voucher(VoucherData),
    GiftCard(Box<GiftCardData>),
    CardToken(CardToken),
    OpenBanking(OpenBankingData),
    NetworkToken(NetworkTokenData),
    MobilePayment(MobilePaymentData),
}

#[derive(PartialEq, Clone, Debug, Serialize, Deserialize)]
pub enum ExternalVaultPaymentMethodData {
    Card(Box<ExternalVaultCard>),
    VaultToken(VaultToken),
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(tag = "type", content = "data", rename_all = "snake_case")]
pub enum RecurringDetails {
    MandateId(String),
    PaymentMethodId(String),
    ProcessorPaymentToken(ProcessorPaymentToken),
    /// Network transaction ID and Card Details for MIT payments when payment_method_data
    /// is not stored in the application
    NetworkTransactionIdAndCardDetails(Box<NetworkTransactionIdAndCardDetails>),

    /// Network transaction ID and Network Token Details for MIT payments when payment_method_data
    /// is not stored in the application
    NetworkTransactionIdAndNetworkTokenDetails(Box<NetworkTransactionIdAndNetworkTokenDetails>),

    /// Network transaction ID and Wallet Token details for MIT payments when payment_method_data
    /// is not stored in the application
    /// Applicable for wallet tokens such as Apple Pay and Google Pay.
    NetworkTransactionIdAndDecryptedWalletTokenDetails(
        Box<common_types::payments::NetworkTransactionIdAndDecryptedWalletTokenDetails>,
    ),

    CardWithLimitedData(Box<CardWithLimitedData>),
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct ProcessorPaymentToken {
    pub processor_payment_token: String,
    pub merchant_connector_id: Option<id_type::MerchantConnectorAccountId>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct NetworkTransactionIdAndCardDetails {
    /// The card number
    pub card_number: cards::CardNumber,

    /// The card's expiry month
    pub card_exp_month: Secret<String>,

    /// The card's expiry year
    pub card_exp_year: Secret<String>,

    /// The card holder's name
    pub card_holder_name: Option<Secret<String>>,

    /// The name of the issuer of card
    pub card_issuer: Option<String>,

    /// The card network for the card
    pub card_network: Option<api_enums::CardNetwork>,

    pub card_type: Option<String>,

    pub card_issuing_country: Option<String>,

    pub card_issuing_country_code: Option<String>,

    pub bank_code: Option<String>,

    /// The card holder's nick name
    pub nick_name: Option<Secret<String>>,

    /// The network transaction ID provided by the card network during a CIT (Customer Initiated Transaction),
    /// when `setup_future_usage` is set to `off_session`.
    pub network_transaction_id: Secret<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct NetworkTransactionIdAndNetworkTokenDetails {
    /// The Network Token
    pub network_token: cards::NetworkToken,

    /// The token's expiry month
    pub token_exp_month: Secret<String>,

    /// The token's expiry year
    pub token_exp_year: Secret<String>,

    /// The card network for the card
    pub card_network: Option<api_enums::CardNetwork>,

    /// The type of the card such as Credit, Debit
    pub card_type: Option<String>,

    /// The country in which the card was issued
    pub card_issuing_country: Option<String>,

    /// The bank code of the bank that issued the card
    pub bank_code: Option<String>,

    /// The card holder's name
    pub card_holder_name: Option<Secret<String>>,

    /// The name of the issuer of card
    pub card_issuer: Option<String>,

    /// The card holder's nick name
    pub nick_name: Option<Secret<String>>,

    /// The ECI(Electronic Commerce Indicator) value for this authentication.
    pub eci: Option<String>,

    /// The network transaction ID provided by the card network during a Customer Initiated Transaction (CIT)
    /// when `setup_future_usage` is set to `off_session`.
    pub network_transaction_id: Secret<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct CardWithLimitedData {
    /// The card number
    pub card_number: cards::CardNumber,

    /// The card's expiry month
    pub card_exp_month: Option<Secret<String>>,

    /// The card's expiry year
    pub card_exp_year: Option<Secret<String>>,

    /// The card holder's name
    pub card_holder_name: Option<Secret<String>>,

    /// The ECI(Electronic Commerce Indicator) value for this authentication.
    pub eci: Option<String>,
}

// Determines if decryption should be performed
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub enum ApplePayFlow {
    // Either Merchant provided certificates i.e decryption by hyperswitch or Hyperswitch certificates i.e simplified flow
    // decryption is performed in hyperswitch
    DecryptAtApplication(api_models::payments::PaymentProcessingDetails),
    // decryption by connector or predecrypted token
    SkipDecryption,
}

impl PaymentMethodData {
    pub fn apply_additional_payment_data(
        &self,
        additional_payment_data: api_models::payments::AdditionalPaymentData,
    ) -> Self {
        if let api_models::payments::AdditionalPaymentData::Card(additional_card_info) =
            additional_payment_data
        {
            match self {
                Self::Card(card) => {
                    Self::Card(card.apply_additional_card_info(*additional_card_info))
                }
                Self::CardWithLimitedDetails(card_with_limited_details) => {
                    Self::CardWithLimitedDetails(
                        card_with_limited_details.apply_additional_card_info(*additional_card_info),
                    )
                }
                Self::CardDetailsForNetworkTransactionId(card_with_network_transaction_id) => {
                    Self::CardDetailsForNetworkTransactionId(
                        card_with_network_transaction_id
                            .apply_additional_card_info(*additional_card_info),
                    )
                }
                _ => self.to_owned(),
            }
        } else {
            self.to_owned()
        }
    }

    pub fn get_payment_method(&self) -> Option<common_enums::PaymentMethod> {
        match self {
            Self::Card(_)
            | Self::NetworkToken(_)
            | Self::CardDetailsForNetworkTransactionId(_)
            | Self::NetworkTokenDetailsForNetworkTransactionId(_)
            | Self::DecryptedWalletTokenDetailsForNetworkTransactionId(_)
            | Self::CardWithLimitedDetails(_) => Some(common_enums::PaymentMethod::Card),
            Self::CardRedirect(_) => Some(common_enums::PaymentMethod::CardRedirect),
            Self::Wallet(_) => Some(common_enums::PaymentMethod::Wallet),
            Self::PayLater(_) => Some(common_enums::PaymentMethod::PayLater),
            Self::BankRedirect(_) => Some(common_enums::PaymentMethod::BankRedirect),
            Self::BankDebit(_) => Some(common_enums::PaymentMethod::BankDebit),
            Self::BankTransfer(_) => Some(common_enums::PaymentMethod::BankTransfer),
            Self::Crypto(_) => Some(common_enums::PaymentMethod::Crypto),
            Self::Reward => Some(common_enums::PaymentMethod::Reward),
            Self::RealTimePayment(_) => Some(common_enums::PaymentMethod::RealTimePayment),
            Self::Upi(_) => Some(common_enums::PaymentMethod::Upi),
            Self::Voucher(_) => Some(common_enums::PaymentMethod::Voucher),
            Self::GiftCard(_) => Some(common_enums::PaymentMethod::GiftCard),
            Self::OpenBanking(_) => Some(common_enums::PaymentMethod::OpenBanking),
            Self::MobilePayment(_) => Some(common_enums::PaymentMethod::MobilePayment),
            Self::CardToken(_) | Self::MandatePayment => None,
        }
    }

    pub fn get_wallet_data(&self) -> Option<&WalletData> {
        if let Self::Wallet(wallet_data) = self {
            Some(wallet_data)
        } else {
            None
        }
    }

    pub fn is_network_token_payment_method_data(&self) -> bool {
        matches!(self, Self::NetworkToken(_))
    }

    pub fn get_co_badged_card_data(&self) -> Option<&payment_methods::CoBadgedCardData> {
        if let Self::Card(card) = self {
            card.co_badged_card_data.as_ref()
        } else {
            None
        }
    }

    pub fn get_card_data(&self) -> Option<&Card> {
        if let Self::Card(card) = self {
            Some(card)
        } else {
            None
        }
    }

    pub fn extract_debit_routing_saving_percentage(
        &self,
        network: &common_enums::CardNetwork,
    ) -> Option<f64> {
        self.get_co_badged_card_data()?
            .co_badged_card_networks_info
            .0
            .iter()
            .find(|info| &info.network == network)
            .and_then(|info| info.saving_percentage)
    }
}

#[derive(PartialEq, Clone, Debug, Serialize, Deserialize, Default)]
pub struct Card {
    pub card_number: cards::CardNumber,
    pub card_exp_month: Secret<String>,
    pub card_exp_year: Secret<String>,
    pub card_cvc: Secret<String>,
    pub card_issuer: Option<String>,
    pub card_network: Option<common_enums::CardNetwork>,
    pub card_type: Option<String>,
    pub card_issuing_country: Option<String>,
    pub card_issuing_country_code: Option<String>,
    pub bank_code: Option<String>,
    pub nick_name: Option<Secret<String>>,
    pub card_holder_name: Option<Secret<String>>,
    pub co_badged_card_data: Option<payment_methods::CoBadgedCardData>,
}

impl Card {
    fn apply_additional_card_info(
        &self,
        additional_card_info: api_models::payments::AdditionalCardInfo,
    ) -> Self {
        Self {
            card_number: self.card_number.clone(),
            card_exp_month: self.card_exp_month.clone(),
            card_exp_year: self.card_exp_year.clone(),
            card_holder_name: self.card_holder_name.clone(),
            card_cvc: self.card_cvc.clone(),
            card_issuer: self
                .card_issuer
                .clone()
                .or(additional_card_info.card_issuer),
            card_network: self
                .card_network
                .clone()
                .or(additional_card_info.card_network.clone()),
            card_type: self.card_type.clone().or(additional_card_info.card_type),
            card_issuing_country: self
                .card_issuing_country
                .clone()
                .or(additional_card_info.card_issuing_country),
            card_issuing_country_code: self
                .card_issuing_country_code
                .clone()
                .or(additional_card_info.card_issuing_country_code),
            bank_code: self.bank_code.clone().or(additional_card_info.bank_code),
            nick_name: self.nick_name.clone(),
            co_badged_card_data: self.co_badged_card_data.clone(),
        }
    }
}

#[derive(PartialEq, Clone, Debug, Serialize, Deserialize, Default)]
pub struct ExternalVaultCard {
    pub card_number: Secret<String>,
    pub card_exp_month: Secret<String>,
    pub card_exp_year: Secret<String>,
    pub card_cvc: Secret<String>,
    pub bin_number: Option<String>,
    pub last_four: Option<String>,
    pub card_issuer: Option<String>,
    pub card_network: Option<common_enums::CardNetwork>,
    pub card_type: Option<String>,
    pub card_issuing_country: Option<String>,
    pub bank_code: Option<String>,
    pub nick_name: Option<Secret<String>>,
    pub card_holder_name: Option<Secret<String>>,
    pub co_badged_card_data: Option<payment_methods::CoBadgedCardData>,
}

#[derive(PartialEq, Clone, Debug, Serialize, Deserialize, Default)]
pub struct VaultToken {
    pub card_cvc: Secret<String>,
    pub card_holder_name: Option<Secret<String>>,
}

#[derive(Eq, PartialEq, Clone, Debug, Serialize, Deserialize, Default)]
pub struct CardDetailsForNetworkTransactionId {
    pub card_number: cards::CardNumber,
    pub card_exp_month: Secret<String>,
    pub card_exp_year: Secret<String>,
    pub card_issuer: Option<String>,
    pub card_network: Option<common_enums::CardNetwork>,
    pub card_type: Option<String>,
    pub card_issuing_country: Option<String>,
    pub card_issuing_country_code: Option<String>,
    pub bank_code: Option<String>,
    pub nick_name: Option<Secret<String>>,
    pub card_holder_name: Option<Secret<String>>,
}

impl CardDetailsForNetworkTransactionId {
    fn apply_additional_card_info(
        &self,
        additional_card_info: api_models::payments::AdditionalCardInfo,
    ) -> Self {
        Self {
            card_number: self.card_number.clone(),
            card_exp_month: self.card_exp_month.clone(),
            card_exp_year: self.card_exp_year.clone(),
            card_holder_name: self.card_holder_name.clone(),
            card_issuer: self
                .card_issuer
                .clone()
                .or(additional_card_info.card_issuer),
            card_network: self
                .card_network
                .clone()
                .or(additional_card_info.card_network.clone()),
            card_type: self.card_type.clone().or(additional_card_info.card_type),
            card_issuing_country: self
                .card_issuing_country
                .clone()
                .or(additional_card_info.card_issuing_country),
            card_issuing_country_code: self
                .card_issuing_country_code
                .clone()
                .or(additional_card_info.card_issuing_country_code),
            bank_code: self.bank_code.clone().or(additional_card_info.bank_code),
            nick_name: self.nick_name.clone(),
        }
    }
}

#[derive(Eq, PartialEq, Clone, Debug, Serialize, Deserialize, Default)]
pub struct NetworkTokenDetailsForNetworkTransactionId {
    pub network_token: cards::NetworkToken,
    pub token_exp_month: Secret<String>,
    pub token_exp_year: Secret<String>,
    pub card_issuer: Option<String>,
    pub card_network: Option<common_enums::CardNetwork>,
    pub card_type: Option<String>,
    pub card_issuing_country: Option<String>,
    pub bank_code: Option<String>,
    pub nick_name: Option<Secret<String>>,
    pub card_holder_name: Option<Secret<String>>,
    pub eci: Option<String>,
}

#[derive(Eq, PartialEq, Clone, Debug, Serialize, Deserialize, Default)]
pub struct DecryptedWalletTokenDetailsForNetworkTransactionId {
    pub decrypted_token: cards::NetworkToken,
    pub token_exp_month: Secret<String>,
    pub token_exp_year: Secret<String>,
    pub card_holder_name: Option<Secret<String>>,
    pub eci: Option<String>,
    pub token_source: Option<common_types::payments::TokenSource>,
}

#[derive(PartialEq, Clone, Debug, Serialize, Deserialize, Default)]
pub struct CardDetail {
    pub card_number: cards::CardNumber,
    pub card_exp_month: Secret<String>,
    pub card_exp_year: Secret<String>,
    pub card_issuer: Option<String>,
    pub card_network: Option<api_enums::CardNetwork>,
    pub card_type: Option<String>,
    pub card_issuing_country: Option<String>,
    pub card_issuing_country_code: Option<String>,
    pub bank_code: Option<String>,
    pub nick_name: Option<Secret<String>>,
    pub card_holder_name: Option<Secret<String>>,
    pub co_badged_card_data: Option<payment_methods::CoBadgedCardData>,
}

#[derive(PartialEq, Clone, Debug, Serialize, Deserialize, Default)]
pub struct CardWithLimitedDetails {
    pub card_number: cards::CardNumber,
    pub card_exp_month: Option<Secret<String>>,
    pub card_exp_year: Option<Secret<String>>,
    pub card_issuer: Option<String>,
    pub card_network: Option<api_enums::CardNetwork>,
    pub card_type: Option<String>,
    pub card_issuing_country: Option<String>,
    pub card_issuing_country_code: Option<String>,
    pub bank_code: Option<String>,
    pub nick_name: Option<Secret<String>>,
    pub card_holder_name: Option<Secret<String>>,
    pub eci: Option<String>,
}

impl CardWithLimitedDetails {
    fn apply_additional_card_info(
        &self,
        additional_card_info: api_models::payments::AdditionalCardInfo,
    ) -> Self {
        Self {
            card_number: self.card_number.clone(),
            card_exp_month: self.card_exp_month.clone(),
            card_exp_year: self.card_exp_year.clone(),
            card_holder_name: self.card_holder_name.clone(),
            card_issuer: self
                .card_issuer
                .clone()
                .or(additional_card_info.card_issuer),
            card_network: self
                .card_network
                .clone()
                .or(additional_card_info.card_network.clone()),
            card_type: self.card_type.clone().or(additional_card_info.card_type),
            card_issuing_country: self
                .card_issuing_country
                .clone()
                .or(additional_card_info.card_issuing_country),
            card_issuing_country_code: self
                .card_issuing_country_code
                .clone()
                .or(additional_card_info.card_issuing_country_code),
            bank_code: self.bank_code.clone().or(additional_card_info.bank_code),
            nick_name: self.nick_name.clone(),
            eci: self.eci.clone(),
        }
    }

    pub fn get_card_details_for_mit_flow(
        card_with_limited_data: CardWithLimitedData,
    ) -> (api_models::payments::MandateReferenceId, PaymentMethodData) {
        (
            api_models::payments::MandateReferenceId::CardWithLimitedData,
            PaymentMethodData::CardWithLimitedDetails(card_with_limited_data.into()),
        )
    }
}

impl CardDetailsForNetworkTransactionId {
    pub fn get_nti_and_card_details_for_mit_flow(
        network_transaction_id_and_card_details: NetworkTransactionIdAndCardDetails,
    ) -> (api_models::payments::MandateReferenceId, PaymentMethodData) {
        let mandate_reference_id = api_models::payments::MandateReferenceId::NetworkMandateId(
            network_transaction_id_and_card_details
                .network_transaction_id
                .peek()
                .to_string(),
        );

        (
            mandate_reference_id,
            PaymentMethodData::CardDetailsForNetworkTransactionId(
                network_transaction_id_and_card_details.into(),
            ),
        )
    }
}

impl NetworkTokenDetailsForNetworkTransactionId {
    pub fn get_nti_and_network_token_details_for_mit_flow(
        network_transaction_id_and_network_token_details: NetworkTransactionIdAndNetworkTokenDetails,
    ) -> (api_models::payments::MandateReferenceId, PaymentMethodData) {
        let mandate_reference_id = api_models::payments::MandateReferenceId::NetworkMandateId(
            network_transaction_id_and_network_token_details
                .network_transaction_id
                .peek()
                .to_string(),
        );

        (
            mandate_reference_id,
            PaymentMethodData::NetworkTokenDetailsForNetworkTransactionId(
                network_transaction_id_and_network_token_details.into(),
            ),
        )
    }
}

impl DecryptedWalletTokenDetailsForNetworkTransactionId {
    pub fn get_nti_and_decrypted_wallet_token_details_for_mit_flow(
        network_transaction_id_and_decrypted_wallet_token_details: common_types::payments::NetworkTransactionIdAndDecryptedWalletTokenDetails,
    ) -> (api_models::payments::MandateReferenceId, PaymentMethodData) {
        let mandate_reference_id = api_models::payments::MandateReferenceId::NetworkMandateId(
            network_transaction_id_and_decrypted_wallet_token_details
                .network_transaction_id
                .peek()
                .to_string(),
        );

        (
            mandate_reference_id,
            PaymentMethodData::DecryptedWalletTokenDetailsForNetworkTransactionId(
                network_transaction_id_and_decrypted_wallet_token_details.into(),
            ),
        )
    }
}

impl From<&Card> for CardDetail {
    fn from(item: &Card) -> Self {
        Self {
            card_number: item.card_number.to_owned(),
            card_exp_month: item.card_exp_month.to_owned(),
            card_exp_year: item.card_exp_year.to_owned(),
            card_issuer: item.card_issuer.to_owned(),
            card_network: item.card_network.to_owned(),
            card_type: item.card_type.to_owned(),
            card_issuing_country: item.card_issuing_country.to_owned(),
            card_issuing_country_code: item.card_issuing_country_code.to_owned(),
            bank_code: item.bank_code.to_owned(),
            nick_name: item.nick_name.to_owned(),
            card_holder_name: item.card_holder_name.to_owned(),
            co_badged_card_data: item.co_badged_card_data.to_owned(),
        }
    }
}

impl From<CardWithLimitedData> for CardWithLimitedDetails {
    fn from(card_with_limited_data: CardWithLimitedData) -> Self {
        Self {
            card_number: card_with_limited_data.card_number,
            card_exp_month: card_with_limited_data.card_exp_month,
            card_exp_year: card_with_limited_data.card_exp_year,
            card_issuer: None,
            card_network: None,
            card_type: None,
            card_issuing_country: None,
            card_issuing_country_code: None,
            bank_code: None,
            nick_name: None,
            card_holder_name: card_with_limited_data.card_holder_name,
            eci: card_with_limited_data.eci,
        }
    }
}

impl From<NetworkTransactionIdAndCardDetails> for CardDetailsForNetworkTransactionId {
    fn from(card_details_for_nti: NetworkTransactionIdAndCardDetails) -> Self {
        Self {
            card_number: card_details_for_nti.card_number,
            card_exp_month: card_details_for_nti.card_exp_month,
            card_exp_year: card_details_for_nti.card_exp_year,
            card_issuer: card_details_for_nti.card_issuer,
            card_network: card_details_for_nti.card_network,
            card_type: card_details_for_nti.card_type,
            card_issuing_country: card_details_for_nti.card_issuing_country,
            card_issuing_country_code: card_details_for_nti.card_issuing_country_code,
            bank_code: card_details_for_nti.bank_code,
            nick_name: card_details_for_nti.nick_name,
            card_holder_name: card_details_for_nti.card_holder_name,
        }
    }
}

impl From<NetworkTransactionIdAndNetworkTokenDetails>
    for NetworkTokenDetailsForNetworkTransactionId
{
    fn from(network_token_details_for_nti: NetworkTransactionIdAndNetworkTokenDetails) -> Self {
        Self {
            network_token: network_token_details_for_nti.network_token,
            token_exp_month: network_token_details_for_nti.token_exp_month,
            token_exp_year: network_token_details_for_nti.token_exp_year,
            card_network: network_token_details_for_nti.card_network,
            card_issuer: network_token_details_for_nti.card_issuer,
            card_type: network_token_details_for_nti.card_type,
            card_issuing_country: network_token_details_for_nti.card_issuing_country,
            bank_code: network_token_details_for_nti.bank_code,
            nick_name: network_token_details_for_nti.nick_name,
            card_holder_name: network_token_details_for_nti.card_holder_name,
            eci: network_token_details_for_nti.eci,
        }
    }
}

#[cfg(feature = "v1")]
impl From<NetworkTokenDetailsForNetworkTransactionId> for NetworkTokenData {
    fn from(network_token_details_for_nti: NetworkTokenDetailsForNetworkTransactionId) -> Self {
        Self {
            token_number: network_token_details_for_nti.network_token,
            token_exp_month: network_token_details_for_nti.token_exp_month,
            token_exp_year: network_token_details_for_nti.token_exp_year,
            token_cryptogram: None,
            card_issuer: network_token_details_for_nti.card_issuer,
            card_network: network_token_details_for_nti.card_network,
            card_type: network_token_details_for_nti.card_type,
            card_issuing_country: network_token_details_for_nti.card_issuing_country,
            bank_code: network_token_details_for_nti.bank_code,
            nick_name: network_token_details_for_nti.nick_name,
            eci: network_token_details_for_nti.eci,
        }
    }
}

impl From<common_types::payments::NetworkTransactionIdAndDecryptedWalletTokenDetails>
    for DecryptedWalletTokenDetailsForNetworkTransactionId
{
    fn from(
        decrypted_token_details_for_nti: common_types::payments::NetworkTransactionIdAndDecryptedWalletTokenDetails,
    ) -> Self {
        Self {
            decrypted_token: decrypted_token_details_for_nti.decrypted_token,
            token_exp_month: decrypted_token_details_for_nti.token_exp_month,
            token_exp_year: decrypted_token_details_for_nti.token_exp_year,
            card_holder_name: decrypted_token_details_for_nti.card_holder_name,
            token_source: decrypted_token_details_for_nti.token_source,
            eci: decrypted_token_details_for_nti.eci,
        }
    }
}

#[derive(Eq, PartialEq, Clone, Debug, Serialize, Deserialize)]
pub enum CardRedirectData {
    Knet {},
    Benefit {},
    MomoAtm {},
    CardRedirect {},
}

#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize)]
pub enum PayLaterData {
    KlarnaRedirect {},
    KlarnaSdk { token: String },
    AffirmRedirect {},
    AfterpayClearpayRedirect {},
    PayBrightRedirect {},
    WalleyRedirect {},
    FlexitiRedirect {},
    AlmaRedirect {},
    AtomeRedirect {},
    BreadpayRedirect {},
    PayjustnowRedirect {},
}

#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize)]
pub enum WalletData {
    AliPayQr(Box<AliPayQr>),
    AliPayRedirect(AliPayRedirection),
    AliPayHkRedirect(AliPayHkRedirection),
    AmazonPay(AmazonPayWalletData),
    AmazonPayRedirect(Box<AmazonPayRedirect>),
    BluecodeRedirect {},
    Paysera(Box<PayseraData>),
    Skrill(Box<SkrillData>),
    MomoRedirect(MomoRedirection),
    KakaoPayRedirect(KakaoPayRedirection),
    GoPayRedirect(GoPayRedirection),
    GcashRedirect(GcashRedirection),
    ApplePay(ApplePayWalletData),
    ApplePayRedirect(Box<ApplePayRedirectData>),
    ApplePayThirdPartySdk(Box<ApplePayThirdPartySdkData>),
    DanaRedirect {},
    GooglePay(GooglePayWalletData),
    GooglePayRedirect(Box<GooglePayRedirectData>),
    GooglePayThirdPartySdk(Box<GooglePayThirdPartySdkData>),
    MbWayRedirect(Box<MbWayRedirection>),
    MobilePayRedirect(Box<MobilePayRedirection>),
    PaypalRedirect(PaypalRedirection),
    PaypalSdk(PayPalWalletData),
    Paze(PazeWalletData),
    SamsungPay(Box<SamsungPayWalletData>),
    TwintRedirect {},
    VippsRedirect {},
    TouchNGoRedirect(Box<TouchNGoRedirection>),
    WeChatPayRedirect(Box<WeChatPayRedirection>),
    WeChatPayQr(Box<WeChatPayQr>),
    CashappQr(Box<CashappQr>),
    SwishQr(SwishQrData),
    Mifinity(MifinityData),
    RevolutPay(RevolutPayData),
}

impl WalletData {
    pub fn get_paze_wallet_data(&self) -> Option<&PazeWalletData> {
        if let Self::Paze(paze_wallet_data) = self {
            Some(paze_wallet_data)
        } else {
            None
        }
    }

    pub fn get_apple_pay_wallet_data(&self) -> Option<&ApplePayWalletData> {
        if let Self::ApplePay(apple_pay_wallet_data) = self {
            Some(apple_pay_wallet_data)
        } else {
            None
        }
    }

    pub fn get_google_pay_wallet_data(&self) -> Option<&GooglePayWalletData> {
        if let Self::GooglePay(google_pay_wallet_data) = self {
            Some(google_pay_wallet_data)
        } else {
            None
        }
    }
}

#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct MifinityData {
    pub date_of_birth: Secret<Date>,
    pub language_preference: Option<String>,
}

#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub struct PazeWalletData {
    pub complete_response: Secret<String>,
}

#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub struct SamsungPayWalletData {
    pub payment_credential: SamsungPayWalletCredentials,
}

#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub struct SamsungPayWalletCredentials {
    pub method: Option<String>,
    pub recurring_payment: Option<bool>,
    pub card_brand: common_enums::SamsungPayCardBrand,
    pub dpan_last_four_digits: Option<String>,
    #[serde(rename = "card_last4digits")]
    pub card_last_four_digits: String,
    #[serde(rename = "3_d_s")]
    pub token_data: SamsungPayTokenData,
}

#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub struct SamsungPayTokenData {
    #[serde(rename = "type")]
    pub three_ds_type: Option<String>,
    pub version: String,
    pub data: Secret<String>,
}

#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct GooglePayWalletData {
    /// The type of payment method
    pub pm_type: String,
    /// User-facing message to describe the payment method that funds this transaction.
    pub description: String,
    /// The information of the payment method
    pub info: GooglePayPaymentMethodInfo,
    /// The tokenization data of Google pay
    pub tokenization_data: common_types::payments::GpayTokenizationData,
}

#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct ApplePayRedirectData {}
#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct RevolutPayData {}

#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct GooglePayRedirectData {}

#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct GooglePayThirdPartySdkData {
    pub token: Option<Secret<String>>,
}

#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct ApplePayThirdPartySdkData {
    pub token: Option<Secret<String>>,
}

#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct WeChatPayRedirection {}

#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct WeChatPay {}

#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct WeChatPayQr {}

#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct CashappQr {}

#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct PaypalRedirection {
    /// paypal's email address
    pub email: Option<Email>,
}

#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct AliPayQr {}

#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct AliPayRedirection {}

#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct AliPayHkRedirection {}

#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct AmazonPayRedirect {}

#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct BluecodeQrRedirect {}

#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct PayseraData {}

#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct SkrillData {}

#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct MomoRedirection {}

#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct KakaoPayRedirection {}

#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct GoPayRedirection {}

#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct GcashRedirection {}

#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct MobilePayRedirection {}

#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct MbWayRedirection {}

#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct GooglePayPaymentMethodInfo {
    /// The name of the card network
    pub card_network: String,
    /// The details of the card
    pub card_details: String,
    /// assurance_details of the card
    pub assurance_details: Option<GooglePayAssuranceDetails>,
    /// Card funding source for the selected payment method
    pub card_funding_source: Option<GooglePayCardFundingSource>,
}

#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub struct GooglePayAssuranceDetails {
    ///indicates that Cardholder possession validation has been performed
    pub card_holder_authenticated: bool,
    /// indicates that identification and verifications (ID&V) was performed
    pub account_verified: bool,
}

#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct PayPalWalletData {
    /// Token generated for the Apple pay
    pub token: String,
}

#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct TouchNGoRedirection {}

#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct SwishQrData {}

#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct ApplePayWalletData {
    /// The payment data of Apple pay
    pub payment_data: common_types::payments::ApplePayPaymentData,
    /// The payment method of Apple pay
    pub payment_method: ApplepayPaymentMethod,
    /// The unique identifier for the transaction
    pub transaction_identifier: String,
}

impl ApplePayWalletData {
    pub fn get_payment_method_type(&self) -> Option<api_enums::PaymentMethodType> {
        self.payment_method
            .pm_type
            .clone()
            .parse_enum("ApplePayPaymentMethodType")
            .ok()
            .and_then(|payment_type| match payment_type {
                common_enums::ApplePayPaymentMethodType::Debit => {
                    Some(api_enums::PaymentMethodType::Debit)
                }
                common_enums::ApplePayPaymentMethodType::Credit => {
                    Some(api_enums::PaymentMethodType::Credit)
                }
                common_enums::ApplePayPaymentMethodType::Prepaid
                | common_enums::ApplePayPaymentMethodType::Store => None,
            })
    }
}

#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct ApplepayPaymentMethod {
    pub display_name: String,
    pub network: String,
    pub pm_type: String,
}

#[derive(Eq, PartialEq, Clone, Default, Debug, serde::Deserialize, serde::Serialize)]
pub struct AmazonPayWalletData {
    pub checkout_session_id: String,
}

#[derive(Debug, Clone, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub enum RealTimePaymentData {
    DuitNow {},
    Fps {},
    PromptPay {},
    VietQr {},
    Qris {},
}

#[derive(Debug, Clone, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub enum BankRedirectData {
    BancontactCard {
        card_number: Option<cards::CardNumber>,
        card_exp_month: Option<Secret<String>>,
        card_exp_year: Option<Secret<String>>,
        card_holder_name: Option<Secret<String>>,
    },
    Bizum {},
    Blik {
        blik_code: Option<String>,
    },
    Eps {
        bank_name: Option<common_enums::BankNames>,
        country: Option<api_enums::CountryAlpha2>,
    },
    Giropay {
        bank_account_bic: Option<Secret<String>>,
        bank_account_iban: Option<Secret<String>>,
        country: Option<api_enums::CountryAlpha2>,
    },
    Ideal {
        bank_name: Option<common_enums::BankNames>,
    },
    Interac {
        country: Option<api_enums::CountryAlpha2>,
        email: Option<Email>,
    },
    OnlineBankingCzechRepublic {
        issuer: common_enums::BankNames,
    },
    OnlineBankingFinland {
        email: Option<Email>,
    },
    OnlineBankingPoland {
        issuer: common_enums::BankNames,
    },
    OnlineBankingSlovakia {
        issuer: common_enums::BankNames,
    },
    OpenBankingUk {
        issuer: Option<common_enums::BankNames>,
        country: Option<api_enums::CountryAlpha2>,
    },
    Przelewy24 {
        bank_name: Option<common_enums::BankNames>,
    },
    Sofort {
        country: Option<api_enums::CountryAlpha2>,
        preferred_language: Option<String>,
    },
    Trustly {
        country: Option<api_enums::CountryAlpha2>,
    },
    OnlineBankingFpx {
        issuer: common_enums::BankNames,
    },
    OnlineBankingThailand {
        issuer: common_enums::BankNames,
    },
    LocalBankRedirect {},
    Eft {
        provider: String,
    },
    OpenBanking {},
}

#[derive(Debug, Clone, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum OpenBankingData {
    OpenBankingPIS {},
}

#[derive(Debug, Clone, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub struct CryptoData {
    pub pay_currency: Option<String>,
    pub network: Option<String>,
}

#[derive(Debug, Clone, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum UpiData {
    UpiCollect(UpiCollectData),
    UpiIntent(UpiIntentData),
    UpiQr(UpiQrData),
}

#[derive(Debug, Clone, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum UpiSource {
    UpiCc,      // UPI Credit Card
    UpiCl,      // UPI Credit Line
    UpiAccount, // UPI Bank Account (Savings)
    UpiCcCl,    // UPI Credit Card + Credit Line
    UpiPpi,     // UPI Prepaid Payment Instrument
    UpiVoucher, // UPI Voucher
}

impl From<api_models::payments::UpiSource> for UpiSource {
    fn from(value: api_models::payments::UpiSource) -> Self {
        match value {
            api_models::payments::UpiSource::UpiCc => Self::UpiCc,
            api_models::payments::UpiSource::UpiCl => Self::UpiCl,
            api_models::payments::UpiSource::UpiAccount => Self::UpiAccount,
            api_models::payments::UpiSource::UpiCcCl => Self::UpiCcCl,
            api_models::payments::UpiSource::UpiPpi => Self::UpiPpi,
            api_models::payments::UpiSource::UpiVoucher => Self::UpiVoucher,
        }
    }
}

impl From<UpiSource> for api_models::payments::UpiSource {
    fn from(value: UpiSource) -> Self {
        match value {
            UpiSource::UpiCc => Self::UpiCc,
            UpiSource::UpiCl => Self::UpiCl,
            UpiSource::UpiAccount => Self::UpiAccount,
            UpiSource::UpiCcCl => Self::UpiCcCl,
            UpiSource::UpiPpi => Self::UpiPpi,
            UpiSource::UpiVoucher => Self::UpiVoucher,
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub struct UpiCollectData {
    pub vpa_id: Option<Secret<String, pii::UpiVpaMaskingStrategy>>,
    pub upi_source: Option<UpiSource>,
}

#[derive(Debug, Clone, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct UpiIntentData {
    pub upi_source: Option<UpiSource>,
    pub app_name: Option<String>,
}

#[derive(Debug, Clone, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct UpiQrData {
    pub upi_source: Option<UpiSource>,
}

#[derive(Debug, Clone, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VoucherData {
    Boleto(Box<BoletoVoucherData>),
    Efecty,
    PagoEfectivo,
    RedCompra,
    RedPagos,
    Alfamart(Box<AlfamartVoucherData>),
    Indomaret(Box<IndomaretVoucherData>),
    Oxxo,
    SevenEleven(Box<JCSVoucherData>),
    Lawson(Box<JCSVoucherData>),
    MiniStop(Box<JCSVoucherData>),
    FamilyMart(Box<JCSVoucherData>),
    Seicomart(Box<JCSVoucherData>),
    PayEasy(Box<JCSVoucherData>),
}

#[derive(Debug, Clone, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct BoletoVoucherData {
    /// The shopper's social security number
    pub social_security_number: Option<Secret<String>>,
    /// The bank number associated with the boleto
    pub bank_number: Option<Secret<String>>,
    /// The type of document (e.g., CPF, CNPJ)
    pub document_type: Option<common_types::customers::DocumentKind>,
    /// The percentage of fine applied for late payment
    pub fine_percentage: Option<String>,
    /// The number of days after due date when fine is applied
    pub fine_quantity_days: Option<String>,
    /// The percentage of interest applied for late payment
    pub interest_percentage: Option<String>,
    /// Number of days after which the boleto can be written off
    pub write_off_quantity_days: Option<String>,
    /// Additional messages to display to the shopper
    pub messages: Option<Vec<String>>,
    /// The date upon which the boleto is due and is of format: "YYYY-MM-DD"
    pub due_date: Option<String>,
}

#[derive(Debug, Clone, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct AlfamartVoucherData {}

#[derive(Debug, Clone, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct IndomaretVoucherData {}

#[derive(Debug, Clone, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct JCSVoucherData {}

#[derive(serde::Deserialize, serde::Serialize, Debug, Clone, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum GiftCardData {
    Givex(GiftCardDetails),
    PaySafeCard {},
    BhnCardNetwork(BHNGiftCardDetails),
}

impl GiftCardData {
    pub fn is_givex(&self) -> bool {
        matches!(self, Self::Givex(_))
    }
    /// Returns a key that uniquely identifies the gift card. Used in
    /// Payment Method Balance Check Flow for storing the balance
    /// data in Redis.
    ///
    pub fn get_payment_method_key(
        &self,
    ) -> Result<Secret<String>, error_stack::Report<common_utils::errors::ValidationError>> {
        match self {
            Self::Givex(givex) => Ok(givex.number.clone()),
            Self::PaySafeCard {} =>
            // Generate a validation error here as we don't support balance check flow for it
            {
                Err(error_stack::Report::new(
                    common_utils::errors::ValidationError::InvalidValue {
                        message: "PaySafeCard doesn't support balance check flow".to_string(),
                    },
                ))
            }
            Self::BhnCardNetwork(bhn) => Ok(bhn.account_number.clone()),
        }
    }
}

#[derive(serde::Deserialize, serde::Serialize, Debug, Clone, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct GiftCardDetails {
    /// The gift card number
    pub number: Secret<String>,
    /// The card verification code.
    pub cvc: Secret<String>,
}

#[derive(serde::Deserialize, serde::Serialize, Debug, Clone, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct BHNGiftCardDetails {
    /// The gift card or account number
    pub account_number: Secret<String>,
    /// The security PIN for gift cards requiring it
    pub pin: Option<Secret<String>>,
    /// The CVV2 code for Open Loop/VPLN products
    pub cvv2: Option<Secret<String>>,
    /// The expiration date in MMYYYY format for Open Loop/VPLN products
    pub expiration_date: Option<String>,
}

#[derive(Eq, PartialEq, Debug, serde::Deserialize, serde::Serialize, Clone, Default)]
#[serde(rename_all = "snake_case")]
pub struct CardToken {
    /// The card holder's name
    pub card_holder_name: Option<Secret<String>>,

    /// The CVC number for the card
    pub card_cvc: Option<Secret<String>>,
}

#[derive(serde::Deserialize, serde::Serialize, Debug, Clone, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum BankDebitData {
    AchBankDebit {
        account_number: Secret<String>,
        routing_number: Secret<String>,
        card_holder_name: Option<Secret<String>>,
        bank_account_holder_name: Option<Secret<String>>,
        bank_name: Option<common_enums::BankNames>,
        bank_type: Option<common_enums::BankType>,
        bank_holder_type: Option<common_enums::BankHolderType>,
    },
    SepaBankDebit {
        iban: Secret<String>,
        bank_account_holder_name: Option<Secret<String>>,
    },
    SepaGuarenteedBankDebit {
        iban: Secret<String>,
        bank_account_holder_name: Option<Secret<String>>,
    },
    BecsBankDebit {
        account_number: Secret<String>,
        bsb_number: Secret<String>,
        bank_account_holder_name: Option<Secret<String>>,
    },
    BacsBankDebit {
        account_number: Secret<String>,
        sort_code: Secret<String>,
        bank_account_holder_name: Option<Secret<String>>,
    },
}

impl BankDebitData {
    pub fn get_bank_debit_details(self) -> Option<BankDebitDetailsPaymentMethod> {
        match self {
            Self::AchBankDebit {
                account_number,
                routing_number,
                card_holder_name,
                bank_account_holder_name,
                bank_name,
                bank_type,
                bank_holder_type,
            } => Some(BankDebitDetailsPaymentMethod::AchBankDebit {
                masked_account_number: account_number
                    .peek()
                    .chars()
                    .rev()
                    .take(4)
                    .collect::<String>()
                    .chars()
                    .rev()
                    .collect::<String>(),
                masked_routing_number: routing_number
                    .peek()
                    .chars()
                    .rev()
                    .take(4)
                    .collect::<String>()
                    .chars()
                    .rev()
                    .collect::<String>(),
                card_holder_name,
                bank_account_holder_name,
                bank_name,
                bank_type,
                bank_holder_type,
            }),
            Self::SepaBankDebit { .. }
            | Self::SepaGuarenteedBankDebit { .. }
            | Self::BecsBankDebit { .. }
            | Self::BacsBankDebit { .. } => None,
        }
    }
}

#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BankTransferData {
    AchBankTransfer {},
    SepaBankTransfer {},
    BacsBankTransfer {},
    MultibancoBankTransfer {},
    PermataBankTransfer {},
    BcaBankTransfer {},
    BniVaBankTransfer {},
    BriVaBankTransfer {},
    CimbVaBankTransfer {},
    DanamonVaBankTransfer {},
    MandiriVaBankTransfer {},
    Pix {
        /// Unique key for pix transfer
        pix_key: Option<Secret<String>>,
        /// CPF is a Brazilian tax identification number
        cpf: Option<Secret<String>>,
        /// CNPJ is a Brazilian company tax identification number
        cnpj: Option<Secret<String>>,
        /// Source bank account UUID
        source_bank_account_id: Option<MaskedBankAccount>,
        /// Destination bank account UUID.
        destination_bank_account_id: Option<MaskedBankAccount>,
        /// The expiration date and time for the Pix QR code
        expiry_date: Option<time::PrimitiveDateTime>,
    },
    Pse {},
    LocalBankTransfer {
        bank_code: Option<String>,
    },
    InstantBankTransfer {},
    InstantBankTransferFinland {},
    InstantBankTransferPoland {},
    IndonesianBankTransfer {
        bank_name: Option<common_enums::BankNames>,
    },
}

#[derive(Debug, Clone, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct SepaAndBacsBillingDetails {
    /// The Email ID for SEPA and BACS billing
    pub email: Email,
    /// The billing name for SEPA and BACS billing
    pub name: Secret<String>,
}

#[cfg(feature = "v1")]
#[derive(Eq, PartialEq, Clone, Debug, Serialize, Deserialize, Default)]
pub struct NetworkTokenData {
    pub token_number: cards::NetworkToken,
    pub token_exp_month: Secret<String>,
    pub token_exp_year: Secret<String>,
    pub token_cryptogram: Option<Secret<String>>,
    pub card_issuer: Option<String>,
    pub card_network: Option<common_enums::CardNetwork>,
    pub card_type: Option<String>,
    pub card_issuing_country: Option<String>,
    pub bank_code: Option<String>,
    pub nick_name: Option<Secret<String>>,
    pub eci: Option<String>,
}

#[cfg(feature = "v2")]
#[derive(Eq, PartialEq, Clone, Debug, Serialize, Deserialize, Default)]
pub struct NetworkTokenData {
    pub network_token: cards::NetworkToken,
    pub network_token_exp_month: Secret<String>,
    pub network_token_exp_year: Secret<String>,
    pub cryptogram: Option<Secret<String>>,
    pub card_issuer: Option<String>, //since network token is tied to card, so its issuer will be same as card issuer
    pub card_network: Option<common_enums::CardNetwork>,
    pub card_type: Option<payment_methods::CardType>,
    pub card_issuing_country: Option<common_enums::CountryAlpha2>,
    pub bank_code: Option<String>,
    pub card_holder_name: Option<Secret<String>>,
    pub nick_name: Option<Secret<String>>,
    pub eci: Option<String>,
}

#[derive(Eq, PartialEq, Clone, Debug, Serialize, Deserialize, Default)]
pub struct NetworkTokenDetails {
    pub network_token: cards::NetworkToken,
    pub network_token_exp_month: Secret<String>,
    pub network_token_exp_year: Secret<String>,
    pub cryptogram: Option<Secret<String>>,
    pub card_issuer: Option<String>, //since network token is tied to card, so its issuer will be same as card issuer
    pub card_network: Option<common_enums::CardNetwork>,
    pub card_type: Option<payment_methods::CardType>,
    pub card_issuing_country: Option<api_enums::CountryAlpha2>,
    pub card_holder_name: Option<Secret<String>>,
    pub nick_name: Option<Secret<String>>,
}

#[derive(Eq, PartialEq, Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MobilePaymentData {
    DirectCarrierBilling {
        /// The phone number of the user
        msisdn: String,
        /// Unique user identifier
        client_uid: Option<String>,
    },
}

#[cfg(feature = "v2")]
impl TryFrom<payment_methods::PaymentMethodCreateData> for PaymentMethodData {
    type Error = error_stack::Report<common_utils::errors::ValidationError>;

    fn try_from(value: payment_methods::PaymentMethodCreateData) -> Result<Self, Self::Error> {
        match value {
            payment_methods::PaymentMethodCreateData::Card(payment_methods::CardDetail {
                card_number,
                card_exp_month,
                card_exp_year,
                card_cvc,
                card_issuer,
                card_network,
                card_type,
                card_issuing_country,
                nick_name,
                card_holder_name,
            }) => Ok(Self::Card(Card {
                card_number,
                card_exp_month,
                card_exp_year,
                card_cvc: card_cvc.get_required_value("card_cvc")?,
                card_issuer,
                card_network,
                card_type: card_type.map(|card_type| card_type.to_string()),
                card_issuing_country: card_issuing_country.map(|country| country.to_string()),
                card_issuing_country_code: None,
                bank_code: None,
                nick_name,
                card_holder_name,
                co_badged_card_data: None,
            })),
            payment_methods::PaymentMethodCreateData::ProxyCard(_) => Err(
                common_utils::errors::ValidationError::IncorrectValueProvided {
                    field_name: "Payment method data",
                }
                .into(),
            ),
        }
    }
}

impl From<api_models::payments::PaymentMethodData> for PaymentMethodData {
    fn from(api_model_payment_method_data: api_models::payments::PaymentMethodData) -> Self {
        match api_model_payment_method_data {
            api_models::payments::PaymentMethodData::Card(card_data) => {
                Self::Card(Card::from((card_data, None)))
            }
            api_models::payments::PaymentMethodData::CardRedirect(card_redirect) => {
                Self::CardRedirect(From::from(card_redirect))
            }
            api_models::payments::PaymentMethodData::Wallet(wallet_data) => {
                Self::Wallet(From::from(wallet_data))
            }
            api_models::payments::PaymentMethodData::PayLater(pay_later_data) => {
                Self::PayLater(From::from(pay_later_data))
            }
            api_models::payments::PaymentMethodData::BankRedirect(bank_redirect_data) => {
                Self::BankRedirect(From::from(bank_redirect_data))
            }
            api_models::payments::PaymentMethodData::BankDebit(bank_debit_data) => {
                Self::BankDebit(From::from(bank_debit_data))
            }
            api_models::payments::PaymentMethodData::BankTransfer(bank_transfer_data) => {
                Self::BankTransfer(Box::new(From::from(*bank_transfer_data)))
            }
            api_models::payments::PaymentMethodData::Crypto(crypto_data) => {
                Self::Crypto(From::from(crypto_data))
            }
            api_models::payments::PaymentMethodData::MandatePayment => Self::MandatePayment,
            api_models::payments::PaymentMethodData::Reward => Self::Reward,
            api_models::payments::PaymentMethodData::RealTimePayment(real_time_payment_data) => {
                Self::RealTimePayment(Box::new(From::from(*real_time_payment_data)))
            }
            api_models::payments::PaymentMethodData::Upi(upi_data) => {
                Self::Upi(From::from(upi_data))
            }
            api_models::payments::PaymentMethodData::Voucher(voucher_data) => {
                Self::Voucher(From::from(voucher_data))
            }
            api_models::payments::PaymentMethodData::GiftCard(gift_card) => {
                Self::GiftCard(Box::new(From::from(*gift_card)))
            }
            api_models::payments::PaymentMethodData::CardToken(card_token) => {
                Self::CardToken(From::from(card_token))
            }
            api_models::payments::PaymentMethodData::OpenBanking(ob_data) => {
                Self::OpenBanking(From::from(ob_data))
            }
            api_models::payments::PaymentMethodData::MobilePayment(mobile_payment_data) => {
                Self::MobilePayment(From::from(mobile_payment_data))
            }
            api_models::payments::PaymentMethodData::NetworkToken(network_token_data) => {
                Self::NetworkToken(From::from(network_token_data))
            }
        }
    }
}

impl From<api_models::payments::ProxyPaymentMethodData> for ExternalVaultPaymentMethodData {
    fn from(api_model_payment_method_data: api_models::payments::ProxyPaymentMethodData) -> Self {
        match api_model_payment_method_data {
            api_models::payments::ProxyPaymentMethodData::VaultDataCard(card_data) => {
                Self::Card(Box::new(ExternalVaultCard::from(*card_data)))
            }
            api_models::payments::ProxyPaymentMethodData::VaultToken(vault_data) => {
                Self::VaultToken(VaultToken::from(vault_data))
            }
        }
    }
}
impl From<api_models::payments::ProxyCardData> for ExternalVaultCard {
    fn from(value: api_models::payments::ProxyCardData) -> Self {
        let api_models::payments::ProxyCardData {
            card_number,
            card_exp_month,
            card_exp_year,
            card_holder_name,
            card_cvc,
            bin_number,
            last_four,
            card_issuer,
            card_network,
            card_type,
            card_issuing_country,
            bank_code,
            nick_name,
        } = value;

        Self {
            card_number,
            card_exp_month,
            card_exp_year,
            card_cvc,
            bin_number,
            last_four,
            card_issuer,
            card_network,
            card_type,
            card_issuing_country,
            bank_code,
            nick_name,
            card_holder_name,
            co_badged_card_data: None,
        }
    }
}
impl From<api_models::payments::VaultToken> for VaultToken {
    fn from(value: api_models::payments::VaultToken) -> Self {
        let api_models::payments::VaultToken {
            card_cvc,
            card_holder_name,
        } = value;

        Self {
            card_cvc,
            card_holder_name,
        }
    }
}
impl
    From<(
        api_models::payments::Card,
        Option<payment_methods::CoBadgedCardData>,
    )> for Card
{
    fn from(
        (value, co_badged_card_data_optional): (
            api_models::payments::Card,
            Option<payment_methods::CoBadgedCardData>,
        ),
    ) -> Self {
        let api_models::payments::Card {
            card_number,
            card_exp_month,
            card_exp_year,
            card_holder_name,
            card_cvc,
            card_issuer,
            card_network,
            card_type,
            card_issuing_country,
            card_issuing_country_code,
            bank_code,
            nick_name,
        } = value;

        Self {
            card_number,
            card_exp_month,
            card_exp_year,
            card_cvc,
            card_issuer,
            card_network,
            card_type,
            card_issuing_country,
            card_issuing_country_code,
            bank_code,
            nick_name,
            card_holder_name,
            co_badged_card_data: co_badged_card_data_optional,
        }
    }
}

#[cfg(feature = "v2")]
impl
    From<(
        payment_methods::CardDetail,
        Secret<String>,
        Option<Secret<String>>,
    )> for Card
{
    fn from(
        (card_detail, card_cvc, card_holder_name): (
            payment_methods::CardDetail,
            Secret<String>,
            Option<Secret<String>>,
        ),
    ) -> Self {
        Self {
            card_number: card_detail.card_number,
            card_exp_month: card_detail.card_exp_month,
            card_exp_year: card_detail.card_exp_year,
            card_cvc,
            card_issuer: card_detail.card_issuer,
            card_network: card_detail.card_network,
            card_type: card_detail.card_type.map(|val| val.to_string()),
            card_issuing_country: card_detail.card_issuing_country.map(|val| val.to_string()),
            card_issuing_country_code: None,
            bank_code: None,
            nick_name: card_detail.nick_name,
            card_holder_name: card_holder_name.or(card_detail.card_holder_name),
            co_badged_card_data: None,
        }
    }
}

#[cfg(feature = "v2")]
impl From<Card> for payment_methods::CardDetail {
    fn from(card: Card) -> Self {
        Self {
            card_number: card.card_number,
            card_exp_month: card.card_exp_month,
            card_exp_year: card.card_exp_year,
            card_holder_name: card.card_holder_name,
            nick_name: card.nick_name,
            card_issuing_country: None,
            card_network: card.card_network,
            card_issuer: card.card_issuer,
            card_type: None,
            card_cvc: Some(card.card_cvc),
        }
    }
}

#[cfg(feature = "v2")]
impl From<ExternalVaultCard> for payment_methods::ProxyCardDetails {
    fn from(card: ExternalVaultCard) -> Self {
        Self {
            card_number: card.card_number,
            card_exp_month: card.card_exp_month,
            card_exp_year: card.card_exp_year,
            card_holder_name: card.card_holder_name,
            nick_name: card.nick_name,
            card_issuing_country: card.card_issuing_country,
            card_network: card.card_network,
            card_issuer: card.card_issuer,
            card_type: card.card_type,
            card_cvc: Some(card.card_cvc),
            bin_number: card.bin_number,
            last_four: card.last_four,
        }
    }
}

impl From<api_models::payments::CardRedirectData> for CardRedirectData {
    fn from(value: api_models::payments::CardRedirectData) -> Self {
        match value {
            api_models::payments::CardRedirectData::Knet {} => Self::Knet {},
            api_models::payments::CardRedirectData::Benefit {} => Self::Benefit {},
            api_models::payments::CardRedirectData::MomoAtm {} => Self::MomoAtm {},
            api_models::payments::CardRedirectData::CardRedirect {} => Self::CardRedirect {},
        }
    }
}

impl From<CardRedirectData> for api_models::payments::CardRedirectData {
    fn from(value: CardRedirectData) -> Self {
        match value {
            CardRedirectData::Knet {} => Self::Knet {},
            CardRedirectData::Benefit {} => Self::Benefit {},
            CardRedirectData::MomoAtm {} => Self::MomoAtm {},
            CardRedirectData::CardRedirect {} => Self::CardRedirect {},
        }
    }
}

impl From<api_models::payments::WalletData> for WalletData {
    fn from(value: api_models::payments::WalletData) -> Self {
        match value {
            api_models::payments::WalletData::AliPayQr(_) => Self::AliPayQr(Box::new(AliPayQr {})),
            api_models::payments::WalletData::AliPayRedirect(_) => {
                Self::AliPayRedirect(AliPayRedirection {})
            }
            api_models::payments::WalletData::AliPayHkRedirect(_) => {
                Self::AliPayHkRedirect(AliPayHkRedirection {})
            }
            api_models::payments::WalletData::AmazonPay(amazon_pay_data) => {
                Self::AmazonPay(AmazonPayWalletData::from(amazon_pay_data))
            }
            api_models::payments::WalletData::AmazonPayRedirect(_) => {
                Self::AmazonPayRedirect(Box::new(AmazonPayRedirect {}))
            }
            api_models::payments::WalletData::Skrill(_) => Self::Skrill(Box::new(SkrillData {})),
            api_models::payments::WalletData::Paysera(_) => Self::Paysera(Box::new(PayseraData {})),
            api_models::payments::WalletData::MomoRedirect(_) => {
                Self::MomoRedirect(MomoRedirection {})
            }
            api_models::payments::WalletData::KakaoPayRedirect(_) => {
                Self::KakaoPayRedirect(KakaoPayRedirection {})
            }
            api_models::payments::WalletData::GoPayRedirect(_) => {
                Self::GoPayRedirect(GoPayRedirection {})
            }
            api_models::payments::WalletData::GcashRedirect(_) => {
                Self::GcashRedirect(GcashRedirection {})
            }
            api_models::payments::WalletData::ApplePay(apple_pay_data) => {
                Self::ApplePay(ApplePayWalletData::from(apple_pay_data))
            }
            api_models::payments::WalletData::ApplePayRedirect(_) => {
                Self::ApplePayRedirect(Box::new(ApplePayRedirectData {}))
            }
            api_models::payments::WalletData::ApplePayThirdPartySdk(apple_pay_sdk_data) => {
                Self::ApplePayThirdPartySdk(Box::new(ApplePayThirdPartySdkData {
                    token: apple_pay_sdk_data.token,
                }))
            }
            api_models::payments::WalletData::DanaRedirect {} => Self::DanaRedirect {},
            api_models::payments::WalletData::GooglePay(google_pay_data) => {
                Self::GooglePay(GooglePayWalletData::from(google_pay_data))
            }
            api_models::payments::WalletData::GooglePayRedirect(_) => {
                Self::GooglePayRedirect(Box::new(GooglePayRedirectData {}))
            }
            api_models::payments::WalletData::GooglePayThirdPartySdk(google_pay_sdk_data) => {
                Self::GooglePayThirdPartySdk(Box::new(GooglePayThirdPartySdkData {
                    token: google_pay_sdk_data.token,
                }))
            }
            api_models::payments::WalletData::MbWayRedirect(..) => {
                Self::MbWayRedirect(Box::new(MbWayRedirection {}))
            }
            api_models::payments::WalletData::MobilePayRedirect(_) => {
                Self::MobilePayRedirect(Box::new(MobilePayRedirection {}))
            }
            api_models::payments::WalletData::PaypalRedirect(paypal_redirect_data) => {
                Self::PaypalRedirect(PaypalRedirection {
                    email: paypal_redirect_data.email,
                })
            }
            api_models::payments::WalletData::PaypalSdk(paypal_sdk_data) => {
                Self::PaypalSdk(PayPalWalletData {
                    token: paypal_sdk_data.token,
                })
            }
            api_models::payments::WalletData::Paze(paze_data) => {
                Self::Paze(PazeWalletData::from(paze_data))
            }
            api_models::payments::WalletData::SamsungPay(samsung_pay_data) => {
                Self::SamsungPay(Box::new(SamsungPayWalletData::from(samsung_pay_data)))
            }
            api_models::payments::WalletData::TwintRedirect {} => Self::TwintRedirect {},
            api_models::payments::WalletData::VippsRedirect {} => Self::VippsRedirect {},
            api_models::payments::WalletData::TouchNGoRedirect(_) => {
                Self::TouchNGoRedirect(Box::new(TouchNGoRedirection {}))
            }
            api_models::payments::WalletData::WeChatPayRedirect(_) => {
                Self::WeChatPayRedirect(Box::new(WeChatPayRedirection {}))
            }
            api_models::payments::WalletData::WeChatPayQr(_) => {
                Self::WeChatPayQr(Box::new(WeChatPayQr {}))
            }
            api_models::payments::WalletData::CashappQr(_) => {
                Self::CashappQr(Box::new(CashappQr {}))
            }
            api_models::payments::WalletData::SwishQr(_) => Self::SwishQr(SwishQrData {}),
            api_models::payments::WalletData::Mifinity(mifinity_data) => {
                Self::Mifinity(MifinityData {
                    date_of_birth: mifinity_data.date_of_birth,
                    language_preference: mifinity_data.language_preference,
                })
            }
            api_models::payments::WalletData::BluecodeRedirect {} => Self::BluecodeRedirect {},
            api_models::payments::WalletData::RevolutPay(_) => Self::RevolutPay(RevolutPayData {}),
        }
    }
}

impl From<api_models::payments::GooglePayWalletData> for GooglePayWalletData {
    fn from(value: api_models::payments::GooglePayWalletData) -> Self {
        Self {
            pm_type: value.pm_type,
            description: value.description,
            info: GooglePayPaymentMethodInfo {
                card_network: value.info.card_network,
                card_details: value.info.card_details,
                assurance_details: value.info.assurance_details.map(|info| {
                    GooglePayAssuranceDetails {
                        card_holder_authenticated: info.card_holder_authenticated,
                        account_verified: info.account_verified,
                    }
                }),
                card_funding_source: value.info.card_funding_source,
            },
            tokenization_data: value.tokenization_data,
        }
    }
}

impl From<api_models::payments::ApplePayWalletData> for ApplePayWalletData {
    fn from(value: api_models::payments::ApplePayWalletData) -> Self {
        Self {
            payment_data: value.payment_data,
            payment_method: ApplepayPaymentMethod {
                display_name: value.payment_method.display_name,
                network: value.payment_method.network,
                pm_type: value.payment_method.pm_type,
            },
            transaction_identifier: value.transaction_identifier,
        }
    }
}

impl From<api_models::payments::AmazonPayWalletData> for AmazonPayWalletData {
    fn from(value: api_models::payments::AmazonPayWalletData) -> Self {
        Self {
            checkout_session_id: value.checkout_session_id,
        }
    }
}

impl From<api_models::payments::SamsungPayTokenData> for SamsungPayTokenData {
    fn from(samsung_pay_token_data: api_models::payments::SamsungPayTokenData) -> Self {
        Self {
            three_ds_type: samsung_pay_token_data.three_ds_type,
            version: samsung_pay_token_data.version,
            data: samsung_pay_token_data.data,
        }
    }
}

impl From<api_models::payments::PazeWalletData> for PazeWalletData {
    fn from(value: api_models::payments::PazeWalletData) -> Self {
        Self {
            complete_response: value.complete_response,
        }
    }
}

impl From<Box<api_models::payments::SamsungPayWalletData>> for SamsungPayWalletData {
    fn from(value: Box<api_models::payments::SamsungPayWalletData>) -> Self {
        match value.payment_credential {
            api_models::payments::SamsungPayWalletCredentials::SamsungPayWalletDataForApp(
                samsung_pay_app_wallet_data,
            ) => Self {
                payment_credential: SamsungPayWalletCredentials {
                    method: samsung_pay_app_wallet_data.method,
                    recurring_payment: samsung_pay_app_wallet_data.recurring_payment,
                    card_brand: samsung_pay_app_wallet_data.payment_card_brand.into(),
                    dpan_last_four_digits: samsung_pay_app_wallet_data.payment_last4_dpan,
                    card_last_four_digits: samsung_pay_app_wallet_data.payment_last4_fpan,
                    token_data: samsung_pay_app_wallet_data.token_data.into(),
                },
            },
            api_models::payments::SamsungPayWalletCredentials::SamsungPayWalletDataForWeb(
                samsung_pay_web_wallet_data,
            ) => Self {
                payment_credential: SamsungPayWalletCredentials {
                    method: samsung_pay_web_wallet_data.method,
                    recurring_payment: samsung_pay_web_wallet_data.recurring_payment,
                    card_brand: samsung_pay_web_wallet_data.card_brand.into(),
                    dpan_last_four_digits: None,
                    card_last_four_digits: samsung_pay_web_wallet_data.card_last_four_digits,
                    token_data: samsung_pay_web_wallet_data.token_data.into(),
                },
            },
        }
    }
}

impl From<api_models::payments::PayLaterData> for PayLaterData {
    fn from(value: api_models::payments::PayLaterData) -> Self {
        match value {
            api_models::payments::PayLaterData::KlarnaRedirect { .. } => Self::KlarnaRedirect {},
            api_models::payments::PayLaterData::KlarnaSdk { token } => Self::KlarnaSdk { token },
            api_models::payments::PayLaterData::AffirmRedirect {} => Self::AffirmRedirect {},
            api_models::payments::PayLaterData::FlexitiRedirect {} => Self::FlexitiRedirect {},
            api_models::payments::PayLaterData::AfterpayClearpayRedirect { .. } => {
                Self::AfterpayClearpayRedirect {}
            }
            api_models::payments::PayLaterData::PayBrightRedirect {} => Self::PayBrightRedirect {},
            api_models::payments::PayLaterData::WalleyRedirect {} => Self::WalleyRedirect {},
            api_models::payments::PayLaterData::AlmaRedirect {} => Self::AlmaRedirect {},
            api_models::payments::PayLaterData::AtomeRedirect {} => Self::AtomeRedirect {},
            api_models::payments::PayLaterData::BreadpayRedirect {} => Self::BreadpayRedirect {},
            api_models::payments::PayLaterData::PayjustnowRedirect {} => {
                Self::PayjustnowRedirect {}
            }
        }
    }
}

impl From<api_models::payments::BankRedirectData> for BankRedirectData {
    fn from(value: api_models::payments::BankRedirectData) -> Self {
        match value {
            api_models::payments::BankRedirectData::BancontactCard {
                card_number,
                card_exp_month,
                card_exp_year,
                card_holder_name,
                ..
            } => Self::BancontactCard {
                card_number,
                card_exp_month,
                card_exp_year,
                card_holder_name,
            },
            api_models::payments::BankRedirectData::Bizum {} => Self::Bizum {},
            api_models::payments::BankRedirectData::Blik { blik_code } => Self::Blik { blik_code },
            api_models::payments::BankRedirectData::Eps {
                bank_name, country, ..
            } => Self::Eps { bank_name, country },
            api_models::payments::BankRedirectData::Giropay {
                bank_account_bic,
                bank_account_iban,
                country,
                ..
            } => Self::Giropay {
                bank_account_bic,
                bank_account_iban,
                country,
            },
            api_models::payments::BankRedirectData::Ideal { bank_name, .. } => {
                Self::Ideal { bank_name }
            }
            api_models::payments::BankRedirectData::Interac { country, email } => {
                Self::Interac { country, email }
            }
            api_models::payments::BankRedirectData::OnlineBankingCzechRepublic { issuer } => {
                Self::OnlineBankingCzechRepublic { issuer }
            }
            api_models::payments::BankRedirectData::OnlineBankingFinland { email } => {
                Self::OnlineBankingFinland { email }
            }
            api_models::payments::BankRedirectData::OnlineBankingPoland { issuer } => {
                Self::OnlineBankingPoland { issuer }
            }
            api_models::payments::BankRedirectData::OnlineBankingSlovakia { issuer } => {
                Self::OnlineBankingSlovakia { issuer }
            }
            api_models::payments::BankRedirectData::OpenBankingUk {
                country, issuer, ..
            } => Self::OpenBankingUk { country, issuer },
            api_models::payments::BankRedirectData::Przelewy24 { bank_name, .. } => {
                Self::Przelewy24 { bank_name }
            }
            api_models::payments::BankRedirectData::Sofort {
                preferred_language,
                country,
                ..
            } => Self::Sofort {
                country,
                preferred_language,
            },
            api_models::payments::BankRedirectData::Trustly { country } => Self::Trustly {
                country: Some(country),
            },
            api_models::payments::BankRedirectData::OnlineBankingFpx { issuer } => {
                Self::OnlineBankingFpx { issuer }
            }
            api_models::payments::BankRedirectData::OnlineBankingThailand { issuer } => {
                Self::OnlineBankingThailand { issuer }
            }
            api_models::payments::BankRedirectData::LocalBankRedirect { .. } => {
                Self::LocalBankRedirect {}
            }
            api_models::payments::BankRedirectData::Eft { provider } => Self::Eft { provider },
            api_models::payments::BankRedirectData::OpenBanking { .. } => Self::OpenBanking {},
        }
    }
}

impl From<api_models::payments::CryptoData> for CryptoData {
    fn from(value: api_models::payments::CryptoData) -> Self {
        let api_models::payments::CryptoData {
            pay_currency,
            network,
        } = value;
        Self {
            pay_currency,
            network,
        }
    }
}

impl From<CryptoData> for api_models::payments::CryptoData {
    fn from(value: CryptoData) -> Self {
        let CryptoData {
            pay_currency,
            network,
        } = value;
        Self {
            pay_currency,
            network,
        }
    }
}

impl From<api_models::payments::UpiData> for UpiData {
    fn from(value: api_models::payments::UpiData) -> Self {
        match value {
            api_models::payments::UpiData::UpiCollect(upi) => Self::UpiCollect(UpiCollectData {
                vpa_id: upi.vpa_id,
                upi_source: upi.upi_source.map(UpiSource::from),
            }),
            api_models::payments::UpiData::UpiIntent(upi) => Self::UpiIntent(UpiIntentData {
                upi_source: upi.upi_source.map(UpiSource::from),
                app_name: upi.app_name,
            }),
            api_models::payments::UpiData::UpiQr(upi) => Self::UpiQr(UpiQrData {
                upi_source: upi.upi_source.map(UpiSource::from),
            }),
        }
    }
}

impl From<UpiData> for api_models::payments::additional_info::UpiAdditionalData {
    fn from(value: UpiData) -> Self {
        match value {
            UpiData::UpiCollect(upi) => Self::UpiCollect(Box::new(
                payment_additional_types::UpiCollectAdditionalData {
                    vpa_id: upi.vpa_id.map(MaskedUpiVpaId::from),
                    upi_source: upi.upi_source.map(api_models::payments::UpiSource::from),
                },
            )),
            UpiData::UpiIntent(upi) => {
                Self::UpiIntent(Box::new(api_models::payments::UpiIntentData {
                    upi_source: upi.upi_source.map(api_models::payments::UpiSource::from),
                    app_name: upi.app_name,
                }))
            }
            UpiData::UpiQr(upi) => Self::UpiQr(Box::new(api_models::payments::UpiQrData {
                upi_source: upi.upi_source.map(api_models::payments::UpiSource::from),
            })),
        }
    }
}

impl From<api_models::payments::VoucherData> for VoucherData {
    fn from(value: api_models::payments::VoucherData) -> Self {
        match value {
            api_models::payments::VoucherData::Boleto(boleto_data) => {
                Self::Boleto(Box::new(BoletoVoucherData {
                    social_security_number: boleto_data.social_security_number,
                    bank_number: boleto_data.bank_number,
                    document_type: boleto_data.document_type,
                    fine_percentage: boleto_data.fine_percentage,
                    fine_quantity_days: boleto_data.fine_quantity_days,
                    interest_percentage: boleto_data.interest_percentage,
                    write_off_quantity_days: boleto_data.write_off_quantity_days,
                    messages: boleto_data.messages,
                    due_date: boleto_data.due_date,
                }))
            }
            api_models::payments::VoucherData::Alfamart(_) => {
                Self::Alfamart(Box::new(AlfamartVoucherData {}))
            }
            api_models::payments::VoucherData::Indomaret(_) => {
                Self::Indomaret(Box::new(IndomaretVoucherData {}))
            }
            api_models::payments::VoucherData::SevenEleven(_)
            | api_models::payments::VoucherData::Lawson(_)
            | api_models::payments::VoucherData::MiniStop(_)
            | api_models::payments::VoucherData::FamilyMart(_)
            | api_models::payments::VoucherData::Seicomart(_)
            | api_models::payments::VoucherData::PayEasy(_) => {
                Self::SevenEleven(Box::new(JCSVoucherData {}))
            }
            api_models::payments::VoucherData::Efecty => Self::Efecty,
            api_models::payments::VoucherData::PagoEfectivo => Self::PagoEfectivo,
            api_models::payments::VoucherData::RedCompra => Self::RedCompra,
            api_models::payments::VoucherData::RedPagos => Self::RedPagos,
            api_models::payments::VoucherData::Oxxo => Self::Oxxo,
        }
    }
}

impl From<Box<BoletoVoucherData>> for Box<api_models::payments::BoletoVoucherData> {
    fn from(value: Box<BoletoVoucherData>) -> Self {
        Self::new(api_models::payments::BoletoVoucherData {
            social_security_number: value.social_security_number,
            bank_number: value.bank_number,
            document_type: value.document_type,
            fine_percentage: value.fine_percentage,
            fine_quantity_days: value.fine_quantity_days,
            interest_percentage: value.interest_percentage,
            write_off_quantity_days: value.write_off_quantity_days,
            messages: value.messages,
            due_date: value.due_date,
        })
    }
}

impl From<Box<AlfamartVoucherData>> for Box<api_models::payments::AlfamartVoucherData> {
    fn from(_value: Box<AlfamartVoucherData>) -> Self {
        Self::new(api_models::payments::AlfamartVoucherData {
            first_name: None,
            last_name: None,
            email: None,
        })
    }
}

impl From<Box<IndomaretVoucherData>> for Box<api_models::payments::IndomaretVoucherData> {
    fn from(_value: Box<IndomaretVoucherData>) -> Self {
        Self::new(api_models::payments::IndomaretVoucherData {
            first_name: None,
            last_name: None,
            email: None,
        })
    }
}

impl From<Box<JCSVoucherData>> for Box<api_models::payments::JCSVoucherData> {
    fn from(_value: Box<JCSVoucherData>) -> Self {
        Self::new(api_models::payments::JCSVoucherData {
            first_name: None,
            last_name: None,
            email: None,
            phone_number: None,
        })
    }
}

impl From<VoucherData> for api_models::payments::VoucherData {
    fn from(value: VoucherData) -> Self {
        match value {
            VoucherData::Boleto(boleto_data) => Self::Boleto(boleto_data.into()),
            VoucherData::Alfamart(alfa_mart) => Self::Alfamart(alfa_mart.into()),
            VoucherData::Indomaret(info_maret) => Self::Indomaret(info_maret.into()),
            VoucherData::SevenEleven(jcs_data)
            | VoucherData::Lawson(jcs_data)
            | VoucherData::MiniStop(jcs_data)
            | VoucherData::FamilyMart(jcs_data)
            | VoucherData::Seicomart(jcs_data)
            | VoucherData::PayEasy(jcs_data) => Self::SevenEleven(jcs_data.into()),
            VoucherData::Efecty => Self::Efecty,
            VoucherData::PagoEfectivo => Self::PagoEfectivo,
            VoucherData::RedCompra => Self::RedCompra,
            VoucherData::RedPagos => Self::RedPagos,
            VoucherData::Oxxo => Self::Oxxo,
        }
    }
}

impl From<api_models::payments::GiftCardData> for GiftCardData {
    fn from(value: api_models::payments::GiftCardData) -> Self {
        match value {
            api_models::payments::GiftCardData::Givex(details) => Self::Givex(GiftCardDetails {
                number: details.number,
                cvc: details.cvc,
            }),
            api_models::payments::GiftCardData::PaySafeCard {} => Self::PaySafeCard {},
            api_models::payments::GiftCardData::BhnCardNetwork(details) => {
                Self::BhnCardNetwork(BHNGiftCardDetails {
                    account_number: details.account_number,
                    pin: details.pin,
                    cvv2: details.cvv2,
                    expiration_date: details.expiration_date,
                })
            }
        }
    }
}

impl From<GiftCardData> for payment_additional_types::GiftCardAdditionalData {
    fn from(value: GiftCardData) -> Self {
        match value {
            GiftCardData::Givex(details) => Self::Givex(Box::new(
                payment_additional_types::GivexGiftCardAdditionalData {
                    last4: details
                        .number
                        .peek()
                        .chars()
                        .rev()
                        .take(4)
                        .collect::<String>()
                        .chars()
                        .rev()
                        .collect::<String>()
                        .into(),
                },
            )),
            GiftCardData::PaySafeCard {} => Self::PaySafeCard {},
            GiftCardData::BhnCardNetwork(_) => Self::BhnCardNetwork {},
        }
    }
}

impl From<api_models::payments::CardToken> for CardToken {
    fn from(value: api_models::payments::CardToken) -> Self {
        let api_models::payments::CardToken {
            card_holder_name,
            card_cvc,
        } = value;
        Self {
            card_holder_name,
            card_cvc,
        }
    }
}

impl From<CardToken> for payment_additional_types::CardTokenAdditionalData {
    fn from(value: CardToken) -> Self {
        let CardToken {
            card_holder_name, ..
        } = value;
        Self { card_holder_name }
    }
}

impl From<api_models::payments::BankDebitData> for BankDebitData {
    fn from(value: api_models::payments::BankDebitData) -> Self {
        match value {
            api_models::payments::BankDebitData::AchBankDebit {
                account_number,
                routing_number,
                card_holder_name,
                bank_account_holder_name,
                bank_name,
                bank_type,
                bank_holder_type,
                ..
            } => Self::AchBankDebit {
                account_number,
                routing_number,
                card_holder_name,
                bank_account_holder_name,
                bank_name,
                bank_type,
                bank_holder_type,
            },
            api_models::payments::BankDebitData::SepaBankDebit {
                iban,
                bank_account_holder_name,
                ..
            } => Self::SepaBankDebit {
                iban,
                bank_account_holder_name,
            },
            api_models::payments::BankDebitData::SepaGuarenteedBankDebit {
                iban,
                bank_account_holder_name,
                ..
            } => Self::SepaBankDebit {
                iban,
                bank_account_holder_name,
            },
            api_models::payments::BankDebitData::BecsBankDebit {
                account_number,
                bsb_number,
                bank_account_holder_name,
                ..
            } => Self::BecsBankDebit {
                account_number,
                bsb_number,
                bank_account_holder_name,
            },
            api_models::payments::BankDebitData::BacsBankDebit {
                account_number,
                sort_code,
                bank_account_holder_name,
                ..
            } => Self::BacsBankDebit {
                account_number,
                sort_code,
                bank_account_holder_name,
            },
        }
    }
}

impl From<BankDebitData> for api_models::payments::additional_info::BankDebitAdditionalData {
    fn from(value: BankDebitData) -> Self {
        match value {
            BankDebitData::AchBankDebit {
                account_number,
                routing_number,
                bank_name,
                bank_type,
                bank_holder_type,
                card_holder_name,
                bank_account_holder_name,
            } => Self::Ach(Box::new(
                payment_additional_types::AchBankDebitAdditionalData {
                    account_number: MaskedBankAccount::from(account_number),
                    routing_number: MaskedRoutingNumber::from(routing_number),
                    bank_name,
                    bank_type,
                    bank_holder_type,
                    card_holder_name,
                    bank_account_holder_name,
                },
            )),
            BankDebitData::SepaBankDebit {
                iban,
                bank_account_holder_name,
            } => Self::Sepa(Box::new(
                payment_additional_types::SepaBankDebitAdditionalData {
                    iban: MaskedIban::from(iban),
                    bank_account_holder_name,
                },
            )),
            BankDebitData::SepaGuarenteedBankDebit {
                iban,
                bank_account_holder_name,
            } => Self::SepaGuarenteedDebit(Box::new(
                payment_additional_types::SepaBankDebitAdditionalData {
                    iban: MaskedIban::from(iban),
                    bank_account_holder_name,
                },
            )),
            BankDebitData::BecsBankDebit {
                account_number,
                bsb_number,
                bank_account_holder_name,
            } => Self::Becs(Box::new(
                payment_additional_types::BecsBankDebitAdditionalData {
                    account_number: MaskedBankAccount::from(account_number),
                    bsb_number,
                    bank_account_holder_name,
                },
            )),
            BankDebitData::BacsBankDebit {
                account_number,
                sort_code,
                bank_account_holder_name,
            } => Self::Bacs(Box::new(
                payment_additional_types::BacsBankDebitAdditionalData {
                    account_number: MaskedBankAccount::from(account_number),
                    sort_code: MaskedSortCode::from(sort_code),
                    bank_account_holder_name,
                },
            )),
        }
    }
}

impl From<api_models::payments::BankTransferData> for BankTransferData {
    fn from(value: api_models::payments::BankTransferData) -> Self {
        match value {
            api_models::payments::BankTransferData::AchBankTransfer { .. } => {
                Self::AchBankTransfer {}
            }
            api_models::payments::BankTransferData::SepaBankTransfer { .. } => {
                Self::SepaBankTransfer {}
            }
            api_models::payments::BankTransferData::BacsBankTransfer { .. } => {
                Self::BacsBankTransfer {}
            }
            api_models::payments::BankTransferData::MultibancoBankTransfer { .. } => {
                Self::MultibancoBankTransfer {}
            }
            api_models::payments::BankTransferData::PermataBankTransfer { .. } => {
                Self::PermataBankTransfer {}
            }
            api_models::payments::BankTransferData::BcaBankTransfer { .. } => {
                Self::BcaBankTransfer {}
            }
            api_models::payments::BankTransferData::BniVaBankTransfer { .. } => {
                Self::BniVaBankTransfer {}
            }
            api_models::payments::BankTransferData::BriVaBankTransfer { .. } => {
                Self::BriVaBankTransfer {}
            }
            api_models::payments::BankTransferData::CimbVaBankTransfer { .. } => {
                Self::CimbVaBankTransfer {}
            }
            api_models::payments::BankTransferData::DanamonVaBankTransfer { .. } => {
                Self::DanamonVaBankTransfer {}
            }
            api_models::payments::BankTransferData::MandiriVaBankTransfer { .. } => {
                Self::MandiriVaBankTransfer {}
            }
            api_models::payments::BankTransferData::Pix {
                pix_key,
                cpf,
                cnpj,
                source_bank_account_id,
                destination_bank_account_id,
                expiry_date,
            } => Self::Pix {
                pix_key,
                cpf,
                cnpj,
                source_bank_account_id,
                destination_bank_account_id,
                expiry_date,
            },
            api_models::payments::BankTransferData::Pse {} => Self::Pse {},
            api_models::payments::BankTransferData::LocalBankTransfer { bank_code } => {
                Self::LocalBankTransfer { bank_code }
            }
            api_models::payments::BankTransferData::InstantBankTransfer {} => {
                Self::InstantBankTransfer {}
            }
            api_models::payments::BankTransferData::InstantBankTransferFinland {} => {
                Self::InstantBankTransferFinland {}
            }
            api_models::payments::BankTransferData::InstantBankTransferPoland {} => {
                Self::InstantBankTransferPoland {}
            }
            api_models::payments::BankTransferData::IndonesianBankTransfer { bank_name } => {
                Self::IndonesianBankTransfer { bank_name }
            }
        }
    }
}

impl From<BankTransferData> for api_models::payments::additional_info::BankTransferAdditionalData {
    fn from(value: BankTransferData) -> Self {
        match value {
            BankTransferData::AchBankTransfer {} => Self::Ach {},

            BankTransferData::SepaBankTransfer {} => Self::Sepa(Box::new(
                api_models::payments::additional_info::SepaBankTransferPaymentAdditionalData {
                    debitor_iban: None,
                    debitor_bic: None,
                    debitor_name: None,
                    debitor_email: None,
                },
            )),
            BankTransferData::BacsBankTransfer {} => Self::Bacs {},
            BankTransferData::MultibancoBankTransfer {} => Self::Multibanco {},
            BankTransferData::PermataBankTransfer {} => Self::Permata {},
            BankTransferData::BcaBankTransfer {} => Self::Bca {},
            BankTransferData::BniVaBankTransfer {} => Self::BniVa {},
            BankTransferData::BriVaBankTransfer {} => Self::BriVa {},
            BankTransferData::CimbVaBankTransfer {} => Self::CimbVa {},
            BankTransferData::DanamonVaBankTransfer {} => Self::DanamonVa {},
            BankTransferData::MandiriVaBankTransfer {} => Self::MandiriVa {},
            BankTransferData::Pix {
                pix_key,
                cpf,
                cnpj,
                source_bank_account_id,
                destination_bank_account_id,
                expiry_date,
            } => Self::Pix(Box::new(
                api_models::payments::additional_info::PixBankTransferAdditionalData {
                    pix_key: pix_key.map(MaskedBankAccount::from),
                    cpf: cpf.map(MaskedBankAccount::from),
                    cnpj: cnpj.map(MaskedBankAccount::from),
                    source_bank_account_id,
                    destination_bank_account_id,
                    expiry_date,
                },
            )),
            BankTransferData::Pse {} => Self::Pse {},
            BankTransferData::LocalBankTransfer { bank_code } => Self::LocalBankTransfer(Box::new(
                api_models::payments::additional_info::LocalBankTransferAdditionalData {
                    bank_code: bank_code.map(MaskedBankAccount::from),
                },
            )),
            BankTransferData::InstantBankTransfer {} => Self::InstantBankTransfer {},
            BankTransferData::InstantBankTransferFinland {} => Self::InstantBankTransferFinland {},
            BankTransferData::InstantBankTransferPoland {} => Self::InstantBankTransferPoland {},
            BankTransferData::IndonesianBankTransfer { bank_name } => {
                Self::IndonesianBankTransfer { bank_name }
            }
        }
    }
}

impl From<api_models::payments::RealTimePaymentData> for RealTimePaymentData {
    fn from(value: api_models::payments::RealTimePaymentData) -> Self {
        match value {
            api_models::payments::RealTimePaymentData::Fps {} => Self::Fps {},
            api_models::payments::RealTimePaymentData::DuitNow {} => Self::DuitNow {},
            api_models::payments::RealTimePaymentData::PromptPay {} => Self::PromptPay {},
            api_models::payments::RealTimePaymentData::VietQr {} => Self::VietQr {},
            api_models::payments::RealTimePaymentData::Qris {} => Self::VietQr {},
        }
    }
}

impl From<RealTimePaymentData> for api_models::payments::RealTimePaymentData {
    fn from(value: RealTimePaymentData) -> Self {
        match value {
            RealTimePaymentData::Fps {} => Self::Fps {},
            RealTimePaymentData::DuitNow {} => Self::DuitNow {},
            RealTimePaymentData::PromptPay {} => Self::PromptPay {},
            RealTimePaymentData::VietQr {} => Self::VietQr {},
            RealTimePaymentData::Qris {} => Self::Qris {},
        }
    }
}

impl From<api_models::payments::OpenBankingData> for OpenBankingData {
    fn from(value: api_models::payments::OpenBankingData) -> Self {
        match value {
            api_models::payments::OpenBankingData::OpenBankingPIS {} => Self::OpenBankingPIS {},
        }
    }
}

impl From<OpenBankingData> for api_models::payments::OpenBankingData {
    fn from(value: OpenBankingData) -> Self {
        match value {
            OpenBankingData::OpenBankingPIS {} => Self::OpenBankingPIS {},
        }
    }
}

impl From<api_models::payments::MobilePaymentData> for MobilePaymentData {
    fn from(value: api_models::payments::MobilePaymentData) -> Self {
        match value {
            api_models::payments::MobilePaymentData::DirectCarrierBilling {
                msisdn,
                client_uid,
            } => Self::DirectCarrierBilling { msisdn, client_uid },
        }
    }
}

impl From<MobilePaymentData> for api_models::payments::MobilePaymentData {
    fn from(value: MobilePaymentData) -> Self {
        match value {
            MobilePaymentData::DirectCarrierBilling { msisdn, client_uid } => {
                Self::DirectCarrierBilling { msisdn, client_uid }
            }
        }
    }
}

#[cfg(feature = "v1")]
impl From<api_models::payments::NetworkTokenData> for NetworkTokenData {
    fn from(network_token_data: api_models::payments::NetworkTokenData) -> Self {
        Self {
            token_number: network_token_data.network_token,
            token_exp_month: network_token_data.token_exp_month,
            token_exp_year: network_token_data.token_exp_year,
            token_cryptogram: Some(network_token_data.token_cryptogram),
            card_issuer: network_token_data.card_issuer,
            card_network: network_token_data.card_network,
            card_type: network_token_data.card_type,
            card_issuing_country: network_token_data.card_issuing_country,
            bank_code: network_token_data.bank_code,
            nick_name: network_token_data.nick_name,
            eci: network_token_data.eci,
        }
    }
}

#[cfg(feature = "v2")]
impl From<api_models::payments::NetworkTokenData> for NetworkTokenData {
    fn from(network_token_data: api_models::payments::NetworkTokenData) -> Self {
        Self {
            network_token: network_token_data.network_token,
            network_token_exp_month: network_token_data.token_exp_month,
            network_token_exp_year: network_token_data.token_exp_year,
            cryptogram: Some(network_token_data.token_cryptogram),
            card_issuer: network_token_data.card_issuer,
            card_network: network_token_data.card_network,
            card_type: network_token_data
                .card_type
                .and_then(|ct| payment_methods::CardType::from_str(&ct).ok()),
            card_issuing_country: network_token_data
                .card_issuing_country
                .and_then(|country| api_enums::CountryAlpha2::from_str(&country).ok()),
            bank_code: network_token_data.bank_code,
            card_holder_name: network_token_data.card_holder_name,
            nick_name: network_token_data.nick_name,
            eci: network_token_data.eci,
        }
    }
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenizedCardValue1 {
    pub card_number: String,
    pub exp_year: String,
    pub exp_month: String,
    pub nickname: Option<String>,
    pub card_last_four: Option<String>,
    pub card_token: Option<String>,
    pub card_holder_name: Option<Secret<String>>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenizedCardValue2 {
    pub card_security_code: Option<String>,
    pub card_fingerprint: Option<String>,
    pub external_id: Option<String>,
    pub customer_id: Option<id_type::CustomerId>,
    pub payment_method_id: Option<String>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct TokenizedWalletValue1 {
    pub data: WalletData,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct TokenizedWalletValue2 {
    pub customer_id: Option<id_type::CustomerId>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct TokenizedBankTransferValue1 {
    pub data: BankTransferData,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct TokenizedBankTransferValue2 {
    pub customer_id: Option<id_type::CustomerId>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct TokenizedBankRedirectValue1 {
    pub data: BankRedirectData,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct TokenizedBankRedirectValue2 {
    pub customer_id: Option<id_type::CustomerId>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct TokenizedBankDebitValue2 {
    pub customer_id: Option<id_type::CustomerId>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct TokenizedBankDebitValue1 {
    pub data: BankDebitData,
}

pub trait GetPaymentMethodType {
    fn get_payment_method_type(&self) -> api_enums::PaymentMethodType;
}

impl GetPaymentMethodType for CardRedirectData {
    fn get_payment_method_type(&self) -> api_enums::PaymentMethodType {
        match self {
            Self::Knet {} => api_enums::PaymentMethodType::Knet,
            Self::Benefit {} => api_enums::PaymentMethodType::Benefit,
            Self::MomoAtm {} => api_enums::PaymentMethodType::MomoAtm,
            Self::CardRedirect {} => api_enums::PaymentMethodType::CardRedirect,
        }
    }
}

impl GetPaymentMethodType for WalletData {
    fn get_payment_method_type(&self) -> api_enums::PaymentMethodType {
        match self {
            Self::AliPayQr(_) | Self::AliPayRedirect(_) => api_enums::PaymentMethodType::AliPay,
            Self::AliPayHkRedirect(_) => api_enums::PaymentMethodType::AliPayHk,
            Self::AmazonPayRedirect(_) => api_enums::PaymentMethodType::AmazonPay,
            Self::Skrill(_) => api_enums::PaymentMethodType::Skrill,
            Self::Paysera(_) => api_enums::PaymentMethodType::Paysera,
            Self::MomoRedirect(_) => api_enums::PaymentMethodType::Momo,
            Self::KakaoPayRedirect(_) => api_enums::PaymentMethodType::KakaoPay,
            Self::GoPayRedirect(_) => api_enums::PaymentMethodType::GoPay,
            Self::GcashRedirect(_) => api_enums::PaymentMethodType::Gcash,
            Self::AmazonPay(_) => api_enums::PaymentMethodType::AmazonPay,
            Self::ApplePay(_) | Self::ApplePayRedirect(_) | Self::ApplePayThirdPartySdk(_) => {
                api_enums::PaymentMethodType::ApplePay
            }
            Self::DanaRedirect {} => api_enums::PaymentMethodType::Dana,
            Self::GooglePay(_) | Self::GooglePayRedirect(_) | Self::GooglePayThirdPartySdk(_) => {
                api_enums::PaymentMethodType::GooglePay
            }
            Self::BluecodeRedirect {} => api_enums::PaymentMethodType::Bluecode,
            Self::MbWayRedirect(_) => api_enums::PaymentMethodType::MbWay,
            Self::MobilePayRedirect(_) => api_enums::PaymentMethodType::MobilePay,
            Self::PaypalRedirect(_) | Self::PaypalSdk(_) => api_enums::PaymentMethodType::Paypal,
            Self::Paze(_) => api_enums::PaymentMethodType::Paze,
            Self::SamsungPay(_) => api_enums::PaymentMethodType::SamsungPay,
            Self::TwintRedirect {} => api_enums::PaymentMethodType::Twint,
            Self::VippsRedirect {} => api_enums::PaymentMethodType::Vipps,
            Self::TouchNGoRedirect(_) => api_enums::PaymentMethodType::TouchNGo,
            Self::WeChatPayRedirect(_) | Self::WeChatPayQr(_) => {
                api_enums::PaymentMethodType::WeChatPay
            }
            Self::CashappQr(_) => api_enums::PaymentMethodType::Cashapp,
            Self::SwishQr(_) => api_enums::PaymentMethodType::Swish,
            Self::Mifinity(_) => api_enums::PaymentMethodType::Mifinity,
            Self::RevolutPay(_) => api_enums::PaymentMethodType::RevolutPay,
        }
    }
}

impl GetPaymentMethodType for PayLaterData {
    fn get_payment_method_type(&self) -> api_enums::PaymentMethodType {
        match self {
            Self::KlarnaRedirect { .. } => api_enums::PaymentMethodType::Klarna,
            Self::KlarnaSdk { .. } => api_enums::PaymentMethodType::Klarna,
            Self::FlexitiRedirect { .. } => api_enums::PaymentMethodType::Flexiti,
            Self::AffirmRedirect {} => api_enums::PaymentMethodType::Affirm,
            Self::AfterpayClearpayRedirect { .. } => api_enums::PaymentMethodType::AfterpayClearpay,
            Self::PayBrightRedirect {} => api_enums::PaymentMethodType::PayBright,
            Self::WalleyRedirect {} => api_enums::PaymentMethodType::Walley,
            Self::AlmaRedirect {} => api_enums::PaymentMethodType::Alma,
            Self::AtomeRedirect {} => api_enums::PaymentMethodType::Atome,
            Self::BreadpayRedirect {} => api_enums::PaymentMethodType::Breadpay,
            Self::PayjustnowRedirect {} => api_enums::PaymentMethodType::Payjustnow,
        }
    }
}

impl GetPaymentMethodType for BankRedirectData {
    fn get_payment_method_type(&self) -> api_enums::PaymentMethodType {
        match self {
            Self::BancontactCard { .. } => api_enums::PaymentMethodType::BancontactCard,
            Self::Bizum {} => api_enums::PaymentMethodType::Bizum,
            Self::Blik { .. } => api_enums::PaymentMethodType::Blik,
            Self::Eft { .. } => api_enums::PaymentMethodType::Eft,
            Self::Eps { .. } => api_enums::PaymentMethodType::Eps,
            Self::Giropay { .. } => api_enums::PaymentMethodType::Giropay,
            Self::Ideal { .. } => api_enums::PaymentMethodType::Ideal,
            Self::Interac { .. } => api_enums::PaymentMethodType::Interac,
            Self::OnlineBankingCzechRepublic { .. } => {
                api_enums::PaymentMethodType::OnlineBankingCzechRepublic
            }
            Self::OnlineBankingFinland { .. } => api_enums::PaymentMethodType::OnlineBankingFinland,
            Self::OnlineBankingPoland { .. } => api_enums::PaymentMethodType::OnlineBankingPoland,
            Self::OnlineBankingSlovakia { .. } => {
                api_enums::PaymentMethodType::OnlineBankingSlovakia
            }
            Self::OpenBankingUk { .. } => api_enums::PaymentMethodType::OpenBankingUk,
            Self::Przelewy24 { .. } => api_enums::PaymentMethodType::Przelewy24,
            Self::Sofort { .. } => api_enums::PaymentMethodType::Sofort,
            Self::Trustly { .. } => api_enums::PaymentMethodType::Trustly,
            Self::OnlineBankingFpx { .. } => api_enums::PaymentMethodType::OnlineBankingFpx,
            Self::OnlineBankingThailand { .. } => {
                api_enums::PaymentMethodType::OnlineBankingThailand
            }
            Self::LocalBankRedirect { .. } => api_enums::PaymentMethodType::LocalBankRedirect,
            Self::OpenBanking { .. } => api_enums::PaymentMethodType::OpenBanking,
        }
    }
}

impl GetPaymentMethodType for BankDebitData {
    fn get_payment_method_type(&self) -> api_enums::PaymentMethodType {
        match self {
            Self::AchBankDebit { .. } => api_enums::PaymentMethodType::Ach,
            Self::SepaBankDebit { .. } => api_enums::PaymentMethodType::Sepa,
            Self::SepaGuarenteedBankDebit { .. } => {
                api_enums::PaymentMethodType::SepaGuarenteedDebit
            }
            Self::BecsBankDebit { .. } => api_enums::PaymentMethodType::Becs,
            Self::BacsBankDebit { .. } => api_enums::PaymentMethodType::Bacs,
        }
    }
}

impl GetPaymentMethodType for BankTransferData {
    fn get_payment_method_type(&self) -> api_enums::PaymentMethodType {
        match self {
            Self::AchBankTransfer { .. } => api_enums::PaymentMethodType::Ach,
            Self::SepaBankTransfer { .. } => api_enums::PaymentMethodType::Sepa,
            Self::BacsBankTransfer { .. } => api_enums::PaymentMethodType::Bacs,
            Self::MultibancoBankTransfer { .. } => api_enums::PaymentMethodType::Multibanco,
            Self::PermataBankTransfer { .. } => api_enums::PaymentMethodType::PermataBankTransfer,
            Self::BcaBankTransfer { .. } => api_enums::PaymentMethodType::BcaBankTransfer,
            Self::BniVaBankTransfer { .. } => api_enums::PaymentMethodType::BniVa,
            Self::BriVaBankTransfer { .. } => api_enums::PaymentMethodType::BriVa,
            Self::CimbVaBankTransfer { .. } => api_enums::PaymentMethodType::CimbVa,
            Self::DanamonVaBankTransfer { .. } => api_enums::PaymentMethodType::DanamonVa,
            Self::MandiriVaBankTransfer { .. } => api_enums::PaymentMethodType::MandiriVa,
            Self::Pix { .. } => api_enums::PaymentMethodType::Pix,
            Self::Pse {} => api_enums::PaymentMethodType::Pse,
            Self::LocalBankTransfer { .. } => api_enums::PaymentMethodType::LocalBankTransfer,
            Self::InstantBankTransfer {} => api_enums::PaymentMethodType::InstantBankTransfer,
            Self::InstantBankTransferFinland {} => {
                api_enums::PaymentMethodType::InstantBankTransferFinland
            }
            Self::InstantBankTransferPoland {} => {
                api_enums::PaymentMethodType::InstantBankTransferPoland
            }
            Self::IndonesianBankTransfer { .. } => {
                api_enums::PaymentMethodType::IndonesianBankTransfer
            }
        }
    }
}

impl GetPaymentMethodType for CryptoData {
    fn get_payment_method_type(&self) -> api_enums::PaymentMethodType {
        api_enums::PaymentMethodType::CryptoCurrency
    }
}

impl GetPaymentMethodType for RealTimePaymentData {
    fn get_payment_method_type(&self) -> api_enums::PaymentMethodType {
        match self {
            Self::Fps {} => api_enums::PaymentMethodType::Fps,
            Self::DuitNow {} => api_enums::PaymentMethodType::DuitNow,
            Self::PromptPay {} => api_enums::PaymentMethodType::PromptPay,
            Self::VietQr {} => api_enums::PaymentMethodType::VietQr,
            Self::Qris {} => api_enums::PaymentMethodType::Qris {},
        }
    }
}

impl GetPaymentMethodType for UpiData {
    fn get_payment_method_type(&self) -> api_enums::PaymentMethodType {
        match self {
            Self::UpiCollect(_) => api_enums::PaymentMethodType::UpiCollect,
            Self::UpiIntent(_) => api_enums::PaymentMethodType::UpiIntent,
            Self::UpiQr(_) => api_enums::PaymentMethodType::UpiQr,
        }
    }
}
impl GetPaymentMethodType for VoucherData {
    fn get_payment_method_type(&self) -> api_enums::PaymentMethodType {
        match self {
            Self::Boleto(_) => api_enums::PaymentMethodType::Boleto,
            Self::Efecty => api_enums::PaymentMethodType::Efecty,
            Self::PagoEfectivo => api_enums::PaymentMethodType::PagoEfectivo,
            Self::RedCompra => api_enums::PaymentMethodType::RedCompra,
            Self::RedPagos => api_enums::PaymentMethodType::RedPagos,
            Self::Alfamart(_) => api_enums::PaymentMethodType::Alfamart,
            Self::Indomaret(_) => api_enums::PaymentMethodType::Indomaret,
            Self::Oxxo => api_enums::PaymentMethodType::Oxxo,
            Self::SevenEleven(_) => api_enums::PaymentMethodType::SevenEleven,
            Self::Lawson(_) => api_enums::PaymentMethodType::Lawson,
            Self::MiniStop(_) => api_enums::PaymentMethodType::MiniStop,
            Self::FamilyMart(_) => api_enums::PaymentMethodType::FamilyMart,
            Self::Seicomart(_) => api_enums::PaymentMethodType::Seicomart,
            Self::PayEasy(_) => api_enums::PaymentMethodType::PayEasy,
        }
    }
}
impl GetPaymentMethodType for GiftCardData {
    fn get_payment_method_type(&self) -> api_enums::PaymentMethodType {
        match self {
            Self::Givex(_) => api_enums::PaymentMethodType::Givex,
            Self::PaySafeCard {} => api_enums::PaymentMethodType::PaySafeCard,
            Self::BhnCardNetwork(_) => api_enums::PaymentMethodType::BhnCardNetwork,
        }
    }
}

impl GetPaymentMethodType for OpenBankingData {
    fn get_payment_method_type(&self) -> api_enums::PaymentMethodType {
        match self {
            Self::OpenBankingPIS {} => api_enums::PaymentMethodType::OpenBankingPIS,
        }
    }
}

impl GetPaymentMethodType for MobilePaymentData {
    fn get_payment_method_type(&self) -> api_enums::PaymentMethodType {
        match self {
            Self::DirectCarrierBilling { .. } => api_enums::PaymentMethodType::DirectCarrierBilling,
        }
    }
}

impl From<Card> for ExtendedCardInfo {
    fn from(value: Card) -> Self {
        Self {
            card_number: value.card_number,
            card_exp_month: value.card_exp_month,
            card_exp_year: value.card_exp_year,
            card_holder_name: None,
            card_cvc: value.card_cvc,
            card_issuer: value.card_issuer,
            card_network: value.card_network,
            card_type: value.card_type,
            card_issuing_country: value.card_issuing_country,
            bank_code: value.bank_code,
        }
    }
}

pub fn get_applepay_wallet_info(
    item: ApplePayWalletData,
    payment_method_token: Option<PaymentMethodToken>,
) -> payment_methods::PaymentMethodDataWalletInfo {
    let (card_exp_month, card_exp_year) = match payment_method_token {
        Some(PaymentMethodToken::ApplePayDecrypt(token)) => (
            Some(token.application_expiration_month.clone()),
            Some(token.application_expiration_year.clone()),
        ),
        _ => (None, None),
    };
    payment_methods::PaymentMethodDataWalletInfo {
        last4: item
            .payment_method
            .display_name
            .chars()
            .rev()
            .take(4)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect(),
        card_network: item.payment_method.network,
        card_type: Some(item.payment_method.pm_type),
        card_exp_month,
        card_exp_year,
        // To be populated after connector response
        auth_code: None,
    }
}

pub fn get_googlepay_wallet_info(
    item: GooglePayWalletData,
    payment_method_token: Option<PaymentMethodToken>,
) -> payment_methods::PaymentMethodDataWalletInfo {
    let (card_exp_month, card_exp_year) = match payment_method_token {
        Some(PaymentMethodToken::GooglePayDecrypt(token)) => (
            Some(token.card_exp_month.clone()),
            Some(token.card_exp_year.clone()),
        ),
        _ => (None, None),
    };
    payment_methods::PaymentMethodDataWalletInfo {
        last4: item.info.card_details,
        card_network: item.info.card_network,
        card_type: Some(item.pm_type),
        card_exp_month,
        card_exp_year,
        // to be populated after connector response
        auth_code: None,
    }
}

#[derive(Clone, Debug, PartialEq, serde::Deserialize, serde::Serialize)]
pub enum PaymentMethodsData {
    Card(CardDetailsPaymentMethod),
    BankDetails(payment_methods::PaymentMethodDataBankCreds), //PaymentMethodDataBankCreds and its transformations should be moved to the domain models
    WalletDetails(payment_methods::PaymentMethodDataWalletInfo), //PaymentMethodDataWalletInfo and its transformations should be moved to the domain models
    NetworkToken(NetworkTokenDetailsPaymentMethod),
    BankDebit(BankDebitDetailsPaymentMethod),
}

impl PaymentMethodsData {
    #[cfg(feature = "v1")]
    pub fn get_co_badged_card_data(&self) -> Option<payment_methods::CoBadgedCardData> {
        if let Self::Card(card) = self {
            card.co_badged_card_data
                .clone()
                .map(|co_badged_card_data_to_be_saved| co_badged_card_data_to_be_saved.into())
        } else {
            None
        }
    }
    #[cfg(feature = "v2")]
    pub fn get_co_badged_card_data(&self) -> Option<payment_methods::CoBadgedCardData> {
        todo!()
    }

    #[cfg(feature = "v1")]
    pub fn get_additional_payout_method_data(
        &self,
    ) -> Option<payout_method_utils::AdditionalPayoutMethodData> {
        match self {
            Self::Card(card_details) => {
                router_env::logger::info!("Populating AdditionalPayoutMethodData from Card payment method data for recurring payout");
                Some(payout_method_utils::AdditionalPayoutMethodData::Card(
                    Box::new(payout_method_utils::CardAdditionalData {
                        card_issuer: card_details.card_issuer.clone(),
                        card_network: card_details.card_network.clone(),
                        bank_code: None,
                        card_type: card_details.card_type.clone(),
                        card_issuing_country: card_details.issuer_country.clone(),
                        last4: card_details.last4_digits.clone(),
                        card_isin: card_details.card_isin.clone(),
                        card_extended_bin: None,
                        card_exp_month: card_details.expiry_month.clone(),
                        card_exp_year: card_details.expiry_year.clone(),
                        card_holder_name: card_details.card_holder_name.clone(),
                    }),
                ))
            }
            Self::BankDetails(_)
            | Self::WalletDetails(_)
            | Self::NetworkToken(_)
            | Self::BankDebit(_) => None,
        }
    }
    pub fn get_card_details(&self) -> Option<CardDetailsPaymentMethod> {
        if let Self::Card(card) = self {
            Some(card.clone())
        } else {
            None
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct NetworkTokenDetailsPaymentMethod {
    pub last4_digits: Option<String>,
    pub issuer_country: Option<common_enums::CountryAlpha2>,
    pub network_token_expiry_month: Option<Secret<String>>,
    pub network_token_expiry_year: Option<Secret<String>>,
    pub nick_name: Option<Secret<String>>,
    pub card_holder_name: Option<Secret<String>>,
    pub card_isin: Option<String>,
    pub card_issuer: Option<String>,
    pub card_network: Option<api_enums::CardNetwork>,
    pub card_type: Option<String>,
    #[serde(default = "saved_in_locker_default")]
    pub saved_to_locker: bool,
}

fn saved_in_locker_default() -> bool {
    true
}

#[derive(serde::Deserialize, serde::Serialize, Debug, Clone, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum BankDebitDetailsPaymentMethod {
    AchBankDebit {
        masked_account_number: String,
        masked_routing_number: String,
        card_holder_name: Option<Secret<String>>,
        bank_account_holder_name: Option<Secret<String>>,
        bank_name: Option<common_enums::BankNames>,
        bank_type: Option<common_enums::BankType>,
        bank_holder_type: Option<common_enums::BankHolderType>,
    },
}

#[cfg(feature = "v1")]
#[derive(Clone, Debug, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct CardDetailsPaymentMethod {
    pub last4_digits: Option<String>,
    pub issuer_country: Option<String>,
    pub issuer_country_code: Option<String>,
    pub expiry_month: Option<Secret<String>>,
    pub expiry_year: Option<Secret<String>>,
    pub nick_name: Option<Secret<String>>,
    pub card_holder_name: Option<Secret<String>>,
    pub card_isin: Option<String>,
    pub card_issuer: Option<String>,
    pub card_network: Option<api_enums::CardNetwork>,
    pub card_type: Option<String>,
    #[serde(default = "saved_in_locker_default")]
    pub saved_to_locker: bool,
    pub co_badged_card_data: Option<payment_methods::CoBadgedCardDataToBeSaved>,
}

#[cfg(feature = "v2")]
#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct CardDetailsPaymentMethod {
    pub last4_digits: Option<String>,
    pub issuer_country: Option<String>,
    pub expiry_month: Option<Secret<String>>,
    pub expiry_year: Option<Secret<String>>,
    pub nick_name: Option<Secret<String>>,
    pub card_holder_name: Option<Secret<String>>,
    pub card_isin: Option<String>,
    pub card_issuer: Option<String>,
    pub card_network: Option<api_enums::CardNetwork>,
    pub card_type: Option<String>,
    #[serde(default = "saved_in_locker_default")]
    pub saved_to_locker: bool,
}

#[cfg(feature = "v2")]
pub struct CardNumberWithStoredDetails {
    pub card_number: cards::CardNumber,
    pub card_cvc: Option<Secret<String>>,
    pub card_details: CardDetailsPaymentMethod,
}

#[cfg(feature = "v2")]
impl CardNumberWithStoredDetails {
    pub fn new(
        card_number: cards::CardNumber,
        card_cvc: Option<Secret<String>>,
        card_details: CardDetailsPaymentMethod,
    ) -> Self {
        Self {
            card_number,
            card_cvc,
            card_details,
        }
    }
}

#[cfg(feature = "v2")]
impl TryFrom<CardNumberWithStoredDetails> for payment_methods::CardDetail {
    type Error = error_stack::Report<common_utils::errors::ValidationError>;

    fn try_from(testing: CardNumberWithStoredDetails) -> Result<Self, Self::Error> {
        let card_number = testing.card_number;
        let card_cvc = testing.card_cvc;
        let item = testing.card_details;
        Ok(Self {
            card_number,
            card_exp_month: item
                .expiry_month
                .get_required_value("expiry_month")?
                .clone(),
            card_exp_year: item.expiry_year.get_required_value("expiry_year")?.clone(),
            card_holder_name: item.card_holder_name,
            nick_name: item.nick_name,
            card_issuing_country: item
                .issuer_country
                .as_ref()
                .map(|country| api_enums::CountryAlpha2::from_str(country))
                .transpose()
                .ok()
                .flatten(),
            card_network: item.card_network,
            card_issuer: item.card_issuer,
            card_type: item
                .card_type
                .and_then(|card_type| payment_methods::CardType::from_str(&card_type).ok()),
            card_cvc,
        })
    }
}

#[cfg(feature = "v2")]
impl CardDetailsPaymentMethod {
    pub fn to_card_details_from_locker(self) -> payment_methods::CardDetailFromLocker {
        payment_methods::CardDetailFromLocker {
            card_number: None,
            card_holder_name: self.card_holder_name.clone(),
            card_issuer: self.card_issuer.clone(),
            card_network: self.card_network.clone(),
            card_type: self.card_type.clone(),
            issuer_country: self.clone().get_issuer_country_alpha2(),
            last4_digits: self.last4_digits,
            expiry_month: self.expiry_month,
            expiry_year: self.expiry_year,
            card_fingerprint: None,
            nick_name: self.nick_name,
            card_isin: self.card_isin,
            saved_to_locker: self.saved_to_locker,
        }
    }

    pub fn get_issuer_country_alpha2(self) -> Option<common_enums::CountryAlpha2> {
        self.issuer_country
            .as_ref()
            .map(|c| api_enums::CountryAlpha2::from_str(c))
            .transpose()
            .ok()
            .flatten()
    }
}

#[cfg(feature = "v1")]
impl From<payment_methods::CardDetail> for CardDetailsPaymentMethod {
    fn from(item: payment_methods::CardDetail) -> Self {
        Self {
            issuer_country: item.card_issuing_country.map(|c| c.to_string()),
            issuer_country_code: item
                .card_issuing_country_code
                .map(|country_code| country_code.to_string()),
            last4_digits: Some(item.card_number.get_last4()),
            expiry_month: Some(item.card_exp_month),
            expiry_year: Some(item.card_exp_year),
            card_holder_name: item.card_holder_name,
            nick_name: item.nick_name,
            card_isin: None,
            card_issuer: item.card_issuer,
            card_network: item.card_network,
            card_type: item.card_type.map(|card| card.to_string()),
            saved_to_locker: true,
            co_badged_card_data: None,
        }
    }
}

#[cfg(feature = "v1")]
impl
    From<(
        payment_methods::CardDetailFromLocker,
        Option<&payment_methods::CoBadgedCardData>,
    )> for CardDetailsPaymentMethod
{
    fn from(
        (item, co_badged_card_data): (
            payment_methods::CardDetailFromLocker,
            Option<&payment_methods::CoBadgedCardData>,
        ),
    ) -> Self {
        Self {
            issuer_country: item.issuer_country,
            last4_digits: item.last4_digits,
            expiry_month: item.expiry_month,
            issuer_country_code: item.issuer_country_code,
            expiry_year: item.expiry_year,
            nick_name: item.nick_name,
            card_holder_name: item.card_holder_name,
            card_isin: item.card_isin,
            card_issuer: item.card_issuer,
            card_network: item.card_network,
            card_type: item.card_type,
            saved_to_locker: item.saved_to_locker,
            co_badged_card_data: co_badged_card_data
                .map(payment_methods::CoBadgedCardDataToBeSaved::from),
        }
    }
}

#[cfg(feature = "v2")]
impl From<payment_methods::CardDetailsPaymentMethod> for CardDetailsPaymentMethod {
    fn from(item: payment_methods::CardDetailsPaymentMethod) -> Self {
        Self {
            issuer_country: item.issuer_country,
            last4_digits: item.last4_digits,
            expiry_month: item.expiry_month,
            expiry_year: item.expiry_year,
            nick_name: item.nick_name,
            card_holder_name: item.card_holder_name,
            card_isin: item.card_isin,
            card_issuer: item.card_issuer,
            card_network: item.card_network,
            card_type: item.card_type,
            saved_to_locker: item.saved_to_locker,
        }
    }
}

#[cfg(feature = "v2")]
impl From<payment_methods::CardDetail> for CardDetailsPaymentMethod {
    fn from(item: payment_methods::CardDetail) -> Self {
        Self {
            issuer_country: item.card_issuing_country.map(|c| c.to_string()),
            last4_digits: Some(item.card_number.get_last4()),
            expiry_month: Some(item.card_exp_month),
            expiry_year: Some(item.card_exp_year),
            card_holder_name: item.card_holder_name,
            nick_name: item.nick_name,
            card_isin: None,
            card_issuer: item.card_issuer,
            card_network: item.card_network,
            card_type: item.card_type.map(|card| card.to_string()),
            saved_to_locker: true,
        }
    }
}

impl From<NetworkTokenDetails> for NetworkTokenDetailsPaymentMethod {
    fn from(item: NetworkTokenDetails) -> Self {
        Self {
            issuer_country: item.card_issuing_country,
            last4_digits: Some(item.network_token.get_last4()),
            network_token_expiry_month: Some(item.network_token_exp_month),
            network_token_expiry_year: Some(item.network_token_exp_year),
            card_holder_name: item.card_holder_name,
            nick_name: item.nick_name,
            card_isin: None,
            card_issuer: item.card_issuer,
            card_network: item.card_network,
            card_type: item.card_type.map(|card| card.to_string()),
            saved_to_locker: true,
        }
    }
}

#[cfg(feature = "v2")]
#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct SingleUsePaymentMethodToken {
    pub token: Secret<String>,
    pub merchant_connector_id: id_type::MerchantConnectorAccountId,
}

#[cfg(feature = "v2")]
impl SingleUsePaymentMethodToken {
    pub fn get_single_use_token_from_payment_method_token(
        token: Secret<String>,
        mca_id: id_type::MerchantConnectorAccountId,
    ) -> Self {
        Self {
            token,
            merchant_connector_id: mca_id,
        }
    }
}

impl From<NetworkTokenDetailsPaymentMethod> for payment_methods::NetworkTokenDetailsPaymentMethod {
    fn from(item: NetworkTokenDetailsPaymentMethod) -> Self {
        Self {
            last4_digits: item.last4_digits,
            issuer_country: item.issuer_country,
            network_token_expiry_month: item.network_token_expiry_month,
            network_token_expiry_year: item.network_token_expiry_year,
            nick_name: item.nick_name,
            card_holder_name: item.card_holder_name,
            card_isin: item.card_isin,
            card_issuer: item.card_issuer,
            card_network: item.card_network,
            card_type: item.card_type,
            saved_to_locker: item.saved_to_locker,
        }
    }
}

#[cfg(feature = "v2")]
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct SingleUseTokenKey(String);

#[cfg(feature = "v2")]
impl SingleUseTokenKey {
    pub fn store_key(payment_method_id: &id_type::GlobalPaymentMethodId) -> Self {
        let new_token = format!("single_use_token_{}", payment_method_id.get_string_repr());
        Self(new_token)
    }

    pub fn get_store_key(&self) -> &str {
        &self.0
    }
}

#[cfg(feature = "v1")]
impl From<Card> for payment_methods::CardDetail {
    fn from(card_data: Card) -> Self {
        Self {
            card_number: card_data.card_number.clone(),
            card_exp_month: card_data.card_exp_month.clone(),
            card_exp_year: card_data.card_exp_year.clone(),
            card_cvc: None, // DO NOT POPULATE CVC FOR ADDITIONAL PAYMENT METHOD DATA
            card_holder_name: None,
            nick_name: None,
            card_issuing_country: None,
            card_issuing_country_code: None,
            card_network: card_data.card_network.clone(),
            card_issuer: None,
            card_type: None,
        }
    }
}

#[cfg(feature = "v1")]
impl From<NetworkTokenData> for payment_methods::CardDetail {
    fn from(network_token_data: NetworkTokenData) -> Self {
        Self {
            card_number: network_token_data.token_number.into(),
            card_exp_month: network_token_data.token_exp_month.clone(),
            card_exp_year: network_token_data.token_exp_year.clone(),
            card_cvc: None,
            card_holder_name: None,
            nick_name: None,
            card_issuing_country: None,
            card_issuing_country_code: None,
            card_network: network_token_data.card_network.clone(),
            card_issuer: None,
            card_type: None,
        }
    }
}

#[cfg(feature = "v1")]
impl From<NetworkTokenData> for AdditionalNetworkTokenInfo {
    fn from(network_token_data: NetworkTokenData) -> Self {
        Self {
            card_issuer: network_token_data.card_issuer.to_owned(),
            card_network: network_token_data.card_network.to_owned(),
            card_type: network_token_data.card_type.to_owned(),
            card_issuing_country: network_token_data.card_issuing_country.to_owned(),
            bank_code: network_token_data.bank_code.to_owned(),
            token_exp_month: Some(network_token_data.token_exp_month.clone()),
            token_exp_year: Some(network_token_data.token_exp_year.clone()),
            card_holder_name: None,
            last4: Some(network_token_data.token_number.get_last4().clone()),
            token_isin: Some(network_token_data.token_number.get_card_isin().clone()),
        }
    }
}

#[cfg(feature = "v2")]
impl From<NetworkTokenData> for AdditionalNetworkTokenInfo {
    fn from(network_token_data: NetworkTokenData) -> Self {
        Self {
            card_issuer: network_token_data.card_issuer.to_owned(),
            card_network: network_token_data.card_network.to_owned(),
            card_type: network_token_data.card_type.map(|ct| ct.to_string()),
            card_issuing_country: network_token_data
                .card_issuing_country
                .map(|country| country.to_string()),
            bank_code: network_token_data.bank_code.to_owned(),
            token_exp_month: Some(network_token_data.network_token_exp_month.clone()),
            token_exp_year: Some(network_token_data.network_token_exp_year.clone()),
            card_holder_name: network_token_data.card_holder_name.to_owned(),
            last4: Some(network_token_data.network_token.get_last4().clone()),
            token_isin: Some(network_token_data.network_token.get_card_isin().clone()),
        }
    }
}

impl From<NetworkTokenDetailsForNetworkTransactionId> for AdditionalNetworkTokenInfo {
    fn from(network_token_with_ntid: NetworkTokenDetailsForNetworkTransactionId) -> Self {
        Self {
            card_issuer: network_token_with_ntid.card_issuer.to_owned(),
            card_network: network_token_with_ntid.card_network.to_owned(),
            card_type: network_token_with_ntid.card_type.to_owned(),
            card_issuing_country: network_token_with_ntid.card_issuing_country.to_owned(),
            bank_code: network_token_with_ntid.bank_code.to_owned(),
            token_exp_month: Some(network_token_with_ntid.token_exp_month.clone()),
            token_exp_year: Some(network_token_with_ntid.token_exp_year.clone()),
            card_holder_name: network_token_with_ntid.card_holder_name.clone(),
            last4: Some(network_token_with_ntid.network_token.get_last4().clone()),
            token_isin: Some(
                network_token_with_ntid
                    .network_token
                    .get_card_isin()
                    .clone(),
            ),
        }
    }
}

impl From<DecryptedWalletTokenDetailsForNetworkTransactionId> for AdditionalNetworkTokenInfo {
    fn from(network_token_with_ntid: DecryptedWalletTokenDetailsForNetworkTransactionId) -> Self {
        Self {
            card_issuer: None,
            card_network: None,
            card_type: None,
            card_issuing_country: None,
            bank_code: None,
            token_exp_month: Some(network_token_with_ntid.token_exp_month.clone()),
            token_exp_year: Some(network_token_with_ntid.token_exp_year.clone()),
            card_holder_name: network_token_with_ntid.card_holder_name.clone(),
            last4: Some(network_token_with_ntid.decrypted_token.get_last4().clone()),
            token_isin: Some(
                network_token_with_ntid
                    .decrypted_token
                    .get_card_isin()
                    .clone(),
            ),
        }
    }
}

#[cfg(feature = "v1")]
impl
    From<(
        payment_methods::CardDetail,
        Option<&CardToken>,
        Option<payment_methods::CoBadgedCardData>,
    )> for Card
{
    fn from(
        value: (
            payment_methods::CardDetail,
            Option<&CardToken>,
            Option<payment_methods::CoBadgedCardData>,
        ),
    ) -> Self {
        let (
            payment_methods::CardDetail {
                card_number,
                card_exp_month,
                card_exp_year,
                card_holder_name,
                nick_name,
                card_network,
                card_issuer,
                card_issuing_country,
                card_issuing_country_code,
                card_type,
                ..
            },
            card_token_data,
            co_badged_card_data,
        ) = value;

        // The card_holder_name from locker retrieved card is considered if it is a non-empty string or else card_holder_name is picked
        let name_on_card = if let Some(name) = card_holder_name.clone() {
            if name.clone().expose().is_empty() {
                card_token_data
                    .and_then(|token_data| token_data.card_holder_name.clone())
                    .or(Some(name))
            } else {
                card_holder_name
            }
        } else {
            card_token_data.and_then(|token_data| token_data.card_holder_name.clone())
        };

        Self {
            card_number,
            card_exp_month,
            card_exp_year,
            card_holder_name: name_on_card,
            card_cvc: card_token_data
                .cloned()
                .unwrap_or_default()
                .card_cvc
                .unwrap_or_default(),
            card_issuer,
            card_network,
            card_type,
            card_issuing_country,
            card_issuing_country_code,
            bank_code: None,
            nick_name,
            co_badged_card_data,
        }
    }
}

#[cfg(feature = "v1")]
impl
    TryFrom<(
        cards::CardNumber,
        Option<&CardToken>,
        Option<payment_methods::CoBadgedCardData>,
        CardDetailsPaymentMethod,
    )> for Card
{
    type Error = error_stack::Report<common_utils::errors::ValidationError>;
    fn try_from(
        value: (
            cards::CardNumber,
            Option<&CardToken>,
            Option<payment_methods::CoBadgedCardData>,
            CardDetailsPaymentMethod,
        ),
    ) -> Result<Self, Self::Error> {
        let (card_number, card_token_data, co_badged_card_data, card_details) = value;

        // The card_holder_name from locker retrieved card is considered if it is a non-empty string or else card_holder_name is picked
        let name_on_card = if let Some(name) = card_details.card_holder_name.clone() {
            if name.clone().expose().is_empty() {
                card_token_data
                    .and_then(|token_data| token_data.card_holder_name.clone())
                    .or(Some(name))
            } else {
                Some(name)
            }
        } else {
            card_token_data.and_then(|token_data| token_data.card_holder_name.clone())
        };

        Ok(Self {
            card_number,
            card_exp_month: card_details
                .expiry_month
                .get_required_value("expiry_month")?
                .clone(),
            card_exp_year: card_details
                .expiry_year
                .get_required_value("expiry_year")?
                .clone(),
            card_holder_name: name_on_card,
            card_cvc: card_token_data
                .cloned()
                .unwrap_or_default()
                .card_cvc
                .unwrap_or_default(),
            card_issuer: card_details.card_issuer,
            card_network: card_details.card_network,
            card_type: card_details.card_type,
            card_issuing_country: card_details.issuer_country,
            card_issuing_country_code: card_details.issuer_country_code,
            bank_code: None,
            nick_name: card_details.nick_name,
            co_badged_card_data,
        })
    }
}

impl From<mandates::RecurringDetails> for RecurringDetails {
    fn from(value: mandates::RecurringDetails) -> Self {
        match value {
            mandates::RecurringDetails::MandateId(mandate_id) => Self::MandateId(mandate_id),
            mandates::RecurringDetails::PaymentMethodId(payment_method_id) => {
                Self::PaymentMethodId(payment_method_id)
            }
            mandates::RecurringDetails::ProcessorPaymentToken(token) => {
                Self::ProcessorPaymentToken(ProcessorPaymentToken {
                    processor_payment_token: token.processor_payment_token,
                    merchant_connector_id: token.merchant_connector_id,
                })
            }
            mandates::RecurringDetails::NetworkTransactionIdAndCardDetails(
                network_transaction_id_and_card_details,
            ) => Self::NetworkTransactionIdAndCardDetails(Box::new(
                (*network_transaction_id_and_card_details).into(),
            )),
            mandates::RecurringDetails::NetworkTransactionIdAndNetworkTokenDetails(
                network_transaction_id_and_network_token_details,
            ) => Self::NetworkTransactionIdAndNetworkTokenDetails(Box::new(
                (*network_transaction_id_and_network_token_details).into(),
            )),
            mandates::RecurringDetails::NetworkTransactionIdAndDecryptedWalletTokenDetails(
                network_transaction_id_and_decrypted_wallet_token_details,
            ) => Self::NetworkTransactionIdAndDecryptedWalletTokenDetails(Box::new(
                *network_transaction_id_and_decrypted_wallet_token_details,
            )),
            mandates::RecurringDetails::CardWithLimitedData(card_with_limited_data) => {
                Self::CardWithLimitedData(Box::new((*card_with_limited_data).into()))
            }
        }
    }
}

impl From<mandates::NetworkTransactionIdAndCardDetails> for NetworkTransactionIdAndCardDetails {
    fn from(value: mandates::NetworkTransactionIdAndCardDetails) -> Self {
        Self {
            card_number: value.card_number,
            card_exp_month: value.card_exp_month,
            card_exp_year: value.card_exp_year,
            network_transaction_id: value.network_transaction_id,
            card_holder_name: value.card_holder_name,
            card_issuer: value.card_issuer,
            card_network: value.card_network,
            card_type: value.card_type,
            card_issuing_country: value.card_issuing_country,
            card_issuing_country_code: value.card_issuing_country_code,
            bank_code: value.bank_code,
            nick_name: value.nick_name,
        }
    }
}

impl From<mandates::NetworkTransactionIdAndNetworkTokenDetails>
    for NetworkTransactionIdAndNetworkTokenDetails
{
    fn from(value: mandates::NetworkTransactionIdAndNetworkTokenDetails) -> Self {
        Self {
            network_token: value.network_token,
            token_exp_month: value.token_exp_month,
            token_exp_year: value.token_exp_year,
            network_transaction_id: value.network_transaction_id,
            card_holder_name: value.card_holder_name,
            card_issuer: value.card_issuer,
            card_network: value.card_network,
            card_type: value.card_type,
            card_issuing_country: value.card_issuing_country,
            bank_code: value.bank_code,
            nick_name: value.nick_name,
            eci: value.eci,
        }
    }
}

impl From<mandates::CardWithLimitedData> for CardWithLimitedData {
    fn from(card_with_limited_data: mandates::CardWithLimitedData) -> Self {
        Self {
            card_number: card_with_limited_data.card_number,
            card_exp_month: card_with_limited_data.card_exp_month,
            card_exp_year: card_with_limited_data.card_exp_year,
            card_holder_name: card_with_limited_data.card_holder_name,
            eci: card_with_limited_data.eci,
        }
    }
}

impl RecurringDetails {
    pub fn get_mandate_reference_id_and_payment_method_data_for_proxy_flow(
        &self,
    ) -> Option<(api_models::payments::MandateReferenceId, PaymentMethodData)> {
        match self.clone() {
            Self::NetworkTransactionIdAndCardDetails(network_transaction_id_and_card_details) => {
                Some(CardDetailsForNetworkTransactionId::get_nti_and_card_details_for_mit_flow(*network_transaction_id_and_card_details))
            }
            Self::NetworkTransactionIdAndNetworkTokenDetails(network_transaction_id_and_network_token_details) => {
                Some(NetworkTokenDetailsForNetworkTransactionId::get_nti_and_network_token_details_for_mit_flow(*network_transaction_id_and_network_token_details))
            }
            Self::CardWithLimitedData(card_with_limited_data) => {
                Some(CardWithLimitedDetails::get_card_details_for_mit_flow(*card_with_limited_data))
            }
            Self::NetworkTransactionIdAndDecryptedWalletTokenDetails(network_transaction_id_and_decrypted_wallet_token_details) => {
                Some(DecryptedWalletTokenDetailsForNetworkTransactionId::get_nti_and_decrypted_wallet_token_details_for_mit_flow(*network_transaction_id_and_decrypted_wallet_token_details))
            }
            Self::PaymentMethodId(_)
            | Self::MandateId(_)
            | Self::ProcessorPaymentToken(_) => None,
        }
    }
}
