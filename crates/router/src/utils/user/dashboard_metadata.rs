use std::{net::IpAddr, str::FromStr};

use actix_web::http::header::HeaderMap;
use api_models::user::dashboard_metadata::{
    GetMetaDataRequest, GetMultipleMetaDataPayload, SetMetaDataRequest,
};
use diesel_models::{
    enums::DashboardMetadata as DBEnum,
    user::dashboard_metadata::{DashboardMetadata, DashboardMetadataNew, DashboardMetadataUpdate},
};
use error_stack::{IntoReport, ResultExt};
use masking::Secret;

use crate::{
    core::errors::{UserErrors, UserResult},
    headers, AppState,
};

pub async fn insert_merchant_scoped_metadata_to_db(
    state: &AppState,
    user_id: String,
    merchant_id: String,
    org_id: String,
    metadata_key: DBEnum,
    metadata_value: impl serde::Serialize,
) -> UserResult<DashboardMetadata> {
    let now = common_utils::date_time::now();
    let data_value = serde_json::to_value(metadata_value)
        .into_report()
        .change_context(UserErrors::InternalServerError)
        .attach_printable("Error Converting Struct To Serde Value")?;
    state
        .store
        .insert_metadata(DashboardMetadataNew {
            user_id: None,
            merchant_id,
            org_id,
            data_key: metadata_key,
            data_value,
            created_by: user_id.clone(),
            created_at: now,
            last_modified_by: user_id,
            last_modified_at: now,
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
    state: &AppState,
    user_id: String,
    merchant_id: String,
    org_id: String,
    metadata_key: DBEnum,
    metadata_value: impl serde::Serialize,
) -> UserResult<DashboardMetadata> {
    let now = common_utils::date_time::now();
    let data_value = serde_json::to_value(metadata_value)
        .into_report()
        .change_context(UserErrors::InternalServerError)
        .attach_printable("Error Converting Struct To Serde Value")?;
    state
        .store
        .insert_metadata(DashboardMetadataNew {
            user_id: Some(user_id.clone()),
            merchant_id,
            org_id,
            data_key: metadata_key,
            data_value,
            created_by: user_id.clone(),
            created_at: now,
            last_modified_by: user_id,
            last_modified_at: now,
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
    state: &AppState,
    merchant_id: String,
    org_id: String,
    metadata_keys: Vec<DBEnum>,
) -> UserResult<Vec<DashboardMetadata>> {
    match state
        .store
        .find_merchant_scoped_dashboard_metadata(&merchant_id, &org_id, metadata_keys)
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
pub async fn get_user_scoped_metadata_from_db(
    state: &AppState,
    user_id: String,
    merchant_id: String,
    org_id: String,
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
    state: &AppState,
    user_id: String,
    merchant_id: String,
    org_id: String,
    metadata_key: DBEnum,
    metadata_value: impl serde::Serialize,
) -> UserResult<DashboardMetadata> {
    let data_value = serde_json::to_value(metadata_value)
        .into_report()
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
                data_value,
                last_modified_by: user_id,
            },
        )
        .await
        .change_context(UserErrors::InternalServerError)
}
pub async fn update_user_scoped_metadata(
    state: &AppState,
    user_id: String,
    merchant_id: String,
    org_id: String,
    metadata_key: DBEnum,
    metadata_value: impl serde::Serialize,
) -> UserResult<DashboardMetadata> {
    let data_value = serde_json::to_value(metadata_value)
        .into_report()
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
                data_value,
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
    data.map(|metadata| serde_json::from_value(metadata.data_value.clone()))
        .transpose()
        .map_err(|_| UserErrors::InternalServerError.into())
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
            | DBEnum::IsMultipleConfiguration => merchant_scoped.push(key),
            DBEnum::Feedback | DBEnum::ProdIntent => user_scoped.push(key),
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

pub fn is_backfill_required(metadata_key: &DBEnum) -> bool {
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
            .ok_or(UserErrors::IpAddressParsingFailed.into())
            .attach_printable("X-Forwarded-For header not found")?
            .to_str()
            .map_err(|_| UserErrors::IpAddressParsingFailed.into())
            .attach_printable("Error converting Header Value to Str")?
            .split(',')
            .next()
            .and_then(|ip| {
                let ip_addr: Result<IpAddr, _> = ip.parse();
                ip_addr.ok()
            })
            .ok_or(UserErrors::IpAddressParsingFailed.into())
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
            .map_err(|_| UserErrors::InvalidMetadataRequest.into())
            .attach_printable("Error Parsing to DashboardMetadata enums")?,
    })
}
