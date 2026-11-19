//! AWS SES client, sending with credentials from an STS-assumed role.

use std::time::{Duration, SystemTime};

use aws_sdk_sesv2::{
    config::Region,
    operation::send_email::SendEmailError,
    types::{Body, Content, Destination, EmailContent, Message},
    Client,
};
use aws_sdk_sts::{config::Credentials, operation::assume_role::AssumeRoleError};
use aws_smithy_runtime::client::http::hyper_014::HyperClientBuilder;

use super::EmailMessage;

/// The role SES sends as, assumed through STS.
#[derive(Clone, Debug, Default, serde::Deserialize)]
pub struct SesConfig {
    /// The ARN of the role to assume.
    pub email_role_arn: String,
    /// The STS session name for the assumed role.
    pub sts_role_session_name: String,
}

impl SesConfig {
    /// Checks that the role is set.
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.email_role_arn.is_empty() {
            return Err("email.aws_ses.email_role_arn must not be empty");
        }
        if self.sts_role_session_name.is_empty() {
            return Err("email.aws_ses.sts_role_session_name must not be empty");
        }
        Ok(())
    }
}

/// Errors from sending through SES.
#[derive(Debug, thiserror::Error)]
pub enum SesError {
    /// STS refused to assume the role.
    #[error(
        "Failed to STS assume role: Role ARN: {role_arn}, Session name: {session_name}, Region: {region}"
    )]
    AssumeRoleFailure {
        /// The AWS region.
        region: String,
        /// The ARN of the role.
        role_arn: String,
        /// The STS session name.
        session_name: String,
        /// The STS error.
        #[source]
        source: Box<aws_sdk_sts::error::SdkError<AssumeRoleError>>,
    },
    /// The assumed role came without credentials.
    #[error("Assumed role does not contain credentials for role user: {0}")]
    TemporaryCredentialsMissing(String),
    /// The proxy URL could not be used.
    #[error("The proxy connector cannot be built: {0}")]
    BuildingProxyConnectorFailed(String),
    /// The message could not be built.
    #[error("Failed to build the email content")]
    ContentBuildFailure(#[source] aws_sdk_sesv2::error::BuildError),
    /// SES refused or failed to send.
    #[error("Failed to send email")]
    SendingFailure(#[source] Box<aws_sdk_sesv2::error::SdkError<SendEmailError>>),
}

impl SesError {
    /// Whether the error happened while building the client, before any message was sent.
    pub fn is_client_building_failure(&self) -> bool {
        matches!(
            self,
            Self::AssumeRoleFailure { .. }
                | Self::TemporaryCredentialsMissing(_)
                | Self::BuildingProxyConnectorFailed(_)
        )
    }
}

/// Sends email through SES in a region, as an STS-assumed role.
#[derive(Clone, Debug)]
pub struct SesClient {
    region: String,
    config: SesConfig,
}

impl SesClient {
    /// Creates the client; nothing is contacted until [`Self::send`] or [`Self::sdk_client`].
    pub fn new(region: String, config: SesConfig) -> Self {
        Self { region, config }
    }

    /// Assumes the role and builds an SDK client, through `proxy_url` when given. [`Self::send`]
    /// does this for every message, since the role's session expires.
    pub async fn sdk_client(&self, proxy_url: Option<&str>) -> Result<Client, SesError> {
        let sts_config = shared_config(&self.region, proxy_url)?.load().await;

        let role = aws_sdk_sts::Client::new(&sts_config)
            .assume_role()
            .role_arn(&self.config.email_role_arn)
            .role_session_name(&self.config.sts_role_session_name)
            .send()
            .await
            .map_err(|source| SesError::AssumeRoleFailure {
                region: self.region.clone(),
                role_arn: self.config.email_role_arn.clone(),
                session_name: self.config.sts_role_session_name.clone(),
                source: Box::new(source),
            })?;

        let creds = role
            .credentials()
            .ok_or_else(|| SesError::TemporaryCredentialsMissing(format!("{role:?}")))?;

        let credentials = Credentials::new(
            creds.access_key_id(),
            creds.secret_access_key(),
            Some(creds.session_token().to_owned()),
            u64::try_from(creds.expiration().as_nanos())
                .ok()
                .map(Duration::from_nanos)
                .and_then(|val| SystemTime::UNIX_EPOCH.checked_add(val)),
            "custom_provider",
        );

        let ses_config = shared_config(&self.region, proxy_url)?
            .credentials_provider(credentials)
            .load()
            .await;

        Ok(Client::new(&ses_config))
    }

    /// Sends `message` with fresh credentials, through `proxy_url` when given.
    pub async fn send(
        &self,
        message: EmailMessage,
        proxy_url: Option<&str>,
    ) -> Result<(), SesError> {
        let client = self.sdk_client(proxy_url).await?;

        let body = Body::builder()
            .html(
                Content::builder()
                    .data(message.html_body)
                    .charset("UTF-8")
                    .build()
                    .map_err(SesError::ContentBuildFailure)?,
            )
            .build();
        let subject = Content::builder()
            .data(message.subject)
            .build()
            .map_err(SesError::ContentBuildFailure)?;

        client
            .send_email()
            .from_email_address(message.from)
            .destination(
                Destination::builder()
                    .set_to_addresses(Some(message.to))
                    .set_cc_addresses((!message.cc.is_empty()).then_some(message.cc))
                    .set_bcc_addresses((!message.bcc.is_empty()).then_some(message.bcc))
                    .build(),
            )
            .content(
                EmailContent::builder()
                    .simple(Message::builder().subject(subject).body(body).build())
                    .build(),
            )
            .send()
            .await
            .map_err(|error| SesError::SendingFailure(Box::new(error)))?;

        Ok(())
    }
}

fn shared_config(
    region: &str,
    proxy_url: Option<&str>,
) -> Result<aws_config::ConfigLoader, SesError> {
    let mut config = aws_config::from_env().region(Region::new(region.to_owned()));

    if let Some(proxy_url) = proxy_url {
        let proxy_uri = proxy_url
            .parse::<hyper::Uri>()
            .map_err(|error| SesError::BuildingProxyConnectorFailed(error.to_string()))?;
        let proxy = hyper_proxy::Proxy::new(hyper_proxy::Intercept::All, proxy_uri);
        let proxy_connector =
            hyper_proxy::ProxyConnector::from_proxy(hyper::client::HttpConnector::new(), proxy)
                .map_err(|error| SesError::BuildingProxyConnectorFailed(error.to_string()))?;
        config = config.http_client(HyperClientBuilder::new().build(proxy_connector));
    }
    Ok(config)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_requires_the_role() {
        assert!(SesConfig::default().validate().is_err());
        let config = SesConfig {
            email_role_arn: "arn:aws:iam::123456789012:role/ses".to_owned(),
            sts_role_session_name: "hyperswitch".to_owned(),
        };
        assert!(config.validate().is_ok());
    }

    #[test]
    fn client_building_failures_are_told_apart() {
        assert!(
            SesError::BuildingProxyConnectorFailed("bad".to_owned()).is_client_building_failure()
        );
        assert!(
            SesError::TemporaryCredentialsMissing("role".to_owned()).is_client_building_failure()
        );
    }
}
