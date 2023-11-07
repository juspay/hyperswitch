use aws_config::{self, meta::region::RegionProviderChain};
use aws_sdk_s3::{config::Region, Client};
use common_utils::errors::CustomResult;
use error_stack::{IntoReport, ResultExt};
use futures::TryStreamExt;

use crate::{core::errors, routes};

async fn get_aws_client(state: &routes::AppState) -> Client {
    let region_provider =
        RegionProviderChain::first_try(Region::new(state.conf.file_upload_config.region.clone()));
    let sdk_config = aws_config::from_env().region(region_provider).load().await;
    Client::new(&sdk_config)
}

pub async fn upload_file_to_s3(
    state: &routes::AppState,
    file_key: String,
    file: Vec<u8>,
) -> CustomResult<(), errors::ApiErrorResponse> {
    let client = get_aws_client(state).await;
    let bucket_name = &state.conf.file_upload_config.bucket_name;
    // Upload file to S3
    let upload_res = client
        .put_object()
        .bucket(bucket_name)
        .key(file_key.clone())
        .body(file.into())
        .send()
        .await;
    upload_res
        .into_report()
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("File upload to S3 failed")?;
    Ok(())
}

pub async fn delete_file_from_s3(
    state: &routes::AppState,
    file_key: String,
) -> CustomResult<(), errors::ApiErrorResponse> {
    let client = get_aws_client(state).await;
    let bucket_name = &state.conf.file_upload_config.bucket_name;
    // Delete file from S3
    let delete_res = client
        .delete_object()
        .bucket(bucket_name)
        .key(file_key)
        .send()
        .await;
    delete_res
        .into_report()
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("File delete from S3 failed")?;
    Ok(())
}

pub async fn retrieve_file_from_s3(
    state: &routes::AppState,
    file_key: String,
) -> CustomResult<Vec<u8>, errors::ApiErrorResponse> {
    let client = get_aws_client(state).await;
    let bucket_name = &state.conf.file_upload_config.bucket_name;
    // Get file data from S3
    let get_res = client
        .get_object()
        .bucket(bucket_name)
        .key(file_key)
        .send()
        .await;
    let mut object = get_res
        .into_report()
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("File retrieve from S3 failed")?;
    let mut received_data: Vec<u8> = Vec::new();
    while let Some(bytes) = object
        .body
        .try_next()
        .await
        .into_report()
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Invalid file data received from S3")?
    {
        received_data.extend_from_slice(&bytes); // Collect the bytes in the Vec
    }
    Ok(received_data)
}
