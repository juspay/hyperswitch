use common_types::payments as common_payments_types;
use hyperswitch_masking::Secret;
use serde::{Deserialize, Serialize};
use smithy::SmithyModel;
use time::PrimitiveDateTime;
use utoipa::ToSchema;

use crate::enums as api_enums;

#[derive(Default, Debug, Deserialize, Serialize)]
pub struct MandateId {
    pub mandate_id: String,
}

#[derive(Default, Debug, Deserialize, Serialize, ToSchema, SmithyModel)]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub struct MandateRevokedResponse {
    /// The identifier for mandate
    #[smithy(value_type = "String")]
    pub mandate_id: String,
    /// The status for mandates
    #[schema(value_type = MandateStatus)]
    #[smithy(value_type = "MandateStatus")]
    pub status: api_enums::MandateStatus,
    /// If there was an error while calling the connectors the code is received here
    #[schema(example = "E0001")]
    #[smithy(value_type = "Option<String>")]
    pub error_code: Option<String>,
    /// If there was an error while calling the connector the error message is received here
    #[schema(example = "Failed while verifying the card")]
    #[smithy(value_type = "Option<String>")]
    pub error_message: Option<String>,
}

#[derive(Default, Debug, Deserialize, Serialize, ToSchema, Clone, SmithyModel)]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub struct MandateResponse {
    /// The identifier for mandate
    #[smithy(value_type = "String")]
    pub mandate_id: String,
    /// The status for mandates
    #[schema(value_type = MandateStatus)]
    #[smithy(value_type = "MandateStatus")]
    pub status: api_enums::MandateStatus,
    /// The identifier for payment method
    #[smithy(value_type = "String")]
    pub payment_method_id: String,
    /// The payment method
    #[smithy(value_type = "String")]
    pub payment_method: String,
    /// The payment method type
    #[smithy(value_type = "Option<String>")]
    pub payment_method_type: Option<String>,
    /// The card details for mandate
    #[smithy(value_type = "Option<MandateCardDetails>")]
    pub card: Option<MandateCardDetails>,
    /// Details about the customer’s acceptance
    #[schema(value_type = Option<CustomerAcceptance>)]
    #[smithy(value_type = "Option<CustomerAcceptance>")]
    pub customer_acceptance: Option<common_payments_types::CustomerAcceptance>,
}

#[derive(Default, Debug, Deserialize, Serialize, ToSchema, Clone, SmithyModel)]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub struct MandateCardDetails {
    /// The last 4 digits of card
    #[smithy(value_type = "Option<String>")]
    pub last4_digits: Option<String>,
    /// The expiry month of card
    #[schema(value_type = Option<String>)]
    #[smithy(value_type = "Option<String>")]
    pub card_exp_month: Option<Secret<String>>,
    /// The expiry year of card
    #[schema(value_type = Option<String>)]
    #[smithy(value_type = "Option<String>")]
    pub card_exp_year: Option<Secret<String>>,
    /// The card holder name
    #[schema(value_type = Option<String>)]
    #[smithy(value_type = "Option<String>")]
    pub card_holder_name: Option<Secret<String>>,
    /// The token from card locker
    #[schema(value_type = Option<String>)]
    #[smithy(value_type = "Option<String>")]
    pub card_token: Option<Secret<String>>,
    /// The card scheme network for the particular card
    #[smithy(value_type = "Option<String>")]
    pub scheme: Option<String>,
    /// The country code in in which the card was issued
    #[smithy(value_type = "Option<String>")]
    pub issuer_country: Option<String>,
    #[schema(value_type = Option<String>)]
    #[smithy(value_type = "Option<String>")]
    /// A unique identifier alias to identify a particular card
    pub card_fingerprint: Option<Secret<String>>,
    /// The first 6 digits of card
    #[smithy(value_type = "Option<String>")]
    pub card_isin: Option<String>,
    /// The bank that issued the card
    #[smithy(value_type = "Option<String>")]
    pub card_issuer: Option<String>,
    /// The network that facilitates payment card transactions
    #[schema(value_type = Option<CardNetwork>)]
    #[smithy(value_type = "Option<CardNetwork>")]
    pub card_network: Option<api_enums::CardNetwork>,
    /// The type of the payment card
    #[smithy(value_type = "Option<String>")]
    pub card_type: Option<String>,
    /// The nick_name of the card holder
    #[schema(value_type = Option<String>)]
    #[smithy(value_type = "Option<String>")]
    pub nick_name: Option<Secret<String>>,
}

#[derive(Clone, Debug, Deserialize, ToSchema, Serialize, SmithyModel)]
#[serde(deny_unknown_fields)]
#[smithy(namespace = "com.hyperswitch.smithy.types", mixin = true)]
pub struct MandateListConstraints {
    /// limit on the number of objects to return
    #[smithy(value_type = "Option<i64>", http_query = "limit")]
    pub limit: Option<i64>,
    /// offset on the number of objects to return
    #[smithy(value_type = "Option<i64>", http_query = "offset")]
    pub offset: Option<i64>,
    /// status of the mandate
    #[smithy(value_type = "Option<MandateStatus>", http_query = "mandate_status")]
    pub mandate_status: Option<api_enums::MandateStatus>,
    /// connector linked to mandate
    #[smithy(value_type = "Option<String>", http_query = "connector")]
    pub connector: Option<String>,
    /// The time at which mandate is created
    #[schema(example = "2022-09-10T10:11:12Z")]
    #[smithy(value_type = "Option<String>", http_query = "created_time")]
    pub created_time: Option<PrimitiveDateTime>,
    /// Time less than the mandate created time
    #[schema(example = "2022-09-10T10:11:12Z")]
    #[serde(rename = "created_time.lt")]
    #[smithy(value_type = "Option<String>", http_query = "created_time.lt")]
    pub created_time_lt: Option<PrimitiveDateTime>,
    /// Time greater than the mandate created time
    #[schema(example = "2022-09-10T10:11:12Z")]
    #[serde(rename = "created_time.gt")]
    #[smithy(value_type = "Option<String>", http_query = "created_time.gt")]
    pub created_time_gt: Option<PrimitiveDateTime>,
    /// Time less than or equals to the mandate created time
    #[schema(example = "2022-09-10T10:11:12Z")]
    #[serde(rename = "created_time.lte")]
    #[smithy(value_type = "Option<String>", http_query = "created_time.lte")]
    pub created_time_lte: Option<PrimitiveDateTime>,
    /// Time greater than or equals to the mandate created time
    #[schema(example = "2022-09-10T10:11:12Z")]
    #[serde(rename = "created_time.gte")]
    #[smithy(value_type = "Option<String>", http_query = "created_time.gte")]
    pub created_time_gte: Option<PrimitiveDateTime>,
}

/// Details required for recurring payment
#[derive(
    Debug, Clone, serde::Serialize, serde::Deserialize, ToSchema, PartialEq, Eq, SmithyModel,
)]
#[serde(tag = "type", content = "data", rename_all = "snake_case")]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub enum RecurringDetails {
    #[smithy(value_type = "String")]
    MandateId(String),
    #[smithy(value_type = "String")]
    PaymentMethodId(String),
    #[smithy(value_type = "ProcessorPaymentToken")]
    ProcessorPaymentToken(ProcessorPaymentToken),

    /// Network transaction ID and Card Details for MIT payments when payment_method_data
    /// is not stored in the application
    #[smithy(value_type = "NetworkTransactionIdAndCardDetails")]
    NetworkTransactionIdAndCardDetails(Box<NetworkTransactionIdAndCardDetails>),

    /// Network transaction ID and Network Token Details for MIT payments when payment_method_data
    /// is not stored in the application
    #[smithy(value_type = "NetworkTransactionIdAndNetworkTokenDetails")]
    NetworkTransactionIdAndNetworkTokenDetails(Box<NetworkTransactionIdAndNetworkTokenDetails>),

    /// Network transaction ID and Wallet Token details for MIT payments when payment_method_data
    /// is not stored in the application
    /// Applicable for wallet tokens such as Apple Pay and Google Pay.
    #[smithy(value_type = "NetworkTransactionIdAndDecryptedWalletTokenDetails")]
    #[schema(value_type = NetworkTransactionIdAndDecryptedWalletTokenDetails)]
    NetworkTransactionIdAndDecryptedWalletTokenDetails(
        Box<common_payments_types::NetworkTransactionIdAndDecryptedWalletTokenDetails>,
    ),

    /// Network transaction ID and external vault card details for MIT payments where the card
    /// is held in an external vault and referenced by a vault alias rather than by PAN.
    #[smithy(value_type = "NetworkTransactionIdAndVaultCardDetails")]
    NetworkTransactionIdAndVaultCardDetails(Box<NetworkTransactionIdAndVaultCardDetails>),

    /// Card with Limited Data to do MIT payment
    /// Can only be used if enabled for Merchant
    /// Allows doing MIT with only Card data (no reference id)
    #[smithy(value_type = "CardWithLimitedData")]
    CardWithLimitedData(Box<CardWithLimitedData>),
}

/// Processor payment token for MIT payments where payment_method_data is not available
#[derive(
    Debug, Clone, serde::Serialize, serde::Deserialize, ToSchema, PartialEq, Eq, SmithyModel,
)]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub struct ProcessorPaymentToken {
    #[smithy(value_type = "String")]
    pub processor_payment_token: String,
    #[schema(value_type = Option<String>)]
    #[smithy(value_type = "Option<String>")]
    pub merchant_connector_id: Option<common_utils::id_type::MerchantConnectorAccountId>,
}

#[derive(
    Debug, Clone, serde::Serialize, serde::Deserialize, ToSchema, PartialEq, Eq, SmithyModel,
)]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub struct NetworkTransactionIdAndCardDetails {
    /// The card number
    #[schema(value_type = String, example = "4242424242424242")]
    #[smithy(value_type = "String")]
    pub card_number: cards::CardNumber,

    /// The card's expiry month
    #[schema(value_type = String, example = "24")]
    #[smithy(value_type = "String")]
    pub card_exp_month: Secret<String>,

    /// The card's expiry year
    #[schema(value_type = String, example = "24")]
    #[smithy(value_type = "String")]
    pub card_exp_year: Secret<String>,

    /// The card holder's name
    #[schema(value_type = String, example = "John Test")]
    #[smithy(value_type = "Option<String>")]
    pub card_holder_name: Option<Secret<String>>,

    /// The name of the issuer of card
    #[schema(example = "chase")]
    #[smithy(value_type = "Option<String>")]
    pub card_issuer: Option<String>,

    /// The card network for the card
    #[schema(value_type = Option<CardNetwork>, example = "Visa")]
    #[smithy(value_type = "Option<CardNetwork>")]
    pub card_network: Option<api_enums::CardNetwork>,

    #[schema(example = "CREDIT")]
    #[smithy(value_type = "Option<String>")]
    pub card_type: Option<String>,

    /// The product the card is issued under, e.g. `CLASSIC` or `ELECTRON`
    #[schema(example = "CLASSIC")]
    #[smithy(value_type = "Option<String>")]
    pub card_subtype: Option<String>,

    /// The segment the card is issued to
    #[schema(value_type = Option<CardSegmentType>, example = "consumer")]
    #[smithy(value_type = "Option<CardSegmentType>")]
    pub card_segment_type: Option<api_enums::CardSegmentType>,

    /// How the card is funded. More granular than `card_type`, which collapses
    /// deferred debit and charge cards.
    #[schema(value_type = Option<FundingSource>, example = "CREDIT")]
    #[smithy(value_type = "Option<FundingSource>")]
    pub funding_source: Option<api_enums::FundingSource>,

    #[schema(example = "INDIA")]
    #[smithy(value_type = "Option<String>")]
    pub card_issuing_country: Option<String>,

    #[schema(example = "IN")]
    #[smithy(value_type = "Option<String>")]
    pub card_issuing_country_code: Option<String>,

    #[schema(example = "JP_AMEX")]
    #[smithy(value_type = "Option<String>")]
    pub bank_code: Option<String>,

    /// The card holder's nick name
    #[schema(value_type = Option<String>, example = "John Test")]
    #[smithy(value_type = "Option<String>")]
    pub nick_name: Option<Secret<String>>,

    /// The network transaction ID provided by the card network during a CIT (Customer Initiated Transaction),
    /// when `setup_future_usage` is set to `off_session`.
    #[schema(value_type = String)]
    #[smithy(value_type = "String")]
    pub network_transaction_id: Secret<String>,

    /// The Mastercard Transaction Link Identifier (TLID) provided by the card network during a CIT (Customer Initiated Transaction),
    /// when `setup_future_usage` is set to `off_session`.
    #[schema(value_type = Option<String>)]
    #[smithy(value_type = "Option<String>")]
    pub transaction_link_id: Option<String>,
}

#[derive(
    Debug, Clone, serde::Serialize, serde::Deserialize, ToSchema, PartialEq, Eq, SmithyModel,
)]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub struct CardWithLimitedData {
    /// The card number
    #[schema(value_type = String, example = "4242424242424242")]
    #[smithy(value_type = "String")]
    pub card_number: cards::CardNumber,

    /// The card's expiry month
    #[schema(value_type = Option<String>, example = "24")]
    #[smithy(value_type = "Option<String>")]
    pub card_exp_month: Option<Secret<String>>,

    /// The card's expiry year
    #[schema(value_type = Option<String>, example = "24")]
    #[smithy(value_type = "Option<String>")]
    pub card_exp_year: Option<Secret<String>>,

    /// The card holder's name
    #[schema(value_type = Option<String>, example = "John Test")]
    #[smithy(value_type = "Option<String>")]
    pub card_holder_name: Option<Secret<String>>,

    /// The ECI(Electronic Commerce Indicator) value for this authentication.
    #[schema(value_type = Option<String>)]
    #[smithy(value_type = "Option<String>")]
    pub eci: Option<String>,

    /// The network transaction ID provided by the card network during a CIT (Customer Initiated Transaction),
    /// when `setup_future_usage` is set to `off_session`.
    #[schema(value_type = Option<String>)]
    #[smithy(value_type = "Option<String>")]
    pub network_transaction_id: Option<Secret<String>>,

    /// The Mastercard Transaction Link Identifier (TLID) provided by the card network during a CIT (Customer Initiated Transaction),
    /// when `setup_future_usage` is set to `off_session`.
    #[schema(value_type = Option<String>)]
    #[smithy(value_type = "Option<String>")]
    pub transaction_link_id: Option<String>,
}

#[derive(
    Debug, Clone, serde::Serialize, serde::Deserialize, ToSchema, PartialEq, Eq, SmithyModel,
)]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub struct NetworkTransactionIdAndNetworkTokenDetails {
    /// The Network Token
    #[schema(value_type = String, example = "4604000460040787")]
    #[smithy(value_type = "String")]
    pub network_token: cards::NetworkToken,

    /// The token's expiry month
    #[schema(value_type = String, example = "05")]
    #[smithy(value_type = "String")]
    pub token_exp_month: Secret<String>,

    /// The token's expiry year
    #[schema(value_type = String, example = "24")]
    #[smithy(value_type = "String")]
    pub token_exp_year: Secret<String>,

    /// The card network for the card
    #[schema(value_type = Option<CardNetwork>, example = "Visa")]
    #[smithy(value_type = "Option<CardNetwork>")]
    pub card_network: Option<api_enums::CardNetwork>,

    /// The type of the card such as Credit, Debit
    #[schema(example = "CREDIT")]
    #[smithy(value_type = "Option<String>")]
    pub card_type: Option<String>,

    /// The country in which the card was issued
    #[schema(example = "INDIA")]
    #[smithy(value_type = "Option<String>")]
    pub card_issuing_country: Option<String>,

    /// The bank code of the bank that issued the card
    #[schema(example = "JP_AMEX")]
    #[smithy(value_type = "Option<String>")]
    pub bank_code: Option<String>,

    /// The card holder's name
    #[schema(value_type = String, example = "John Test")]
    #[smithy(value_type = "Option<String>")]
    pub card_holder_name: Option<Secret<String>>,

    /// The name of the issuer of card
    #[schema(example = "chase")]
    #[smithy(value_type = "Option<String>")]
    pub card_issuer: Option<String>,

    /// The card holder's nick name
    #[schema(value_type = Option<String>, example = "John Test")]
    #[smithy(value_type = "Option<String>")]
    pub nick_name: Option<Secret<String>>,

    /// The ECI(Electronic Commerce Indicator) value for this authentication.
    #[schema(value_type = Option<String>)]
    #[smithy(value_type = "Option<String>")]
    pub eci: Option<String>,

    /// The network transaction ID provided by the card network during a Customer Initiated Transaction (CIT)
    /// when `setup_future_usage` is set to `off_session`.
    #[schema(value_type = String)]
    #[smithy(value_type = "String")]
    pub network_transaction_id: Secret<String>,

    /// The Mastercard Transaction Link Identifier (TLID) provided by the card network during a CIT (Customer Initiated Transaction),
    /// when `setup_future_usage` is set to `off_session`.
    #[schema(value_type = Option<String>)]
    #[smithy(value_type = "Option<String>")]
    pub transaction_link_id: Option<String>,
}

/// Card details held in an external vault and referenced by vault aliases rather than by PAN.
///
/// `card_number` is the vault's alias for the card, not the card number itself, so this
/// payload stays non-PCI and is authorized through the external vault proxy.
#[derive(
    Debug, Clone, serde::Serialize, serde::Deserialize, ToSchema, PartialEq, Eq, SmithyModel,
)]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub struct VaultCardData {
    /// The vault alias which refers to the card number
    #[schema(value_type = String, example = "token_card_number")]
    #[smithy(value_type = "String")]
    pub card_number: Secret<String>,

    /// The card's expiry month
    #[schema(value_type = String, example = "24")]
    #[smithy(value_type = "String")]
    pub card_exp_month: Secret<String>,

    /// The card's expiry year
    #[schema(value_type = String, example = "24")]
    #[smithy(value_type = "String")]
    pub card_exp_year: Secret<String>,

    /// The card holder's name
    #[schema(value_type = Option<String>, example = "John Test")]
    #[smithy(value_type = "Option<String>")]
    pub card_holder_name: Option<Secret<String>>,

    /// The name of the issuer of card
    #[schema(example = "chase")]
    #[smithy(value_type = "Option<String>")]
    pub card_issuer: Option<String>,

    /// The card network for the card
    #[schema(value_type = Option<CardNetwork>, example = "Visa")]
    #[smithy(value_type = "Option<CardNetwork>")]
    pub card_network: Option<api_enums::CardNetwork>,

    /// The type of the card such as Credit, Debit
    #[schema(example = "CREDIT")]
    #[smithy(value_type = "Option<String>")]
    pub card_type: Option<String>,

    /// The country in which the card was issued
    #[schema(example = "INDIA")]
    #[smithy(value_type = "Option<String>")]
    pub card_issuing_country: Option<String>,

    /// The bank code of the bank that issued the card
    #[schema(example = "JP_AMEX")]
    #[smithy(value_type = "Option<String>")]
    pub bank_code: Option<String>,

    /// The card holder's nick name
    #[schema(value_type = Option<String>, example = "John Test")]
    #[smithy(value_type = "Option<String>")]
    pub nick_name: Option<Secret<String>>,

    /// The first six digits of the card number
    #[schema(value_type = Option<String>, example = "424242")]
    #[smithy(value_type = "Option<String>")]
    pub bin_number: Option<String>,

    /// The last four digits of the card number
    #[schema(value_type = Option<String>, example = "4242")]
    #[smithy(value_type = "Option<String>")]
    pub last_four: Option<String>,
}

/// Network transaction ID and external vault card details for MIT payments where the card is
/// held in an external vault. Authorized through the external vault proxy.
#[derive(
    Debug, Clone, serde::Serialize, serde::Deserialize, ToSchema, PartialEq, Eq, SmithyModel,
)]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub struct NetworkTransactionIdAndVaultCardDetails {
    /// The external vault card details
    #[smithy(value_type = "VaultCardData")]
    pub vault_card_data: VaultCardData,

    /// The network transaction ID provided by the card network during a Customer Initiated
    /// Transaction (CIT) when `setup_future_usage` is set to `off_session`.
    #[schema(value_type = String)]
    #[smithy(value_type = "String")]
    pub network_transaction_id: Secret<String>,

    /// The Mastercard Transaction Link Identifier (TLID) provided by the card network during a CIT
    /// (Customer Initiated Transaction), when `setup_future_usage` is set to `off_session`.
    #[schema(value_type = Option<String>)]
    #[smithy(value_type = "Option<String>")]
    pub transaction_link_id: Option<String>,
}

impl RecurringDetails {
    pub fn is_network_transaction_id_and_card_details_flow(self) -> bool {
        matches!(self, Self::NetworkTransactionIdAndCardDetails(_))
    }

    pub fn is_network_transaction_id_and_network_token_details_flow(self) -> bool {
        matches!(self, Self::NetworkTransactionIdAndNetworkTokenDetails(_))
    }

    pub fn is_network_transaction_id_and_decrypted_wallet_token_details_flow(self) -> bool {
        matches!(
            self,
            Self::NetworkTransactionIdAndDecryptedWalletTokenDetails(_)
        )
    }

    pub fn is_network_transaction_id_and_vault_card_details_flow(self) -> bool {
        matches!(self, Self::NetworkTransactionIdAndVaultCardDetails(_))
    }

    pub fn is_card_limited_details_flow(self) -> bool {
        matches!(self, Self::CardWithLimitedData(_))
    }
}

#[cfg(test)]
mod vault_card_recurring_details_tests {
    use super::*;

    fn vault_card_data() -> VaultCardData {
        VaultCardData {
            card_number: Secret::new("tok_4242424242424242".to_string()),
            card_exp_month: Secret::new("03".to_string()),
            card_exp_year: Secret::new("30".to_string()),
            card_holder_name: Some(Secret::new("John Test".to_string())),
            card_issuer: Some("chase".to_string()),
            card_network: Some(api_enums::CardNetwork::Visa),
            card_type: Some("CREDIT".to_string()),
            card_issuing_country: Some("INDIA".to_string()),
            bank_code: Some("JP_AMEX".to_string()),
            nick_name: Some(Secret::new("my card".to_string())),
            bin_number: Some("424242".to_string()),
            last_four: Some("4242".to_string()),
        }
    }

    fn vault_card_recurring_details() -> RecurringDetails {
        RecurringDetails::NetworkTransactionIdAndVaultCardDetails(Box::new(
            NetworkTransactionIdAndVaultCardDetails {
                vault_card_data: vault_card_data(),
                network_transaction_id: Secret::new("MCC12345678".to_string()),
                transaction_link_id: Some("TLID-99".to_string()),
            },
        ))
    }

    #[test]
    fn vault_card_details_round_trip_through_serde() {
        let recurring_details = vault_card_recurring_details();

        let serialized = serde_json::to_string(&recurring_details)
            .expect("vault card recurring details should serialize");
        let deserialized: RecurringDetails =
            serde_json::from_str(&serialized).expect("vault card recurring details should decode");

        assert_eq!(recurring_details, deserialized);
    }

    /// The externally tagged representation is part of the API contract, so pin the tag the
    /// merchant has to send rather than only asserting a round trip.
    #[test]
    fn vault_card_details_use_the_documented_type_tag() {
        let serialized = serde_json::to_value(vault_card_recurring_details())
            .expect("vault card recurring details should serialize");

        assert_eq!(
            serialized["type"],
            serde_json::json!("network_transaction_id_and_vault_card_details")
        );
        assert_eq!(
            serialized["data"]["network_transaction_id"],
            serde_json::json!("MCC12345678")
        );
        assert_eq!(
            serialized["data"]["vault_card_data"]["card_number"],
            serde_json::json!("tok_4242424242424242")
        );
    }

    /// `transaction_link_id` is Mastercard-only, so the payload has to decode without it.
    #[test]
    fn vault_card_details_decode_without_transaction_link_id() {
        let payload = serde_json::json!({
            "type": "network_transaction_id_and_vault_card_details",
            "data": {
                "vault_card_data": {
                    "card_number": "tok_4242424242424242",
                    "card_exp_month": "03",
                    "card_exp_year": "30"
                },
                "network_transaction_id": "MCC12345678"
            }
        });

        let decoded: RecurringDetails =
            serde_json::from_value(payload).expect("minimal vault card payload should decode");

        match decoded {
            RecurringDetails::NetworkTransactionIdAndVaultCardDetails(details) => {
                assert!(details.transaction_link_id.is_none());
                assert!(details.vault_card_data.card_network.is_none());
            }
            other => panic!("expected vault card details, got {other:?}"),
        }
    }

    #[test]
    fn vault_card_details_are_recognised_as_their_own_flow() {
        assert!(
            vault_card_recurring_details().is_network_transaction_id_and_vault_card_details_flow()
        );
    }

    /// The vault card flow must not be mistaken for the raw-card or network-token NTI flows:
    /// those route to the plain proxy core, this one routes to the external vault proxy.
    #[test]
    fn vault_card_details_are_not_confused_with_other_nti_flows() {
        let recurring_details = vault_card_recurring_details();

        assert!(!recurring_details
            .clone()
            .is_network_transaction_id_and_card_details_flow());
        assert!(!recurring_details
            .clone()
            .is_network_transaction_id_and_network_token_details_flow());
        assert!(!recurring_details
            .clone()
            .is_network_transaction_id_and_decrypted_wallet_token_details_flow());
        assert!(!recurring_details.is_card_limited_details_flow());
    }

    #[test]
    fn other_flows_are_not_reported_as_vault_card_details() {
        assert!(!RecurringDetails::MandateId("mandate_1".to_string())
            .is_network_transaction_id_and_vault_card_details_flow());
        assert!(!RecurringDetails::PaymentMethodId("pm_1".to_string())
            .is_network_transaction_id_and_vault_card_details_flow());
    }
}
