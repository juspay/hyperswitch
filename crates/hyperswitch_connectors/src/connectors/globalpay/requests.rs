use common_utils::types::StringMinorUnit;
use masking::Secret;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
pub struct GlobalPayRouterData<T> {
    pub amount: StringMinorUnit,
    pub router_data: T,
}

#[derive(Debug, Serialize)]
pub struct GlobalpayCancelRouterData<T> {
    pub amount: Option<StringMinorUnit>,
    pub router_data: T,
}

#[derive(Debug, Serialize)]
pub struct GlobalpayPaymentsRequest {
    pub account_name: Secret<String>,
    pub amount: Option<StringMinorUnit>,
    pub currency: String,
    pub reference: String,
    pub country: api_models::enums::CountryAlpha2,
    pub capture_mode: Option<CaptureMode>,
    pub notifications: Option<Notifications>,
    pub payment_method: GlobalPayPaymentMethodData,
    pub channel: Channel,
    pub initiator: Option<Initiator>,
    pub stored_credential: Option<StoredCredential>,
}

#[derive(Debug, Serialize)]
pub struct GlobalpayRefreshTokenRequest {
    pub app_id: Secret<String>,
    pub nonce: Secret<String>,
    pub secret: Secret<String>,
    pub grant_type: String,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct Notifications {
    pub return_url: Option<String>,
    pub status_url: Option<String>,
    pub cancel_url: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PaymentMethodData {
    Card(Card),
    Apm(Apm),
    DigitalWallet(DigitalWallet),
    Token(TokenizationData),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CommonPaymentMethodData {
    #[serde(flatten)]
    pub payment_method_data: PaymentMethodData,
    pub entry_mode: PaymentMethodEntryMode,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MandatePaymentMethodData {
    pub entry_mode: PaymentMethodEntryMode,
    pub id: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum GlobalPayPaymentMethodData {
    Common(CommonPaymentMethodData),
    Mandate(MandatePaymentMethodData),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Apm {
    /// A string used to identify the payment method provider being used to execute this
    /// transaction.
    pub provider: Option<ApmProvider>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Card {
    pub cvv: Secret<String>,
    pub expiry_month: Secret<String>,
    pub expiry_year: Secret<String>,
    pub number: cards::CardNumber,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct TokenizationData {
    pub brand_reference: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DigitalWallet {
    /// Identifies who provides the digital wallet for the Payer.
    pub provider: Option<DigitalWalletProvider>,
    /// A token that represents, or is the payment method, stored with  the digital wallet.
    pub payment_token: Option<serde_json::Value>,
}

/// Stored data information used to create a transaction.
#[derive(Debug, Serialize, Deserialize)]
pub struct StoredCredential {
    /// Indicates the transaction processing model being executed when using stored
    /// credentials.
    pub model: Option<Model>,
    /// Indicates the order of this transaction in the sequence of a planned repeating
    /// transaction processing model.
    pub sequence: Option<Sequence>,
}

/// Indicates whether the transaction is to be captured automatically, later or later using
/// more than 1 partial capture.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CaptureMode {
    /// If a transaction is authorized, funds will exchange between the payer and
    /// merchant automatically and as soon as possible.
    Auto,
    /// If a transaction is authorized, funds will not exchange between the payer and
    /// merchant automatically and will require a subsequent separate action to capture that
    /// transaction and start the funding process. Only one successful capture is permitted.
    Later,
    /// If a transaction is authorized, funds will not exchange between the payer
    /// and merchant automatically. One or more subsequent separate capture actions are required
    /// to capture that transaction in parts and start the funding process for the part captured.
    /// One or many successful capture are permitted once the total amount captured is within a
    /// range of the original authorized amount.'
    Multiple,
}

/// Describes whether the transaction was processed in a face to face(CP) scenario or a
/// Customer Not Present (CNP) scenario.
#[derive(Debug, Default, Serialize, Deserialize)]
pub enum Channel {
    #[default]
    #[serde(rename = "CNP")]
    /// A Customer NOT Present transaction is when the payer and the merchant are not
    /// together when exchanging payment method information to fulfill a transaction. e.g. a
    /// transaction executed from a merchant's website or over the phone
    CustomerNotPresent,
    #[serde(rename = "CP")]
    /// A Customer Present transaction is when the payer and the merchant are in direct
    /// face to face contact when exchanging payment method information to fulfill a transaction.
    /// e.g. in a store and paying at the counter that is attended by a clerk.
    CustomerPresent,
}

/// Indicates whether the Merchant or the Payer initiated the creation of a transaction.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Initiator {
    /// The transaction was initiated by the merchant, who is getting paid by the
    /// payer.'
    Merchant,
    /// The transaction was initiated by the customer who is paying the merchant.
    Payer,
}

/// A string used to identify the payment method provider being used to execute this
/// transaction.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApmProvider {
    Giropay,
    Ideal,
    Paypal,
    Sofort,
    Eps,
    Testpay,
}

/// Identifies who provides the digital wallet for the Payer.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DigitalWalletProvider {
    Applepay,
    PayByGoogle,
}

/// Indicates how the payment method information was obtained by the Merchant for this
/// transaction.
#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PaymentMethodEntryMode {
    /// A CP channel entry mode where the payment method information was obtained from a
    /// chip. E.g. card is inserted into a device to read the chip.
    Chip,
    ///  A CP channel entry mode where the payment method information was
    /// obtained by bringing the payment method to close proximity of a device. E.g. tap a cardon
    /// or near a device to exchange card information.
    ContactlessChip,
    ///  A CP channel entry mode where the payment method information was
    /// obtained by bringing the payment method to close proximity of a device and also swiping
    /// the card. E.g. tap a card on or near a device and swipe it through device to exchange
    /// card information
    ContactlessSwipe,
    #[default]
    /// A CNP channel entry mode where the payment method was obtained via a browser.
    Ecom,
    /// A CNP channel entry mode where the payment method was obtained via an
    /// application and applies to digital wallets only.
    InApp,
    /// A CNP channel entry mode where the payment method was obtained via postal mail.
    Mail,
    /// A CP channel entry mode where the payment method information was obtained by
    /// manually keying the payment method information into the device.
    Manual,
    /// A CNP channel entry mode where the payment method information was obtained over
    /// the phone or via postal mail.
    Moto,
    /// A CNP channel entry mode where the payment method was obtained over the
    /// phone.
    Phone,
    /// A CP channel entry mode where the payment method information was obtained from
    /// swiping a magnetic strip. E.g. card's magnetic strip is swiped through a device to read
    /// the card information.
    Swipe,
}

/// Indicates the transaction processing model being executed when using stored
/// credentials.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Model {
    /// The transaction is a repeat transaction initiated by the merchant and
    /// taken using the payment method stored with the merchant, as part of an agreed schedule of
    /// transactions and where the amount is known and agreed in advanced. For example the
    /// payment in full of a good in fixed installments over a defined period of time.'
    Installment,
    /// The transaction is a repeat transaction initiated by the merchant and taken
    /// using the payment method stored with the merchant, as part of an agreed schedule of
    /// transactions.
    Recurring,
    /// The transaction is a repeat transaction initiated by the merchant and
    /// taken using the payment method stored with the merchant, as part of an agreed schedule of
    /// transactions. The amount taken is based on the usage by the payer of the good or service.
    /// for example a monthly mobile phone bill.
    Subscription,
    /// the transaction is adhoc or unscheduled. For example a payer visiting a
    /// merchant to make purchase using the payment method stored with the merchant.
    Unscheduled,
}

/// Indicates the order of this transaction in the sequence of a planned repeating
/// transaction processing model.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Sequence {
    First,
    Last,
    Subsequent,
}

#[derive(Default, Debug, Serialize)]
pub struct GlobalpayRefundRequest {
    pub amount: StringMinorUnit,
}

#[derive(Default, Debug, Serialize)]
pub struct GlobalpayCaptureRequest {
    pub amount: Option<StringMinorUnit>,
    pub capture_sequence: Option<Sequence>,
    pub reference: Option<String>,
}

#[derive(Default, Debug, Serialize)]
pub struct GlobalpayCancelRequest {
    pub amount: Option<StringMinorUnit>,
}

#[derive(Default, Debug, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum UsageMode {
    /// This value must be used if using the Hosted Fields or the Drop-in UI integration types.
    /// When creating the payment method token, this option ensures the payment method token is temporary and will be removed once a transaction is executed or after a short period of time.
    #[default]
    Single,
    /// When creating the payment method token, this indicates it is permanent and can be used to create many transactions.
    Multiple,
    /// When using the payment method token to transaction process, this indicates to use the card number also known as the PAN or FPAN when both the card number and the network token are available.
    UseCardNumber,
    /// When using the payment method token to transaction process, this indicates to use the network token instead of the card number if both are available.
    UseNetworkToken,
}

#[derive(Default, Debug, Serialize)]
pub struct GlobalPayPaymentMethodsRequest {
    pub reference: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage_mode: Option<UsageMode>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub card: Option<Card>,
}
