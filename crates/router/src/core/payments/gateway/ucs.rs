//! Unified Connector Service Gateway Implementation
//!
//! NOTE: This gateway is currently not fully implemented due to architectural constraints.
//! UCS requires MerchantContext and PaymentData which are not available in the simplified
//! PaymentGateway trait interface. UCS calls should continue to be made directly in the
//! payment flow until the gateway trait is extended to support these requirements.

use std::marker::PhantomData;

use async_trait::async_trait;
use common_enums::CallConnectorAction;
use common_utils::errors::CustomResult;
use hyperswitch_domain_models::{
    router_data::RouterData,
    router_request_types::{PaymentsAuthorizeData, PaymentsSyncData, SetupMandateRequestData},
    router_response_types::PaymentsResponseData,
};
use hyperswitch_interfaces::api::gateway as gateway_interface;

use crate::{routes::SessionState, types::api};
use hyperswitch_interfaces::configs::MerchantConnectorAccountType;

/// Gateway that executes through Unified Connector Service
///
/// NOTE: This is currently a stub implementation. UCS integration requires
/// additional context (MerchantContext, PaymentData) that is not available
/// through the PaymentGateway trait interface.
pub struct UnifiedConnectorServiceGateway<F> {
    flow_type: PhantomData<F>,
}

impl<F> UnifiedConnectorServiceGateway<F> {
    /// Create a new UCS gateway for the given flow type
    pub fn new() -> Self {
        Self {
            flow_type: PhantomData,
        }
    }
}

impl<F> Default for UnifiedConnectorServiceGateway<F> {
    fn default() -> Self {
        Self::new()
    }
}

// Implementation for Authorize flow
#[async_trait]
impl
    gateway_interface::PaymentGateway<
        SessionState,
        api::ConnectorData,
        MerchantConnectorAccountType,
        api::Authorize,
        PaymentsAuthorizeData,
        PaymentsResponseData,
    > for UnifiedConnectorServiceGateway<api::Authorize>
{
    async fn execute(
        self,
        _state: &SessionState,
        _router_data: RouterData<api::Authorize, PaymentsAuthorizeData, PaymentsResponseData>,
        _connector: &api::ConnectorData,
        _merchant_connector_account: &MerchantConnectorAccountType,
        _call_connector_action: CallConnectorAction,
    ) -> CustomResult<
        RouterData<api::Authorize, PaymentsAuthorizeData, PaymentsResponseData>,
        hyperswitch_interfaces::errors::ConnectorError,
    > {
        // UCS gateway is not yet fully implemented
        // Return an error indicating this should not be used
        todo!()
    }
}

// Implementation for PSync flow
#[async_trait]
impl
    gateway_interface::PaymentGateway<
        SessionState,
        api::ConnectorData,
        MerchantConnectorAccountType,
        api::PSync,
        PaymentsSyncData,
        PaymentsResponseData,
    > for UnifiedConnectorServiceGateway<api::PSync>
{
    async fn execute(
        self,
        _state: &SessionState,
        _router_data: RouterData<api::PSync, PaymentsSyncData, PaymentsResponseData>,
        _connector: &api::ConnectorData,
        _merchant_connector_account: &MerchantConnectorAccountType,
        _call_connector_action: CallConnectorAction,
    ) -> CustomResult<
        RouterData<api::PSync, PaymentsSyncData, PaymentsResponseData>,
        hyperswitch_interfaces::errors::ConnectorError,
    > {
        todo!()
    }
}

// Implementation for SetupMandate flow
#[async_trait]
impl
    gateway_interface::PaymentGateway<
        SessionState,
        api::ConnectorData,
        MerchantConnectorAccountType,
        api::SetupMandate,
        SetupMandateRequestData,
        PaymentsResponseData,
    > for UnifiedConnectorServiceGateway<api::SetupMandate>
{
    async fn execute(
        self,
        _state: &SessionState,
        _router_data: RouterData<api::SetupMandate, SetupMandateRequestData, PaymentsResponseData>,
        _connector: &api::ConnectorData,
        _merchant_connector_account: &MerchantConnectorAccountType,
        _call_connector_action: CallConnectorAction,
    ) -> CustomResult<
        RouterData<api::SetupMandate, SetupMandateRequestData, PaymentsResponseData>,
        hyperswitch_interfaces::errors::ConnectorError,
    > {
        todo!()
    }
}