use api_models::user::dashboard_metadata::{self as api, GetMultipleMetaDataPayload};
use diesel_models::{
    enums::DashboardMetadata as DBEnum, user::dashboard_metadata::DashboardMetadata,
};
use error_stack::ResultExt;

use crate::{
    core::errors::{UserErrors, UserResponse, UserResult},
    routes::AppState,
    services::{authentication::UserFromToken, ApplicationResponse},
    types::domain::{user::dashboard_metadata as types, MerchantKeyStore},
    utils::user::dashboard_metadata as utils,
};

/// Asynchronously sets the metadata for a user based on the provided request and user token. 
/// 
/// # Arguments
/// 
/// * `state` - The application state
/// * `user` - The user information obtained from the authentication token
/// * `request` - The request containing the metadata to be set
/// 
/// # Returns
/// 
/// Returns a `UserResponse` indicating the success or failure of the operation.
pub async fn set_metadata(
    state: AppState,
    user: UserFromToken,
    request: api::SetMetaDataRequest,
) -> UserResponse<()> {
    let metadata_value = parse_set_request(request)?;
    let metadata_key = DBEnum::from(&metadata_value);

    insert_metadata(&state, user, metadata_key, metadata_value).await?;

    Ok(ApplicationResponse::StatusOk)
}

/// Retrieves multiple metadata entries based on the provided keys from the database,
/// and returns a vector of responses containing the requested metadata.
pub async fn get_multiple_metadata(
    state: AppState,
    user: UserFromToken,
    request: GetMultipleMetaDataPayload,
) -> UserResponse<Vec<api::GetMetaDataResponse>> {
    let metadata_keys: Vec<DBEnum> = request.results.into_iter().map(parse_get_request).collect();

    let metadata = fetch_metadata(&state, &user, metadata_keys.clone()).await?;

    let mut response = Vec::with_capacity(metadata_keys.len());
    for key in metadata_keys {
        let data = metadata.iter().find(|ele| ele.data_key == key);
        let resp;
        if data.is_none() && utils::is_backfill_required(&key) {
            let backfill_data = backfill_metadata(&state, &user, &key).await?;
            resp = into_response(backfill_data.as_ref(), &key)?;
        } else {
            resp = into_response(data, &key)?;
        }
        response.push(resp);
    }

    Ok(ApplicationResponse::Json(response))
}

/// Parses the given SetMetaDataRequest enum and returns the corresponding types::MetaData value.
fn parse_set_request(data_enum: api::SetMetaDataRequest) -> UserResult<types::MetaData> {
    match data_enum {
        api::SetMetaDataRequest::ProductionAgreement(req) => {
            let ip_address = req
                .ip_address
                .ok_or(UserErrors::InternalServerError.into())
                .attach_printable("Error Getting Ip Address")?;
            Ok(types::MetaData::ProductionAgreement(
                types::ProductionAgreementValue {
                    version: req.version,
                    ip_address,
                    timestamp: common_utils::date_time::now(),
                },
            ))
        }
        api::SetMetaDataRequest::SetupProcessor(req) => Ok(types::MetaData::SetupProcessor(req)),
        api::SetMetaDataRequest::ConfigureEndpoint => Ok(types::MetaData::ConfigureEndpoint(true)),
        api::SetMetaDataRequest::SetupComplete => Ok(types::MetaData::SetupComplete(true)),
        api::SetMetaDataRequest::FirstProcessorConnected(req) => {
            Ok(types::MetaData::FirstProcessorConnected(req))
        }
        api::SetMetaDataRequest::SecondProcessorConnected(req) => {
            Ok(types::MetaData::SecondProcessorConnected(req))
        }
        api::SetMetaDataRequest::ConfiguredRouting(req) => {
            Ok(types::MetaData::ConfiguredRouting(req))
        }
        api::SetMetaDataRequest::TestPayment(req) => Ok(types::MetaData::TestPayment(req)),
        api::SetMetaDataRequest::IntegrationMethod(req) => {
            Ok(types::MetaData::IntegrationMethod(req))
        }
        api::SetMetaDataRequest::ConfigurationType(req) => {
            Ok(types::MetaData::ConfigurationType(req))
        }
        api::SetMetaDataRequest::IntegrationCompleted => {
            Ok(types::MetaData::IntegrationCompleted(true))
        }
        api::SetMetaDataRequest::SPRoutingConfigured(req) => {
            Ok(types::MetaData::SPRoutingConfigured(req))
        }
        api::SetMetaDataRequest::Feedback(req) => Ok(types::MetaData::Feedback(req)),
        api::SetMetaDataRequest::ProdIntent(req) => Ok(types::MetaData::ProdIntent(req)),
        api::SetMetaDataRequest::SPTestPayment => Ok(types::MetaData::SPTestPayment(true)),
        api::SetMetaDataRequest::DownloadWoocom => Ok(types::MetaData::DownloadWoocom(true)),
        api::SetMetaDataRequest::ConfigureWoocom => Ok(types::MetaData::ConfigureWoocom(true)),
        api::SetMetaDataRequest::SetupWoocomWebhook => {
            Ok(types::MetaData::SetupWoocomWebhook(true))
        }
        api::SetMetaDataRequest::IsMultipleConfiguration => {
            Ok(types::MetaData::IsMultipleConfiguration(true))
        }
    }
}

/// Parses the given API GetMetaDataRequest enum and returns the corresponding DBEnum value.
fn parse_get_request(data_enum: api::GetMetaDataRequest) -> DBEnum {
    match data_enum {
        api::GetMetaDataRequest::ProductionAgreement => DBEnum::ProductionAgreement,
        api::GetMetaDataRequest::SetupProcessor => DBEnum::SetupProcessor,
        api::GetMetaDataRequest::ConfigureEndpoint => DBEnum::ConfigureEndpoint,
        api::GetMetaDataRequest::SetupComplete => DBEnum::SetupComplete,
        api::GetMetaDataRequest::FirstProcessorConnected => DBEnum::FirstProcessorConnected,
        api::GetMetaDataRequest::SecondProcessorConnected => DBEnum::SecondProcessorConnected,
        api::GetMetaDataRequest::ConfiguredRouting => DBEnum::ConfiguredRouting,
        api::GetMetaDataRequest::TestPayment => DBEnum::TestPayment,
        api::GetMetaDataRequest::IntegrationMethod => DBEnum::IntegrationMethod,
        api::GetMetaDataRequest::ConfigurationType => DBEnum::ConfigurationType,
        api::GetMetaDataRequest::IntegrationCompleted => DBEnum::IntegrationCompleted,
        api::GetMetaDataRequest::StripeConnected => DBEnum::StripeConnected,
        api::GetMetaDataRequest::PaypalConnected => DBEnum::PaypalConnected,
        api::GetMetaDataRequest::SPRoutingConfigured => DBEnum::SpRoutingConfigured,
        api::GetMetaDataRequest::Feedback => DBEnum::Feedback,
        api::GetMetaDataRequest::ProdIntent => DBEnum::ProdIntent,
        api::GetMetaDataRequest::SPTestPayment => DBEnum::SpTestPayment,
        api::GetMetaDataRequest::DownloadWoocom => DBEnum::DownloadWoocom,
        api::GetMetaDataRequest::ConfigureWoocom => DBEnum::ConfigureWoocom,
        api::GetMetaDataRequest::SetupWoocomWebhook => DBEnum::SetupWoocomWebhook,
        api::GetMetaDataRequest::IsMultipleConfiguration => DBEnum::IsMultipleConfiguration,
    }
}

/// Converts the provided data and data type into an appropriate response for the API.
fn into_response(
    data: Option<&DashboardMetadata>,
    data_type: &DBEnum,
) -> UserResult<api::GetMetaDataResponse> {
    match data_type {
        DBEnum::ProductionAgreement => Ok(api::GetMetaDataResponse::ProductionAgreement(
            data.is_some(),
        )),
        DBEnum::SetupProcessor => {
            let resp = utils::deserialize_to_response(data)?;
            Ok(api::GetMetaDataResponse::SetupProcessor(resp))
        }
        DBEnum::ConfigureEndpoint => {
            Ok(api::GetMetaDataResponse::ConfigureEndpoint(data.is_some()))
        }
        DBEnum::SetupComplete => Ok(api::GetMetaDataResponse::SetupComplete(data.is_some())),
        DBEnum::FirstProcessorConnected => {
            let resp = utils::deserialize_to_response(data)?;
            Ok(api::GetMetaDataResponse::FirstProcessorConnected(resp))
        }
        DBEnum::SecondProcessorConnected => {
            let resp = utils::deserialize_to_response(data)?;
            Ok(api::GetMetaDataResponse::SecondProcessorConnected(resp))
        }
        DBEnum::ConfiguredRouting => {
            let resp = utils::deserialize_to_response(data)?;
            Ok(api::GetMetaDataResponse::ConfiguredRouting(resp))
        }
        DBEnum::TestPayment => {
            let resp = utils::deserialize_to_response(data)?;
            Ok(api::GetMetaDataResponse::TestPayment(resp))
        }
        DBEnum::IntegrationMethod => {
            let resp = utils::deserialize_to_response(data)?;
            Ok(api::GetMetaDataResponse::IntegrationMethod(resp))
        }
        DBEnum::ConfigurationType => {
            let resp = utils::deserialize_to_response(data)?;
            Ok(api::GetMetaDataResponse::ConfigurationType(resp))
        }
        DBEnum::IntegrationCompleted => Ok(api::GetMetaDataResponse::IntegrationCompleted(
            data.is_some(),
        )),
        DBEnum::StripeConnected => {
            let resp = utils::deserialize_to_response(data)?;
            Ok(api::GetMetaDataResponse::StripeConnected(resp))
        }
        DBEnum::PaypalConnected => {
            let resp = utils::deserialize_to_response(data)?;
            Ok(api::GetMetaDataResponse::PaypalConnected(resp))
        }
        DBEnum::SpRoutingConfigured => {
            let resp = utils::deserialize_to_response(data)?;
            Ok(api::GetMetaDataResponse::SPRoutingConfigured(resp))
        }
        DBEnum::Feedback => {
            let resp = utils::deserialize_to_response(data)?;
            Ok(api::GetMetaDataResponse::Feedback(resp))
        }
        DBEnum::ProdIntent => {
            let resp = utils::deserialize_to_response(data)?;
            Ok(api::GetMetaDataResponse::ProdIntent(resp))
        }
        DBEnum::SpTestPayment => Ok(api::GetMetaDataResponse::SPTestPayment(data.is_some())),
        DBEnum::DownloadWoocom => Ok(api::GetMetaDataResponse::DownloadWoocom(data.is_some())),
        DBEnum::ConfigureWoocom => Ok(api::GetMetaDataResponse::ConfigureWoocom(data.is_some())),
        DBEnum::SetupWoocomWebhook => {
            Ok(api::GetMetaDataResponse::SetupWoocomWebhook(data.is_some()))
        }

        DBEnum::IsMultipleConfiguration => Ok(api::GetMetaDataResponse::IsMultipleConfiguration(
            data.is_some(),
        )),
    }
}

/// Inserts metadata into the database based on the type of metadata and performs necessary operations if update is required.
async fn insert_metadata(
    state: &AppState,
    user: UserFromToken,
    metadata_key: DBEnum,
    metadata_value: types::MetaData,
) -> UserResult<DashboardMetadata> {
    match metadata_value {
        types::MetaData::ProductionAgreement(data) => {
            utils::insert_merchant_scoped_metadata_to_db(
                state,
                user.user_id,
                user.merchant_id,
                user.org_id,
                metadata_key,
                data,
            )
            .await
        }
        types::MetaData::SetupProcessor(data) => {
            utils::insert_merchant_scoped_metadata_to_db(
                state,
                user.user_id,
                user.merchant_id,
                user.org_id,
                metadata_key,
                data,
            )
            .await
        }
        types::MetaData::ConfigureEndpoint(data) => {
            utils::insert_merchant_scoped_metadata_to_db(
                state,
                user.user_id,
                user.merchant_id,
                user.org_id,
                metadata_key,
                data,
            )
            .await
        }
        types::MetaData::SetupComplete(data) => {
            utils::insert_merchant_scoped_metadata_to_db(
                state,
                user.user_id,
                user.merchant_id,
                user.org_id,
                metadata_key,
                data,
            )
            .await
        }
        types::MetaData::FirstProcessorConnected(data) => {
            utils::insert_merchant_scoped_metadata_to_db(
                state,
                user.user_id,
                user.merchant_id,
                user.org_id,
                metadata_key,
                data,
            )
            .await
        }
        types::MetaData::SecondProcessorConnected(data) => {
            utils::insert_merchant_scoped_metadata_to_db(
                state,
                user.user_id,
                user.merchant_id,
                user.org_id,
                metadata_key,
                data,
            )
            .await
        }
        types::MetaData::ConfiguredRouting(data) => {
            utils::insert_merchant_scoped_metadata_to_db(
                state,
                user.user_id,
                user.merchant_id,
                user.org_id,
                metadata_key,
                data,
            )
            .await
        }
        types::MetaData::TestPayment(data) => {
            utils::insert_merchant_scoped_metadata_to_db(
                state,
                user.user_id,
                user.merchant_id,
                user.org_id,
                metadata_key,
                data,
            )
            .await
        }
        types::MetaData::IntegrationMethod(data) => {
            let mut metadata = utils::insert_merchant_scoped_metadata_to_db(
                state,
                user.user_id.clone(),
                user.merchant_id.clone(),
                user.org_id.clone(),
                metadata_key,
                data.clone(),
            )
            .await;

            if utils::is_update_required(&metadata) {
                metadata = utils::update_merchant_scoped_metadata(
                    state,
                    user.user_id,
                    user.merchant_id,
                    user.org_id,
                    metadata_key,
                    data,
                )
                .await
                .change_context(UserErrors::InternalServerError);
            }
            metadata
        }
        types::MetaData::ConfigurationType(data) => {
            let mut metadata = utils::insert_merchant_scoped_metadata_to_db(
                state,
                user.user_id.clone(),
                user.merchant_id.clone(),
                user.org_id.clone(),
                metadata_key,
                data.clone(),
            )
            .await;

            if utils::is_update_required(&metadata) {
                metadata = utils::update_merchant_scoped_metadata(
                    state,
                    user.user_id,
                    user.merchant_id,
                    user.org_id,
                    metadata_key,
                    data,
                )
                .await
                .change_context(UserErrors::InternalServerError);
            }
            metadata
        }
        types::MetaData::IntegrationCompleted(data) => {
            utils::insert_merchant_scoped_metadata_to_db(
                state,
                user.user_id,
                user.merchant_id,
                user.org_id,
                metadata_key,
                data,
            )
            .await
        }
        types::MetaData::StripeConnected(data) => {
            utils::insert_merchant_scoped_metadata_to_db(
                state,
                user.user_id,
                user.merchant_id,
                user.org_id,
                metadata_key,
                data,
            )
            .await
        }
        types::MetaData::PaypalConnected(data) => {
            utils::insert_merchant_scoped_metadata_to_db(
                state,
                user.user_id,
                user.merchant_id,
                user.org_id,
                metadata_key,
                data,
            )
            .await
        }
        types::MetaData::SPRoutingConfigured(data) => {
            utils::insert_merchant_scoped_metadata_to_db(
                state,
                user.user_id,
                user.merchant_id,
                user.org_id,
                metadata_key,
                data,
            )
            .await
        }
        types::MetaData::Feedback(data) => {
            let mut metadata = utils::insert_user_scoped_metadata_to_db(
                state,
                user.user_id.clone(),
                user.merchant_id.clone(),
                user.org_id.clone(),
                metadata_key,
                data.clone(),
            )
            .await;

            if utils::is_update_required(&metadata) {
                metadata = utils::update_user_scoped_metadata(
                    state,
                    user.user_id,
                    user.merchant_id,
                    user.org_id,
                    metadata_key,
                    data,
                )
                .await
                .change_context(UserErrors::InternalServerError);
            }
            metadata
        }
        types::MetaData::ProdIntent(data) => {
            let mut metadata = utils::insert_user_scoped_metadata_to_db(
                state,
                user.user_id.clone(),
                user.merchant_id.clone(),
                user.org_id.clone(),
                metadata_key,
                data.clone(),
            )
            .await;

            if utils::is_update_required(&metadata) {
                metadata = utils::update_user_scoped_metadata(
                    state,
                    user.user_id,
                    user.merchant_id,
                    user.org_id,
                    metadata_key,
                    data,
                )
                .await
                .change_context(UserErrors::InternalServerError);
            }
            metadata
        }
        types::MetaData::SPTestPayment(data) => {
            utils::insert_merchant_scoped_metadata_to_db(
                state,
                user.user_id,
                user.merchant_id,
                user.org_id,
                metadata_key,
                data,
            )
            .await
        }
        types::MetaData::DownloadWoocom(data) => {
            utils::insert_merchant_scoped_metadata_to_db(
                state,
                user.user_id,
                user.merchant_id,
                user.org_id,
                metadata_key,
                data,
            )
            .await
        }
        types::MetaData::ConfigureWoocom(data) => {
            utils::insert_merchant_scoped_metadata_to_db(
                state,
                user.user_id,
                user.merchant_id,
                user.org_id,
                metadata_key,
                data,
            )
            .await
        }
        types::MetaData::SetupWoocomWebhook(data) => {
            utils::insert_merchant_scoped_metadata_to_db(
                state,
                user.user_id,
                user.merchant_id,
                user.org_id,
                metadata_key,
                data,
            )
            .await
        }
        types::MetaData::IsMultipleConfiguration(data) => {
            utils::insert_merchant_scoped_metadata_to_db(
                state,
                user.user_id,
                user.merchant_id,
                user.org_id,
                metadata_key,
                data,
            )
            .await
        }
    }
}

/// Asynchronously fetches metadata for the user's dashboard based on the provided metadata keys.
async fn fetch_metadata(
    state: &AppState,
    user: &UserFromToken,
    metadata_keys: Vec<DBEnum>,
) -> UserResult<Vec<DashboardMetadata>> {
    let mut dashboard_metadata = Vec::with_capacity(metadata_keys.len());
    let (merchant_scoped_enums, user_scoped_enums) =
        utils::separate_metadata_type_based_on_scope(metadata_keys);

    if !merchant_scoped_enums.is_empty() {
        let mut res = utils::get_merchant_scoped_metadata_from_db(
            state,
            user.merchant_id.to_owned(),
            user.org_id.to_owned(),
            merchant_scoped_enums,
        )
        .await?;
        dashboard_metadata.append(&mut res);
    }

    if !user_scoped_enums.is_empty() {
        let mut res = utils::get_user_scoped_metadata_from_db(
            state,
            user.user_id.to_owned(),
            user.merchant_id.to_owned(),
            user.org_id.to_owned(),
            user_scoped_enums,
        )
        .await?;
        dashboard_metadata.append(&mut res);
    }

    Ok(dashboard_metadata)
}

/// This method backfills the metadata for a user based on the key type provided. It retrieves the key store for the user's merchant ID, then based on the key type (StripeConnected or PaypalConnected), it retrieves the corresponding connector account and inserts the metadata into the database. If the key type is not recognized, it returns None. The method returns a Result containing an Option of DashboardMetadata or an error if the operation fails.
pub async fn backfill_metadata(
    state: &AppState,
    user: &UserFromToken,
    key: &DBEnum,
) -> UserResult<Option<DashboardMetadata>> {
    let key_store = state
        .store
        .get_merchant_key_store_by_merchant_id(
            &user.merchant_id,
            &state.store.get_master_key().to_vec().into(),
        )
        .await
        .change_context(UserErrors::InternalServerError)?;

    match key {
        DBEnum::StripeConnected => {
            let mca = if let Some(stripe_connected) = get_merchant_connector_account_by_name(
                state,
                &user.merchant_id,
                api_models::enums::RoutableConnectors::Stripe
                    .to_string()
                    .as_str(),
                &key_store,
            )
            .await?
            {
                stripe_connected
            } else if let Some(stripe_test_connected) = get_merchant_connector_account_by_name(
                state,
                &user.merchant_id,
                //TODO: Use Enum with proper feature flag
                "stripe_test",
                &key_store,
            )
            .await?
            {
                stripe_test_connected
            } else {
                return Ok(None);
            };

            Some(
                insert_metadata(
                    state,
                    user.to_owned(),
                    DBEnum::StripeConnected,
                    types::MetaData::StripeConnected(api::ProcessorConnected {
                        processor_id: mca.merchant_connector_id,
                        processor_name: mca.connector_name,
                    }),
                )
                .await,
            )
            .transpose()
        }
        DBEnum::PaypalConnected => {
            let mca = if let Some(paypal_connected) = get_merchant_connector_account_by_name(
                state,
                &user.merchant_id,
                api_models::enums::RoutableConnectors::Paypal
                    .to_string()
                    .as_str(),
                &key_store,
            )
            .await?
            {
                paypal_connected
            } else if let Some(paypal_test_connected) = get_merchant_connector_account_by_name(
                state,
                &user.merchant_id,
                //TODO: Use Enum with proper feature flag
                "paypal_test",
                &key_store,
            )
            .await?
            {
                paypal_test_connected
            } else {
                return Ok(None);
            };

            Some(
                insert_metadata(
                    state,
                    user.to_owned(),
                    DBEnum::PaypalConnected,
                    types::MetaData::PaypalConnected(api::ProcessorConnected {
                        processor_id: mca.merchant_connector_id,
                        processor_name: mca.connector_name,
                    }),
                )
                .await,
            )
            .transpose()
        }
        _ => Ok(None),
    }
}

/// Retrieves a merchant connector account by the specified merchant ID, connector name, and key store. Returns a result containing an optional MerchantConnectorAccount. If successful, the method will retrieve the merchant connector account from the store and return it. If an error occurs during the retrieval process, an internal server error will be returned with a printable error message indicating a database error fetching DashboardMetaData.
pub async fn get_merchant_connector_account_by_name(
    state: &AppState,
    merchant_id: &str,
    connector_name: &str,
    key_store: &MerchantKeyStore,
) -> UserResult<Option<crate::types::domain::MerchantConnectorAccount>> {
    state
        .store
        .find_merchant_connector_account_by_merchant_id_connector_name(
            merchant_id,
            connector_name,
            key_store,
        )
        .await
        .map_err(|e| {
            e.change_context(UserErrors::InternalServerError)
                .attach_printable("DB Error Fetching DashboardMetaData")
        })
        .map(|data| data.first().cloned())
}
