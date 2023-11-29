pub mod customer;
pub mod gsm;
mod locker_migration;
pub mod payment;
#[cfg(feature = "payouts")]
pub mod payouts;
pub mod refund;
pub mod routing;
pub mod user;

use common_utils::{
    events::{ApiEventMetric, ApiEventsType},
    impl_misc_api_event_type,
};

use crate::{
    admin::*, api_keys::*, cards_info::*, disputes::*, files::*, mandates::*, payment_methods::*,
    payments::*, verifications::*,
};

impl ApiEventMetric for TimeRange {}

impl_misc_api_event_type!(
    PaymentMethodId,
    PaymentsSessionResponse,
    PaymentMethodListResponse,
    PaymentMethodCreate,
    PaymentLinkInitiateRequest,
    RetrievePaymentLinkResponse,
    MandateListConstraints,
    CreateFileResponse,
    DisputeResponse,
    SubmitEvidenceRequest,
    MerchantConnectorResponse,
    MerchantConnectorId,
    MandateResponse,
    MandateRevokedResponse,
    RetrievePaymentLinkRequest,
    PaymentLinkListConstraints,
    MandateId,
    DisputeListConstraints,
    RetrieveApiKeyResponse,
    BusinessProfileResponse,
    BusinessProfileUpdate,
    BusinessProfileCreate,
    RevokeApiKeyResponse,
    ToggleKVResponse,
    ToggleKVRequest,
    MerchantAccountDeleteResponse,
    MerchantAccountUpdate,
    CardInfoResponse,
    CreateApiKeyResponse,
    CreateApiKeyRequest,
    MerchantConnectorDeleteResponse,
    MerchantConnectorUpdate,
    MerchantConnectorCreate,
    MerchantId,
    CardsInfoRequest,
    MerchantAccountResponse,
    MerchantAccountListRequest,
    MerchantAccountCreate,
    PaymentsSessionRequest,
    ApplepayMerchantVerificationRequest,
    ApplepayMerchantResponse,
    ApplepayVerifiedDomainsResponse,
    UpdateApiKeyRequest
);

#[cfg(feature = "stripe")]
impl_misc_api_event_type!(
    StripeSetupIntentResponse,
    StripeRefundResponse,
    StripePaymentIntentListResponse,
    StripePaymentIntentResponse,
    CustomerDeleteResponse,
    CustomerPaymentMethodListResponse,
    CreateCustomerResponse
);
