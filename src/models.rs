use actix_web::{
    body::BoxBody,
    http::header::ContentType,
    HttpRequest,
    HttpResponse,
    Responder,
};
use serde::Serialize;

#[derive(Queryable, Serialize)]
pub struct Payments {
	pub id: i32,
	pub payment_id: String,
	pub merchant_id: String,
	pub status: String,
}

impl Responder for Payments {
    type Body = BoxBody;

    fn respond_to(self, _req: &HttpRequest) -> HttpResponse<Self::Body> {
        let body = serde_json::to_string(&self).unwrap();

        HttpResponse::Ok()
            .content_type(ContentType::json())
            .body(body)
    }   
}
