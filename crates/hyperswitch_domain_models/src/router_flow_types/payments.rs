// Core related api layer.
#[derive(Debug, Clone)]
pub struct Authorize;

#[derive(Debug, Clone)]
pub struct AuthorizeSessionToken;

#[derive(Debug, Clone)]
pub struct CompleteAuthorize;

#[derive(Debug, Clone)]
pub struct Approve;

// Used in gift cards balance check
#[derive(Debug, Clone)]
pub struct Balance;

#[derive(Debug, Clone)]
pub struct InitPayment;

#[derive(Debug, Clone)]
pub struct Capture;

#[derive(Debug, Clone)]
pub struct PSync;
#[derive(Debug, Clone)]
pub struct Void;

#[derive(Debug, Clone)]
pub struct PostCaptureVoid;

#[derive(Debug, Clone)]
pub struct PostCaptureVoidSync;

#[derive(Debug, Clone)]
pub struct PreAuthorizeVoid;

#[derive(Debug, Clone)]
pub struct Reject;

#[derive(Debug, Clone)]
pub struct Session;

#[derive(Debug, Clone)]
pub struct PaymentMethodToken;

#[derive(Debug, Clone)]
pub struct CreateConnectorCustomer;

#[derive(Debug, Clone)]
pub struct SetupMandate;

#[derive(Debug, Clone)]
pub struct PreProcessing;

#[derive(Debug, Clone)]
pub struct PushNotification;

#[derive(Debug, Clone)]
pub struct GenerateQr;

#[derive(Debug, Clone)]
pub struct IncrementalAuthorization;

#[derive(Debug, Clone)]
pub struct ExtendAuthorization;

#[derive(Debug, Clone)]
pub struct PostProcessing;

#[derive(Debug, Clone)]
pub struct CalculateTax;

#[derive(Debug, Clone)]
pub struct CalculateSurcharge;

#[derive(Debug, Clone)]
pub struct CompleteSurcharge;

#[derive(Debug, Clone)]
pub struct CompleteRefundSurchrge;

#[derive(Debug, Clone)]
pub struct SdkSessionUpdate;

#[derive(Debug, Clone)]
pub struct PaymentCreateIntent;

#[derive(Debug, Clone)]
pub struct PaymentGetIntent;

#[derive(Debug, Clone)]
pub struct PaymentUpdateIntent;

#[derive(Debug, Clone)]
pub struct PostSessionTokens;

#[derive(Debug, Clone)]
pub struct RecordAttempt;

#[derive(Debug, Clone)]
pub struct UpdateMetadata;

#[derive(Debug, Clone)]
pub struct CreateOrder;

#[derive(Debug, Clone)]
pub struct PaymentGetListAttempts;

#[derive(Debug, Clone)]
pub struct ExternalVaultProxy;

#[derive(Debug, Clone)]
pub struct GiftCardBalanceCheck;

#[derive(Debug, Clone)]
pub struct SettlementSplitCreate;

#[derive(Debug, Clone)]
pub struct UpdatePostConfirm;

/// Flows that a `PreDetermined` connector may fail over to the next acquirer for, on a
/// post-authentication external-3DS decline. Only `Authorize` and `SetupMandate` re-enter
/// confirm after external authentication, so every other flow stays ineligible by default.
pub trait ExternalThreeDsRetryEligible {
    fn supports_external_three_ds_retry() -> bool {
        false
    }
}

impl ExternalThreeDsRetryEligible for Authorize {
    fn supports_external_three_ds_retry() -> bool {
        true
    }
}

impl ExternalThreeDsRetryEligible for SetupMandate {
    fn supports_external_three_ds_retry() -> bool {
        true
    }
}

/// Resolves eligibility for an otherwise-unconstrained generic `F` by matching its `TypeId`
/// against the flows above, so callers don't need to carry `F: ExternalThreeDsRetryEligible`
/// through their whole generic call chain.
pub fn is_external_three_ds_retry_eligible_flow<F: 'static>() -> bool {
    use std::any::TypeId;

    (TypeId::of::<F>() == TypeId::of::<Authorize>()
        && Authorize::supports_external_three_ds_retry())
        || (TypeId::of::<F>() == TypeId::of::<SetupMandate>()
            && SetupMandate::supports_external_three_ds_retry())
}
