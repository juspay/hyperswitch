use api_models::offer_engine::{BrowseOffer, BrowseOffersRequest, BrowseOffersResponse};
use common_utils::id_type;
use error_stack::ResultExt;

use super::{
    client::OfferEngineClient,
    config::resolve_offer_engine_credential_source,
    types::{BrowseOfferListRequest, OfferEngineCredentialSource, OfferStatus},
};
use crate::{
    core::{
        configs::dimension_state::Dimensions,
        errors::{self, RouterResponse},
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
    let processor = platform.get_processor();

    // Each `with_*` yields a distinct type, so the profile-scoped and merchant-scoped lookups
    // cannot share one binding.
    let dimensions = Dimensions::new()
        .with_processor_merchant_id(processor.get_processor_merchant_id())
        .with_organization_id(processor.get_account().get_org_id().clone());

    let credential_source = match profile_id {
        Some(profile_id) => {
            resolve_offer_engine_credential_source(&state, &dimensions.with_profile_id(profile_id))
                .await
        }
        None => resolve_offer_engine_credential_source(&state, &dimensions).await,
    };

    let config = match credential_source {
        OfferEngineCredentialSource::None => {
            return Err(error_stack::report!(
                errors::ApiErrorResponse::AccessForbidden {
                    resource: "offer_engine".to_string(),
                }
            ));
        }
        OfferEngineCredentialSource::Application => {
            OfferEngineCredentialSource::resolve_application_offer_config(&state)
        }
        OfferEngineCredentialSource::Merchant => {
            OfferEngineCredentialSource::resolve_merchant_offer_config(
                &state,
                processor.get_account(),
            )
        }
    }
    .change_context(errors::ApiErrorResponse::AccessForbidden {
        resource: "offer_engine".to_string(),
    })?;

    let list_request = BrowseOfferListRequest {
        currency: request.offer_payment_info.map(|info| info.currency),
    };

    let client = OfferEngineClient::new(config, &state.conf.trace_header.header_name);
    let response = client
        .browse_offers(&state, list_request)
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Offer Engine /list failed")?;

    Ok(service_api::ApplicationResponse::Json(
        BrowseOffersResponse {
            offers: response
                .offers
                .into_iter()
                .filter(|entry| entry.status == OfferStatus::Eligible)
                .map(BrowseOffer::from)
                .collect(),
        },
    ))
}
