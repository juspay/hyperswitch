#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, strum::Display, strum::EnumString)]
#[strum(serialize_all = "snake_case")]
pub enum Connector {
    Adyen,
    Stripe,
    Checkout,
    Aci,
    Authorizedotnet,
    Braintree,
    Klarna,
    #[default]
    Dummy,
}
