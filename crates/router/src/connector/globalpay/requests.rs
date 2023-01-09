use serde::{Deserialize, Serialize};

/// A geographical coordinate
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct GlobalpayPaymentsRequest {
    /// A meaningful label for the merchant account set by Global Payments.
    pub account_name: String,
    /// The amount to transfer between Payer and Merchant for a SALE or a REFUND. It is always
    /// represented in the lowest denomiation of the related currency.
    pub amount: String,
    /// Indicates if the merchant would accept an authorization for an amount less than the
    /// requested amount. This is available for CP channel
    /// only where the balance not authorized can be processed again using a different card.
    /// * PARTIAL - Indicates merchant would accept an authorization for an amount less than the
    /// requested amount.
    /// example: PARTIAL
    pub authorization_mode: Option<AuthorizationMode>,
    /// Indicates whether the transaction is to be captured automatically, later or later using
    /// more than 1 partial capture.
    /// * AUTO - If a transaction is authorized, funds will exchange between the payer and
    /// merchant automatically and as soon as possible.
    /// * LATER - If a transaction is authorized, funds will not exchange between the payer and
    /// merchant automatically and will require a subsequent separate action to capture that
    /// transaction and start the funding process. Only one successful capture is permitted.
    /// * MULTIPLE - If a transaction is authorized, funds will not exchange between the payer
    /// and merchant automatically. One or more subsequent separate capture actions are required
    /// to capture that transaction in parts and start the funding process for the part captured.
    /// One or many successful capture are permitted once the total amount captured is within a
    /// range of the original authorized amount.'
    pub capture_mode: Option<CaptureMode>,
    /// The amount of the transaction that relates to cashback.It is always represented in the
    /// lowest denomiation of the related currency.
    pub cashback_amount: Option<String>,
    /// Describes whether the transaction was processed in a face to face(CP) scenario or a
    /// Customer Not Present (CNP) scenario.
    /// * CP - A Customer Present transaction is when the payer and the merchant are in direct
    /// face to face contact when exchanging payment method information to fulfill a transaction.
    /// e.g. in a store and paying at the counter that is attended by a clerk.
    /// * CNP - A Customer NOT Present transaction is when the payer and the merchant are not
    /// together when exchanging payment method information to fulfill a transaction. e.g. a
    /// transaction executed from a merchant's website or over the phone
    pub channel: Channel,
    /// The amount that reflects the charge the merchant applied to the transaction for availing
    /// of a more convenient purchase.It is always represented in the lowest denomiation of the
    /// related currency.
    pub convenience_amount: Option<String>,
    /// The country in ISO-3166-1(alpha-2 code) format.
    pub country: String,
    /// The currency of the amount in ISO-4217(alpha-3)
    pub currency: String,
    pub currency_conversion: Option<CurrencyConversion>,
    /// Merchant defined field to describe the transaction.
    pub description: Option<String>,
    pub device: Option<Device>,
    /// The amount of the gratuity for a transaction.It is always represented in the lowest
    /// denomiation of the related currency.
    pub gratuity_amount: Option<String>,
    /// Indicates whether the Merchant or the Payer initiated the creation of a transaction.
    /// * PAYER - The transaction was initated by the customer who is paying the merchant.
    /// * MERCHANT - The transaction was initated by the merchant, who is getting paid by the
    /// payer.'
    pub initiator: Option<Initiator>,
    /// Indicates the source IP Address of the system used to create the transaction.
    pub ip_address: Option<String>,
    /// Indicates the language the transaction was executed in. In the format ISO-639-1 (alpha-2)
    /// or ISO-639-1 (alpha-2)_ISO-3166(alpha-2)
    pub language: Option<Language>,
    pub lodging: Option<Lodging>,
    /// Indicates to Global Payments where the merchant wants to receive notifications of certain
    /// events that occur on the Global Payments system.
    pub notifications: Option<Notifications>,
    pub order: Option<Order>,
    /// The merchant's payer reference for the transaction
    pub payer_reference: Option<String>,
    pub payment_method: PaymentMethod,
    /// Merchant defined field to reference the transaction.
    pub reference: String,
    /// A merchant defined reference for the location that created the transaction.
    pub site_reference: Option<String>,
    /// Stored data information used to create a transaction.
    pub stored_credential: Option<StoredCredential>,
    /// The amount that reflects the additional charge the merchant applied to the transaction
    /// for using a specific payment method.It is always represented in the lowest denomiation of
    /// the related currency.
    pub surcharge_amount: Option<String>,
    /// Indicates the total or expected total of captures that will executed against a
    /// transaction flagged as being captured multiple times.
    pub total_capture_count: Option<i64>,
    /// Describes whether the transaction is a SALE, that moves funds from Payer to Merchant, or
    /// a REFUND where funds move from Merchant to Payer.
    /// * SALE - indicates the movement, or the attempt to move, funds from payer to a
    /// merchant.
    /// * REFUND - indicates the movement, or the attempt to move, funds from merchant to the
    /// payer.
    #[serde(rename = "type")]
    pub globalpay_payments_request_type: Option<GlobalpayPaymentsRequestType>,
    /// The merchant's user reference for the transaction. This represents the person who
    /// processed the transaction on the merchant's behalf like a clerk or cashier reference.
    pub user_reference: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CurrencyConversion {
    /// A unique identifier generated by Global Payments to identify the currency conversion. It
    /// can be used to reference a currency conversion when processing a sale or a refund
    /// transaction.
    pub id: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Device {
    pub capabilities: Option<Capabilities>,
    pub entry_modes: Option<Vec<Vec<DeviceEntryMode>>>,
    /// Describes whether a device prompts a payer for a gratuity when the payer is entering
    /// their payment method details to the device.
    pub gratuity_prompt_mode: Option<GratuityPromptMode>,
    /// Describes the receipts a device prints when processing a transaction.
    pub print_receipt_mode: Option<PrintReceiptMode>,
    /// The sequence number from the device used to align with processing platform.
    pub sequence_number: Option<String>,
    /// A unique identifier for the physical device. This value persists with the device even if
    /// it is repurposed.
    pub serial_number: Option<String>,
    /// The time from the device in ISO8601 format
    pub time: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Capabilities {
    pub authorization_modes: Option<Vec<AuthorizationMode>>,
    /// The number of lines that can be used to display information on the device.
    pub display_line_count: Option<f64>,
    pub enabled_response: Option<Vec<EnabledResponse>>,
    pub entry_modes: Option<Vec<CapabilitiesEntryMode>>,
    pub fraud: Option<Vec<AuthorizationMode>>,
    pub mobile: Option<Vec<Mobile>>,
    pub payer_verifications: Option<Vec<PayerVerification>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Lodging {
    /// A reference that identifies the booking reference for a lodging stay.
    pub booking_reference: Option<String>,
    /// The amount charged for one nights lodging.
    pub daily_rate_amount: Option<String>,
    /// A reference that identifies the booking reference for a lodging stay.
    pub date_checked_in: Option<String>,
    /// The check out date for a lodging stay.
    pub date_checked_out: Option<String>,
    /// The total number of days of the lodging stay.
    pub duration_days: Option<f64>,
    #[serde(rename = "lodging.charge_items")]
    pub lodging_charge_items: Option<Vec<LodgingChargeItem>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LodgingChargeItem {
    pub payment_method_program_codes: Option<Vec<PaymentMethodProgramCode>>,
    /// A reference that identifies the charge item, such as a lodging folio number.
    pub reference: Option<String>,
    /// The total amount for the list of charge types for a charge item.
    pub total_amount: Option<String>,
    pub types: Option<Vec<TypeElement>>,
}

/// Indicates to Global Payments where the merchant wants to receive notifications of certain
/// events that occur on the Global Payments system.
#[derive(Debug, Serialize, Deserialize)]
pub struct Notifications {
    /// The merchant URL that will receive the notification when the customer has completed the
    /// authentication.
    pub challenge_return_url: Option<String>,
    /// The merchant URL that will receive the notification when the customer has completed the
    /// authentication when the authentication is decoupled and separate to the purchase.
    pub decoupled_challenge_return_url: Option<String>,
    /// The merchant URL to return the payer to, once the payer has completed payment using the
    /// payment method. This returns control of the payer's payment experience to the merchant.
    pub return_url: Option<String>,
    /// The merchant URL to notify the merchant of the latest status of the transaction.
    pub status_url: Option<String>,
    /// The merchant URL that will receive the notification when the 3DS ACS successfully gathers
    /// de ice informatiSon and tonotification_configurations.cordingly.
    pub three_ds_method_return_url: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Order {
    /// Merchant defined field common to all transactions that are part of the same order.
    pub reference: Option<String>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct PaymentMethod {
    pub apm: Option<Apm>,
    pub authentication: Option<Authentication>,
    pub bank_transfer: Option<BankTransfer>,
    pub card: Option<Card>,
    pub digital_wallet: Option<DigitalWallet>,
    pub encryption: Option<Encryption>,
    /// Indicates how the payment method information was obtained by the Merchant for this
    /// transaction.
    /// * MOTO - A CNP channel entry mode where the payment method information was obtained over
    /// the phone or via postal mail.
    /// * ECOM - A CNP channel entry mode where the payment method was obtained via a browser.
    /// * IN_APP - A CNP channel entry mode where the payment method was obtained via an
    /// application and applies to digital wallets only.
    /// * CHIP - A CP channel entry mode where the payment method information was obtained from a
    /// chip. E.g. card is inserted into a device to read the chip.
    /// * SWIPE - A CP channel entry mode where the payment method information was obtained from
    /// swiping a magnetic strip. E.g. card's magnetic strip is swiped through a device to read
    /// the card information.
    /// * MANUAL - A CP channel entry mode where the payment method information was obtained by
    /// manually keying the payment method information into the device.
    /// * CONTACTLESS_CHIP -  A CP channel entry mode where the payment method information was
    /// obtained by bringing the payment method to close proximity of a device. E.g. tap a cardon
    /// or near a device to exchange card information.
    /// * CONTACTLESS_SWIPE -  A CP channel entry mode where the payment method information was
    /// obtained by bringing the payment method to close proximity of a device and also swiping
    /// the card. E.g. tap a card on or near a device and swipe it through device to exchange
    /// card information
    /// * PHONE - A CNP channel entry mode where the payment method was obtained over the
    /// phone.
    /// * MAIL - A CNP channel entry mode where the payment method was obtained via postal mail.
    pub entry_mode: PaymentMethodEntryMode,
    /// Indicates whether to execute the fingerprint signature functionality.
    /// * ALWAYS - Always check and create the fingerprint value regardless of the result of the
    /// card authorization.
    /// * ON_SUCCESS - Always check and create the fingerprint value when the card authorization
    /// is successful.
    pub fingerprint_mode: Option<Mode>,
    /// Specify the first name of the owner of the payment method.
    pub first_name: Option<String>,
    /// Unique Global Payments generated id used to reference a stored payment method on the
    /// Global Payments system. Often referred to as the payment method token. This value can be
    /// used instead of payment method details such as a card number and expiry date.
    pub id: Option<String>,
    /// Specify the surname of the owner of the payment method.
    pub last_name: Option<String>,
    /// The full name of the owner of the payment method.
    pub name: Option<String>,
    /// Contains the value a merchant wishes to appear on the payer's payment method statement
    /// for this transaction
    pub narrative: Option<String>,
    /// Indicates whether to store the card as part of a transaction.
    /// * ALWAYS -  The card information is always stored irrespective of whether the payment
    /// method authorization was successful or not.
    /// * ON_SUCCESS - The card information is only storedif the payment method authorization was
    /// successful.
    pub storage_mode: Option<Mode>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Apm {
    /// A string used to identify the payment method provider being used to execute this
    /// transaction.
    pub provider: Option<ApmProvider>,
}

/// Information outlining the degree of authentication executed related to a transaction.
#[derive(Debug, Serialize, Deserialize)]
pub struct Authentication {
    /// Information outlining the degree of 3D Secure authentication executed.
    pub three_ds: Option<ThreeDs>,
    /// A message authentication code that is used to confirm the security and integrity of the
    /// messaging to Global Payments.
    pub mac: Option<String>,
}

/// Information outlining the degree of 3D Secure authentication executed.
#[derive(Debug, Serialize, Deserialize)]
pub struct ThreeDs {
    /// The reference created by the 3DSecure Directory Server to identify the specific
    /// authentication attempt.
    pub ds_trans_reference: Option<String>,
    /// An indication of the degree of the authentication and liability shift obtained for this
    /// transaction. It is determined during the 3D Secure process. 2 or 1  for Mastercard
    /// indicates the merchant has a liability shift. 5 or 6  for Visa or Amex indicates the
    /// merchant has a liability shift. However for Amex if the payer is not enrolled the eci may
    /// still be 6 but liability shift has not bee achieved.
    pub eci: Option<String>,
    /// Indicates if any exemptions apply to this transaction.
    pub exempt_status: Option<ExemptStatus>,
    /// Indicates the version of 3DS
    pub message_version: Option<String>,
    /// The reference created by the 3DSecure provider to identify the specific authentication
    /// attempt.
    pub server_trans_reference: Option<String>,
    /// The authentication value created as part of the 3D Secure process.
    pub value: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BankTransfer {
    /// The number or reference for the payer's bank account.
    pub account_number: Option<String>,
    pub bank: Option<Bank>,
    /// The number or reference for the check
    pub check_reference: Option<String>,
    /// The type of bank account associated with the payer's bank account.
    pub number_type: Option<NumberType>,
    /// Indicates how the transaction was authorized by the merchant.
    /// * CCD - Cash Concentration or Disbursement - Can be either a credit or debit application
    /// where funds are wither distributed or consolidated between corporate entities.
    /// * POP - Point of Sale Entry - Point of sale debit applications non-shared (POS)
    /// environment. These transactions are most often initiated by the consumer via a plastic
    /// access card. This is only support for normal ACH transactions
    /// * PPD - Prearranged Payment and Deposits - used to credit or debit a consumer account.
    /// Popularity used for payroll direct deposits and pre-authorized bill payments.
    /// * TEL - Telephone-Initiated Entry - Used for the origination of a single entry debit
    /// transaction to a consumer's account pursuant to a verbal authorization obtained from the
    /// consumer via the telephone.
    /// * WEB - Internet (Web)-Initiated Entry - Used for the origination of debit entries
    /// (either Single or Recurring Entry) to a consumer's account pursuant to a to an
    /// authorization that is obtained from the Receiver via the Internet.
    pub sec_code: Option<SecCode>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Bank {
    pub address: Option<Address>,
    /// The local identifier code for the bank.
    pub code: Option<String>,
    /// The name of the bank.
    pub name: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Address {
    /// Merchant defined field common to all transactions that are part of the same order.
    pub city: Option<String>,
    /// The country in ISO-3166-1(alpha-2 code) format.
    pub country: Option<String>,
    /// First line of the address.
    pub line_1: Option<String>,
    /// Second line of the address.
    pub line_2: Option<String>,
    /// Third  line of the address.
    pub line_3: Option<String>,
    /// The city or town of the address.
    pub postal_code: Option<String>,
    /// The state or region of the address. ISO 3166-2 minus the country code itself. For
    /// example, US Illinois = IL, or in the case of GB counties Wiltshire = WI or Aberdeenshire
    /// = ABD
    pub state: Option<String>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Card {
    /// The card providers description of their card product.
    pub account_type: Option<String>,
    /// Code generated when the card is successfully authorized.
    pub authcode: Option<String>,
    /// First line of the address associated with the card.
    pub avs_address: Option<String>,
    /// Postal code of the address associated with the card.
    pub avs_postal_code: Option<String>,
    /// The unique reference created by the brands/schemes to uniquely identify the transaction.
    pub brand_reference: Option<String>,
    /// Indicates if a fallback mechanism was used to obtain the card information when EMV/chip
    /// did not work as expected.
    /// * PREV_SUCCESS - indicates the previous transaction with this card was a success.
    /// * PREV_FAILED - indicates the previous transaction with this card failed.
    pub chip_condition: Option<ChipCondition>,
    /// The numeric value printed on the physical card.
    pub cvv: String,
    /// Card Verification Value Indicator sent by the Merchant indicating the CVV
    /// availability.
    /// * ILLEGIBLE - indicates the cvv is present but cannot be read.
    /// * NOT_PRESENT - indicates the cvv is not present on the card.
    /// * PRESENT - indicates the cvv is present.
    pub cvv_indicator: CvvIndicator,
    /// The 2 digit expiry date month of the card.
    pub expiry_month: String,
    /// The 2 digit expiry date year of the card.
    pub expiry_year: String,
    /// Indicates whether the card is a debit or credit card.
    /// * DEBIT - indicates the card is a debit card where the funds may be present in an account
    /// to fulfill the transaction amount.
    /// * CREDIT - indicates the card is a credit card where the funds may be available on credit
    /// to the payer to fulfill the transaction amount.
    /// * FOOD_STAMP - indicates the card is an, Electronic Benefits Transfer, for food stamps.
    /// * CASH_BENEFITS - indicates the card is an, Electronic Benefits Transfer, for cash
    /// benefits.
    /// * PREPAID - indicates the card is a prepaid card where the funds are loaded to the card
    /// account to fulfill the transaction amount. Unlike a debit card, a prepaid is not linked
    /// to a bank account.
    pub funding: Option<Funding>,
    /// The the card account number used to authorize the transaction. Also known as PAN.
    pub number: String,
    /// Contains the pin block info, relating to the pin code the Payer entered.
    pub pin_block: Option<String>,
    /// The full card tag data for an EMV/chip card transaction.
    pub tag: Option<String>,
    /// Data from magnetic stripe of a card
    pub track: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DigitalWallet {
    /// First line of the address associated with the card.
    pub avs_address: Option<String>,
    /// Postal code of the address associated with the card.
    pub avs_postal_code: Option<String>,
    /// The authentication value use to verify the validity of the digit wallet transaction.
    pub cryptogram: Option<String>,
    /// The numeric value printed on the physical card.
    pub cvv: Option<String>,
    /// Card Verification Value Indicator sent by the Merchant indicating the CVV
    /// availability.
    /// * ILLEGIBLE - indicates the cvv is present but cannot be read.
    /// * NOT_PRESENT - indicates the cvv is not present on the card.
    /// * PRESENT - indicates the cvv is present.
    pub cvv_indicator: Option<CvvIndicator>,
    /// An indication of the degree of the authentication and liability shift obtained for this
    /// transaction. It is determined during the 3D Secure process. 2 or 1  for Mastercard
    /// indicates the merchant has a liability shift. 5 or 6  for Visa or Amex indicates the
    /// merchant has a liability shift. However for Amex if the payer is not enrolled the eci may
    /// still be 6 but liability shift has not bee achieved.
    pub eci: Option<String>,
    /// The 2 digit expiry date month of the card.
    pub expiry_month: Option<String>,
    /// The 2 digit expiry date year of the card.
    pub expiry_year: Option<String>,
    /// Identifies who provides the digital wallet for the Payer.
    pub provider: Option<DigitalWalletProvider>,
    /// A token that represents, or is the payment method, stored with  the digital wallet.
    pub token: Option<String>,
    /// Indicates if the actual card number or a token is being used to process the
    /// transaction.
    /// * CARD_NUMBER - The value in the digital wallet token field is a real card number
    /// (PAN)
    /// * CARD_TOKEN - The value in the digital wallet token field is a temporary token in the
    /// format of a card number (PAN) but is not a real card number.
    pub token_format: Option<TokenFormat>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Encryption {
    /// The encryption info used when sending encrypted card data to Global Payments.
    pub info: Option<String>,
    /// The encryption method used when sending encrypted card data to Global Payments.
    pub method: Option<Method>,
    /// The version of encryption being used.
    pub version: Option<String>,
}

/// Stored data information used to create a transaction.
#[derive(Debug, Serialize, Deserialize)]
pub struct StoredCredential {
    /// Indicates the transaction processing model being executed when using stored
    /// credentials.
    /// * UNSCHEDULED - the transaction is adhoc or unscheduled. For example a payer visiting a
    /// merchant to make purchase using the payment method stored with the merchant.
    /// * RECURRING - The transaction is a repeat transaction initiated my the merchant and taken
    /// using the payment method stored with the merchant, as part of an agreed schedule of
    /// transactions.
    /// * SUBSCRIPTION - The transaction is a repeat transaction initiated my the merchant and
    /// taken using the payment method stored with the merchant, as part of an agreed schedule of
    /// transactions. The amount taken is based on the usage by the payer of the good or service.
    /// for example a monthly mobile phone bill.
    /// * INSTALLMENT - The transaction is a repeat transaction initiated my the merchant and
    /// taken using the payment method stored with the merchant, as part of an agreed schedule of
    /// transactions and where the amount is known and agreed in advanced. For example the
    /// payment in full of a good in fixed installments over a defined period of time.'
    pub model: Option<Model>,
    /// The reason stored credentials are being used to to create a transaction.
    pub reason: Option<Reason>,
    /// Indiciates the order of this transaction in the sequence of a planned repeating
    /// transaction processing model.
    pub sequence: Option<Sequence>,
}

/// Indicates if the merchant would accept an authorization for an amount less than the
/// requested amount. This is available for CP channel
/// only where the balance not authorized can be processed again using a different card.
/// * PARTIAL - Indicates merchant would accept an authorization for an amount less than the
/// requested amount.
///  pub example: PARTIAL
///
///
/// Describes whether the device can process partial authorizations.
///
/// Describes the instruction a device can indicate to the clerk in the case of fraud.
#[derive(Debug, Serialize, Deserialize)]
pub enum AuthorizationMode {
    #[serde(rename = "PARTIAL")]
    Partial,
}

/// Indicates whether the transaction is to be captured automatically, later or later using
/// more than 1 partial capture.
/// * AUTO - If a transaction is authorized, funds will exchange between the payer and
/// merchant automatically and as soon as possible.
/// * LATER - If a transaction is authorized, funds will not exchange between the payer and
/// merchant automatically and will require a subsequent separate action to capture that
/// transaction and start the funding process. Only one successful capture is permitted.
/// * MULTIPLE - If a transaction is authorized, funds will not exchange between the payer
/// and merchant automatically. One or more subsequent separate capture actions are required
/// to capture that transaction in parts and start the funding process for the part captured.
/// One or many successful capture are permitted once the total amount captured is within a
/// range of the original authorized amount.'
#[derive(Debug, Serialize, Deserialize)]
pub enum CaptureMode {
    #[serde(rename = "AUTO")]
    Auto,
    #[serde(rename = "LATER")]
    Later,
    #[serde(rename = "MULTIPLE")]
    Multiple,
}

/// Describes whether the transaction was processed in a face to face(CP) scenario or a
/// Customer Not Present (CNP) scenario.
/// * CP - A Customer Present transaction is when the payer and the merchant are in direct
/// face to face contact when exchanging payment method information to fulfill a transaction.
/// e.g. in a store and paying at the counter that is attended by a clerk.
/// * CNP - A Customer NOT Present transaction is when the payer and the merchant are not
/// together when exchanging payment method information to fulfill a transaction. e.g. a
/// transaction executed from a merchant's website or over the phone
#[derive(Debug, Default, Serialize, Deserialize)]
pub enum Channel {
    #[default]
    #[serde(rename = "CNP")]
    Cnp,
    #[serde(rename = "CP")]
    Cp,
}

/// Describes the data the device can handle when it receives a response for a card
/// authorization.
#[derive(Debug, Serialize, Deserialize)]
pub enum EnabledResponse {
    #[serde(rename = "AVS")]
    Avs,
    #[serde(rename = "BRAND_REFERENCE")]
    BrandReference,
    #[serde(rename = "CVV")]
    Cvv,
    #[serde(rename = "MASKED_NUMBER_LAST4")]
    MaskedNumberLast4,
}

/// Describes the entry mode capabilities a device has.
#[derive(Debug, Serialize, Deserialize)]
pub enum CapabilitiesEntryMode {
    #[serde(rename = "CHIP")]
    Chip,
    #[serde(rename = "CONTACTLESS")]
    Contactless,
    #[serde(rename = "CONTACTLESS_SWIPE")]
    ContactlessSwipe,
    #[serde(rename = "MANUAL")]
    Manual,
    #[serde(rename = "SWIPE")]
    Swipe,
}

/// Describes the mobile features a device has
#[derive(Debug, Serialize, Deserialize)]
pub enum Mobile {
    #[serde(rename = "INTEGRATED_CARD_READER")]
    IntegratedCardReader,
    #[serde(rename = "SEPARATE_CARD_READER")]
    SeparateCardReader,
}

/// Describes the capabilities a device has to verify a payer.
#[derive(Debug, Serialize, Deserialize)]
pub enum PayerVerification {
    #[serde(rename = "CONTACTLESS_SIGNATURE")]
    ContactlessSignature,
    #[serde(rename = "PAYER_DEVICE")]
    PayerDevice,
    #[serde(rename = "PINPAD")]
    Pinpad,
}

/// Describes the allowed entry modes to obtain payment method information from the payer as
/// part of a transaction request.
#[derive(Debug, Serialize, Deserialize)]
pub enum DeviceEntryMode {
    #[serde(rename = "CHIP")]
    Chip,
    #[serde(rename = "CONTACTLESS")]
    Contactless,
    #[serde(rename = "MANUAL")]
    Manual,
    #[serde(rename = "SWIPE")]
    Swipe,
}

/// Describes whether a device prompts a payer for a gratuity when the payer is entering
/// their payment method details to the device.
#[derive(Debug, Serialize, Deserialize)]
pub enum GratuityPromptMode {
    #[serde(rename = "NOT_REQUIRED")]
    NotRequired,
    #[serde(rename = "PROMPT")]
    Prompt,
}

/// Describes the receipts a device prints when processing a transaction.
#[derive(Debug, Serialize, Deserialize)]
pub enum PrintReceiptMode {
    #[serde(rename = "BOTH")]
    Both,
    #[serde(rename = "MERCHANT")]
    Merchant,
    #[serde(rename = "NONE")]
    None,
    #[serde(rename = "PAYER")]
    Payer,
}

/// Describes whether the transaction is a SALE, that moves funds from Payer to Merchant, or
/// a REFUND where funds move from Merchant to Payer.
/// * SALE - indicates the movement, or the attempt to move, funds from payer to a
/// merchant.
/// * REFUND - indicates the movement, or the attempt to move, funds from merchant to the
/// payer.
#[derive(Debug, Serialize, Deserialize)]
pub enum GlobalpayPaymentsRequestType {
    #[serde(rename = "REFUND")]
    Refund,
    #[serde(rename = "SALE")]
    Sale,
}

/// Indicates whether the Merchant or the Payer initiated the creation of a transaction.
/// * PAYER - The transaction was initated by the customer who is paying the merchant.
/// * MERCHANT - The transaction was initated by the merchant, who is getting paid by the
/// payer.'
#[derive(Debug, Serialize, Deserialize)]
pub enum Initiator {
    #[serde(rename = "MERCHANT")]
    Merchant,
    #[serde(rename = "PAYER")]
    Payer,
}

/// Indicates the language the transaction was executed in. In the format ISO-639-1 (alpha-2)
/// or ISO-639-1 (alpha-2)_ISO-3166(alpha-2)
#[derive(Debug, Serialize, Deserialize)]
pub enum Language {
    #[serde(rename = "fr")]
    Fr,
    #[serde(rename = "fr_CA")]
    FrCa,
    #[serde(rename = "ISO-639(alpha-2)")]
    Iso639Alpha2,
    #[serde(rename = "ISO-639(alpha-2)_ISO-3166(alpha-2)")]
    Iso639alpha2Iso3166alpha2,
}

/// Describes the payment method programs, typically run by card brands such as Amex, Visa
/// and MC.
#[derive(Debug, Serialize, Deserialize)]
pub enum PaymentMethodProgramCode {
    #[serde(rename = "ASSURED_RESERVATION")]
    AssuredReservation,
    #[serde(rename = "CARD_DEPOSIT")]
    CardDeposit,
    #[serde(rename = "OTHER")]
    Other,
    #[serde(rename = "PURCHASE")]
    Purchase,
}

/// Describes the types of charges associated with a transaction. This can be one or more
/// than more charge type.
#[derive(Debug, Serialize, Deserialize)]
pub enum TypeElement {
    #[serde(rename = "GIFT_SHOP")]
    GiftShop,
    #[serde(rename = "LAUNDRY")]
    Laundry,
    #[serde(rename = "MINI_BAR")]
    MiniBar,
    #[serde(rename = "NO_SHOW")]
    NoShow,
    #[serde(rename = "OTHER")]
    Other,
    #[serde(rename = "PHONE")]
    Phone,
    #[serde(rename = "RESTAURANT")]
    Restaurant,
}

/// A string used to identify the payment method provider being used to execute this
/// transaction.
#[derive(Debug, Serialize, Deserialize)]
pub enum ApmProvider {
    #[serde(rename = "GIROPAY")]
    Giropay,
    #[serde(rename = "IDEAL")]
    Ideal,
    #[serde(rename = "PAYPAL")]
    Paypal,
    #[serde(rename = "SOFORT")]
    Sofort,
    #[serde(rename = "TESTPAY")]
    Testpay,
}

/// Indicates if any exemptions apply to this transaction.
#[derive(Debug, Serialize, Deserialize)]
pub enum ExemptStatus {
    #[serde(rename = "LOW_VALUE")]
    LowValue,
    #[serde(rename = "SCA_DELEGATION")]
    ScaDelegation,
    #[serde(rename = "SECURE_CORPORATE_PAYMENT")]
    SecureCorporatePayment,
    #[serde(rename = "TRANSACTION_RISK_ANALYSIS")]
    TransactionRiskAnalysis,
    #[serde(rename = "TRUSTED_MERCHANT")]
    TrustedMerchant,
}

/// The type of bank account associated with the payer's bank account.
#[derive(Debug, Serialize, Deserialize)]
pub enum NumberType {
    #[serde(rename = "CHECKING")]
    Checking,
    #[serde(rename = "SAVINGS")]
    Savings,
}

/// Indicates how the transaction was authorized by the merchant.
/// * CCD - Cash Concentration or Disbursement - Can be either a credit or debit application
/// where funds are wither distributed or consolidated between corporate entities.
/// * POP - Point of Sale Entry - Point of sale debit applications non-shared (POS)
/// environment. These transactions are most often initiated by the consumer via a plastic
/// access card. This is only support for normal ACH transactions
/// * PPD - Prearranged Payment and Deposits - used to credit or debit a consumer account.
/// Popularity used for payroll direct deposits and pre-authorized bill payments.
/// * TEL - Telephone-Initiated Entry - Used for the origination of a single entry debit
/// transaction to a consumer's account pursuant to a verbal authorization obtained from the
/// consumer via the telephone.
/// * WEB - Internet (Web)-Initiated Entry - Used for the origination of debit entries
/// (either Single or Recurring Entry) to a consumer's account pursuant to a to an
/// authorization that is obtained from the Receiver via the Internet.
#[derive(Debug, Serialize, Deserialize)]
pub enum SecCode {
    #[serde(rename = "CCD")]
    Ccd,
    #[serde(rename = "POP")]
    Pop,
    #[serde(rename = "PPD")]
    Ppd,
    #[serde(rename = "TEL")]
    Tel,
    #[serde(rename = "WEB")]
    Web,
}

/// Indicates if a fallback mechanism was used to obtain the card information when EMV/chip
/// did not work as expected.
/// * PREV_SUCCESS - indicates the previous transaction with this card was a success.
/// * PREV_FAILED - indicates the previous transaction with this card failed.
#[derive(Debug, Serialize, Deserialize)]
pub enum ChipCondition {
    #[serde(rename = "PREV_FAILED")]
    PrevFailed,
    #[serde(rename = "PREV_SUCCESS")]
    PrevSuccess,
}

/// Card Verification Value Indicator sent by the Merchant indicating the CVV
/// availability.
/// * ILLEGIBLE - indicates the cvv is present but cannot be read.
/// * NOT_PRESENT - indicates the cvv is not present on the card.
/// * PRESENT - indicates the cvv is present.
#[derive(Debug, Default, Serialize, Deserialize)]
pub enum CvvIndicator {
    #[serde(rename = "ILLEGIBLE")]
    Illegible,
    #[serde(rename = "NOT_PRESENT")]
    NotPresent,
    #[default]
    #[serde(rename = "PRESENT")]
    Present,
}

/// Indicates whether the card is a debit or credit card.
/// * DEBIT - indicates the card is a debit card where the funds may be present in an account
/// to fulfill the transaction amount.
/// * CREDIT - indicates the card is a credit card where the funds may be available on credit
/// to the payer to fulfill the transaction amount.
/// * FOOD_STAMP - indicates the card is an, Electronic Benefits Transfer, for food stamps.
/// * CASH_BENEFITS - indicates the card is an, Electronic Benefits Transfer, for cash
/// benefits.
/// * PREPAID - indicates the card is a prepaid card where the funds are loaded to the card
/// account to fulfill the transaction amount. Unlike a debit card, a prepaid is not linked
/// to a bank account.
#[derive(Debug, Serialize, Deserialize)]
pub enum Funding {
    #[serde(rename = "CASH_BENEFITS")]
    CashBenefits,
    #[serde(rename = "CREDIT")]
    Credit,
    #[serde(rename = "DEBIT")]
    Debit,
    #[serde(rename = "FOOD_STAMP")]
    FoodStamp,
    #[serde(rename = "PREPAID")]
    Prepaid,
}

/// Identifies who provides the digital wallet for the Payer.
#[derive(Debug, Serialize, Deserialize)]
pub enum DigitalWalletProvider {
    #[serde(rename = "APPLEPAY")]
    Applepay,
    #[serde(rename = "PAY_BY_GOOGLE")]
    PayByGoogle,
}

/// Indicates if the actual card number or a token is being used to process the
/// transaction.
/// * CARD_NUMBER - The value in the digital wallet token field is a real card number
/// (PAN)
/// * CARD_TOKEN - The value in the digital wallet token field is a temporary token in the
/// format of a card number (PAN) but is not a real card number.
#[derive(Debug, Serialize, Deserialize)]
pub enum TokenFormat {
    #[serde(rename = "CARD_NUMBER")]
    CardNumber,
    #[serde(rename = "CARD_TOKEN")]
    CardToken,
}

/// The encryption method used when sending encrypted card data to Global Payments.
#[derive(Debug, Serialize, Deserialize)]
pub enum Method {
    #[serde(rename = "KSN")]
    Ksn,
    #[serde(rename = "KTB")]
    Ktb,
}

/// Indicates how the payment method information was obtained by the Merchant for this
/// transaction.
/// * MOTO - A CNP channel entry mode where the payment method information was obtained over
/// the phone or via postal mail.
/// * ECOM - A CNP channel entry mode where the payment method was obtained via a browser.
/// * IN_APP - A CNP channel entry mode where the payment method was obtained via an
/// application and applies to digital wallets only.
/// * CHIP - A CP channel entry mode where the payment method information was obtained from a
/// chip. E.g. card is inserted into a device to read the chip.
/// * SWIPE - A CP channel entry mode where the payment method information was obtained from
/// swiping a magnetic strip. E.g. card's magnetic strip is swiped through a device to read
/// the card information.
/// * MANUAL - A CP channel entry mode where the payment method information was obtained by
/// manually keying the payment method information into the device.
/// * CONTACTLESS_CHIP -  A CP channel entry mode where the payment method information was
/// obtained by bringing the payment method to close proximity of a device. E.g. tap a cardon
/// or near a device to exchange card information.
/// * CONTACTLESS_SWIPE -  A CP channel entry mode where the payment method information was
/// obtained by bringing the payment method to close proximity of a device and also swiping
/// the card. E.g. tap a card on or near a device and swipe it through device to exchange
/// card information
/// * PHONE - A CNP channel entry mode where the payment method was obtained over the
/// phone.
/// * MAIL - A CNP channel entry mode where the payment method was obtained via postal mail.
#[derive(Debug, Default, Serialize, Deserialize)]
pub enum PaymentMethodEntryMode {
    #[serde(rename = "CHIP")]
    Chip,
    #[serde(rename = "CONTACTLESS_CHIP")]
    ContactlessChip,
    #[serde(rename = "CONTACTLESS_SWIPE")]
    ContactlessSwipe,
    #[default]
    #[serde(rename = "ECOM")]
    Ecom,
    #[serde(rename = "IN_APP")]
    InApp,
    #[serde(rename = "MAIL")]
    Mail,
    #[serde(rename = "MANUAL")]
    Manual,
    #[serde(rename = "MOTO")]
    Moto,
    #[serde(rename = "PHONE")]
    Phone,
    #[serde(rename = "SWIPE")]
    Swipe,
}

/// Indicates whether to execute the fingerprint signature functionality.
/// * ALWAYS - Always check and create the fingerprint value regardless of the result of the
/// card authorization.
/// * ON_SUCCESS - Always check and create the fingerprint value when the card authorization
/// is successful.
///
///
/// Indicates whether to store the card as part of a transaction.
/// * ALWAYS -  The card information is always stored irrespective of whether the payment
/// method authorization was successful or not.
/// * ON_SUCCESS - The card information is only storedif the payment method authorization was
/// successful.
#[derive(Debug, Serialize, Deserialize)]
pub enum Mode {
    #[serde(rename = "ALWAYS")]
    Always,
    #[serde(rename = "ON_SUCCESS")]
    OnSuccess,
}

/// Indicates the transaction processing model being executed when using stored
/// credentials.
/// * UNSCHEDULED - the transaction is adhoc or unscheduled. For example a payer visiting a
/// merchant to make purchase using the payment method stored with the merchant.
/// * RECURRING - The transaction is a repeat transaction initiated my the merchant and taken
/// using the payment method stored with the merchant, as part of an agreed schedule of
/// transactions.
/// * SUBSCRIPTION - The transaction is a repeat transaction initiated my the merchant and
/// taken using the payment method stored with the merchant, as part of an agreed schedule of
/// transactions. The amount taken is based on the usage by the payer of the good or service.
/// for example a monthly mobile phone bill.
/// * INSTALLMENT - The transaction is a repeat transaction initiated my the merchant and
/// taken using the payment method stored with the merchant, as part of an agreed schedule of
/// transactions and where the amount is known and agreed in advanced. For example the
/// payment in full of a good in fixed installments over a defined period of time.'
#[derive(Debug, Serialize, Deserialize)]
pub enum Model {
    #[serde(rename = "INSTALLMENT")]
    Installment,
    #[serde(rename = "RECURRING")]
    Recurring,
    #[serde(rename = "SUBSCRIPTION")]
    Subscription,
    #[serde(rename = "UNSCHEDULED")]
    Unscheduled,
}

/// The reason stored credentials are being used to to create a transaction.
#[derive(Debug, Serialize, Deserialize)]
pub enum Reason {
    #[serde(rename = "DELAYED")]
    Delayed,
    #[serde(rename = "INCREMENTAL")]
    Incremental,
    #[serde(rename = "NO_SHOW")]
    NoShow,
    #[serde(rename = "REAUTHORIZATION")]
    Reauthorization,
    #[serde(rename = "RESUBMISSION")]
    Resubmission,
}

/// Indiciates the order of this transaction in the sequence of a planned repeating
/// transaction processing model.
#[derive(Debug, Serialize, Deserialize)]
pub enum Sequence {
    #[serde(rename = "FIRST")]
    First,
    #[serde(rename = "LAST")]
    Last,
    #[serde(rename = "SUBSEQUENT")]
    Subsequent,
}

#[derive(Default, Debug, Serialize)]
pub struct GlobalpayRefundRequest {
    pub amount: String,
}
