use api_models::card_issuer as api_types;
use common_utils::date_time;
use diesel_models::card_issuer::{NewCardIssuer, UpdateCardIssuer};
use router_env::{instrument, tracing};

use crate::{
    consts::{ALPHABETS, CARD_ISSUER_ID_LENGTH},
    core::errors::{self, RouterResponse, StorageErrorExt},
    routes::SessionState,
    services::ApplicationResponse,
    types::transformers::ForeignInto,
};

#[instrument(skip_all)]
pub async fn add_card_issuer(
    state: SessionState,
    body: api_types::CardIssuerRequest,
) -> RouterResponse<api_types::CardIssuerResponse> {
    let now = date_time::now();
    let new = NewCardIssuer {
        id: nanoid::nanoid!(CARD_ISSUER_ID_LENGTH, &ALPHABETS),
        issuer_name: body.issuer_name,
        created_at: now,
        last_modified_at: now,
    };

    let issuer = state
        .store
        .insert_card_issuer(new)
        .await
        .to_duplicate_response(errors::ApiErrorResponse::GenericDuplicateError {
            message: "A card issuer with this name already exists".to_string(),
        })?;

    Ok(ApplicationResponse::Json(issuer.foreign_into()))
}

#[instrument(skip_all)]
pub async fn update_card_issuer(
    state: SessionState,
    id: String,
    body: api_types::CardIssuerUpdateRequest,
) -> RouterResponse<api_types::CardIssuerResponse> {
    let update = UpdateCardIssuer {
        issuer_name: body.issuer_name,
        last_modified_at: date_time::now(),
    };

    let issuer = state
        .store
        .update_card_issuer(id.clone(), update)
        .await
        .to_not_found_response(errors::ApiErrorResponse::GenericNotFoundError {
            message: format!("Card issuer with id {id} not found"),
        })?;

    Ok(ApplicationResponse::Json(issuer.foreign_into()))
}

#[instrument(skip_all)]
pub async fn list_card_issuers(
    state: SessionState,
    query: api_types::CardIssuerListQuery,
) -> RouterResponse<api_types::CardIssuerListResponse> {
    let issuers = state
        .store
        .list_card_issuers(query.query, Some(query.limit))
        .await
        .to_not_found_response(errors::ApiErrorResponse::GenericNotFoundError {
            message: "Failed to fetch card issuers".to_string(),
        })?;

    Ok(ApplicationResponse::Json(
        api_types::CardIssuerListResponse {
            issuers: issuers.into_iter().map(ForeignInto::foreign_into).collect(),
        },
    ))
}
