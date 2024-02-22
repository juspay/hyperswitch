use api_models::analytics::{GetGlobalSearchRequest, GetSearchRequest, SearchIndex};
use common_utils::errors::CustomResult;
use opensearch::{MsearchParts, OpenSearch, SearchParts};

use crate::errors::AnalyticsError;
use crate::AnalyticsProvider;

async fn get_opensearch_client(region: String) -> Client {
    let sdk_config = aws_config::from_env().region(region_provider).load().await;
    Client::new(&sdk_config)
}

async fn msearch_results(
    pool: &AnalyticsProvider,
    req: GetGlobalSearchRequest,
    merchant_id: &String,
) -> CustomResult<(), AnalyticsError> {
    let client = OpenSearch::default();
    let json_bytes = serde_json::to_vec(&req).map_err(|_| AnalyticsError::UnknownError)?;

    let mut response_body = client
        .msearch(MsearchParts::Index(&[
            "hyperswitch-payment-attempt-events",
            "hyperswitch-payment-intent-events",
            "hyperswitch-refund-events",
        ]))
        .body(json_bytes)
        .send()
        .await?
        .json::<Value>()
        .await?;

    Ok(())
}

async fn search_results(
    pool: &AnalyticsProvider,
    req: GetSearchRequest,
    merchant_id: &String,
    index: SearchIndex,
) -> CustomResult<(), AnalyticsError> {
    let client = OpenSearch::default();
    let json_bytes = serde_json::to_vec(&req).map_err(|_| AnalyticsError::UnknownError)?;

    let mut response_body = client
        .search(SearchParts::Index(&[index.to_string()]))
        .body(json_bytes)
        .send()
        .await?
        .json::<Value>()
        .await?;

    Ok(())
}
