#![allow(
    clippy::expect_used,
    clippy::panic,
    clippy::unwrap_in_result,
    clippy::unwrap_used
)]

mod aci;
mod adyen;
mod adyen_uk_ui;
mod airwallex;
mod authorizedotnet;
mod bambora;
mod bitpay;
mod bluesnap;
mod checkout;
mod coinbase;
mod connector_auth;
mod cybersource;
mod dlocal;
#[cfg(feature = "dummy_connector")]
mod dummyconnector;
mod fiserv;
mod forte;
mod globalpay;
mod iatapay;
mod mollie;
mod multisafepay;
mod nexinets;
mod nmi;
mod noon;
mod nuvei;
mod nuvei_ui;
mod opennode;
mod payeezy;
mod paypal;
mod payu;
mod payu_ui;
mod rapyd;
mod selenium;
mod shift4;
mod stripe;
mod stripe_ui;
mod trustpay;
mod utils;
mod worldline;
mod worldline_ui;
mod worldpay;
mod zen;
