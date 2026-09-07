/// Offers - Browse
///
/// Lists the offers currently available to the merchant. Offers that are not eligible are
/// excluded, so the response contains only offers that can be used.
#[utoipa::path(
    post,
    path = "/offer_engine/offers/list",
    request_body (
        content = BrowseOffersRequest,
        examples ((
            "Browse offers for a currency" = (
                value = json!({"offer_payment_info": {"currency": "USD"}})
            )
        ))
    ),
    responses(
        (status = 200, description = "Offers available to the merchant", body = BrowseOffersResponse),
        (status = 403, description = "Offer Engine is not enabled for this merchant")
    ),
    tag = "Offers",
    operation_id = "Browse offers",
    security(("api_key" = []))
)]
pub async fn offer_engine_browse_offers() {}
