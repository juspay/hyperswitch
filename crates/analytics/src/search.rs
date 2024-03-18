mod config;
mod client;
mod error;

use actix_web::web::Json;
use api_models::analytics::search::{
    GetGlobalSearchRequest, GetSearchRequestWithIndex, GetSearchResponse, OpenMsearchOutput,
    OpensearchOutput, SearchIndex,
};
use aws_config::{self, meta::region::RegionProviderChain, Region};
use common_utils::errors::CustomResult;
use opensearch::{
    auth::Credentials,
    cert::CertificateValidation,
    http::{
        request::JsonBody,
        transport::{SingleNodeConnectionPool, TransportBuilder},
        Url,
    },
    MsearchParts, OpenSearch, SearchParts,
};
use serde_json::{json, Value};
use strum::IntoEnumIterator;

use crate::{errors::AnalyticsError, OpensearchAuth, OpensearchConfig, OpensearchIndexes};

#[derive(Debug, thiserror::Error)]
pub enum OpensearchError {
    #[error("Opensearch connection error")]
    ConnectionError,
    #[error("Opensearch NON-200 response content: '{0}'")]
    ResponseNotOK(String),
    #[error("Opensearch response error")]
    ResponseError,
}

pub fn search_index_to_opensearch_index(index: SearchIndex, config: &OpensearchIndexes) -> String {
    match index {
        SearchIndex::PaymentAttempts => config.payment_attempts.clone(),
        SearchIndex::PaymentIntents => config.payment_intents.clone(),
        SearchIndex::Refunds => config.refunds.clone(),
        SearchIndex::Disputes => config.disputes.clone(),
    }
}

async fn get_opensearch_client(config: OpensearchConfig) -> Result<OpenSearch, OpensearchError> {
    let url = Url::parse(&config.host).map_err(|_| OpensearchError::ConnectionError)?;
    let transport = match config.auth {
        OpensearchAuth::Basic { username, password } => {
            let credentials = Credentials::Basic(username, password);
            TransportBuilder::new(SingleNodeConnectionPool::new(url))
                .cert_validation(CertificateValidation::None)
                .auth(credentials)
                .build()
                .map_err(|_| OpensearchError::ConnectionError)?
        }
        OpensearchAuth::Aws { region } => {
            let region_provider = RegionProviderChain::first_try(Region::new(region));
            let sdk_config = aws_config::from_env().region(region_provider).load().await;
            let conn_pool = SingleNodeConnectionPool::new(url);
            TransportBuilder::new(conn_pool)
                .auth(
                    sdk_config
                        .clone()
                        .try_into()
                        .map_err(|_| OpensearchError::ConnectionError)?,
                )
                .service_name("es")
                .build()
                .map_err(|_| OpensearchError::ConnectionError)?
        }
    };
    Ok(OpenSearch::new(transport))
}

pub fn search_filter_maker(
    index: SearchIndex,
    json_body: GetGlobalSearchRequest, 
    merchant_id: &String,
    config: OpensearchConfig,
) -> String {
    let req_filters = json_body.filters;
    let merchant_id_filter = json!({"match_phrase": {"merchant_id": merchant_id}});
    let filters_array = [merchant_id_filter];
    for filter in &req_filters{
        println!("---------------------");
        format!("{:?}",filter);
        println!("filter hai okay");
    }
    // println!("{:?} , {:?}",filters_array, req_filters);
    return format!("{}",json!({"ok":"ok"}));
}


pub async fn msearch_results(
    req: GetGlobalSearchRequest,
    merchant_id: &String,
    config: OpensearchConfig,
) -> CustomResult<Vec<GetSearchResponse>, AnalyticsError> {
    let client = get_opensearch_client(config.clone())
        .await
        .map_err(|_| AnalyticsError::UnknownError)?;

    let mut msearch_vector: Vec<JsonBody<Value>> = vec![];
    for index in SearchIndex::iter() {
        msearch_vector
            .push(json!({"index": search_index_to_opensearch_index(index.clone(),&config.indexes)}).into());
        let json_search_struct = json!({"query": {"bool": {"must": {"query_string": {"query": req.query}}, "filter": {"match_phrase": {"merchant_id": merchant_id}}}}}).into();
        // println!("SEARCH REQ");
        println!("{}",(search_filter_maker(index,req.clone(), &merchant_id, config.clone())));
        println!("{:?}",req);
        msearch_vector.push(json_search_struct);
    }

    let response = client
        .msearch(MsearchParts::None)
        .body(msearch_vector)
        .send()
        .await
        .map_err(|_| AnalyticsError::UnknownError)?;

    let response_body = response
        .json::<OpenMsearchOutput<Value>>()
        .await
        .map_err(|_| AnalyticsError::UnknownError)?;

    Ok(response_body
        .responses
        .into_iter()
        .zip(SearchIndex::iter())
        .map(|(index_hit, index)| GetSearchResponse {
            count: index_hit.hits.total.value,
            index,
            hits: index_hit
                .hits
                .hits
                .into_iter()
                .map(|hit| hit._source)
                .collect(),
        })
        .collect())
}

pub async fn search_results(
    req: GetSearchRequestWithIndex,
    merchant_id: &String,
    config: OpensearchConfig,
) -> CustomResult<GetSearchResponse, AnalyticsError> {
    let search_req = req.search_req;

    let client = get_opensearch_client(config.clone())
        .await
        .map_err(|_| AnalyticsError::UnknownError)?;

    let response = client
        .search(SearchParts::Index(&[&search_index_to_opensearch_index(req.index.clone(),&config.indexes)]))
        .from(search_req.offset)
        .size(search_req.count)
        .body(json!({"query": {"bool": {"must": {"query_string": {"query": search_req.query}}, "filter": {"match_phrase": {"merchant_id": merchant_id}}}}}))
        .send()
        .await
        .map_err(|_| AnalyticsError::UnknownError)?;

    let response_body = response
        .json::<OpensearchOutput<Value>>()
        .await
        .map_err(|_| AnalyticsError::UnknownError)?;

    Ok(GetSearchResponse {
        count: response_body.hits.total.value,
        index: req.index,
        hits: response_body
            .hits
            .hits
            .into_iter()
            .map(|hit| hit._source)
            .collect(),
    })
}