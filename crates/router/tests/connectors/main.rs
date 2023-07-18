#![allow(
    clippy::expect_used,
    clippy::panic,
    clippy::unwrap_in_result,
    clippy::unwrap_used
)]
use test_utils::connector_auth;

mod aci;
mod adyen;
mod adyen_uk_ui;
mod airwallex;
mod airwallex_ui;
mod authorizedotnet;
mod authorizedotnet_ui;
mod bambora;
mod bambora_ui;
mod bitpay;
mod bluesnap;
mod bluesnap_ui;
mod cashtocode;
mod checkout;
mod checkout_ui;
mod coinbase;
mod cryptopay;
mod cybersource;
mod dlocal;
#[cfg(feature = "dummy_connector")]
mod dummyconnector;
mod fiserv;
mod forte;
mod globalpay;
mod globalpay_ui;
mod globepay;
mod iatapay;
mod mollie;
mod mollie_ui;
mod multisafepay;
mod multisafepay_ui;
mod nexinets;
mod nmi;
mod noon;
mod nuvei;
mod nuvei_ui;
mod opayo;
mod opennode;
mod payeezy;
mod payme;
mod paypal;
mod paypal_ui;
mod payu;
mod payu_ui;
mod powertranz;
mod rapyd;
mod selenium;
mod shift4;
mod shift4_ui;
mod stax;
mod stripe;
mod stripe_ui;
mod trustpay;
mod trustpay_3ds_ui;
mod tsys;
mod utils;
mod worldline;
mod worldline_ui;
mod worldpay;
mod zen;
