use std::marker::PhantomData;

use common_utils::ext_traits::BytesExt;
use error_stack::ResultExt;
use hyperswitch_domain_models::router_data::RouterData;

use crate::errors;

pub mod domain_types {
    pub use hyperswitch_domain_models::router_data::RouterData;
}

pub trait FlowTypes {
    type Flow;
    type Request;
    type Response;
}

impl<F, Req, Resp> FlowTypes for RouterData<F, Req, Resp> {
    type Flow = F;
    type Request = Req;
    type Response = Resp;
}

pub trait GetFormData {
    fn get_form_data(&self) -> reqwest::multipart::Form;
}

pub struct NoRequestBody;
pub struct NoRequestBodyTemplating;

impl<F, Req, Resp> TryFrom<RouterData<F, Req, Resp>> for NoRequestBody {
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(_value: RouterData<F, Req, Resp>) -> Result<Self, Self::Error> {
        Ok(NoRequestBody)
    }
}

pub trait BridgeRequestResponse: Send + Sync {
    type RequestBody;
    type ResponseBody;
    type ConnectorInputData: FlowTypes;
    fn request_body(
        &self,
        rd: Self::ConnectorInputData,
    ) -> errors::CustomResult<Self::RequestBody, errors::ConnectorError>
    where
        Self::RequestBody:
            TryFrom<Self::ConnectorInputData, Error = error_stack::Report<errors::ConnectorError>>,
    {
        // <Self::ConnectorRouterData as TryInto<Self::RequestBody>>::try_into(rd)
        Self::RequestBody::try_from(rd)
    }

    fn response(
        &self,
        bytes: bytes::Bytes,
    ) -> errors::CustomResult<Self::ResponseBody, errors::ConnectorError>
    where
        Self::ResponseBody: for<'a> serde::Deserialize<'a>,
    {
        bytes
            .parse_struct(std::any::type_name::<Self::ResponseBody>())
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)
    }

    fn router_data(
        &self,
        response: crate::types::ResponseRouterData<
            <Self::ConnectorInputData as FlowTypes>::Flow,
            Self::ResponseBody,
            <Self::ConnectorInputData as FlowTypes>::Request,
            <Self::ConnectorInputData as FlowTypes>::Response,
        >,
    ) -> errors::CustomResult<
        RouterData<
            <Self::ConnectorInputData as FlowTypes>::Flow,
            <Self::ConnectorInputData as FlowTypes>::Request,
            <Self::ConnectorInputData as FlowTypes>::Response,
        >,
        errors::ConnectorError,
    >
    where
        RouterData<
            <Self::ConnectorInputData as FlowTypes>::Flow,
            <Self::ConnectorInputData as FlowTypes>::Request,
            <Self::ConnectorInputData as FlowTypes>::Response,
        >: TryFrom<
            crate::types::ResponseRouterData<
                <Self::ConnectorInputData as FlowTypes>::Flow,
                Self::ResponseBody,
                <Self::ConnectorInputData as FlowTypes>::Request,
                <Self::ConnectorInputData as FlowTypes>::Response,
            >,
            Error = error_stack::Report<errors::ConnectorError>,
        >,
    {
        // <Self::ResponseBody as TryInto<Self::RouterData, Error = error_stack::Report<errors::ConnectorError>>>::try_into(response)
        RouterData::<
            <Self::ConnectorInputData as FlowTypes>::Flow,
            <Self::ConnectorInputData as FlowTypes>::Request,
            <Self::ConnectorInputData as FlowTypes>::Response,
        >::try_from(response)
    }
}

#[derive(Clone)]
pub struct Bridge<Q, S>(pub PhantomData<(Q, S)>);

macro_rules! expand_fn_get_request_body {
    (
        $connector: ty,
        $curl_req: ty,
        FormData,
        $curl_res: ty,
        $flow: ident,
        $resource_common_data: ty,
        $request: ty,
        $response: ty
        $(,amount_conversion: {
            amount_converter: $amount_converter: expr,
            minor_amount: $minor_amount: expr,
            currency: $currency: expr
        })?
    ) => {
        paste::paste! {
            fn get_request_body(
                &self,
                req: &crate::types::RouterData<domain_types::$flow, $request, $response>,
                connectors: &crate::settings::Connectors,
            ) -> CustomResult<RequestContent, errors::ConnectorError>
            {
                let bridge = self.[< $flow:snake >];
                let amount =
                let input_data = [<$connector InputData>] {
                    connector: self.to_owned(),
                    router_data: req.clone(),
                    connectors.clone(),
                };
                let request = bridge.request_body(input_data)?;
                let form_data = <&curl_req as GetFormData>::get_form_data(request)
                Ok(common_utils::request::RequestContent::FormData(form_data))
            }
        }
    };
    (
        $connector: ty,
        $curl_req: ty,
        $content_type: ident,
        $curl_res: ty,
        $flow: ident,
        $resource_common_data: ty,
        $request: ty,
        $response: ty
    ) => {
        paste::paste! {
            fn get_request_body(
                &self,
                req: &crate::types::RouterData<domain_types::$flow, $request, $response>,
                connectors: &crate::settings::Connectors,
            ) -> CustomResult<RequestContent, errors::ConnectorError>
            {
                let bridge = self.[< $flow:snake >];
                let input_data = [< $connector InputData >] {
                    connector: self.to_owned(),
                    router_data: req.clone(),
                    connectors: connectors.clone(),
                };
                let request = bridge.request_body(input_data)?;
                Ok(common_utils::request::RequestContent::$content_type(Box::new(request)))
            }
        }
    };
    ($connector: ty, $curl_res: ty, $flow: ident, $resource_common_data: ty, $request: ty, $response: ty) => {
        paste::paste! {
            fn get_request_body(
                &self,
                _req: &crate::types::RouterData<domain_types::$flow, $request, $response>,
                _connectors: &crate::settings::Connectors,
            ) -> CustomResult<RequestContent, errors::ConnectorError>
            {
                // always return None
                Ok(common_utils::request::RequestContent::RawBytes(vec![1]))
            }
        }
    };
}
pub(crate) use expand_fn_get_request_body;

macro_rules! expand_fn_handle_response{
    ($connector: ty, $flow: ident, $resource_common_data: ty, $request: ty, $response: ty) => {
        fn handle_response(
            &self,
            data: &crate::types::RouterData<domain_types::$flow,  $request, $response>,
            event_builder: Option<&mut ConnectorEvent>,
            res: types::Response,
        ) -> CustomResult<crate::types::RouterData<domain_types::$flow,  $request, $response>,  errors::ConnectorError>
        {
            paste::paste!{let bridge = self.[< $flow:snake >];}
            let response_body = bridge.response(res.response)?;
            event_builder.map(|i| i.set_response_body(&response_body));
            router_env::logger::info!(connector_response=?response_body);
            let response_router_data = crate::types::ResponseRouterData {
                response: response_body,
                data: data.clone(),
                http_code: res.status_code,
            };
            let result = bridge.router_data(response_router_data)?;
            Ok(result)
        }
    }
}
pub(crate) use expand_fn_handle_response;

macro_rules! expand_default_functions {
    (
        function: get_headers,
        flow_name:$flow: ident,
        resource_common_data:$resource_common_data: ty,
        flow_request:$request: ty,
        flow_response:$response: ty,
    ) => {
        fn get_headers(
            &self,
            req: &crate::types::RouterData<domain_types::$flow, $request, $response>,
            connectors: &settings::Connectors,
        ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
            self.build_headers(req, connectors)
        }
    };
    (
        function: get_content_type,
        flow_name:$flow: ident,
        resource_common_data:$resource_common_data: ty,
        flow_request:$request: ty,
        flow_response:$response: ty,
    ) => {
        fn get_content_type(&self) -> &'static str {
            self.common_get_content_type()
        }
    };
    (
        function: get_error_response,
        flow_name:$flow: ident,
        resource_common_data:$resource_common_data: ty,
        flow_request:$request: ty,
        flow_response:$response: ty,
    ) => {
        fn get_error_response(
            &self,
            res: types::Response,
            event_builder: Option<&mut ConnectorEvent>,
        ) -> CustomResult<types::ErrorResponse, errors::ConnectorError> {
            self.build_error_response(res, event_builder)
        }
    };
}
pub(crate) use expand_default_functions;

macro_rules! macro_connector_implementation {
    (
        connector_default_implementations: [$($function_name: ident), *],
        connector: $connector: ty,
        $(curl_request: $content_type:ident($curl_req: ty),)?
        curl_response:$curl_res: ty,
        flow_name:$flow: ident,
        resource_common_data:$resource_common_data: ty,
        flow_request:$request: ty,
        flow_response:$response: ty,
        $(amount_conversion: {
            amount_converter: $amount_converter: expr,
            minor_amount: $minor_amount: expr,
            currency: $currency: expr
        })?
    ) => {
        $(
            crate::connector::macros::expand_default_functions!(
                function: $function_name,
                flow_name:$flow,
                resource_common_data:$resource_common_data,
                flow_request:$request,
                flow_response:$response,
            );
        )*
        crate::connector::macros::expand_fn_get_request_body!(
            $connector,
            $($curl_req,)?
            $($content_type,)?
            $curl_res,
            $flow,
            $resource_common_data,
            $request,
            $response

        );
        crate::connector::macros::expand_fn_handle_response!(
            $connector,
            $flow,
            $resource_common_data,
            $request,
            $response
        );
    }
}
pub(crate) use macro_connector_implementation;

/// This macro will create two modules within connector modules, namely connector_types and domain_types
/// connector_types will contain all
/// All macros will
macro_rules! create_module_and_template_for_request_and_response_types{
    (
        connector_types: {
            $((
                path: $conector_path: path,
                types:{ $($connector_type_name:ident),+}
            ),)+
        },
        domain_types: {
            $((
                path: $flow_path: path,
                types:{ $($flow_type_name:ident),+}
            ),)+
        }
    ) => {
        $($(
            paste::paste!{pub struct [<$connector_type_name Templating>]; }
        )+)+
        paste::paste!{
            pub mod connector_types {
                $(
                    pub use $conector_path::{
                        $($connector_type_name,)+
                    };
                )+

                pub use crate::connector::macros::NoRequestBody;
            }
            pub mod domain_types {
                $(
                    pub use $flow_path::{
                        $($flow_type_name,)+
                    };
                )+
            }
        }
    };
}
pub(crate) use create_module_and_template_for_request_and_response_types;
macro_rules! impl_templating {
    (
        connector: $connector: ty,
        curl_request: $curl_req: ty,
        curl_response: $curl_res: ty,
        router_data: $router_data: ty
    ) => {
        paste::paste!{
            impl BridgeRequestResponse for Bridge<[<$curl_req Templating>], [<$curl_res Templating>]> {
                type RequestBody = connector_types::$curl_req;
                type ResponseBody = connector_types::$curl_res;
                type ConnectorInputData = [<$connector InputData>]<$router_data>;
            }
        }
    };
    (
        connector: $connector: ty,
        curl_response: $curl_res: ty,
        router_data: $router_data: ty
    ) => {
        paste::paste!{
            impl BridgeRequestResponse for Bridge<crate::connector::macros::NoRequestBodyTemplating, [<$curl_res Templating>]> {
                type RequestBody = NoRequestBody;
                type ResponseBody = connector_types::$curl_res;
                type ConnectorInputData = [<$connector InputData>]<$router_data>;
            }
        }
    }
}

macro_rules! expand_imports{
    (connector: $connector: ident)=>{
        paste::paste! {
            use crate::services;
            use crate::types::api;
            use crate::configs::settings;
            use crate::connector::macros::{Bridge, NoRequestBodyTemplating, BridgeRequestResponse,NoRequestBody, macro_connector_implementation};
        }
    }
}
pub(crate) use expand_imports;
pub(crate) use impl_templating;

macro_rules! expand_connector_input_data {
    ($connector: ident) => {
        paste::paste! {
            pub struct [<$connector InputData>]<RD: crate::connector::macros::FlowTypes> {
                connector: $connector,
                router_data: RD,
                connectors: settings::Connectors,
            }
            impl<RD: crate::connector::macros::FlowTypes> crate::connector::macros::FlowTypes for [<$connector InputData>]<RD> {
                type Flow = RD::Flow;
                type Request = RD::Request;
                type Response = RD::Response;
            }
        }
    }
}
pub(crate) use expand_connector_input_data;

macro_rules! create_all_prerequisites2 {
    (
        connector_name: $connector: ident,
        api: [
            $(
                (
                    flow: $flow_name: ident,
                    request_body: $flow_request: ident,
                    response_body: $flow_response: ident,
                    router_data: $router_data_type: ty
                )
            ),*
        ],
        amount_converters: [
            $($converter_name:ident : $amount_unit:ty),*
        ]
    ) => {
        crate::connector::macros::expand_imports!(connector: $connector);
        crate::connector::macros::expand_connector_input_data!($connector);
        paste::paste! {
            #[derive(Clone)]
            pub struct $connector {
                $(
                    $converter_name: &'static (dyn common_utils::types::AmountConvertor<Output = $amount_unit> + Sync),
                )*
                $(
                    [<$flow_name:snake>]: &'static (dyn BridgeRequestResponse<
                        RequestBody = connector_types::$flow_request,
                        ResponseBody = connector_types::$flow_response,
                        ConnectorInputData = [<$connector InputData>]<$router_data_type>,
                    >),
                )*
            }
            impl $connector {
                pub const fn new() -> &'static Self {
                    &Self{
                        $(
                            $converter_name: &common_utils::types::[<$amount_unit ForConnector>],
                        )*
                        $(
                            [<$flow_name:snake>]: &Bridge::<
                                    [<$flow_request Templating>],
                                    [<$flow_response Templating>]
                                >(PhantomData),
                        )*
                    }
                }
            }

        }
    }
}
pub(crate) use create_all_prerequisites2;
