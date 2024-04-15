use std::collections::HashMap;

use common_utils::pii::Email;
use serde::{Deserialize, Serialize};

use crate::types::api::MessageCategory;

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(untagged)]
pub enum SingleOrListElement<T> {
    Single(T),
    List(Vec<T>),
}

impl<T> SingleOrListElement<T> {
    pub fn new_single(value: T) -> Self {
        Self::Single(value)
    }

    pub fn new_list(value: Vec<T>) -> Self {
        Self::List(value)
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub enum NetceteraDeviceChannel {
    #[serde(rename = "01")]
    AppBased,
    #[serde(rename = "02")]
    Browser,
    #[serde(rename = "03")]
    ThreeDsRequestorInitiated,
}

impl From<api_models::payments::DeviceChannel> for NetceteraDeviceChannel {
    fn from(value: api_models::payments::DeviceChannel) -> Self {
        match value {
            api_models::payments::DeviceChannel::App => Self::AppBased,
            api_models::payments::DeviceChannel::Browser => Self::Browser,
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub enum NetceteraMessageCategory {
    #[serde(rename = "01")]
    PaymentAuthentication,
    #[serde(rename = "02")]
    NonPaymentAuthentication,
}

impl From<MessageCategory> for NetceteraMessageCategory {
    fn from(value: MessageCategory) -> Self {
        match value {
            MessageCategory::NonPayment => Self::NonPaymentAuthentication,
            MessageCategory::Payment => Self::PaymentAuthentication,
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub enum ThreeDSMethodCompletionIndicator {
    /// Successfully completed
    Y,
    /// Did not successfully complete
    N,
    /// Unavailable - 3DS Method URL was not present in the PRes message data
    U,
}
impl From<api_models::payments::ThreeDsCompletionIndicator> for ThreeDSMethodCompletionIndicator {
    fn from(value: api_models::payments::ThreeDsCompletionIndicator) -> Self {
        match value {
            api_models::payments::ThreeDsCompletionIndicator::Success => Self::Y,
            api_models::payments::ThreeDsCompletionIndicator::Failure => Self::N,
            api_models::payments::ThreeDsCompletionIndicator::NotAvailable => Self::U,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ThreeDSRequestor {
    #[serde(rename = "threeDSRequestorAuthenticationInd")]
    pub three_ds_requestor_authentication_ind: ThreeDSRequestorAuthenticationIndicator,
    /// Format of this field was changed with EMV 3DS 2.3.1 version:
    /// In versions prior to 2.3.1, this field is a single object.
    /// Starting from EMVCo version 2.3.1, this field is now an array of objects. Accepted value length is 1-3 elements.
    ///   
    /// This field is optional, but recommended to include.
    #[serde(rename = "threeDSRequestorAuthenticationInfo")]
    pub three_ds_requestor_authentication_info:
        Option<SingleOrListElement<ThreeDSRequestorAuthenticationInformation>>,
    #[serde(rename = "threeDSRequestorChallengeInd")]
    pub three_ds_requestor_challenge_ind:
        Option<SingleOrListElement<ThreeDSRequestorChallengeIndicator>>,
    #[serde(rename = "threeDSRequestorPriorAuthenticationInfo")]
    pub three_ds_requestor_prior_authentication_info:
        Option<SingleOrListElement<ThreeDSRequestorPriorTransactionAuthenticationInformation>>,
    #[serde(rename = "threeDSRequestorDecReqInd")]
    pub three_ds_requestor_dec_req_ind: Option<ThreeDSRequestorDecoupledRequestIndicator>,
    /// Indicates the maximum amount of time that the 3DS Requestor will wait for an ACS to provide the results
    /// of a Decoupled Authentication transaction (in minutes). Valid values are between 1 and 10080.
    ///
    /// The field is optional and if value is not present, the expected action is for the ACS to interpret it as
    /// 10080 minutes (7 days).
    /// Available for supporting EMV 3DS 2.2.0 and later versions.
    ///
    /// Starting from EMV 3DS 2.3.1:
    /// This field is required if threeDSRequestorDecReqInd = Y, F or B.
    #[serde(rename = "threeDSRequestorDecMaxTime")]
    pub three_ds_requestor_dec_max_time: Option<u32>,
    /// External IP address (i.e., the device public IP address) used by the 3DS Requestor App when it connects to the
    /// 3DS Requestor environment. The value length is maximum 45 characters. Accepted values are:
    ///
    ///     IPv4 address is represented in the dotted decimal f. Refer to RFC 791.
    ///     IPv6 address. Refer to RFC 4291.
    ///
    /// This field is required when deviceChannel = 01 (APP) and unless market or regional mandate restricts sending
    /// this information.
    /// Available for supporting EMV 3DS 2.3.1 and later versions.
    pub app_ip: Option<String>,
    /// Indicate if the 3DS Requestor supports the SPC authentication.
    ///
    /// The accepted values are:
    ///
    /// - Y -> Supported
    ///
    /// This field is required if deviceChannel = 02 (BRW) and it is supported by the 3DS Requestor.
    /// Available for supporting EMV 3DS 2.3.1 and later versions.
    #[serde(rename = "threeDSRequestorSpcSupport")]
    pub three_ds_requestor_spc_support: Option<String>,
    /// Reason that the SPC authentication was not completed.
    /// Accepted value length is 2 characters.
    ///
    /// The accepted values are:
    ///
    /// - 01 -> SPC did not run or did not successfully complete
    /// - 02 -> Cardholder cancels the SPC authentication
    ///
    /// This field is required if deviceChannel = 02 (BRW) and the 3DS Requestor attempts to invoke SPC API and there is an
    /// error.
    /// Available for supporting EMV 3DS 2.3.1 and later versions.
    pub spc_incomp_ind: Option<String>,
}

/// Indicates the type of Authentication request.
///
/// This data element provides additional information to the ACS to determine the best approach for handling an authentication request.
///
/// This value is used for App-based and Browser flows.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum ThreeDSRequestorAuthenticationIndicator {
    #[serde(rename = "01")]
    Payment,
    #[serde(rename = "02")]
    Recurring,
    #[serde(rename = "03")]
    Installment,
    #[serde(rename = "04")]
    AddCard,
    #[serde(rename = "05")]
    MaintainCard,
    #[serde(rename = "06")]
    CardholderVerification,
    #[serde(rename = "07")]
    BillingAgreement,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ThreeDSRequestorAuthenticationInformation {
    /// Mechanism used by the Cardholder to authenticate to the 3DS Requestor. Accepted values are:
    /// - 01 -> No 3DS Requestor authentication occurred (i.e. cardholder "logged in" as guest)
    /// - 02 -> Login to the cardholder account at the 3DS Requestor system using 3DS Requestor's own credentials
    /// - 03 -> Login to the cardholder account at the 3DS Requestor system using federated ID
    /// - 04 -> Login to the cardholder account at the 3DS Requestor system using issuer credentials
    /// - 05 -> Login to the cardholder account at the 3DS Requestor system using third-party authentication
    /// - 06 -> Login to the cardholder account at the 3DS Requestor system using FIDO Authenticator.
    ///
    /// The next values are accepted as well if 3DS Server initiates authentication with EMV 3DS 2.2.0 version or greater (required protocol version can be set in ThreeDSServerAuthenticationRequest#preferredProtocolVersion field):
    /// - 07 -> Login to the cardholder account at the 3DS Requestor system using FIDO Authenticator (FIDO assurance data signed).
    /// - 08 -> SRC Assurance Data.
    /// - Additionally, 80-99 can be used for PS-specific values, regardless of protocol version.
    #[serde(rename = "threeDSReqAuthMethod")]
    pub three_ds_req_auth_method: String,
    /// Date and time converted into UTC of the cardholder authentication. Field is limited to 12 characters and accepted format is YYYYMMDDHHMM
    #[serde(rename = "threeDSReqAuthTimestamp")]
    pub three_ds_req_auth_timestamp: String,
    /// Data that documents and supports a specific authentication process. In the current version of the specification, this data element is not defined in detail, however the intention is that for each 3DS Requestor Authentication Method, this field carry data that the ACS can use to verify the authentication process.
    /// For example, if the 3DS Requestor Authentication Method is:
    ///
    ///  - 03 -> then this element can carry information about the provider of the federated ID and related information
    ///  - 06 -> then this element can carry the FIDO attestation data (incl. the signature)
    ///  - 07 -> then this element can carry FIDO Attestation data with the FIDO assurance data signed.
    ///  - 08 -> then this element can carry the SRC assurance data.
    #[serde(rename = "threeDSReqAuthData")]
    pub three_ds_req_auth_data: Option<String>,
}

/// Indicates whether a challenge is requested for this transaction. For example: For 01-PA, a 3DS Requestor may have
/// concerns about the transaction, and request a challenge. For 02-NPA, a challenge may be necessary when adding a new
/// card to a wallet.
///    
/// This field is optional. The accepted values are:
///    
///  - 01 -> No preference
///  - 02 -> No challenge requested
///  - 03 -> Challenge requested: 3DS Requestor Preference
///  - 04 -> Challenge requested: Mandate.
///  The next values are accepted as well if 3DS Server initiates authentication with EMV 3DS 2.2.0 version
/// or greater (required protocol version can be set in
///   ThreeDSServerAuthenticationRequest#preferredProtocolVersion field):
///
///  - 05 -> No challenge requested (transactional risk analysis is already performed)
///  - 06 -> No challenge requested (Data share only)
///  - 07 -> No challenge requested (strong consumer authentication is already performed)
///  - 08 -> No challenge requested (utilise whitelist exemption if no challenge required)
///  - 09 -> Challenge requested (whitelist prompt requested if challenge required).
///  - Additionally, 80-99 can be used for PS-specific values, regardless of protocol version.
///    
/// If the element is not provided, the expected action is that the ACS would interpret as 01 -> No preference.
///    
/// Format of this field was changed with EMV 3DS 2.3.1 version:
/// In versions prior to 2.3.1, this field is a String.
/// Starting from EMVCo version 2.3.1, this field is now an array of objects. Accepted value length is 1-2 elements.
/// When providing two preferences, the 3DS Requestor ensures that they are in preference order and are not
/// conflicting. For example, 02 = No challenge requested and 04 = Challenge requested (Mandate).
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum ThreeDSRequestorChallengeIndicator {
    #[serde(rename = "01")]
    NoPreference,
    #[serde(rename = "02")]
    NoChallengeRequested,
    #[serde(rename = "03")]
    ChallengeRequested3DSRequestorPreference,
    #[serde(rename = "04")]
    ChallengeRequestedMandate,
    #[serde(rename = "05")]
    NoChallengeRequestedTransactionalRiskAnalysis,
    #[serde(rename = "06")]
    NoChallengeRequestedDataShareOnly,
    #[serde(rename = "07")]
    NoChallengeRequestedStrongConsumerAuthentication,
    #[serde(rename = "08")]
    NoChallengeRequestedWhitelistExemption,
    #[serde(rename = "09")]
    ChallengeRequestedWhitelistPrompt,
}

/// This field contains information about how the 3DS Requestor authenticated the cardholder as part of a previous 3DS transaction.
/// Format of this field was changed with EMV 3DS 2.3.1 version:
/// In versions prior to 2.3.1, this field is a single object.
/// Starting from EMVCo version 2.3.1, this field is now an array of objects. Accepted value length is 1-3 elements.
///    
/// This field is optional, but recommended to include for versions prior to 2.3.1. From 2.3.1,
/// it is required for 3RI in the case of Decoupled Authentication Fallback or for SPC.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ThreeDSRequestorPriorTransactionAuthenticationInformation {
    /// This data element provides additional information to the ACS to determine the best
    /// approach for handling a request. The field is limited to 36 characters containing
    /// ACS Transaction ID for a prior authenticated transaction (for example, the first
    /// recurring transaction that was authenticated with the cardholder).
    pub three_ds_req_prior_ref: String,

    /// Mechanism used by the Cardholder to previously authenticate to the 3DS Requestor.
    /// Accepted values for this field are:
    ///    - 01 -> Frictionless authentication occurred by ACS
    ///    - 02 -> Cardholder challenge occurred by ACS
    ///    - 03 -> AVS verified
    ///    - 04 -> Other issuer methods
    ///    - 80-99 -> PS-specific value (dependent on the payment scheme type).
    pub three_ds_req_prior_auth_method: String,

    /// Date and time converted into UTC of the prior authentication. Accepted date
    /// format is YYYYMMDDHHMM.
    pub three_ds_req_prior_auth_timestamp: String,

    /// Data that documents and supports a specific authentication process. In the current
    /// version of the specification this data element is not defined in detail, however
    /// the intention is that for each 3DS Requestor Authentication Method, this field carry
    /// data that the ACS can use to verify the authentication process. In future versions
    /// of the application, these details are expected to be included. Field is limited to
    /// maximum 2048 characters.
    pub three_ds_req_prior_auth_data: String,
}

/// Enum indicating whether the 3DS Requestor requests the ACS to utilize Decoupled Authentication.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum ThreeDSRequestorDecoupledRequestIndicator {
    /// Decoupled Authentication is supported and preferred if challenge is necessary.
    Y,
    /// Do not use Decoupled Authentication.
    N,
    /// Decoupled Authentication is supported and is to be used only as a fallback challenge method
    /// if a challenge is necessary (Transaction Status = D in RReq).
    F,
    /// Decoupled Authentication is supported and can be used as a primary or fallback challenge method
    /// if a challenge is necessary (Transaction Status = D in either ARes or RReq).
    B,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CardholderAccount {
    /// Indicates the type of account.
    /// This is required if 3DS Requestor is asking Cardholder which Account Type they are using before making
    /// the purchase. This field is required in some markets. Otherwise, it is optional.
    pub acct_type: Option<AccountType>,
    /// Expiry date of the PAN or token supplied to the 3DS Requestor by the Cardholder.
    /// The field has 4 characters in a format YYMM.
    ///
    /// The requirements of the presence of this field are DS specific.
    pub card_expiry_date: Option<masking::Secret<String>>,
    /// This field contains additional information about the Cardholder’s account provided by the 3DS Requestor.
    ///
    /// The field is optional but recommended to include.
    ///
    /// Starting from EMV 3DS 2.3.1, added new field:
    /// - `ch_acc_req_id` -> The 3DS Requestor assigned account identifier of the transacting Cardholder.
    ///   This identifier is a unique representation of the account identifier for the 3DS Requestor and
    ///   is provided as a String.
    pub acct_info: Option<CardHolderAccountInformation>,
    /// Account number that will be used in the authorization request for payment transactions.
    /// May be represented by PAN or token.
    ///
    /// This field is required.
    pub acct_number: cards::CardNumber,
    /// ID for the scheme to which the Cardholder's acctNumber belongs to.
    /// It will be used to identify the Scheme from the 3DS Server configuration.
    ///
    /// This field is optional, but recommended to include.
    /// It should be present when it is not one of the schemes for which scheme resolving regular expressions
    /// are provided in the 3DS Server Configuration Properties. Additionally,
    /// if the schemeId is present in the request and there are card ranges found by multiple schemes, the schemeId will be
    /// used for proper resolving of the versioning data.
    pub scheme_id: Option<String>,
    /// Additional information about the account optionally provided by the 3DS Requestor.
    ///
    /// This field is limited to 64 characters and it is optional to use.
    #[serde(rename = "acctID")]
    pub acct_id: Option<String>,
    /// Indicates if the transaction was de-tokenized prior to being received by the ACS.
    ///
    /// The boolean value of true is the only valid response for this field when it is present.
    ///
    /// The field is required only if there is a de-tokenization of an Account Number.
    pub pay_token_ind: Option<bool>,
    /// Information about the de-tokenised Payment Token.
    /// Note: Data will be formatted into a JSON object prior to being placed into the EMV Payment Token field of the message.
    ///
    /// This field is optional.
    pub pay_token_info: Option<String>,
    /// Three or four-digit security code printed on the card.
    /// The value is numeric and limited to 3-4 characters.
    ///
    /// This field is required depending on the rules provided by the Directory Server.
    /// Available for supporting EMV 3DS 2.3.1 and later versions.
    pub card_security_code: Option<masking::Secret<String>>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum AccountType {
    #[serde(rename = "01")]
    NotApplicable,
    #[serde(rename = "02")]
    Credit,
    #[serde(rename = "03")]
    Debit,
    #[serde(rename = "80")]
    Jcb,
    /// 81-99 -> PS-specific value (dependent on the payment scheme type).
    #[serde(untagged)]
    PsSpecificValue(String),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CardHolderAccountInformation {
    /// Length of time that the cardholder has had the account with the 3DS Requestor.
    ///
    /// Accepted values are:
    /// - `01` -> No account
    /// - `02` -> Created during this transaction
    /// - `03` -> Less than 30 days
    /// - `04` -> Between 30 and 60 days
    /// - `05` -> More than 60 days
    pub ch_acc_age_ind: Option<String>,

    /// Date converted into UTC that the cardholder opened the account with the 3DS Requestor.
    ///
    /// Date format = YYYYMMDD.
    pub ch_acc_date: Option<String>,

    /// Length of time since the cardholder’s account information with the 3DS Requestor was
    /// last changed.
    ///
    /// Includes Billing or Shipping address, new payment account, or new user(s) added.
    ///
    /// Accepted values are:
    /// - `01` -> Changed during this transaction
    /// - `02` -> Less than 30 days
    /// - `03` -> 30 - 60 days
    /// - `04` -> More than 60 days
    pub ch_acc_change_ind: Option<String>,

    /// Date converted into UTC that the cardholder’s account with the 3DS Requestor was last changed.
    ///
    /// Including Billing or Shipping address, new payment account, or new user(s) added.
    ///
    /// Date format = YYYYMMDD.
    pub ch_acc_change: Option<String>,

    /// Length of time since the cardholder’s account with the 3DS Requestor had a password change
    /// or account reset.
    ///
    /// The accepted values are:
    /// - `01` -> No change
    /// - `02` -> Changed during this transaction
    /// - `03` -> Less than 30 days
    /// - `04` -> 30 - 60 days
    /// - `05` -> More than 60 days
    pub ch_acc_pw_change_ind: Option<String>,

    /// Date converted into UTC that cardholder’s account with the 3DS Requestor had a password
    /// change or account reset.
    ///
    /// Date format must be YYYYMMDD.
    pub ch_acc_pw_change: Option<String>,

    /// Indicates when the shipping address used for this transaction was first used with the
    /// 3DS Requestor.
    ///
    /// Accepted values are:
    /// - `01` -> This transaction
    /// - `02` -> Less than 30 days
    /// - `03` -> 30 - 60 days
    /// - `04` -> More than 60 days
    pub ship_address_usage_ind: Option<String>,

    /// Date converted into UTC when the shipping address used for this transaction was first
    /// used with the 3DS Requestor.
    ///
    /// Date format must be YYYYMMDD.
    pub ship_address_usage: Option<String>,

    /// Number of transactions (successful and abandoned) for this cardholder account with the
    /// 3DS Requestor across all payment accounts in the previous 24 hours.
    pub txn_activity_day: Option<u32>,

    /// Number of transactions (successful and abandoned) for this cardholder account with the
    /// 3DS Requestor across all payment accounts in the previous year.
    pub txn_activity_year: Option<u32>,

    /// Number of Add Card attempts in the last 24 hours.
    pub provision_attempts_day: Option<u32>,

    /// Number of purchases with this cardholder account during the previous six months.
    pub nb_purchase_account: Option<u32>,

    /// Indicates whether the 3DS Requestor has experienced suspicious activity
    /// (including previous fraud) on the cardholder account.
    ///
    /// Accepted values are:
    /// - `01` -> No suspicious activity has been observed
    /// - `02` -> Suspicious activity has been observed
    pub suspicious_acc_activity: Option<String>,

    /// Indicates if the Cardholder Name on the account is identical to the shipping Name used
    /// for this transaction.
    ///
    /// Accepted values are:
    /// - `01` -> Account Name identical to shipping Name
    /// - `02` -> Account Name different than shipping Name
    pub ship_name_indicator: Option<String>,

    /// Indicates the length of time that the payment account was enrolled in the cardholder’s
    /// account with the 3DS Requester.
    ///
    /// Accepted values are:
    /// - `01` -> No account (guest check-out)
    /// - `02` -> During this transaction
    /// - `03` -> Less than 30 days
    /// - `04` -> 30 - 60 days
    /// - `05` -> More than 60 days
    pub payment_acc_ind: Option<String>,

    /// Date converted into UTC that the payment account was enrolled in the cardholder’s account with
    /// the 3DS Requestor.
    ///
    /// Date format must be YYYYMMDD.
    pub payment_acc_age: Option<String>,

    /// The 3DS Requestor assigned account identifier of the transacting Cardholder.
    ///
    /// This identifier is a unique representation of the account identifier for the 3DS Requestor and
    /// is provided as a String. Accepted value length is maximum 64 characters.
    ///
    /// Added starting from EMV 3DS 2.3.1.
    pub ch_acc_req_id: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
#[serde_with::skip_serializing_none]
pub struct Cardholder {
    /// Indicates whether the Cardholder Shipping Address and Cardholder Billing Address are the same.
    ///
    /// Accepted values:
    /// - `Y` -> Shipping Address matches Billing Address
    /// - `N` -> Shipping Address does not match Billing Address
    ///
    /// If the field is not set and the shipping and billing addresses are the same, the 3DS Server will set the value to
    /// `Y`. Otherwise, the value will not be changed.
    ///
    /// This field is optional.
    addr_match: Option<String>,

    /// The city of the Cardholder billing address associated with the card used for this purchase.
    ///
    /// This field is limited to a maximum of 50 characters.
    ///
    /// This field is required unless market or regional mandate restricts sending this information.
    bill_addr_city: Option<String>,

    /// The country of the Cardholder billing address associated with the card used for this purchase.
    ///
    /// This field is limited to 3 characters. This value shall be the ISO 3166-1 numeric country code, except values
    /// from range 901 - 999 which are reserved by ISO.
    ///
    /// The field is required if Cardholder Billing Address State is present and unless market or regional mandate
    /// restricts sending this information.
    bill_addr_country: Option<String>,

    /// First line of the street address or equivalent local portion of the Cardholder billing address associated with
    /// the card use for this purchase.
    ///
    /// This field is limited to a maximum of 50 characters.
    ///
    /// This field is required unless market or regional mandate restricts sending this information.
    bill_addr_line1: Option<masking::Secret<String>>,

    /// Second line of the street address or equivalent local portion of the Cardholder billing address associated with
    /// the card use for this purchase.
    ///
    /// This field is limited to a maximum of 50 characters.
    ///
    /// This field is required unless market or regional mandate restricts sending this information.
    bill_addr_line2: Option<masking::Secret<String>>,

    /// Third line of the street address or equivalent local portion of the Cardholder billing address associated with
    /// the card use for this purchase.
    ///
    /// This field is limited to a maximum of 50 characters.
    ///
    /// This field is required unless market or regional mandate restricts sending this information.
    bill_addr_line3: Option<masking::Secret<String>>,

    /// ZIP or other postal code of the Cardholder billing address associated with the card used for this purchase.
    ///
    /// This field is limited to a maximum of 16 characters.
    ///
    /// This field is required unless market or regional mandate restricts sending this information.
    bill_addr_post_code: Option<masking::Secret<String>>,

    /// The state or province of the Cardholder billing address associated with the card used for this purchase.
    ///
    /// This field is limited to 3 characters. The value should be the country subdivision code defined in ISO 3166-2.
    ///
    /// This field is required unless State is not applicable for this country and unless market or regional mandate
    /// restricts sending this information.
    bill_addr_state: Option<masking::Secret<String>>,

    /// The email address associated with the account that is either entered by the Cardholder, or is on file with
    /// the 3DS Requestor.
    ///
    /// This field is limited to a maximum of 256 characters and shall meet requirements of Section 3.4 of
    /// IETF RFC 5322.
    ///
    /// This field is required unless market or regional mandate restricts sending this information.
    email: Option<Email>,

    /// The home phone provided by the Cardholder.
    ///
    /// Refer to ITU-E.164 for additional information on format and length.
    ///
    /// This field is required if available, unless market or regional mandate restricts sending this information.
    home_phone: Option<PhoneNumber>,

    /// The mobile phone provided by the Cardholder.
    ///
    /// Refer to ITU-E.164 for additional information on format and length.
    ///
    /// This field is required if available, unless market or regional mandate restricts sending this information.
    mobile_phone: Option<PhoneNumber>,

    /// The work phone provided by the Cardholder.
    ///
    /// Refer to ITU-E.164 for additional information on format and length.
    ///
    /// This field is required if available, unless market or regional mandate restricts sending this information.
    work_phone: Option<PhoneNumber>,

    /// Name of the Cardholder.
    ///
    /// This field is limited to 2-45 characters.
    ///
    /// This field is required unless market or regional mandate restricts sending this information.
    ///
    /// Starting from EMV 3DS 2.3.1:
    /// This field is limited to 1-45 characters.
    cardholder_name: Option<masking::Secret<String>>,

    /// City portion of the shipping address requested by the Cardholder.
    ///
    /// This field is required unless shipping information is the same as billing information, or market or regional
    /// mandate restricts sending this information.
    ship_addr_city: Option<String>,

    /// Country of the shipping address requested by the Cardholder.
    ///
    /// This field is limited to 3 characters. This value shall be the ISO 3166-1 numeric country code, except values
    /// from range 901 - 999 which are reserved by ISO.
    ///
    /// This field is required if Cardholder Shipping Address State is present and if shipping information are not the same
    /// as billing information. This field can be omitted if market or regional mandate restricts sending this information.
    ship_addr_country: Option<String>,

    /// First line of the street address or equivalent local portion of the shipping address associated with
    /// the card use for this purchase.
    ///
    /// This field is limited to a maximum of 50 characters.
    ///
    /// This field is required unless shipping information is the same as billing information, or market or regional
    /// mandate restricts sending this information.
    ship_addr_line1: Option<masking::Secret<String>>,

    /// Second line of the street address or equivalent local portion of the shipping address associated with
    /// the card use for this purchase.
    ///
    /// This field is limited to a maximum of 50 characters.
    ///
    /// This field is required unless shipping information is the same as billing information, or market or regional
    /// mandate restricts sending this information.
    ship_addr_line2: Option<masking::Secret<String>>,

    /// Third line of the street address or equivalent local portion of the shipping address associated with
    /// the card use for this purchase.
    ///
    /// This field is limited to a maximum of 50 characters.
    ///
    /// This field is required unless shipping information is the same as billing information, or market or regional
    /// mandate restricts sending this information.
    ship_addr_line3: Option<masking::Secret<String>>,

    /// ZIP or other postal code of the shipping address associated with the card used for this purchase.
    ///
    /// This field is limited to a maximum of 16 characters.
    ///
    /// This field is required unless shipping information is the same as billing information, or market or regional
    /// mandate restricts sending this information.
    ship_addr_post_code: Option<masking::Secret<String>>,

    /// The state or province of the shipping address associated with the card used for this purchase.
    ///
    /// This field is limited to 3 characters. The value should be the country subdivision code defined in ISO 3166-2.
    ///
    /// This field is required unless shipping information is the same as billing information, or State is not applicable
    /// for this country, or market or regional mandate restricts sending this information.
    ship_addr_state: Option<masking::Secret<String>>,

    /// Tax ID is the Cardholder's tax identification.
    ///
    /// The value is limited to 45 characters.
    ///
    /// This field is required depending on the rules provided by the Directory Server.
    /// Available for supporting EMV 3DS 2.3.1 and later versions.
    tax_id: Option<String>,
}

impl
    From<(
        api_models::payments::Address,
        Option<api_models::payments::Address>,
    )> for Cardholder
{
    fn from(
        (billing_address, shipping_address): (
            api_models::payments::Address,
            Option<api_models::payments::Address>,
        ),
    ) -> Self {
        Self {
            addr_match: None,
            bill_addr_city: billing_address
                .address
                .as_ref()
                .and_then(|add| add.city.clone()),
            bill_addr_country: None,
            bill_addr_line1: billing_address
                .address
                .as_ref()
                .and_then(|add| add.line1.clone()),
            bill_addr_line2: billing_address
                .address
                .as_ref()
                .and_then(|add| add.line2.clone()),
            bill_addr_line3: billing_address
                .address
                .as_ref()
                .and_then(|add| add.line3.clone()),
            bill_addr_post_code: billing_address
                .address
                .as_ref()
                .and_then(|add| add.zip.clone()),
            bill_addr_state: billing_address
                .address
                .as_ref()
                .and_then(|add| add.state.clone()),
            email: billing_address.email,
            home_phone: billing_address.phone.clone().map(Into::into),
            mobile_phone: billing_address.phone.clone().map(Into::into),
            work_phone: billing_address.phone.clone().map(Into::into),
            cardholder_name: billing_address
                .address
                .as_ref()
                .and_then(|add| add.first_name.clone()),
            ship_addr_city: shipping_address
                .as_ref()
                .and_then(|shipping_add| shipping_add.address.as_ref())
                .and_then(|add| add.city.clone()),
            ship_addr_country: None,
            ship_addr_line1: shipping_address
                .as_ref()
                .and_then(|shipping_add| shipping_add.address.as_ref())
                .and_then(|add| add.line1.clone()),
            ship_addr_line2: shipping_address
                .as_ref()
                .and_then(|shipping_add| shipping_add.address.as_ref())
                .and_then(|add| add.line2.clone()),
            ship_addr_line3: shipping_address
                .as_ref()
                .and_then(|shipping_add| shipping_add.address.as_ref())
                .and_then(|add| add.line3.clone()),
            ship_addr_post_code: shipping_address
                .as_ref()
                .and_then(|shipping_add| shipping_add.address.as_ref())
                .and_then(|add| add.zip.clone()),
            ship_addr_state: shipping_address
                .as_ref()
                .and_then(|shipping_add| shipping_add.address.as_ref())
                .and_then(|add| add.state.clone()),
            tax_id: None,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PhoneNumber {
    /// Country Code of the phone, limited to 1-3 characters
    #[serde(rename = "cc")]
    country_code: Option<String>,
    subscriber: Option<masking::Secret<String>>,
}

impl From<api_models::payments::PhoneDetails> for PhoneNumber {
    fn from(value: api_models::payments::PhoneDetails) -> Self {
        Self {
            country_code: value.country_code,
            subscriber: value.number,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Purchase {
    /// Indicates the maximum number of authorisations permitted for instalment payments.
    ///
    /// The field is limited to a maximum of 3 characters and value shall be greater than 1.
    ///
    /// The field is required if the Merchant and Cardholder have agreed to installment payments, i.e. if 3DS Requestor
    /// Authentication Indicator = 03. Omitted if not an installment payment authentication.
    ///
    /// Starting from EMV 3DS 2.3.1:
    /// Additionally this field is required for deviceChannel = 03 (3RI) if threeRIInd = 02.
    pub purchase_instal_data: Option<i32>,

    /// Merchant's assessment of the level of fraud risk for the specific authentication for both the cardholder and the
    /// authentication being conducted.
    ///
    /// The field is optional but strongly recommended to include.
    pub merchant_risk_indicator: Option<MerchantRiskIndicator>,

    /// Purchase amount in minor units of currency with all punctuation removed. When used in conjunction with the Purchase
    /// Currentcy Exponent field, proper punctuation can be calculated. Example: If the purchase amount is USD 123.45,
    /// element will contain the value 12345. The field is limited to maximum 48 characters.
    ///
    /// This field is required for 02 - NPA message category if 3DS Requestor Authentication Indicator = 02 or 03.
    ///
    /// Starting from EMV 3DS 2.3.1:
    /// Additionally this field is required for messageCategory = 02 (NPA) if threeRIInd = 01, 02, 06, 07, 08, 09, or 11.
    pub purchase_amount: Option<i64>,

    /// Currency in which purchase amount is expressed. The value is limited to 3 numeric characters and is represented by
    /// the ISO 4217 three-digit currency code, except 955-964 and 999.
    ///
    /// This field is required for requests where messageCategory = 01-PA and for 02-NPA if 3DS Requestor Authentication
    /// Indicator = 02 or 03.
    ///
    /// Starting from EMV 3DS 2.3.1:
    /// Additionally this field is required for messageCategory = 02 (NPA) if threeRIInd = 01, 02, 06, 07, 08, 09, or 11.
    pub purchase_currency: String,

    /// Minor units of currency as specified in the ISO 4217 currency exponent. The field is limited to 1 character and it
    /// is required for 01-PA and for 02-NPA if 3DS Requestor Authentication Indicator = 02 or 03.
    ///
    /// Example: for currency USD the exponent should be 2, and for Yen the exponent should be 0.
    ///
    /// Starting from EMV 3DS 2.3.1:
    /// Additionally this field is required for messageCategory = 02 (NPA) if threeRIInd = 01, 02, 06, 07, 08, 09, or 11.
    pub purchase_exponent: u8,

    /// Date and time of the purchase, converted into UTC. The field is limited to 14 characters,
    /// formatted as YYYYMMDDHHMMSS.
    ///
    /// This field is required for 01-PA and for 02-NPA, if 3DS Requestor Authentication Indicator = 02 or 03.
    ///
    /// Starting from EMV 3DS 2.3.1:
    /// Additionally this field is required for messageCategory = 02 (NPA) if threeRIInd = 01, 02, 06, 07, 08, 09, or 11.
    pub purchase_date: Option<String>,

    /// Date after which no further authorizations shall be performed. This field is limited to 8 characters, and the
    /// accepted format is YYYYMMDD.
    ///
    /// This field is required for 01-PA and for 02-NPA, if 3DS Requestor Authentication Indicator = 02 or 03.
    ///
    /// Starting from EMV 3DS 2.3.1:
    /// This field is required if recurringInd = 01.
    pub recurring_expiry: Option<String>,

    /// Indicates the minimum number of days between authorizations. The field is limited to maximum 4 characters.
    ///
    /// This field is required if 3DS Requestor Authentication Indicator = 02 or 03.
    ///
    /// Starting from EMV 3DS 2.3.1:
    /// This field is required if recurringInd = 01
    pub recurring_frequency: Option<i32>,

    /// Identifies the type of transaction being authenticated. The values are derived from ISO 8583. Accepted values are:
    ///    - 01 -> Goods / Service purchase
    ///    - 03 -> Check Acceptance
    ///    - 10 -> Account Funding
    ///    - 11 -> Quasi-Cash Transaction
    ///    - 28 -> Prepaid activation and Loan
    ///
    /// This field is required in some markets. Otherwise, the field is optional.
    ///
    /// This field is required if 3DS Requestor Authentication Indicator = 02 or 03.
    pub trans_type: Option<String>,

    /// Recurring amount after first/promotional payment in minor units of currency with all punctuation removed.
    /// Example: If the recurring amount is USD 123.45, element will contain the value 12345. The field is limited to
    /// maximum 48 characters.
    ///
    /// The field is required if threeDSRequestorAuthenticationInd = 02 or 03 OR threeRIInd = 01 or 02 AND
    /// purchaseAmount != recurringAmount AND recurringInd = 01.
    ///
    /// Available for supporting EMV 3DS 2.3.1 and later versions.
    pub recurring_amount: Option<i64>,

    /// Currency in which recurring amount is expressed. The value is limited to 3 numeric characters and is represented by
    /// the ISO 4217 three-digit currency code, except 955-964 and 999.
    ///
    /// This field is required if recurringAmount is present.
    ///
    /// Available for supporting EMV 3DS 2.3.1 and later versions.
    pub recurring_currency: Option<String>,

    /// Minor units of currency as specified in the ISO 4217 currency exponent. Example: USD = 2, Yen = 0. The value is
    /// limited to 1 numeric character.
    ///
    /// This field is required if recurringAmount is present.
    ///
    /// Available for supporting EMV 3DS 2.3.1 and later versions.
    pub recurring_exponent: Option<i32>,

    /// Effective date of new authorised amount following first/promotional payment in recurring transaction. The value
    /// is limited to 8 characters. Accepted format: YYYYMMDD.
    ///
    /// This field is required if recurringInd = 01.
    ///
    /// Available for supporting EMV 3DS 2.3.1 and later versions.
    pub recurring_date: Option<String>,

    /// Part of the indication whether the recurring or instalment payment has a fixed or variable amount.
    ///
    /// Accepted values are:
    ///    - 01 -> Fixed Purchase Amount
    ///    - 02 -> Variable Purchase Amount
    ///    - 03–79 -> Reserved for EMVCo future use (values invalid until defined by EMVCo)
    ///    - 80-99 -> PS-specific value (dependent on the payment scheme type)
    ///
    /// Available for supporting EMV 3DS 2.3.1 and later versions.
    pub amount_ind: Option<String>,

    /// Part of the indication whether the recurring or instalment payment has a fixed or variable frequency.
    ///
    /// Accepted values are:
    ///    - 01 -> Fixed Frequency
    ///    - 02 -> Variable Frequency
    ///    - 03–79 -> Reserved for EMVCo future use (values invalid until defined by EMVCo)
    ///    - 80-99 -> PS-specific value (dependent on the payment scheme type)
    ///
    /// Available for supporting EMV 3DS 2.3.1 and later versions.
    pub frequency_ind: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct MerchantRiskIndicator {
    /// Indicates the shipping method chosen for the transaction.
    ///
    /// Merchants must choose the Shipping Indicator code that most accurately describes the cardholder's specific transaction.
    /// If one or more items are included in the sale, use the Shipping Indicator code for the physical goods, or if all digital goods,
    /// use the code that describes the most expensive item.
    ///
    /// Accepted values:
    ///   - Ship to cardholder's billing address (01)
    ///   - Ship to another verified address on file with merchant (02)
    ///   - Ship to address that is different than the cardholder's billing address (03)
    ///   - Ship to Store / Pick-up at local store (Store address shall be populated in shipping address fields) (04)
    ///   - Digital goods (includes online services, electronic gift cards and redemption codes) (05)
    ///   - Travel and Event tickets, not shipped (06)
    ///   - Other (for example, Gaming, digital services not shipped, e-media subscriptions, etc.) (07)
    ///   - PS-specific value (dependent on the payment scheme type) (80-81)
    ///
    /// Starting from EMV 3DS 2.3.1:
    /// Changed values to shipIndicator -> Accepted values are:
    ///                        - 01 -> Ship to cardholder's billing address
    ///                        - 02 -> Ship to another verified address on file with merchant
    ///                        - 03 -> Ship to address that is different than the cardholder's billing address
    ///                        - 04 -> "Ship to Store" / Pick-up at local store (Store address shall be populated in shipping
    ///                              address fields)
    ///                        - 05 -> Digital goods (includes online services, electronic gift cards and redemption codes)
    ///                        - 06 -> Travel and Event tickets, not shipped
    ///                        - 07 -> Other (for example, Gaming, digital services not shipped, e-media subscriptions, etc.)
    ///                        - 08 -> Pick-up and go delivery
    ///                        - 09 -> Locker delivery (or other automated pick-up)
    ship_indicator: Option<String>,

    /// Indicates the merchandise delivery timeframe.
    ///
    /// Accepted values:
    ///   - Electronic Delivery (01)
    ///   - Same day shipping (02)
    ///   - Overnight shipping (03)
    ///   - Two-day or more shipping (04)
    delivery_timeframe: Option<String>,

    /// For electronic delivery, the email address to which the merchandise was delivered.
    delivery_email_address: Option<String>,

    /// Indicates whether the cardholder is reordering previously purchased merchandise.
    ///
    /// Accepted values:
    ///   - First time ordered (01)
    ///   - Reordered (02)
    reorder_items_ind: Option<String>,

    /// Indicates whether Cardholder is placing an order for merchandise with a future availability or release date.
    ///
    /// Accepted values:
    ///   - Merchandise available (01)
    ///   - Future availability (02)
    pre_order_purchase_ind: Option<String>,

    /// For a pre-ordered purchase, the expected date that the merchandise will be available.
    ///
    /// Date format: YYYYMMDD
    pre_order_date: Option<String>,

    /// For prepaid or gift card purchase, the purchase amount total of prepaid or gift card(s) in major units.
    gift_card_amount: Option<i32>,

    /// For prepaid or gift card purchase, the currency code of the card as defined in ISO 4217 except 955 - 964 and 999.
    gift_card_curr: Option<String>, // ISO 4217 currency code

    /// For prepaid or gift card purchase, total count of individual prepaid or gift cards/codes purchased.
    ///
    /// Field is limited to 2 characters.
    gift_card_count: Option<i32>,
    /// Starting from EMV 3DS 2.3.1.1:
    /// New field introduced:
    /// - transChar -> Indicates to the ACS specific transactions identified by the Merchant.
    ///      - Size: Variable, 1-2 elements. JSON Data Type: Array of String. Accepted values:
    ///                     - 01 -> Cryptocurrency transaction
    ///                     - 02 -> NFT transaction
    trans_char: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AcquirerData {
    /// Acquiring institution identification code as assigned by the DS receiving the AReq message.
    ///
    /// This field is limited to 11 characters. This field can be omitted if it there is a MerchantAcquirer already configured for 3DS Server,
    /// referenced by the acquirerMerchantId.
    ///
    /// This field is required if no MerchantAcquirer is present for the acquirer BIN in the 3DS Server configuration and
    /// for requests where messageCategory = 01 (PA). For requests where messageCategory=02 (NPA), the field is required
    /// only if scheme is Mastercard, for other schemes it is optional.
    pub acquirer_bin: Option<String>,

    /// Acquirer-assigned Merchant identifier.
    ///
    /// This may be the same value that is used in authorization requests sent on behalf of the 3DS Requestor and is represented in ISO 8583 formatting requirements.
    /// The field is limited to maximum 35 characters. Individual Directory Servers may impose specific format and character requirements on
    /// the contents of this field.
    ///
    /// This field will be used to identify the Directory Server where the AReq will be sent and the acquirerBin from the 3DS Server configuration.
    /// If no MerchantAcquirer configuration is present in the 3DS Server, the DirectoryServer information will be resolved from the scheme to which the cardholder account belongs to.
    ///
    /// This field is required if merchantConfigurationId is not provided in the request and messageCategory = 01 (PA).
    /// For Mastercard, if merchantConfigurationId is not provided, the field must be present if messageCategory = 02 (NPA).
    pub acquirer_merchant_id: Option<String>,

    /// Acquirer Country Code.
    ///
    /// This is the code of the country where the acquiring institution is located. The specified
    /// length of this field is 3 characters and will accept values according to the ISO 3166-1 numeric three-digit
    /// country code.
    ///
    /// The Directory Server may edit the value of this field provided by the 3DS Server.
    ///
    /// This field is required.
    pub acquirer_country_code: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
#[serde_with::skip_serializing_none]
pub struct MerchantData {
    /// ID of the merchant. This value will be used to find merchant information from the configuration.
    /// From the merchant configuration the 3DS Server can fill the other values (mcc, merchantCountryCode and merchantName), if provided.
    ///
    /// This field can be left out if merchant information are provided in the request.
    pub merchant_configuration_id: Option<String>,

    /// Merchant Category Code. This is the DS-specific code describing the Merchant's type of business, product or service.
    /// The field is limited to 4 characters. The value correlates to the Merchant Category Code as defined by each Payment System or DS.
    ///
    /// If not present in the request it will be filled from the merchant configuration referenced by the merchantConfigurationId.
    ///
    /// This field is required for messageCategory=01 (PA) and optional, but strongly recommended for 02 (NPA).
    pub mcc: Option<String>,

    /// Country code for the merchant. This value correlates to the Merchant Country Code as defined by each Payment System or DS.
    /// The field is limited to 3 characters accepting ISO 3166-1 format, except 901-999.
    ///
    /// If not present in the request it will be filled from the merchant configuration referenced by the merchantConfigurationId.
    ///
    /// This field is required for messageCategory=01 (PA) and optional, but strongly recommended for 02 (NPA).
    pub merchant_country_code: Option<String>,

    /// Merchant name assigned by the Acquirer or Payment System. This field is limited to maximum 40 characters,
    /// and it is the same name used in the authorisation message as defined in ISO 8583.
    ///
    /// If not present in the request it will be filled from the merchant configuration referenced by the merchantConfigurationId.
    ///
    /// This field is required for messageCategory=01 (PA) and optional, but strongly recommended for 02 (NPA).
    pub merchant_name: Option<String>,

    /// Fully qualified URL of the merchant that receives the CRes message or Error Message.
    /// Incorrect formatting will result in a failure to deliver the notification of the final CRes message.
    /// This field is limited to 256 characters.
    ///
    /// This field should be present if the merchant will receive the final CRes message and the device channel is BROWSER.
    /// If not present in the request it will be filled from the notificationURL configured in the XML or database configuration.
    #[serde(rename = "notificationURL")]
    pub notification_url: Option<String>,

    /// Each DS provides rules for the 3DS Requestor ID. The 3DS Requestor is responsible for providing the 3DS Requestor ID according to the DS rules.
    ///
    /// This value is mandatory, therefore it should be either configured for each Merchant Acquirer, or should be
    /// passed in the transaction payload as part of the Merchant data.
    #[serde(rename = "threeDSRequestorId")]
    pub three_ds_requestor_id: Option<String>,

    /// Each DS provides rules for the 3DS Requestor Name. The 3DS Requestor is responsible for providing the 3DS Requestor Name according to the DS rules.
    ///
    /// This value is mandatory, therefore it should be either configured for each Merchant Acquirer, or should be
    /// passed in the transaction payload as part of the Merchant data.
    #[serde(rename = "threeDSRequestorName")]
    pub three_ds_requestor_name: Option<String>,

    /// Set whitelisting status of the merchant.
    ///
    /// The field is optional and if value is not present, the whitelist remains unchanged.
    /// This field is only available for supporting EMV 3DS 2.2.0.
    pub white_list_status: Option<WhitelistStatus>,

    /// Set trustlisting status of the merchant.
    ///
    /// The field is optional and if value is not present, the trustlist remains unchanged.
    /// From EMV 3DS 2.3.1 this field replaces whiteListStatus.
    pub trust_list_status: Option<WhitelistStatus>,

    /// Additional transaction information for transactions where merchants submit transaction details on behalf of another entity.
    /// The accepted value length is 1-50 elements.
    ///
    /// This field is optional.
    pub seller_info: Option<Vec<SellerInfo>>,

    /// Fully qualified URL of the merchant that receives the RRes message or Error Message.
    /// Incorrect formatting will result in a failure to deliver the notification of the final RRes message.
    /// This field is limited to 256 characters.
    ///
    /// This field is not mandatory and could be present if the Results Response (in case of a challenge transaction)
    /// should be sent to a dynamic URL different from the one present in the configuration, only if dynamic provision
    /// of the Results Response notification URL is allowed per the license.
    ///
    /// If not present in the request it will be filled from the notificationURL configured in the XML or database
    /// configuration.
    pub results_response_notification_url: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub enum WhitelistStatus {
    /// 3DS Requestor is whitelisted by cardholder
    Y,
    /// 3DS Requestor is not whitelisted by cardholder
    N,
    /// Not eligible as determined by issuer
    E,
    /// Pending confirmation by cardholder
    P,
    /// Cardholder rejected
    R,
    /// Whitelist status unknown, unavailable, or does not apply.
    U,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
#[serde_with::skip_serializing_none]
pub struct SellerInfo {
    /// Name of the Seller. The value length is maximum 100 characters. This field is required.
    seller_name: String,

    /// Merchant-assigned Seller identifier. If this data element is present, this must match the Seller ID field
    /// in the Seller Information object. The value length is maximum 50 characters. This field is required if
    /// sellerId in multiTransaction object is present.
    seller_id: Option<String>,

    /// Business name of the Seller. The value length is maximum 100 characters. This field is optional.
    seller_business_name: Option<String>,

    /// Date converted into UTC that the Seller started using the Merchant's services. The accepted value length is
    /// 8 characters. The accepted format is: YYYYMMDD.
    seller_acc_date: Option<String>,

    /// First line of the business or contact street address of the Seller. The value length is maximum 50 characters.
    /// This field is optional.
    seller_addr_line1: Option<String>,

    /// Second line of the business or contact street address of the Seller. The value length is maximum 50 characters.
    /// This field is optional.
    seller_addr_line2: Option<String>,

    /// Third line of the business or contact street address of the Seller. The value length is maximum 50 characters.
    /// This field is optional.
    seller_addr_line3: Option<String>,

    /// Business or contact city of the Seller. The value length is maximum 50 characters. This field is optional.
    seller_addr_city: Option<String>,

    /// Business or contact state or province of the Seller. The value length is maximum 3 characters. Accepted values
    /// are: Country subdivision code defined in ISO 3166-2. For example, using the ISO entry US-CA (California,
    /// United States), the correct value for this field = CA. Note that the country and hyphen are not included in
    /// this value. This field is optional.
    seller_addr_state: Option<String>,

    /// Business or contact ZIP or other postal code of the Seller. The value length is maximum 16 characters.
    /// This field is optional.
    seller_addr_post_code: Option<String>,

    /// Business or contact country of the Seller. The accepted value length is 3 characters. Accepted values are
    /// ISO 3166-1 numeric three-digit country code, except 955-964 and 999. This field is optional.
    seller_addr_country: Option<String>,

    /// Business or contact email address of the Seller. The value length is maximum 254 characters. Accepted values
    /// shall meet requirements of Section 3.4 of IETF RFC 5322. This field is optional.
    seller_email: Option<String>,

    /// Business or contact phone number of the Seller. Country Code and Subscriber sections of the number represented
    /// by the following named fields:
    ///     - cc -> Accepted value length is 1-3 characters.
    ///     - subscriber -> Accepted value length is maximum 15 characters.
    /// This field is optional.
    seller_phone: Option<PhoneNumber>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Browser {
    /// Exact content of the HTTP accept headers as sent to the 3DS Requestor from the Cardholder's browser.
    /// This field is limited to maximum 2048 characters  and if the total length exceeds the limit, the 3DS Server
    /// truncates the excess portion.
    ///
    /// This field is required for requests where deviceChannel=02 (BRW).
    browser_accept_header: Option<String>,

    /// IP address of the browser as returned by the HTTP headers to the 3DS Requestor. The field is limited to maximum 45
    /// characters and the accepted values are as following:
    ///      - IPv4 address is represented in the dotted decimal format of 4 sets of decimal numbers separated by dots. The
    ///        decimal number in each and every set is in the range 0 - 255. Example: 1.12.123.255
    ///      - IPv6 address is represented as eight groups of four hexadecimal digits, each group representing 16 bits (two
    ///        octets). The groups are separated by colons (:). Example: 2011:0db8:85a3:0101:0101:8a2e:0370:7334
    ///
    /// This field is required for requests when deviceChannel = 02 (BRW) where regionally acceptable.
    #[serde(rename = "browserIP")]
    browser_ip: Option<masking::Secret<String, common_utils::pii::IpAddress>>,

    /// Boolean that represents the ability of the cardholder browser to execute Java. Value is returned from the
    /// navigator.javaEnabled property.
    ///
    /// Depending on the message version, the field is required for requests:
    /// - with message version = 2.1.0 and deviceChannel = 02 (BRW).
    /// - with message version = 2.2.0 and deviceChannel = 02 (BRW) and browserJavascriptEnabled = true.
    browser_java_enabled: Option<bool>,

    /// Value representing the browser language as defined in IETF BCP47.
    ///
    /// Until EMV 3DS 2.2.0:
    /// The value is limited to 1-8 characters. If the value exceeds 8 characters, it will be truncated to a
    /// semantically valid value, if possible. The value is returned from navigator.language property.
    ///
    /// This field is required for requests where deviceChannel = 02 (BRW)
    /// In other cases this field is optional.
    ///
    /// Starting from EMV 3DS 2.3.1:
    /// The value is limited to 35 characters. If the value exceeds 35 characters, it will be truncated to a
    /// semantically valid value, if possible. The value is returned from navigator.language property.
    ///
    /// This field is required for requests where deviceChannel = 02 (BRW) and browserJavascriptEnabled = true.
    /// In other cases this field is optional.
    browser_language: Option<String>,

    /// Value representing the bit depth of the colour palette for displaying images, in bits per pixel. Obtained from
    /// Cardholder browser using the screen.colorDepth property. The field is limited to 1-2 characters.
    ///
    /// Accepted values are:
    ///  - 1 -> 1 bit
    ///  - 4 -> 4 bits
    ///  - 8 -> 8 bits
    ///  - 15 -> 15 bits
    ///  - 16 -> 16 bits
    ///  - 24 -> 24 bits
    ///  - 32 -> 32 bits
    ///  - 48 -> 48 bits
    ///
    ///  If the value is not in the accepted values, it will be resolved to the first accepted value lower from the one
    ///  provided.
    ///
    /// Depending on the message version, the field is required for requests:
    /// - with message version = 2.1.0 and deviceChannel = 02 (BRW).
    /// - with message version = 2.2.0 and deviceChannel = 02 (BRW) and browserJavascriptEnabled = true.
    browser_color_depth: Option<String>,

    /// Total height of the Cardholder's screen in pixels. Value is returned from the screen.height property. The value is
    /// limited to 1-6 characters.
    ///
    /// Depending on the message version, the field is required for requests:
    /// - with message version = 2.1.0 and deviceChannel = 02 (BRW).
    /// - with message version = 2.2.0 and deviceChannel = 02 (BRW) and browserJavascriptEnabled = true.
    browser_screen_height: Option<u32>,

    /// Total width of the Cardholder's screen in pixels. Value is returned from the screen.width property. The value is
    /// limited to 1-6 characters.
    ///
    /// Depending on the message version, the field is required for requests:
    /// - with message version = 2.1.0 and deviceChannel = 02 (BRW).
    /// - with message version = 2.2.0 and deviceChannel = 02 (BRW) and browserJavascriptEnabled = true.
    browser_screen_width: Option<u32>,

    /// Time difference between UTC time and the Cardholder browser local time, in minutes. The field is limited to 1-5
    /// characters where the vauyes is returned from the getTimezoneOffset() method.
    ///
    /// Depending on the message version, the field is required for requests:
    /// - with message version = 2.1.0 and deviceChannel = 02 (BRW).
    /// - with message version = 2.2.0 and deviceChannel = 02 (BRW) and browserJavascriptEnabled = true.
    #[serde(rename = "browserTZ")]
    browser_tz: Option<u32>,

    /// Exact content of the HTTP user-agent header. The field is limited to maximum 2048 characters. If the total length of
    /// the User-Agent sent by the browser exceeds 2048 characters, the 3DS Server truncates the excess portion.
    ///
    /// This field is required for requests where deviceChannel = 02 (BRW).
    browser_user_agent: Option<String>,

    /// Dimensions of the challenge window that has been displayed to the Cardholder. The ACS shall reply with content
    /// that is formatted to appropriately render in this window to provide the best possible user experience.
    ///
    /// Preconfigured sizes are width X height in pixels of the window displayed in the Cardholder browser window. This is
    /// used only to prepare the CReq request and it is not part of the AReq flow. If not present it will be omitted.
    ///
    /// However, when sending the Challenge Request, this field is required when deviceChannel = 02 (BRW).
    ///
    /// Accepted values are:
    ///  - 01 -> 250 x 400
    ///  - 02 -> 390 x 400
    ///  - 03 -> 500 x 600
    ///  - 04 -> 600 x 400
    ///  - 05 -> Full screen
    challenge_window_size: Option<ChallengeWindowSizeEnum>,

    /// Boolean that represents the ability of the cardholder browser to execute JavaScript.
    ///
    /// This field is required for requests where deviceChannel = 02 (BRW).
    /// Available for supporting EMV 3DS 2.2.0 and later versions.
    browser_javascript_enabled: Option<bool>,

    /// Value representing the browser language preference present in the http header, as defined in IETF BCP 47.
    ///
    /// The value is limited to 1-99 elements. Each element should contain a maximum of 100 characters.
    ///
    /// This field is required for requests where deviceChannel = 02 (BRW).
    /// Available for supporting EMV 3DS 2.3.1 and later versions.
    accept_language: Option<Vec<String>>,
}

impl From<crate::types::BrowserInformation> for Browser {
    fn from(value: crate::types::BrowserInformation) -> Self {
        Self {
            browser_accept_header: value.accept_header,
            browser_ip: value
                .ip_address
                .map(|ip| masking::Secret::new(ip.to_string())),
            browser_java_enabled: value.java_enabled,
            browser_language: value.language,
            browser_color_depth: value.color_depth.map(|cd| cd.to_string()),
            browser_screen_height: value.screen_height,
            browser_screen_width: value.screen_width,
            browser_tz: Some(1),
            browser_user_agent: value.user_agent,
            challenge_window_size: Some(ChallengeWindowSizeEnum::FullScreen),
            browser_javascript_enabled: value.java_script_enabled,
            accept_language: None,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum ChallengeWindowSizeEnum {
    #[serde(rename = "01")]
    Size250x400,
    #[serde(rename = "02")]
    Size390x400,
    #[serde(rename = "03")]
    Size500x600,
    #[serde(rename = "04")]
    Size600x400,
    #[serde(rename = "05")]
    FullScreen,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Sdk {
    /// Universally unique ID created upon all installations and updates of the 3DS Requestor App on a Customer Device.
    /// This will be newly generated and stored by the 3DS SDK for each installation or update. The field is limited to 36
    /// characters and it shall have a canonical format as defined in IETF RFC 4122. This may utilize any of the specified
    /// versions as long as the output meets specified requirements.
    ///
    /// Starting from EMV 3DS 2.3.1:
    /// In case of Browser-SDK, the SDK App ID value is not reliable, and may change for each transaction.
    sdk_app_id: Option<String>,

    /// JWE Object as defined Section 6.2.2.1 containing data encrypted by the SDK for the DS to decrypt. This element is
    /// the only field encrypted in this version of the EMV 3-D Secure specification. The field is sent from the SDK and it
    /// is limited to 64.000 characters. The data will be present when sending to DS, but not present from DS to ACS.
    sdk_enc_data: Option<String>,

    /// Public key component of the ephemeral key pair generated by the 3DS SDK and used to establish session keys between
    /// the 3DS SDK and ACS. In AReq, this data element is contained within the ACS Signed Content JWS Object. The field is
    /// limited to maximum 256 characters.
    sdk_ephem_pub_key: Option<HashMap<String, String>>,

    /// Indicates the maximum amount of time (in minutes) for all exchanges. The field shall have value greater or equals
    /// than 05.
    sdk_max_timeout: Option<u8>,

    /// Identifies the vendor and version of the 3DS SDK that is integrated in a 3DS Requestor App, assigned by EMVCo when
    /// the 3DS SDK is approved. The field is limited to 32 characters.
    ///
    /// Starting from EMV 3DS 2.3.1:
    /// Identifies the vendor and version of the 3DS SDK that is utilised for a specific transaction. The value is
    /// assigned by EMVCo when the Letter of Approval of the specific 3DS SDK is issued.
    sdk_reference_number: Option<String>,

    /// Universally unique transaction identifier assigned by the 3DS SDK to identify a single transaction. The field is
    /// limited to 36 characters and it shall be in a canonical format as defined in IETF RFC 4122. This may utilize any of
    /// the specified versions as long as the output meets specific requirements.
    sdk_trans_id: Option<String>,

    /// Contains the JWS object(represented as a string) created by the Split-SDK Server for the AReq message. A
    /// Split-SDK Server creates a time-stamped signature on certain transaction data that is sent to the DS for
    /// verification. As a prerequisite, the Split-SDK Server has a key pair PbSDK, PvSDK certificate Cert (PbSDK). This
    /// certificate is an X.509 certificate signed by a DS CA whose public key is known to the DS.
    ///
    /// The Split-SDK Server:
    ///    Creates a JSON object of the following data as the JWS payload to be signed:
    ///
    ///        SDK Reference Number -> Identifies the vendor and version of the 3DS SDK that is utilised for a specific
    ///                                transaction. The value is assigned by EMVCo when the Letter of Approval of the
    ///                                specific 3DS SDK is issued. The field is limited to 32 characters.
    ///        SDK Signature Timestamp -> Date and time indicating when the 3DS SDK generated the Split-SDK Server Signed
    ///                                   Content converted into UTC. The value is limited to 14 characters. Accepted
    ///                                   format: YYYYMMDDHHMMSS.
    ///        SDK Transaction ID -> Universally unique transaction identifier assigned by the 3DS SDK to identify a
    ///                              single transaction. The field is limited to 36 characters and it shall be in a
    ///                              canonical format as defined in IETF RFC 4122. This may utilize any of the specified
    ///                              versions as long as the output meets specific requirements.
    ///        Split-SDK Server ID -> DS assigned Split-SDK Server identifier. Each DS can provide a unique ID to each
    ///                               Split-SDK Server on an individual basis. The field is limited to 32 characters.
    ///                               Any individual DS may impose specific formatting and character requirements on the
    ///                               contents of this field.
    ///
    ///    Generates a digital signature of the full JSON object according to JWS (RFC 7515) using JWS Compact
    ///    Serialization. The parameter values for this version of the specification and to be included in the JWS
    ///    header are:
    ///
    ///        "alg": PS2567 or ES256
    ///        "x5c": X.5C v3: Cert (PbSDK) and chaining certificates if present
    ///
    ///    All other parameters: optional
    ///
    ///    Includes the resulting JWS in the AReq message as SDK Server Signed Content
    ///
    /// This field is required if sdkType = 02 or 03 and deviceChannel = 01 (APP)
    /// Available for supporting EMV 3DS 2.3.1 and later versions.
    sdk_server_signed_content: Option<String>,

    /// Indicates the type of 3DS SDK.
    /// This data element provides additional information to the DS and ACS to determine the best approach for handling
    /// the transaction. Accepted values are:
    ///
    ///    - 01 -> Default SDK
    ///    - 02 -> Split-SDK
    ///    - 03 -> Limited-SDK
    ///    - 04 -> Browser-SDK
    ///    - 05 -> Shell-SDK
    ///    - 80-99 -> PS-specific value (dependent on the payment scheme type)
    ///
    /// This field is required for requests where deviceChannel = 01 (APP).
    /// Available for supporting EMV 3DS 2.3.1 and later versions.
    sdk_type: Option<SdkTypeEnum>,

    /// Indicates the characteristics of a Default-SDK.
    ///
    /// This field is required for requests where deviceChannel = 01 (APP) and SDK Type = 01.
    /// Available for supporting EMV 3DS 2.3.1 and later versions.
    default_sdk_type: Option<DefaultSdkType>,

    /// Indicates the characteristics of a Split-SDK.
    ///
    /// This field is required for requests where deviceChannel = 01 (APP) and SDK Type = 02.
    /// Available for supporting EMV 3DS 2.3.1 and later versions.
    split_sdk_type: Option<SplitSdkType>,
}

impl From<api_models::payments::SdkInformation> for Sdk {
    fn from(sdk_info: api_models::payments::SdkInformation) -> Self {
        Self {
            sdk_app_id: Some(sdk_info.sdk_app_id),
            sdk_enc_data: Some(sdk_info.sdk_enc_data),
            sdk_ephem_pub_key: Some(sdk_info.sdk_ephem_pub_key),
            sdk_max_timeout: Some(sdk_info.sdk_max_timeout),
            sdk_reference_number: Some(sdk_info.sdk_reference_number),
            sdk_trans_id: Some(sdk_info.sdk_trans_id),
            sdk_server_signed_content: None,
            sdk_type: None,
            default_sdk_type: None,
            split_sdk_type: None,
        }
    }
}

/// Enum representing the type of 3DS SDK.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum SdkTypeEnum {
    #[serde(rename = "01")]
    DefaultSdk,
    #[serde(rename = "02")]
    SplitSdk,
    #[serde(rename = "03")]
    LimitedSdk,
    #[serde(rename = "04")]
    BrowserSdk,
    #[serde(rename = "05")]
    ShellSdk,
    ///    - 80-99 -> PS-specific value (dependent on the payment scheme type)
    #[serde(untagged)]
    PsSpecific(String),
}

/// Struct representing characteristics of a Default-SDK.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DefaultSdkType {
    /// SDK Variant: SDK implementation characteristics
    ///    - Length: 2 characters
    ///    - Values accepted:
    ///       - 01 = Native
    ///       - 02–79 = Reserved for EMVCo future use (values invalid until defined by EMVCo)
    ///       - 80–99 = Reserved for DS use
    sdk_variant: String,

    /// Wrapped Indicator: If the Default-SDK is embedded as a wrapped component in the 3DS Requestor App
    ///    - Length: 1 character
    ///    - Value accepted: Y = Wrapped
    ///    - Only present if value = Y
    wrapped_ind: Option<String>,
}

/// Struct representing characteristics of a Split-SDK.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SplitSdkType {
    /// Split-SDK Variant: Implementation characteristics of the Split-SDK client
    ///    - Length: 2 characters
    ///    - Values accepted:
    ///       - 01 = Native Client
    ///       - 02 = Browser
    ///       - 03 = Shell
    ///       - 04–79 = Reserved for EMVCo future use (values invalid until defined by EMVCo)
    ///       - 80–99 = Reserved for DS use
    sdk_variant: String,

    /// Limited Split-SDK Indicator: If the Split-SDK client has limited capabilities
    ///    - Length: 1 character
    ///    - Value accepted:
    ///       • Y = Limited
    ///    - Only present if value = Y
    limited_ind: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct MessageExtensionAttribute {
    id: String,
    name: String,
    criticality_indicator: bool,
    data: String,
}
