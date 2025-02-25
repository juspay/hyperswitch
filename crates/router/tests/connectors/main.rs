#![allow(
    clippy::expect_used,
    clippy::panic,
    clippy::unwrap_in_result,
    clippy::unwrap_used
)]
use test_utils::connector_auth;

mod aci;
mod adyen;
mod adyenplatform;
mod airwallex;
mod amazonpay;
mod authorizedotnet;
mod bambora;
mod bamboraapac;
#[cfg(feature = "dummy_connector")]
mod bankofamerica;
#[cfg(feature = "dummy_connector")]
mod billwerk;
mod bitpay;
mod bluesnap;
mod boku;
mod cashtocode;
mod chargebee;
mod checkout;
mod coinbase;
mod cryptopay;
mod cybersource;
mod datatrans;
mod deutschebank;
mod dlocal;
#[cfg(feature = "dummy_connector")]
mod dummyconnector;
mod ebanx;
mod elavon;
mod fiserv;
mod fiservemea;
mod fiuu;
mod forte;
mod getnet;
mod globalpay;
mod globepay;
mod gocardless;
mod gpayments;
mod helcim;
mod iatapay;
mod inespay;
mod itaubank;
mod jpmorgan;
mod mifinity;
mod mollie;
mod moneris;
mod multisafepay;
mod netcetera;
mod nexinets;
mod nexixpay;
mod nmi;
mod nomupay;
mod noon;
mod novalnet;
mod nuvei;
#[cfg(feature = "dummy_connector")]
mod opayo;
mod opennode;
mod paybox;
#[cfg(feature = "dummy_connector")]
mod payeezy;
mod payme;
mod payone;
mod paypal;
mod payu;
mod placetopay;
mod plaid;
mod powertranz;
#[cfg(feature = "dummy_connector")]
mod prophetpay;
mod rapyd;
mod razorpay;
mod redsys;
mod shift4;
mod square;
mod stax;
mod stripe;
mod taxjar;
mod trustpay;
mod tsys;
mod unified_authentication_service;
mod utils;
mod volt;
mod wellsfargo;
// mod wellsfargopayout;
#[cfg(feature = "payouts")]
mod wise;
mod worldline;
mod worldpay;
mod zen;
mod zsl;
