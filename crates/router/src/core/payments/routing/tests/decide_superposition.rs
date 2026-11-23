use common_utils::{
    errors::{CustomResult, ValidationError},
    id_type,
};
use external_services::superposition::{
    Config, SuperpositionClient, SuperpositionClientConfig, SuperpositionError,
};
use hyperswitch_masking::Secret;
use serde_json::{json, Value};
use wiremock::{
    matchers::{method, path},
    Mock, MockServer, ResponseTemplate,
};

use crate::core::configs::{
    dimension_config::DecisionEngineDecideEnabled,
    dimension_state::{
        Dimensions, DimensionsWithProcessorAndProviderMerchantIdAndProfileId, ProviderMerchantId,
    },
};

fn dimensions(
    processor: &str,
    provider: &str,
    profile: &str,
) -> CustomResult<DimensionsWithProcessorAndProviderMerchantIdAndProfileId, ValidationError> {
    Ok(Dimensions::new()
        .with_processor_merchant_id(
            id_type::MerchantId::try_from(std::borrow::Cow::Owned(processor.to_string()))?.into(),
        )
        .with_provider_merchant_id(ProviderMerchantId::new(id_type::MerchantId::try_from(
            std::borrow::Cow::Owned(provider.to_string()),
        )?))
        .with_profile_id(profile.parse()?))
}

fn config(default: Value, scoped_override: Option<bool>) -> Value {
    let (contexts, overrides) = if let Some(enabled) = scoped_override {
        (
            json!([{
                "id": "decide_rollout_context",
                "condition": {
                    "processor_merchant_id": "processor_one",
                    "provider_merchant_id": "provider_one",
                    "profile_id": "pro_enabled"
                },
                "priority": 1,
                "weight": 1,
                "override_with_keys": ["decide_rollout_override"]
            }]),
            json!({
                "decide_rollout_override": {
                    (DecisionEngineDecideEnabled::SUPERPOSITION_KEY): enabled
                }
            }),
        )
    } else {
        (json!([]), json!({}))
    };
    json!({
        "default_configs": default,
        "contexts": contexts,
        "overrides": overrides,
        "dimensions": {
            "processor_merchant_id": {
                "schema": {"type": "string"}, "position": 1,
                "dimension_type": {"REGULAR": {}}, "dependency_graph": {}
            },
            "provider_merchant_id": {
                "schema": {"type": "string"}, "position": 2,
                "dimension_type": {"REGULAR": {}}, "dependency_graph": {}
            },
            "profile_id": {
                "schema": {"type": "string"}, "position": 3,
                "dimension_type": {"REGULAR": {}}, "dependency_graph": {}
            }
        }
    })
}

async fn client(
    config: Value,
) -> CustomResult<(SuperpositionClient, MockServer), SuperpositionError> {
    let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/config"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("last-modified", "2026-11-16T00:00:00Z")
                .insert_header("x-config-version", "decide_rollout_test")
                .set_body_json(config),
        )
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/experiment-config"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("last-modified", "2026-11-16T00:00:00Z")
                .set_body_json(json!({"experiments": [], "experiment_groups": []})),
        )
        .mount(&server)
        .await;

    let client = SuperpositionClient::new(
        SuperpositionClientConfig {
            endpoint: server.uri(),
            token: Secret::new("test_superposition_token".to_string()),
            org_id: "test_org".to_string(),
            workspace_id: "test_workspace".to_string(),
            polling_interval_milliseconds: 3_600_000,
            request_timeout_milliseconds: Some(100),
            backup_file_path: None,
        },
        "decide_rollout_test",
    )
    .await?;
    Ok((client, server))
}

// Superposition registers its provider on the global OpenFeature singleton. Keep
// all fixture replacements in one serialized test so clients cannot cross providers.
#[tokio::test]
#[serial_test::serial]
async fn decide_superposition_resolves_scoped_rollout_and_defaults_safely() {
    let enabled = dimensions("processor_one", "provider_one", "pro_enabled")
        .expect("valid rollout dimensions");
    let other_profile =
        dimensions("processor_one", "provider_one", "pro_other").expect("valid rollout dimensions");
    let other_processor = dimensions("processor_other", "provider_one", "pro_enabled")
        .expect("valid rollout dimensions");
    let other_provider = dimensions("processor_one", "provider_other", "pro_enabled")
        .expect("valid rollout dimensions");
    let flag = DecisionEngineDecideEnabled::SUPERPOSITION_KEY;

    {
        let (client, _server) = client(config(json!({(flag): false}), Some(true)))
            .await
            .expect("initialize mock Superposition provider");
        assert!(enabled.get_decision_engine_decide_enabled(&client).await);
        for other in [&other_profile, &other_processor, &other_provider] {
            assert!(
                !other.get_decision_engine_decide_enabled(&client).await,
                "a different merchant or profile must retain hybrid"
            );
        }
    }

    for default in [json!({"unrelated_config": true}), json!({(flag): "true"})] {
        let (client, _server) = client(config(default, None))
            .await
            .expect("initialize mock Superposition provider");
        assert!(
            !enabled.get_decision_engine_decide_enabled(&client).await,
            "missing or incorrectly typed rollout config must retain hybrid"
        );
    }

    {
        let (client, _server) = client(config(json!({(flag): true}), Some(false)))
            .await
            .expect("initialize mock Superposition provider");
        assert!(
            !enabled.get_decision_engine_decide_enabled(&client).await,
            "a scoped false override must roll this profile back to hybrid"
        );
        assert!(
            other_profile
                .get_decision_engine_decide_enabled(&client)
                .await
        );
    }
}
