$version: "2"

namespace com.hyperswitch.smithy.types

structure PhoneDetails {
    /// The country code attached to the number
    country_code: smithy.api#String
    /// The contact number
    number: smithy.api#String
}

/// Specifies how the payment is captured. - `automatic`: Funds are captured immediately after successful authorization. This is the default behavior if the field is omitted. - `manual`: Funds are authorized but not captured. A separate request to the `/payments/{payment_id}/capture` endpoint is required to capture the funds.
enum CaptureMethod {
    /// Handles separate auth and capture sequentially; effectively the same as `Automatic` for most connectors.
    SequentialAutomatic
    /// The capture will happen only if the merchant triggers a Capture API request. Allows for multiple partial captures up to the authorized amount.
    ManualMultiple
    /// Post the payment authorization, the capture will be executed on the full amount immediately.
    Automatic
    /// The capture will happen only if the merchant triggers a Capture API request. Allows for a single capture of the authorized amount.
    Manual
    /// The capture can be scheduled to automatically get triggered at a specific date & time.
    Scheduled
}

/// Address details
structure AddressDetails {
    /// The first line of the street address or P.O. Box.
    line1: smithy.api#String
    /// The zip/postal code for the address
    zip: smithy.api#String
    /// The last name for the address
    last_name: smithy.api#String
    /// The two-letter ISO 3166-1 alpha-2 country code (e.g., US, GB).
    country: CountryAlpha2
    /// The address state
    state: smithy.api#String
    /// The first name for the address
    first_name: smithy.api#String
    /// The city, district, suburb, town, or village of the address.
    city: smithy.api#String
    /// The second line of the street address or P.O. Box (e.g., apartment, suite, unit, or building).
    line2: smithy.api#String
    /// The third line of the street address, if applicable.
    line3: smithy.api#String
}

/// The three-letter ISO 4217 currency code (e.g., "USD", "EUR") for the payment amount. This field is mandatory for creating a payment.
enum Currency {
    BND
    BSD
    OMR
    PEN
    CRC
    CUC
    SDG
    COP
    SGD
    TJS
    TZS
    LSL
    BRL
    AWG
    MZN
    TWD
    USD
    VES
    BIF
    BTN
    CDF
    NOK
    UGX
    VND
    TRY
    JMD
    DJF
    KRW
    SCR
    XOF
    PAB
    AED
    GEL
    RON
    MUR
    SEK
    JOD
    LYD
    BOB
    YER
    GTQ
    SYP
    BAM
    BYN
    CUP
    MGA
    INR
    TND
    MVR
    ANG
    DKK
    NPR
    KHR
    XCD
    BMD
    RUB
    ALL
    HUF
    MMK
    KYD
    ZMW
    AZN
    UZS
    KGS
    NAD
    CZK
    BDT
    GNF
    STD
    AOA
    KZT
    PLN
    MAD
    ISK
    CVE
    DOP
    SOS
    PGK
    CNY
    GIP
    THB
    WST
    HKD
    AFN
    TTD
    MXN
    NZD
    NGN
    PYG
    GBP
    CLP
    MOP
    STN
    HNL
    TMT
    GHS
    TOP
    SLL
    LRD
    LKR
    KES
    MYR
    ZAR
    DZD
    MWK
    SAR
    FJD
    GYD
    SHP
    HTG
    KPW
    CAD
    MRU
    BWP
    ARS
    BZD
    ERN
    JPY
    VUV
    BHD
    EUR
    SSP
    AUD
    PHP
    CHF
    BGN
    SRD
    IDR
    IRR
    HRK
    MKD
    XPF
    XAF
    SBD
    KWD
    QAR
    SLE
    IQD
    LBP
    SVC
    PKR
    AMD
    ILS
    CLF
    MNT
    RWF
    UYU
    FKP
    GMD
    BBD
    RSD
    NIO
    KMF
    MDL
    UAH
    ETB
    LAK
    EGP
    SZL
    ZWL
}

enum CountryAlpha2 {
    CM
    CR
    CW
    KR
    MH
    TT
    KH
    UZ
    CN
    VI
    FR
    JO
    GT
    MK
    SJ
    BF
    MA
    NU
    BL
    PM
    EG
    GW
    JM
    GD
    SS
    RW
    UM
    IO
    DO
    AE
    FK
    VE
    BJ
    MN
    RU
    ZM
    HT
    BZ
    HN
    GE
    NC
    SD
    CU
    AL
    KZ
    YT
    GU
    BH
    BO
    GA
    SV
    SE
    ER
    LR
    BB
    TH
    DE
    HM
    VU
    AS
    FI
    SB
    KW
    AW
    CA
    GH
    SC
    SX
    KP
    LB
    KG
    HK
    MR
    PK
    AD
    NR
    TJ
    LT
    DZ
    YE
    ZW
    CY
    MW
    SY
    PY
    TN
    AX
    BQ
    BW
    BV
    ST
    PH
    FJ
    SR
    AG
    PG
    TZ
    MD
    ES
    TD
    JP
    IR
    IE
    NZ
    GQ
    SN
    SO
    SG
    AI
    GR
    SM
    MF
    DM
    MZ
    ME
    TG
    CL
    MP
    BG
    AF
    SI
    NL
    KE
    TK
    PF
    NI
    ET
    LK
    ZA
    UG
    NP
    IM
    TC
    BT
    BM
    VC
    GP
    JE
    BY
    TV
    AT
    VG
    PL
    BE
    WS
    SK
    VA
    DK
    FO
    SL
    UA
    GF
    LU
    OM
    SZ
    BD
    EH
    BN
    MY
    PW
    BA
    CX
    PS
    TF
    MV
    EC
    PR
    AU
    SH
    MT
    CG
    IT
    FM
    UY
    LA
    MM
    BS
    NE
    LY
    GL
    RS
    GY
    HU
    ML
    NA
    CZ
    MS
    CD
    AO
    CC
    AZ
    KM
    NO
    LC
    KN
    PA
    MC
    SA
    PN
    GS
    AR
    CH
    GB
    EE
    IL
    LI
    MQ
    NF
    TL
    PT
    TW
    RO
    LS
    CV
    RE
    QA
    GN
    KI
    CK
    GI
    IQ
    MO
    KY
    GG
    TM
    MG
    MU
    LV
    IS
    CI
    AM
    US
    CF
    ID
    IN
    PE
    WF
    AQ
    HR
    DJ
    CO
    MX
    TO
    BI
    GM
    TR
    VN
    NG
    BR
}

/// Specifies how the payment method can be used for future payments. - `off_session`: The payment method can be used for future payments when the customer is not present. - `on_session`: The payment method is intended for use only when the customer is present during checkout. If omitted, defaults to `on_session`.
enum FutureUsage {
    OffSession
    OnSession
}

/// Specifies the type of cardholder authentication to be applied for a payment.  - `ThreeDs`: Requests 3D Secure (3DS) authentication. If the card is enrolled, 3DS authentication will be activated, potentially shifting chargeback liability to the issuer. - `NoThreeDs`: Indicates that 3D Secure authentication should not be performed. The liability for chargebacks typically remains with the merchant. This is often the default if not specified.  Note: The actual authentication behavior can also be influenced by merchant configuration and specific connector defaults. Some connectors might still enforce 3DS or bypass it regardless of this parameter.
enum AuthenticationType {
    /// If the card is enrolled for 3DS authentication, the 3DS based authentication will be activated. The liability of chargeback shift to the issuer
    ThreeDs
    /// 3DS based authentication will not be activated. The liability of chargeback stays with the merchant.
    NoThreeDs
}

structure PaymentsRequest {
    /// Custom payment link config id set at business profile, send only if business_specific_configs is configured
    payment_link_config_id: smithy.api#String
    /// If enabled, provides whole connector response
    all_keys_required: smithy.api#Boolean
    /// The identifier for the customer
    customer_id: smithy.api#String
    /// Business sub label for the payment
    business_sub_label: smithy.api#String
    /// Whether to generate the payment link for this payment or not (if applicable)
    payment_link: smithy.api#Boolean
    /// The URL to redirect the customer to after they complete the payment process or authentication. This is crucial for flows that involve off-site redirection (e.g., 3DS, some bank redirects, wallet payments).
    return_url: smithy.api#String
    /// As Hyperswitch tokenises the sensitive details about the payments method, it provides the payment_token as a reference to a stored payment method, ensuring that the sensitive details are not exposed in any manner.
    payment_token: smithy.api#String
    authentication_type: AuthenticationType
    /// The country code for the customer phone number This field will be deprecated soon, use the customer object instead
    phone_country_code: smithy.api#String
    setup_future_usage: FutureUsage
    /// Indicates if 3ds challenge is forced
    force_3ds_challenge: smithy.api#Boolean
    /// Total tax amount applicable to the order, in the lowest denomination of the currency.
    order_tax_amount: smithy.api#Long
    /// Whether to calculate tax for this payment intent
    skip_external_tax_calculation: smithy.api#Boolean
    /// The business profile to be used for this payment, if not passed the default business profile associated with the merchant account will be used. It is mandatory in case multiple business profiles have been set up.
    profile_id: smithy.api#String
    /// The amount to be captured from the user's payment method, in the lowest denomination. If not provided, and `capture_method` is `automatic`, the full payment `amount` will be captured. If `capture_method` is `manual`, this can be specified in the `/capture` call. Must be less than or equal to the authorized amount.
    amount_to_capture: smithy.api#Long
    /// This allows to manually select a connector with which the payment can go through.
    connector: ConnectorList
    /// Indicates if the redirection has to open in the iframe
    is_iframe_redirection_enabled: smithy.api#Boolean
    /// This is an identifier for the merchant account. This is inferred from the API key provided during the request
    merchant_id: smithy.api#String
    /// An arbitrary string attached to the payment. Often useful for displaying to users or for your own internal record-keeping.
    description: smithy.api#String
    /// Will be used to expire client secret after certain amount of time to be supplied in seconds (900) for 15 mins
    session_expiry: smithy.api#Integer
    capture_method: CaptureMethod
    /// The primary amount for the payment, provided in the lowest denomination of the specified currency (e.g., 6540 for $65.40 USD). This field is mandatory for creating a payment.
    amount: smithy.api#Long
    /// Optional. A merchant-provided unique identifier for the payment, contains 30 characters long (e.g., "pay_mbabizu24mvu3mela5njyhpit4"). If provided, it ensures idempotency for the payment creation request. If omitted, Hyperswitch generates a unique ID for the payment.
    payment_id: smithy.api#String
    /// The shipping cost for the payment. This is required for tax calculation in some regions.
    shipping_cost: smithy.api#Long
    /// For non-card charges, you can use this value as the complete description that appears on your customers’ statements. Must contain at least one letter, maximum 22 characters.
    statement_descriptor_name: smithy.api#String
    /// The customer's phone number This field will be deprecated soon, use the customer object instead
    phone: smithy.api#String
    /// It's a token used for client side verification.
    client_secret: smithy.api#String
    /// Business label of the merchant for this payment. To be deprecated soon. Pass the profile_id instead
    business_label: smithy.api#String
    /// The three-letter ISO 4217 currency code (e.g., "USD", "EUR") for the payment amount. This field is mandatory for creating a payment.
    currency: Currency
    /// Provides information about a card payment that customers see on their statements. Concatenated with the prefix (shortened descriptor) or statement descriptor that’s set on the account to form the complete statement descriptor. Maximum 22 characters for the concatenated descriptor.
    statement_descriptor_suffix: smithy.api#String
    /// The billing details of the payment. This address will be used for invoicing.
    billing: Address
    /// This is used along with the payment_token field while collecting during saved card payments. This field will be deprecated soon, use the payment_method_data.card_token object instead
    card_cvc: smithy.api#String
    /// Whether to perform external authentication (if applicable)
    request_external_three_ds_authentication: smithy.api#Boolean
    /// Your unique identifier for this payment or order. This ID helps you reconcile payments on your system. If provided, it is passed to the connector if supported.
    merchant_order_reference_id: smithy.api#String
    /// The customer's name. This field will be deprecated soon, use the customer object instead.
    name: smithy.api#String
    /// The customer's email address. This field will be deprecated soon, use the customer object instead
    email: smithy.api#String
    /// Request an incremental authorization, i.e., increase the authorized amount on a confirmed payment before you capture it.
    request_incremental_authorization: smithy.api#Boolean
    /// Set to true to indicate that the customer is not in your checkout flow during this payment, and therefore is unable to authenticate. This parameter is intended for scenarios where you collect card details and charge them later. When making a recurring payment by passing a mandate_id, this parameter is mandatory
    off_session: smithy.api#Boolean
    /// If set to `true`, Hyperswitch attempts to confirm and authorize the payment immediately after creation, provided sufficient payment method details are included. If `false` or omitted (default is `false`), the payment is created with a status such as `requires_payment_method` or `requires_confirmation`, and a separate `POST /payments/{payment_id}/confirm` call is necessary to proceed with authorization.
    confirm: smithy.api#Boolean
    /// A unique identifier to link the payment to a mandate. To do Recurring payments after a mandate has been created, pass the mandate_id instead of payment_method_data
    mandate_id: smithy.api#String
}

list ConnectorList {
    member: Connector
}

enum Connector {
    Zsl
    Bluesnap
    HyperswitchVault
    Digitalvirgo
    Hipay
    Forte
    Nuvei
    Trustpay
    Celero
    Deutschebank
    Facilitapay
    Inespay
    Mifinity
    Adyenplatform
    Fiserv
    Mollie
    Cryptopay
    Coinbase
    Nexixpay
    Nmi
    Globalpay
    Noon
    Authorizedotnet
    Globepay
    Square
    Fiservemea
    Razorpay
    Volt
    Taxjar
    Bitpay
    CtpVisa
    Tsys
    Worldline
    Aci
    Bamboraapac
    Payme
    Payone
    Zen
    Netcetera
    Adyen
    Boku
    Cashtocode
    Checkout
    Jpmorgan
    Airwallex
    Coingate
    Payu
    Multisafepay
    Santander
    Stripe
    Chargebee
    Dlocal
    Nexinets
    CtpMastercard
    Ebanx
    Nomupay
    Billwerk
    Helcim
    Paypal
    Prophetpay
    Rapyd
    Stax
    Wellsfargo
    Itaubank
    Signifyd
    Novalnet
    Archipel
    Plaid
    Vgs
    Xendit
    Opennode
    Payload
    Worldpay
    Juspaythreedsserver
    Getnet
    Powertranz
    Bankofamerica
    Datatrans
    Elavon
    Fiuu
    Authipay
    Bambora
    Paystack
    Threedsecureio
    Barclaycard
    Gocardless
    Tokenio
    Riskified
    Placetopay
    Shift4
    Wise
    Recurly
    Worldpayvantiv
    Worldpayxml
    Gpayments
    Redsys
    Klarna
    Cybersource
    Paybox
    Stripebilling
    Moneris
    Braintree
    Iatapay
}

structure Address {
    /// Provide the address details
    address: AddressDetails
    phone: PhoneDetails
    email: smithy.api#String
}

/// RoutableConnectors are the subset of Connectors that are eligible for payments routing
enum RoutableConnectors {
    Zen
    Elavon
    Fiserv
    Airwallex
    Payload
    Fiservemea
    Moneris
    Coinbase
    Worldpayvantiv
    Forte
    Xendit
    Itaubank
    Payu
    Bitpay
    Dlocal
    Placetopay
    Powertranz
    Stripe
    Adyen
    Authipay
    Nmi
    Plaid
    Klarna
    Rapyd
    Helcim
    Santander
    Riskified
    Worldpay
    Worldline
    Paybox
    Noon
    Getnet
    Nuvei
    Paypal
    Shift4
    Digitalvirgo
    Worldpayxml
    Iatapay
    Bambora
    Cashtocode
    Bankofamerica
    Authorizedotnet
    Nomupay
    Nexinets
    Gocardless
    Aci
    Datatrans
    Coingate
    Stax
    Braintree
    Tokenio
    Trustpay
    Billwerk
    Razorpay
    Nexixpay
    Redsys
    Boku
    Checkout
    Hipay
    Opennode
    Novalnet
    Fiuu
    Paystack
    Bamboraapac
    Wellsfargo
    Cybersource
    Mifinity
    Payme
    Globalpay
    Archipel
    Cryptopay
    Prophetpay
    Bluesnap
    Facilitapay
    Deutschebank
    Chargebee
    Ebanx
    Globepay
    Adyenplatform
    Inespay
    Jpmorgan
    Barclaycard
    Mollie
    Multisafepay
    Payone
    Square
    Signifyd
    Stripebilling
    Tsys
    Volt
    Wise
    Zsl
    Celero
    Recurly
}

