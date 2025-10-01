use api_models::user::dashboard_metadata::ProdIntent;
use common_enums::{EntityType, MerchantProductType};
use common_utils::{errors::CustomResult, pii, types::user::EmailThemeConfig};
use error_stack::ResultExt;
use external_services::email::{EmailContents, EmailData, EmailError};
use masking::{ExposeInterface, PeekInterface, Secret};

use crate::{configs, consts, routes::SessionState};
#[cfg(feature = "olap")]
use crate::{
    core::errors::{UserErrors, UserResult},
    services::jwt,
    types::domain,
};

pub enum EmailBody {
    Verify {
        link: String,
        entity_name: String,
        entity_logo_url: String,
        primary_color: String,
        background_color: String,
        foreground_color: String,
    },
    Reset {
        link: String,
        user_name: String,
        entity_name: String,
        entity_logo_url: String,
        primary_color: String,
        background_color: String,
        foreground_color: String,
    },
    MagicLink {
        link: String,
        user_name: String,
        entity_name: String,
        entity_logo_url: String,
        primary_color: String,
        background_color: String,
        foreground_color: String,
    },
    InviteUser {
        link: String,
        user_name: String,
        entity_name: String,
        entity_logo_url: String,
        primary_color: String,
        background_color: String,
        foreground_color: String,
    },
    AcceptInviteFromEmail {
        link: String,
        user_name: String,
        entity_name: String,
        entity_logo_url: String,
        primary_color: String,
        background_color: String,
        foreground_color: String,
    },
    BizEmailProd {
        user_name: String,
        poc_email: String,
        legal_business_name: String,
        business_location: String,
        business_website: String,
        product_type: MerchantProductType,
    },
    ReconActivation {
        user_name: String,
    },
    ProFeatureRequest {
        feature_name: String,
        merchant_id: common_utils::id_type::MerchantId,
        user_name: String,
        user_email: String,
    },
    ApiKeyExpiryReminder {
        expires_in: u8,
        api_key_name: String,
        prefix: String,
    },
    WelcomeToCommunity,
}

pub mod html {
    use crate::services::email::types::EmailBody;

    pub fn get_html_body(email_body: EmailBody) -> String {
        match email_body {
            EmailBody::Verify {
                link,
                entity_name,
                entity_logo_url,
                primary_color,
                background_color,
                foreground_color,
            } => {
                format!(
                    include_str!("assets/verify.html"),
                    link = link,
                    entity_name = entity_name,
                    entity_logo_url = entity_logo_url,
                    primary_color = primary_color,
                    background_color = background_color,
                    foreground_color = foreground_color
                )
            }
            EmailBody::Reset {
                link,
                user_name,
                entity_name,
                entity_logo_url,
                primary_color,
                background_color,
                foreground_color,
            } => {
                format!(
                    include_str!("assets/reset.html"),
                    link = link,
                    username = user_name,
                    entity_name = entity_name,
                    entity_logo_url = entity_logo_url,
                    primary_color = primary_color,
                    background_color = background_color,
                    foreground_color = foreground_color
                )
            }
            EmailBody::MagicLink {
                link,
                user_name,
                entity_name,
                entity_logo_url,
                primary_color,
                background_color,
                foreground_color,
            } => {
                format!(
                    include_str!("assets/magic_link.html"),
                    username = user_name,
                    link = link,
                    entity_name = entity_name,
                    entity_logo_url = entity_logo_url,
                    primary_color = primary_color,
                    background_color = background_color,
                    foreground_color = foreground_color
                )
            }

            EmailBody::InviteUser {
                link,
                user_name,
                entity_name,
                entity_logo_url,
                primary_color,
                background_color,
                foreground_color,
            } => {
                format!(
                    include_str!("assets/invite.html"),
                    username = user_name,
                    link = link,
                    entity_name = entity_name,
                    entity_logo_url = entity_logo_url,
                    primary_color = primary_color,
                    background_color = background_color,
                    foreground_color = foreground_color
                )
            }
            // TODO: Change the linked html for accept invite from email
            EmailBody::AcceptInviteFromEmail {
                link,
                user_name,
                entity_name,
                entity_logo_url,
                primary_color,
                background_color,
                foreground_color,
            } => {
                format!(
                    include_str!("assets/invite.html"),
                    username = user_name,
                    link = link,
                    entity_name = entity_name,
                    entity_logo_url = entity_logo_url,
                    primary_color = primary_color,
                    background_color = background_color,
                    foreground_color = foreground_color
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
                product_type,
            } => {
                format!(
                    include_str!("assets/bizemailprod.html"),
                    poc_email = poc_email,
                    legal_business_name = legal_business_name,
                    business_location = business_location,
                    business_website = business_website,
                    username = user_name,
                    product_type = product_type
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
Merchant ID   : {}
Merchant Name : {user_name}
Email         : {user_email}

(note: This is an auto generated email. Use merchant email for any further communications)",
                merchant_id.get_string_repr()
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
            EmailBody::WelcomeToCommunity => {
                include_str!("assets/welcome_to_community.html").to_string()
            }
        }
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct EmailToken {
    email: String,
    flow: domain::Origin,
    exp: u64,
    entity: Option<Entity>,
}

#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct Entity {
    pub entity_id: String,
    pub entity_type: EntityType,
}

impl Entity {
    pub fn get_entity_type(&self) -> EntityType {
        self.entity_type
    }

    pub fn get_entity_id(&self) -> &str {
        &self.entity_id
    }
}

impl EmailToken {
    pub async fn new_token(
        email: domain::UserEmail,
        entity: Option<Entity>,
        flow: domain::Origin,
        settings: &configs::Settings,
    ) -> UserResult<String> {
        let expiration_duration = std::time::Duration::from_secs(consts::EMAIL_TOKEN_TIME_IN_SECS);
        let exp = jwt::generate_exp(expiration_duration)?.as_secs();
        let token_payload = Self {
            email: email.get_secret().expose(),
            flow,
            exp,
            entity,
        };
        jwt::generate_jwt(&token_payload, settings).await
    }

    pub fn get_email(&self) -> UserResult<domain::UserEmail> {
        pii::Email::try_from(self.email.clone())
            .change_context(UserErrors::InternalServerError)
            .and_then(domain::UserEmail::from_pii_email)
    }

    pub fn get_entity(&self) -> Option<&Entity> {
        self.entity.as_ref()
    }

    pub fn get_flow(&self) -> domain::Origin {
        self.flow.clone()
    }
}

pub fn get_link_with_token(
    base_url: impl std::fmt::Display,
    token: impl std::fmt::Display,
    action: impl std::fmt::Display,
    auth_id: &Option<impl std::fmt::Display>,
    theme_id: &Option<impl std::fmt::Display>,
) -> String {
    let mut email_url = format!("{base_url}/user/{action}?token={token}");
    if let Some(auth_id) = auth_id {
        email_url = format!("{email_url}&auth_id={auth_id}");
    }
    if let Some(theme_id) = theme_id {
        email_url = format!("{email_url}&theme_id={theme_id}");
    }

    email_url
}
pub struct VerifyEmail {
    pub recipient_email: domain::UserEmail,
    pub settings: std::sync::Arc<configs::Settings>,
    pub auth_id: Option<String>,
    pub theme_id: Option<String>,
    pub theme_config: EmailThemeConfig,
}

/// Currently only HTML is supported
#[async_trait::async_trait]
impl EmailData for VerifyEmail {
    async fn get_email_data(&self, base_url: &str) -> CustomResult<EmailContents, EmailError> {
        let token = EmailToken::new_token(
            self.recipient_email.clone(),
            None,
            domain::Origin::VerifyEmail,
            &self.settings,
        )
        .await
        .change_context(EmailError::TokenGenerationFailure)?;

        let verify_email_link = get_link_with_token(
            base_url,
            token,
            "verify_email",
            &self.auth_id,
            &self.theme_id,
        );

        let body = html::get_html_body(EmailBody::Verify {
            link: verify_email_link,
            entity_name: self.theme_config.entity_name.clone(),
            entity_logo_url: self.theme_config.entity_logo_url.clone(),
            primary_color: self.theme_config.primary_color.clone(),
            background_color: self.theme_config.background_color.clone(),
            foreground_color: self.theme_config.foreground_color.clone(),
        });

        Ok(EmailContents {
            subject: format!(
                "Welcome to the {} community!",
                self.theme_config.entity_name
            ),
            body: external_services::email::IntermediateString::new(body),
            recipient: self.recipient_email.clone().into_inner(),
        })
    }
}

pub struct ResetPassword {
    pub recipient_email: domain::UserEmail,
    pub user_name: domain::UserName,
    pub settings: std::sync::Arc<configs::Settings>,
    pub auth_id: Option<String>,
    pub theme_id: Option<String>,
    pub theme_config: EmailThemeConfig,
}

#[async_trait::async_trait]
impl EmailData for ResetPassword {
    async fn get_email_data(&self, base_url: &str) -> CustomResult<EmailContents, EmailError> {
        let token = EmailToken::new_token(
            self.recipient_email.clone(),
            None,
            domain::Origin::ResetPassword,
            &self.settings,
        )
        .await
        .change_context(EmailError::TokenGenerationFailure)?;

        let reset_password_link = get_link_with_token(
            base_url,
            token,
            "set_password",
            &self.auth_id,
            &self.theme_id,
        );

        let body = html::get_html_body(EmailBody::Reset {
            link: reset_password_link,
            user_name: self.user_name.clone().get_secret().expose(),
            entity_name: self.theme_config.entity_name.clone(),
            entity_logo_url: self.theme_config.entity_logo_url.clone(),
            primary_color: self.theme_config.primary_color.clone(),
            background_color: self.theme_config.background_color.clone(),
            foreground_color: self.theme_config.foreground_color.clone(),
        });

        Ok(EmailContents {
            subject: format!(
                "Get back to {} - Reset Your Password Now!",
                self.theme_config.entity_name
            ),
            body: external_services::email::IntermediateString::new(body),
            recipient: self.recipient_email.clone().into_inner(),
        })
    }
}

pub struct MagicLink {
    pub recipient_email: domain::UserEmail,
    pub user_name: domain::UserName,
    pub settings: std::sync::Arc<configs::Settings>,
    pub auth_id: Option<String>,
    pub theme_id: Option<String>,
    pub theme_config: EmailThemeConfig,
}

#[async_trait::async_trait]
impl EmailData for MagicLink {
    async fn get_email_data(&self, base_url: &str) -> CustomResult<EmailContents, EmailError> {
        let token = EmailToken::new_token(
            self.recipient_email.clone(),
            None,
            domain::Origin::MagicLink,
            &self.settings,
        )
        .await
        .change_context(EmailError::TokenGenerationFailure)?;

        let magic_link_login = get_link_with_token(
            base_url,
            token,
            "verify_email",
            &self.auth_id,
            &self.theme_id,
        );

        let body = html::get_html_body(EmailBody::MagicLink {
            link: magic_link_login,
            user_name: self.user_name.clone().get_secret().expose(),
            entity_name: self.theme_config.entity_name.clone(),
            entity_logo_url: self.theme_config.entity_logo_url.clone(),
            primary_color: self.theme_config.primary_color.clone(),
            background_color: self.theme_config.background_color.clone(),
            foreground_color: self.theme_config.foreground_color.clone(),
        });

        Ok(EmailContents {
            subject: format!(
                "Unlock {}: Use Your Magic Link to Sign In",
                self.theme_config.entity_name
            ),
            body: external_services::email::IntermediateString::new(body),
            recipient: self.recipient_email.clone().into_inner(),
        })
    }
}

pub struct InviteUser {
    pub recipient_email: domain::UserEmail,
    pub user_name: domain::UserName,
    pub settings: std::sync::Arc<configs::Settings>,
    pub entity: Entity,
    pub auth_id: Option<String>,
    pub theme_id: Option<String>,
    pub theme_config: EmailThemeConfig,
}

#[async_trait::async_trait]
impl EmailData for InviteUser {
    async fn get_email_data(&self, base_url: &str) -> CustomResult<EmailContents, EmailError> {
        let token = EmailToken::new_token(
            self.recipient_email.clone(),
            Some(self.entity.clone()),
            domain::Origin::AcceptInvitationFromEmail,
            &self.settings,
        )
        .await
        .change_context(EmailError::TokenGenerationFailure)?;

        let invite_user_link = get_link_with_token(
            base_url,
            token,
            "accept_invite_from_email",
            &self.auth_id,
            &self.theme_id,
        );
        let body = html::get_html_body(EmailBody::AcceptInviteFromEmail {
            link: invite_user_link,
            user_name: self.user_name.clone().get_secret().expose(),
            entity_name: self.theme_config.entity_name.clone(),
            entity_logo_url: self.theme_config.entity_logo_url.clone(),
            primary_color: self.theme_config.primary_color.clone(),
            background_color: self.theme_config.background_color.clone(),
            foreground_color: self.theme_config.foreground_color.clone(),
        });

        Ok(EmailContents {
            subject: format!(
                "You have been invited to join {} Community!",
                self.theme_config.entity_name
            ),
            body: external_services::email::IntermediateString::new(body),
            recipient: self.recipient_email.clone().into_inner(),
        })
    }
}

pub struct ReconActivation {
    pub recipient_email: domain::UserEmail,
    pub user_name: domain::UserName,
    pub subject: &'static str,
    pub theme_id: Option<String>,
    pub theme_config: EmailThemeConfig,
}

#[async_trait::async_trait]
impl EmailData for ReconActivation {
    async fn get_email_data(&self, _base_url: &str) -> CustomResult<EmailContents, EmailError> {
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
    pub theme_id: Option<String>,
    pub theme_config: EmailThemeConfig,
    pub product_type: MerchantProductType,
}

impl BizEmailProd {
    pub fn new(
        state: &SessionState,
        data: ProdIntent,
        theme_id: Option<String>,
        theme_config: EmailThemeConfig,
    ) -> UserResult<Self> {
        Ok(Self {
            recipient_email: domain::UserEmail::from_pii_email(
                state.conf.email.prod_intent_recipient_email.clone(),
            )?,
            settings: state.conf.clone(),
            user_name: data
                .poc_name
                .map(|s| Secret::new(s.peek().clone().into_inner()))
                .unwrap_or_default(),
            poc_email: data
                .poc_email
                .map(|s| Secret::new(s.peek().clone()))
                .unwrap_or_default(),
            legal_business_name: data
                .legal_business_name
                .map(|s| s.into_inner())
                .unwrap_or_default(),
            business_location: data
                .business_location
                .unwrap_or(common_enums::CountryAlpha2::AD)
                .to_string(),
            business_website: data
                .business_website
                .map(|s| s.into_inner())
                .unwrap_or_default(),
            theme_id,
            theme_config,
            product_type: data.product_type,
        })
    }
}

#[async_trait::async_trait]
impl EmailData for BizEmailProd {
    async fn get_email_data(&self, _base_url: &str) -> CustomResult<EmailContents, EmailError> {
        let body = html::get_html_body(EmailBody::BizEmailProd {
            user_name: self.user_name.clone().expose(),
            poc_email: self.poc_email.clone().expose(),
            legal_business_name: self.legal_business_name.clone(),
            business_location: self.business_location.clone(),
            business_website: self.business_website.clone(),
            product_type: self.product_type,
        });

        Ok(EmailContents {
            subject: "New Prod Intent".to_string(),
            body: external_services::email::IntermediateString::new(body),
            recipient: self.recipient_email.clone().into_inner(),
        })
    }
}

pub struct ProFeatureRequest {
    pub recipient_email: domain::UserEmail,
    pub feature_name: String,
    pub merchant_id: common_utils::id_type::MerchantId,
    pub user_name: domain::UserName,
    pub user_email: domain::UserEmail,
    pub subject: String,
    pub theme_id: Option<String>,
    pub theme_config: EmailThemeConfig,
}

#[async_trait::async_trait]
impl EmailData for ProFeatureRequest {
    async fn get_email_data(&self, _base_url: &str) -> CustomResult<EmailContents, EmailError> {
        let recipient = self.recipient_email.clone().into_inner();

        let body = html::get_html_body(EmailBody::ProFeatureRequest {
            user_name: self.user_name.clone().get_secret().expose(),
            feature_name: self.feature_name.clone(),
            merchant_id: self.merchant_id.clone(),
            user_email: self.user_email.clone().get_secret().expose(),
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
    pub theme_id: Option<String>,
    pub theme_config: EmailThemeConfig,
}

#[async_trait::async_trait]
impl EmailData for ApiKeyExpiryReminder {
    async fn get_email_data(&self, _base_url: &str) -> CustomResult<EmailContents, EmailError> {
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

pub struct WelcomeToCommunity {
    pub recipient_email: domain::UserEmail,
}

#[async_trait::async_trait]
impl EmailData for WelcomeToCommunity {
    async fn get_email_data(&self, _base_url: &str) -> CustomResult<EmailContents, EmailError> {
        let body = html::get_html_body(EmailBody::WelcomeToCommunity);

        Ok(EmailContents {
            subject: "Thank you for signing up on Hyperswitch Dashboard!".to_string(),
            body: external_services::email::IntermediateString::new(body),
            recipient: self.recipient_email.clone().into_inner(),
        })
    }
}
