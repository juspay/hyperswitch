use utoipa::ToSchema;

/// Request to browse the offers available to a merchant.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct BrowseOffersRequest {
    /// Order context. Omit to browse without one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub offer_payment_info: Option<OfferPaymentInfo>,
}

/// Order context supplied when browsing offers.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct OfferPaymentInfo {
    /// Order currency.
    #[schema(value_type = Currency, example = "USD")]
    pub currency: common_enums::Currency,
}

/// The offers a merchant can currently use.
#[derive(Debug, Clone, serde::Serialize, ToSchema)]
pub struct BrowseOffersResponse {
    pub offers: Vec<BrowseOffer>,
}

/// A single offer available to the merchant.
#[derive(Debug, Clone, serde::Serialize, ToSchema)]
pub struct BrowseOffer {
    /// Offer code, as configured in Offer Engine.
    #[schema(example = "TESTHS")]
    pub code: String,
    /// Full offer title.
    #[schema(example = "Test offer HS")]
    pub title: Option<String>,
    /// Shortened title, for constrained layouts.
    #[schema(example = "TSH")]
    pub display_title: Option<String>,
    /// Offer description.
    #[schema(example = "Testing offers")]
    pub description: Option<String>,
    /// Currency the offer applies to.
    #[schema(value_type = Option<Currency>, example = "USD")]
    pub currency: Option<common_enums::Currency>,
    /// When the offer stops being valid.
    #[serde(with = "common_utils::custom_serde::iso8601::option")]
    #[schema(value_type = Option<PrimitiveDateTime>, example = "2026-08-31T18:29:59.000Z")]
    pub valid_till: Option<time::PrimitiveDateTime>,
}
