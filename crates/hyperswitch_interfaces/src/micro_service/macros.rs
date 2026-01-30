/// Generate a `ClientOperation` impl and a `call` helper for a flow type.
///
/// # Examples
///
/// ```rust
/// use common_utils::request::{Headers, Method, RequestContent};
/// use hyperswitch_interfaces::micro_service::MicroserviceClient;
/// use router_env::RequestIdentifier;
/// use url::Url;
///
/// struct ExampleFlow;
/// struct ExampleV1Request;
/// struct ExampleV2Request;
/// struct ExampleV2Response;
/// struct ExampleResponse;
///
/// struct ExampleClient<'a> {
///     base_url: &'a Url,
///     headers: &'a Headers,
///     trace: &'a RequestIdentifier,
/// }
///
/// impl<'a> MicroserviceClient for ExampleClient<'a> {
///     fn base_url(&self) -> &Url { self.base_url }
///     fn parent_headers(&self) -> &Headers { self.headers }
///     fn trace(&self) -> &RequestIdentifier { self.trace }
/// }
///
/// impl TryFrom<&ExampleV1Request> for ExampleV2Request {
///     type Error = hyperswitch_interfaces::micro_service::MicroserviceClientError;
///
///     fn try_from(_: &ExampleV1Request) -> Result<Self, Self::Error> {
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
///     v1_request = ExampleV1Request,
///     v2_request = ExampleV2Request,
///     v2_response = ExampleV2Response,
///     v1_response = ExampleResponse,
///     client = ExampleClient<'_>,
///     body = |_, _| Some(RequestContent::Json(Box::new(serde_json::json!({}))))
/// );
/// ```
#[macro_export]
macro_rules! impl_microservice_flow {
    (
        $flow:ty,
        method = $method:expr,
        path = $path:expr,
        v1_request = $v1_req:ty,
        v2_request = $v2_req:ty,
        v2_response = $v2_resp:ty,
        v1_response = $v1_resp:ty,
        client = $client_ty:ty
        $(, body = $body_fn:expr)?
        $(, path_params = $path_params_fn:expr)?
        $(, query_params = $query_params_fn:expr)?
        $(, validate = $validate_fn:expr)?
    ) => {
        #[async_trait::async_trait]
        impl $crate::micro_service::ClientOperation for $flow {
            const METHOD: common_utils::request::Method = $method;
            const PATH_TEMPLATE: &'static str = $path;

            type V1Response = $v1_resp;
            type V1Request = $v1_req;
            type V2Request = $v2_req;
            type V2Response = $v2_resp;

            fn validate(
                &self,
                request: &Self::V1Request,
            ) -> Result<(), $crate::micro_service::MicroserviceClientError> {
                $($validate_fn(self, request)?;)?
                Ok(())
            }

            fn from_request(_request: &Self::V1Request) -> Self {
                Self
            }

            fn transform_request(
                &self,
                request: &Self::V1Request,
            ) -> Result<Self::V2Request, $crate::micro_service::MicroserviceClientError> {
                <Self::V2Request as TryFrom<&Self::V1Request>>::try_from(request)
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
                request: &Self::V1Request,
            ) -> Vec<(&'static str, String)> {
                $path_params_fn(self, request)
            }
            )?

            $(
            fn query_params(
                &self,
                request: &Self::V1Request,
            ) -> Vec<(&'static str, String)> {
                $query_params_fn(self, request)
            }
            )?
        }

        impl $flow {
            /// Execute the flow using the generic microservice pipeline.
            pub async fn call(
                state: &dyn $crate::api_client::ApiClientWrapper,
                client: &$client_ty,
                request: <Self as $crate::micro_service::ClientOperation>::V1Request,
            ) -> Result<
                <Self as $crate::micro_service::ClientOperation>::V1Response,
                $crate::micro_service::MicroserviceClientError,
            > {
                $crate::micro_service::execute_microservice_operation::<Self>(
                    state,
                    client,
                    request,
                )
                .await
            }
        }
    };
}
