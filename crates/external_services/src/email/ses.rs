use crate::email::{EmailClient, EmailError, EmailResult, EmailSettings, IntermediateString};

use aws_sdk_sesv2::types::{Body, Content, Destination, EmailContent, Message};
use common_utils::{errors::CustomResult, ext_traits::OptionExt, pii};
use error_stack::{report, IntoReport, ResultExt};
use masking::PeekInterface;
use router_env::logger;

use actix_web::http::Uri;
use std::time::{Duration, SystemTime};
use tokio::sync::OnceCell;

use aws_sdk_sesv2::{config::Region, operation::send_email::SendEmailError, Client};
use aws_sdk_sts::config::Credentials;

/// Client for AWS SES operation
#[derive(Debug, Clone)]
pub struct AwsSes {
    ses_client: OnceCell<Client>,
    sender: String,
    settings: EmailSettings,
}

/// Errors that could occur during SES operations.
#[derive(Debug, thiserror::Error)]
pub enum AwsSesError {
    /// An error occurred in the SDK while sending email.
    #[error("Failed to Send Email {0:?}")]
    SendingFailure(aws_smithy_client::SdkError<SendEmailError>),

    /// Configuration variable is missing to construct the email client
    #[error("Missing configuration variable {0}")]
    MissingConfigurationVariable(&'static str),

    /// Failed to assume the given STS role
    #[error("Failed to STS assume role: {0:?}")]
    AssumeRoleFailure(String),

    /// Temporary credentials are missing
    #[error("Assumed role does not contain credentials for role user: {0:?}")]
    TemporaryCredentialsMissing(String),
}

impl AwsSes {
    /// Constructs a new AwsSes client
    pub async fn create(conf: &EmailSettings) -> Self {
        Self {
            ses_client: OnceCell::new_with(
                Self::create_client(conf)
                    .await
                    .map_err(|error| logger::error!(?error, "Failed to initialize SES Client"))
                    .ok(),
            ),
            sender: conf.sender_email.clone(),
            settings: conf.clone(),
        }
    }

    /// A helper function to create ses client
    pub async fn create_client(conf: &EmailSettings) -> CustomResult<Client, AwsSesError> {
        let sts_config = Self::get_shared_config(conf.aws_region.to_owned())
            .load()
            .await;

        let email_role_arn = conf
            .email_role_arn
            .as_ref()
            .get_required_value("email_role_arn")
            .change_context(AwsSesError::MissingConfigurationVariable("email_role_arn"))?;

        let sts_session_id = conf
            .sts_session_id
            .as_ref()
            .get_required_value("sts_session_id")
            .change_context(AwsSesError::MissingConfigurationVariable("sts_session_id"))?;

        let role = aws_sdk_sts::Client::new(&sts_config)
            .assume_role()
            .role_arn(email_role_arn)
            .role_session_name(sts_session_id)
            .send()
            .await
            .into_report()
            .attach_printable(format!("Role ARN {email_role_arn}"))
            .attach_printable(format!("Role Session name {sts_session_id}"))
            .attach_printable(format!("Region {}", conf.aws_region))
            .change_context(AwsSesError::AssumeRoleFailure(sts_session_id.clone()))?;

        let creds = role.credentials().ok_or(
            report!(AwsSesError::TemporaryCredentialsMissing(format!(
                "{role:?}"
            )))
            .attach_printable("Credentials object not available"),
        )?;

        let credentials = Credentials::new(
            creds
                .access_key_id()
                .ok_or(
                    report!(AwsSesError::TemporaryCredentialsMissing(format!(
                        "{role:?}"
                    )))
                    .attach_printable("Access Key ID not found"),
                )?
                .to_owned(),
            creds
                .secret_access_key()
                .ok_or(
                    report!(AwsSesError::TemporaryCredentialsMissing(format!(
                        "{role:?}"
                    )))
                    .attach_printable("Secret Access Key not found"),
                )?
                .to_owned(),
            creds.session_token().map(|s| s.to_owned()),
            creds.expiration().and_then(|dt| {
                SystemTime::UNIX_EPOCH
                    .checked_add(Duration::from_nanos(u64::try_from(dt.as_nanos()).ok()?))
            }),
            "custom_provider",
        );

        logger::debug!(
            "SES temporary credentials with expiry {:?}",
            credentials.expiry()
        );

        let ses_config = Self::get_shared_config(conf.aws_region.to_owned())
            .credentials_provider(credentials)
            .load()
            .await;

        Ok(Client::new(&ses_config))
    }

    fn get_shared_config(region: String) -> aws_config::ConfigLoader {
        let region_provider = Region::new(region);
        let mut config = aws_config::from_env().region(region_provider);
        if let Some(proxy_connector) = Self::get_connector() {
            let provider_config = aws_config::provider_config::ProviderConfig::default()
                .with_tcp_connector(proxy_connector.clone());
            let http_connector =
                aws_smithy_client::hyper_ext::Adapter::builder().build(proxy_connector);
            config = config
                .configure(provider_config)
                .http_connector(http_connector);
        };
        config
    }

    fn get_connector() -> Option<hyper_proxy::ProxyConnector<hyper::client::HttpConnector>> {
        std::env::var("ROUTER_HTTPS_PROXY")
            .ok()
            .and_then(|var| var.parse::<Uri>().ok())
            .map(|url| hyper_proxy::Proxy::new(hyper_proxy::Intercept::All, url))
            .and_then(|proxy| {
                hyper_proxy::ProxyConnector::from_proxy(hyper::client::HttpConnector::new(), proxy)
                    .ok()
            })
    }
}

#[async_trait::async_trait]
impl EmailClient for AwsSes {
    fn convert_to_rich_text(
        &self,
        intermediate_string: IntermediateString,
    ) -> CustomResult<String, EmailError> {
        let content = Content::builder()
            .data(intermediate_string.into_inner())
            .charset("UTF-8")
            .build();

        let rich_text_body = Body::builder()
            .html(content)
            .build()
            .html()
            .and_then(|body| body.data())
            .map(|data| data.to_owned())
            .ok_or(EmailError::EmailSendingFailure)
            .into_report()
            .attach_printable("Failed to convert email body into text")?;

        Ok(rich_text_body)
    }

    async fn send_email(
        &self,
        recipient: pii::Email,
        subject: String,
        body: String,
    ) -> EmailResult<()> {
        self.ses_client
            .get_or_try_init(|| async {
                Self::create_client(&self.settings)
                    .await
                    .change_context(EmailError::ClientBuildingFailure)
            })
            .await?
            .send_email()
            .from_email_address(self.sender.to_owned())
            .destination(
                Destination::builder()
                    .to_addresses(recipient.peek())
                    .build(),
            )
            .content(
                EmailContent::builder()
                    .simple(
                        Message::builder()
                            .subject(Content::builder().data(subject).build())
                            .body(
                                Body::builder()
                                    .html(Content::builder().data(body).charset("UTF-8").build())
                                    .build(),
                            )
                            .build(),
                    )
                    .build(),
            )
            .send()
            .await
            .map_err(AwsSesError::SendingFailure)
            .into_report()
            .change_context(EmailError::EmailSendingFailure)?;

        Ok(())
    }
}
