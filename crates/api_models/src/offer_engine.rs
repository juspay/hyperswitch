use utoipa::ToSchema;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OfferEngineCredentialSource {
    None,
    Application,
    Merchant,
}

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

/// The Offer Engine dashboard route a proxy request is meant for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum OfferEngineDashboardFlow {
    /// List the merchant's offers.
    ListOffers,
    /// Fetch full details for one or more offers.
    OfferDetail,
    /// Create an offer.
    CreateOffer,
    /// Delete an offer.
    DeleteOffer,
    /// Change an offer's status.
    UpdateOfferStatus,
}

/// A dashboard request to be forwarded to Offer Engine. Hyperswitch authenticates the caller,
/// attaches the merchant's Offer Engine credentials and returns the response unchanged.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct OfferEngineDashboardProxyRequest {
    /// The Offer Engine route to call.
    #[schema(value_type = OfferEngineDashboardFlow, example = "list_offers")]
    pub flow: OfferEngineDashboardFlow,
    /// The offer the route acts on. Required by `delete_offer` and `update_offer_status`,
    /// ignored by the other flows.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(example = "1e4f0a3c-5f2b-4a9e-8f10-3b6c7d8e9f01")]
    pub offer_id: Option<String>,
    /// Create the offer as a draft. Only read by `create_offer`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub draft: Option<bool>,
    /// The Offer Engine request body, forwarded as-is. `merchant_id` is always set by
    /// Hyperswitch, so a value supplied here is overwritten.
    #[serde(default)]
    #[schema(value_type = Object)]
    pub payload: serde_json::Value,
}
