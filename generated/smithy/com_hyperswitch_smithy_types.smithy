$version: "2"

namespace com.hyperswitch.smithy.types

/// Specifies how the payment is captured. - `automatic`: Funds are captured immediately after successful authorization. This is the default behavior if the field is omitted. - `manual`: Funds are authorized but not captured. A separate request to the `/payments/{payment_id}/capture` endpoint is required to capture the funds.
enum CaptureMethod {
    /// The capture will happen only if the merchant triggers a Capture API request. Allows for multiple partial captures up to the authorized amount.
    ManualMultiple
    /// The capture will happen only if the merchant triggers a Capture API request. Allows for a single capture of the authorized amount.
    Manual
    /// The capture can be scheduled to automatically get triggered at a specific date & time.
    Scheduled
    /// Handles separate auth and capture sequentially; effectively the same as `Automatic` for most connectors.
    SequentialAutomatic
    /// Post the payment authorization, the capture will be executed on the full amount immediately.
    Automatic
}

enum CountryAlpha2 {
    BF
    MF
    PL
    RO
    LS
    KH
    KZ
    PR
    KN
    AU
    CM
    EG
    AS
    ML
    IL
    SN
    CA
    DM
    GP
    DE
    KW
    MK
    ET
    NF
    VE
    GL
    MW
    SS
    TJ
    MG
    BJ
    US
    BL
    CC
    MS
    GY
    BG
    SX
    JO
    TD
    CK
    PW
    LI
    NC
    MT
    NP
    IQ
    UG
    VA
    SC
    CU
    AZ
    TT
    TC
    PY
    HT
    MP
    TZ
    SM
    DJ
    VI
    ER
    JE
    BB
    BA
    AW
    KY
    AQ
    KG
    LA
    FJ
    LY
    GB
    IT
    NO
    SD
    CX
    UZ
    SJ
    TM
    CO
    CN
    MC
    QA
    TF
    BR
    CR
    BE
    FI
    YE
    NG
    IM
    PH
    AX
    KE
    GN
    LV
    GU
    AL
    CV
    PA
    CW
    PN
    RS
    PM
    SB
    SV
    HU
    ID
    TN
    NL
    TO
    BZ
    GF
    IS
    GS
    ZA
    LR
    GA
    WS
    ZM
    BV
    KR
    PG
    VC
    SL
    MV
    LT
    DK
    SZ
    SK
    MZ
    AM
    IO
    CY
    GR
    KP
    MN
    LU
    NA
    BQ
    PT
    SI
    BO
    KM
    FM
    BI
    SY
    FO
    IE
    EH
    CD
    AI
    HK
    DZ
    GG
    SA
    MO
    GW
    YT
    IR
    OM
    SO
    SE
    AE
    MU
    TG
    UY
    AO
    BN
    MR
    TW
    VU
    DO
    NE
    UA
    KI
    LB
    GI
    SR
    BS
    SH
    TH
    UM
    JM
    BH
    CH
    WF
    RW
    BM
    PS
    PE
    MH
    LC
    AR
    ST
    IN
    AD
    MD
    CI
    MQ
    HN
    RE
    FK
    FR
    ES
    NU
    NR
    EC
    CF
    GM
    ME
    NZ
    PK
    GD
    CG
    TL
    PF
    MX
    GE
    CZ
    HM
    GQ
    NI
    RU
    BT
    HR
    CL
    BW
    MA
    LK
    TR
    TV
    MY
    AT
    BD
    EE
    MM
    SG
    VN
    TK
    AF
    AG
    BY
    GH
    GT
    JP
    VG
    ZW
}

structure Card {
    /// The card holder's name
    card_holder_name: smithy.api#String
    /// The CVC number for the card
    @required
    card_cvc: smithy.api#String
    /// The card number
    @required
    card_number: smithy.api#String
    card_type: smithy.api#String
    /// The card's expiry month
    @required
    card_exp_month: smithy.api#String
    card_issuing_country: smithy.api#String
    /// The name of the issuer of card
    card_issuer: smithy.api#String
    /// The card holder's nick name
    nick_name: smithy.api#String
    /// The card's expiry year
    @required
    card_exp_year: smithy.api#String
    bank_code: smithy.api#String
    /// The card network for the card
    card_network: CardNetwork
}

enum Connector {
    Bluesnap
    Recurly
    Wellsfargo
    Gocardless
    Opennode
    Worldpayxml
    Taxjar
    Zen
    Checkout
    Prophetpay
    Zsl
    Airwallex
    Moneris
    Stripebilling
    Vgs
    Xendit
    Fiservemea
    Adyen
    Rapyd
    Dlocal
    Aci
    Fiuu
    Forte
    Billwerk
    Threedsecureio
    Coinbase
    Hipay
    Stax
    Barclaycard
    Archipel
    Mifinity
    Adyenplatform
    CtpMastercard
    Juspaythreedsserver
    Gpayments
    Nexixpay
    Payload
    Razorpay
    Paybox
    CtpVisa
    Ebanx
    Shift4
    Volt
    Inespay
    Signifyd
    Redsys
    Santander
    Worldline
    Bitpay
    Mollie
    Paypal
    Novalnet
    Nmi
    Nuvei
    Deutschebank
    Noon
    Authorizedotnet
    Netcetera
    Payu
    Globepay
    Nexinets
    Payone
    Stripe
    Tsys
    Itaubank
    Helcim
    Celero
    Iatapay
    Paystack
    Datatrans
    Square
    Worldpayvantiv
    Plaid
    Cashtocode
    Bambora
    Boku
    Coingate
    Digitalvirgo
    Placetopay
    Facilitapay
    Globalpay
    Cybersource
    Bamboraapac
    Braintree
    Fiserv
    Getnet
    Payme
    HyperswitchVault
    Bankofamerica
    Cryptopay
    Jpmorgan
    Chargebee
    Elavon
    Klarna
    Nomupay
    Tokenio
    Trustpay
    Wise
    Riskified
    Worldpay
    Multisafepay
    Powertranz
    Authipay
}

/// Specifies the type of cardholder authentication to be applied for a payment.  - `ThreeDs`: Requests 3D Secure (3DS) authentication. If the card is enrolled, 3DS authentication will be activated, potentially shifting chargeback liability to the issuer. - `NoThreeDs`: Indicates that 3D Secure authentication should not be performed. The liability for chargebacks typically remains with the merchant. This is often the default if not specified.  Note: The actual authentication behavior can also be influenced by merchant configuration and specific connector defaults. Some connectors might still enforce 3DS or bypass it regardless of this parameter.
enum AuthenticationType {
    /// 3DS based authentication will not be activated. The liability of chargeback stays with the merchant.
    NoThreeDs
    /// If the card is enrolled for 3DS authentication, the 3DS based authentication will be activated. The liability of chargeback shift to the issuer
    ThreeDs
}

/// The three-letter ISO 4217 currency code (e.g., "USD", "EUR") for the payment amount. This field is mandatory for creating a payment.
enum Currency {
    AUD
    CLP
    SOS
    KZT
    SVC
    EGP
    SBD
    GIP
    PLN
    TMT
    TOP
    IRR
    SRD
    DOP
    AFN
    EUR
    MZN
    NPR
    XPF
    QAR
    LBP
    GBP
    IDR
    MAD
    PHP
    GTQ
    LYD
    BRL
    CNY
    TWD
    SYP
    JMD
    SZL
    NOK
    BIF
    BND
    OMR
    HNL
    SGD
    ILS
    BBD
    KPW
    BYN
    INR
    RSD
    UYU
    WST
    CZK
    KGS
    LSL
    MRU
    USD
    ZMW
    FJD
    TRY
    YER
    SAR
    KWD
    SSP
    PGK
    TJS
    VES
    BHD
    AOA
    CLF
    HTG
    SEK
    PKR
    NGN
    DJF
    CAD
    AZN
    GYD
    MNT
    BZD
    GNF
    JOD
    CHF
    ERN
    MOP
    CVE
    COP
    RUB
    MVR
    UZS
    MXN
    MYR
    XAF
    GHS
    STN
    SDG
    TZS
    CDF
    GEL
    PAB
    HUF
    MGA
    PYG
    ALL
    ETB
    HKD
    SLL
    ZAR
    AMD
    BMD
    TND
    STD
    KYD
    UAH
    VUV
    BSD
    ANG
    CUC
    XOF
    AWG
    RON
    IQD
    DKK
    JPY
    KES
    MMK
    SLE
    THB
    MUR
    UGX
    BTN
    MWK
    CRC
    NAD
    KRW
    SHP
    MKD
    BGN
    CUP
    MDL
    NZD
    NIO
    BAM
    RWF
    BDT
    LKR
    SCR
    TTD
    KMF
    LAK
    AED
    DZD
    ZWL
    BOB
    ISK
    GMD
    HRK
    LRD
    ARS
    PEN
    FKP
    KHR
    VND
    XCD
    BWP
}

/// Address details
structure AddressDetails {
    /// The last name for the address
    last_name: smithy.api#String
    /// The zip/postal code for the address
    zip: smithy.api#String
    /// The two-letter ISO 3166-1 alpha-2 country code (e.g., US, GB).
    country: CountryAlpha2
    /// The city, district, suburb, town, or village of the address.
    city: smithy.api#String
    /// The first line of the street address or P.O. Box.
    line1: smithy.api#String
    /// The third line of the street address, if applicable.
    line3: smithy.api#String
    /// The address state
    state: smithy.api#String
    /// The first name for the address
    first_name: smithy.api#String
    /// The second line of the street address or P.O. Box (e.g., apartment, suite, unit, or building).
    line2: smithy.api#String
}

/// Indicates the card network.
enum CardNetwork {
    DinersClub
    Nyce
    Mastercard
    AmericanExpress
    Accel
    CartesBancaires
    Pulse
    Visa
    Star
    Interac
    JCB
    RuPay
    Maestro
    Discover
    UnionPay
}

structure PaymentsRequest {
    authentication_type: AuthenticationType
    /// This is an identifier for the merchant account. This is inferred from the API key provided during the request
    merchant_id: smithy.api#String
    /// The primary amount for the payment, provided in the lowest denomination of the specified currency (e.g., 6540 for $65.40 USD). This field is mandatory for creating a payment.
    amount: smithy.api#Long
    /// The amount to be captured from the user's payment method, in the lowest denomination. If not provided, and `capture_method` is `automatic`, the full payment `amount` will be captured. If `capture_method` is `manual`, this can be specified in the `/capture` call. Must be less than or equal to the authorized amount.
    amount_to_capture: smithy.api#Long
    /// The customer's email address. This field will be deprecated soon, use the customer object instead
    email: smithy.api#String
    /// The customer's phone number This field will be deprecated soon, use the customer object instead
    phone: smithy.api#String
    /// For non-card charges, you can use this value as the complete description that appears on your customers’ statements. Must contain at least one letter, maximum 22 characters.
    statement_descriptor_name: smithy.api#String
    /// The business profile to be used for this payment, if not passed the default business profile associated with the merchant account will be used. It is mandatory in case multiple business profiles have been set up.
    profile_id: smithy.api#String
    /// The shipping cost for the payment. This is required for tax calculation in some regions.
    shipping_cost: smithy.api#Long
    /// It's a token used for client side verification.
    client_secret: smithy.api#String
    /// Whether to perform external authentication (if applicable)
    request_external_three_ds_authentication: smithy.api#Boolean
    /// Indicates if the redirection has to open in the iframe
    is_iframe_redirection_enabled: smithy.api#Boolean
    /// This allows to manually select a connector with which the payment can go through.
    connector: ConnectorList
    /// The billing details of the payment. This address will be used for invoicing.
    billing: Address
    /// The country code for the customer phone number This field will be deprecated soon, use the customer object instead
    phone_country_code: smithy.api#String
    /// Total tax amount applicable to the order, in the lowest denomination of the currency.
    order_tax_amount: smithy.api#Long
    /// A unique identifier to link the payment to a mandate. To do Recurring payments after a mandate has been created, pass the mandate_id instead of payment_method_data
    mandate_id: smithy.api#String
    /// Whether to generate the payment link for this payment or not (if applicable)
    payment_link: smithy.api#Boolean
    /// Request an incremental authorization, i.e., increase the authorized amount on a confirmed payment before you capture it.
    request_incremental_authorization: smithy.api#Boolean
    /// Optional. A merchant-provided unique identifier for the payment, contains 30 characters long (e.g., "pay_mbabizu24mvu3mela5njyhpit4"). If provided, it ensures idempotency for the payment creation request. If omitted, Hyperswitch generates a unique ID for the payment.
    payment_id: smithy.api#String
    /// Set to true to indicate that the customer is not in your checkout flow during this payment, and therefore is unable to authenticate. This parameter is intended for scenarios where you collect card details and charge them later. When making a recurring payment by passing a mandate_id, this parameter is mandatory
    off_session: smithy.api#Boolean
    /// As Hyperswitch tokenises the sensitive details about the payments method, it provides the payment_token as a reference to a stored payment method, ensuring that the sensitive details are not exposed in any manner.
    payment_token: smithy.api#String
    /// The identifier for the customer
    customer_id: smithy.api#String
    capture_method: CaptureMethod
    /// Custom payment link config id set at business profile, send only if business_specific_configs is configured
    payment_link_config_id: smithy.api#String
    /// Whether to calculate tax for this payment intent
    skip_external_tax_calculation: smithy.api#Boolean
    /// Business label of the merchant for this payment. To be deprecated soon. Pass the profile_id instead
    business_label: smithy.api#String
    /// Provides information about a card payment that customers see on their statements. Concatenated with the prefix (shortened descriptor) or statement descriptor that’s set on the account to form the complete statement descriptor. Maximum 22 characters for the concatenated descriptor.
    statement_descriptor_suffix: smithy.api#String
    /// Indicates if 3ds challenge is forced
    force_3ds_challenge: smithy.api#Boolean
    setup_future_usage: FutureUsage
    /// The three-letter ISO 4217 currency code (e.g., "USD", "EUR") for the payment amount. This field is mandatory for creating a payment.
    currency: Currency
    /// An arbitrary string attached to the payment. Often useful for displaying to users or for your own internal record-keeping.
    description: smithy.api#String
    payment_method_data: PaymentMethodDataRequest
    /// The URL to redirect the customer to after they complete the payment process or authentication. This is crucial for flows that involve off-site redirection (e.g., 3DS, some bank redirects, wallet payments).
    return_url: smithy.api#String
    /// Business sub label for the payment
    business_sub_label: smithy.api#String
    /// Your unique identifier for this payment or order. This ID helps you reconcile payments on your system. If provided, it is passed to the connector if supported.
    merchant_order_reference_id: smithy.api#String
    /// If set to `true`, Hyperswitch attempts to confirm and authorize the payment immediately after creation, provided sufficient payment method details are included. If `false` or omitted (default is `false`), the payment is created with a status such as `requires_payment_method` or `requires_confirmation`, and a separate `POST /payments/{payment_id}/confirm` call is necessary to proceed with authorization.
    confirm: smithy.api#Boolean
    /// The customer's name. This field will be deprecated soon, use the customer object instead.
    name: smithy.api#String
    /// This is used along with the payment_token field while collecting during saved card payments. This field will be deprecated soon, use the payment_method_data.card_token object instead
    card_cvc: smithy.api#String
    /// If enabled, provides whole connector response
    all_keys_required: smithy.api#Boolean
    /// Will be used to expire client secret after certain amount of time to be supplied in seconds (900) for 15 mins
    session_expiry: smithy.api#Integer
}

/// The payment method information provided for making a payment
structure PaymentMethodDataRequest {
}

union PaymentMethodData {
    Card: Card
}

list ConnectorList {
    member: Connector
}

structure PhoneDetails {
    /// The contact number
    number: smithy.api#String
    /// The country code attached to the number
    country_code: smithy.api#String
}

/// RoutableConnectors are the subset of Connectors that are eligible for payments routing
enum RoutableConnectors {
    Riskified
    Santander
    Signifyd
    Xendit
    Zen
    Tsys
    Braintree
    Authorizedotnet
    Nuvei
    Square
    Payload
    Nexinets
    Trustpay
    Facilitapay
    Klarna
    Coingate
    Celero
    Archipel
    Mifinity
    Mollie
    Worldline
    Barclaycard
    Authipay
    Adyenplatform
    Wellsfargo
    Opennode
    Zsl
    Checkout
    Worldpay
    Cryptopay
    Recurly
    Helcim
    Getnet
    Fiserv
    Coinbase
    Cashtocode
    Cybersource
    Nmi
    Rapyd
    Fiservemea
    Moneris
    Adyen
    Paybox
    Razorpay
    Stax
    Elavon
    Itaubank
    Bankofamerica
    Nexixpay
    Datatrans
    Jpmorgan
    Chargebee
    Globepay
    Paystack
    Redsys
    Nomupay
    Fiuu
    Stripe
    Multisafepay
    Worldpayxml
    Placetopay
    Boku
    Dlocal
    Hipay
    Globalpay
    Billwerk
    Payu
    Ebanx
    Plaid
    Wise
    Novalnet
    Bamboraapac
    Airwallex
    Bluesnap
    Aci
    Iatapay
    Payme
    Deutschebank
    Payone
    Bambora
    Paypal
    Powertranz
    Prophetpay
    Inespay
    Shift4
    Bitpay
    Stripebilling
    Digitalvirgo
    Tokenio
    Gocardless
    Volt
    Worldpayvantiv
    Noon
    Forte
}

/// Specifies how the payment method can be used for future payments. - `off_session`: The payment method can be used for future payments when the customer is not present. - `on_session`: The payment method is intended for use only when the customer is present during checkout. If omitted, defaults to `on_session`.
enum FutureUsage {
    OffSession
    OnSession
}

structure Address {
    email: smithy.api#String
    /// Provide the address details
    address: AddressDetails
    phone: PhoneDetails
}

