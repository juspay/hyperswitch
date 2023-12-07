use common_utils::errors::CustomResult;
use error_stack::ResultExt;
use external_services::email::{EmailContents, EmailData, EmailError};
use masking::ExposeInterface;

use crate::{configs, consts};
#[cfg(feature = "olap")]
use crate::{core::errors::UserErrors, services::jwt, types::domain};

pub enum EmailBody {
    Verify { link: String },
    Reset { link: String, user_name: String },
    MagicLink { link: String, user_name: String },
    InviteUser { link: String, user_name: String },
}

pub mod html {
    use crate::services::email::types::EmailBody;

    pub fn get_html_body(email_body: EmailBody) -> String {
        match email_body {
            EmailBody::Verify { link } => {
                format!(include_str!("assets/verify.html"), link = link)
            }
            EmailBody::Reset { link, user_name } => {
                format!(
                    include_str!("assets/reset.html"),
                    link = link,
                    username = user_name
                )
            }
            EmailBody::MagicLink { link, user_name } => {
                format!(
                    include_str!("assets/magic_link.html"),
                    user_name = user_name,
                    link = link
                )
            }
            EmailBody::InviteUser { link, user_name } => {
                format!(
                    include_str!("assets/invite.html"),
                    username = user_name,
                    link = link
                )
            }
        }
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct EmailToken {
    email: String,
    exp: u64,
}

impl EmailToken {
    pub async fn new_token(
        email: domain::UserEmail,
        settings: &configs::settings::Settings,
    ) -> CustomResult<String, UserErrors> {
        let expiration_duration = std::time::Duration::from_secs(consts::EMAIL_TOKEN_TIME_IN_SECS);
        let exp = jwt::generate_exp(expiration_duration)?.as_secs();
        let token_payload = Self {
            email: email.get_secret().expose(),
            exp,
        };
        jwt::generate_jwt(&token_payload, settings).await
    }

    pub fn get_email(&self) -> &str {
        self.email.as_str()
    }
}

pub fn get_link_with_token(
    base_url: impl std::fmt::Display,
    token: impl std::fmt::Display,
    action: impl std::fmt::Display,
) -> String {
    format!("{base_url}/user/{action}/?token={token}")
}

pub struct VerifyEmail {
    pub recipient_email: domain::UserEmail,
    pub settings: std::sync::Arc<configs::settings::Settings>,
    pub subject: &'static str,
}

/// Currently only HTML is supported
#[async_trait::async_trait]
impl EmailData for VerifyEmail {
    async fn get_email_data(&self) -> CustomResult<EmailContents, EmailError> {
        let token = EmailToken::new_token(self.recipient_email.clone(), &self.settings)
            .await
            .change_context(EmailError::TokenGenerationFailure)?;

        let verify_email_link =
            get_link_with_token(&self.settings.email.base_url, token, "verify_email");

        let body = html::get_html_body(EmailBody::Verify {
            link: verify_email_link,
        });

        Ok(EmailContents {
            subject: self.subject.to_string(),
            body: external_services::email::IntermediateString::new(body),
            recipient: self.recipient_email.clone().into_inner(),
        })
    }
}

pub struct ResetPassword {
    pub recipient_email: domain::UserEmail,
    pub user_name: domain::UserName,
    pub settings: std::sync::Arc<configs::settings::Settings>,
    pub subject: &'static str,
}

#[async_trait::async_trait]
impl EmailData for ResetPassword {
    async fn get_email_data(&self) -> CustomResult<EmailContents, EmailError> {
        let token = EmailToken::new_token(self.recipient_email.clone(), &self.settings)
            .await
            .change_context(EmailError::TokenGenerationFailure)?;

        let reset_password_link =
            get_link_with_token(&self.settings.email.base_url, token, "set_password");

        let body = html::get_html_body(EmailBody::Reset {
            link: reset_password_link,
            user_name: self.user_name.clone().get_secret().expose(),
        });

        Ok(EmailContents {
            subject: self.subject.to_string(),
            body: external_services::email::IntermediateString::new(body),
            recipient: self.recipient_email.clone().into_inner(),
        })
    }
}

pub struct MagicLink {
    pub recipient_email: domain::UserEmail,
    pub user_name: domain::UserName,
    pub settings: std::sync::Arc<configs::settings::Settings>,
    pub subject: &'static str,
}

#[async_trait::async_trait]
impl EmailData for MagicLink {
    async fn get_email_data(&self) -> CustomResult<EmailContents, EmailError> {
        let token = EmailToken::new_token(self.recipient_email.clone(), &self.settings)
            .await
            .change_context(EmailError::TokenGenerationFailure)?;

        let magic_link_login = get_link_with_token(&self.settings.email.base_url, token, "login");

        let body = html::get_html_body(EmailBody::MagicLink {
            link: magic_link_login,
            user_name: self.user_name.clone().get_secret().expose(),
        });

        Ok(EmailContents {
            subject: self.subject.to_string(),
            body: external_services::email::IntermediateString::new(body),
            recipient: self.recipient_email.clone().into_inner(),
        })
    }
}

pub struct InviteUser {
    pub recipient_email: domain::UserEmail,
    pub user_name: domain::UserName,
    pub settings: std::sync::Arc<configs::settings::Settings>,
    pub subject: &'static str,
}

#[async_trait::async_trait]
impl EmailData for InviteUser {
    async fn get_email_data(&self) -> CustomResult<EmailContents, EmailError> {
        let token = EmailToken::new_token(self.recipient_email.clone(), &self.settings)
            .await
            .change_context(EmailError::TokenGenerationFailure)?;

        let invite_user_link =
            get_link_with_token(&self.settings.email.base_url, token, "set_password");

        let body = html::get_html_body(EmailBody::MagicLink {
            link: invite_user_link,
            user_name: self.user_name.clone().get_secret().expose(),
        });

        Ok(EmailContents {
            subject: self.subject.to_string(),
            body: external_services::email::IntermediateString::new(body),
            recipient: self.recipient_email.clone().into_inner(),
        })
    }
}
