use api_models::enums as api_enums;
use euclid::{
    backend::BackendInput,
    dirval,
    dssa::types::AnalysisErrorType,
    frontend::{ast, dir},
    types::{NumValue, StrValue},
};

use crate::error::KgraphError;

pub trait IntoContext {
    fn into_context(self) -> Result<Vec<dir::DirValue>, KgraphError>;
}

impl IntoContext for BackendInput {
    fn into_context(self) -> Result<Vec<dir::DirValue>, KgraphError> {
        let mut ctx: Vec<dir::DirValue> = Vec::new();

        ctx.push(dir::DirValue::PaymentAmount(NumValue {
            number: self.payment.amount,
            refinement: None,
        }));

        ctx.push(dir::DirValue::PaymentCurrency(self.payment.currency));

        if let Some(auth_type) = self.payment.authentication_type {
            ctx.push(dir::DirValue::AuthenticationType(auth_type));
        }

        if let Some(capture_method) = self.payment.capture_method {
            ctx.push(dir::DirValue::CaptureMethod(capture_method));
        }

        if let Some(business_country) = self.payment.business_country {
            ctx.push(dir::DirValue::BusinessCountry(business_country));
        }
        if let Some(business_label) = self.payment.business_label {
            ctx.push(dir::DirValue::BusinessLabel(StrValue {
                value: business_label,
            }));
        }
        if let Some(billing_country) = self.payment.billing_country {
            ctx.push(dir::DirValue::BillingCountry(billing_country));
        }

        if let Some(payment_method) = self.payment_method.payment_method {
            ctx.push(dir::DirValue::PaymentMethod(payment_method));
        }

        if let (Some(pm_type), Some(payment_method)) = (
            self.payment_method.payment_method_type,
            self.payment_method.payment_method,
        ) {
            ctx.push((pm_type, payment_method).into_dir_value()?)
        }

        if let Some(card_network) = self.payment_method.card_network {
            ctx.push(dir::DirValue::CardNetwork(card_network));
        }
        if let Some(setup_future_usage) = self.payment.setup_future_usage {
            ctx.push(dir::DirValue::SetupFutureUsage(setup_future_usage));
        }
        if let Some(mandate_acceptance_type) = self.mandate.mandate_acceptance_type {
            ctx.push(dir::DirValue::MandateAcceptanceType(
                mandate_acceptance_type,
            ));
        }
        if let Some(mandate_type) = self.mandate.mandate_type {
            ctx.push(dir::DirValue::MandateType(mandate_type));
        }
        if let Some(payment_type) = self.mandate.payment_type {
            ctx.push(dir::DirValue::PaymentType(payment_type));
        }

        Ok(ctx)
    }
}

pub trait IntoDirValue {
    fn into_dir_value(self) -> Result<dir::DirValue, KgraphError>;
}

impl IntoDirValue for ast::ConnectorChoice {
    fn into_dir_value(self) -> Result<dir::DirValue, KgraphError> {
        Ok(dir::DirValue::Connector(Box::new(self)))
    }
}

impl IntoDirValue for api_enums::PaymentMethod {
    fn into_dir_value(self) -> Result<dir::DirValue, KgraphError> {
        match self {
            Self::Card => Ok(dirval!(PaymentMethod = Card)),
            Self::Wallet => Ok(dirval!(PaymentMethod = Wallet)),
            Self::PayLater => Ok(dirval!(PaymentMethod = PayLater)),
            Self::BankRedirect => Ok(dirval!(PaymentMethod = BankRedirect)),
            Self::Crypto => Ok(dirval!(PaymentMethod = Crypto)),
            Self::BankDebit => Ok(dirval!(PaymentMethod = BankDebit)),
            Self::BankTransfer => Ok(dirval!(PaymentMethod = BankTransfer)),
            Self::Reward => Ok(dirval!(PaymentMethod = Reward)),
            Self::Upi => Ok(dirval!(PaymentMethod = Upi)),
            Self::Voucher => Ok(dirval!(PaymentMethod = Voucher)),
            Self::GiftCard => Ok(dirval!(PaymentMethod = GiftCard)),
            Self::CardRedirect => Ok(dirval!(PaymentMethod = CardRedirect)),
        }
    }
}

impl IntoDirValue for api_enums::AuthenticationType {
    fn into_dir_value(self) -> Result<dir::DirValue, KgraphError> {
        match self {
            Self::ThreeDs => Ok(dirval!(AuthenticationType = ThreeDs)),
            Self::NoThreeDs => Ok(dirval!(AuthenticationType = NoThreeDs)),
        }
    }
}

impl IntoDirValue for api_enums::FutureUsage {
    fn into_dir_value(self) -> Result<dir::DirValue, KgraphError> {
        match self {
            Self::OnSession => Ok(dirval!(SetupFutureUsage = OnSession)),
            Self::OffSession => Ok(dirval!(SetupFutureUsage = OffSession)),
        }
    }
}

impl IntoDirValue for (api_enums::PaymentMethodType, api_enums::PaymentMethod) {
    fn into_dir_value(self) -> Result<dir::DirValue, KgraphError> {
        match self.0 {
            api_enums::PaymentMethodType::Credit => Ok(dirval!(CardType = Credit)),
            api_enums::PaymentMethodType::Debit => Ok(dirval!(CardType = Debit)),
            api_enums::PaymentMethodType::Giropay => Ok(dirval!(BankRedirectType = Giropay)),
            api_enums::PaymentMethodType::Ideal => Ok(dirval!(BankRedirectType = Ideal)),
            api_enums::PaymentMethodType::Sofort => Ok(dirval!(BankRedirectType = Sofort)),
            api_enums::PaymentMethodType::Eps => Ok(dirval!(BankRedirectType = Eps)),
            api_enums::PaymentMethodType::Klarna => Ok(dirval!(PayLaterType = Klarna)),
            api_enums::PaymentMethodType::Affirm => Ok(dirval!(PayLaterType = Affirm)),
            api_enums::PaymentMethodType::AfterpayClearpay => {
                Ok(dirval!(PayLaterType = AfterpayClearpay))
            }
            api_enums::PaymentMethodType::GooglePay => Ok(dirval!(WalletType = GooglePay)),
            api_enums::PaymentMethodType::ApplePay => Ok(dirval!(WalletType = ApplePay)),
            api_enums::PaymentMethodType::Paypal => Ok(dirval!(WalletType = Paypal)),
            api_enums::PaymentMethodType::CryptoCurrency => {
                Ok(dirval!(CryptoType = CryptoCurrency))
            }
            api_enums::PaymentMethodType::Ach => match self.1 {
                api_enums::PaymentMethod::BankDebit => Ok(dirval!(BankDebitType = Ach)),
                api_enums::PaymentMethod::BankTransfer => Ok(dirval!(BankTransferType = Ach)),
                api_enums::PaymentMethod::BankRedirect
                | api_enums::PaymentMethod::Card
                | api_enums::PaymentMethod::CardRedirect
                | api_enums::PaymentMethod::PayLater
                | api_enums::PaymentMethod::Wallet
                | api_enums::PaymentMethod::Crypto
                | api_enums::PaymentMethod::Reward
                | api_enums::PaymentMethod::Upi
                | api_enums::PaymentMethod::Voucher
                | api_enums::PaymentMethod::GiftCard => Err(KgraphError::ContextConstructionError(
                    AnalysisErrorType::NotSupported,
                )),
            },
            api_enums::PaymentMethodType::Bacs => match self.1 {
                api_enums::PaymentMethod::BankDebit => Ok(dirval!(BankDebitType = Bacs)),
                api_enums::PaymentMethod::BankTransfer => Ok(dirval!(BankTransferType = Bacs)),
                api_enums::PaymentMethod::BankRedirect
                | api_enums::PaymentMethod::Card
                | api_enums::PaymentMethod::CardRedirect
                | api_enums::PaymentMethod::PayLater
                | api_enums::PaymentMethod::Wallet
                | api_enums::PaymentMethod::Crypto
                | api_enums::PaymentMethod::Reward
                | api_enums::PaymentMethod::Upi
                | api_enums::PaymentMethod::Voucher
                | api_enums::PaymentMethod::GiftCard => Err(KgraphError::ContextConstructionError(
                    AnalysisErrorType::NotSupported,
                )),
            },
            api_enums::PaymentMethodType::Becs => Ok(dirval!(BankDebitType = Becs)),
            api_enums::PaymentMethodType::Sepa => match self.1 {
                api_enums::PaymentMethod::BankDebit => Ok(dirval!(BankDebitType = Sepa)),
                api_enums::PaymentMethod::BankTransfer => Ok(dirval!(BankTransferType = Sepa)),
                api_enums::PaymentMethod::BankRedirect
                | api_enums::PaymentMethod::Card
                | api_enums::PaymentMethod::CardRedirect
                | api_enums::PaymentMethod::PayLater
                | api_enums::PaymentMethod::Wallet
                | api_enums::PaymentMethod::Crypto
                | api_enums::PaymentMethod::Reward
                | api_enums::PaymentMethod::Upi
                | api_enums::PaymentMethod::Voucher
                | api_enums::PaymentMethod::GiftCard => Err(KgraphError::ContextConstructionError(
                    AnalysisErrorType::NotSupported,
                )),
            },
            api_enums::PaymentMethodType::AliPay => Ok(dirval!(WalletType = AliPay)),
            api_enums::PaymentMethodType::AliPayHk => Ok(dirval!(WalletType = AliPayHk)),
            api_enums::PaymentMethodType::BancontactCard => {
                Ok(dirval!(BankRedirectType = BancontactCard))
            }
            api_enums::PaymentMethodType::Blik => Ok(dirval!(BankRedirectType = Blik)),
            api_enums::PaymentMethodType::MbWay => Ok(dirval!(WalletType = MbWay)),
            api_enums::PaymentMethodType::MobilePay => Ok(dirval!(WalletType = MobilePay)),
            api_enums::PaymentMethodType::Cashapp => Ok(dirval!(WalletType = Cashapp)),
            api_enums::PaymentMethodType::Multibanco => Ok(dirval!(BankTransferType = Multibanco)),
            api_enums::PaymentMethodType::Pix => Ok(dirval!(BankTransferType = Pix)),
            api_enums::PaymentMethodType::Pse => Ok(dirval!(BankTransferType = Pse)),
            api_enums::PaymentMethodType::Interac => Ok(dirval!(BankRedirectType = Interac)),
            api_enums::PaymentMethodType::OnlineBankingCzechRepublic => {
                Ok(dirval!(BankRedirectType = OnlineBankingCzechRepublic))
            }
            api_enums::PaymentMethodType::OnlineBankingFinland => {
                Ok(dirval!(BankRedirectType = OnlineBankingFinland))
            }
            api_enums::PaymentMethodType::OnlineBankingPoland => {
                Ok(dirval!(BankRedirectType = OnlineBankingPoland))
            }
            api_enums::PaymentMethodType::OnlineBankingSlovakia => {
                Ok(dirval!(BankRedirectType = OnlineBankingSlovakia))
            }
            api_enums::PaymentMethodType::Swish => Ok(dirval!(WalletType = Swish)),
            api_enums::PaymentMethodType::Trustly => Ok(dirval!(BankRedirectType = Trustly)),
            api_enums::PaymentMethodType::Bizum => Ok(dirval!(BankRedirectType = Bizum)),

            api_enums::PaymentMethodType::PayBright => Ok(dirval!(PayLaterType = PayBright)),
            api_enums::PaymentMethodType::Walley => Ok(dirval!(PayLaterType = Walley)),
            api_enums::PaymentMethodType::Przelewy24 => Ok(dirval!(BankRedirectType = Przelewy24)),
            api_enums::PaymentMethodType::WeChatPay => Ok(dirval!(WalletType = WeChatPay)),

            api_enums::PaymentMethodType::ClassicReward => Ok(dirval!(RewardType = ClassicReward)),
            api_enums::PaymentMethodType::Evoucher => Ok(dirval!(RewardType = Evoucher)),
            api_enums::PaymentMethodType::UpiCollect => Ok(dirval!(UpiType = UpiCollect)),
            api_enums::PaymentMethodType::SamsungPay => Ok(dirval!(WalletType = SamsungPay)),
            api_enums::PaymentMethodType::GoPay => Ok(dirval!(WalletType = GoPay)),
            api_enums::PaymentMethodType::KakaoPay => Ok(dirval!(WalletType = KakaoPay)),
            api_enums::PaymentMethodType::Twint => Ok(dirval!(WalletType = Twint)),
            api_enums::PaymentMethodType::Gcash => Ok(dirval!(WalletType = Gcash)),
            api_enums::PaymentMethodType::Vipps => Ok(dirval!(WalletType = Vipps)),
            api_enums::PaymentMethodType::Momo => Ok(dirval!(WalletType = Momo)),
            api_enums::PaymentMethodType::Alma => Ok(dirval!(PayLaterType = Alma)),
            api_enums::PaymentMethodType::Dana => Ok(dirval!(WalletType = Dana)),
            api_enums::PaymentMethodType::OnlineBankingFpx => {
                Ok(dirval!(BankRedirectType = OnlineBankingFpx))
            }
            api_enums::PaymentMethodType::OnlineBankingThailand => {
                Ok(dirval!(BankRedirectType = OnlineBankingThailand))
            }
            api_enums::PaymentMethodType::TouchNGo => Ok(dirval!(WalletType = TouchNGo)),
            api_enums::PaymentMethodType::Atome => Ok(dirval!(PayLaterType = Atome)),
            api_enums::PaymentMethodType::Boleto => Ok(dirval!(VoucherType = Boleto)),
            api_enums::PaymentMethodType::Efecty => Ok(dirval!(VoucherType = Efecty)),
            api_enums::PaymentMethodType::PagoEfectivo => Ok(dirval!(VoucherType = PagoEfectivo)),
            api_enums::PaymentMethodType::RedCompra => Ok(dirval!(VoucherType = RedCompra)),
            api_enums::PaymentMethodType::RedPagos => Ok(dirval!(VoucherType = RedPagos)),
            api_enums::PaymentMethodType::Alfamart => Ok(dirval!(VoucherType = Alfamart)),
            api_enums::PaymentMethodType::BcaBankTransfer => {
                Ok(dirval!(BankTransferType = BcaBankTransfer))
            }
            api_enums::PaymentMethodType::BniVa => Ok(dirval!(BankTransferType = BniVa)),
            api_enums::PaymentMethodType::BriVa => Ok(dirval!(BankTransferType = BriVa)),
            api_enums::PaymentMethodType::CimbVa => Ok(dirval!(BankTransferType = CimbVa)),
            api_enums::PaymentMethodType::DanamonVa => Ok(dirval!(BankTransferType = DanamonVa)),
            api_enums::PaymentMethodType::Indomaret => Ok(dirval!(VoucherType = Indomaret)),
            api_enums::PaymentMethodType::MandiriVa => Ok(dirval!(BankTransferType = MandiriVa)),
            api_enums::PaymentMethodType::PermataBankTransfer => {
                Ok(dirval!(BankTransferType = PermataBankTransfer))
            }
            api_enums::PaymentMethodType::PaySafeCard => Ok(dirval!(GiftCardType = PaySafeCard)),
            api_enums::PaymentMethodType::SevenEleven => Ok(dirval!(VoucherType = SevenEleven)),
            api_enums::PaymentMethodType::Lawson => Ok(dirval!(VoucherType = Lawson)),
            api_enums::PaymentMethodType::MiniStop => Ok(dirval!(VoucherType = MiniStop)),
            api_enums::PaymentMethodType::FamilyMart => Ok(dirval!(VoucherType = FamilyMart)),
            api_enums::PaymentMethodType::Seicomart => Ok(dirval!(VoucherType = Seicomart)),
            api_enums::PaymentMethodType::PayEasy => Ok(dirval!(VoucherType = PayEasy)),
            api_enums::PaymentMethodType::Givex => Ok(dirval!(GiftCardType = Givex)),
            api_enums::PaymentMethodType::Benefit => Ok(dirval!(CardRedirectType = Benefit)),
            api_enums::PaymentMethodType::Knet => Ok(dirval!(CardRedirectType = Knet)),
            api_enums::PaymentMethodType::OpenBankingUk => {
                Ok(dirval!(BankRedirectType = OpenBankingUk))
            }
            api_enums::PaymentMethodType::MomoAtm => Ok(dirval!(CardRedirectType = MomoAtm)),
            api_enums::PaymentMethodType::Oxxo => Ok(dirval!(VoucherType = Oxxo)),
        }
    }
}

impl IntoDirValue for api_enums::CardNetwork {
    fn into_dir_value(self) -> Result<dir::DirValue, KgraphError> {
        match self {
            Self::Visa => Ok(dirval!(CardNetwork = Visa)),
            Self::Mastercard => Ok(dirval!(CardNetwork = Mastercard)),
            Self::AmericanExpress => Ok(dirval!(CardNetwork = AmericanExpress)),
            Self::JCB => Ok(dirval!(CardNetwork = JCB)),
            Self::DinersClub => Ok(dirval!(CardNetwork = DinersClub)),
            Self::Discover => Ok(dirval!(CardNetwork = Discover)),
            Self::CartesBancaires => Ok(dirval!(CardNetwork = CartesBancaires)),
            Self::UnionPay => Ok(dirval!(CardNetwork = UnionPay)),
            Self::Interac => Ok(dirval!(CardNetwork = Interac)),
            Self::RuPay => Ok(dirval!(CardNetwork = RuPay)),
            Self::Maestro => Ok(dirval!(CardNetwork = Maestro)),
        }
    }
}

impl IntoDirValue for api_enums::Currency {
    fn into_dir_value(self) -> Result<dir::DirValue, KgraphError> {
        match self {
            Self::AED => Ok(dirval!(PaymentCurrency = AED)),
            Self::ALL => Ok(dirval!(PaymentCurrency = ALL)),
            Self::AMD => Ok(dirval!(PaymentCurrency = AMD)),
            Self::ANG => Ok(dirval!(PaymentCurrency = ANG)),
            Self::ARS => Ok(dirval!(PaymentCurrency = ARS)),
            Self::AUD => Ok(dirval!(PaymentCurrency = AUD)),
            Self::AWG => Ok(dirval!(PaymentCurrency = AWG)),
            Self::AZN => Ok(dirval!(PaymentCurrency = AZN)),
            Self::BBD => Ok(dirval!(PaymentCurrency = BBD)),
            Self::BDT => Ok(dirval!(PaymentCurrency = BDT)),
            Self::BHD => Ok(dirval!(PaymentCurrency = BHD)),
            Self::BIF => Ok(dirval!(PaymentCurrency = BIF)),
            Self::BMD => Ok(dirval!(PaymentCurrency = BMD)),
            Self::BND => Ok(dirval!(PaymentCurrency = BND)),
            Self::BOB => Ok(dirval!(PaymentCurrency = BOB)),
            Self::BRL => Ok(dirval!(PaymentCurrency = BRL)),
            Self::BSD => Ok(dirval!(PaymentCurrency = BSD)),
            Self::BWP => Ok(dirval!(PaymentCurrency = BWP)),
            Self::BZD => Ok(dirval!(PaymentCurrency = BZD)),
            Self::CAD => Ok(dirval!(PaymentCurrency = CAD)),
            Self::CHF => Ok(dirval!(PaymentCurrency = CHF)),
            Self::CLP => Ok(dirval!(PaymentCurrency = CLP)),
            Self::CNY => Ok(dirval!(PaymentCurrency = CNY)),
            Self::COP => Ok(dirval!(PaymentCurrency = COP)),
            Self::CRC => Ok(dirval!(PaymentCurrency = CRC)),
            Self::CUP => Ok(dirval!(PaymentCurrency = CUP)),
            Self::CZK => Ok(dirval!(PaymentCurrency = CZK)),
            Self::DJF => Ok(dirval!(PaymentCurrency = DJF)),
            Self::DKK => Ok(dirval!(PaymentCurrency = DKK)),
            Self::DOP => Ok(dirval!(PaymentCurrency = DOP)),
            Self::DZD => Ok(dirval!(PaymentCurrency = DZD)),
            Self::EGP => Ok(dirval!(PaymentCurrency = EGP)),
            Self::ETB => Ok(dirval!(PaymentCurrency = ETB)),
            Self::EUR => Ok(dirval!(PaymentCurrency = EUR)),
            Self::FJD => Ok(dirval!(PaymentCurrency = FJD)),
            Self::GBP => Ok(dirval!(PaymentCurrency = GBP)),
            Self::GHS => Ok(dirval!(PaymentCurrency = GHS)),
            Self::GIP => Ok(dirval!(PaymentCurrency = GIP)),
            Self::GMD => Ok(dirval!(PaymentCurrency = GMD)),
            Self::GNF => Ok(dirval!(PaymentCurrency = GNF)),
            Self::GTQ => Ok(dirval!(PaymentCurrency = GTQ)),
            Self::GYD => Ok(dirval!(PaymentCurrency = GYD)),
            Self::HKD => Ok(dirval!(PaymentCurrency = HKD)),
            Self::HNL => Ok(dirval!(PaymentCurrency = HNL)),
            Self::HRK => Ok(dirval!(PaymentCurrency = HRK)),
            Self::HTG => Ok(dirval!(PaymentCurrency = HTG)),
            Self::HUF => Ok(dirval!(PaymentCurrency = HUF)),
            Self::IDR => Ok(dirval!(PaymentCurrency = IDR)),
            Self::ILS => Ok(dirval!(PaymentCurrency = ILS)),
            Self::INR => Ok(dirval!(PaymentCurrency = INR)),
            Self::JMD => Ok(dirval!(PaymentCurrency = JMD)),
            Self::JOD => Ok(dirval!(PaymentCurrency = JOD)),
            Self::JPY => Ok(dirval!(PaymentCurrency = JPY)),
            Self::KES => Ok(dirval!(PaymentCurrency = KES)),
            Self::KGS => Ok(dirval!(PaymentCurrency = KGS)),
            Self::KHR => Ok(dirval!(PaymentCurrency = KHR)),
            Self::KMF => Ok(dirval!(PaymentCurrency = KMF)),
            Self::KRW => Ok(dirval!(PaymentCurrency = KRW)),
            Self::KWD => Ok(dirval!(PaymentCurrency = KWD)),
            Self::KYD => Ok(dirval!(PaymentCurrency = KYD)),
            Self::KZT => Ok(dirval!(PaymentCurrency = KZT)),
            Self::LAK => Ok(dirval!(PaymentCurrency = LAK)),
            Self::LBP => Ok(dirval!(PaymentCurrency = LBP)),
            Self::LKR => Ok(dirval!(PaymentCurrency = LKR)),
            Self::LRD => Ok(dirval!(PaymentCurrency = LRD)),
            Self::LSL => Ok(dirval!(PaymentCurrency = LSL)),
            Self::MAD => Ok(dirval!(PaymentCurrency = MAD)),
            Self::MDL => Ok(dirval!(PaymentCurrency = MDL)),
            Self::MGA => Ok(dirval!(PaymentCurrency = MGA)),
            Self::MKD => Ok(dirval!(PaymentCurrency = MKD)),
            Self::MMK => Ok(dirval!(PaymentCurrency = MMK)),
            Self::MNT => Ok(dirval!(PaymentCurrency = MNT)),
            Self::MOP => Ok(dirval!(PaymentCurrency = MOP)),
            Self::MUR => Ok(dirval!(PaymentCurrency = MUR)),
            Self::MVR => Ok(dirval!(PaymentCurrency = MVR)),
            Self::MWK => Ok(dirval!(PaymentCurrency = MWK)),
            Self::MXN => Ok(dirval!(PaymentCurrency = MXN)),
            Self::MYR => Ok(dirval!(PaymentCurrency = MYR)),
            Self::NAD => Ok(dirval!(PaymentCurrency = NAD)),
            Self::NGN => Ok(dirval!(PaymentCurrency = NGN)),
            Self::NIO => Ok(dirval!(PaymentCurrency = NIO)),
            Self::NOK => Ok(dirval!(PaymentCurrency = NOK)),
            Self::NPR => Ok(dirval!(PaymentCurrency = NPR)),
            Self::NZD => Ok(dirval!(PaymentCurrency = NZD)),
            Self::OMR => Ok(dirval!(PaymentCurrency = OMR)),
            Self::PEN => Ok(dirval!(PaymentCurrency = PEN)),
            Self::PGK => Ok(dirval!(PaymentCurrency = PGK)),
            Self::PHP => Ok(dirval!(PaymentCurrency = PHP)),
            Self::PKR => Ok(dirval!(PaymentCurrency = PKR)),
            Self::PLN => Ok(dirval!(PaymentCurrency = PLN)),
            Self::PYG => Ok(dirval!(PaymentCurrency = PYG)),
            Self::QAR => Ok(dirval!(PaymentCurrency = QAR)),
            Self::RON => Ok(dirval!(PaymentCurrency = RON)),
            Self::RUB => Ok(dirval!(PaymentCurrency = RUB)),
            Self::RWF => Ok(dirval!(PaymentCurrency = RWF)),
            Self::SAR => Ok(dirval!(PaymentCurrency = SAR)),
            Self::SCR => Ok(dirval!(PaymentCurrency = SCR)),
            Self::SEK => Ok(dirval!(PaymentCurrency = SEK)),
            Self::SGD => Ok(dirval!(PaymentCurrency = SGD)),
            Self::SLL => Ok(dirval!(PaymentCurrency = SLL)),
            Self::SOS => Ok(dirval!(PaymentCurrency = SOS)),
            Self::SSP => Ok(dirval!(PaymentCurrency = SSP)),
            Self::SVC => Ok(dirval!(PaymentCurrency = SVC)),
            Self::SZL => Ok(dirval!(PaymentCurrency = SZL)),
            Self::THB => Ok(dirval!(PaymentCurrency = THB)),
            Self::TRY => Ok(dirval!(PaymentCurrency = TRY)),
            Self::TTD => Ok(dirval!(PaymentCurrency = TTD)),
            Self::TWD => Ok(dirval!(PaymentCurrency = TWD)),
            Self::TZS => Ok(dirval!(PaymentCurrency = TZS)),
            Self::UGX => Ok(dirval!(PaymentCurrency = UGX)),
            Self::USD => Ok(dirval!(PaymentCurrency = USD)),
            Self::UYU => Ok(dirval!(PaymentCurrency = UYU)),
            Self::UZS => Ok(dirval!(PaymentCurrency = UZS)),
            Self::VND => Ok(dirval!(PaymentCurrency = VND)),
            Self::VUV => Ok(dirval!(PaymentCurrency = VUV)),
            Self::XAF => Ok(dirval!(PaymentCurrency = XAF)),
            Self::XOF => Ok(dirval!(PaymentCurrency = XOF)),
            Self::XPF => Ok(dirval!(PaymentCurrency = XPF)),
            Self::YER => Ok(dirval!(PaymentCurrency = YER)),
            Self::ZAR => Ok(dirval!(PaymentCurrency = ZAR)),
        }
    }
}

pub fn get_dir_country_dir_value(c: api_enums::Country) -> dir::enums::Country {
    match c {
        api_enums::Country::Afghanistan => dir::enums::Country::Afghanistan,
        api_enums::Country::AlandIslands => dir::enums::Country::AlandIslands,
        api_enums::Country::Albania => dir::enums::Country::Albania,
        api_enums::Country::Algeria => dir::enums::Country::Algeria,
        api_enums::Country::AmericanSamoa => dir::enums::Country::AmericanSamoa,
        api_enums::Country::Andorra => dir::enums::Country::Andorra,
        api_enums::Country::Angola => dir::enums::Country::Angola,
        api_enums::Country::Anguilla => dir::enums::Country::Anguilla,
        api_enums::Country::Antarctica => dir::enums::Country::Antarctica,
        api_enums::Country::AntiguaAndBarbuda => dir::enums::Country::AntiguaAndBarbuda,
        api_enums::Country::Argentina => dir::enums::Country::Argentina,
        api_enums::Country::Armenia => dir::enums::Country::Armenia,
        api_enums::Country::Aruba => dir::enums::Country::Aruba,
        api_enums::Country::Australia => dir::enums::Country::Australia,
        api_enums::Country::Austria => dir::enums::Country::Austria,
        api_enums::Country::Azerbaijan => dir::enums::Country::Azerbaijan,
        api_enums::Country::Bahamas => dir::enums::Country::Bahamas,
        api_enums::Country::Bahrain => dir::enums::Country::Bahrain,
        api_enums::Country::Bangladesh => dir::enums::Country::Bangladesh,
        api_enums::Country::Barbados => dir::enums::Country::Barbados,
        api_enums::Country::Belarus => dir::enums::Country::Belarus,
        api_enums::Country::Belgium => dir::enums::Country::Belgium,
        api_enums::Country::Belize => dir::enums::Country::Belize,
        api_enums::Country::Benin => dir::enums::Country::Benin,
        api_enums::Country::Bermuda => dir::enums::Country::Bermuda,
        api_enums::Country::Bhutan => dir::enums::Country::Bhutan,
        api_enums::Country::BoliviaPlurinationalState => {
            dir::enums::Country::BoliviaPlurinationalState
        }
        api_enums::Country::BonaireSintEustatiusAndSaba => {
            dir::enums::Country::BonaireSintEustatiusAndSaba
        }
        api_enums::Country::BosniaAndHerzegovina => dir::enums::Country::BosniaAndHerzegovina,
        api_enums::Country::Botswana => dir::enums::Country::Botswana,
        api_enums::Country::BouvetIsland => dir::enums::Country::BouvetIsland,
        api_enums::Country::Brazil => dir::enums::Country::Brazil,
        api_enums::Country::BritishIndianOceanTerritory => {
            dir::enums::Country::BritishIndianOceanTerritory
        }
        api_enums::Country::BruneiDarussalam => dir::enums::Country::BruneiDarussalam,
        api_enums::Country::Bulgaria => dir::enums::Country::Bulgaria,
        api_enums::Country::BurkinaFaso => dir::enums::Country::BurkinaFaso,
        api_enums::Country::Burundi => dir::enums::Country::Burundi,
        api_enums::Country::CaboVerde => dir::enums::Country::CaboVerde,
        api_enums::Country::Cambodia => dir::enums::Country::Cambodia,
        api_enums::Country::Cameroon => dir::enums::Country::Cameroon,
        api_enums::Country::Canada => dir::enums::Country::Canada,
        api_enums::Country::CaymanIslands => dir::enums::Country::CaymanIslands,
        api_enums::Country::CentralAfricanRepublic => dir::enums::Country::CentralAfricanRepublic,
        api_enums::Country::Chad => dir::enums::Country::Chad,
        api_enums::Country::Chile => dir::enums::Country::Chile,
        api_enums::Country::China => dir::enums::Country::China,
        api_enums::Country::ChristmasIsland => dir::enums::Country::ChristmasIsland,
        api_enums::Country::CocosKeelingIslands => dir::enums::Country::CocosKeelingIslands,
        api_enums::Country::Colombia => dir::enums::Country::Colombia,
        api_enums::Country::Comoros => dir::enums::Country::Comoros,
        api_enums::Country::Congo => dir::enums::Country::Congo,
        api_enums::Country::CongoDemocraticRepublic => dir::enums::Country::CongoDemocraticRepublic,
        api_enums::Country::CookIslands => dir::enums::Country::CookIslands,
        api_enums::Country::CostaRica => dir::enums::Country::CostaRica,
        api_enums::Country::CotedIvoire => dir::enums::Country::CotedIvoire,
        api_enums::Country::Croatia => dir::enums::Country::Croatia,
        api_enums::Country::Cuba => dir::enums::Country::Cuba,
        api_enums::Country::Curacao => dir::enums::Country::Curacao,
        api_enums::Country::Cyprus => dir::enums::Country::Cyprus,
        api_enums::Country::Czechia => dir::enums::Country::Czechia,
        api_enums::Country::Denmark => dir::enums::Country::Denmark,
        api_enums::Country::Djibouti => dir::enums::Country::Djibouti,
        api_enums::Country::Dominica => dir::enums::Country::Dominica,
        api_enums::Country::DominicanRepublic => dir::enums::Country::DominicanRepublic,
        api_enums::Country::Ecuador => dir::enums::Country::Ecuador,
        api_enums::Country::Egypt => dir::enums::Country::Egypt,
        api_enums::Country::ElSalvador => dir::enums::Country::ElSalvador,
        api_enums::Country::EquatorialGuinea => dir::enums::Country::EquatorialGuinea,
        api_enums::Country::Eritrea => dir::enums::Country::Eritrea,
        api_enums::Country::Estonia => dir::enums::Country::Estonia,
        api_enums::Country::Ethiopia => dir::enums::Country::Ethiopia,
        api_enums::Country::FalklandIslandsMalvinas => dir::enums::Country::FalklandIslandsMalvinas,
        api_enums::Country::FaroeIslands => dir::enums::Country::FaroeIslands,
        api_enums::Country::Fiji => dir::enums::Country::Fiji,
        api_enums::Country::Finland => dir::enums::Country::Finland,
        api_enums::Country::France => dir::enums::Country::France,
        api_enums::Country::FrenchGuiana => dir::enums::Country::FrenchGuiana,
        api_enums::Country::FrenchPolynesia => dir::enums::Country::FrenchPolynesia,
        api_enums::Country::FrenchSouthernTerritories => {
            dir::enums::Country::FrenchSouthernTerritories
        }
        api_enums::Country::Gabon => dir::enums::Country::Gabon,
        api_enums::Country::Gambia => dir::enums::Country::Gambia,
        api_enums::Country::Georgia => dir::enums::Country::Georgia,
        api_enums::Country::Germany => dir::enums::Country::Germany,
        api_enums::Country::Ghana => dir::enums::Country::Ghana,
        api_enums::Country::Gibraltar => dir::enums::Country::Gibraltar,
        api_enums::Country::Greece => dir::enums::Country::Greece,
        api_enums::Country::Greenland => dir::enums::Country::Greenland,
        api_enums::Country::Grenada => dir::enums::Country::Grenada,
        api_enums::Country::Guadeloupe => dir::enums::Country::Guadeloupe,
        api_enums::Country::Guam => dir::enums::Country::Guam,
        api_enums::Country::Guatemala => dir::enums::Country::Guatemala,
        api_enums::Country::Guernsey => dir::enums::Country::Guernsey,
        api_enums::Country::Guinea => dir::enums::Country::Guinea,
        api_enums::Country::GuineaBissau => dir::enums::Country::GuineaBissau,
        api_enums::Country::Guyana => dir::enums::Country::Guyana,
        api_enums::Country::Haiti => dir::enums::Country::Haiti,
        api_enums::Country::HeardIslandAndMcDonaldIslands => {
            dir::enums::Country::HeardIslandAndMcDonaldIslands
        }
        api_enums::Country::HolySee => dir::enums::Country::HolySee,
        api_enums::Country::Honduras => dir::enums::Country::Honduras,
        api_enums::Country::HongKong => dir::enums::Country::HongKong,
        api_enums::Country::Hungary => dir::enums::Country::Hungary,
        api_enums::Country::Iceland => dir::enums::Country::Iceland,
        api_enums::Country::India => dir::enums::Country::India,
        api_enums::Country::Indonesia => dir::enums::Country::Indonesia,
        api_enums::Country::IranIslamicRepublic => dir::enums::Country::IranIslamicRepublic,
        api_enums::Country::Iraq => dir::enums::Country::Iraq,
        api_enums::Country::Ireland => dir::enums::Country::Ireland,
        api_enums::Country::IsleOfMan => dir::enums::Country::IsleOfMan,
        api_enums::Country::Israel => dir::enums::Country::Israel,
        api_enums::Country::Italy => dir::enums::Country::Italy,
        api_enums::Country::Jamaica => dir::enums::Country::Jamaica,
        api_enums::Country::Japan => dir::enums::Country::Japan,
        api_enums::Country::Jersey => dir::enums::Country::Jersey,
        api_enums::Country::Jordan => dir::enums::Country::Jordan,
        api_enums::Country::Kazakhstan => dir::enums::Country::Kazakhstan,
        api_enums::Country::Kenya => dir::enums::Country::Kenya,
        api_enums::Country::Kiribati => dir::enums::Country::Kiribati,
        api_enums::Country::KoreaDemocraticPeoplesRepublic => {
            dir::enums::Country::KoreaDemocraticPeoplesRepublic
        }
        api_enums::Country::KoreaRepublic => dir::enums::Country::KoreaRepublic,
        api_enums::Country::Kuwait => dir::enums::Country::Kuwait,
        api_enums::Country::Kyrgyzstan => dir::enums::Country::Kyrgyzstan,
        api_enums::Country::LaoPeoplesDemocraticRepublic => {
            dir::enums::Country::LaoPeoplesDemocraticRepublic
        }
        api_enums::Country::Latvia => dir::enums::Country::Latvia,
        api_enums::Country::Lebanon => dir::enums::Country::Lebanon,
        api_enums::Country::Lesotho => dir::enums::Country::Lesotho,
        api_enums::Country::Liberia => dir::enums::Country::Liberia,
        api_enums::Country::Libya => dir::enums::Country::Libya,
        api_enums::Country::Liechtenstein => dir::enums::Country::Liechtenstein,
        api_enums::Country::Lithuania => dir::enums::Country::Lithuania,
        api_enums::Country::Luxembourg => dir::enums::Country::Luxembourg,
        api_enums::Country::Macao => dir::enums::Country::Macao,
        api_enums::Country::MacedoniaTheFormerYugoslavRepublic => {
            dir::enums::Country::MacedoniaTheFormerYugoslavRepublic
        }
        api_enums::Country::Madagascar => dir::enums::Country::Madagascar,
        api_enums::Country::Malawi => dir::enums::Country::Malawi,
        api_enums::Country::Malaysia => dir::enums::Country::Malaysia,
        api_enums::Country::Maldives => dir::enums::Country::Maldives,
        api_enums::Country::Mali => dir::enums::Country::Mali,
        api_enums::Country::Malta => dir::enums::Country::Malta,
        api_enums::Country::MarshallIslands => dir::enums::Country::MarshallIslands,
        api_enums::Country::Martinique => dir::enums::Country::Martinique,
        api_enums::Country::Mauritania => dir::enums::Country::Mauritania,
        api_enums::Country::Mauritius => dir::enums::Country::Mauritius,
        api_enums::Country::Mayotte => dir::enums::Country::Mayotte,
        api_enums::Country::Mexico => dir::enums::Country::Mexico,
        api_enums::Country::MicronesiaFederatedStates => {
            dir::enums::Country::MicronesiaFederatedStates
        }
        api_enums::Country::MoldovaRepublic => dir::enums::Country::MoldovaRepublic,
        api_enums::Country::Monaco => dir::enums::Country::Monaco,
        api_enums::Country::Mongolia => dir::enums::Country::Mongolia,
        api_enums::Country::Montenegro => dir::enums::Country::Montenegro,
        api_enums::Country::Montserrat => dir::enums::Country::Montserrat,
        api_enums::Country::Morocco => dir::enums::Country::Morocco,
        api_enums::Country::Mozambique => dir::enums::Country::Mozambique,
        api_enums::Country::Myanmar => dir::enums::Country::Myanmar,
        api_enums::Country::Namibia => dir::enums::Country::Namibia,
        api_enums::Country::Nauru => dir::enums::Country::Nauru,
        api_enums::Country::Nepal => dir::enums::Country::Nepal,
        api_enums::Country::Netherlands => dir::enums::Country::Netherlands,
        api_enums::Country::NewCaledonia => dir::enums::Country::NewCaledonia,
        api_enums::Country::NewZealand => dir::enums::Country::NewZealand,
        api_enums::Country::Nicaragua => dir::enums::Country::Nicaragua,
        api_enums::Country::Niger => dir::enums::Country::Niger,
        api_enums::Country::Nigeria => dir::enums::Country::Nigeria,
        api_enums::Country::Niue => dir::enums::Country::Niue,
        api_enums::Country::NorfolkIsland => dir::enums::Country::NorfolkIsland,
        api_enums::Country::NorthernMarianaIslands => dir::enums::Country::NorthernMarianaIslands,
        api_enums::Country::Norway => dir::enums::Country::Norway,
        api_enums::Country::Oman => dir::enums::Country::Oman,
        api_enums::Country::Pakistan => dir::enums::Country::Pakistan,
        api_enums::Country::Palau => dir::enums::Country::Palau,
        api_enums::Country::PalestineState => dir::enums::Country::PalestineState,
        api_enums::Country::Panama => dir::enums::Country::Panama,
        api_enums::Country::PapuaNewGuinea => dir::enums::Country::PapuaNewGuinea,
        api_enums::Country::Paraguay => dir::enums::Country::Paraguay,
        api_enums::Country::Peru => dir::enums::Country::Peru,
        api_enums::Country::Philippines => dir::enums::Country::Philippines,
        api_enums::Country::Pitcairn => dir::enums::Country::Pitcairn,

        api_enums::Country::Poland => dir::enums::Country::Poland,
        api_enums::Country::Portugal => dir::enums::Country::Portugal,
        api_enums::Country::PuertoRico => dir::enums::Country::PuertoRico,

        api_enums::Country::Qatar => dir::enums::Country::Qatar,
        api_enums::Country::Reunion => dir::enums::Country::Reunion,
        api_enums::Country::Romania => dir::enums::Country::Romania,
        api_enums::Country::RussianFederation => dir::enums::Country::RussianFederation,
        api_enums::Country::Rwanda => dir::enums::Country::Rwanda,
        api_enums::Country::SaintBarthelemy => dir::enums::Country::SaintBarthelemy,
        api_enums::Country::SaintHelenaAscensionAndTristandaCunha => {
            dir::enums::Country::SaintHelenaAscensionAndTristandaCunha
        }
        api_enums::Country::SaintKittsAndNevis => dir::enums::Country::SaintKittsAndNevis,
        api_enums::Country::SaintLucia => dir::enums::Country::SaintLucia,
        api_enums::Country::SaintMartinFrenchpart => dir::enums::Country::SaintMartinFrenchpart,
        api_enums::Country::SaintPierreAndMiquelon => dir::enums::Country::SaintPierreAndMiquelon,
        api_enums::Country::SaintVincentAndTheGrenadines => {
            dir::enums::Country::SaintVincentAndTheGrenadines
        }
        api_enums::Country::Samoa => dir::enums::Country::Samoa,
        api_enums::Country::SanMarino => dir::enums::Country::SanMarino,
        api_enums::Country::SaoTomeAndPrincipe => dir::enums::Country::SaoTomeAndPrincipe,
        api_enums::Country::SaudiArabia => dir::enums::Country::SaudiArabia,
        api_enums::Country::Senegal => dir::enums::Country::Senegal,
        api_enums::Country::Serbia => dir::enums::Country::Serbia,
        api_enums::Country::Seychelles => dir::enums::Country::Seychelles,
        api_enums::Country::SierraLeone => dir::enums::Country::SierraLeone,
        api_enums::Country::Singapore => dir::enums::Country::Singapore,
        api_enums::Country::SintMaartenDutchpart => dir::enums::Country::SintMaartenDutchpart,
        api_enums::Country::Slovakia => dir::enums::Country::Slovakia,
        api_enums::Country::Slovenia => dir::enums::Country::Slovenia,
        api_enums::Country::SolomonIslands => dir::enums::Country::SolomonIslands,
        api_enums::Country::Somalia => dir::enums::Country::Somalia,
        api_enums::Country::SouthAfrica => dir::enums::Country::SouthAfrica,
        api_enums::Country::SouthGeorgiaAndTheSouthSandwichIslands => {
            dir::enums::Country::SouthGeorgiaAndTheSouthSandwichIslands
        }
        api_enums::Country::SouthSudan => dir::enums::Country::SouthSudan,
        api_enums::Country::Spain => dir::enums::Country::Spain,
        api_enums::Country::SriLanka => dir::enums::Country::SriLanka,
        api_enums::Country::Sudan => dir::enums::Country::Sudan,
        api_enums::Country::Suriname => dir::enums::Country::Suriname,
        api_enums::Country::SvalbardAndJanMayen => dir::enums::Country::SvalbardAndJanMayen,
        api_enums::Country::Swaziland => dir::enums::Country::Swaziland,
        api_enums::Country::Sweden => dir::enums::Country::Sweden,
        api_enums::Country::Switzerland => dir::enums::Country::Switzerland,
        api_enums::Country::SyrianArabRepublic => dir::enums::Country::SyrianArabRepublic,
        api_enums::Country::TaiwanProvinceOfChina => dir::enums::Country::TaiwanProvinceOfChina,
        api_enums::Country::Tajikistan => dir::enums::Country::Tajikistan,
        api_enums::Country::TanzaniaUnitedRepublic => dir::enums::Country::TanzaniaUnitedRepublic,
        api_enums::Country::Thailand => dir::enums::Country::Thailand,
        api_enums::Country::TimorLeste => dir::enums::Country::TimorLeste,
        api_enums::Country::Togo => dir::enums::Country::Togo,
        api_enums::Country::Tokelau => dir::enums::Country::Tokelau,
        api_enums::Country::Tonga => dir::enums::Country::Tonga,
        api_enums::Country::TrinidadAndTobago => dir::enums::Country::TrinidadAndTobago,
        api_enums::Country::Tunisia => dir::enums::Country::Tunisia,
        api_enums::Country::Turkey => dir::enums::Country::Turkey,
        api_enums::Country::Turkmenistan => dir::enums::Country::Turkmenistan,
        api_enums::Country::TurksAndCaicosIslands => dir::enums::Country::TurksAndCaicosIslands,
        api_enums::Country::Tuvalu => dir::enums::Country::Tuvalu,
        api_enums::Country::Uganda => dir::enums::Country::Uganda,
        api_enums::Country::Ukraine => dir::enums::Country::Ukraine,
        api_enums::Country::UnitedArabEmirates => dir::enums::Country::UnitedArabEmirates,
        api_enums::Country::UnitedKingdomOfGreatBritainAndNorthernIreland => {
            dir::enums::Country::UnitedKingdomOfGreatBritainAndNorthernIreland
        }
        api_enums::Country::UnitedStatesOfAmerica => dir::enums::Country::UnitedStatesOfAmerica,
        api_enums::Country::UnitedStatesMinorOutlyingIslands => {
            dir::enums::Country::UnitedStatesMinorOutlyingIslands
        }
        api_enums::Country::Uruguay => dir::enums::Country::Uruguay,
        api_enums::Country::Uzbekistan => dir::enums::Country::Uzbekistan,
        api_enums::Country::Vanuatu => dir::enums::Country::Vanuatu,
        api_enums::Country::VenezuelaBolivarianRepublic => {
            dir::enums::Country::VenezuelaBolivarianRepublic
        }
        api_enums::Country::Vietnam => dir::enums::Country::Vietnam,
        api_enums::Country::VirginIslandsBritish => dir::enums::Country::VirginIslandsBritish,
        api_enums::Country::VirginIslandsUS => dir::enums::Country::VirginIslandsUS,
        api_enums::Country::WallisAndFutuna => dir::enums::Country::WallisAndFutuna,
        api_enums::Country::WesternSahara => dir::enums::Country::WesternSahara,
        api_enums::Country::Yemen => dir::enums::Country::Yemen,
        api_enums::Country::Zambia => dir::enums::Country::Zambia,
        api_enums::Country::Zimbabwe => dir::enums::Country::Zimbabwe,
    }
}

pub fn business_country_to_dir_value(c: api_enums::Country) -> dir::DirValue {
    dir::DirValue::BusinessCountry(get_dir_country_dir_value(c))
}

pub fn billing_country_to_dir_value(c: api_enums::Country) -> dir::DirValue {
    dir::DirValue::BillingCountry(get_dir_country_dir_value(c))
}
