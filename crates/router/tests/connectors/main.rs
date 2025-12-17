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
mod bluesnap;
mod boku;
mod breadpay;
mod calida;
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
mod envoy;
mod facilitapay;
mod finix;
mod fiserv;
mod fiservemea;
mod fiuu;
mod flexiti;
mod forte;
mod getnet;
mod gigadat;
mod globalpay;
mod globepay;
mod gocardless;
mod gpayments;
mod helcim;
mod hipay;
mod hyperswitch_vault;
mod hyperwallet;
mod iatapay;
mod inespay;
mod itaubank;
mod jpmorgan;
mod juspaythreedsserver;
mod katapult;
mod loonio;
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
mod payjustnow;
mod payload;
mod payme;
mod payone;
mod paypal;
mod paysafe;
mod paystack;
mod paytm;
mod payu;
mod peachpayments;
mod phonepe;
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
mod sift;
mod silverflow;
mod square;
mod stax;
mod stripe;
mod stripebilling;
mod taxjar;
mod tesouro;
mod tokenex;
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
mod zift;
// mod wellsfargopayout;
#[cfg(feature = "payouts")]
mod wise;
mod worldline;
mod worldpay;
mod worldpayxml;
mod zen;
mod zsl;
