use common_utils::{
    new_type::{
        MaskedBankAccount, MaskedBic, MaskedCardNumber, MaskedEmail, MaskedIban, MaskedPhoneNumber,
        MaskedRoutingNumber, MaskedValue,
    },
    pii::Email,
    transformers::ForeignFrom,
};
use hyperswitch_masking::Secret;
use utoipa::ToSchema;

use crate::payments::AddressDetails;

#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema)]
#[serde(tag = "bank_account_type", rename_all = "snake_case")]
pub enum RecipientBankAccount {
    /// An International Bank Account Number.
    Iban {
        /// The recipient's IBAN.
        #[schema(value_type = String, example = "GB29NWBK70361331946864")]
        iban: Secret<String>,
    },
    /// A bank account number paired with the routing number of the holding bank.
    RoutingNumber {
        /// The recipient's bank account number.
        #[schema(value_type = String, example = "000123456789")]
        account_number: Secret<String>,
        /// The routing number of the recipient's bank.
        #[schema(value_type = String, example = "110000000")]
        routing_number: Secret<String>,
    },
    /// A bank account number paired with the BIC of the holding bank.
    Bic {
        /// The recipient's bank account number.
        #[schema(value_type = String, example = "09875432")]
        account_number: Secret<String>,
        /// The Bank Identification Code of the recipient's bank.
        #[schema(value_type = String, example = "HBUKGB4B")]
        bic: Secret<String>,
    },
    /// A bare bank account number, where the scheme does not require a bank identifier.
    AccountNumber {
        /// The recipient's bank account number.
        #[schema(value_type = String, example = "000123456789")]
        account_number: Secret<String>,
    },
    /// A truncated primary account number, made up of the first six and last four digits.
    TruncatedPan {
        /// The first six digits of the recipient's PAN.
        #[schema(value_type = String, max_length = 6, example = "411111")]
        card_isin: Secret<String>,
        /// The last four digits of the recipient's PAN.
        #[schema(value_type = String, max_length = 4, example = "1111")]
        last4: Secret<String>,
    },
}

/// The account that receives the funds in an account funded transaction.
#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum RecipientAccount {
    /// Funds are credited to a bank account.
    BankAccount(RecipientBankAccount),
    /// Funds are credited to a card.
    Card {
        /// The recipient's card number.
        #[schema(value_type = String, example = "4111111111111111")]
        card_number: cards::CardNumber,
    },
    /// Funds are credited to a wallet held for the recipient.
    Wallet {
        /// The identifier of the recipient's wallet.
        #[schema(value_type = String, example = "wallet_8891")]
        wallet_id: Secret<String>,
    },
    /// The recipient's account is identified by their email address.
    Email {
        /// The email address that identifies the recipient's account.
        #[schema(value_type = String, example = "jane.doe@example.com")]
        email: Email,
    },
    /// The recipient's account is identified by their phone number.
    Phone {
        /// The phone number that identifies the recipient's account.
        #[schema(value_type = String, example = "9123456789")]
        phone_number: Secret<String>,
    },
    /// The recipient's account is identified by a social network handle.
    SocialNetwork {
        /// The social network identifier of the recipient.
        #[schema(value_type = String, example = "jane.doe")]
        social_network_id: Secret<String>,
    },
}

/// Details of the party receiving the funds in an account funded transaction.
#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema)]
pub struct RecipientDetails {
    /// The account the funds are credited to.
    #[schema(value_type = Option<RecipientAccount>)]
    pub account: Option<RecipientAccount>,

    /// The recipient's phone number.
    #[schema(value_type = Option<String>, max_length = 20, example = "9123456789")]
    pub phone_number: Option<Secret<String>>,

    /// The recipient's tax identifier. Some connectors require this for recipients in Brazil
    /// (CPF or CNPJ) and Argentina.
    #[schema(value_type = Option<String>, max_length = 25, example = "162.152.541-42")]
    pub tax_id: Option<Secret<String>>,

    /// The recipient's address.
    #[schema(value_type = Option<AddressDetails>)]
    pub address: Option<AddressDetails>,
}

/// A partially masked view of `RecipientBankAccount`, safe to return in API responses.
#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema)]
#[serde(tag = "bank_account_type", rename_all = "snake_case")]
pub enum MaskedRecipientBankAccount {
    /// A partially masked International Bank Account Number.
    Iban {
        /// The recipient's partially masked IBAN.
        #[schema(value_type = String, example = "GB29N************46864")]
        iban: MaskedIban,
    },
    /// A partially masked account number and routing number.
    RoutingNumber {
        /// The recipient's partially masked bank account number.
        #[schema(value_type = String, example = "0001****6789")]
        account_number: MaskedBankAccount,
        /// The partially masked routing number of the recipient's bank.
        #[schema(value_type = String, example = "110***000")]
        routing_number: MaskedRoutingNumber,
    },
    /// A partially masked account number and BIC.
    Bic {
        /// The recipient's partially masked bank account number.
        #[schema(value_type = String, example = "0987****5432")]
        account_number: MaskedBankAccount,
        /// The partially masked Bank Identification Code of the recipient's bank.
        #[schema(value_type = String, example = "HBU***4B")]
        bic: MaskedBic,
    },
    /// A partially masked bare bank account number.
    AccountNumber {
        /// The recipient's partially masked bank account number.
        #[schema(value_type = String, example = "0001****6789")]
        account_number: MaskedBankAccount,
    },
    /// A truncated primary account number. This is already the industry standard truncated form,
    /// so it is returned as supplied.
    TruncatedPan {
        /// The first six digits of the recipient's PAN.
        #[schema(value_type = String, example = "411111")]
        card_isin: Secret<String>,
        /// The last four digits of the recipient's PAN.
        #[schema(value_type = String, example = "1111")]
        last4: Secret<String>,
    },
}

/// A partially masked view of `RecipientAccount`, safe to return in API responses.
#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum MaskedRecipientAccount {
    /// A partially masked bank account.
    BankAccount(MaskedRecipientBankAccount),
    /// A partially masked card.
    Card {
        /// The recipient's partially masked card number.
        #[schema(value_type = String, example = "411111******1111")]
        card_number: MaskedCardNumber,
    },
    /// A partially masked wallet.
    Wallet {
        /// The partially masked identifier of the recipient's wallet.
        #[schema(value_type = String, example = "wall****8891")]
        wallet_id: MaskedValue,
    },
    /// A masked email address.
    Email {
        /// The email address that identifies the recipient's account.
        #[schema(value_type = String, example = "ja******@example.com")]
        email: MaskedEmail,
    },
    /// A partially masked phone number.
    Phone {
        /// The phone number that identifies the recipient's account.
        #[schema(value_type = String, example = "9123****6789")]
        phone_number: MaskedPhoneNumber,
    },
    /// A partially masked social network handle.
    SocialNetwork {
        /// The social network identifier of the recipient.
        #[schema(value_type = String, example = "jane****.doe")]
        social_network_id: MaskedValue,
    },
}

/// A partially masked view of `RecipientDetails`, safe to return in API responses.
#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema)]
pub struct MaskedRecipientDetails {
    /// The partially masked account the funds are credited to.
    #[schema(value_type = Option<MaskedRecipientAccount>)]
    pub account: Option<MaskedRecipientAccount>,

    /// The recipient's partially masked phone number.
    #[schema(value_type = Option<String>, example = "9123****6789")]
    pub phone_number: Option<MaskedPhoneNumber>,

    /// The recipient's partially masked tax identifier.
    #[schema(value_type = Option<String>, example = "162.****41-42")]
    pub tax_id: Option<MaskedValue>,

    /// The recipient's address.
    #[schema(value_type = Option<AddressDetails>)]
    pub address: Option<AddressDetails>,
}

impl From<RecipientBankAccount> for MaskedRecipientBankAccount {
    fn from(bank_account: RecipientBankAccount) -> Self {
        match bank_account {
            RecipientBankAccount::Iban { iban } => Self::Iban {
                iban: MaskedIban::from(iban),
            },
            RecipientBankAccount::RoutingNumber {
                account_number,
                routing_number,
            } => Self::RoutingNumber {
                account_number: MaskedBankAccount::from(account_number),
                routing_number: MaskedRoutingNumber::from(routing_number),
            },
            RecipientBankAccount::Bic {
                account_number,
                bic,
            } => Self::Bic {
                account_number: MaskedBankAccount::from(account_number),
                bic: MaskedBic::from(bic),
            },
            RecipientBankAccount::AccountNumber { account_number } => Self::AccountNumber {
                account_number: MaskedBankAccount::from(account_number),
            },
            // A truncated PAN is already the industry standard truncated form, so it is returned
            // as supplied.
            RecipientBankAccount::TruncatedPan { card_isin, last4 } => {
                Self::TruncatedPan { card_isin, last4 }
            }
        }
    }
}

impl From<RecipientAccount> for MaskedRecipientAccount {
    fn from(account: RecipientAccount) -> Self {
        match account {
            RecipientAccount::BankAccount(bank_account) => {
                Self::BankAccount(MaskedRecipientBankAccount::from(bank_account))
            }
            RecipientAccount::Card { card_number } => Self::Card {
                card_number: MaskedCardNumber::from(Secret::new(card_number.get_card_no())),
            },
            RecipientAccount::Wallet { wallet_id } => Self::Wallet {
                wallet_id: MaskedValue::from(wallet_id),
            },
            RecipientAccount::Email { email } => Self::Email {
                email: MaskedEmail::foreign_from(email),
            },
            RecipientAccount::Phone { phone_number } => Self::Phone {
                phone_number: MaskedPhoneNumber::from(phone_number),
            },
            RecipientAccount::SocialNetwork { social_network_id } => Self::SocialNetwork {
                social_network_id: MaskedValue::from(social_network_id),
            },
        }
    }
}

impl From<RecipientDetails> for MaskedRecipientDetails {
    fn from(recipient_details: RecipientDetails) -> Self {
        Self {
            account: recipient_details.account.map(MaskedRecipientAccount::from),
            phone_number: recipient_details.phone_number.map(MaskedPhoneNumber::from),
            tax_id: recipient_details.tax_id.map(MaskedValue::from),
            address: recipient_details.address,
        }
    }
}
