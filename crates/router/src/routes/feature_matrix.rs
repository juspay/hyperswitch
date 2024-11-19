use api_models::feature_matrix;
use hyperswitch_domain_models::api::ApplicationResponse;
use hyperswitch_interfaces::api::ConnectorCommon;
use actix_web::{web, HttpRequest, Responder};
use router_env::{instrument, tracing, Flow};
use api_models::connector_enums::Connector;
use strum::IntoEnumIterator;
use crate::{
    self as app,
    core::{
        errors::RouterResponse,
        api_locking::LockAction
    },
    services::{api, authentication as auth},
    types::api::{
            self as api_types,
            payments as payment_types,
        }
    
};


#[cfg(all(feature = "olap", feature = "v1"))]
#[instrument(skip_all)]
pub async fn fetch_connector_feature_matrix(
    state: web::Data<app::AppState>,
    req: HttpRequest,
    json_payload: web::Json<payment_types::FeatureMatrixRequest>,
) -> impl Responder {
    let flow: Flow = Flow::FeatureMatrix;
    let payload = json_payload.into_inner();
    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        payload,
        |state, (), req , _| {
            connector_feature_matrix(
                state,
                req
            )
        },
        &auth::NoAuth,
        LockAction::NotApplicable,
    )).await

}


#[cfg(feature = "v1")]
pub async fn connector_feature_matrix(
    _state: app::SessionState,
    req: payment_types::FeatureMatrixRequest,
) -> RouterResponse<feature_matrix::FeatureMatrixListResponse> {
    let connector_list = req.connectors.unwrap_or_else(|| Connector::iter().collect());
    let feature_matrix_response: Vec<payment_types::FeatureMatrixResponse> = connector_list
        .into_iter()
        .filter_map(|connector_name| {
            api_types::ConnectorData::convert_connector(&connector_name.to_string()).ok().and_then(|connector| {
                connector.get_supported_payment_methods().map(|supported_methods| {
                    let payment_method_types = supported_methods
                        .into_iter()
                        .map(|(payment_method, supported_payment_method_types)| {
                            let payment_methods = supported_payment_method_types
                                .into_iter()
                                .map(|(payment_method_type, feature_metadata)| {
                                    feature_matrix::SupportedPaymentMethod {
                                        payment_method: payment_method_type,
                                        availability_status: feature_metadata.availability_status,
                                        supports_mandates: feature_metadata.supports_mandates,
                                    }
                                })
                                .collect();

                            feature_matrix::SupportedPaymentMethodTypes {
                                payment_method_type: payment_method,
                                payment_methods,
                            }
                        })
                        .collect();

                    payment_types::FeatureMatrixResponse {
                        connector: connector_name,
                        payment_method_types,
                    }
                })
            })
        })
        .collect();

    Ok(ApplicationResponse::Json(payment_types::FeatureMatrixListResponse {
        size: feature_matrix_response.len(),
        data: feature_matrix_response,
    }))
}