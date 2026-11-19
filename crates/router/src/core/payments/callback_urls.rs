//! Collection and persistence of payment callback URLs for connectors and the SDK.

use hyperswitch_domain_models::router_response_types::RedirectCallback;
use hyperswitch_masking::Secret;

use crate::{
    core::{configs::payment_callback::PaymentCallbackMode, errors::RouterResult},
    routes::{
        payments::{
            callback_routes::{
                CallbackPaymentId, ConnectorCallbackPath, ConnectorCallbackWithCredentialsPath,
                PaymentCallbackRoute, StartCallbackPath,
            },
            callback_tokens::PaymentCallbackUrl,
        },
        SessionState,
    },
};

/// Connector URLs created during router-data construction, pending token registration.
#[derive(Clone, Default)]
pub struct ConnectorCallbackUrls {
    pub redirect_response: Option<PaymentCallbackUrl>,
    pub redirect_complete: Option<PaymentCallbackUrl>,
    pub notification: Option<PaymentCallbackUrl>,
}

/// SDK URLs returned by response generation, pending token registration.
#[derive(Clone, Default)]
pub struct SdkCallbackUrls {
    pub start: Option<PaymentCallbackUrl>,
    pub authorize: Option<PaymentCallbackUrl>,
    pub redirect_response: Option<PaymentCallbackUrl>,
    pub redirect_complete: Option<PaymentCallbackUrl>,
}

impl SdkCallbackUrls {
    pub fn redirection_form_callback_url(
        &mut self,
        callback: Option<RedirectCallback>,
        base_url: &str,
        callback_mode: PaymentCallbackMode,
        path: &ConnectorCallbackPath,
        creds_identifier: Option<&str>,
    ) -> Option<&Secret<String>> {
        Some(match callback? {
            RedirectCallback::Response => self.redirect_response_url(
                base_url,
                callback_mode,
                &path.payment_id,
                &path.merchant_id,
                &path.connector,
                None,
            ),
            RedirectCallback::Complete => self.redirect_complete_url(
                base_url,
                callback_mode,
                &path.payment_id,
                &path.merchant_id,
                &path.connector,
                creds_identifier,
            ),
        })
    }

    pub fn start_url(
        &mut self,
        base_url: &str,
        callback_mode: PaymentCallbackMode,
        payment_id: &CallbackPaymentId,
        merchant_id: &common_utils::id_type::MerchantId,
        attempt_id: &str,
    ) -> &Secret<String> {
        self.start
            .get_or_insert_with(|| {
                PaymentCallbackRoute::start_url(
                    base_url,
                    callback_mode,
                    &StartCallbackPath {
                        payment_id: payment_id.clone(),
                        merchant_id: merchant_id.clone(),
                        attempt_id: attempt_id.to_string(),
                    },
                )
            })
            .url()
    }

    pub fn authorize_url(
        &mut self,
        base_url: &str,
        callback_mode: PaymentCallbackMode,
        payment_id: &CallbackPaymentId,
        merchant_id: &common_utils::id_type::MerchantId,
        connector: &str,
    ) -> &Secret<String> {
        self.authorize
            .get_or_insert_with(|| {
                PaymentCallbackRoute::authorize_url(
                    base_url,
                    callback_mode,
                    &ConnectorCallbackPath {
                        payment_id: payment_id.clone(),
                        merchant_id: merchant_id.clone(),
                        connector: connector.to_string(),
                    },
                )
            })
            .url()
    }

    pub fn redirect_response_url(
        &mut self,
        base_url: &str,
        callback_mode: PaymentCallbackMode,
        payment_id: &CallbackPaymentId,
        merchant_id: &common_utils::id_type::MerchantId,
        connector: &str,
        creds_identifier: Option<&str>,
    ) -> &Secret<String> {
        self.redirect_response
            .get_or_insert_with(|| match creds_identifier {
                Some(creds) => PaymentCallbackRoute::redirect_response_with_credentials_url(
                    base_url,
                    callback_mode,
                    &ConnectorCallbackWithCredentialsPath {
                        payment_id: payment_id.clone(),
                        merchant_id: merchant_id.clone(),
                        connector: connector.to_string(),
                        creds_identifier: creds.to_string(),
                    },
                ),
                None => PaymentCallbackRoute::redirect_response_url(
                    base_url,
                    callback_mode,
                    &ConnectorCallbackPath {
                        payment_id: payment_id.clone(),
                        merchant_id: merchant_id.clone(),
                        connector: connector.to_string(),
                    },
                ),
            })
            .url()
    }

    pub fn redirect_complete_url(
        &mut self,
        base_url: &str,
        callback_mode: PaymentCallbackMode,
        payment_id: &CallbackPaymentId,
        merchant_id: &common_utils::id_type::MerchantId,
        connector: &str,
        creds_identifier: Option<&str>,
    ) -> &Secret<String> {
        self.redirect_complete
            .get_or_insert_with(|| match creds_identifier {
                Some(creds) => PaymentCallbackRoute::redirect_complete_with_credentials_url(
                    base_url,
                    callback_mode,
                    &ConnectorCallbackWithCredentialsPath {
                        payment_id: payment_id.clone(),
                        merchant_id: merchant_id.clone(),
                        connector: connector.to_string(),
                        creds_identifier: creds.to_string(),
                    },
                ),
                None => PaymentCallbackRoute::redirect_complete_url(
                    base_url,
                    callback_mode,
                    &ConnectorCallbackPath {
                        payment_id: payment_id.clone(),
                        merchant_id: merchant_id.clone(),
                        connector: connector.to_string(),
                    },
                ),
            })
            .url()
    }

    pub async fn persist(self, state: &SessionState) -> RouterResult<()> {
        let Self {
            start,
            authorize,
            redirect_response,
            redirect_complete,
        } = self;

        futures::future::try_join_all(
            [start, authorize, redirect_response, redirect_complete]
                .into_iter()
                .flatten()
                .map(|callback_url| callback_url.persist(state)),
        )
        .await?;
        Ok(())
    }
}

impl ConnectorCallbackUrls {
    /// Notifications must not consume the token used by the customer's browser.
    pub fn notification_url(
        &mut self,
        callback: Option<RedirectCallback>,
        base_url: &str,
        callback_mode: PaymentCallbackMode,
        path: &ConnectorCallbackPath,
        creds_identifier: Option<&str>,
    ) -> Option<Secret<String>> {
        let callback = callback?;
        let notification = self.notification.get_or_insert_with(|| {
            let credentials_path =
                creds_identifier.map(|creds| ConnectorCallbackWithCredentialsPath {
                    payment_id: path.payment_id.clone(),
                    merchant_id: path.merchant_id.clone(),
                    connector: path.connector.clone(),
                    creds_identifier: creds.to_string(),
                });
            match (callback, credentials_path.as_ref()) {
                (RedirectCallback::Response, None) => {
                    PaymentCallbackRoute::redirect_response_url(base_url, callback_mode, path)
                }
                (RedirectCallback::Complete, None) => {
                    PaymentCallbackRoute::redirect_complete_url(base_url, callback_mode, path)
                }
                (RedirectCallback::Response, Some(path)) => {
                    PaymentCallbackRoute::redirect_response_with_credentials_url(
                        base_url,
                        callback_mode,
                        path,
                    )
                }
                (RedirectCallback::Complete, Some(path)) => {
                    PaymentCallbackRoute::redirect_complete_with_credentials_url(
                        base_url,
                        callback_mode,
                        path,
                    )
                }
            }
        });
        Some(notification.url().clone())
    }

    pub fn redirect_response_url(
        &mut self,
        base_url: &str,
        callback_mode: PaymentCallbackMode,
        payment_id: &CallbackPaymentId,
        merchant_id: &common_utils::id_type::MerchantId,
        connector: &str,
        creds_identifier: Option<&str>,
    ) -> &Secret<String> {
        self.redirect_response
            .get_or_insert_with(|| match creds_identifier {
                Some(creds) => PaymentCallbackRoute::redirect_response_with_credentials_url(
                    base_url,
                    callback_mode,
                    &ConnectorCallbackWithCredentialsPath {
                        payment_id: payment_id.clone(),
                        merchant_id: merchant_id.clone(),
                        connector: connector.to_string(),
                        creds_identifier: creds.to_string(),
                    },
                ),
                None => PaymentCallbackRoute::redirect_response_url(
                    base_url,
                    callback_mode,
                    &ConnectorCallbackPath {
                        payment_id: payment_id.clone(),
                        merchant_id: merchant_id.clone(),
                        connector: connector.to_string(),
                    },
                ),
            })
            .url()
    }

    pub fn redirect_complete_url(
        &mut self,
        base_url: &str,
        callback_mode: PaymentCallbackMode,
        payment_id: &CallbackPaymentId,
        merchant_id: &common_utils::id_type::MerchantId,
        connector: &str,
        creds_identifier: Option<&str>,
    ) -> &Secret<String> {
        self.redirect_complete
            .get_or_insert_with(|| match creds_identifier {
                Some(creds) => PaymentCallbackRoute::redirect_complete_with_credentials_url(
                    base_url,
                    callback_mode,
                    &ConnectorCallbackWithCredentialsPath {
                        payment_id: payment_id.clone(),
                        merchant_id: merchant_id.clone(),
                        connector: connector.to_string(),
                        creds_identifier: creds.to_string(),
                    },
                ),
                None => PaymentCallbackRoute::redirect_complete_url(
                    base_url,
                    callback_mode,
                    &ConnectorCallbackPath {
                        payment_id: payment_id.clone(),
                        merchant_id: merchant_id.clone(),
                        connector: connector.to_string(),
                    },
                ),
            })
            .url()
    }

    pub async fn persist(self, state: &SessionState) -> RouterResult<()> {
        let Self {
            redirect_response,
            redirect_complete,
            notification,
        } = self;

        futures::future::try_join_all(
            [redirect_response, redirect_complete, notification]
                .into_iter()
                .flatten()
                .map(|callback_url| callback_url.persist(state)),
        )
        .await?;
        Ok(())
    }
}
