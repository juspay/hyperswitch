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
mod affirm;
mod airwallex;
mod amazonpay;
mod authorizedotnet;
mod bambora;
mod bamboraapac;
#[cfg(feature = "dummy_connector")]
mod bankofamerica;
mod barclaycard;
#[cfg(feature = "dummy_connector")]
mod billwerk;
mod bitpay;
mod blackhawknetwork;
mod bluecode;
mod bluesnap;
mod boku;
mod breadpay;
mod cashtocode;
mod celero;
mod chargebee;
mod checkbook;
mod checkout;
mod coinbase;
mod cryptopay;
mod cybersource;
mod datatrans;
mod deutschebank;
mod dlocal;
#[cfg(feature = "dummy_connector")]
mod dummyconnector;
mod dwolla;
mod ebanx;
mod elavon;
mod facilitapay;
mod fiserv;
mod fiservemea;
mod fiuu;
mod flexiti;
mod forte;
mod getnet;
mod globalpay;
mod globepay;
mod gocardless;
mod gpayments;
mod helcim;
mod hipay;
mod hyperswitch_vault;
mod iatapay;
mod inespay;
mod itaubank;
mod jpmorgan;
mod juspaythreedsserver;
mod mifinity;
mod mollie;
mod moneris;
mod mpgs;
mod multisafepay;
mod netcetera;
mod nexinets;
mod nexixpay;
mod nmi;
mod nomupay;
mod noon;
mod nordea;
mod novalnet;
mod nuvei;
#[cfg(feature = "dummy_connector")]
mod opayo;
mod opennode;
mod paybox;
#[cfg(feature = "dummy_connector")]
mod payeezy;
mod payload;
mod payme;
mod payone;
mod paypal;
mod paystack;
mod payu;
mod placetopay;
mod plaid;
mod powertranz;
#[cfg(feature = "dummy_connector")]
mod prophetpay;
mod rapyd;
mod razorpay;
mod redsys;
mod santander;
mod shift4;
mod silverflow;
mod square;
mod stax;
mod stripe;
mod stripebilling;
mod taxjar;
mod tokenio;
mod trustpay;
mod trustpayments;
mod tsys;
mod unified_authentication_service;
mod utils;
mod vgs;
mod volt;
mod wellsfargo;
mod worldpayvantiv;
// mod wellsfargopayout;
#[cfg(feature = "payouts")]
mod wise;
mod worldline;
mod worldpay;
mod worldpayxml;
mod zen;
mod zsl;
