use utoipa::ToSchema;

use crate::enums as api_enums;

#[derive(Debug, serde::Deserialize, serde::Serialize, ToSchema)]
pub struct GsmCreateRequest {
    /// The connector through which payment has gone through
    #[schema(value_type = Connector)]
    pub connector: api_enums::Connector,
    /// The flow in which the code and message occurred for a connector
    pub flow: String,
    /// The sub_flow in which the code and message occurred  for a connector
    pub sub_flow: String,
    /// code received from the connector
    pub code: String,
    /// message received from the connector
    pub message: String,
    /// status provided by the router
    pub status: String,
    /// optional error provided by the router
    pub router_error: Option<String>,
    /// decision to be taken for auto retries flow
    /// **Deprecated**: This field is now included as part of `feature_data` under the `Retry` variant.
    #[schema(deprecated)]
    #[schema(value_type = GsmDecision)]
    pub decision: api_enums::GsmDecision,
    /// indicates if step_up retry is possible
    /// **Deprecated**: This field is now included as part of `feature_data` under the `Retry` variant.
    #[schema(deprecated)]
    pub step_up_possible: bool,
    /// error code unified across the connectors
    pub unified_code: Option<String>,
    /// error message unified across the connectors
    pub unified_message: Option<String>,
    /// category in which error belongs to
    #[schema(value_type = Option<ErrorCategory>)]
    pub error_category: Option<api_enums::ErrorCategory>,
    /// indicates if retry with pan is possible
    /// **Deprecated**: This field is now included as part of `feature_data` under the `Retry` variant.
    #[schema(deprecated)]
    pub clear_pan_possible: bool,
    /// Indicates the GSM feature associated with the request,
    /// such as retry mechanisms or other specific functionalities provided by the system.
    #[schema(value_type = Option<GsmFeature>)]
    pub feature: Option<api_enums::GsmFeature>,
    /// Contains the data relevant to the specified GSM feature, if applicable.
    /// For example, if the `feature` is `Retry`, this will include configuration
    /// details specific to the retry behavior.
    #[schema(value_type = Option<GsmFeatureData>)]
    pub feature_data: Option<common_types::domain::GsmFeatureData>,
}

#[derive(Debug, serde::Deserialize, serde::Serialize, ToSchema)]
pub struct GsmRetrieveRequest {
    /// The connector through which payment has gone through
    #[schema(value_type = Connector)]
    pub connector: api_enums::Connector,
    /// The flow in which the code and message occurred for a connector
    pub flow: String,
    /// The sub_flow in which the code and message occurred  for a connector
    pub sub_flow: String,
    /// code received from the connector
    pub code: String,
    /// message received from the connector
    pub message: String,
}

#[derive(Debug, serde::Deserialize, serde::Serialize, ToSchema)]
pub struct GsmUpdateRequest {
    /// The connector through which payment has gone through
    pub connector: String,
    /// The flow in which the code and message occurred for a connector
    pub flow: String,
    /// The sub_flow in which the code and message occurred  for a connector
    pub sub_flow: String,
    /// code received from the connector
    pub code: String,
    /// message received from the connector
    pub message: String,
    /// status provided by the router
    pub status: Option<String>,
    /// optional error provided by the router
    pub router_error: Option<String>,
    /// decision to be taken for auto retries flow
    /// **Deprecated**: This field is now included as part of `feature_data` under the `Retry` variant.
    #[schema(deprecated)]
    #[schema(value_type = Option<GsmDecision>)]
    pub decision: Option<api_enums::GsmDecision>,
    /// indicates if step_up retry is possible
    /// **Deprecated**: This field is now included as part of `feature_data` under the `Retry` variant.
    #[schema(deprecated)]
    pub step_up_possible: Option<bool>,
    /// error code unified across the connectors
    pub unified_code: Option<String>,
    /// error message unified across the connectors
    pub unified_message: Option<String>,
    /// category in which error belongs to
    #[schema(value_type = Option<ErrorCategory>)]
    pub error_category: Option<api_enums::ErrorCategory>,
    /// indicates if retry with pan is possible
    /// **Deprecated**: This field is now included as part of `feature_data` under the `Retry` variant.
    #[schema(deprecated)]
    pub clear_pan_possible: Option<bool>,
    /// Indicates the GSM feature associated with the request,
    /// such as retry mechanisms or other specific functionalities provided by the system.
    #[schema(value_type = Option<GsmFeature>)]
    pub feature: Option<api_enums::GsmFeature>,
    /// Contains the data relevant to the specified GSM feature, if applicable.
    /// For example, if the `feature` is `Retry`, this will include configuration
    /// details specific to the retry behavior.
    #[schema(value_type = Option<GsmFeatureData>)]
    pub feature_data: Option<common_types::domain::GsmFeatureData>,
}

#[derive(Debug, serde::Deserialize, serde::Serialize, ToSchema)]
pub struct GsmDeleteRequest {
    /// The connector through which payment has gone through
    pub connector: String,
    /// The flow in which the code and message occurred for a connector
    pub flow: String,
    /// The sub_flow in which the code and message occurred  for a connector
    pub sub_flow: String,
    /// code received from the connector
    pub code: String,
    /// message received from the connector
    pub message: String,
}

#[derive(Debug, serde::Serialize, ToSchema)]
pub struct GsmDeleteResponse {
    pub gsm_rule_delete: bool,
    /// The connector through which payment has gone through
    pub connector: String,
    /// The flow in which the code and message occurred for a connector
    pub flow: String,
    /// The sub_flow in which the code and message occurred  for a connector
    pub sub_flow: String,
    /// code received from the connector
    pub code: String,
}

#[derive(serde::Serialize, Debug, ToSchema)]
pub struct GsmResponse {
    /// The connector through which payment has gone through
    pub connector: String,
    /// The flow in which the code and message occurred for a connector
    pub flow: String,
    /// The sub_flow in which the code and message occurred  for a connector
    pub sub_flow: String,
    /// code received from the connector
    pub code: String,
    /// message received from the connector
    pub message: String,
    /// status provided by the router
    pub status: String,
    /// optional error provided by the router
    pub router_error: Option<String>,
    /// decision to be taken for auto retries flow
    /// **Deprecated**: This field is now included as part of `feature_data` under the `Retry` variant.
    #[schema(deprecated)]
    #[schema(value_type = GsmDecision)]
    pub decision: api_enums::GsmDecision,
    /// indicates if step_up retry is possible
    /// **Deprecated**: This field is now included as part of `feature_data` under the `Retry` variant.
    #[schema(deprecated)]
    pub step_up_possible: bool,
    /// error code unified across the connectors
    pub unified_code: Option<String>,
    /// error message unified across the connectors
    pub unified_message: Option<String>,
    /// category in which error belongs to
    #[schema(value_type = Option<ErrorCategory>)]
    pub error_category: Option<api_enums::ErrorCategory>,
    /// indicates if retry with pan is possible
    /// **Deprecated**: This field is now included as part of `feature_data` under the `Retry` variant.
    #[schema(deprecated)]
    pub clear_pan_possible: bool,
    /// Indicates the GSM feature associated with the request,
    /// such as retry mechanisms or other specific functionalities provided by the system.
    #[schema(value_type = GsmFeature)]
    pub feature: api_enums::GsmFeature,
    /// Contains the data relevant to the specified GSM feature, if applicable.
    /// For example, if the `feature` is `Retry`, this will include configuration
    /// details specific to the retry behavior.
    #[schema(value_type = GsmFeatureData)]
    pub feature_data: Option<common_types::domain::GsmFeatureData>,
}
