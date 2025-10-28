//! Helper functions for Payment Gateway implementations
//!
//! This module contains shared utility functions used across different
//! payment gateway implementations.

use std::str::FromStr;
use common_enums::ExecutionMode;
use common_utils::{errors::CustomResult, id_type, ucs_types};
use error_stack::ResultExt;
use external_services::grpc_client::{self, unified_connector_service::{ConnectorAuthMetadata, UnifiedConnectorServiceClient}, GrpcHeadersUcsBuilder, LineageIds};
use hyperswitch_domain_models::{merchant_context::MerchantContext, payments::HeaderPayload};
use hyperswitch_interfaces::errors::ConnectorError;

use crate::{core::payments::helpers, routes::SessionState};

// use super::ucs_context::RouterUcsContext;

// /// Build GRPC auth metadata from merchant connector account
// pub fn build_grpc_auth_metadata(
//     #[cfg(feature = "v1")] merchant_connector_account: &helpers::MerchantConnectorAccountType,
//     #[cfg(feature = "v2")]
//     merchant_connector_account: &hyperswitch_domain_models::merchant_connector_account::MerchantConnectorAccountTypeDetails,
//     merchant_context: &MerchantContext,
// ) -> CustomResult<ConnectorAuthMetadata, ConnectorError> {
//     crate::core::unified_connector_service::build_unified_connector_service_auth_metadata(
//         merchant_connector_account,
//         merchant_context,
//     )
//     .change_context(ConnectorError::InvalidConnectorConfig {
//         config: "auth_metadata".into(),
//     })
// }

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

/// Prepare UCS infrastructure (client, auth metadata, headers)
///
/// This function consolidates the common setup steps (Steps 1-3) required by all UCS flows:
/// 1. Get GRPC client from state
/// 2. Build connector authentication metadata
/// 3. Build GRPC headers with merchant reference ID and lineage IDs
///
/// # Arguments
/// - `state`: Session state containing GRPC client
/// - `merchant_context`: Merchant context for authentication
/// - `header_payload`: Header payload for reference ID
/// - `lineage_ids`: Lineage IDs for tracing
/// - `merchant_connector_account`: Merchant connector account details
/// - `execution_mode`: Execution mode (Live/Test)
///
/// # Returns
/// Tuple of (GRPC client, auth metadata, headers builder)
pub fn prepare_ucs_infrastructure(
    state: &SessionState,
    merchant_context: &MerchantContext,
    header_payload: &HeaderPayload,
    lineage_ids: LineageIds,
    #[cfg(feature = "v1")] merchant_connector_account: &helpers::MerchantConnectorAccountType,
    #[cfg(feature = "v2")]
    merchant_connector_account: &hyperswitch_domain_models::merchant_connector_account::MerchantConnectorAccountTypeDetails,
    execution_mode: ExecutionMode,
) -> CustomResult<
    (
        UnifiedConnectorServiceClient,
        ConnectorAuthMetadata,
        external_services::grpc_client::GrpcHeadersUcsBuilderFinal,
    ),
    ConnectorError,
> {
    // Step 1: Get GRPC client
    let client = get_grpc_client(state)?;

    // Step 2: Build auth metadata
    let connector_auth_metadata = crate::core::unified_connector_service::build_unified_connector_service_auth_metadata(
        merchant_connector_account,
        merchant_context,
    ).change_context(ConnectorError::InvalidConnectorConfig {
        config: "auth_metadata".into(),
    })?;

    // Step 3: Build GRPC headers
    let merchant_order_reference_id = build_merchant_reference_id(header_payload);

    let headers_builder = state
        .get_grpc_headers_ucs(execution_mode)
        .external_vault_proxy_metadata(None)
        .merchant_reference_id(merchant_order_reference_id)
        .lineage_ids(lineage_ids);

    Ok((client, connector_auth_metadata, headers_builder))
}

// / Create UCS context from infrastructure components
// /
// / This function creates a RouterUcsContext from the prepared infrastructure.
// /
// / # Arguments
// / - `auth`: Connector authentication metadata
// / - `headers`: GRPC headers
// / - `lineage_ids`: Lineage IDs for tracing
// /
// / # Returns
// / RouterUcsContext containing all context information
// pub fn create_ucs_context(
//     auth: ConnectorAuthMetadata,
//     headers: grpc_client::GrpcHeadersUcs,
//     lineage_ids: LineageIds,
// ) -> RouterUcsContext {
//     RouterUcsContext::new(auth, headers, lineage_ids)
// }