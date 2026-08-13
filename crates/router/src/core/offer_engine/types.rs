use common_utils::types::StringMajorUnit;
use hyperswitch_masking::Secret;

use super::amount;

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    serde::Serialize,
    serde::Deserialize,
    strum::Display,
    strum::EnumString,
)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum OfferEngineCredentialSource {
    None,
    Application,
}

#[derive(Debug, Clone)]
pub struct ResolvedOfferEngineConfig {
    pub base_url: url::Url,
    pub api_key: Secret<String>,
    pub merchant_id: String,
}

#[derive(Debug, thiserror::Error)]
pub enum OfferEngineError {
    #[error("Offer Engine application config is missing or invalid: {0}")]
    MissingApplicationConfig(String),
    #[error("Offer Engine request failed")]
    RequestFailed,
    #[error("Failed to parse Offer Engine response")]
    ResponseParseFailed,
    #[error("Offer Engine amount conversion failed")]
    AmountConversion,
    #[error("Offer Engine /apply rejected the offer")]
    ApplyRejected,
}

/// Offer Engine order type.
#[derive(Debug, Clone, serde::Serialize)]
pub enum OfferOrderType {
    #[serde(rename = "ORDER_PAYMENT")]
    OrderPayment,
}

/// Order context sent to Offer Engine.
#[derive(Debug, Clone, serde::Serialize)]
pub struct OfferOrder {
    pub order_id: String,
    pub merchant_id: String,
    pub amount: StringMajorUnit,
    pub currency: common_enums::Currency,
    pub order_type: OfferOrderType,
}

/// Payment-method attributes sent to Offer Engine.
#[derive(Debug, Clone, serde::Serialize)]
pub struct OfferPaymentMethodInfo {
    pub payment_method_reference: String,
    pub payment_method_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payment_method: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub card_bin: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub card_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bank_code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub card_country: Option<String>,
}

/// Customer sent to Offer Engine.
#[derive(Debug, Clone, serde::Serialize)]
pub struct OfferCustomer {
    pub id: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize)]
pub enum OfferStatus {
    #[serde(rename = "ELIGIBLE")]
    Eligible,
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct OfferListRequest {
    pub order: OfferOrder,
    pub payment_method_info: Vec<OfferPaymentMethodInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub customer: Option<OfferCustomer>,
}

/// Response body for `/offers/list`.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct OfferListResponse {
    #[serde(default)]
    pub best_offer_combinations: Vec<OfferCombination>,
    #[serde(default)]
    pub offers: Vec<OfferListEntry>,
}

/// A candidate offer combination.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct OfferCombination {
    #[serde(default)]
    pub offers: Vec<OfferCombinationEntry>,
}

/// An offer inside a combination.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct OfferCombinationEntry {
    pub offer_id: String,
    pub discount_amount: StringMajorUnit,
}

/// A per-offer entry in `offers[]`.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct OfferListEntry {
    pub offer_id: String,
    pub status: OfferStatus,
    // Offer Engine always returns a code (mandatory `Text` in the `/list` response).
    pub offer_code: String,
    #[serde(default)]
    pub offer_description: Option<OfferDescription>,
}

/// Nested `offer_description` display copy.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct OfferDescription {
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
}

/// Request body for `/offers/apply`.
#[derive(Debug, Clone, serde::Serialize)]
pub struct OfferApplyRequest {
    pub txn_id: String,
    pub offers: Vec<String>,
    pub order: OfferOrder,
    pub payment_method_info: OfferPaymentMethodInfo,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub customer: Option<OfferCustomer>,
}

/// Response body for `/offers/apply`.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct OfferApplyResponse {
    #[serde(default)]
    pub offers: Vec<AppliedOfferEntry>,
}

/// An applied offer with its order breakup.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct AppliedOfferEntry {
    pub offer_id: String,
    pub status: OfferStatus,
    #[serde(default)]
    pub order_breakup: Option<OfferOrderBreakup>,
}

/// Per-offer order breakup.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct OfferOrderBreakup {
    pub discount_amount: StringMajorUnit,
}

// Selection helpers

/// A selected offer resolved from `/list` (discount in minor units).
#[derive(Debug, Clone)]
pub struct SelectedOffer {
    pub offer_id: String,
    pub offer_amount: common_utils::types::MinorUnit,
    pub currency: common_enums::Currency,
    pub code: String,
    pub title: Option<String>,
    pub description: Option<String>,
}

impl OfferListResponse {
    /// First-launch selection: `best_offer_combinations[0]` must hold exactly one
    /// `ELIGIBLE` offer. Returns `Ok(None)` when there's no supported offer.
    pub fn select_best_offer(
        &self,
        order_amount: common_utils::types::MinorUnit,
        currency: common_enums::Currency,
    ) -> error_stack::Result<Option<SelectedOffer>, OfferEngineError> {
        let Some(best) = self.best_offer_combinations.first() else {
            return Ok(None);
        };

        // First launch supports single-offer combinations only.
        let [combination_offer] = best.offers.as_slice() else {
            return Ok(None);
        };

        let list_entry = self
            .offers
            .iter()
            .find(|entry| entry.offer_id == combination_offer.offer_id);

        let Some(list_entry) = list_entry else {
            return Ok(None);
        };

        if list_entry.status != OfferStatus::Eligible {
            return Ok(None);
        }

        // Offer Engine settles the order at `orderAmount - discountAmount`;
        // `merchantDiscountAmount` is merchant-funded (display only), so the
        // customer's charge is reduced by `discountAmount` alone.
        let offer_amount =
            amount::from_offer_engine_amount(&combination_offer.discount_amount, currency)?;

        // Net amount after the offer must be positive; otherwise it isn't a
        // supported offer (same rule enforced at `/apply`).
        if order_amount <= offer_amount {
            return Ok(None);
        }

        Ok(Some(SelectedOffer {
            offer_id: combination_offer.offer_id.clone(),
            offer_amount,
            currency,
            code: list_entry.offer_code.clone(),
            title: list_entry
                .offer_description
                .as_ref()
                .and_then(|description| description.title.clone()),
            description: list_entry
                .offer_description
                .as_ref()
                .and_then(|description| description.description.clone()),
        }))
    }
}

/// An applied offer resolved from `/apply`.
#[derive(Debug, Clone)]
pub struct AppliedOffer {
    pub offer_id: String,
    pub offer_amount: common_utils::types::MinorUnit,
}

impl OfferApplyResponse {
    /// Extract the single `ELIGIBLE` applied offer (discount in minor units).
    /// Quote consistency is enforced by the confirm caller.
    pub fn single_applied_offer(
        &self,
        currency: common_enums::Currency,
    ) -> error_stack::Result<AppliedOffer, OfferEngineError> {
        let [applied] = self.offers.as_slice() else {
            return Err(error_stack::report!(OfferEngineError::ApplyRejected)
                .attach_printable("Offer Engine /apply did not return exactly one applied offer"));
        };

        common_utils::fp_utils::when(applied.status != OfferStatus::Eligible, || {
            Err(error_stack::report!(OfferEngineError::ApplyRejected)
                .attach_printable("Offer Engine /apply returned a non-eligible offer"))
        })?;

        let order_breakup = applied.order_breakup.as_ref().ok_or_else(|| {
            error_stack::report!(OfferEngineError::ApplyRejected)
                .attach_printable("Offer Engine /apply response is missing order_breakup")
        })?;
        // Charge reduction is `discountAmount` only (see `select_best_offer`).
        let offer_amount =
            amount::from_offer_engine_amount(&order_breakup.discount_amount, currency)?;

        Ok(AppliedOffer {
            offer_id: applied.offer_id.clone(),
            offer_amount,
        })
    }
}
