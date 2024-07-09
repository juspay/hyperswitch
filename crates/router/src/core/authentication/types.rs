use error_stack::{Report, ResultExt};
pub use hyperswitch_domain_models::router_request_types::authentication::{
    AcquirerDetails, ExternalThreeDSConnectorMetadata, PreAuthenticationData, ThreeDsMethodData,
};

use crate::{
    core::errors,
    types::{storage, transformers::ForeignTryFrom},
    utils::OptionExt,
};

impl ForeignTryFrom<&storage::Authentication> for PreAuthenticationData {
    type Error = Report<errors::ApiErrorResponse>;

    fn foreign_try_from(authentication: &storage::Authentication) -> Result<Self, Self::Error> {
        let error_message = errors::ApiErrorResponse::UnprocessableEntity { message: "Pre Authentication must be completed successfully before Authentication can be performed".to_string() };
        let threeds_server_transaction_id = authentication
            .threeds_server_transaction_id
            .clone()
            .get_required_value("threeds_server_transaction_id")
            .change_context(error_message)?;
        let message_version = authentication
            .message_version
            .clone()
            .get_required_value("message_version")?;
        Ok(Self {
            threeds_server_transaction_id,
            message_version,
            acquirer_bin: authentication.acquirer_bin.clone(),
            acquirer_merchant_id: authentication.acquirer_merchant_id.clone(),
            acquirer_country_code: authentication.acquirer_country_code.clone(),
            connector_metadata: authentication.connector_metadata.clone(),
        })
    }
}
