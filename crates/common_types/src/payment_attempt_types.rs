//! Payment attempt related types shared across request/response and database types

use std::collections::HashMap;

use common_utils::pii;
use serde::{Deserialize, Serialize};

// --- Connector Mandate Reference ---

common_utils::impl_to_sql_from_sql_json!(ConnectorMandateReferenceId);
/// Connector mandate reference ID
#[derive(
    Clone, Debug, serde::Deserialize, serde::Serialize, Eq, PartialEq, diesel::AsExpression,
)]
#[diesel(sql_type = diesel::sql_types::Jsonb)]
pub struct ConnectorMandateReferenceId {
    /// Connector mandate ID
    pub connector_mandate_id: Option<String>,
    /// Payment method ID
    pub payment_method_id: Option<String>,
    /// Mandate metadata
    pub mandate_metadata: Option<pii::SecretSerdeValue>,
    /// Connector mandate request reference ID
    pub connector_mandate_request_reference_id: Option<String>,
}

impl ConnectorMandateReferenceId {
    /// Get connector mandate request reference ID
    pub fn get_connector_mandate_request_reference_id(&self) -> Option<String> {
        self.connector_mandate_request_reference_id.clone()
    }

    /// Check if connector mandate ID is present
    pub fn is_connector_mandate_id_present(&self) -> bool {
        self.connector_mandate_id.is_some()
    }
}

// --- Network Details ---

common_utils::impl_to_sql_from_sql_json!(NetworkDetails);
/// Network details
#[derive(
    Clone, Default, Debug, serde::Deserialize, Eq, PartialEq, serde::Serialize, diesel::AsExpression,
)]
#[diesel(sql_type = diesel::sql_types::Jsonb)]
pub struct NetworkDetails {
    /// Network advice code
    pub network_advice_code: Option<String>,
}

// --- Error Details ---

common_utils::impl_to_sql_from_sql_json!(ErrorDetails);
/// Error details nested structs for V1 payment_attempt
#[derive(
    Clone, Default, Debug, serde::Deserialize, Eq, PartialEq, serde::Serialize, diesel::AsExpression,
)]
#[diesel(sql_type = diesel::sql_types::Jsonb)]
pub struct ErrorDetails {
    /// Unified error details
    pub unified_details: Option<UnifiedErrorDetails>,
    /// Issuer error details
    pub issuer_details: Option<IssuerErrorDetails>,
    /// Connector error details
    pub connector_details: Option<ConnectorErrorDetails>,
}

/// Unified error details
#[derive(Clone, Default, Debug, serde::Deserialize, Eq, PartialEq, serde::Serialize)]
pub struct UnifiedErrorDetails {
    /// Category
    pub category: Option<String>,
    /// Message
    pub message: Option<String>,
    /// Standardised code
    pub standardised_code: Option<common_enums::StandardisedCode>,
    /// Description
    pub description: Option<String>,
    /// User guidance message
    pub user_guidance_message: Option<String>,
    /// Recommended action
    pub recommended_action: Option<common_enums::RecommendedAction>,
}

/// Issuer error details
#[derive(Clone, Default, Debug, serde::Deserialize, Eq, PartialEq, serde::Serialize)]
pub struct IssuerErrorDetails {
    /// Code
    pub code: Option<String>,
    /// Message
    pub message: Option<String>,
    /// Network details
    pub network_details: Option<NetworkErrorDetails>,
}

/// Network error details
#[derive(Clone, Default, Debug, serde::Deserialize, Eq, PartialEq, serde::Serialize)]
pub struct NetworkErrorDetails {
    /// Name
    pub name: Option<common_enums::CardNetwork>,
    /// Advice code
    pub advice_code: Option<String>,
    /// Advice message
    pub advice_message: Option<String>,
}

/// Connector error details
#[derive(Clone, Default, Debug, serde::Deserialize, Eq, PartialEq, serde::Serialize)]
pub struct ConnectorErrorDetails {
    /// Code
    pub code: Option<String>,
    /// Message
    pub message: Option<String>,
    /// Reason
    pub reason: Option<String>,
}

// --- Redirect Form ---

/// Redirect form for payment processing
#[derive(Eq, PartialEq, Clone, Debug, Deserialize, Serialize, diesel::AsExpression)]
#[diesel(sql_type = diesel::sql_types::Jsonb)]
pub enum RedirectForm {
    /// Standard form redirect
    Form {
        /// Endpoint URL
        endpoint: String,
        /// HTTP method
        method: common_utils::request::Method,
        /// Form fields
        form_fields: HashMap<String, String>,
    },
    /// HTML redirect
    Html {
        /// HTML data
        html_data: String,
    },
    /// Barclaycard auth setup
    BarclaycardAuthSetup {
        /// Access token
        access_token: String,
        /// DDC URL
        ddc_url: String,
        /// Reference ID
        reference_id: String,
    },
    /// Barclaycard consumer auth
    BarclaycardConsumerAuth {
        /// Access token
        access_token: String,
        /// Step up URL
        step_up_url: String,
    },
    /// BlueSnap redirect
    BlueSnap {
        /// Payment fields token
        payment_fields_token: String,
    },
    /// Cybersource auth setup
    CybersourceAuthSetup {
        /// Access token
        access_token: String,
        /// DDC URL
        ddc_url: String,
        /// Reference ID
        reference_id: String,
    },
    /// Cybersource consumer auth
    CybersourceConsumerAuth {
        /// Access token
        access_token: String,
        /// Step up URL
        step_up_url: String,
    },
    /// Deutschebank 3DS challenge flow
    DeutschebankThreeDSChallengeFlow {
        /// ACS URL
        acs_url: String,
        /// CReq
        creq: String,
    },
    /// Payme redirect
    Payme,
    /// Braintree redirect
    Braintree {
        /// Client token
        client_token: String,
        /// Card token
        card_token: String,
        /// BIN
        bin: String,
        /// ACS URL
        acs_url: String,
    },
    /// NMI redirect
    Nmi {
        /// Amount
        amount: String,
        /// Currency
        currency: common_enums::Currency,
        /// Public key
        public_key: hyperswitch_masking::Secret<String>,
        /// Customer vault ID
        customer_vault_id: String,
        /// Order ID
        order_id: String,
    },
    /// Mifinity redirect
    Mifinity {
        /// Initialization token
        initialization_token: String,
    },
    /// Worldpay DDC form
    WorldpayDDCForm {
        /// Endpoint URL
        endpoint: common_utils::types::Url,
        /// HTTP method
        method: common_utils::request::Method,
        /// Form fields
        form_fields: HashMap<String, String>,
        /// Collection ID
        collection_id: Option<String>,
    },
    /// Worldpay XML redirect form
    WorldpayxmlRedirectForm {
        /// JWT
        jwt: String,
    },
}

common_utils::impl_to_sql_from_sql_json!(RedirectForm);
