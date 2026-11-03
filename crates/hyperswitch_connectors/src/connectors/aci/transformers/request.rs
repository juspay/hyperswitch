//! Request bodies, as ACI's form-urlencoded wire format names their fields.

use cards::NetworkToken;
use common_utils::{
    id_type,
    pii::{Email, IpAddress},
    types::StringMajorUnit,
};
use hyperswitch_masking::Secret;
use serde::{Deserialize, Serialize};

/// Dynamic `customParameters[key]` entries forwarded to ACI.
/// Each entry serializes as its own form field, e.g. `customParameters[paymentId]=pay_xxx`.
#[derive(Debug, Default)]
pub struct AciCustomParameters(Vec<(String, String)>);

/// OPPWA constrains `customParameters[<name>]`: the name must match `[a-zA-Z0-9\._]{3,64}` and the
/// value is AN2048. A single entry outside those bounds rejects the whole request, which would turn
/// merchant metadata — inert on every other connector — into an authorization-blocking error here.
const ACI_CUSTOM_PARAM_NAME_MIN_LEN: usize = 3;

const ACI_CUSTOM_PARAM_NAME_MAX_LEN: usize = 64;

const ACI_CUSTOM_PARAM_VALUE_MAX_LEN: usize = 2048;

/// Names the connector emits itself. The body is a flat list of pairs with no de-duplication, so a
/// merchant metadata key equal to any of these would be serialized twice and which one OPPWA
/// honours is undefined. (`3DS2_enrolled` is emitted from a separate field on the request struct
/// rather than through this newtype, but it occupies the same `customParameters` namespace.)
const ACI_RESERVED_CUSTOM_PARAM_NAMES: [&str; 3] = ["orchestrator", "paymentId", "3DS2_enrolled"];

impl AciCustomParameters {
    /// Unvalidated push, for the connector's own known-good fixed keys.
    pub fn insert(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.0.push((key.into(), value.into()));
    }

    /// Push a merchant-supplied entry, dropping it when ACI would reject the parameter or when it
    /// collides with a name the connector already emits.
    pub fn insert_merchant_supplied(&mut self, key: &str, value: String) {
        let name_len = key.chars().count();
        // `is_ascii_alphanumeric`, not `is_alphanumeric`: the latter accepts 'é', '４' and Cyrillic,
        // none of which match OPPWA's ASCII-only pattern.
        let name_is_valid = (ACI_CUSTOM_PARAM_NAME_MIN_LEN..=ACI_CUSTOM_PARAM_NAME_MAX_LEN)
            .contains(&name_len)
            && key
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '_');
        let is_reserved = ACI_RESERVED_CUSTOM_PARAM_NAMES.contains(&key);
        // Character count, not byte length — AN2048 counts characters.
        let value_is_valid =
            !value.is_empty() && value.chars().count() <= ACI_CUSTOM_PARAM_VALUE_MAX_LEN;

        if !name_is_valid || is_reserved || !value_is_valid {
            // Log the name only; the value is merchant-controlled and may carry PII.
            router_env::logger::warn!(
                custom_parameter_name = key,
                "aci: dropping customParameters entry (invalid name/value, or reserved name)"
            );
            return;
        }
        self.0.push((key.to_string(), value));
    }
}

impl Serialize for AciCustomParameters {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeMap;
        let mut map = serializer.serialize_map(Some(self.0.len()))?;
        for (k, v) in &self.0 {
            map.serialize_entry(&format!("customParameters[{k}]"), v)?;
        }
        map.end()
    }
}

/// Values for OPPWA's `standingInstruction.industryPractice`, the merchant-initiated follow-up
/// category for a card-on-file agreement.
///
/// The full documented set is
/// `INCREMENTAL_AUTH | RESUBMISSION | REAUTHORIZATION | DELAYED_CHARGES | NO_SHOW`. Only
/// RESUBMISSION is modelled: it is the sole value `common_enums::MitCategory` carries a signal
/// for, and unconstructible variants are dead code. Extend this enum as `MitCategory` grows. The
/// acquirer sheets map the value onwards — ABSA to `CH.RecurringTypePaymentInd`, Nedbank to
/// `127.22 MessageReasonCode` (3901), CyberSource to
/// `processingInformation.authorizationOptions.initiator.merchantInitiatedTransaction.reason`.
#[derive(Debug, Serialize, Clone)]
pub enum AciIndustryPractice {
    #[serde(rename = "RESUBMISSION")]
    Resubmission,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum AciTestMode {
    #[serde(rename = "EXTERNAL")]
    External,
    #[serde(rename = "INTERNAL")]
    Internal,
}

/// OPPWA's `customer.browser.challengeWindow` (`N1`): `1` 250x400, `2` 390x400, `3` 500x600,
/// `4` 600x400, `5` full screen. It tells the ACS which window the challenge will be rendered
/// into so it can format its content to fit; we render full screen, so `5` is always what we
/// send.
///
/// The API Reference documents the format as `N1` while ACI's 3DS Guide writes the same values
/// as `01`..`05` (PT-2524). The single digit is what the format regex admits, so that is what
/// goes on the wire.
pub(crate) const ACI_CHALLENGE_WINDOW_FULL_SCREEN: &str = "5";

#[derive(Debug, Serialize, Default)]
pub struct AciCustomerBrowserInfo {
    #[serde(rename = "customer.browser.acceptHeader")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub accept_header: Option<String>,
    #[serde(rename = "customer.browser.language")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
    #[serde(rename = "customer.browser.screenHeight")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub screen_height: Option<String>,
    #[serde(rename = "customer.browser.screenWidth")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub screen_width: Option<String>,
    #[serde(rename = "customer.browser.timezone")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timezone: Option<String>,
    #[serde(rename = "customer.browser.userAgent")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_agent: Option<String>,
    #[serde(rename = "customer.browser.javascriptEnabled")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub javascript_enabled: Option<bool>,
    #[serde(rename = "customer.browser.javaEnabled")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub java_enabled: Option<bool>,
    #[serde(rename = "customer.browser.screenColorDepth")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color_depth: Option<String>,
    #[serde(rename = "customer.browser.challengeWindow")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub challenge_window: Option<String>,
}

#[derive(Debug, Serialize, Default)]
pub struct AciCustomerData {
    #[serde(rename = "customer.ip")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ip: Option<Secret<String, IpAddress>>,
    #[serde(rename = "customer.email")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<Email>,
    #[serde(rename = "customer.givenName")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub given_name: Option<Secret<String>>,
    #[serde(rename = "customer.surname")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub surname: Option<Secret<String>>,
    /// OPPWA's `customer.merchantCustomerId` (`AN255`): "the ID that identifies the shopper in
    /// the shop's system". Lives here rather than on any one payment method's struct so that
    /// every flow sends it, and sends it exactly once.
    #[serde(rename = "customer.merchantCustomerId")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub merchant_customer_id: Option<Secret<id_type::CustomerId>>,
}

#[derive(Debug, Serialize, Default)]
pub struct AciBillingAddress {
    #[serde(rename = "billing.street1")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub street1: Option<Secret<String>>,
    #[serde(rename = "billing.city")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub city: Option<String>,
    #[serde(rename = "billing.state")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state: Option<Secret<String>>,
    #[serde(rename = "billing.country")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub country: Option<api_models::enums::CountryAlpha2>,
    #[serde(rename = "billing.postcode")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub postcode: Option<Secret<String>>,
}

#[derive(Debug, Serialize, Default)]
pub struct AciExternalThreeDsData {
    #[serde(rename = "threeDSecure.eci")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub eci: Option<String>,
    #[serde(rename = "threeDSecure.verificationId")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verification_id: Option<Secret<String>>,
    #[serde(rename = "threeDSecure.dsTransactionId")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ds_transaction_id: Option<String>,
    #[serde(rename = "threeDSecure.acsTransactionId")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub acs_transaction_id: Option<String>,
    /// OPPWA `threeDSecure.authenticationStatus`, format AN1, legal values `[YIAUN]`.
    ///
    /// Held as a `String` deliberately: `enums::TransactionStatus` can also represent R/C/D, which
    /// OPPWA rejects, so the narrowing is explicit in `map_authentication_status` rather than
    /// delegated to serde.
    #[serde(rename = "threeDSecure.authenticationStatus")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub authentication_status: Option<String>,
    /// OPPWA `threeDSecure.version` — 3DS protocol version as `major.minor`, e.g. `2.2`.
    #[serde(rename = "threeDSecure.version")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum AciRecurringType {
    Initial,
    Repeated,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AciPaymentsRequest {
    #[serde(flatten)]
    pub txn_details: TransactionDetails,
    #[serde(flatten)]
    pub payment_method: PaymentDetails,
    #[serde(flatten)]
    pub instruction: Option<Instruction>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shopper_result_url: Option<String>,
    #[serde(rename = "customParameters[3DS2_enrolled]")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub three_ds_two_enrolled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recurring_type: Option<AciRecurringType>,
    /// The channel the payment came in through (`EC`/`MO`/`TO`/`PO`/`PM`/`MOTO`), from the MCA
    /// metadata or, when that is silent, the request's `payment_channel`. Emitted as
    /// `transactionCategory` by the struct-level camelCase rename.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transaction_category: Option<AciTransactionCategory>,
    #[serde(rename = "merchantTransactionId")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub merchant_transaction_id: Option<String>,
    /// EXTERNAL: forwards to processor's test system. INTERNAL: routes to ACI simulator.
    /// Omitted on live transactions.
    #[serde(rename = "testMode")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub test_mode: Option<AciTestMode>,
    #[serde(flatten)]
    pub customer_browser_info: AciCustomerBrowserInfo,
    #[serde(flatten)]
    pub customer_data: AciCustomerData,
    #[serde(flatten)]
    pub billing_address: AciBillingAddress,
    #[serde(flatten)]
    pub external_three_ds: AciExternalThreeDsData,
    #[serde(flatten)]
    pub custom_parameters: AciCustomParameters,
    /// OPPWA's `descriptor`: "can be used to populate all or part of the Merchant Name
    /// descriptor, which often appears on the first line of the shopper's statement". Emitted
    /// under its own name by the struct-level camelCase rename.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub descriptor: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransactionDetails {
    pub entity_id: Secret<String>,
    pub amount: StringMajorUnit,
    pub currency: String,
    pub payment_type: AciPaymentType,
}

#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
pub enum PaymentDetails {
    #[serde(rename = "card")]
    AciCard(Box<CardDetails>),
    BankRedirect(Box<BankRedirectionPMData>),
    Wallet(Box<WalletPMData>),
    /// Carries no fields: a mandate payment addresses the stored credential through the request
    /// URL (`/v1/registrations/{id}/payments`), so no instrument data belongs in the body.
    Mandate,
    AciNetworkToken(Box<AciNetworkTokenData>),
    AciApplePayEncrypted(Box<AciApplePayEncryptedData>),
    AciGooglePayEncrypted(Box<AciGooglePayEncryptedData>),
    AciSamsungPay(Box<AciSamsungPayData>),
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum AciTokenAccountType {
    Network,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AciNetworkTokenData {
    #[serde(rename = "tokenAccount.type")]
    pub token_type: AciTokenAccountType,
    #[serde(rename = "tokenAccount.number")]
    pub token_number: NetworkToken,
    #[serde(rename = "tokenAccount.expiryMonth")]
    pub token_expiry_month: Secret<String>,
    #[serde(rename = "tokenAccount.expiryYear")]
    pub token_expiry_year: Secret<String>,
    #[serde(rename = "tokenAccount.cryptogram")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token_cryptogram: Option<Secret<String>>,
    #[serde(rename = "threeDSecure.eci")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub eci: Option<String>,
    #[serde(rename = "paymentBrand")]
    pub payment_brand: PaymentBrand,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AciApplePayEncryptedData {
    #[serde(rename = "applePay.paymentToken")]
    pub payment_token: Secret<String>,
    /// Required by ACI: indicates the Apple Pay integration channel.
    /// "web" for browser-based flows, "app" for native iOS app flows.
    #[serde(rename = "applePay.source")]
    pub source: String,
    #[serde(rename = "paymentBrand")]
    pub payment_brand: PaymentBrand,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AciGooglePayEncryptedData {
    #[serde(rename = "googlePay.paymentToken")]
    pub payment_token: Secret<String>,
    /// Required by ACI: indicates the Google Pay integration channel.
    /// "web" for browser-based flows, "app" for native Android app flows.
    #[serde(rename = "googlePay.source")]
    pub source: String,
    #[serde(rename = "paymentBrand")]
    pub payment_brand: PaymentBrand,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AciSamsungPayData {
    #[serde(rename = "samsungPay.paymentToken")]
    pub payment_token: Secret<String>,
    /// Required by ACI: Samsung Pay is always app-based.
    #[serde(rename = "samsungPay.source")]
    pub source: String,
    #[serde(rename = "paymentBrand")]
    pub payment_brand: PaymentBrand,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BankRedirectionPMData {
    pub(super) payment_brand: PaymentBrand,
    #[serde(rename = "bankAccount.country")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) bank_account_country: Option<api_models::enums::CountryAlpha2>,
    #[serde(rename = "bankAccount.bankName")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) bank_account_bank_name: Option<common_enums::BankNames>,
    #[serde(rename = "bankAccount.bic")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) bank_account_bic: Option<Secret<String>>,
    #[serde(rename = "bankAccount.iban")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) bank_account_iban: Option<Secret<String>>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WalletPMData {
    pub(super) payment_brand: PaymentBrand,
    #[serde(rename = "virtualAccount.accountId")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) account_id: Option<Secret<String>>,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PaymentBrand {
    Eps,
    Eft,
    Ideal,
    Giropay,
    Sofortueberweisung,
    InteracOnline,
    Przelewy,
    Trustly,
    Mbway,
    #[serde(rename = "ALIPAY")]
    AliPay,
    Klarna,
    /// Wallet brands for encrypted token passthrough. OPPWA requires these on
    /// `paymentBrand` — sending the underlying card network (VISA/MASTER) makes the
    /// gateway treat the request as a PAN payment and reject the wallet token.
    #[serde(rename = "APPLEPAY")]
    ApplePay,
    #[serde(rename = "GOOGLEPAY")]
    GooglePay,
    #[serde(rename = "SAMSUNGPAY")]
    SamsungPay,
    // Card network brands
    #[serde(rename = "VISA")]
    Visa,
    #[serde(rename = "MASTER")]
    Mastercard,
    #[serde(rename = "AMEX")]
    AmericanExpress,
    #[serde(rename = "JCB")]
    Jcb,
    #[serde(rename = "DINERS")]
    DinersClub,
    #[serde(rename = "DISCOVER")]
    Discover,
    #[serde(rename = "UNIONPAY")]
    UnionPay,
    #[serde(rename = "MAESTRO")]
    Maestro,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize)]
pub struct CardDetails {
    #[serde(rename = "card.number")]
    pub card_number: cards::CardNumber,
    #[serde(rename = "card.holder")]
    pub card_holder: Secret<String>,
    #[serde(rename = "card.expiryMonth")]
    pub card_expiry_month: Secret<String>,
    #[serde(rename = "card.expiryYear")]
    pub card_expiry_year: Secret<String>,
    /// Optional so it can be omitted on a merchant-initiated REPEATED charge: the spec forbids
    /// shopper authentication data (CVV included) when the shopper is not present. Always sent on
    /// cardholder-present legs.
    #[serde(rename = "card.cvv")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub card_cvv: Option<Secret<String>>,
    #[serde(rename = "paymentBrand")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payment_brand: Option<PaymentBrand>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum InstructionMode {
    Initial,
    /// ACI API uses REPEATED on the wire (confirmed by live rejection when
    /// sending SUBSEQUENT despite some local reference docs listing it).
    Repeated,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum InstructionType {
    Unscheduled,
    Recurring,
    Installment,
}

/// OPPWA `transactionCategory`. The documented set is `EC | MO | TO | PO | PM | MOTO`.
///
/// Hyperswitch distinguishes mail from telephone order, so the request-level `payment_channel`
/// mapping emits the specific `MO`/`TO` codes rather than the combined `MOTO`. `PO` (point of
/// sale), `PM` (mPOS) and `MOTO` have no corresponding `PaymentChannel` variant and are reachable
/// only through the MCA metadata, which `Channel::resolve` reads first.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum AciTransactionCategory {
    #[serde(rename = "EC")]
    Ecommerce,
    #[serde(rename = "MO")]
    MailOrder,
    #[serde(rename = "TO")]
    TelephoneOrder,
    #[serde(rename = "PO")]
    PointOfSale,
    #[serde(rename = "PM")]
    MobilePointOfSale,
    #[serde(rename = "MOTO")]
    MailOrTelephoneOrder,
}

/// Values for OPPWA's `standingInstruction.recurringType`, which declares whether the amounts in a
/// recurring series vary. Maps onwards to MPGS `agreement.amountVariability` and CyberSource
/// `recurringPaymentInformation.amountType`.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AciRecurringAmountType {
    /// Variable amount, fixed frequency — e.g. a metered utility bill.
    StandingOrder,
    /// Fixed amount, fixed frequency — e.g. a subscription.
    Subscription,
}

#[derive(Debug, Clone, Serialize)]
pub enum InstructionSource {
    #[serde(rename = "CIT")]
    CardholderInitiatedTransaction,
    #[serde(rename = "MIT")]
    MerchantInitiatedTransaction,
}

#[derive(Debug, Default, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Instruction {
    // The three fields that describe the agreement are optional only so that a MOTO order can be
    // submitted with `createRegistration` and no `standingInstruction.*` block at all. Every
    // non-MOTO instruction sets all three; see `Channel::declare`.
    #[serde(rename = "standingInstruction.mode")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) mode: Option<InstructionMode>,

    #[serde(rename = "standingInstruction.type")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) transaction_type: Option<InstructionType>,

    #[serde(rename = "standingInstruction.source")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) source: Option<InstructionSource>,

    #[serde(rename = "standingInstruction.initialTransactionId")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) initial_transaction_id: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) create_registration: Option<bool>,

    /// OPPWA's merchant-initiated follow-up category.
    ///
    /// This was previously serialized as `standingInstruction.reason`, which is not an OPPWA
    /// parameter — it appears in neither the API reference nor any of the four acquirer
    /// integration sheets, so the indicator never reached the acquirer. The correct key is
    /// `standingInstruction.industryPractice`, used verbatim in Nedbank's own worked example.
    #[serde(rename = "standingInstruction.industryPractice")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub industry_practice: Option<AciIndustryPractice>,

    /// Amount variability of a recurring agreement. Required for all Mastercard CIT and MIT, and
    /// for Visa recurring on India-issued cards; MPGS marks it mandatory for card-on-file CITs.
    ///
    /// Distinct from the legacy top-level `recurringType` (INITIAL/REPEATED) on
    /// [`AciPaymentsRequest`] — different wire key, different meaning. Do not conflate the two.
    #[serde(rename = "standingInstruction.recurringType")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recurring_amount_type: Option<AciRecurringAmountType>,

    /// Date after which no further authorizations will be performed, `yyyy-mm-dd`. Sent on the
    /// INITIAL leg only — the expiry is an attribute of the agreement, not of each charge.
    #[serde(rename = "standingInstruction.expiry")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expiry: Option<String>,

    /// Minimum days between authorizations (N4).
    ///
    /// Always `None`: hyperswitch carries no schedule interval anywhere. `RecurringDetails` has no
    /// frequency variant, and `InstallmentData.billing_frequency` means installment billing cadence
    /// rather than "minimum days between authorizations", so synthesising a value from it would be
    /// wrong. MPGS marks this mandatory for recurring/installment CITs, so if certification pushes
    /// back the honest fix is a merchant-level default read from connector metadata — a new
    /// configuration surface, and its own change.
    #[serde(rename = "standingInstruction.frequency")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frequency: Option<String>,

    /// Mastercard Economically Related Transaction ID (TLID).
    /// Returned by Mastercard on initial CIT and must be echoed on all subsequent MITs.
    /// During the transitional period both this and `initialTransactionId` must be sent.
    #[serde(rename = "standingInstruction.agreementId")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agreement_id: Option<String>,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize)]
pub struct BankDetails {
    #[serde(rename = "bankAccount.holder")]
    pub account_holder: Secret<String>,
}

/// OPPWA's `paymentType`: the transaction type.
///
/// A payment is created as `DB` or `PA` (see `PaymentPurpose`); `CP`, `RV` and
/// `RF` act on an existing one (see `transaction`). `CD` is never sent, but responses
/// may carry it.
#[derive(Debug, Default, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub enum AciPaymentType {
    /// Preauthorization: reserve funds for a later capture.
    #[serde(rename = "PA")]
    Preauthorization,
    /// Debit: authorize and capture in one.
    #[default]
    #[serde(rename = "DB")]
    Debit,
    /// Credit: an unreferenced payout.
    #[serde(rename = "CD")]
    Credit,
    /// Capture all or part of a preauthorization.
    #[serde(rename = "CP")]
    Capture,
    /// Reversal: void a preauthorization or a same-day debit.
    #[serde(rename = "RV")]
    Reversal,
    /// Refund all or part of a captured payment.
    #[serde(rename = "RF")]
    Refund,
}

#[cfg(test)]
mod tests {

    use super::*;
    use crate::connectors::aci::test_fixtures::*;

    // ---------------------------------------------------------------------
    // Card-on-file: request-side wire format
    // ---------------------------------------------------------------------

    /// The whole point of the `standingInstruction.*` block is the exact wire names. These are
    /// dotted form keys, not a nested object, and one wrong name is silently ignored by OPPWA —
    /// which is how `standingInstruction.reason` survived. Serializing the struct pins every key.
    #[test]
    fn instruction_serializes_the_documented_wire_names() {
        let instruction = Instruction {
            mode: Some(InstructionMode::Repeated),
            transaction_type: Some(InstructionType::Recurring),
            source: Some(InstructionSource::MerchantInitiatedTransaction),
            initial_transaction_id: Some("MCC00013N0417".to_string()),
            create_registration: None,
            industry_practice: Some(AciIndustryPractice::Resubmission),
            recurring_amount_type: Some(AciRecurringAmountType::StandingOrder),
            expiry: Some("2027-12-31".to_string()),
            frequency: None,
            agreement_id: Some("TLID-123".to_string()),
        };

        let encoded = serde_urlencoded::to_string(&instruction)
            .expect("instruction should form-urlencode cleanly");

        for expected in [
            "standingInstruction.mode=REPEATED",
            "standingInstruction.type=RECURRING",
            "standingInstruction.source=MIT",
            "standingInstruction.initialTransactionId=MCC00013N0417",
            "standingInstruction.industryPractice=RESUBMISSION",
            "standingInstruction.recurringType=STANDING_ORDER",
            "standingInstruction.expiry=2027-12-31",
            "standingInstruction.agreementId=TLID-123",
        ] {
            assert!(
                encoded.contains(expected),
                "missing `{expected}` in encoded instruction: {encoded}"
            );
        }

        assert!(
            !encoded.contains("standingInstruction.reason"),
            "`standingInstruction.reason` is not an OPPWA parameter and must never be sent: {encoded}"
        );
        assert!(
            !encoded.contains("standingInstruction.frequency"),
            "an absent frequency must be omitted, not sent empty: {encoded}"
        );
    }

    /// A single out-of-spec `customParameters` entry rejects the whole request, so merchant
    /// metadata — inert on other connectors — must be filtered rather than forwarded verbatim.
    #[test]
    fn custom_parameters_drops_entries_aci_would_reject() {
        let mut params = AciCustomParameters::default();
        params.insert_merchant_supplied("order_id", "ord_123".to_string());
        params.insert_merchant_supplied("valid.key", "v".to_string());
        // Too short, illegal character, non-ASCII, empty value, oversized value, reserved name.
        params.insert_merchant_supplied("id", "x".to_string());
        params.insert_merchant_supplied("order-id", "x".to_string());
        params.insert_merchant_supplied("ordré", "x".to_string());
        params.insert_merchant_supplied("empty_value", String::new());
        params.insert_merchant_supplied("too_long", "x".repeat(ACI_CUSTOM_PARAM_VALUE_MAX_LEN + 1));
        params.insert_merchant_supplied("paymentId", "spoofed".to_string());

        let encoded =
            serde_urlencoded::to_string(&params).expect("custom parameters should encode cleanly");

        assert!(encoded.contains("customParameters%5Border_id%5D=ord_123"));
        assert!(encoded.contains("customParameters%5Bvalid.key%5D=v"));
        for dropped in ["id", "order-id", "empty_value", "too_long", "paymentId"] {
            assert!(
                !encoded.contains(&format!("customParameters%5B{dropped}%5D")),
                "`{dropped}` should have been dropped: {encoded}"
            );
        }
    }

    /// Encrypted wallet passthrough must send the wallet brand on `paymentBrand`.
    /// OPPWA rejects `*.cardBrand` (`200.300.404` — not an allowed parameter) and
    /// treats a card-network `paymentBrand` as a PAN payment.
    #[test]
    fn encrypted_wallet_payloads_carry_wallet_payment_brand() {
        let apple = AciApplePayEncryptedData {
            payment_token: Secret::new("apple-token".to_string()),
            source: "web".to_string(),
            payment_brand: PaymentBrand::ApplePay,
        };
        let body = serde_urlencoded::to_string(&apple).expect("apple pay payload serializes");
        assert!(body.contains("applePay.paymentToken=apple-token"), "{body}");
        assert!(body.contains("paymentBrand=APPLEPAY"), "{body}");
        assert!(!body.contains("cardBrand"), "{body}");

        let google = AciGooglePayEncryptedData {
            payment_token: Secret::new("google-token".to_string()),
            source: "web".to_string(),
            payment_brand: PaymentBrand::GooglePay,
        };
        let body = serde_urlencoded::to_string(&google).expect("google pay payload serializes");
        assert!(
            body.contains("googlePay.paymentToken=google-token"),
            "{body}"
        );
        assert!(body.contains("paymentBrand=GOOGLEPAY"), "{body}");
        assert!(!body.contains("cardBrand"), "{body}");

        let samsung = AciSamsungPayData {
            payment_token: Secret::new("samsung-token".to_string()),
            source: "app".to_string(),
            payment_brand: PaymentBrand::SamsungPay,
        };
        let body = serde_urlencoded::to_string(&samsung).expect("samsung pay payload serializes");
        assert!(
            body.contains("samsungPay.paymentToken=samsung-token"),
            "{body}"
        );
        assert!(body.contains("paymentBrand=SAMSUNGPAY"), "{body}");
        assert!(!body.contains("cardBrand"), "{body}");
    }

    /// The MCA-level codes serialize to the documented wire values.
    #[test]
    fn transaction_category_codes_serialize_to_wire_values() {
        for (category, wire) in [
            (AciTransactionCategory::PointOfSale, "PO"),
            (AciTransactionCategory::MobilePointOfSale, "PM"),
            (AciTransactionCategory::MailOrTelephoneOrder, "MOTO"),
        ] {
            assert_eq!(
                serde_json::to_value(category).expect("category serializes"),
                serde_json::json!(wire)
            );
        }
    }

    /// Mandate setup with a CVV-optional card must omit `card.cvv` from the body rather than
    /// sending it empty.
    #[test]
    fn mandate_setup_without_cvc_omits_cvv() {
        use std::str::FromStr;

        let payment_details = PaymentDetails::AciCard(Box::new(CardDetails {
            card_number: cards::CardNumber::from_str("4111111111111111")
                .expect("valid visa test PAN"),
            card_holder: Secret::new("Jane Cardholder".to_string()),
            card_expiry_month: Secret::new("03".to_string()),
            card_expiry_year: Secret::new("2030".to_string()),
            card_cvv: None,
            payment_brand: Some(PaymentBrand::Visa),
        }));

        let body =
            serde_urlencoded::to_string(&payment_details).expect("card details serialize cleanly");
        assert!(!body.contains("card.cvv"), "{body}");
        assert!(body.contains("card.number=4111111111111111"), "{body}");
    }

    // ---------------------------------------------------------------------
    // Customer parameters
    //
    // `AciPaymentsRequest` flattens five sub-structs into a single flat
    // `serde_urlencoded` body, so a `customer.*` key is only provably present — and provably
    // unique — on the composed request. Serializing a sub-struct on its own proves neither.
    // ---------------------------------------------------------------------

    /// [PT-4117] `customer.browser.challengeWindow` was declared on the request struct but
    /// hardcoded to `None`, so the ACS never learned which window the challenge would be
    /// rendered into. We render full screen, which OPPWA writes as `5`.
    #[test]
    fn full_screen_challenge_window_rides_with_the_browser_block() {
        let request = customer_request(
            populated_browser_info(),
            AciCustomerData::default(),
            PaymentDetails::Mandate,
            None,
        );
        let body = serde_urlencoded::to_string(&request).expect("request serializes cleanly");

        assert_eq!(
            key_occurrences(&body, "customer.browser.challengeWindow"),
            1,
            "{body}"
        );
        assert!(
            body.contains("customer.browser.challengeWindow=5"),
            "{body}"
        );

        // A browserless flow (MIT/MOTO) sends no `customer.browser.*` keys at all, and the
        // challenge window must not be the one that sneaks in.
        let browserless = customer_request(
            AciCustomerBrowserInfo::default(),
            AciCustomerData::default(),
            PaymentDetails::Mandate,
            None,
        );
        let body = serde_urlencoded::to_string(&browserless).expect("request serializes cleanly");
        assert!(!body.contains("customer.browser."), "{body}");
    }

    /// [PT-4117] `customer.merchantCustomerId` used to hang off `BankRedirectionPMData`, so only
    /// Trustly ever sent it. It now rides the shared customer block — which means the Trustly
    /// path must no longer carry its own copy, or the flat body would hold the key twice.
    #[test]
    fn merchant_customer_id_is_sent_once_for_every_payment_method() {
        let card = customer_request(
            populated_browser_info(),
            shopper_customer_data(),
            PaymentDetails::Mandate,
            None,
        );
        let body = serde_urlencoded::to_string(&card).expect("request serializes cleanly");
        assert_eq!(
            key_occurrences(&body, "customer.merchantCustomerId"),
            1,
            "{body}"
        );
        assert!(
            body.contains("customer.merchantCustomerId=cus_shopper_1"),
            "{body}"
        );

        let trustly = customer_request(
            populated_browser_info(),
            shopper_customer_data(),
            PaymentDetails::BankRedirect(Box::new(BankRedirectionPMData {
                payment_brand: PaymentBrand::Trustly,
                bank_account_country: None,
                bank_account_bank_name: None,
                bank_account_bic: None,
                bank_account_iban: None,
            })),
            None,
        );
        let body = serde_urlencoded::to_string(&trustly).expect("request serializes cleanly");
        assert_eq!(
            key_occurrences(&body, "customer.merchantCustomerId"),
            1,
            "Trustly must not duplicate the key: {body}"
        );
    }
}
