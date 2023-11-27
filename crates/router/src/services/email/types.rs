use common_utils::errors::CustomResult;
use error_stack::ResultExt;
use external_services::email::{EmailContents, EmailData, EmailError};
use masking::ExposeInterface;

use crate::{configs, consts};
#[cfg(feature = "olap")]
use crate::{core::errors::UserErrors, services::jwt, types::domain::UserEmail};

pub enum EmailBody {
    Verify { link: String },
}

pub mod html {
    use crate::services::email::types::EmailBody;

    pub fn get_html_body(email_body: EmailBody) -> String {
        match email_body {
            EmailBody::Verify { link } => {
                format!(include_str!("assets/verify.html"), link = link)
            }
        }
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct EmailToken {
    email: String,
    expiration: u64,
}

impl EmailToken {
    pub async fn new_token(
        email: UserEmail,
        settings: &configs::settings::Settings,
    ) -> CustomResult<String, UserErrors> {
        let expiration_duration = std::time::Duration::from_secs(consts::EMAIL_TOKEN_TIME_IN_SECS);
        let expiration = jwt::generate_exp(expiration_duration)?.as_secs();
        let token_payload = Self {
            email: email.get_secret().expose(),
            expiration,
        };
        jwt::generate_jwt(&token_payload, settings).await
    }
}

pub struct WelcomeEmail {
    pub recipient_email: UserEmail,
    pub settings: std::sync::Arc<configs::settings::Settings>,
}

pub fn get_email_verification_link(
    base_url: impl std::fmt::Display,
    token: impl std::fmt::Display,
) -> String {
    format!("{base_url}/user/verify_email/?token={token}")
}

/// Currently only HTML is supported
#[async_trait::async_trait]
impl EmailData for WelcomeEmail {
    async fn get_email_data(&self) -> CustomResult<EmailContents, EmailError> {
        let token = EmailToken::new_token(self.recipient_email.clone(), &self.settings)
            .await
            .change_context(EmailError::TokenGenerationFailure)?;

        let verify_email_link = get_email_verification_link(&self.settings.server.base_url, token);

        let body = html::get_html_body(EmailBody::Verify {
            link: verify_email_link,
        });
        let subject = "Welcome to the Hyperswitch community!".to_string();

        Ok(EmailContents {
            subject,
            body: external_services::email::IntermediateString::new(body),
            recipient: self.recipient_email.clone().into_inner(),
        })
    }
}
