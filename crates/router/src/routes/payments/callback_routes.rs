//! Names of the payment callbacks registered with Actix.
//!
//! URL generation uses these names via `HttpRequest::url_for`. Actix
//! registration iterates every variant, so adding a route requires a matching
//! path and handler at compile time.

use actix_web::{web, Resource};

use super as payments;

#[derive(Clone, Copy, Debug, Eq, PartialEq, strum::EnumIter)]
pub enum PaymentCallbackRoute {
    Start,
    RedirectResponse,
    RedirectResponseWithCredentials,
    RedirectComplete,
    RedirectCompleteWithCredentials,
    Authorize,
    Authentication,
}

impl PaymentCallbackRoute {
    pub const fn name(self) -> &'static str {
        match self {
            Self::Start => "payment_callback_start",
            Self::RedirectResponse => "payment_callback_redirect_response",
            Self::RedirectResponseWithCredentials => {
                "payment_callback_redirect_response_with_credentials"
            }
            Self::RedirectComplete => "payment_callback_redirect_complete",
            Self::RedirectCompleteWithCredentials => {
                "payment_callback_redirect_complete_with_credentials"
            }
            Self::Authorize => "payment_callback_authorize",
            Self::Authentication => "payment_callback_authentication",
        }
    }

    pub const fn path(self) -> &'static str {
        match self {
            Self::Start => "/redirect/{payment_id}/{merchant_id}/{attempt_id}",
            Self::RedirectResponse => "/{payment_id}/{merchant_id}/redirect/response/{connector}",
            Self::RedirectResponseWithCredentials => {
                "/{payment_id}/{merchant_id}/redirect/response/{connector}/{creds_identifier}"
            }
            Self::RedirectComplete => "/{payment_id}/{merchant_id}/redirect/complete/{connector}",
            Self::RedirectCompleteWithCredentials => {
                "/{payment_id}/{merchant_id}/redirect/complete/{connector}/{creds_identifier}"
            }
            Self::Authorize => "/{payment_id}/{merchant_id}/authorize/{connector}",
            Self::Authentication => "/{payment_id}/3ds/authentication",
        }
    }

    /// Build the named resource with the methods and handlers for this callback.
    pub fn resource(self) -> Resource {
        let resource = web::resource(self.path()).name(self.name());
        match self {
            Self::Start => resource.route(web::get().to(payments::payments_start)),
            Self::RedirectResponse => resource
                .route(web::get().to(payments::payments_redirect_response))
                .route(web::post().to(payments::payments_redirect_response)),
            Self::RedirectResponseWithCredentials => resource
                .route(web::get().to(payments::payments_redirect_response_with_creds_identifier)),
            Self::RedirectComplete => resource
                .route(web::get().to(payments::payments_complete_authorize_redirect))
                .route(web::post().to(payments::payments_complete_authorize_redirect)),
            Self::RedirectCompleteWithCredentials => resource
                .route(
                    web::get()
                        .to(payments::payments_complete_authorize_redirect_with_creds_identifier),
                )
                .route(
                    web::post()
                        .to(payments::payments_complete_authorize_redirect_with_creds_identifier),
                ),
            Self::Authorize => resource
                .route(web::post().to(payments::post_3ds_payments_authorize))
                .route(web::get().to(payments::post_3ds_payments_authorize)),
            Self::Authentication => {
                resource.route(web::post().to(payments::payments_external_authentication))
            }
        }
    }
}
