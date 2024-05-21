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
#[cfg(feature = "dummy_connector")]
mod bankofamerica;
#[cfg(feature = "dummy_connector")]
mod billwerk;
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
mod ebanx;
mod fiserv;
mod forte;
mod globalpay;
mod globepay;
mod gocardless;
mod gpayments;
mod helcim;
mod iatapay;
mod mifinity;
mod mollie;
mod multisafepay;
mod netcetera;
mod nexinets;
mod nmi;
mod noon;
mod nuvei;
#[cfg(feature = "dummy_connector")]
mod opayo;
mod opennode;
#[cfg(feature = "dummy_connector")]
mod payeezy;
mod payme;
mod payone;
mod paypal;
mod payu;
mod placetopay;
mod powertranz;
#[cfg(feature = "dummy_connector")]
mod prophetpay;
mod rapyd;
mod shift4;
mod square;
mod stax;
mod stripe;
mod trustpay;
mod tsys;
mod utils;
mod volt;
#[cfg(feature = "payouts")]
mod wise;
mod worldline;
mod worldpay;
mod zen;
mod zsl;
