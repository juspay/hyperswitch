use api_models::connector_onboarding as api;
use error_stack::{IntoReport, ResultExt};

use crate::core::errors::{ApiErrorResponse, RouterResult};

#[derive(serde::Deserialize, Debug)]
pub struct HateoasLink {
    pub href: String,
    pub rel: String,
    pub method: String,
}

#[derive(serde::Deserialize, Debug)]
pub struct PartnerReferralResponse {
    pub links: Vec<HateoasLink>,
}

#[derive(serde::Serialize, Debug)]
pub struct PartnerReferralRequest {
    pub tracking_id: String,
    pub operations: Vec<PartnerReferralOperations>,
    pub products: Vec<PayPalProducts>,
    pub capabilities: Vec<PayPalCapabilities>,
    pub partner_config_override: PartnerConfigOverride,
    pub legal_consents: Vec<LegalConsent>,
}

#[derive(serde::Serialize, Debug)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PayPalProducts {
    Ppcp,
    AdvancedVaulting,
}

#[derive(serde::Serialize, Debug)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PayPalCapabilities {
    PaypalWalletVaultingAdvanced,
}

#[derive(serde::Serialize, Debug)]
pub struct PartnerReferralOperations {
    pub operation: PayPalReferralOperationType,
    pub api_integration_preference: PartnerReferralIntegrationPreference,
}

#[derive(serde::Serialize, Debug)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PayPalReferralOperationType {
    ApiIntegration,
}

#[derive(serde::Serialize, Debug)]
pub struct PartnerReferralIntegrationPreference {
    pub rest_api_integration: PartnerReferralRestApiIntegration,
}

#[derive(serde::Serialize, Debug)]
pub struct PartnerReferralRestApiIntegration {
    pub integration_method: IntegrationMethod,
    pub integration_type: PayPalIntegrationType,
    pub third_party_details: PartnerReferralThirdPartyDetails,
}

#[derive(serde::Serialize, Debug)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum IntegrationMethod {
    Paypal,
}

#[derive(serde::Serialize, Debug)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PayPalIntegrationType {
    ThirdParty,
}

#[derive(serde::Serialize, Debug)]
pub struct PartnerReferralThirdPartyDetails {
    pub features: Vec<PayPalFeatures>,
}

#[derive(serde::Serialize, Debug)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PayPalFeatures {
    Payment,
    Refund,
    Vault,
    AccessMerchantInformation,
    BillingAgreement,
    ReadSellerDispute,
}

#[derive(serde::Serialize, Debug)]
pub struct PartnerConfigOverride {
    pub partner_logo_url: String,
    pub return_url: String,
}

#[derive(serde::Serialize, Debug)]
pub struct LegalConsent {
    #[serde(rename = "type")]
    pub consent_type: LegalConsentType,
    pub granted: bool,
}

#[derive(serde::Serialize, Debug)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum LegalConsentType {
    ShareDataConsent,
}

impl PartnerReferralRequest {
        /// Creates a new instance of PartnerReferralRequest with the specified tracking ID and return URL.
    pub fn new(tracking_id: String, return_url: String) -> Self {
        Self {
            tracking_id,
            operations: vec![PartnerReferralOperations {
                operation: PayPalReferralOperationType::ApiIntegration,
                api_integration_preference: PartnerReferralIntegrationPreference {
                    rest_api_integration: PartnerReferralRestApiIntegration {
                        integration_method: IntegrationMethod::Paypal,
                        integration_type: PayPalIntegrationType::ThirdParty,
                        third_party_details: PartnerReferralThirdPartyDetails {
                            features: vec![
                                PayPalFeatures::Payment,
                                PayPalFeatures::Refund,
                                PayPalFeatures::Vault,
                                PayPalFeatures::AccessMerchantInformation,
                                PayPalFeatures::BillingAgreement,
                                PayPalFeatures::ReadSellerDispute,
                            ],
                        },
                    },
                },
            }],
            products: vec![PayPalProducts::Ppcp, PayPalProducts::AdvancedVaulting],
            capabilities: vec![PayPalCapabilities::PaypalWalletVaultingAdvanced],
            partner_config_override: PartnerConfigOverride {
                partner_logo_url: "https://hyperswitch.io/img/websiteIcon.svg".to_string(),
                return_url,
            },
            legal_consents: vec![LegalConsent {
                consent_type: LegalConsentType::ShareDataConsent,
                granted: true,
            }],
        }
    }
}

#[derive(serde::Deserialize, Debug)]
pub struct SellerStatusResponse {
    pub merchant_id: String,
    pub links: Vec<HateoasLink>,
}

#[derive(serde::Deserialize, Debug)]
pub struct SellerStatusDetailsResponse {
    pub merchant_id: String,
    pub primary_email_confirmed: bool,
    pub payments_receivable: bool,
    pub products: Vec<SellerStatusProducts>,
}

#[derive(serde::Deserialize, Debug)]
pub struct SellerStatusProducts {
    pub name: String,
    pub vetting_status: Option<VettingStatus>,
}

#[derive(serde::Deserialize, Debug, Clone)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum VettingStatus {
    NeedMoreData,
    Subscribed,
    Denied,
}

impl SellerStatusResponse {
        /// Extracts the merchant details URL by combining the first link's href with the provided PayPal base URL.
    pub fn extract_merchant_details_url(self, paypal_base_url: &str) -> RouterResult<String> {
        self.links
            .first()
            .and_then(|link| link.href.strip_prefix('/'))
            .map(|link| format!("{}{}", paypal_base_url, link))
            .ok_or(ApiErrorResponse::InternalServerError)
            .into_report()
            .attach_printable("Merchant details not received in onboarding status")
    }
}

impl SellerStatusDetailsResponse {
        /// Checks if payments are receivable for the current user.
    /// If payments are not receivable, returns Some(api::PayPalOnboardingStatus::PaymentsNotReceivable),
    /// otherwise returns None.
    pub fn check_payments_receivable(&self) -> Option<api::PayPalOnboardingStatus> {
        if !self.payments_receivable {
            return Some(api::PayPalOnboardingStatus::PaymentsNotReceivable);
        }
        None
    }

        /// Checks the custom PayPal onboarding status and returns the corresponding PayPal onboarding status.
    pub fn check_ppcp_custom_status(&self) -> Option<api::PayPalOnboardingStatus> {
        match self.get_ppcp_custom_status() {
            Some(VettingStatus::Denied) => Some(api::PayPalOnboardingStatus::PpcpCustomDenied),
            Some(VettingStatus::Subscribed) => None,
            _ => Some(api::PayPalOnboardingStatus::MorePermissionsNeeded),
        }
    }

        /// Checks if the primary email for the PayPal onboarding is confirmed. 
    /// If the email is not confirmed, it returns Some(api::PayPalOnboardingStatus::EmailNotVerified), 
    /// otherwise it returns None.
    fn check_email_confirmation(&self) -> Option<api::PayPalOnboardingStatus> {
        if !self.primary_email_confirmed {
            return Some(api::PayPalOnboardingStatus::EmailNotVerified);
        }
        None
    }

        /// Retrieves the eligibility status for PayPal onboarding.
    pub async fn get_eligibility_status(&self) -> RouterResult<api::PayPalOnboardingStatus> {
        Ok(self
            .check_payments_receivable()
            .or(self.check_email_confirmation())
            .or(self.check_ppcp_custom_status())
            .unwrap_or(api::PayPalOnboardingStatus::Success(
                api::PayPalOnboardingDone {
                    payer_id: self.get_payer_id(),
                },
            )))
    }

        /// Returns the vetting status of the product with the name "PPCP_CUSTOM" if it exists.
    fn get_ppcp_custom_status(&self) -> Option<VettingStatus> {
        self.products
            .iter()
            .find(|product| product.name == "PPCP_CUSTOM")
            .and_then(|ppcp_custom| ppcp_custom.vetting_status.clone())
    }

        /// Returns the payer ID associated with the merchant.
    fn get_payer_id(&self) -> String {
        self.merchant_id.to_string()
    }
}

impl PartnerReferralResponse {
        /// Extracts the action URL from the links in the response and returns it as a Result.
    pub fn extract_action_url(self) -> RouterResult<String> {
        Ok(self
            .links
            .into_iter()
            .find(|hateoas_link| hateoas_link.rel == "action_url")
            .ok_or(ApiErrorResponse::InternalServerError)
            .into_report()
            .attach_printable("Failed to get action_url from paypal response")?
            .href)
    }
}
