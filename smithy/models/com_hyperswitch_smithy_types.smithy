$version: "2"

namespace com.hyperswitch.smithy.types

structure BecsBankDebitNestedType {
    /// Billing details for bank debit
    billing_details: BankDebitBilling
    /// Account number for Becs payment method
    @required
    account_number: smithy.api#String
    /// Bank-State-Branch (bsb) number
    @required
    bsb_number: smithy.api#String
    /// Owner name for bank debit
    bank_account_holder_name: smithy.api#String
}

union UpiData {
    upi_intent: UpiIntentData
    upi_collect: UpiCollectData
}

structure OnlineBankingThailandNestedType {
    @required
    issuer: BankNames
}

union RealTimePaymentData {
    fps: FpsNestedType
    viet_qr: VietQrNestedType
    duit_now: DuitNowNestedType
    prompt_pay: PromptPayNestedType
    qris: QrisNestedType
}

structure BacsBankTransferInstructions {
    @required
    sort_code: smithy.api#String
    @required
    account_holder_name: smithy.api#String
    @required
    account_number: smithy.api#String
}

/// To indicate the type of payment experience that the customer would go through
enum PaymentExperience {
    /// Contains the data for invoking the sdk client for completing the payment.
    invoke_sdk_client
    /// Contains the data for invoking the sdk client for completing the payment.
    invoke_payment_app
    /// The URL to which the customer needs to be redirected for completing the payment.
    redirect_to_url
    /// Contains data to finish one click payment.
    one_click
    /// Redirect customer to link wallet
    link_wallet
    /// Represents that otp needs to be collect and contains if consent is required
    collect_otp
    /// The QR code data to be displayed to the customer.
    display_qr_code
    /// Contains the data for displaying wait screen
    display_wait_screen
}

/// Indicates the card network.
enum CardNetwork {
    CartesBancaires
    Visa
    UnionPay
    Maestro
    Pulse
    Nyce
    Dinacard
    AmericanExpress
    JCB
    Discover
    RuPay
    Interac
    Prop
    PrivateLabel
    DinersClub
    Star
    Accel
    Mastercard
}

structure OpenBankingUkNestedType {
    /// The country for bank payment
    country: CountryAlpha2
    issuer: BankNames
}

structure ApplePayWalletData {
    /// The unique identifier for the transaction
    @required
    transaction_identifier: smithy.api#String
    /// The payment method of Apple pay
    @required
    payment_method: ApplepayPaymentMethod
    /// The payment data of Apple pay
    @required
    payment_data: smithy.api#Document
}

list OrderDetailsWithAmountList {
    member: OrderDetailsWithAmount
}

structure LocalBankRedirectNestedType {
}

enum CountryAlpha2 {
    UG
    CD
    PE
    AE
    TM
    GQ
    CI
    SY
    TR
    GU
    TT
    VI
    ZW
    MF
    IL
    SC
    GI
    HU
    CN
    GD
    GP
    PF
    NZ
    CV
    NU
    SZ
    VE
    CF
    BV
    EC
    CL
    HK
    KY
    TN
    PL
    AX
    AR
    GH
    KE
    CY
    GN
    GA
    GG
    DO
    NC
    PK
    TD
    HN
    CH
    BD
    BO
    IO
    PY
    KN
    US
    AW
    HR
    ST
    AI
    SL
    LV
    CW
    QA
    FM
    DZ
    MN
    HM
    RU
    WS
    MD
    PM
    LI
    PN
    RE
    SG
    PR
    TW
    KH
    KG
    VC
    SR
    CC
    MS
    NF
    AM
    JP
    UM
    NL
    ME
    KR
    SI
    AU
    NO
    AO
    PT
    FI
    NA
    BF
    PG
    PH
    WF
    GR
    SD
    FO
    FJ
    BW
    CU
    IS
    MZ
    IE
    BM
    UZ
    DE
    VG
    LU
    MG
    BI
    NE
    GE
    GY
    IN
    KP
    LY
    SN
    MA
    UY
    MX
    LT
    VU
    ID
    AF
    PA
    GB
    CK
    MK
    CX
    CZ
    MV
    AL
    GM
    ZA
    SV
    MT
    SE
    EG
    DJ
    LR
    AG
    CA
    KI
    SM
    PW
    AZ
    BH
    MW
    RO
    IM
    TC
    UA
    MY
    TV
    ET
    SS
    GF
    CG
    CM
    CR
    GL
    HT
    IQ
    BE
    SH
    IR
    BA
    BG
    KZ
    TZ
    TG
    TK
    BQ
    AQ
    BY
    JE
    BZ
    YT
    MP
    NP
    LC
    AD
    LA
    RW
    ER
    ML
    SO
    LB
    BR
    LS
    BJ
    JM
    SJ
    JO
    SA
    GW
    ES
    LK
    MU
    TJ
    GT
    TO
    MO
    AT
    EE
    NG
    BT
    BL
    TH
    NI
    SB
    EH
    GS
    MH
    MR
    MQ
    PS
    YE
    SX
    TL
    AS
    BN
    KW
    MC
    TF
    SK
    ZM
    FR
    IT
    MM
    VN
    DK
    RS
    KM
    VA
    BB
    CO
    DM
    FK
    OM
    NR
    BS
}

structure PeachpaymentsData {
    /// Indicates the card-on-file transaction classification to use for Peach Payments when recurring_details.card_with_limited_data is supplied.
    card_on_file_transaction_type: PeachpaymentsCardOnFileTransactionType
    /// A numeric reference number supplied by the system retaining the original source information and used to assist in locating that information or a copy thereof.
    rrn: smithy.api#String
}

/// Details of online mandate
structure OnlineMandate {
    /// The user-agent of the customer's browser
    @required
    user_agent: smithy.api#String
    /// Ip address of the customer machine from which the mandate was created
    @required
    ip_address: smithy.api#String
}

structure PaymentProcessingDetails {
    @required
    payment_processing_certificate: smithy.api#String
    @required
    payment_processing_certificate_key: smithy.api#String
}

/// Represents the specific data for Santander Pix Automatico (recurring PIX payments) Split into CIT (Customer Initiated Transaction) and MIT (Merchant Initiated Transaction) variants
enum PixAutomaticoAdditionalDetails {
    /// Customer Initiated Transaction - used during mandate setup for PixAutomaticoPush Payment Method Type
    pix_automatico_push
    /// Customer Initiated Transaction - used during mandate setup + non 0$ mandate setup for PixAutomaticoQr Payment Method Type
    pix_automatico_qr
    /// Merchant Initiated Transaction - used during recurring charge creation
    pix_automatico_mit
}

structure GiropayNestedType {
    /// The country for bank payment
    country: CountryAlpha2
    /// The billing details for bank redirection
    billing_details: BankRedirectBilling
    /// Bank account bic code
    bank_account_bic: smithy.api#String
    /// Bank account iban
    bank_account_iban: smithy.api#String
}

union WalletData {
    /// The wallet data for Google pay
    google_pay: GooglePayWalletData
    /// The wallet data for Touch n Go Redirection
    touch_n_go_redirect: TouchNGoRedirection
    /// The wallet data for Amazon Pay redirect
    amazon_pay_redirect: AmazonPayRedirectData
    revolut_pay: RevolutPayData
    /// The wallet data for Samsung Pay
    samsung_pay: SamsungPayWalletData
    /// The wallet data for Paypal
    paypal_sdk: PayPalWalletData
    /// The wallet data for Paysera
    paysera: PayseraData
    /// This is for paypal redirection
    paypal_redirect: PaypalRedirection
    swish_qr: SwishQrData
    /// The wallet data for MobilePay redirect
    mobile_pay_redirect: MobilePayRedirection
    /// Wallet data for DANA redirect flow
    dana_redirect: DanaRedirectNestedType
    /// The wallet data for WeChat Pay Redirection
    we_chat_pay_redirect: WeChatPayRedirection
    /// The wallet data for KakaoPay redirect
    kakao_pay_redirect: KakaoPayRedirection
    /// Wallet data for Vipps Redirection
    vipps_redirect: VippsRedirectNestedType
    /// The wallet data for Ali Pay HK redirect
    ali_pay_hk_redirect: AliPayHkRedirection
    /// The wallet data for Bluecode QR Code Redirect
    bluecode_redirect: BluecodeRedirectNestedType
    /// The wallet data for Ali Pay redirect
    ali_pay_redirect: AliPayRedirection
    /// Wallet data for google pay redirect flow
    google_pay_redirect: GooglePayRedirectData
    /// The wallet data for Cashapp Qr
    cashapp_qr: CashappQr
    /// The wallet data for Apple pay
    apple_pay: ApplePayWalletData
    /// Wallet data for MbWay redirect flow
    mb_way_redirect: MbWayRedirection
    /// The wallet data for Skrill
    skrill: SkrillData
    /// The wallet data for GoPay redirect
    go_pay_redirect: GoPayRedirection
    /// The wallet data for Momo redirect
    momo_redirect: MomoRedirection
    /// Wallet data for Google pay third party sdk flow
    google_pay_third_party_sdk: GooglePayThirdPartySdkData
    /// The wallet data for Amazon Pay
    amazon_pay: AmazonPayWalletData
    /// Wallet data for apple pay third party sdk flow
    apple_pay_third_party_sdk: ApplePayThirdPartySdkData
    /// The wallet data for Ali Pay QrCode
    ali_pay_qr: AliPayQr
    /// The wallet data for WeChat Pay Display QrCode
    we_chat_pay_qr: WeChatPayQr
    /// The wallet data for Gcash redirect
    gcash_redirect: GcashRedirection
    mifinity: MifinityData
    /// Wallet data for Twint Redirection
    twint_redirect: TwintRedirectNestedType
    /// The wallet data for Paze
    paze: PazeWalletData
    /// Wallet data for apple pay redirect flow
    apple_pay_redirect: ApplePayRedirectData
}

union BankDebitAdditionalData {
    bacs: BacsBankDebitAdditionalData
    becs: BecsBankDebitAdditionalData
    sepa: SepaBankDebitAdditionalData
    ach: AchBankDebitAdditionalData
    eft_debit_order: EftDebitOrderAdditionalData
}

/// Details of customer attached to this payment
structure CustomerDetailsResponse {
    /// The customer's email address
    email: smithy.api#String
    /// The customer's name
    name: smithy.api#String
    /// Customer’s country-specific identification number and type used for regulatory or tax purposes
    customer_document_details: CustomerDocumentDetails
    /// The identifier for the customer.
    id: smithy.api#String
    /// The customer's phone number
    phone: smithy.api#String
    /// The country code for the customer's phone number
    phone_country_code: smithy.api#String
}

structure CardWithLimitedData {
    /// The card's expiry month
    card_exp_month: smithy.api#String
    /// The Mastercard Transaction Link Identifier (TLID) provided by the card network during a CIT (Customer Initiated Transaction), when `setup_future_usage` is set to `off_session`.
    transaction_link_id: smithy.api#String
    /// The card number
    @required
    card_number: smithy.api#String
    /// The card holder's name
    card_holder_name: smithy.api#String
    /// The card's expiry year
    card_exp_year: smithy.api#String
    /// The ECI(Electronic Commerce Indicator) value for this authentication.
    eci: smithy.api#String
    /// The network transaction ID provided by the card network during a CIT (Customer Initiated Transaction), when `setup_future_usage` is set to `off_session`.
    network_transaction_id: smithy.api#String
}

/// The three-letter ISO 4217 currency code (e.g., "USD", "EUR") for the payment amount. This field is mandatory for creating a payment.
enum Currency {
    DKK
    COP
    SVC
    CZK
    FKP
    UGX
    UAH
    CVE
    KES
    MNT
    EUR
    TRY
    BIF
    ALL
    RSD
    TND
    CLF
    AZN
    QAR
    GYD
    AUD
    CLP
    JOD
    LKR
    AOA
    HKD
    ILS
    TTD
    MXN
    HTG
    KZT
    CUP
    ZAR
    KYD
    AMD
    GBP
    KWD
    AWG
    BRL
    MWK
    BWP
    MKD
    DOP
    SLE
    TJS
    MDL
    ARS
    VND
    XAF
    BDT
    TWD
    INR
    LAK
    STD
    WST
    SGD
    ANG
    CUC
    IRR
    PKR
    SHP
    MYR
    MRU
    PEN
    NZD
    KMF
    HNL
    IQD
    YER
    SZL
    XCD
    CAD
    MOP
    MAD
    PHP
    LYD
    TZS
    STN
    AFN
    BBD
    GEL
    GHS
    IDR
    JMD
    NOK
    VUV
    LSL
    LBP
    ERN
    JPY
    ETB
    MUR
    NGN
    PLN
    SOS
    BSD
    KRW
    DZD
    USD
    ISK
    AED
    TMT
    NAD
    RWF
    PYG
    GIP
    XPF
    GNF
    SDG
    UYU
    BTN
    NPR
    MZN
    SCR
    BOB
    RON
    PGK
    OMR
    RUB
    MVR
    SRD
    FJD
    BGN
    CDF
    KGS
    XOF
    BYN
    THB
    NIO
    BAM
    CNY
    HRK
    BND
    EGP
    TOP
    GMD
    SSP
    MMK
    VES
    ZWL
    PAB
    SEK
    BHD
    UZS
    MGA
    BMD
    BZD
    CHF
    SAR
    GTQ
    SBD
    HUF
    DJF
    CRC
    KHR
    SLL
    SYP
    LRD
    KPW
    ZMW
}

/// Details of surcharge applied on this payment, if applicable
structure RequestSurchargeDetails {
    tax_amount: smithy.api#Long
    @required
    surcharge_amount: smithy.api#Long
}

structure AmazonPayShippingMethod {
    /// Name of the shipping method
    @required
    shipping_method_name: smithy.api#String
    /// Code of the shipping method
    @required
    shipping_method_code: smithy.api#String
}

structure MerchantConnectorDetails {
    /// Account details of the Connector. You can specify up to 50 keys, with key names up to 40 characters long and values up to 500 characters long. Useful for storing additional, structured information on an object.
    connector_account_details: smithy.api#Document
    /// Metadata is useful for storing additional, unstructured information on an object.
    metadata: smithy.api#Document
}

enum ConnectorMetadataResponse {
    santander
}

structure GpayShippingAddressParameters {
    /// Is shipping phone number required
    @required
    phone_number_required: smithy.api#Boolean
}

/// Charge specific fields for controlling the revert of funds from either platform or connected account for Stripe. Check sub-fields for more details.
structure StripeSplitRefundRequest {
    /// Toggle for reverting the application fee that was collected for the payment. If set to false, the funds are pulled from the destination account.
    revert_platform_fee: smithy.api#Boolean
    /// Toggle for reverting the transfer that was made during the charge. If set to false, the funds are pulled from the main platform's account.
    revert_transfer: smithy.api#Boolean
}

structure OnlineBankingCzechRepublicNestedType {
    @required
    issuer: BankNames
}

structure BankRedirectResponse {
    /// Name of the bank
    bank_name: BankNames
    BancontactCard: BancontactBankRedirectAdditionalData
    Giropay: GiropayBankRedirectAdditionalData
    Blik: BlikBankRedirectAdditionalData
}

structure ApplePayPaymentRequest {
    /// The list of merchant capabilities(ex: whether capable of 3ds or no-3ds)
    merchant_capabilities: StringList
    merchant_identifier: smithy.api#String
    /// The code for country
    @required
    country_code: CountryAlpha2
    /// Represents the total for the payment.
    @required
    total: AmountInfo
    /// The required billing contact fields for connector
    required_billing_contact_fields: ApplePayAddressParametersList
    /// The code for currency
    @required
    currency_code: Currency
    /// The required shipping contacht fields for connector
    required_shipping_contact_fields: ApplePayAddressParametersList
    /// The list of supported networks
    supported_networks: StringList
    /// Recurring payment request for apple pay Merchant Token
    recurring_payment_request: ApplePayRecurringPaymentRequest
}

structure AmazonPaySessionTokenResponse {
    /// Amazon Pay store ID
    @required
    store_id: smithy.api#String
    /// Amazon Pay merchant account identifier
    @required
    merchant_id: smithy.api#String
    /// Ledger currency provided during registration for the given merchant identifier
    @required
    ledger_currency: Currency
    /// The total amount for items in the cart
    @required
    total_base_amount: smithy.api#String
    /// The total shipping costs
    @required
    total_shipping_amount: smithy.api#String
    /// The delivery options available for the provided address
    @required
    delivery_options: AmazonPayDeliveryOptionsList
    /// Payment flow for charging the buyer
    @required
    payment_intent: AmazonPayPaymentIntent
    /// The total tax amount for the order
    @required
    total_tax_amount: smithy.api#String
}

/// Represents the rules for applying discounts to a payment, such as a percentage discount or a fixed amount discount, along with any applicable grace periods.
structure PenaltyRules {
    /// Fixed fee applied once after the due date (Fine)
    fixed_penalty: PenaltyDetail
    /// Recurring cost applied over time (Interest)
    interest: InterestDetail
}

structure ApplepaySessionTokenResponse {
    /// Identifier for the delayed session response
    @required
    delayed_session_token: smithy.api#Boolean
    /// Session object for Apple Pay The session_token_data will be null for iOS devices because the Apple Pay session call is skipped, as there is no web domain involved
    session_token_data: ApplePaySessionResponse
    /// Payment request object for Apple Pay
    payment_request_data: ApplePayPaymentRequest
    /// The public key id is to invoke third party sdk
    connector_sdk_public_key: smithy.api#String
    /// The connector merchant id
    connector_merchant_id: smithy.api#String
    /// The session token is w.r.t this connector
    @required
    connector: smithy.api#String
    /// The next action for the sdk (ex: calling confirm or sync call)
    @required
    sdk_next_action: SdkNextAction
    /// The connector transaction id
    connector_reference_id: smithy.api#String
}

structure AffirmRedirectNestedType {
}

structure AmazonPayWalletData {
    /// Checkout Session identifier
    @required
    checkout_session_id: smithy.api#String
}

enum SamsungPayCardBrand {
    unknown
    mastercard
    amex
    discover
    visa
}

structure AmountFilter {
    /// The start amount to filter list of transactions which are greater than or equal to the start amount
    start_amount: smithy.api#Long
    /// The end amount to filter list of transactions which are less than or equal to the end amount
    end_amount: smithy.api#Long
}

enum SantanderMandatePeriodicity {
    /// Every 3 months
    quarterly
    /// Every 6 months
    semiannually
    /// Every week
    weekly
    /// Every year
    annually
    /// Every month
    monthly
}

structure GooglePayAssuranceDetails {
    /// indicates that Cardholder possession validation has been performed
    @required
    card_holder_authenticated: smithy.api#Boolean
    /// indicates that identification and verifications (ID&V) was performed
    @required
    account_verified: smithy.api#Boolean
}

structure ApplePayRegularBillingDetails {
    /// The amount of time — in calendar units, such as day, month, or year — that represents a fraction of the total payment interval
    recurring_payment_interval_unit: RecurringPaymentIntervalUnit
    /// The date of the first payment
    recurring_payment_start_date: smithy.api#String
    /// The number of interval units that make up the total payment interval
    recurring_payment_interval_count: smithy.api#Integer
    /// The date of the final payment
    recurring_payment_end_date: smithy.api#String
}

union PayLaterData {
    /// For Affirm redirect as PayLater Option
    affirm_redirect: AffirmRedirectNestedType
    /// For KlarnaRedirect as PayLater Option
    klarna_redirect: KlarnaRedirectNestedType
    /// For AfterpayClearpay redirect as PayLater Option
    afterpay_clearpay_redirect: AfterpayClearpayRedirectNestedType
    /// For Flexiti Redirect as PayLater long term finance Option
    flexiti_redirect: FlexitiRedirectNestedType
    /// For Alma Redirection as PayLater Option
    alma_redirect: AlmaRedirectNestedType
    breadpay_redirect: BreadpayRedirectNestedType
    /// For Klarna Sdk as PayLater Option
    klarna_sdk: KlarnaSdkNestedType
    atome_redirect: AtomeRedirectNestedType
    /// For PayBright Redirect as PayLater Option
    pay_bright_redirect: PayBrightRedirectNestedType
    /// For WalleyRedirect as PayLater Option
    walley_redirect: WalleyRedirectNestedType
}

structure BankDebitBilling {
    /// The billing address for bank debits
    address: AddressDetails
    /// The billing name for bank debits
    name: smithy.api#String
    /// The billing email for bank debits
    email: smithy.api#String
}

/// Fee information for Split Payments to be charged on the payment being collected for Adyen
structure AdyenSplitData {
    /// The store identifier
    store: smithy.api#String
    /// Data for the split items
    @required
    split_items: AdyenSplitItemList
}

/// SCA Exemptions types available for authentication
enum ScaExemptionType {
    low_value
    transaction_risk_analysis
}

/// Represents the rules for applying discounts to a payment, such as a percentage discount or a fixed amount discount, along with any applicable grace periods.
structure PenaltyDetail {
    /// The numeric value (as a string to preserve decimal precision)
    value: smithy.api#String
    /// Grace period: Days after due date before this applies
    grace_period_days: smithy.api#Integer
}

/// This struct represents the decrypted Google Pay payment data
structure GPayPredecryptData {
    /// The card's expiry year
    @required
    card_exp_year: smithy.api#String
    /// Electronic Commerce Indicator
    eci_indicator: smithy.api#String
    /// Cryptogram generated by the Network
    cryptogram: smithy.api#String
    /// The card's expiry month
    @required
    card_exp_month: smithy.api#String
    /// The Primary Account Number (PAN) of the card
    @required
    application_primary_account_number: smithy.api#String
}

/// Fee information charged on the payment being collected via xendit
structure XenditMultipleSplitResponse {
    /// Identifier for split rule created for the payment
    @required
    split_rule_id: smithy.api#String
    /// Array of objects that define how the platform wants to route the fees and to which accounts.
    @required
    routes: XenditSplitRouteList
    /// Name to identify split rule. Not required to be unique. Typically based on transaction and/or sub-merchant types.
    @required
    name: smithy.api#String
    /// Description to identify fee rule
    @required
    description: smithy.api#String
    /// The sub-account user-id that you want to make this transaction for.
    for_user_id: smithy.api#String
}

structure CimbVaNestedType {
}

/// Passing this object during payments creates a mandate. The mandate_type sub object is passed by the server.
structure MandateData {
    /// A consent from the customer to store the payment method
    customer_acceptance: CustomerAcceptance
    /// A way to select the type of mandate used
    mandate_type: MandateType
    /// A way to update the mandate's payment method details
    update_mandate_id: smithy.api#String
}

structure BankRedirectBilling {
    /// The name for which billing is issued
    billing_name: smithy.api#String
    /// The billing email for bank redirect
    email: smithy.api#String
}

structure DanamonVaBankTransferNestedType {
    /// The billing details for BniVa Bank Transfer
    billing_details: DokuBillingDetails
}

/// The status of a post-capture void operation
enum PostCaptureVoidStatus {
    pending
    failed
    succeeded
}

/// Some connectors like Apple Pay, Airwallex and Noon might require some additional information, find specific details in the child attributes below.
structure ConnectorMetadata {
    airwallex: AirwallexData
    braintree: BraintreeData
    noon: NoonData
    peachpayments: PeachpaymentsData
    apple_pay: ApplepayConnectorMetadataRequest
    santander: SantanderData
    adyen: AdyenConnectorMetadata
}

structure EftDebitOrderAdditionalData {
    /// Name of the bank
    bank_name: BankNames
    /// Bank account type
    bank_type: BankType
    /// Bank account's owner name
    bank_account_holder_name: smithy.api#String
    /// Partially masked branch code for eft bank debit payment
    branch_code: smithy.api#String
    /// Partially masked account number for eft bank debit payment
    @required
    account_number: smithy.api#String
}

structure DokuBillingDetails {
    /// The billing first name for Doku
    first_name: smithy.api#String
    /// The billing second name for Doku
    last_name: smithy.api#String
    /// The Email ID for Doku billing
    email: smithy.api#String
}

structure MbWayRedirection {
    /// Telephone number of the shopper. Should be Portuguese phone number.
    telephone_number: smithy.api#String
}

structure CardWithNoCVC {
    /// The card number
    @required
    card_number: smithy.api#String
    /// The card's expiry year
    @required
    card_exp_year: smithy.api#String
    /// The name of the issuer of card
    card_issuer: smithy.api#String
    /// The card's expiry month
    @required
    card_exp_month: smithy.api#String
    /// The card holder's name
    card_holder_name: smithy.api#String
    /// The card network for the card
    card_network: CardNetwork
    card_type: smithy.api#String
    bank_code: smithy.api#String
    /// The card holder's nick name
    nick_name: smithy.api#String
    card_issuing_country_code: smithy.api#String
    card_issuing_country: smithy.api#String
}

structure VippsRedirectNestedType {
}

enum CaptureStatus {
    pending
    failed
    started
    charged
}

structure SamsungPayAmountDetails {
    /// The total amount of the transaction
    @jsonName("total")
    @required
    total_amount: smithy.api#String
    /// Amount format to be displayed
    @jsonName("option")
    @required
    amount_format: SamsungPayAmountFormat
    /// The currency code
    @required
    currency_code: Currency
}

structure BlikBankRedirectAdditionalData {
    blik_code: smithy.api#String
}

/// additional data that might be required by hyperswitch
structure FeatureMetadata {
    /// Additional tags to be used for global search
    search_tags: StringList
    /// Extra information for Pix Payment Method Type like fine expiry, pix key etc
    pix_additional_details: PixAdditionalDetails
    /// Redirection response coming in request as metadata field only for redirection scenarios
    redirect_response: RedirectResponse
    /// Recurring payment details required for apple pay Merchant Token
    apple_pay_recurring_details: ApplePayRecurringDetails
    /// Extra information like fine percentage, interest percentage etc required for Pix payment method
    boleto_additional_details: BoletoAdditionalDetails
    /// Pix Automatico additional details for Push Notification and QR based flows
    pix_automatico_additional_details: PixAutomaticoAdditionalDetails
    /// Extra information for Finix connector for fraud checks and risk evaluation
    finix_additional_details: FinixAdditionalDetails
}

structure EftDebitOrderNestedType {
    /// Billing details for bank debit
    billing_details: BankDebitBilling
    /// Account number for eft bank debit payment
    @required
    account_number: smithy.api#String
    /// Branch code for eft bank debit payment
    branch_code: smithy.api#String
    bank_name: BankNames
    bank_account_holder_name: smithy.api#String
    bank_type: BankType
}

enum RecurringPaymentIntervalUnit {
    minute
    month
    day
    year
    hour
}

structure InstantBankTransferPolandNestedType {
}

/// Payment method data request for eligibility check
union EligibilityPaymentMethodDataRequest {
    card_redirect: CardRedirectData
    voucher: VoucherData
    open_banking: OpenBankingData
    mobile_payment: MobilePaymentData
    reward: smithy.api#Unit
    mandate_payment: smithy.api#Unit
    real_time_payment: RealTimePaymentData
    crypto: CryptoData
    gift_card: GiftCardData
    card: EligibilityCard
    bank_redirect: BankRedirectData
    card_token: CardToken
    upi: UpiData
    bank_transfer: BankTransferData
    bank_debit: BankDebitData
    pay_later: PayLaterData
}

structure MandateResponse {
    /// The card details for mandate
    card: MandateCardDetails
    /// The identifier for payment method
    @required
    payment_method_id: smithy.api#String
    /// The identifier for mandate
    @required
    mandate_id: smithy.api#String
    /// Details about the customer’s acceptance
    customer_acceptance: CustomerAcceptance
    /// The payment method
    @required
    payment_method: smithy.api#String
    /// The payment method type
    payment_method_type: smithy.api#String
    /// The status for mandates
    @required
    status: MandateStatus
}

/// The status for refunds
enum RefundStatus {
    review
    pending
    succeeded
    failed
}

structure ThirdPartySdkSessionResponse {
    @required
    secrets: SecretInfoToInitiateSdk
}

/// Details required for recurring payment
structure RecurringDetails {
    field_0: CardWithLimitedData
    /// Discriminator field for the tagged enum
    @required
    type: RecurringDetailsEnumVariants
}

/// This is used to indicate if the mandate was accepted online or offline
enum AcceptanceType {
    /// Online
    online
    /// Offline
    offline
}

structure BcaBankTransferNestedType {
    /// The billing details for BCA Bank Transfer
    billing_details: DokuBillingDetails
}

structure CustomerDeleteResponse {
    /// Whether address was deleted or not
    @required
    address_deleted: smithy.api#Boolean
    /// Whether customer was deleted or not
    @required
    customer_deleted: smithy.api#Boolean
    /// The identifier for the customer object
    @required
    customer_id: smithy.api#String
    /// Whether payment methods deleted or not
    @required
    payment_methods_deleted: smithy.api#Boolean
}

structure MandiriVaNestedType {
}

/// Enum variants for PaymentProcessingDetailsAt
enum PaymentProcessingDetailsAtEnumVariants {
    Hyperswitch
    Connector
}

/// This "CustomerAcceptance" object is passed during Payments-Confirm request, it enlists the type, time, and mode of acceptance properties related to an acceptance done by the customer. The customer_acceptance sub object is usually passed by the SDK or client.
structure CustomerAcceptance {
    /// Type of acceptance provided by the
    @required
    acceptance_type: AcceptanceType
    /// Information required for online mandate generation
    online: OnlineMandate
    /// Specifying when the customer acceptance was provided
    accepted_at: smithy.api#Timestamp
}

list StringList {
    member: smithy.api#String
}

structure TrustlyNestedType {
    /// The country for bank payment
    country: CountryAlpha2
}

/// The type of the payment that differentiates between normal and various types of mandate payments. Use 'setup_mandate' in case of zero auth flow.
enum PaymentType {
    recurring_mandate
    setup_mandate
    new_mandate
    normal
    installment
}

structure MandiriVaBankTransferNestedType {
    /// The billing details for BniVa Bank Transfer
    billing_details: DokuBillingDetails
}

structure Card {
    card_type: smithy.api#String
    bank_code: smithy.api#String
    /// The card holder's nick name
    nick_name: smithy.api#String
    /// The card holder's name
    card_holder_name: smithy.api#String
    /// The CVC number for the card
    @required
    card_cvc: smithy.api#String
    /// The card's expiry month
    @required
    card_exp_month: smithy.api#String
    /// The card network for the card
    card_network: CardNetwork
    /// The card's expiry year
    @required
    card_exp_year: smithy.api#String
    card_issuing_country_code: smithy.api#String
    /// The name of the issuer of card
    card_issuer: smithy.api#String
    card_issuing_country: smithy.api#String
    /// The card number
    @required
    card_number: smithy.api#String
}

structure AchBankTransferNestedType {
    /// The billing details for ACH Bank Transfer
    billing_details: AchBillingDetails
}

structure BreadpayRedirectNestedType {
}

structure AchBillingDetails {
    /// The Email ID for ACH billing
    email: smithy.api#String
}

union UpiAdditionalData {
    upi_collect: UpiCollectAdditionalData
    upi_intent: UpiIntentData
}

list XenditSplitRouteList {
    member: XenditSplitRoute
}

structure PixEmvNestedType {
}

list DisputeResponsePaymentsRetrieveList {
    member: DisputeResponsePaymentsRetrieve
}

structure GpayTransactionInfo {
    /// The total price
    @required
    total_price: smithy.api#String
    /// The currency code
    @required
    currency_code: Currency
    /// The total price status (ex: 'FINAL')
    @required
    total_price_status: smithy.api#String
    /// The country code
    @required
    country_code: CountryAlpha2
}

structure CryptoData {
    pay_currency: smithy.api#String
    network: smithy.api#String
}

structure Przelewy24NestedType {
    bank_name: BankNames
    billing_details: BankRedirectBilling
}

list ApplePayAddressParametersList {
    member: ApplePayAddressParameters
}

list CardNetworkList {
    member: CardNetwork
}

/// Defines the type of discount applied to a payment, such as whether it's a fixed date discount, a daily calendar discount, or a daily business discount.
structure SantanderPaymentDiscountRules {
    /// Generic label for the logic (e.g., "tier_based", "early_bird")
    discount_type: DiscountType
    /// A generic vector of discount tiers
    tiers: DiscountType
}

structure GpayTokenizationSpecification {
    /// The token specification type(ex: PAYMENT_GATEWAY)
    @jsonName("type")
    @required
    token_specification_type: smithy.api#String
    /// The parameters for the token specification Google Pay
    @required
    parameters: GpayTokenParameters
}

structure PazeSessionTokenResponse {
    /// Email Address
    email_address: smithy.api#String
    /// Paze Client ID
    @required
    client_id: smithy.api#String
    /// The transaction currency code
    @required
    transaction_currency_code: Currency
    /// Client Name to be displayed on the Paze screen
    @required
    client_name: smithy.api#String
    /// Paze Client Profile ID
    @required
    client_profile_id: smithy.api#String
    /// The transaction amount
    @required
    transaction_amount: smithy.api#String
}

/// This struct represents the encrypted Gpay payment data
structure GpayEcryptedTokenizationData {
    /// Token generated for the wallet
    @required
    token: smithy.api#String
    /// The type of the token
    @jsonName("type")
    @required
    token_type: smithy.api#String
}

structure CustomerResponse {
    /// The identifier for the customer object
    @required
    customer_id: smithy.api#String
    /// The customer's name
    name: smithy.api#String
    /// An arbitrary string that you can attach to a customer object.
    description: smithy.api#String
    /// The customer's tax registration number.
    tax_registration_id: smithy.api#String
    /// The address for the customer
    address: AddressDetails
    /// Customer’s country-specific identification number and type used for regulatory or tax purposes
    document_details: CustomerDocumentDetails
    /// The customer's email address
    email: smithy.api#String
    /// You can specify up to 50 keys, with key names up to 40 characters long and values up to 500 characters long. Metadata is useful for storing additional, structured information on an object.
    metadata: smithy.api#Document
    /// The customer's phone number
    phone: smithy.api#String
    /// The country code for the customer phone number
    phone_country_code: smithy.api#String
    /// A timestamp (ISO 8601 code) that determines when the customer was created
    created_at: smithy.api#String
    /// The identifier for the default payment method.
    default_payment_method_id: smithy.api#String
}

structure SepaBankTransferPaymentAdditionalData {
    /// debitor name
    debitor_name: smithy.api#String
    /// debitor IBAN
    debitor_iban: smithy.api#String
    /// debitor BIC
    debitor_bic: smithy.api#String
    /// debitor email
    debitor_email: smithy.api#String
}

union BankTransferAdditionalData {
    instant_bank_transfer: InstantBankTransferNestedType
    bacs: BacsNestedType
    permata: PermataNestedType
    pix: PixBankTransferAdditionalData
    sepa: SepaBankTransferPaymentAdditionalData
    pse: PseNestedType
    instant_bank_transfer_finland: InstantBankTransferFinlandNestedType
    indonesian_bank_transfer: IndonesianBankTransferNestedType
    instant_bank_transfer_poland: InstantBankTransferPolandNestedType
    bri_va: BriVaNestedType
    bni_va: BniVaNestedType
    pix_automatico_push: PixAutomaticoPushAdditionalData
    bca: BcaNestedType
    pix_qr: PixQrNestedType
    mandiri_va: MandiriVaNestedType
    pix_automatico_qr: PixAutomaticoQrNestedType
    pix_emv: PixEmvNestedType
    local_bank_transfer: LocalBankTransferAdditionalData
    multibanco: MultibancoNestedType
    danamon_va: DanamonVaNestedType
    ach: AchNestedType
    cimb_va: CimbVaNestedType
}

/// Xendit Charge Request
union XenditSplitRequest {
    /// Split Between Multiple Accounts
    multiple_splits: XenditMultipleSplitRequest
    /// Collect Fee for Single Account
    single_split: XenditSplitSubMerchantData
}

structure NetworkTokenData {
    /// The country in which the card was issued
    card_issuing_country: smithy.api#String
    /// The name of the issuer of card
    card_issuer: smithy.api#String
    /// The network token
    @required
    network_token: smithy.api#String
    /// The token cryptogram
    @required
    token_cryptogram: smithy.api#String
    /// The type of the card such as Credit, Debit
    card_type: smithy.api#String
    /// The card network for the card
    card_network: CardNetwork
    /// The card holder's name
    card_holder_name: smithy.api#String
    /// The card holder's nick name
    nick_name: smithy.api#String
    /// The ECI(Electronic Commerce Indicator) value for this authentication.
    eci: smithy.api#String
    /// The Payment Account Reference (PAR) for this card.
    par: smithy.api#String
    /// The token's expiry year
    @required
    token_exp_year: smithy.api#String
    /// The bank code of the bank that issued the card
    bank_code: smithy.api#String
    /// The token's expiry month
    @required
    token_exp_month: smithy.api#String
}

structure BizumNestedType {
}

structure PixAutomaticoQrNestedType {
}

/// Fee information to be charged on the payment being collected via xendit
structure XenditSplitRoute {
    /// ID of the destination account where the amount will be routed to
    @required
    destination_account_id: smithy.api#String
    /// Amount of payments to be split
    flat_amount: smithy.api#Long
    /// Reference ID which acts as an identifier of the route itself
    @required
    reference_id: smithy.api#String
    /// Amount of payments to be split, using a percent rate as unit
    percent_amount: smithy.api#Long
    /// Currency code
    @required
    currency: Currency
}

/// Name of banks supported by Hyperswitch
enum BankNames {
    tsb_bank
    bank_nowy_s_a
    rhb_bank
    dolomitenbank
    aib
    etransfer_pocztowy24
    the_siam_commercial_bank
    chase
    am_bank
    bank_millennium
    hypo_alpeadriabank_international_ag
    public_bank
    arzte_und_apotheker_bank
    bank_p_e_k_a_o_s_a
    standard_chartered_bank
    sporo_pay
    discover
    uob_bank
    maybank
    platnosc_online_karta_platnicza
    banki_spoldzielcze
    e_transfer_pocztowy24
    bank_nowy_bfg_sa
    velo_bank
    viamo
    bank_pekao_sa
    starling
    sparda_bank_wien
    yoursafe
    capital_bank_grawe_gruppe_ag
    bank_of_scotland
    b_n_p_paribas_poland
    lloyds
    schoellerbank_ag
    pay_with_plus_bank
    credit_agricole
    knab
    bks_bank_ag
    bankhaus_schelhammer_und_schattera_ag
    oberbank_ag
    open_bank_success
    alior_bank
    bank_of_america
    blik
    moneyou
    bank_austria
    pay_with_alior_bank
    bunq
    bankhaus_carl_spangler
    hypo_noe_lb_fur_niederosterreich_u_wien
    sns_bank
    citi
    volksbank_gruppe
    volkskreditbank_ag
    bank_simpanan_nasional
    barclays
    n26
    austrian_anadi_bank_ag
    osterreichische_arzte_und_apothekerbank
    easybank_ag
    hsbc_bank
    m_bank
    toyota_bank
    btv_vier_lander_bank
    ulster_bank
    bank_rakyat
    marchfelder_bank
    bank_islam
    friesland_bank
    pay_with_inteligo
    bank99_ag
    tesco_bank
    handelsbanken
    american_express
    blik_p_s_p
    posojilnica_bank_e_gen
    pentagon_federal_credit_union
    hypo_oberosterreich_salzburg_steiermark
    pay_with_b_o_s
    erste_bank_und_sparkassen
    getin_bank
    ocbc_bank
    monzo
    tatra_pay
    asn_bank
    raiffeisen_bankengruppe_osterreich
    plus_bank
    envelo_bank
    volkswagen_bank
    royal_bank_of_scotland
    danske_bank
    affin_bank
    regiobank
    bank_of_china
    cimb_bank
    hypo_bank_burgenland_aktiengesellschaft
    triodos_bank
    santander_przelew24
    pay_with_citi_handlowy
    nest_przelew
    nationale_nederlanden
    hong_leong_bank
    revolut
    kasikorn_bank
    open_bank_cancelled
    capital_one
    e_platby_v_u_b
    krungsri_bank
    nat_west
    vr_bank_braunau
    synchrony_bank
    agro_bank
    ceska_sporitelna
    hypo_tirol_bank_ag
    ing
    rabobank
    first_direct
    first_trust
    bnp_paribas
    postova_banka
    bank_muamalat
    bangkok_bank
    wells_fargo
    pbac_z_ipko
    place_z_i_p_k_o
    schelhammer_capital_bank_ag
    idea_bank
    brull_kallmus_bank_ag
    nationwide_bank
    alliance_bank
    banki_spbdzielcze
    kuwait_finance_house
    bawag_psk_ag
    boz
    pay_with_i_n_g
    absa
    inteligo
    open_bank_failure
    hypo_vorarlberg_bank_ag
    krung_thai_bank
    van_lanschot
    navy_federal_credit_union
    mbank_mtransfer
    komercni_banka
    noble_pay
    abn_amro
    halifax
}

structure SamsungPayAppWalletData {
    /// Last 4 digits of the card number
    @required
    payment_last4_fpan: smithy.api#String
    /// Value if credential is enabled for recurring payment
    recurring_payment: smithy.api#Boolean
    /// Merchant reference id that was passed in the session call request
    merchant_ref: smithy.api#String
    /// Samsung Pay token data
    @jsonName("3_d_s")
    @required
    token_data: SamsungPayTokenData
    /// Brand of the payment card
    @required
    payment_card_brand: SamsungPayCardBrand
    /// Last 4 digits of the device specific card number
    payment_last4_dpan: smithy.api#String
    /// Currency type of the payment
    @required
    payment_currency_type: smithy.api#String
    /// Specifies authentication method used
    method: smithy.api#String
}

/// Fee information for Split Payments to be charged on the payment being collected
union SplitPaymentsRequest {
    /// XenditSplitPayment
    xendit_split_payment: XenditSplitRequest
    /// StripeSplitPayment
    stripe_split_payment: StripeSplitPaymentRequest
    /// AdyenSplitPayment
    adyen_split_payment: AdyenSplitData
}

union MandateType {
    /// If the mandate should be valid for multiple debits
    multi_use: MandateAmountData
    /// If the mandate should only be valid for 1 off-session use
    single_use: MandateAmountData
}

structure MobilePayRedirection {
}

union ThreeDsMethodData {
    AcsThreeDsMethodData: ThreeDsMethodDataAcsThreeDsMethodDataData
}

structure SepaBankTransferInstructions {
    @required
    iban: smithy.api#String
    @required
    bic: smithy.api#String
    @required
    account_holder_name: smithy.api#String
    @required
    country: smithy.api#String
    @required
    reference: smithy.api#String
}

structure BlikNestedType {
    blik_code: smithy.api#String
}

structure AchBankDebitNestedType {
    /// Account number for ach bank debit payment
    @required
    account_number: smithy.api#String
    bank_name: BankNames
    bank_holder_type: BankHolderType
    bank_type: BankType
    /// Billing details for bank debit
    billing_details: BankDebitBilling
    bank_account_holder_name: smithy.api#String
    /// Routing number for ach bank debit payment
    @required
    routing_number: smithy.api#String
}

structure SwishQrData {
}

structure GooglePayThirdPartySdkData {
    token: smithy.api#String
}

/// Stage of the dispute
enum DisputeStage {
    arbitration
    dispute_reversal
    pre_dispute
    pre_arbitration
    dispute
}

/// Status of the dispute
enum DisputeStatus {
    dispute_expired
    dispute_cancelled
    dispute_challenged
    dispute_won
    dispute_opened
    dispute_accepted
    dispute_lost
}

/// This enum is used to represent the Gpay payment data, which can either be encrypted or decrypted.
union GpayTokenizationData {
    /// This variant contains the encrypted Gpay payment data as a string.
    encrypted: GpayEcryptedTokenizationData
    /// This variant contains the decrypted Gpay payment data as a structured object.
    decrypted: GPayPredecryptData
}

structure CardTokenResponse {
    /// The card holder's name
    @required
    card_holder_name: smithy.api#String
}

structure RewardNestedType {
}

structure BacsBankDebitNestedType {
    /// Billing details for bank debit
    billing_details: BankDebitBilling
    /// Sort code for Bacs payment method
    @required
    sort_code: smithy.api#String
    /// Account number for Bacs payment method
    @required
    account_number: smithy.api#String
    /// holder name for bank debit
    bank_account_holder_name: smithy.api#String
}

structure CardResponse {
    card_isin: smithy.api#String
    card_extended_bin: smithy.api#String
    card_holder_name: smithy.api#String
    card_type: smithy.api#String
    card_exp_year: smithy.api#String
    card_network: CardNetwork
    card_exp_month: smithy.api#String
    authentication_data: smithy.api#Document
    card_issuer: smithy.api#String
    card_issuing_country: smithy.api#String
    payment_checks: smithy.api#Document
    auth_code: smithy.api#String
    last4: smithy.api#String
}

structure CryptoResponse {
    network: smithy.api#String
    pay_currency: smithy.api#String
}

structure IndomaretVoucherData {
    /// The billing first name for Alfamart
    first_name: smithy.api#String
    /// The Email ID for Alfamart
    email: smithy.api#String
    /// The billing second name for Alfamart
    last_name: smithy.api#String
}

structure ApplePayRedirectData {
}

union WalletResponse {
    samsung_pay: WalletAdditionalDataForCard
    apple_pay: WalletAdditionalDataForCard
    google_pay: WalletAdditionalDataForCard
}

/// Address details
structure AddressDetails {
    /// The address state
    state: smithy.api#String
    /// The city, district, suburb, town, or village of the address.
    city: smithy.api#String
    /// The first line of the street address or P.O. Box.
    line1: smithy.api#String
    /// The last name for the address
    last_name: smithy.api#String
    /// The two-letter ISO 3166-1 alpha-2 country code (e.g., US, GB).
    country: CountryAlpha2
    /// The zip/postal code for the address
    zip: smithy.api#String
    /// The second line of the street address or P.O. Box (e.g., apartment, suite, unit, or building).
    line2: smithy.api#String
    /// The first name for the address
    first_name: smithy.api#String
    /// The third line of the street address, if applicable.
    line3: smithy.api#String
    /// The zip/postal code of the origin
    origin_zip: smithy.api#String
}

structure PaymentsCaptureRequest {
    /// The amount to capture, in the lowest denomination of the currency. If omitted, the entire `amount_capturable` of the payment will be captured. Must be less than or equal to the current `amount_capturable`.
    amount_to_capture: smithy.api#Long
    /// Decider to refund the uncaptured amount. (Currently not fully supported or behavior may vary by connector).
    refund_uncaptured_amount: smithy.api#Boolean
    /// The unique identifier for the merchant. This is usually inferred from the API key.
    merchant_id: smithy.api#String
    /// A dynamic suffix that appears on your customer's credit card statement. This is concatenated with the (shortened) descriptor prefix set on your account to form the complete statement descriptor. The combined length should not exceed connector-specific limits (typically 22 characters). To be deprecated soon, use billing_descriptor instead.
    statement_descriptor_suffix: smithy.api#String
    /// An optional prefix for the statement descriptor that appears on your customer's credit card statement. This can override the default prefix set on your merchant account. The combined length of prefix and suffix should not exceed connector-specific limits (typically 22 characters).
    statement_descriptor_prefix: smithy.api#String
    /// Merchant connector details used to make payments. (Deprecated)
    merchant_connector_details: MerchantConnectorDetailsWrap
}

structure IncrementalAuthorizationResponse {
    /// The status of the authorization
    @required
    status: AuthorizationStatus
    /// Error message sent by the connector for authorization
    error_message: smithy.api#String
    /// The unique identifier of authorization
    @required
    authorization_id: smithy.api#String
    /// Previously authorized amount for the payment
    @required
    previously_authorized_amount: smithy.api#Long
    /// Error code sent by the connector for authorization
    error_code: smithy.api#String
    /// Amount the authorization has been made for
    @required
    amount: smithy.api#Long
}

enum ApplePayAddressParameters {
    email
    postalAddress
    phone
}

structure ClickToPaySessionResponse {
    phone_country_code: smithy.api#String
    @required
    card_brands: CardNetworkList
    @required
    dpa_name: smithy.api#String
    @required
    acquirer_merchant_id: smithy.api#String
    merchant_category_code: smithy.api#String
    dpa_client_id: smithy.api#String
    phone_number: smithy.api#String
    email: smithy.api#String
    /// provider Eg: Visa, Mastercard
    provider: CtpServiceProvider
    @required
    dpa_id: smithy.api#String
    @required
    transaction_amount: smithy.api#String
    @required
    locale: smithy.api#String
    @required
    acquirer_bin: smithy.api#String
    @required
    merchant_country_code: smithy.api#String
    @required
    transaction_currency_code: Currency
}

structure FlexitiRedirectNestedType {
}

structure MandateRevokedResponse {
    /// If there was an error while calling the connectors the code is received here
    error_code: smithy.api#String
    /// The status for mandates
    @required
    status: MandateStatus
    /// The identifier for mandate
    @required
    mandate_id: smithy.api#String
    /// If there was an error while calling the connector the error message is received here
    error_message: smithy.api#String
}

/// Describes the channel through which the payment was initiated.
enum PaymentChannel {
    ecommerce
    other
    mail_order
    telephone_order
}

structure MultibancoTransferInstructions {
    @required
    entity: smithy.api#String
    @required
    reference: smithy.api#String
}

structure AdyenTestingData {
    /// Holder name to be sent to Adyen for a card payment(CIT) or a generic payment(MIT). This value overrides the values for card.card_holder_name and applies during both CIT and MIT payment transactions.
    @required
    holder_name: smithy.api#String
}

/// Denotes the retry action
enum RetryAction {
    /// Denotes that the payment is requeued
    requeue
    /// Manual retry through request is being deprecated, now it is available through profile
    manual_retry
}

/// Specifies how the payment method can be used for future payments. - `off_session`: The payment method can be used for future payments when the customer is not present. - `on_session`: The payment method is intended for use only when the customer is present during checkout. If omitted, defaults to `on_session`.
enum FutureUsage {
    off_session
    on_session
}

/// Defines how the payment amount is calculated for penalties or discounts, either as a percentage or as a flat amount.
enum CalculationType {
    /// The value is treated as a percentage (e.g., "2.00" represents 2%). In financial contexts, this is often used for late fees (fines) or monthly interest rates.
    percentage
    /// The value is treated as a fixed monetary amount in the currency's major or minor unit (e.g., "10.00" represents $10.00). Typically used for flat-fee penalties or specific rebate amounts.
    flat_amount
}

structure DocumentDetails {
}

/// Defines the type of payment allowance for a boleto, such as whether only the exact amount is accepted, partial payments are allowed, or overpayment is permitted.
enum PaymentAllowanceType {
    /// Overpayment allowed (common in some B2B contexts)
    Flexible
    /// Only the exact amount is accepted
    Exact
    /// Any amount between min and max
    Partial
}

list ConnectorList {
    member: Connector
}

/// Account type for Santander Pix Automatico recurring charges
enum AccountType {
    /// Checking account (Conta Corrente)
    current
    /// Savings account (Conta Poupança)
    savings
    /// Payment account (Conta Pagamento)
    payment
}

structure EpsNestedType {
    /// The country for bank payment
    country: CountryAlpha2
    /// The hyperswitch bank code for eps
    bank_name: BankNames
    /// The billing details for bank redirection
    billing_details: BankRedirectBilling
}

structure GpayBillingAddressParameters {
    /// Billing address format
    @required
    format: GpayBillingAddressFormat
    /// Is billing phone number required
    @required
    phone_number_required: smithy.api#Boolean
}

structure CashappQr {
}

structure PayseraData {
}

structure GoPayRedirection {
}

structure PollConfigResponse {
    /// Interval of the poll
    @required
    delay_in_secs: smithy.api#Integer
    /// Poll Id
    @required
    poll_id: smithy.api#String
    /// Frequency of the poll
    @required
    frequency: smithy.api#Integer
}

structure NetworkTokenResponse {
    /// The last four digit of the network token
    last4: smithy.api#String
    /// The type of the card such as Credit, Debit
    card_type: smithy.api#String
    /// The card network for the card
    card_network: CardNetwork
    /// The ISIN of the token
    token_isin: smithy.api#String
    /// The name of the issuer of card
    card_issuer: smithy.api#String
    /// The country in which the card was issued
    card_issuing_country: smithy.api#String
    /// The expiry month of the network token
    token_exp_month: smithy.api#String
    /// The card holder's name
    card_holder_name: smithy.api#String
    /// The Payment Account Reference (PAR) for this card
    par: smithy.api#String
    /// The expiry year of the network token
    token_exp_year: smithy.api#String
}

structure GooglePayWalletData {
    /// User-facing message to describe the payment method that funds this transaction.
    @required
    description: smithy.api#String
    /// The tokenization data of Google pay
    @required
    tokenization_data: smithy.api#Document
    /// The information of the payment method
    @required
    info: GooglePayPaymentMethodInfo
    /// The type of payment method
    @jsonName("type")
    @required
    pm_type: smithy.api#String
}

/// Represents the legal or administrative actions that may be taken for non-payment, such as protest rules and automatic write-off timelines.
structure CollectionActions {
    /// Days after which the bill is automatically cancelled/written off
    auto_write_off_days: smithy.api#Integer
    /// Logic for legal protest (official debt registration)
    legal_protest: ProtestRules
}

structure CardTokenAdditionalData {
    /// The card holder's name
    @required
    card_holder_name: smithy.api#String
}

structure AchNestedType {
}

structure DirectCarrierBillingNestedType {
    /// The phone number of the user
    @required
    msisdn: smithy.api#String
    /// Unique user id
    client_uid: smithy.api#String
}

/// Passing this object creates a new customer or attaches an existing customer to the payment
structure CustomerDetails {
    /// The identifier for the customer.
    id: smithy.api#String
    /// The customer's phone number
    phone: smithy.api#String
    /// The customer's name
    name: smithy.api#String
    /// The customer's email address
    email: smithy.api#String
    /// The country code for the customer's phone number
    phone_country_code: smithy.api#String
    /// The tax registration identifier of the customer.
    tax_registration_id: smithy.api#String
    /// Customer’s country-specific identification number and type used for regulatory or tax purposes
    document_details: CustomerDocumentDetails
}

structure PixNestedType {
    /// Unique key for pix transfer
    pix_key: smithy.api#String
    /// Source bank account number
    source_bank_account_id: smithy.api#String
    /// CNPJ is a Brazilian company tax identification number
    cnpj: smithy.api#String
    /// CPF is a Brazilian tax identification number
    cpf: smithy.api#String
    /// Partially masked destination bank account number _Deprecated: Will be removed in next stable release._
    destination_bank_account_id: smithy.api#String
    /// The expiration date and time for the Pix QR code in ISO 8601 format
    expiry_date: smithy.api#String
}

structure WeChatPay {
}

structure NextActionData {
    poll_config: PollConfig
    ddc_data: com.hyperswitch.default#DDCData
    /// Discriminator field for the tagged enum
    @required
    type: NextActionDataEnumVariants
    redirect_to_url: smithy.api#String
    /// The url for Qr code given by the connector
    qr_code_url: smithy.api#String
    popup_url: smithy.api#String
    display_from_timestamp: smithy.api#Long
    /// Hyperswitch generated image data source url
    image_data_url: smithy.api#String
    bank_transfer_steps_and_charges_details: BankTransferNextStepsData
    session_token: smithy.api#Document
    voucher_details: VoucherNextStepData
    border_color: smithy.api#String
    redirect_response_url: smithy.api#String
    next_action_data: SdkNextActionData
    display_text: smithy.api#String
    iframe_data: IframeData
    display_to_timestamp: smithy.api#Long
    three_ds_data: ThreeDsData
    consent_data_required: MobilePaymentConsent
    qr_code_fetch_url: smithy.api#String
    /// The raw QR code data (EMV copy and paste) used for Brazilian payment methods like Pix
    raw_qr_data: smithy.api#String
}

enum MobilePaymentConsent {
    consent_not_required
    consent_required
    consent_optional
}

/// The source type for UPI payments. This indicates what payment source is being used for the UPI transaction.
enum UpiSource {
    /// UPI payment using a credit line
    UPI_CL
    /// UPI payment using a prepaid payment instrument
    UPI_PPI
    /// UPI payment using a voucher
    UPI_VOUCHER
    /// UPI payment using a bank account (savings)
    UPI_ACCOUNT
    /// UPI payment using a credit card
    UPI_CC
    /// UPI payment using a combination of credit card and credit line
    UPI_CC_CL
}

structure BacsBankTransferNestedType {
    /// The billing details for SEPA
    billing_details: SepaAndBacsBillingDetails
}

structure PaymentsResponse {
    /// If the payment intent was cancelled, this field provides a textual reason for the cancellation (e.g., "requested_by_customer", "abandoned").
    cancellation_reason: smithy.api#String
    /// Token containing encoded information for sdk authorization.
    sdk_authorization: smithy.api#String
    /// The shipping address for the payment
    shipping: Address
    /// For non-card charges, you can use this value as the complete description that appears on your customers’ statements. Must contain at least one letter, maximum 22 characters. To be deprecated soon, use billing_descriptor instead.
    statement_descriptor_name: smithy.api#String
    /// Connector Identifier for the payment method
    connector_mandate_id: smithy.api#String
    /// Unique identifier for the payment. This ensures idempotency for multiple payments that have been done by a single merchant.
    @required
    payment_id: smithy.api#String
    /// A unique identifier for the payment method used in this payment. If the payment method was saved or tokenized, this ID can be used to reference it for future transactions or recurring payments. Refer `payment_method_tokenization_details` for detailed view of payment method tokenization
    payment_method_id: smithy.api#String
    /// Set to true to indicate that the customer is not in your checkout flow during this payment, and therefore is unable to authenticate. This parameter is intended for scenarios where you collect card details and charge them later. This parameter can only be used with confirm=true.
    off_session: smithy.api#Boolean
    /// The identifier for the processor merchant account. In platform-connected setups, this is the connected merchant ID. For standard merchants, this is same as merchant_id.
    @required
    processor_merchant_id: smithy.api#String
    /// If true the payment can be retried with same or different payment method which means the confirm call can be made again.
    manual_retry_allowed: smithy.api#Boolean
    /// The connector-specific error code from the last failed payment attempt associated with this payment intent.
    error_code: smithy.api#String
    /// A unique identifier to link the payment to a mandate, can be used instead of payment_method_data, in case of setting up recurring payments
    mandate_id: smithy.api#String
    /// Complete error details containing unified, issuer, and connector-level error information.
    error_details: PaymentErrorDetails
    /// Describes the type of payment flow experienced by the customer (e.g., 'redirect_to_url', 'invoke_sdk', 'display_qr_code').
    payment_experience: PaymentExperience
    /// The payment net amount. net_amount = amount + surcharge_details.surcharge_amount + surcharge_details.tax_amount + shipping_cost + order_tax_amount, If no surcharge_details, shipping_cost, order_tax_amount, net_amount = amount
    @required
    net_amount: smithy.api#Long
    /// Indicates that you intend to make future payments with this Payment’s payment method. Providing this parameter will attach the payment method to the Customer, if present, after the Payment is confirmed and any required actions from the user are complete.
    setup_future_usage: FutureUsage
    /// The shipping cost for the payment.
    shipping_cost: smithy.api#Long
    /// List of disputes that happened on this intent
    disputes: DisputeResponsePaymentsRetrieveList
    /// An optional sub-label for further categorization of the business unit or profile used for this payment. To be deprecated soon. Pass the profile_id instead
    business_sub_label: smithy.api#String
    /// Allowed Payment Method Types for a given PaymentIntent
    allowed_payment_method_types: PaymentMethodTypeList
    /// A unique identifier for a payment provided by the connector
    connector_transaction_id: smithy.api#String
    /// The two-letter ISO country code (e.g., US, GB) of the business unit or profile under which this payment was processed. To be deprecated soon. Pass the profile_id instead
    business_country: CountryAlpha2
    /// Additional data that might be required by hyperswitch, to enable some specific features.
    feature_metadata: FeatureMetadata
    /// The specific payment method subtype used for this payment (e.g., 'credit_card', 'klarna', 'gpay'). This provides more granularity than the 'payment_method' field.
    payment_method_type: PaymentMethodType
    /// Identifier of the connector ( merchant connector account ) which was chosen to make the payment
    merchant_connector_id: smithy.api#String
    /// The label identifying the specific business unit or profile under which this payment was processed by the merchant. To be deprecated soon. Pass the profile_id instead
    business_label: smithy.api#String
    /// Details of external authentication
    external_authentication_details: ExternalAuthenticationDetailsResponse
    /// Details of surcharge applied on this payment
    surcharge_details: RequestSurchargeDetails
    /// date and time after which this payment cannot be captured
    capture_before: smithy.api#String
    /// Flag indicating if external 3ds authentication is made or not
    external_3ds_authentication_attempted: smithy.api#Boolean
    /// Bool indicating if overcapture  must be requested for this payment
    enable_overcapture: smithy.api#Boolean
    /// Boolean indicating whether overcapture is effectively enabled for this payment
    is_overcapture_enabled: smithy.api#Boolean
    /// The Mastercard Transaction Link Identifier (TLID) for this payment. Returned on CITs that set up stored credentials. External-vault merchants should persist this and echo it back on subsequent MIT requests. Mandatory for Mastercard recurring/MIT (no static fallback).
    network_transaction_link_id: smithy.api#String
    /// You can specify up to 50 keys, with key names up to 40 characters long and values up to 500 characters long. Metadata is useful for storing additional, structured information on an object.
    metadata: smithy.api#Document
    /// Denotes the action(approve or reject) taken by merchant in case of manual review. Manual review can occur when the transaction is marked as risky by the frm_processor, payment processor or when there is underpayment/over payment incase of crypto payment
    merchant_decision: smithy.api#String
    /// Contains whole connector response
    whole_connector_response: smithy.api#String
    /// Contains card network response details (e.g., Visa/Mastercard advice codes).
    network_details: NetworkDetails
    /// This is an identifier for the merchant account. This is inferred from the API key provided during the request
    @required
    merchant_id: smithy.api#String
    /// Error code received from the issuer in case of failed payments
    issuer_error_code: smithy.api#String
    /// A connector-specific identifier representing the stored payment instrument
    sender_payment_instrument_id: smithy.api#String
    /// Total number of attempts associated with this payment
    @required
    attempt_count: smithy.api#Integer
    /// error message unified across the connectors is received here if there was an error while calling connector
    unified_message: smithy.api#String
    /// Date time at which payment was updated
    updated: smithy.api#String
    /// Fee information to be charged on the payment being collected
    split_payments: ConnectorChargeResponseData
    /// You can specify up to 50 keys, with key names up to 40 characters long and values up to 500 characters long. FRM Metadata is useful for storing additional, structured information on an object related to FRM.
    frm_metadata: smithy.api#Document
    /// Indicates who initiated the payment in platform-connected setups. - Some(Platform): Platform merchant initiated payment on behalf of connected merchant - Some(Connected): Connected merchant initiated payment directly in a platform setup - None: Standard merchant flow, JWT/Admin initiator, or insufficient information
    initiator: Initiator
    /// reference(Identifier) to the payment at connector side
    reference_id: smithy.api#String
    /// List of incremental authorizations happened to the payment
    incremental_authorizations: IncrementalAuthorizationResponseList
    /// List of attempts that happened on this intent
    attempts: PaymentAttemptResponseList
    /// A label identifying the specific merchant connector account (MCA) used for this payment. This often combines the connector name, business country, and a custom label (e.g., "stripe_US_primary").
    connector_label: smithy.api#String
    /// The name of the payment connector (e.g., 'stripe', 'adyen') that processed or is processing this payment.
    connector: smithy.api#String
    /// An array of refund objects associated with this payment. Empty or null if no refunds have been processed.
    refunds: RefundResponseList
    /// Optional boolean value to extent authorization period of this payment  capture method must be manual or manual_multiple
    request_extended_authorization: smithy.api#Boolean
    /// Allow partial authorization for this payment
    enable_partial_authorization: smithy.api#Boolean
    /// Method through which card was discovered
    card_discovery: CardDiscovery
    /// Specifies the category of a Merchant Initiated Transaction (MIT). In the case of MIT, `mit_category` tells what kind of MIT is being processed. In the case of CIT, it tells the future intended MIT type.
    mit_category: MitCategory
    /// Three-letter ISO currency code (e.g., USD, EUR) for the payment amount.
    @required
    currency: Currency
    /// Information about the product , quantity and amount for connectors. (e.g. Klarna)
    order_details: OrderDetailsWithAmountList
    customer: CustomerDetailsResponse
    /// A unique identifier for a customer provided by the connector.
    connector_customer_id: smithy.api#String
    /// The identifier for the customer object. If not provided the customer ID will be autogenerated. This field will be deprecated soon. Please refer to `customer.id`
    customer_id: smithy.api#String
    /// The billing address for the payment
    billing: Address
    /// Provide a reference to a stored payment method
    payment_token: smithy.api#String
    /// A human-readable error message from the last failed payment attempt associated with this payment intent.
    error_message: smithy.api#String
    /// error code unified across the connectors is received here if there was an error while calling connector
    unified_code: smithy.api#String
    /// Frm message contains information about the frm response
    frm_message: FrmMessage
    /// Timestamp indicating when this payment intent was last modified, in ISO 8601 format.
    modified_at: smithy.api#String
    /// Additional data related to some connectors
    connector_metadata: ConnectorMetadata
    /// Total number of authorizations happened in an incremental_authorization payment
    authorization_count: smithy.api#Integer
    /// Date Time for expiry of the payment
    expires_on: smithy.api#String
    /// description: The customer's name This field will be deprecated soon. Please refer to `customer.name` object
    name: smithy.api#String
    /// If true, incremental authorization can be performed on this payment, in case the funds authorized initially fall short.
    incremental_authorization_allowed: smithy.api#Boolean
    /// Indicates how the payment was initiated (e.g., ecommerce, mail, or telephone).
    payment_channel: PaymentChannel
    /// order tax amount calculated by tax connectors
    order_tax_amount: smithy.api#Long
    /// If the payment requires further action from the customer (e.g., 3DS authentication, redirect to a bank page), this object will contain the necessary information for the client to proceed. Null if no further action is needed from the customer at this stage.
    next_action: NextActionData
    /// Indicates if 3ds challenge is forced
    force_3ds_challenge: smithy.api#Boolean
    /// Timestamp indicating when this payment intent was created, in ISO 8601 format.
    created: smithy.api#String
    /// Indicates if the redirection has to open in the iframe
    is_iframe_redirection_enabled: smithy.api#Boolean
    /// The customer's phone number This field will be deprecated soon. Please refer to `customer.phone` object
    phone: smithy.api#String
    /// A timestamp (ISO 8601 code) that determines when the payment should be captured. Providing this field will automatically set `capture` to true
    capture_on: smithy.api#String
    /// description: The customer's email address This field will be deprecated soon. Please refer to `customer.email` object
    email: smithy.api#String
    /// The payment method information provided for making a payment
    payment_method_data: PaymentMethodDataResponseWithBilling
    /// The payment amount. Amount for the payment in lowest denomination of the currency. (i.e) in cents for USD denomination, in paisa for INR denomination etc.,
    @required
    amount: smithy.api#Long
    /// The URL to redirect after the completion of the operation
    return_url: smithy.api#String
    /// A secret token unique to this payment intent. It is primarily used by client-side applications (e.g., Hyperswitch SDKs) to authenticate actions like confirming the payment or handling next actions. This secret should be handled carefully and not exposed publicly beyond its intended client-side use.
    client_secret: smithy.api#String
    /// The total amount (in minor units) that has been captured for this payment. For `fauxpay` sandbox connector, this might reflect the authorized amount if `status` is `succeeded` even if `capture_method` was `manual`.
    amount_received: smithy.api#Long
    /// Provided mandate information for creating a mandate
    mandate_data: MandateData
    /// The transaction authentication can be set to undergo payer authentication. By default, the authentication will be marked as NO_THREE_DS, as the 3DS method helps with more robust payer authentication
    authentication_type: AuthenticationType
    /// Provides information about a card payment that customers see on their statements. Concatenated with the prefix (shortened descriptor) or statement descriptor that’s set on the account to form the complete statement descriptor. Maximum 255 characters for the concatenated descriptor. To be deprecated soon, use billing_descriptor instead.
    statement_descriptor_suffix: smithy.api#String
    /// Returns additional provider-specific metadata for certain connectors
    connector_response_metadata: ConnectorMetadataResponse
    /// The browser information used for this payment
    browser_info: BrowserInformation
    /// flag that indicates if extended authorization is applied on this payment or not
    extended_authorization_applied: smithy.api#Boolean
    /// List of captures done on latest attempt
    captures: CaptureResponseList
    /// This is the instruction for capture/ debit the money from the users' card. On the other hand authorization refers to blocking the amount on the users' payment method.
    capture_method: CaptureMethod
    /// Payment Method Status, refers to the status of the payment method used for this payment. Refer `payment_method_tokenization_details` for detailed view of payment method tokenization
    payment_method_status: PaymentMethodStatus
    /// An arbitrary string providing a description for the payment, often useful for display or internal record-keeping.
    description: smithy.api#String
    /// The amount (in minor units) that can still be captured for this payment. This is relevant when `capture_method` is `manual`. Once fully captured, or if `capture_method` is `automatic` and payment succeeded, this will be 0.
    @required
    amount_capturable: smithy.api#Long
    @required
    status: IntentStatus
    /// The payment method that is to be used
    payment_method: PaymentMethod
    /// Boolean flag indicating whether this payment method is stored and has been previously used for payments
    is_stored_credential: smithy.api#Boolean
    /// Error message received from the issuer in case of failed payments
    issuer_error_message: smithy.api#String
    /// The business profile that is associated with this payment
    profile_id: smithy.api#String
    /// The network transaction ID is a unique identifier for the transaction as recognized by the payment network (e.g., Visa, Mastercard), this ID can be used to reference it for future transactions or recurring payments. Refer `payment_method_tokenization_details` for detailed view of payment method tokenization
    network_transaction_id: smithy.api#String
    /// Payment Fingerprint, to identify a particular card. It is a 20 character long alphanumeric code.
    fingerprint: smithy.api#String
    /// Merchant's identifier for the payment/invoice. This will be sent to the connector if the connector provides support to accept multiple reference ids. In case the connector supports only one reference id, Hyperswitch's Payment ID will be sent as reference.
    merchant_order_reference_id: smithy.api#String
    /// Indicates if 3ds challenge is triggered
    force_3ds_challenge_trigger: smithy.api#Boolean
}

union MobilePaymentResponse {
    direct_carrier_billing: DirectCarrierBillingNestedType
}

enum ThreeDsMethodKey {
    @enumValue("threeDSMethodData")
    ThreeDsMethodData
    @enumValue("JWT")
    JWT
}

structure RevolutPayData {
}

structure MandatePaymentNestedType {
}

structure PaymentProcessingDetailsAt {
    field_0: PaymentProcessingDetails
    /// Discriminator field for the tagged enum
    @required
    payment_processing_details_at: PaymentProcessingDetailsAtEnumVariants
}

enum GooglePayCardFundingSource {
    PREPAID
    UNKNOWN
    CREDIT
    DEBIT
}

structure SecretInfoToInitiateSdk {
    @required
    display: smithy.api#String
    payment: smithy.api#String
}

structure UpiQrData {
    /// The UPI source type (Credit Card, Credit Line, Account, or Credit Card + Credit Line)
    upi_source: UpiSource
}

structure RefundUpdateRequest {
    /// You can specify up to 50 keys, with key names up to 40 characters long and values up to 500 characters long. Metadata is useful for storing additional, structured information on an object.
    metadata: smithy.api#Document
    /// An arbitrary string attached to the object. Often useful for displaying to users and your customer support executive
    reason: smithy.api#String
}

structure BhnCardNetworkNestedType {
}

structure RefundListResponse {
    /// The total number of refunds in the list
    @required
    total_count: smithy.api#Long
    /// The number of refunds included in the list
    @required
    count: smithy.api#Long
    /// The List of refund response object
    @required
    data: RefundResponseList
}

structure PaymentAttemptResponse {
    /// The transaction authentication can be set to undergo payer authentication. By default, the authentication will be marked as NO_THREE_DS
    authentication_type: AuthenticationType
    /// The currency of the amount of the payment attempt
    currency: Currency
    /// If a tokenized (saved) payment method was used for this attempt, this field contains the payment token representing that payment method.
    payment_token: smithy.api#String
    /// The status of the attempt
    @required
    status: AttemptStatus
    /// (This field is not live yet)Error code unified across the connectors is received here if there was an error while calling connector
    unified_code: smithy.api#String
    /// If this payment attempt is associated with a mandate (e.g., for a recurring or subsequent payment), this field will contain the ID of that mandate.
    mandate_id: smithy.api#String
    /// A unique identifier for a payment provided by the connector
    connector_transaction_id: smithy.api#String
    /// Complete error details containing unified, issuer, and connector-level error information
    error_details: PaymentErrorDetails
    /// Value passed in X-CLIENT-SOURCE header during payments confirm request by the client
    client_source: smithy.api#String
    /// The payment attempt amount. Amount for the payment in lowest denomination of the currency. (i.e) in cents for USD denomination, in paisa for INR denomination etc.,
    @required
    amount: smithy.api#Long
    /// Payment Experience for the current payment
    payment_experience: PaymentExperience
    /// A human-readable message from the connector explaining the error, if one occurred during this payment attempt.
    error_message: smithy.api#String
    /// The name of the payment connector (e.g., 'stripe', 'adyen') used for this attempt.
    connector: smithy.api#String
    /// The connector's own reference or transaction ID for this specific payment attempt. Useful for reconciliation with the connector.
    reference_id: smithy.api#String
    /// The payment method that is to be used
    payment_method: PaymentMethod
    /// Payment Method Type
    payment_method_type: PaymentMethodType
    /// (This field is not live yet)Error message unified across the connectors is received here if there was an error while calling connector
    unified_message: smithy.api#String
    /// If the payment was cancelled the reason will be provided here
    cancellation_reason: smithy.api#String
    /// A unique identifier for this specific payment attempt.
    @required
    attempt_id: smithy.api#String
    /// This is the instruction for capture/ debit the money from the users' card. On the other hand authorization refers to blocking the amount on the users' payment method.
    capture_method: CaptureMethod
    /// Additional data related to some connectors
    connector_metadata: smithy.api#Document
    /// Value passed in X-CLIENT-VERSION header during payments confirm request by the client
    client_version: smithy.api#String
    /// Time at which the payment attempt was last modified
    @required
    modified_at: smithy.api#String
    /// The payment attempt tax_amount.
    order_tax_amount: smithy.api#Long
    /// Time at which the payment attempt was created
    @required
    created_at: smithy.api#String
    /// The error code returned by the connector if this payment attempt failed. This code is specific to the connector.
    error_code: smithy.api#String
}

structure CardToken {
    /// The card holder's name
    card_holder_name: smithy.api#String
    /// Token referencing a CVC vaulted in the hyperswitch (self-hosted) vault. Used by the self-hosted default-vault repeat-customer flow, where the card is referenced by the top-level `payment_token` and the freshly-tokenized CVC arrives as this token; the server resolves it to the raw CVC. Not used by the external vault proxy flow.
    card_cvc_token: smithy.api#String
    /// The CVC number for the card
    card_cvc: smithy.api#String
}

structure CimbVaBankTransferNestedType {
    /// The billing details for BniVa Bank Transfer
    billing_details: DokuBillingDetails
}

union MobilePaymentData {
    direct_carrier_billing: DirectCarrierBillingNestedType
}

structure KlarnaRedirectNestedType {
    /// The billing email
    billing_email: smithy.api#String
    billing_country: CountryAlpha2
}

structure AfterpayClearpayRedirectNestedType {
    /// The billing email
    billing_email: smithy.api#String
    /// The billing name
    billing_name: smithy.api#String
}

structure PaypalRedirection {
    /// paypal's email address
    email: smithy.api#String
}

structure MomoRedirection {
}

union BankTransferResponse {
    pix: PixBankTransferAdditionalData
    pix_emv: PixEmvNestedType
    pix_automatico_push: PixAutomaticoPushAdditionalData
    instant_bank_transfer_finland: InstantBankTransferFinlandNestedType
    bri_va: BriVaNestedType
    local_bank_transfer: LocalBankTransferAdditionalData
    pse: PseNestedType
    pix_qr: PixQrNestedType
    pix_automatico_qr: PixAutomaticoQrNestedType
    cimb_va: CimbVaNestedType
    bacs: BacsNestedType
    permata: PermataNestedType
    instant_bank_transfer_poland: InstantBankTransferPolandNestedType
    sepa: SepaBankTransferPaymentAdditionalData
    instant_bank_transfer: InstantBankTransferNestedType
    multibanco: MultibancoNestedType
    bni_va: BniVaNestedType
    ach: AchNestedType
    mandiri_va: MandiriVaNestedType
    indonesian_bank_transfer: IndonesianBankTransferNestedType
    bca: BcaNestedType
    danamon_va: DanamonVaNestedType
}

/// Hyperswitch supports SDK integration with Apple Pay and Google Pay wallets. For other wallets, we integrate with their respective connectors, redirecting the customer to the connector for wallet payments. As a result, we don’t receive any payment method data in the confirm call for payments made through other wallets.
union WalletResponseData {
    apple_pay: WalletAdditionalDataForCard
    google_pay: WalletAdditionalDataForCard
    samsung_pay: WalletAdditionalDataForCard
}

structure AdyenConnectorMetadata {
    @required
    testing: AdyenTestingData
}

structure ApplepayConnectorMetadataRequest {
    session_token_data: SessionTokenInfo
}

structure TouchNGoRedirection {
}

enum SamsungPayProtocolType {
    PROTOCOL3DS
}

structure ApplePayRecurringDetails {
    /// A description of the recurring payment that Apple Pay displays to the user in the payment sheet
    @required
    payment_description: smithy.api#String
    /// A URL to a web page where the user can update or delete the payment method for the recurring payment
    @required
    management_url: smithy.api#String
    /// The regular billing cycle for the recurring payment, including start and end dates, an interval, and an interval count
    @required
    regular_billing: ApplePayRegularBillingDetails
    /// A localized billing agreement that the payment sheet displays to the user before the user authorizes the payment
    billing_agreement: smithy.api#String
}

structure PseNestedType {
}

/// Data for PixAutomaticoPush Payment Method Type CIT (Customer Initiated Transaction) - used during mandate setup
structure PixAutomaticoPushData {
    /// Enable retry policy for failed payments (maps to PERMITE_3R_7D if true)
    retry_policy: smithy.api#Boolean
    /// Mandate details for the recurring charge
    mandate_details: SantanderMandateDetails
}

structure OrderDetailsWithAmount {
    /// the amount per quantity of product
    @required
    amount: smithy.api#Long
    /// Code describing a commodity or a group of commodities pertaining to goods classification.
    commodity_code: smithy.api#String
    /// The tax code for the product
    product_tax_code: smithy.api#String
    /// tax rate applicable to the product
    tax_rate: smithy.api#Double
    /// Stock Keeping Unit (SKU) or the item identifier for this item.
    sku: smithy.api#String
    /// Total amount for the item.
    total_amount: smithy.api#Long
    /// Sub category of the product that is being purchased
    sub_category: smithy.api#String
    /// Discount name applied to this item.
    discount_name: smithy.api#String
    /// Name of the product that is being purchased
    @required
    product_name: smithy.api#String
    /// The quantity of the product to be purchased
    @required
    quantity: smithy.api#Integer
    /// Discount percentage applied to this item.
    discount_percentage: PercentageValue
    requires_shipping: smithy.api#Boolean
    /// Category of the product that is being purchased
    category: smithy.api#String
    /// Universal Product Code for the item.
    upc: smithy.api#String
    /// Unit of measure used for the item quantity.
    unit_of_measure: smithy.api#String
    /// Brand of the product that is being purchased
    brand: smithy.api#String
    /// Discount amount applied to this item.
    unit_discount_amount: smithy.api#Long
    /// Discount type applied to this item.
    discount_type: smithy.api#String
    /// Description for the item
    description: smithy.api#String
    /// total tax amount applicable to the product
    total_tax_amount: smithy.api#Long
    /// ID of the product that is being purchased
    product_id: smithy.api#String
    /// The image URL of the product
    product_img_link: smithy.api#String
}

/// frm message is an object sent inside the payments response...when frm is invoked, its value is Some(...), else its None
structure FrmMessage {
    @required
    frm_name: smithy.api#String
    frm_transaction_id: smithy.api#String
    frm_transaction_type: smithy.api#String
    frm_reason: smithy.api#Document
    frm_score: smithy.api#Integer
    frm_error: smithy.api#String
    frm_status: smithy.api#String
}

/// Represents the rules for applying discounts to a payment, such as a percentage discount or a fixed amount discount, along with any applicable grace periods.
structure InterestDetail {
    /// Percentage of IOF (Financial Operations Tax). Pattern: \d{3}$\.\d{5} Only provided if the agreement is "Cobra IOF na Barra ou Cadastro"
    iof_percentage: smithy.api#String
    /// Percentage of Juros (Interest). Pattern: ^[0-9]{1,3}\.[0-9]{2}$
    interest_percentage: smithy.api#String
}

structure AchTransfer {
    @required
    account_number: smithy.api#String
    @required
    routing_number: smithy.api#String
    @required
    bank_name: smithy.api#String
    @required
    swift_code: smithy.api#String
}

structure IndonesianBankTransferNestedType {
    bank_name: BankNames
}

structure PixAutomaticoPushAdditionalData {
    /// Account number for Pix Automatico Push payment method
    account_number: smithy.api#String
    /// Branch code for Pix Automatico Push payment method
    branch_code: smithy.api#String
    /// Bank identifier for Pix Automatico Push payment method
    bank_identifier: smithy.api#String
}

union PaymentMethodDataResponseWithBilling {
    card: CardResponse
    bank_redirect: BankRedirectResponse
    gift_card: GiftCardResponse
    bank_debit: BankDebitResponse
    crypto: CryptoResponse
    card_redirect: CardRedirectResponse
    voucher: VoucherResponse
    card_token: CardTokenResponse
    real_time_payment: RealTimePaymentDataResponse
    mobile_payment: MobilePaymentResponse
    bank_transfer: BankTransferResponse
    upi: UpiResponse
    reward: RewardNestedType
    wallet: WalletResponse
    open_banking: OpenBankingResponse
    pay_later: PaylaterResponse
    mandate_payment: MandatePaymentNestedType
}

/// Represents the end-recipient of a payout or fund transfer.
structure BeneficiaryDetails {
    /// The category of identification provided (e.g., Passport, National ID, CPF).
    document_type: DocumentKind
    /// The customer's unique identification number (e.g., Tax ID, SSN, Passport Number). Used by processors to verify the identity of the recipient and prevent fraud. Length of the document number depends upon the document_type. For CPF/CNPJ it is typically 11/14 digits long.
    document_number: smithy.api#String
    /// The full legal name of the individual or entity receiving the funds.
    name: smithy.api#String
}

structure SofortNestedType {
    /// The billing details for bank redirection
    billing_details: BankRedirectBilling
    /// The country for bank payment
    country: CountryAlpha2
    /// The preferred language
    preferred_language: smithy.api#String
}

/// The customer details
structure CustomerRequest {
    /// You can specify up to 50 keys, with key names up to 40 characters long and values up to 500 characters long. Metadata is useful for storing additional, structured information on an object.
    metadata: smithy.api#Document
    /// The customer's email address
    email: smithy.api#String
    /// The identifier for the customer object. If not provided the customer ID will be autogenerated.
    customer_id: smithy.api#String
    /// The customer's phone number
    phone: smithy.api#String
    /// The country code for the customer phone number
    phone_country_code: smithy.api#String
    /// Customer’s country-specific identification number and type used for regulatory or tax purposes
    document_details: CustomerDocumentDetails
    /// An arbitrary string that you can attach to a customer object.
    description: smithy.api#String
    /// The customer's name
    name: smithy.api#String
    /// Customer's tax registration ID
    tax_registration_id: smithy.api#String
    /// The address for the customer
    address: AddressDetails
}

enum ApplePayPaymentTiming {
    /// A value that specifies that the payment occurs on a regular basis
    recurring
    /// A value that specifies that the payment occurs when the transaction is complete
    immediate
}

/// Processor payment token for MIT payments where payment_method_data is not available
structure ProcessorPaymentToken {
    @required
    processor_payment_token: smithy.api#String
    merchant_connector_id: smithy.api#String
}

structure ThreeDsData {
    /// Message Version
    message_version: smithy.api#String
    /// ThreeDS authorize url - to complete the payment authorization after authentication
    @required
    three_ds_authorize_url: smithy.api#String
    /// The card network for the card
    card_network: CardNetwork
    /// ThreeDS method details
    @required
    three_ds_method_details: ThreeDsMethodData
    /// Directory Server ID
    directory_server_id: smithy.api#String
    /// ThreeDS authentication url - to initiate authentication
    @required
    three_ds_authentication_url: smithy.api#String
    /// Poll config for a connector
    @required
    poll_config: PollConfigResponse
    /// Preferred 3ds Connector
    three_ds_connector: smithy.api#String
}

/// Fee information to be charged on the payment being collected via Stripe
structure StripeChargeResponseData {
    /// The Stripe account ID that these funds are intended for
    on_behalf_of: smithy.api#String
    /// Identifier for the reseller's account where the funds were transferred
    @required
    transfer_account_id: smithy.api#String
    /// Type of charge (connector specific)
    @required
    charge_type: PaymentChargeType
    /// Identifier for charge created for the payment
    charge_id: smithy.api#String
    /// Platform fees collected on the payment
    application_fees: smithy.api#Long
}

structure DanaRedirectNestedType {
}

/// Represents the connector-specific metadata for Santander payments, including boleto data and related rules.
structure SantanderConnectorMetadataData {
    /// Boleto-specific data and rules for Santander payments
    boleto: SantanderBoletoData
}

structure NetworkTransactionIdAndNetworkTokenDetails {
    /// The network transaction ID provided by the card network during a Customer Initiated Transaction (CIT) when `setup_future_usage` is set to `off_session`.
    @required
    network_transaction_id: smithy.api#String
    /// The country in which the card was issued
    card_issuing_country: smithy.api#String
    /// The name of the issuer of card
    card_issuer: smithy.api#String
    /// The bank code of the bank that issued the card
    bank_code: smithy.api#String
    /// The token's expiry month
    @required
    token_exp_month: smithy.api#String
    /// The card holder's nick name
    nick_name: smithy.api#String
    /// The card holder's name
    card_holder_name: smithy.api#String
    /// The ECI(Electronic Commerce Indicator) value for this authentication.
    eci: smithy.api#String
    /// The type of the card such as Credit, Debit
    card_type: smithy.api#String
    /// The Network Token
    @required
    network_token: smithy.api#String
    /// The Mastercard Transaction Link Identifier (TLID) provided by the card network during a CIT (Customer Initiated Transaction), when `setup_future_usage` is set to `off_session`.
    transaction_link_id: smithy.api#String
    /// The card network for the card
    card_network: CardNetwork
    /// The token's expiry year
    @required
    token_exp_year: smithy.api#String
}

structure SepaBankTransferNestedType {
    /// The billing details for SEPA
    billing_details: SepaAndBacsBillingDetails
    /// The two-letter ISO country code for SEPA and BACS
    country: CountryAlpha2
}

structure BriVaNestedType {
}

structure KnetNestedType {
}

/// Card token data carried by the external vault proxy `vault_card_token_data` variant. Its `card_cvc` is a vault token detokenized on the wire by the external vault (e.g. VGS). Kept as a distinct type from [`CardToken`] so the self-hosted `card_cvc_token` field does not leak into the proxy contract.
structure VaultCardToken {
    /// The CVC number for the card (a vault token for the external vault proxy flow)
    card_cvc: smithy.api#String
    /// The card holder's name
    card_holder_name: smithy.api#String
}

union SamsungPayWalletCredentials {
    samsung_pay_wallet_data_for_web: SamsungPayWebWalletData
    samsung_pay_wallet_data_for_app: SamsungPayAppWalletData
}

union GiftCardResponse {
    bhn_card_network: BhnCardNetworkNestedType
    givex: smithy.api#String
    pay_safe_card: PaySafeCardNestedType
}

union OpenBankingData {
    open_banking_p_i_s: OpenBankingPISNestedType
}

/// ephemeral_key for the customer_id mentioned
structure EphemeralKeyCreateResponse {
    /// customer_id to which this ephemeral key belongs to
    @required
    customer_id: smithy.api#String
    /// time at which this ephemeral key was created
    @required
    created_at: smithy.api#Long
    /// time at which this ephemeral key would expire
    @required
    expires: smithy.api#Long
    /// ephemeral key
    @required
    secret: smithy.api#String
}

union OpenBankingResponse {
    open_banking_p_i_s: OpenBankingPISNestedType
}

/// Fee information for Split Payments to be charged on the payment being collected for Stripe
structure StripeSplitPaymentRequest {
    /// Platform fees to be collected on the payment
    application_fees: smithy.api#Long
    /// Stripe's charge type
    @required
    charge_type: PaymentChargeType
    /// Identifier for the reseller's account where the funds were transferred
    @required
    transfer_account_id: smithy.api#String
    /// The Stripe account ID that these funds are intended for
    on_behalf_of: smithy.api#String
}

/// Indicates the method by which a card is discovered during a payment
enum CardDiscovery {
    manual
    saved_card
    click_to_pay
}

structure PixBankTransferAdditionalData {
    /// The expiration date and time for the Pix QR code in ISO 8601 format
    expiry_date: smithy.api#String
    /// Partially masked CPF - CPF is a Brazilian tax identification number
    cpf: smithy.api#String
    /// Partially masked unique key for pix transfer
    pix_key: smithy.api#String
    /// Partially masked source bank account number
    source_bank_account_id: smithy.api#String
    /// Partially masked CNPJ - CNPJ is a Brazilian company tax identification number
    cnpj: smithy.api#String
    /// Partially masked destination bank account number _Deprecated: Will be removed in next stable release._
    destination_bank_account_id: smithy.api#String
}

structure VoucherNextStepData {
    /// Url to payment instruction page
    instructions_url: smithy.api#String
    /// Machine-readable numeric code used to generate the barcode representation.
    barcode: smithy.api#String
    /// The url for Pix Qr code given by the connector associated with the voucher
    qr_code_url: smithy.api#String
    /// The raw QR code data (EMV copy and paste) used for Brazilian payment methods like Pix
    raw_qr_data: smithy.api#String
    /// Human-readable numeric version of the barcode.
    digitable_line: smithy.api#String
    /// Voucher expiry date and time
    expiry_date: smithy.api#String
    /// Voucher expiry date and time
    expires_at: smithy.api#Long
    /// Voucher entry date
    entry_date: smithy.api#String
    /// Url to download the payment instruction
    download_url: smithy.api#String
    /// Reference number required for the transaction
    @required
    reference: smithy.api#String
}

structure WeChatPayQr {
}

structure DuitNowNestedType {
}

enum AmazonPayPaymentIntent {
    /// Authorize funds immediately and capture at a later time
    Authorize
    /// Authorize and capture funds immediately
    AuthorizeWithCapture
    /// Create a Charge Permission to authorize and capture funds at a later time
    Confirm
}

structure BecsBankDebitAdditionalData {
    /// Bank-State-Branch (bsb) number
    @required
    bsb_number: smithy.api#String
    /// Bank account's owner name
    bank_account_holder_name: smithy.api#String
    /// Partially masked account number for Becs payment method
    @required
    account_number: smithy.api#String
}

structure KlarnaSessionTokenResponse {
    /// The identifier for the session
    @required
    session_id: smithy.api#String
    /// The session token for Klarna
    @required
    session_token: smithy.api#String
}

/// Represents the receiver details for Santander Pix Automatico recurring charges
structure SantanderPixAutomaticoReceiverDetails {
    /// Account type (tipoConta) - CORRENTE, POUPANCA, or PAGAMENTO
    account_type: AccountType
    /// Branch code (agencia) of the receiver's bank account
    branch_code: smithy.api#String
    /// Account number (conta) of the receiver
    account_number: smithy.api#String
}

/// Charge Information
union ConnectorChargeResponseData {
    /// StripeChargeResponseData
    stripe_split_payment: StripeChargeResponseData
    /// XenditChargeResponseData
    xendit_split_payment: XenditChargeResponseData
    /// AdyenChargeResponseData
    adyen_split_payment: AdyenSplitData
}

structure PaypalSessionTokenResponse {
    /// Name of the connector
    @required
    connector: smithy.api#String
    /// Transaction currency code
    currency: Currency
    /// The next action for the sdk (ex: calling confirm or sync call)
    @required
    sdk_next_action: SdkNextAction
    /// The session token for PayPal
    @required
    session_token: smithy.api#String
    /// PayPal capture method
    intent: PaypalCaptureMethod
}

union BankTransferData {
    mandiri_va_bank_transfer: MandiriVaBankTransferNestedType
    sepa_bank_transfer: SepaBankTransferNestedType
    ach_bank_transfer: AchBankTransferNestedType
    indonesian_bank_transfer: IndonesianBankTransferNestedType
    bca_bank_transfer: BcaBankTransferNestedType
    pix_qr: PixQrNestedType
    pix: PixNestedType
    pix_automatico_push: PixAutomaticoPushNestedType
    bni_va_bank_transfer: BniVaBankTransferNestedType
    multibanco_bank_transfer: MultibancoBankTransferNestedType
    cimb_va_bank_transfer: CimbVaBankTransferNestedType
    local_bank_transfer: LocalBankTransferNestedType
    instant_bank_transfer_finland: InstantBankTransferFinlandNestedType
    danamon_va_bank_transfer: DanamonVaBankTransferNestedType
    permata_bank_transfer: PermataBankTransferNestedType
    pix_emv: PixEmvNestedType
    instant_bank_transfer: InstantBankTransferNestedType
    pix_automatico_qr: PixAutomaticoQrNestedType
    bacs_bank_transfer: BacsBankTransferNestedType
    instant_bank_transfer_poland: InstantBankTransferPolandNestedType
    bri_va_bank_transfer: BriVaBankTransferNestedType
    pse: PseNestedType
}

/// Represents the overall status of a payment intent. The status transitions through various states depending on the payment method, confirmation, capture method, and any subsequent actions (like customer authentication or manual capture).
enum IntentStatus {
    /// The payment has been captured partially and the remaining amount is capturable
    partially_captured_and_capturable
    /// The payment has been captured partially. The remaining amount is cannot be captured.
    partially_captured
    /// The payment expired before it could be captured.
    expired
    /// The payment has been marked for manual review due to anomalous response from the connector. This can occur when a capture fails after the payment was initially marked as successful (e.g., Adyen CAPTURE_FAILED webhook after successful CAPTURE). The merchant can explicitly resolve this status via the API or a webhook from the connector can update the status
    review
    /// This payment has been cancelled post capture.
    cancelled_post_capture
    /// The payment has succeeded. Refunds and disputes can be initiated. Manual retries are not allowed to be performed.
    succeeded
    /// The payment has been authorized for a partial amount and requires capture
    partially_authorized_and_requires_capture
    /// This payment has been cancelled.
    cancelled
    /// The payment is waiting on some action from the customer.
    requires_customer_action
    /// The payment is waiting to be confirmed with the payment method by the customer.
    requires_payment_method
    /// The payment has been captured partially and the remaining amount can be authorized/capturable. The other amount is still being processed by the payment processor. The status update might happen through webhooks or polling with the connector.
    partially_captured_and_processing
    /// The payment has been authorized, and it waiting to be captured.
    requires_capture
    /// This payment is still being processed by the payment processor. The status update might happen through webhooks or polling with the connector.
    processing
    /// There has been a discrepancy between the amount/currency sent in the request and the amount/currency received by the processor
    conflicted
    /// The payment has failed. Refunds and disputes cannot be initiated. This payment can be retried manually with a new payment attempt.
    failed
    /// The payment is waiting on some action from the merchant This would be in case of manual fraud approval
    requires_merchant_action
    requires_confirmation
}

/// The status of the mandate, which indicates whether it can be used to initiate a payment.
enum MandateStatus {
    pending
    inactive
    revoked
    active
}

structure BraintreeData {
    /// Information about the merchant_account_id that merchant wants to specify at connector level.
    @required
    merchant_account_id: smithy.api#String
    /// Information about the merchant_config_currency that merchant wants to specify at connector level.
    @required
    merchant_config_currency: smithy.api#String
}

structure GcashRedirection {
}

union BankDebitResponse {
    sepa: SepaBankDebitAdditionalData
    eft_debit_order: EftDebitOrderAdditionalData
    bacs: BacsBankDebitAdditionalData
    becs: BecsBankDebitAdditionalData
    ach: AchBankDebitAdditionalData
}

structure RefundListRequest {
    /// Limit on the number of objects to return
    limit: smithy.api#Long
    /// The list of connectors to filter refunds list
    connector: StringList
    /// The amount to filter reufnds list. Amount takes two option fields start_amount and end_amount from which objects can be filtered as per required scenarios (less_than, greater_than, equal_to and range)
    amount_filter: AmountFilter
    /// The identifier for the payment
    payment_id: smithy.api#String
    /// The list of merchant connector ids to filter the refunds list for selected label
    merchant_connector_id: StringList
    /// The list of refund statuses to filter refunds list
    refund_status: RefundStatusList
    /// The identifier for business profile
    profile_id: smithy.api#String
    /// The identifier for the refund
    refund_id: smithy.api#String
    /// The list of currencies to filter refunds list
    currency: CurrencyList
    /// The starting point within a list of objects
    offset: smithy.api#Long
}

/// Specifies the category of a Merchant Initiated Transaction (MIT). In the case of MIT, `mit_category` tells what kind of MIT is being processed. In the case of CIT, it tells the future intended MIT type.
enum MitCategory {
    /// A fixed purchase amount split into multiple scheduled payments until the total is paid.
    installment
    /// A retried MIT after a previous transaction failed or was declined.
    resubmission
    /// Merchant-initiated transaction using stored credentials, but not tied to a fixed schedule
    unscheduled
    /// Merchant-initiated payments that happen at regular intervals (usually the same amount each time).
    recurring
}

structure AmazonPayDeliveryPrice {
    /// Transaction currency code in ISO 4217 format
    @required
    currency_code: Currency
    /// Transaction amount in MinorUnit
    @required
    amount: smithy.api#Long
    /// Transaction amount in StringMajorUnit
    @required
    display_amount: smithy.api#String
}

/// Defines the type of protest for non-payment, such as whether the count is based on calendar days, business days, or if the protest logic is determined by a pre-signed contract with the bank.
enum ProtestType {
    /// No legal protest will be initiated
    disabled
    /// Protest logic is handled based on the merchant's pre-signed contract/agreement with the bank
    contract_default
    /// Count is based on business days (Mon-Fri, excluding bank holidays)
    business_days
    /// Count is based on calendar days (Standard)
    calendar_days
}

union CardRedirectResponse {
    knet: KnetNestedType
    momo_atm: MomoAtmNestedType
    benefit: BenefitNestedType
    card_redirect: CardRedirectNestedType
}

union PaymentMethodData {
    voucher: VoucherData
    pay_later: PayLaterData
    bank_debit: BankDebitData
    card: Card
    card_redirect: CardRedirectData
    bank_redirect: BankRedirectData
    reward: smithy.api#Unit
    upi: UpiData
    mobile_payment: MobilePaymentData
    card_with_no_c_v_c: CardWithNoCVC
    real_time_payment: RealTimePaymentData
    mandate_payment: smithy.api#Unit
    card_token: CardToken
    open_banking: OpenBankingData
    crypto: CryptoData
    gift_card: GiftCardData
    bank_transfer: BankTransferData
}

/// The payment method information provided for making a payment
union PaymentMethodDataRequest {
    real_time_payment: RealTimePaymentData
    card_token: CardToken
    crypto: CryptoData
    bank_transfer: BankTransferData
    bank_debit: BankDebitData
    open_banking: OpenBankingData
    pay_later: PayLaterData
    card_redirect: CardRedirectData
    bank_redirect: BankRedirectData
    mobile_payment: MobilePaymentData
    upi: UpiData
    mandate_payment: smithy.api#Unit
    card_with_no_c_v_c: CardWithNoCVC
    gift_card: GiftCardData
    voucher: VoucherData
    reward: smithy.api#Unit
    card: Card
}

/// Merchant connector details used to make payments.
structure MerchantConnectorDetailsWrap {
    /// Merchant connector details type type. Base64 Encode the credentials and send it in  this type and send as a string.
    encoded_data: MerchantConnectorDetails
    /// Creds Identifier is to uniquely identify the credentials. Do not send any sensitive info, like encoded_data in this field. And do not send the string "null".
    @required
    creds_identifier: smithy.api#String
}

structure DanamonVaNestedType {
}

structure AlmaRedirectNestedType {
}

structure ThreeDsMethodDataAcsThreeDsMethodDataData {
    /// Indicates whether to wait for Post message after 3DS method data submission
    @required
    consume_post_message_for_three_ds_method_completion: smithy.api#Boolean
    /// Whether ThreeDS method data submission is required
    @required
    three_ds_method_data_submission: smithy.api#Boolean
    /// ThreeDS method data
    three_ds_method_data: smithy.api#String
    /// ThreeDS method url
    three_ds_method_url: smithy.api#String
    /// Three DS Method Key
    three_ds_method_key: ThreeDsMethodKey
}

structure BacsBankDebitAdditionalData {
    /// Partially masked sort code for Bacs payment method
    @required
    sort_code: smithy.api#String
    /// Partially masked account number for Bacs payment method
    @required
    account_number: smithy.api#String
    /// Bank account's owner name
    bank_account_holder_name: smithy.api#String
}

/// Indicates the sub type of payment method. Eg: 'google_pay' & 'apple_pay' for wallets.
enum PaymentMethodType {
    paypal
    ali_pay
    alfamart
    pix_emv
    upi_collect
    affirm
    crypto_currency
    instant_bank_transfer
    instant_bank_transfer_finland
    bluecode
    credit
    red_pagos
    efecty
    open_banking_uk
    oxxo
    bizum
    knet
    permata_bank_transfer
    afterpay_clearpay
    eft
    breadpay
    local_bank_redirect
    prompt_pay
    we_chat_pay
    givex
    direct_carrier_billing
    payjustnow
    touch_n_go
    walley
    family_mart
    upi_qr
    local_bank_transfer
    mifinity
    trustly
    ideal
    benefit
    bhn_card_network
    danamon_va
    online_banking_thailand
    red_compra
    seicomart
    paze
    giropay
    pix_automatico_qr
    venmo
    atome
    online_banking_fpx
    sepa
    mini_stop
    mandiri_va
    revolut_pay
    bancontact_card
    fps
    gcash
    indomaret
    flexiti
    twint
    bri_va
    multibanco
    becs
    qris
    viet_qr
    mobile_pay
    momo
    apple_pay
    pay_safe_card
    ach
    pse
    pay_easy
    open_banking
    przelewy24
    skrill
    pago_efectivo
    eft_debit_order
    paysera
    pay_bright
    pix
    pix_automatico_push
    bca_bank_transfer
    vipps
    @enumValue("classic")
    classic_reward
    network_token
    lawson
    instant_bank_transfer_poland
    cimb_va
    swish
    klarna
    pix_qr
    bacs
    sepa_guarenteed_debit
    momo_atm
    pix_key
    blik
    boleto
    go_pay
    sepa_bank_transfer
    cashapp
    online_banking_finland
    upi_intent
    online_banking_poland
    seven_eleven
    alma
    interac
    duit_now
    bni_va
    evoucher
    indonesian_bank_transfer
    google_pay
    amazon_pay
    ali_pay_hk
    sofort
    @enumValue("open_banking_pis")
    open_banking_p_i_s
    debit
    dana
    online_banking_czech_republic
    online_banking_slovakia
    samsung_pay
    mb_way
    card_redirect
    eps
    kakao_pay
}

structure BcaNestedType {
}

/// The identifier for the customer object. If not provided the customer ID will be autogenerated.
structure CustomerUpdateRequest {
    /// The address for the customer
    address: AddressDetails
    /// You can specify up to 50 keys, with key names up to 40 characters long and values up to 500 characters long. Metadata is useful for storing additional, structured information on an object.
    metadata: smithy.api#Document
    /// The customer's name
    name: smithy.api#String
    /// The country code for the customer phone number
    phone_country_code: smithy.api#String
    /// Customer’s country-specific identification number and type used for regulatory or tax purposes
    document_details: CustomerDocumentDetails
    /// The customer's email address
    email: smithy.api#String
    /// An arbitrary string that you can attach to a customer object.
    description: smithy.api#String
    /// The customer's phone number
    phone: smithy.api#String
    /// Customer's tax registration ID
    tax_registration_id: smithy.api#String
}

structure AmountInfo {
    /// A value that indicates whether the line item(Ex: total, tax, discount, or grand total) is final or pending.
    @jsonName("type")
    total_type: smithy.api#String
    /// The total amount for the payment in majot unit string (Ex: 38.02)
    @required
    amount: smithy.api#String
    /// The label must be the name of the merchant.
    @required
    label: smithy.api#String
}

list RefundResponseList {
    member: RefundResponse
}

structure BriVaBankTransferNestedType {
    /// The billing details for BniVa Bank Transfer
    billing_details: DokuBillingDetails
}

/// This struct represents the decrypted Apple Pay payment data
structure ApplePayPredecryptData {
    /// The application expiration date (PAN expiry year)
    @required
    application_expiration_year: smithy.api#String
    /// The application expiration date (PAN expiry month)
    @required
    application_expiration_month: smithy.api#String
    /// The primary account number
    @required
    application_primary_account_number: smithy.api#String
    /// The payment data, which contains the cryptogram and ECI indicator
    @required
    payment_data: ApplePayCryptogramData
}

union BankRedirectData {
    sofort: SofortNestedType
    bizum: BizumNestedType
    eft: EftNestedType
    online_banking_finland: OnlineBankingFinlandNestedType
    trustly: TrustlyNestedType
    eps: EpsNestedType
    ideal: IdealNestedType
    online_banking_czech_republic: OnlineBankingCzechRepublicNestedType
    online_banking_slovakia: OnlineBankingSlovakiaNestedType
    online_banking_fpx: OnlineBankingFpxNestedType
    online_banking_thailand: OnlineBankingThailandNestedType
    local_bank_redirect: LocalBankRedirectNestedType
    przelewy24: Przelewy24NestedType
    bancontact_card: BancontactCardNestedType
    blik: BlikNestedType
    giropay: GiropayNestedType
    interac: InteracNestedType
    online_banking_poland: OnlineBankingPolandNestedType
    open_banking_uk: OpenBankingUkNestedType
}

union PaymentMethodDataResponse {
    card_token: CardTokenResponse
    bank_transfer: BankTransferResponse
    pay_later: PaylaterResponse
    card_redirect: CardRedirectResponse
    upi: UpiResponse
    reward: RewardNestedType
    card: CardResponse
    wallet: WalletResponse
    bank_redirect: BankRedirectResponse
    open_banking: OpenBankingResponse
    mandate_payment: MandatePaymentNestedType
    bank_debit: BankDebitResponse
    mobile_payment: MobilePaymentResponse
    gift_card: GiftCardResponse
    crypto: CryptoResponse
    real_time_payment: RealTimePaymentDataResponse
    voucher: VoucherResponse
}

structure GooglePaySessionResponse {
    /// The next action for the sdk (ex: calling confirm or sync call)
    @required
    sdk_next_action: SdkNextAction
    /// The transaction info Google Pay requires
    @required
    transaction_info: GpayTransactionInfo
    /// List of the allowed payment methods
    @required
    allowed_payment_methods: GpayAllowedPaymentMethodsList
    /// Is shipping address required
    @required
    shipping_address_required: smithy.api#Boolean
    /// Shipping address parameters
    @required
    shipping_address_parameters: GpayShippingAddressParameters
    /// The merchant info
    @required
    merchant_info: GpayMerchantInfo
    /// The name of the connector
    @required
    connector: smithy.api#String
    /// Identifier for the delayed session response
    @required
    delayed_session_token: smithy.api#Boolean
    /// Secrets for sdk display and payment
    secrets: SecretInfoToInitiateSdk
    /// Is email required
    @required
    email_required: smithy.api#Boolean
}

structure QrisNestedType {
}

structure PaymentsCancelRequest {
    /// If enabled, provides whole connector response
    all_keys_required: smithy.api#Boolean
    /// The reason for the payment cancel
    cancellation_reason: smithy.api#String
    /// Merchant connector details used to make payments.
    merchant_connector_details: MerchantConnectorDetailsWrap
}

/// The status of the attempt
enum AttemptStatus {
    integrity_failure
    router_declined
    authentication_successful
    capture_initiated
    authorization_failed
    pending
    auto_refunded
    started
    authorized
    partially_authorized
    void_initiated
    void_failed
    payment_method_awaited
    device_data_collection_pending
    capture_failed
    expired
    charged
    capture_review
    authorizing
    cod_initiated
    authentication_failed
    authentication_pending
    unresolved
    failure
    confirmation_awaited
    voided_post_charge
    partial_charged
    partial_charged_and_chargeable
    voided
}

structure AliPayQr {
}

enum PeachpaymentsCardOnFileTransactionType {
    /// Merchant-initiated charge where the merchant holds the card credentials.
    merchant_initiated_transaction
    /// One-off card payment without CVV.
    one_off
    /// Card without CVV where the customer is present — telephone sales where the customer reads the card, hospitality pre-auth, etc.
    customer_initiated_transaction
    /// Card without CVV where the customer is not present — backoffice recurring setup, merchant loading credentials on behalf of the customer.
    merchant_initiated_mandate
}

structure LocalBankTransferNestedType {
    bank_code: smithy.api#String
}

structure SdkNextAction {
    /// The type of next action
    @required
    next_action: NextActionCall
}

structure Address {
    /// Provide the address details
    address: AddressDetails
    email: smithy.api#String
    phone: PhoneDetails
}

/// To indicate whether to refund needs to be instant or scheduled
enum RefundType {
    scheduled
    instant
}

structure SantanderData {
}

enum ApplepayInitiative {
    ios
    web
}

/// Data for Santander Pix Automatico MIT (Merchant Initiated Transaction) - used during recurring charge creation
structure PixAutomaticoMitData {
    /// Receiver details for the recurring charge
    receiver_details: SantanderPixAutomaticoReceiverDetails
    /// Whether to automatically adjust the due date to the next business day if it falls on a non-business day. Maps to ajuste_dia_util in Santander API. Defaults to true if not provided.
    auto_adjust_date: smithy.api#Boolean
}

union CardRedirectData {
    knet: KnetNestedType
    benefit: BenefitNestedType
    card_redirect: CardRedirectNestedType
    momo_atm: MomoAtmNestedType
}

enum GpayBillingAddressFormat {
    MIN
    FULL
}

structure PromptPayNestedType {
}

structure ApplepayPaymentMethod {
    /// The network of the Apple pay payment method
    @required
    network: smithy.api#String
    /// The type of the payment method
    @jsonName("type")
    @required
    pm_type: smithy.api#String
    /// The name to be displayed on Apple Pay button
    @required
    display_name: smithy.api#String
}

/// This enum is used to represent the Apple Pay payment data, which can either be encrypted or decrypted.
union ApplePayPaymentData {
    /// This variant contains the decrypted Apple Pay payment data as a structured object.
    decrypted: ApplePayPredecryptData
    /// This variant contains the encrypted Apple Pay payment data as a string.
    encrypted: smithy.api#String
}

enum DecoupledAuthenticationType {
    frictionless
    challenge
}

list CurrencyList {
    member: Currency
}

structure BluecodeRedirectNestedType {
}

/// Indicates if 3DS method data was successfully completed or not
enum ThreeDsCompletionIndicator {
    /// 3DS method was not successful
    @enumValue("N")
    Failure
    /// 3DS method URL was unavailable
    @enumValue("U")
    NotAvailable
    /// 3DS method successfully completed
    @enumValue("Y")
    Success
}

structure SantanderMandateDetails {
    /// Fixed amount for each recurring charge in minor units (e.g., cents). If not provided, the mandate will allow variable amounts.
    fixed_recurring_amount: smithy.api#Long
    /// Minimum amount for each recurring charge in minor units (e.g., cents). If not provided, there will be no minimum limit on the amount.
    min_recurring_amount: smithy.api#Long
    /// Frequency of the recurring charges (e.g., weekly, monthly). If not provided, defaults to monthly.
    periodicity: SantanderMandatePeriodicity
}

structure SepaBankDebitNestedType {
    /// International bank account number (iban) for SEPA
    @required
    iban: smithy.api#String
    /// Billing details for bank debit
    billing_details: BankDebitBilling
    /// Owner name for bank debit
    bank_account_holder_name: smithy.api#String
}

structure MandateCardDetails {
    /// The expiry month of card
    card_exp_month: smithy.api#String
    /// The country code in in which the card was issued
    issuer_country: smithy.api#String
    /// The card holder name
    card_holder_name: smithy.api#String
    /// A unique identifier alias to identify a particular card
    card_fingerprint: smithy.api#String
    /// The first 6 digits of card
    card_isin: smithy.api#String
    /// The card scheme network for the particular card
    scheme: smithy.api#String
    /// The token from card locker
    card_token: smithy.api#String
    /// The bank that issued the card
    card_issuer: smithy.api#String
    /// The network that facilitates payment card transactions
    card_network: CardNetwork
    /// The last 4 digits of card
    last4_digits: smithy.api#String
    /// The nick_name of the card holder
    nick_name: smithy.api#String
    /// The expiry year of card
    card_exp_year: smithy.api#String
    /// The type of the payment card
    card_type: smithy.api#String
}

structure JCSVoucherData {
    /// The billing first name for Japanese convenience stores
    first_name: smithy.api#String
    /// The billing second name Japanese convenience stores
    last_name: smithy.api#String
    /// The telephone number for Japanese convenience stores
    phone_number: smithy.api#String
    /// The Email ID for Japanese convenience stores
    email: smithy.api#String
}

structure IdealNestedType {
    /// The billing details for bank redirection
    billing_details: BankRedirectBilling
    /// The country for bank payment
    country: CountryAlpha2
    /// The hyperswitch bank code for ideal
    bank_name: BankNames
}

structure BoletoVoucherData {
    due_date: smithy.api#String
    /// The shopper's bank account number associated with the boleto
    bank_number: smithy.api#String
    /// The fine percentage charged if payment is overdue
    fine_percentage: smithy.api#String
    /// The interest percentage charged on late payments
    interest_percentage: smithy.api#String
    /// The number of days after which the boleto is written off (canceled)
    write_off_quantity_days: smithy.api#String
    /// The shopper's social security number (CPF or CNPJ)
    social_security_number: smithy.api#String
    /// Custom messages or instructions to display on the boleto
    messages: StringList
    /// The number of days after the due date when the fine is applied
    fine_quantity_days: smithy.api#String
    /// The type of identification document used (e.g., CPF or CNPJ)
    document_type: DocumentKind
}

structure FetchQrCodeInformation {
    @required
    qr_code_fetch_url: smithy.api#String
}

/// Defines the type of discount applied to a payment, such as whether it's a fixed date discount, a daily calendar discount, or a daily business discount.
enum DiscountType {
    /// No discount logic will be applied. The payment amount remains at the base value.
    standard
    /// A sliding discount calculated per business day until the due date.
    daily_business
    /// A sliding discount calculated per calendar day until the due date.
    daily_calendar
    /// A fixed amount reduction if paid on or before a specific date.
    fixed_date
}

structure WalletAdditionalDataForCard {
    /// Last 4 digits of the card number
    last4: smithy.api#String
    /// The information of the payment method
    card_network: smithy.api#String
    /// The type of payment method
    @jsonName("type")
    card_type: smithy.api#String
}

/// Payment method data for eligibility check
union EligibilityPaymentMethodData {
    card: EligibilityCard
    real_time_payment: RealTimePaymentData
    bank_transfer: BankTransferData
    crypto: CryptoData
    open_banking: OpenBankingData
    reward: smithy.api#Unit
    bank_debit: BankDebitData
    voucher: VoucherData
    gift_card: GiftCardData
    mandate_payment: smithy.api#Unit
    pay_later: PayLaterData
    card_token: CardToken
    upi: UpiData
    card_redirect: CardRedirectData
    mobile_payment: MobilePaymentData
    bank_redirect: BankRedirectData
}

/// Enum variants for SessionToken
enum SessionTokenEnumVariants {
    /// The session response structure for Apple Pay
    apple_pay
    /// The session response structure for Paze
    paze
    /// The session response structure for Samsung Pay
    samsung_pay
    /// The session response structure for Google Pay
    google_pay
    /// The sessions response structure for ClickToPay
    click_to_pay
    /// The session response structure for Amazon Pay
    amazon_pay
    /// The session response structure for Klarna
    klarna
    /// Session token for OpenBanking PIS flow
    open_banking
    /// The session response structure for PayPal
    paypal
    /// Whenever there is no session token response or an error in session response
    no_session_token_received
}

structure PhoneDetails {
    /// The contact number
    number: smithy.api#String
    /// The country code attached to the number
    country_code: smithy.api#String
}

/// Browser information to be used for 3DS 2.0
structure BrowserInformation {
    /// The os version of the client device
    os_version: smithy.api#String
    /// Language supported
    language: smithy.api#String
    /// Whether javascript is enabled in the browser
    java_script_enabled: smithy.api#Boolean
    /// The os type of the client device
    os_type: smithy.api#String
    /// Ip address of the client
    ip_address: smithy.api#String
    /// Identifier of the source that initiated the request.
    referer: smithy.api#String
    /// Time zone of the client
    time_zone: smithy.api#Integer
    /// User-agent of the browser
    user_agent: smithy.api#String
    /// The screen width in pixels
    screen_width: smithy.api#Integer
    /// The device model of the client
    device_model: smithy.api#String
    /// Accept-language of the browser
    accept_language: smithy.api#String
    /// Whether java is enabled in the browser
    java_enabled: smithy.api#Boolean
    /// The screen height in pixels
    screen_height: smithy.api#Integer
    /// Color depth supported by the browser
    color_depth: smithy.api#Integer
    /// List of headers that are accepted
    accept_header: smithy.api#String
}

structure GpayAllowedPaymentMethods {
    /// The tokenization specification for Google Pay
    @required
    tokenization_specification: GpayTokenizationSpecification
    /// The type of payment method
    @jsonName("type")
    @required
    payment_method_type: smithy.api#String
    /// The parameters Google Pay requires
    @required
    parameters: GpayAllowedMethodsParameters
}

enum BankType {
    checking
    savings
}

structure AlfamartVoucherData {
    /// The billing second name for Alfamart
    last_name: smithy.api#String
    /// The Email ID for Alfamart
    email: smithy.api#String
    /// The billing first name for Alfamart
    first_name: smithy.api#String
}

union RealTimePaymentDataResponse {
    duit_now: DuitNowNestedType
    prompt_pay: PromptPayNestedType
    qris: QrisNestedType
    fps: FpsNestedType
    viet_qr: VietQrNestedType
}

structure MandateAmountData {
    /// Additional details required by mandate
    metadata: smithy.api#Document
    /// Specifying end date of the mandate
    end_date: smithy.api#Timestamp
    /// The maximum amount to be debited for the mandate transaction
    amount: smithy.api#Long
    /// Specifying start date of the mandate
    start_date: smithy.api#Timestamp
    /// The currency for the transaction
    @required
    currency: Currency
}

structure AmazonPayDeliveryOptions {
    /// Specifies if this delivery option is the default
    @required
    is_default: smithy.api#Boolean
    /// Shipping method details
    @required
    shipping_method: AmazonPayShippingMethod
    /// Total delivery cost
    @required
    price: AmazonPayDeliveryPrice
    /// Delivery Option identifier
    @required
    id: smithy.api#String
}

structure RefundRequest {
    /// To indicate whether to refund needs to be instant or scheduled. Default value is instant
    refund_type: RefundType
    /// Unique Identifier for the Refund. This is to ensure idempotency for multiple partial refunds initiated against the same payment. If this is not passed by the merchant, this field shall be auto generated and provided in the API response. It is recommended to generate uuid(v4) as the refund_id.
    refund_id: smithy.api#String
    /// Merchant connector details used to make payments.
    merchant_connector_details: MerchantConnectorDetailsWrap
    /// Reason for the refund. Often useful for displaying to users and your customer support executive. In case the payment went through Stripe, this field needs to be passed with one of these enums: `duplicate`, `fraudulent`, or `requested_by_customer`
    reason: smithy.api#String
    /// Charge specific fields for controlling the revert of funds from either platform or connected account
    split_refunds: SplitRefund
    /// The payment id against which refund is to be initiated
    @required
    payment_id: smithy.api#String
    /// You can specify up to 50 keys, with key names up to 40 characters long and values up to 500 characters long. Metadata is useful for storing additional, structured information on an object.
    metadata: smithy.api#Document
    /// The identifier for the Merchant Account
    merchant_id: smithy.api#String
    /// Total amount for which the refund is to be initiated. Amount for the payment in lowest denomination of the currency. (i.e) in cents for USD denomination, in paisa for INR denomination etc., If not provided, this will default to the full payment amount
    amount: smithy.api#Long
}

structure CardRedirectNestedType {
}

structure BniVaNestedType {
}

structure MultibancoBillingDetails {
    email: smithy.api#String
}

structure AliPayRedirection {
}

structure GpayTokenParameters {
    @jsonName("stripe:version")
    stripe_version: smithy.api#String
    /// The public key provided by the merchant
    public_key: smithy.api#String
    /// The name of the connector
    gateway: smithy.api#String
    /// The merchant ID registered in the connector associated
    gateway_merchant_id: smithy.api#String
    @jsonName("stripe:publishableKey")
    stripe_publishable_key: smithy.api#String
    /// The protocol version for encryption
    protocol_version: smithy.api#String
}

structure BacsNestedType {
}

/// Data for the split items
structure AdyenSplitItem {
    /// Defines type of split item
    @required
    split_type: AdyenSplitType
    /// The unique identifier of the account to which the split amount is allocated.
    account: smithy.api#String
    /// Unique Identifier for the split item
    @required
    reference: smithy.api#String
    /// The amount of the split item
    amount: smithy.api#Long
    /// Description for the part of the payment that will be allocated to the specified account.
    description: smithy.api#String
}

union BankRedirectDetails {
    Blik: BlikBankRedirectAdditionalData
    Giropay: GiropayBankRedirectAdditionalData
    BancontactCard: BancontactBankRedirectAdditionalData
}

structure GooglePayThirdPartySdk {
    /// The next action for the sdk (ex: calling confirm or sync call)
    @required
    sdk_next_action: SdkNextAction
    /// Identifier for the delayed session response
    @required
    delayed_session_token: smithy.api#Boolean
    /// The name of the connector
    @required
    connector: smithy.api#String
}

structure ReceiverDetails {
    /// The amount charged by ACH
    amount_charged: smithy.api#Long
    /// The amount received by receiver
    @required
    amount_received: smithy.api#Long
    /// The amount remaining to be sent via ACH
    amount_remaining: smithy.api#Long
}

structure GpayMerchantInfo {
    /// The merchant Identifier that needs to be passed while invoking Gpay SDK
    merchant_id: smithy.api#String
    /// The name of the merchant that needs to be displayed on Gpay PopUp
    @required
    merchant_name: smithy.api#String
}

structure PermataBankTransferNestedType {
    /// The billing details for Permata Bank Transfer
    billing_details: DokuBillingDetails
}

enum TaxStatus {
    taxable
    exempt
}

structure MultibancoBankTransferNestedType {
    /// The billing details for Multibanco
    billing_details: MultibancoBillingDetails
}

structure PixQrNestedType {
}

@mixin
structure CustomerListRequest {
    /// Offset
    @httpQuery("offset")
    offset: smithy.api#Integer
    /// Limit
    @httpQuery("limit")
    limit: smithy.api#Integer
}

union GiftCardData {
    bhn_card_network: BHNGiftCardDetails
    pay_safe_card: PaySafeCardNestedType
    givex: GiftCardDetails
}

structure InstantBankTransferNestedType {
}

/// Installment selection sent by the customer during payment confirmation.
structure InstallmentRequest {
}

enum CtpServiceProvider {
    mastercard
    visa
}

structure OpenBankingPISNestedType {
}

structure OnlineBankingFinlandNestedType {
    email: smithy.api#String
}

structure SamsungPayMerchantPaymentInformation {
    /// Merchant domain that process payments, required for web payments
    url: smithy.api#String
    /// Merchant name, this will be displayed on the Samsung Pay screen
    @required
    name: smithy.api#String
    /// Merchant country code
    @required
    country_code: CountryAlpha2
}

structure GooglePayRedirectData {
}

structure KakaoPayRedirection {
}

union UpiResponse {
    upi_intent: UpiIntentData
    upi_collect: UpiCollectAdditionalData
}

structure SdkNextActionData {
    order_id: smithy.api#String
    @required
    next_action: NextActionCall
}

structure InteracNestedType {
    email: smithy.api#String
    /// The country for bank payment
    country: CountryAlpha2
}

structure SamsungPayWalletData {
    @required
    payment_credential: SamsungPayWalletCredentials
}

structure SessionToken {
    /// Discriminator field for the tagged enum
    @required
    wallet_name: SessionTokenEnumVariants
    field_0: AmazonPaySessionTokenResponse
}

/// Indicates the type of payment method. Eg: 'card', 'wallet', etc.
enum PaymentMethod {
    gift_card
    network_token
    crypto
    bank_debit
    bank_redirect
    reward
    card
    voucher
    wallet
    card_redirect
    bank_transfer
    real_time_payment
    upi
    pay_later
    open_banking
    mobile_payment
}

structure CaptureResponse {
    /// The status of the capture
    @required
    status: CaptureStatus
    /// The name of the payment connector that processed this capture.
    @required
    connector: smithy.api#String
    /// The ID of the payment attempt that was successfully authorized and subsequently captured by this operation.
    @required
    authorized_attempt_id: smithy.api#String
    /// The error code returned by the connector if this capture operation failed. This code is connector-specific.
    error_code: smithy.api#String
    /// The capture amount. Amount for the payment in lowest denomination of the currency. (i.e) in cents for USD denomination, in paisa for INR denomination etc.,
    @required
    amount: smithy.api#Long
    /// A more detailed reason from the connector explaining the capture failure, if available.
    error_reason: smithy.api#String
    /// The currency of the amount of the capture
    currency: Currency
    /// Sequence number of this capture, in the series of captures made for the parent attempt
    @required
    capture_sequence: smithy.api#Integer
    /// A human-readable message from the connector explaining why this capture operation failed, if applicable.
    error_message: smithy.api#String
    /// The connector's own reference or transaction ID for this specific capture operation. Useful for reconciliation.
    reference_id: smithy.api#String
    /// A unique identifier for this specific capture operation.
    @required
    capture_id: smithy.api#String
    /// A unique identifier for this capture provided by the connector
    connector_capture_id: smithy.api#String
}

structure BluecodeQrRedirect {
}

structure GooglePayPaymentMethodInfo {
    assurance_details: GooglePayAssuranceDetails
    /// The name of the card network
    @required
    card_network: smithy.api#String
    /// Card funding source for the selected payment method
    card_funding_source: GooglePayCardFundingSource
    /// The details of the card
    @required
    card_details: smithy.api#String
}

structure PaylaterResponse {
    klarna_sdk: KlarnaSdkPaymentMethodResponse
}

list RefundStatusList {
    member: RefundStatus
}

list CaptureResponseList {
    member: CaptureResponse
}

/// Enum variants for NextActionData
enum NextActionDataEnumVariants {
    /// Contains duration for displaying a wait screen, wait screen with timer is displayed by sdk
    wait_screen_information
    /// Contains the information regarding three_ds_method_data submission, three_ds authentication, and authorization flows
    three_ds_invoke
    invoke_upi_qr_flow
    redirect_inside_popup
    /// Contains data required to invoke hidden iframe
    invoke_hidden_iframe
    /// Contains url to fetch Qr code data
    fetch_qr_code_information
    /// Contains the download url and the reference number for transaction
    display_voucher_information
    /// Contains the url for redirection flow
    redirect_to_url
    /// Contains consent to collect otp for mobile payment
    collect_otp
    /// Contains third party sdk session token response
    third_party_sdk_session_token
    invoke_upi_intent_sdk
    /// The data required to trigger the DDC (Device Data Collection) flow by rendering the provided URL in a hidden iframe.
    invoke_ddc
    /// Contains url for Qr code image, this qr code has to be shown in sdk
    qr_code_information
    /// Informs the next steps for bank transfer and also contains the charges details (ex: amount received, amount charged etc)
    display_bank_transfer_information
    invoke_sdk_client
}

structure BHNGiftCardDetails {
    /// The gift card or account number
    @required
    account_number: smithy.api#String
    /// The CVV2 code for Open Loop/VPLN products
    cvv2: smithy.api#String
    /// The security PIN for gift cards requiring it
    pin: smithy.api#String
    /// The expiration date in MMYYYY format for Open Loop/VPLN products
    expiration_date: smithy.api#String
}

structure SessionTokenInfo {
    @required
    certificate_keys: smithy.api#String
    @required
    certificate: smithy.api#String
    @required
    merchant_identifier: smithy.api#String
    @required
    display_name: smithy.api#String
    initiative_context: smithy.api#String
    @required
    initiative: ApplepayInitiative
    merchant_business_country: CountryAlpha2
}

structure SamsungPaySessionTokenResponse {
    /// Samsung Pay API version
    @required
    version: smithy.api#String
    /// Samsung Pay service ID to which session call needs to be made
    @required
    service_id: smithy.api#String
    /// Field containing merchant information
    @jsonName("merchant")
    @required
    merchant_payment_information: SamsungPayMerchantPaymentInformation
    /// Is shipping address required to be collected from wallet
    @required
    shipping_address_required: smithy.api#Boolean
    /// Order number of the transaction
    @required
    order_number: smithy.api#String
    /// Is billing address required to be collected from wallet
    @required
    billing_address_required: smithy.api#Boolean
    /// Payment protocol type
    @required
    protocol: SamsungPayProtocolType
    /// List of supported card brands
    @required
    allowed_brands: StringList
    /// Field containing the payment amount
    @required
    amount: SamsungPayAmountDetails
}

structure PollConfig {
    /// Interval of the poll
    @required
    delay_in_secs: smithy.api#Integer
    /// Frequency of the poll
    @required
    frequency: smithy.api#Integer
}

enum PaypalCaptureMethod {
    authorize
    capture
}

structure ApplePayRecurringPaymentRequest {
    /// The regular billing cycle for the recurring payment, including start and end dates, an interval, and an interval count
    @required
    regular_billing: ApplePayRegularBillingRequest
    /// A localized billing agreement that the payment sheet displays to the user before the user authorizes the payment
    billing_agreement: smithy.api#String
    /// A URL to a web page where the user can update or delete the payment method for the recurring payment
    @required
    management_u_r_l: smithy.api#String
    /// A description of the recurring payment that Apple Pay displays to the user in the payment sheet
    @required
    payment_description: smithy.api#String
}

/// Represents the specific data and rules related to Santander Boleto payments, including discounts, penalties, collection actions, payment constraints, beneficiary details, and document kind.
structure SantanderBoletoData {
    /// Constraints on how the payment can be made (Partial payments/Limits)
    payment_constraints: com.hyperswitch.default#BoletoPaymentTypeConstraints
    document_kind: BoletoDocumentKind
    discount_rules: SantanderPaymentDiscountRules
    /// Legal or administrative actions for non-payment (Protest/Write-off)
    collection_actions: CollectionActions
    beneficiary: BeneficiaryDetails
    /// Rules for late payments (Interest and Fines)
    penalties: PenaltyRules
}

structure MultibancoNestedType {
}

structure AtomeRedirectNestedType {
}

enum StripeChargeType {
    destination
    direct
}

structure PazeWalletData {
    @required
    complete_response: smithy.api#String
}

structure MifinityData {
    language_preference: smithy.api#String
    @required
    date_of_birth: smithy.api#String
}

/// Represents a percentage value between 0 and 100, precise to a fixed number of decimal digits
structure PercentageValue {
    /// Percentage value ranging between 0 and 100
    @required
    percentage: smithy.api#Double
}

enum AdyenSplitType {
    /// The value-added tax charged on the payment, booked to your platforms liable balance account.
    Vat
    /// Books split amount to the specified account.
    BalanceAccount
    /// The aggregated amount of all transaction fees.
    PaymentFee
    /// The transaction fees due to Adyen under blended rates.
    AdyenCommission
    /// The fees paid to the card scheme for using their network.
    SchemeFee
    /// The aggregated amount of the interchange and scheme fees.
    AcquiringFees
    /// The fees paid to the issuer for each payment made with the card network.
    Interchange
    /// The aggregated amount of Adyen's commission and markup fees.
    AdyenFees
    /// The transaction fees due to Adyen under Interchange ++ pricing.
    AdyenMarkup
    /// Your platform's commission on the payment (specified in amount), booked to your liable balance account.
    Commission
    /// Allows you and your users to top up balance accounts using direct debit, card payments, or other payment methods.
    TopUp
}

structure DiscountTier {
    /// The discount value (e.g., "5.50").
    amount: smithy.api#String
    /// The ISO-8601 date until which this discount is valid
    end_date: smithy.api#String
}

enum Connector {
    powertranz
    cybersourcedecisionmanager
    nomupay
    novalnet
    payu
    archipel
    wise
    airwallex
    billwerk
    worldpayxml
    worldpaymodular
    trustpay
    taxjar
    bitpay
    hyperpg
    dwolla
    zen
    checkout
    helcim
    revolv3
    shift4
    tesouro
    plaid
    paystack
    stripebilling
    itaubank
    bamboraapac
    celero
    authipay
    nordea
    absa_sanlam
    ctp_visa
    gpayments
    payone
    worldpay
    trustly
    trustpayments
    square
    signifyd
    inespay
    payjustnowinstore
    stripe
    worldpayvantiv
    authorizedotnet
    finix
    forte
    dlocal
    bluesnap
    getnet
    hipay
    silverflow
    wellsfargo
    payconex
    volt
    cashtocode
    tokenex
    multisafepay
    paytm
    santander
    vgs
    paybox
    nmi
    custombilling
    mollie
    nexixpay
    tsys
    juspaythreedsserver
    ctp_mastercard
    fiservcommercehub
    facilitapay
    amazonpay
    adyenplatform
    mifinity
    noon
    calida
    jpmorgan
    zift
    ebanx
    blackhawknetwork
    barclaycard
    digitalvirgo
    paysafe
    placetopay
    rapyd
    paypal
    bankofamerica
    interpayments
    boku
    opennode
    klarna
    nuvei
    adyen
    peachpayments
    redsys
    affirm
    cybersource
    payload
    riskified
    deutschebank
    globalpay
    globepay
    chargebee
    threedsecureio
    worldline
    checkbook
    tokenio
    coinbase
    cardinal
    imerchantsolutions
    zsl
    coingate
    recurly
    bambora
    braintree
    cryptopay
    flexiti
    stax
    fiuu
    iatapay
    gigadat
    payjustnow
    truelayer
    loonio
    payme
    givepayments
    gocardless
    tsys_transit
    netcetera
    xendit
    aci
    envoy
    breadpay
    fiservemea
    razorpay
    hyperswitch_vault
    elavon
    datatrans
    moneris
    prophetpay
    fiserv
    phonepe
    nexinets
}

/// Network Transaction ID and Decrypted Wallet Token Details
structure NetworkTransactionIdAndDecryptedWalletTokenDetails {
    /// The card holder's name
    card_holder_name: smithy.api#String
    /// The Mastercard Transaction Link Identifier (TLID) provided by the card network during a CIT (Customer Initiated Transaction), when `setup_future_usage` is set to `off_session`.
    transaction_link_id: smithy.api#String
    /// The token's expiry year
    @required
    token_exp_year: smithy.api#String
    /// The network transaction ID provided by the card network during a Customer Initiated Transaction (CIT) when `setup_future_usage` is set to `off_session`.
    @required
    network_transaction_id: smithy.api#String
    /// The network that facilitates payment card transactions
    card_network: CardNetwork
    /// The token's expiry month
    @required
    token_exp_month: smithy.api#String
    /// The Decrypted Token
    @required
    decrypted_token: smithy.api#String
}

structure GiftCardDetails {
    /// The gift card number
    @required
    number: smithy.api#String
    /// The card verification code.
    @required
    cvc: smithy.api#String
}

/// Charge Information
union XenditChargeResponseData {
    /// Split Between Multiple Accounts
    multiple_splits: XenditMultipleSplitResponse
    /// Collect Fee for Single Account
    single_split: XenditSplitSubMerchantData
}

structure UpiCollectData {
    /// The Virtual Payment Address (VPA) for UPI collect payment
    vpa_id: smithy.api#String
    /// The UPI source type (Credit Card, Credit Line, Account, or Credit Card + Credit Line)
    upi_source: UpiSource
}

structure GivexGiftCardAdditionalData {
    /// Last 4 digits of the gift card number
    @required
    last4: smithy.api#String
}

structure InstantBankTransferFinlandNestedType {
}

structure PixAutomaticoPushNestedType {
    /// Account number for Pix Automatico Push payment method
    account_number: smithy.api#String
    /// Bank identifier for Pix Automatico Push payment method
    bank_identifier: smithy.api#String
    /// Branch code for Pix Automatico Push payment method
    branch_code: smithy.api#String
}

union VoucherResponse {
    red_compra: smithy.api#Unit
    indomaret: IndomaretVoucherData
    boleto: BoletoVoucherData
    lawson: JCSVoucherData
    mini_stop: JCSVoucherData
    pay_easy: JCSVoucherData
    red_pagos: smithy.api#Unit
    alfamart: AlfamartVoucherData
    family_mart: JCSVoucherData
    oxxo: smithy.api#Unit
    efecty: smithy.api#Unit
    pago_efectivo: smithy.api#Unit
    seicomart: JCSVoucherData
    seven_eleven: JCSVoucherData
}

structure AirwallexData {
    /// payload required by airwallex
    payload: smithy.api#String
}

structure AchBankDebitAdditionalData {
    /// Bank holder entity type
    bank_holder_type: BankHolderType
    /// Bank account type
    bank_type: BankType
    /// Partially masked routing number for ach bank debit payment
    @required
    routing_number: smithy.api#String
    /// Bank account's owner name
    bank_account_holder_name: smithy.api#String
    /// Name of the bank
    bank_name: BankNames
    /// Partially masked account number for ach bank debit payment
    @required
    account_number: smithy.api#String
}

structure SamsungPayTokenData {
    /// Samsung Pay encrypted payment credential data
    @required
    data: smithy.api#String
    /// 3DS type used by Samsung Pay
    @jsonName("type")
    three_ds_type: smithy.api#String
    /// 3DS version used by Samsung Pay
    @required
    version: smithy.api#String
}

structure SkrillData {
}

list AmazonPayDeliveryOptionsList {
    member: AmazonPayDeliveryOptions
}

structure DisputeResponsePaymentsRetrieve {
    /// Dispute updated time sent by connector
    connector_updated_at: smithy.api#String
    /// Time at which dispute is received
    @required
    created_at: smithy.api#String
    /// The dispute amount
    @required
    amount: smithy.api#String
    /// Evidence deadline of dispute sent by connector
    challenge_required_by: smithy.api#String
    /// The identifier for dispute
    @required
    dispute_id: smithy.api#String
    /// Stage of the dispute
    @required
    dispute_stage: DisputeStage
    /// Status of the dispute sent by connector
    @required
    connector_status: smithy.api#String
    /// Dispute id sent by connector
    @required
    connector_dispute_id: smithy.api#String
    /// Dispute created time sent by connector
    connector_created_at: smithy.api#String
    /// Status of the dispute
    @required
    dispute_status: DisputeStatus
    /// Reason of dispute sent by connector
    connector_reason: smithy.api#String
    /// Reason code of dispute sent by connector
    connector_reason_code: smithy.api#String
}

structure TwintRedirectNestedType {
}

structure UpiIntentData {
    /// The UPI source type (Credit Card, Credit Line, Account, or Credit Card + Credit Line)
    upi_source: UpiSource
}

list PaymentAttemptResponseList {
    member: PaymentAttemptResponse
}

structure PaymentMethodTokenizationDetails {
}

structure ApplePayRegularBillingRequest {
    /// The time that the payment occurs as part of a successful transaction
    @required
    payment_timing: ApplePayPaymentTiming
    /// The label that Apple Pay displays to the user in the payment sheet with the recurring details
    @required
    label: smithy.api#String
    /// The number of interval units that make up the total payment interval
    recurring_payment_interval_count: smithy.api#Integer
    /// The amount of the recurring payment
    @required
    amount: smithy.api#String
    /// The amount of time — in calendar units, such as day, month, or year — that represents a fraction of the total payment interval
    recurring_payment_interval_unit: RecurringPaymentIntervalUnit
    /// The date of the final payment
    recurring_payment_end_date: smithy.api#String
    /// The date of the first payment
    recurring_payment_start_date: smithy.api#String
}

structure SepaBankDebitAdditionalData {
    /// Bank account's owner name
    bank_account_holder_name: smithy.api#String
    /// Partially masked international bank account number (iban) for SEPA
    @required
    iban: smithy.api#String
}

structure EftNestedType {
    /// The preferred eft provider
    @required
    provider: smithy.api#String
}

structure VietQrNestedType {
}

union GpaySessionTokenResponse {
    /// Google pay response involving third party sdk
    ThirdPartyResponse: GooglePayThirdPartySdk
    /// Google pay session response for non third party sdk
    GooglePaySession: GooglePaySessionResponse
}

structure PaymentsRequest {
    /// Boolean indicating whether to enable overcapture for this payment
    enable_overcapture: smithy.api#Boolean
    /// This allows to manually select a connector with which the payment can go through.
    connector: ConnectorList
    /// An arbitrary string attached to the payment. Often useful for displaying to users or for your own internal record-keeping.
    description: smithy.api#String
    payment_method_data: PaymentMethodDataRequest
    /// Whether to calculate tax for this payment intent
    skip_external_tax_calculation: smithy.api#Boolean
    payment_method: PaymentMethod
    /// Use this parameter to restrict the Payment Method Types to show for a given PaymentIntent
    allowed_payment_method_types: PaymentMethodTypeList
    merchant_connector_details: MerchantConnectorDetailsWrap
    /// The shipping cost for the payment. This is required for tax calculation in some regions.
    shipping_cost: smithy.api#Long
    /// Total tax amount applicable to the order, in the lowest denomination of the currency.
    order_tax_amount: smithy.api#Long
    /// The primary amount for the payment, provided in the lowest denomination of the specified currency (e.g., 6540 for $65.40 USD). This field is mandatory for creating a payment.
    amount: smithy.api#Long
    /// Use this object to capture the details about the different products for which the payment is being made. The sum of amount across different products here should be equal to the overall payment amount
    order_details: OrderDetailsWithAmount
    /// Provides information about a card payment that customers see on their statements. Concatenated with the prefix (shortened descriptor) or statement descriptor that’s set on the account to form the complete statement descriptor. Maximum 22 characters for the concatenated descriptor. To be deprecated soon, use billing_descriptor instead.
    statement_descriptor_suffix: smithy.api#String
    /// Optional boolean value to extent authorization period of this payment  capture method must be manual or manual_multiple
    request_extended_authorization: smithy.api#Boolean
    /// Whether to generate the payment link for this payment or not (if applicable)
    payment_link: smithy.api#Boolean
    /// Service details for click to pay external authentication
    ctp_service_details: CtpServiceDetails
    authentication_type: AuthenticationType
    /// Business label of the merchant for this payment. To be deprecated soon. Pass the profile_id instead
    business_label: smithy.api#String
    /// Your tax status for this order or transaction.
    tax_status: TaxStatus
    /// The URL to redirect the customer to after they complete the payment process or authentication. This is crucial for flows that involve off-site redirection (e.g., 3DS, some bank redirects, wallet payments).
    return_url: smithy.api#String
    /// Tax amount applied to shipping charges.
    shipping_amount_tax: smithy.api#Long
    /// The business profile to be used for this payment, if not passed the default business profile associated with the merchant account will be used. It is mandatory in case multiple business profiles have been set up.
    profile_id: smithy.api#String
    /// The type of the payment that differentiates between normal and various types of mandate payments
    payment_type: PaymentType
    /// The identifier for the customer
    customer_id: smithy.api#String
    /// Custom payment link config id set at business profile, send only if business_specific_configs is configured
    payment_link_config_id: smithy.api#String
    /// Choose what kind of sca exemption is required for this payment
    psd2_sca_exemption_type: ScaExemptionType
    /// Set to true to indicate that the customer is not in your checkout flow during this payment, and therefore is unable to authenticate. This parameter is intended for scenarios where you collect card details and charge them later. When making a recurring payment by passing a mandate_id, this parameter is mandatory
    off_session: smithy.api#Boolean
    /// Will be used to expire client secret after certain amount of time to be supplied in seconds (900) for 15 mins
    session_expiry: smithy.api#Integer
    /// Whether to perform external authentication (if applicable)
    request_external_three_ds_authentication: smithy.api#Boolean
    /// If enabled, provides whole connector response
    all_keys_required: smithy.api#Boolean
    surcharge_details: RequestSurchargeDetails
    /// Passing this object during payments creates a mandate. The mandate_type sub object is passed by the server.
    mandate_data: MandateData
    /// Passing this object creates a new customer or attaches an existing customer to the payment
    customer: CustomerDetails
    /// For non-card charges, you can use this value as the complete description that appears on your customers’ statements. Must contain at least one letter, maximum 22 characters. To be deprecated soon, use billing_descriptor instead.
    statement_descriptor_name: smithy.api#String
    /// To indicate the type of payment experience that the payment method would go through
    payment_experience: PaymentExperience
    /// The shipping address for the payment
    shipping: Address
    /// Optional. A merchant-provided unique identifier for the payment, contains 30 characters long (e.g., "pay_mbabizu24mvu3mela5njyhpit4"). If provided, it ensures idempotency for the payment creation request. If omitted, Hyperswitch generates a unique ID for the payment.
    @length(min: 30, max: 30)
    payment_id: smithy.api#String
    /// Additional details required by 3DS 2.0
    browser_info: BrowserInformation
    /// Some connectors like Airwallex and Noon might require some additional information, find specific details in the child attributes below.
    connector_metadata: ConnectorMetadata
    /// Total amount of the discount you have applied to the order or transaction.
    discount_amount: smithy.api#Long
    /// Allow partial authorization for this payment
    enable_partial_authorization: smithy.api#Boolean
    /// As Hyperswitch tokenises the sensitive details about the payments method, it provides the payment_token as a reference to a stored payment method, ensuring that the sensitive details are not exposed in any manner.
    payment_token: smithy.api#String
    /// It's a token used for client side verification.
    client_secret: smithy.api#String
    /// The amount to be captured from the user's payment method, in the lowest denomination. If not provided, and `capture_method` is `automatic`, the full payment `amount` will be captured. If `capture_method` is `manual`, this can be specified in the `/capture` call. Must be less than or equal to the authorized amount.
    amount_to_capture: smithy.api#Long
    /// You can specify up to 50 keys, with key names up to 40 characters long and values up to 500 characters long. Metadata is useful for storing additional, structured information on an object.
    metadata: smithy.api#Document
    setup_future_usage: FutureUsage
    /// Request an incremental authorization, i.e., increase the authorized amount on a confirmed payment before you capture it.
    request_incremental_authorization: smithy.api#Boolean
    /// If set to `true`, Hyperswitch attempts to confirm and authorize the payment immediately after creation, provided sufficient payment method details are included. If `false` or omitted (default is `false`), the payment is created with a status such as `requires_payment_method` or `requires_confirmation`, and a separate `POST /payments/{payment_id}/confirm` call is necessary to proceed with authorization.
    confirm: smithy.api#Boolean
    /// Can be used to specify the Payment Method Type
    payment_method_type: PaymentMethodType
    /// Indicates if 3DS method data was successfully completed or not
    threeds_method_comp_ind: ThreeDsCompletionIndicator
    /// Denotes the retry action
    retry_action: RetryAction
    /// Business country of the merchant for this payment. To be deprecated soon. Pass the profile_id instead
    business_country: CountryAlpha2
    /// Boolean flag indicating whether this payment method is stored and has been previously used for payments
    is_stored_credential: smithy.api#Boolean
    /// Your unique identifier for this payment or order. This ID helps you reconcile payments on your system. If provided, it is passed to the connector if supported.
    merchant_order_reference_id: smithy.api#String
    /// The billing details of the payment. This address will be used for invoicing.
    billing: Address
    /// Indicates if the redirection has to open in the iframe
    is_iframe_redirection_enabled: smithy.api#Boolean
    /// Details of the routing configuration for that payment
    routing: smithy.api#Document
    /// Indicates how the payment was initiated (e.g., ecommerce, mail, or telephone).
    payment_channel: PaymentChannel
    /// A unique identifier to link the payment to a mandate. To do Recurring payments after a mandate has been created, pass the mandate_id instead of payment_method_data
    mandate_id: smithy.api#String
    /// Duty or customs fee amount for international transactions.
    duty_amount: smithy.api#Long
    /// Additional data related to some frm(Fraud Risk Management) connectors
    frm_metadata: smithy.api#Document
    /// Indicates if 3ds challenge is forced
    force_3ds_challenge: smithy.api#Boolean
    /// Fee information to be charged on the payment being collected
    split_payments: SplitPaymentsRequest
    /// Details required for recurring payment
    recurring_details: RecurringDetails
    /// This "CustomerAcceptance" object is passed during Payments-Confirm request, it enlists the type, time, and mode of acceptance properties related to an acceptance done by the customer. The customer_acceptance sub object is usually passed by the SDK or client.
    customer_acceptance: CustomerAcceptance
    /// Specifies the category of a Merchant Initiated Transaction (MIT). In the case of MIT, `mit_category` tells what kind of MIT is being processed. In the case of CIT, it tells the future intended MIT type.
    mit_category: MitCategory
    /// The three-letter ISO 4217 currency code (e.g., "USD", "EUR") for the payment amount. This field is mandatory for creating a payment.
    currency: Currency
    capture_method: CaptureMethod
}

union BankDebitData {
    /// Payment Method data for Ach bank debit
    ach_bank_debit: AchBankDebitNestedType
    becs_bank_debit: BecsBankDebitNestedType
    eft_debit_order: EftDebitOrderNestedType
    sepa_guarenteed_bank_debit: SepaGuarenteedBankDebitNestedType
    sepa_bank_debit: SepaBankDebitNestedType
    bacs_bank_debit: BacsBankDebitNestedType
}

/// Specifies the type of cardholder authentication to be applied for a payment.  - `ThreeDs`: Requests 3D Secure (3DS) authentication. If the card is enrolled, 3DS authentication will be activated, potentially shifting chargeback liability to the issuer. - `NoThreeDs`: Indicates that 3D Secure authentication should not be performed. The liability for chargebacks typically remains with the merchant. This is often the default if not specified.  Note: The actual authentication behavior can also be influenced by merchant configuration and specific connector defaults. Some connectors might still enforce 3DS or bypass it regardless of this parameter.
enum AuthenticationType {
    /// If the card is enrolled for 3DS authentication, the 3DS based authentication will be activated. The liability of chargeback shift to the issuer
    three_ds
    /// 3DS based authentication will not be activated. The liability of chargeback stays with the merchant.
    no_three_ds
}

structure PaySafeCardNestedType {
}

structure BankTransferNextStepsData {
    /// The instructions for Multibanco bank transactions
    multibanco: MultibancoTransferInstructions
    /// The instructions for BACS bank transactions
    bacs_bank_instructions: BacsBankTransferInstructions
    /// The details received by the receiver
    receiver: ReceiverDetails
    /// The credit transfer for ACH transactions
    ach_credit_transfer: AchTransfer
    /// The instructions for Doku bank transactions
    doku_bank_transfer_instructions: DokuBankTransferInstructions
    /// The instructions for SEPA bank transactions
    sepa_bank_instructions: SepaBankTransferInstructions
}

union ApplePaySessionResponse {
    /// We get this session response, when there is no involvement of third party sdk This is the common response most of the times
    NoThirdPartySdk: smithy.api#Document
    /// We get this session response, when third party sdk is involved
    ThirdPartySdk: ThirdPartySdkSessionResponse
    /// This is for the empty session response
    NoSessionResponse: smithy.api#Unit
}

/// Data for PixAutomaticoQr Payment Method Type CIT (Customer Initiated Transaction) - used during mandate setup + non 0$ mandate setup
structure PixAutomaticoQrData {
    /// Mandate details for the recurring charge
    mandate_details: SantanderMandateDetails
    /// Enable retry policy for failed payments (maps to PERMITE_3R_7D if true)
    retry_policy: smithy.api#Boolean
}

structure ProtestRules {
    /// The timing logic for when the protest should occur
    @required
    protest_type: ProtestType
    /// Number of days after the due date to initiate the protest
    @required
    days_after_due_date: smithy.api#Integer
}

/// Fee information to be charged on the payment being collected via xendit
structure XenditMultipleSplitRequest {
    /// The sub-account user-id that you want to make this transaction for.
    for_user_id: smithy.api#String
    /// Name to identify split rule. Not required to be unique. Typically based on transaction and/or sub-merchant types.
    @required
    name: smithy.api#String
    /// Description to identify fee rule
    @required
    description: smithy.api#String
    /// Array of objects that define how the platform wants to route the fees and to which accounts.
    @required
    routes: XenditSplitRouteList
}

structure SepaGuarenteedBankDebitNestedType {
    /// Owner name for bank debit
    bank_account_holder_name: smithy.api#String
    /// Billing details for bank debit
    billing_details: BankDebitBilling
    /// International bank account number (iban) for SEPA
    @required
    iban: smithy.api#String
}

union BankTransferInstructions {
    /// The instructions for Doku bank transactions
    doku_bank_transfer_instructions: DokuBankTransferInstructions
    /// The credit transfer for ACH transactions
    ach_credit_transfer: AchTransfer
    /// The instructions for SEPA bank transactions
    sepa_bank_instructions: SepaBankTransferInstructions
    /// The instructions for BACS bank transactions
    bacs_bank_instructions: BacsBankTransferInstructions
    /// The instructions for Multibanco bank transactions
    multibanco: MultibancoTransferInstructions
}

/// Specifies how the payment is captured. - `automatic`: Funds are captured immediately after successful authorization. This is the default behavior if the field is omitted. - `manual`: Funds are authorized but not captured. A separate request to the `/payments/{payment_id}/capture` endpoint is required to capture the funds.
enum CaptureMethod {
    /// The capture will happen only if the merchant triggers a Capture API request. Allows for a single capture of the authorized amount.
    manual
    /// Post the payment authorization, the capture will be executed on the full amount immediately.
    automatic
    /// The capture can be scheduled to automatically get triggered at a specific date & time.
    scheduled
    /// Handles separate auth and capture sequentially; effectively the same as `Automatic` for most connectors.
    sequential_automatic
    /// The capture will happen only if the merchant triggers a Capture API request. Allows for multiple partial captures up to the authorized amount.
    manual_multiple
}

structure PermataNestedType {
}

structure NoonData {
    /// Information about the order category that merchant wants to specify at connector level. (e.g. In Noon Payments it can take values like "pay", "food", or any other custom string set by the merchant in Noon's Dashboard)
    order_category: smithy.api#String
}

enum SamsungPayAmountFormat {
    /// Display "Total (Estimated amount)" and total amount
    FORMAT_TOTAL_ESTIMATED_AMOUNT
    /// Display the total amount only
    FORMAT_TOTAL_PRICE_ONLY
}

/// Charge specific fields for controlling the revert of funds from either platform or connected account. Check sub-fields for more details.
union SplitRefund {
    /// StripeSplitRefundRequest
    stripe_split_refund: StripeSplitRefundRequest
    /// XenditSplitRefundRequest
    xendit_split_refund: XenditSplitSubMerchantData
    /// AdyenSplitRefundRequest
    adyen_split_refund: AdyenSplitData
}

structure NetworkDetails {
    network_advice_code: smithy.api#String
}

enum AuthenticationStatus {
    success
    started
    pending
    failed
}

list PaymentMethodTypeList {
    member: PaymentMethodType
}

/// Payment Method Status
enum PaymentMethodStatus {
    /// Indicates that the payment method is active and can be used for payments.
    active
    /// Indicates that the payment method is in new state
    new
    /// Indicates that the payment method is not active and hence cannot be used for payments.
    inactive
    /// Indicates that the payment method is awaiting some data before changing state to active
    awaiting_data
    /// Indicates that the payment method has been redacted/deleted and cannot be used or recovered
    redacted
    /// Indicates that the payment method is awaiting some data or action before it can be marked as 'active'.
    processing
}

/// This struct represents the cryptogram data for Apple Pay transactions
structure ApplePayCryptogramData {
    /// The online payment cryptogram
    @required
    online_payment_cryptogram: smithy.api#String
    /// The ECI (Electronic Commerce Indicator) value
    eci_indicator: smithy.api#String
}

structure MomoAtmNestedType {
}

structure BancontactCardNestedType {
    /// The card's expiry month
    card_exp_month: smithy.api#String
    /// The card number
    card_number: smithy.api#String
    /// The card holder's name
    card_holder_name: smithy.api#String
    billing_details: BankRedirectBilling
    /// The card's expiry year
    card_exp_year: smithy.api#String
}

structure OpenBankingSessionToken {
    /// The session token for OpenBanking Connectors
    @required
    open_banking_session_token: smithy.api#String
}

structure PayBrightRedirectNestedType {
}

structure KlarnaSdkPaymentMethodResponse {
    payment_type: smithy.api#String
}

list GpayAllowedPaymentMethodsList {
    member: GpayAllowedPaymentMethods
}

structure NetworkTransactionIdAndCardDetails {
    /// The card number
    @required
    card_number: smithy.api#String
    /// The name of the issuer of card
    card_issuer: smithy.api#String
    /// The card's expiry year
    @required
    card_exp_year: smithy.api#String
    card_type: smithy.api#String
    /// The card holder's nick name
    nick_name: smithy.api#String
    /// The network transaction ID provided by the card network during a CIT (Customer Initiated Transaction), when `setup_future_usage` is set to `off_session`.
    @required
    network_transaction_id: smithy.api#String
    bank_code: smithy.api#String
    card_issuing_country_code: smithy.api#String
    card_issuing_country: smithy.api#String
    /// The card's expiry month
    @required
    card_exp_month: smithy.api#String
    /// The card network for the card
    card_network: CardNetwork
    /// The Mastercard Transaction Link Identifier (TLID) provided by the card network during a CIT (Customer Initiated Transaction), when `setup_future_usage` is set to `off_session`.
    transaction_link_id: smithy.api#String
    /// The card holder's name
    card_holder_name: smithy.api#String
}

structure KlarnaSdkNestedType {
    /// The token for the sdk workflow
    @required
    token: smithy.api#String
}

structure OnlineBankingPolandNestedType {
    @required
    issuer: BankNames
}

structure GiropayBankRedirectAdditionalData {
    /// Masked bank account bic code
    bic: smithy.api#String
    /// Partially masked international bank account number (iban) for SEPA
    iban: smithy.api#String
    /// Country for bank payment
    country: CountryAlpha2
}

list AdyenSplitItemList {
    member: AdyenSplitItem
}

structure ApplePayThirdPartySdkData {
    token: smithy.api#String
}

structure WeChatPayRedirection {
}

structure WalleyRedirectNestedType {
}

/// Enum variants for IframeData
enum IframeDataEnumVariants {
    ThreedsInvokeAndCompleteAutorize
}

structure BenefitNestedType {
}

structure OnlineBankingSlovakiaNestedType {
    @required
    issuer: BankNames
}

structure DokuBankTransferInstructions {
    @required
    instructions_url: smithy.api#String
    @required
    expires_at: smithy.api#String
    @required
    reference: smithy.api#String
}

structure LocalBankTransferAdditionalData {
    /// Partially masked bank code
    bank_code: smithy.api#String
}

list IncrementalAuthorizationResponseList {
    member: IncrementalAuthorizationResponse
}

@mixin
structure MandateListConstraints {
    /// Time greater than the mandate created time
    @httpQuery("created_time.gt")
    @jsonName("created_time.gt")
    created_time_gt: smithy.api#String
    /// status of the mandate
    @httpQuery("mandate_status")
    mandate_status: MandateStatus
    /// Time less than the mandate created time
    @httpQuery("created_time.lt")
    @jsonName("created_time.lt")
    created_time_lt: smithy.api#String
    /// offset on the number of objects to return
    @httpQuery("offset")
    offset: smithy.api#Long
    /// limit on the number of objects to return
    @httpQuery("limit")
    limit: smithy.api#Long
    /// connector linked to mandate
    @httpQuery("connector")
    connector: smithy.api#String
    /// Time greater than or equals to the mandate created time
    @httpQuery("created_time.gte")
    @jsonName("created_time.gte")
    created_time_gte: smithy.api#String
    /// The time at which mandate is created
    @httpQuery("created_time")
    created_time: smithy.api#String
    /// Time less than or equals to the mandate created time
    @httpQuery("created_time.lte")
    @jsonName("created_time.lte")
    created_time_lte: smithy.api#String
}

structure UpiCollectAdditionalData {
    /// Masked VPA ID
    vpa_id: smithy.api#String
    upi_source: UpiSource
}

union VoucherData {
    red_pagos: smithy.api#Unit
    alfamart: AlfamartVoucherData
    mini_stop: JCSVoucherData
    efecty: smithy.api#Unit
    red_compra: smithy.api#Unit
    indomaret: IndomaretVoucherData
    oxxo: smithy.api#Unit
    pago_efectivo: smithy.api#Unit
    lawson: JCSVoucherData
    family_mart: JCSVoucherData
    seicomart: JCSVoucherData
    boleto: BoletoVoucherData
    pay_easy: JCSVoucherData
    seven_eleven: JCSVoucherData
}

structure RedirectResponse {
    json_payload: smithy.api#Document
    param: smithy.api#String
}

enum AuthorizationStatus {
    success
    unresolved
    failure
    processing
}

enum BankHolderType {
    personal
    business
}

structure OnlineBankingFpxNestedType {
    @required
    issuer: BankNames
}

structure BancontactBankRedirectAdditionalData {
    /// Last 4 digits of the card number
    last4: smithy.api#String
    /// The card's expiry month
    card_exp_month: smithy.api#String
    /// The card's expiry year
    card_exp_year: smithy.api#String
    /// The card holder's name
    card_holder_name: smithy.api#String
}

structure PayPalWalletData {
    /// Token generated for the Apple pay
    @required
    token: smithy.api#String
}

structure CtpServiceDetails {
    /// network transaction correlation id
    correlation_id: smithy.api#String
    /// merchant transaction id
    merchant_transaction_id: smithy.api#String
    /// Encrypted payload
    encrypted_payload: smithy.api#String
    /// provider Eg: Visa, Mastercard
    provider: CtpServiceProvider
    /// session transaction flow id
    x_src_flow_id: smithy.api#String
}

union PaymentChargeType {
    Stripe: StripeChargeType
}

structure SamsungPayWebWalletData {
    /// Specifies authentication method used
    method: smithy.api#String
    /// Brand of the payment card
    @required
    card_brand: SamsungPayCardBrand
    /// Last 4 digits of the card number
    @jsonName("card_last4digits")
    @required
    card_last_four_digits: smithy.api#String
    /// Samsung Pay token data
    @jsonName("3_d_s")
    @required
    token_data: SamsungPayTokenData
    /// Value if credential is enabled for recurring payment
    recurring_payment: smithy.api#Boolean
}

structure AmazonPayRedirectData {
}

structure GpayAllowedMethodsParameters {
    /// Billing address parameters
    billing_address_parameters: GpayBillingAddressParameters
    /// Set to false if you don't want to allow credit cards
    allow_credit_cards: smithy.api#Boolean
    /// The list of allowed auth methods (ex: 3DS, No3DS, PAN_ONLY etc)
    @required
    allowed_auth_methods: StringList
    /// Is billing address required
    billing_address_required: smithy.api#Boolean
    /// Whether assurance details are required
    assurance_details_required: smithy.api#Boolean
    /// The list of allowed card networks (ex: AMEX,JCB etc)
    @required
    allowed_card_networks: StringList
}

enum NextActionCall {
    /// The next action call is sync
    sync
    /// The next action is to await for a merchant callback
    await_merchant_callback
    /// The next action call is confirm
    confirm
    /// The next action call is Complete Authorize
    complete_authorize
    /// The next action is to deny the payment with an error message
    deny
    /// The next action is to perform eligibility check
    eligibility_check
    /// The next action call is Post Session Tokens
    post_session_tokens
}

union GiftCardAdditionalData {
    bhn_card_network: BhnCardNetworkNestedType
    pay_safe_card: PaySafeCardNestedType
    givex: smithy.api#String
}

/// Fee information to be charged on the payment being collected for sub-merchant via xendit
structure XenditSplitSubMerchantData {
    /// The sub-account user-id that you want to make this transaction for.
    @required
    for_user_id: smithy.api#String
}

/// Details of external authentication
structure ExternalAuthenticationDetailsResponse {
    /// Error Message
    error_message: smithy.api#String
    /// Authentication Status
    @required
    status: AuthenticationStatus
    /// Authentication Type - Challenge / Frictionless
    authentication_flow: DecoupledAuthenticationType
    /// Message Version
    version: smithy.api#String
    /// DS Transaction ID
    ds_transaction_id: smithy.api#String
    /// Electronic Commerce Indicator (eci)
    electronic_commerce_indicator: smithy.api#String
    /// Error Code
    error_code: smithy.api#String
}

structure IframeData {
    /// Discriminator field for the tagged enum
    @required
    method_key: IframeDataEnumVariants
    /// ThreeDS Server ID
    directory_server_id: smithy.api#String
    /// Whether ThreeDS method data submission is required
    three_ds_method_data_submission: smithy.api#Boolean
    /// ThreeDS Protocol version
    message_version: smithy.api#String
    /// ThreeDS method url
    three_ds_method_url: smithy.api#String
    /// ThreeDS method data
    three_ds_method_data: smithy.api#String
}

structure RefundResponse {
    /// A unique identifier for a payment provided by the connector
    connector_refund_id: smithy.api#String
    /// The merchant_connector_id of the processor through which this payment went through
    merchant_connector_id: smithy.api#String
    /// The refund amount, which should be less than or equal to the total payment amount. Amount for the payment in lowest denomination of the currency. (i.e) in cents for USD denomination, in paisa for INR denomination etc
    @required
    amount: smithy.api#Long
    /// The timestamp at which refund is created
    created_at: smithy.api#String
    /// The id of business profile for this refund
    profile_id: smithy.api#String
    /// The payment id against which refund is initiated
    @required
    payment_id: smithy.api#String
    /// Error message received from the issuer in case of failed refunds
    issuer_error_message: smithy.api#String
    /// Unique Identifier for the refund
    @required
    refund_id: smithy.api#String
    /// An arbitrary string attached to the object. Often useful for displaying to users and your customer support executive
    reason: smithy.api#String
    /// The connector used for the refund and the corresponding payment
    @required
    connector: smithy.api#String
    /// The code for the error
    error_code: smithy.api#String
    /// Error code unified across the connectors is received here if there was an error while calling connector
    unified_code: smithy.api#String
    /// Error message unified across the connectors is received here if there was an error while calling connector
    unified_message: smithy.api#String
    /// The error message
    error_message: smithy.api#String
    /// You can specify up to 50 keys, with key names up to 40 characters long and values up to 500 characters long. Metadata is useful for storing additional, structured information on an object
    metadata: smithy.api#Document
    /// The three-letter ISO currency code
    @required
    currency: smithy.api#String
    /// The timestamp at which refund is updated
    updated_at: smithy.api#String
    /// Charge specific fields for controlling the revert of funds from either platform or connected account
    split_refunds: SplitRefund
    /// The status for refund
    @required
    status: RefundStatus
    /// Error code received from the issuer in case of failed refunds
    issuer_error_code: smithy.api#String
}

structure BniVaBankTransferNestedType {
    /// The billing details for BniVa Bank Transfer
    billing_details: DokuBillingDetails
}

/// Enum variants for RecurringDetails
enum RecurringDetailsEnumVariants {
    /// Network transaction ID and Card Details for MIT payments when payment_method_data is not stored in the application
    network_transaction_id_and_card_details
    /// Network transaction ID and Network Token Details for MIT payments when payment_method_data is not stored in the application
    network_transaction_id_and_network_token_details
    /// Network transaction ID and Wallet Token details for MIT payments when payment_method_data is not stored in the application Applicable for wallet tokens such as Apple Pay and Google Pay.
    network_transaction_id_and_decrypted_wallet_token_details
    /// Card with Limited Data to do MIT payment Can only be used if enabled for Merchant Allows doing MIT with only Card data (no reference id)
    card_with_limited_data
    mandate_id
    payment_method_id
    processor_payment_token
}

structure AliPayHkRedirection {
}

structure FpsNestedType {
}

structure SepaAndBacsBillingDetails {
    /// The Email ID for SEPA and BACS billing
    email: smithy.api#String
    /// The billing name for SEPA and BACS billing
    name: smithy.api#String
}

