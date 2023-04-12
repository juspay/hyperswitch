use crate::{routes::AppState, services::{self, request::ContentType}, headers};


pub fn build_file_upload_request(
    state: &AppState,
    file_data: Vec<u8>,
) -> services::Request {
    let file_upload_config = &state.conf.file_upload_config;
    let mut url = format!("{}", file_upload_config.host);
    let mut request = services::Request::new(services::Method::Put, &url);
    request.add_header(headers::CONTENT_TYPE, "text/plain");
    request.set_file_data(file_data);
    request.add_content_type(ContentType::TextPlain);
    request
}
