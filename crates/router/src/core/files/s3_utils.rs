use aws_config::{self, meta::region::RegionProviderChain};
use aws_sdk_s3::{config::Region, Client};
use common_utils::errors::CustomResult;
use futures::TryStreamExt;

use crate::{core::errors, logger, routes::AppState};

async fn get_aws_client(state: &AppState) -> Client {
    let region_provider =
        RegionProviderChain::first_try(Region::new(state.conf.file_upload_config.region.clone()));
    let sdk_config = aws_config::from_env().region(region_provider).load().await;
    Client::new(&sdk_config)
}

pub async fn upload_file_to_s3(
    state: &AppState,
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
        Err(error) => {
            logger::error!(?error);
            Err(errors::ApiErrorResponse::InternalServerError.into())
        }
    }
}

pub async fn delete_file_from_s3(
    state: &AppState,
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
        Err(error) => {
            logger::error!(?error);
            Err(errors::ApiErrorResponse::InternalServerError.into())
        }
    }
}

pub async fn retrieve_file_from_s3(
    state: &AppState,
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
        Err(error) => {
            logger::error!(?error);
            Err(errors::ApiErrorResponse::InternalServerError)?
        }
    };
    let mut received_data: Vec<u8> = Vec::new();
    while let Some(bytes) = object
        .body
        .try_next()
        .await
        .map_err(|_| errors::ApiErrorResponse::InternalServerError)?
    {
        received_data.extend_from_slice(&bytes); // Collect the bytes in the Vec
    }
    Ok(received_data)
}
