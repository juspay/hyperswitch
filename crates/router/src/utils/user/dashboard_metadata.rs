use std::{net::IpAddr, ops::Not, str::FromStr};

use actix_web::http::header::HeaderMap;
use api_models::user::dashboard_metadata::{
    GetMetaDataRequest, GetMultipleMetaDataPayload, ProdIntent, SetMetaDataRequest,
};
use common_enums::EntityType;
use common_utils::id_type;
use diesel_models::{
    enums::DashboardMetadata as DBEnum,
    user::dashboard_metadata::{DashboardMetadata, DashboardMetadataNew, DashboardMetadataUpdate},
};
use error_stack::{report, ResultExt};
use masking::{ExposeInterface, PeekInterface, Secret};
use router_env::logger;

use crate::{
    core::errors::{UserErrors, UserResult},
    headers,
    services::authentication::UserFromToken,
    types::domain::user::dashboard_metadata as types,
    SessionState,
};

pub const MAX_SAVED_VIEWS: usize = 5;

pub async fn insert_merchant_scoped_metadata_to_db(
    state: &SessionState,
    user_id: String,
    merchant_id: id_type::MerchantId,
    org_id: id_type::OrganizationId,
    metadata_key: DBEnum,
    metadata_value: impl serde::Serialize,
) -> UserResult<DashboardMetadata> {
    let now = common_utils::date_time::now();
    let data_value = serde_json::to_value(metadata_value)
        .change_context(UserErrors::InternalServerError)
        .attach_printable("Error Converting Struct To Serde Value")?;
    state
        .store
        .insert_metadata(DashboardMetadataNew {
            user_id: None,
            merchant_id,
            org_id,
            data_key: metadata_key,
            data_value: Secret::from(data_value),
            created_by: user_id.clone(),
            created_at: now,
            last_modified_by: user_id,
            last_modified_at: now,
            profile_id: None,
        })
        .await
        .map_err(|e| {
            if e.current_context().is_db_unique_violation() {
                return e.change_context(UserErrors::MetadataAlreadySet);
            }
            e.change_context(UserErrors::InternalServerError)
        })
}
pub async fn insert_user_scoped_metadata_to_db(
    state: &SessionState,
    user_id: String,
    merchant_id: id_type::MerchantId,
    org_id: id_type::OrganizationId,
    metadata_key: DBEnum,
    metadata_value: impl serde::Serialize,
) -> UserResult<DashboardMetadata> {
    let now = common_utils::date_time::now();
    let data_value = serde_json::to_value(metadata_value)
        .change_context(UserErrors::InternalServerError)
        .attach_printable("Error Converting Struct To Serde Value")?;
    state
        .store
        .insert_metadata(DashboardMetadataNew {
            user_id: Some(user_id.clone()),
            merchant_id,
            org_id,
            data_key: metadata_key,
            data_value: Secret::from(data_value),
            created_by: user_id.clone(),
            created_at: now,
            last_modified_by: user_id,
            last_modified_at: now,
            profile_id: None,
        })
        .await
        .map_err(|e| {
            if e.current_context().is_db_unique_violation() {
                return e.change_context(UserErrors::MetadataAlreadySet);
            }
            e.change_context(UserErrors::InternalServerError)
        })
}

pub async fn get_merchant_scoped_metadata_from_db(
    state: &SessionState,
    merchant_id: id_type::MerchantId,
    org_id: id_type::OrganizationId,
    metadata_keys: Vec<DBEnum>,
) -> UserResult<Vec<DashboardMetadata>> {
    state
        .store
        .find_merchant_scoped_dashboard_metadata(&merchant_id, &org_id, metadata_keys)
        .await
        .change_context(UserErrors::InternalServerError)
        .attach_printable("DB Error Fetching DashboardMetaData")
}
pub async fn get_user_scoped_metadata_from_db(
    state: &SessionState,
    user_id: String,
    merchant_id: id_type::MerchantId,
    org_id: id_type::OrganizationId,
    metadata_keys: Vec<DBEnum>,
) -> UserResult<Vec<DashboardMetadata>> {
    match state
        .store
        .find_user_scoped_dashboard_metadata(&user_id, &merchant_id, &org_id, metadata_keys)
        .await
    {
        Ok(data) => Ok(data),
        Err(e) => {
            if e.current_context().is_db_not_found() {
                return Ok(Vec::with_capacity(0));
            }
            Err(e
                .change_context(UserErrors::InternalServerError)
                .attach_printable("DB Error Fetching DashboardMetaData"))
        }
    }
}

pub async fn get_profile_user_scoped_metadata_from_db(
    state: &SessionState,
    user_id: String,
    merchant_id: id_type::MerchantId,
    org_id: id_type::OrganizationId,
    profile_id: Option<String>,
    metadata_keys: Vec<DBEnum>,
) -> UserResult<Vec<DashboardMetadata>> {
    let mut results = Vec::with_capacity(metadata_keys.len());
    for key in metadata_keys {
        if let Some(metadata) = state
            .store
            .find_saved_view_metadata(&user_id, &merchant_id, &org_id, profile_id.clone(), key)
            .await
            .change_context(UserErrors::InternalServerError)
            .attach_printable("DB Error Fetching DashboardMetaData")?
        {
            results.push(metadata);
        }
    }
    Ok(results)
}

pub async fn update_merchant_scoped_metadata(
    state: &SessionState,
    user_id: String,
    merchant_id: id_type::MerchantId,
    org_id: id_type::OrganizationId,
    metadata_key: DBEnum,
    metadata_value: impl serde::Serialize,
) -> UserResult<DashboardMetadata> {
    let data_value = serde_json::to_value(metadata_value)
        .change_context(UserErrors::InternalServerError)
        .attach_printable("Error Converting Struct To Serde Value")?;

    state
        .store
        .update_metadata(
            None,
            merchant_id,
            org_id,
            None,
            metadata_key,
            DashboardMetadataUpdate::UpdateData {
                data_key: metadata_key,
                data_value: Secret::from(data_value),
                last_modified_by: user_id,
            },
        )
        .await
        .change_context(UserErrors::InternalServerError)
}
pub async fn update_user_scoped_metadata(
    state: &SessionState,
    user_id: String,
    merchant_id: id_type::MerchantId,
    org_id: id_type::OrganizationId,
    metadata_key: DBEnum,
    metadata_value: impl serde::Serialize,
) -> UserResult<DashboardMetadata> {
    let data_value = serde_json::to_value(metadata_value)
        .change_context(UserErrors::InternalServerError)
        .attach_printable("Error Converting Struct To Serde Value")?;

    state
        .store
        .update_metadata(
            Some(user_id.clone()),
            merchant_id,
            org_id,
            None,
            metadata_key,
            DashboardMetadataUpdate::UpdateData {
                data_key: metadata_key,
                data_value: Secret::from(data_value),
                last_modified_by: user_id,
            },
        )
        .await
        .change_context(UserErrors::InternalServerError)
}

pub fn deserialize_to_response<T>(data: Option<&DashboardMetadata>) -> UserResult<Option<T>>
where
    T: serde::de::DeserializeOwned,
{
    data.map(|metadata| serde_json::from_value(metadata.data_value.clone().expose()))
        .transpose()
        .change_context(UserErrors::InternalServerError)
        .attach_printable("Error Serializing Metadata from DB")
}

pub fn separate_metadata_type_based_on_scope(
    metadata_keys: Vec<DBEnum>,
) -> (Vec<DBEnum>, Vec<DBEnum>, Vec<DBEnum>) {
    let (mut merchant_scoped, mut user_scoped, mut profile_user_scoped) = (
        Vec::with_capacity(metadata_keys.len()),
        Vec::with_capacity(metadata_keys.len()),
        Vec::with_capacity(metadata_keys.len()),
    );
    for key in metadata_keys {
        match key {
            DBEnum::ProductionAgreement
            | DBEnum::SetupProcessor
            | DBEnum::ConfigureEndpoint
            | DBEnum::SetupComplete
            | DBEnum::FirstProcessorConnected
            | DBEnum::SecondProcessorConnected
            | DBEnum::ConfiguredRouting
            | DBEnum::TestPayment
            | DBEnum::IntegrationMethod
            | DBEnum::ConfigurationType
            | DBEnum::IntegrationCompleted
            | DBEnum::StripeConnected
            | DBEnum::PaypalConnected
            | DBEnum::SpRoutingConfigured
            | DBEnum::SpTestPayment
            | DBEnum::DownloadWoocom
            | DBEnum::ConfigureWoocom
            | DBEnum::SetupWoocomWebhook
            | DBEnum::OnboardingSurvey
            | DBEnum::IsMultipleConfiguration
            | DBEnum::ReconStatus
            | DBEnum::ProdIntent => merchant_scoped.push(key),
            #[cfg(feature = "v1")]
            DBEnum::PaymentViews => profile_user_scoped.push(key),
            DBEnum::Feedback | DBEnum::IsChangePasswordRequired => user_scoped.push(key),
        }
    }
    (merchant_scoped, user_scoped, profile_user_scoped)
}

pub fn is_update_required(metadata: &UserResult<DashboardMetadata>) -> bool {
    match metadata {
        Ok(_) => false,
        Err(e) => matches!(e.current_context(), UserErrors::MetadataAlreadySet),
    }
}

pub fn is_backfill_required(metadata_key: DBEnum) -> bool {
    matches!(
        metadata_key,
        DBEnum::StripeConnected | DBEnum::PaypalConnected
    )
}

pub fn set_ip_address_if_required(
    request: &mut SetMetaDataRequest,
    headers: &HeaderMap,
) -> UserResult<()> {
    if let SetMetaDataRequest::ProductionAgreement(req) = request {
        let ip_address_from_request: Secret<String, common_utils::pii::IpAddress> = headers
            .get(headers::X_FORWARDED_FOR)
            .ok_or(report!(UserErrors::IpAddressParsingFailed))
            .attach_printable("X-Forwarded-For header not found")?
            .to_str()
            .change_context(UserErrors::IpAddressParsingFailed)
            .attach_printable("Error converting Header Value to Str")?
            .split(',')
            .next()
            .and_then(|ip| {
                let ip_addr: Result<IpAddr, _> = ip.parse();
                ip_addr.ok()
            })
            .ok_or(report!(UserErrors::IpAddressParsingFailed))
            .attach_printable("Error Parsing header value to ip")?
            .to_string()
            .into();
        req.ip_address = Some(ip_address_from_request)
    }
    Ok(())
}

pub fn parse_string_to_enums(query: String) -> UserResult<GetMultipleMetaDataPayload> {
    Ok(GetMultipleMetaDataPayload {
        results: query
            .split(',')
            .map(GetMetaDataRequest::from_str)
            .collect::<Result<Vec<GetMetaDataRequest>, _>>()
            .change_context(UserErrors::InvalidMetadataRequest)
            .attach_printable("Error Parsing to DashboardMetadata enums")?,
    })
}

fn not_contains_string(value: Option<&str>, value_to_be_checked: &str) -> bool {
    value.is_some_and(|mail| !mail.contains(value_to_be_checked))
}

pub fn is_prod_email_required(data: &ProdIntent, user_email: String) -> bool {
    let poc_email_check = not_contains_string(
        data.poc_email.as_ref().map(|email| email.peek().as_str()),
        "juspay",
    );
    let business_website_check =
        not_contains_string(data.business_website.as_ref().map(|s| s.as_str()), "juspay")
            && not_contains_string(
                data.business_website.as_ref().map(|s| s.as_str()),
                "hyperswitch",
            );
    let user_email_check = not_contains_string(Some(&user_email), "juspay");

    if (poc_email_check && business_website_check && user_email_check).not() {
        logger::info!(prod_intent_email = poc_email_check);
        logger::info!(prod_intent_email = business_website_check);
        logger::info!(prod_intent_email = user_email_check);
    }

    poc_email_check && business_website_check && user_email_check
}

#[cfg(feature = "v1")]
pub fn entity_to_data_key(
    entity: &api_models::user::dashboard_metadata::SavedViewEntity,
) -> DBEnum {
    match entity {
        api_models::user::dashboard_metadata::SavedViewEntity::PaymentViews => DBEnum::PaymentViews,
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

#[allow(clippy::too_many_arguments)]
pub async fn modify_dashboard_metadata<T, F>(
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

#[cfg(feature = "v1")]
pub async fn handle_saved_view_operations(
    state: &SessionState,
    user: UserFromToken,
    metadata_key: DBEnum,
    operation: api_models::user::dashboard_metadata::SavedViewOperation,
) -> UserResult<DashboardMetadata> {
    let profile_id = get_profile_id_from_role(state, &user).await?;
    let last_modified_by = user.user_id.clone();
    match operation {
        api_models::user::dashboard_metadata::SavedViewOperation::Create(request) => {
            request.validate().map_err(|_| {
                report!(UserErrors::InvalidSavedViewName)
                    .attach_printable("Validation failed for create saved view request")
            })?;

            let now = common_utils::date_time::now();
            let new_view_domain = types::SavedViewV1 {
                view_name: request.view_name.clone(),
                version: api_models::user::dashboard_metadata::SavedViewVersion::V1,
                filters: match request.data {
                    api_models::user::dashboard_metadata::SavedViewFilters::V1(f) => match f {
                        api_models::user::dashboard_metadata::SavedViewFiltersV1::PaymentViews(
                            p,
                        ) => p,
                    },
                },
                created_at: now.to_string(),
                updated_at: now.to_string(),
            };

            modify_dashboard_metadata(
                state,
                user,
                metadata_key,
                profile_id,
                Some(last_modified_by.clone()),
                true,
                last_modified_by,
                |existing: Option<types::PaymentViewsValue>| {
                    let mut views_data =
                        existing.unwrap_or(types::PaymentViewsValue { views: vec![] });

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
            .await
        }
        api_models::user::dashboard_metadata::SavedViewOperation::Update(request) => {
            modify_dashboard_metadata(
                state,
                user,
                metadata_key,
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
                    view.version = api_models::user::dashboard_metadata::SavedViewVersion::V1;
                    view.view_name = request.view_name.clone();
                    view.filters = match request.data {
                        api_models::user::dashboard_metadata::SavedViewFilters::V1(f) => match f {
                            api_models::user::dashboard_metadata::SavedViewFiltersV1::PaymentViews(
                                p,
                            ) => p,
                        },
                    };
                    view.updated_at = now.to_string();

                    Ok(views_data)
                },
            )
            .await
        }
        api_models::user::dashboard_metadata::SavedViewOperation::Delete(request) => {
            modify_dashboard_metadata(
                state,
                user,
                metadata_key,
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
            .await
        }
    }
}
