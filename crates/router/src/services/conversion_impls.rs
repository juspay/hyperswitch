#[cfg(feature = "frm")]
use hyperswitch_domain_models::router_data_new::flow_common_types::FrmFlowData;
#[cfg(feature = "payouts")]
use hyperswitch_domain_models::router_data_new::flow_common_types::PayoutFlowData;
use hyperswitch_domain_models::{
    router_data::RouterData,
    router_data_new::{
        flow_common_types::{
            AccessTokenFlowData, DisputesFlowData, ExternalAuthenticationFlowData, FilesFlowData,
            MandateRevokeFlowData, PaymentFlowData, RefundFlowData, WebhookSourceVerifyData,
        },
        RouterDataNew,
    },
};

use super::connector_integration_interface::Conversion;
use crate::errors;

impl<T, Req, Resp> Conversion<T, Req, Resp> for AccessTokenFlowData {
    fn from_old_router_data(
        _old_router_data: &RouterData<T, Req, Resp>,
    ) -> errors::CustomResult<RouterDataNew<T, Self, Req, Resp>, errors::ConnectorError>
    where
        Self: Sized,
    {
        todo!()
    }

    fn to_old_router_data(
        _new_router_data: RouterDataNew<T, Self, Req, Resp>,
    ) -> errors::CustomResult<RouterData<T, Req, Resp>, errors::ConnectorError>
    where
        Self: Sized,
    {
        todo!()
    }
}

impl<T, Req, Resp> Conversion<T, Req, Resp> for PaymentFlowData {
    fn from_old_router_data(
        _old_router_data: &RouterData<T, Req, Resp>,
    ) -> errors::CustomResult<RouterDataNew<T, Self, Req, Resp>, errors::ConnectorError>
    where
        Self: Sized,
    {
        todo!()
    }

    fn to_old_router_data(
        _new_router_data: RouterDataNew<T, Self, Req, Resp>,
    ) -> errors::CustomResult<RouterData<T, Req, Resp>, errors::ConnectorError>
    where
        Self: Sized,
    {
        todo!()
    }
}

impl<T, Req, Resp> Conversion<T, Req, Resp> for RefundFlowData {
    fn from_old_router_data(
        _old_router_data: &RouterData<T, Req, Resp>,
    ) -> errors::CustomResult<RouterDataNew<T, Self, Req, Resp>, errors::ConnectorError>
    where
        Self: Sized,
    {
        todo!()
    }

    fn to_old_router_data(
        _new_router_data: RouterDataNew<T, Self, Req, Resp>,
    ) -> errors::CustomResult<RouterData<T, Req, Resp>, errors::ConnectorError>
    where
        Self: Sized,
    {
        todo!()
    }
}

impl<T, Req, Resp> Conversion<T, Req, Resp> for DisputesFlowData {
    fn from_old_router_data(
        _old_router_data: &RouterData<T, Req, Resp>,
    ) -> errors::CustomResult<RouterDataNew<T, Self, Req, Resp>, errors::ConnectorError>
    where
        Self: Sized,
    {
        todo!()
    }

    fn to_old_router_data(
        _new_router_data: RouterDataNew<T, Self, Req, Resp>,
    ) -> errors::CustomResult<RouterData<T, Req, Resp>, errors::ConnectorError>
    where
        Self: Sized,
    {
        todo!()
    }
}

#[cfg(feature = "frm")]
impl<T, Req, Resp> Conversion<T, Req, Resp> for FrmFlowData {
    fn from_old_router_data(
        _old_router_data: &RouterData<T, Req, Resp>,
    ) -> errors::CustomResult<RouterDataNew<T, Self, Req, Resp>, errors::ConnectorError>
    where
        Self: Sized,
    {
        todo!()
    }

    fn to_old_router_data(
        _new_router_data: RouterDataNew<T, Self, Req, Resp>,
    ) -> errors::CustomResult<RouterData<T, Req, Resp>, errors::ConnectorError>
    where
        Self: Sized,
    {
        todo!()
    }
}

impl<T, Req, Resp> Conversion<T, Req, Resp> for FilesFlowData {
    fn from_old_router_data(
        _old_router_data: &RouterData<T, Req, Resp>,
    ) -> errors::CustomResult<RouterDataNew<T, Self, Req, Resp>, errors::ConnectorError>
    where
        Self: Sized,
    {
        todo!()
    }

    fn to_old_router_data(
        _new_router_data: RouterDataNew<T, Self, Req, Resp>,
    ) -> errors::CustomResult<RouterData<T, Req, Resp>, errors::ConnectorError>
    where
        Self: Sized,
    {
        todo!()
    }
}

impl<T, Req, Resp> Conversion<T, Req, Resp> for WebhookSourceVerifyData {
    fn from_old_router_data(
        _old_router_data: &RouterData<T, Req, Resp>,
    ) -> errors::CustomResult<RouterDataNew<T, Self, Req, Resp>, errors::ConnectorError>
    where
        Self: Sized,
    {
        todo!()
    }

    fn to_old_router_data(
        _new_router_data: RouterDataNew<T, Self, Req, Resp>,
    ) -> errors::CustomResult<RouterData<T, Req, Resp>, errors::ConnectorError>
    where
        Self: Sized,
    {
        todo!()
    }
}

impl<T, Req, Resp> Conversion<T, Req, Resp> for MandateRevokeFlowData {
    fn from_old_router_data(
        _old_router_data: &RouterData<T, Req, Resp>,
    ) -> errors::CustomResult<RouterDataNew<T, Self, Req, Resp>, errors::ConnectorError>
    where
        Self: Sized,
    {
        todo!()
    }

    fn to_old_router_data(
        _new_router_data: RouterDataNew<T, Self, Req, Resp>,
    ) -> errors::CustomResult<RouterData<T, Req, Resp>, errors::ConnectorError>
    where
        Self: Sized,
    {
        todo!()
    }
}

#[cfg(feature = "payouts")]
impl<T, Req, Resp> Conversion<T, Req, Resp> for PayoutFlowData {
    fn from_old_router_data(
        _old_router_data: &RouterData<T, Req, Resp>,
    ) -> errors::CustomResult<RouterDataNew<T, Self, Req, Resp>, errors::ConnectorError>
    where
        Self: Sized,
    {
        todo!()
    }

    fn to_old_router_data(
        _new_router_data: RouterDataNew<T, Self, Req, Resp>,
    ) -> errors::CustomResult<RouterData<T, Req, Resp>, errors::ConnectorError>
    where
        Self: Sized,
    {
        todo!()
    }
}
impl<T, Req, Resp> Conversion<T, Req, Resp> for ExternalAuthenticationFlowData {
    fn from_old_router_data(
        _old_router_data: &RouterData<T, Req, Resp>,
    ) -> errors::CustomResult<RouterDataNew<T, Self, Req, Resp>, errors::ConnectorError>
    where
        Self: Sized,
    {
        todo!()
    }

    fn to_old_router_data(
        _new_router_data: RouterDataNew<T, Self, Req, Resp>,
    ) -> errors::CustomResult<RouterData<T, Req, Resp>, errors::ConnectorError>
    where
        Self: Sized,
    {
        todo!()
    }
}
