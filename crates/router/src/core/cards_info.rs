use actix_multipart::form::{bytes::Bytes, MultipartForm};
use api_models::cards_info as cards_info_api_types;
use common_utils::fp_utils::when;
use csv::Reader;
use diesel_models::cards_info as card_info_models;
use error_stack::{report, ResultExt};
use rdkafka::message::ToBytes;
use router_env::{instrument, tracing};

use crate::{
    core::{
        errors::{self, RouterResponse, RouterResult, StorageErrorExt},
        payments::helpers,
    },
    db::cards_info::CardsInfoInterface,
    routes,
    services::ApplicationResponse,
    types::{
        domain,
        transformers::{ForeignFrom, ForeignInto},
    },
};

fn verify_iin_length(card_iin: &str) -> Result<(), errors::ApiErrorResponse> {
    let is_bin_length_in_range = card_iin.len() == 6 || card_iin.len() == 8;
    when(!is_bin_length_in_range, || {
        Err(errors::ApiErrorResponse::InvalidCardIinLength)
    })
}

#[instrument(skip_all)]
pub async fn retrieve_card_info(
    state: routes::SessionState,
    merchant_account: domain::MerchantAccount,
    key_store: domain::MerchantKeyStore,
    request: cards_info_api_types::CardsInfoRequest,
) -> RouterResponse<cards_info_api_types::CardInfoResponse> {
    let db = state.store.as_ref();

    verify_iin_length(&request.card_iin)?;
    helpers::verify_payment_intent_time_and_client_secret(
        &state,
        &merchant_account,
        &key_store,
        request.client_secret,
    )
    .await?;

    let card_info = db
        .get_card_info(&request.card_iin)
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to retrieve card information")?
        .ok_or(report!(errors::ApiErrorResponse::InvalidCardIin))?;

    Ok(ApplicationResponse::Json(
        cards_info_api_types::CardInfoResponse::foreign_from(card_info),
    ))
}

#[instrument(skip_all)]
pub async fn create_card_info(
    state: routes::SessionState,
    card_info_request: cards_info_api_types::CardInfoCreateRequest,
) -> RouterResponse<cards_info_api_types::CardInfoResponse> {
    let db = state.store.as_ref();
    CardsInfoInterface::add_card_info(db, card_info_request.foreign_into())
        .await
        .to_duplicate_response(errors::ApiErrorResponse::GenericDuplicateError {
            message: "CardInfo with given key already exists in our records".to_string(),
        })
        .map(|card_info| ApplicationResponse::Json(card_info.foreign_into()))
}

#[instrument(skip_all)]
pub async fn update_card_info(
    state: routes::SessionState,
    card_info_request: cards_info_api_types::CardInfoUpdateRequest,
) -> RouterResponse<cards_info_api_types::CardInfoResponse> {
    let db = state.store.as_ref();
    CardsInfoInterface::update_card_info(
        db,
        card_info_request.card_iin,
        card_info_models::UpdateCardInfo {
            card_issuer: card_info_request.card_issuer,
            card_network: card_info_request.card_network,
            card_type: card_info_request.card_type,
            card_subtype: card_info_request.card_subtype,
            card_issuing_country: card_info_request.card_issuing_country,
            bank_code_id: card_info_request.bank_code_id,
            bank_code: card_info_request.bank_code,
            country_code: card_info_request.country_code,
            last_updated: Some(common_utils::date_time::now()),
            last_updated_provider: card_info_request.last_updated_provider,
        },
    )
    .await
    .to_not_found_response(errors::ApiErrorResponse::GenericNotFoundError {
        message: "Card info with given key does not exist in our records".to_string(),
    })
    .attach_printable("Failed while updating card info")
    .map(|card_info| ApplicationResponse::Json(card_info.foreign_into()))
}

#[derive(Debug, MultipartForm)]
pub struct CardsInfoUpdateForm {
    #[multipart(limit = "1MB")]
    pub file: Bytes,
}

fn parse_cards_bin_csv(
    data: &[u8],
) -> csv::Result<Vec<cards_info_api_types::CardInfoUpdateRequest>> {
    let mut csv_reader = Reader::from_reader(data);
    let mut records = Vec::new();
    let mut id_counter = 0;
    for result in csv_reader.deserialize() {
        let mut record: cards_info_api_types::CardInfoUpdateRequest = result?;
        id_counter += 1;
        record.line_number = Some(id_counter);
        records.push(record);
    }
    Ok(records)
}

pub fn get_cards_bin_records(
    form: CardsInfoUpdateForm,
) -> Result<Vec<cards_info_api_types::CardInfoUpdateRequest>, errors::ApiErrorResponse> {
    match parse_cards_bin_csv(form.file.data.to_bytes()) {
        Ok(records) => Ok(records),
        Err(e) => Err(errors::ApiErrorResponse::PreconditionFailed {
            message: e.to_string(),
        }),
    }
}

#[instrument(skip_all)]
pub async fn migrate_cards_info(
    state: routes::SessionState,
    card_info_records: Vec<cards_info_api_types::CardInfoUpdateRequest>,
) -> RouterResponse<Vec<cards_info_api_types::CardInfoMigrationResponse>> {
    let mut result = Vec::new();
    for record in card_info_records {
        let res = card_info_flow(record.clone(), state.clone()).await;
        result.push(cards_info_api_types::CardInfoMigrationResponse::from((
            match res {
                Ok(ApplicationResponse::Json(response)) => Ok(response),
                Err(e) => Err(e.to_string()),
                _ => Err("Failed to migrate card info".to_string()),
            },
            record,
        )));
    }
    Ok(ApplicationResponse::Json(result))
}

pub trait State {}
pub trait TransitionTo<S: State> {}
// Available states for card info migration
pub struct CardInfoFetch;
pub struct CardInfoAdd;
pub struct CardInfoUpdate;
pub struct CardInfoResponse;

impl State for CardInfoFetch {}
impl State for CardInfoAdd {}
impl State for CardInfoUpdate {}
impl State for CardInfoResponse {}

// State transitions for card info migration
impl TransitionTo<CardInfoAdd> for CardInfoFetch {}
impl TransitionTo<CardInfoUpdate> for CardInfoFetch {}
impl TransitionTo<CardInfoResponse> for CardInfoAdd {}
impl TransitionTo<CardInfoResponse> for CardInfoUpdate {}

// Async executor
pub struct CardInfoMigrateExecutor<'a> {
    state: &'a routes::SessionState,
    record: &'a cards_info_api_types::CardInfoUpdateRequest,
}

impl<'a> CardInfoMigrateExecutor<'a> {
    fn new(
        state: &'a routes::SessionState,
        record: &'a cards_info_api_types::CardInfoUpdateRequest,
    ) -> Self {
        Self { state, record }
    }

    async fn fetch_card_info(&self) -> RouterResult<Option<card_info_models::CardInfo>> {
        let db = self.state.store.as_ref();
        let maybe_card_info = db
            .get_card_info(&self.record.card_iin)
            .await
            .change_context(errors::ApiErrorResponse::InvalidCardIin)?;
        Ok(maybe_card_info)
    }

    async fn add_card_info(&self) -> RouterResult<card_info_models::CardInfo> {
        let db = self.state.store.as_ref();
        let card_info = CardsInfoInterface::add_card_info(db, self.record.clone().foreign_into())
            .await
            .to_duplicate_response(errors::ApiErrorResponse::GenericDuplicateError {
                message: "CardInfo with given key already exists in our records".to_string(),
            })?;
        Ok(card_info)
    }

    async fn update_card_info(&self) -> RouterResult<card_info_models::CardInfo> {
        let db = self.state.store.as_ref();
        let card_info = CardsInfoInterface::update_card_info(
            db,
            self.record.card_iin.clone(),
            card_info_models::UpdateCardInfo {
                card_issuer: self.record.card_issuer.clone(),
                card_network: self.record.card_network.clone(),
                card_type: self.record.card_type.clone(),
                card_subtype: self.record.card_subtype.clone(),
                card_issuing_country: self.record.card_issuing_country.clone(),
                bank_code_id: self.record.bank_code_id.clone(),
                bank_code: self.record.bank_code.clone(),
                country_code: self.record.country_code.clone(),
                last_updated: Some(common_utils::date_time::now()),
                last_updated_provider: self.record.last_updated_provider.clone(),
            },
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::GenericNotFoundError {
            message: "Card info with given key does not exist in our records".to_string(),
        })
        .attach_printable("Failed while updating card info")?;
        Ok(card_info)
    }
}

// Builder
pub struct CardInfoBuilder<S: State> {
    state: std::marker::PhantomData<S>,
    pub card_info: Option<card_info_models::CardInfo>,
}

impl CardInfoBuilder<CardInfoFetch> {
    fn new() -> Self {
        Self {
            state: std::marker::PhantomData,
            card_info: None,
        }
    }
}

impl CardInfoBuilder<CardInfoFetch> {
    fn set_card_info(
        self,
        card_info: card_info_models::CardInfo,
    ) -> CardInfoBuilder<CardInfoUpdate> {
        CardInfoBuilder {
            state: std::marker::PhantomData,
            card_info: Some(card_info),
        }
    }

    fn transition(self) -> CardInfoBuilder<CardInfoAdd> {
        CardInfoBuilder {
            state: std::marker::PhantomData,
            card_info: None,
        }
    }
}

impl CardInfoBuilder<CardInfoUpdate> {
    fn set_updated_card_info(
        self,
        card_info: card_info_models::CardInfo,
    ) -> CardInfoBuilder<CardInfoResponse> {
        CardInfoBuilder {
            state: std::marker::PhantomData,
            card_info: Some(card_info),
        }
    }
}

impl CardInfoBuilder<CardInfoAdd> {
    fn set_added_card_info(
        self,
        card_info: card_info_models::CardInfo,
    ) -> CardInfoBuilder<CardInfoResponse> {
        CardInfoBuilder {
            state: std::marker::PhantomData,
            card_info: Some(card_info),
        }
    }
}

impl CardInfoBuilder<CardInfoResponse> {
    pub fn build(self) -> cards_info_api_types::CardInfoMigrateResponseRecord {
        match self.card_info {
            Some(card_info) => cards_info_api_types::CardInfoMigrateResponseRecord {
                card_iin: Some(card_info.card_iin),
                card_issuer: card_info.card_issuer,
                card_network: card_info.card_network.map(|cn| cn.to_string()),
                card_type: card_info.card_type,
                card_sub_type: card_info.card_subtype,
                card_issuing_country: card_info.card_issuing_country,
            },
            None => cards_info_api_types::CardInfoMigrateResponseRecord {
                card_iin: None,
                card_issuer: None,
                card_network: None,
                card_type: None,
                card_sub_type: None,
                card_issuing_country: None,
            },
        }
    }
}

async fn card_info_flow(
    record: cards_info_api_types::CardInfoUpdateRequest,
    state: routes::SessionState,
) -> RouterResponse<cards_info_api_types::CardInfoMigrateResponseRecord> {
    let builder = CardInfoBuilder::new();
    let executor = CardInfoMigrateExecutor::new(&state, &record);
    let fetched_card_info_details = executor.fetch_card_info().await?;

    let builder = match fetched_card_info_details {
        Some(card_info) => {
            let builder = builder.set_card_info(card_info);
            let updated_card_info = executor.update_card_info().await?;
            builder.set_updated_card_info(updated_card_info)
        }
        None => {
            let builder = builder.transition();
            let added_card_info = executor.add_card_info().await?;
            builder.set_added_card_info(added_card_info)
        }
    };

    Ok(ApplicationResponse::Json(builder.build()))
}
