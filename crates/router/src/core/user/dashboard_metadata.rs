use api_models::user::dashboard_metadata::{self as api, GetMultipleMetaDataPayload};
// #[cfg(feature = "email")]
use common_enums::EntityType;
use diesel_models::{
    enums::DashboardMetadata as DBEnum,
    user::dashboard_metadata::{DashboardMetadata, DashboardMetadataNew, DashboardMetadataUpdate},
};
use error_stack::{report, ResultExt};
use hyperswitch_interfaces::crm::CrmPayload;
#[cfg(feature = "email")]
use masking::ExposeInterface;
use masking::{PeekInterface, Secret};
use router_env::logger;

use crate::{
    core::errors::{UserErrors, UserResponse, UserResult},
    routes::{app::ReqState, SessionState},
    services::{authentication::UserFromToken, ApplicationResponse},
    types::domain::{self, user::dashboard_metadata as types, MerchantKeyStore},
    utils::user::{self as user_utils, dashboard_metadata as utils},
};
#[cfg(feature = "email")]
use crate::{services::email::types as email_types, utils::user::theme as theme_utils};

pub async fn set_metadata(
    state: SessionState,
    user: UserFromToken,
    request: api::SetMetaDataRequest,
    _req_state: ReqState,
) -> UserResponse<()> {
    let metadata_value = parse_set_request(request)?;
    let metadata_key = DBEnum::from(&metadata_value);

    insert_metadata(&state, user, metadata_key, metadata_value).await?;

    Ok(ApplicationResponse::StatusOk)
}

pub async fn get_multiple_metadata(
    state: SessionState,
    user: UserFromToken,
    request: GetMultipleMetaDataPayload,
    _req_state: ReqState,
) -> UserResponse<Vec<api::GetMetaDataResponse>> {
    let metadata_keys: Vec<DBEnum> = request.results.into_iter().map(parse_get_request).collect();

    let metadata = fetch_metadata(&state, &user, metadata_keys.clone()).await?;

    let mut response = Vec::with_capacity(metadata_keys.len());
    for key in metadata_keys {
        let data = metadata.iter().find(|ele| ele.data_key == key);
        let resp;
        if data.is_none() && utils::is_backfill_required(key) {
            let backfill_data = backfill_metadata(&state, &user, &key).await?;
            resp = into_response(backfill_data.as_ref(), key)?;
        } else {
            resp = into_response(data, key)?;
        }
        response.push(resp);
    }

    Ok(ApplicationResponse::Json(response))
}

fn parse_set_request(data_enum: api::SetMetaDataRequest) -> UserResult<types::MetaData> {
    match data_enum {
        api::SetMetaDataRequest::ProductionAgreement(req) => {
            let ip_address = req
                .ip_address
                .ok_or(report!(UserErrors::InternalServerError))
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
        api::SetMetaDataRequest::IsChangePasswordRequired => {
            Ok(types::MetaData::IsChangePasswordRequired(true))
        }
        api::SetMetaDataRequest::OnboardingSurvey(req) => {
            Ok(types::MetaData::OnboardingSurvey(req))
        }
        api::SetMetaDataRequest::ReconStatus(req) => Ok(types::MetaData::ReconStatus(req)),
    }
}

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
        api::GetMetaDataRequest::IsChangePasswordRequired => DBEnum::IsChangePasswordRequired,
        api::GetMetaDataRequest::OnboardingSurvey => DBEnum::OnboardingSurvey,
        api::GetMetaDataRequest::ReconStatus => DBEnum::ReconStatus,
    }
}

fn into_response(
    data: Option<&DashboardMetadata>,
    data_type: DBEnum,
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
        DBEnum::IsChangePasswordRequired => Ok(api::GetMetaDataResponse::IsChangePasswordRequired(
            data.is_some(),
        )),
        DBEnum::OnboardingSurvey => {
            let resp = utils::deserialize_to_response(data)?;
            Ok(api::GetMetaDataResponse::OnboardingSurvey(resp))
        }
        DBEnum::ReconStatus => {
            let resp = utils::deserialize_to_response(data)?;
            Ok(api::GetMetaDataResponse::ReconStatus(resp))
        }
        // Saved view variants use separate CRUD flow, not the generic GET metadata API
        DBEnum::Payments
        | DBEnum::Refunds
        | DBEnum::Customers
        | DBEnum::Disputes
        | DBEnum::Payouts => Err(report!(UserErrors::InvalidMetadataRequest))
            .attach_printable("Saved view keys should use /views endpoints"),
    }
}

async fn insert_metadata(
    state: &SessionState,
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
                    user.user_id.clone(),
                    user.merchant_id.clone(),
                    user.org_id.clone(),
                    metadata_key,
                    data.clone(),
                )
                .await
                .change_context(UserErrors::InternalServerError);
            }

            #[cfg(feature = "email")]
            {
                let user_data = user.get_user_from_db(state).await?;
                let user_email = domain::UserEmail::from_pii_email(user_data.get_email())
                    .change_context(UserErrors::InternalServerError)?
                    .get_secret()
                    .expose();

                if utils::is_prod_email_required(&data, user_email) {
                    let theme = theme_utils::get_most_specific_theme_using_token_and_min_entity(
                        state,
                        &user,
                        EntityType::Merchant,
                    )
                    .await?;
                    let email_contents = email_types::BizEmailProd::new(
                        state,
                        data.clone(),
                        theme.as_ref().map(|theme| theme.theme_id.clone()),
                        theme
                            .map(|theme| theme.email_config())
                            .unwrap_or(state.conf.theme.email_config.clone()),
                    )?;
                    let send_email_result = state
                        .email_client
                        .compose_and_send_email(
                            user_utils::get_base_url(state),
                            Box::new(email_contents),
                            state.conf.proxy.https_url.as_ref(),
                        )
                        .await;
                    logger::info!(prod_intent_email=?send_email_result);
                }
            }

            // Hubspot integration
            let hubspot_body = state
                .crm_client
                .make_body(CrmPayload {
                    legal_business_name: data.legal_business_name.map(|s| s.into_inner()),
                    business_label: data.business_label.map(|s| s.into_inner()),
                    business_location: data.business_location,
                    display_name: data.display_name.map(|s| s.into_inner()),
                    poc_email: data.poc_email.map(|s| Secret::new(s.peek().clone())),
                    business_type: data.business_type.map(|s| s.into_inner()),
                    business_identifier: data.business_identifier.map(|s| s.into_inner()),
                    business_website: data.business_website.map(|s| s.into_inner()),
                    poc_name: data
                        .poc_name
                        .map(|s| Secret::new(s.peek().clone().into_inner())),
                    poc_contact: data
                        .poc_contact
                        .map(|s| Secret::new(s.peek().clone().into_inner())),
                    comments: data.comments.map(|s| s.into_inner()),
                    is_completed: data.is_completed,
                    business_country_name: data.business_country_name.map(|s| s.into_inner()),
                })
                .await;
            let base_url = user_utils::get_base_url(state);
            let hubspot_request = state
                .crm_client
                .make_request(hubspot_body, base_url.to_string())
                .await;

            let _ = state
                .crm_client
                .send_request(&state.conf.proxy, hubspot_request)
                .await
                .inspect_err(|err| {
                    logger::error!(
                        "An error occurred while sending data to hubspot for user_id {}: {:?}",
                        user.user_id,
                        err
                    );
                });

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
        types::MetaData::IsChangePasswordRequired(data) => {
            utils::insert_user_scoped_metadata_to_db(
                state,
                user.user_id,
                user.merchant_id,
                user.org_id,
                metadata_key,
                data,
            )
            .await
        }
        types::MetaData::OnboardingSurvey(data) => {
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
        types::MetaData::ReconStatus(data) => {
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
                .await;
            }
            metadata
        }
    }
}

async fn fetch_metadata(
    state: &SessionState,
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

pub async fn backfill_metadata(
    state: &SessionState,
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

            #[cfg(feature = "v1")]
            let processor_name = mca.connector_name.clone();

            #[cfg(feature = "v2")]
            let processor_name = mca.connector_name.to_string().clone();
            Some(
                insert_metadata(
                    state,
                    user.to_owned(),
                    DBEnum::StripeConnected,
                    types::MetaData::StripeConnected(api::ProcessorConnected {
                        processor_id: mca.get_id(),
                        processor_name,
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

            #[cfg(feature = "v1")]
            let processor_name = mca.connector_name.clone();

            #[cfg(feature = "v2")]
            let processor_name = mca.connector_name.to_string().clone();
            Some(
                insert_metadata(
                    state,
                    user.to_owned(),
                    DBEnum::PaypalConnected,
                    types::MetaData::PaypalConnected(api::ProcessorConnected {
                        processor_id: mca.get_id(),
                        processor_name,
                    }),
                )
                .await,
            )
            .transpose()
        }
        _ => Ok(None),
    }
}

pub async fn get_merchant_connector_account_by_name(
    state: &SessionState,
    merchant_id: &common_utils::id_type::MerchantId,
    connector_name: &str,
    key_store: &MerchantKeyStore,
) -> UserResult<Option<domain::MerchantConnectorAccount>> {
    #[cfg(feature = "v1")]
    {
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
    #[cfg(feature = "v2")]
    {
        let _ = state;
        let _ = merchant_id;
        let _ = connector_name;
        let _ = key_store;
        todo!()
    }
}

// === Saved Views CRUD ===

const MAX_SAVED_VIEWS: usize = 5;

fn entity_to_data_key(entity: &api::SavedViewEntity) -> DBEnum {
    match entity {
        api::SavedViewEntity::Payments => DBEnum::Payments,
        api::SavedViewEntity::Refunds => DBEnum::Refunds,
        api::SavedViewEntity::Customers => DBEnum::Customers,
        api::SavedViewEntity::Disputes => DBEnum::Disputes,
        api::SavedViewEntity::Payouts => DBEnum::Payouts,
    }
}

pub async fn get_profile_id_from_role(
    state: &SessionState,
    user: &UserFromToken,
) -> UserResult<Option<String>> {
    let tenant_id = user
        .tenant_id
        .clone()
        .unwrap_or(state.tenant.tenant_id.clone());

    let role_info = crate::services::authorization::roles::RoleInfo::from_role_id_in_lineage(
        state,
        &user.role_id,
        &user.merchant_id,
        &user.org_id,
        &user.profile_id,
        &tenant_id,
    )
    .await
    .change_context(UserErrors::InternalServerError)
    .attach_printable("Failed to fetch role info")?;

    match role_info.get_entity_type() {
        EntityType::Profile => Ok(Some(user.profile_id.get_string_repr().to_owned())),
        _ => Ok(None),
    }
}

pub async fn create_saved_view(
    state: SessionState,
    user: UserFromToken,
    request: api::CreateSavedViewRequest,
    _req_state: ReqState,
) -> UserResponse<api::SavedViewResponse> {
    request.validate().map_err(|_| {
        report!(UserErrors::InvalidSavedViewName)
            .attach_printable("Validation failed for create saved view request")
    })?;

    let data_key = entity_to_data_key(&request.data.get_entity());
    let profile_id = get_profile_id_from_role(&state, &user).await?;

    // Fetch existing row (if any)
    let existing = state
        .store
        .find_saved_view_metadata(
            &user.user_id,
            &user.merchant_id,
            &user.org_id,
            profile_id.clone(),
            data_key,
        )
        .await
        .change_context(UserErrors::InternalServerError)
        .attach_printable("Error fetching saved view metadata")?;

    let now = common_utils::date_time::now().to_string();

    let new_view = api::SavedView {
        view_name: request.view_name.clone(),
        data: request.data,
        created_at: now.clone(),
        updated_at: now,
    };

    let updated_views = if let Some(row) = existing {
        // Deserialize existing filters
        let mut views_data: api::SavedViewsData =
            serde_json::from_value(row.data_value.peek().clone())
                .change_context(UserErrors::InternalServerError)
                .attach_printable("Error deserializing saved views")?;

        // Check max limit
        if views_data.views.len() >= MAX_SAVED_VIEWS {
            return Err(report!(UserErrors::MaxSavedViewsReached))
                .attach_printable("Maximum of 5 saved views reached");
        }

        // Check duplicate name (case-insensitive)
        let name_lower = request.view_name.to_lowercase();
        if views_data
            .views
            .iter()
            .any(|v| v.view_name.to_lowercase() == name_lower)
        {
            return Err(report!(UserErrors::SavedViewNameAlreadyExists))
                .attach_printable("A saved view with this name already exists");
        }

        views_data.views.push(new_view);

        let filters_json =
            serde_json::to_value(&views_data).change_context(UserErrors::InternalServerError)?;

        // Update the existing row
        state
            .store
            .update_metadata(
                Some(user.user_id.clone()),
                user.merchant_id.clone(),
                user.org_id.clone(),
                profile_id,
                data_key,
                DashboardMetadataUpdate::UpdateData {
                    data_key,
                    data_value: Secret::new(filters_json),
                    last_modified_by: user.user_id,
                },
            )
            .await
            .change_context(UserErrors::InternalServerError)?;

        views_data.views
    } else {
        // Insert new row
        let views_data = api::SavedViewsData {
            views: vec![new_view],
        };
        let filters_json =
            serde_json::to_value(&views_data).change_context(UserErrors::InternalServerError)?;
        let now_ts = common_utils::date_time::now();

        state
            .store
            .insert_metadata(DashboardMetadataNew {
                user_id: Some(user.user_id.clone()),
                merchant_id: user.merchant_id,
                org_id: user.org_id,
                data_key,
                data_value: Secret::from(filters_json),
                created_by: user.user_id.clone(),
                created_at: now_ts,
                last_modified_by: user.user_id,
                last_modified_at: now_ts,
                profile_id,
            })
            .await
            .change_context(UserErrors::InternalServerError)?;

        views_data.views
    };

    Ok(ApplicationResponse::Json(api::SavedViewResponse {
        count: updated_views.len(),
        views: updated_views,
    }))
}

pub async fn list_saved_views(
    state: SessionState,
    user: UserFromToken,
    request: api::ListSavedViewsRequest,
    _req_state: ReqState,
) -> UserResponse<api::SavedViewResponse> {
    let data_key = entity_to_data_key(&request.entity);
    let profile_id = get_profile_id_from_role(&state, &user).await?;

    let existing = state
        .store
        .find_saved_view_metadata(
            &user.user_id,
            &user.merchant_id,
            &user.org_id,
            profile_id,
            data_key,
        )
        .await
        .change_context(UserErrors::InternalServerError)?;

    let views = match existing {
        Some(row) => {
            let views_data: api::SavedViewsData =
                serde_json::from_value(row.data_value.peek().clone())
                    .change_context(UserErrors::InternalServerError)
                    .attach_printable("Error deserializing saved views")?;
            views_data.views
        }
        None => vec![],
    };

    Ok(ApplicationResponse::Json(api::SavedViewResponse {
        count: views.len(),
        views,
    }))
}

pub async fn update_saved_view(
    state: SessionState,
    user: UserFromToken,
    request: api::UpdateSavedViewRequest,
    _req_state: ReqState,
) -> UserResponse<api::SavedViewResponse> {
    let data_key = entity_to_data_key(&request.data.get_entity());
    let profile_id = get_profile_id_from_role(&state, &user).await?;

    let existing = state
        .store
        .find_saved_view_metadata(
            &user.user_id,
            &user.merchant_id,
            &user.org_id,
            profile_id.clone(),
            data_key,
        )
        .await
        .change_context(UserErrors::InternalServerError)?
        .ok_or(report!(UserErrors::SavedViewNotFound))
        .attach_printable("No saved views found for this entity")?;

    let mut views_data: api::SavedViewsData =
        serde_json::from_value(existing.data_value.peek().clone())
            .change_context(UserErrors::InternalServerError)
            .attach_printable("Error deserializing saved views")?;

    let name_lower = request.view_name.to_lowercase();
    let view = views_data
        .views
        .iter_mut()
        .find(|v| v.view_name.to_lowercase() == name_lower)
        .ok_or(report!(UserErrors::SavedViewNotFound))
        .attach_printable("Saved view with this name not found")?;

    view.data = request.data;
    view.updated_at = common_utils::date_time::now().to_string();

    let filters_json =
        serde_json::to_value(&views_data).change_context(UserErrors::InternalServerError)?;

    state
        .store
        .update_metadata(
            Some(user.user_id.clone()),
            user.merchant_id,
            user.org_id,
            profile_id,
            data_key,
            DashboardMetadataUpdate::UpdateData {
                data_key,
                data_value: Secret::new(filters_json),
                last_modified_by: user.user_id,
            },
        )
        .await
        .change_context(UserErrors::InternalServerError)?;

    Ok(ApplicationResponse::Json(api::SavedViewResponse {
        count: views_data.views.len(),
        views: views_data.views,
    }))
}

pub async fn delete_saved_view(
    state: SessionState,
    user: UserFromToken,
    request: api::DeleteSavedViewRequest,
    _req_state: ReqState,
) -> UserResponse<api::SavedViewResponse> {
    let data_key = entity_to_data_key(&request.entity);
    let profile_id = get_profile_id_from_role(&state, &user).await?;

    let existing = state
        .store
        .find_saved_view_metadata(
            &user.user_id,
            &user.merchant_id,
            &user.org_id,
            profile_id.clone(),
            data_key,
        )
        .await
        .change_context(UserErrors::InternalServerError)?
        .ok_or(report!(UserErrors::SavedViewNotFound))
        .attach_printable("No saved views found for this entity")?;

    let mut views_data: api::SavedViewsData =
        serde_json::from_value(existing.data_value.peek().clone())
            .change_context(UserErrors::InternalServerError)
            .attach_printable("Error deserializing saved views")?;

    let name_lower = request.view_name.to_lowercase();
    let initial_len = views_data.views.len();
    views_data
        .views
        .retain(|v| v.view_name.to_lowercase() != name_lower);

    if views_data.views.len() == initial_len {
        return Err(report!(UserErrors::SavedViewNotFound))
            .attach_printable("Saved view with this name not found");
    }

    let filters_json =
        serde_json::to_value(&views_data).change_context(UserErrors::InternalServerError)?;

    state
        .store
        .update_metadata(
            Some(user.user_id.clone()),
            user.merchant_id,
            user.org_id,
            profile_id,
            data_key,
            DashboardMetadataUpdate::UpdateData {
                data_key,
                data_value: Secret::new(filters_json),
                last_modified_by: user.user_id,
            },
        )
        .await
        .change_context(UserErrors::InternalServerError)?;

    Ok(ApplicationResponse::Json(api::SavedViewResponse {
        count: views_data.views.len(),
        views: views_data.views,
    }))
}
