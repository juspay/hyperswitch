/// Generate a `ClientOperation` impl and a `call` helper for a flow type.
///
/// # Examples
///
/// ```rust
/// use common_utils::request::{Method, RequestContent};
/// use hyperswitch_interfaces::micro_service::payment_method::PaymentMethodClient;
///
/// struct ExampleFlow;
/// struct ExampleV2Request;
/// struct ExampleV2Response;
/// struct ExampleResponse;
///
/// impl TryFrom<&ExampleFlow> for ExampleV2Request {
///     type Error = hyperswitch_interfaces::micro_service::MicroserviceClientError;
///
///     fn try_from(_: &ExampleFlow) -> Result<Self, Self::Error> {
///         Ok(Self)
///     }
/// }
///
/// impl TryFrom<ExampleV2Response> for ExampleResponse {
///     type Error = hyperswitch_interfaces::micro_service::MicroserviceClientError;
///
///     fn try_from(_: ExampleV2Response) -> Result<Self, Self::Error> {
///         Ok(Self)
///     }
/// }
///
/// impl_microservice_flow!(
///     ExampleFlow,
///     method = Method::Post,
///     path = "/v2/example",
///     v2_request = ExampleV2Request,
///     v2_response = ExampleV2Response,
///     v1_response = ExampleResponse,
///     client = PaymentMethodClient<'_>,
///     body = |_, _| Some(RequestContent::Json(Box::new(serde_json::json!({}))))
/// );
/// ```
#[macro_export]
macro_rules! impl_microservice_flow {
    (
        $flow:ty,
        method = $method:expr,
        path = $path:expr,
        v2_request = $v2_req:ty,
        v2_response = $v2_resp:ty,
        v1_response = $v1_resp:ty,
        client = $client_ty:ty
        $(, body = $body_fn:expr)?
        $(, path_params = $path_params_fn:expr)?
        $(, validate = $validate_fn:expr)?
    ) => {
        #[async_trait::async_trait]
        impl $crate::micro_service::ClientOperation for $flow {
            const METHOD: common_utils::request::Method = $method;
            const PATH_TEMPLATE: &'static str = $path;

            type V1Response = $v1_resp;
            type V2Request = $v2_req;
            type V2Response = $v2_resp;

            fn validate(&self) -> Result<(), $crate::micro_service::MicroserviceClientError> {
                $($validate_fn(self)?;)?
                Ok(())
            }

            fn transform_request(
                &self,
            ) -> Result<Self::V2Request, $crate::micro_service::MicroserviceClientError> {
                <Self::V2Request as TryFrom<&Self>>::try_from(self)
            }

            fn transform_response(
                &self,
                response: Self::V2Response,
            ) -> Result<Self::V1Response, $crate::micro_service::MicroserviceClientError> {
                <Self::V1Response as TryFrom<Self::V2Response>>::try_from(response)
            }

            $(
            fn body(
                &self,
                request: Self::V2Request,
            ) -> Option<common_utils::request::RequestContent> {
                $body_fn(self, request)
            }
            )?

            $(
            fn path_params(
                &self,
                request: &Self::V2Request,
            ) -> Vec<(&'static str, String)> {
                $path_params_fn(self, request)
            }
            )?
        }

        impl $flow {
            /// Execute the flow using the generic microservice pipeline.
            pub async fn call(
                state: &dyn $crate::api_client::ApiClientWrapper,
                client: &$client_ty,
                request: Self,
            ) -> Result<
                <Self as $crate::micro_service::ClientOperation>::V1Response,
                $crate::micro_service::MicroserviceClientError,
            > {
                $crate::micro_service::execute_microservice_operation(state, client, request).await
            }
        }
    };
}
