//! Common types

use std::collections::HashMap;

use common_enums::enums;
use common_utils::{impl_to_sql_from_sql_json, types::MinorUnit};
use diesel::{sql_types::Jsonb, AsExpression, FromSqlRow};
#[cfg(feature = "v2")]
use masking::Secret;
use serde::{Deserialize, Serialize};
use smithy::SmithyModel;
use utoipa::ToSchema;

#[derive(
    Serialize,
    Deserialize,
    Debug,
    Clone,
    PartialEq,
    Eq,
    FromSqlRow,
    AsExpression,
    ToSchema,
    SmithyModel,
)]
#[diesel(sql_type = Jsonb)]
#[serde(deny_unknown_fields)]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
/// Fee information for Split Payments to be charged on the payment being collected for Adyen
pub struct AdyenSplitData {
    /// The store identifier
    #[smithy(value_type = "Option<String>")]
    pub store: Option<String>,
    /// Data for the split items
    #[smithy(value_type = "Vec<AdyenSplitItem>")]
    pub split_items: Vec<AdyenSplitItem>,
}
impl_to_sql_from_sql_json!(AdyenSplitData);

#[derive(
    Serialize,
    Deserialize,
    Debug,
    Clone,
    PartialEq,
    Eq,
    FromSqlRow,
    AsExpression,
    ToSchema,
    SmithyModel,
)]
#[diesel(sql_type = Jsonb)]
#[serde(deny_unknown_fields)]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
/// Data for the split items
pub struct AdyenSplitItem {
    /// The amount of the split item
    #[schema(value_type = i64, example = 6540)]
    #[smithy(value_type = "Option<i64>")]
    pub amount: Option<MinorUnit>,
    /// Defines type of split item
    #[schema(value_type = AdyenSplitType, example = "BalanceAccount")]
    #[smithy(value_type = "AdyenSplitType")]
    pub split_type: enums::AdyenSplitType,
    /// The unique identifier of the account to which the split amount is allocated.
    #[smithy(value_type = "Option<String>")]
    pub account: Option<String>,
    /// Unique Identifier for the split item
    #[smithy(value_type = "String")]
    pub reference: String,
    /// Description for the part of the payment that will be allocated to the specified account.
    #[smithy(value_type = "Option<String>")]
    pub description: Option<String>,
}
impl_to_sql_from_sql_json!(AdyenSplitItem);

/// Fee information to be charged on the payment being collected for sub-merchant via xendit
#[derive(
    Serialize,
    Deserialize,
    Debug,
    Clone,
    PartialEq,
    Eq,
    FromSqlRow,
    AsExpression,
    ToSchema,
    SmithyModel,
)]
#[diesel(sql_type = Jsonb)]
#[serde(deny_unknown_fields)]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub struct XenditSplitSubMerchantData {
    /// The sub-account user-id that you want to make this transaction for.
    #[smithy(value_type = "String")]
    pub for_user_id: String,
}
impl_to_sql_from_sql_json!(XenditSplitSubMerchantData);

/// Acquirer configuration
#[derive(Clone, Debug, Deserialize, ToSchema, Serialize, PartialEq)]
pub struct AcquirerConfig {
    /// The merchant id assigned by the acquirer
    #[schema(value_type= String,example = "M123456789")]
    pub acquirer_assigned_merchant_id: String,
    /// merchant name
    #[schema(value_type= String,example = "NewAge Retailer")]
    pub merchant_name: String,
    /// Network provider
    #[schema(value_type= String,example = "VISA")]
    pub network: common_enums::CardNetwork,
    /// Acquirer bin
    #[schema(value_type= String,example = "456789")]
    pub acquirer_bin: String,
    /// Acquirer ica provided by acquirer
    #[schema(value_type= Option<String>,example = "401288")]
    pub acquirer_ica: Option<String>,
    /// Fraud rate for the particular acquirer configuration
    #[schema(value_type= String,example = "0.01")]
    pub acquirer_fraud_rate: f64,
}

#[derive(Serialize, Deserialize, Debug, Clone, FromSqlRow, AsExpression, ToSchema)]
#[diesel(sql_type = Jsonb)]
/// Acquirer configs
pub struct AcquirerConfigMap(pub HashMap<common_utils::id_type::ProfileAcquirerId, AcquirerConfig>);

impl_to_sql_from_sql_json!(AcquirerConfigMap);

/// Merchant connector details
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, ToSchema)]
#[cfg(feature = "v2")]
pub struct MerchantConnectorAuthDetails {
    /// The connector used for the payment
    #[schema(value_type = Connector)]
    pub connector_name: common_enums::connector_enums::Connector,

    /// The merchant connector credentials used for the payment
    #[schema(value_type = Object, example = r#"{
        "merchant_connector_creds": {
            "auth_type": "HeaderKey",
            "api_key":"sk_test_xxxxxexamplexxxxxx12345"
        },
    }"#)]
    pub merchant_connector_creds: common_utils::pii::SecretSerdeValue,
}

/// Connector Response Data that are required to be populated in response, but not persisted in DB.
#[cfg(feature = "v2")]
#[derive(Clone, Debug)]
pub struct ConnectorResponseData {
    /// Stringified connector raw response body
    pub raw_connector_response: Option<Secret<String>>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq, AsExpression, ToSchema)]
#[diesel(sql_type = Jsonb)]
#[serde(rename_all = "snake_case")]
/// Contains the data relevant to the specified GSM feature, if applicable.
/// For example, if the `feature` is `Retry`, this will include configuration
/// details specific to the retry behavior.
pub enum GsmFeatureData {
    /// Represents the data associated with a retry feature in GSM.
    Retry(RetryFeatureData),
}

/// Represents the data associated with a retry feature in GSM.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq, AsExpression, ToSchema)]
#[diesel(sql_type = Jsonb)]
pub struct RetryFeatureData {
    /// indicates if step_up retry is possible
    pub step_up_possible: bool,
    /// indicates if retry with pan is possible
    pub clear_pan_possible: bool,
    /// indicates if retry with alternate network possible
    pub alternate_network_possible: bool,
    /// decision to be taken for auto retries flow
    #[schema(value_type = GsmDecision)]
    pub decision: common_enums::GsmDecision,
}

impl_to_sql_from_sql_json!(GsmFeatureData);
impl_to_sql_from_sql_json!(RetryFeatureData);

impl GsmFeatureData {
    /// Retrieves the retry feature data if it exists.
    pub fn get_retry_feature_data(&self) -> Option<RetryFeatureData> {
        match self {
            Self::Retry(data) => Some(data.clone()),
        }
    }

    /// Retrieves the decision from the retry feature data.
    pub fn get_decision(&self) -> common_enums::GsmDecision {
        match self {
            Self::Retry(data) => data.decision,
        }
    }
}

/// Implementation of methods for `RetryFeatureData`
impl RetryFeatureData {
    /// Checks if step-up retry is possible.
    pub fn is_step_up_possible(&self) -> bool {
        self.step_up_possible
    }

    /// Checks if retry with PAN is possible.
    pub fn is_clear_pan_possible(&self) -> bool {
        self.clear_pan_possible
    }

    /// Checks if retry with alternate network is possible.
    pub fn is_alternate_network_possible(&self) -> bool {
        self.alternate_network_possible
    }

    /// Retrieves the decision to be taken for auto retries flow.
    pub fn get_decision(&self) -> common_enums::GsmDecision {
        self.decision
    }
}
