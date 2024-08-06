#[cfg(feature = "frm")]
use hyperswitch_domain_models::router_data_v2::flow_common_types::FrmFlowData;
#[cfg(feature = "payouts")]
use hyperswitch_domain_models::router_data_v2::flow_common_types::PayoutFlowData;
use hyperswitch_domain_models::{
    payment_address::PaymentAddress,
    router_data::{self, RouterData},
    router_data_v2::{
        flow_common_types::{
            AccessTokenFlowData, DisputesFlowData, ExternalAuthenticationFlowData, FilesFlowData,
            MandateRevokeFlowData, PaymentFlowData, RefundFlowData, WebhookSourceVerifyData,
        },
        RouterDataV2,
    },
};

use super::connector_integration_interface::RouterDataConversion;
use crate::errors;

fn get_irrelevant_id_string(id_name: &str, flow_name: &str) -> String {
    format!("irrelevant {id_name} in {flow_name} flow")
}
fn get_default_router_data<F, Req, Resp>(
    flow_name: &str,
    request: Req,
    response: Result<Resp, router_data::ErrorResponse>,
) -> RouterData<F, Req, Resp> {
    RouterData {
        flow: std::marker::PhantomData,
        merchant_id: common_utils::id_type::MerchantId::get_irrelevant_merchant_id(),
        customer_id: None,
        connector_customer: None,
        connector: get_irrelevant_id_string("connector", flow_name),
        payment_id: get_irrelevant_id_string("payment_id", flow_name),
        attempt_id: get_irrelevant_id_string("attempt_id", flow_name),
        status: common_enums::AttemptStatus::default(),
        payment_method: common_enums::PaymentMethod::default(),
        connector_auth_type: router_data::ConnectorAuthType::default(),
        description: None,
        return_url: None,
        address: PaymentAddress::default(),
        auth_type: common_enums::AuthenticationType::default(),
        connector_meta_data: None,
        connector_wallets_details: None,
        amount_captured: None,
        access_token: None,
        session_token: None,
        reference_id: None,
        payment_method_token: None,
        recurring_mandate_payment_data: None,
        preprocessing_id: None,
        payment_method_balance: None,
        connector_api_version: None,
        request,
        response,
        connector_request_reference_id: get_irrelevant_id_string(
            "connector_request_reference_id",
            flow_name,
        ),
        #[cfg(feature = "payouts")]
        payout_method_data: None,
        #[cfg(feature = "payouts")]
        quote_id: None,
        test_mode: None,
        connector_http_status_code: None,
        external_latency: None,
        apple_pay_flow: None,
        frm_metadata: None,
        dispute_id: None,
        refund_id: None,
        connector_response: None,
        payment_method_status: None,
        minor_amount_captured: None,
        integrity_check: Ok(()),
    }
}

impl<T, Req: Clone, Resp: Clone> RouterDataConversion<T, Req, Resp> for AccessTokenFlowData {
    fn from_old_router_data(
        old_router_data: &RouterData<T, Req, Resp>,
    ) -> errors::CustomResult<RouterDataV2<T, Self, Req, Resp>, errors::ConnectorError>
    where
        Self: Sized,
    {
        let resource_common_data = Self {};
        Ok(RouterDataV2 {
            flow: std::marker::PhantomData,
            resource_common_data,
            connector_auth_type: old_router_data.connector_auth_type.clone(),
            request: old_router_data.request.clone(),
            response: old_router_data.response.clone(),
        })
    }

    fn to_old_router_data(
        new_router_data: RouterDataV2<T, Self, Req, Resp>,
    ) -> errors::CustomResult<RouterData<T, Req, Resp>, errors::ConnectorError>
    where
        Self: Sized,
    {
        let Self {} = new_router_data.resource_common_data;
        let request = new_router_data.request.clone();
        let response = new_router_data.response.clone();
        let router_data = get_default_router_data("access token", request, response);
        Ok(router_data)
    }
}

impl<T, Req: Clone, Resp: Clone> RouterDataConversion<T, Req, Resp> for PaymentFlowData {
    fn from_old_router_data(
        old_router_data: &RouterData<T, Req, Resp>,
    ) -> errors::CustomResult<RouterDataV2<T, Self, Req, Resp>, errors::ConnectorError>
    where
        Self: Sized,
    {
        let resource_common_data = Self {
            merchant_id: old_router_data.merchant_id.clone(),
            customer_id: old_router_data.customer_id.clone(),
            connector_customer: old_router_data.connector_customer.clone(),
            payment_id: old_router_data.payment_id.clone(),
            attempt_id: old_router_data.attempt_id.clone(),
            status: old_router_data.status,
            payment_method: old_router_data.payment_method,
            description: old_router_data.description.clone(),
            return_url: old_router_data.return_url.clone(),
            address: old_router_data.address.clone(),
            auth_type: old_router_data.auth_type,
            connector_meta_data: old_router_data.connector_meta_data.clone(),
            amount_captured: old_router_data.amount_captured,
            minor_amount_captured: old_router_data.minor_amount_captured,
            access_token: old_router_data.access_token.clone(),
            session_token: old_router_data.session_token.clone(),
            reference_id: old_router_data.reference_id.clone(),
            payment_method_token: old_router_data.payment_method_token.clone(),
            recurring_mandate_payment_data: old_router_data.recurring_mandate_payment_data.clone(),
            preprocessing_id: old_router_data.preprocessing_id.clone(),
            payment_method_balance: old_router_data.payment_method_balance.clone(),
            connector_api_version: old_router_data.connector_api_version.clone(),
            connector_request_reference_id: old_router_data.connector_request_reference_id.clone(),
            test_mode: old_router_data.test_mode,
            connector_http_status_code: old_router_data.connector_http_status_code,
            external_latency: old_router_data.external_latency,
            apple_pay_flow: old_router_data.apple_pay_flow.clone(),
            connector_response: old_router_data.connector_response.clone(),
            payment_method_status: old_router_data.payment_method_status,
        };
        Ok(RouterDataV2 {
            flow: std::marker::PhantomData,
            resource_common_data,
            connector_auth_type: old_router_data.connector_auth_type.clone(),
            request: old_router_data.request.clone(),
            response: old_router_data.response.clone(),
        })
    }

    fn to_old_router_data(
        new_router_data: RouterDataV2<T, Self, Req, Resp>,
    ) -> errors::CustomResult<RouterData<T, Req, Resp>, errors::ConnectorError>
    where
        Self: Sized,
    {
        let Self {
            merchant_id,
            customer_id,
            connector_customer,
            payment_id,
            attempt_id,
            status,
            payment_method,
            description,
            return_url,
            address,
            auth_type,
            connector_meta_data,
            amount_captured,
            minor_amount_captured,
            access_token,
            session_token,
            reference_id,
            payment_method_token,
            recurring_mandate_payment_data,
            preprocessing_id,
            payment_method_balance,
            connector_api_version,
            connector_request_reference_id,
            test_mode,
            connector_http_status_code,
            external_latency,
            apple_pay_flow,
            connector_response,
            payment_method_status,
        } = new_router_data.resource_common_data;
        let mut router_data =
            get_default_router_data("payment", new_router_data.request, new_router_data.response);
        router_data.merchant_id = merchant_id;
        router_data.customer_id = customer_id;
        router_data.connector_customer = connector_customer;
        router_data.payment_id = payment_id;
        router_data.attempt_id = attempt_id;
        router_data.status = status;
        router_data.payment_method = payment_method;
        router_data.description = description;
        router_data.return_url = return_url;
        router_data.address = address;
        router_data.auth_type = auth_type;
        router_data.connector_meta_data = connector_meta_data;
        router_data.amount_captured = amount_captured;
        router_data.minor_amount_captured = minor_amount_captured;
        router_data.access_token = access_token;
        router_data.session_token = session_token;
        router_data.reference_id = reference_id;
        router_data.payment_method_token = payment_method_token;
        router_data.recurring_mandate_payment_data = recurring_mandate_payment_data;
        router_data.preprocessing_id = preprocessing_id;
        router_data.payment_method_balance = payment_method_balance;
        router_data.connector_api_version = connector_api_version;
        router_data.connector_request_reference_id = connector_request_reference_id;
        router_data.test_mode = test_mode;
        router_data.connector_http_status_code = connector_http_status_code;
        router_data.external_latency = external_latency;
        router_data.apple_pay_flow = apple_pay_flow;
        router_data.connector_response = connector_response;
        router_data.payment_method_status = payment_method_status;
        Ok(router_data)
    }
}

impl<T, Req: Clone, Resp: Clone> RouterDataConversion<T, Req, Resp> for RefundFlowData {
    fn from_old_router_data(
        old_router_data: &RouterData<T, Req, Resp>,
    ) -> errors::CustomResult<RouterDataV2<T, Self, Req, Resp>, errors::ConnectorError>
    where
        Self: Sized,
    {
        let resource_common_data = Self {
            merchant_id: old_router_data.merchant_id.clone(),
            customer_id: old_router_data.customer_id.clone(),
            payment_id: old_router_data.payment_id.clone(),
            attempt_id: old_router_data.attempt_id.clone(),
            status: old_router_data.status,
            payment_method: old_router_data.payment_method,
            return_url: old_router_data.return_url.clone(),
            connector_meta_data: old_router_data.connector_meta_data.clone(),
            amount_captured: old_router_data.amount_captured,
            minor_amount_captured: old_router_data.minor_amount_captured,
            connector_request_reference_id: old_router_data.connector_request_reference_id.clone(),
            refund_id: old_router_data.refund_id.clone().ok_or(
                errors::ConnectorError::MissingRequiredField {
                    field_name: "refund_id",
                },
            )?,
        };
        Ok(RouterDataV2 {
            flow: std::marker::PhantomData,
            resource_common_data,
            connector_auth_type: old_router_data.connector_auth_type.clone(),
            request: old_router_data.request.clone(),
            response: old_router_data.response.clone(),
        })
    }

    fn to_old_router_data(
        new_router_data: RouterDataV2<T, Self, Req, Resp>,
    ) -> errors::CustomResult<RouterData<T, Req, Resp>, errors::ConnectorError>
    where
        Self: Sized,
    {
        let Self {
            merchant_id,
            customer_id,
            payment_id,
            attempt_id,
            status,
            payment_method,
            return_url,
            connector_meta_data,
            amount_captured,
            minor_amount_captured,
            connector_request_reference_id,
            refund_id,
        } = new_router_data.resource_common_data;
        let mut router_data =
            get_default_router_data("refund", new_router_data.request, new_router_data.response);
        router_data.merchant_id = merchant_id;
        router_data.customer_id = customer_id;
        router_data.payment_id = payment_id;
        router_data.attempt_id = attempt_id;
        router_data.status = status;
        router_data.payment_method = payment_method;
        router_data.return_url = return_url;
        router_data.connector_meta_data = connector_meta_data;
        router_data.amount_captured = amount_captured;
        router_data.minor_amount_captured = minor_amount_captured;
        router_data.connector_request_reference_id = connector_request_reference_id;
        router_data.refund_id = Some(refund_id);
        Ok(router_data)
    }
}

impl<T, Req: Clone, Resp: Clone> RouterDataConversion<T, Req, Resp> for DisputesFlowData {
    fn from_old_router_data(
        old_router_data: &RouterData<T, Req, Resp>,
    ) -> errors::CustomResult<RouterDataV2<T, Self, Req, Resp>, errors::ConnectorError>
    where
        Self: Sized,
    {
        let resource_common_data = Self {
            merchant_id: old_router_data.merchant_id.clone(),
            payment_id: old_router_data.payment_id.clone(),
            attempt_id: old_router_data.attempt_id.clone(),
            payment_method: old_router_data.payment_method,
            return_url: old_router_data.return_url.clone(),
            connector_meta_data: old_router_data.connector_meta_data.clone(),
            amount_captured: old_router_data.amount_captured,
            minor_amount_captured: old_router_data.minor_amount_captured,
            connector_request_reference_id: old_router_data.connector_request_reference_id.clone(),
            dispute_id: old_router_data.dispute_id.clone().ok_or(
                errors::ConnectorError::MissingRequiredField {
                    field_name: "dispute_id",
                },
            )?,
        };
        Ok(RouterDataV2 {
            flow: std::marker::PhantomData,
            resource_common_data,
            connector_auth_type: old_router_data.connector_auth_type.clone(),
            request: old_router_data.request.clone(),
            response: old_router_data.response.clone(),
        })
    }

    fn to_old_router_data(
        new_router_data: RouterDataV2<T, Self, Req, Resp>,
    ) -> errors::CustomResult<RouterData<T, Req, Resp>, errors::ConnectorError>
    where
        Self: Sized,
    {
        let Self {
            merchant_id,
            payment_id,
            attempt_id,
            payment_method,
            return_url,
            connector_meta_data,
            amount_captured,
            minor_amount_captured,
            connector_request_reference_id,
            dispute_id,
        } = new_router_data.resource_common_data;
        let mut router_data = get_default_router_data(
            "Disputes",
            new_router_data.request,
            new_router_data.response,
        );
        router_data.merchant_id = merchant_id;
        router_data.payment_id = payment_id;
        router_data.attempt_id = attempt_id;
        router_data.payment_method = payment_method;
        router_data.return_url = return_url;
        router_data.connector_meta_data = connector_meta_data;
        router_data.amount_captured = amount_captured;
        router_data.minor_amount_captured = minor_amount_captured;
        router_data.connector_request_reference_id = connector_request_reference_id;
        router_data.dispute_id = Some(dispute_id);
        Ok(router_data)
    }
}

#[cfg(feature = "frm")]
impl<T, Req: Clone, Resp: Clone> RouterDataConversion<T, Req, Resp> for FrmFlowData {
    fn from_old_router_data(
        old_router_data: &RouterData<T, Req, Resp>,
    ) -> errors::CustomResult<RouterDataV2<T, Self, Req, Resp>, errors::ConnectorError>
    where
        Self: Sized,
    {
        let resource_common_data = Self {
            merchant_id: old_router_data.merchant_id.clone(),
            payment_id: old_router_data.payment_id.clone(),
            attempt_id: old_router_data.attempt_id.clone(),
            payment_method: old_router_data.payment_method,
            connector_request_reference_id: old_router_data.connector_request_reference_id.clone(),
            return_url: old_router_data.return_url.clone(),
            auth_type: old_router_data.auth_type,
            connector_wallets_details: old_router_data.connector_wallets_details.clone(),
            connector_meta_data: old_router_data.connector_meta_data.clone(),
            amount_captured: old_router_data.amount_captured,
            minor_amount_captured: old_router_data.minor_amount_captured,
        };
        Ok(RouterDataV2 {
            flow: std::marker::PhantomData,
            resource_common_data,
            connector_auth_type: old_router_data.connector_auth_type.clone(),
            request: old_router_data.request.clone(),
            response: old_router_data.response.clone(),
        })
    }

    fn to_old_router_data(
        new_router_data: RouterDataV2<T, Self, Req, Resp>,
    ) -> errors::CustomResult<RouterData<T, Req, Resp>, errors::ConnectorError>
    where
        Self: Sized,
    {
        let Self {
            merchant_id,
            payment_id,
            attempt_id,
            payment_method,
            connector_request_reference_id,
            return_url,
            auth_type,
            connector_wallets_details,
            connector_meta_data,
            amount_captured,
            minor_amount_captured,
        } = new_router_data.resource_common_data;
        let mut router_data =
            get_default_router_data("frm", new_router_data.request, new_router_data.response);

        router_data.merchant_id = merchant_id;
        router_data.payment_id = payment_id;
        router_data.attempt_id = attempt_id;
        router_data.payment_method = payment_method;
        router_data.connector_request_reference_id = connector_request_reference_id;
        router_data.return_url = return_url;
        router_data.auth_type = auth_type;
        router_data.connector_wallets_details = connector_wallets_details;
        router_data.connector_meta_data = connector_meta_data;
        router_data.amount_captured = amount_captured;
        router_data.minor_amount_captured = minor_amount_captured;

        Ok(router_data)
    }
}

impl<T, Req: Clone, Resp: Clone> RouterDataConversion<T, Req, Resp> for FilesFlowData {
    fn from_old_router_data(
        old_router_data: &RouterData<T, Req, Resp>,
    ) -> errors::CustomResult<RouterDataV2<T, Self, Req, Resp>, errors::ConnectorError>
    where
        Self: Sized,
    {
        let resource_common_data = Self {
            merchant_id: old_router_data.merchant_id.clone(),
            payment_id: old_router_data.payment_id.clone(),
            attempt_id: old_router_data.attempt_id.clone(),
            return_url: old_router_data.return_url.clone(),
            connector_meta_data: old_router_data.connector_meta_data.clone(),
            connector_request_reference_id: old_router_data.connector_request_reference_id.clone(),
        };
        Ok(RouterDataV2 {
            flow: std::marker::PhantomData,
            resource_common_data,
            connector_auth_type: old_router_data.connector_auth_type.clone(),
            request: old_router_data.request.clone(),
            response: old_router_data.response.clone(),
        })
    }

    fn to_old_router_data(
        new_router_data: RouterDataV2<T, Self, Req, Resp>,
    ) -> errors::CustomResult<RouterData<T, Req, Resp>, errors::ConnectorError>
    where
        Self: Sized,
    {
        let Self {
            merchant_id,
            payment_id,
            attempt_id,
            return_url,
            connector_meta_data,
            connector_request_reference_id,
        } = new_router_data.resource_common_data;
        let mut router_data =
            get_default_router_data("files", new_router_data.request, new_router_data.response);
        router_data.merchant_id = merchant_id;
        router_data.payment_id = payment_id;
        router_data.attempt_id = attempt_id;
        router_data.return_url = return_url;
        router_data.connector_meta_data = connector_meta_data;
        router_data.connector_request_reference_id = connector_request_reference_id;

        Ok(router_data)
    }
}

impl<T, Req: Clone, Resp: Clone> RouterDataConversion<T, Req, Resp> for WebhookSourceVerifyData {
    fn from_old_router_data(
        old_router_data: &RouterData<T, Req, Resp>,
    ) -> errors::CustomResult<RouterDataV2<T, Self, Req, Resp>, errors::ConnectorError>
    where
        Self: Sized,
    {
        let resource_common_data = Self {
            merchant_id: old_router_data.merchant_id.clone(),
        };
        Ok(RouterDataV2 {
            flow: std::marker::PhantomData,
            resource_common_data,
            connector_auth_type: old_router_data.connector_auth_type.clone(),
            request: old_router_data.request.clone(),
            response: old_router_data.response.clone(),
        })
    }

    fn to_old_router_data(
        new_router_data: RouterDataV2<T, Self, Req, Resp>,
    ) -> errors::CustomResult<RouterData<T, Req, Resp>, errors::ConnectorError>
    where
        Self: Sized,
    {
        let Self { merchant_id } = new_router_data.resource_common_data;
        let mut router_data = get_default_router_data(
            "webhook source verify",
            new_router_data.request,
            new_router_data.response,
        );
        router_data.merchant_id = merchant_id;
        Ok(router_data)
    }
}

impl<T, Req: Clone, Resp: Clone> RouterDataConversion<T, Req, Resp> for MandateRevokeFlowData {
    fn from_old_router_data(
        old_router_data: &RouterData<T, Req, Resp>,
    ) -> errors::CustomResult<RouterDataV2<T, Self, Req, Resp>, errors::ConnectorError>
    where
        Self: Sized,
    {
        let resource_common_data = Self {
            merchant_id: old_router_data.merchant_id.clone(),
            customer_id: old_router_data.customer_id.clone().ok_or(
                errors::ConnectorError::MissingRequiredField {
                    field_name: "customer_id",
                },
            )?,
            payment_id: Some(old_router_data.payment_id.clone()),
        };
        Ok(RouterDataV2 {
            flow: std::marker::PhantomData,
            resource_common_data,
            connector_auth_type: old_router_data.connector_auth_type.clone(),
            request: old_router_data.request.clone(),
            response: old_router_data.response.clone(),
        })
    }

    fn to_old_router_data(
        new_router_data: RouterDataV2<T, Self, Req, Resp>,
    ) -> errors::CustomResult<RouterData<T, Req, Resp>, errors::ConnectorError>
    where
        Self: Sized,
    {
        let Self {
            merchant_id,
            customer_id,
            payment_id,
        } = new_router_data.resource_common_data;
        let mut router_data = get_default_router_data(
            "mandate revoke",
            new_router_data.request,
            new_router_data.response,
        );
        router_data.merchant_id = merchant_id;
        router_data.customer_id = Some(customer_id);
        router_data.payment_id =
            payment_id.unwrap_or_else(|| get_irrelevant_id_string("payment_id", "mandate revoke"));
        Ok(router_data)
    }
}

#[cfg(feature = "payouts")]
impl<T, Req: Clone, Resp: Clone> RouterDataConversion<T, Req, Resp> for PayoutFlowData {
    fn from_old_router_data(
        old_router_data: &RouterData<T, Req, Resp>,
    ) -> errors::CustomResult<RouterDataV2<T, Self, Req, Resp>, errors::ConnectorError>
    where
        Self: Sized,
    {
        let resource_common_data = Self {
            merchant_id: old_router_data.merchant_id.clone(),
            customer_id: old_router_data.customer_id.clone(),
            connector_customer: old_router_data.connector_customer.clone(),
            return_url: old_router_data.return_url.clone(),
            address: old_router_data.address.clone(),
            connector_meta_data: old_router_data.connector_meta_data.clone(),
            connector_wallets_details: old_router_data.connector_wallets_details.clone(),
            connector_request_reference_id: old_router_data.connector_request_reference_id.clone(),
            payout_method_data: old_router_data.payout_method_data.clone(),
            quote_id: old_router_data.quote_id.clone(),
        };
        Ok(RouterDataV2 {
            flow: std::marker::PhantomData,
            resource_common_data,
            connector_auth_type: old_router_data.connector_auth_type.clone(),
            request: old_router_data.request.clone(),
            response: old_router_data.response.clone(),
        })
    }

    fn to_old_router_data(
        new_router_data: RouterDataV2<T, Self, Req, Resp>,
    ) -> errors::CustomResult<RouterData<T, Req, Resp>, errors::ConnectorError>
    where
        Self: Sized,
    {
        let Self {
            merchant_id,
            customer_id,
            connector_customer,
            return_url,
            address,
            connector_meta_data,
            connector_wallets_details,
            connector_request_reference_id,
            payout_method_data,
            quote_id,
        } = new_router_data.resource_common_data;
        let mut router_data =
            get_default_router_data("payout", new_router_data.request, new_router_data.response);
        router_data.merchant_id = merchant_id;
        router_data.customer_id = customer_id;
        router_data.connector_customer = connector_customer;
        router_data.return_url = return_url;
        router_data.address = address;
        router_data.connector_meta_data = connector_meta_data;
        router_data.connector_wallets_details = connector_wallets_details;
        router_data.connector_request_reference_id = connector_request_reference_id;
        router_data.payout_method_data = payout_method_data;
        router_data.quote_id = quote_id;
        Ok(router_data)
    }
}
impl<T, Req: Clone, Resp: Clone> RouterDataConversion<T, Req, Resp>
    for ExternalAuthenticationFlowData
{
    fn from_old_router_data(
        old_router_data: &RouterData<T, Req, Resp>,
    ) -> errors::CustomResult<RouterDataV2<T, Self, Req, Resp>, errors::ConnectorError>
    where
        Self: Sized,
    {
        let resource_common_data = Self {
            merchant_id: old_router_data.merchant_id.clone(),
            connector_meta_data: old_router_data.connector_meta_data.clone(),
            address: old_router_data.address.clone(),
        };
        Ok(RouterDataV2 {
            flow: std::marker::PhantomData,
            resource_common_data,
            connector_auth_type: old_router_data.connector_auth_type.clone(),
            request: old_router_data.request.clone(),
            response: old_router_data.response.clone(),
        })
    }

    fn to_old_router_data(
        new_router_data: RouterDataV2<T, Self, Req, Resp>,
    ) -> errors::CustomResult<RouterData<T, Req, Resp>, errors::ConnectorError>
    where
        Self: Sized,
    {
        let Self {
            merchant_id,
            connector_meta_data,
            address,
        } = new_router_data.resource_common_data;
        let mut router_data = get_default_router_data(
            "external authentication",
            new_router_data.request,
            new_router_data.response,
        );
        router_data.merchant_id = merchant_id;
        router_data.connector_meta_data = connector_meta_data;
        router_data.address = address;
        Ok(router_data)
    }
}
