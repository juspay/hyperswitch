use aws_config::{self, meta::region::RegionProviderChain};
use aws_sdk_lambda::{config::Region, types::InvocationType::Event, Client};
use aws_smithy_types::Blob;
use common_utils::errors::CustomResult;
use error_stack::{IntoReport, ResultExt};

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
        .into_report()
        .map_err(|er| {
            let er_rep = format!("{er:?}");
            er.attach_printable(er_rep)
        })
        .change_context(AnalyticsError::UnknownError)
        .attach_printable("Lambda invocation failed")?;
    Ok(())
}
