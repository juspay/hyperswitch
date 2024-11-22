use utoipa::ToSchema;
#[derive(
    Clone,
    Copy,
    Debug,
    Eq,
    Hash,
    PartialEq,
    serde::Serialize,
    serde::Deserialize,
    strum::Display,
    strum::EnumString,
    strum::EnumIter,
    strum::VariantNames,
    ToSchema,
)]
#[router_derive::diesel_enum(storage_type = "db_enum")]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
/// Connectors eligible for payments routing
pub enum RoutableConnectors {
    Adyenplatform,
    #[cfg(feature = "dummy_connector")]
    #[serde(rename = "phonypay")]
    #[strum(serialize = "phonypay")]
    DummyConnector1,
    #[cfg(feature = "dummy_connector")]
    #[serde(rename = "fauxpay")]
    #[strum(serialize = "fauxpay")]
    DummyConnector2,
    #[cfg(feature = "dummy_connector")]
    #[serde(rename = "pretendpay")]
    #[strum(serialize = "pretendpay")]
    DummyConnector3,
    #[cfg(feature = "dummy_connector")]
    #[serde(rename = "stripe_test")]
    #[strum(serialize = "stripe_test")]
    DummyConnector4,
    #[cfg(feature = "dummy_connector")]
    #[serde(rename = "adyen_test")]
    #[strum(serialize = "adyen_test")]
    DummyConnector5,
    #[cfg(feature = "dummy_connector")]
    #[serde(rename = "checkout_test")]
    #[strum(serialize = "checkout_test")]
    DummyConnector6,
    #[cfg(feature = "dummy_connector")]
    #[serde(rename = "paypal_test")]
    #[strum(serialize = "paypal_test")]
    DummyConnector7,
    Aci,
    Adyen,
    Airwallex,
    // Amazonpay,
    Authorizedotnet,
    Bankofamerica,
    Billwerk,
    Bitpay,
    Bambora,
    Bamboraapac,
    Bluesnap,
    Boku,
    Braintree,
    Cashtocode,
    Checkout,
    Coinbase,
    Cryptopay,
    Cybersource,
    Datatrans,
    Deutschebank,
    Digitalvirgo,
    Dlocal,
    Ebanx,
    Fiserv,
    Fiservemea,
    Fiuu,
    Forte,
    Globalpay,
    Globepay,
    Gocardless,
    Helcim,
    Iatapay,
    // Inespay,
    Itaubank,
    //Jpmorgan,
    Klarna,
    Mifinity,
    Mollie,
    Multisafepay,
    Nexinets,
    Nexixpay,
    Nmi,
    // Nomupay,
    Noon,
    Novalnet,
    Nuvei,
    // Opayo, added as template code for future usage
    Opennode,
    // Payeezy, As psync and rsync are not supported by this connector, it is added as template code for future usage
    Paybox,
    Payme,
    Payone,
    Paypal,
    Payu,
    Placetopay,
    Powertranz,
    Prophetpay,
    Rapyd,
    Razorpay,
    Riskified,
    Shift4,
    Signifyd,
    Square,
    Stax,
    Stripe,
    // Taxjar,
    Trustpay,
    // Thunes
    // Tsys,
    Tsys,
    Volt,
    Wellsfargo,
    // Wellsfargopayout,
    Wise,
    Worldline,
    Worldpay,
    Xendit,
    Zen,
    Plaid,
    Zsl,
}
