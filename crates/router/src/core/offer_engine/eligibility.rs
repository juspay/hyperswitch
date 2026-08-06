use common_utils::{errors::CustomResult, id_type, types::MinorUnit};

use super::{
    amount,
    client::OfferEngineClient,
    types::{
        OfferCustomer, OfferEngineError, OfferListRequest, OfferOrder, OfferOrderType,
        OfferPaymentMethodInfo, ResolvedOfferEngineConfig, SelectedOffer,
    },
};
use crate::routes::SessionState;

pub struct OfferEligibilityContext {
    /// Hyperswitch payment id, used as the Offer Engine order id.
    pub payment_id: id_type::PaymentId,
    /// Order amount (not surcharge-inclusive), in minor units.
    pub order_amount: MinorUnit,
    /// Order currency.
    pub currency: common_enums::Currency,
    /// Customer id, if available.
    pub customer_id: Option<id_type::CustomerId>,
    /// Reference for the payment method (active attempt id, or payment id).
    pub payment_method_reference: String,
    /// Offer Engine payment method type (e.g. `CARD`).
    pub payment_method_type: String,
    /// Card network, if available.
    pub payment_method: Option<String>,
    /// Card BIN/IIN, if available.
    pub card_bin: Option<String>,
    /// Card type (e.g. `CREDIT`), if available.
    pub card_type: Option<String>,
    /// Issuing bank code, if available.
    pub bank_code: Option<String>,
    /// Card issuing country, if available.
    pub card_country: Option<String>,
}

impl OfferEligibilityContext {
    fn to_payment_method_info(&self) -> OfferPaymentMethodInfo {
        OfferPaymentMethodInfo {
            payment_method_reference: self.payment_method_reference.clone(),
            payment_method_type: self.payment_method_type.clone(),
            payment_method: self.payment_method.clone(),
            card_bin: self.card_bin.clone(),
            card_type: self.card_type.clone(),
            bank_code: self.bank_code.clone(),
            card_country: self.card_country.clone(),
        }
    }

    fn to_order(&self, merchant_id: &str) -> CustomResult<OfferOrder, OfferEngineError> {
        Ok(OfferOrder {
            order_id: self.payment_id.get_string_repr().to_string(),
            merchant_id: merchant_id.to_string(),
            amount: amount::to_offer_engine_amount(self.order_amount, self.currency)?,
            currency: self.currency,
            order_type: OfferOrderType::OrderPayment,
        })
    }

    fn to_customer(&self) -> Option<OfferCustomer> {
        self.customer_id.as_ref().map(|id| OfferCustomer {
            id: id.get_string_repr().to_string(),
        })
    }
}

pub async fn run_offer_eligibility(
    state: &SessionState,
    config: ResolvedOfferEngineConfig,
    ctx: OfferEligibilityContext,
) -> CustomResult<Option<SelectedOffer>, OfferEngineError> {
    let request = OfferListRequest {
        order: ctx.to_order(&config.merchant_id)?,
        payment_method_info: vec![ctx.to_payment_method_info()],
        customer: ctx.to_customer(),
    };

    let client = OfferEngineClient::new(config, &state.conf.trace_header.header_name);
    let response = client.list_offers(state, request).await?;
    response.select_best_offer(ctx.order_amount, ctx.currency)
}
