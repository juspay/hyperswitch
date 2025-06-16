use common_utils::{errors::CustomResult, request::RequestContent};
use masking::{ErasedMaskSerialize, Secret};
use serde::Serialize;
use storage_impl::errors::ApiClientError;

use crate::{
    core::metrics,
    routes::{app::settings::DecisionConfig, SessionState},
};

// # Consts
//

const DECISION_ENDPOINT: &str = "/rule";
const RULE_ADD_METHOD: common_utils::request::Method = common_utils::request::Method::Post;
const RULE_DELETE_METHOD: common_utils::request::Method = common_utils::request::Method::Delete;

pub const REVOKE: &str = "REVOKE";
pub const ADD: &str = "ADD";

// # Types
//

/// [`RuleRequest`] is a request body used to register a new authentication method in the proxy.
#[derive(Debug, Serialize)]
pub struct RuleRequest {
    /// [`tag`] similar to a partition key, which can be used by the decision service to tag rules
    /// by partitioning identifiers. (e.g. `tenant_id`)
    pub tag: String,
    /// [`variant`] is the type of authentication method to be registered.
    #[serde(flatten)]
    pub variant: AuthRuleType,
    /// [`expiry`] is the time **in seconds** after which the rule should be removed
    pub expiry: Option<u64>,
}

#[derive(Debug, Serialize)]
pub struct RuleDeleteRequest {
    pub tag: String,
    #[serde(flatten)]
    pub variant: AuthType,
}

#[derive(Debug, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AuthType {
    /// [`ApiKey`] is an authentication method that uses an API key. This is used with [`ApiKey`]
    ApiKey { api_key: Secret<String> },
}

#[derive(Debug, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AuthRuleType {
    /// [`ApiKey`] is an authentication method that uses an API key. This is used with [`ApiKey`]
    /// and [`PublishableKey`] authentication methods.
    ApiKey {
        api_key: Secret<String>,
        identifiers: Identifiers,
    },
}

#[allow(clippy::enum_variant_names)]
#[derive(Debug, Serialize, Clone)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Identifiers {
    /// [`ApiKey`] is an authentication method that uses an API key. This is used with [`ApiKey`]
    ApiKey {
        merchant_id: common_utils::id_type::MerchantId,
        key_id: common_utils::id_type::ApiKeyId,
    },
    /// [`PublishableKey`] is an authentication method that uses a publishable key. This is used with [`PublishableKey`]
    PublishableKey { merchant_id: String },
}

// # Decision Service
//

pub async fn add_api_key(
    state: &SessionState,
    api_key: Secret<String>,
    merchant_id: common_utils::id_type::MerchantId,
    key_id: common_utils::id_type::ApiKeyId,
    expiry: Option<u64>,
) -> CustomResult<(), ApiClientError> {
    let decision_config = if let Some(config) = &state.conf.decision {
        config
    } else {
        return Ok(());
    };

    let rule = RuleRequest {
        tag: state.tenant.schema.clone(),
        expiry,
        variant: AuthRuleType::ApiKey {
            api_key,
            identifiers: Identifiers::ApiKey {
                merchant_id,
                key_id,
            },
        },
    };

    call_decision_service(state, decision_config, rule, RULE_ADD_METHOD).await
}

pub async fn add_publishable_key(
    state: &SessionState,
    api_key: Secret<String>,
    merchant_id: common_utils::id_type::MerchantId,
    expiry: Option<u64>,
) -> CustomResult<(), ApiClientError> {
    let decision_config = if let Some(config) = &state.conf.decision {
        config
    } else {
        return Ok(());
    };

    let rule = RuleRequest {
        tag: state.tenant.schema.clone(),
        expiry,
        variant: AuthRuleType::ApiKey {
            api_key,
            identifiers: Identifiers::PublishableKey {
                merchant_id: merchant_id.get_string_repr().to_owned(),
            },
        },
    };

    call_decision_service(state, decision_config, rule, RULE_ADD_METHOD).await
}

async fn call_decision_service<T: ErasedMaskSerialize + Send + 'static>(
    state: &SessionState,
    decision_config: &DecisionConfig,
    rule: T,
    method: common_utils::request::Method,
) -> CustomResult<(), ApiClientError> {
    let mut request = common_utils::request::Request::new(
        method,
        &(decision_config.base_url.clone() + DECISION_ENDPOINT),
    );

    request.set_body(RequestContent::Json(Box::new(rule)));
    request.add_default_headers();

    let response = state
        .api_client
        .send_request(state, request, None, false)
        .await;

    match response {
        Err(error) => {
            router_env::error!("Failed while calling the decision service: {:?}", error);
            Err(error)
        }
        Ok(response) => {
            router_env::info!("Decision service response: {:?}", response);
            Ok(())
        }
    }
}

pub async fn revoke_api_key(
    state: &SessionState,
    api_key: Secret<String>,
) -> CustomResult<(), ApiClientError> {
    let decision_config = if let Some(config) = &state.conf.decision {
        config
    } else {
        return Ok(());
    };

    let rule = RuleDeleteRequest {
        tag: state.tenant.schema.clone(),
        variant: AuthType::ApiKey { api_key },
    };

    call_decision_service(state, decision_config, rule, RULE_DELETE_METHOD).await
}

/// Safety: i64::MAX < u64::MAX
#[allow(clippy::as_conversions)]
pub fn convert_expiry(expiry: time::PrimitiveDateTime) -> u64 {
    let now = common_utils::date_time::now();
    let duration = expiry - now;
    let output = duration.whole_seconds();

    match output {
        i64::MIN..=0 => 0,
        _ => output as u64,
    }
}

pub fn spawn_tracked_job<E, F>(future: F, request_type: &'static str)
where
    E: std::fmt::Debug,
    F: futures::Future<Output = Result<(), E>> + Send + 'static,
{
    metrics::API_KEY_REQUEST_INITIATED
        .add(1, router_env::metric_attributes!(("type", request_type)));
    tokio::spawn(async move {
        match future.await {
            Ok(_) => {
                metrics::API_KEY_REQUEST_COMPLETED
                    .add(1, router_env::metric_attributes!(("type", request_type)));
            }
            Err(e) => {
                router_env::error!("Error in tracked job: {:?}", e);
            }
        }
    });
}
