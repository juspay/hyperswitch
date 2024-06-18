#[cfg(feature = "frm")]
use hyperswitch_domain_models::router_data_v2::flow_common_types::FrmFlowData;
#[cfg(feature = "payouts")]
use hyperswitch_domain_models::router_data_v2::flow_common_types::PayoutFlowData;
use hyperswitch_domain_models::{
    router_data::RouterData,
    router_data_v2::{
        flow_common_types::{
            AccessTokenFlowData, DisputesFlowData, ExternalAuthenticationFlowData, FilesFlowData,
            MandateRevokeFlowData, PaymentFlowData, RefundFlowData, WebhookSourceVerifyData,
        },
        RouterDataV2,
    },
};

use super::connector_integration_interface::Conversion;
use crate::errors;

impl<T, Req:Clone, Resp:Clone> Conversion<T, Req, Resp> for AccessTokenFlowData {
    fn from_old_router_data(
        old_router_data: &RouterData<T, Req, Resp>,
    ) -> errors::CustomResult<RouterDataV2<T, Self, Req, Resp>, errors::ConnectorError>
    where
        Self: Sized,
    {
        let resource_common_data = Self {};
        Ok(RouterDataV2{
            flow: std::marker::PhantomData,
            resource_common_data,
            connector_auth_type: old_router_data.connector_auth_type.clone(),
            request: old_router_data.request.clone(),
            response: old_router_data.response.clone(),
        })
    }

    fn to_old_router_data(
        _new_router_data: RouterDataV2<T, Self, Req, Resp>,
    ) -> errors::CustomResult<RouterData<T, Req, Resp>, errors::ConnectorError>
    where
        Self: Sized,
    {
        todo!()
    }
}

impl<T, Req:Clone, Resp:Clone> Conversion<T, Req, Resp> for PaymentFlowData {
    fn from_old_router_data(
        _old_router_data: &RouterData<T, Req, Resp>,
    ) -> errors::CustomResult<RouterDataV2<T, Self, Req, Resp>, errors::ConnectorError>
    where
        Self: Sized,
    {
        todo!()
    }

    fn to_old_router_data(
        _new_router_data: RouterDataV2<T, Self, Req, Resp>,
    ) -> errors::CustomResult<RouterData<T, Req, Resp>, errors::ConnectorError>
    where
        Self: Sized,
    {
        todo!()
    }
}

impl<T, Req:Clone, Resp:Clone> Conversion<T, Req, Resp> for RefundFlowData {
    fn from_old_router_data(
        _old_router_data: &RouterData<T, Req, Resp>,
    ) -> errors::CustomResult<RouterDataV2<T, Self, Req, Resp>, errors::ConnectorError>
    where
        Self: Sized,
    {
        todo!()
    }

    fn to_old_router_data(
        _new_router_data: RouterDataV2<T, Self, Req, Resp>,
    ) -> errors::CustomResult<RouterData<T, Req, Resp>, errors::ConnectorError>
    where
        Self: Sized,
    {
        todo!()
    }
}

impl<T, Req:Clone, Resp:Clone> Conversion<T, Req, Resp> for DisputesFlowData {
    fn from_old_router_data(
        _old_router_data: &RouterData<T, Req, Resp>,
    ) -> errors::CustomResult<RouterDataV2<T, Self, Req, Resp>, errors::ConnectorError>
    where
        Self: Sized,
    {
        todo!()
    }

    fn to_old_router_data(
        _new_router_data: RouterDataV2<T, Self, Req, Resp>,
    ) -> errors::CustomResult<RouterData<T, Req, Resp>, errors::ConnectorError>
    where
        Self: Sized,
    {
        todo!()
    }
}

#[cfg(feature = "frm")]
impl<T, Req:Clone, Resp:Clone> Conversion<T, Req, Resp> for FrmFlowData {
    fn from_old_router_data(
        _old_router_data: &RouterData<T, Req, Resp>,
    ) -> errors::CustomResult<RouterDataV2<T, Self, Req, Resp>, errors::ConnectorError>
    where
        Self: Sized,
    {
        todo!()
    }

    fn to_old_router_data(
        _new_router_data: RouterDataV2<T, Self, Req, Resp>,
    ) -> errors::CustomResult<RouterData<T, Req, Resp>, errors::ConnectorError>
    where
        Self: Sized,
    {
        todo!()
    }
}

impl<T, Req:Clone, Resp:Clone> Conversion<T, Req, Resp> for FilesFlowData {
    fn from_old_router_data(
        _old_router_data: &RouterData<T, Req, Resp>,
    ) -> errors::CustomResult<RouterDataV2<T, Self, Req, Resp>, errors::ConnectorError>
    where
        Self: Sized,
    {
        todo!()
    }

    fn to_old_router_data(
        _new_router_data: RouterDataV2<T, Self, Req, Resp>,
    ) -> errors::CustomResult<RouterData<T, Req, Resp>, errors::ConnectorError>
    where
        Self: Sized,
    {
        todo!()
    }
}

impl<T, Req:Clone, Resp:Clone> Conversion<T, Req, Resp> for WebhookSourceVerifyData {
    fn from_old_router_data(
        _old_router_data: &RouterData<T, Req, Resp>,
    ) -> errors::CustomResult<RouterDataV2<T, Self, Req, Resp>, errors::ConnectorError>
    where
        Self: Sized,
    {
        todo!()
    }

    fn to_old_router_data(
        _new_router_data: RouterDataV2<T, Self, Req, Resp>,
    ) -> errors::CustomResult<RouterData<T, Req, Resp>, errors::ConnectorError>
    where
        Self: Sized,
    {
        todo!()
    }
}

impl<T, Req:Clone, Resp:Clone> Conversion<T, Req, Resp> for MandateRevokeFlowData {
    fn from_old_router_data(
        _old_router_data: &RouterData<T, Req, Resp>,
    ) -> errors::CustomResult<RouterDataV2<T, Self, Req, Resp>, errors::ConnectorError>
    where
        Self: Sized,
    {
        todo!()
    }

    fn to_old_router_data(
        _new_router_data: RouterDataV2<T, Self, Req, Resp>,
    ) -> errors::CustomResult<RouterData<T, Req, Resp>, errors::ConnectorError>
    where
        Self: Sized,
    {
        todo!()
    }
}

#[cfg(feature = "payouts")]
impl<T, Req:Clone, Resp:Clone> Conversion<T, Req, Resp> for PayoutFlowData {
    fn from_old_router_data(
        _old_router_data: &RouterData<T, Req, Resp>,
    ) -> errors::CustomResult<RouterDataV2<T, Self, Req, Resp>, errors::ConnectorError>
    where
        Self: Sized,
    {
        todo!()
    }

    fn to_old_router_data(
        _new_router_data: RouterDataV2<T, Self, Req, Resp>,
    ) -> errors::CustomResult<RouterData<T, Req, Resp>, errors::ConnectorError>
    where
        Self: Sized,
    {
        todo!()
    }
}
impl<T, Req:Clone, Resp:Clone> Conversion<T, Req, Resp> for ExternalAuthenticationFlowData {
    fn from_old_router_data(
        _old_router_data: &RouterData<T, Req, Resp>,
    ) -> errors::CustomResult<RouterDataV2<T, Self, Req, Resp>, errors::ConnectorError>
    where
        Self: Sized,
    {
        todo!()
    }

    fn to_old_router_data(
        _new_router_data: RouterDataV2<T, Self, Req, Resp>,
    ) -> errors::CustomResult<RouterData<T, Req, Resp>, errors::ConnectorError>
    where
        Self: Sized,
    {
        todo!()
    }
}
