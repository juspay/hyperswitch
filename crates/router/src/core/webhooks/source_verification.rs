//! Source verification for incoming webhooks.

use actix_web::http::header::HeaderMap;
use common_enums::connector_enums::Connector;
use common_utils::errors::ErrorSwitch;
use error_stack::{report, ResultExt};
use hyperswitch_interfaces::api::{ConnectorCommon, ConnectorSpecifications};
use router_env::logger;

use super::ip_verification::validate_source_ip_against_allowed_ip_ranges;
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

/// The outcome of an incoming webhook network verification check.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum VerificationStatus {
    /// The source passed this check.
    Verified,
    /// The connector requires this check, but runtime config disables it; the source remains unverified.
    RequiredButDisabled,
    /// The connector does not require this check.
    NotRequired,
}

impl VerificationStatus {
    fn is_verified(self) -> bool {
        self == Self::Verified
    }

    fn is_required_but_disabled(self) -> bool {
        self == Self::RequiredButDisabled
    }
}

/// Retains each network check's outcome until connector source verification is resolved.
#[derive(Debug)]
pub(super) struct NetworkVerification {
    mtls: VerificationStatus,
    source_ip: VerificationStatus,
}

impl NetworkVerification {
    /// A required check disabled by config keeps the source unverified even when another check passes.
    /// Use connector source verification only when no network check is required.
    pub(super) fn resolve_source_verified(self, connector_source_verified: bool) -> bool {
        if self.mtls.is_required_but_disabled() || self.source_ip.is_required_but_disabled() {
            false
        } else if self.mtls.is_verified() || self.source_ip.is_verified() {
            true
        } else {
            connector_source_verified
        }
    }
}

impl<'a> IncomingWebhookVerifier<'a> {
    pub(super) fn new(settings: &'a IncomingWebhooksSettings) -> Self {
        Self { settings }
    }

    /// Run router-managed source verification before processing the webhook.
    /// Use the resolved connector name for policy lookup; an adapter's ID can differ.
    pub(super) async fn verify(
        &self,
        state: &SessionState,
        connector: &ConnectorEnum,
        connector_name: &str,
        request_headers: &HeaderMap,
    ) -> RouterResult<NetworkVerification> {
        Ok(NetworkVerification {
            mtls: self.verify_mtls(state, connector, request_headers).await?,
            source_ip: self
                .verify_source_ip(state, connector, connector_name, request_headers)
                .await?,
        })
    }

    /// Read the incoming webhooks allowed IP ranges config and validate that the IP is allowed for
    /// this connector.
    ///
    /// Only connectors requiring IP verification consult the config.
    /// The global IP verification toggle defaults to false and bypasses policy lookup when disabled.
    /// A missing or invalid policy or current connector's IP ranges is a configuration error.
    /// Malformed entries invalidate the entire policy during deserialization.
    async fn verify_source_ip(
        &self,
        state: &SessionState,
        connector: &ConnectorEnum,
        connector_name: &str,
        request_headers: &HeaderMap,
    ) -> RouterResult<VerificationStatus> {
        if connector.is_webhook_ip_verification_required() {
            let dimensions = dimension_state::Dimensions::new();
            let enabled = dimensions
                .get_incoming_webhook_ip_verification_enabled(
                    state.store.as_ref(),
                    state.superposition_service.as_ref(),
                    None,
                )
                .await;
            if enabled {
                let connector_name = connector_name
                    .parse::<Connector>()
                    .change_context(errors::ApiErrorResponse::IncorrectConnectorNameGiven)?;
                let policy = dimensions
                    .get_incoming_webhook_allowed_ip_ranges(
                        state.store.as_ref(),
                        state.superposition_service.as_ref(),
                        None,
                    )
                    .await;

                validate_source_ip_against_allowed_ip_ranges(
                    policy,
                    connector_name,
                    request_headers,
                    self.settings.source_ip_header.as_deref(),
                )
                .map_err(|error| {
                    let api_error: errors::ApiErrorResponse = error.current_context().switch();
                    let reason: &'static str = (*error.current_context()).into();
                    metrics::WEBHOOK_IP_VERIFICATION_FAILURE_COUNT.add(
                        1,
                        router_env::metric_attributes!(
                            ("connector", connector_name.to_string()),
                            ("reason", reason)
                        ),
                    );
                    error.change_context(api_error).attach_printable(format!(
                        "Webhook source IP verification failed: {reason}"
                    ))
                })
            } else {
                logger::info!(
                    connector = %connector_name,
                    "Skipping incoming webhook IP verification because webhooks.incoming_webhooks_ip_verification_enabled is disabled"
                );
                Ok(VerificationStatus::RequiredButDisabled)
            }
        } else {
            Ok(VerificationStatus::NotRequired)
        }
    }

    /// Enforce mTLS before processing when enabled in Superposition for the connector.
    /// Trusted ingress must set the configured header to `true` only on verified mTLS
    /// endpoints and strip caller-supplied values from all other endpoints.
    async fn verify_mtls(
        &self,
        state: &SessionState,
        connector: &ConnectorEnum,
        request_headers: &HeaderMap,
    ) -> RouterResult<VerificationStatus> {
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
                Ok(VerificationStatus::RequiredButDisabled)
            }
        } else {
            Ok(VerificationStatus::NotRequired)
        }
    }

    /// Validate the header after connector requirements and enablement have been checked.
    fn validate_mtls(
        &self,
        connector: &ConnectorEnum,
        request_headers: &HeaderMap,
    ) -> RouterResult<VerificationStatus> {
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
                    Ok(VerificationStatus::Verified)
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
    use hyperswitch_interfaces::api::ConnectorSpecifications;

    use super::{IncomingWebhookVerifier, NetworkVerification, VerificationStatus};
    use crate::{
        configs::settings::IncomingWebhooksSettings,
        core::{
            configs::dimension_config::{
                IncomingWebhookIpVerificationEnabled, IncomingWebhookMtlsVerificationEnabled,
            },
            errors,
        },
        services::connector_integration_interface::ConnectorEnum,
        types::api::ConnectorData,
    };

    const TEST_MTLS_HEADER: &str = "x-test-mtls-verified";

    fn settings() -> IncomingWebhooksSettings {
        IncomingWebhooksSettings {
            mtls_verified_header: Some(TEST_MTLS_HEADER.to_string()),
            ..Default::default()
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
    fn ip_verification_is_disabled_by_default() {
        assert!(!IncomingWebhookIpVerificationEnabled::default_value());
    }

    #[test]
    fn mtls_verification_is_disabled_by_default() {
        assert!(!IncomingWebhookMtlsVerificationEnabled::default_value());
    }

    #[test]
    fn requires_ip_verification_only_for_opted_in_connectors() {
        for (name, required) in [("envoy", true), ("netcetera", true), ("stripe", false)] {
            assert_eq!(
                connector(name).is_webhook_ip_verification_required(),
                required,
                "{name}"
            );
        }
    }

    #[test]
    fn resolves_source_verification_from_all_network_checks() {
        use VerificationStatus::{NotRequired, RequiredButDisabled, Verified};

        for (mtls, source_ip, expected) in [
            (NotRequired, NotRequired, None),
            (NotRequired, Verified, Some(true)),
            (NotRequired, RequiredButDisabled, Some(false)),
            (Verified, NotRequired, Some(true)),
            (Verified, Verified, Some(true)),
            (Verified, RequiredButDisabled, Some(false)),
            (RequiredButDisabled, NotRequired, Some(false)),
            (RequiredButDisabled, Verified, Some(false)),
            (RequiredButDisabled, RequiredButDisabled, Some(false)),
        ] {
            for connector_source_verified in [false, true] {
                let verification = NetworkVerification { mtls, source_ip };
                assert_eq!(
                    verification.resolve_source_verified(connector_source_verified),
                    expected.unwrap_or(connector_source_verified),
                    "mTLS={mtls:?}, IP={source_ip:?}, connector={connector_source_verified}"
                );
            }
        }
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
                VerificationStatus::Verified
            );
        }
    }

    #[test]
    fn uses_only_the_configured_header() {
        let settings = IncomingWebhooksSettings {
            mtls_verified_header: Some("x-custom-mtls-verified".to_string()),
            ..Default::default()
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
