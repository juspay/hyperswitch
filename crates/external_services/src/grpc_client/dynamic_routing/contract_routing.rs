use api_models::routing::{
    ContractBasedRoutingConfig, ContractBasedRoutingConfigBody, ContractBasedTimeScale,
    LabelInformation, RoutableConnectorChoice, RoutableConnectorChoiceWithStatus,
};
use common_utils::{
    ext_traits::OptionExt,
    transformers::{ForeignFrom, ForeignTryFrom},
};
pub use contract_routing::{
    contract_score_calculator_client::ContractScoreCalculatorClient, CalContractScoreConfig,
    CalContractScoreRequest, CalContractScoreResponse, InvalidateContractRequest,
    InvalidateContractResponse, LabelInformation as ProtoLabelInfo, TimeScale,
    UpdateContractRequest, UpdateContractResponse,
};
use error_stack::ResultExt;
#[allow(
    missing_docs,
    unused_qualifications,
    clippy::unwrap_used,
    clippy::as_conversions,
    clippy::use_self
)]
pub mod contract_routing {
    tonic::include_proto!("contract_routing");
}
use super::{Client, DynamicRoutingError, DynamicRoutingResult};
/// The trait Success Based Dynamic Routing would have the functions required to support the calculation and updation window
#[async_trait::async_trait]
pub trait ContractBasedDynamicRouting: dyn_clone::DynClone + Send + Sync {
    /// To calculate the contract scores for the list of chosen connectors
    async fn calculate_contract_score(
        &self,
        id: String,
        config: ContractBasedRoutingConfig,
        params: String,
        label_input: Vec<RoutableConnectorChoice>,
    ) -> DynamicRoutingResult<CalContractScoreResponse>;
    /// To update the contract scores with the given labels
    async fn update_contracts(
        &self,
        id: String,
        label_info: Vec<LabelInformation>,
        params: String,
        response: Vec<RoutableConnectorChoiceWithStatus>,
    ) -> DynamicRoutingResult<UpdateContractResponse>;
    /// To invalidates the contract scores against the id
    async fn invalidate_contracts(
        &self,
        id: String,
    ) -> DynamicRoutingResult<InvalidateContractResponse>;
}

#[async_trait::async_trait]
impl ContractBasedDynamicRouting for ContractScoreCalculatorClient<Client> {
    async fn calculate_contract_score(
        &self,
        id: String,
        config: ContractBasedRoutingConfig,
        params: String,
        label_input: Vec<RoutableConnectorChoice>,
    ) -> DynamicRoutingResult<CalContractScoreResponse> {
        let labels = label_input
            .into_iter()
            .map(|conn_choice| conn_choice.to_string())
            .collect::<Vec<_>>();

        let config = config
            .config
            .map(ForeignTryFrom::foreign_try_from)
            .transpose()?;

        let request = tonic::Request::new(CalContractScoreRequest {
            id,
            params,
            labels,
            config,
        });

        let response = self
            .clone()
            .fetch_contract_score(request)
            .await
            .change_context(DynamicRoutingError::ContractBasedRoutingFailure(
                "Failed to fetch the contract score".to_string(),
            ))?
            .into_inner();

        Ok(response)
    }

    async fn update_contracts(
        &self,
        id: String,
        label_info: Vec<LabelInformation>,
        params: String,
        _response: Vec<RoutableConnectorChoiceWithStatus>,
    ) -> DynamicRoutingResult<UpdateContractResponse> {
        let labels_information = label_info
            .into_iter()
            .map(ProtoLabelInfo::foreign_from)
            .collect::<Vec<_>>();

        let request = tonic::Request::new(UpdateContractRequest {
            id,
            params,
            labels_information,
        });

        let response = self
            .clone()
            .update_contract(request)
            .await
            .change_context(DynamicRoutingError::ContractBasedRoutingFailure(
                "Failed to update the contracts".to_string(),
            ))?
            .into_inner();

        Ok(response)
    }
    async fn invalidate_contracts(
        &self,
        id: String,
    ) -> DynamicRoutingResult<InvalidateContractResponse> {
        let request = tonic::Request::new(InvalidateContractRequest { id });

        let response = self
            .clone()
            .invalidate_contract(request)
            .await
            .change_context(DynamicRoutingError::ContractBasedRoutingFailure(
                "Failed to invalidate the contracts".to_string(),
            ))?
            .into_inner();
        Ok(response)
    }
}

impl ForeignFrom<ContractBasedTimeScale> for TimeScale {
    fn foreign_from(scale: ContractBasedTimeScale) -> Self {
        Self {
            time_scale: match scale {
                ContractBasedTimeScale::Day => 0,
                _ => 1,
            },
        }
    }
}

impl ForeignTryFrom<ContractBasedRoutingConfigBody> for CalContractScoreConfig {
    type Error = error_stack::Report<DynamicRoutingError>;
    fn foreign_try_from(config: ContractBasedRoutingConfigBody) -> Result<Self, Self::Error> {
        Ok(Self {
            constants: config
                .constants
                .get_required_value("constants")
                .change_context(DynamicRoutingError::MissingRequiredField {
                    field: "constants".to_string(),
                })?,
            time_scale: config.time_scale.clone().map(TimeScale::foreign_from),
        })
    }
}

impl ForeignFrom<LabelInformation> for ProtoLabelInfo {
    fn foreign_from(config: LabelInformation) -> Self {
        Self {
            label: config.label,
            target_count: config.target_count,
            target_time: config.target_time,
            current_count: 1,
        }
    }
}
