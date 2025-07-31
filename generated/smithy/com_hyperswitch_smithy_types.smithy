$version: "2"

namespace com.hyperswitch.smithy.types

/// Represents the overall status of a payment intent. The status transitions through various states depending on the payment method, confirmation, capture method, and any subsequent actions (like customer authentication or manual capture).
enum IntentStatus {
    /// This payment is still being processed by the payment processor. The status update might happen through webhooks or polling with the connector.
    Processing
    /// There has been a discrepancy between the amount/currency sent in the request and the amount/currency received by the processor
    Conflicted
    /// The payment has succeeded. Refunds and disputes can be initiated. Manual retries are not allowed to be performed.
    Succeeded
    /// The payment is waiting on some action from the customer.
    RequiresCustomerAction
    /// The payment has been captured partially and the remaining amount is capturable
    PartiallyCapturedAndCapturable
    /// The payment is waiting to be confirmed with the payment method by the customer.
    RequiresPaymentMethod
    /// This payment has been cancelled.
    Cancelled
    /// The payment has failed. Refunds and disputes cannot be initiated. This payment can be retried manually with a new payment attempt.
    Failed
    /// The payment has been authorized, and it waiting to be captured.
    RequiresCapture
    RequiresConfirmation
    /// The payment is waiting on some action from the merchant This would be in case of manual fraud approval
    RequiresMerchantAction
    /// The payment has been captured partially. The remaining amount is cannot be captured.
    PartiallyCaptured
}

/// Indicates the type of payment method. Eg: 'card', 'wallet', etc.
enum PaymentMethod {
    RealTimePayment
    Crypto
    OpenBanking
    BankRedirect
    GiftCard
    BankTransfer
    Reward
    Voucher
    Card
    MobilePayment
    PayLater
    CardRedirect
    Upi
    BankDebit
    Wallet
}

/// Address details
structure AddressDetails {
    /// The second line of the street address or P.O. Box (e.g., apartment, suite, unit, or building).
    line2: smithy.api#String
    /// The zip/postal code for the address
    zip: smithy.api#String
    /// The third line of the street address, if applicable.
    line3: smithy.api#String
    /// The address state
    state: smithy.api#String
    /// The last name for the address
    last_name: smithy.api#String
    /// The city, district, suburb, town, or village of the address.
    city: smithy.api#String
    /// The first line of the street address or P.O. Box.
    line1: smithy.api#String
    /// The two-letter ISO 3166-1 alpha-2 country code (e.g., US, GB).
    country: CountryAlpha2
    /// The first name for the address
    first_name: smithy.api#String
}

/// Indicates the card network.
enum CardNetwork {
    Maestro
    JCB
    Interac
    Visa
    DinersClub
    UnionPay
    CartesBancaires
    RuPay
    Accel
    Mastercard
    Discover
    Nyce
    Star
    Pulse
    AmericanExpress
}

/// The payment method information provided for making a payment
structure PaymentMethodDataRequest {
    Card: Card
}

union PaymentMethodData {
    Card: Card
}

structure PaymentsRequest {
    capture_method: CaptureMethod
    /// A unique identifier to link the payment to a mandate. To do Recurring payments after a mandate has been created, pass the mandate_id instead of payment_method_data
    mandate_id: smithy.api#String
    /// Will be used to expire client secret after certain amount of time to be supplied in seconds (900) for 15 mins
    session_expiry: smithy.api#Integer
    /// Indicates if the redirection has to open in the iframe
    is_iframe_redirection_enabled: smithy.api#Boolean
    /// The primary amount for the payment, provided in the lowest denomination of the specified currency (e.g., 6540 for $65.40 USD). This field is mandatory for creating a payment.
    amount: smithy.api#Long
    /// Whether to perform external authentication (if applicable)
    request_external_three_ds_authentication: smithy.api#Boolean
    /// The URL to redirect the customer to after they complete the payment process or authentication. This is crucial for flows that involve off-site redirection (e.g., 3DS, some bank redirects, wallet payments).
    return_url: smithy.api#String
    /// Whether to generate the payment link for this payment or not (if applicable)
    payment_link: smithy.api#Boolean
    /// Business sub label for the payment
    business_sub_label: smithy.api#String
    /// The business profile to be used for this payment, if not passed the default business profile associated with the merchant account will be used. It is mandatory in case multiple business profiles have been set up.
    profile_id: smithy.api#String
    /// The identifier for the customer
    customer_id: smithy.api#String
    /// Optional. A merchant-provided unique identifier for the payment, contains 30 characters long (e.g., "pay_mbabizu24mvu3mela5njyhpit4"). If provided, it ensures idempotency for the payment creation request. If omitted, Hyperswitch generates a unique ID for the payment.
    payment_id: smithy.api#String
    setup_future_usage: FutureUsage
    /// Use this object to capture the details about the different products for which the payment is being made. The sum of amount across different products here should be equal to the overall payment amount
    order_details: OrderDetailsWithAmount
    /// Custom payment link config id set at business profile, send only if business_specific_configs is configured
    payment_link_config_id: smithy.api#String
    /// Your unique identifier for this payment or order. This ID helps you reconcile payments on your system. If provided, it is passed to the connector if supported.
    merchant_order_reference_id: smithy.api#String
    /// Total tax amount applicable to the order, in the lowest denomination of the currency.
    order_tax_amount: smithy.api#Long
    /// The billing details of the payment. This address will be used for invoicing.
    billing: Address
    /// The shipping cost for the payment. This is required for tax calculation in some regions.
    shipping_cost: smithy.api#Long
    /// The three-letter ISO 4217 currency code (e.g., "USD", "EUR") for the payment amount. This field is mandatory for creating a payment.
    currency: Currency
    /// As Hyperswitch tokenises the sensitive details about the payments method, it provides the payment_token as a reference to a stored payment method, ensuring that the sensitive details are not exposed in any manner.
    payment_token: smithy.api#String
    /// For non-card charges, you can use this value as the complete description that appears on your customers’ statements. Must contain at least one letter, maximum 22 characters.
    statement_descriptor_name: smithy.api#String
    /// It's a token used for client side verification.
    client_secret: smithy.api#String
    /// Provides information about a card payment that customers see on their statements. Concatenated with the prefix (shortened descriptor) or statement descriptor that’s set on the account to form the complete statement descriptor. Maximum 22 characters for the concatenated descriptor.
    statement_descriptor_suffix: smithy.api#String
    /// If enabled, provides whole connector response
    all_keys_required: smithy.api#Boolean
    /// Set to true to indicate that the customer is not in your checkout flow during this payment, and therefore is unable to authenticate. This parameter is intended for scenarios where you collect card details and charge them later. When making a recurring payment by passing a mandate_id, this parameter is mandatory
    off_session: smithy.api#Boolean
    /// The country code for the customer phone number This field will be deprecated soon, use the customer object instead
    phone_country_code: smithy.api#String
    /// An arbitrary string attached to the payment. Often useful for displaying to users or for your own internal record-keeping.
    description: smithy.api#String
    payment_method: PaymentMethod
    /// The shipping address for the payment
    shipping: Address
    /// Business label of the merchant for this payment. To be deprecated soon. Pass the profile_id instead
    business_label: smithy.api#String
    /// Business country of the merchant for this payment. To be deprecated soon. Pass the profile_id instead
    business_country: CountryAlpha2
    /// This allows to manually select a connector with which the payment can go through.
    connector: ConnectorList
    /// Whether to calculate tax for this payment intent
    skip_external_tax_calculation: smithy.api#Boolean
    /// This is an identifier for the merchant account. This is inferred from the API key provided during the request
    merchant_id: smithy.api#String
    authentication_type: AuthenticationType
    /// This is used along with the payment_token field while collecting during saved card payments. This field will be deprecated soon, use the payment_method_data.card_token object instead
    card_cvc: smithy.api#String
    /// If set to `true`, Hyperswitch attempts to confirm and authorize the payment immediately after creation, provided sufficient payment method details are included. If `false` or omitted (default is `false`), the payment is created with a status such as `requires_payment_method` or `requires_confirmation`, and a separate `POST /payments/{payment_id}/confirm` call is necessary to proceed with authorization.
    confirm: smithy.api#Boolean
    payment_method_data: PaymentMethodDataRequest
    /// The customer's email address. This field will be deprecated soon, use the customer object instead
    email: smithy.api#String
    /// The amount to be captured from the user's payment method, in the lowest denomination. If not provided, and `capture_method` is `automatic`, the full payment `amount` will be captured. If `capture_method` is `manual`, this can be specified in the `/capture` call. Must be less than or equal to the authorized amount.
    amount_to_capture: smithy.api#Long
    /// The customer's phone number This field will be deprecated soon, use the customer object instead
    phone: smithy.api#String
    /// Indicates if 3ds challenge is forced
    force_3ds_challenge: smithy.api#Boolean
    /// Request an incremental authorization, i.e., increase the authorized amount on a confirmed payment before you capture it.
    request_incremental_authorization: smithy.api#Boolean
    /// The customer's name. This field will be deprecated soon, use the customer object instead.
    name: smithy.api#String
}

list ConnectorList {
    member: Connector
}

/// Specifies how the payment is captured. - `automatic`: Funds are captured immediately after successful authorization. This is the default behavior if the field is omitted. - `manual`: Funds are authorized but not captured. A separate request to the `/payments/{payment_id}/capture` endpoint is required to capture the funds.
enum CaptureMethod {
    /// Post the payment authorization, the capture will be executed on the full amount immediately.
    Automatic
    /// The capture will happen only if the merchant triggers a Capture API request. Allows for a single capture of the authorized amount.
    Manual
    /// The capture will happen only if the merchant triggers a Capture API request. Allows for multiple partial captures up to the authorized amount.
    ManualMultiple
    /// The capture can be scheduled to automatically get triggered at a specific date & time.
    Scheduled
    /// Handles separate auth and capture sequentially; effectively the same as `Automatic` for most connectors.
    SequentialAutomatic
}

enum Connector {
    Riskified
    Tokenio
    Xendit
    Santander
    Archipel
    Bambora
    Getnet
    Dlocal
    Globalpay
    Gocardless
    Nexinets
    Nuvei
    Taxjar
    Worldline
    Adyen
    Fiuu
    Deutschebank
    Authipay
    HyperswitchVault
    Hipay
    Nmi
    Noon
    Inespay
    Checkout
    Juspaythreedsserver
    Bankofamerica
    Bamboraapac
    Fiserv
    Stax
    Volt
    Cybersource
    Opennode
    Worldpayvantiv
    Redsys
    Zen
    Nexixpay
    Payme
    Plaid
    Worldpay
    Datatrans
    Coingate
    Klarna
    Stripebilling
    Worldpayxml
    Mollie
    Mifinity
    Billwerk
    Cryptopay
    Facilitapay
    Bitpay
    Helcim
    Fiservemea
    Forte
    Netcetera
    Ebanx
    Stripe
    Chargebee
    Cashtocode
    Itaubank
    Authorizedotnet
    Boku
    Payone
    Prophetpay
    CtpMastercard
    Rapyd
    Airwallex
    Elavon
    Tsys
    Powertranz
    Globepay
    Shift4
    Vgs
    Zsl
    Jpmorgan
    Coinbase
    Wellsfargo
    Moneris
    Barclaycard
    Celero
    CtpVisa
    Paystack
    Placetopay
    Nomupay
    Square
    Threedsecureio
    Payload
    Trustpay
    Iatapay
    Multisafepay
    Novalnet
    Paybox
    Aci
    Bluesnap
    Braintree
    Gpayments
    Payu
    Digitalvirgo
    Recurly
    Adyenplatform
    Razorpay
    Signifyd
    Paypal
    Wise
}

/// Specifies how the payment method can be used for future payments. - `off_session`: The payment method can be used for future payments when the customer is not present. - `on_session`: The payment method is intended for use only when the customer is present during checkout. If omitted, defaults to `on_session`.
enum FutureUsage {
    OnSession
    OffSession
}

/// RoutableConnectors are the subset of Connectors that are eligible for payments routing
enum RoutableConnectors {
    Powertranz
    Globepay
    Bluesnap
    Volt
    Chargebee
    Riskified
    Bankofamerica
    Checkout
    Worldpayxml
    Getnet
    Iatapay
    Worldline
    Moneris
    Razorpay
    Recurly
    Fiservemea
    Stax
    Authipay
    Worldpay
    Multisafepay
    Plaid
    Payload
    Paystack
    Paypal
    Coingate
    Digitalvirgo
    Rapyd
    Cashtocode
    Cybersource
    Payu
    Adyen
    Dlocal
    Bambora
    Inespay
    Wellsfargo
    Bitpay
    Celero
    Mollie
    Payme
    Zen
    Nmi
    Payone
    Airwallex
    Stripebilling
    Wise
    Bamboraapac
    Globalpay
    Xendit
    Datatrans
    Noon
    Braintree
    Redsys
    Forte
    Nuvei
    Opennode
    Prophetpay
    Stripe
    Elavon
    Nomupay
    Zsl
    Coinbase
    Jpmorgan
    Santander
    Itaubank
    Ebanx
    Aci
    Worldpayvantiv
    Fiserv
    Mifinity
    Billwerk
    Boku
    Authorizedotnet
    Helcim
    Trustpay
    Novalnet
    Tsys
    Archipel
    Facilitapay
    Hipay
    Nexinets
    Adyenplatform
    Tokenio
    Shift4
    Barclaycard
    Deutschebank
    Nexixpay
    Cryptopay
    Signifyd
    Fiuu
    Paybox
    Placetopay
    Klarna
    Gocardless
    Square
}

structure Card {
    /// The card's expiry month
    @required
    card_exp_month: smithy.api#String
    /// The card holder's name
    card_holder_name: smithy.api#String
    /// The name of the issuer of card
    card_issuer: smithy.api#String
    card_issuing_country: smithy.api#String
    /// The card holder's nick name
    nick_name: smithy.api#String
    /// The card network for the card
    card_network: CardNetwork
    /// The card's expiry year
    @required
    card_exp_year: smithy.api#String
    bank_code: smithy.api#String
    /// The CVC number for the card
    @required
    card_cvc: smithy.api#String
    card_type: smithy.api#String
    /// The card number
    @required
    card_number: smithy.api#String
}

/// Specifies the type of cardholder authentication to be applied for a payment.  - `ThreeDs`: Requests 3D Secure (3DS) authentication. If the card is enrolled, 3DS authentication will be activated, potentially shifting chargeback liability to the issuer. - `NoThreeDs`: Indicates that 3D Secure authentication should not be performed. The liability for chargebacks typically remains with the merchant. This is often the default if not specified.  Note: The actual authentication behavior can also be influenced by merchant configuration and specific connector defaults. Some connectors might still enforce 3DS or bypass it regardless of this parameter.
enum AuthenticationType {
    /// If the card is enrolled for 3DS authentication, the 3DS based authentication will be activated. The liability of chargeback shift to the issuer
    ThreeDs
    /// 3DS based authentication will not be activated. The liability of chargeback stays with the merchant.
    NoThreeDs
}

structure PhoneDetails {
    /// The country code attached to the number
    country_code: smithy.api#String
    /// The contact number
    number: smithy.api#String
}

structure Address {
    /// Provide the address details
    address: AddressDetails
    phone: PhoneDetails
    email: smithy.api#String
}

enum CountryAlpha2 {
    MD
    AW
    SJ
    IQ
    AL
    KP
    MG
    GH
    US
    LB
    ML
    LA
    VU
    NP
    AD
    EG
    BR
    SG
    MA
    PK
    TG
    AZ
    DM
    BO
    SC
    EE
    RS
    BV
    NO
    LR
    WS
    LT
    GW
    BL
    KY
    CV
    CZ
    GU
    GG
    SK
    UZ
    LV
    JM
    JE
    SZ
    SH
    LU
    NI
    AF
    JO
    TF
    TH
    SO
    TJ
    MZ
    YE
    PA
    CL
    IL
    KG
    MO
    RW
    HR
    ET
    MY
    CC
    CX
    ZM
    MR
    NR
    CI
    GR
    LK
    BA
    DO
    TN
    AR
    SS
    CN
    BD
    PL
    UG
    BJ
    BB
    GB
    CG
    TZ
    TC
    AQ
    CU
    BN
    ZW
    FO
    BZ
    HM
    BE
    FK
    LC
    GL
    BI
    AG
    TL
    LS
    PE
    CO
    CF
    CK
    ID
    PW
    AX
    GM
    VI
    SB
    HT
    RU
    FM
    TV
    JP
    PH
    PT
    CH
    VN
    AO
    SI
    IM
    AT
    FR
    MQ
    NF
    PG
    NU
    AU
    TD
    IS
    SN
    OM
    GP
    BG
    CM
    CA
    EH
    NL
    GS
    ZA
    VA
    KH
    MN
    NZ
    KI
    CD
    GE
    MP
    IT
    AS
    WF
    MF
    PS
    EC
    BY
    BW
    MS
    PN
    PM
    HN
    SD
    PY
    FJ
    TT
    LY
    CY
    MV
    MU
    UA
    IN
    SY
    TO
    MT
    SE
    BT
    DK
    IE
    GQ
    NG
    AM
    PR
    LI
    MC
    UM
    KZ
    MW
    GI
    SL
    RO
    KN
    AI
    GD
    YT
    BM
    TW
    GN
    BH
    TR
    KR
    SM
    RE
    ER
    BQ
    KE
    AE
    SX
    VC
    NA
    GY
    DE
    PF
    GT
    IO
    CW
    SA
    FI
    UY
    NE
    ST
    IR
    QA
    SV
    BS
    ES
    TM
    SR
    GA
    VG
    BF
    KM
    KW
    GF
    ME
    CR
    HU
    HK
    MH
    MK
    MM
    MX
    DZ
    VE
    DJ
    TK
    NC
}

structure PaymentsResponse {
    /// The total amount (in minor units) that has been captured for this payment. For `fauxpay` sandbox connector, this might reflect the authorized amount if `status` is `succeeded` even if `capture_method` was `manual`.
    amount_received: smithy.api#Long
    /// The name of the payment connector (e.g., 'stripe', 'adyen') that processed or is processing this payment.
    connector: smithy.api#String
    @required
    status: IntentStatus
    /// The payment net amount. net_amount = amount + surcharge_details.surcharge_amount + surcharge_details.tax_amount + shipping_cost + order_tax_amount, If no surcharge_details, shipping_cost, order_tax_amount, net_amount = amount
    @required
    net_amount: smithy.api#Long
    /// Unique identifier for the payment. This ensures idempotency for multiple payments that have been done by a single merchant.
    @required
    payment_id: smithy.api#String
    /// A secret token unique to this payment intent. It is primarily used by client-side applications (e.g., Hyperswitch SDKs) to authenticate actions like confirming the payment or handling next actions. This secret should be handled carefully and not exposed publicly beyond its intended client-side use.
    client_secret: smithy.api#String
    /// The amount (in minor units) that can still be captured for this payment. This is relevant when `capture_method` is `manual`. Once fully captured, or if `capture_method` is `automatic` and payment succeeded, this will be 0.
    @required
    amount_capturable: smithy.api#Long
    /// The payment amount. Amount for the payment in lowest denomination of the currency. (i.e) in cents for USD denomination, in paisa for INR denomination etc.,
    @required
    amount: smithy.api#Long
    /// This is an identifier for the merchant account. This is inferred from the API key provided during the request
    @required
    merchant_id: smithy.api#String
    /// The shipping cost for the payment.
    shipping_cost: smithy.api#Long
}

structure OrderDetailsWithAmount {
    /// The image URL of the product
    product_img_link: smithy.api#String
    /// The tax code for the product
    product_tax_code: smithy.api#String
    /// the amount per quantity of product
    @required
    amount: smithy.api#Long
    /// tax rate applicable to the product
    tax_rate: smithy.api#Double
    /// Category of the product that is being purchased
    category: smithy.api#String
    requires_shipping: smithy.api#Boolean
    /// Sub category of the product that is being purchased
    sub_category: smithy.api#String
    /// Brand of the product that is being purchased
    brand: smithy.api#String
    /// total tax amount applicable to the product
    total_tax_amount: smithy.api#Long
    /// Name of the product that is being purchased
    @required
    product_name: smithy.api#String
    /// The quantity of the product to be purchased
    @required
    quantity: smithy.api#Integer
    /// ID of the product that is being purchased
    product_id: smithy.api#String
}

/// The three-letter ISO 4217 currency code (e.g., "USD", "EUR") for the payment amount. This field is mandatory for creating a payment.
enum Currency {
    GTQ
    GIP
    RSD
    GMD
    CAD
    MXN
    RWF
    SCR
    STN
    TWD
    CUP
    IDR
    UGX
    DOP
    UZS
    YER
    LKR
    RUB
    IQD
    GYD
    AWG
    BBD
    ERN
    SSP
    DJF
    JOD
    KHR
    KZT
    MWK
    SOS
    ZWL
    HKD
    AUD
    NIO
    AZN
    UYU
    JPY
    LAK
    IRR
    CRC
    SYP
    BAM
    MRU
    EUR
    INR
    DKK
    MGA
    ISK
    MNT
    SLL
    ARS
    LYD
    BGN
    CVE
    TMT
    UAH
    XAF
    BND
    KRW
    MZN
    VND
    BDT
    CUC
    PKR
    ETB
    RON
    VUV
    NZD
    AMD
    STD
    BSD
    FKP
    KMF
    ANG
    SDG
    ILS
    SAR
    BMD
    NAD
    CZK
    XPF
    MKD
    LSL
    CNY
    VES
    CLF
    LBP
    LRD
    TTD
    XOF
    SLE
    NOK
    TJS
    KYD
    PHP
    GBP
    TZS
    MAD
    PEN
    DZD
    ZMW
    CLP
    THB
    OMR
    ZAR
    CDF
    MMK
    AOA
    TRY
    BRL
    AED
    GNF
    HUF
    ALL
    NGN
    CHF
    SBD
    SGD
    JMD
    TND
    SVC
    PYG
    BYN
    HNL
    KPW
    NPR
    GHS
    SZL
    AFN
    SRD
    BOB
    BWP
    PGK
    XCD
    QAR
    BIF
    USD
    PAB
    WST
    KGS
    TOP
    PLN
    EGP
    BHD
    BTN
    FJD
    MUR
    KES
    GEL
    BZD
    MOP
    COP
    SEK
    SHP
    KWD
    HRK
    HTG
    MVR
    MDL
    MYR
}

