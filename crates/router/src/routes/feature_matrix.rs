use actix_web::{web, HttpRequest, Responder};
use api_models::{connector_enums::Connector, feature_matrix};
use common_enums::enums;
use hyperswitch_domain_models::{
    api::ApplicationResponse, router_response_types::PaymentMethodTypeMetadata,
};
use hyperswitch_interfaces::api::{ConnectorCommon, ConnectorSpecifications};
use router_env::{instrument, tracing, Flow};
use strum::IntoEnumIterator;

use crate::{
    self as app,
    core::{api_locking::LockAction, errors::RouterResponse},
    services::{api, authentication as auth, connector_integration_interface::ConnectorEnum},
    settings,
    types::api::{self as api_types, payments as payment_types},
};

#[instrument(skip_all)]
pub async fn fetch_feature_matrix(
    state: web::Data<app::AppState>,
    req: HttpRequest,
    json_payload: Option<web::Json<payment_types::FeatureMatrixRequest>>,
) -> impl Responder {
    let flow: Flow = Flow::FeatureMatrix;
    let payload = json_payload
        .map(|json_request| json_request.into_inner())
        .unwrap_or_else(|| payment_types::FeatureMatrixRequest { connectors: None });

    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        payload,
        |state, (), req, _| generate_feature_matrix(state, req),
        &auth::NoAuth,
        LockAction::NotApplicable,
    ))
    .await
}

pub async fn generate_feature_matrix(
    state: app::SessionState,
    req: payment_types::FeatureMatrixRequest,
) -> RouterResponse<feature_matrix::FeatureMatrixListResponse> {
    let connector_list = req
        .connectors
        .unwrap_or_else(|| Connector::iter().collect());

    let feature_matrix_response: Vec<payment_types::ConnectorFeatureMatrixResponse> =
        connector_list
            .into_iter()
            .filter_map(|connector_name| {
                api_types::feature_matrix::FeatureMatrixConnectorData::convert_connector(
                    &connector_name.to_string(),
                )
                .inspect_err(|_| {
                    router_env::logger::warn!("Failed to fetch {:?} details", connector_name)
                })
                .ok()
                .and_then(|connector| {
                    build_connector_feature_details(&state, connector, connector_name.to_string())
                })
            })
            .collect();

    Ok(ApplicationResponse::Json(
        payment_types::FeatureMatrixListResponse {
            connector_count: feature_matrix_response.len(),
            connectors: feature_matrix_response,
        },
    ))
}

fn build_connector_feature_details(
    state: &app::SessionState,
    connector: ConnectorEnum,
    connector_name: String,
) -> Option<feature_matrix::ConnectorFeatureMatrixResponse> {
    let connector_integration_features = connector.get_supported_payment_methods();
    let supported_payment_methods =
        connector_integration_features.map(|connector_integration_feature_data| {
            connector_integration_feature_data
                .iter()
                .flat_map(|(payment_method, supported_payment_method_types)| {
                    build_payment_method_wise_feature_details(
                        state,
                        &connector_name,
                        *payment_method,
                        supported_payment_method_types,
                    )
                })
                .collect::<Vec<feature_matrix::SupportedPaymentMethod>>()
        });
    let supported_webhook_flows = connector
        .get_supported_webhook_flows()
        .map(|webhook_flows| webhook_flows.to_vec());
    let connector_about = connector.get_connector_about();

    connector_about.map(
        |connector_about| feature_matrix::ConnectorFeatureMatrixResponse {
            name: connector_name.to_uppercase(),
            display_name: connector_about.display_name.to_string(),
            description: connector_about.description.to_string(),
            base_url: Some(connector.base_url(&state.conf.connectors).to_string()),
            integration_status: connector_about.integration_status,
            category: connector_about.connector_type,
            supported_webhook_flows,
            supported_payment_methods,
        },
    )
}

fn build_payment_method_wise_feature_details(
    state: &app::SessionState,
    connector_name: &str,
    payment_method: enums::PaymentMethod,
    supported_payment_method_types: &PaymentMethodTypeMetadata,
) -> Vec<feature_matrix::SupportedPaymentMethod> {
    supported_payment_method_types
        .iter()
        .map(|(payment_method_type, feature_metadata)| {
            let payment_method_type_config =
                get_pm_filter(&state.conf.pm_filters, connector_name, *payment_method_type);

            let supported_countries = payment_method_type_config.and_then(|config| {
                config.country.clone().map(|set| {
                    set.into_iter()
                        .map(common_enums::CountryAlpha2::from_alpha2_to_alpha3)
                        .collect::<std::collections::HashSet<_>>()
                })
            });

            let supported_currencies =
                payment_method_type_config.and_then(|config| config.currency.clone());

            feature_matrix::SupportedPaymentMethod {
                payment_method,
                payment_method_type: *payment_method_type,
                payment_method_type_display_name: payment_method_type.to_display_name(),
                mandates: feature_metadata.mandates,
                refunds: feature_metadata.refunds,
                supported_capture_methods: feature_metadata.supported_capture_methods.clone(),
                payment_method_specific_features: feature_metadata.specific_features.clone(),
                supported_countries,
                supported_currencies,
            }
        })
        .collect()
}

/// Retrieves the payment-method-type level filter (country/currency market
/// restrictions) configured under `[pm_filters.<connector>]` for the connector.
fn get_pm_filter<'a>(
    pm_filters: &'a settings::ConnectorFilters,
    connector_name: &str,
    payment_method_type: enums::PaymentMethodType,
) -> Option<&'a settings::CurrencyCountryFlowFilter> {
    pm_filters
        .0
        .get(connector_name)
        .and_then(|selected_connector| {
            selected_connector
                .0
                .get(&settings::PaymentMethodFilterKey::PaymentMethodType(
                    payment_method_type,
                ))
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::connector::Flexiti;

    fn flexiti_pm_filters() -> settings::ConnectorFilters {
        config::Config::builder()
            .add_source(config::File::from_str(
                "[flexiti]\nflexiti = { country = \"CA\", currency = \"CAD\" }\n",
                config::FileFormat::Toml,
            ))
            .build()
            .expect("valid pm_filters toml")
            .try_deserialize()
            .expect("valid ConnectorFilters")
    }

    #[test]
    fn flexiti_declares_flexiti_type_not_klarna() {
        let supported = Flexiti::new()
            .get_supported_payment_methods()
            .expect("Flexiti declares supported payment methods");
        let pay_later = supported
            .get(&enums::PaymentMethod::PayLater)
            .expect("Flexiti declares PayLater");
        assert!(pay_later.contains_key(&enums::PaymentMethodType::Flexiti));
        assert!(!pay_later.contains_key(&enums::PaymentMethodType::Klarna));
    }

    #[test]
    fn flexiti_market_filters_resolve_for_declared_type() {
        let filters = flexiti_pm_filters();

        let filter = get_pm_filter(&filters, "flexiti", enums::PaymentMethodType::Flexiti)
            .expect("filter must resolve for the declared payment method type");
        assert!(filter
            .currency
            .clone()
            .unwrap_or_default()
            .contains(&enums::Currency::CAD));
        assert!(filter
            .country
            .clone()
            .unwrap_or_default()
            .contains(&enums::CountryAlpha2::CA));

        // The original defect (#14611): the declared Klarna type had no
        // matching key under [pm_filters.flexiti], so the feature matrix
        // rendered empty country/currency cells.
        assert!(get_pm_filter(&filters, "flexiti", enums::PaymentMethodType::Klarna).is_none());
    }
}
