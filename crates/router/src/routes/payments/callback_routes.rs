//! Named payment callbacks and their URL path constructors.
//!
//! URL constructors include the base URL and `/payments` scope. Actix registration
//! iterates every variant, so adding a route requires a matching path and handler
//! at compile time.

#[cfg(feature = "v1")]
use actix_web::{web, Resource};

#[cfg(feature = "v1")]
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
    pub const PAYMENT_ID: &'static str = "{payment_id}";
    pub const MERCHANT_ID: &'static str = "{merchant_id}";
    pub const ATTEMPT_ID: &'static str = "{attempt_id}";
    pub const CONNECTOR: &'static str = "{connector}";
    pub const CREDS_IDENTIFIER: &'static str = "{creds_identifier}";

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

    pub fn path(self) -> String {
        match self {
            Self::Start => format!(
                "/redirect/{}/{}/{}",
                Self::PAYMENT_ID,
                Self::MERCHANT_ID,
                Self::ATTEMPT_ID,
            ),
            Self::RedirectResponse => format!(
                "/{}/{}/redirect/response/{}",
                Self::PAYMENT_ID,
                Self::MERCHANT_ID,
                Self::CONNECTOR,
            ),
            Self::RedirectResponseWithCredentials => format!(
                "/{}/{}/redirect/response/{}/{}",
                Self::PAYMENT_ID,
                Self::MERCHANT_ID,
                Self::CONNECTOR,
                Self::CREDS_IDENTIFIER,
            ),
            Self::RedirectComplete => format!(
                "/{}/{}/redirect/complete/{}",
                Self::PAYMENT_ID,
                Self::MERCHANT_ID,
                Self::CONNECTOR,
            ),
            Self::RedirectCompleteWithCredentials => format!(
                "/{}/{}/redirect/complete/{}/{}",
                Self::PAYMENT_ID,
                Self::MERCHANT_ID,
                Self::CONNECTOR,
                Self::CREDS_IDENTIFIER,
            ),
            Self::Authorize => format!(
                "/{}/{}/authorize/{}",
                Self::PAYMENT_ID,
                Self::MERCHANT_ID,
                Self::CONNECTOR,
            ),
            Self::Authentication => format!("/{}/3ds/authentication", Self::PAYMENT_ID),
        }
    }

    fn url(self, base_url: &str) -> String {
        format!("{base_url}/payments{}", self.path())
    }

    pub fn start_url(
        base_url: &str,
        payment_id: &str,
        merchant_id: &str,
        attempt_id: &str,
    ) -> String {
        Self::Start
            .url(base_url)
            .replace(Self::PAYMENT_ID, payment_id)
            .replace(Self::MERCHANT_ID, merchant_id)
            .replace(Self::ATTEMPT_ID, attempt_id)
    }

    pub fn redirect_response_url(
        base_url: &str,
        payment_id: &str,
        merchant_id: &str,
        connector: &str,
    ) -> String {
        Self::RedirectResponse
            .url(base_url)
            .replace(Self::PAYMENT_ID, payment_id)
            .replace(Self::MERCHANT_ID, merchant_id)
            .replace(Self::CONNECTOR, connector)
    }

    pub fn redirect_response_with_credentials_url(
        base_url: &str,
        payment_id: &str,
        merchant_id: &str,
        connector: &str,
        creds_identifier: &str,
    ) -> String {
        Self::RedirectResponseWithCredentials
            .url(base_url)
            .replace(Self::PAYMENT_ID, payment_id)
            .replace(Self::MERCHANT_ID, merchant_id)
            .replace(Self::CONNECTOR, connector)
            .replace(Self::CREDS_IDENTIFIER, creds_identifier)
    }

    pub fn redirect_complete_url(
        base_url: &str,
        payment_id: &str,
        merchant_id: &str,
        connector: &str,
    ) -> String {
        Self::RedirectComplete
            .url(base_url)
            .replace(Self::PAYMENT_ID, payment_id)
            .replace(Self::MERCHANT_ID, merchant_id)
            .replace(Self::CONNECTOR, connector)
    }

    pub fn redirect_complete_with_credentials_url(
        base_url: &str,
        payment_id: &str,
        merchant_id: &str,
        connector: &str,
        creds_identifier: &str,
    ) -> String {
        Self::RedirectCompleteWithCredentials
            .url(base_url)
            .replace(Self::PAYMENT_ID, payment_id)
            .replace(Self::MERCHANT_ID, merchant_id)
            .replace(Self::CONNECTOR, connector)
            .replace(Self::CREDS_IDENTIFIER, creds_identifier)
    }

    pub fn authorize_url(
        base_url: &str,
        payment_id: &str,
        merchant_id: &str,
        connector: &str,
    ) -> String {
        Self::Authorize
            .url(base_url)
            .replace(Self::PAYMENT_ID, payment_id)
            .replace(Self::MERCHANT_ID, merchant_id)
            .replace(Self::CONNECTOR, connector)
    }

    pub fn authentication_url(base_url: &str, payment_id: &str) -> String {
        Self::Authentication
            .url(base_url)
            .replace(Self::PAYMENT_ID, payment_id)
    }

    /// Build the named resource with the methods and handlers for this callback.
    #[cfg(feature = "v1")]
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
