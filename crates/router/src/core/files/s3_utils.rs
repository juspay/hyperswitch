use aws_config::{self, meta::region::RegionProviderChain};
use aws_sdk_s3::{config::Region, Client};
use common_utils::errors::CustomResult;
use error_stack::{IntoReport, ResultExt};
use futures::TryStreamExt;

use crate::{core::errors, logger, routes};

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
    match upload_res {
        Ok(_) => Ok(()),
        Err(error) => Err(errors::ApiErrorResponse::InternalServerError.into())
            .attach_printable(format!("{}{}", "File upload to S3 failed: ", error)),
    }
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
    match delete_res {
        Ok(_) => Ok(()),
        Err(error) => Err(errors::ApiErrorResponse::InternalServerError.into())
            .attach_printable(format!("{}{}", "File delete from S3 failed: ", error)),
    }
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
    let mut object = match get_res {
        Ok(valid_res) => valid_res,
        Err(error) => Err(errors::ApiErrorResponse::InternalServerError.into())
            .attach_printable(format!("{}{}", "File retrieve from S3 failed: ", error))?,
    };
    let mut received_data: Vec<u8> = Vec::new();
    while let Some(bytes) = object
        .body
        .try_next()
        .await
        .map_err(|err| {
            logger::error!(%err, "Failed reading file data from S3");
            errors::ApiErrorResponse::InternalServerError
        })
        .into_report()
        .attach_printable("Invalid file data received from S3")?
    {
        received_data.extend_from_slice(&bytes); // Collect the bytes in the Vec
    }
    Ok(received_data)
}
