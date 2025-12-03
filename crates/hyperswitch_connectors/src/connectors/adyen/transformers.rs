use std::{ops::Deref, str::FromStr};

#[cfg(feature = "payouts")]
use api_models::payouts::{self, PayoutMethodData};
use api_models::{
    enums,
    payments::{self, PollConfig, QrCodeInformation, VoucherNextStepData},
};
use cards::CardNumber;
use common_enums::enums as storage_enums;
#[cfg(feature = "payouts")]
use common_utils::ext_traits::OptionExt;
use common_utils::{
    errors::{CustomResult, ParsingError},
    ext_traits::{Encode, ValueExt},
    pii::Email,
    request::Method,
    types::{MinorUnit, SemanticVersion},
};
use error_stack::{report, ResultExt};
use hyperswitch_domain_models::{
    network_tokenization::NetworkTokenNumber,
    payment_method_data::{
        BankDebitData, BankRedirectData, BankTransferData, Card, CardRedirectData, GiftCardData,
        NetworkTokenData, PayLaterData, PaymentMethodData, VoucherData, WalletData,
    },
    router_data::{
        ConnectorAuthType, ConnectorResponseData, ErrorResponse, ExtendedAuthorizationResponseData,
        PaymentMethodBalance, PaymentMethodToken, RouterData,
    },
    router_flow_types::GiftCardBalanceCheck,
    router_request_types::{
        GiftCardBalanceCheckRequestData, ResponseId, SubmitEvidenceRequestData,
    },
    router_response_types::{
        AcceptDisputeResponse, DefendDisputeResponse, GiftCardBalanceCheckResponseData,
        MandateReference, PaymentsResponseData, RedirectForm, RefundsResponseData,
        SubmitEvidenceResponse,
    },
    types::{
        PaymentsAuthorizeRouterData, PaymentsCancelRouterData, PaymentsCaptureRouterData,
        PaymentsExtendAuthorizationRouterData, PaymentsGiftCardBalanceCheckRouterData,
        PaymentsPreProcessingRouterData, RefundsRouterData,
    },
};
#[cfg(feature = "payouts")]
use hyperswitch_domain_models::{
    router_response_types::PayoutsResponseData, types::PayoutsRouterData,
};
use hyperswitch_interfaces::{
    consts::{NO_ERROR_CODE, NO_ERROR_MESSAGE},
    errors,
};
use masking::{ExposeInterface, PeekInterface, Secret};
use serde::{Deserialize, Serialize};
use time::{Duration, OffsetDateTime, PrimitiveDateTime};
use url::Url;

#[cfg(feature = "payouts")]
use crate::{types::PayoutsResponseRouterData, utils::PayoutsData};
use crate::{
    types::{
        AcceptDisputeRouterData, DefendDisputeRouterData, PaymentsCancelResponseRouterData,
        PaymentsCaptureResponseRouterData, PaymentsExtendAuthorizationResponseRouterData,
        PaymentsPreprocessingResponseRouterData, RefundsResponseRouterData, ResponseRouterData,
        SubmitEvidenceRouterData,
    },
    utils::{
        self, is_manual_capture, missing_field_err, AddressDetailsData, BrowserInformationData,
        CardData, ForeignTryFrom, NetworkTokenData as UtilsNetworkTokenData,
        PaymentsAuthorizeRequestData, PhoneDetailsData, RouterData as OtherRouterData,
    },
};

type Error = error_stack::Report<errors::ConnectorError>;

#[derive(Debug, Serialize)]
pub struct AdyenRouterData<T> {
    pub amount: MinorUnit,
    pub router_data: T,
}

impl<T> TryFrom<(MinorUnit, T)> for AdyenRouterData<T> {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from((amount, item): (MinorUnit, T)) -> Result<Self, Self::Error> {
        Ok(Self {
            amount,
            router_data: item,
        })
    }
}
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct AdyenConnectorMetadataObject {
    pub endpoint_prefix: Option<String>,
}

impl TryFrom<&Option<common_utils::pii::SecretSerdeValue>> for AdyenConnectorMetadataObject {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        meta_data: &Option<common_utils::pii::SecretSerdeValue>,
    ) -> Result<Self, Self::Error> {
        match meta_data {
            Some(metadata) => utils::to_connector_meta_from_secret::<Self>(Some(metadata.clone()))
                .change_context(errors::ConnectorError::InvalidConnectorConfig {
                    config: "metadata",
                }),
            None => Ok(Self::default()),
        }
    }
}

// Adyen Types Definition
// Payments Request and Response Types
#[derive(Default, Debug, Serialize, Deserialize)]
pub enum AdyenShopperInteraction {
    #[default]
    Ecommerce,
    #[serde(rename = "ContAuth")]
    ContinuedAuthentication,
    Moto,
    #[serde(rename = "POS")]
    Pos,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum AdyenRecurringModel {
    UnscheduledCardOnFile,
    CardOnFile,
}

#[derive(Clone, Default, Debug, Serialize, Deserialize)]
pub enum AuthType {
    #[default]
    PreAuth,
}
#[serde_with::skip_serializing_none]
#[derive(Clone, Default, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdditionalData {
    authorisation_type: Option<AuthType>,
    manual_capture: Option<String>,
    execute_three_d: Option<String>,
    pub recurring_processing_model: Option<AdyenRecurringModel>,
    /// Enable recurring details in dashboard to receive this ID, https://docs.adyen.com/online-payments/tokenization/create-and-use-tokens#test-and-go-live
    #[serde(rename = "recurring.recurringDetailReference")]
    recurring_detail_reference: Option<Secret<String>>,
    #[serde(rename = "recurring.shopperReference")]
    recurring_shopper_reference: Option<String>,
    network_tx_reference: Option<Secret<String>>,
    #[cfg(feature = "payouts")]
    payout_eligible: Option<PayoutEligibility>,
    funds_availability: Option<String>,
    refusal_reason_raw: Option<String>,
    refusal_code_raw: Option<String>,
    merchant_advice_code: Option<String>,
    #[serde(flatten)]
    riskdata: Option<RiskData>,
    sca_exemption: Option<AdyenExemptionValues>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AdyenExemptionValues {
    LowValue,
    SecureCorporate,
    TrustedBeneficiary,
    TransactionRiskAnalysis,
}

fn to_adyen_exemption(data: &enums::ExemptionIndicator) -> Option<AdyenExemptionValues> {
    match data {
        enums::ExemptionIndicator::LowValue => Some(AdyenExemptionValues::LowValue),
        enums::ExemptionIndicator::SecureCorporatePayment => {
            Some(AdyenExemptionValues::SecureCorporate)
        }
        enums::ExemptionIndicator::TrustedListing => Some(AdyenExemptionValues::TrustedBeneficiary),
        enums::ExemptionIndicator::TransactionRiskAssessment => {
            Some(AdyenExemptionValues::TransactionRiskAnalysis)
        }
        _ => None,
    }
}

#[serde_with::skip_serializing_none]
#[derive(Default, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShopperName {
    first_name: Option<Secret<String>>,
    last_name: Option<Secret<String>>,
}

#[serde_with::skip_serializing_none]
#[derive(Default, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Address {
    city: String,
    country: enums::CountryAlpha2,
    house_number_or_name: Secret<String>,
    postal_code: Secret<String>,
    state_or_province: Option<Secret<String>>,
    street: Option<Secret<String>>,
}

#[serde_with::skip_serializing_none]
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LineItem {
    amount_excluding_tax: Option<MinorUnit>,
    amount_including_tax: Option<MinorUnit>,
    description: Option<String>,
    id: Option<String>,
    tax_amount: Option<MinorUnit>,
    quantity: Option<u16>,
}

#[serde_with::skip_serializing_none]
#[derive(Clone, Default, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RiskData {
    #[serde(rename = "riskdata.basket.item1.itemID")]
    item_i_d: Option<String>,
    #[serde(rename = "riskdata.basket.item1.productTitle")]
    product_title: Option<String>,
    #[serde(rename = "riskdata.basket.item1.amountPerItem")]
    amount_per_item: Option<String>,
    #[serde(rename = "riskdata.basket.item1.currency")]
    currency: Option<String>,
    #[serde(rename = "riskdata.basket.item1.upc")]
    upc: Option<String>,
    #[serde(rename = "riskdata.basket.item1.brand")]
    brand: Option<String>,
    #[serde(rename = "riskdata.basket.item1.manufacturer")]
    manufacturer: Option<String>,
    #[serde(rename = "riskdata.basket.item1.category")]
    category: Option<String>,
    #[serde(rename = "riskdata.basket.item1.quantity")]
    quantity: Option<String>,
    #[serde(rename = "riskdata.basket.item1.color")]
    color: Option<String>,
    #[serde(rename = "riskdata.basket.item1.size")]
    size: Option<String>,
    #[serde(rename = "riskdata.deviceCountry")]
    device_country: Option<String>,
    #[serde(rename = "riskdata.houseNumberorName")]
    house_numberor_name: Option<String>,
    #[serde(rename = "riskdata.accountCreationDate")]
    account_creation_date: Option<String>,
    #[serde(rename = "riskdata.affiliateChannel")]
    affiliate_channel: Option<String>,
    #[serde(rename = "riskdata.avgOrderValue")]
    avg_order_value: Option<String>,
    #[serde(rename = "riskdata.deliveryMethod")]
    delivery_method: Option<String>,
    #[serde(rename = "riskdata.emailName")]
    email_name: Option<String>,
    #[serde(rename = "riskdata.emailDomain")]
    email_domain: Option<String>,
    #[serde(rename = "riskdata.lastOrderDate")]
    last_order_date: Option<String>,
    #[serde(rename = "riskdata.merchantReference")]
    merchant_reference: Option<String>,
    #[serde(rename = "riskdata.paymentMethod")]
    payment_method: Option<String>,
    #[serde(rename = "riskdata.promotionName")]
    promotion_name: Option<String>,
    #[serde(rename = "riskdata.secondaryPhoneNumber")]
    secondary_phone_number: Option<String>,
    #[serde(rename = "riskdata.timefromLogintoOrder")]
    timefrom_loginto_order: Option<String>,
    #[serde(rename = "riskdata.totalSessionTime")]
    total_session_time: Option<String>,
    #[serde(rename = "riskdata.totalAuthorizedAmountInLast30Days")]
    total_authorized_amount_in_last30_days: Option<String>,
    #[serde(rename = "riskdata.totalOrderQuantity")]
    total_order_quantity: Option<String>,
    #[serde(rename = "riskdata.totalLifetimeValue")]
    total_lifetime_value: Option<String>,
    #[serde(rename = "riskdata.visitsMonth")]
    visits_month: Option<String>,
    #[serde(rename = "riskdata.visitsWeek")]
    visits_week: Option<String>,
    #[serde(rename = "riskdata.visitsYear")]
    visits_year: Option<String>,
    #[serde(rename = "riskdata.shipToName")]
    ship_to_name: Option<String>,
    #[serde(rename = "riskdata.first8charactersofAddressLine1Zip")]
    first8charactersof_address_line1_zip: Option<String>,
    #[serde(rename = "riskdata.affiliateOrder")]
    affiliate_order: Option<bool>,
}

#[serde_with::skip_serializing_none]
#[derive(Clone, Default, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplicationInfo {
    external_platform: Option<ExternalPlatform>,
    merchant_application: Option<MerchantApplication>,
}

#[serde_with::skip_serializing_none]
#[derive(Clone, Default, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExternalPlatform {
    name: Option<String>,
    version: Option<String>,
    integrator: Option<String>,
}

#[serde_with::skip_serializing_none]
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MerchantApplication {
    name: Option<String>,
    version: Option<String>,
}

#[serde_with::skip_serializing_none]
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AdyenPaymentRequest<'a> {
    amount: Amount,
    merchant_account: Secret<String>,
    payment_method: PaymentMethod<'a>,
    mpi_data: Option<AdyenMpiData>,
    reference: String,
    return_url: String,
    browser_info: Option<AdyenBrowserInfo>,
    shopper_interaction: AdyenShopperInteraction,
    recurring_processing_model: Option<AdyenRecurringModel>,
    additional_data: Option<AdditionalData>,
    shopper_reference: Option<String>,
    store_payment_method: Option<bool>,
    shopper_name: Option<ShopperName>,
    #[serde(rename = "shopperIP")]
    shopper_ip: Option<Secret<String, common_utils::pii::IpAddress>>,
    shopper_locale: Option<String>,
    shopper_email: Option<Email>,
    shopper_statement: Option<String>,
    social_security_number: Option<Secret<String>>,
    telephone_number: Option<Secret<String>>,
    billing_address: Option<Address>,
    delivery_address: Option<Address>,
    country_code: Option<enums::CountryAlpha2>,
    line_items: Option<Vec<LineItem>>,
    channel: Option<Channel>,
    merchant_order_reference: Option<String>,
    splits: Option<Vec<AdyenSplitData>>,
    /// metadata.store
    store: Option<String>,
    device_fingerprint: Option<Secret<String>>,
    #[serde(with = "common_utils::custom_serde::iso8601::option")]
    session_validity: Option<PrimitiveDateTime>,
    metadata: Option<serde_json::Value>,
    platform_chargeback_logic: Option<AdyenPlatformChargeBackLogicMetadata>,
    application_info: Option<ApplicationInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AdyenSplitData {
    amount: Option<Amount>,
    #[serde(rename = "type")]
    split_type: common_enums::AdyenSplitType,
    account: Option<String>,
    reference: String,
    description: Option<String>,
}

#[serde_with::skip_serializing_none]
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct AdyenMpiData {
    directory_response: common_enums::TransactionStatus,
    authentication_response: common_enums::TransactionStatus,
    cavv: Option<Secret<String>>,
    token_authentication_verification_value: Option<Secret<String>>,
    eci: Option<String>,
    #[serde(rename = "dsTransID")]
    ds_trans_id: Option<String>,
    #[serde(rename = "threeDSVersion")]
    three_ds_version: Option<SemanticVersion>,
    challenge_cancel: Option<String>,
    risk_score: Option<String>,
    cavv_algorithm: Option<enums::CavvAlgorithm>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct AdyenBrowserInfo {
    user_agent: String,
    accept_header: String,
    language: String,
    color_depth: u8,
    screen_height: u32,
    screen_width: u32,
    time_zone_offset: i32,
    java_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AdyenStatus {
    AuthenticationFinished,
    AuthenticationNotRequired,
    Authorised,
    Cancelled,
    ChallengeShopper,
    Error,
    Pending,
    Received,
    RedirectShopper,
    Refused,
    PresentToShopper,
    #[cfg(feature = "payouts")]
    #[serde(rename = "[payout-confirm-received]")]
    PayoutConfirmReceived,
    #[cfg(feature = "payouts")]
    #[serde(rename = "[payout-decline-received]")]
    PayoutDeclineReceived,
    #[cfg(feature = "payouts")]
    #[serde(rename = "[payout-submit-received]")]
    PayoutSubmitReceived,
}

#[derive(Debug, Clone, Serialize)]
pub enum Channel {
    Web,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AdyenBalanceRequest<'a> {
    pub payment_method: AdyenPaymentMethod<'a>,
    pub merchant_account: Secret<String>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AdyenBalanceResponse {
    pub psp_reference: String,
    pub balance: Amount,
}

/// This implementation will be used only in Authorize, Automatic capture flow.
/// It is also being used in Psync flow, However Psync will be called only after create payment call that too in redirect flow.
fn get_adyen_payment_status(
    is_manual_capture: bool,
    adyen_status: AdyenStatus,
    pmt: Option<common_enums::PaymentMethodType>,
) -> storage_enums::AttemptStatus {
    match adyen_status {
        AdyenStatus::AuthenticationFinished => {
            storage_enums::AttemptStatus::AuthenticationSuccessful
        }
        AdyenStatus::AuthenticationNotRequired | AdyenStatus::Received => {
            storage_enums::AttemptStatus::Pending
        }
        AdyenStatus::Authorised => match is_manual_capture {
            true => storage_enums::AttemptStatus::Authorized,
            // In case of Automatic capture Authorized is the final status of the payment
            false => storage_enums::AttemptStatus::Charged,
        },
        AdyenStatus::Cancelled => storage_enums::AttemptStatus::Voided,
        AdyenStatus::ChallengeShopper
        | AdyenStatus::RedirectShopper
        | AdyenStatus::PresentToShopper => storage_enums::AttemptStatus::AuthenticationPending,
        AdyenStatus::Error | AdyenStatus::Refused => storage_enums::AttemptStatus::Failure,
        AdyenStatus::Pending => match pmt {
            Some(common_enums::PaymentMethodType::Pix) => {
                storage_enums::AttemptStatus::AuthenticationPending
            }
            _ => storage_enums::AttemptStatus::Pending,
        },
        #[cfg(feature = "payouts")]
        AdyenStatus::PayoutConfirmReceived => storage_enums::AttemptStatus::Started,
        #[cfg(feature = "payouts")]
        AdyenStatus::PayoutSubmitReceived => storage_enums::AttemptStatus::Pending,
        #[cfg(feature = "payouts")]
        AdyenStatus::PayoutDeclineReceived => storage_enums::AttemptStatus::Voided,
    }
}

impl ForeignTryFrom<(bool, AdyenWebhookStatus)> for storage_enums::AttemptStatus {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn foreign_try_from(
        (is_manual_capture, adyen_webhook_status): (bool, AdyenWebhookStatus),
    ) -> Result<Self, Self::Error> {
        match adyen_webhook_status {
            AdyenWebhookStatus::Authorised | AdyenWebhookStatus::AdjustedAuthorization => {
                match is_manual_capture {
                    true => Ok(Self::Authorized),
                    // In case of Automatic capture Authorized is the final status of the payment
                    false => Ok(Self::Charged),
                }
            }
            AdyenWebhookStatus::AuthorisationFailed
            | AdyenWebhookStatus::AdjustAuthorizationFailed => Ok(Self::Failure),
            AdyenWebhookStatus::Cancelled => Ok(Self::Voided),
            AdyenWebhookStatus::CancelFailed => Ok(Self::VoidFailed),
            AdyenWebhookStatus::Captured => Ok(Self::Charged),
            AdyenWebhookStatus::CaptureFailed => Ok(Self::CaptureFailed),
            AdyenWebhookStatus::Expired => Ok(Self::Expired),
            //If Unexpected Event is received, need to understand how it reached this point
            //Webhooks with Payment Events only should try to conume this resource object.
            AdyenWebhookStatus::UnexpectedEvent | AdyenWebhookStatus::Reversed => {
                Err(report!(errors::ConnectorError::WebhookBodyDecodingFailed))
            }
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct AdyenRedirectRequest {
    pub details: AdyenRedirectRequestTypes,
}

#[derive(Debug, Clone, Serialize, serde::Deserialize, Eq, PartialEq)]
#[serde(untagged)]
pub enum AdyenRedirectRequestTypes {
    AdyenRedirection(AdyenRedirection),
    AdyenThreeDS(AdyenThreeDS),
    AdyenRefusal(AdyenRefusal),
}

#[derive(Debug, Clone, Serialize, serde::Deserialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AdyenRefusal {
    pub payload: String,
    #[serde(rename = "type")]
    pub type_of_redirection_result: Option<String>,
    pub result_code: Option<String>,
}

#[derive(Debug, Clone, Serialize, serde::Deserialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AdyenRedirection {
    pub redirect_result: String,
    #[serde(rename = "type")]
    pub type_of_redirection_result: Option<String>,
    pub result_code: Option<String>,
}

#[derive(Debug, Clone, Serialize, serde::Deserialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AdyenThreeDS {
    #[serde(rename = "threeDSResult")]
    pub three_ds_result: String,
    #[serde(rename = "type")]
    pub type_of_redirection_result: Option<String>,
    pub result_code: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum AdyenPaymentResponse {
    Response(Box<AdyenResponse>),
    PresentToShopper(Box<PresentToShopperResponse>),
    QrCodeResponse(Box<QrCodeResponseResponse>),
    RedirectionResponse(Box<RedirectionResponse>),
    RedirectionErrorResponse(Box<RedirectionErrorResponse>),
    WebhookResponse(Box<AdyenWebhookResponse>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdyenResponse {
    psp_reference: String,
    result_code: AdyenStatus,
    amount: Option<Amount>,
    merchant_reference: String,
    refusal_reason: Option<String>,
    refusal_reason_code: Option<String>,
    additional_data: Option<AdditionalData>,
    splits: Option<Vec<AdyenSplitData>>,
    store: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AdyenWebhookStatus {
    Authorised,
    AuthorisationFailed,
    Cancelled,
    CancelFailed,
    Captured,
    CaptureFailed,
    Reversed,
    UnexpectedEvent,
    Expired,
    AdjustedAuthorization,
    AdjustAuthorizationFailed,
}

//Creating custom struct which can be consumed in Psync Handler triggered from Webhooks
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdyenWebhookResponse {
    transaction_id: String,
    payment_reference: Option<String>,
    status: AdyenWebhookStatus,
    amount: Option<Amount>,
    merchant_reference_id: String,
    refusal_reason: Option<String>,
    refusal_reason_code: Option<String>,
    event_code: WebhookEventCode,
    #[serde(with = "common_utils::custom_serde::iso8601::option")]
    event_date: Option<PrimitiveDateTime>,
    // Raw acquirer refusal code
    refusal_code_raw: Option<String>,
    // Raw acquirer refusal reason
    refusal_reason_raw: Option<String>,
    recurring_detail_reference: Option<Secret<String>>,
    recurring_shopper_reference: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RedirectionErrorResponse {
    result_code: AdyenStatus,
    refusal_reason: Option<String>,
    psp_reference: Option<String>,
    merchant_reference: Option<String>,
    additional_data: Option<AdditionalData>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RedirectionResponse {
    result_code: AdyenStatus,
    action: AdyenRedirectAction,
    amount: Option<Amount>,
    refusal_reason: Option<String>,
    refusal_reason_code: Option<String>,
    psp_reference: Option<String>,
    merchant_reference: Option<String>,
    store: Option<String>,
    splits: Option<Vec<AdyenSplitData>>,
    additional_data: Option<AdditionalData>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PresentToShopperResponse {
    psp_reference: Option<String>,
    result_code: AdyenStatus,
    action: AdyenPtsAction,
    amount: Option<Amount>,
    refusal_reason: Option<String>,
    refusal_reason_code: Option<String>,
    merchant_reference: Option<String>,
    store: Option<String>,
    splits: Option<Vec<AdyenSplitData>>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QrCodeResponseResponse {
    result_code: AdyenStatus,
    action: AdyenQrCodeAction,
    amount: Option<Amount>,
    refusal_reason: Option<String>,
    refusal_reason_code: Option<String>,
    additional_data: Option<QrCodeAdditionalData>,
    psp_reference: Option<String>,
    merchant_reference: Option<String>,
    store: Option<String>,
    splits: Option<Vec<AdyenSplitData>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdyenQrCodeAction {
    payment_method_type: PaymentType,
    #[serde(rename = "type")]
    type_of_response: ActionType,
    #[serde(rename = "url")]
    qr_code_url: Option<Url>,
    qr_code_data: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QrCodeAdditionalData {
    #[serde(rename = "pix.expirationDate")]
    #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
    pix_expiration_date: Option<PrimitiveDateTime>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdyenPtsAction {
    reference: String,
    download_url: Option<Url>,
    payment_method_type: PaymentType,
    #[serde(rename = "expiresAt")]
    #[serde(
        default,
        with = "common_utils::custom_serde::iso8601::option_without_timezone"
    )]
    expires_at: Option<PrimitiveDateTime>,
    initial_amount: Option<Amount>,
    pass_creation_token: Option<String>,
    total_amount: Option<Amount>,
    #[serde(rename = "type")]
    type_of_response: ActionType,
    instructions_url: Option<Url>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdyenRedirectAction {
    payment_method_type: PaymentType,
    url: Option<Url>,
    method: Option<Method>,
    #[serde(rename = "type")]
    type_of_response: ActionType,
    data: Option<std::collections::HashMap<String, String>>,
    payment_data: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ActionType {
    Redirect,
    Await,
    #[serde(rename = "qrCode")]
    QrCode,
    Voucher,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct Amount {
    pub currency: storage_enums::Currency,
    pub value: MinorUnit,
}

#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
pub enum PaymentMethod<'a> {
    AdyenPaymentMethod(Box<AdyenPaymentMethod<'a>>),
    AdyenMandatePaymentMethod(Box<AdyenMandate>),
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type")]
#[serde(rename_all = "lowercase")]
pub enum AdyenPaymentMethod<'a> {
    #[serde(rename = "affirm")]
    AdyenAffirm,
    #[serde(rename = "scheme")]
    AdyenCard(Box<AdyenCard>),
    #[serde(rename = "klarna")]
    AdyenKlarna,
    #[serde(rename = "paypal")]
    AdyenPaypal,
    #[serde(rename = "networkToken")]
    AdyenPaze(Box<AdyenPazeData>),
    #[serde(rename = "afterpaytouch")]
    AfterPay,
    #[serde(rename = "alma")]
    AlmaPayLater,
    AliPay,
    #[serde(rename = "alipay_hk")]
    AliPayHk,
    ApplePay(Box<AdyenApplePay>),
    ApplePayDecrypt(Box<AdyenApplePayDecryptData>),
    Atome,
    #[serde(rename = "scheme")]
    BancontactCard(Box<AdyenCard>),
    Bizum,
    Blik(Box<BlikRedirectionData>),
    #[serde(rename = "boletobancario")]
    BoletoBancario,
    #[serde(rename = "clearpay")]
    ClearPay,
    #[serde(rename = "dana")]
    Dana,
    Eps(Box<BankRedirectionWithIssuer<'a>>),
    #[serde(rename = "gcash")]
    Gcash(Box<GcashData>),
    #[serde(rename = "googlepay")]
    Gpay(Box<AdyenGPay>),
    #[serde(rename = "gopay_wallet")]
    GoPay(Box<GoPayData>),
    Ideal,
    #[serde(rename = "kakaopay")]
    Kakaopay(Box<KakaoPayData>),
    Mbway(Box<MbwayData>),
    MobilePay,
    #[serde(rename = "momo_wallet")]
    Momo(Box<MomoData>),
    #[serde(rename = "momo_atm")]
    MomoAtm,
    #[serde(rename = "touchngo")]
    TouchNGo(Box<TouchNGoData>),
    #[serde(rename = "onlineBanking_CZ")]
    OnlineBankingCzechRepublic(Box<OnlineBankingCzechRepublicData>),
    #[serde(rename = "ebanking_FI")]
    OnlineBankingFinland,
    #[serde(rename = "onlineBanking_PL")]
    OnlineBankingPoland(Box<OnlineBankingPolandData>),
    #[serde(rename = "onlineBanking_SK")]
    OnlineBankingSlovakia(Box<OnlineBankingSlovakiaData>),
    #[serde(rename = "molpay_ebanking_fpx_MY")]
    OnlineBankingFpx(Box<OnlineBankingFpxData>),
    #[serde(rename = "molpay_ebanking_TH")]
    OnlineBankingThailand(Box<OnlineBankingThailandData>),
    #[serde(rename = "paybybank")]
    OpenBankingUK(Box<OpenBankingUKData>),
    #[serde(rename = "oxxo")]
    Oxxo,
    #[serde(rename = "paysafecard")]
    PaySafeCard,
    #[serde(rename = "paybright")]
    PayBright,
    #[serde(rename = "doku_permata_lite_atm")]
    PermataBankTransfer(Box<DokuBankData>),
    #[serde(rename = "trustly")]
    Trustly,
    #[serde(rename = "walley")]
    Walley,
    #[serde(rename = "wechatpayWeb")]
    WeChatPayWeb,
    #[serde(rename = "ach")]
    AchDirectDebit(Box<AchDirectDebitData>),
    #[serde(rename = "sepadirectdebit")]
    SepaDirectDebit(Box<SepaDirectDebitData>),
    #[serde(rename = "directdebit_GB")]
    BacsDirectDebit(Box<BacsDirectDebitData>),
    SamsungPay(Box<SamsungPayPmData>),
    #[serde(rename = "doku_bca_va")]
    BcaBankTransfer(Box<DokuBankData>),
    #[serde(rename = "doku_bni_va")]
    BniVa(Box<DokuBankData>),
    #[serde(rename = "doku_bri_va")]
    BriVa(Box<DokuBankData>),
    #[serde(rename = "doku_cimb_va")]
    CimbVa(Box<DokuBankData>),
    #[serde(rename = "doku_danamon_va")]
    DanamonVa(Box<DokuBankData>),
    #[serde(rename = "doku_mandiri_va")]
    MandiriVa(Box<DokuBankData>),
    #[serde(rename = "twint")]
    Twint,
    #[serde(rename = "vipps")]
    Vipps,
    #[serde(rename = "doku_indomaret")]
    Indomaret(Box<DokuBankData>),
    #[serde(rename = "doku_alfamart")]
    Alfamart(Box<DokuBankData>),
    #[serde(rename = "givex")]
    PaymentMethodBalance(Box<BalancePmData>),
    #[serde(rename = "giftcard")]
    AdyenGiftCard(Box<AdyenGiftCardData>),
    #[serde(rename = "swish")]
    Swish,
    #[serde(rename = "benefit")]
    Benefit,
    #[serde(rename = "knet")]
    Knet,
    #[serde(rename = "econtext_seven_eleven")]
    SevenEleven(Box<JCSVoucherData>),
    #[serde(rename = "econtext_stores")]
    JapaneseConvenienceStores(Box<JCSVoucherData>),
    Pix,
    #[serde(rename = "networkToken")]
    NetworkToken(Box<AdyenNetworkTokenData>),
}

#[derive(Debug, Clone, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JCSVoucherData {
    first_name: Secret<String>,
    last_name: Option<Secret<String>>,
    shopper_email: Email,
    telephone_number: Secret<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BalancePmData {
    number: Secret<String>,
    cvc: Secret<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AdyenGiftCardData {
    brand: GiftCardBrand,
    number: Secret<String>,
    cvc: Secret<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AchDirectDebitData {
    bank_account_number: Secret<String>,
    bank_location_id: Secret<String>,
    owner_name: Secret<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SepaDirectDebitData {
    #[serde(rename = "sepa.ownerName")]
    owner_name: Secret<String>,
    #[serde(rename = "sepa.ibanNumber")]
    iban_number: Secret<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BacsDirectDebitData {
    bank_account_number: Secret<String>,
    bank_location_id: Secret<String>,
    holder_name: Secret<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MbwayData {
    telephone_number: Secret<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SamsungPayPmData {
    #[serde(rename = "samsungPayToken")]
    samsung_pay_token: Secret<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct OnlineBankingCzechRepublicData {
    issuer: OnlineBankingCzechRepublicBanks,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum OnlineBankingCzechRepublicBanks {
    KB,
    CS,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AdyenPlatformChargeBackBehaviour {
    #[serde(alias = "deduct_from_liable_account")]
    DeductFromLiableAccount,
    #[serde(alias = "deduct_from_one_balance_account")]
    DeductFromOneBalanceAccount,
    #[serde(alias = "deduct_according_to_split_ratio")]
    DeductAccordingToSplitRatio,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdyenPlatformChargeBackLogicMetadata {
    pub behavior: Option<AdyenPlatformChargeBackBehaviour>,
    #[serde(alias = "target_account")]
    pub target_account: Option<Secret<String>>,
    #[serde(alias = "cost_allocation_account")]
    pub cost_allocation_account: Option<Secret<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct AdyenMetadata {
    #[serde(alias = "device_fingerprint")]
    pub device_fingerprint: Option<Secret<String>>,
    pub store: Option<String>,
    #[serde(alias = "platform_chargeback_logic")]
    pub platform_chargeback_logic: Option<AdyenPlatformChargeBackLogicMetadata>,
}

fn filter_adyen_metadata(metadata: serde_json::Value) -> serde_json::Value {
    if let serde_json::Value::Object(mut map) = metadata.clone() {
        // Remove the fields that are specific to Adyen and should not be passed in metadata
        map.remove("device_fingerprint");
        map.remove("deviceFingerprint");
        map.remove("platform_chargeback_logic");
        map.remove("platformChargebackLogic");
        map.remove("store");

        serde_json::Value::Object(map)
    } else {
        metadata.clone()
    }
}
impl TryFrom<&PaymentsAuthorizeRouterData> for JCSVoucherData {
    type Error = Error;
    fn try_from(item: &PaymentsAuthorizeRouterData) -> Result<Self, Self::Error> {
        Ok(Self {
            first_name: item.get_billing_first_name()?,
            last_name: item.get_optional_billing_last_name(),
            shopper_email: item.get_billing_email()?,
            telephone_number: item.get_billing_phone_number()?,
        })
    }
}

impl TryFrom<&common_enums::BankNames> for OnlineBankingCzechRepublicBanks {
    type Error = Error;
    fn try_from(bank_name: &common_enums::BankNames) -> Result<Self, Self::Error> {
        match bank_name {
            common_enums::BankNames::KomercniBanka => Ok(Self::KB),
            common_enums::BankNames::CeskaSporitelna => Ok(Self::CS),
            _ => Err(errors::ConnectorError::NotImplemented(
                utils::get_unimplemented_payment_method_error_message("Adyen"),
            ))?,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct OnlineBankingPolandData {
    issuer: OnlineBankingPolandBanks,
}

#[derive(Debug, Clone, Serialize)]
pub enum OnlineBankingPolandBanks {
    #[serde(rename = "154")]
    BlikPSP,
    #[serde(rename = "31")]
    PlaceZIPKO,
    #[serde(rename = "243")]
    MBank,
    #[serde(rename = "112")]
    PayWithING,
    #[serde(rename = "20")]
    SantanderPrzelew24,
    #[serde(rename = "65")]
    BankPEKAOSA,
    #[serde(rename = "85")]
    BankMillennium,
    #[serde(rename = "88")]
    PayWithAliorBank,
    #[serde(rename = "143")]
    BankiSpoldzielcze,
    #[serde(rename = "26")]
    PayWithInteligo,
    #[serde(rename = "33")]
    BNPParibasPoland,
    #[serde(rename = "144")]
    BankNowySA,
    #[serde(rename = "45")]
    CreditAgricole,
    #[serde(rename = "99")]
    PayWithBOS,
    #[serde(rename = "119")]
    PayWithCitiHandlowy,
    #[serde(rename = "131")]
    PayWithPlusBank,
    #[serde(rename = "64")]
    ToyotaBank,
    #[serde(rename = "153")]
    VeloBank,
    #[serde(rename = "141")]
    ETransferPocztowy24,
}

impl TryFrom<&common_enums::BankNames> for OnlineBankingPolandBanks {
    type Error = Error;
    fn try_from(bank_name: &common_enums::BankNames) -> Result<Self, Self::Error> {
        match bank_name {
            common_enums::BankNames::BlikPSP => Ok(Self::BlikPSP),
            common_enums::BankNames::PlaceZIPKO => Ok(Self::PlaceZIPKO),
            common_enums::BankNames::MBank => Ok(Self::MBank),
            common_enums::BankNames::PayWithING => Ok(Self::PayWithING),
            common_enums::BankNames::SantanderPrzelew24 => Ok(Self::SantanderPrzelew24),
            common_enums::BankNames::BankPEKAOSA => Ok(Self::BankPEKAOSA),
            common_enums::BankNames::BankMillennium => Ok(Self::BankMillennium),
            common_enums::BankNames::PayWithAliorBank => Ok(Self::PayWithAliorBank),
            common_enums::BankNames::BankiSpoldzielcze => Ok(Self::BankiSpoldzielcze),
            common_enums::BankNames::PayWithInteligo => Ok(Self::PayWithInteligo),
            common_enums::BankNames::BNPParibasPoland => Ok(Self::BNPParibasPoland),
            common_enums::BankNames::BankNowySA => Ok(Self::BankNowySA),
            common_enums::BankNames::CreditAgricole => Ok(Self::CreditAgricole),
            common_enums::BankNames::PayWithBOS => Ok(Self::PayWithBOS),
            common_enums::BankNames::PayWithCitiHandlowy => Ok(Self::PayWithCitiHandlowy),
            common_enums::BankNames::PayWithPlusBank => Ok(Self::PayWithPlusBank),
            common_enums::BankNames::ToyotaBank => Ok(Self::ToyotaBank),
            common_enums::BankNames::VeloBank => Ok(Self::VeloBank),
            common_enums::BankNames::ETransferPocztowy24 => Ok(Self::ETransferPocztowy24),
            _ => Err(errors::ConnectorError::NotImplemented(
                utils::get_unimplemented_payment_method_error_message("Adyen"),
            ))?,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OnlineBankingSlovakiaData {
    issuer: OnlineBankingSlovakiaBanks,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OnlineBankingFpxData {
    issuer: OnlineBankingFpxIssuer,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OnlineBankingThailandData {
    issuer: OnlineBankingThailandIssuer,
}

#[derive(Debug, Clone, Serialize)]
pub struct OpenBankingUKData {
    issuer: Option<OpenBankingUKIssuer>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum OnlineBankingSlovakiaBanks {
    Vub,
    Posto,
    Sporo,
    Tatra,
    Viamo,
}

impl TryFrom<&common_enums::BankNames> for OnlineBankingSlovakiaBanks {
    type Error = Error;
    fn try_from(bank_name: &common_enums::BankNames) -> Result<Self, Self::Error> {
        match bank_name {
            common_enums::BankNames::EPlatbyVUB => Ok(Self::Vub),
            common_enums::BankNames::PostovaBanka => Ok(Self::Posto),
            common_enums::BankNames::SporoPay => Ok(Self::Sporo),
            common_enums::BankNames::TatraPay => Ok(Self::Tatra),
            common_enums::BankNames::Viamo => Ok(Self::Viamo),
            _ => Err(errors::ConnectorError::NotImplemented(
                utils::get_unimplemented_payment_method_error_message("Adyen"),
            ))?,
        }
    }
}

impl TryFrom<&common_enums::BankNames> for OnlineBankingFpxIssuer {
    type Error = Error;
    fn try_from(bank_name: &common_enums::BankNames) -> Result<Self, Self::Error> {
        match bank_name {
            common_enums::BankNames::AffinBank => Ok(Self::FpxAbb),
            common_enums::BankNames::AgroBank => Ok(Self::FpxAgrobank),
            common_enums::BankNames::AllianceBank => Ok(Self::FpxAbmb),
            common_enums::BankNames::AmBank => Ok(Self::FpxAmb),
            common_enums::BankNames::BankIslam => Ok(Self::FpxBimb),
            common_enums::BankNames::BankMuamalat => Ok(Self::FpxBmmb),
            common_enums::BankNames::BankRakyat => Ok(Self::FpxBkrm),
            common_enums::BankNames::BankSimpananNasional => Ok(Self::FpxBsn),
            common_enums::BankNames::CimbBank => Ok(Self::FpxCimbclicks),
            common_enums::BankNames::HongLeongBank => Ok(Self::FpxHlb),
            common_enums::BankNames::HsbcBank => Ok(Self::FpxHsbc),
            common_enums::BankNames::KuwaitFinanceHouse => Ok(Self::FpxKfh),
            common_enums::BankNames::Maybank => Ok(Self::FpxMb2u),
            common_enums::BankNames::OcbcBank => Ok(Self::FpxOcbc),
            common_enums::BankNames::PublicBank => Ok(Self::FpxPbb),
            common_enums::BankNames::RhbBank => Ok(Self::FpxRhb),
            common_enums::BankNames::StandardCharteredBank => Ok(Self::FpxScb),
            common_enums::BankNames::UobBank => Ok(Self::FpxUob),
            _ => Err(errors::ConnectorError::NotImplemented(
                utils::get_unimplemented_payment_method_error_message("Adyen"),
            ))?,
        }
    }
}

impl TryFrom<&common_enums::BankNames> for OnlineBankingThailandIssuer {
    type Error = Error;
    fn try_from(bank_name: &common_enums::BankNames) -> Result<Self, Self::Error> {
        match bank_name {
            common_enums::BankNames::BangkokBank => Ok(Self::Bangkokbank),
            common_enums::BankNames::KrungsriBank => Ok(Self::Krungsribank),
            common_enums::BankNames::KrungThaiBank => Ok(Self::Krungthaibank),
            common_enums::BankNames::TheSiamCommercialBank => Ok(Self::Siamcommercialbank),
            common_enums::BankNames::KasikornBank => Ok(Self::Kbank),
            _ => Err(errors::ConnectorError::NotImplemented(
                utils::get_unimplemented_payment_method_error_message("Adyen"),
            ))?,
        }
    }
}

impl TryFrom<&common_enums::BankNames> for OpenBankingUKIssuer {
    type Error = Error;
    fn try_from(bank_name: &common_enums::BankNames) -> Result<Self, Self::Error> {
        match bank_name {
            common_enums::BankNames::OpenBankSuccess => Ok(Self::RedirectSuccess),
            common_enums::BankNames::OpenBankFailure => Ok(Self::RedirectFailure),
            common_enums::BankNames::OpenBankCancelled => Ok(Self::RedirectCancelled),
            common_enums::BankNames::Aib => Ok(Self::Aib),
            common_enums::BankNames::BankOfScotland => Ok(Self::BankOfScotland),
            common_enums::BankNames::Barclays => Ok(Self::Barclays),
            common_enums::BankNames::DanskeBank => Ok(Self::DanskeBank),
            common_enums::BankNames::FirstDirect => Ok(Self::FirstDirect),
            common_enums::BankNames::FirstTrust => Ok(Self::FirstTrust),
            common_enums::BankNames::HsbcBank => Ok(Self::HsbcBank),
            common_enums::BankNames::Halifax => Ok(Self::Halifax),
            common_enums::BankNames::Lloyds => Ok(Self::Lloyds),
            common_enums::BankNames::Monzo => Ok(Self::Monzo),
            common_enums::BankNames::NatWest => Ok(Self::NatWest),
            common_enums::BankNames::NationwideBank => Ok(Self::NationwideBank),
            common_enums::BankNames::Revolut => Ok(Self::Revolut),
            common_enums::BankNames::RoyalBankOfScotland => Ok(Self::RoyalBankOfScotland),
            common_enums::BankNames::SantanderPrzelew24 => Ok(Self::SantanderPrzelew24),
            common_enums::BankNames::Starling => Ok(Self::Starling),
            common_enums::BankNames::TsbBank => Ok(Self::TsbBank),
            common_enums::BankNames::TescoBank => Ok(Self::TescoBank),
            common_enums::BankNames::UlsterBank => Ok(Self::UlsterBank),
            common_enums::BankNames::AmericanExpress
            | common_enums::BankNames::AffinBank
            | common_enums::BankNames::AgroBank
            | common_enums::BankNames::AllianceBank
            | common_enums::BankNames::AmBank
            | common_enums::BankNames::BankOfAmerica
            | common_enums::BankNames::BankOfChina
            | common_enums::BankNames::BankIslam
            | common_enums::BankNames::BankMuamalat
            | common_enums::BankNames::BankRakyat
            | common_enums::BankNames::BankSimpananNasional
            | common_enums::BankNames::BlikPSP
            | common_enums::BankNames::CapitalOne
            | common_enums::BankNames::Chase
            | common_enums::BankNames::Citi
            | common_enums::BankNames::CimbBank
            | common_enums::BankNames::Discover
            | common_enums::BankNames::NavyFederalCreditUnion
            | common_enums::BankNames::PentagonFederalCreditUnion
            | common_enums::BankNames::SynchronyBank
            | common_enums::BankNames::WellsFargo
            | common_enums::BankNames::AbnAmro
            | common_enums::BankNames::AsnBank
            | common_enums::BankNames::Bunq
            | common_enums::BankNames::Handelsbanken
            | common_enums::BankNames::HongLeongBank
            | common_enums::BankNames::Ing
            | common_enums::BankNames::Knab
            | common_enums::BankNames::KuwaitFinanceHouse
            | common_enums::BankNames::Moneyou
            | common_enums::BankNames::Rabobank
            | common_enums::BankNames::Regiobank
            | common_enums::BankNames::SnsBank
            | common_enums::BankNames::TriodosBank
            | common_enums::BankNames::VanLanschot
            | common_enums::BankNames::ArzteUndApothekerBank
            | common_enums::BankNames::AustrianAnadiBankAg
            | common_enums::BankNames::BankAustria
            | common_enums::BankNames::Bank99Ag
            | common_enums::BankNames::BankhausCarlSpangler
            | common_enums::BankNames::BankhausSchelhammerUndSchatteraAg
            | common_enums::BankNames::BankMillennium
            | common_enums::BankNames::BankPEKAOSA
            | common_enums::BankNames::BawagPskAg
            | common_enums::BankNames::BksBankAg
            | common_enums::BankNames::BrullKallmusBankAg
            | common_enums::BankNames::BtvVierLanderBank
            | common_enums::BankNames::CapitalBankGraweGruppeAg
            | common_enums::BankNames::CeskaSporitelna
            | common_enums::BankNames::Dolomitenbank
            | common_enums::BankNames::EasybankAg
            | common_enums::BankNames::EPlatbyVUB
            | common_enums::BankNames::ErsteBankUndSparkassen
            | common_enums::BankNames::FrieslandBank
            | common_enums::BankNames::HypoAlpeadriabankInternationalAg
            | common_enums::BankNames::HypoNoeLbFurNiederosterreichUWien
            | common_enums::BankNames::HypoOberosterreichSalzburgSteiermark
            | common_enums::BankNames::HypoTirolBankAg
            | common_enums::BankNames::HypoVorarlbergBankAg
            | common_enums::BankNames::HypoBankBurgenlandAktiengesellschaft
            | common_enums::BankNames::KomercniBanka
            | common_enums::BankNames::MBank
            | common_enums::BankNames::MarchfelderBank
            | common_enums::BankNames::Maybank
            | common_enums::BankNames::OberbankAg
            | common_enums::BankNames::OsterreichischeArzteUndApothekerbank
            | common_enums::BankNames::OcbcBank
            | common_enums::BankNames::PayWithING
            | common_enums::BankNames::PlaceZIPKO
            | common_enums::BankNames::PlatnoscOnlineKartaPlatnicza
            | common_enums::BankNames::PosojilnicaBankEGen
            | common_enums::BankNames::PostovaBanka
            | common_enums::BankNames::PublicBank
            | common_enums::BankNames::RaiffeisenBankengruppeOsterreich
            | common_enums::BankNames::RhbBank
            | common_enums::BankNames::SchelhammerCapitalBankAg
            | common_enums::BankNames::StandardCharteredBank
            | common_enums::BankNames::SchoellerbankAg
            | common_enums::BankNames::SpardaBankWien
            | common_enums::BankNames::SporoPay
            | common_enums::BankNames::TatraPay
            | common_enums::BankNames::Viamo
            | common_enums::BankNames::VolksbankGruppe
            | common_enums::BankNames::VolkskreditbankAg
            | common_enums::BankNames::VrBankBraunau
            | common_enums::BankNames::UobBank
            | common_enums::BankNames::PayWithAliorBank
            | common_enums::BankNames::BankiSpoldzielcze
            | common_enums::BankNames::PayWithInteligo
            | common_enums::BankNames::BNPParibasPoland
            | common_enums::BankNames::BankNowySA
            | common_enums::BankNames::CreditAgricole
            | common_enums::BankNames::PayWithBOS
            | common_enums::BankNames::PayWithCitiHandlowy
            | common_enums::BankNames::PayWithPlusBank
            | common_enums::BankNames::ToyotaBank
            | common_enums::BankNames::VeloBank
            | common_enums::BankNames::ETransferPocztowy24
            | common_enums::BankNames::PlusBank
            | common_enums::BankNames::EtransferPocztowy24
            | common_enums::BankNames::BankiSpbdzielcze
            | common_enums::BankNames::BankNowyBfgSa
            | common_enums::BankNames::GetinBank
            | common_enums::BankNames::Blik
            | common_enums::BankNames::NoblePay
            | common_enums::BankNames::IdeaBank
            | common_enums::BankNames::EnveloBank
            | common_enums::BankNames::NestPrzelew
            | common_enums::BankNames::MbankMtransfer
            | common_enums::BankNames::Inteligo
            | common_enums::BankNames::PbacZIpko
            | common_enums::BankNames::BnpParibas
            | common_enums::BankNames::BankPekaoSa
            | common_enums::BankNames::VolkswagenBank
            | common_enums::BankNames::AliorBank
            | common_enums::BankNames::Boz
            | common_enums::BankNames::BangkokBank
            | common_enums::BankNames::KrungsriBank
            | common_enums::BankNames::KrungThaiBank
            | common_enums::BankNames::TheSiamCommercialBank
            | common_enums::BankNames::Yoursafe
            | common_enums::BankNames::N26
            | common_enums::BankNames::NationaleNederlanden
            | common_enums::BankNames::KasikornBank => {
                Err(errors::ConnectorError::NotImplemented(
                    utils::get_unimplemented_payment_method_error_message("Adyen"),
                ))?
            }
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BlikRedirectionData {
    blik_code: Secret<String>,
}

#[serde_with::skip_serializing_none]
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BankRedirectionWithIssuer<'a> {
    issuer: Option<&'a str>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdyenMandate {
    #[serde(rename = "type")]
    payment_type: PaymentType,
    stored_payment_method_id: Secret<String>,
    holder_name: Option<Secret<String>>,
}

#[serde_with::skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdyenCard {
    number: CardNumber,
    expiry_month: Secret<String>,
    expiry_year: Secret<String>,
    cvc: Option<Secret<String>>,
    holder_name: Option<Secret<String>>,
    brand: Option<CardBrand>, //Mandatory for mandate using network_txns_id
    network_payment_reference: Option<Secret<String>>,
}

#[serde_with::skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdyenPazeData {
    number: NetworkTokenNumber,
    expiry_month: Secret<String>,
    expiry_year: Secret<String>,
    cvc: Option<Secret<String>>,
    holder_name: Option<Secret<String>>,
    brand: Option<CardBrand>, //Mandatory for mandate using network_txns_id
    network_payment_reference: Option<Secret<String>>,
}

#[serde_with::skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdyenApplePayDecryptData {
    number: CardNumber,
    expiry_month: Secret<String>,
    expiry_year: Secret<String>,
    brand: String,
    #[serde(rename = "type")]
    payment_type: PaymentType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CardBrand {
    Visa,
    MC,
    Amex,
    Accel,
    Argencard,
    Bcmc,
    Bijcard,
    Cabal,
    Cartebancaire,
    Codensa,
    Cup,
    Dankort,
    Diners,
    Discover,
    Electron,
    Elo,
    Forbrugsforeningen,
    Hiper,
    Hipercard,
    Jcb,
    Karenmillen,
    Laser,
    Maestro,
    Maestrouk,
    Mcalphabankbonus,
    Mir,
    Naranja,
    Oasis,
    Pulse,
    Rupay,
    Shopping,
    Star,
    Solo,
    Troy,
    Uatp,
    Visaalphabankbonus,
    Visadankort,
    Nyce,
    Warehouse,
}

#[derive(Default, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdyenCancelRequest {
    merchant_account: Secret<String>,
    reference: String,
}

#[derive(Default, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AdyenCancelResponse {
    payment_psp_reference: String,
    status: CancelStatus,
    reference: String,
}

#[derive(Default, Debug, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum CancelStatus {
    Received,
    #[default]
    Processing,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoPayData {}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KakaoPayData {}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GcashData {}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MomoData {}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TouchNGoData {}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdyenGPay {
    #[serde(rename = "googlePayToken")]
    google_pay_token: Secret<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdyenApplePay {
    #[serde(rename = "applePayToken")]
    apple_pay_token: Secret<String>,
}

#[serde_with::skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdyenNetworkTokenData {
    number: NetworkTokenNumber,
    expiry_month: Secret<String>,
    expiry_year: Secret<String>,
    holder_name: Option<Secret<String>>,
    brand: Option<CardBrand>, //Mandatory for mandate using network_txns_id
    network_payment_reference: Option<Secret<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DokuBankData {
    first_name: Secret<String>,
    last_name: Option<Secret<String>>,
    shopper_email: Email,
}
// Refunds Request and Response
#[serde_with::skip_serializing_none]
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdyenRefundRequest {
    merchant_account: Secret<String>,
    amount: Amount,
    merchant_refund_reason: Option<AdyenRefundRequestReason>,
    reference: String,
    splits: Option<Vec<AdyenSplitData>>,
    /// refund_connector_metadata.store
    store: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum AdyenRefundRequestReason {
    FRAUD,
    #[serde(rename = "CUSTOMER REQUEST")]
    CUSTOMERREQUEST,
    RETURN,
    DUPLICATE,
    OTHER,
}

impl FromStr for AdyenRefundRequestReason {
    type Err = error_stack::Report<errors::ConnectorError>;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_uppercase().as_str() {
            "FRAUD" => Ok(Self::FRAUD),
            "CUSTOMER REQUEST" | "CUSTOMERREQUEST" => Ok(Self::CUSTOMERREQUEST),
            "RETURN" => Ok(Self::RETURN),
            "DUPLICATE" => Ok(Self::DUPLICATE),
            "OTHER" => Ok(Self::OTHER),
            _ => Ok(Self::OTHER),
        }
    }
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdyenRefundResponse {
    merchant_account: Secret<String>,
    psp_reference: String,
    payment_psp_reference: String,
    reference: String,
    status: String,
}

pub struct AdyenAuthType {
    pub(super) api_key: Secret<String>,
    pub(super) merchant_account: Secret<String>,
    #[allow(dead_code)]
    pub(super) review_key: Option<Secret<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PaymentType {
    Affirm,
    Afterpaytouch,
    Alipay,
    #[serde(rename = "alipay_hk")]
    AlipayHk,
    #[serde(rename = "doku_alfamart")]
    Alfamart,
    Alma,
    Applepay,
    Bizum,
    Atome,
    Blik,
    #[serde(rename = "boletobancario")]
    BoletoBancario,
    ClearPay,
    Dana,
    Eps,
    Gcash,
    Googlepay,
    #[serde(rename = "gopay_wallet")]
    GoPay,
    Ideal,
    #[serde(rename = "doku_indomaret")]
    Indomaret,
    Klarna,
    Kakaopay,
    Mbway,
    MobilePay,
    #[serde(rename = "momo_wallet")]
    Momo,
    #[serde(rename = "momo_atm")]
    MomoAtm,
    #[serde(rename = "onlineBanking_CZ")]
    OnlineBankingCzechRepublic,
    #[serde(rename = "ebanking_FI")]
    OnlineBankingFinland,
    #[serde(rename = "onlineBanking_PL")]
    OnlineBankingPoland,
    #[serde(rename = "onlineBanking_SK")]
    OnlineBankingSlovakia,
    #[serde(rename = "molpay_ebanking_fpx_MY")]
    OnlineBankingFpx,
    #[serde(rename = "molpay_ebanking_TH")]
    OnlineBankingThailand,
    #[serde(rename = "paybybank")]
    OpenBankingUK,
    #[serde(rename = "oxxo")]
    Oxxo,
    #[serde(rename = "paysafecard")]
    PaySafeCard,
    PayBright,
    Paypal,
    Scheme,
    #[serde(rename = "networkToken")]
    NetworkToken,
    #[serde(rename = "trustly")]
    Trustly,
    #[serde(rename = "touchngo")]
    TouchNGo,
    Walley,
    #[serde(rename = "wechatpayWeb")]
    WeChatPayWeb,
    #[serde(rename = "ach")]
    AchDirectDebit,
    SepaDirectDebit,
    #[serde(rename = "directdebit_GB")]
    BacsDirectDebit,
    Samsungpay,
    Twint,
    Vipps,
    Giftcard,
    Knet,
    Benefit,
    Swish,
    #[serde(rename = "doku_permata_lite_atm")]
    PermataBankTransfer,
    #[serde(rename = "doku_bca_va")]
    BcaBankTransfer,
    #[serde(rename = "doku_bni_va")]
    BniVa,
    #[serde(rename = "doku_bri_va")]
    BriVa,
    #[serde(rename = "doku_cimb_va")]
    CimbVa,
    #[serde(rename = "doku_danamon_va")]
    DanamonVa,
    #[serde(rename = "doku_mandiri_va")]
    MandiriVa,
    #[serde(rename = "econtext_seven_eleven")]
    SevenEleven,
    #[serde(rename = "econtext_stores")]
    JapaneseConvenienceStores,
    Pix,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum GiftCardBrand {
    Givex,
    Auriga,
    Babygiftcard,
}

#[derive(Debug, Eq, PartialEq, Serialize, Clone)]
#[serde(rename_all = "snake_case")]
pub enum OnlineBankingFpxIssuer {
    FpxAbb,
    FpxAgrobank,
    FpxAbmb,
    FpxAmb,
    FpxBimb,
    FpxBmmb,
    FpxBkrm,
    FpxBsn,
    FpxCimbclicks,
    FpxHlb,
    FpxHsbc,
    FpxKfh,
    FpxMb2u,
    FpxOcbc,
    FpxPbb,
    FpxRhb,
    FpxScb,
    FpxUob,
}

#[derive(Debug, Eq, PartialEq, Serialize, Clone)]
pub enum OnlineBankingThailandIssuer {
    #[serde(rename = "molpay_bangkokbank")]
    Bangkokbank,
    #[serde(rename = "molpay_krungsribank")]
    Krungsribank,
    #[serde(rename = "molpay_krungthaibank")]
    Krungthaibank,
    #[serde(rename = "molpay_siamcommercialbank")]
    Siamcommercialbank,
    #[serde(rename = "molpay_kbank")]
    Kbank,
}

#[derive(Debug, Eq, PartialEq, Serialize, Clone)]
pub enum OpenBankingUKIssuer {
    #[serde(rename = "uk-test-open-banking-redirect")]
    RedirectSuccess,
    #[serde(rename = "uk-test-open-banking-redirect-failed")]
    RedirectFailure,
    #[serde(rename = "uk-test-open-banking-redirect-cancelled")]
    RedirectCancelled,
    #[serde(rename = "uk-aib-oauth2")]
    Aib,
    #[serde(rename = "uk-bankofscotland-oauth2")]
    BankOfScotland,
    #[serde(rename = "uk-barclays-oauth2")]
    Barclays,
    #[serde(rename = "uk-danskebank-oauth2")]
    DanskeBank,
    #[serde(rename = "uk-firstdirect-oauth2")]
    FirstDirect,
    #[serde(rename = "uk-firsttrust-oauth2")]
    FirstTrust,
    #[serde(rename = "uk-hsbc-oauth2")]
    HsbcBank,
    #[serde(rename = "uk-halifax-oauth2")]
    Halifax,
    #[serde(rename = "uk-lloyds-oauth2")]
    Lloyds,
    #[serde(rename = "uk-monzo-oauth2")]
    Monzo,
    #[serde(rename = "uk-natwest-oauth2")]
    NatWest,
    #[serde(rename = "uk-nationwide-oauth2")]
    NationwideBank,
    #[serde(rename = "uk-revolut-oauth2")]
    Revolut,
    #[serde(rename = "uk-rbs-oauth2")]
    RoyalBankOfScotland,
    #[serde(rename = "uk-santander-oauth2")]
    SantanderPrzelew24,
    #[serde(rename = "uk-starling-oauth2")]
    Starling,
    #[serde(rename = "uk-tsb-oauth2")]
    TsbBank,
    #[serde(rename = "uk-tesco-oauth2")]
    TescoBank,
    #[serde(rename = "uk-ulster-oauth2")]
    UlsterBank,
}

pub struct AdyenTestBankNames<'a>(&'a str);

impl TryFrom<&common_enums::BankNames> for AdyenTestBankNames<'_> {
    type Error = Error;
    fn try_from(bank: &common_enums::BankNames) -> Result<Self, Self::Error> {
        Ok(match bank {
            common_enums::BankNames::AbnAmro => Self("1121"),
            common_enums::BankNames::AsnBank => Self("1151"),
            common_enums::BankNames::Bunq => Self("1152"),
            common_enums::BankNames::Ing => Self("1154"),
            common_enums::BankNames::Knab => Self("1155"),
            common_enums::BankNames::N26 => Self("1156"),
            common_enums::BankNames::NationaleNederlanden => Self("1157"),
            common_enums::BankNames::Rabobank => Self("1157"),
            common_enums::BankNames::Regiobank => Self("1158"),
            common_enums::BankNames::Revolut => Self("1159"),
            common_enums::BankNames::SnsBank => Self("1159"),
            common_enums::BankNames::TriodosBank => Self("1159"),
            common_enums::BankNames::VanLanschot => Self("1159"),
            common_enums::BankNames::Yoursafe => Self("1159"),
            common_enums::BankNames::BankAustria => Self("e6819e7a-f663-414b-92ec-cf7c82d2f4e5"),
            common_enums::BankNames::BawagPskAg => Self("ba7199cc-f057-42f2-9856-2378abf21638"),
            common_enums::BankNames::Dolomitenbank => Self("d5d5b133-1c0d-4c08-b2be-3c9b116dc326"),
            common_enums::BankNames::EasybankAg => Self("eff103e6-843d-48b7-a6e6-fbd88f511b11"),
            common_enums::BankNames::ErsteBankUndSparkassen => {
                Self("3fdc41fc-3d3d-4ee3-a1fe-cd79cfd58ea3")
            }
            common_enums::BankNames::HypoTirolBankAg => {
                Self("6765e225-a0dc-4481-9666-e26303d4f221")
            }
            common_enums::BankNames::PosojilnicaBankEGen => {
                Self("65ef4682-4944-499f-828f-5d74ad288376")
            }
            common_enums::BankNames::RaiffeisenBankengruppeOsterreich => {
                Self("ee9fc487-ebe0-486c-8101-17dce5141a67")
            }
            common_enums::BankNames::SchoellerbankAg => {
                Self("1190c4d1-b37a-487e-9355-e0a067f54a9f")
            }
            common_enums::BankNames::SpardaBankWien => Self("8b0bfeea-fbb0-4337-b3a1-0e25c0f060fc"),
            common_enums::BankNames::VolksbankGruppe => {
                Self("e2e97aaa-de4c-4e18-9431-d99790773433")
            }
            common_enums::BankNames::VolkskreditbankAg => {
                Self("4a0a975b-0594-4b40-9068-39f77b3a91f9")
            }
            _ => Err(errors::ConnectorError::NotImplemented(
                utils::get_unimplemented_payment_method_error_message("Adyen"),
            ))?,
        })
    }
}

impl TryFrom<&ConnectorAuthType> for AdyenAuthType {
    type Error = Error;
    fn try_from(auth_type: &ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            ConnectorAuthType::BodyKey { api_key, key1 } => Ok(Self {
                api_key: api_key.to_owned(),
                merchant_account: key1.to_owned(),
                review_key: None,
            }),
            ConnectorAuthType::SignatureKey {
                api_key,
                key1,
                api_secret,
            } => Ok(Self {
                api_key: api_key.to_owned(),
                merchant_account: key1.to_owned(),
                review_key: Some(api_secret.to_owned()),
            }),
            _ => Err(errors::ConnectorError::FailedToObtainAuthType)?,
        }
    }
}

fn get_application_info(
    item: &AdyenRouterData<&PaymentsAuthorizeRouterData>,
) -> Option<ApplicationInfo> {
    item.router_data
        .request
        .partner_merchant_identifier_details
        .as_ref()
        .map(|partner_merchant_identifier_details| ApplicationInfo {
            merchant_application: partner_merchant_identifier_details
                .merchant_details
                .as_ref()
                .map(|merchant_details| MerchantApplication {
                    name: merchant_details.name.clone(),
                    version: merchant_details.version.clone(),
                }),
            external_platform: partner_merchant_identifier_details
                .partner_details
                .as_ref()
                .map(|platform_details| ExternalPlatform {
                    name: platform_details.name.clone(),
                    version: platform_details.version.clone(),
                    integrator: platform_details.integrator.clone(),
                }),
        })
}

impl TryFrom<&AdyenRouterData<&PaymentsAuthorizeRouterData>> for AdyenPaymentRequest<'_> {
    type Error = Error;
    fn try_from(item: &AdyenRouterData<&PaymentsAuthorizeRouterData>) -> Result<Self, Self::Error> {
        match item
            .router_data
            .request
            .mandate_id
            .to_owned()
            .and_then(|mandate_ids| mandate_ids.mandate_reference_id)
        {
            Some(mandate_ref) => AdyenPaymentRequest::try_from((item, mandate_ref)),
            None => match item.router_data.request.payment_method_data {
                PaymentMethodData::Card(ref card) => AdyenPaymentRequest::try_from((item, card)),
                PaymentMethodData::Wallet(ref wallet) => {
                    AdyenPaymentRequest::try_from((item, wallet))
                }
                PaymentMethodData::PayLater(ref pay_later) => {
                    AdyenPaymentRequest::try_from((item, pay_later))
                }
                PaymentMethodData::BankRedirect(ref bank_redirect) => {
                    AdyenPaymentRequest::try_from((item, bank_redirect))
                }
                PaymentMethodData::BankDebit(ref bank_debit) => {
                    AdyenPaymentRequest::try_from((item, bank_debit))
                }
                PaymentMethodData::BankTransfer(ref bank_transfer) => {
                    AdyenPaymentRequest::try_from((item, bank_transfer.as_ref()))
                }
                PaymentMethodData::CardRedirect(ref card_redirect_data) => {
                    AdyenPaymentRequest::try_from((item, card_redirect_data))
                }
                PaymentMethodData::Voucher(ref voucher_data) => {
                    AdyenPaymentRequest::try_from((item, voucher_data))
                }
                PaymentMethodData::GiftCard(ref gift_card_data) => {
                    AdyenPaymentRequest::try_from((item, gift_card_data.as_ref()))
                }
                PaymentMethodData::NetworkToken(ref token_data) => {
                    AdyenPaymentRequest::try_from((item, token_data))
                }
                PaymentMethodData::Crypto(_)
                | PaymentMethodData::MandatePayment
                | PaymentMethodData::Reward
                | PaymentMethodData::RealTimePayment(_)
                | PaymentMethodData::MobilePayment(_)
                | PaymentMethodData::Upi(_)
                | PaymentMethodData::OpenBanking(_)
                | PaymentMethodData::CardToken(_)
                | PaymentMethodData::CardDetailsForNetworkTransactionId(_) => {
                    Err(errors::ConnectorError::NotImplemented(
                        utils::get_unimplemented_payment_method_error_message("Adyen"),
                    ))?
                }
            },
        }
    }
}

impl TryFrom<&PaymentsPreProcessingRouterData> for AdyenBalanceRequest<'_> {
    type Error = Error;
    fn try_from(item: &PaymentsPreProcessingRouterData) -> Result<Self, Self::Error> {
        let payment_method = match &item.request.payment_method_data {
            Some(PaymentMethodData::GiftCard(gift_card_data)) => match gift_card_data.as_ref() {
                GiftCardData::Givex(gift_card_data) => {
                    let balance_pm = BalancePmData {
                        number: gift_card_data.number.clone(),
                        cvc: gift_card_data.cvc.clone(),
                    };
                    Ok(AdyenPaymentMethod::PaymentMethodBalance(Box::new(
                        balance_pm,
                    )))
                }
                GiftCardData::PaySafeCard {} | GiftCardData::BhnCardNetwork(_) => {
                    Err(errors::ConnectorError::FlowNotSupported {
                        flow: "Balance".to_string(),
                        connector: "adyen".to_string(),
                    })
                }
            },
            _ => Err(errors::ConnectorError::FlowNotSupported {
                flow: "Balance".to_string(),
                connector: "adyen".to_string(),
            }),
        }?;
        let auth_type = AdyenAuthType::try_from(&item.connector_auth_type)?;
        Ok(Self {
            payment_method,
            merchant_account: auth_type.merchant_account,
        })
    }
}

impl TryFrom<&PaymentsGiftCardBalanceCheckRouterData> for AdyenBalanceRequest<'_> {
    type Error = Error;
    fn try_from(item: &PaymentsGiftCardBalanceCheckRouterData) -> Result<Self, Self::Error> {
        let payment_method = match &item.request.payment_method_data {
            PaymentMethodData::GiftCard(gift_card_data) => match gift_card_data.as_ref() {
                GiftCardData::Givex(gift_card_data) => {
                    let balance_pm = BalancePmData {
                        number: gift_card_data.number.clone(),
                        cvc: gift_card_data.cvc.clone(),
                    };
                    Ok(AdyenPaymentMethod::PaymentMethodBalance(Box::new(
                        balance_pm,
                    )))
                }
                GiftCardData::PaySafeCard {} | GiftCardData::BhnCardNetwork(_) => {
                    Err(errors::ConnectorError::FlowNotSupported {
                        flow: "Balance".to_string(),
                        connector: "adyen".to_string(),
                    })
                }
            },
            _ => Err(errors::ConnectorError::FlowNotSupported {
                flow: "Balance".to_string(),
                connector: "adyen".to_string(),
            }),
        }?;

        let auth_type = AdyenAuthType::try_from(&item.connector_auth_type)?;
        Ok(Self {
            payment_method,
            merchant_account: auth_type.merchant_account,
        })
    }
}

impl From<&PaymentsAuthorizeRouterData> for AdyenShopperInteraction {
    fn from(item: &PaymentsAuthorizeRouterData) -> Self {
        match item.request.off_session {
            Some(true) => Self::ContinuedAuthentication,
            _ => match item.request.payment_channel {
                Some(common_enums::PaymentChannel::Ecommerce)
                | None
                | Some(common_enums::PaymentChannel::Other(_)) => Self::Ecommerce,
                Some(common_enums::PaymentChannel::MailOrder)
                | Some(common_enums::PaymentChannel::TelephoneOrder) => Self::Moto,
            },
        }
    }
}
type RecurringDetails = (Option<AdyenRecurringModel>, Option<bool>, Option<String>);

fn get_shopper_statement(item: &PaymentsAuthorizeRouterData) -> Option<String> {
    item.request
        .billing_descriptor
        .clone()
        .and_then(|descriptor| descriptor.statement_descriptor)
}

fn get_recurring_processing_model(
    item: &PaymentsAuthorizeRouterData,
) -> Result<RecurringDetails, Error> {
    let shopper_reference = item.get_connector_customer_id()?;

    match (item.request.setup_future_usage, item.request.off_session) {
        (Some(storage_enums::FutureUsage::OffSession), _) => {
            let store_payment_method = item.request.is_mandate_payment();
            Ok((
                Some(AdyenRecurringModel::UnscheduledCardOnFile),
                Some(store_payment_method),
                Some(shopper_reference),
            ))
        }
        (_, Some(true)) => Ok((
            Some(AdyenRecurringModel::UnscheduledCardOnFile),
            None,
            Some(shopper_reference),
        )),
        _ => Ok((None, None, None)),
    }
}

fn get_browser_info(item: &PaymentsAuthorizeRouterData) -> Result<Option<AdyenBrowserInfo>, Error> {
    if item.auth_type == storage_enums::AuthenticationType::ThreeDs
        || item.payment_method == storage_enums::PaymentMethod::Card
        || item.payment_method == storage_enums::PaymentMethod::BankRedirect
        || item.request.payment_method_type == Some(storage_enums::PaymentMethodType::GoPay)
        || item.request.payment_method_type == Some(storage_enums::PaymentMethodType::GooglePay)
    {
        let info = item.request.get_browser_info()?;
        Ok(Some(AdyenBrowserInfo {
            accept_header: info.get_accept_header()?,
            language: info.get_language()?,
            screen_height: info.get_screen_height()?,
            screen_width: info.get_screen_width()?,
            color_depth: info.get_color_depth()?,
            user_agent: info.get_user_agent()?,
            time_zone_offset: info.get_time_zone()?,
            java_enabled: info.get_java_enabled()?,
        }))
    } else {
        Ok(None)
    }
}

fn get_additional_data(item: &PaymentsAuthorizeRouterData) -> Option<AdditionalData> {
    let (authorisation_type, manual_capture) = match item.request.capture_method {
        Some(storage_enums::CaptureMethod::Manual) | Some(enums::CaptureMethod::ManualMultiple) => {
            (Some(AuthType::PreAuth), Some("true".to_string()))
        }
        _ => (None, None),
    };
    let riskdata = item.request.metadata.clone().and_then(get_risk_data);

    let execute_three_d = if matches!(item.auth_type, storage_enums::AuthenticationType::ThreeDs) {
        Some("true".to_string())
    } else {
        Some("false".to_string())
    };
    Some(AdditionalData {
        authorisation_type,
        manual_capture,
        execute_three_d,
        network_tx_reference: None,
        recurring_detail_reference: None,
        recurring_shopper_reference: None,
        recurring_processing_model: None,
        riskdata,
        sca_exemption: item.request.authentication_data.as_ref().and_then(|data| {
            data.exemption_indicator
                .as_ref()
                .and_then(to_adyen_exemption)
        }),
        ..AdditionalData::default()
    })
}

fn get_channel_type(pm_type: Option<storage_enums::PaymentMethodType>) -> Option<Channel> {
    pm_type.as_ref().and_then(|pmt| match pmt {
        storage_enums::PaymentMethodType::GoPay | storage_enums::PaymentMethodType::Vipps => {
            Some(Channel::Web)
        }
        _ => None,
    })
}

fn get_amount_data(item: &AdyenRouterData<&PaymentsAuthorizeRouterData>) -> Amount {
    Amount {
        currency: item.router_data.request.currency,
        value: item.amount.to_owned(),
    }
}

pub fn get_address_info(
    address: Option<&hyperswitch_domain_models::address::Address>,
) -> Option<Result<Address, error_stack::Report<errors::ConnectorError>>> {
    address.and_then(|add| {
        add.address.as_ref().map(
            |a| -> Result<Address, error_stack::Report<errors::ConnectorError>> {
                Ok(Address {
                    city: a.get_city()?.to_owned(),
                    country: a.get_country()?.to_owned(),
                    house_number_or_name: a.get_line1()?.to_owned(),
                    postal_code: a.get_zip()?.to_owned(),
                    state_or_province: a.state.clone(),
                    street: a.get_optional_line2().to_owned(),
                })
            },
        )
    })
}

fn get_line_items(item: &AdyenRouterData<&PaymentsAuthorizeRouterData>) -> Vec<LineItem> {
    let order_details = item.router_data.request.order_details.clone();
    match order_details {
        Some(od) => od
            .iter()
            .enumerate()
            .map(|(i, data)| LineItem {
                amount_including_tax: Some(data.amount),
                amount_excluding_tax: Some(data.amount),
                description: Some(data.product_name.clone()),
                id: Some(format!("Items #{i}")),
                tax_amount: None,
                quantity: Some(data.quantity),
            })
            .collect(),
        None => {
            let line_item = LineItem {
                amount_including_tax: Some(item.amount.to_owned()),
                amount_excluding_tax: Some(item.amount.to_owned()),
                description: item.router_data.description.clone(),
                id: Some(String::from("Items #1")),
                tax_amount: None,
                quantity: Some(1),
            };
            vec![line_item]
        }
    }
}

fn get_telephone_number(item: &PaymentsAuthorizeRouterData) -> Option<Secret<String>> {
    let phone = item
        .get_optional_billing()
        .and_then(|billing| billing.phone.as_ref());

    phone.as_ref().and_then(|phone| {
        phone.number.as_ref().and_then(|number| {
            phone
                .country_code
                .as_ref()
                .map(|cc| Secret::new(format!("{}{}", cc, number.peek())))
        })
    })
}

fn get_shopper_name(
    address: Option<&hyperswitch_domain_models::address::Address>,
) -> Option<ShopperName> {
    let billing = address.and_then(|billing| billing.address.as_ref());
    Some(ShopperName {
        first_name: billing.and_then(|a| a.first_name.clone()),
        last_name: billing.and_then(|a| a.last_name.clone()),
    })
}

fn get_country_code(
    address: Option<&hyperswitch_domain_models::address::Address>,
) -> Option<storage_enums::CountryAlpha2> {
    address.and_then(|billing| billing.address.as_ref().and_then(|address| address.country))
}

fn get_social_security_number(voucher_data: &VoucherData) -> Option<Secret<String>> {
    match voucher_data {
        VoucherData::Boleto(boleto_data) => boleto_data.social_security_number.clone(),
        VoucherData::Alfamart { .. }
        | VoucherData::Indomaret { .. }
        | VoucherData::Efecty
        | VoucherData::PagoEfectivo
        | VoucherData::RedCompra
        | VoucherData::Oxxo
        | VoucherData::RedPagos
        | VoucherData::SevenEleven { .. }
        | VoucherData::Lawson { .. }
        | VoucherData::MiniStop { .. }
        | VoucherData::FamilyMart { .. }
        | VoucherData::Seicomart { .. }
        | VoucherData::PayEasy { .. } => None,
    }
}

impl TryFrom<(&BankDebitData, &PaymentsAuthorizeRouterData)> for AdyenPaymentMethod<'_> {
    type Error = Error;
    fn try_from(
        (bank_debit_data, item): (&BankDebitData, &PaymentsAuthorizeRouterData),
    ) -> Result<Self, Self::Error> {
        match bank_debit_data {
            BankDebitData::AchBankDebit {
                account_number,
                routing_number,
                ..
            } => Ok(AdyenPaymentMethod::AchDirectDebit(Box::new(
                AchDirectDebitData {
                    bank_account_number: account_number.clone(),
                    bank_location_id: routing_number.clone(),
                    owner_name: item.get_billing_full_name()?,
                },
            ))),
            BankDebitData::SepaBankDebit { iban, .. } => Ok(AdyenPaymentMethod::SepaDirectDebit(
                Box::new(SepaDirectDebitData {
                    owner_name: item.get_billing_full_name()?,
                    iban_number: iban.clone(),
                }),
            )),
            BankDebitData::BacsBankDebit {
                account_number,
                sort_code,
                ..
            } => {
                let testing_data = item
                    .request
                    .get_connector_testing_data()
                    .map(AdyenTestingData::try_from)
                    .transpose()?;
                let test_holder_name = testing_data.and_then(|test_data| test_data.holder_name);
                Ok(AdyenPaymentMethod::BacsDirectDebit(Box::new(
                    BacsDirectDebitData {
                        bank_account_number: account_number.clone(),
                        bank_location_id: sort_code.clone(),
                        holder_name: test_holder_name.unwrap_or(item.get_billing_full_name()?),
                    },
                )))
            }

            BankDebitData::BecsBankDebit { .. } | BankDebitData::SepaGuarenteedBankDebit { .. } => {
                Err(errors::ConnectorError::NotImplemented(
                    utils::get_unimplemented_payment_method_error_message("Adyen"),
                )
                .into())
            }
        }
    }
}

impl TryFrom<(&VoucherData, &PaymentsAuthorizeRouterData)> for AdyenPaymentMethod<'_> {
    type Error = Error;
    fn try_from(
        (voucher_data, item): (&VoucherData, &PaymentsAuthorizeRouterData),
    ) -> Result<Self, Self::Error> {
        match voucher_data {
            VoucherData::Boleto { .. } => Ok(AdyenPaymentMethod::BoletoBancario),
            VoucherData::Alfamart(_) => Ok(AdyenPaymentMethod::Alfamart(Box::new(
                DokuBankData::try_from(item)?,
            ))),

            VoucherData::Indomaret(_) => Ok(AdyenPaymentMethod::Indomaret(Box::new(
                DokuBankData::try_from(item)?,
            ))),

            VoucherData::Oxxo => Ok(AdyenPaymentMethod::Oxxo),
            VoucherData::SevenEleven(_) => Ok(AdyenPaymentMethod::SevenEleven(Box::new(
                JCSVoucherData::try_from(item)?,
            ))),
            VoucherData::Lawson(_)
            | VoucherData::MiniStop(_)
            | VoucherData::FamilyMart(_)
            | VoucherData::Seicomart(_)
            | VoucherData::PayEasy(_) => Ok(AdyenPaymentMethod::JapaneseConvenienceStores(
                Box::new(JCSVoucherData::try_from(item)?),
            )),
            VoucherData::Efecty
            | VoucherData::PagoEfectivo
            | VoucherData::RedCompra
            | VoucherData::RedPagos => Err(errors::ConnectorError::NotImplemented(
                utils::get_unimplemented_payment_method_error_message("Adyen"),
            )
            .into()),
        }
    }
}

impl TryFrom<&GiftCardData> for AdyenPaymentMethod<'_> {
    type Error = Error;
    fn try_from(gift_card_data: &GiftCardData) -> Result<Self, Self::Error> {
        match gift_card_data {
            GiftCardData::PaySafeCard {} => Ok(AdyenPaymentMethod::PaySafeCard),
            GiftCardData::Givex(givex_data) => {
                let gift_card_pm = AdyenGiftCardData {
                    brand: GiftCardBrand::Givex,
                    number: givex_data.number.clone(),
                    cvc: givex_data.cvc.clone(),
                };
                Ok(AdyenPaymentMethod::AdyenGiftCard(Box::new(gift_card_pm)))
            }
            GiftCardData::BhnCardNetwork(_) => Err(errors::ConnectorError::NotImplemented(
                utils::get_unimplemented_payment_method_error_message("Adyen"),
            )
            .into()),
        }
    }
}

fn get_adyen_card_network(card_network: common_enums::CardNetwork) -> Option<CardBrand> {
    match card_network {
        common_enums::CardNetwork::Visa => Some(CardBrand::Visa),
        common_enums::CardNetwork::Mastercard => Some(CardBrand::MC),
        common_enums::CardNetwork::CartesBancaires => Some(CardBrand::Cartebancaire),
        common_enums::CardNetwork::AmericanExpress => Some(CardBrand::Amex),
        common_enums::CardNetwork::JCB => Some(CardBrand::Jcb),
        common_enums::CardNetwork::DinersClub => Some(CardBrand::Diners),
        common_enums::CardNetwork::Discover => Some(CardBrand::Discover),
        common_enums::CardNetwork::UnionPay => Some(CardBrand::Cup),
        common_enums::CardNetwork::RuPay => Some(CardBrand::Rupay),
        common_enums::CardNetwork::Maestro => Some(CardBrand::Maestro),
        common_enums::CardNetwork::Star => Some(CardBrand::Star),
        common_enums::CardNetwork::Accel => Some(CardBrand::Accel),
        common_enums::CardNetwork::Pulse => Some(CardBrand::Pulse),
        common_enums::CardNetwork::Nyce => Some(CardBrand::Nyce),
        common_enums::CardNetwork::Interac => None,
    }
}

impl TryFrom<(&Card, Option<Secret<String>>)> for AdyenPaymentMethod<'_> {
    type Error = Error;
    fn try_from(
        (card, card_holder_name): (&Card, Option<Secret<String>>),
    ) -> Result<Self, Self::Error> {
        let adyen_card = AdyenCard {
            number: card.card_number.clone(),
            expiry_month: card.card_exp_month.clone(),
            expiry_year: card.get_expiry_year_4_digit(),
            cvc: Some(card.card_cvc.clone()),
            holder_name: card_holder_name,
            brand: if card
                .card_number
                .is_cobadged_card()
                .change_context(errors::ConnectorError::RequestEncodingFailed)?
            {
                card.card_network.clone().and_then(get_adyen_card_network)
            } else {
                None
            },
            network_payment_reference: None,
        };
        Ok(AdyenPaymentMethod::AdyenCard(Box::new(adyen_card)))
    }
}

impl TryFrom<&storage_enums::PaymentMethodType> for PaymentType {
    type Error = Error;
    fn try_from(item: &storage_enums::PaymentMethodType) -> Result<Self, Self::Error> {
        match item {
            storage_enums::PaymentMethodType::Credit
            | storage_enums::PaymentMethodType::Debit
            | storage_enums::PaymentMethodType::Klarna
            | storage_enums::PaymentMethodType::BancontactCard
            | storage_enums::PaymentMethodType::Blik
            | storage_enums::PaymentMethodType::Eps
            | storage_enums::PaymentMethodType::Ideal
            | storage_enums::PaymentMethodType::OnlineBankingCzechRepublic
            | storage_enums::PaymentMethodType::OnlineBankingFinland
            | storage_enums::PaymentMethodType::OnlineBankingPoland
            | storage_enums::PaymentMethodType::OnlineBankingSlovakia
            | storage_enums::PaymentMethodType::Trustly
            | storage_enums::PaymentMethodType::GooglePay
            | storage_enums::PaymentMethodType::AliPay
            | storage_enums::PaymentMethodType::ApplePay
            | storage_enums::PaymentMethodType::AliPayHk
            | storage_enums::PaymentMethodType::MbWay
            | storage_enums::PaymentMethodType::MobilePay
            | storage_enums::PaymentMethodType::WeChatPay
            | storage_enums::PaymentMethodType::SamsungPay
            | storage_enums::PaymentMethodType::Affirm
            | storage_enums::PaymentMethodType::AfterpayClearpay
            | storage_enums::PaymentMethodType::PayBright
            | storage_enums::PaymentMethodType::Walley => Ok(Self::Scheme),
            storage_enums::PaymentMethodType::Sepa => Ok(Self::SepaDirectDebit),
            storage_enums::PaymentMethodType::Bacs => Ok(Self::BacsDirectDebit),
            storage_enums::PaymentMethodType::Ach => Ok(Self::AchDirectDebit),
            storage_enums::PaymentMethodType::Paypal => Ok(Self::Paypal),
            _ => Err(errors::ConnectorError::NotImplemented(
                utils::get_unimplemented_payment_method_error_message("Adyen"),
            ))?,
        }
    }
}

impl TryFrom<&utils::CardIssuer> for CardBrand {
    type Error = Error;
    fn try_from(card_issuer: &utils::CardIssuer) -> Result<Self, Self::Error> {
        match card_issuer {
            utils::CardIssuer::AmericanExpress => Ok(Self::Amex),
            utils::CardIssuer::Master => Ok(Self::MC),
            utils::CardIssuer::Visa => Ok(Self::Visa),
            utils::CardIssuer::Maestro => Ok(Self::Maestro),
            utils::CardIssuer::Discover => Ok(Self::Discover),
            utils::CardIssuer::DinersClub => Ok(Self::Diners),
            utils::CardIssuer::JCB => Ok(Self::Jcb),
            utils::CardIssuer::CarteBlanche => Ok(Self::Cartebancaire),
            utils::CardIssuer::CartesBancaires => Ok(Self::Cartebancaire),
            utils::CardIssuer::UnionPay => Ok(Self::Cup),
        }
    }
}

impl TryFrom<(&WalletData, &PaymentsAuthorizeRouterData)> for AdyenPaymentMethod<'_> {
    type Error = Error;
    fn try_from(value: (&WalletData, &PaymentsAuthorizeRouterData)) -> Result<Self, Self::Error> {
        let (wallet_data, item) = value;
        match wallet_data {
            WalletData::GooglePay(data) => {
                let gpay_data = AdyenGPay {
                    google_pay_token: Secret::new(
                        data.tokenization_data
                            .get_encrypted_google_pay_token()
                            .change_context(errors::ConnectorError::MissingRequiredField {
                                field_name: "gpay wallet_token",
                            })?
                            .to_owned(),
                    ),
                };
                Ok(AdyenPaymentMethod::Gpay(Box::new(gpay_data)))
            }
            WalletData::ApplePay(data) => {
                if let Some(PaymentMethodToken::ApplePayDecrypt(apple_pay_decrypte)) =
                    item.payment_method_token.clone()
                {
                    let expiry_year_4_digit = apple_pay_decrypte.get_four_digit_expiry_year();
                    let exp_month = apple_pay_decrypte.get_expiry_month().change_context(
                        errors::ConnectorError::InvalidDataFormat {
                            field_name: "expiration_month",
                        },
                    )?;
                    let apple_pay_decrypted_data = AdyenApplePayDecryptData {
                        number: apple_pay_decrypte.application_primary_account_number,
                        expiry_month: exp_month,
                        expiry_year: expiry_year_4_digit,
                        brand: data.payment_method.network.clone(),
                        payment_type: PaymentType::Scheme,
                    };
                    Ok(AdyenPaymentMethod::ApplePayDecrypt(Box::new(
                        apple_pay_decrypted_data,
                    )))
                } else {
                    let apple_pay_encrypted_data = data
                        .payment_data
                        .get_encrypted_apple_pay_payment_data_mandatory()
                        .change_context(errors::ConnectorError::MissingRequiredField {
                            field_name: "Apple pay encrypted data",
                        })?;
                    let apple_pay_data = AdyenApplePay {
                        apple_pay_token: Secret::new(apple_pay_encrypted_data.to_string()),
                    };
                    Ok(AdyenPaymentMethod::ApplePay(Box::new(apple_pay_data)))
                }
            }
            WalletData::PaypalRedirect(_) => Ok(AdyenPaymentMethod::AdyenPaypal),
            WalletData::AliPayRedirect(_) => Ok(AdyenPaymentMethod::AliPay),
            WalletData::AliPayHkRedirect(_) => Ok(AdyenPaymentMethod::AliPayHk),
            WalletData::GoPayRedirect(_) => {
                let go_pay_data = GoPayData {};
                Ok(AdyenPaymentMethod::GoPay(Box::new(go_pay_data)))
            }
            WalletData::KakaoPayRedirect(_) => {
                let kakao_pay_data = KakaoPayData {};
                Ok(AdyenPaymentMethod::Kakaopay(Box::new(kakao_pay_data)))
            }
            WalletData::GcashRedirect(_) => {
                let gcash_data = GcashData {};
                Ok(AdyenPaymentMethod::Gcash(Box::new(gcash_data)))
            }
            WalletData::MomoRedirect(_) => {
                let momo_data = MomoData {};
                Ok(AdyenPaymentMethod::Momo(Box::new(momo_data)))
            }
            WalletData::TouchNGoRedirect(_) => {
                let touch_n_go_data = TouchNGoData {};
                Ok(AdyenPaymentMethod::TouchNGo(Box::new(touch_n_go_data)))
            }
            WalletData::MbWayRedirect(_) => {
                let phone_details = item.get_billing_phone()?;
                let mbway_data = MbwayData {
                    telephone_number: phone_details.get_number_with_country_code()?,
                };
                Ok(AdyenPaymentMethod::Mbway(Box::new(mbway_data)))
            }
            WalletData::MobilePayRedirect(_) => Ok(AdyenPaymentMethod::MobilePay),
            WalletData::WeChatPayRedirect(_) => Ok(AdyenPaymentMethod::WeChatPayWeb),
            WalletData::SamsungPay(samsung_data) => {
                let data = SamsungPayPmData {
                    samsung_pay_token: samsung_data.payment_credential.token_data.data.to_owned(),
                };
                Ok(AdyenPaymentMethod::SamsungPay(Box::new(data)))
            }
            WalletData::Paze(_) => match item.payment_method_token.clone() {
                Some(PaymentMethodToken::PazeDecrypt(paze_decrypted_data)) => {
                    let data = AdyenPazeData {
                        number: paze_decrypted_data.token.payment_token,
                        expiry_month: paze_decrypted_data.token.token_expiration_month,
                        expiry_year: paze_decrypted_data.token.token_expiration_year,
                        cvc: None,
                        holder_name: paze_decrypted_data
                            .billing_address
                            .name
                            .or(item.get_optional_billing_full_name()),
                        brand: Some(paze_decrypted_data.payment_card_network.clone())
                            .and_then(get_adyen_card_network),
                        network_payment_reference: None,
                    };
                    Ok(AdyenPaymentMethod::AdyenPaze(Box::new(data)))
                }
                _ => Err(errors::ConnectorError::NotImplemented(
                    utils::get_unimplemented_payment_method_error_message("Cybersource"),
                )
                .into()),
            },
            WalletData::TwintRedirect { .. } => Ok(AdyenPaymentMethod::Twint),
            WalletData::VippsRedirect { .. } => Ok(AdyenPaymentMethod::Vipps),
            WalletData::DanaRedirect { .. } => Ok(AdyenPaymentMethod::Dana),
            WalletData::SwishQr(_) => Ok(AdyenPaymentMethod::Swish),
            WalletData::AliPayQr(_)
            | WalletData::AmazonPayRedirect(_)
            | WalletData::Paysera(_)
            | WalletData::Skrill(_)
            | WalletData::ApplePayRedirect(_)
            | WalletData::ApplePayThirdPartySdk(_)
            | WalletData::GooglePayRedirect(_)
            | WalletData::GooglePayThirdPartySdk(_)
            | WalletData::BluecodeRedirect {}
            | WalletData::AmazonPay(_)
            | WalletData::PaypalSdk(_)
            | WalletData::WeChatPayQr(_)
            | WalletData::CashappQr(_)
            | WalletData::Mifinity(_)
            | WalletData::RevolutPay(_) => Err(errors::ConnectorError::NotImplemented(
                utils::get_unimplemented_payment_method_error_message("Adyen"),
            )
            .into()),
        }
    }
}

pub fn check_required_field<'a, T>(
    field: &'a Option<T>,
    message: &'static str,
) -> Result<&'a T, errors::ConnectorError> {
    field
        .as_ref()
        .ok_or(errors::ConnectorError::MissingRequiredField {
            field_name: message,
        })
}

impl
    TryFrom<(
        &PayLaterData,
        &Option<storage_enums::CountryAlpha2>,
        &Option<Email>,
        &Option<String>,
        &Option<ShopperName>,
        &Option<Secret<String>>,
        &Option<Address>,
        &Option<Address>,
    )> for AdyenPaymentMethod<'_>
{
    type Error = Error;
    fn try_from(
        value: (
            &PayLaterData,
            &Option<storage_enums::CountryAlpha2>,
            &Option<Email>,
            &Option<String>,
            &Option<ShopperName>,
            &Option<Secret<String>>,
            &Option<Address>,
            &Option<Address>,
        ),
    ) -> Result<Self, Self::Error> {
        let (
            pay_later_data,
            country_code,
            shopper_email,
            shopper_reference,
            shopper_name,
            telephone_number,
            billing_address,
            delivery_address,
        ) = value;
        match pay_later_data {
            PayLaterData::KlarnaRedirect { .. } => {
                check_required_field(shopper_email, "email")?;
                check_required_field(shopper_reference, "customer_id")?;
                check_required_field(country_code, "billing.country")?;

                Ok(AdyenPaymentMethod::AdyenKlarna)
            }
            PayLaterData::AffirmRedirect { .. } => {
                check_required_field(shopper_email, "email")?;
                check_required_field(shopper_name, "billing.first_name, billing.last_name")?;
                check_required_field(telephone_number, "billing.phone")?;
                check_required_field(billing_address, "billing")?;

                Ok(AdyenPaymentMethod::AdyenAffirm)
            }
            PayLaterData::AfterpayClearpayRedirect { .. } => {
                check_required_field(shopper_email, "email")?;
                check_required_field(shopper_name, "billing.first_name, billing.last_name")?;
                check_required_field(delivery_address, "shipping")?;
                check_required_field(billing_address, "billing")?;

                if let Some(country) = country_code {
                    match country {
                        storage_enums::CountryAlpha2::IT
                        | storage_enums::CountryAlpha2::FR
                        | storage_enums::CountryAlpha2::ES
                        | storage_enums::CountryAlpha2::GB => Ok(AdyenPaymentMethod::ClearPay),
                        _ => Ok(AdyenPaymentMethod::AfterPay),
                    }
                } else {
                    Err(errors::ConnectorError::MissingRequiredField {
                        field_name: "country",
                    })?
                }
            }
            PayLaterData::PayBrightRedirect { .. } => {
                check_required_field(shopper_name, "billing.first_name, billing.last_name")?;
                check_required_field(telephone_number, "billing.phone")?;
                check_required_field(shopper_email, "email")?;
                check_required_field(billing_address, "billing")?;
                check_required_field(delivery_address, "shipping")?;
                check_required_field(country_code, "billing.country")?;
                Ok(AdyenPaymentMethod::PayBright)
            }
            PayLaterData::WalleyRedirect { .. } => {
                //[TODO: Line items specific sub-fields are mandatory]
                check_required_field(telephone_number, "billing.phone")?;
                check_required_field(shopper_email, "email")?;
                Ok(AdyenPaymentMethod::Walley)
            }
            PayLaterData::AlmaRedirect { .. } => {
                check_required_field(telephone_number, "billing.phone")?;
                check_required_field(shopper_email, "email")?;
                check_required_field(billing_address, "billing")?;
                check_required_field(delivery_address, "shipping")?;
                Ok(AdyenPaymentMethod::AlmaPayLater)
            }
            PayLaterData::AtomeRedirect { .. } => {
                check_required_field(shopper_email, "email")?;
                check_required_field(shopper_name, "billing.first_name, billing.last_name")?;
                check_required_field(telephone_number, "billing.phone")?;
                check_required_field(billing_address, "billing")?;
                Ok(AdyenPaymentMethod::Atome)
            }
            PayLaterData::KlarnaSdk { .. }
            | PayLaterData::BreadpayRedirect {}
            | PayLaterData::FlexitiRedirect {}
            | PayLaterData::PayjustnowRedirect {} => Err(errors::ConnectorError::NotImplemented(
                utils::get_unimplemented_payment_method_error_message("Adyen"),
            )
            .into()),
        }
    }
}

impl TryFrom<(&BankRedirectData, &PaymentsAuthorizeRouterData)> for AdyenPaymentMethod<'_> {
    type Error = Error;
    fn try_from(
        (bank_redirect_data, item): (&BankRedirectData, &PaymentsAuthorizeRouterData),
    ) -> Result<Self, Self::Error> {
        match bank_redirect_data {
            BankRedirectData::BancontactCard {
                card_number,
                card_exp_month,
                card_exp_year,
                ..
            } => {
                let testing_data = item
                    .request
                    .get_connector_testing_data()
                    .map(AdyenTestingData::try_from)
                    .transpose()?;
                let test_holder_name = testing_data.and_then(|test_data| test_data.holder_name);
                Ok(AdyenPaymentMethod::BancontactCard(Box::new(AdyenCard {
                    brand: Some(CardBrand::Bcmc),
                    number: card_number
                        .as_ref()
                        .ok_or(errors::ConnectorError::MissingRequiredField {
                            field_name: "bancontact_card.card_number",
                        })?
                        .clone(),
                    expiry_month: card_exp_month
                        .as_ref()
                        .ok_or(errors::ConnectorError::MissingRequiredField {
                            field_name: "bancontact_card.card_exp_month",
                        })?
                        .clone(),
                    expiry_year: card_exp_year
                        .as_ref()
                        .ok_or(errors::ConnectorError::MissingRequiredField {
                            field_name: "bancontact_card.card_exp_year",
                        })?
                        .clone(),
                    holder_name: test_holder_name.or(Some(item.get_billing_full_name()?)),
                    cvc: None,
                    network_payment_reference: None,
                })))
            }
            BankRedirectData::Bizum { .. } => Ok(AdyenPaymentMethod::Bizum),
            BankRedirectData::Blik { blik_code } => {
                Ok(AdyenPaymentMethod::Blik(Box::new(BlikRedirectionData {
                    blik_code: Secret::new(blik_code.clone().ok_or(
                        errors::ConnectorError::MissingRequiredField {
                            field_name: "blik_code",
                        },
                    )?),
                })))
            }
            BankRedirectData::Eps { bank_name, .. } => Ok(AdyenPaymentMethod::Eps(Box::new(
                BankRedirectionWithIssuer {
                    issuer: Some(
                        AdyenTestBankNames::try_from(&bank_name.ok_or(
                            errors::ConnectorError::MissingRequiredField {
                                field_name: "eps.bank_name",
                            },
                        )?)?
                        .0,
                    ),
                },
            ))),
            BankRedirectData::Ideal { .. } => Ok(AdyenPaymentMethod::Ideal),
            BankRedirectData::OnlineBankingCzechRepublic { issuer } => {
                Ok(AdyenPaymentMethod::OnlineBankingCzechRepublic(Box::new(
                    OnlineBankingCzechRepublicData {
                        issuer: OnlineBankingCzechRepublicBanks::try_from(issuer)?,
                    },
                )))
            }
            BankRedirectData::OnlineBankingFinland { .. } => {
                Ok(AdyenPaymentMethod::OnlineBankingFinland)
            }
            BankRedirectData::OnlineBankingPoland { issuer } => Ok(
                AdyenPaymentMethod::OnlineBankingPoland(Box::new(OnlineBankingPolandData {
                    issuer: OnlineBankingPolandBanks::try_from(issuer)?,
                })),
            ),
            BankRedirectData::OnlineBankingSlovakia { issuer } => Ok(
                AdyenPaymentMethod::OnlineBankingSlovakia(Box::new(OnlineBankingSlovakiaData {
                    issuer: OnlineBankingSlovakiaBanks::try_from(issuer)?,
                })),
            ),
            BankRedirectData::OnlineBankingFpx { issuer } => Ok(
                AdyenPaymentMethod::OnlineBankingFpx(Box::new(OnlineBankingFpxData {
                    issuer: OnlineBankingFpxIssuer::try_from(issuer)?,
                })),
            ),
            BankRedirectData::OnlineBankingThailand { issuer } => Ok(
                AdyenPaymentMethod::OnlineBankingThailand(Box::new(OnlineBankingThailandData {
                    issuer: OnlineBankingThailandIssuer::try_from(issuer)?,
                })),
            ),
            BankRedirectData::OpenBankingUk { issuer, .. } => Ok(
                AdyenPaymentMethod::OpenBankingUK(Box::new(OpenBankingUKData {
                    issuer: match issuer {
                        Some(bank_name) => Some(OpenBankingUKIssuer::try_from(bank_name)?),
                        None => None,
                    },
                })),
            ),
            BankRedirectData::Trustly { .. } => Ok(AdyenPaymentMethod::Trustly),
            BankRedirectData::Giropay { .. }
            | BankRedirectData::Eft { .. }
            | BankRedirectData::Interac { .. }
            | BankRedirectData::LocalBankRedirect {}
            | BankRedirectData::Przelewy24 { .. }
            | BankRedirectData::Sofort { .. } => Err(errors::ConnectorError::NotImplemented(
                utils::get_unimplemented_payment_method_error_message("Adyen"),
            )
            .into()),
        }
    }
}

impl TryFrom<(&BankTransferData, &PaymentsAuthorizeRouterData)> for AdyenPaymentMethod<'_> {
    type Error = Error;
    fn try_from(
        (bank_transfer_data, item): (&BankTransferData, &PaymentsAuthorizeRouterData),
    ) -> Result<Self, Self::Error> {
        match bank_transfer_data {
            BankTransferData::PermataBankTransfer {} => Ok(
                AdyenPaymentMethod::PermataBankTransfer(Box::new(DokuBankData::try_from(item)?)),
            ),

            BankTransferData::BcaBankTransfer {} => Ok(AdyenPaymentMethod::BcaBankTransfer(
                Box::new(DokuBankData::try_from(item)?),
            )),
            BankTransferData::BniVaBankTransfer {} => Ok(AdyenPaymentMethod::BniVa(Box::new(
                DokuBankData::try_from(item)?,
            ))),
            BankTransferData::BriVaBankTransfer {} => Ok(AdyenPaymentMethod::BriVa(Box::new(
                DokuBankData::try_from(item)?,
            ))),
            BankTransferData::CimbVaBankTransfer {} => Ok(AdyenPaymentMethod::CimbVa(Box::new(
                DokuBankData::try_from(item)?,
            ))),
            BankTransferData::DanamonVaBankTransfer {} => Ok(AdyenPaymentMethod::DanamonVa(
                Box::new(DokuBankData::try_from(item)?),
            )),
            BankTransferData::MandiriVaBankTransfer {} => Ok(AdyenPaymentMethod::MandiriVa(
                Box::new(DokuBankData::try_from(item)?),
            )),
            BankTransferData::Pix { .. } => Ok(AdyenPaymentMethod::Pix),
            BankTransferData::AchBankTransfer { .. }
            | BankTransferData::SepaBankTransfer { .. }
            | BankTransferData::BacsBankTransfer { .. }
            | BankTransferData::MultibancoBankTransfer { .. }
            | BankTransferData::LocalBankTransfer { .. }
            | BankTransferData::InstantBankTransfer {}
            | BankTransferData::InstantBankTransferFinland {}
            | BankTransferData::InstantBankTransferPoland {}
            | BankTransferData::IndonesianBankTransfer { .. }
            | BankTransferData::Pse {} => Err(errors::ConnectorError::NotImplemented(
                utils::get_unimplemented_payment_method_error_message("Adyen"),
            )
            .into()),
        }
    }
}

impl TryFrom<&PaymentsAuthorizeRouterData> for DokuBankData {
    type Error = Error;
    fn try_from(item: &PaymentsAuthorizeRouterData) -> Result<Self, Self::Error> {
        Ok(Self {
            first_name: item.get_billing_first_name()?,
            last_name: item.get_optional_billing_last_name(),
            shopper_email: item.get_billing_email()?,
        })
    }
}

impl TryFrom<&CardRedirectData> for AdyenPaymentMethod<'_> {
    type Error = Error;
    fn try_from(card_redirect_data: &CardRedirectData) -> Result<Self, Self::Error> {
        match card_redirect_data {
            CardRedirectData::Knet {} => Ok(AdyenPaymentMethod::Knet),
            CardRedirectData::Benefit {} => Ok(AdyenPaymentMethod::Benefit),
            CardRedirectData::MomoAtm {} => Ok(AdyenPaymentMethod::MomoAtm),
            CardRedirectData::CardRedirect {} => Err(errors::ConnectorError::NotImplemented(
                utils::get_unimplemented_payment_method_error_message("Adyen"),
            )
            .into()),
        }
    }
}

fn get_str(key: &str, riskdata: &serde_json::Value) -> Option<String> {
    riskdata
        .get(key)
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
}

fn get_bool(key: &str, riskdata: &serde_json::Value) -> Option<bool> {
    riskdata.get(key).and_then(|v| v.as_bool())
}

pub fn get_risk_data(metadata: serde_json::Value) -> Option<RiskData> {
    let item_i_d = get_str("riskdata.basket.item1.itemID", &metadata);
    let product_title = get_str("riskdata.basket.item1.productTitle", &metadata);
    let amount_per_item = get_str("riskdata.basket.item1.amountPerItem", &metadata);
    let currency = get_str("riskdata.basket.item1.currency", &metadata);
    let upc = get_str("riskdata.basket.item1.upc", &metadata);
    let brand = get_str("riskdata.basket.item1.brand", &metadata);
    let manufacturer = get_str("riskdata.basket.item1.manufacturer", &metadata);
    let category = get_str("riskdata.basket.item1.category", &metadata);
    let quantity = get_str("riskdata.basket.item1.quantity", &metadata);
    let color = get_str("riskdata.basket.item1.color", &metadata);
    let size = get_str("riskdata.basket.item1.size", &metadata);

    let device_country = get_str("riskdata.deviceCountry", &metadata);
    let house_numberor_name = get_str("riskdata.houseNumberorName", &metadata);
    let account_creation_date = get_str("riskdata.accountCreationDate", &metadata);
    let affiliate_channel = get_str("riskdata.affiliateChannel", &metadata);
    let avg_order_value = get_str("riskdata.avgOrderValue", &metadata);
    let delivery_method = get_str("riskdata.deliveryMethod", &metadata);
    let email_name = get_str("riskdata.emailName", &metadata);
    let email_domain = get_str("riskdata.emailDomain", &metadata);
    let last_order_date = get_str("riskdata.lastOrderDate", &metadata);
    let merchant_reference = get_str("riskdata.merchantReference", &metadata);
    let payment_method = get_str("riskdata.paymentMethod", &metadata);
    let promotion_name = get_str("riskdata.promotionName", &metadata);
    let secondary_phone_number = get_str("riskdata.secondaryPhoneNumber", &metadata);
    let timefrom_loginto_order = get_str("riskdata.timefromLogintoOrder", &metadata);
    let total_session_time = get_str("riskdata.totalSessionTime", &metadata);
    let total_authorized_amount_in_last30_days =
        get_str("riskdata.totalAuthorizedAmountInLast30Days", &metadata);
    let total_order_quantity = get_str("riskdata.totalOrderQuantity", &metadata);
    let total_lifetime_value = get_str("riskdata.totalLifetimeValue", &metadata);
    let visits_month = get_str("riskdata.visitsMonth", &metadata);
    let visits_week = get_str("riskdata.visitsWeek", &metadata);
    let visits_year = get_str("riskdata.visitsYear", &metadata);
    let ship_to_name = get_str("riskdata.shipToName", &metadata);
    let first8charactersof_address_line1_zip =
        get_str("riskdata.first8charactersofAddressLine1Zip", &metadata);
    let affiliate_order = get_bool("riskdata.affiliateOrder", &metadata);

    Some(RiskData {
        item_i_d,
        product_title,
        amount_per_item,
        currency,
        upc,
        brand,
        manufacturer,
        category,
        quantity,
        color,
        size,
        device_country,
        house_numberor_name,
        account_creation_date,
        affiliate_channel,
        avg_order_value,
        delivery_method,
        email_name,
        email_domain,
        last_order_date,
        merchant_reference,
        payment_method,
        promotion_name,
        secondary_phone_number,
        timefrom_loginto_order,
        total_session_time,
        total_authorized_amount_in_last30_days,
        total_order_quantity,
        total_lifetime_value,
        visits_month,
        visits_week,
        visits_year,
        ship_to_name,
        first8charactersof_address_line1_zip,
        affiliate_order,
    })
}

fn get_store_id(metadata: serde_json::Value) -> Option<String> {
    metadata
        .get("store")
        .and_then(|store| store.as_str())
        .map(|store| store.to_string())
}

fn get_adyen_metadata(metadata: Option<serde_json::Value>) -> AdyenMetadata {
    metadata
        .and_then(|value| serde_json::from_value(value).ok())
        .unwrap_or_default()
}
impl
    TryFrom<(
        &AdyenRouterData<&PaymentsAuthorizeRouterData>,
        payments::MandateReferenceId,
    )> for AdyenPaymentRequest<'_>
{
    type Error = Error;
    fn try_from(
        value: (
            &AdyenRouterData<&PaymentsAuthorizeRouterData>,
            payments::MandateReferenceId,
        ),
    ) -> Result<Self, Self::Error> {
        let (item, mandate_ref_id) = value;
        let amount = get_amount_data(item);
        let auth_type = AdyenAuthType::try_from(&item.router_data.connector_auth_type)?;
        let shopper_interaction = AdyenShopperInteraction::from(item.router_data);
        let (recurring_processing_model, store_payment_method, shopper_reference) =
            get_recurring_processing_model(item.router_data)?;
        let browser_info = None;
        let additional_data = get_additional_data(item.router_data);
        let return_url = item.router_data.request.get_router_return_url()?;
        let payment_method_type = item.router_data.request.payment_method_type;
        let testing_data = item
            .router_data
            .request
            .get_connector_testing_data()
            .map(AdyenTestingData::try_from)
            .transpose()?;
        let test_holder_name = testing_data.and_then(|test_data| test_data.holder_name);
        let payment_method = match mandate_ref_id {
            payments::MandateReferenceId::ConnectorMandateId(connector_mandate_ids) => {
                let adyen_mandate = AdyenMandate {
                    payment_type: match payment_method_type {
                        Some(pm_type) => PaymentType::try_from(&pm_type)?,
                        None => PaymentType::Scheme,
                    },
                    stored_payment_method_id: Secret::new(
                        connector_mandate_ids
                            .get_connector_mandate_id()
                            .ok_or_else(missing_field_err("mandate_id"))?,
                    ),
                    holder_name: test_holder_name,
                };
                Ok::<PaymentMethod<'_>, Self::Error>(PaymentMethod::AdyenMandatePaymentMethod(
                    Box::new(adyen_mandate),
                ))
            }
            payments::MandateReferenceId::NetworkMandateId(network_mandate_id) => {
                match item.router_data.request.payment_method_data {
                    PaymentMethodData::CardDetailsForNetworkTransactionId(
                        ref card_details_for_network_transaction_id,
                    ) => {
                        let brand = match card_details_for_network_transaction_id
                            .card_network
                            .clone()
                            .and_then(get_adyen_card_network)
                        {
                            Some(card_network) => card_network,
                            None => CardBrand::try_from(
                                &card_details_for_network_transaction_id.get_card_issuer()?,
                            )?,
                        };

                        let card_holder_name = item.router_data.get_optional_billing_full_name();
                        let adyen_card = AdyenCard {
                            number: card_details_for_network_transaction_id.card_number.clone(),
                            expiry_month: card_details_for_network_transaction_id
                                .card_exp_month
                                .clone(),
                            expiry_year: card_details_for_network_transaction_id
                                .get_expiry_year_4_digit()
                                .clone(),
                            cvc: None,
                            holder_name: test_holder_name.or(card_holder_name),
                            brand: Some(brand),
                            network_payment_reference: Some(Secret::new(network_mandate_id)),
                        };
                        Ok(PaymentMethod::AdyenPaymentMethod(Box::new(
                            AdyenPaymentMethod::AdyenCard(Box::new(adyen_card)),
                        )))
                    }
                    PaymentMethodData::CardRedirect(_)
                    | PaymentMethodData::Wallet(_)
                    | PaymentMethodData::PayLater(_)
                    | PaymentMethodData::BankRedirect(_)
                    | PaymentMethodData::BankDebit(_)
                    | PaymentMethodData::BankTransfer(_)
                    | PaymentMethodData::Crypto(_)
                    | PaymentMethodData::MandatePayment
                    | PaymentMethodData::Reward
                    | PaymentMethodData::RealTimePayment(_)
                    | PaymentMethodData::MobilePayment(_)
                    | PaymentMethodData::Upi(_)
                    | PaymentMethodData::Voucher(_)
                    | PaymentMethodData::GiftCard(_)
                    | PaymentMethodData::OpenBanking(_)
                    | PaymentMethodData::CardToken(_)
                    | PaymentMethodData::NetworkToken(_)
                    | PaymentMethodData::Card(_) => Err(errors::ConnectorError::NotSupported {
                        message: "Network tokenization for payment method".to_string(),
                        connector: "Adyen",
                    })?,
                }
            }
            payments::MandateReferenceId::NetworkTokenWithNTI(network_mandate_id) => {
                match item.router_data.request.payment_method_data {
                    PaymentMethodData::NetworkToken(ref token_data) => {
                        let card_issuer = token_data.get_card_issuer()?;
                        let brand = CardBrand::try_from(&card_issuer)?;
                        let card_holder_name = item.router_data.get_optional_billing_full_name();
                        let adyen_network_token = AdyenNetworkTokenData {
                            number: token_data.get_network_token(),
                            expiry_month: token_data.get_network_token_expiry_month(),
                            expiry_year: token_data.get_expiry_year_4_digit(),
                            holder_name: test_holder_name.or(card_holder_name),
                            brand: Some(brand), // FIXME: Remove hardcoding
                            network_payment_reference: Some(Secret::new(
                                network_mandate_id.network_transaction_id,
                            )),
                        };
                        Ok(PaymentMethod::AdyenPaymentMethod(Box::new(
                            AdyenPaymentMethod::NetworkToken(Box::new(adyen_network_token)),
                        )))
                    }

                    PaymentMethodData::Card(_)
                    | PaymentMethodData::CardRedirect(_)
                    | PaymentMethodData::Wallet(_)
                    | PaymentMethodData::PayLater(_)
                    | PaymentMethodData::BankRedirect(_)
                    | PaymentMethodData::BankDebit(_)
                    | PaymentMethodData::BankTransfer(_)
                    | PaymentMethodData::Crypto(_)
                    | PaymentMethodData::MandatePayment
                    | PaymentMethodData::Reward
                    | PaymentMethodData::RealTimePayment(_)
                    | PaymentMethodData::Upi(_)
                    | PaymentMethodData::Voucher(_)
                    | PaymentMethodData::GiftCard(_)
                    | PaymentMethodData::OpenBanking(_)
                    | PaymentMethodData::CardToken(_)
                    | PaymentMethodData::MobilePayment(_)
                    | PaymentMethodData::CardDetailsForNetworkTransactionId(_) => {
                        Err(errors::ConnectorError::NotSupported {
                            message: "Network tokenization for payment method".to_string(),
                            connector: "Adyen",
                        })?
                    }
                }
            } //
        }?;

        let adyen_metadata = get_adyen_metadata(item.router_data.request.metadata.clone());

        let (store, splits) = match item.router_data.request.split_payments.as_ref() {
            Some(common_types::payments::SplitPaymentsRequest::AdyenSplitPayment(
                adyen_split_payment,
            )) => get_adyen_split_request(adyen_split_payment, item.router_data.request.currency),
            _ => (adyen_metadata.store.clone(), None),
        };
        let device_fingerprint = adyen_metadata.device_fingerprint.clone();
        let platform_chargeback_logic = adyen_metadata.platform_chargeback_logic.clone();

        let billing_address =
            get_address_info(item.router_data.get_optional_billing()).and_then(Result::ok);
        let delivery_address =
            get_address_info(item.router_data.get_optional_shipping()).and_then(Result::ok);
        let telephone_number = item.router_data.get_optional_billing_phone_number();
        let application_info = get_application_info(item);

        Ok(AdyenPaymentRequest {
            amount,
            merchant_account: auth_type.merchant_account,
            payment_method,
            reference: item.router_data.connector_request_reference_id.clone(),
            return_url,
            shopper_interaction,
            recurring_processing_model,
            browser_info,
            additional_data,
            mpi_data: None,
            telephone_number,
            shopper_name: None,
            shopper_email: None,
            shopper_locale: item.router_data.request.locale.clone(),
            social_security_number: None,
            billing_address,
            delivery_address,
            country_code: None,
            line_items: None,
            shopper_reference,
            store_payment_method,
            channel: None,
            shopper_statement: get_shopper_statement(item.router_data),
            shopper_ip: item.router_data.request.get_ip_address_as_optional(),
            merchant_order_reference: item.router_data.request.merchant_order_reference_id.clone(),
            store,
            splits,
            device_fingerprint,
            session_validity: None,
            metadata: item
                .router_data
                .request
                .metadata
                .clone()
                .map(filter_adyen_metadata),
            platform_chargeback_logic,
            application_info,
        })
    }
}
impl TryFrom<(&AdyenRouterData<&PaymentsAuthorizeRouterData>, &Card)> for AdyenPaymentRequest<'_> {
    type Error = Error;
    fn try_from(
        value: (&AdyenRouterData<&PaymentsAuthorizeRouterData>, &Card),
    ) -> Result<Self, Self::Error> {
        let (item, card_data) = value;
        let amount = get_amount_data(item);
        let auth_type = AdyenAuthType::try_from(&item.router_data.connector_auth_type)?;
        let shopper_interaction = AdyenShopperInteraction::from(item.router_data);
        let shopper_reference = item.router_data.get_connector_customer_id().ok();
        let (recurring_processing_model, store_payment_method, _) =
            get_recurring_processing_model(item.router_data)?;
        let browser_info = get_browser_info(item.router_data)?;
        let billing_address =
            get_address_info(item.router_data.get_optional_billing()).and_then(Result::ok);
        let country_code = get_country_code(item.router_data.get_optional_billing());
        let additional_data = get_additional_data(item.router_data);
        let return_url = item.router_data.request.get_router_return_url()?;
        let testing_data = item
            .router_data
            .request
            .get_connector_testing_data()
            .map(AdyenTestingData::try_from)
            .transpose()?;
        let test_holder_name = testing_data.and_then(|test_data| test_data.holder_name);
        let card_holder_name =
            test_holder_name.or(item.router_data.get_optional_billing_full_name());
        let payment_method = PaymentMethod::AdyenPaymentMethod(Box::new(
            AdyenPaymentMethod::try_from((card_data, card_holder_name))?,
        ));

        let shopper_email = item.router_data.get_optional_billing_email();
        let shopper_name = get_shopper_name(item.router_data.get_optional_billing());
        let adyen_metadata = get_adyen_metadata(item.router_data.request.metadata.clone());
        let (store, splits) = match item.router_data.request.split_payments.as_ref() {
            Some(common_types::payments::SplitPaymentsRequest::AdyenSplitPayment(
                adyen_split_payment,
            )) => get_adyen_split_request(adyen_split_payment, item.router_data.request.currency),
            _ => (adyen_metadata.store.clone(), None),
        };
        let device_fingerprint = adyen_metadata.device_fingerprint.clone();
        let platform_chargeback_logic = adyen_metadata.platform_chargeback_logic.clone();

        let delivery_address =
            get_address_info(item.router_data.get_optional_shipping()).and_then(Result::ok);
        let telephone_number = item.router_data.get_optional_billing_phone_number();
        let application_info = get_application_info(item);

        let mpi_data =
            if let Some(auth_data) = value.0.router_data.request.authentication_data.as_ref() {
                let (cavv_algorithm, challenge_cancel, risk_score) =
                    match &value.0.router_data.request.payment_method_data {
                        PaymentMethodData::Card(card)
                            if matches!(
                                card.card_network,
                                Some(common_enums::CardNetwork::CartesBancaires)
                            ) =>
                        {
                            let cartes_params = auth_data
                                .cb_network_params
                                .as_ref()
                                .and_then(|net| net.cartes_bancaires.as_ref());

                            (
                                cartes_params.as_ref().map(|cb| cb.cavv_algorithm.clone()),
                                cartes_params.as_ref().map(|cb| cb.cb_exemption.clone()),
                                cartes_params.as_ref().map(|cb| cb.cb_score.to_string()),
                            )
                        }
                        _ => (None, None, None),
                    };

                Some(AdyenMpiData {
                    directory_response: auth_data.transaction_status.clone().ok_or(
                        errors::ConnectorError::MissingRequiredField {
                            field_name: "three_ds_data.transaction_status",
                        },
                    )?,
                    authentication_response: auth_data.transaction_status.clone().ok_or(
                        errors::ConnectorError::MissingRequiredField {
                            field_name: "three_ds_data.transaction_status",
                        },
                    )?,
                    cavv: Some(auth_data.cavv.clone()),
                    token_authentication_verification_value: None,
                    eci: auth_data.eci.clone(),
                    ds_trans_id: auth_data.ds_trans_id.clone(),
                    three_ds_version: auth_data.message_version.clone(),
                    cavv_algorithm,
                    challenge_cancel,
                    risk_score,
                })
            } else {
                None
            };

        Ok(AdyenPaymentRequest {
            amount,
            merchant_account: auth_type.merchant_account,
            payment_method,
            reference: item.router_data.connector_request_reference_id.clone(),
            return_url,
            shopper_interaction,
            recurring_processing_model,
            browser_info,
            additional_data,
            mpi_data,
            telephone_number,
            shopper_name,
            shopper_email,
            shopper_locale: item.router_data.request.locale.clone(),
            social_security_number: None,
            billing_address,
            delivery_address,
            country_code,
            line_items: None,
            shopper_reference,
            store_payment_method,
            channel: None,
            shopper_statement: get_shopper_statement(item.router_data),
            shopper_ip: item.router_data.request.get_ip_address_as_optional(),
            merchant_order_reference: item.router_data.request.merchant_order_reference_id.clone(),
            store,
            splits,
            device_fingerprint,
            session_validity: None,
            metadata: item
                .router_data
                .request
                .metadata
                .clone()
                .map(filter_adyen_metadata),
            platform_chargeback_logic,
            application_info,
        })
    }
}

impl
    TryFrom<(
        &AdyenRouterData<&PaymentsAuthorizeRouterData>,
        &BankDebitData,
    )> for AdyenPaymentRequest<'_>
{
    type Error = Error;

    fn try_from(
        value: (
            &AdyenRouterData<&PaymentsAuthorizeRouterData>,
            &BankDebitData,
        ),
    ) -> Result<Self, Self::Error> {
        let (item, bank_debit_data) = value;
        let amount = get_amount_data(item);
        let auth_type = AdyenAuthType::try_from(&item.router_data.connector_auth_type)?;
        let shopper_interaction = AdyenShopperInteraction::from(item.router_data);
        let (recurring_processing_model, store_payment_method, shopper_reference) =
            get_recurring_processing_model(item.router_data)?;
        let browser_info = get_browser_info(item.router_data)?;
        let additional_data = get_additional_data(item.router_data);
        let return_url = item.router_data.request.get_router_return_url()?;
        let payment_method = PaymentMethod::AdyenPaymentMethod(Box::new(
            AdyenPaymentMethod::try_from((bank_debit_data, item.router_data))?,
        ));

        let country_code = get_country_code(item.router_data.get_optional_billing());
        let adyen_metadata = get_adyen_metadata(item.router_data.request.metadata.clone());

        let (store, splits) = match item.router_data.request.split_payments.as_ref() {
            Some(common_types::payments::SplitPaymentsRequest::AdyenSplitPayment(
                adyen_split_payment,
            )) => get_adyen_split_request(adyen_split_payment, item.router_data.request.currency),
            _ => (adyen_metadata.store.clone(), None),
        };
        let device_fingerprint = adyen_metadata.device_fingerprint.clone();
        let platform_chargeback_logic = adyen_metadata.platform_chargeback_logic.clone();

        let mut billing_address =
            get_address_info(item.router_data.get_optional_billing()).and_then(Result::ok);
        let delivery_address =
            get_address_info(item.router_data.get_optional_shipping()).and_then(Result::ok);
        let telephone_number = item.router_data.get_optional_billing_phone_number();

        if let BankDebitData::AchBankDebit { .. } = bank_debit_data {
            if let Some(addr) = billing_address.as_mut() {
                addr.state_or_province = Some(item.router_data.get_billing_state_code()?);
            }
        }
        let application_info = get_application_info(item);

        let request = AdyenPaymentRequest {
            amount,
            merchant_account: auth_type.merchant_account,
            payment_method,
            reference: item.router_data.connector_request_reference_id.clone(),
            return_url,
            browser_info,
            shopper_interaction,
            recurring_processing_model,
            additional_data,
            mpi_data: None,
            shopper_name: None,
            shopper_locale: item.router_data.request.locale.clone(),
            shopper_email: item.router_data.get_optional_billing_email(),
            social_security_number: None,
            telephone_number,
            billing_address,
            delivery_address,
            country_code,
            line_items: None,
            shopper_reference,
            store_payment_method,
            channel: None,
            shopper_statement: get_shopper_statement(item.router_data),
            shopper_ip: item.router_data.request.get_ip_address_as_optional(),
            merchant_order_reference: item.router_data.request.merchant_order_reference_id.clone(),
            store,
            splits,
            device_fingerprint,
            session_validity: None,
            metadata: item
                .router_data
                .request
                .metadata
                .clone()
                .map(filter_adyen_metadata),
            platform_chargeback_logic,
            application_info,
        };
        Ok(request)
    }
}

impl TryFrom<(&AdyenRouterData<&PaymentsAuthorizeRouterData>, &VoucherData)>
    for AdyenPaymentRequest<'_>
{
    type Error = Error;

    fn try_from(
        value: (&AdyenRouterData<&PaymentsAuthorizeRouterData>, &VoucherData),
    ) -> Result<Self, Self::Error> {
        let (item, voucher_data) = value;
        let amount = get_amount_data(item);
        let auth_type = AdyenAuthType::try_from(&item.router_data.connector_auth_type)?;
        let shopper_interaction = AdyenShopperInteraction::from(item.router_data);
        let recurring_processing_model = get_recurring_processing_model(item.router_data)?.0;
        let browser_info = get_browser_info(item.router_data)?;
        let additional_data = get_additional_data(item.router_data);
        let payment_method = PaymentMethod::AdyenPaymentMethod(Box::new(
            AdyenPaymentMethod::try_from((voucher_data, item.router_data))?,
        ));
        let return_url = item.router_data.request.get_router_return_url()?;
        let social_security_number = get_social_security_number(voucher_data);
        let billing_address =
            get_address_info(item.router_data.get_optional_billing()).and_then(Result::ok);
        let shopper_name = get_shopper_name(item.router_data.get_optional_billing());
        let adyen_metadata = get_adyen_metadata(item.router_data.request.metadata.clone());

        let (store, splits) = match item.router_data.request.split_payments.as_ref() {
            Some(common_types::payments::SplitPaymentsRequest::AdyenSplitPayment(
                adyen_split_payment,
            )) => get_adyen_split_request(adyen_split_payment, item.router_data.request.currency),
            _ => (adyen_metadata.store.clone(), None),
        };
        let device_fingerprint = adyen_metadata.device_fingerprint.clone();
        let platform_chargeback_logic = adyen_metadata.platform_chargeback_logic.clone();

        let delivery_address =
            get_address_info(item.router_data.get_optional_shipping()).and_then(Result::ok);
        let telephone_number = item.router_data.get_optional_billing_phone_number();
        let application_info = get_application_info(item);

        let request = AdyenPaymentRequest {
            amount,
            merchant_account: auth_type.merchant_account,
            payment_method,
            reference: item.router_data.connector_request_reference_id.to_string(),
            return_url,
            browser_info,
            shopper_interaction,
            recurring_processing_model,
            additional_data,
            shopper_name,
            shopper_locale: item.router_data.request.locale.clone(),
            shopper_email: item.router_data.get_optional_billing_email(),
            social_security_number,
            mpi_data: None,
            telephone_number,
            billing_address,
            delivery_address,
            country_code: None,
            line_items: None,
            shopper_reference: None,
            store_payment_method: None,
            channel: None,
            shopper_statement: get_shopper_statement(item.router_data),
            shopper_ip: item.router_data.request.get_ip_address_as_optional(),
            merchant_order_reference: item.router_data.request.merchant_order_reference_id.clone(),
            store,
            splits,
            device_fingerprint,
            session_validity: None,
            metadata: item
                .router_data
                .request
                .metadata
                .clone()
                .map(filter_adyen_metadata),

            platform_chargeback_logic,
            application_info,
        };
        Ok(request)
    }
}

impl
    TryFrom<(
        &AdyenRouterData<&PaymentsAuthorizeRouterData>,
        &BankTransferData,
    )> for AdyenPaymentRequest<'_>
{
    type Error = Error;

    fn try_from(
        value: (
            &AdyenRouterData<&PaymentsAuthorizeRouterData>,
            &BankTransferData,
        ),
    ) -> Result<Self, Self::Error> {
        let (item, bank_transfer_data) = value;
        let amount = get_amount_data(item);
        let auth_type = AdyenAuthType::try_from(&item.router_data.connector_auth_type)?;
        let shopper_interaction = AdyenShopperInteraction::from(item.router_data);
        let payment_method = PaymentMethod::AdyenPaymentMethod(Box::new(
            AdyenPaymentMethod::try_from((bank_transfer_data, item.router_data))?,
        ));

        let return_url = item.router_data.request.get_router_return_url()?;
        let adyen_metadata = get_adyen_metadata(item.router_data.request.metadata.clone());

        let (store, splits) = match item.router_data.request.split_payments.as_ref() {
            Some(common_types::payments::SplitPaymentsRequest::AdyenSplitPayment(
                adyen_split_payment,
            )) => get_adyen_split_request(adyen_split_payment, item.router_data.request.currency),
            _ => (adyen_metadata.store.clone(), None),
        };
        let device_fingerprint = adyen_metadata.device_fingerprint.clone();
        let platform_chargeback_logic = adyen_metadata.platform_chargeback_logic.clone();
        let billing_address =
            get_address_info(item.router_data.get_optional_billing()).and_then(Result::ok);
        let delivery_address =
            get_address_info(item.router_data.get_optional_shipping()).and_then(Result::ok);
        let telephone_number = item.router_data.get_optional_billing_phone_number();
        let (session_validity, social_security_number) = match bank_transfer_data {
            BankTransferData::Pix {
                cpf,
                cnpj,
                expiry_date,
                ..
            } => {
                // Validate expiry_date doesn't exceed 5 days from now
                if let Some(expiry) = expiry_date {
                    let now = OffsetDateTime::now_utc();
                    let max_expiry = now + Duration::days(5);
                    let max_expiry_primitive =
                        PrimitiveDateTime::new(max_expiry.date(), max_expiry.time());

                    if *expiry > max_expiry_primitive {
                        return Err(report!(errors::ConnectorError::InvalidDataFormat {
                            field_name: "expiry_date cannot be more than 5 days from now",
                        }));
                    }
                }

                (*expiry_date, cpf.as_ref().or(cnpj.as_ref()).cloned())
            }
            BankTransferData::LocalBankTransfer { .. } => (None, None),
            BankTransferData::AchBankTransfer {}
            | BankTransferData::SepaBankTransfer {}
            | BankTransferData::BacsBankTransfer {}
            | BankTransferData::MultibancoBankTransfer {}
            | BankTransferData::PermataBankTransfer {}
            | BankTransferData::BcaBankTransfer {}
            | BankTransferData::BniVaBankTransfer {}
            | BankTransferData::BriVaBankTransfer {}
            | BankTransferData::CimbVaBankTransfer {}
            | BankTransferData::DanamonVaBankTransfer {}
            | BankTransferData::MandiriVaBankTransfer {}
            | BankTransferData::Pse {}
            | BankTransferData::InstantBankTransfer {}
            | BankTransferData::InstantBankTransferFinland {}
            | BankTransferData::InstantBankTransferPoland {}
            | BankTransferData::IndonesianBankTransfer { .. } => (None, None),
        };
        let application_info = get_application_info(item);

        let request = AdyenPaymentRequest {
            amount,
            merchant_account: auth_type.merchant_account,
            payment_method,
            reference: item.router_data.connector_request_reference_id.to_string(),
            return_url,
            browser_info: None,
            shopper_interaction,
            recurring_processing_model: None,
            additional_data: None,
            mpi_data: None,
            shopper_name: None,
            shopper_locale: item.router_data.request.locale.clone(),
            shopper_email: item.router_data.get_optional_billing_email(),
            social_security_number,
            telephone_number,
            billing_address,
            delivery_address,
            country_code: None,
            line_items: None,
            shopper_reference: None,
            store_payment_method: None,
            channel: None,
            shopper_statement: get_shopper_statement(item.router_data),
            shopper_ip: item.router_data.request.get_ip_address_as_optional(),
            merchant_order_reference: item.router_data.request.merchant_order_reference_id.clone(),
            store,
            splits,
            device_fingerprint,
            session_validity,
            metadata: item
                .router_data
                .request
                .metadata
                .clone()
                .map(filter_adyen_metadata),

            platform_chargeback_logic,
            application_info,
        };
        Ok(request)
    }
}

impl
    TryFrom<(
        &AdyenRouterData<&PaymentsAuthorizeRouterData>,
        &GiftCardData,
    )> for AdyenPaymentRequest<'_>
{
    type Error = Error;

    fn try_from(
        value: (
            &AdyenRouterData<&PaymentsAuthorizeRouterData>,
            &GiftCardData,
        ),
    ) -> Result<Self, Self::Error> {
        let (item, gift_card_data) = value;
        let amount = get_amount_data(item);
        let auth_type = AdyenAuthType::try_from(&item.router_data.connector_auth_type)?;
        let shopper_interaction = AdyenShopperInteraction::from(item.router_data);
        let return_url = item.router_data.request.get_router_return_url()?;
        let payment_method = PaymentMethod::AdyenPaymentMethod(Box::new(
            AdyenPaymentMethod::try_from(gift_card_data)?,
        ));
        let adyen_metadata = get_adyen_metadata(item.router_data.request.metadata.clone());

        let (store, splits) = match item.router_data.request.split_payments.as_ref() {
            Some(common_types::payments::SplitPaymentsRequest::AdyenSplitPayment(
                adyen_split_payment,
            )) => get_adyen_split_request(adyen_split_payment, item.router_data.request.currency),
            _ => (adyen_metadata.store.clone(), None),
        };
        let device_fingerprint = adyen_metadata.device_fingerprint.clone();
        let platform_chargeback_logic = adyen_metadata.platform_chargeback_logic.clone();

        let billing_address =
            get_address_info(item.router_data.get_optional_billing()).and_then(Result::ok);
        let delivery_address =
            get_address_info(item.router_data.get_optional_shipping()).and_then(Result::ok);
        let telephone_number = item.router_data.get_optional_billing_phone_number();
        let application_info = get_application_info(item);

        let request = AdyenPaymentRequest {
            amount,
            merchant_account: auth_type.merchant_account,
            payment_method,
            reference: item.router_data.connector_request_reference_id.to_string(),
            return_url,
            browser_info: None,
            shopper_interaction,
            recurring_processing_model: None,
            additional_data: None,
            mpi_data: None,
            shopper_name: None,
            shopper_locale: item.router_data.request.locale.clone(),
            shopper_email: item.router_data.get_optional_billing_email(),
            telephone_number,
            billing_address,
            delivery_address,
            country_code: None,
            line_items: None,
            shopper_reference: None,
            store_payment_method: None,
            channel: None,
            social_security_number: None,
            shopper_statement: get_shopper_statement(item.router_data),
            shopper_ip: item.router_data.request.get_ip_address_as_optional(),
            merchant_order_reference: item.router_data.request.merchant_order_reference_id.clone(),
            store,
            splits,
            device_fingerprint,
            session_validity: None,
            metadata: item
                .router_data
                .request
                .metadata
                .clone()
                .map(filter_adyen_metadata),
            platform_chargeback_logic,
            application_info,
        };
        Ok(request)
    }
}

impl
    TryFrom<(
        &AdyenRouterData<&PaymentsAuthorizeRouterData>,
        &BankRedirectData,
    )> for AdyenPaymentRequest<'_>
{
    type Error = Error;
    fn try_from(
        value: (
            &AdyenRouterData<&PaymentsAuthorizeRouterData>,
            &BankRedirectData,
        ),
    ) -> Result<Self, Self::Error> {
        let (item, bank_redirect_data) = value;
        let amount = get_amount_data(item);
        let auth_type = AdyenAuthType::try_from(&item.router_data.connector_auth_type)?;
        let shopper_interaction = AdyenShopperInteraction::from(item.router_data);
        let (recurring_processing_model, store_payment_method, shopper_reference) =
            get_recurring_processing_model(item.router_data)?;
        let browser_info = get_browser_info(item.router_data)?;
        let additional_data = get_additional_data(item.router_data);
        let return_url = item.router_data.request.get_router_return_url()?;
        let payment_method = PaymentMethod::AdyenPaymentMethod(Box::new(
            AdyenPaymentMethod::try_from((bank_redirect_data, item.router_data))?,
        ));
        let (shopper_locale, country) = get_redirect_extra_details(item.router_data)?;
        let line_items = Some(get_line_items(item));
        let billing_address =
            get_address_info(item.router_data.get_optional_billing()).and_then(Result::ok);
        let adyen_metadata = get_adyen_metadata(item.router_data.request.metadata.clone());

        let (store, splits) = match item.router_data.request.split_payments.as_ref() {
            Some(common_types::payments::SplitPaymentsRequest::AdyenSplitPayment(
                adyen_split_payment,
            )) => get_adyen_split_request(adyen_split_payment, item.router_data.request.currency),
            _ => (adyen_metadata.store.clone(), None),
        };
        let device_fingerprint = adyen_metadata.device_fingerprint.clone();
        let platform_chargeback_logic = adyen_metadata.platform_chargeback_logic.clone();

        let delivery_address =
            get_address_info(item.router_data.get_optional_shipping()).and_then(Result::ok);
        let telephone_number = item.router_data.get_optional_billing_phone_number();
        let application_info = get_application_info(item);

        Ok(AdyenPaymentRequest {
            amount,
            merchant_account: auth_type.merchant_account,
            payment_method,
            reference: item.router_data.connector_request_reference_id.clone(),
            return_url,
            shopper_interaction,
            recurring_processing_model,
            browser_info,
            additional_data,
            mpi_data: None,
            telephone_number,
            shopper_name: None,
            shopper_email: item.router_data.get_optional_billing_email(),
            shopper_locale,
            social_security_number: None,
            billing_address,
            delivery_address,
            country_code: country,
            line_items,
            shopper_reference,
            store_payment_method,
            channel: None,
            shopper_statement: get_shopper_statement(item.router_data),
            shopper_ip: item.router_data.request.get_ip_address_as_optional(),
            merchant_order_reference: item.router_data.request.merchant_order_reference_id.clone(),
            store,
            splits,
            device_fingerprint,
            session_validity: None,
            metadata: item
                .router_data
                .request
                .metadata
                .clone()
                .map(filter_adyen_metadata),
            platform_chargeback_logic,
            application_info,
        })
    }
}

fn get_redirect_extra_details(
    item: &PaymentsAuthorizeRouterData,
) -> CustomResult<(Option<String>, Option<storage_enums::CountryAlpha2>), errors::ConnectorError> {
    match item.request.payment_method_data {
        PaymentMethodData::BankRedirect(
            BankRedirectData::Trustly { .. } | BankRedirectData::OpenBankingUk { .. },
        ) => {
            let country = item.get_optional_billing_country();
            Ok((item.request.locale.clone(), country))
        }
        _ => Ok((None, None)),
    }
}

fn get_shopper_email(
    item: &PaymentsAuthorizeRouterData,
    is_mandate_payment: bool,
) -> CustomResult<Option<Email>, errors::ConnectorError> {
    if is_mandate_payment {
        let payment_method_type = item
            .request
            .payment_method_type
            .as_ref()
            .ok_or(errors::ConnectorError::MissingPaymentMethodType)?;
        match payment_method_type {
            storage_enums::PaymentMethodType::Paypal => Ok(Some(item.get_billing_email()?)),
            _ => Ok(item.get_optional_billing_email()),
        }
    } else {
        Ok(item.get_optional_billing_email())
    }
}

impl TryFrom<(&AdyenRouterData<&PaymentsAuthorizeRouterData>, &WalletData)>
    for AdyenPaymentRequest<'_>
{
    type Error = Error;
    fn try_from(
        value: (&AdyenRouterData<&PaymentsAuthorizeRouterData>, &WalletData),
    ) -> Result<Self, Self::Error> {
        let (item, wallet_data) = value;
        let amount = get_amount_data(item);
        let auth_type = AdyenAuthType::try_from(&item.router_data.connector_auth_type)?;
        let browser_info = get_browser_info(item.router_data)?;
        let additional_data = get_additional_data(item.router_data);
        let payment_method = PaymentMethod::AdyenPaymentMethod(Box::new(
            AdyenPaymentMethod::try_from((wallet_data, item.router_data))?,
        ));
        let shopper_interaction = AdyenShopperInteraction::from(item.router_data);
        let channel = get_channel_type(item.router_data.request.payment_method_type);
        let (recurring_processing_model, store_payment_method, shopper_reference) =
            get_recurring_processing_model(item.router_data)?;
        let return_url = item.router_data.request.get_router_return_url()?;
        let shopper_email = get_shopper_email(item.router_data, store_payment_method.is_some())?;
        let billing_address =
            get_address_info(item.router_data.get_optional_billing()).and_then(Result::ok);
        let mpi_data = if matches!(wallet_data, WalletData::Paze(_) | WalletData::ApplePay(_)) {
            match item.router_data.payment_method_token.clone() {
                Some(PaymentMethodToken::PazeDecrypt(paze_data)) => Some(AdyenMpiData {
                    directory_response: common_enums::TransactionStatus::Success,
                    authentication_response: common_enums::TransactionStatus::Success,
                    cavv: None,
                    token_authentication_verification_value: Some(
                        paze_data.token.payment_account_reference,
                    ),
                    eci: paze_data.eci.clone(),
                    ds_trans_id: None,
                    three_ds_version: None,
                    challenge_cancel: None,
                    risk_score: None,
                    cavv_algorithm: None,
                }),
                Some(PaymentMethodToken::ApplePayDecrypt(apple_data)) => Some(AdyenMpiData {
                    directory_response: common_enums::TransactionStatus::Success,
                    authentication_response: common_enums::TransactionStatus::Success,
                    cavv: Some(apple_data.payment_data.online_payment_cryptogram),
                    token_authentication_verification_value: None,
                    eci: apple_data.payment_data.eci_indicator.clone(),
                    ds_trans_id: None,
                    three_ds_version: None,
                    challenge_cancel: None,
                    risk_score: None,
                    cavv_algorithm: None,
                }),
                _ => None,
            }
        } else {
            None
        };

        let adyen_metadata = get_adyen_metadata(item.router_data.request.metadata.clone());

        let (store, splits) = match item.router_data.request.split_payments.as_ref() {
            Some(common_types::payments::SplitPaymentsRequest::AdyenSplitPayment(
                adyen_split_payment,
            )) => get_adyen_split_request(adyen_split_payment, item.router_data.request.currency),
            _ => (adyen_metadata.store.clone(), None),
        };
        let device_fingerprint = adyen_metadata.device_fingerprint.clone();
        let platform_chargeback_logic = adyen_metadata.platform_chargeback_logic.clone();

        let delivery_address =
            get_address_info(item.router_data.get_optional_shipping()).and_then(Result::ok);
        let telephone_number = item.router_data.get_optional_billing_phone_number();
        let application_info = get_application_info(item);

        Ok(AdyenPaymentRequest {
            amount,
            merchant_account: auth_type.merchant_account,
            payment_method,
            reference: item.router_data.connector_request_reference_id.clone(),
            return_url,
            shopper_interaction,
            recurring_processing_model,
            browser_info,
            additional_data,
            mpi_data,
            telephone_number,
            shopper_name: None,
            shopper_email,
            shopper_locale: item.router_data.request.locale.clone(),
            social_security_number: None,
            billing_address,
            delivery_address,
            country_code: None,
            line_items: None,
            shopper_reference,
            store_payment_method,
            channel,
            shopper_statement: get_shopper_statement(item.router_data),
            shopper_ip: item.router_data.request.get_ip_address_as_optional(),
            merchant_order_reference: item.router_data.request.merchant_order_reference_id.clone(),
            store,
            splits,
            device_fingerprint,
            session_validity: None,
            metadata: item
                .router_data
                .request
                .metadata
                .clone()
                .map(filter_adyen_metadata),
            platform_chargeback_logic,
            application_info,
        })
    }
}

impl
    TryFrom<(
        &AdyenRouterData<&PaymentsAuthorizeRouterData>,
        &PayLaterData,
    )> for AdyenPaymentRequest<'_>
{
    type Error = Error;
    fn try_from(
        value: (
            &AdyenRouterData<&PaymentsAuthorizeRouterData>,
            &PayLaterData,
        ),
    ) -> Result<Self, Self::Error> {
        let (item, paylater_data) = value;
        let amount = get_amount_data(item);
        let auth_type = AdyenAuthType::try_from(&item.router_data.connector_auth_type)?;
        let browser_info = get_browser_info(item.router_data)?;
        let additional_data = get_additional_data(item.router_data);
        let country_code = get_country_code(item.router_data.get_optional_billing());
        let shopper_interaction = AdyenShopperInteraction::from(item.router_data);
        let shopper_reference = item.router_data.get_connector_customer_id().ok();
        let (recurring_processing_model, store_payment_method, _) =
            get_recurring_processing_model(item.router_data)?;
        let return_url = item.router_data.request.get_router_return_url()?;
        let shopper_name = get_shopper_name(item.router_data.get_optional_billing());
        let shopper_email = item.router_data.get_optional_billing_email();
        let billing_address =
            get_address_info(item.router_data.get_optional_billing()).and_then(Result::ok);
        let delivery_address =
            get_address_info(item.router_data.get_optional_shipping()).and_then(Result::ok);
        let line_items = Some(get_line_items(item));
        let telephone_number = get_telephone_number(item.router_data);
        let payment_method =
            PaymentMethod::AdyenPaymentMethod(Box::new(AdyenPaymentMethod::try_from((
                paylater_data,
                &country_code,
                &shopper_email,
                &shopper_reference,
                &shopper_name,
                &telephone_number,
                &billing_address,
                &delivery_address,
            ))?));
        let adyen_metadata = get_adyen_metadata(item.router_data.request.metadata.clone());
        let (store, splits) = match item.router_data.request.split_payments.as_ref() {
            Some(common_types::payments::SplitPaymentsRequest::AdyenSplitPayment(
                adyen_split_payment,
            )) => get_adyen_split_request(adyen_split_payment, item.router_data.request.currency),
            _ => (adyen_metadata.store.clone(), None),
        };
        let device_fingerprint = adyen_metadata.device_fingerprint.clone();
        let platform_chargeback_logic = adyen_metadata.platform_chargeback_logic.clone();
        let application_info = get_application_info(item);

        Ok(AdyenPaymentRequest {
            amount,
            merchant_account: auth_type.merchant_account,
            payment_method,
            reference: item.router_data.connector_request_reference_id.clone(),
            return_url,
            shopper_interaction,
            recurring_processing_model,
            browser_info,
            additional_data,
            telephone_number,
            shopper_name,
            shopper_email,
            mpi_data: None,
            shopper_locale: item.router_data.request.locale.clone(),
            social_security_number: None,
            billing_address,
            delivery_address,
            country_code,
            line_items,
            shopper_reference,
            store_payment_method,
            channel: None,
            shopper_statement: get_shopper_statement(item.router_data),
            shopper_ip: item.router_data.request.get_ip_address_as_optional(),
            merchant_order_reference: item.router_data.request.merchant_order_reference_id.clone(),
            store,
            splits,
            device_fingerprint,
            session_validity: None,
            metadata: item
                .router_data
                .request
                .metadata
                .clone()
                .map(filter_adyen_metadata),
            platform_chargeback_logic,
            application_info,
        })
    }
}

impl
    TryFrom<(
        &AdyenRouterData<&PaymentsAuthorizeRouterData>,
        &CardRedirectData,
    )> for AdyenPaymentRequest<'_>
{
    type Error = Error;
    fn try_from(
        value: (
            &AdyenRouterData<&PaymentsAuthorizeRouterData>,
            &CardRedirectData,
        ),
    ) -> Result<Self, Self::Error> {
        let (item, card_redirect_data) = value;
        let amount = get_amount_data(item);
        let auth_type = AdyenAuthType::try_from(&item.router_data.connector_auth_type)?;
        let payment_method = PaymentMethod::AdyenPaymentMethod(Box::new(
            AdyenPaymentMethod::try_from(card_redirect_data)?,
        ));

        let shopper_interaction = AdyenShopperInteraction::from(item.router_data);
        let return_url = item.router_data.request.get_router_return_url()?;
        let shopper_name = get_shopper_name(item.router_data.get_optional_billing());
        let shopper_email = item.router_data.get_optional_billing_email();
        let telephone_number = item
            .router_data
            .get_billing_phone()
            .change_context(errors::ConnectorError::MissingRequiredField {
                field_name: "billing.phone",
            })?
            .number
            .to_owned();
        let adyen_metadata = get_adyen_metadata(item.router_data.request.metadata.clone());

        let (store, splits) = match item.router_data.request.split_payments.as_ref() {
            Some(common_types::payments::SplitPaymentsRequest::AdyenSplitPayment(
                adyen_split_payment,
            )) => get_adyen_split_request(adyen_split_payment, item.router_data.request.currency),
            _ => (adyen_metadata.store.clone(), None),
        };
        let device_fingerprint = adyen_metadata.device_fingerprint.clone();
        let platform_chargeback_logic = adyen_metadata.platform_chargeback_logic.clone();
        let billing_address =
            get_address_info(item.router_data.get_optional_billing()).and_then(Result::ok);
        let delivery_address =
            get_address_info(item.router_data.get_optional_shipping()).and_then(Result::ok);
        let application_info = get_application_info(item);
        Ok(AdyenPaymentRequest {
            amount,
            merchant_account: auth_type.merchant_account,
            payment_method,
            reference: item.router_data.connector_request_reference_id.to_string(),
            return_url,
            shopper_interaction,
            recurring_processing_model: None,
            browser_info: None,
            additional_data: None,
            mpi_data: None,
            telephone_number,
            shopper_name,
            shopper_email,
            shopper_locale: item.router_data.request.locale.clone(),
            billing_address,
            delivery_address,
            country_code: None,
            line_items: None,
            shopper_reference: None,
            store_payment_method: None,
            channel: None,
            social_security_number: None,
            shopper_statement: get_shopper_statement(item.router_data),
            shopper_ip: item.router_data.request.get_ip_address_as_optional(),
            merchant_order_reference: item.router_data.request.merchant_order_reference_id.clone(),
            store,
            splits,
            device_fingerprint,
            session_validity: None,
            metadata: item
                .router_data
                .request
                .metadata
                .clone()
                .map(filter_adyen_metadata),
            platform_chargeback_logic,
            application_info,
        })
    }
}

impl TryFrom<&PaymentsCancelRouterData> for AdyenCancelRequest {
    type Error = Error;
    fn try_from(item: &PaymentsCancelRouterData) -> Result<Self, Self::Error> {
        let auth_type = AdyenAuthType::try_from(&item.connector_auth_type)?;
        Ok(Self {
            merchant_account: auth_type.merchant_account,
            reference: item.connector_request_reference_id.clone(),
        })
    }
}

fn get_adyen_split_request(
    split_request: &common_types::domain::AdyenSplitData,
    currency: common_enums::enums::Currency,
) -> (Option<String>, Option<Vec<AdyenSplitData>>) {
    let splits = split_request
        .split_items
        .iter()
        .map(|split_item| {
            let amount = split_item.amount.map(|value| Amount { currency, value });
            AdyenSplitData {
                amount,
                reference: split_item.reference.clone(),
                split_type: split_item.split_type.clone(),
                account: split_item.account.clone(),
                description: split_item.description.clone(),
            }
        })
        .collect();
    (split_request.store.clone(), Some(splits))
}

impl TryFrom<PaymentsCancelResponseRouterData<AdyenCancelResponse>> for PaymentsCancelRouterData {
    type Error = Error;
    fn try_from(
        item: PaymentsCancelResponseRouterData<AdyenCancelResponse>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            status: storage_enums::AttemptStatus::Pending,
            response: Ok(PaymentsResponseData::TransactionResponse {
                resource_id: ResponseId::ConnectorTransactionId(
                    item.response.payment_psp_reference,
                ),
                redirection_data: Box::new(None),
                mandate_reference: Box::new(None),
                connector_metadata: None,
                network_txn_id: None,
                connector_response_reference_id: Some(item.response.reference),
                incremental_authorization_allowed: None,
                charges: None,
            }),
            ..item.data
        })
    }
}

impl TryFrom<PaymentsPreprocessingResponseRouterData<AdyenBalanceResponse>>
    for PaymentsPreProcessingRouterData
{
    type Error = Error;
    fn try_from(
        item: PaymentsPreprocessingResponseRouterData<AdyenBalanceResponse>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(PaymentsResponseData::TransactionResponse {
                resource_id: ResponseId::ConnectorTransactionId(item.response.psp_reference),
                redirection_data: Box::new(None),
                mandate_reference: Box::new(None),
                connector_metadata: None,
                network_txn_id: None,
                connector_response_reference_id: None,
                incremental_authorization_allowed: None,
                charges: None,
            }),
            payment_method_balance: Some(PaymentMethodBalance {
                currency: item.response.balance.currency,
                amount: item.response.balance.value,
            }),
            ..item.data
        })
    }
}

impl
    TryFrom<
        ResponseRouterData<
            GiftCardBalanceCheck,
            AdyenBalanceResponse,
            GiftCardBalanceCheckRequestData,
            GiftCardBalanceCheckResponseData,
        >,
    >
    for RouterData<
        GiftCardBalanceCheck,
        GiftCardBalanceCheckRequestData,
        GiftCardBalanceCheckResponseData,
    >
{
    type Error = Error;
    fn try_from(
        item: ResponseRouterData<
            GiftCardBalanceCheck,
            AdyenBalanceResponse,
            GiftCardBalanceCheckRequestData,
            GiftCardBalanceCheckResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(GiftCardBalanceCheckResponseData {
                balance: item.response.balance.value,
                currency: item.response.balance.currency,
            }),
            ..item.data
        })
    }
}

fn build_connector_response(
    adyen_webhook_response: &AdyenWebhookResponse,
) -> Option<ConnectorResponseData> {
    let extended_authentication_applied = match adyen_webhook_response.status {
        AdyenWebhookStatus::AdjustedAuthorization => {
            Some(common_types::primitive_wrappers::ExtendedAuthorizationAppliedBool::from(true))
        }
        AdyenWebhookStatus::AdjustAuthorizationFailed => {
            Some(common_types::primitive_wrappers::ExtendedAuthorizationAppliedBool::from(false))
        }
        _ => None,
    };

    let extended_authorization_last_applied_at = extended_authentication_applied
        .filter(|val| *val.deref())
        .and(adyen_webhook_response.event_date);

    let extend_authorization_response = ExtendedAuthorizationResponseData {
        extended_authentication_applied,
        capture_before: None,
        extended_authorization_last_applied_at,
    };

    Some(ConnectorResponseData::new(
        None,
        None,
        Some(extend_authorization_response),
        None,
    ))
}

pub fn get_adyen_response(
    response: AdyenResponse,
    is_capture_manual: bool,
    status_code: u16,
    pmt: Option<storage_enums::PaymentMethodType>,
) -> CustomResult<AdyenPaymentsResponseData, errors::ConnectorError> {
    let status = get_adyen_payment_status(is_capture_manual, response.result_code, pmt);
    let error = if response.refusal_reason.is_some()
        || response.refusal_reason_code.is_some()
        || status == storage_enums::AttemptStatus::Failure
    {
        let (network_decline_code, network_error_message) = response
            .additional_data
            .as_ref()
            .map(|data| {
                match (
                    data.refusal_code_raw.clone(),
                    data.refusal_reason_raw
                        .clone()
                        .or(data.merchant_advice_code.clone()),
                ) {
                    (None, Some(reason_raw)) => match reason_raw.split_once(':') {
                        Some((code, msg)) => {
                            (Some(code.trim().to_string()), Some(msg.trim().to_string()))
                        }
                        None => (None, Some(reason_raw.trim().to_string())),
                    },
                    (code, reason) => (code, reason),
                }
            })
            .unwrap_or((None, None));

        Some(ErrorResponse {
            code: response
                .refusal_reason_code
                .unwrap_or_else(|| NO_ERROR_CODE.to_string()),
            message: response
                .refusal_reason
                .clone()
                .unwrap_or_else(|| NO_ERROR_MESSAGE.to_string()),
            reason: response.refusal_reason,
            status_code,
            attempt_status: None,
            connector_transaction_id: Some(response.psp_reference.clone()),
            network_advice_code: response
                .additional_data
                .as_ref()
                .and_then(|data| data.extract_network_advice_code()),
            network_decline_code,
            network_error_message,
            connector_metadata: None,
        })
    } else {
        None
    };
    let mandate_reference = response
        .additional_data
        .as_ref()
        .and_then(|data| data.recurring_detail_reference.to_owned())
        .map(|mandate_id| MandateReference {
            connector_mandate_id: Some(mandate_id.expose()),
            payment_method_id: None,
            mandate_metadata: None,
            connector_mandate_request_reference_id: None,
        });
    let network_txn_id = response.additional_data.and_then(|additional_data| {
        additional_data
            .network_tx_reference
            .map(|network_tx_id| network_tx_id.expose())
    });

    let charges = match &response.splits {
        Some(split_items) => Some(construct_charge_response(response.store, split_items)),
        None => None,
    };

    let payments_response_data = PaymentsResponseData::TransactionResponse {
        resource_id: ResponseId::ConnectorTransactionId(response.psp_reference),
        redirection_data: Box::new(None),
        mandate_reference: Box::new(mandate_reference),
        connector_metadata: None,
        network_txn_id,
        connector_response_reference_id: Some(response.merchant_reference),
        incremental_authorization_allowed: None,
        charges,
    };

    let txn_amount = response.amount.map(|amount| amount.value);

    Ok(AdyenPaymentsResponseData {
        status,
        error,
        payments_response_data,
        txn_amount,
        connector_response: None,
    })
}

pub struct AdyenPaymentsResponseData {
    pub status: storage_enums::AttemptStatus,
    pub error: Option<ErrorResponse>,
    pub payments_response_data: PaymentsResponseData,
    pub txn_amount: Option<MinorUnit>,
    pub connector_response: Option<ConnectorResponseData>,
}

pub fn get_webhook_response(
    response: AdyenWebhookResponse,
    is_capture_manual: bool,
    is_multiple_capture_psync_flow: bool,
    status_code: u16,
) -> CustomResult<AdyenPaymentsResponseData, errors::ConnectorError> {
    let status = storage_enums::AttemptStatus::foreign_try_from((
        is_capture_manual,
        response.status.clone(),
    ))?;
    let error = if response.refusal_reason.is_some()
        || response.refusal_reason_code.is_some()
        || status == storage_enums::AttemptStatus::Failure
    {
        let (network_decline_code, network_error_message) = match (
            response.refusal_code_raw.clone(),
            response.refusal_reason_raw.clone(),
        ) {
            (None, Some(reason_raw)) => match reason_raw.split_once(':') {
                Some((code, msg)) => (Some(code.trim().to_string()), Some(msg.trim().to_string())),
                None => (None, Some(reason_raw.trim().to_string())),
            },
            (code, reason) => (code, reason),
        };

        Some(ErrorResponse {
            code: response
                .refusal_reason_code
                .clone()
                .unwrap_or_else(|| NO_ERROR_CODE.to_string()),
            message: response
                .refusal_reason
                .clone()
                .unwrap_or_else(|| NO_ERROR_MESSAGE.to_string()),
            reason: response.refusal_reason.clone(),
            status_code,
            attempt_status: None,
            connector_transaction_id: Some(response.transaction_id.clone()),
            network_advice_code: None,
            network_decline_code,
            network_error_message,
            connector_metadata: None,
        })
    } else {
        None
    };

    let txn_amount = response.amount.as_ref().map(|amount| amount.value);
    let connector_response = build_connector_response(&response);

    if is_multiple_capture_psync_flow {
        let capture_sync_response_list =
            utils::construct_captures_response_hashmap(vec![response.clone()])?;
        Ok(AdyenPaymentsResponseData {
            status,
            error,
            payments_response_data: PaymentsResponseData::MultipleCaptureResponse {
                capture_sync_response_list,
            },
            txn_amount,
            connector_response,
        })
    } else {
        let mandate_reference = response
            .recurring_detail_reference
            .as_ref()
            .map(|mandate_id| MandateReference {
                connector_mandate_id: Some(mandate_id.clone().expose()),
                payment_method_id: response.recurring_shopper_reference.clone(),
                mandate_metadata: None,
                connector_mandate_request_reference_id: None,
            });
        let payments_response_data = PaymentsResponseData::TransactionResponse {
            resource_id: ResponseId::ConnectorTransactionId(
                response
                    .payment_reference
                    .unwrap_or(response.transaction_id),
            ),
            redirection_data: Box::new(None),
            mandate_reference: Box::new(mandate_reference),
            connector_metadata: None,
            network_txn_id: None,
            connector_response_reference_id: Some(response.merchant_reference_id),
            incremental_authorization_allowed: None,
            charges: None,
        };

        Ok(AdyenPaymentsResponseData {
            status,
            error,
            payments_response_data,
            txn_amount,
            connector_response,
        })
    }
}

pub fn get_redirection_response(
    response: RedirectionResponse,
    is_manual_capture: bool,
    status_code: u16,
    pmt: Option<storage_enums::PaymentMethodType>,
) -> CustomResult<AdyenPaymentsResponseData, errors::ConnectorError> {
    let status = get_adyen_payment_status(is_manual_capture, response.result_code.clone(), pmt);
    let error = if response.refusal_reason.is_some()
        || response.refusal_reason_code.is_some()
        || status == storage_enums::AttemptStatus::Failure
    {
        let (network_decline_code, network_error_message) = response
            .additional_data
            .as_ref()
            .map(|data| {
                match (
                    data.refusal_code_raw.clone(),
                    data.refusal_reason_raw.clone(),
                ) {
                    (None, Some(reason_raw)) => match reason_raw.split_once(':') {
                        Some((code, msg)) => {
                            (Some(code.trim().to_string()), Some(msg.trim().to_string()))
                        }
                        None => (None, Some(reason_raw.trim().to_string())),
                    },
                    (code, reason) => (code, reason),
                }
            })
            .unwrap_or((None, None));

        Some(ErrorResponse {
            code: response
                .refusal_reason_code
                .clone()
                .unwrap_or_else(|| NO_ERROR_CODE.to_string()),
            message: response
                .refusal_reason
                .clone()
                .unwrap_or_else(|| NO_ERROR_MESSAGE.to_string()),
            reason: response.refusal_reason.to_owned(),
            status_code,
            attempt_status: None,
            connector_transaction_id: response.psp_reference.clone(),
            network_advice_code: None,
            network_decline_code,
            network_error_message,
            connector_metadata: None,
        })
    } else {
        None
    };

    let redirection_data = response.action.url.clone().map(|url| {
        let form_fields = response.action.data.clone().unwrap_or_else(|| {
            std::collections::HashMap::from_iter(
                url.query_pairs()
                    .map(|(key, value)| (key.to_string(), value.to_string())),
            )
        });
        RedirectForm::Form {
            endpoint: url.to_string(),
            method: response.action.method.unwrap_or(Method::Get),
            form_fields,
        }
    });

    let connector_metadata = get_wait_screen_metadata(&response)?;

    let charges = match &response.splits {
        Some(split_items) => Some(construct_charge_response(response.store, split_items)),
        None => None,
    };

    let payments_response_data = PaymentsResponseData::TransactionResponse {
        resource_id: match response.psp_reference.as_ref() {
            Some(psp) => ResponseId::ConnectorTransactionId(psp.to_string()),
            None => ResponseId::NoResponseId,
        },
        redirection_data: Box::new(redirection_data),
        mandate_reference: Box::new(None),
        connector_metadata,
        network_txn_id: None,
        connector_response_reference_id: response
            .merchant_reference
            .clone()
            .or(response.psp_reference),
        incremental_authorization_allowed: None,
        charges,
    };

    let txn_amount = response.amount.map(|amount| amount.value);

    Ok(AdyenPaymentsResponseData {
        status,
        error,
        payments_response_data,
        txn_amount,
        connector_response: None,
    })
}

pub fn get_present_to_shopper_response(
    response: PresentToShopperResponse,
    is_manual_capture: bool,
    status_code: u16,
    pmt: Option<storage_enums::PaymentMethodType>,
) -> CustomResult<AdyenPaymentsResponseData, errors::ConnectorError> {
    let status = get_adyen_payment_status(is_manual_capture, response.result_code.clone(), pmt);
    let error = if response.refusal_reason.is_some()
        || response.refusal_reason_code.is_some()
        || status == storage_enums::AttemptStatus::Failure
    {
        Some(ErrorResponse {
            code: response
                .refusal_reason_code
                .clone()
                .unwrap_or_else(|| NO_ERROR_CODE.to_string()),
            message: response
                .refusal_reason
                .clone()
                .unwrap_or_else(|| NO_ERROR_MESSAGE.to_string()),
            reason: response.refusal_reason.to_owned(),
            status_code,
            attempt_status: None,
            connector_transaction_id: response.psp_reference.clone(),
            network_advice_code: None,
            network_decline_code: None,
            network_error_message: None,
            connector_metadata: None,
        })
    } else {
        None
    };

    let charges = match &response.splits {
        Some(split_items) => Some(construct_charge_response(
            response.store.clone(),
            split_items,
        )),
        None => None,
    };

    let connector_metadata = get_present_to_shopper_metadata(&response)?;
    // We don't get connector transaction id for redirections in Adyen.
    let payments_response_data = PaymentsResponseData::TransactionResponse {
        resource_id: match response.psp_reference.as_ref() {
            Some(psp) => ResponseId::ConnectorTransactionId(psp.to_string()),
            None => ResponseId::NoResponseId,
        },
        redirection_data: Box::new(None),
        mandate_reference: Box::new(None),
        connector_metadata,
        network_txn_id: None,
        connector_response_reference_id: response
            .merchant_reference
            .clone()
            .or(response.psp_reference),
        incremental_authorization_allowed: None,
        charges,
    };
    let txn_amount = response.amount.map(|amount| amount.value);

    Ok(AdyenPaymentsResponseData {
        status,
        error,
        payments_response_data,
        txn_amount,
        connector_response: None,
    })
}

pub fn get_qr_code_response(
    response: QrCodeResponseResponse,
    is_manual_capture: bool,
    status_code: u16,
    pmt: Option<storage_enums::PaymentMethodType>,
) -> CustomResult<AdyenPaymentsResponseData, errors::ConnectorError> {
    let status = get_adyen_payment_status(is_manual_capture, response.result_code.clone(), pmt);
    let error = if response.refusal_reason.is_some()
        || response.refusal_reason_code.is_some()
        || status == storage_enums::AttemptStatus::Failure
    {
        Some(ErrorResponse {
            code: response
                .refusal_reason_code
                .clone()
                .unwrap_or_else(|| NO_ERROR_CODE.to_string()),
            message: response
                .refusal_reason
                .clone()
                .unwrap_or_else(|| NO_ERROR_MESSAGE.to_string()),
            reason: response.refusal_reason.to_owned(),
            status_code,
            attempt_status: None,
            connector_transaction_id: response.psp_reference.clone(),
            network_advice_code: None,
            network_decline_code: None,
            network_error_message: None,
            connector_metadata: None,
        })
    } else {
        None
    };

    let charges = match &response.splits {
        Some(split_items) => Some(construct_charge_response(
            response.store.clone(),
            split_items,
        )),
        None => None,
    };

    let connector_metadata = get_qr_metadata(&response)?;
    let payments_response_data = PaymentsResponseData::TransactionResponse {
        resource_id: match response.psp_reference.as_ref() {
            Some(psp) => ResponseId::ConnectorTransactionId(psp.to_string()),
            None => ResponseId::NoResponseId,
        },
        redirection_data: Box::new(None),
        mandate_reference: Box::new(None),
        connector_metadata,
        network_txn_id: None,
        connector_response_reference_id: response
            .merchant_reference
            .clone()
            .or(response.psp_reference),
        incremental_authorization_allowed: None,
        charges,
    };

    let txn_amount = response.amount.map(|amount| amount.value);

    Ok(AdyenPaymentsResponseData {
        status,
        error,
        payments_response_data,
        txn_amount,
        connector_response: None,
    })
}

pub fn get_redirection_error_response(
    response: RedirectionErrorResponse,
    is_manual_capture: bool,
    status_code: u16,
    pmt: Option<storage_enums::PaymentMethodType>,
) -> CustomResult<AdyenPaymentsResponseData, errors::ConnectorError> {
    let status = get_adyen_payment_status(is_manual_capture, response.result_code, pmt);
    let error = {
        let (network_decline_code, network_error_message) = response
            .additional_data
            .as_ref()
            .map(|data| {
                match (
                    data.refusal_code_raw.clone(),
                    data.refusal_reason_raw.clone(),
                ) {
                    (None, Some(reason_raw)) => match reason_raw.split_once(':') {
                        Some((code, msg)) => {
                            (Some(code.trim().to_string()), Some(msg.trim().to_string()))
                        }
                        None => (None, Some(reason_raw.trim().to_string())),
                    },
                    (code, reason) => (code, reason),
                }
            })
            .unwrap_or((None, None));

        Some(ErrorResponse {
            code: status.to_string(),
            message: response
                .refusal_reason
                .clone()
                .unwrap_or_else(|| NO_ERROR_MESSAGE.to_string()),
            reason: response.refusal_reason,
            status_code,
            attempt_status: None,
            connector_transaction_id: response.psp_reference.clone(),
            network_advice_code: response
                .additional_data
                .as_ref()
                .and_then(|data| data.extract_network_advice_code()),
            network_decline_code,
            network_error_message,

            connector_metadata: None,
        })
    };
    // We don't get connector transaction id for redirections in Adyen.
    let payments_response_data = PaymentsResponseData::TransactionResponse {
        resource_id: ResponseId::NoResponseId,
        redirection_data: Box::new(None),
        mandate_reference: Box::new(None),
        connector_metadata: None,
        network_txn_id: None,
        connector_response_reference_id: response
            .merchant_reference
            .clone()
            .or(response.psp_reference),
        incremental_authorization_allowed: None,
        charges: None,
    };

    Ok(AdyenPaymentsResponseData {
        status,
        error,
        payments_response_data,
        txn_amount: None,
        connector_response: None,
    })
}

pub fn get_qr_metadata(
    response: &QrCodeResponseResponse,
) -> CustomResult<Option<serde_json::Value>, errors::ConnectorError> {
    let image_data = utils::QrImage::new_from_data(response.action.qr_code_data.clone())
        .change_context(errors::ConnectorError::ResponseHandlingFailed)?;

    let image_data_url = Url::parse(image_data.data.clone().as_str()).ok();
    let qr_code_url = response.action.qr_code_url.clone();
    let display_to_timestamp = response
        .additional_data
        .clone()
        .and_then(|additional_data| additional_data.pix_expiration_date)
        .map(|time| utils::get_timestamp_in_milliseconds(&time));

    if let (Some(image_data_url), Some(qr_code_url)) = (image_data_url.clone(), qr_code_url.clone())
    {
        let qr_code_info = QrCodeInformation::QrCodeUrl {
            image_data_url,
            qr_code_url,
            display_to_timestamp,
        };
        Some(qr_code_info.encode_to_value())
            .transpose()
            .change_context(errors::ConnectorError::ResponseHandlingFailed)
    } else if let (None, Some(qr_code_url)) = (image_data_url.clone(), qr_code_url.clone()) {
        let qr_code_info = QrCodeInformation::QrCodeImageUrl {
            qr_code_url,
            display_to_timestamp,
        };
        Some(qr_code_info.encode_to_value())
            .transpose()
            .change_context(errors::ConnectorError::ResponseHandlingFailed)
    } else if let (Some(image_data_url), None) = (image_data_url, qr_code_url) {
        let qr_code_info = QrCodeInformation::QrDataUrl {
            image_data_url,
            display_to_timestamp,
        };

        Some(qr_code_info.encode_to_value())
            .transpose()
            .change_context(errors::ConnectorError::ResponseHandlingFailed)
    } else {
        Ok(None)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WaitScreenData {
    display_from_timestamp: i128,
    display_to_timestamp: Option<i128>,
    poll_config: Option<PollConfig>,
}

pub fn get_wait_screen_metadata(
    next_action: &RedirectionResponse,
) -> CustomResult<Option<serde_json::Value>, errors::ConnectorError> {
    match next_action.action.payment_method_type {
        PaymentType::Blik => {
            let current_time = OffsetDateTime::now_utc().unix_timestamp_nanos();
            Ok(Some(serde_json::json!(WaitScreenData {
                display_from_timestamp: current_time,
                display_to_timestamp: Some(current_time + Duration::minutes(1).whole_nanoseconds()),
                poll_config: None
            })))
        }
        PaymentType::Mbway => {
            let current_time = OffsetDateTime::now_utc().unix_timestamp_nanos();
            Ok(Some(serde_json::json!(WaitScreenData {
                display_from_timestamp: current_time,
                display_to_timestamp: None,
                poll_config: None
            })))
        }
        PaymentType::Affirm
        | PaymentType::Oxxo
        | PaymentType::Afterpaytouch
        | PaymentType::Alipay
        | PaymentType::AlipayHk
        | PaymentType::Alfamart
        | PaymentType::Alma
        | PaymentType::Applepay
        | PaymentType::Bizum
        | PaymentType::Atome
        | PaymentType::BoletoBancario
        | PaymentType::ClearPay
        | PaymentType::Dana
        | PaymentType::Eps
        | PaymentType::Gcash
        | PaymentType::Googlepay
        | PaymentType::GoPay
        | PaymentType::Ideal
        | PaymentType::Indomaret
        | PaymentType::Klarna
        | PaymentType::Kakaopay
        | PaymentType::MobilePay
        | PaymentType::Momo
        | PaymentType::MomoAtm
        | PaymentType::OnlineBankingCzechRepublic
        | PaymentType::OnlineBankingFinland
        | PaymentType::OnlineBankingPoland
        | PaymentType::OnlineBankingSlovakia
        | PaymentType::OnlineBankingFpx
        | PaymentType::OnlineBankingThailand
        | PaymentType::OpenBankingUK
        | PaymentType::PayBright
        | PaymentType::Paypal
        | PaymentType::Scheme
        | PaymentType::NetworkToken
        | PaymentType::Trustly
        | PaymentType::TouchNGo
        | PaymentType::Walley
        | PaymentType::WeChatPayWeb
        | PaymentType::AchDirectDebit
        | PaymentType::SepaDirectDebit
        | PaymentType::BacsDirectDebit
        | PaymentType::Samsungpay
        | PaymentType::Twint
        | PaymentType::Vipps
        | PaymentType::Swish
        | PaymentType::Knet
        | PaymentType::Benefit
        | PaymentType::PermataBankTransfer
        | PaymentType::BcaBankTransfer
        | PaymentType::BniVa
        | PaymentType::BriVa
        | PaymentType::CimbVa
        | PaymentType::DanamonVa
        | PaymentType::Giftcard
        | PaymentType::MandiriVa
        | PaymentType::PaySafeCard
        | PaymentType::SevenEleven
        | PaymentType::JapaneseConvenienceStores
        | PaymentType::Pix => Ok(None),
    }
}

pub fn get_present_to_shopper_metadata(
    response: &PresentToShopperResponse,
) -> CustomResult<Option<serde_json::Value>, errors::ConnectorError> {
    let reference = response.action.reference.clone();
    let expires_at = response
        .action
        .expires_at
        .map(|time| utils::get_timestamp_in_milliseconds(&time));

    match response.action.payment_method_type {
        PaymentType::Alfamart
        | PaymentType::Indomaret
        | PaymentType::BoletoBancario
        | PaymentType::Oxxo
        | PaymentType::JapaneseConvenienceStores => {
            let voucher_data = VoucherNextStepData {
                expires_at,
                reference,
                download_url: response.action.download_url.clone(),
                instructions_url: response.action.instructions_url.clone(),
                entry_date: None,
                digitable_line: None,
            };

            Some(voucher_data.encode_to_value())
                .transpose()
                .change_context(errors::ConnectorError::ResponseHandlingFailed)
        }
        PaymentType::PermataBankTransfer
        | PaymentType::BcaBankTransfer
        | PaymentType::BniVa
        | PaymentType::BriVa
        | PaymentType::CimbVa
        | PaymentType::DanamonVa
        | PaymentType::Giftcard
        | PaymentType::MandiriVa => {
            let voucher_data = payments::BankTransferInstructions::DokuBankTransferInstructions(
                Box::new(payments::DokuBankTransferInstructions {
                    reference: Secret::new(response.action.reference.clone()),
                    instructions_url: response.action.instructions_url.clone(),
                    expires_at,
                }),
            );

            Some(voucher_data.encode_to_value())
                .transpose()
                .change_context(errors::ConnectorError::ResponseHandlingFailed)
        }
        PaymentType::Affirm
        | PaymentType::Afterpaytouch
        | PaymentType::Alipay
        | PaymentType::AlipayHk
        | PaymentType::Alma
        | PaymentType::Applepay
        | PaymentType::Bizum
        | PaymentType::Atome
        | PaymentType::Blik
        | PaymentType::ClearPay
        | PaymentType::Dana
        | PaymentType::Eps
        | PaymentType::Gcash
        | PaymentType::Googlepay
        | PaymentType::GoPay
        | PaymentType::Ideal
        | PaymentType::Klarna
        | PaymentType::Kakaopay
        | PaymentType::Mbway
        | PaymentType::Knet
        | PaymentType::Benefit
        | PaymentType::MobilePay
        | PaymentType::Momo
        | PaymentType::MomoAtm
        | PaymentType::OnlineBankingCzechRepublic
        | PaymentType::OnlineBankingFinland
        | PaymentType::OnlineBankingPoland
        | PaymentType::OnlineBankingSlovakia
        | PaymentType::OnlineBankingFpx
        | PaymentType::OnlineBankingThailand
        | PaymentType::OpenBankingUK
        | PaymentType::PayBright
        | PaymentType::Paypal
        | PaymentType::Scheme
        | PaymentType::NetworkToken
        | PaymentType::Trustly
        | PaymentType::TouchNGo
        | PaymentType::Walley
        | PaymentType::WeChatPayWeb
        | PaymentType::AchDirectDebit
        | PaymentType::SepaDirectDebit
        | PaymentType::BacsDirectDebit
        | PaymentType::Samsungpay
        | PaymentType::Twint
        | PaymentType::Vipps
        | PaymentType::Swish
        | PaymentType::PaySafeCard
        | PaymentType::SevenEleven
        | PaymentType::Pix => Ok(None),
    }
}

impl<F, Req>
    ForeignTryFrom<(
        ResponseRouterData<F, AdyenPaymentResponse, Req, PaymentsResponseData>,
        Option<storage_enums::CaptureMethod>,
        bool,
        Option<storage_enums::PaymentMethodType>,
    )> for RouterData<F, Req, PaymentsResponseData>
{
    type Error = Error;
    fn foreign_try_from(
        (item, capture_method, is_multiple_capture_psync_flow, pmt): (
            ResponseRouterData<F, AdyenPaymentResponse, Req, PaymentsResponseData>,
            Option<storage_enums::CaptureMethod>,
            bool,
            Option<storage_enums::PaymentMethodType>,
        ),
    ) -> Result<Self, Self::Error> {
        let is_manual_capture = is_manual_capture(capture_method);
        let adyen_payments_response_data = match item.response {
            AdyenPaymentResponse::Response(response) => {
                get_adyen_response(*response, is_manual_capture, item.http_code, pmt)?
            }
            AdyenPaymentResponse::PresentToShopper(response) => {
                get_present_to_shopper_response(*response, is_manual_capture, item.http_code, pmt)?
            }
            AdyenPaymentResponse::QrCodeResponse(response) => {
                get_qr_code_response(*response, is_manual_capture, item.http_code, pmt)?
            }
            AdyenPaymentResponse::RedirectionResponse(response) => {
                get_redirection_response(*response, is_manual_capture, item.http_code, pmt)?
            }
            AdyenPaymentResponse::RedirectionErrorResponse(response) => {
                get_redirection_error_response(*response, is_manual_capture, item.http_code, pmt)?
            }
            AdyenPaymentResponse::WebhookResponse(response) => get_webhook_response(
                *response,
                is_manual_capture,
                is_multiple_capture_psync_flow,
                item.http_code,
            )?,
        };

        let minor_amount_captured = match adyen_payments_response_data.status {
            enums::AttemptStatus::Charged
            | enums::AttemptStatus::PartialCharged
            | enums::AttemptStatus::PartialChargedAndChargeable => {
                adyen_payments_response_data.txn_amount
            }
            _ => None,
        };

        Ok(Self {
            status: adyen_payments_response_data.status,
            amount_captured: minor_amount_captured.map(|amount| amount.get_amount_as_i64()),
            response: adyen_payments_response_data.error.map_or_else(
                || Ok(adyen_payments_response_data.payments_response_data),
                Err,
            ),
            connector_response: adyen_payments_response_data.connector_response,
            minor_amount_captured,
            ..item.data
        })
    }
}

#[derive(Default, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdyenCaptureRequest {
    merchant_account: Secret<String>,
    amount: Amount,
    reference: String,
}

#[derive(Default, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdyenExtendAuthorizationRequest {
    merchant_account: Secret<String>,
    amount: Amount,
}

impl TryFrom<&AdyenRouterData<&PaymentsExtendAuthorizationRouterData>>
    for AdyenExtendAuthorizationRequest
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &AdyenRouterData<&PaymentsExtendAuthorizationRouterData>,
    ) -> Result<Self, Self::Error> {
        let auth_type = AdyenAuthType::try_from(&item.router_data.connector_auth_type)?;
        let amount = Amount {
            currency: item.router_data.request.currency,
            value: item.amount,
        };
        Ok(Self {
            merchant_account: auth_type.merchant_account,
            amount,
        })
    }
}

#[derive(Default, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdyenExtendAuthorizationResponse {
    merchant_account: Secret<String>,
    payment_psp_reference: Option<String>,
    psp_reference: Option<String>,
    amount: Amount,
}

impl TryFrom<PaymentsExtendAuthorizationResponseRouterData<AdyenExtendAuthorizationResponse>>
    for PaymentsExtendAuthorizationRouterData
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: PaymentsExtendAuthorizationResponseRouterData<AdyenExtendAuthorizationResponse>,
    ) -> Result<Self, Self::Error> {
        // Asynchronous authorization adjustment
        Ok(Self {
            status: storage_enums::AttemptStatus::Pending,
            ..item.data
        })
    }
}

impl TryFrom<&AdyenRouterData<&PaymentsCaptureRouterData>> for AdyenCaptureRequest {
    type Error = Error;
    fn try_from(item: &AdyenRouterData<&PaymentsCaptureRouterData>) -> Result<Self, Self::Error> {
        let auth_type = AdyenAuthType::try_from(&item.router_data.connector_auth_type)?;
        let reference = match item.router_data.request.multiple_capture_data.clone() {
            // if multiple capture request, send capture_id as our reference for the capture
            Some(multiple_capture_request_data) => multiple_capture_request_data.capture_reference,
            // if single capture request, send connector_request_reference_id(attempt_id)
            None => item.router_data.connector_request_reference_id.clone(),
        };
        Ok(Self {
            merchant_account: auth_type.merchant_account,
            reference,
            amount: Amount {
                currency: item.router_data.request.currency,
                value: item.router_data.request.minor_amount_to_capture,
            },
        })
    }
}

#[derive(Default, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdyenCaptureResponse {
    merchant_account: Secret<String>,
    payment_psp_reference: String,
    psp_reference: String,
    reference: String,
    status: String,
    amount: Amount,
    merchant_reference: Option<String>,
    store: Option<String>,
    splits: Option<Vec<AdyenSplitData>>,
}

impl TryFrom<PaymentsCaptureResponseRouterData<AdyenCaptureResponse>>
    for PaymentsCaptureRouterData
{
    type Error = Error;
    fn try_from(
        item: PaymentsCaptureResponseRouterData<AdyenCaptureResponse>,
    ) -> Result<Self, Self::Error> {
        let connector_transaction_id = if item.data.request.multiple_capture_data.is_some() {
            item.response.psp_reference.clone()
        } else {
            item.response.payment_psp_reference
        };
        let charges = match &item.response.splits {
            Some(split_items) => Some(construct_charge_response(item.response.store, split_items)),
            None => None,
        };

        Ok(Self {
            // From the docs, the only value returned is "received", outcome of refund is available
            // through refund notification webhook
            // For more info: https://docs.adyen.com/online-payments/capture
            status: storage_enums::AttemptStatus::Pending,
            response: Ok(PaymentsResponseData::TransactionResponse {
                resource_id: ResponseId::ConnectorTransactionId(connector_transaction_id),
                redirection_data: Box::new(None),
                mandate_reference: Box::new(None),
                connector_metadata: None,
                network_txn_id: None,
                connector_response_reference_id: Some(item.response.reference),
                incremental_authorization_allowed: None,
                charges,
            }),
            amount_captured: None, // updated by Webhooks
            ..item.data
        })
    }
}

fn construct_charge_response(
    store: Option<String>,
    split_item: &[AdyenSplitData],
) -> common_types::payments::ConnectorChargeResponseData {
    let splits: Vec<common_types::domain::AdyenSplitItem> = split_item
        .iter()
        .map(|split_item| common_types::domain::AdyenSplitItem {
            amount: split_item.amount.as_ref().map(|amount| amount.value),
            reference: split_item.reference.clone(),
            split_type: split_item.split_type.clone(),
            account: split_item.account.clone(),
            description: split_item.description.clone(),
        })
        .collect();

    common_types::payments::ConnectorChargeResponseData::AdyenSplitPayment(
        common_types::domain::AdyenSplitData {
            store,
            split_items: splits,
        },
    )
}

// Refund Request Transform
impl<F> TryFrom<&AdyenRouterData<&RefundsRouterData<F>>> for AdyenRefundRequest {
    type Error = Error;
    fn try_from(item: &AdyenRouterData<&RefundsRouterData<F>>) -> Result<Self, Self::Error> {
        let auth_type = AdyenAuthType::try_from(&item.router_data.connector_auth_type)?;
        let (store, splits) = match item
        .router_data
        .request
        .split_refunds
        .as_ref()
        {
                Some(hyperswitch_domain_models::router_request_types::SplitRefundsRequest::AdyenSplitRefund(adyen_split_data)) =>  get_adyen_split_request(adyen_split_data, item.router_data.request.currency),
                _ => (
                item.router_data
                    .request
                    .refund_connector_metadata
                    .clone()
                    .and_then(|metadata| get_store_id(metadata.expose())),
                None,
            ),
        };

        Ok(Self {
            merchant_account: auth_type.merchant_account,
            amount: Amount {
                currency: item.router_data.request.currency,
                value: item.amount,
            },
            merchant_refund_reason: item
                .router_data
                .request
                .reason
                .as_ref()
                .map(|reason| AdyenRefundRequestReason::from_str(reason))
                .transpose()?,
            reference: item.router_data.request.refund_id.clone(),
            store,
            splits,
        })
    }
}

// Refund Response Transform
impl<F> TryFrom<RefundsResponseRouterData<F, AdyenRefundResponse>> for RefundsRouterData<F> {
    type Error = Error;
    fn try_from(
        item: RefundsResponseRouterData<F, AdyenRefundResponse>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(RefundsResponseData {
                connector_refund_id: item.response.psp_reference,
                // From the docs, the only value returned is "received", outcome of refund is available
                // through refund notification webhook
                // For more info: https://docs.adyen.com/online-payments/refund
                refund_status: storage_enums::RefundStatus::Pending,
            }),
            ..item.data
        })
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdyenErrorResponse {
    pub status: i32,
    pub error_code: String,
    pub message: String,
    pub error_type: String,
    pub psp_reference: Option<String>,
}

// #[cfg(test)]
// mod test_adyen_transformers {
//     use super::*;

//     #[test]
//     fn verify_transform_from_router_to_adyen_req() {
//         let router_req = PaymentsRequest {
//             amount: 0.0,
//             currency: "None".to_string(),
//             ..Default::default()
//         };
//         println!("{:#?}", &router_req);
//         let adyen_req = AdyenPaymentRequest::from(router_req);
//         println!("{:#?}", &adyen_req);
//         let adyen_req_json: String = serde_json::to_string(&adyen_req).unwrap();
//         println!("{}", adyen_req_json);
//         assert_eq!(true, true)
//     }
// }

#[derive(Debug, Deserialize)]
pub enum DisputeStatus {
    Undefended,
    Pending,
    Lost,
    Accepted,
    Won,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdyenAdditionalDataWH {
    pub hmac_signature: Secret<String>,
    pub dispute_status: Option<DisputeStatus>,
    pub chargeback_reason_code: Option<String>,
    #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
    pub defense_period_ends_at: Option<PrimitiveDateTime>,
    /// Enable recurring details in dashboard to receive this ID, https://docs.adyen.com/online-payments/tokenization/create-and-use-tokens#test-and-go-live
    #[serde(rename = "recurring.recurringDetailReference")]
    pub recurring_detail_reference: Option<Secret<String>>,
    #[serde(rename = "recurring.shopperReference")]
    pub recurring_shopper_reference: Option<String>,
    pub network_tx_reference: Option<Secret<String>>,
    /// [only for cards] Enable raw acquirer from Adyen dashboard to receive this (https://docs.adyen.com/development-resources/raw-acquirer-responses/#search-modal)
    pub refusal_reason_raw: Option<String>,
    /// [only for cards] This is only available for Visa and Mastercard
    pub refusal_code_raw: Option<String>,
    #[serde(rename = "shopperEmail")]
    pub shopper_email: Option<String>,
    #[serde(rename = "shopperReference")]
    pub shopper_reference: Option<String>,
    pub expiry_date: Option<Secret<String>>,
    pub card_summary: Option<Secret<String>>,
    pub card_issuing_country: Option<String>,
    pub card_issuing_bank: Option<String>,
    pub payment_method_variant: Option<Secret<String>>,
}

#[derive(Debug, Deserialize)]
pub struct AdyenAmountWH {
    pub value: MinorUnit,
    pub currency: storage_enums::Currency,
}

#[derive(Clone, Debug, Deserialize, Serialize, strum::Display, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[strum(serialize_all = "SCREAMING_SNAKE_CASE")]
pub enum WebhookEventCode {
    Authorisation,
    AuthorisationAdjustment,
    Refund,
    CancelOrRefund,
    Cancellation,
    Capture,
    CaptureFailed,
    RefundFailed,
    RefundReversed,
    NotificationOfChargeback,
    Chargeback,
    ChargebackReversed,
    SecondChargeback,
    PrearbitrationWon,
    PrearbitrationLost,
    OfferClosed,
    RecurringContract,
    #[cfg(feature = "payouts")]
    PayoutThirdparty,
    #[cfg(feature = "payouts")]
    PayoutDecline,
    #[cfg(feature = "payouts")]
    PayoutExpire,
    #[cfg(feature = "payouts")]
    PayoutReversed,
    #[serde(other)]
    Unknown,
}

pub fn is_transaction_event(event_code: &WebhookEventCode) -> bool {
    matches!(
        event_code,
        WebhookEventCode::Authorisation
            | WebhookEventCode::OfferClosed
            | WebhookEventCode::RecurringContract
    )
}

pub fn is_capture_or_cancel_or_adjust_event(event_code: &WebhookEventCode) -> bool {
    matches!(
        event_code,
        WebhookEventCode::AuthorisationAdjustment
            | WebhookEventCode::Capture
            | WebhookEventCode::CaptureFailed
            | WebhookEventCode::Cancellation
    )
}

pub fn is_refund_event(event_code: &WebhookEventCode) -> bool {
    matches!(
        event_code,
        WebhookEventCode::Refund
            | WebhookEventCode::CancelOrRefund
            | WebhookEventCode::RefundFailed
            | WebhookEventCode::RefundReversed
    )
}

pub fn is_chargeback_event(event_code: &WebhookEventCode) -> bool {
    matches!(
        event_code,
        WebhookEventCode::NotificationOfChargeback
            | WebhookEventCode::Chargeback
            | WebhookEventCode::ChargebackReversed
            | WebhookEventCode::SecondChargeback
            | WebhookEventCode::PrearbitrationWon
            | WebhookEventCode::PrearbitrationLost
    )
}

#[cfg(feature = "payouts")]
pub fn is_payout_event(event_code: &WebhookEventCode) -> bool {
    matches!(
        event_code,
        WebhookEventCode::PayoutThirdparty
            | WebhookEventCode::PayoutDecline
            | WebhookEventCode::PayoutExpire
            | WebhookEventCode::PayoutReversed
    )
}

fn is_success_scenario(is_success: String) -> bool {
    is_success.as_str() == "true"
}

pub(crate) fn get_adyen_webhook_event(
    code: WebhookEventCode,
    is_success: String,
    dispute_status: Option<DisputeStatus>,
) -> api_models::webhooks::IncomingWebhookEvent {
    match code {
        WebhookEventCode::Authorisation => {
            if is_success_scenario(is_success) {
                api_models::webhooks::IncomingWebhookEvent::PaymentIntentSuccess
            } else {
                api_models::webhooks::IncomingWebhookEvent::PaymentIntentFailure
            }
        }
        WebhookEventCode::Refund | WebhookEventCode::CancelOrRefund => {
            if is_success_scenario(is_success) {
                api_models::webhooks::IncomingWebhookEvent::RefundSuccess
            } else {
                api_models::webhooks::IncomingWebhookEvent::RefundFailure
            }
        }
        WebhookEventCode::Cancellation => {
            if is_success_scenario(is_success) {
                api_models::webhooks::IncomingWebhookEvent::PaymentIntentCancelled
            } else {
                api_models::webhooks::IncomingWebhookEvent::PaymentIntentCancelFailure
            }
        }
        WebhookEventCode::AuthorisationAdjustment => {
            if is_success_scenario(is_success) {
                api_models::webhooks::IncomingWebhookEvent::PaymentIntentExtendAuthorizationSuccess
            } else {
                api_models::webhooks::IncomingWebhookEvent::PaymentIntentExtendAuthorizationFailure
            }
        }
        WebhookEventCode::RefundFailed | WebhookEventCode::RefundReversed => {
            api_models::webhooks::IncomingWebhookEvent::RefundFailure
        }
        WebhookEventCode::NotificationOfChargeback => {
            api_models::webhooks::IncomingWebhookEvent::DisputeOpened
        }
        WebhookEventCode::Chargeback => match dispute_status {
            Some(DisputeStatus::Won) => api_models::webhooks::IncomingWebhookEvent::DisputeWon,
            Some(DisputeStatus::Lost) | None => {
                api_models::webhooks::IncomingWebhookEvent::DisputeLost
            }
            Some(_) => api_models::webhooks::IncomingWebhookEvent::DisputeOpened,
        },
        WebhookEventCode::ChargebackReversed => match dispute_status {
            Some(DisputeStatus::Pending) => {
                api_models::webhooks::IncomingWebhookEvent::DisputeChallenged
            }
            _ => api_models::webhooks::IncomingWebhookEvent::DisputeWon,
        },
        WebhookEventCode::SecondChargeback => {
            api_models::webhooks::IncomingWebhookEvent::DisputeLost
        }
        WebhookEventCode::PrearbitrationWon => match dispute_status {
            Some(DisputeStatus::Pending) => {
                api_models::webhooks::IncomingWebhookEvent::DisputeOpened
            }
            _ => api_models::webhooks::IncomingWebhookEvent::DisputeWon,
        },
        WebhookEventCode::PrearbitrationLost => {
            api_models::webhooks::IncomingWebhookEvent::DisputeLost
        }
        WebhookEventCode::Capture => {
            if is_success_scenario(is_success) {
                api_models::webhooks::IncomingWebhookEvent::PaymentIntentCaptureSuccess
            } else {
                api_models::webhooks::IncomingWebhookEvent::PaymentIntentCaptureFailure
            }
        }
        WebhookEventCode::CaptureFailed => {
            api_models::webhooks::IncomingWebhookEvent::PaymentIntentCaptureFailure
        }
        WebhookEventCode::OfferClosed => {
            api_models::webhooks::IncomingWebhookEvent::PaymentIntentExpired
        }
        WebhookEventCode::RecurringContract => {
            if is_success_scenario(is_success) {
                api_models::webhooks::IncomingWebhookEvent::PaymentIntentSuccess
            } else {
                api_models::webhooks::IncomingWebhookEvent::PaymentIntentFailure
            }
        }
        #[cfg(feature = "payouts")]
        WebhookEventCode::PayoutThirdparty => {
            api_models::webhooks::IncomingWebhookEvent::PayoutCreated
        }
        #[cfg(feature = "payouts")]
        WebhookEventCode::PayoutDecline => {
            api_models::webhooks::IncomingWebhookEvent::PayoutFailure
        }
        #[cfg(feature = "payouts")]
        WebhookEventCode::PayoutExpire => api_models::webhooks::IncomingWebhookEvent::PayoutExpired,
        #[cfg(feature = "payouts")]
        WebhookEventCode::PayoutReversed => {
            api_models::webhooks::IncomingWebhookEvent::PayoutReversed
        }
        WebhookEventCode::Unknown => api_models::webhooks::IncomingWebhookEvent::EventNotSupported,
    }
}

impl From<WebhookEventCode> for storage_enums::DisputeStage {
    fn from(code: WebhookEventCode) -> Self {
        match code {
            WebhookEventCode::NotificationOfChargeback => Self::PreDispute,
            WebhookEventCode::SecondChargeback => Self::PreArbitration,
            WebhookEventCode::PrearbitrationWon => Self::PreArbitration,
            WebhookEventCode::PrearbitrationLost => Self::PreArbitration,
            _ => Self::Dispute,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdyenNotificationRequestItemWH {
    pub additional_data: AdyenAdditionalDataWH,
    pub amount: AdyenAmountWH,
    pub original_reference: Option<String>,
    pub psp_reference: String,
    pub event_code: WebhookEventCode,
    pub merchant_account_code: String,
    pub merchant_reference: String,
    pub success: String,
    pub reason: Option<String>,
    #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
    pub event_date: Option<PrimitiveDateTime>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct AdyenItemObjectWH {
    pub notification_request_item: AdyenNotificationRequestItemWH,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdyenIncomingWebhook {
    pub notification_items: Vec<AdyenItemObjectWH>,
}

impl From<AdyenNotificationRequestItemWH> for AdyenWebhookResponse {
    fn from(notif: AdyenNotificationRequestItemWH) -> Self {
        let (refusal_reason, refusal_reason_code) = if !is_success_scenario(notif.success.clone()) {
            (
                notif.reason.or(Some(NO_ERROR_MESSAGE.to_string())),
                Some(NO_ERROR_CODE.to_string()),
            )
        } else {
            (None, None)
        };
        Self {
            transaction_id: notif.psp_reference,
            payment_reference: notif.original_reference,
            //Translating into custom status so that it can be clearly mapped to out attempt_status
            status: match notif.event_code {
                WebhookEventCode::Authorisation => {
                    if is_success_scenario(notif.success) {
                        AdyenWebhookStatus::Authorised
                    } else {
                        AdyenWebhookStatus::AuthorisationFailed
                    }
                }
                WebhookEventCode::RecurringContract => {
                    if is_success_scenario(notif.success) {
                        AdyenWebhookStatus::Authorised
                    } else {
                        AdyenWebhookStatus::AuthorisationFailed
                    }
                }
                WebhookEventCode::Cancellation => {
                    if is_success_scenario(notif.success) {
                        AdyenWebhookStatus::Cancelled
                    } else {
                        AdyenWebhookStatus::CancelFailed
                    }
                }
                WebhookEventCode::AuthorisationAdjustment => {
                    if is_success_scenario(notif.success) {
                        AdyenWebhookStatus::AdjustedAuthorization
                    } else {
                        AdyenWebhookStatus::AdjustAuthorizationFailed
                    }
                }
                WebhookEventCode::OfferClosed => AdyenWebhookStatus::Expired,
                WebhookEventCode::Capture => {
                    if is_success_scenario(notif.success) {
                        AdyenWebhookStatus::Captured
                    } else {
                        AdyenWebhookStatus::CaptureFailed
                    }
                }
                #[cfg(feature = "payouts")]
                WebhookEventCode::PayoutThirdparty => {
                    if is_success_scenario(notif.success) {
                        AdyenWebhookStatus::Authorised
                    } else {
                        AdyenWebhookStatus::AuthorisationFailed
                    }
                }
                #[cfg(feature = "payouts")]
                WebhookEventCode::PayoutDecline => AdyenWebhookStatus::Cancelled,
                #[cfg(feature = "payouts")]
                WebhookEventCode::PayoutExpire => AdyenWebhookStatus::AuthorisationFailed,
                #[cfg(feature = "payouts")]
                WebhookEventCode::PayoutReversed => AdyenWebhookStatus::Reversed,
                WebhookEventCode::CaptureFailed => AdyenWebhookStatus::CaptureFailed,
                WebhookEventCode::CancelOrRefund
                | WebhookEventCode::Refund
                | WebhookEventCode::RefundFailed
                | WebhookEventCode::RefundReversed
                | WebhookEventCode::NotificationOfChargeback
                | WebhookEventCode::Chargeback
                | WebhookEventCode::ChargebackReversed
                | WebhookEventCode::SecondChargeback
                | WebhookEventCode::PrearbitrationWon
                | WebhookEventCode::PrearbitrationLost
                | WebhookEventCode::Unknown => AdyenWebhookStatus::UnexpectedEvent,
            },
            amount: Some(Amount {
                value: notif.amount.value,
                currency: notif.amount.currency,
            }),
            merchant_reference_id: notif.merchant_reference,
            refusal_reason,
            refusal_reason_code,
            event_code: notif.event_code,
            event_date: notif.event_date,
            refusal_code_raw: notif.additional_data.refusal_code_raw,
            refusal_reason_raw: notif.additional_data.refusal_reason_raw,
            recurring_detail_reference: notif.additional_data.recurring_detail_reference,
            recurring_shopper_reference: notif.additional_data.recurring_shopper_reference,
        }
    }
}

//This will be triggered in Psync handler of webhook response
impl utils::MultipleCaptureSyncResponse for AdyenWebhookResponse {
    fn get_connector_capture_id(&self) -> String {
        self.transaction_id.clone()
    }

    fn get_capture_attempt_status(&self) -> storage_enums::AttemptStatus {
        match self.status {
            AdyenWebhookStatus::Captured => storage_enums::AttemptStatus::Charged,
            _ => storage_enums::AttemptStatus::CaptureFailed,
        }
    }

    fn is_capture_response(&self) -> bool {
        matches!(
            self.event_code,
            WebhookEventCode::Capture | WebhookEventCode::CaptureFailed
        )
    }

    fn get_connector_reference_id(&self) -> Option<String> {
        Some(self.merchant_reference_id.clone())
    }

    fn get_amount_captured(&self) -> Result<Option<MinorUnit>, error_stack::Report<ParsingError>> {
        Ok(self.amount.clone().map(|amount| amount.value))
    }
}

// Payouts
#[cfg(feature = "payouts")]
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdyenPayoutCreateRequest {
    amount: Amount,
    recurring: RecurringContract,
    merchant_account: Secret<String>,
    #[serde(flatten)]
    payment_data: PayoutPaymentMethodData,
    reference: String,
    shopper_reference: String,
    shopper_email: Option<Email>,
    shopper_name: ShopperName,
    date_of_birth: Option<Secret<String>>,
    entity_type: Option<storage_enums::PayoutEntityType>,
    nationality: Option<storage_enums::CountryAlpha2>,
    billing_address: Option<Address>,
}

#[cfg(feature = "payouts")]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum PayoutPaymentMethodData {
    PayoutBankData(PayoutBankData),
    PayoutWalletData(PayoutWalletData),
}

#[cfg(feature = "payouts")]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PayoutBankData {
    bank: PayoutBankDetails,
}

#[cfg(feature = "payouts")]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PayoutWalletData {
    selected_brand: PayoutBrand,
    additional_data: PayoutAdditionalData,
}

#[cfg(feature = "payouts")]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PayoutBrand {
    Paypal,
}

#[cfg(feature = "payouts")]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PayoutAdditionalData {
    token_data_type: PayoutTokenDataType,
    email_id: Email,
}

#[cfg(feature = "payouts")]
#[derive(Debug, Clone, Serialize, Deserialize)]
enum PayoutTokenDataType {
    PayPal,
}

#[cfg(feature = "payouts")]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PayoutBankDetails {
    iban: Secret<String>,
    owner_name: Secret<String>,
    bank_city: Option<String>,
    bank_name: Option<String>,
    bic: Option<Secret<String>>,
    country_code: Option<storage_enums::CountryAlpha2>,
    tax_id: Option<Secret<String>>,
}

#[cfg(feature = "payouts")]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RecurringContract {
    contract: Contract,
}

#[cfg(feature = "payouts")]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
enum Contract {
    Oneclick,
    Recurring,
    Payout,
}

#[cfg(feature = "payouts")]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdyenPayoutResponse {
    psp_reference: String,
    result_code: Option<AdyenStatus>,
    response: Option<AdyenStatus>,
    amount: Option<Amount>,
    merchant_reference: Option<String>,
    refusal_reason: Option<String>,
    refusal_reason_code: Option<String>,
    additional_data: Option<AdditionalData>,
    auth_code: Option<String>,
}

#[cfg(feature = "payouts")]
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdyenPayoutEligibilityRequest {
    amount: Amount,
    merchant_account: Secret<String>,
    payment_method: PayoutCardDetails,
    reference: String,
    shopper_reference: String,
}

#[cfg(feature = "payouts")]
#[derive(Default, Debug, Serialize, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PayoutCardDetails {
    #[serde(rename = "type")]
    payment_method_type: String,
    number: CardNumber,
    expiry_month: Secret<String>,
    expiry_year: Secret<String>,
    holder_name: Secret<String>,
}

#[cfg(feature = "payouts")]
#[derive(Clone, Default, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub enum PayoutEligibility {
    #[serde(rename = "Y")]
    Yes,
    #[serde(rename = "N")]
    #[default]
    No,
    #[serde(rename = "D")]
    Domestic,
    #[serde(rename = "U")]
    Unknown,
}

#[cfg(feature = "payouts")]
#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum AdyenPayoutFulfillRequest {
    GenericFulfillRequest(PayoutFulfillGenericRequest),
    Card(Box<PayoutFulfillCardRequest>),
}

#[cfg(feature = "payouts")]
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PayoutFulfillGenericRequest {
    merchant_account: Secret<String>,
    original_reference: String,
}

#[cfg(feature = "payouts")]
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PayoutFulfillCardRequest {
    amount: Amount,
    card: PayoutCardDetails,
    billing_address: Option<Address>,
    merchant_account: Secret<String>,
    reference: String,
    shopper_name: ShopperName,
    nationality: Option<storage_enums::CountryAlpha2>,
    entity_type: Option<storage_enums::PayoutEntityType>,
}

#[cfg(feature = "payouts")]
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdyenPayoutCancelRequest {
    original_reference: String,
    merchant_account: Secret<String>,
}

#[cfg(feature = "payouts")]
impl TryFrom<&PayoutMethodData> for PayoutCardDetails {
    type Error = Error;
    fn try_from(item: &PayoutMethodData) -> Result<Self, Self::Error> {
        match item {
            PayoutMethodData::Card(card) => Ok(Self {
                payment_method_type: "scheme".to_string(), // FIXME: Remove hardcoding
                number: card.card_number.clone(),
                expiry_month: card.expiry_month.clone(),
                expiry_year: card.expiry_year.clone(),
                holder_name: card
                    .card_holder_name
                    .clone()
                    .get_required_value("card_holder_name")
                    .change_context(errors::ConnectorError::MissingRequiredField {
                        field_name: "payout_method_data.card.holder_name",
                    })?,
            }),
            _ => Err(errors::ConnectorError::MissingRequiredField {
                field_name: "payout_method_data.card",
            })?,
        }
    }
}

// Payouts eligibility request transform
#[cfg(feature = "payouts")]
impl<F> TryFrom<&AdyenRouterData<&PayoutsRouterData<F>>> for AdyenPayoutEligibilityRequest {
    type Error = Error;
    fn try_from(item: &AdyenRouterData<&PayoutsRouterData<F>>) -> Result<Self, Self::Error> {
        let auth_type = AdyenAuthType::try_from(&item.router_data.connector_auth_type)?;
        let payout_method_data =
            PayoutCardDetails::try_from(&item.router_data.get_payout_method_data()?)?;
        Ok(Self {
            amount: Amount {
                currency: item.router_data.request.destination_currency,
                value: item.amount.to_owned(),
            },
            merchant_account: auth_type.merchant_account,
            payment_method: payout_method_data,
            reference: item.router_data.connector_request_reference_id.clone(),
            shopper_reference: item.router_data.merchant_id.get_string_repr().to_owned(),
        })
    }
}

// Payouts create request transform
#[cfg(feature = "payouts")]
impl<F> TryFrom<&PayoutsRouterData<F>> for AdyenPayoutCancelRequest {
    type Error = Error;
    fn try_from(item: &PayoutsRouterData<F>) -> Result<Self, Self::Error> {
        let auth_type = AdyenAuthType::try_from(&item.connector_auth_type)?;

        let merchant_account = auth_type.merchant_account;
        if let Some(id) = &item.request.connector_payout_id {
            Ok(Self {
                merchant_account,
                original_reference: id.to_string(),
            })
        } else {
            Err(errors::ConnectorError::MissingRequiredField {
                field_name: "connector_payout_id",
            })?
        }
    }
}

// Payouts cancel request transform
#[cfg(feature = "payouts")]
impl<F> TryFrom<&AdyenRouterData<&PayoutsRouterData<F>>> for AdyenPayoutCreateRequest {
    type Error = Error;
    fn try_from(item: &AdyenRouterData<&PayoutsRouterData<F>>) -> Result<Self, Self::Error> {
        let auth_type = AdyenAuthType::try_from(&item.router_data.connector_auth_type)?;
        let merchant_account = auth_type.merchant_account;
        let (owner_name, customer_email) = item
            .router_data
            .request
            .customer_details
            .to_owned()
            .map_or((None, None), |c| (c.name, c.email));
        let owner_name = owner_name.get_required_value("owner_name").change_context(
            errors::ConnectorError::MissingRequiredField {
                field_name: "payout_method_data.bank.owner_name",
            },
        )?;

        match item.router_data.get_payout_method_data()? {
            PayoutMethodData::Card(_) => Err(errors::ConnectorError::NotSupported {
                message: "Card payout creation is not supported".to_string(),
                connector: "Adyen",
            })?,
            PayoutMethodData::Bank(bd) => {
                let bank_details = match bd {
                    payouts::Bank::Sepa(b) => PayoutBankDetails {
                        bank_name: b.bank_name,
                        country_code: b.bank_country_code,
                        bank_city: b.bank_city,
                        owner_name,
                        bic: b.bic,
                        iban: b.iban,
                        tax_id: None,
                    },
                    payouts::Bank::Ach(..) => Err(errors::ConnectorError::NotSupported {
                        message: "Bank transfer via ACH is not supported".to_string(),
                        connector: "Adyen",
                    })?,
                    payouts::Bank::Bacs(..) => Err(errors::ConnectorError::NotSupported {
                        message: "Bank transfer via Bacs is not supported".to_string(),
                        connector: "Adyen",
                    })?,
                    payouts::Bank::Pix(..) => Err(errors::ConnectorError::NotSupported {
                        message: "Bank transfer via Pix is not supported".to_string(),
                        connector: "Adyen",
                    })?,
                };
                let bank_data = PayoutBankData { bank: bank_details };
                let address: &hyperswitch_domain_models::address::AddressDetails =
                    item.router_data.get_billing_address()?;
                Ok(Self {
                    amount: Amount {
                        value: item.amount.to_owned(),
                        currency: item.router_data.request.destination_currency,
                    },
                    recurring: RecurringContract {
                        contract: Contract::Payout,
                    },
                    merchant_account,
                    payment_data: PayoutPaymentMethodData::PayoutBankData(bank_data),
                    reference: item.router_data.connector_request_reference_id.to_owned(),
                    shopper_reference: item.router_data.merchant_id.get_string_repr().to_owned(),
                    shopper_email: customer_email,
                    shopper_name: ShopperName {
                        first_name: Some(address.get_first_name()?.to_owned()), // it is a required field for payouts
                        last_name: Some(address.get_last_name()?.to_owned()), // it is a required field for payouts
                    },
                    date_of_birth: None,
                    entity_type: Some(item.router_data.request.entity_type),
                    nationality: get_country_code(item.router_data.get_optional_billing()),
                    billing_address: get_address_info(item.router_data.get_optional_billing())
                        .transpose()?,
                })
            }
            PayoutMethodData::Wallet(wallet_data) => {
                let additional_data = match wallet_data {
                    payouts::Wallet::Paypal(paypal_data) => PayoutAdditionalData {
                        token_data_type: PayoutTokenDataType::PayPal,
                        email_id: paypal_data.email.clone().ok_or(
                            errors::ConnectorError::MissingRequiredField {
                                field_name: "email_address",
                            },
                        )?,
                    },
                    payouts::Wallet::Venmo(_) => Err(errors::ConnectorError::NotSupported {
                        message: "Venmo Wallet is not supported".to_string(),
                        connector: "Adyen",
                    })?,
                    payouts::Wallet::ApplePayDecrypt(_) => {
                        Err(errors::ConnectorError::NotSupported {
                            message: "Apple Pay Decrypt Wallet is not supported".to_string(),
                            connector: "Adyen",
                        })?
                    }
                };
                let address: &hyperswitch_domain_models::address::AddressDetails =
                    item.router_data.get_billing_address()?;
                let payout_wallet = PayoutWalletData {
                    selected_brand: PayoutBrand::Paypal,
                    additional_data,
                };
                Ok(Self {
                    amount: Amount {
                        value: item.amount.to_owned(),
                        currency: item.router_data.request.destination_currency,
                    },
                    recurring: RecurringContract {
                        contract: Contract::Payout,
                    },
                    merchant_account,
                    payment_data: PayoutPaymentMethodData::PayoutWalletData(payout_wallet),
                    reference: item.router_data.connector_request_reference_id.clone(),
                    shopper_reference: item.router_data.merchant_id.get_string_repr().to_owned(),
                    shopper_email: customer_email,
                    shopper_name: ShopperName {
                        first_name: Some(address.get_first_name()?.to_owned()), // it is a required field for payouts
                        last_name: Some(address.get_last_name()?.to_owned()), // it is a required field for payouts
                    },
                    date_of_birth: None,
                    entity_type: Some(item.router_data.request.entity_type),
                    nationality: get_country_code(item.router_data.get_optional_billing()),
                    billing_address: get_address_info(item.router_data.get_optional_billing())
                        .transpose()?,
                })
            }
            PayoutMethodData::BankRedirect(_) => Err(errors::ConnectorError::NotSupported {
                message: "Bank redirect payout creation is not supported".to_string(),
                connector: "Adyen",
            })?,
            PayoutMethodData::Passthrough(_) => Err(errors::ConnectorError::NotSupported {
                message: "Passthrough payout creation is not supported".to_string(),
                connector: "Adyen",
            })?,
        }
    }
}

// Payouts fulfill request transform
#[cfg(feature = "payouts")]
impl<F> TryFrom<&AdyenRouterData<&PayoutsRouterData<F>>> for AdyenPayoutFulfillRequest {
    type Error = Error;
    fn try_from(item: &AdyenRouterData<&PayoutsRouterData<F>>) -> Result<Self, Self::Error> {
        let auth_type = AdyenAuthType::try_from(&item.router_data.connector_auth_type)?;
        let payout_type = item.router_data.request.get_payout_type()?;
        let merchant_account = auth_type.merchant_account;
        match payout_type {
            storage_enums::PayoutType::Bank
            | storage_enums::PayoutType::Wallet
            | storage_enums::PayoutType::BankRedirect => {
                Ok(Self::GenericFulfillRequest(PayoutFulfillGenericRequest {
                    merchant_account,
                    original_reference: item
                        .router_data
                        .request
                        .connector_payout_id
                        .clone()
                        .ok_or(errors::ConnectorError::MissingRequiredField {
                            field_name: "connector_payout_id",
                        })?,
                }))
            }
            storage_enums::PayoutType::Card => {
                let address = item.router_data.get_billing_address()?;
                Ok(Self::Card(Box::new(PayoutFulfillCardRequest {
                    amount: Amount {
                        value: item.amount.to_owned(),
                        currency: item.router_data.request.destination_currency,
                    },
                    card: PayoutCardDetails::try_from(&item.router_data.get_payout_method_data()?)?,
                    billing_address: get_address_info(item.router_data.get_billing().ok())
                        .transpose()?,
                    merchant_account,
                    reference: item.router_data.connector_request_reference_id.clone(),
                    shopper_name: ShopperName {
                        first_name: Some(address.get_first_name()?.to_owned()), // it is a required field for payouts
                        last_name: Some(address.get_last_name()?.to_owned()), // it is a required field for payouts
                    },
                    nationality: get_country_code(item.router_data.get_optional_billing()),
                    entity_type: Some(item.router_data.request.entity_type),
                })))
            }
        }
    }
}

// Payouts response transform
#[cfg(feature = "payouts")]
impl<F> TryFrom<PayoutsResponseRouterData<F, AdyenPayoutResponse>> for PayoutsRouterData<F> {
    type Error = Error;
    fn try_from(
        item: PayoutsResponseRouterData<F, AdyenPayoutResponse>,
    ) -> Result<Self, Self::Error> {
        let response: AdyenPayoutResponse = item.response;
        let payout_eligible = response
            .additional_data
            .and_then(|pa| pa.payout_eligible)
            .map(|pe| pe == PayoutEligibility::Yes || pe == PayoutEligibility::Domestic);

        let status = payout_eligible.map_or(
            {
                response.result_code.map_or(
                    response.response.map(storage_enums::PayoutStatus::from),
                    |rc| Some(storage_enums::PayoutStatus::from(rc)),
                )
            },
            |pe| {
                if pe {
                    Some(storage_enums::PayoutStatus::RequiresFulfillment)
                } else {
                    Some(storage_enums::PayoutStatus::Ineligible)
                }
            },
        );

        Ok(Self {
            response: Ok(PayoutsResponseData {
                status,
                connector_payout_id: Some(response.psp_reference),
                payout_eligible,
                should_add_next_step_to_process_tracker: false,
                error_code: None,
                error_message: None,
                payout_connector_metadata: None,
            }),
            ..item.data
        })
    }
}

#[cfg(feature = "payouts")]
impl From<AdyenStatus> for storage_enums::PayoutStatus {
    fn from(adyen_status: AdyenStatus) -> Self {
        match adyen_status {
            AdyenStatus::Authorised => Self::Success,
            AdyenStatus::PayoutConfirmReceived => Self::Initiated,
            AdyenStatus::Cancelled | AdyenStatus::PayoutDeclineReceived => Self::Cancelled,
            AdyenStatus::Error => Self::Failed,
            AdyenStatus::Pending => Self::Pending,
            AdyenStatus::PayoutSubmitReceived => Self::RequiresFulfillment,
            _ => Self::Ineligible,
        }
    }
}

fn get_merchant_account_code(
    auth_type: &ConnectorAuthType,
) -> CustomResult<Secret<String>, errors::ConnectorError> {
    let auth = AdyenAuthType::try_from(auth_type)
        .change_context(errors::ConnectorError::FailedToObtainAuthType)?;
    Ok(auth.merchant_account.clone())
}

#[derive(Default, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdyenAcceptDisputeRequest {
    dispute_psp_reference: String,
    merchant_account_code: Secret<String>,
}

#[derive(Default, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdyenDisputeResponse {
    pub error_message: Option<String>,
    pub success: bool,
}

impl TryFrom<&AcceptDisputeRouterData> for AdyenAcceptDisputeRequest {
    type Error = Error;
    fn try_from(item: &AcceptDisputeRouterData) -> Result<Self, Self::Error> {
        let merchant_account_code = get_merchant_account_code(&item.connector_auth_type)?;
        Ok(Self {
            dispute_psp_reference: item.clone().request.connector_dispute_id,
            merchant_account_code,
        })
    }
}

#[derive(Default, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdyenDefendDisputeRequest {
    dispute_psp_reference: String,
    merchant_account_code: Secret<String>,
    defense_reason_code: String,
}

impl TryFrom<&DefendDisputeRouterData> for AdyenDefendDisputeRequest {
    type Error = Error;
    fn try_from(item: &DefendDisputeRouterData) -> Result<Self, Self::Error> {
        let merchant_account_code = get_merchant_account_code(&item.connector_auth_type)?;
        Ok(Self {
            dispute_psp_reference: item.request.connector_dispute_id.clone(),
            merchant_account_code,
            defense_reason_code: "SupplyDefenseMaterial".into(),
        })
    }
}

#[derive(Default, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Evidence {
    defense_documents: Vec<DefenseDocuments>,
    merchant_account_code: Secret<String>,
    dispute_psp_reference: String,
}

#[derive(Default, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DefenseDocuments {
    content: Secret<String>,
    content_type: Option<String>,
    defense_document_type_code: String,
}

#[derive(Debug, Deserialize)]
pub struct AdyenTestingData {
    holder_name: Option<Secret<String>>,
}

impl TryFrom<common_utils::pii::SecretSerdeValue> for AdyenTestingData {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(testing_data: common_utils::pii::SecretSerdeValue) -> Result<Self, Self::Error> {
        let testing_data = testing_data
            .expose()
            .parse_value::<Self>("AdyenTestingData")
            .change_context(errors::ConnectorError::InvalidDataFormat {
                field_name: "connector_metadata.adyen.testing",
            })?;
        Ok(testing_data)
    }
}

impl TryFrom<&SubmitEvidenceRouterData> for Evidence {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &SubmitEvidenceRouterData) -> Result<Self, Self::Error> {
        let merchant_account_code = get_merchant_account_code(&item.connector_auth_type)?;
        let submit_evidence_request_data = item.request.clone();
        Ok(Self {
            defense_documents: get_defence_documents(submit_evidence_request_data).ok_or(
                errors::ConnectorError::MissingRequiredField {
                    field_name: "Missing Defence Documents",
                },
            )?,
            merchant_account_code,
            dispute_psp_reference: item.request.connector_dispute_id.clone(),
        })
    }
}

fn get_defence_documents(item: SubmitEvidenceRequestData) -> Option<Vec<DefenseDocuments>> {
    let mut defense_documents: Vec<DefenseDocuments> = Vec::new();
    if let Some(shipping_documentation) = item.shipping_documentation {
        defense_documents.push(DefenseDocuments {
            content: get_content(shipping_documentation).into(),
            content_type: item.receipt_file_type,
            defense_document_type_code: "DefenseMaterial".into(),
        })
    }
    if let Some(receipt) = item.receipt {
        defense_documents.push(DefenseDocuments {
            content: get_content(receipt).into(),
            content_type: item.shipping_documentation_file_type,
            defense_document_type_code: "DefenseMaterial".into(),
        })
    }
    if let Some(invoice_showing_distinct_transactions) = item.invoice_showing_distinct_transactions
    {
        defense_documents.push(DefenseDocuments {
            content: get_content(invoice_showing_distinct_transactions).into(),
            content_type: item.invoice_showing_distinct_transactions_file_type,
            defense_document_type_code: "DefenseMaterial".into(),
        })
    }
    if let Some(customer_communication) = item.customer_communication {
        defense_documents.push(DefenseDocuments {
            content: get_content(customer_communication).into(),
            content_type: item.customer_communication_file_type,
            defense_document_type_code: "DefenseMaterial".into(),
        })
    }
    if let Some(refund_policy) = item.refund_policy {
        defense_documents.push(DefenseDocuments {
            content: get_content(refund_policy).into(),
            content_type: item.refund_policy_file_type,
            defense_document_type_code: "DefenseMaterial".into(),
        })
    }
    if let Some(recurring_transaction_agreement) = item.recurring_transaction_agreement {
        defense_documents.push(DefenseDocuments {
            content: get_content(recurring_transaction_agreement).into(),
            content_type: item.recurring_transaction_agreement_file_type,
            defense_document_type_code: "DefenseMaterial".into(),
        })
    }
    if let Some(uncategorized_file) = item.uncategorized_file {
        defense_documents.push(DefenseDocuments {
            content: get_content(uncategorized_file).into(),
            content_type: item.uncategorized_file_type,
            defense_document_type_code: "DefenseMaterial".into(),
        })
    }
    if let Some(cancellation_policy) = item.cancellation_policy {
        defense_documents.push(DefenseDocuments {
            content: get_content(cancellation_policy).into(),
            content_type: item.cancellation_policy_file_type,
            defense_document_type_code: "DefenseMaterial".into(),
        })
    }
    if let Some(customer_signature) = item.customer_signature {
        defense_documents.push(DefenseDocuments {
            content: get_content(customer_signature).into(),
            content_type: item.customer_signature_file_type,
            defense_document_type_code: "DefenseMaterial".into(),
        })
    }
    if let Some(service_documentation) = item.service_documentation {
        defense_documents.push(DefenseDocuments {
            content: get_content(service_documentation).into(),
            content_type: item.service_documentation_file_type,
            defense_document_type_code: "DefenseMaterial".into(),
        })
    }

    if defense_documents.is_empty() {
        None
    } else {
        Some(defense_documents)
    }
}

fn get_content(item: Vec<u8>) -> String {
    String::from_utf8_lossy(&item).to_string()
}

impl ForeignTryFrom<(&Self, AdyenDisputeResponse)> for AcceptDisputeRouterData {
    type Error = errors::ConnectorError;

    fn foreign_try_from(item: (&Self, AdyenDisputeResponse)) -> Result<Self, Self::Error> {
        let (data, response) = item;

        if response.success {
            Ok(AcceptDisputeRouterData {
                response: Ok(AcceptDisputeResponse {
                    dispute_status: storage_enums::DisputeStatus::DisputeAccepted,
                    connector_status: None,
                }),
                ..data.clone()
            })
        } else {
            Ok(AcceptDisputeRouterData {
                response: Err(ErrorResponse {
                    code: response
                        .error_message
                        .clone()
                        .unwrap_or_else(|| NO_ERROR_CODE.to_string()),
                    message: response
                        .error_message
                        .clone()
                        .unwrap_or_else(|| NO_ERROR_MESSAGE.to_string()),
                    reason: response.error_message,
                    status_code: data.connector_http_status_code.ok_or(
                        errors::ConnectorError::MissingRequiredField {
                            field_name: "http code",
                        },
                    )?,
                    attempt_status: None,
                    connector_transaction_id: None,
                    network_advice_code: None,
                    network_decline_code: None,
                    network_error_message: None,
                    connector_metadata: None,
                }),
                ..data.clone()
            })
        }
    }
}

impl ForeignTryFrom<(&Self, AdyenDisputeResponse)> for SubmitEvidenceRouterData {
    type Error = errors::ConnectorError;
    fn foreign_try_from(item: (&Self, AdyenDisputeResponse)) -> Result<Self, Self::Error> {
        let (data, response) = item;
        if response.success {
            Ok(SubmitEvidenceRouterData {
                response: Ok(SubmitEvidenceResponse {
                    dispute_status: storage_enums::DisputeStatus::DisputeChallenged,
                    connector_status: None,
                }),
                ..data.clone()
            })
        } else {
            Ok(SubmitEvidenceRouterData {
                response: Err(ErrorResponse {
                    code: response
                        .error_message
                        .clone()
                        .unwrap_or_else(|| NO_ERROR_CODE.to_string()),
                    message: response
                        .error_message
                        .clone()
                        .unwrap_or_else(|| NO_ERROR_MESSAGE.to_string()),
                    reason: response.error_message,
                    status_code: data.connector_http_status_code.ok_or(
                        errors::ConnectorError::MissingRequiredField {
                            field_name: "http code",
                        },
                    )?,
                    attempt_status: None,
                    connector_transaction_id: None,
                    network_advice_code: None,
                    network_decline_code: None,
                    network_error_message: None,
                    connector_metadata: None,
                }),
                ..data.clone()
            })
        }
    }
}

impl ForeignTryFrom<(&Self, AdyenDisputeResponse)> for DefendDisputeRouterData {
    type Error = errors::ConnectorError;

    fn foreign_try_from(item: (&Self, AdyenDisputeResponse)) -> Result<Self, Self::Error> {
        let (data, response) = item;

        if response.success {
            Ok(DefendDisputeRouterData {
                response: Ok(DefendDisputeResponse {
                    dispute_status: storage_enums::DisputeStatus::DisputeChallenged,
                    connector_status: None,
                }),
                ..data.clone()
            })
        } else {
            Ok(DefendDisputeRouterData {
                response: Err(ErrorResponse {
                    code: response
                        .error_message
                        .clone()
                        .unwrap_or_else(|| NO_ERROR_CODE.to_string()),
                    message: response
                        .error_message
                        .clone()
                        .unwrap_or_else(|| NO_ERROR_MESSAGE.to_string()),
                    reason: response.error_message,
                    status_code: data.connector_http_status_code.ok_or(
                        errors::ConnectorError::MissingRequiredField {
                            field_name: "http code",
                        },
                    )?,
                    attempt_status: None,
                    connector_transaction_id: None,
                    network_advice_code: None,
                    network_decline_code: None,
                    network_error_message: None,
                    connector_metadata: None,
                }),
                ..data.clone()
            })
        }
    }
}

impl TryFrom<(&NetworkTokenData, Option<Secret<String>>)> for AdyenPaymentMethod<'_> {
    type Error = Error;
    fn try_from(
        (token_data, card_holder_name): (&NetworkTokenData, Option<Secret<String>>),
    ) -> Result<Self, Self::Error> {
        let adyen_network_token = AdyenNetworkTokenData {
            number: token_data.get_network_token(),
            expiry_month: token_data.get_network_token_expiry_month(),
            expiry_year: token_data.get_expiry_year_4_digit(),
            holder_name: card_holder_name,
            brand: None, // FIXME: Remove hardcoding
            network_payment_reference: None,
        };
        Ok(AdyenPaymentMethod::NetworkToken(Box::new(
            adyen_network_token,
        )))
    }
}

impl
    TryFrom<(
        &AdyenRouterData<&PaymentsAuthorizeRouterData>,
        &NetworkTokenData,
    )> for AdyenPaymentRequest<'_>
{
    type Error = Error;
    fn try_from(
        value: (
            &AdyenRouterData<&PaymentsAuthorizeRouterData>,
            &NetworkTokenData,
        ),
    ) -> Result<Self, Self::Error> {
        let (item, token_data) = value;
        let amount = get_amount_data(item);
        let auth_type = AdyenAuthType::try_from(&item.router_data.connector_auth_type)?;
        let shopper_interaction = AdyenShopperInteraction::from(item.router_data);
        let shopper_reference = item.router_data.get_connector_customer_id().ok();
        let (recurring_processing_model, store_payment_method, _) =
            get_recurring_processing_model(item.router_data)?;
        let browser_info = get_browser_info(item.router_data)?;
        let billing_address =
            get_address_info(item.router_data.get_optional_billing()).transpose()?;
        let country_code = get_country_code(item.router_data.get_optional_billing());
        let additional_data = get_additional_data(item.router_data);
        let return_url = item.router_data.request.get_router_return_url()?;
        let testing_data = item
            .router_data
            .request
            .get_connector_testing_data()
            .map(AdyenTestingData::try_from)
            .transpose()?;
        let test_holder_name = testing_data.and_then(|test_data| test_data.holder_name);
        let card_holder_name =
            test_holder_name.or(item.router_data.get_optional_billing_full_name());
        let payment_method = PaymentMethod::AdyenPaymentMethod(Box::new(
            AdyenPaymentMethod::try_from((token_data, card_holder_name))?,
        ));

        let shopper_email = item.router_data.request.email.clone();
        let shopper_name = get_shopper_name(item.router_data.get_optional_billing());
        let mpi_data = AdyenMpiData {
            directory_response: common_enums::TransactionStatus::Success,
            authentication_response: common_enums::TransactionStatus::Success,
            cavv: None,
            token_authentication_verification_value: Some(
                token_data.get_cryptogram().clone().ok_or(
                    errors::ConnectorError::MissingRequiredField {
                        field_name: "network_token_data.token_cryptogram",
                    },
                )?,
            ),
            eci: Some("02".to_string()),
            ds_trans_id: None,
            three_ds_version: None,
            challenge_cancel: None,
            risk_score: None,
            cavv_algorithm: None,
        };
        let adyen_metadata = get_adyen_metadata(item.router_data.request.metadata.clone());

        let (store, splits) = match item.router_data.request.split_payments.as_ref() {
            Some(common_types::payments::SplitPaymentsRequest::AdyenSplitPayment(
                adyen_split_payment,
            )) => get_adyen_split_request(adyen_split_payment, item.router_data.request.currency),
            _ => (adyen_metadata.store.clone(), None),
        };
        let device_fingerprint = adyen_metadata.device_fingerprint.clone();
        let platform_chargeback_logic = adyen_metadata.platform_chargeback_logic.clone();

        let delivery_address =
            get_address_info(item.router_data.get_optional_shipping()).and_then(Result::ok);
        let telephone_number = item.router_data.get_optional_billing_phone_number();
        let application_info = get_application_info(item);

        Ok(AdyenPaymentRequest {
            amount,
            merchant_account: auth_type.merchant_account,
            payment_method,
            reference: item.router_data.connector_request_reference_id.clone(),
            return_url,
            shopper_interaction,
            recurring_processing_model,
            browser_info,
            additional_data,
            telephone_number,
            shopper_name,
            shopper_email,
            shopper_locale: item.router_data.request.locale.clone(),
            social_security_number: None,
            billing_address,
            delivery_address,
            country_code,
            line_items: None,
            shopper_reference,
            store_payment_method,
            channel: None,
            shopper_statement: get_shopper_statement(item.router_data),
            shopper_ip: item.router_data.request.get_ip_address_as_optional(),
            merchant_order_reference: item.router_data.request.merchant_order_reference_id.clone(),
            mpi_data: Some(mpi_data),
            store,
            splits,
            device_fingerprint,
            session_validity: None,
            metadata: item
                .router_data
                .request
                .metadata
                .clone()
                .map(filter_adyen_metadata),
            platform_chargeback_logic,
            application_info,
        })
    }
}

impl AdditionalData {
    // Split merchant advice code into at most 2 parts and get the first part and trim spaces,
    // Return the first part as a String.
    pub fn extract_network_advice_code(&self) -> Option<String> {
        self.merchant_advice_code.as_ref().and_then(|code| {
            let mut parts = code.splitn(2, ':');
            let first_part = parts.next()?.trim();
            // Ensure there is a second part (meaning ':' was present).
            parts.next()?;
            Some(first_part.to_string())
        })
    }
}

pub fn from_payment_method_variant(value: Option<String>) -> Option<common_enums::CardNetwork> {
    let val = value?.to_lowercase();
    let cleaned = val.trim().split('_').next().unwrap_or("");

    match cleaned {
        "visa"
        | "visacredit"
        | "visadebit"
        | "visaelectron"
        | "vpay"
        | "visacorporate"
        | "visacommercialcredit"
        | "visacommercialdebit"
        | "visapurchasing"
        | "visastandardcredit"
        | "visastandarddebit"
        | "visapremiumcredit"
        | "visapremiumdebit"
        | "visasignature"
        | "visagold"
        | "visaplatinum"
        | "visaproprietary"
        | "visacheckout"
        | "visafleetcredit"
        | "visafleetdebit"
        | "visaalphabankbonus"
        | "visacorporatedebit"
        | "visacorporatecredit"
        | "visacommercialsuperpremiumcredit"
        | "visacommercialsuperpremiumdebit" => Some(common_enums::CardNetwork::Visa),

        "mc"
        | "mccredit"
        | "mcdebit"
        | "mcpremiumcredit"
        | "mcpremiumdebit"
        | "mcstandardcredit"
        | "mcstandarddebit"
        | "mcsuperpremiumcredit"
        | "mcsuperpremiumdebit"
        | "mccommercialcredit"
        | "mccommercialdebit"
        | "mccommercialpremiumcredit"
        | "mccommercialpremiumdebit"
        | "mcpurchasingcredit"
        | "mcpurchasingdebit"
        | "mcfleetcredit"
        | "mcfleetdebit"
        | "mcalphabankbonus"
        | "mccorporate"
        | "mccorporatecredit"
        | "mccorporatedebit" => Some(common_enums::CardNetwork::Mastercard),

        "amex"
        | "amexcommercial"
        | "amexconsumer"
        | "amexcorporate"
        | "amexdebit"
        | "amexprepaid"
        | "amexprepaidreloadable"
        | "amexsmallbusiness" => Some(common_enums::CardNetwork::AmericanExpress),

        "jcb" | "jcbcredit" | "jcbdebit" | "jcbprepaid" | "jcbprepaidanonymous" => {
            Some(common_enums::CardNetwork::JCB)
        }

        "diners" | "dinersclub" => Some(common_enums::CardNetwork::DinersClub),

        "discover" => Some(common_enums::CardNetwork::Discover),

        "cartesbancaires" | "cartebancaire" => Some(common_enums::CardNetwork::CartesBancaires),

        "cup" | "cupdebit" | "cupcredit" | "cupprepaid" => {
            Some(common_enums::CardNetwork::UnionPay)
        }

        "interac" | "interac_card" => Some(common_enums::CardNetwork::Interac),

        "rupay" => Some(common_enums::CardNetwork::RuPay),

        "maestro" | "maestrouk" | "maestro_usa" => Some(common_enums::CardNetwork::Maestro),

        "star" => Some(common_enums::CardNetwork::Star),

        "pulse" => Some(common_enums::CardNetwork::Pulse),

        "accel" => Some(common_enums::CardNetwork::Accel),

        "nyce" => Some(common_enums::CardNetwork::Nyce),

        _ => None,
    }
}

#[derive(Debug, Clone)]
pub struct CardExpiry {
    month: u32,
    year: u32,
}

impl CardExpiry {
    pub fn new(month: u32, year: u32) -> Result<Self, errors::ConnectorError> {
        if !(1..=12).contains(&month) {
            return Err(errors::ConnectorError::ParsingFailed);
        }
        Ok(Self { month, year })
    }

    pub fn parse(raw: &str) -> Result<Self, errors::ConnectorError> {
        let cleaned = raw.replace('\\', "");
        let parts: Vec<&str> = cleaned.split('/').collect();

        let (month_str, year_str) = match (parts.first(), parts.get(1)) {
            (Some(m), Some(y)) => (*m, *y),
            _ => return Err(errors::ConnectorError::ParsingFailed),
        };

        let month: u32 = month_str
            .parse()
            .map_err(|_| errors::ConnectorError::ParsingFailed)?;
        let year: u32 = year_str
            .parse()
            .map_err(|_| errors::ConnectorError::ParsingFailed)?;

        Self::new(month, year)
    }

    pub fn month(&self) -> Secret<String> {
        Secret::new(format!("{:02}", self.month))
    }

    pub fn year(&self) -> Secret<String> {
        Secret::new(format!("{:02}", self.year % 100))
    }
}
