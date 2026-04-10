use std::{net::IpAddr, ops::Not, str::FromStr};

use actix_web::http::header::HeaderMap;
use api_models::user::dashboard_metadata::{
    DashboardOperation, GetMetaDataRequest, GetMultipleMetaDataPayload, ProdIntent,
    SetMetaDataRequest,
};
use common_utils::id_type;
use diesel_models::{
    enums::DashboardMetadata as DBEnum,
    user::dashboard_metadata::{DashboardMetadata, DashboardMetadataNew, DashboardMetadataUpdate},
};
use error_stack::{report, ResultExt};
use hyperswitch_masking::{ExposeInterface, PeekInterface, Secret};
use router_env::logger;

use crate::{
    core::errors::{UserErrors, UserResult},
    headers,
    types::domain::user::dashboard_metadata as types,
    SessionState,
};

pub const MAX_DASHBOARDS: usize = 10;
pub const MAX_WIDGETS_PER_DASHBOARD: usize = 20;

/// Maps role_id to entity_type for custom dashboard scoping
pub fn get_entity_type_from_role(role_id: &str) -> &'static str {
    match role_id {
        "org_admin" => "org",
        "merchant_admin" => "merchant",
        "profile_admin" | "profile_user" => "profile",
        _ => "user",
    }
}

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
            entity_type: None,
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
            entity_type: None,
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
) -> (Vec<DBEnum>, Vec<DBEnum>) {
    let (mut merchant_scoped, mut user_scoped) = (
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
            DBEnum::Feedback | DBEnum::IsChangePasswordRequired | DBEnum::CustomDashboards => {
                user_scoped.push(key)
            }
        }
    }
    (merchant_scoped, user_scoped)
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

// === Custom Dashboard Operations ===

/// Generic fetch-transform-persist for user-scoped dashboard metadata.
pub async fn modify_dashboard_metadata<T, F>(
    state: &SessionState,
    user_id: String,
    merchant_id: id_type::MerchantId,
    org_id: id_type::OrganizationId,
    metadata_key: DBEnum,
    entity_type: String,
    transform: F,
) -> UserResult<DashboardMetadata>
where
    T: serde::Serialize + serde::de::DeserializeOwned,
    F: FnOnce(Option<T>) -> UserResult<T>,
{
    let existing = if entity_type == "org" {
        state
            .store
            .find_org_scoped_dashboard_metadata(&user_id, &org_id, &entity_type, vec![metadata_key])
            .await
    } else {
        state
            .store
            .find_user_scoped_dashboard_metadata(
                &user_id,
                &merchant_id,
                &org_id,
                vec![metadata_key],
            )
            .await
    }
    .change_context(UserErrors::InternalServerError)
    .attach_printable("Error fetching dashboard metadata")?;

    let existing_record = existing.first();

    let existing_value: Option<T> = existing_record
        .map(|m| serde_json::from_value(m.data_value.clone().expose()))
        .transpose()
        .change_context(UserErrors::InternalServerError)
        .attach_printable("Error deserializing dashboard metadata")?;

    let updated_value = transform(existing_value)?;
    let data_value = serde_json::to_value(&updated_value)
        .change_context(UserErrors::InternalServerError)
        .attach_printable("Error serializing dashboard metadata")?;

    match existing_record {
        Some(record) => {
            let update_merchant_id = if entity_type == "org" {
                record.merchant_id.clone()
            } else {
                merchant_id
            };
            state
                .store
                .update_metadata(
                    Some(user_id.clone()),
                    update_merchant_id,
                    org_id,
                    metadata_key,
                    DashboardMetadataUpdate::UpdateData {
                        data_key: metadata_key,
                        data_value: Secret::new(data_value),
                        last_modified_by: user_id,
                    },
                )
                .await
                .change_context(UserErrors::InternalServerError)
                .attach_printable("Error updating dashboard metadata")
        }
        None => {
            let now = common_utils::date_time::now();
            state
                .store
                .insert_metadata(DashboardMetadataNew {
                    user_id: Some(user_id.clone()),
                    merchant_id,
                    org_id,
                    data_key: metadata_key,
                    data_value: Secret::new(data_value),
                    created_by: user_id.clone(),
                    created_at: now,
                    last_modified_by: user_id,
                    last_modified_at: now,
                    profile_id: None,
                    entity_type: Some(entity_type),
                })
                .await
                .change_context(UserErrors::InternalServerError)
                .attach_printable("Error inserting dashboard metadata")
        }
    }
}

pub async fn handle_dashboard_operations(
    state: &SessionState,
    user_id: String,
    merchant_id: id_type::MerchantId,
    org_id: id_type::OrganizationId,
    metadata_key: DBEnum,
    operation: DashboardOperation,
    entity_type: String,
) -> UserResult<DashboardMetadata> {
    match operation {
        DashboardOperation::Create(request) => {
            create_dashboard(
                state,
                user_id,
                merchant_id,
                org_id,
                metadata_key,
                request,
                entity_type,
            )
            .await
        }
        DashboardOperation::Update(request) => {
            update_dashboard(
                state,
                user_id,
                merchant_id,
                org_id,
                metadata_key,
                request,
                entity_type,
            )
            .await
        }
        DashboardOperation::Delete(request) => {
            delete_dashboard(
                state,
                user_id,
                merchant_id,
                org_id,
                metadata_key,
                request,
                entity_type,
            )
            .await
        }
        DashboardOperation::AddWidget(request) => {
            add_widget(
                state,
                user_id,
                merchant_id,
                org_id,
                metadata_key,
                request,
                entity_type,
            )
            .await
        }
        DashboardOperation::UpdateWidget(request) => {
            update_widget(
                state,
                user_id,
                merchant_id,
                org_id,
                metadata_key,
                request,
                entity_type,
            )
            .await
        }
        DashboardOperation::RemoveWidget(request) => {
            remove_widget(
                state,
                user_id,
                merchant_id,
                org_id,
                metadata_key,
                request,
                entity_type,
            )
            .await
        }
        DashboardOperation::UpdateLayout(request) => {
            update_layout(
                state,
                user_id,
                merchant_id,
                org_id,
                metadata_key,
                request,
                entity_type,
            )
            .await
        }
    }
}

async fn create_dashboard(
    state: &SessionState,
    user_id: String,
    merchant_id: id_type::MerchantId,
    org_id: id_type::OrganizationId,
    metadata_key: DBEnum,
    request: api_models::user::dashboard_metadata::CreateDashboardRequest,
    entity_type: String,
) -> UserResult<DashboardMetadata> {
    if request.dashboard_name.trim().is_empty() {
        return Err(report!(UserErrors::InvalidDashboardName))
            .attach_printable("Dashboard name cannot be empty");
    }

    let now = common_utils::date_time::now();
    let widgets: Vec<types::WidgetV1> = request
        .widgets
        .unwrap_or_default()
        .into_iter()
        .map(|w| types::WidgetV1 {
            widget_id: uuid::Uuid::new_v4().to_string(),
            widget_name: w.widget_name,
            chart_type: w.chart_type,
            position: w.position,
            config: w.config,
        })
        .collect();

    let new_dashboard = types::DashboardV1 {
        dashboard_name: request.dashboard_name.clone(),
        description: request.description,
        is_default: false,
        widgets,
        created_at: now.to_string(),
        updated_at: now.to_string(),
    };

    modify_dashboard_metadata(
        state,
        user_id,
        merchant_id,
        org_id,
        metadata_key,
        entity_type,
        |existing: Option<types::CustomDashboardsValue>| {
            let mut data = existing.unwrap_or(types::CustomDashboardsValue { dashboards: vec![] });

            if data.dashboards.len() >= MAX_DASHBOARDS {
                return Err(report!(UserErrors::MaxDashboardsReached));
            }

            if data
                .dashboards
                .iter()
                .any(|d| d.dashboard_name == request.dashboard_name)
            {
                return Err(report!(UserErrors::DashboardNameAlreadyExists));
            }

            data.dashboards.push(new_dashboard);
            Ok(data)
        },
    )
    .await
}

async fn update_dashboard(
    state: &SessionState,
    user_id: String,
    merchant_id: id_type::MerchantId,
    org_id: id_type::OrganizationId,
    metadata_key: DBEnum,
    request: api_models::user::dashboard_metadata::UpdateDashboardRequest,
    entity_type: String,
) -> UserResult<DashboardMetadata> {
    modify_dashboard_metadata(
        state,
        user_id,
        merchant_id,
        org_id,
        metadata_key,
        entity_type,
        |existing: Option<types::CustomDashboardsValue>| {
            let mut data = existing.ok_or(report!(UserErrors::DashboardNotFound))?;

            if let Some(ref new_name) = request.new_dashboard_name {
                if new_name.trim().is_empty() {
                    return Err(report!(UserErrors::InvalidDashboardName));
                }
                if data.dashboards.iter().any(|d| {
                    d.dashboard_name == *new_name && d.dashboard_name != request.dashboard_name
                }) {
                    return Err(report!(UserErrors::DashboardNameAlreadyExists));
                }
            }

            if request.is_default == Some(true) {
                for d in &mut data.dashboards {
                    d.is_default = false;
                }
            }

            let dashboard = data
                .dashboards
                .iter_mut()
                .find(|d| d.dashboard_name == request.dashboard_name)
                .ok_or(report!(UserErrors::DashboardNotFound))?;

            if let Some(ref new_name) = request.new_dashboard_name {
                dashboard.dashboard_name = new_name.clone();
            }
            if let Some(desc) = request.description {
                dashboard.description = Some(desc);
            }
            if let Some(is_default) = request.is_default {
                dashboard.is_default = is_default;
            }
            dashboard.updated_at = common_utils::date_time::now().to_string();

            Ok(data)
        },
    )
    .await
}

async fn delete_dashboard(
    state: &SessionState,
    user_id: String,
    merchant_id: id_type::MerchantId,
    org_id: id_type::OrganizationId,
    metadata_key: DBEnum,
    request: api_models::user::dashboard_metadata::DeleteDashboardRequest,
    entity_type: String,
) -> UserResult<DashboardMetadata> {
    modify_dashboard_metadata(
        state,
        user_id,
        merchant_id,
        org_id,
        metadata_key,
        entity_type,
        |existing: Option<types::CustomDashboardsValue>| {
            let mut data = existing.ok_or(report!(UserErrors::DashboardNotFound))?;
            let initial_len = data.dashboards.len();
            data.dashboards
                .retain(|d| d.dashboard_name != request.dashboard_name);
            if data.dashboards.len() == initial_len {
                return Err(report!(UserErrors::DashboardNotFound));
            }
            Ok(data)
        },
    )
    .await
}

async fn add_widget(
    state: &SessionState,
    user_id: String,
    merchant_id: id_type::MerchantId,
    org_id: id_type::OrganizationId,
    metadata_key: DBEnum,
    request: api_models::user::dashboard_metadata::AddWidgetRequest,
    entity_type: String,
) -> UserResult<DashboardMetadata> {
    modify_dashboard_metadata(
        state,
        user_id,
        merchant_id,
        org_id,
        metadata_key,
        entity_type,
        |existing: Option<types::CustomDashboardsValue>| {
            let mut data = existing.ok_or(report!(UserErrors::DashboardNotFound))?;
            let dashboard = data
                .dashboards
                .iter_mut()
                .find(|d| d.dashboard_name == request.dashboard_name)
                .ok_or(report!(UserErrors::DashboardNotFound))?;

            if dashboard.widgets.len() >= MAX_WIDGETS_PER_DASHBOARD {
                return Err(report!(UserErrors::MaxWidgetsReached));
            }

            dashboard.widgets.push(types::WidgetV1 {
                widget_id: uuid::Uuid::new_v4().to_string(),
                widget_name: request.widget.widget_name,
                chart_type: request.widget.chart_type,
                position: request.widget.position,
                config: request.widget.config,
            });
            dashboard.updated_at = common_utils::date_time::now().to_string();
            Ok(data)
        },
    )
    .await
}

async fn update_widget(
    state: &SessionState,
    user_id: String,
    merchant_id: id_type::MerchantId,
    org_id: id_type::OrganizationId,
    metadata_key: DBEnum,
    request: api_models::user::dashboard_metadata::UpdateWidgetRequest,
    entity_type: String,
) -> UserResult<DashboardMetadata> {
    modify_dashboard_metadata(
        state,
        user_id,
        merchant_id,
        org_id,
        metadata_key,
        entity_type,
        |existing: Option<types::CustomDashboardsValue>| {
            let mut data = existing.ok_or(report!(UserErrors::DashboardNotFound))?;
            let dashboard = data
                .dashboards
                .iter_mut()
                .find(|d| d.dashboard_name == request.dashboard_name)
                .ok_or(report!(UserErrors::DashboardNotFound))?;
            let widget = dashboard
                .widgets
                .iter_mut()
                .find(|w| w.widget_id == request.widget_id)
                .ok_or(report!(UserErrors::WidgetNotFound))?;
            widget.widget_name = request.widget.widget_name;
            widget.chart_type = request.widget.chart_type;
            widget.position = request.widget.position;
            widget.config = request.widget.config;
            dashboard.updated_at = common_utils::date_time::now().to_string();
            Ok(data)
        },
    )
    .await
}

async fn remove_widget(
    state: &SessionState,
    user_id: String,
    merchant_id: id_type::MerchantId,
    org_id: id_type::OrganizationId,
    metadata_key: DBEnum,
    request: api_models::user::dashboard_metadata::RemoveWidgetRequest,
    entity_type: String,
) -> UserResult<DashboardMetadata> {
    modify_dashboard_metadata(
        state,
        user_id,
        merchant_id,
        org_id,
        metadata_key,
        entity_type,
        |existing: Option<types::CustomDashboardsValue>| {
            let mut data = existing.ok_or(report!(UserErrors::DashboardNotFound))?;
            let dashboard = data
                .dashboards
                .iter_mut()
                .find(|d| d.dashboard_name == request.dashboard_name)
                .ok_or(report!(UserErrors::DashboardNotFound))?;
            let initial_len = dashboard.widgets.len();
            dashboard
                .widgets
                .retain(|w| w.widget_id != request.widget_id);
            if dashboard.widgets.len() == initial_len {
                return Err(report!(UserErrors::WidgetNotFound));
            }
            dashboard.updated_at = common_utils::date_time::now().to_string();
            Ok(data)
        },
    )
    .await
}

async fn update_layout(
    state: &SessionState,
    user_id: String,
    merchant_id: id_type::MerchantId,
    org_id: id_type::OrganizationId,
    metadata_key: DBEnum,
    request: api_models::user::dashboard_metadata::UpdateLayoutRequest,
    entity_type: String,
) -> UserResult<DashboardMetadata> {
    modify_dashboard_metadata(
        state,
        user_id,
        merchant_id,
        org_id,
        metadata_key,
        entity_type,
        |existing: Option<types::CustomDashboardsValue>| {
            let mut data = existing.ok_or(report!(UserErrors::DashboardNotFound))?;
            let dashboard = data
                .dashboards
                .iter_mut()
                .find(|d| d.dashboard_name == request.dashboard_name)
                .ok_or(report!(UserErrors::DashboardNotFound))?;
            for entry in &request.layout {
                if let Some(widget) = dashboard
                    .widgets
                    .iter_mut()
                    .find(|w| w.widget_id == entry.widget_id)
                {
                    widget.position = entry.position.clone();
                }
            }
            dashboard.updated_at = common_utils::date_time::now().to_string();
            Ok(data)
        },
    )
    .await
}
