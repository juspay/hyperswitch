//! Source verification for incoming webhooks.

use actix_web::http::header::HeaderMap;
use common_enums::connector_enums::Connector;
use error_stack::{report, ResultExt};
use hyperswitch_interfaces::api::{ConnectorCommon, ConnectorSpecifications};
use router_env::logger;

use crate::{
    configs::settings::IncomingWebhooksSettings,
    core::{
        configs::dimension_state,
        errors::{self, RouterResult},
        metrics,
    },
    routes::SessionState,
    services::connector_integration_interface::ConnectorEnum,
};

/// Verifies incoming webhook sources using connector requirements and deployment settings.
#[derive(Debug)]
pub(super) struct IncomingWebhookVerifier<'a> {
    settings: &'a IncomingWebhooksSettings,
}

/// The outcome of incoming webhook network verification, such as mTLS or source IP checks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum NetworkVerification {
    /// The source passed network verification.
    Verified,
    /// Required verification was disabled; the source remains unverified.
    Skipped,
    /// The connector does not require network verification.
    NotRequired,
}

impl NetworkVerification {
    /// Use connector source verification only when network verification is not required.
    pub(super) fn resolve_source_verified(self, connector_source_verified: bool) -> bool {
        match self {
            Self::Verified => true,
            Self::Skipped => false,
            Self::NotRequired => connector_source_verified,
        }
    }
}

impl<'a> IncomingWebhookVerifier<'a> {
    pub(super) fn new(settings: &'a IncomingWebhooksSettings) -> Self {
        Self { settings }
    }

    /// Run router-managed source verification before processing the webhook.
    pub(super) async fn verify(
        &self,
        state: &SessionState,
        connector: &ConnectorEnum,
        request_headers: &HeaderMap,
    ) -> RouterResult<NetworkVerification> {
        self.verify_mtls(state, connector, request_headers).await
    }

    /// Enforce mTLS before processing when enabled in Superposition for the connector.
    /// Trusted ingress must set the configured header to `true` only on verified mTLS
    /// endpoints and strip caller-supplied values from all other endpoints.
    async fn verify_mtls(
        &self,
        state: &SessionState,
        connector: &ConnectorEnum,
        request_headers: &HeaderMap,
    ) -> RouterResult<NetworkVerification> {
        if connector.is_webhook_mtls_required() {
            let connector_name = connector
                .id()
                .parse::<Connector>()
                .change_context(errors::ApiErrorResponse::IncorrectConnectorNameGiven)?;
            let enabled = dimension_state::Dimensions::new()
                .with_connector(connector_name)
                .get_incoming_webhook_mtls_verification_enabled(
                    state.store.as_ref(),
                    state.superposition_service.as_ref(),
                    None,
                )
                .await;
            if enabled {
                self.validate_mtls(connector, request_headers)
            } else {
                logger::info!(
                    connector = %connector_name,
                    "Skipping incoming webhook mTLS verification because incoming_webhooks.mtls_verification_enabled is disabled"
                );
                Ok(NetworkVerification::Skipped)
            }
        } else {
            Ok(NetworkVerification::NotRequired)
        }
    }

    /// Validate the header after connector requirements and enablement have been checked.
    fn validate_mtls(
        &self,
        connector: &ConnectorEnum,
        request_headers: &HeaderMap,
    ) -> RouterResult<NetworkVerification> {
        let connector_name = connector.id();
        self.is_mtls_verification_header_present(request_headers)
            .and_then(|verified| {
                if !verified {
                    let message = "Incoming webhook mTLS verification failed: the request has missing or invalid verification header";
                    Err(
                        report!(errors::ApiErrorResponse::WebhookAuthenticationFailed)
                            .attach_printable(message),
                    )
                } else {
                    Ok(NetworkVerification::Verified)
                }
            })
            .inspect_err(|_| {
                metrics::WEBHOOK_MTLS_VERIFICATION_FAILURE_COUNT.add(
                    1,
                    router_env::metric_attributes!(("connector", connector_name.to_string())),
                );
            })
    }

    /// Require exactly one configured mTLS verification header with the value `true`.
    fn is_mtls_verification_header_present(
        &self,
        request_headers: &HeaderMap,
    ) -> RouterResult<bool> {
        match self
            .settings
            .mtls_verified_header
            .as_deref()
            .filter(|header_name| !header_name.is_empty())
        {
            Some(header_name) => {
                let mut header_values = request_headers.get_all(header_name);
                Ok(header_values
                    .next()
                    .is_some_and(|value| value.as_bytes() == b"true")
                    && header_values.next().is_none())
            }
            None => Err(
                report!(errors::ApiErrorResponse::WebhookAuthenticationFailed)
                    .attach_printable("incoming_webhooks.mtls_verified_header is not configured"),
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use actix_web::http::header::{HeaderMap, HeaderName, HeaderValue};
    use external_services::superposition::Config;

    use super::{IncomingWebhookVerifier, NetworkVerification};
    use crate::{
        configs::settings::IncomingWebhooksSettings,
        core::{configs::dimension_config::IncomingWebhookMtlsVerificationEnabled, errors},
        services::connector_integration_interface::ConnectorEnum,
        types::api::ConnectorData,
    };

    const TEST_MTLS_HEADER: &str = "x-test-mtls-verified";

    fn settings() -> IncomingWebhooksSettings {
        IncomingWebhooksSettings {
            mtls_verified_header: Some(TEST_MTLS_HEADER.to_string()),
        }
    }

    fn connector(name: &str) -> ConnectorEnum {
        ConnectorData::convert_connector(name).unwrap()
    }

    fn headers_with(values: &[&'static str]) -> HeaderMap {
        let mut request_headers = HeaderMap::new();
        for value in values {
            request_headers.append(
                HeaderName::from_static(TEST_MTLS_HEADER),
                HeaderValue::from_static(value),
            );
        }
        request_headers
    }

    #[test]
    fn mtls_verification_is_disabled_by_default() {
        assert!(!IncomingWebhookMtlsVerificationEnabled::default_value());
    }

    #[test]
    fn skipped_network_verification_does_not_trust_connector_result() {
        assert!(!NetworkVerification::Skipped.resolve_source_verified(true));
        assert!(!NetworkVerification::Skipped.resolve_source_verified(false));
    }

    #[test]
    fn network_verification_establishes_source_verification() {
        assert!(NetworkVerification::Verified.resolve_source_verified(false));
        assert!(NetworkVerification::Verified.resolve_source_verified(true));
    }

    #[test]
    fn connectors_without_network_verification_use_connector_result() {
        assert!(NetworkVerification::NotRequired.resolve_source_verified(true));
        assert!(!NetworkVerification::NotRequired.resolve_source_verified(false));
    }

    #[test]
    fn accepts_single_true_value() {
        assert!(IncomingWebhookVerifier::new(&settings())
            .is_mtls_verification_header_present(&headers_with(&["true"]))
            .unwrap());
    }

    #[test]
    fn rejects_missing_header() {
        assert!(!IncomingWebhookVerifier::new(&settings())
            .is_mtls_verification_header_present(&headers_with(&[]))
            .unwrap());
    }

    #[test]
    fn rejects_other_values() {
        for value in ["false", "TRUE", "1", "true,true", " true", ""] {
            assert!(!IncomingWebhookVerifier::new(&settings())
                .is_mtls_verification_header_present(&headers_with(&[value]))
                .unwrap());
        }
    }

    #[test]
    fn rejects_repeated_header() {
        assert!(!IncomingWebhookVerifier::new(&settings())
            .is_mtls_verification_header_present(&headers_with(&["true", "true"]))
            .unwrap());
    }

    #[test]
    fn rejects_requests_without_verified_mtls() {
        let settings = settings();
        let verifier = IncomingWebhookVerifier::new(&settings);
        for name in ["santander", "worldpayxml"] {
            let connector = connector(name);
            let error = verifier
                .validate_mtls(&connector, &HeaderMap::new())
                .unwrap_err();
            assert!(matches!(
                error.current_context(),
                errors::ApiErrorResponse::WebhookAuthenticationFailed
            ));
            assert!(format!("{error:?}").contains("missing or invalid verification header"));
            assert_eq!(
                verifier
                    .validate_mtls(&connector, &headers_with(&["true"]))
                    .unwrap(),
                NetworkVerification::Verified
            );
        }
    }

    #[test]
    fn uses_only_the_configured_header() {
        let settings = IncomingWebhooksSettings {
            mtls_verified_header: Some("x-custom-mtls-verified".to_string()),
        };
        let verifier = IncomingWebhookVerifier::new(&settings);
        let connector = connector("santander");
        assert!(verifier
            .validate_mtls(&connector, &headers_with(&["true"]))
            .is_err());
        let mut request_headers = HeaderMap::new();
        request_headers.append(
            HeaderName::from_static("x-custom-mtls-verified"),
            HeaderValue::from_static("true"),
        );
        assert!(verifier.validate_mtls(&connector, &request_headers).is_ok());
        request_headers.append(
            HeaderName::from_static("x-custom-mtls-verified"),
            HeaderValue::from_static("true"),
        );
        assert!(verifier
            .validate_mtls(&connector, &request_headers)
            .is_err());
    }

    #[test]
    fn deserializes_default_and_custom_header_names() {
        let settings: IncomingWebhooksSettings =
            serde_json::from_value(serde_json::json!({})).unwrap();
        assert!(settings.mtls_verified_header.is_none());
        let connector = connector("worldpayxml");
        let error = IncomingWebhookVerifier::new(&settings)
            .validate_mtls(&connector, &headers_with(&["true"]))
            .unwrap_err();
        assert!(format!("{error:?}")
            .contains("incoming_webhooks.mtls_verified_header is not configured"));
        let settings: IncomingWebhooksSettings = serde_json::from_value(serde_json::json!({
            "mtls_verified_header": "X-Custom-Mtls-Verified"
        }))
        .unwrap();
        let mut request_headers = HeaderMap::new();
        request_headers.insert(
            HeaderName::from_static("x-custom-mtls-verified"),
            HeaderValue::from_static("true"),
        );
        assert!(IncomingWebhookVerifier::new(&settings)
            .validate_mtls(&connector, &request_headers)
            .is_ok());
    }
}
