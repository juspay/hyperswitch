#![allow(missing_docs)]

use actix_web::web::Data;
use actix_web::HttpRequest;
use memory_adapter::InMemoryPayments;

use crate::connector::Stripe;

#[ext_trait::extension(pub trait HttpRequestExt)]
impl HttpRequest {
    fn payments(&self) -> &InMemoryPayments {
        self.data()
    }

    fn connector(&self) -> &Stripe {
        self.data()
    }

    fn data<T: 'static>(&self) -> &T {
        self.app_data::<Data<T>>().unwrap().as_ref()
    }
}
