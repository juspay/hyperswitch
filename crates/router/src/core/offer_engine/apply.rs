use common_utils::{errors::CustomResult, id_type, types::MinorUnit};

use super::{
    client::OfferEngineClient,
    types::{
        OfferApplyRequest, OfferCustomer, OfferEngineError, OfferOrder, OfferOrderType,
        OfferPaymentMethodInfo, ResolvedOfferEngineConfig,
    },
};
use crate::routes::SessionState;

#[derive(Debug, Clone)]
pub struct OfferQuoteRef {
    pub offer_id: String,
    pub offer_amount: MinorUnit,
    pub currency: common_enums::Currency,
}

/// Non-sensitive context needed to build an Offer Engine `/apply` request.
pub struct OfferApplyContext {
    /// Hyperswitch payment id, used as the Offer Engine order id.
    pub payment_id: id_type::PaymentId,
    /// Current payment attempt id, used as the Offer Engine `txn_id`.
    pub attempt_id: String,
    /// Order amount (not surcharge-inclusive), in minor units.
    pub order_amount: MinorUnit,
    /// Order currency.
    pub currency: common_enums::Currency,
    /// Customer id, if available.
    pub customer_id: Option<id_type::CustomerId>,
    /// Offer Engine payment method type (e.g. `CARD`).
    pub payment_method_type: String,
    /// Card network, if available.
    pub payment_method: Option<String>,
    /// Card BIN/IIN, if available.
    pub card_bin: Option<String>,
    /// Card type, if available.
    pub card_type: Option<String>,
    /// Issuing bank code, if available.
    pub bank_code: Option<String>,
    /// Card issuing country, if available.
    pub card_country: Option<String>,
    /// Card fingerprint for once-per-card offer velocity, if available.
    pub card_alias: Option<String>,
}

#[derive(Debug, Clone)]
pub struct AppliedOfferResult {
    pub offer_id: String,
    pub offer_amount: MinorUnit,
    pub currency: common_enums::Currency,
    pub offer_engine_merchant_id: String,
    pub offer_engine_txn_id: String,
}

pub async fn run_offer_apply(
    state: &SessionState,
    config: ResolvedOfferEngineConfig,
    quote: &OfferQuoteRef,
    ctx: OfferApplyContext,
) -> CustomResult<AppliedOfferResult, OfferEngineError> {
    common_utils::fp_utils::when(quote.currency != ctx.currency, || {
        Err(error_stack::report!(OfferEngineError::ApplyRejected)
            .attach_printable("Offer quote currency does not match payment currency"))
    })?;

    let order = OfferOrder {
        order_id: ctx.payment_id.get_string_repr().to_string(),
        merchant_id: config.merchant_id.clone(),
        amount: super::amount::to_offer_engine_amount(ctx.order_amount, ctx.currency)?,
        currency: ctx.currency,
        order_type: OfferOrderType::OrderPayment,
    };

    let request = OfferApplyRequest {
        txn_id: ctx.attempt_id.clone(),
        offers: vec![quote.offer_id.clone()],
        order,
        payment_method_info: OfferPaymentMethodInfo {
            payment_method_reference: ctx.attempt_id.clone(),
            payment_method_type: ctx.payment_method_type.clone(),
            payment_method: ctx.payment_method.clone(),
            card_bin: ctx.card_bin.clone(),
            card_type: ctx.card_type.clone(),
            bank_code: ctx.bank_code.clone(),
            card_country: ctx.card_country.clone(),
            card_alias: ctx.card_alias.clone(),
        },
        customer: ctx.customer_id.map(|id| OfferCustomer {
            id: id.get_string_repr().to_string(),
        }),
    };

    let merchant_id = config.merchant_id.clone();
    let client = OfferEngineClient::new(config, &state.conf.trace_header.header_name);
    let response = client.apply_offer(state, request).await?;

    let applied = response.single_applied_offer(ctx.currency)?;

    common_utils::fp_utils::when(applied.offer_id != quote.offer_id, || {
        Err(error_stack::report!(OfferEngineError::ApplyRejected)
            .attach_printable("Applied offer id does not match the eligibility quote"))
    })?;
    common_utils::fp_utils::when(applied.offer_amount != quote.offer_amount, || {
        Err(error_stack::report!(OfferEngineError::ApplyRejected)
            .attach_printable("Applied offer amount does not match the eligibility quote"))
    })?;
    common_utils::fp_utils::when(ctx.order_amount <= applied.offer_amount, || {
        Err(error_stack::report!(OfferEngineError::ApplyRejected)
            .attach_printable("Offer-adjusted net amount is not positive"))
    })?;

    Ok(AppliedOfferResult {
        offer_id: applied.offer_id,
        offer_amount: applied.offer_amount,
        currency: ctx.currency,
        offer_engine_merchant_id: merchant_id,
        offer_engine_txn_id: ctx.attempt_id,
    })
}
