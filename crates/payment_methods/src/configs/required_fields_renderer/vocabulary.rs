//! Explicit compiled vocabulary. No catch-all: adding an enum variant requires review.
use api_models::enums::{Connector, PaymentMethod, PaymentMethodType};

pub(super) fn method_label(method: PaymentMethod) -> &'static str {
    match method {
        PaymentMethod::Card => "Card",
        PaymentMethod::CardRedirect => "CardRedirect",
        PaymentMethod::PayLater => "PayLater",
        PaymentMethod::Wallet => "Wallet",
        PaymentMethod::BankRedirect => "BankRedirect",
        PaymentMethod::BankTransfer => "BankTransfer",
        PaymentMethod::Crypto => "Crypto",
        PaymentMethod::BankDebit => "BankDebit",
        PaymentMethod::Reward => "Reward",
        PaymentMethod::RealTimePayment => "RealTimePayment",
        PaymentMethod::Upi => "Upi",
        PaymentMethod::Voucher => "Voucher",
        PaymentMethod::GiftCard => "GiftCard",
        PaymentMethod::OpenBanking => "OpenBanking",
        PaymentMethod::MobilePayment => "MobilePayment",
        PaymentMethod::NetworkToken => "NetworkToken",
    }
}

pub(super) fn type_label(kind: PaymentMethodType) -> Option<&'static str> {
    match kind {
        PaymentMethodType::Ach => Some("Ach"),
        PaymentMethodType::Affirm => Some("Affirm"),
        PaymentMethodType::AfterpayClearpay => Some("AfterpayClearpay"),
        PaymentMethodType::Alfamart => Some("Alfamart"),
        PaymentMethodType::AliPay => Some("AliPay"),
        PaymentMethodType::AliPayHk => Some("AliPayHk"),
        PaymentMethodType::Alma => Some("Alma"),
        PaymentMethodType::AmazonPay => Some("AmazonPay"),
        PaymentMethodType::Paysera => Some("Paysera"),
        PaymentMethodType::ApplePay => Some("ApplePay"),
        PaymentMethodType::Atome => Some("Atome"),
        PaymentMethodType::Bacs => Some("Bacs"),
        PaymentMethodType::BancontactCard => Some("BancontactCard"),
        PaymentMethodType::Becs => Some("Becs"),
        PaymentMethodType::Benefit => Some("Benefit"),
        PaymentMethodType::Bizum => Some("Bizum"),
        PaymentMethodType::Blik => Some("Blik"),
        PaymentMethodType::Bluecode => Some("Bluecode"),
        PaymentMethodType::Boleto => Some("Boleto"),
        PaymentMethodType::BcaBankTransfer => Some("BcaBankTransfer"),
        PaymentMethodType::BniVa => Some("BniVa"),
        PaymentMethodType::Breadpay => Some("Breadpay"),
        PaymentMethodType::BriVa => Some("BriVa"),
        PaymentMethodType::BhnCardNetwork => Some("BhnCardNetwork"),
        PaymentMethodType::CardRedirect => Some("CardRedirect"),
        PaymentMethodType::CimbVa => Some("CimbVa"),
        // Display is classic_reward, API serde is classic. No approved SDK choice yet.
        PaymentMethodType::ClassicReward => None,
        PaymentMethodType::Credit => Some("Credit"),
        PaymentMethodType::CryptoCurrency => Some("CryptoCurrency"),
        PaymentMethodType::Cashapp => Some("Cashapp"),
        PaymentMethodType::Dana => Some("Dana"),
        PaymentMethodType::DanamonVa => Some("DanamonVa"),
        PaymentMethodType::Debit => Some("Debit"),
        PaymentMethodType::DuitNow => Some("DuitNow"),
        PaymentMethodType::Efecty => Some("Efecty"),
        PaymentMethodType::Eft => Some("Eft"),
        PaymentMethodType::EftDebitOrder => Some("EftDebitOrder"),
        PaymentMethodType::Eps => Some("Eps"),
        PaymentMethodType::Flexiti => Some("Flexiti"),
        PaymentMethodType::Fps => Some("Fps"),
        PaymentMethodType::Evoucher => Some("Evoucher"),
        PaymentMethodType::Giropay => Some("Giropay"),
        PaymentMethodType::Givex => Some("Givex"),
        PaymentMethodType::GooglePay => Some("GooglePay"),
        PaymentMethodType::GoPay => Some("GoPay"),
        PaymentMethodType::Gcash => Some("Gcash"),
        PaymentMethodType::Ideal => Some("Ideal"),
        PaymentMethodType::Interac => Some("Interac"),
        PaymentMethodType::Indomaret => Some("Indomaret"),
        PaymentMethodType::Klarna => Some("Klarna"),
        PaymentMethodType::KakaoPay => Some("KakaoPay"),
        PaymentMethodType::LocalBankRedirect => Some("LocalBankRedirect"),
        PaymentMethodType::MandiriVa => Some("MandiriVa"),
        PaymentMethodType::Knet => Some("Knet"),
        PaymentMethodType::MbWay => Some("MbWay"),
        PaymentMethodType::MobilePay => Some("MobilePay"),
        PaymentMethodType::Momo => Some("Momo"),
        PaymentMethodType::MomoAtm => Some("MomoAtm"),
        PaymentMethodType::Multibanco => Some("Multibanco"),
        PaymentMethodType::Neteller => Some("Neteller"),
        PaymentMethodType::OnlineBankingThailand => Some("OnlineBankingThailand"),
        PaymentMethodType::OnlineBankingCzechRepublic => Some("OnlineBankingCzechRepublic"),
        PaymentMethodType::OnlineBankingFinland => Some("OnlineBankingFinland"),
        PaymentMethodType::OnlineBankingFpx => Some("OnlineBankingFpx"),
        PaymentMethodType::OnlineBankingPoland => Some("OnlineBankingPoland"),
        PaymentMethodType::OnlineBankingSlovakia => Some("OnlineBankingSlovakia"),
        PaymentMethodType::Oxxo => Some("Oxxo"),
        PaymentMethodType::PagoEfectivo => Some("PagoEfectivo"),
        PaymentMethodType::PermataBankTransfer => Some("PermataBankTransfer"),
        PaymentMethodType::OpenBankingUk => Some("OpenBankingUk"),
        PaymentMethodType::PayBright => Some("PayBright"),
        PaymentMethodType::Payjustnow => Some("Payjustnow"),
        PaymentMethodType::Paypal => Some("Paypal"),
        PaymentMethodType::Paze => Some("Paze"),
        PaymentMethodType::Pix => Some("Pix"),
        PaymentMethodType::PixKey => Some("PixKey"),
        PaymentMethodType::PixEmv => Some("PixEmv"),
        PaymentMethodType::PixQr => Some("PixQr"),
        PaymentMethodType::PixAutomaticoQr => Some("PixAutomaticoQr"),
        PaymentMethodType::PixAutomaticoPush => Some("PixAutomaticoPush"),
        PaymentMethodType::PaySafeCard => Some("PaySafeCard"),
        PaymentMethodType::Przelewy24 => Some("Przelewy24"),
        PaymentMethodType::PromptPay => Some("PromptPay"),
        PaymentMethodType::Pse => Some("Pse"),
        PaymentMethodType::Qris => Some("Qris"),
        PaymentMethodType::RedCompra => Some("RedCompra"),
        PaymentMethodType::RedPagos => Some("RedPagos"),
        PaymentMethodType::SamsungPay => Some("SamsungPay"),
        PaymentMethodType::Sepa => Some("Sepa"),
        PaymentMethodType::SepaBankTransfer => Some("SepaBankTransfer"),
        PaymentMethodType::SepaGuarenteedDebit => Some("SepaGuarenteedDebit"),
        PaymentMethodType::Skrill => Some("Skrill"),
        PaymentMethodType::Sofort => Some("Sofort"),
        PaymentMethodType::Swish => Some("Swish"),
        PaymentMethodType::TouchNGo => Some("TouchNGo"),
        PaymentMethodType::Trustly => Some("Trustly"),
        PaymentMethodType::Twint => Some("Twint"),
        PaymentMethodType::UpiCollect => Some("UpiCollect"),
        PaymentMethodType::UpiIntent => Some("UpiIntent"),
        PaymentMethodType::UpiQr => Some("UpiQr"),
        PaymentMethodType::Vipps => Some("Vipps"),
        PaymentMethodType::VietQr => Some("VietQr"),
        PaymentMethodType::Venmo => Some("Venmo"),
        PaymentMethodType::Walley => Some("Walley"),
        PaymentMethodType::Wero => Some("Wero"),
        PaymentMethodType::WeChatPay => Some("WeChatPay"),
        PaymentMethodType::SevenEleven => Some("SevenEleven"),
        PaymentMethodType::Lawson => Some("Lawson"),
        PaymentMethodType::MiniStop => Some("MiniStop"),
        PaymentMethodType::FamilyMart => Some("FamilyMart"),
        PaymentMethodType::Seicomart => Some("Seicomart"),
        PaymentMethodType::PayEasy => Some("PayEasy"),
        PaymentMethodType::LocalBankTransfer => Some("LocalBankTransfer"),
        PaymentMethodType::Mifinity => Some("Mifinity"),
        PaymentMethodType::OpenBankingPIS => Some("OpenBankingPis"),
        PaymentMethodType::DirectCarrierBilling => Some("DirectCarrierBilling"),
        PaymentMethodType::InstantBankTransfer => Some("InstantBankTransfer"),
        PaymentMethodType::InstantBankTransferFinland => Some("InstantBankTransferFinland"),
        PaymentMethodType::InstantBankTransferPoland => Some("InstantBankTransferPoland"),
        PaymentMethodType::RevolutPay => Some("RevolutPay"),
        PaymentMethodType::IndonesianBankTransfer => Some("IndonesianBankTransfer"),
        PaymentMethodType::OpenBanking => Some("OpenBanking"),
        PaymentMethodType::NetworkToken => Some("NetworkToken"),
        PaymentMethodType::Payshap => Some("Payshap"),
        PaymentMethodType::PayshapProxy => Some("PayshapProxy"),
        PaymentMethodType::Ted => Some("Ted"),
    }
}

#[allow(clippy::match_same_arms)] // Separate cfg arm is necessary for compiled dummy availability.
pub(super) fn connector_label(connector: Connector) -> String {
    match connector {
        Connector::AbsaSanlam
        | Connector::Authipay
        | Connector::Adyenplatform
        | Connector::Aci
        | Connector::Adyen
        | Connector::Affirm
        | Connector::Airwallex
        | Connector::Amazonpay
        | Connector::Archipel
        | Connector::Authorizedotnet
        | Connector::Bambora
        | Connector::Bamboraapac
        | Connector::Bankofamerica
        | Connector::Barclaycard
        | Connector::Betterpayment
        | Connector::Billwerk
        | Connector::Bitpay
        | Connector::Bluesnap
        | Connector::Blackhawknetwork
        | Connector::Calida
        | Connector::Boku
        | Connector::Braintree
        | Connector::Breadpay
        | Connector::Cardinal
        | Connector::Cashtocode
        | Connector::Celero
        | Connector::Chargebee
        | Connector::Checkbook
        | Connector::Checkout
        | Connector::Citigate
        | Connector::Coinbase
        | Connector::Coingate
        | Connector::Custombilling
        | Connector::Cryptopay
        | Connector::CtpMastercard
        | Connector::CtpVisa
        | Connector::Cybersource
        | Connector::Cybersourcedecisionmanager
        | Connector::D24
        | Connector::Datatrans
        | Connector::Deutschebank
        | Connector::Digitalvirgo
        | Connector::Dlocal
        | Connector::Dwolla
        | Connector::Ebanx
        | Connector::Ecommpay
        | Connector::Envoy
        | Connector::Elavon
        | Connector::Etisalat
        | Connector::ElavonPg
        | Connector::Facilitapay
        | Connector::Finix
        | Connector::Fiserv
        | Connector::Fiservemea
        | Connector::Fiservcommercehub
        | Connector::Fiuu
        | Connector::Flexiti
        | Connector::Forte
        | Connector::Getnet
        | Connector::Gigadat
        | Connector::Givepayments
        | Connector::Globalpay
        | Connector::GlobalpaymentsHeartland
        | Connector::GlobalpaymentsRealex
        | Connector::Globepay
        | Connector::Gocardless
        | Connector::GotymeSanlam
        | Connector::Gpayments
        | Connector::Hipay
        | Connector::Helcim
        | Connector::Hyperpg
        | Connector::HyperswitchVault
        | Connector::Interpayments
        | Connector::Inespay
        | Connector::Iatapay
        | Connector::Ilixium
        | Connector::Imerchantsolutions
        | Connector::Itaubank
        | Connector::Jpmorgan
        | Connector::JpmorganOrbital
        | Connector::Juspay
        | Connector::Juspaythreedsserver
        | Connector::Klarna
        | Connector::Loonio
        | Connector::Merchante
        | Connector::Mifinity
        | Connector::Mollie
        | Connector::Moneris
        | Connector::Multisafepay
        | Connector::Netcetera
        | Connector::Nexinets
        | Connector::Nexixpay
        | Connector::Nmi
        | Connector::Nomupay
        | Connector::Noon
        | Connector::Nordea
        | Connector::Novalnet
        | Connector::Nuvei
        | Connector::Opennode
        | Connector::Paybox
        | Connector::Paydotcom
        | Connector::Payhound
        | Connector::Payload
        | Connector::Payme
        | Connector::Paynearme
        | Connector::Payone
        | Connector::Paypal
        | Connector::Paysafe
        | Connector::Paystack
        | Connector::Paytm
        | Connector::Payconex
        | Connector::Payu
        | Connector::Peachpayments
        | Connector::Payjustnow
        | Connector::Payjustnowinstore
        | Connector::Phonepe
        | Connector::Placetopay
        | Connector::Powertranz
        | Connector::Prophetpay
        | Connector::Rapyd
        | Connector::Razorpay
        | Connector::Recurly
        | Connector::Redsys
        | Connector::Revolv3
        | Connector::Saferpay
        | Connector::Santander
        | Connector::Shift4
        | Connector::Silverflow
        | Connector::Square
        | Connector::Stax
        | Connector::Stripe
        | Connector::Stripebilling
        | Connector::Taxjar
        | Connector::Threedsecureio
        | Connector::Tesouro
        | Connector::Tokenex
        | Connector::Tokenio
        | Connector::Trustly
        | Connector::Truelayer
        | Connector::Trustpay
        | Connector::Trustpayments
        | Connector::Tsys
        | Connector::TsysTransit
        | Connector::Vgs
        | Connector::Volt
        | Connector::Wellsfargo
        | Connector::Wise
        | Connector::Worldline
        | Connector::Worldpay
        | Connector::Worldpayraft
        | Connector::Worldpayvantiv
        | Connector::Worldpayxml
        | Connector::Worldpaymodular
        | Connector::Signifyd
        | Connector::Nsure
        | Connector::Plaid
        | Connector::Riskified
        | Connector::SanlamPayshield
        | Connector::Xendit
        | Connector::Zen
        | Connector::Zift
        | Connector::Zsl => connector.to_string(),
        #[cfg(feature = "dummy_connector")]
        Connector::DummyBillingConnector
        | Connector::DummyConnector1
        | Connector::DummyConnector2
        | Connector::DummyConnector3
        | Connector::DummyConnector4
        | Connector::DummyConnector5
        | Connector::DummyConnector6
        | Connector::DummyConnector7 => connector.to_string(),
    }
}
