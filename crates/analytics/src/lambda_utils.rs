use api_models::analytics::LambdaResponse;
use aws_config::{self, meta::region::RegionProviderChain, Region};
use aws_sdk_lambda::{types::InvocationType::Event, Client};
use aws_smithy_types::Blob;
use common_utils::errors::CustomResult;
use error_stack::{report, ResultExt};

use crate::errors::AnalyticsError;

async fn get_aws_client(region: String) -> Client {
    let region_provider = RegionProviderChain::first_try(Region::new(region));
    let sdk_config = aws_config::from_env().region(region_provider).load().await;
    Client::new(&sdk_config)
}

pub async fn invoke_lambda(
    function_name: &str,
    region: &str,
    json_bytes: &[u8],
) -> CustomResult<(), AnalyticsError> {
    get_aws_client(region.to_string())
        .await
        .invoke()
        .function_name(function_name)
        .invocation_type(Event)
        .payload(Blob::new(json_bytes.to_owned()))
        .send()
        .await
        .map_err(|er| {
            let er_rep = format!("{er:?}");
            report!(er).attach_printable(er_rep)
        })
        .change_context(AnalyticsError::UnknownError)
        .attach_printable("Lambda invocation failed")?;
    Ok(())
}

pub async fn invoke_lambda_get_status(
    function_name: &str,
    region: &str,
    json_bytes: &[u8],
) -> CustomResult<i32, AnalyticsError> {
    let invoke_output = get_aws_client(region.to_string())
        .await
        .invoke()
        .function_name(function_name)
        .invocation_type(Event)
        .payload(Blob::new(json_bytes.to_owned()))
        .send()
        .await
        .map_err(|er| {
            let er_rep = format!("{er:?}");
            report!(er).attach_printable(er_rep)
        })
        .change_context(AnalyticsError::UnknownError)
        .attach_printable("Lambda invocation failed")?;

    Ok(invoke_output.status_code)
}

pub async fn lambda_handler(
    function_name: &str,
    region: &str,
    json_bytes: &[u8],
    s3_path: &String,
) -> CustomResult<LambdaResponse, AnalyticsError> {
    let invoke_lambda_status_code =
        invoke_lambda_get_status(function_name, region, json_bytes)
        .await
        .change_context(AnalyticsError::UnknownError)
        .attach_printable("Lambda invocation failed")?;
    let response = LambdaResponse {
        s3_path: s3_path.clone(),
        invocation_status_code: invoke_lambda_status_code,
    };
    print!("Lambda invocation response: {:?}", response.clone());
    Ok(response)
}
