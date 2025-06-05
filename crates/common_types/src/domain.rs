//! Common types

use std::collections::HashMap;

use common_enums::enums;
use common_utils::{impl_to_sql_from_sql_json, types::MinorUnit};
use diesel::{sql_types::Jsonb, AsExpression, FromSqlRow};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(
    Serialize, Deserialize, Debug, Clone, PartialEq, Eq, FromSqlRow, AsExpression, ToSchema,
)]
#[diesel(sql_type = Jsonb)]
#[serde(deny_unknown_fields)]
/// Fee information for Split Payments to be charged on the payment being collected for Adyen
pub struct AdyenSplitData {
    /// The store identifier
    pub store: Option<String>,
    /// Data for the split items
    pub split_items: Vec<AdyenSplitItem>,
}
impl_to_sql_from_sql_json!(AdyenSplitData);

#[derive(
    Serialize, Deserialize, Debug, Clone, PartialEq, Eq, FromSqlRow, AsExpression, ToSchema,
)]
#[diesel(sql_type = Jsonb)]
#[serde(deny_unknown_fields)]
/// Data for the split items
pub struct AdyenSplitItem {
    /// The amount of the split item
    #[schema(value_type = i64, example = 6540)]
    pub amount: Option<MinorUnit>,
    /// Defines type of split item
    #[schema(value_type = AdyenSplitType, example = "BalanceAccount")]
    pub split_type: enums::AdyenSplitType,
    /// The unique identifier of the account to which the split amount is allocated.
    pub account: Option<String>,
    /// Unique Identifier for the split item
    pub reference: String,
    /// Description for the part of the payment that will be allocated to the specified account.
    pub description: Option<String>,
}
impl_to_sql_from_sql_json!(AdyenSplitItem);

/// Fee information to be charged on the payment being collected for sub-merchant via xendit
#[derive(
    Serialize, Deserialize, Debug, Clone, PartialEq, Eq, FromSqlRow, AsExpression, ToSchema,
)]
#[diesel(sql_type = Jsonb)]
#[serde(deny_unknown_fields)]
pub struct XenditSplitSubMerchantData {
    /// The sub-account user-id that you want to make this transaction for.
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
    /// Merchant category code assigned by acquirer
    #[schema(value_type= String,example = "5812")]
    pub mcc: String,
    /// Merchant country code assigned by acquirer
    #[schema(value_type= String,example = "US")]
    pub merchant_country_code: common_enums::CountryAlpha2,
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
pub struct AcquirerConfigs(pub HashMap<common_utils::id_type::ProfileAcquirerId, AcquirerConfig>);

impl_to_sql_from_sql_json!(AcquirerConfigs);
