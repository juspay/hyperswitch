use api_models::offer_engine::{BrowseOffer, BrowseOffersRequest, BrowseOffersResponse};
use common_utils::id_type;
use error_stack::ResultExt;

use super::{
    client::OfferEngineClient,
    config::resolve_offer_engine_credential_source,
    types::{
        BrowseOfferListEntry, BrowseOfferListRequest, BrowseOfferOrder,
        OfferEngineCredentialSource, OfferStatus, ResolvedOfferEngineConfig,
    },
};
use crate::{
    core::{
        configs::dimension_state::Dimensions,
        errors::{self, RouterResponse, RouterResult},
    },
    routes::SessionState,
    services::api as service_api,
    types::domain,
};

/// Lists the offers a merchant can currently use. Ineligible offers are dropped, so the caller
/// sees only what is usable.
pub async fn browse_offers(
    state: SessionState,
    platform: domain::Platform,
    profile_id: Option<id_type::ProfileId>,
    request: BrowseOffersRequest,
) -> RouterResponse<BrowseOffersResponse> {
    let config = resolve_offer_config(&state, &platform, profile_id).await?;

    let list_request = BrowseOfferListRequest {
        order: BrowseOfferOrder {
            merchant_id: config.merchant_id.clone(),
            currency: request.offer_payment_info.map(|info| info.currency),
        },
    };

    let client = OfferEngineClient::new(config, &state.conf.trace_header.header_name);
    let response = client
        .browse_offers(&state, list_request)
        .await
        // No gateway-level variant exists on `ApiErrorResponse`, and `ConnectorError` would
        // mislabel this as a payment connector failure, so an unreachable Offer Engine surfaces
        // as a 500 — the same treatment the eligibility flow gives it.
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Offer Engine /list failed")?;

    Ok(service_api::ApplicationResponse::Json(
        BrowseOffersResponse {
            offers: response
                .offers
                .into_iter()
                .filter(|entry| entry.status == OfferStatus::Eligible)
                .map(to_browse_offer)
                .collect(),
        },
    ))
}

/// Resolves the Offer Engine credentials for the merchant behind the request. Anything that
/// leaves us without a usable config means Offer Engine is not available to them.
async fn resolve_offer_config(
    state: &SessionState,
    platform: &domain::Platform,
    profile_id: Option<id_type::ProfileId>,
) -> RouterResult<ResolvedOfferEngineConfig> {
    let processor = platform.get_processor();

    // Each `with_*` yields a distinct type, so the profile-scoped and merchant-scoped lookups
    // cannot share one binding.
    let dimensions = Dimensions::new()
        .with_processor_merchant_id(processor.get_processor_merchant_id())
        .with_organization_id(processor.get_account().get_org_id().clone());

    let credential_source = match profile_id {
        Some(profile_id) => {
            resolve_offer_engine_credential_source(state, &dimensions.with_profile_id(profile_id))
                .await
        }
        None => resolve_offer_engine_credential_source(state, &dimensions).await,
    };

    match credential_source {
        OfferEngineCredentialSource::None => {
            return Err(error_stack::report!(
                errors::ApiErrorResponse::AccessForbidden {
                    resource: "offer_engine".to_string(),
                }
            ));
        }
        OfferEngineCredentialSource::Application => {
            OfferEngineCredentialSource::resolve_application_offer_config(state)
        }
        OfferEngineCredentialSource::Merchant => {
            OfferEngineCredentialSource::resolve_merchant_offer_config(
                state,
                processor.get_account(),
            )
        }
    }
    .change_context(errors::ApiErrorResponse::AccessForbidden {
        resource: "offer_engine".to_string(),
    })
}

fn to_browse_offer(entry: BrowseOfferListEntry) -> BrowseOffer {
    let (title, description) = entry.offer_description.map_or((None, None), |description| {
        (description.title, description.description)
    });

    BrowseOffer {
        code: entry.offer_code,
        title,
        display_title: entry.display_title,
        description,
        currency: entry.currency,
        valid_till: entry.valid_till,
    }
}
