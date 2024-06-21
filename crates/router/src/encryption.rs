use crate::{
    errors, headers, services,
    types::{domain::EncryptionCreateRequest, RequestContent, Response},
    SessionState,
};

pub async fn call_encryption_service<T>(
    state: &SessionState,
    endpoint: &str,
    request_body: T,
) -> errors::CustomResult<Result<Response, Response>, errors::ApiClientError>
where
    T: masking::ErasedMaskSerialize + Send + Sync + 'static,
{
    let url = format!("{}/{}", &state.conf.key_manager.url, endpoint);

    let encryption_req = services::RequestBuilder::new()
        .method(services::Method::Post)
        .url(&url)
        .attach_default_headers()
        .headers(vec![(
            headers::CONTENT_TYPE.to_string(),
            "application/json".to_string().into(),
        )])
        .set_body(RequestContent::Json(Box::new(request_body)))
        .build();

    services::call_connector_api(state, encryption_req, "EncryptionServiceRequest").await
}

pub async fn create_key_in_key_manager(
    state: &SessionState,
    request_body: EncryptionCreateRequest,
) -> errors::CustomResult<(), errors::ApiClientError> {
    let _ = call_encryption_service(state, "/key/create", request_body).await?;

    Ok(())
}
