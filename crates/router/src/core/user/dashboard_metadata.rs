use api_models::user::dashboard_metadata::{self as api, GetMultipleMetaDataPayload};
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

const MAX_SAVED_VIEWS: usize = 5;

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
        #[cfg(feature = "v1")]
        DBEnum::PaymentViews => {
            let resp: Option<types::PaymentViewsValue> = utils::deserialize_to_response(data)?;
            Ok(api::GetMetaDataResponse::PaymentViews(resp.map(|d| {
                d.views
                    .into_iter()
                    .map(|v| api::SavedView {
                        version: v.version,
                        view_name: v.view_name,
                        data: api::SavedViewFilters::V1(api::SavedViewFiltersV1::PaymentViews(
                            v.filters,
                        )),
                        created_at: v.created_at.to_string(),
                        updated_at: v.updated_at.to_string(),
                    })
                    .collect()
            })))
        }
    }
}

async fn insert_metadata(
    state: &SessionState,
    user: UserFromToken,
    metadata_key: DBEnum,
    metadata_value: types::MetaData,
) -> UserResult<DashboardMetadata> {
    let last_modified_by = user.user_id.clone();
    let (merchant_scoped, _) = utils::separate_metadata_type_based_on_scope(vec![metadata_key]);

    let (user_id_to_store, is_user_scoped) = if merchant_scoped.is_empty() {
        (Some(user.user_id.clone()), true)
    } else {
        (None, false)
    };

    let metadata_value_to_store = metadata_value.clone();
    let metadata = modify_dashboard_metadata(
        state,
        user.clone(),
        metadata_key,
        None,
        user_id_to_store,
        is_user_scoped,
        last_modified_by,
        |_: Option<serde_json::Value>| {
            serde_json::to_value(&metadata_value_to_store)
                .change_context(UserErrors::InternalServerError)
                .attach_printable("Error serializing metadata inner value")
        },
    )
    .await?;

    if let types::MetaData::ProdIntent(data) = metadata_value {
        #[cfg(feature = "email")]
        {
            let user_data = user.get_active_user_from_db(state).await?;
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
    }

    Ok(metadata)
}

#[allow(clippy::too_many_arguments)]
async fn modify_dashboard_metadata<T, F>(
    state: &SessionState,
    user: UserFromToken,
    metadata_key: DBEnum,
    profile_id: Option<String>,
    user_id_to_store: Option<String>,
    is_user_scoped: bool,
    last_modified_by: String,
    transform: F,
) -> UserResult<DashboardMetadata>
where
    T: serde::Serialize + serde::de::DeserializeOwned,
    F: FnOnce(Option<T>) -> UserResult<T>,
{
    #[cfg(feature = "v1")]
    let is_payment_view = metadata_key == DBEnum::PaymentViews;
    #[cfg(not(feature = "v1"))]
    let is_payment_view = false;

    let existing = if is_user_scoped || is_payment_view {
        state
            .store
            .find_saved_view_metadata(
                &user.user_id,
                &user.merchant_id,
                &user.org_id,
                profile_id.clone(),
                metadata_key,
            )
            .await
    } else {
        state
            .store
            .find_merchant_scoped_dashboard_metadata(
                &user.merchant_id,
                &user.org_id,
                vec![metadata_key],
            )
            .await
            .map(|v| v.into_iter().next())
    }
    .change_context(UserErrors::InternalServerError)
    .attach_printable("Error fetching dashboard metadata")?;

    let existing_value: Option<T> = existing
        .as_ref()
        .map(|m| serde_json::from_value(m.data_value.clone().peek().clone()))
        .transpose()
        .change_context(UserErrors::InternalServerError)
        .attach_printable("Error deserializing dashboard metadata")?;

    let updated_value = transform(existing_value)?;
    let data_value = serde_json::to_value(&updated_value)
        .change_context(UserErrors::InternalServerError)
        .attach_printable("Error serializing dashboard metadata")?;

    match existing {
        Some(metadata) => state
            .store
            .update_metadata(
                metadata.user_id,
                metadata.merchant_id,
                metadata.org_id,
                metadata.profile_id,
                metadata_key,
                DashboardMetadataUpdate::UpdateData {
                    data_key: metadata_key,
                    data_value: Secret::new(data_value),
                    last_modified_by,
                },
            )
            .await
            .change_context(UserErrors::InternalServerError)
            .attach_printable("Error updating dashboard metadata"),
        None => {
            let now = common_utils::date_time::now();
            state
                .store
                .insert_metadata(DashboardMetadataNew {
                    user_id: user_id_to_store,
                    merchant_id: user.merchant_id,
                    org_id: user.org_id,
                    data_key: metadata_key,
                    data_value: Secret::new(data_value),
                    created_by: last_modified_by.clone(),
                    created_at: now,
                    last_modified_by: last_modified_by.clone(),
                    last_modified_at: now,
                    profile_id,
                })
                .await
                .change_context(UserErrors::InternalServerError)
                .attach_printable("Error inserting dashboard metadata")
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

#[cfg(feature = "v1")]
fn entity_to_data_key(entity: &api::SavedViewEntity) -> DBEnum {
    match entity {
        api::SavedViewEntity::PaymentViews => DBEnum::PaymentViews,
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

#[cfg(feature = "v1")]
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
            let views_data: types::PaymentViewsValue =
                serde_json::from_value(row.data_value.peek().clone())
                    .change_context(UserErrors::InternalServerError)
                    .attach_printable("Error deserializing saved views")?;

            views_data
                .views
                .into_iter()
                .map(|v| api::SavedView {
                    version: v.version,
                    view_name: v.view_name,
                    data: api::SavedViewFilters::V1(api::SavedViewFiltersV1::PaymentViews(
                        v.filters,
                    )),
                    created_at: v.created_at,
                    updated_at: v.updated_at,
                })
                .collect()
        }
        None => vec![],
    };

    Ok(ApplicationResponse::Json(api::SavedViewResponse {
        count: views.len(),
        views,
    }))
}

#[cfg(feature = "v1")]
pub async fn create_saved_view(
    state: SessionState,
    user: UserFromToken,
    request: api::CreateSavedViewRequest,
    _req_state: ReqState,
) -> UserResponse<()> {
    request.validate().map_err(|_| {
        report!(UserErrors::InvalidSavedViewName)
            .attach_printable("Validation failed for create saved view request")
    })?;

    let data_key = entity_to_data_key(&request.data.get_entity());
    let profile_id = get_profile_id_from_role(&state, &user).await?;
    let last_modified_by = user.user_id.clone();
    let now = common_utils::date_time::now();

    let new_view_domain = types::SavedViewV1 {
        view_name: request.view_name.clone(),
        version: api::SavedViewVersion::V1,
        filters: match request.data {
            api::SavedViewFilters::V1(f) => match f {
                api::SavedViewFiltersV1::PaymentViews(p) => p,
            },
        },
        created_at: now.to_string(),
        updated_at: now.to_string(),
    };

    modify_dashboard_metadata(
        &state,
        user,
        data_key,
        profile_id,
        Some(last_modified_by.clone()),
        true,
        last_modified_by,
        |existing: Option<types::PaymentViewsValue>| {
            let mut views_data = existing.unwrap_or(types::PaymentViewsValue { views: vec![] });

            if views_data.views.len() >= MAX_SAVED_VIEWS {
                return Err(report!(UserErrors::MaxSavedViewsReached))
                    .attach_printable("Maximum of 5 saved views reached");
            }

            let name_lower = request.view_name.to_lowercase();
            if views_data
                .views
                .iter()
                .any(|v| v.view_name.to_lowercase() == name_lower)
            {
                return Err(report!(UserErrors::SavedViewNameAlreadyExists))
                    .attach_printable("A saved view with this name already exists");
            }

            views_data.views.push(new_view_domain);
            Ok(views_data)
        },
    )
    .await?;

    Ok(ApplicationResponse::StatusOk)
}

#[cfg(feature = "v1")]
pub async fn update_saved_view(
    state: SessionState,
    user: UserFromToken,
    request: api::UpdateSavedViewRequest,
    _req_state: ReqState,
) -> UserResponse<()> {
    let data_key = entity_to_data_key(&request.data.get_entity());
    let profile_id = get_profile_id_from_role(&state, &user).await?;
    let last_modified_by = user.user_id.clone();

    modify_dashboard_metadata(
        &state,
        user,
        data_key,
        profile_id,
        Some(last_modified_by.clone()),
        true,
        last_modified_by,
        |existing: Option<types::PaymentViewsValue>| {
            let mut views_data = existing.ok_or(report!(UserErrors::SavedViewNotFound))?;

            let name_lower = request.view_name.to_lowercase();
            let view = views_data
                .views
                .iter_mut()
                .find(|v| v.view_name.to_lowercase() == name_lower)
                .ok_or(report!(UserErrors::SavedViewNotFound))
                .attach_printable("Saved view with this name not found")?;

            let now = common_utils::date_time::now();
            view.version = api::SavedViewVersion::V1;
            view.view_name = request.view_name.clone();
            view.filters = match request.data {
                api::SavedViewFilters::V1(f) => match f {
                    api::SavedViewFiltersV1::PaymentViews(p) => p,
                },
            };
            view.updated_at = now.to_string();

            Ok(views_data)
        },
    )
    .await?;

    Ok(ApplicationResponse::StatusOk)
}

#[cfg(feature = "v1")]
pub async fn delete_saved_view(
    state: SessionState,
    user: UserFromToken,
    request: api::DeleteSavedViewRequest,
    _req_state: ReqState,
) -> UserResponse<()> {
    let data_key = entity_to_data_key(&request.entity);
    let profile_id = get_profile_id_from_role(&state, &user).await?;
    let last_modified_by = user.user_id.clone();

    modify_dashboard_metadata(
        &state,
        user,
        data_key,
        profile_id,
        Some(last_modified_by.clone()),
        true,
        last_modified_by,
        |existing: Option<types::PaymentViewsValue>| {
            let mut views_data = existing.ok_or(report!(UserErrors::SavedViewNotFound))?;

            let name_lower = request.view_name.to_lowercase();
            let initial_len = views_data.views.len();
            views_data
                .views
                .retain(|v| v.view_name.to_lowercase() != name_lower);

            if views_data.views.len() == initial_len {
                return Err(report!(UserErrors::SavedViewNotFound))
                    .attach_printable("Saved view with this name not found");
            }

            Ok(views_data)
        },
    )
    .await?;

    Ok(ApplicationResponse::StatusOk)
}
