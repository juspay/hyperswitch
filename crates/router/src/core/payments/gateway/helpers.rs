//! Helper functions for Payment Gateway implementations
//!
//! This module contains shared utility functions used across different
//! payment gateway implementations.

use std::str::FromStr;
use common_utils::{errors::CustomResult, id_type, ucs_types};
use error_stack::ResultExt;
use external_services::grpc_client::{self, unified_connector_service::{ConnectorAuthMetadata, UnifiedConnectorServiceClient}};
use hyperswitch_domain_models::{merchant_context::MerchantContext, payments::HeaderPayload};
use hyperswitch_interfaces::errors::ConnectorError;

use crate::{core::payments::helpers, routes::SessionState};

/// Build GRPC auth metadata from merchant connector account
pub fn build_grpc_auth_metadata(
    #[cfg(feature = "v1")] merchant_connector_account: &helpers::MerchantConnectorAccountType,
    #[cfg(feature = "v2")]
    merchant_connector_account: &hyperswitch_domain_models::merchant_connector_account::MerchantConnectorAccountTypeDetails,
    merchant_context: &MerchantContext,
) -> CustomResult<ConnectorAuthMetadata, ConnectorError> {
    crate::core::unified_connector_service::build_unified_connector_service_auth_metadata(
        merchant_connector_account,
        merchant_context,
    )
    .change_context(ConnectorError::InvalidConnectorConfig {
        config: "auth_metadata".into(),
    })
}

/// Build merchant reference ID from header payload
pub fn build_merchant_reference_id(
    header_payload: &HeaderPayload,
) -> Option<ucs_types::UcsReferenceId> {
    header_payload
        .x_reference_id
        .clone()
        .and_then(|id| {
            id_type::PaymentReferenceId::from_str(id.as_str())
                .ok()
                .map(ucs_types::UcsReferenceId::Payment)
        })
}

/// Get GRPC client from state
pub fn get_grpc_client(
    state: &SessionState,
) -> CustomResult<UnifiedConnectorServiceClient, ConnectorError> {
    state
        .grpc_client
        .unified_connector_service_client
        .clone()
        .ok_or(ConnectorError::ProcessingStepFailed(Some(
            "Failed to fetch Unified Connector Service client".to_string().into(),
        )))
        .map_err(Into::into)
}