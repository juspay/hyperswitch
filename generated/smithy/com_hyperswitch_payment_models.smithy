$version: "2"

namespace com.hyperswitch.payment.models

/// RoutableConnectors are the subset of Connectors that are eligible for payments routing
enum RoutableConnectors {
    Cryptopay
    Fiservemea
    Globepay
    Klarna
    Fiuu
    Iatapay
    Prophetpay
    Elavon
    Coingate
    Bankofamerica
    Jpmorgan
    Shift4
    Xendit
    Zen
    Paypal
    Mollie
    Stripebilling
    Powertranz
    Adyenplatform
    Mifinity
    Moneris
    Novalnet
    Riskified
    Wellsfargo
    Globalpay
    Wise
    Billwerk
    Nomupay
    Bluesnap
    Braintree
    Gocardless
    Coinbase
    Datatrans
    Forte
    Digitalvirgo
    Bambora
    Payone
    Signifyd
    Barclaycard
    Multisafepay
    Authipay
    Nmi
    Adyen
    Fiserv
    Nuvei
    Payu
    Worldline
    Worldpayvantiv
    Bamboraapac
    Redsys
    Stax
    Rapyd
    Tokenio
    Recurly
    Deutschebank
    Cybersource
    Cashtocode
    Razorpay
    Trustpay
    Worldpayxml
    Bitpay
    Dlocal
    Celero
    Opennode
    Stripe
    Helcim
    Authorizedotnet
    Chargebee
    Ebanx
    Inespay
    Itaubank
    Facilitapay
    Hipay
    Nexixpay
    Paybox
    Payload
    Santander
    Worldpay
    Checkout
    Square
    Plaid
    Boku
    Payme
    Paystack
    Archipel
    Aci
    Nexinets
    Noon
    Airwallex
    Zsl
    Placetopay
    Getnet
    Volt
    Tsys
}

enum Connector {
    CtpVisa
    Bambora
    Inespay
    Elavon
    Forte
    Globalpay
    Globepay
    Nexixpay
    Paybox
    Celero
    Redsys
    Santander
    Stripe
    Bitpay
    Cybersource
    Datatrans
    Mollie
    Moneris
    Wellsfargo
    Square
    Worldpayvantiv
    Trustpay
    Xendit
    Billwerk
    Chargebee
    Digitalvirgo
    Barclaycard
    Paypal
    Payme
    Rapyd
    Archipel
    Mifinity
    Prophetpay
    Stripebilling
    Taxjar
    Itaubank
    Tsys
    Vgs
    Wise
    Plaid
    Coingate
    Airwallex
    Facilitapay
    Gpayments
    Volt
    Coinbase
    Bamboraapac
    CtpMastercard
    Fiserv
    Payu
    Recurly
    Signifyd
    Zsl
    Adyen
    Checkout
    Worldpay
    Jpmorgan
    Stax
    Fiuu
    Threedsecureio
    Payload
    Opennode
    Klarna
    Novalnet
    Nexinets
    HyperswitchVault
    Gocardless
    Zen
    Worldpayxml
    Authorizedotnet
    Fiservemea
    Ebanx
    Aci
    Cashtocode
    Nomupay
    Netcetera
    Noon
    Tokenio
    Nuvei
    Worldline
    Riskified
    Paystack
    Placetopay
    Juspaythreedsserver
    Adyenplatform
    Cryptopay
    Bluesnap
    Getnet
    Hipay
    Payone
    Powertranz
    Dlocal
    Nmi
    Authipay
    Razorpay
    Shift4
    Deutschebank
    Braintree
    Multisafepay
    Boku
    Bankofamerica
    Helcim
    Iatapay
}

list ConnectorList {
    member: Connector
}

structure Address {
    /// Provide the address details
    address: AddressDetails
    email: smithy.api#String
    phone: PhoneDetails
}

structure PaymentsRequest {
    /// The customer's name. This field will be deprecated soon, use the customer object instead.
    name: smithy.api#String
    /// The three-letter ISO 4217 currency code (e.g., "USD", "EUR") for the payment amount. This field is mandatory for creating a payment.
    currency: Currency
    /// Indicates if the redirection has to open in the iframe
    is_iframe_redirection_enabled: smithy.api#Boolean
    /// The amount to be captured from the user's payment method, in the lowest denomination. If not provided, and `capture_method` is `automatic`, the full payment `amount` will be captured. If `capture_method` is `manual`, this can be specified in the `/capture` call. Must be less than or equal to the authorized amount.
    amount_to_capture: smithy.api#Long
    /// For non-card charges, you can use this value as the complete description that appears on your customers’ statements. Must contain at least one letter, maximum 22 characters.
    statement_descriptor_name: smithy.api#String
    /// Whether to generate the payment link for this payment or not (if applicable)
    payment_link: smithy.api#Boolean
    /// If set to `true`, Hyperswitch attempts to confirm and authorize the payment immediately after creation, provided sufficient payment method details are included. If `false` or omitted (default is `false`), the payment is created with a status such as `requires_payment_method` or `requires_confirmation`, and a separate `POST /payments/{payment_id}/confirm` call is necessary to proceed with authorization.
    confirm: smithy.api#Boolean
    /// The shipping cost for the payment. This is required for tax calculation in some regions.
    shipping_cost: smithy.api#Long
    /// As Hyperswitch tokenises the sensitive details about the payments method, it provides the payment_token as a reference to a stored payment method, ensuring that the sensitive details are not exposed in any manner.
    payment_token: smithy.api#String
    /// Set to true to indicate that the customer is not in your checkout flow during this payment, and therefore is unable to authenticate. This parameter is intended for scenarios where you collect card details and charge them later. When making a recurring payment by passing a mandate_id, this parameter is mandatory
    off_session: smithy.api#Boolean
    /// Business sub label for the payment
    business_sub_label: smithy.api#String
    /// A unique identifier to link the payment to a mandate. To do Recurring payments after a mandate has been created, pass the mandate_id instead of payment_method_data
    mandate_id: smithy.api#String
    /// Request an incremental authorization, i.e., increase the authorized amount on a confirmed payment before you capture it.
    request_incremental_authorization: smithy.api#Boolean
    /// Will be used to expire client secret after certain amount of time to be supplied in seconds (900) for 15 mins
    session_expiry: smithy.api#Integer
    /// Whether to perform external authentication (if applicable)
    request_external_three_ds_authentication: smithy.api#Boolean
    /// Custom payment link config id set at business profile, send only if business_specific_configs is configured
    payment_link_config_id: smithy.api#String
    /// This allows to manually select a connector with which the payment can go through.
    connector: ConnectorList
    /// An arbitrary string attached to the payment. Often useful for displaying to users or for your own internal record-keeping.
    description: smithy.api#String
    /// Your unique identifier for this payment or order. This ID helps you reconcile payments on your system. If provided, it is passed to the connector if supported.
    merchant_order_reference_id: smithy.api#String
    /// The customer's phone number This field will be deprecated soon, use the customer object instead
    phone: smithy.api#String
    /// The primary amount for the payment, provided in the lowest denomination of the specified currency (e.g., 6540 for $65.40 USD). This field is mandatory for creating a payment.
    amount: smithy.api#Long
    /// Optional. A merchant-provided unique identifier for the payment, contains 30 characters long (e.g., "pay_mbabizu24mvu3mela5njyhpit4"). If provided, it ensures idempotency for the payment creation request. If omitted, Hyperswitch generates a unique ID for the payment.
    payment_id: smithy.api#String
    /// The billing details of the payment. This address will be used for invoicing.
    billing: Address
    /// The customer's email address. This field will be deprecated soon, use the customer object instead
    email: smithy.api#String
    /// The URL to redirect the customer to after they complete the payment process or authentication. This is crucial for flows that involve off-site redirection (e.g., 3DS, some bank redirects, wallet payments).
    return_url: smithy.api#String
    /// Provides information about a card payment that customers see on their statements. Concatenated with the prefix (shortened descriptor) or statement descriptor that’s set on the account to form the complete statement descriptor. Maximum 22 characters for the concatenated descriptor.
    statement_descriptor_suffix: smithy.api#String
    /// It's a token used for client side verification.
    client_secret: smithy.api#String
    /// Business label of the merchant for this payment. To be deprecated soon. Pass the profile_id instead
    business_label: smithy.api#String
    /// Whether to calculate tax for this payment intent
    skip_external_tax_calculation: smithy.api#Boolean
    /// If enabled, provides whole connector response
    all_keys_required: smithy.api#Boolean
    /// This is used along with the payment_token field while collecting during saved card payments. This field will be deprecated soon, use the payment_method_data.card_token object instead
    card_cvc: smithy.api#String
    /// The business profile to be used for this payment, if not passed the default business profile associated with the merchant account will be used. It is mandatory in case multiple business profiles have been set up.
    profile_id: smithy.api#String
    /// Indicates if 3ds challenge is forced
    force_3ds_challenge: smithy.api#Boolean
    /// Total tax amount applicable to the order, in the lowest denomination of the currency.
    order_tax_amount: smithy.api#Long
    /// This is an identifier for the merchant account. This is inferred from the API key provided during the request
    merchant_id: smithy.api#String
    /// The identifier for the customer
    customer_id: smithy.api#String
    /// The country code for the customer phone number This field will be deprecated soon, use the customer object instead
    phone_country_code: smithy.api#String
}

enum CountryAlpha2 {
    PE
    BT
    CN
    BO
    KN
    ST
    IN
    CG
    GM
    CI
    SA
    SE
    TO
    BF
    NC
    CM
    MZ
    SN
    ID
    CW
    SV
    BW
    NZ
    LC
    TM
    AL
    CH
    VN
    IM
    IT
    AF
    CZ
    ZM
    TG
    AS
    GI
    GN
    FM
    NU
    AD
    CV
    CL
    SJ
    EC
    SG
    ZW
    GR
    NG
    CR
    GY
    PW
    HN
    CO
    PS
    DO
    MF
    DZ
    SZ
    SY
    AQ
    KH
    BV
    AT
    EG
    PY
    VC
    IL
    PR
    PT
    RU
    GS
    LV
    CF
    VE
    LT
    KI
    LU
    PH
    AU
    VU
    GA
    BG
    PA
    GG
    LA
    UG
    RS
    SM
    KE
    TN
    LY
    JO
    TV
    JE
    BM
    US
    CK
    GE
    TR
    KY
    KW
    LS
    GU
    SS
    GP
    AX
    WS
    SO
    AW
    PN
    GH
    MH
    MR
    TL
    GW
    MK
    SD
    DK
    BJ
    WF
    GT
    MC
    MP
    NA
    HT
    JM
    IE
    MD
    NO
    MG
    CD
    BL
    KG
    MN
    QA
    SX
    LR
    HM
    SK
    BB
    MU
    IO
    NR
    GF
    EE
    IS
    UM
    AE
    MO
    AI
    AR
    JP
    BY
    CU
    IR
    UA
    FI
    GL
    PG
    TT
    HK
    LK
    BR
    GB
    VI
    FK
    BE
    MQ
    TH
    EH
    ER
    NI
    FO
    KR
    GD
    UY
    BI
    MV
    BS
    YE
    SB
    TZ
    BZ
    NF
    SI
    TJ
    AZ
    MY
    AO
    VG
    PK
    VA
    MW
    BH
    MS
    DJ
    IQ
    ML
    HU
    PM
    NL
    CX
    TW
    UZ
    GQ
    PF
    FR
    LI
    KM
    MA
    NP
    SR
    FJ
    MT
    KZ
    MX
    DE
    KP
    ZA
    LB
    SC
    BN
    ES
    BQ
    YT
    HR
    RE
    AM
    RW
    SL
    PL
    ME
    TD
    CY
    BA
    ET
    NE
    BD
    MM
    CA
    TC
    TK
    RO
    TF
    OM
    AG
    CC
    DM
    SH
}

structure PhoneDetails {
    /// The country code attached to the number
    country_code: smithy.api#String
    /// The contact number
    number: smithy.api#String
}

/// Address details
structure AddressDetails {
    /// The two-letter ISO 3166-1 alpha-2 country code (e.g., US, GB).
    country: CountryAlpha2
    /// The first line of the street address or P.O. Box.
    line1: smithy.api#String
    /// The city, district, suburb, town, or village of the address.
    city: smithy.api#String
    /// The first name for the address
    first_name: smithy.api#String
    /// The second line of the street address or P.O. Box (e.g., apartment, suite, unit, or building).
    line2: smithy.api#String
    /// The last name for the address
    last_name: smithy.api#String
    /// The zip/postal code for the address
    zip: smithy.api#String
    /// The address state
    state: smithy.api#String
    /// The third line of the street address, if applicable.
    line3: smithy.api#String
}

/// The three-letter ISO 4217 currency code (e.g., "USD", "EUR") for the payment amount. This field is mandatory for creating a payment.
enum Currency {
    COP
    DJF
    KPW
    CDF
    DKK
    STD
    VUV
    AOA
    SOS
    XOF
    MKD
    AUD
    KWD
    SGD
    SRD
    IQD
    BAM
    GYD
    SVC
    BIF
    MOP
    SYP
    TRY
    VND
    THB
    LYD
    TJS
    CHF
    TWD
    WST
    HKD
    UGX
    LBP
    HUF
    JPY
    USD
    SDG
    ERN
    BGN
    JOD
    ZMW
    BOB
    GMD
    BWP
    SAR
    TND
    XCD
    KGS
    VES
    KES
    BYN
    LAK
    MWK
    EUR
    NIO
    GTQ
    ILS
    NGN
    LKR
    YER
    KHR
    MVR
    JMD
    BRL
    BDT
    ETB
    FKP
    TTD
    PAB
    ARS
    KYD
    NZD
    SZL
    TZS
    CLF
    CVE
    SSP
    AWG
    MYR
    BND
    EGP
    PLN
    GIP
    IDR
    GEL
    BMD
    NAD
    QAR
    RSD
    BSD
    RWF
    UZS
    DOP
    CLP
    MNT
    ISK
    NOK
    BBD
    BTN
    GHS
    CUC
    PKR
    NPR
    HTG
    KMF
    HRK
    IRR
    XAF
    MMK
    BZD
    PGK
    SCR
    INR
    CUP
    UAH
    ANG
    FJD
    PEN
    MGA
    DZD
    KRW
    SEK
    SLL
    STN
    UYU
    HNL
    CAD
    LRD
    MDL
    MZN
    RUB
    SHP
    SLE
    TMT
    AFN
    AMD
    MRU
    CRC
    CZK
    GNF
    ALL
    OMR
    PYG
    MAD
    PHP
    ZAR
    BHD
    AED
    AZN
    CNY
    KZT
    GBP
    SBD
    ZWL
    TOP
    XPF
    MXN
    RON
    LSL
    MUR
}

