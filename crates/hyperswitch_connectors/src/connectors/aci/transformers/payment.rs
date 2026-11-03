//! The payment request: an authorize, or a mandate setup, which ACI runs as a zero-amount
//! authorize.
//!
//! An [`AciPayment`] holds the decisions that shape the request, each resolved once from the
//! router data and each a small strategy type of its own:
//!
//! - [`PaymentPurpose`]: a charge (`DB`, or `PA` for a manual capture) or a mandate setup (`PA`,
//!   always registering the card).
//! - [`Channel`]: e-commerce, point of sale, or mail/telephone order — account first.
//! - [`CardOnFile`]: which leg of a stored-credential agreement this is, and what it declares.
//! - [`ShopperPresence`]: whether shopper authentication (CVV, 3DS) may accompany it.
//!
//! The payment method is an adapter ([`methods::PaymentMethodAdapter`]) that renders ACI's
//! instrument fields and declares which of those rules apply to it. [`AciPayment::into_request`]
//! is the one place a request body is assembled.

mod authentication;
mod card_on_file;
mod channel;
mod merchant_data;
mod methods;
mod shopper;

use hyperswitch_domain_models::types::PaymentsAuthorizeRouterData;

pub(super) use self::{
    authentication::ShopperPresence,
    card_on_file::{recorded_series_type, CofLeg},
    channel::Channel,
    methods::card_details,
};
use self::{
    authentication::{external_three_ds, ConnectorThreeDs},
    card_on_file::CardOnFile,
    methods::PaymentMethodAdapter,
};
use super::{
    metadata::AciMetadata,
    request::{
        AciBillingAddress, AciCustomerBrowserInfo, AciCustomerData, AciPaymentType,
        AciPaymentsRequest, PaymentDetails, TransactionDetails,
    },
    AciAuthType, AciRouterData, Error,
};
use crate::utils::PaymentsAuthorizeRequestData;

/// The router data a payment request is built from.
type PaymentItem<'a> = &'a AciRouterData<&'a PaymentsAuthorizeRouterData>;

/// What the payment request is for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum PaymentPurpose {
    /// A charge, or a preauthorization for a manual capture.
    Authorize,
    /// Storing a card for later charges: a zero-amount CIT on `/v1/payments` rather than bare
    /// tokenization on `/v1/registrations`.
    ///
    /// `/v1/registrations` carries no amount, no `paymentType` and no `standingInstruction`
    /// block, so the acquirer is never told a card-on-file agreement was established and no
    /// scheme reference (CITI) comes back. Every MIT built off such a mandate then goes out
    /// without `standingInstruction.initialTransactionId`, which is the linkage Nedbank (127.22
    /// `<BANKNET_REF_NR><BANKNET_DATE>`), ABSA (`Token20.traceId`) and CyberSource all match the
    /// series on. A zero-amount preauthorization with `createRegistration=true` performs a real
    /// account verification, flags the CIT to the acquirer, and returns both the registration id
    /// and the genuine scheme reference.
    SetupMandate,
}

impl PaymentPurpose {
    /// `paymentType`.
    ///
    /// A verification is an authorization that is never captured. `is_auto_capture()` resolves a
    /// setup intent's absent capture method to `true`, which would submit `DB` and ask the
    /// acquirer to debit zero — so a setup is always `PA`. An unsupported capture method is
    /// rejected for either purpose.
    fn payment_type(self, auto_capture: bool) -> AciPaymentType {
        match (self, auto_capture) {
            (Self::Authorize, true) => AciPaymentType::Debit,
            (Self::Authorize, false) | (Self::SetupMandate, _) => AciPaymentType::Preauthorization,
        }
    }
}

/// The decisions that shape one payment request.
pub(super) struct AciPayment<'a> {
    item: PaymentItem<'a>,
    purpose: PaymentPurpose,
    metadata: AciMetadata,
    channel: Channel,
    card_on_file: CardOnFile,
    shopper: ShopperPresence,
}

impl<'a> AciPayment<'a> {
    fn new<M: PaymentMethodAdapter>(item: PaymentItem<'a>, purpose: PaymentPurpose) -> Self {
        let request = &item.router_data.request;
        let metadata =
            AciMetadata::from_connector_meta(item.router_data.connector_meta_data.as_ref());
        // A method that cannot carry an agreement is a one-off whatever the request says.
        let leg = if M::CARD_ON_FILE {
            CofLeg::of(request)
        } else {
            CofLeg::OneOff
        };
        let card_on_file = match purpose {
            PaymentPurpose::Authorize => CardOnFile::resolve(leg, item, metadata),
            PaymentPurpose::SetupMandate => CardOnFile::mandate_setup(item),
        };
        Self {
            item,
            purpose,
            metadata,
            channel: Channel::resolve(metadata, request.payment_channel.as_ref()),
            card_on_file,
            // From the request's own leg even on a setup, whose declared leg is forced.
            shopper: leg.shopper_presence(),
        }
    }

    fn item(&self) -> PaymentItem<'a> {
        self.item
    }

    fn shopper(&self) -> ShopperPresence {
        self.shopper
    }

    fn transaction_details(&self) -> Result<TransactionDetails, Error> {
        let router_data = self.item.router_data;
        let auth = AciAuthType::try_from(&router_data.connector_auth_type)?;
        let auto_capture = router_data.request.is_auto_capture()?;
        Ok(TransactionDetails {
            entity_id: auth.entity_id,
            amount: self.item.amount.to_owned(),
            currency: router_data.request.currency.to_string(),
            payment_type: self.purpose.payment_type(auto_capture),
        })
    }

    /// The request body, with `payment_method` rendered by the adapter `M`.
    fn into_request<M: PaymentMethodAdapter>(
        self,
        txn_details: TransactionDetails,
        payment_method: PaymentDetails,
    ) -> Result<AciPaymentsRequest, Error> {
        let request = &self.item.router_data.request;

        let connector_three_ds = M::CONNECTOR_THREE_DS
            .then(|| ConnectorThreeDs::of(self.item, self.shopper))
            .flatten();
        let mpgs_browser = connector_three_ds.is_some_and(|tds| tds.needs_browser(self.channel));
        if mpgs_browser {
            request.get_browser_info()?;
        }

        let recurring_type = match self.purpose {
            PaymentPurpose::Authorize if M::LEGACY_RECURRING_TYPE => {
                self.card_on_file.legacy_recurring_type()
            }
            // A setup states its agreement in the block alone: sending both states it twice, in
            // two different vocabularies.
            PaymentPurpose::Authorize | PaymentPurpose::SetupMandate => None,
        };

        Ok(AciPaymentsRequest {
            txn_details,
            instruction: self.card_on_file.instruction(self.channel),
            shopper_result_url: request.router_return_url.clone(),
            three_ds_two_enrolled: connector_three_ds.map(ConnectorThreeDs::enrolled),
            recurring_type,
            transaction_category: self.channel.category(),
            merchant_transaction_id: Some(merchant_data::merchant_transaction_id(
                self.item,
                self.metadata.trim_merchant_transaction_id,
            )),
            test_mode: self.metadata.test_mode,
            customer_browser_info: request
                .browser_info
                .as_ref()
                .map(|browser| AciCustomerBrowserInfo::of(browser, mpgs_browser))
                .unwrap_or_default(),
            customer_data: AciCustomerData::of(self.item),
            billing_address: AciBillingAddress::of(self.item),
            external_three_ds: external_three_ds(
                request.authentication_data.as_ref(),
                self.shopper,
                &payment_method,
            ),
            custom_parameters: merchant_data::custom_parameters(self.item),
            descriptor: merchant_data::descriptor(self.item),
            payment_method,
        })
    }
}

impl TryFrom<&AciRouterData<&PaymentsAuthorizeRouterData>> for AciPaymentsRequest {
    type Error = Error;
    fn try_from(item: &AciRouterData<&PaymentsAuthorizeRouterData>) -> Result<Self, Self::Error> {
        methods::resolve(item, PaymentPurpose::Authorize)
    }
}

/// The mandate setup request, from the setup reshaped as an authorize.
///
/// Known limitation, deliberately not gated: MPGS is reported to reject zero-amount transactions
/// outright. See the MPGS acquirer module.
pub fn setup_mandate_request(
    item: &AciRouterData<&PaymentsAuthorizeRouterData>,
) -> Result<AciPaymentsRequest, Error> {
    methods::resolve(item, PaymentPurpose::SetupMandate)
}
