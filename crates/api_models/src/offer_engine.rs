/// Request to browse the offers available to a merchant.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BrowseOffersRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub offer_payment_info: Option<OfferPaymentInfo>,
}

/// Order context supplied when browsing offers.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct OfferPaymentInfo {
    pub currency: common_enums::Currency,
}

/// The offers a merchant can currently use.
#[derive(Debug, Clone, serde::Serialize)]
pub struct BrowseOffersResponse {
    pub offers: Vec<BrowseOffer>,
}

/// A single offer available to the merchant.
#[derive(Debug, Clone, serde::Serialize)]
pub struct BrowseOffer {
    pub code: String,
    pub title: Option<String>,
    pub display_title: Option<String>,
    pub description: Option<String>,
    pub currency: Option<common_enums::Currency>,
    #[serde(with = "common_utils::custom_serde::iso8601::option")]
    pub valid_till: Option<time::PrimitiveDateTime>,
}
