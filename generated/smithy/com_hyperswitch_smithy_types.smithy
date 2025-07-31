$version: "2"

namespace com.hyperswitch.smithy.types

/// Specifies how the payment method can be used for future payments. - `off_session`: The payment method can be used for future payments when the customer is not present. - `on_session`: The payment method is intended for use only when the customer is present during checkout. If omitted, defaults to `on_session`.
enum FutureUsage {
    OnSession
    OffSession
}

structure Address {
    /// Provide the address details
    address: AddressDetails
    phone: PhoneDetails
    email: smithy.api#String
}

/// Specifies how the payment is captured. - `automatic`: Funds are captured immediately after successful authorization. This is the default behavior if the field is omitted. - `manual`: Funds are authorized but not captured. A separate request to the `/payments/{payment_id}/capture` endpoint is required to capture the funds.
enum CaptureMethod {
    /// Post the payment authorization, the capture will be executed on the full amount immediately.
    Automatic
    /// The capture will happen only if the merchant triggers a Capture API request. Allows for a single capture of the authorized amount.
    Manual
    /// Handles separate auth and capture sequentially; effectively the same as `Automatic` for most connectors.
    SequentialAutomatic
    /// The capture will happen only if the merchant triggers a Capture API request. Allows for multiple partial captures up to the authorized amount.
    ManualMultiple
    /// The capture can be scheduled to automatically get triggered at a specific date & time.
    Scheduled
}

/// Address details
structure AddressDetails {
    /// The first name for the address
    first_name: smithy.api#String
    /// The last name for the address
    last_name: smithy.api#String
    /// The city, district, suburb, town, or village of the address.
    city: smithy.api#String
    /// The second line of the street address or P.O. Box (e.g., apartment, suite, unit, or building).
    line2: smithy.api#String
    /// The third line of the street address, if applicable.
    line3: smithy.api#String
    /// The two-letter ISO 3166-1 alpha-2 country code (e.g., US, GB).
    country: CountryAlpha2
    /// The first line of the street address or P.O. Box.
    line1: smithy.api#String
    /// The address state
    state: smithy.api#String
    /// The zip/postal code for the address
    zip: smithy.api#String
}

/// Specifies the type of cardholder authentication to be applied for a payment.  - `ThreeDs`: Requests 3D Secure (3DS) authentication. If the card is enrolled, 3DS authentication will be activated, potentially shifting chargeback liability to the issuer. - `NoThreeDs`: Indicates that 3D Secure authentication should not be performed. The liability for chargebacks typically remains with the merchant. This is often the default if not specified.  Note: The actual authentication behavior can also be influenced by merchant configuration and specific connector defaults. Some connectors might still enforce 3DS or bypass it regardless of this parameter.
enum AuthenticationType {
    /// 3DS based authentication will not be activated. The liability of chargeback stays with the merchant.
    NoThreeDs
    /// If the card is enrolled for 3DS authentication, the 3DS based authentication will be activated. The liability of chargeback shift to the issuer
    ThreeDs
}

/// RoutableConnectors are the subset of Connectors that are eligible for payments routing
enum RoutableConnectors {
    Bambora
    Cryptopay
    Opennode
    Shift4
    Signifyd
    Worldpayvantiv
    Nexixpay
    Authipay
    Razorpay
    Moneris
    Payload
    Ebanx
    Recurly
    Worldpayxml
    Zsl
    Bluesnap
    Bitpay
    Adyen
    Deutschebank
    Globalpay
    Fiuu
    Itaubank
    Hipay
    Bankofamerica
    Tsys
    Xendit
    Nmi
    Archipel
    Billwerk
    Globepay
    Helcim
    Paybox
    Redsys
    Prophetpay
    Payme
    Square
    Plaid
    Checkout
    Elavon
    Boku
    Inespay
    Trustpay
    Stax
    Coingate
    Airwallex
    Datatrans
    Facilitapay
    Payone
    Multisafepay
    Gocardless
    Authorizedotnet
    Novalnet
    Payu
    Placetopay
    Barclaycard
    Coinbase
    Cybersource
    Stripe
    Mollie
    Santander
    Wellsfargo
    Worldpay
    Zen
    Rapyd
    Chargebee
    Klarna
    Wise
    Paypal
    Dlocal
    Iatapay
    Getnet
    Noon
    Mifinity
    Nomupay
    Nexinets
    Powertranz
    Cashtocode
    Fiservemea
    Celero
    Volt
    Forte
    Worldline
    Tokenio
    Stripebilling
    Digitalvirgo
    Paystack
    Bamboraapac
    Braintree
    Jpmorgan
    Nuvei
    Aci
    Adyenplatform
    Riskified
    Fiserv
}

enum Connector {
    Stripe
    Deutschebank
    Stripebilling
    Tokenio
    Helcim
    Adyenplatform
    Paybox
    Volt
    Worldpayvantiv
    Juspaythreedsserver
    Multisafepay
    Novalnet
    Nomupay
    Airwallex
    HyperswitchVault
    Payu
    Bitpay
    Dlocal
    Klarna
    Cryptopay
    Aci
    Adyen
    Razorpay
    Worldpay
    Chargebee
    Zen
    Fiuu
    Coingate
    Inespay
    Nexixpay
    Bamboraapac
    Powertranz
    Barclaycard
    Gpayments
    Jpmorgan
    Payload
    Globepay
    Payone
    Paystack
    Prophetpay
    Celero
    Fiserv
    Gocardless
    Bluesnap
    Shift4
    Mollie
    CtpMastercard
    Riskified
    Boku
    Zsl
    Cybersource
    Globalpay
    Recurly
    Archipel
    Fiservemea
    Santander
    Digitalvirgo
    Authipay
    Trustpay
    Tsys
    Braintree
    Square
    Wellsfargo
    Worldline
    Nmi
    Hipay
    Nexinets
    Facilitapay
    Threedsecureio
    Bankofamerica
    Datatrans
    Signifyd
    Plaid
    Forte
    Bambora
    Netcetera
    Nuvei
    Taxjar
    Opennode
    CtpVisa
    Paypal
    Vgs
    Authorizedotnet
    Mifinity
    Noon
    Xendit
    Wise
    Worldpayxml
    Checkout
    Iatapay
    Payme
    Cashtocode
    Ebanx
    Placetopay
    Rapyd
    Elavon
    Coinbase
    Itaubank
    Redsys
    Moneris
    Billwerk
    Getnet
    Stax
}

structure Card {
    /// The card's expiry year
    @required
    card_exp_year: smithy.api#String
    /// The card number
    @required
    card_number: smithy.api#String
    card_issuing_country: smithy.api#String
    card_type: smithy.api#String
    /// The card network for the card
    card_network: CardNetwork
    bank_code: smithy.api#String
    /// The card's expiry month
    @required
    card_exp_month: smithy.api#String
    /// The CVC number for the card
    @required
    card_cvc: smithy.api#String
    /// The card holder's name
    card_holder_name: smithy.api#String
    /// The name of the issuer of card
    card_issuer: smithy.api#String
    /// The card holder's nick name
    nick_name: smithy.api#String
}

/// Indicates the card network.
enum CardNetwork {
    Star
    Interac
    JCB
    Pulse
    RuPay
    Discover
    Mastercard
    CartesBancaires
    AmericanExpress
    Maestro
    Accel
    Nyce
    UnionPay
    Visa
    DinersClub
}

enum CountryAlpha2 {
    NE
    CZ
    RO
    BT
    CU
    KZ
    LU
    TC
    HU
    NR
    PS
    CK
    SX
    BE
    AO
    LI
    SN
    GW
    AT
    MQ
    GB
    GG
    NO
    HM
    PK
    NI
    VN
    ME
    BV
    MA
    BO
    SY
    ET
    NU
    KP
    ZA
    MO
    GA
    SA
    BA
    PF
    CW
    MH
    SO
    IS
    DO
    MS
    GD
    MC
    GU
    IN
    ES
    NZ
    BB
    KE
    LV
    PY
    NF
    BQ
    KN
    OM
    BI
    AD
    CC
    KH
    WF
    GY
    MZ
    PN
    NG
    VU
    NA
    IR
    PA
    SE
    GM
    AZ
    NC
    SM
    SB
    CV
    WS
    AI
    ER
    IM
    BW
    EG
    PM
    MK
    AR
    SV
    MV
    TV
    MY
    VC
    AQ
    MT
    ZW
    FM
    FJ
    VE
    TK
    AF
    GP
    TD
    DM
    KR
    MP
    RW
    NP
    MW
    MX
    SC
    GI
    EE
    GF
    DJ
    FO
    SD
    JO
    PW
    PT
    CR
    UY
    CG
    KW
    SZ
    SR
    TO
    SJ
    UZ
    GR
    US
    GH
    HT
    AX
    CD
    MU
    BD
    TL
    VG
    CX
    AL
    TF
    JM
    BG
    VI
    UM
    ID
    LT
    KY
    EC
    IL
    KI
    CO
    PG
    SI
    TT
    UG
    AS
    CL
    TM
    CA
    NL
    UA
    BR
    FR
    LR
    IQ
    HN
    AW
    TH
    AG
    GQ
    BM
    CH
    AM
    LC
    KM
    BH
    IE
    PH
    MF
    CI
    IT
    ZM
    DZ
    ML
    CM
    IO
    GL
    CN
    AU
    YT
    TN
    BF
    DE
    ST
    LY
    JP
    HR
    QA
    SH
    TG
    RS
    SS
    SG
    SK
    VA
    YE
    BY
    MD
    PL
    LK
    BS
    FI
    JE
    GE
    MN
    BJ
    GS
    EH
    DK
    BZ
    TR
    HK
    FK
    KG
    LS
    CY
    GT
    TZ
    AE
    PE
    PR
    GN
    MG
    RU
    CF
    SL
    RE
    BN
    LB
    BL
    MM
    TJ
    LA
    TW
    MR
}

/// The three-letter ISO 4217 currency code (e.g., "USD", "EUR") for the payment amount. This field is mandatory for creating a payment.
enum Currency {
    LBP
    TTD
    BWP
    AMD
    CUP
    DKK
    AWG
    JMD
    LAK
    OMR
    TZS
    KMF
    MRU
    BMD
    HTG
    KYD
    BIF
    JPY
    MOP
    SGD
    VUV
    BTN
    VND
    MNT
    GEL
    EGP
    BRL
    GIP
    WST
    ZMW
    ILS
    BDT
    BZD
    LKR
    AUD
    SOS
    ALL
    JOD
    CVE
    CZK
    CHF
    KES
    BSD
    BAM
    ETB
    MYR
    CLF
    TOP
    TMT
    XAF
    XOF
    DJF
    CDF
    INR
    HRK
    SBD
    PGK
    UZS
    SLE
    GTQ
    HKD
    MUR
    THB
    PAB
    ZWL
    DZD
    VES
    KZT
    SZL
    PLN
    BBD
    QAR
    CRC
    SSP
    LSL
    RWF
    GHS
    MZN
    BND
    PHP
    SVC
    RSD
    PEN
    USD
    DOP
    HUF
    MVR
    XPF
    HNL
    MXN
    EUR
    SAR
    FJD
    SRD
    KHR
    ERN
    SDG
    XCD
    MAD
    STN
    CUC
    GNF
    NOK
    ZAR
    IRR
    ANG
    UAH
    BOB
    IDR
    TRY
    PKR
    LRD
    BGN
    TJS
    MWK
    CNY
    SLL
    MKD
    AED
    MDL
    SCR
    SEK
    RON
    ARS
    GYD
    NIO
    UYU
    YER
    UGX
    TWD
    NPR
    AFN
    AOA
    NZD
    FKP
    MMK
    KRW
    ISK
    COP
    NGN
    CLP
    MGA
    SYP
    RUB
    NAD
    PYG
    BYN
    BHD
    GBP
    IQD
    KWD
    KPW
    CAD
    GMD
    STD
    LYD
    AZN
    SHP
    KGS
    TND
}

structure PhoneDetails {
    /// The country code attached to the number
    country_code: smithy.api#String
    /// The contact number
    number: smithy.api#String
}

/// The payment method information provided for making a payment
structure PaymentMethodDataRequest {
    Voucher: smithy.api#Unit
    BankRedirect: smithy.api#Unit
    BankDebit: smithy.api#Unit
    GiftCard: smithy.api#Unit
    MandatePayment: smithy.api#Unit
    Card: Card
    CardRedirect: smithy.api#Unit
    Wallet: smithy.api#Unit
    PayLater: smithy.api#Unit
    MobilePayment: smithy.api#Unit
    BankTransfer: smithy.api#Unit
    OpenBanking: smithy.api#Unit
    Upi: smithy.api#Unit
    CardToken: smithy.api#Unit
    RealTimePayment: smithy.api#Unit
    Reward: smithy.api#Unit
    Crypto: smithy.api#Unit
}

union PaymentMethodData {
    MandatePayment: smithy.api#Unit
    GiftCard: smithy.api#Unit
    Upi: smithy.api#Unit
    Voucher: smithy.api#Unit
    Wallet: smithy.api#Unit
    RealTimePayment: smithy.api#Unit
    OpenBanking: smithy.api#Unit
    Reward: smithy.api#Unit
    Card: Card
    BankRedirect: smithy.api#Unit
    CardToken: smithy.api#Unit
    Crypto: smithy.api#Unit
    CardRedirect: smithy.api#Unit
    MobilePayment: smithy.api#Unit
    BankDebit: smithy.api#Unit
    PayLater: smithy.api#Unit
    BankTransfer: smithy.api#Unit
}

structure PaymentsRequest {
    /// Request an incremental authorization, i.e., increase the authorized amount on a confirmed payment before you capture it.
    request_incremental_authorization: smithy.api#Boolean
    /// As Hyperswitch tokenises the sensitive details about the payments method, it provides the payment_token as a reference to a stored payment method, ensuring that the sensitive details are not exposed in any manner.
    payment_token: smithy.api#String
    capture_method: CaptureMethod
    /// Indicates if 3ds challenge is forced
    force_3ds_challenge: smithy.api#Boolean
    /// If enabled, provides whole connector response
    all_keys_required: smithy.api#Boolean
    /// The customer's name. This field will be deprecated soon, use the customer object instead.
    name: smithy.api#String
    /// Provides information about a card payment that customers see on their statements. Concatenated with the prefix (shortened descriptor) or statement descriptor that’s set on the account to form the complete statement descriptor. Maximum 22 characters for the concatenated descriptor.
    statement_descriptor_suffix: smithy.api#String
    /// Business sub label for the payment
    business_sub_label: smithy.api#String
    /// A unique identifier to link the payment to a mandate. To do Recurring payments after a mandate has been created, pass the mandate_id instead of payment_method_data
    mandate_id: smithy.api#String
    /// For non-card charges, you can use this value as the complete description that appears on your customers’ statements. Must contain at least one letter, maximum 22 characters.
    statement_descriptor_name: smithy.api#String
    /// The customer's phone number This field will be deprecated soon, use the customer object instead
    phone: smithy.api#String
    /// The shipping cost for the payment. This is required for tax calculation in some regions.
    shipping_cost: smithy.api#Long
    /// If set to `true`, Hyperswitch attempts to confirm and authorize the payment immediately after creation, provided sufficient payment method details are included. If `false` or omitted (default is `false`), the payment is created with a status such as `requires_payment_method` or `requires_confirmation`, and a separate `POST /payments/{payment_id}/confirm` call is necessary to proceed with authorization.
    confirm: smithy.api#Boolean
    /// The customer's email address. This field will be deprecated soon, use the customer object instead
    email: smithy.api#String
    /// The URL to redirect the customer to after they complete the payment process or authentication. This is crucial for flows that involve off-site redirection (e.g., 3DS, some bank redirects, wallet payments).
    return_url: smithy.api#String
    /// The business profile to be used for this payment, if not passed the default business profile associated with the merchant account will be used. It is mandatory in case multiple business profiles have been set up.
    profile_id: smithy.api#String
    /// Set to true to indicate that the customer is not in your checkout flow during this payment, and therefore is unable to authenticate. This parameter is intended for scenarios where you collect card details and charge them later. When making a recurring payment by passing a mandate_id, this parameter is mandatory
    off_session: smithy.api#Boolean
    /// Whether to perform external authentication (if applicable)
    request_external_three_ds_authentication: smithy.api#Boolean
    /// This is an identifier for the merchant account. This is inferred from the API key provided during the request
    merchant_id: smithy.api#String
    /// Indicates if the redirection has to open in the iframe
    is_iframe_redirection_enabled: smithy.api#Boolean
    /// Whether to calculate tax for this payment intent
    skip_external_tax_calculation: smithy.api#Boolean
    /// Your unique identifier for this payment or order. This ID helps you reconcile payments on your system. If provided, it is passed to the connector if supported.
    merchant_order_reference_id: smithy.api#String
    /// An arbitrary string attached to the payment. Often useful for displaying to users or for your own internal record-keeping.
    description: smithy.api#String
    /// Optional. A merchant-provided unique identifier for the payment, contains 30 characters long (e.g., "pay_mbabizu24mvu3mela5njyhpit4"). If provided, it ensures idempotency for the payment creation request. If omitted, Hyperswitch generates a unique ID for the payment.
    payment_id: smithy.api#String
    setup_future_usage: FutureUsage
    /// Whether to generate the payment link for this payment or not (if applicable)
    payment_link: smithy.api#Boolean
    /// The identifier for the customer
    customer_id: smithy.api#String
    /// The primary amount for the payment, provided in the lowest denomination of the specified currency (e.g., 6540 for $65.40 USD). This field is mandatory for creating a payment.
    amount: smithy.api#Long
    /// This allows to manually select a connector with which the payment can go through.
    connector: ConnectorList
    /// The billing details of the payment. This address will be used for invoicing.
    billing: Address
    payment_method_data: PaymentMethodDataRequest
    authentication_type: AuthenticationType
    /// Business label of the merchant for this payment. To be deprecated soon. Pass the profile_id instead
    business_label: smithy.api#String
    /// Total tax amount applicable to the order, in the lowest denomination of the currency.
    order_tax_amount: smithy.api#Long
    /// Will be used to expire client secret after certain amount of time to be supplied in seconds (900) for 15 mins
    session_expiry: smithy.api#Integer
    /// It's a token used for client side verification.
    client_secret: smithy.api#String
    /// Custom payment link config id set at business profile, send only if business_specific_configs is configured
    payment_link_config_id: smithy.api#String
    /// The amount to be captured from the user's payment method, in the lowest denomination. If not provided, and `capture_method` is `automatic`, the full payment `amount` will be captured. If `capture_method` is `manual`, this can be specified in the `/capture` call. Must be less than or equal to the authorized amount.
    amount_to_capture: smithy.api#Long
    /// The country code for the customer phone number This field will be deprecated soon, use the customer object instead
    phone_country_code: smithy.api#String
    /// The three-letter ISO 4217 currency code (e.g., "USD", "EUR") for the payment amount. This field is mandatory for creating a payment.
    currency: Currency
    /// This is used along with the payment_token field while collecting during saved card payments. This field will be deprecated soon, use the payment_method_data.card_token object instead
    card_cvc: smithy.api#String
}

list ConnectorList {
    member: Connector
}

