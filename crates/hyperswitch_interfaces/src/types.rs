#[derive(Clone, Debug)]
pub struct Response {
    pub headers: Option<http::HeaderMap>,
    pub response: bytes::Bytes,
    pub status_code: u16,
}
