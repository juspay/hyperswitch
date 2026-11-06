//! Bank redirects: the shopper completes the payment at their bank.

use hyperswitch_domain_models::{
    payment_method_data::BankRedirectData, types::PaymentsAuthorizeRouterData,
};
use hyperswitch_interfaces::errors;
use hyperswitch_masking::Secret;

use super::{unsupported_variant, AciPayment, PaymentMethodAdapter};
use crate::{
    connectors::aci::transformers::{
        request::{BankRedirectionPMData, PaymentBrand, PaymentDetails},
        Error,
    },
    utils::RouterData as _,
};

pub(in crate::connectors::aci::transformers::payment) struct BankRedirect<'a>(
    pub(in crate::connectors::aci::transformers::payment) &'a BankRedirectData,
);

impl PaymentMethodAdapter for BankRedirect<'_> {
    const CARD_ON_FILE: bool = false;

    /// The bank account's country is the shopper's billing country, for the brands that take
    /// one. `customer.email`, `billing.country`, `merchantTransactionId` and
    /// `customer.merchantCustomerId` are sent once, from the shared request fields: repeating
    /// them here would duplicate the form keys.
    fn instrument(self, payment: &AciPayment<'_>) -> Result<PaymentDetails, Error> {
        let router_data = payment.item().router_data;
        let in_billing_country =
            |brand| BankRedirectionPMData::in_billing_country(brand, router_data);
        let account = match self.0 {
            BankRedirectData::Eps { .. } => in_billing_country(PaymentBrand::Eps)?,
            BankRedirectData::Eft { .. } => in_billing_country(PaymentBrand::Eft)?,
            BankRedirectData::Sofort { .. } => {
                in_billing_country(PaymentBrand::Sofortueberweisung)?
            }
            BankRedirectData::Interac { .. } => in_billing_country(PaymentBrand::InteracOnline)?,
            BankRedirectData::Giropay {
                bank_account_bic,
                bank_account_iban,
                ..
            } => in_billing_country(PaymentBrand::Giropay)?
                .with_account(bank_account_bic.clone(), bank_account_iban.clone()),
            BankRedirectData::Ideal { bank_name, .. } => {
                let account = in_billing_country(PaymentBrand::Ideal)?;
                account.at_bank(
                    bank_name.ok_or(errors::ConnectorError::MissingRequiredField {
                        field_name: "ideal.bank_name".into(),
                    })?,
                )
            }
            BankRedirectData::Przelewy24 { .. } => {
                BankRedirectionPMData::new(PaymentBrand::Przelewy)
            }
            BankRedirectData::Trustly { .. } => {
                // Trustly is the one brand that requires `customer.merchantCustomerId`, so it is
                // still demanded here — just not carried here.
                router_data.get_customer_id()?;
                BankRedirectionPMData::new(PaymentBrand::Trustly)
            }
            BankRedirectData::Bizum { .. }
            | BankRedirectData::Blik { .. }
            | BankRedirectData::BancontactCard { .. }
            | BankRedirectData::OnlineBankingCzechRepublic { .. }
            | BankRedirectData::OnlineBankingFinland { .. }
            | BankRedirectData::OnlineBankingFpx { .. }
            | BankRedirectData::OnlineBankingPoland { .. }
            | BankRedirectData::OnlineBankingSlovakia { .. }
            | BankRedirectData::OnlineBankingThailand { .. }
            | BankRedirectData::LocalBankRedirect {}
            | BankRedirectData::OpenBankingUk { .. }
            | BankRedirectData::OpenBanking { .. } => return Err(unsupported_variant()),
        };
        Ok(PaymentDetails::BankRedirect(Box::new(account)))
    }
}

impl BankRedirectionPMData {
    fn new(payment_brand: PaymentBrand) -> Self {
        Self {
            payment_brand,
            bank_account_country: None,
            bank_account_bank_name: None,
            bank_account_bic: None,
            bank_account_iban: None,
        }
    }

    /// An account in the shopper's billing country.
    fn in_billing_country(
        payment_brand: PaymentBrand,
        router_data: &PaymentsAuthorizeRouterData,
    ) -> Result<Self, Error> {
        Ok(Self {
            bank_account_country: Some(router_data.get_billing_country()?),
            ..Self::new(payment_brand)
        })
    }

    fn at_bank(self, bank_name: common_enums::BankNames) -> Self {
        Self {
            bank_account_bank_name: Some(bank_name),
            ..self
        }
    }

    fn with_account(self, bic: Option<Secret<String>>, iban: Option<Secret<String>>) -> Self {
        Self {
            bank_account_bic: bic,
            bank_account_iban: iban,
            ..self
        }
    }
}
