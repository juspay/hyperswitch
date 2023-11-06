//! Analysis of the lowering logic for the DIR
//!
//! Consists of certain functions that supports the lowering logic from DIR to VIR.
//! These includes the lowering of the DIR program and vector of rules , and the lowering of ifstatements
//! ,and comparisonsLogic and also the lowering of the enums of value variants from DIR to VIR.
use super::enums;
use crate::{
    dssa::types::{AnalysisError, AnalysisErrorType},
    enums as global_enums,
    frontend::{dir, vir},
    types::EuclidValue,
};

impl From<enums::CardType> for global_enums::PaymentMethodType {
    fn from(value: enums::CardType) -> Self {
        match value {
            enums::CardType::Credit => Self::Credit,
            enums::CardType::Debit => Self::Debit,
        }
    }
}

impl From<enums::PayLaterType> for global_enums::PaymentMethodType {
    fn from(value: enums::PayLaterType) -> Self {
        match value {
            enums::PayLaterType::Affirm => Self::Affirm,
            enums::PayLaterType::AfterpayClearpay => Self::AfterpayClearpay,
            enums::PayLaterType::Alma => Self::Alma,
            enums::PayLaterType::Klarna => Self::Klarna,
            enums::PayLaterType::PayBright => Self::PayBright,
            enums::PayLaterType::Walley => Self::Walley,
            enums::PayLaterType::Atome => Self::Atome,
        }
    }
}

impl From<enums::WalletType> for global_enums::PaymentMethodType {
    fn from(value: enums::WalletType) -> Self {
        match value {
            enums::WalletType::GooglePay => Self::GooglePay,
            enums::WalletType::ApplePay => Self::ApplePay,
            enums::WalletType::Paypal => Self::Paypal,
            enums::WalletType::AliPay => Self::AliPay,
            enums::WalletType::AliPayHk => Self::AliPayHk,
            enums::WalletType::MbWay => Self::MbWay,
            enums::WalletType::MobilePay => Self::MobilePay,
            enums::WalletType::WeChatPay => Self::WeChatPay,
            enums::WalletType::SamsungPay => Self::SamsungPay,
            enums::WalletType::GoPay => Self::GoPay,
            enums::WalletType::KakaoPay => Self::KakaoPay,
            enums::WalletType::Twint => Self::Twint,
            enums::WalletType::Gcash => Self::Gcash,
            enums::WalletType::Vipps => Self::Vipps,
            enums::WalletType::Momo => Self::Momo,
            enums::WalletType::Dana => Self::Dana,
            enums::WalletType::TouchNGo => Self::TouchNGo,
            enums::WalletType::Swish => Self::Swish,
            enums::WalletType::Cashapp => Self::Cashapp,
        }
    }
}

impl From<enums::BankDebitType> for global_enums::PaymentMethodType {
    fn from(value: enums::BankDebitType) -> Self {
        match value {
            enums::BankDebitType::Ach => Self::Ach,
            enums::BankDebitType::Sepa => Self::Sepa,
            enums::BankDebitType::Bacs => Self::Bacs,
            enums::BankDebitType::Becs => Self::Becs,
        }
    }
}
impl From<enums::UpiType> for global_enums::PaymentMethodType {
    fn from(value: enums::UpiType) -> Self {
        match value {
            enums::UpiType::UpiCollect => Self::UpiCollect,
        }
    }
}

impl From<enums::VoucherType> for global_enums::PaymentMethodType {
    fn from(value: enums::VoucherType) -> Self {
        match value {
            enums::VoucherType::Boleto => Self::Boleto,
            enums::VoucherType::Efecty => Self::Efecty,
            enums::VoucherType::PagoEfectivo => Self::PagoEfectivo,
            enums::VoucherType::RedCompra => Self::RedCompra,
            enums::VoucherType::RedPagos => Self::RedPagos,
            enums::VoucherType::Alfamart => Self::Alfamart,
            enums::VoucherType::Indomaret => Self::Indomaret,
            enums::VoucherType::SevenEleven => Self::SevenEleven,
            enums::VoucherType::Lawson => Self::Lawson,
            enums::VoucherType::MiniStop => Self::MiniStop,
            enums::VoucherType::FamilyMart => Self::FamilyMart,
            enums::VoucherType::Seicomart => Self::Seicomart,
            enums::VoucherType::PayEasy => Self::PayEasy,
            enums::VoucherType::Oxxo => Self::Oxxo,
        }
    }
}

impl From<enums::BankTransferType> for global_enums::PaymentMethodType {
    fn from(value: enums::BankTransferType) -> Self {
        match value {
            enums::BankTransferType::Multibanco => Self::Multibanco,
            enums::BankTransferType::Pix => Self::Pix,
            enums::BankTransferType::Pse => Self::Pse,
            enums::BankTransferType::Ach => Self::Ach,
            enums::BankTransferType::Sepa => Self::Sepa,
            enums::BankTransferType::Bacs => Self::Bacs,
            enums::BankTransferType::BcaBankTransfer => Self::BcaBankTransfer,
            enums::BankTransferType::BniVa => Self::BniVa,
            enums::BankTransferType::BriVa => Self::BriVa,
            enums::BankTransferType::CimbVa => Self::CimbVa,
            enums::BankTransferType::DanamonVa => Self::DanamonVa,
            enums::BankTransferType::MandiriVa => Self::MandiriVa,
            enums::BankTransferType::PermataBankTransfer => Self::PermataBankTransfer,
        }
    }
}

impl From<enums::GiftCardType> for global_enums::PaymentMethodType {
    fn from(value: enums::GiftCardType) -> Self {
        match value {
            enums::GiftCardType::PaySafeCard => Self::PaySafeCard,
            enums::GiftCardType::Givex => Self::Givex,
        }
    }
}

impl From<enums::CardRedirectType> for global_enums::PaymentMethodType {
    fn from(value: enums::CardRedirectType) -> Self {
        match value {
            enums::CardRedirectType::Benefit => Self::Benefit,
            enums::CardRedirectType::Knet => Self::Knet,
            enums::CardRedirectType::MomoAtm => Self::MomoAtm,
        }
    }
}

impl From<enums::BankRedirectType> for global_enums::PaymentMethodType {
    fn from(value: enums::BankRedirectType) -> Self {
        match value {
            enums::BankRedirectType::Bizum => Self::Bizum,
            enums::BankRedirectType::Giropay => Self::Giropay,
            enums::BankRedirectType::Ideal => Self::Ideal,
            enums::BankRedirectType::Sofort => Self::Sofort,
            enums::BankRedirectType::Eps => Self::Eps,
            enums::BankRedirectType::BancontactCard => Self::BancontactCard,
            enums::BankRedirectType::Blik => Self::Blik,
            enums::BankRedirectType::Interac => Self::Interac,
            enums::BankRedirectType::OnlineBankingCzechRepublic => Self::OnlineBankingCzechRepublic,
            enums::BankRedirectType::OnlineBankingFinland => Self::OnlineBankingFinland,
            enums::BankRedirectType::OnlineBankingPoland => Self::OnlineBankingPoland,
            enums::BankRedirectType::OnlineBankingSlovakia => Self::OnlineBankingSlovakia,
            enums::BankRedirectType::OnlineBankingFpx => Self::OnlineBankingFpx,
            enums::BankRedirectType::OnlineBankingThailand => Self::OnlineBankingThailand,
            enums::BankRedirectType::OpenBankingUk => Self::OpenBankingUk,
            enums::BankRedirectType::Przelewy24 => Self::Przelewy24,
            enums::BankRedirectType::Trustly => Self::Trustly,
        }
    }
}

impl From<enums::CryptoType> for global_enums::PaymentMethodType {
    fn from(value: enums::CryptoType) -> Self {
        match value {
            enums::CryptoType::CryptoCurrency => Self::CryptoCurrency,
        }
    }
}

impl From<enums::RewardType> for global_enums::PaymentMethodType {
    fn from(value: enums::RewardType) -> Self {
        match value {
            enums::RewardType::ClassicReward => Self::ClassicReward,
            enums::RewardType::Evoucher => Self::Evoucher,
        }
    }
}

/// Analyses of the lowering of the DirValues to EuclidValues .
///
/// For example,
/// ```notrust
/// DirValue::PaymentMethod::Cards -> EuclidValue::PaymentMethod::Cards
/// ```notrust
/// This is a function that lowers the Values of the DIR variants into the Value of the VIR variants.
/// The function for each DirValue variant creates a corresponding EuclidValue variants and if there
/// lacks any direct mapping, it return an Error.
fn lower_value(dir_value: dir::DirValue) -> Result<EuclidValue, AnalysisErrorType> {
    Ok(match dir_value {
        dir::DirValue::PaymentMethod(pm) => EuclidValue::PaymentMethod(pm),
        dir::DirValue::CardBin(ci) => EuclidValue::CardBin(ci),
        dir::DirValue::CardType(ct) => EuclidValue::PaymentMethodType(ct.into()),
        dir::DirValue::CardNetwork(cn) => EuclidValue::CardNetwork(cn),
        dir::DirValue::MetaData(md) => EuclidValue::Metadata(md),
        dir::DirValue::PayLaterType(plt) => EuclidValue::PaymentMethodType(plt.into()),
        dir::DirValue::WalletType(wt) => EuclidValue::PaymentMethodType(wt.into()),
        dir::DirValue::UpiType(ut) => EuclidValue::PaymentMethodType(ut.into()),
        dir::DirValue::VoucherType(vt) => EuclidValue::PaymentMethodType(vt.into()),
        dir::DirValue::BankTransferType(btt) => EuclidValue::PaymentMethodType(btt.into()),
        dir::DirValue::GiftCardType(gct) => EuclidValue::PaymentMethodType(gct.into()),
        dir::DirValue::CardRedirectType(crt) => EuclidValue::PaymentMethodType(crt.into()),
        dir::DirValue::BankRedirectType(brt) => EuclidValue::PaymentMethodType(brt.into()),
        dir::DirValue::CryptoType(ct) => EuclidValue::PaymentMethodType(ct.into()),
        dir::DirValue::AuthenticationType(at) => EuclidValue::AuthenticationType(at),
        dir::DirValue::CaptureMethod(cm) => EuclidValue::CaptureMethod(cm),
        dir::DirValue::PaymentAmount(pa) => EuclidValue::PaymentAmount(pa),
        dir::DirValue::PaymentCurrency(pc) => EuclidValue::PaymentCurrency(pc),
        dir::DirValue::BusinessCountry(buc) => EuclidValue::BusinessCountry(buc),
        dir::DirValue::BillingCountry(bic) => EuclidValue::BillingCountry(bic),
        dir::DirValue::MandateAcceptanceType(mat) => EuclidValue::MandateAcceptanceType(mat),
        dir::DirValue::MandateType(mt) => EuclidValue::MandateType(mt),
        dir::DirValue::PaymentType(pt) => EuclidValue::PaymentType(pt),
        dir::DirValue::Connector(_) => Err(AnalysisErrorType::UnsupportedProgramKey(
            dir::DirKeyKind::Connector,
        ))?,
        dir::DirValue::BankDebitType(bdt) => EuclidValue::PaymentMethodType(bdt.into()),
        dir::DirValue::RewardType(rt) => EuclidValue::PaymentMethodType(rt.into()),
        dir::DirValue::BusinessLabel(bl) => EuclidValue::BusinessLabel(bl),
        dir::DirValue::SetupFutureUsage(sfu) => EuclidValue::SetupFutureUsage(sfu),
    })
}

fn lower_comparison(
    dir_comparison: dir::DirComparison,
) -> Result<vir::ValuedComparison, AnalysisErrorType> {
    Ok(vir::ValuedComparison {
        values: dir_comparison
            .values
            .into_iter()
            .map(lower_value)
            .collect::<Result<_, _>>()?,
        logic: match dir_comparison.logic {
            dir::DirComparisonLogic::NegativeConjunction => {
                vir::ValuedComparisonLogic::NegativeConjunction
            }
            dir::DirComparisonLogic::PositiveDisjunction => {
                vir::ValuedComparisonLogic::PositiveDisjunction
            }
        },
        metadata: dir_comparison.metadata,
    })
}

fn lower_if_statement(
    dir_if_statement: dir::DirIfStatement,
) -> Result<vir::ValuedIfStatement, AnalysisErrorType> {
    Ok(vir::ValuedIfStatement {
        condition: dir_if_statement
            .condition
            .into_iter()
            .map(lower_comparison)
            .collect::<Result<_, _>>()?,
        nested: dir_if_statement
            .nested
            .map(|v| {
                v.into_iter()
                    .map(lower_if_statement)
                    .collect::<Result<_, _>>()
            })
            .transpose()?,
    })
}

fn lower_rule<O>(dir_rule: dir::DirRule<O>) -> Result<vir::ValuedRule<O>, AnalysisErrorType> {
    Ok(vir::ValuedRule {
        name: dir_rule.name,
        connector_selection: dir_rule.connector_selection,
        statements: dir_rule
            .statements
            .into_iter()
            .map(lower_if_statement)
            .collect::<Result<_, _>>()?,
    })
}

pub fn lower_program<O>(
    dir_program: dir::DirProgram<O>,
) -> Result<vir::ValuedProgram<O>, AnalysisError> {
    Ok(vir::ValuedProgram {
        default_selection: dir_program.default_selection,
        rules: dir_program
            .rules
            .into_iter()
            .map(lower_rule)
            .collect::<Result<_, _>>()
            .map_err(|e| AnalysisError {
                error_type: e,
                metadata: Default::default(),
            })?,
        metadata: dir_program.metadata,
    })
}
