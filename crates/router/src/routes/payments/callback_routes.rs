//! Named payment callbacks and their URL path constructors.
//!
//! URL constructors include the base URL and `/payments` scope. Actix registration
//! iterates every variant, so adding a route requires a matching path and handler
//! at compile time.

#[cfg(feature = "v1")]
use actix_web::{web, Resource};

#[cfg(feature = "v1")]
use super as payments;
use super::callback_tokens::{self, PaymentCallbackFields, PaymentCallbackUrl};
use crate::configs::settings::PaymentCallbackMode;

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
            Self::Authentication => "payment_callback_authentication",
        }
    }

    pub fn path(self) -> String {
        match self {
            Self::Authentication => self.legacy_path(),
            _ => Self::with_callback_token(&self.legacy_path(), Self::CALLBACK_TOKEN),
        }
    }

    pub(super) fn with_callback_token(path: &str, token: &str) -> String {
        format!("{path}/callback/{token}")
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
            Self::Authentication => format!("/{}/3ds/authentication", Self::PAYMENT_ID),
        }
    }

    fn url(self, base_url: &str) -> String {
        format!("{base_url}/payments{}", self.path())
    }

    pub fn start_url(
        base_url: &str,
        callback_mode: PaymentCallbackMode,
        fields: &StartCallbackPath,
    ) -> PaymentCallbackUrl {
        let path = Self::Start
            .legacy_path()
            .replace(Self::PAYMENT_ID, fields.payment_id.get_string_repr())
            .replace(Self::MERCHANT_ID, fields.merchant_id.get_string_repr())
            .replace(Self::ATTEMPT_ID, &fields.attempt_id);
        callback_tokens::create_url(
            base_url,
            PaymentCallbackFields::Start(fields),
            path,
            callback_mode,
        )
    }

    pub fn redirect_response_url(
        base_url: &str,
        callback_mode: PaymentCallbackMode,
        fields: &ConnectorCallbackPath,
    ) -> PaymentCallbackUrl {
        let path = Self::RedirectResponse
            .legacy_path()
            .replace(Self::PAYMENT_ID, fields.payment_id.get_string_repr())
            .replace(Self::MERCHANT_ID, fields.merchant_id.get_string_repr())
            .replace(Self::CONNECTOR, &fields.connector);
        callback_tokens::create_url(
            base_url,
            PaymentCallbackFields::RedirectResponse(fields),
            path,
            callback_mode,
        )
    }

    pub fn redirect_response_with_credentials_url(
        base_url: &str,
        callback_mode: PaymentCallbackMode,
        fields: &ConnectorCallbackWithCredentialsPath,
    ) -> PaymentCallbackUrl {
        let path = Self::RedirectResponseWithCredentials
            .legacy_path()
            .replace(Self::PAYMENT_ID, fields.payment_id.get_string_repr())
            .replace(Self::MERCHANT_ID, fields.merchant_id.get_string_repr())
            .replace(Self::CONNECTOR, &fields.connector)
            .replace(Self::CREDS_IDENTIFIER, &fields.creds_identifier);
        callback_tokens::create_url(
            base_url,
            PaymentCallbackFields::RedirectResponseWithCredentials(fields),
            path,
            callback_mode,
        )
    }

    pub fn redirect_complete_url(
        base_url: &str,
        callback_mode: PaymentCallbackMode,
        fields: &ConnectorCallbackPath,
    ) -> PaymentCallbackUrl {
        let path = Self::RedirectComplete
            .legacy_path()
            .replace(Self::PAYMENT_ID, fields.payment_id.get_string_repr())
            .replace(Self::MERCHANT_ID, fields.merchant_id.get_string_repr())
            .replace(Self::CONNECTOR, &fields.connector);
        callback_tokens::create_url(
            base_url,
            PaymentCallbackFields::RedirectComplete(fields),
            path,
            callback_mode,
        )
    }

    pub fn redirect_complete_with_credentials_url(
        base_url: &str,
        callback_mode: PaymentCallbackMode,
        fields: &ConnectorCallbackWithCredentialsPath,
    ) -> PaymentCallbackUrl {
        let path = Self::RedirectCompleteWithCredentials
            .legacy_path()
            .replace(Self::PAYMENT_ID, fields.payment_id.get_string_repr())
            .replace(Self::MERCHANT_ID, fields.merchant_id.get_string_repr())
            .replace(Self::CONNECTOR, &fields.connector)
            .replace(Self::CREDS_IDENTIFIER, &fields.creds_identifier);
        callback_tokens::create_url(
            base_url,
            PaymentCallbackFields::RedirectCompleteWithCredentials(fields),
            path,
            callback_mode,
        )
    }

    pub fn authorize_url(
        base_url: &str,
        callback_mode: PaymentCallbackMode,
        fields: &ConnectorCallbackPath,
    ) -> PaymentCallbackUrl {
        let path = Self::Authorize
            .legacy_path()
            .replace(Self::PAYMENT_ID, fields.payment_id.get_string_repr())
            .replace(Self::MERCHANT_ID, fields.merchant_id.get_string_repr())
            .replace(Self::CONNECTOR, &fields.connector);
        callback_tokens::create_url(
            base_url,
            PaymentCallbackFields::Authorize(fields),
            path,
            callback_mode,
        )
    }

    pub fn authentication_url(base_url: &str, payment_id: &str) -> String {
        Self::Authentication
            .url(base_url)
            .replace(Self::PAYMENT_ID, payment_id)
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
            Self::Authentication => {
                resource.route(web::post().to(payments::payments_external_authentication))
            }
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
