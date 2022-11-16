use actix_web::{dev::ServiceResponse, middleware::ErrorHandlerResponse, ResponseError};
use http::StatusCode;

use super::ApiErrorResponse;
pub fn custom_error_handlers<B>(
    res: ServiceResponse<B>,
) -> actix_web::Result<ErrorHandlerResponse<B>> {
    let error_response = match res.status() {
        StatusCode::NOT_FOUND => ApiErrorResponse::InvalidRequestUrl,
        StatusCode::METHOD_NOT_ALLOWED => ApiErrorResponse::InvalidHttpMethod,
        _ => ApiErrorResponse::InternalServerError,
    };
    let (req, _) = res.into_parts();
    let res = error_response.error_response();
    let res = ServiceResponse::new(req, res)
        .map_into_boxed_body()
        .map_into_right_body();
    Ok(ErrorHandlerResponse::Response(res))
}

// can be used as .default_service for web::resource to modify the default behaviour of method_not_found error i.e. raised
// use actix_web::dev::ServiceRequest
// pub async fn default_service_405<E>(req: ServiceRequest) -> Result<ServiceResponse, E> {
//     Ok(req.into_response(ApiErrorResponse::InvalidHttpMethod.error_response()))
// }
