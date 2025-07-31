$version: "2"

namespace com.hyperswitch.smithy.types

/// Address details
structure AddressDetails {
    /// The second line of the street address or P.O. Box (e.g., apartment, suite, unit, or building).
    line2: smithy.api#String
    /// The third line of the street address, if applicable.
    line3: smithy.api#String
    /// The zip/postal code for the address
    zip: smithy.api#String
    /// The last name for the address
    last_name: smithy.api#String
    /// The first name for the address
    first_name: smithy.api#String
    /// The first line of the street address or P.O. Box.
    line1: smithy.api#String
    /// The two-letter ISO 3166-1 alpha-2 country code (e.g., US, GB).
    country: CountryAlpha2
    /// The city, district, suburb, town, or village of the address.
    city: smithy.api#String
    /// The address state
    state: smithy.api#String
}

enum Connector {
    Getnet
    Authipay
    Nexixpay
    Stripe
    Plaid
    Deutschebank
    Zsl
    Cashtocode
    Authorizedotnet
    Nuvei
    Paypal
    Digitalvirgo
    Globepay
    DummyConnector4
    Payu
    Bamboraapac
    Recurly
    CtpMastercard
    Shift4
    Stax
    Wise
    Worldpayvantiv
    Coingate
    Fiserv
    Inespay
    Gpayments
    Threedsecureio
    Airwallex
    Celero
    Helcim
    Square
    CtpVisa
    Tsys
    Riskified
    Braintree
    Xendit
    Payload
    Razorpay
    Rapyd
    Coinbase
    DummyConnector2
    Mifinity
    Stripebilling
    Trustpay
    Wellsfargo
    Iatapay
    Klarna
    Forte
    Vgs
    Cybersource
    DummyConnector6
    Multisafepay
    Nmi
    Bluesnap
    Gocardless
    Novalnet
    Fiuu
    Payme
    Payone
    Nexinets
    Fiservemea
    Santander
    Aci
    DummyConnector3
    Cryptopay
    DummyConnector7
    Billwerk
    Moneris
    Placetopay
    DummyBillingConnector
    Archipel
    Powertranz
    Volt
    Bankofamerica
    Paystack
    Zen
    Worldpayxml
    DummyConnector1
    Barclaycard
    Hipay
    Prophetpay
    Elavon
    Netcetera
    Adyen
    Globalpay
    HyperswitchVault
    Juspaythreedsserver
    Nomupay
    Checkout
    Datatrans
    Jpmorgan
    Noon
    Paybox
    Redsys
    Chargebee
    Mollie
    Tokenio
    Worldline
    Bitpay
    Signifyd
    DummyConnector5
    Boku
    Ebanx
    Itaubank
    Opennode
    Taxjar
    Adyenplatform
    Bambora
    Facilitapay
    Dlocal
    Worldpay
}

structure PaymentsRequest {
    /// Set to true to indicate that the customer is not in your checkout flow during this payment, and therefore is unable to authenticate. This parameter is intended for scenarios where you collect card details and charge them later. When making a recurring payment by passing a mandate_id, this parameter is mandatory
    off_session: smithy.api#Boolean
    /// Whether to generate the payment link for this payment or not (if applicable)
    payment_link: smithy.api#Boolean
    capture_method: CaptureMethod
    /// Whether to calculate tax for this payment intent
    skip_external_tax_calculation: smithy.api#Boolean
    /// The amount to be captured from the user's payment method, in the lowest denomination. If not provided, and `capture_method` is `automatic`, the full payment `amount` will be captured. If `capture_method` is `manual`, this can be specified in the `/capture` call. Must be less than or equal to the authorized amount.
    amount_to_capture: smithy.api#Long
    /// Indicates if 3ds challenge is forced
    force_3ds_challenge: smithy.api#Boolean
    /// If enabled, provides whole connector response
    all_keys_required: smithy.api#Boolean
    /// The shipping cost for the payment. This is required for tax calculation in some regions.
    shipping_cost: smithy.api#Long
    payment_method_data: PaymentMethodDataRequest
    /// For non-card charges, you can use this value as the complete description that appears on your customers’ statements. Must contain at least one letter, maximum 22 characters.
    statement_descriptor_name: smithy.api#String
    /// An arbitrary string attached to the payment. Often useful for displaying to users or for your own internal record-keeping.
    description: smithy.api#String
    /// A unique identifier to link the payment to a mandate. To do Recurring payments after a mandate has been created, pass the mandate_id instead of payment_method_data
    mandate_id: smithy.api#String
    /// Business sub label for the payment
    business_sub_label: smithy.api#String
    /// As Hyperswitch tokenises the sensitive details about the payments method, it provides the payment_token as a reference to a stored payment method, ensuring that the sensitive details are not exposed in any manner.
    payment_token: smithy.api#String
    /// Business label of the merchant for this payment. To be deprecated soon. Pass the profile_id instead
    business_label: smithy.api#String
    /// It's a token used for client side verification.
    client_secret: smithy.api#String
    /// The billing details of the payment. This address will be used for invoicing.
    billing: Address
    /// The customer's name. This field will be deprecated soon, use the customer object instead.
    name: smithy.api#String
    /// Will be used to expire client secret after certain amount of time to be supplied in seconds (900) for 15 mins
    session_expiry: smithy.api#Integer
    authentication_type: AuthenticationType
    /// The customer's phone number This field will be deprecated soon, use the customer object instead
    phone: smithy.api#String
    /// Total tax amount applicable to the order, in the lowest denomination of the currency.
    order_tax_amount: smithy.api#Long
    /// The customer's email address. This field will be deprecated soon, use the customer object instead
    email: smithy.api#String
    setup_future_usage: FutureUsage
    /// The business profile to be used for this payment, if not passed the default business profile associated with the merchant account will be used. It is mandatory in case multiple business profiles have been set up.
    profile_id: smithy.api#String
    /// Indicates if the redirection has to open in the iframe
    is_iframe_redirection_enabled: smithy.api#Boolean
    /// Your unique identifier for this payment or order. This ID helps you reconcile payments on your system. If provided, it is passed to the connector if supported.
    merchant_order_reference_id: smithy.api#String
    /// This allows to manually select a connector with which the payment can go through.
    connector: ConnectorList
    /// Optional. A merchant-provided unique identifier for the payment, contains 30 characters long (e.g., "pay_mbabizu24mvu3mela5njyhpit4"). If provided, it ensures idempotency for the payment creation request. If omitted, Hyperswitch generates a unique ID for the payment.
    payment_id: smithy.api#String
    /// Request an incremental authorization, i.e., increase the authorized amount on a confirmed payment before you capture it.
    request_incremental_authorization: smithy.api#Boolean
    /// Custom payment link config id set at business profile, send only if business_specific_configs is configured
    payment_link_config_id: smithy.api#String
    /// The URL to redirect the customer to after they complete the payment process or authentication. This is crucial for flows that involve off-site redirection (e.g., 3DS, some bank redirects, wallet payments).
    return_url: smithy.api#String
    /// Whether to perform external authentication (if applicable)
    request_external_three_ds_authentication: smithy.api#Boolean
    /// The identifier for the customer
    customer_id: smithy.api#String
    /// This is used along with the payment_token field while collecting during saved card payments. This field will be deprecated soon, use the payment_method_data.card_token object instead
    card_cvc: smithy.api#String
    /// The primary amount for the payment, provided in the lowest denomination of the specified currency (e.g., 6540 for $65.40 USD). This field is mandatory for creating a payment.
    amount: smithy.api#Long
    /// Provides information about a card payment that customers see on their statements. Concatenated with the prefix (shortened descriptor) or statement descriptor that’s set on the account to form the complete statement descriptor. Maximum 22 characters for the concatenated descriptor.
    statement_descriptor_suffix: smithy.api#String
    /// This is an identifier for the merchant account. This is inferred from the API key provided during the request
    merchant_id: smithy.api#String
    /// If set to `true`, Hyperswitch attempts to confirm and authorize the payment immediately after creation, provided sufficient payment method details are included. If `false` or omitted (default is `false`), the payment is created with a status such as `requires_payment_method` or `requires_confirmation`, and a separate `POST /payments/{payment_id}/confirm` call is necessary to proceed with authorization.
    confirm: smithy.api#Boolean
    /// The country code for the customer phone number This field will be deprecated soon, use the customer object instead
    phone_country_code: smithy.api#String
    /// The three-letter ISO 4217 currency code (e.g., "USD", "EUR") for the payment amount. This field is mandatory for creating a payment.
    currency: Currency
}

/// Specifies how the payment is captured. - `automatic`: Funds are captured immediately after successful authorization. This is the default behavior if the field is omitted. - `manual`: Funds are authorized but not captured. A separate request to the `/payments/{payment_id}/capture` endpoint is required to capture the funds.
enum CaptureMethod {
    /// The capture will happen only if the merchant triggers a Capture API request. Allows for multiple partial captures up to the authorized amount.
    ManualMultiple
    /// The capture can be scheduled to automatically get triggered at a specific date & time.
    Scheduled
    /// Handles separate auth and capture sequentially; effectively the same as `Automatic` for most connectors.
    SequentialAutomatic
    /// Post the payment authorization, the capture will be executed on the full amount immediately.
    Automatic
    /// The capture will happen only if the merchant triggers a Capture API request. Allows for a single capture of the authorized amount.
    Manual
}

structure PhoneDetails {
    /// The country code attached to the number
    country_code: smithy.api#String
    /// The contact number
    number: smithy.api#String
}

/// Specifies the type of cardholder authentication to be applied for a payment.  - `ThreeDs`: Requests 3D Secure (3DS) authentication. If the card is enrolled, 3DS authentication will be activated, potentially shifting chargeback liability to the issuer. - `NoThreeDs`: Indicates that 3D Secure authentication should not be performed. The liability for chargebacks typically remains with the merchant. This is often the default if not specified.  Note: The actual authentication behavior can also be influenced by merchant configuration and specific connector defaults. Some connectors might still enforce 3DS or bypass it regardless of this parameter.
enum AuthenticationType {
    /// 3DS based authentication will not be activated. The liability of chargeback stays with the merchant.
    NoThreeDs
    /// If the card is enrolled for 3DS authentication, the 3DS based authentication will be activated. The liability of chargeback shift to the issuer
    ThreeDs
}

list ConnectorList {
    member: Connector
}

/// RoutableConnectors are the subset of Connectors that are eligible for payments routing
enum RoutableConnectors {
    Aci
    Redsys
    Riskified
    Trustpay
    Wise
    DummyBillingConnector
    Forte
    Nuvei
    Volt
    Cybersource
    Placetopay
    Barclaycard
    Worldpay
    Datatrans
    Ebanx
    Nmi
    Airwallex
    Zsl
    Billwerk
    Fiserv
    DummyConnector5
    Bitpay
    Paypal
    Coinbase
    Stax
    Inespay
    Plaid
    Worldpayxml
    Authorizedotnet
    Bluesnap
    Facilitapay
    Rapyd
    Chargebee
    Checkout
    Tokenio
    Worldpayvantiv
    Payme
    DummyConnector6
    Klarna
    Globepay
    Moneris
    DummyConnector3
    Archipel
    Coingate
    Tsys
    Wellsfargo
    Bambora
    Iatapay
    DummyConnector7
    Dlocal
    Deutschebank
    Paybox
    Elavon
    Santander
    Square
    Xendit
    Fiservemea
    Payu
    Razorpay
    Opennode
    Payone
    Signifyd
    Worldline
    Shift4
    Adyen
    Gocardless
    Jpmorgan
    Noon
    Celero
    Zen
    Nexixpay
    Globalpay
    Braintree
    Mifinity
    Nomupay
    Powertranz
    DummyConnector4
    Adyenplatform
    Bamboraapac
    Digitalvirgo
    Hipay
    Helcim
    Nexinets
    Novalnet
    Prophetpay
    Payload
    Itaubank
    Stripebilling
    Mollie
    Multisafepay
    Paystack
    Getnet
    Stripe
    Cashtocode
    Cryptopay
    Fiuu
    DummyConnector2
    Bankofamerica
    Boku
    DummyConnector1
    Authipay
    Recurly
}

structure Address {
    /// Provide the address details
    address: AddressDetails
    phone: PhoneDetails
    email: smithy.api#String
}

/// Indicates the card network.
enum CardNetwork {
    Interac
    Nyce
    RuPay
    Mastercard
    AmericanExpress
    JCB
    DinersClub
    Star
    Accel
    Pulse
    Discover
    UnionPay
    CartesBancaires
    Visa
    Maestro
}

/// The three-letter ISO 4217 currency code (e.g., "USD", "EUR") for the payment amount. This field is mandatory for creating a payment.
enum Currency {
    KPW
    SOS
    GHS
    GBP
    CRC
    LRD
    LYD
    DKK
    HTG
    MDL
    SRD
    CDF
    MXN
    SSP
    AUD
    ANG
    UAH
    XPF
    COP
    KMF
    TRY
    SGD
    SYP
    CUP
    PEN
    PLN
    ALL
    ERN
    GIP
    MNT
    NGN
    SAR
    CHF
    SVC
    MMK
    VES
    CLF
    BWP
    GMD
    TTD
    BZD
    SLL
    KGS
    ETB
    CUC
    BYN
    TOP
    BHD
    BMD
    SHP
    ZWL
    SCR
    CNY
    LKR
    MAD
    UGX
    YER
    CAD
    AED
    HUF
    MGA
    STN
    MWK
    TZS
    MRU
    INR
    MUR
    XAF
    XOF
    ZMW
    EUR
    SBD
    TND
    VND
    BBD
    GTQ
    MOP
    KWD
    RON
    BAM
    AFN
    CVE
    DOP
    AZN
    NOK
    GYD
    HNL
    MZN
    NZD
    DJF
    MKD
    OMR
    PHP
    GEL
    RWF
    TJS
    BDT
    IQD
    RUB
    KES
    NIO
    TWD
    GNF
    TMT
    DZD
    BIF
    THB
    CZK
    JPY
    PAB
    RSD
    ILS
    SDG
    CLP
    AOA
    HRK
    JMD
    BND
    MVR
    LSL
    PGK
    SZL
    EGP
    LAK
    BGN
    PKR
    QAR
    SEK
    BSD
    SLE
    BRL
    UZS
    WST
    AMD
    STD
    ARS
    LBP
    MYR
    IRR
    BTN
    KHR
    NPR
    XCD
    KYD
    BOB
    VUV
    KZT
    ZAR
    FKP
    ISK
    PYG
    IDR
    UYU
    JOD
    HKD
    AWG
    KRW
    USD
    NAD
    FJD
}

/// Specifies how the payment method can be used for future payments. - `off_session`: The payment method can be used for future payments when the customer is not present. - `on_session`: The payment method is intended for use only when the customer is present during checkout. If omitted, defaults to `on_session`.
enum FutureUsage {
    OffSession
    OnSession
}

/// The payment method information provided for making a payment
structure PaymentMethodDataRequest {
    Card: Card
}

structure Card {
    /// The card's expiry month
    @required
    card_exp_month: smithy.api#String
    card_issuing_country: smithy.api#String
    bank_code: smithy.api#String
    /// The name of the issuer of card
    card_issuer: smithy.api#String
    /// The card holder's name
    card_holder_name: smithy.api#String
    /// The card's expiry year
    @required
    card_exp_year: smithy.api#String
    /// The card number
    @required
    card_number: smithy.api#String
    /// The CVC number for the card
    @required
    card_cvc: smithy.api#String
    /// The card holder's nick name
    nick_name: smithy.api#String
    /// The card network for the card
    card_network: CardNetwork
    card_type: smithy.api#String
}

union PaymentMethodData {
    Card: Card
}

enum CountryAlpha2 {
    RS
    SK
    TC
    NF
    BH
    OM
    TD
    GB
    BS
    IO
    MC
    CY
    SO
    AQ
    UA
    AU
    CU
    SJ
    TF
    CV
    SX
    YT
    VE
    LI
    KM
    CK
    SL
    GA
    LS
    CM
    LT
    IM
    BQ
    CD
    FR
    MU
    CR
    CO
    PN
    AE
    ME
    AX
    TM
    BM
    DE
    BE
    BR
    TZ
    UM
    NL
    PG
    BI
    ES
    CF
    CW
    UY
    NO
    KR
    FO
    GW
    NG
    EG
    MD
    NP
    KH
    BB
    SI
    RO
    BG
    DK
    LU
    AW
    MO
    NR
    VU
    EC
    IL
    MY
    AO
    GM
    TR
    MS
    SB
    SR
    KI
    MG
    SY
    CG
    BF
    MK
    VN
    IS
    TO
    JE
    PS
    BV
    JO
    IN
    MT
    ST
    EE
    DO
    TV
    ML
    MQ
    AD
    CI
    PR
    MP
    SA
    AF
    BN
    LB
    TH
    CH
    AR
    LV
    GQ
    ID
    ER
    KW
    KG
    MW
    BW
    MR
    LA
    US
    AL
    TT
    CN
    SN
    FK
    SG
    KP
    WS
    LR
    JP
    ZA
    IT
    IR
    TN
    BD
    VA
    NU
    PL
    PW
    TL
    SD
    FM
    RU
    SC
    PH
    CL
    NC
    LK
    UZ
    AG
    MM
    PE
    CC
    JM
    QA
    BO
    AT
    AS
    GT
    GY
    MN
    TJ
    MA
    KZ
    TG
    AI
    UG
    SZ
    FI
    GE
    GI
    BJ
    HT
    MZ
    NZ
    PM
    HR
    GG
    CX
    PT
    HU
    LY
    BY
    HM
    GF
    NI
    SS
    VG
    TW
    WF
    ZW
    EH
    MF
    PA
    CZ
    PK
    KY
    VC
    PF
    RE
    MH
    NA
    HN
    SE
    GL
    BA
    GN
    GR
    FJ
    BZ
    KE
    MV
    DZ
    HK
    GD
    IE
    SM
    CA
    PY
    DM
    KN
    VI
    SV
    BL
    IQ
    BT
    YE
    GS
    AZ
    GP
    MX
    SH
    DJ
    NE
    RW
    AM
    ZM
    ET
    TK
    LC
    GH
    GU
}

