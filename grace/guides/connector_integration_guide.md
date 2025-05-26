# Hyperswitch Connector Integration: Step-by-Step Guide

This guide provides a reusable, step-by-step process for accurately adding a new payment connector to the Hyperswitch system. It synthesizes information from the "Hyperswitch Connector Integration Assistant" and the general "Connector Integration Process".

Memorize the below types and import accordingly
```
// Std / Built-in
use time::PrimitiveDateTime;
use uuid::Uuid;
// External Crates
use base64::Engine;
use masking::{ExposeInterface, PeekInterface, Secret};
use serde::{Deserialize, Serialize};
use url::Url;
// Common/Internal Utilities
use common_enums::{enums, enums::AuthenticationType, Currency};
use common_utils::{
    consts::{self, BASE64_ENGINE},
    date_time,
    errors::CustomResult,
    ext_traits::ValueExt,
    pii::{self, Email, IpAddress},
    request::Method,
    types::{MinorUnit, StringMajorUnit, StringMinorUnit},
};
// Project Modules - Domain Models
use hyperswitch_domain_models::{
    payment_method_data::{
        BankDebitData, BankRedirectData, BankTransferData, Card, CardRedirectData, GiftCardData,
        PayLaterData, PaymentMethodData, VoucherData, WalletData,
    },
    router_data::{
        AccessToken, AdditionalPaymentMethodConnectorResponse, ConnectorAuthType,
        ConnectorResponseData, ErrorResponse, KlarnaSdkResponse, PaymentMethodToken, RouterData,
    },
    router_flow_types::{
        payments::{Authorize, PostSessionTokens},
        refunds::{Execute, RSync},
        VerifyWebhookSource,
        #[cfg(feature = "payouts")]
        PoFulfill,
    },
    router_request_types::{
        BrowserInformation, CompleteAuthorizeData, PaymentsAuthorizeData, PaymentsCancelData,
        PaymentsCaptureData, PaymentsPostSessionTokensData, PaymentsPreProcessingData,
        PaymentsSetupMandateRequestData, PaymentsSyncData, ResponseId,
        SetupMandateRequestData, VerifyWebhookSourceRequestData,
    },
    router_response_types::{
        MandateReference, PaymentsResponseData, PayoutsResponseData, RedirectForm,
        RefundsResponseData, VerifyWebhookSourceResponseData, VerifyWebhookStatus,
    },
    types::{
        PaymentsAuthorizeRouterData, PaymentsCancelRouterData, PaymentsCaptureRouterData,
        PaymentsCompleteAuthorizeRouterData, PaymentsPostSessionTokensRouterData,
        PaymentsPreProcessingRouterData, RefreshTokenRouterData, RefundsRouterData,
        SdkSessionUpdateRouterData, SetupMandateRouterData, VerifyWebhookSourceRouterData,
    },
};
// Project Modules - Interfaces
use hyperswitch_interfaces::{consts, errors};
// API Models
use api_models::{
    enums,
    payments::{KlarnaSessionTokenResponse, SessionToken},
    webhooks::IncomingWebhookEvent,
    #[cfg(feature = "payouts")]
    payouts::{PayoutMethodData, Wallet as WalletPayout},
};
// Crate (local module) imports
use crate::{
    constants,
    types::{
        PaymentsCaptureResponseRouterData, PaymentsResponseRouterData,
        PaymentsSessionResponseRouterData, PayoutsResponseRouterData, RefundsResponseRouterData,
        ResponseRouterData,
    },
    unimplemented_payment_method,
    utils::{
        self, missing_field_err, to_connector_meta, to_connector_meta_from_secret,
        AccessTokenRequestInfo, AddressData, AddressDetailsData, BrowserInformationData, CardData,
        CardData as CardDataUtil, ForeignTryFrom, PaymentMethodTokenizationRequestData,
        PaymentsAuthorizeRequestData, PaymentsCompleteAuthorizeRequestData,
        PaymentsPostSessionTokensRequestData, PaymentsPreProcessingRequestData,
        PaymentsSetupMandateRequestData, PaymentsSyncRequestData, RouterData as _,
        RouterData as OtherRouterData,
    },
};
```
For more types use `crates/hyperswitch_domain_models/**/types.rs` , `crates/common_utils/src`

# Flow Selection

preprocessing_flow
tokenization_flow
authorize_flow
cancel_flow
capture_flow
psync_flow
access_token_flow
refund
rsync

<|> give examples of how flows work

<|> [Ignore]
complete_authorize_flow
incremental_authorization_flow
post_session_tokens_flow
reject_flow
session_update_flow
setup_mandate_flow
update_metadata_flow
<|> [Ignore]

### Preparation
[IMPORTANT]
[This step can be ignored if already created eg. if grace/connector_integration/{{connector_name}}_plan.md & grace/connector_integration/{{connector_name}}_specs.md is already created, this step can be skipped]


[IMPORTANT]
you have to add all the mandatory and required fields from the reference docs !!!.
Do not miss any thing and do not include any headers structs

First use the  grace/connector_integration/template/tech_spec.md and execute it
post completion use the  grace/connector_integration/template/planner-steps.md and execute it
then follow the implemented plan in the grace/connector_integration/{{connector_name}}_plan.md