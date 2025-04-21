use crate::services::ApplicationResponse;

pub fn track_response_status_code<Q>(response: &ApplicationResponse<Q>) -> i64 {
    match response {
        ApplicationResponse::Json(_)
        | ApplicationResponse::StatusOk
        | ApplicationResponse::TextPlain(_)
        | ApplicationResponse::Form(_)
        | ApplicationResponse::GenericLinkForm(_)
        | ApplicationResponse::PaymentLinkForm(_)
        | ApplicationResponse::FileData(_)
        | ApplicationResponse::JsonWithHeaders(_) => 200,
        ApplicationResponse::JsonForRedirection(_) => 302,
    }
}
