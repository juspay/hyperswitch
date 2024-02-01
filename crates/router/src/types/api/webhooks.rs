use api_models::admin::MerchantConnectorWebhookDetails;
pub use api_models::webhooks::{
    IncomingWebhookDetails, IncomingWebhookEvent, MerchantWebhookConfig, ObjectReferenceId,
    OutgoingWebhook, OutgoingWebhookContent, WebhookFlow,
};
use common_utils::ext_traits::ValueExt;
use error_stack::ResultExt;
use masking::ExposeInterface;

use super::ConnectorCommon;
use crate::{
    core::{
        errors::{self, CustomResult},
        payments,
        webhooks::utils::construct_webhook_router_data,
    },
    db::StorageInterface,
    services::{self},
    types::{self, domain},
    utils::crypto,
};

pub struct IncomingWebhookRequestDetails<'a> {
    pub method: actix_web::http::Method,
    pub uri: actix_web::http::Uri,
    pub headers: &'a actix_web::http::header::HeaderMap,
    pub body: &'a [u8],
    pub query_params: String,
}

#[async_trait::async_trait]
pub trait IncomingWebhook: ConnectorCommon + Sync {
        /// Retrieves the decoding algorithm to use for decoding the body of an incoming webhook request.
    ///
    /// # Arguments
    ///
    /// * `request` - The details of the incoming webhook request.
    ///
    /// # Returns
    ///
    /// A `CustomResult` containing a `Box` of a type that implements the `crypto::DecodeMessage` trait and can be sent across threads, or an error of type `errors::ConnectorError`.
    ///
    fn get_webhook_body_decoding_algorithm(
        &self,
        _request: &IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<Box<dyn crypto::DecodeMessage + Send>, errors::ConnectorError> {
        Ok(Box::new(crypto::NoAlgorithm))
    }

        /// Asynchronously retrieves the webhook body and decodes the merchant secret for a given merchant ID.
    ///
    /// # Arguments
    /// * `&self` - The reference to the current instance of the struct.
    /// * `_db` - A reference to the storage interface for database operations.
    /// * `_merchant_id` - The ID of the merchant whose webhook body and secret are being retrieved.
    ///
    /// # Returns
    /// A `CustomResult` containing the decoded merchant secret as a vector of bytes, or an error of type `ConnectorError`.
    ///
    async fn get_webhook_body_decoding_merchant_secret(
        &self,
        _db: &dyn StorageInterface,
        _merchant_id: &str,
    ) -> CustomResult<Vec<u8>, errors::ConnectorError> {
        Ok(Vec::new())
    }

        /// This method takes an IncomingWebhookRequestDetails reference as input and returns a CustomResult containing the body of the request, decoded as a vector of unsigned bytes. If successful, it returns Ok with the body of the request as a vector of bytes. If there is an error, it returns Err with a ConnectorError.
    fn get_webhook_body_decoding_message(
        &self,
        request: &IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<Vec<u8>, errors::ConnectorError> {
        Ok(request.body.to_vec())
    }

        /// Decodes the body of a webhook request using the specified algorithm and merchant ID,
    /// returning the decoded message as a vector of bytes.
    async fn decode_webhook_body(
        &self,
        db: &dyn StorageInterface,
        request: &IncomingWebhookRequestDetails<'_>,
        merchant_id: &str,
    ) -> CustomResult<Vec<u8>, errors::ConnectorError> {
        let algorithm = self.get_webhook_body_decoding_algorithm(request)?;

        let message = self
            .get_webhook_body_decoding_message(request)
            .change_context(errors::ConnectorError::WebhookBodyDecodingFailed)?;
        let secret = self
            .get_webhook_body_decoding_merchant_secret(db, merchant_id)
            .await
            .change_context(errors::ConnectorError::WebhookBodyDecodingFailed)?;

        algorithm
            .decode_message(&secret, message.into())
            .change_context(errors::ConnectorError::WebhookBodyDecodingFailed)
    }

        /// Retrieves the verification algorithm for the source of a webhook request.
    ///
    /// # Arguments
    ///
    /// * `request` - The details of the incoming webhook request.
    ///
    /// # Returns
    ///
    /// A `CustomResult` containing a `Box` that implements the `VerifySignature` trait and can be sent across threads, or an error of type `ConnectorError`.
    ///
    fn get_webhook_source_verification_algorithm(
        &self,
        _request: &IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<Box<dyn crypto::VerifySignature + Send>, errors::ConnectorError> {
        Ok(Box::new(crypto::NoAlgorithm))
    }

    async fn get_webhook_source_verification_merchant_secret(
        &self,
        merchant_account: &domain::MerchantAccount,
        connector_name: &str,
        merchant_connector_account: domain::MerchantConnectorAccount,
    ) -> CustomResult<api_models::webhooks::ConnectorWebhookSecrets, errors::ConnectorError> {
        let merchant_id = merchant_account.merchant_id.as_str();
        let debug_suffix = format!(
            "For merchant_id: {}, and connector_name: {}",
            merchant_id, connector_name
        );
        let default_secret = "default_secret".to_string();
        let merchant_secret = match merchant_connector_account.connector_webhook_details {
            Some(merchant_connector_webhook_details) => {
                let connector_webhook_details = merchant_connector_webhook_details
                    .parse_value::<MerchantConnectorWebhookDetails>(
                        "MerchantConnectorWebhookDetails",
                    )
                    .change_context_lazy(|| errors::ConnectorError::WebhookSourceVerificationFailed)
                    .attach_printable_lazy(|| {
                        format!(
                            "Deserializing MerchantConnectorWebhookDetails failed {}",
                            debug_suffix
                        )
                    })?;
                api_models::webhooks::ConnectorWebhookSecrets {
                    secret: connector_webhook_details
                        .merchant_secret
                        .expose()
                        .into_bytes(),
                    additional_secret: connector_webhook_details.additional_secret,
                }
            }

            None => api_models::webhooks::ConnectorWebhookSecrets {
                secret: default_secret.into_bytes(),
                additional_secret: None,
            },
        };

        //need to fetch merchant secret from config table with caching in future for enhanced performance

        //If merchant has not set the secret for webhook source verification, "default_secret" is returned.
        //So it will fail during verification step and goes to psync flow.
        Ok(merchant_secret)
    }

        /// Asynchronously retrieves the webhook source verification merchant secret for a given merchant account and connector name.
    ///
    /// # Arguments
    ///
    /// * `merchant_account` - A reference to the merchant account for which the secret is being retrieved.
    /// * `connector_name` - The name of the connector for which the secret is being retrieved.
    /// * `merchant_connector_account` - The merchant connector account containing the webhook details.
    ///
    /// # Returns
    ///
    /// A `CustomResult` containing the `ConnectorWebhookSecrets` if successful, otherwise a `ConnectorError`.
    ///
    /// # Errors
    ///
    /// This method can fail if the webhook source verification fails or if the merchant connector webhook details cannot be deserialized.
    ///
        /// This method is used to retrieve the verification signature for the incoming webhook request. It takes the incoming webhook request details and the connector webhook secrets as input parameters and returns a result containing a vector of bytes representing the verification signature. If successful, it returns an empty vector.
    fn get_webhook_source_verification_signature(
        &self,
        _request: &IncomingWebhookRequestDetails<'_>,
        _connector_webhook_secrets: &api_models::webhooks::ConnectorWebhookSecrets,
    ) -> CustomResult<Vec<u8>, errors::ConnectorError> {
        Ok(Vec::new())
    }

        /// Retrieves the verification message for the webhook source based on the incoming webhook request details, merchant ID, and connector webhook secrets.
    fn get_webhook_source_verification_message(
        &self,
        _request: &IncomingWebhookRequestDetails<'_>,
        _merchant_id: &str,
        _connector_webhook_secrets: &api_models::webhooks::ConnectorWebhookSecrets,
    ) -> CustomResult<Vec<u8>, errors::ConnectorError> {
        Ok(Vec::new())
    }

        /// Asynchronously verifies the source of a webhook call by fetching the connector data, retrieving the webhook secrets, constructing the webhook router data, and executing the connector processing step. Returns a boolean indicating whether the source is verified or not.
    async fn verify_webhook_source_verification_call(
        &self,
        state: &crate::routes::AppState,
        merchant_account: &domain::MerchantAccount,
        merchant_connector_account: domain::MerchantConnectorAccount,
        connector_name: &str,
        request_details: &IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<bool, errors::ConnectorError> {
        let connector_data = types::api::ConnectorData::get_connector_by_name(
            &state.conf.connectors,
            connector_name,
            types::api::GetToken::Connector,
            None,
        )
        .change_context(errors::ConnectorError::WebhookSourceVerificationFailed)
        .attach_printable("invalid connector name received in payment attempt")?;
        let connector_integration: services::BoxedConnectorIntegration<
            '_,
            types::api::VerifyWebhookSource,
            types::VerifyWebhookSourceRequestData,
            types::VerifyWebhookSourceResponseData,
        > = connector_data.connector.get_connector_integration();
        let connector_webhook_secrets = self
            .get_webhook_source_verification_merchant_secret(
                merchant_account,
                connector_name,
                merchant_connector_account.clone(),
            )
            .await
            .change_context(errors::ConnectorError::WebhookSourceVerificationFailed)?;

        let router_data = construct_webhook_router_data(
            connector_name,
            merchant_connector_account,
            merchant_account,
            &connector_webhook_secrets,
            request_details,
        )
        .await
        .change_context(errors::ConnectorError::WebhookSourceVerificationFailed)
        .attach_printable("Failed while constructing webhook router data")?;

        let response = services::execute_connector_processing_step(
            state,
            connector_integration,
            &router_data,
            payments::CallConnectorAction::Trigger,
            None,
        )
        .await?;

        let verification_result = response
            .response
            .map(|response| response.verify_webhook_status);
        match verification_result {
            Ok(types::VerifyWebhookStatus::SourceVerified) => Ok(true),
            _ => Ok(false),
        }
    }

    async fn verify_webhook_source(
        &self,
        request: &IncomingWebhookRequestDetails<'_>,
        merchant_account: &domain::MerchantAccount,
        merchant_connector_account: domain::MerchantConnectorAccount,
        connector_name: &str,
    ) -> CustomResult<bool, errors::ConnectorError> {
        let algorithm = self
            .get_webhook_source_verification_algorithm(request)
            .change_context(errors::ConnectorError::WebhookSourceVerificationFailed)?;

        let connector_webhook_secrets = self
            .get_webhook_source_verification_merchant_secret(
                merchant_account,
                connector_name,
                merchant_connector_account,
            )
            .await
            .change_context(errors::ConnectorError::WebhookSourceVerificationFailed)?;

        let signature = self
            .get_webhook_source_verification_signature(request, &connector_webhook_secrets)
            .change_context(errors::ConnectorError::WebhookSourceVerificationFailed)?;

        let message = self
            .get_webhook_source_verification_message(
                request,
                &merchant_account.merchant_id,
                &connector_webhook_secrets,
            )
            .change_context(errors::ConnectorError::WebhookSourceVerificationFailed)?;

        algorithm
            .verify_signature(&connector_webhook_secrets.secret, &signature, &message)
            .change_context(errors::ConnectorError::WebhookSourceVerificationFailed)
    }

    fn get_webhook_object_reference_id(
        &self,
        _request: &IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<ObjectReferenceId, errors::ConnectorError>;

    fn get_webhook_event_type(
        &self,
        _request: &IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<IncomingWebhookEvent, errors::ConnectorError>;

    fn get_webhook_resource_object(
        &self,
        _request: &IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<Box<dyn masking::ErasedMaskSerialize>, errors::ConnectorError>;

        /// Retrieves the API response for a webhook request and returns it as a Result.
    fn get_webhook_api_response(
        &self,
        _request: &IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<services::api::ApplicationResponse<serde_json::Value>, errors::ConnectorError>
    {
        Ok(services::api::ApplicationResponse::StatusOk)
    }

        /// This method retrieves the details of a dispute from the incoming webhook request.
    fn get_dispute_details(
        &self,
        _request: &IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<super::disputes::DisputePayload, errors::ConnectorError> {
        Err(errors::ConnectorError::NotImplemented("get_dispute_details method".to_string()).into())
    }
}
