use serde::{Deserialize, Serialize};

use super::requests;
#[derive(Debug, Serialize, Deserialize)]
pub struct GlobalpayPaymentsResponse {
    /// A unique identifier for the merchant account set by Global Payments.
    pub account_id: Option<String>,
    /// A meaningful label for the merchant account set by Global Payments.
    pub account_name: Option<String>,
    /// Information about the Action executed.
    pub action: Option<Action>,
    /// The amount to transfer between Payer and Merchant for a SALE or a REFUND. It is always
    /// represented in the lowest denomiation of the related currency.
    pub amount: Option<String>,
    /// Indicates if the merchant would accept an authorization for an amount less than the
    /// requested amount. This is available for CP channel
    /// only where the balance not authorized can be processed again using a different card.
    pub authorization_mode: Option<requests::AuthorizationMode>,
    /// A Global Payments created reference that uniquely identifies the batch.
    pub batch_id: Option<String>,
    /// Indicates whether the transaction is to be captured automatically, later or later using
    /// more than 1 partial capture.
    pub capture_mode: Option<requests::CaptureMode>,
    /// Describes whether the transaction was processed in a face to face(CP) scenario or a
    /// Customer Not Present (CNP) scenario.
    pub channel: Option<requests::Channel>,
    /// The country in ISO-3166-1(alpha-2 code) format.
    pub country: Option<String>,
    /// The currency of the amount in ISO-4217(alpha-3)
    pub currency: Option<String>,
    /// Information relating to a currency conversion.
    pub currency_conversion: Option<requests::CurrencyConversion>,
    /// A unique identifier generated by Global Payments to identify the transaction.
    pub id: String,
    /// A unique identifier for the merchant set by Global Payments.
    pub merchant_id: Option<String>,
    /// A meaningful label for the merchant set by Global Payments.
    pub merchant_name: Option<String>,
    pub payment_method: Option<PaymentMethod>,
    /// Merchant defined field to reference the transaction.
    pub reference: Option<String>,
    /// Indicates where a transaction is in its lifecycle.
    pub status: GlobalpayPaymentStatus,
    /// Global Payments time indicating when the object was created in ISO-8601 format.
    pub time_created: Option<String>,
    /// Describes whether the transaction is a SALE, that moves funds from Payer to Merchant, or
    /// a REFUND where funds move from Merchant to Payer.
    #[serde(rename = "type")]
    pub globalpay_payments_response_type: Option<requests::GlobalpayPaymentsRequestType>,
}

/// Information about the Action executed.
#[derive(Debug, Serialize, Deserialize)]
pub struct Action {
    /// The id of the app that was used to create the token.
    pub app_id: Option<String>,
    /// The name of the app the user gave to the application.
    pub app_name: Option<String>,
    /// A unique identifier for the object created by Global Payments. The first 3 characters
    /// identifies the resource an id relates to.
    pub id: Option<String>,
    /// The result of the action executed.
    pub result_code: Option<ResultCode>,
    /// Global Payments time indicating when the object was created in ISO-8601 format.
    pub time_created: Option<String>,
    /// Indicates the action taken.
    #[serde(rename = "type")]
    pub action_type: Option<String>,
}

/// Information relating to a currency conversion.
#[derive(Debug, Serialize, Deserialize)]
pub struct CurrencyConversion {
    /// The percentage commission taken for providing the currency conversion.
    pub commission_percentage: Option<String>,
    /// The exchange rate used to convert one currency to another.
    pub conversion_rate: Option<String>,
    /// The source of the base exchange rate was obtained to execute the currency conversion.
    pub exchange_rate_source: Option<String>,
    /// The time the base exchange rate was obtained from the source.
    pub exchange_source_time: Option<String>,
    /// The exchange rate used to convert one currency to another.
    pub margin_rate_percentage: Option<String>,
    /// The amount that will affect the payer's account.
    pub payer_amount: Option<String>,
    /// The currency of the amount that will affect the payer's account.
    pub payer_currency: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PaymentMethod {
    /// Data associated with the response of an APM transaction.
    pub apm: Option<Apm>,
    /// Information outlining the degree of authentication executed related to a transaction.
    pub authentication: Option<Authentication>,
    pub bank_transfer: Option<BankTransfer>,
    pub card: Option<Card>,
    pub digital_wallet: Option<requests::DigitalWallet>,
    /// Indicates how the payment method information was obtained by the Merchant for this
    /// transaction.
    pub entry_mode: Option<requests::PaymentMethodEntryMode>,
    /// If enabled, this field contains the unique fingerprint signature for that payment method
    /// for that merchant. If the payment method is seen again this same value is generated. For
    /// cards the primary account number is checked only. The expiry date or the CVV is not used
    /// for this check.
    pub fingerprint: Option<String>,
    /// If enabled, this field indicates whether the payment method has been seen before or is
    /// new.
    /// * EXISTS -  Indicates that the payment method was seen on the platform before by this
    /// merchant.
    /// * NEW - Indicates that the payment method was not seen on the platform before by this
    /// merchant.
    pub fingerprint_presence_indicator: Option<String>,
    /// Unique Global Payments generated id used to reference a stored payment method on the
    /// Global Payments system. Often referred to as the payment method token. This value can be
    /// used instead of payment method details such as a card number and expiry date.
    pub id: Option<String>,
    /// Result message from the payment method provider corresponding to the result code.
    pub message: Option<String>,
    /// Result code from the payment method provider.
    pub result: Option<String>,
}

/// Data associated with the response of an APM transaction.
#[derive(Debug, Serialize, Deserialize)]
pub struct Apm {
    pub bank: Option<Bank>,
    /// A string generated by the payment method that represents to what degree the merchant is
    /// funded for the transaction.
    pub fund_status: Option<FundStatus>,
    pub mandate: Option<Mandate>,
    /// Indicates that a redirect to the payment method is not required. Some payment methods
    /// (for example, SEPA DirectDebit) provide the option to redirect the customer to a page to
    /// display additional information about the payment.
    pub optional_redirect: Option<f64>,
    /// A string used to identify the payment method provider being used to execute this
    /// transaction.
    pub provider: Option<ApmProvider>,
    /// A name of the payer from the payment method system.
    pub provider_payer_name: Option<String>,
    /// The time the payment method provider created the transaction at on their system.
    pub provider_time_created: Option<String>,
    /// The reference the payment method provider created for the transaction.
    pub provider_transaction_reference: Option<String>,
    /// URL to redirect the payer from the merchant's system to the payment method's system.
    pub redirect_url: Option<String>,
    /// A string generated by the payment method to represent the session created on the payment
    /// method's platform to facilitate the creation of a transaction.
    pub session_token: Option<String>,
    /// Indicates to instruct the payer to wait for an update at the time the transaction is
    /// being executed or that an update will be given later.
    pub wait_notification: Option<f64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Bank {
    /// The local identifier of the bank account.
    pub account_number: Option<String>,
    /// The local identifier of the bank.
    pub code: Option<String>,
    /// The international identifier of the bank account.
    pub iban: Option<String>,
    /// The international identifier code for the bank.
    pub identifier_code: Option<String>,
    /// The name associated with the bank account
    pub name: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Mandate {
    /// The reference to identify the mandate.
    pub code: Option<String>,
}

/// Information outlining the degree of authentication executed related to a transaction.
#[derive(Debug, Serialize, Deserialize)]
pub struct Authentication {
    /// Information outlining the degree of 3D Secure authentication executed.
    pub three_ds: Option<ThreeDs>,
}

/// Information outlining the degree of 3D Secure authentication executed.
#[derive(Debug, Serialize, Deserialize)]
pub struct ThreeDs {
    /// The result of the three_ds value validation by the brands or issuing bank.
    pub value_result: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BankTransfer {
    /// The last 4 characters of the local reference for a bank account number.
    pub masked_number_last4: Option<String>,
    /// The name of the bank.
    pub name: Option<String>,
    /// The type of bank account associated with the payer's bank account.
    pub number_type: Option<NumberType>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Card {
    /// Code generated when the card is successfully authorized.
    pub authcode: Option<String>,
    /// The recommended AVS action to be taken by the agent processing the card transaction.
    pub avs_action: Option<String>,
    /// The result of the AVS address check.
    pub avs_address_result: Option<String>,
    /// The result of the AVS postal code check.
    pub avs_postal_code_result: Option<String>,
    /// Indicates the card brand that issued the card.
    pub brand: Option<Brand>,
    /// The unique reference created by the brands/schemes to uniquely identify the transaction.
    pub brand_reference: Option<String>,
    /// The time returned by the card brand indicating when the transaction was processed on
    /// their system.
    pub brand_time_reference: Option<String>,
    /// The result of the CVV check.
    pub cvv_result: Option<String>,
    /// Masked card number with last 4 digits showing.
    pub masked_number_last4: Option<String>,
    /// The result codes directly from the card issuer.
    pub provider: Option<ProviderClass>,
    /// The card EMV tag response data from the card issuer for a contactless or chip card
    /// transaction.
    pub tag_response: Option<String>,
}

/// The result codes directly from the card issuer.
#[derive(Debug, Serialize, Deserialize)]
pub struct ProviderClass {
    /// The result code of the AVS address check from the card issuer.
    #[serde(rename = "card.provider.avs_address_result")]
    pub card_provider_avs_address_result: Option<String>,
    /// The result of the AVS postal code check from the card issuer..
    #[serde(rename = "card.provider.avs_postal_code_result")]
    pub card_provider_avs_postal_code_result: Option<String>,
    /// The result code of the AVS check from the card issuer.
    #[serde(rename = "card.provider.avs_result")]
    pub card_provider_avs_result: Option<String>,
    /// The result code of the CVV check from the card issuer.
    #[serde(rename = "card.provider.cvv_result")]
    pub card_provider_cvv_result: Option<String>,
    /// Result code from the card issuer.
    #[serde(rename = "card.provider.result")]
    pub card_provider_result: Option<String>,
}

/// The result of the action executed.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ResultCode {
    Declined,
    Success,
}

/// A string generated by the payment method that represents to what degree the merchant is
/// funded for the transaction.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum FundStatus {
    Missing,
    NotExpected,
    Received,
    Waiting,
}

/// A string used to identify the payment method provider being used to execute this
/// transaction.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ApmProvider {
    Giropay,
    Ideal,
    Paypal,
    Sofort,
    Testpay,
}

/// The type of bank account associated with the payer's bank account.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum NumberType {
    Checking,
    Savings,
}

/// The recommended AVS action to be taken by the agent processing the card transaction.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AvsAction {
    Accept,
    Decline,
    Prompt,
}

/// The result of the AVS address check.
///
/// The result of the AVS postal code check.
///
/// The result of the CVV check.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum GlobalPayResult {
    Matched,
    NotChecked,
    NotMatched,
}

/// Indicates the card brand that issued the card.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Brand {
    Amex,
    Cup,
    Diners,
    Discover,
    Jcb,
    Mastercard,
    Visa,
}

/// If enabled, this field indicates whether the payment method has been seen before or is
/// new.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum FingerprintPresenceIndicator {
    ///  Indicates that the payment method was seen on the platform before by this
    /// merchant.
    Exists,
    /// Indicates that the payment method was not seen on the platform before by this
    /// merchant.
    New,
}

/// Indicates where a transaction is in its lifecycle.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum GlobalpayPaymentStatus {
    /// A Transaction has been successfully authorized and captured. The funding
    /// process will commence once the transaction remains in this status.
    Captured,
    /// A Transaction where the payment method provider declined the transfer of
    /// funds between the payer and the merchant.
    Declined,
    /// A Transaction where the funds have transferred between payer and merchant as
    /// expected.
    Funded,
    /// A Transaction has been successfully initiated. An update on its status is
    /// expected via a separate asynchronous notification to a webhook.
    Initiated,
    /// A Transaction has been sent to the payment method provider and are waiting
    /// for a result.
    Pending,
    /// A Transaction has been approved but a capture request is required to
    /// commence the movement of funds.
    Preauthorized,
    /// A Transaction where the funds were expected to transfer between payer and
    /// merchant but the transfer was rejected during the funding process. This rarely happens
    /// but when it does it is usually addressed by Global Payments operations.
    Rejected,
    /// A Transaction that had a status of PENDING, PREAUTHORIZED or CAPTURED has
    /// subsequently been reversed which voids/cancels a transaction before it is funded.
    Reversed,
}
