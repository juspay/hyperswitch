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
                state
                    .conf
                    .pm_filters
                    .0
                    .get(connector_name)
                    .and_then(|selected_connector| {
                        selected_connector.0.get(
                            &settings::PaymentMethodFilterKey::PaymentMethodType(
                                *payment_method_type,
                            ),
                        )
                    });

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
