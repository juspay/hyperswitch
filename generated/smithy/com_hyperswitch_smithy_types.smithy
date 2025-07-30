$version: "2"

namespace com.hyperswitch.smithy.types

/// Specifies how the payment method can be used for future payments. - `off_session`: The payment method can be used for future payments when the customer is not present. - `on_session`: The payment method is intended for use only when the customer is present during checkout. If omitted, defaults to `on_session`.
enum FutureUsage {
    OnSession
    OffSession
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
    Powertranz
    Redsys
    Gocardless
    Nuvei
    Paypal
    Airwallex
    Bluesnap
    Adyen
    Paystack
    Novalnet
    Worldline
    Dlocal
    Bamboraapac
    Prophetpay
    Cryptopay
    Fiuu
    Itaubank
    Riskified
    Trustpay
    Facilitapay
    Worldpayvantiv
    Mifinity
    Fiservemea
    Deutschebank
    Tsys
    Volt
    Aci
    Cashtocode
    Payu
    Coingate
    Digitalvirgo
    Stax
    Zen
    Billwerk
    Worldpay
    Multisafepay
    Opennode
    Fiserv
    Payload
    Nexinets
    Bankofamerica
    Barclaycard
    Recurly
    Forte
    Shift4
    Datatrans
    Ebanx
    Cybersource
    Elavon
    Moneris
    Chargebee
    Payme
    Razorpay
    Celero
    Helcim
    Mollie
    Stripe
    Stripebilling
    Tokenio
    Wise
    Worldpayxml
    Plaid
    Zsl
    Payone
    Coinbase
    Square
    Boku
    Inespay
    Globepay
    Santander
    Globalpay
    Nmi
    Rapyd
    Iatapay
    Signifyd
    Xendit
    Nexixpay
    Getnet
    Archipel
    Authorizedotnet
    Noon
    Hipay
    Nomupay
    Bitpay
    Adyenplatform
    Bambora
    Authipay
    Jpmorgan
    Paybox
    Checkout
    Placetopay
    Wellsfargo
    Klarna
    Braintree
}

/// Specifies how the payment is captured. - `automatic`: Funds are captured immediately after successful authorization. This is the default behavior if the field is omitted. - `manual`: Funds are authorized but not captured. A separate request to the `/payments/{payment_id}/capture` endpoint is required to capture the funds.
enum CaptureMethod {
    /// The capture will happen only if the merchant triggers a Capture API request. Allows for a single capture of the authorized amount.
    Manual
    /// Post the payment authorization, the capture will be executed on the full amount immediately.
    Automatic
    /// The capture can be scheduled to automatically get triggered at a specific date & time.
    Scheduled
    /// Handles separate auth and capture sequentially; effectively the same as `Automatic` for most connectors.
    SequentialAutomatic
    /// The capture will happen only if the merchant triggers a Capture API request. Allows for multiple partial captures up to the authorized amount.
    ManualMultiple
}

list ConnectorList {
    member: Connector
}

/// Address details
structure AddressDetails {
    /// The two-letter ISO 3166-1 alpha-2 country code (e.g., US, GB).
    country: CountryAlpha2
    /// The address state
    state: smithy.api#String
    /// The city, district, suburb, town, or village of the address.
    city: smithy.api#String
    /// The first name for the address
    first_name: smithy.api#String
    /// The second line of the street address or P.O. Box (e.g., apartment, suite, unit, or building).
    line2: smithy.api#String
    /// The first line of the street address or P.O. Box.
    line1: smithy.api#String
    /// The zip/postal code for the address
    zip: smithy.api#String
    /// The third line of the street address, if applicable.
    line3: smithy.api#String
    /// The last name for the address
    last_name: smithy.api#String
}

enum Connector {
    Inespay
    Airwallex
    Datatrans
    Prophetpay
    Volt
    Zen
    Aci
    Juspaythreedsserver
    Digitalvirgo
    Bluesnap
    Jpmorgan
    Nexixpay
    Noon
    Hipay
    Iatapay
    Itaubank
    Elavon
    Recurly
    Square
    Razorpay
    Plaid
    Gocardless
    Trustpay
    Authipay
    Braintree
    Zsl
    Forte
    Payone
    Novalnet
    Bamboraapac
    Getnet
    Signifyd
    Helcim
    Payme
    Wise
    Cashtocode
    Gpayments
    Paybox
    Billwerk
    Powertranz
    Klarna
    Facilitapay
    Moneris
    Nomupay
    Mollie
    Mifinity
    Coingate
    CtpVisa
    Globalpay
    Paystack
    Vgs
    Chargebee
    Tokenio
    Wellsfargo
    Barclaycard
    Cryptopay
    Riskified
    Xendit
    Nuvei
    Coinbase
    Santander
    Boku
    Worldpayxml
    Worldpay
    Worldpayvantiv
    CtpMastercard
    Adyenplatform
    Netcetera
    Stripe
    Globepay
    Taxjar
    HyperswitchVault
    Dlocal
    Rapyd
    Cybersource
    Checkout
    Celero
    Bitpay
    Redsys
    Bankofamerica
    Archipel
    Bambora
    Deutschebank
    Fiserv
    Opennode
    Stripebilling
    Authorizedotnet
    Fiservemea
    Payu
    Threedsecureio
    Worldline
    Placetopay
    Shift4
    Nexinets
    Adyen
    Nmi
    Ebanx
    Tsys
    Multisafepay
    Stax
    Payload
    Fiuu
    Paypal
}

/// The three-letter ISO 4217 currency code (e.g., "USD", "EUR") for the payment amount. This field is mandatory for creating a payment.
enum Currency {
    NIO
    SGD
    BSD
    XPF
    HNL
    TZS
    PLN
    SAR
    NPR
    GIP
    PKR
    KYD
    TRY
    IRR
    MUR
    USD
    WST
    UGX
    TJS
    RWF
    TMT
    AOA
    KZT
    CNY
    RSD
    BAM
    HUF
    STN
    VES
    GYD
    CDF
    EGP
    ISK
    INR
    ERN
    MWK
    MAD
    SLE
    BDT
    BZD
    DJF
    GNF
    LYD
    NAD
    ZMW
    MYR
    CHF
    JPY
    HKD
    DKK
    AUD
    AWG
    BRL
    CLP
    XOF
    AZN
    KES
    MDL
    SSP
    ARS
    COP
    LBP
    AED
    GBP
    MXN
    DZD
    IQD
    AFN
    SBD
    UAH
    UZS
    CUC
    JMD
    BBD
    THB
    AMD
    ANG
    PEN
    SDG
    TND
    SOS
    RUB
    SHP
    PYG
    UYU
    CVE
    CRC
    FKP
    NOK
    SZL
    BIF
    KGS
    SEK
    SYP
    VND
    BYN
    ZAR
    ILS
    TWD
    YER
    LSL
    BMD
    NZD
    LKR
    BND
    GHS
    LRD
    MRU
    DOP
    ETB
    KPW
    FJD
    KHR
    BWP
    BOB
    CLF
    PGK
    BGN
    MVR
    RON
    VUV
    SCR
    BHD
    IDR
    STD
    MZN
    ALL
    MMK
    MGA
    KMF
    MOP
    SRD
    OMR
    XAF
    GTQ
    SVC
    SLL
    CZK
    TOP
    XCD
    LAK
    NGN
    KWD
    KRW
    GEL
    EUR
    PAB
    HTG
    MKD
    MNT
    BTN
    ZWL
    CAD
    HRK
    GMD
    JOD
    PHP
    CUP
    QAR
    TTD
}

structure PaymentsRequest {
    /// The three-letter ISO 4217 currency code (e.g., "USD", "EUR") for the payment amount. This field is mandatory for creating a payment.
    currency: Currency
    /// The billing details of the payment. This address will be used for invoicing.
    billing: Address
    /// Set to true to indicate that the customer is not in your checkout flow during this payment, and therefore is unable to authenticate. This parameter is intended for scenarios where you collect card details and charge them later. When making a recurring payment by passing a mandate_id, this parameter is mandatory
    off_session: smithy.api#Boolean
    /// Whether to generate the payment link for this payment or not (if applicable)
    payment_link: smithy.api#Boolean
    /// The customer's phone number This field will be deprecated soon, use the customer object instead
    phone: smithy.api#String
    /// Request an incremental authorization, i.e., increase the authorized amount on a confirmed payment before you capture it.
    request_incremental_authorization: smithy.api#Boolean
    capture_method: CaptureMethod
    /// The primary amount for the payment, provided in the lowest denomination of the specified currency (e.g., 6540 for $65.40 USD). This field is mandatory for creating a payment.
    amount: smithy.api#Long
    /// Business sub label for the payment
    business_sub_label: smithy.api#String
    /// Your unique identifier for this payment or order. This ID helps you reconcile payments on your system. If provided, it is passed to the connector if supported.
    merchant_order_reference_id: smithy.api#String
    /// Indicates if the redirection has to open in the iframe
    is_iframe_redirection_enabled: smithy.api#Boolean
    /// The customer's email address. This field will be deprecated soon, use the customer object instead
    email: smithy.api#String
    /// Total tax amount applicable to the order, in the lowest denomination of the currency.
    order_tax_amount: smithy.api#Long
    /// The URL to redirect the customer to after they complete the payment process or authentication. This is crucial for flows that involve off-site redirection (e.g., 3DS, some bank redirects, wallet payments).
    return_url: smithy.api#String
    /// For non-card charges, you can use this value as the complete description that appears on your customers’ statements. Must contain at least one letter, maximum 22 characters.
    statement_descriptor_name: smithy.api#String
    /// If set to `true`, Hyperswitch attempts to confirm and authorize the payment immediately after creation, provided sufficient payment method details are included. If `false` or omitted (default is `false`), the payment is created with a status such as `requires_payment_method` or `requires_confirmation`, and a separate `POST /payments/{payment_id}/confirm` call is necessary to proceed with authorization.
    confirm: smithy.api#Boolean
    /// The identifier for the customer
    customer_id: smithy.api#String
    /// An arbitrary string attached to the payment. Often useful for displaying to users or for your own internal record-keeping.
    description: smithy.api#String
    /// This allows to manually select a connector with which the payment can go through.
    connector: ConnectorList
    authentication_type: AuthenticationType
    /// Whether to calculate tax for this payment intent
    skip_external_tax_calculation: smithy.api#Boolean
    setup_future_usage: FutureUsage
    /// Business label of the merchant for this payment. To be deprecated soon. Pass the profile_id instead
    business_label: smithy.api#String
    /// Provides information about a card payment that customers see on their statements. Concatenated with the prefix (shortened descriptor) or statement descriptor that’s set on the account to form the complete statement descriptor. Maximum 22 characters for the concatenated descriptor.
    statement_descriptor_suffix: smithy.api#String
    /// Optional. A merchant-provided unique identifier for the payment, contains 30 characters long (e.g., "pay_mbabizu24mvu3mela5njyhpit4"). If provided, it ensures idempotency for the payment creation request. If omitted, Hyperswitch generates a unique ID for the payment.
    payment_id: smithy.api#String
    /// The shipping cost for the payment. This is required for tax calculation in some regions.
    shipping_cost: smithy.api#Long
    /// This is used along with the payment_token field while collecting during saved card payments. This field will be deprecated soon, use the payment_method_data.card_token object instead
    card_cvc: smithy.api#String
    /// The business profile to be used for this payment, if not passed the default business profile associated with the merchant account will be used. It is mandatory in case multiple business profiles have been set up.
    profile_id: smithy.api#String
    /// Custom payment link config id set at business profile, send only if business_specific_configs is configured
    payment_link_config_id: smithy.api#String
    /// The customer's name. This field will be deprecated soon, use the customer object instead.
    name: smithy.api#String
    /// As Hyperswitch tokenises the sensitive details about the payments method, it provides the payment_token as a reference to a stored payment method, ensuring that the sensitive details are not exposed in any manner.
    payment_token: smithy.api#String
    /// Will be used to expire client secret after certain amount of time to be supplied in seconds (900) for 15 mins
    session_expiry: smithy.api#Integer
    /// If enabled, provides whole connector response
    all_keys_required: smithy.api#Boolean
    /// A unique identifier to link the payment to a mandate. To do Recurring payments after a mandate has been created, pass the mandate_id instead of payment_method_data
    mandate_id: smithy.api#String
    /// The amount to be captured from the user's payment method, in the lowest denomination. If not provided, and `capture_method` is `automatic`, the full payment `amount` will be captured. If `capture_method` is `manual`, this can be specified in the `/capture` call. Must be less than or equal to the authorized amount.
    amount_to_capture: smithy.api#Long
    /// This is an identifier for the merchant account. This is inferred from the API key provided during the request
    merchant_id: smithy.api#String
    /// Indicates if 3ds challenge is forced
    force_3ds_challenge: smithy.api#Boolean
    /// The country code for the customer phone number This field will be deprecated soon, use the customer object instead
    phone_country_code: smithy.api#String
    /// Whether to perform external authentication (if applicable)
    request_external_three_ds_authentication: smithy.api#Boolean
    /// It's a token used for client side verification.
    client_secret: smithy.api#String
}

structure PhoneDetails {
    /// The country code attached to the number
    country_code: smithy.api#String
    /// The contact number
    number: smithy.api#String
}

enum CountryAlpha2 {
    CZ
    LA
    TW
    MX
    SI
    VG
    GP
    BY
    KZ
    LT
    FO
    CV
    CD
    KR
    BM
    TG
    IN
    PN
    GW
    HM
    MH
    QA
    AG
    BO
    NU
    SX
    TZ
    ID
    MU
    DJ
    HK
    KE
    SN
    TN
    PM
    MG
    AE
    NG
    TR
    LR
    SJ
    GY
    TH
    CI
    CA
    KY
    BG
    ES
    AI
    SE
    UM
    CM
    SK
    OM
    JP
    LV
    ET
    RW
    AM
    BW
    CH
    RU
    SY
    GL
    IM
    PF
    NI
    BL
    GH
    NA
    MT
    CO
    SV
    TM
    ZM
    KI
    HT
    MF
    EC
    BE
    KM
    GF
    GA
    DE
    ML
    LS
    NE
    US
    JO
    VU
    GI
    AF
    IO
    JM
    FM
    EG
    UY
    LY
    MV
    AD
    NC
    KW
    TJ
    AW
    BH
    CN
    ZW
    BS
    GB
    SB
    ME
    BV
    IT
    KP
    LU
    TC
    GN
    SL
    AU
    PA
    BR
    NP
    GU
    CF
    MR
    BJ
    CW
    CY
    HR
    MQ
    MD
    TL
    BF
    PH
    TK
    GR
    BZ
    IS
    UZ
    TV
    UA
    HN
    AS
    MA
    IQ
    FR
    AQ
    GM
    PK
    VN
    LC
    IE
    NO
    IL
    TD
    AR
    LB
    YE
    GS
    MW
    CX
    GQ
    WS
    JE
    PE
    SS
    TF
    BN
    KG
    YT
    CK
    SO
    BB
    SA
    MP
    AX
    BI
    CR
    PW
    DM
    RE
    EE
    DK
    RS
    TO
    FJ
    MZ
    SR
    GG
    KN
    WF
    MN
    PL
    BQ
    SD
    SZ
    BT
    MM
    LI
    MK
    GE
    NL
    AL
    NR
    FK
    MO
    SM
    VI
    LK
    KH
    SC
    CC
    CU
    ER
    IR
    HU
    MS
    MC
    NF
    AO
    GD
    PY
    BA
    ZA
    TT
    BD
    DO
    NZ
    EH
    AT
    AZ
    PT
    GT
    VC
    FI
    PG
    PS
    PR
    RO
    SH
    SG
    CL
    MY
    VE
    DZ
    ST
    CG
    UG
    VA
}

structure Address {
    /// Provide the address details
    address: AddressDetails
    phone: PhoneDetails
    email: smithy.api#String
}

