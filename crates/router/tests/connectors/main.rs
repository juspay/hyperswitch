#![allow(
    clippy::expect_used,
    clippy::panic,
    clippy::unwrap_in_result,
    clippy::unwrap_used
)]
use test_utils::connector_auth;

mod aci;
mod adyen;
mod airwallex;
mod authorizedotnet;
mod bambora;
mod bitpay;
mod bluesnap;
mod boku;
mod cashtocode;
mod checkout;
mod coinbase;
mod cryptopay;
mod cybersource;
mod dlocal;
#[cfg(feature = "dummy_connector")]
mod dummyconnector;
mod fiserv;
mod forte;
mod globalpay;
mod globepay;
mod iatapay;
mod mollie;
mod multisafepay;
mod nexinets;
mod nmi;
mod noon;
mod nuvei;
mod opayo;
mod opennode;
mod payeezy;
mod payme;
mod paypal;
mod payu;
mod powertranz;
mod rapyd;
mod shift4;
mod square;
mod stax;
mod stripe;
mod trustpay;
mod tsys;
mod utils;
mod wise;
mod worldline;
mod worldpay;
mod zen;
