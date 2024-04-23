use api_models::user::dashboard_metadata::ProdIntent;
use common_utils::{
    errors::{self, CustomResult},
    pii,
};
use error_stack::ResultExt;
use external_services::email::{EmailContents, EmailData, EmailError};
use masking::{ExposeInterface, PeekInterface, Secret};

use crate::{configs, consts, routes::AppState};
#[cfg(feature = "olap")]
use crate::{
    core::errors::{UserErrors, UserResult},
    services::jwt,
    types::domain,
};

pub enum EmailBody {
    Verify {
        link: String,
    },
    Reset {
        link: String,
        user_name: String,
    },
    MagicLink {
        link: String,
        user_name: String,
    },
    InviteUser {
        link: String,
        user_name: String,
    },
    AcceptInviteFromEmail {
        link: String,
        user_name: String,
    },
    BizEmailProd {
        user_name: String,
        poc_email: String,
        legal_business_name: String,
        business_location: String,
        business_website: String,
    },
    ReconActivation {
        user_name: String,
    },
    ProFeatureRequest {
        feature_name: String,
        merchant_id: String,
        user_name: String,
        user_email: String,
    },
    ApiKeyExpiryReminder {
        expires_in: u8,
        api_key_name: String,
        prefix: String,
    },
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
            // TODO: Change the linked html for accept invite from email
            EmailBody::AcceptInviteFromEmail { link, user_name } => {
                format!(
                    include_str!("assets/invite.html"),
                    username = user_name,
                    link = link
                )
            }
            EmailBody::ReconActivation { user_name } => {
                format!(
                    include_str!("assets/recon_activation.html"),
                    username = user_name,
                )
            }
            EmailBody::BizEmailProd {
                user_name,
                poc_email,
                legal_business_name,
                business_location,
                business_website,
            } => {
                format!(
                    include_str!("assets/bizemailprod.html"),
                    poc_email = poc_email,
                    legal_business_name = legal_business_name,
                    business_location = business_location,
                    business_website = business_website,
                    username = user_name,
                )
            }
            EmailBody::ProFeatureRequest {
                feature_name,
                merchant_id,
                user_name,
                user_email,
            } => format!(
                "Dear Hyperswitch Support Team,

Dashboard Pro Feature Request,
Feature name  : {feature_name}
Merchant ID   : {merchant_id}
Merchant Name : {user_name}
Email         : {user_email}

(note: This is an auto generated email. Use merchant email for any further communications)",
            ),
            EmailBody::ApiKeyExpiryReminder {
                expires_in,
                api_key_name,
                prefix,
            } => format!(
                include_str!("assets/api_key_expiry_reminder.html"),
                api_key_name = api_key_name,
                prefix = prefix,
                expires_in = expires_in,
            ),
        }
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct EmailToken {
    email: String,
    merchant_id: Option<String>,
    exp: u64,
}

impl EmailToken {
    pub async fn new_token(
        email: domain::UserEmail,
        merchant_id: Option<String>,
        settings: &configs::Settings,
    ) -> CustomResult<String, UserErrors> {
        let expiration_duration = std::time::Duration::from_secs(consts::EMAIL_TOKEN_TIME_IN_SECS);
        let exp = jwt::generate_exp(expiration_duration)?.as_secs();
        let token_payload = Self {
            email: email.get_secret().expose(),
            merchant_id,
            exp,
        };
        jwt::generate_jwt(&token_payload, settings).await
    }

    pub fn get_email(&self) -> CustomResult<pii::Email, errors::ParsingError> {
        pii::Email::try_from(self.email.clone())
    }

    pub fn get_merchant_id(&self) -> Option<&str> {
        self.merchant_id.as_deref()
    }
}

pub fn get_link_with_token(
    base_url: impl std::fmt::Display,
    token: impl std::fmt::Display,
    action: impl std::fmt::Display,
) -> String {
    format!("{base_url}/user/{action}?token={token}")
}

pub struct VerifyEmail {
    pub recipient_email: domain::UserEmail,
    pub settings: std::sync::Arc<configs::Settings>,
    pub subject: &'static str,
}

/// Currently only HTML is supported
#[async_trait::async_trait]
impl EmailData for VerifyEmail {
    async fn get_email_data(&self) -> CustomResult<EmailContents, EmailError> {
        let token = EmailToken::new_token(self.recipient_email.clone(), None, &self.settings)
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
    pub settings: std::sync::Arc<configs::Settings>,
    pub subject: &'static str,
}

#[async_trait::async_trait]
impl EmailData for ResetPassword {
    async fn get_email_data(&self) -> CustomResult<EmailContents, EmailError> {
        let token = EmailToken::new_token(self.recipient_email.clone(), None, &self.settings)
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
    pub settings: std::sync::Arc<configs::Settings>,
    pub subject: &'static str,
}

#[async_trait::async_trait]
impl EmailData for MagicLink {
    async fn get_email_data(&self) -> CustomResult<EmailContents, EmailError> {
        let token = EmailToken::new_token(self.recipient_email.clone(), None, &self.settings)
            .await
            .change_context(EmailError::TokenGenerationFailure)?;

        let magic_link_login =
            get_link_with_token(&self.settings.email.base_url, token, "verify_email");

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
    pub settings: std::sync::Arc<configs::Settings>,
    pub subject: &'static str,
    pub merchant_id: String,
}

#[async_trait::async_trait]
impl EmailData for InviteUser {
    async fn get_email_data(&self) -> CustomResult<EmailContents, EmailError> {
        let token = EmailToken::new_token(
            self.recipient_email.clone(),
            Some(self.merchant_id.clone()),
            &self.settings,
        )
        .await
        .change_context(EmailError::TokenGenerationFailure)?;

        let invite_user_link =
            get_link_with_token(&self.settings.email.base_url, token, "set_password");

        let body = html::get_html_body(EmailBody::InviteUser {
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
pub struct InviteRegisteredUser {
    pub recipient_email: domain::UserEmail,
    pub user_name: domain::UserName,
    pub settings: std::sync::Arc<configs::Settings>,
    pub subject: &'static str,
    pub merchant_id: String,
}

#[async_trait::async_trait]
impl EmailData for InviteRegisteredUser {
    async fn get_email_data(&self) -> CustomResult<EmailContents, EmailError> {
        let token = EmailToken::new_token(
            self.recipient_email.clone(),
            Some(self.merchant_id.clone()),
            &self.settings,
        )
        .await
        .change_context(EmailError::TokenGenerationFailure)?;

        let invite_user_link = get_link_with_token(
            &self.settings.email.base_url,
            token,
            "accept_invite_from_email",
        );
        let body = html::get_html_body(EmailBody::AcceptInviteFromEmail {
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

pub struct ReconActivation {
    pub recipient_email: domain::UserEmail,
    pub user_name: domain::UserName,
    pub settings: std::sync::Arc<configs::Settings>,
    pub subject: &'static str,
}

#[async_trait::async_trait]
impl EmailData for ReconActivation {
    async fn get_email_data(&self) -> CustomResult<EmailContents, EmailError> {
        let body = html::get_html_body(EmailBody::ReconActivation {
            user_name: self.user_name.clone().get_secret().expose(),
        });

        Ok(EmailContents {
            subject: self.subject.to_string(),
            body: external_services::email::IntermediateString::new(body),
            recipient: self.recipient_email.clone().into_inner(),
        })
    }
}

pub struct BizEmailProd {
    pub recipient_email: domain::UserEmail,
    pub user_name: Secret<String>,
    pub poc_email: Secret<String>,
    pub legal_business_name: String,
    pub business_location: String,
    pub business_website: String,
    pub settings: std::sync::Arc<configs::Settings>,
    pub subject: &'static str,
}

impl BizEmailProd {
    pub fn new(state: &AppState, data: ProdIntent) -> UserResult<Self> {
        Ok(Self {
            recipient_email: (domain::UserEmail::new(
                consts::user::BUSINESS_EMAIL.to_string().into(),
            ))?,
            settings: state.conf.clone(),
            subject: "New Prod Intent",
            user_name: data.poc_name.unwrap_or_default().into(),
            poc_email: data.poc_email.unwrap_or_default().into(),
            legal_business_name: data.legal_business_name.unwrap_or_default(),
            business_location: data
                .business_location
                .unwrap_or(common_enums::CountryAlpha2::AD)
                .to_string(),
            business_website: data.business_website.unwrap_or_default(),
        })
    }
}

#[async_trait::async_trait]
impl EmailData for BizEmailProd {
    async fn get_email_data(&self) -> CustomResult<EmailContents, EmailError> {
        let body = html::get_html_body(EmailBody::BizEmailProd {
            user_name: self.user_name.clone().expose(),
            poc_email: self.poc_email.clone().expose(),
            legal_business_name: self.legal_business_name.clone(),
            business_location: self.business_location.clone(),
            business_website: self.business_website.clone(),
        });

        Ok(EmailContents {
            subject: self.subject.to_string(),
            body: external_services::email::IntermediateString::new(body),
            recipient: self.recipient_email.clone().into_inner(),
        })
    }
}

pub struct ProFeatureRequest {
    pub recipient_email: domain::UserEmail,
    pub feature_name: String,
    pub merchant_id: String,
    pub user_name: domain::UserName,
    pub settings: std::sync::Arc<configs::Settings>,
    pub subject: String,
}

#[async_trait::async_trait]
impl EmailData for ProFeatureRequest {
    async fn get_email_data(&self) -> CustomResult<EmailContents, EmailError> {
        let recipient = self.recipient_email.clone().into_inner();

        let body = html::get_html_body(EmailBody::ProFeatureRequest {
            user_name: self.user_name.clone().get_secret().expose(),
            feature_name: self.feature_name.clone(),
            merchant_id: self.merchant_id.clone(),
            user_email: recipient.peek().to_string(),
        });

        Ok(EmailContents {
            subject: self.subject.clone(),
            body: external_services::email::IntermediateString::new(body),
            recipient,
        })
    }
}

pub struct ApiKeyExpiryReminder {
    pub recipient_email: domain::UserEmail,
    pub subject: &'static str,
    pub expires_in: u8,
    pub api_key_name: String,
    pub prefix: String,
}

#[async_trait::async_trait]
impl EmailData for ApiKeyExpiryReminder {
    async fn get_email_data(&self) -> CustomResult<EmailContents, EmailError> {
        let recipient = self.recipient_email.clone().into_inner();

        let body = html::get_html_body(EmailBody::ApiKeyExpiryReminder {
            expires_in: self.expires_in,
            api_key_name: self.api_key_name.clone(),
            prefix: self.prefix.clone(),
        });

        Ok(EmailContents {
            subject: self.subject.to_string(),
            body: external_services::email::IntermediateString::new(body),
            recipient,
        })
    }
}
