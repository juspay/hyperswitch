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
