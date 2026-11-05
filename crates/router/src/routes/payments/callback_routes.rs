//! Named payment callbacks and their URL path constructors.
//!
//! URL constructors include the base URL and `/payments` scope. Actix registration
//! iterates every variant, so adding a route requires a matching path and handler
//! at compile time.

#[cfg(feature = "v1")]
use actix_web::{web, Resource};

use super::callback_tokens::{PaymentCallbackFields, PaymentCallbackUrl};
use crate::core::configs::payment_callback::PaymentCallbackMode;
#[cfg(feature = "v1")]
use crate::routes::payments;

#[derive(Clone, Copy, Debug, Eq, PartialEq, strum::EnumIter)]
pub enum PaymentCallbackRoute {
    Start,
    RedirectResponse,
    RedirectResponseWithCredentials,
    RedirectComplete,
    RedirectCompleteWithCredentials,
    Authorize,
}

impl PaymentCallbackRoute {
    pub const CALLBACK_TOKEN: &'static str = "{callback_token}";
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
        }
    }

    pub fn path(self) -> String {
        self.with_callback_token(Self::CALLBACK_TOKEN)
    }

    pub(super) fn with_callback_token(self, token: &str) -> String {
        let prefix = format!(
            "/{}/{}/redirect/token/{token}",
            Self::PAYMENT_ID,
            Self::MERCHANT_ID,
        );
        match self {
            Self::Start => format!("{prefix}/start/{}", Self::ATTEMPT_ID),
            Self::RedirectResponse => format!("{prefix}/response/{}", Self::CONNECTOR),
            Self::RedirectResponseWithCredentials => format!(
                "{prefix}/response/{}/{}",
                Self::CONNECTOR,
                Self::CREDS_IDENTIFIER,
            ),
            Self::RedirectComplete => format!("{prefix}/complete/{}", Self::CONNECTOR),
            Self::RedirectCompleteWithCredentials => format!(
                "{prefix}/complete/{}/{}",
                Self::CONNECTOR,
                Self::CREDS_IDENTIFIER,
            ),
            Self::Authorize => format!("{prefix}/authorize/{}", Self::CONNECTOR),
        }
    }

    pub fn legacy_name(self) -> String {
        format!("{}_legacy", self.name())
    }

    pub fn legacy_path(self) -> String {
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
        }
    }

    pub fn start_url(
        base_url: &str,
        callback_mode: PaymentCallbackMode,
        fields: &StartCallbackPath,
    ) -> PaymentCallbackUrl {
        PaymentCallbackFields::Start(fields).create_url(base_url, callback_mode)
    }

    pub fn redirect_response_url(
        base_url: &str,
        callback_mode: PaymentCallbackMode,
        fields: &ConnectorCallbackPath,
    ) -> PaymentCallbackUrl {
        PaymentCallbackFields::RedirectResponse(fields).create_url(base_url, callback_mode)
    }

    pub fn redirect_response_with_credentials_url(
        base_url: &str,
        callback_mode: PaymentCallbackMode,
        fields: &ConnectorCallbackWithCredentialsPath,
    ) -> PaymentCallbackUrl {
        PaymentCallbackFields::RedirectResponseWithCredentials(fields)
            .create_url(base_url, callback_mode)
    }

    pub fn redirect_complete_url(
        base_url: &str,
        callback_mode: PaymentCallbackMode,
        fields: &ConnectorCallbackPath,
    ) -> PaymentCallbackUrl {
        PaymentCallbackFields::RedirectComplete(fields).create_url(base_url, callback_mode)
    }

    pub fn redirect_complete_with_credentials_url(
        base_url: &str,
        callback_mode: PaymentCallbackMode,
        fields: &ConnectorCallbackWithCredentialsPath,
    ) -> PaymentCallbackUrl {
        PaymentCallbackFields::RedirectCompleteWithCredentials(fields)
            .create_url(base_url, callback_mode)
    }

    pub fn authorize_url(
        base_url: &str,
        callback_mode: PaymentCallbackMode,
        fields: &ConnectorCallbackPath,
    ) -> PaymentCallbackUrl {
        PaymentCallbackFields::Authorize(fields).create_url(base_url, callback_mode)
    }

    /// Build the named resource with the methods and handlers for this callback.
    #[cfg(feature = "v1")]
    pub fn resource(self) -> Resource {
        self.routes(web::resource(self.path()).name(self.name()))
    }

    /// Build the original callback resource for the compatibility rollout.
    #[cfg(feature = "v1")]
    pub fn legacy_resource(self) -> Resource {
        self.routes(web::resource(self.legacy_path()).name(&self.legacy_name()))
    }

    #[cfg(feature = "v1")]
    fn routes(self, resource: Resource) -> Resource {
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
        }
    }
}

// The active API version supplies a concrete payment ID type.
#[cfg(feature = "v1")]
pub(crate) type CallbackPaymentId = common_utils::id_type::PaymentId;
#[cfg(feature = "v2")]
pub(crate) type CallbackPaymentId = common_utils::id_type::GlobalPaymentId;

// Named extractors ignore the callback token; authentication reads it separately.
#[derive(Debug, serde::Deserialize)]
pub struct StartCallbackPath {
    pub payment_id: CallbackPaymentId,
    pub merchant_id: common_utils::id_type::MerchantId,
    pub attempt_id: String,
}

#[derive(Debug, serde::Deserialize)]
pub struct ConnectorCallbackPath {
    pub payment_id: CallbackPaymentId,
    pub merchant_id: common_utils::id_type::MerchantId,
    pub connector: String,
}

#[derive(Debug, serde::Deserialize)]
pub struct ConnectorCallbackWithCredentialsPath {
    pub payment_id: CallbackPaymentId,
    pub merchant_id: common_utils::id_type::MerchantId,
    pub connector: String,
    pub creds_identifier: String,
}
