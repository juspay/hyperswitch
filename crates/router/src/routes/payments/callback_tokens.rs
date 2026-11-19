//! Registration and atomic consumption of single-use payment callback URLs.

use error_stack::ResultExt;
#[cfg(all(test, feature = "v1"))]
use hyperswitch_masking::PeekInterface;
use hyperswitch_masking::Secret;
use sha2::{Digest, Sha256};
use uuid::Uuid;

use super::callback_routes::{
    ConnectorCallbackPath, ConnectorCallbackWithCredentialsPath, PaymentCallbackRoute,
    StartCallbackPath,
};
#[cfg(feature = "v1")]
use crate::services::authentication::{
    AuthenticateAndFetch, AuthenticationData, AuthenticationType, MerchantIdAuth,
};
use crate::{
    core::{
        configs::payment_callback::PaymentCallbackMode,
        errors::{self, RouterResult},
    },
    routes::SessionState,
};
const UNUSED: &str = "unused";

/// Route identity plus its concrete, already-validated path parameters.
#[derive(Clone, Copy)]
pub(super) enum PaymentCallbackFields<'a> {
    Start(&'a StartCallbackPath),
    RedirectResponse(&'a ConnectorCallbackPath),
    RedirectResponseWithCredentials(&'a ConnectorCallbackWithCredentialsPath),
    RedirectComplete(&'a ConnectorCallbackPath),
    RedirectCompleteWithCredentials(&'a ConnectorCallbackWithCredentialsPath),
    Authorize(&'a ConnectorCallbackPath),
}

impl<'a> PaymentCallbackFields<'a> {
    pub(super) fn create_url(
        self,
        base_url: &str,
        callback_mode: PaymentCallbackMode,
    ) -> PaymentCallbackUrl {
        let route = self.route();
        let (path, key) = if callback_mode.issues_tokens() {
            let token = common_utils::generate_uuid_v4();
            let path = route.with_callback_token(&token.to_string());
            let key = self.key(&token);
            (path, Some(key))
        } else {
            (route.legacy_path(), None)
        };
        let path = match self {
            Self::Start(fields) => path
                .replace(
                    PaymentCallbackRoute::PAYMENT_ID,
                    fields.payment_id.get_string_repr(),
                )
                .replace(
                    PaymentCallbackRoute::MERCHANT_ID,
                    fields.merchant_id.get_string_repr(),
                )
                .replace(PaymentCallbackRoute::ATTEMPT_ID, &fields.attempt_id),
            Self::RedirectResponse(fields)
            | Self::RedirectComplete(fields)
            | Self::Authorize(fields) => path
                .replace(
                    PaymentCallbackRoute::PAYMENT_ID,
                    fields.payment_id.get_string_repr(),
                )
                .replace(
                    PaymentCallbackRoute::MERCHANT_ID,
                    fields.merchant_id.get_string_repr(),
                )
                .replace(PaymentCallbackRoute::CONNECTOR, &fields.connector),
            Self::RedirectResponseWithCredentials(fields)
            | Self::RedirectCompleteWithCredentials(fields) => path
                .replace(
                    PaymentCallbackRoute::PAYMENT_ID,
                    fields.payment_id.get_string_repr(),
                )
                .replace(
                    PaymentCallbackRoute::MERCHANT_ID,
                    fields.merchant_id.get_string_repr(),
                )
                .replace(PaymentCallbackRoute::CONNECTOR, &fields.connector)
                .replace(
                    PaymentCallbackRoute::CREDS_IDENTIFIER,
                    &fields.creds_identifier,
                ),
        };
        let path = format!("/payments{path}");
        PaymentCallbackUrl {
            key,
            url: Secret::new(format!("{base_url}{path}")),
        }
    }

    fn route(self) -> PaymentCallbackRoute {
        match self {
            Self::Start(_) => PaymentCallbackRoute::Start,
            Self::RedirectResponse(_) => PaymentCallbackRoute::RedirectResponse,
            Self::RedirectResponseWithCredentials(_) => {
                PaymentCallbackRoute::RedirectResponseWithCredentials
            }
            Self::RedirectComplete(_) => PaymentCallbackRoute::RedirectComplete,
            Self::RedirectCompleteWithCredentials(_) => {
                PaymentCallbackRoute::RedirectCompleteWithCredentials
            }
            Self::Authorize(_) => PaymentCallbackRoute::Authorize,
        }
    }

    fn key(self, token: &Uuid) -> redis_interface::RedisKey {
        let route = self.route();
        match self {
            Self::Start(path) => path.callback_key(route, token),
            Self::RedirectResponse(path) | Self::RedirectComplete(path) | Self::Authorize(path) => {
                path.callback_key(route, token)
            }
            Self::RedirectResponseWithCredentials(path)
            | Self::RedirectCompleteWithCredentials(path) => path.callback_key(route, token),
        }
    }

    #[cfg(feature = "v1")]
    fn merchant_id(self) -> &'a common_utils::id_type::MerchantId {
        match self {
            Self::Start(path) => &path.merchant_id,
            Self::RedirectResponse(path) | Self::RedirectComplete(path) | Self::Authorize(path) => {
                &path.merchant_id
            }
            Self::RedirectResponseWithCredentials(path)
            | Self::RedirectCompleteWithCredentials(path) => &path.merchant_id,
        }
    }
}

trait CallbackHashInput {
    fn hash_input(&self) -> String;

    // Hash the bearer credential so Redis keys and command instrumentation do not
    // contain usable tokens. RedisConnectionWithContext applies the tenant prefix.
    fn callback_key(&self, route: PaymentCallbackRoute, token: &Uuid) -> redis_interface::RedisKey {
        let mut digest = Sha256::new();
        digest.update(Self::concatenate_fields(&[route.name()]));
        digest.update(self.hash_input());
        digest.update(token.as_bytes());
        format!("payment_callback_token:{:x}", digest.finalize()).into()
    }

    // Byte-length framing distinguishes values even when they contain delimiters.
    fn concatenate_fields(fields: &[&str]) -> String {
        let mut input = String::new();
        for field in fields {
            input.push_str(&field.len().to_string());
            input.push(':');
            input.push_str(field);
        }
        input
    }
}

impl CallbackHashInput for StartCallbackPath {
    fn hash_input(&self) -> String {
        Self::concatenate_fields(&[
            self.payment_id.get_string_repr(),
            self.merchant_id.get_string_repr(),
            &self.attempt_id,
        ])
    }
}

impl CallbackHashInput for ConnectorCallbackPath {
    fn hash_input(&self) -> String {
        Self::concatenate_fields(&[
            self.payment_id.get_string_repr(),
            self.merchant_id.get_string_repr(),
            &self.connector,
        ])
    }
}

impl CallbackHashInput for ConnectorCallbackWithCredentialsPath {
    fn hash_input(&self) -> String {
        Self::concatenate_fields(&[
            self.payment_id.get_string_repr(),
            self.merchant_id.get_string_repr(),
            &self.connector,
            &self.creds_identifier,
        ])
    }
}

/// A callback URL and its optional pending Redis registration.
#[derive(Clone)]
pub struct PaymentCallbackUrl {
    url: Secret<String>,
    key: Option<redis_interface::RedisKey>,
}

impl PaymentCallbackUrl {
    pub async fn persist(self, state: &SessionState) -> RouterResult<Secret<String>> {
        if let Some(key) = self.key {
            let ttl = common_utils::consts::DEFAULT_INTENT_FULFILLMENT_TIME;
            let connection = state
                .store
                .get_redis_conn()
                .change_context(errors::ApiErrorResponse::InternalServerError)?;
            let result = connection
                .set_key_if_not_exists_with_expiry(&key, UNUSED, Some(ttl))
                .await
                .change_context(errors::ApiErrorResponse::InternalServerError)?;
            if !matches!(result, redis_interface::SetnxReply::KeySet) {
                return Err(errors::ApiErrorResponse::InternalServerError.into());
            }
        }
        Ok(self.url)
    }

    pub fn url(&self) -> &Secret<String> {
        &self.url
    }
}

/// Opt-in authentication for protected callback handlers. Consume the token
/// after merchant authentication succeeds and before server_wrap takes a lock.
/// Original URLs are accepted only in legacy and migration modes.
#[cfg(feature = "v1")]
pub struct PaymentCallbackAuth {
    merchant_auth: MerchantIdAuth,
    key: Option<redis_interface::RedisKey>,
}

#[cfg(feature = "v1")]
impl PaymentCallbackAuth {
    fn parse_token(value: &str) -> RouterResult<Uuid> {
        let token =
            Uuid::parse_str(value).change_context(errors::ApiErrorResponse::Unauthorized)?;
        if token.get_version_num() != 4 || token.to_string() != value {
            return Err(errors::ApiErrorResponse::Unauthorized.into());
        }
        Ok(token)
    }

    fn request_key(
        fields: PaymentCallbackFields<'_>,
        request: &actix_web::HttpRequest,
    ) -> RouterResult<Option<redis_interface::RedisKey>> {
        let route = fields.route();
        match request.match_name() {
            Some(name) if name == route.name() => (),
            Some(name) if name == route.legacy_name() => return Ok(None),
            _ => return Err(errors::ApiErrorResponse::Unauthorized.into()),
        }
        let token = Self::parse_token(
            request
                .match_info()
                .get("callback_token")
                .ok_or(errors::ApiErrorResponse::Unauthorized)?,
        )?;
        Ok(Some(fields.key(&token)))
    }

    pub(super) fn from_request(
        fields: PaymentCallbackFields<'_>,
        request: &actix_web::HttpRequest,
    ) -> RouterResult<Self> {
        Ok(Self {
            merchant_auth: MerchantIdAuth(fields.merchant_id().clone()),
            key: Self::request_key(fields, request)?,
        })
    }

    fn validate_mode(&self, mode: PaymentCallbackMode) -> RouterResult<()> {
        if self.key.is_none() && !mode.allows_legacy() {
            return Err(errors::ApiErrorResponse::Unauthorized.into());
        }
        Ok(())
    }
}

#[cfg(feature = "v1")]
#[async_trait::async_trait]
impl AuthenticateAndFetch<AuthenticationData, SessionState> for PaymentCallbackAuth {
    async fn authenticate_and_fetch(
        &self,
        request_headers: &actix_web::http::header::HeaderMap,
        state: &SessionState,
    ) -> RouterResult<(AuthenticationData, AuthenticationType)> {
        self.validate_mode(state.payment_callback_config().await?.mode)?;
        let auth = self
            .merchant_auth
            .authenticate_and_fetch(request_headers, state)
            .await?;
        let Some(key) = self.key.as_ref() else {
            return Ok(auth);
        };
        let connection = state
            .store
            .get_redis_conn()
            .change_context(errors::ApiErrorResponse::InternalServerError)?;
        let value = connection
            .consume_key(key)
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)?;
        if value.as_deref() != Some(UNUSED) {
            return Err(errors::ApiErrorResponse::Unauthorized.into());
        }
        Ok(auth)
    }
}

#[cfg(test)]
#[cfg(feature = "v1")]
mod tests;
