//! Payment related types

use std::collections::HashMap;

use common_enums::enums;
use common_utils::{errors, events, impl_to_sql_from_sql_json, types::MinorUnit};
use diesel::{sql_types::Jsonb, AsExpression, FromSqlRow};
use euclid::{
    dssa::types::EuclidAnalysable,
    frontend::{
        ast::Program,
        dir::{DirKeyKind, DirValue, EuclidDirFilter},
    },
    types::Metadata,
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(
    Serialize, Deserialize, Debug, Clone, PartialEq, Eq, FromSqlRow, AsExpression, ToSchema,
)]
#[diesel(sql_type = Jsonb)]
#[serde(rename_all = "snake_case")]
#[serde(deny_unknown_fields)]
/// Fee information for Split Payments to be charged on the payment being collected
pub enum SplitPaymentsRequest {
    /// StripeSplitPayment
    StripeSplitPayment(StripeSplitPaymentRequest),
}
impl_to_sql_from_sql_json!(SplitPaymentsRequest);

#[derive(
    Serialize, Deserialize, Debug, Clone, PartialEq, Eq, FromSqlRow, AsExpression, ToSchema,
)]
#[diesel(sql_type = Jsonb)]
#[serde(deny_unknown_fields)]
/// Hashmap to store mca_id's with product names
pub struct AuthenticationConnectorAccountMap(
    HashMap<enums::AuthenticationProduct, common_utils::id_type::MerchantConnectorAccountId>,
);
impl_to_sql_from_sql_json!(AuthenticationConnectorAccountMap);

impl AuthenticationConnectorAccountMap {
    /// fn to get click to pay connector_account_id
    pub fn get_click_to_pay_connector_account_id(
        &self,
    ) -> Result<common_utils::id_type::MerchantConnectorAccountId, errors::ValidationError> {
        self.0
            .get(&enums::AuthenticationProduct::ClickToPay)
            .ok_or(errors::ValidationError::MissingRequiredField {
                field_name: "authentication_product_id.click_to_pay".to_string(),
            })
            .cloned()
    }
}

#[derive(
    Serialize, Deserialize, Debug, Clone, PartialEq, Eq, FromSqlRow, AsExpression, ToSchema,
)]
#[diesel(sql_type = Jsonb)]
#[serde(deny_unknown_fields)]
/// Fee information for Split Payments to be charged on the payment being collected for Stripe
pub struct StripeSplitPaymentRequest {
    /// Stripe's charge type
    #[schema(value_type = PaymentChargeType, example = "direct")]
    pub charge_type: enums::PaymentChargeType,

    /// Platform fees to be collected on the payment
    #[schema(value_type = i64, example = 6540)]
    pub application_fees: MinorUnit,

    /// Identifier for the reseller's account to send the funds to
    pub transfer_account_id: String,
}
impl_to_sql_from_sql_json!(StripeSplitPaymentRequest);

#[derive(
    Clone,
    Debug,
    Hash,
    PartialEq,
    Eq,
    strum::Display,
    strum::VariantNames,
    strum::EnumIter,
    strum::EnumString,
    Serialize,
    Deserialize,
    FromSqlRow,
    AsExpression,
)]
#[diesel(sql_type = Jsonb)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
/// AuthenticationType
pub enum AuthenticationType {
    /// If the card is enrolled for 3DS authentication, the 3DS based authentication will be activated. The liability of chargeback shift to the issuer
    ThreeDs,
    /// 3DS based authentication will not be activated. The liability of chargeback stays with the merchant.
    NoThreeDs,
}

impl_to_sql_from_sql_json!(AuthenticationType);

impl AuthenticationType {
    /// Convert to DirValue
    pub fn to_dir_value(&self) -> DirValue {
        match self {
            Self::ThreeDs => DirValue::AuthenticationType(enums::AuthenticationType::ThreeDs),
            Self::NoThreeDs => DirValue::AuthenticationType(enums::AuthenticationType::NoThreeDs),
        }
    }
}

impl EuclidAnalysable for AuthenticationType {
    fn get_dir_value_for_analysis(&self, rule_name: String) -> Vec<(DirValue, Metadata)> {
        let auth = self.to_string();

        vec![(
            self.to_dir_value(),
            HashMap::from_iter([(
                "AUTHENTICATION_TYPE".to_string(),
                serde_json::json!({
                    "rule_name":rule_name,
                    "Authentication_type": auth,
                }),
            )]),
        )]
    }
}

#[derive(
    Serialize, Default, Deserialize, Debug, Clone, PartialEq, Eq, FromSqlRow, AsExpression, ToSchema,
)]
#[diesel(sql_type = Jsonb)]
/// ConditionalConfigs
pub struct ConditionalConfigs {
    /// Override 3DS
    pub override_3ds: Option<AuthenticationType>,
}
impl EuclidDirFilter for ConditionalConfigs {
    const ALLOWED: &'static [DirKeyKind] = &[
        DirKeyKind::PaymentMethod,
        DirKeyKind::CardType,
        DirKeyKind::CardNetwork,
        DirKeyKind::MetaData,
        DirKeyKind::PaymentAmount,
        DirKeyKind::PaymentCurrency,
        DirKeyKind::CaptureMethod,
        DirKeyKind::BillingCountry,
        DirKeyKind::BusinessCountry,
    ];
}

impl_to_sql_from_sql_json!(ConditionalConfigs);

#[derive(Serialize, Deserialize, Debug, Clone, FromSqlRow, AsExpression, ToSchema)]
#[diesel(sql_type = Jsonb)]
/// DecisionManagerRecord
pub struct DecisionManagerRecord {
    /// Name of the Decision Manager
    pub name: String,
    /// Program to be executed
    pub program: Program<ConditionalConfigs>,
    /// Created at timestamp
    pub created_at: i64,
}

impl events::ApiEventMetric for DecisionManagerRecord {
    fn get_api_event_type(&self) -> Option<events::ApiEventsType> {
        Some(events::ApiEventsType::Routing)
    }
}
impl_to_sql_from_sql_json!(DecisionManagerRecord);

/// DecisionManagerResponse
pub type DecisionManagerResponse = DecisionManagerRecord;
