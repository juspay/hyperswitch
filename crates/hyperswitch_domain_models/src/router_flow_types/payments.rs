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
pub struct IncrementalAuthorization;

#[derive(Debug, Clone)]
pub struct PostProcessing;

#[derive(Debug, Clone)]
pub struct CalculateTax;

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
