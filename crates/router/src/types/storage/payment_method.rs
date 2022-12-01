#[cfg(feature = "diesel")]
use diesel::{Identifiable, Insertable, Queryable};
use time::PrimitiveDateTime;

#[cfg(feature = "diesel")]
use crate::schema::payment_methods;
use crate::{pii::Secret, types::storage::enums};

#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "diesel", derive(Identifiable, Queryable))]
#[cfg_attr(feature = "diesel", diesel(table_name = payment_methods))]
pub struct PaymentMethod {
    pub id: i32,
    pub customer_id: String,
    pub merchant_id: String,
    pub payment_method_id: String,
    #[cfg_attr(feature = "diesel", diesel(deserialize_as = super::OptionalDieselArray<enums::Currency>))]
    pub accepted_currency: Option<Vec<enums::Currency>>,
    pub scheme: Option<String>,
    pub token: Option<String>,
    pub cardholder_name: Option<Secret<String>>,
    pub issuer_name: Option<String>,
    pub issuer_country: Option<String>,
    #[cfg_attr(feature = "diesel", diesel(deserialize_as = super::OptionalDieselArray<String>))]
    pub payer_country: Option<Vec<String>>,
    pub is_stored: Option<bool>,
    pub swift_code: Option<String>,
    pub direct_debit_token: Option<String>,
    pub network_transaction_id: Option<String>,
    pub created_at: PrimitiveDateTime,
    pub last_modified: PrimitiveDateTime,
    pub payment_method: enums::PaymentMethodType,
    pub payment_method_type: Option<enums::PaymentMethodSubType>,
    pub payment_method_issuer: Option<String>,
    pub payment_method_issuer_code: Option<enums::PaymentMethodIssuerCode>,
}

#[derive(Clone, Debug, Eq, PartialEq, router_derive::DebugAsDisplay)]
#[cfg_attr(feature = "diesel", derive(Insertable, Queryable))]
#[cfg_attr(feature = "diesel", diesel(table_name = payment_methods))]
pub struct PaymentMethodNew {
    pub customer_id: String,
    pub merchant_id: String,
    pub payment_method_id: String,
    pub payment_method: enums::PaymentMethodType,
    pub payment_method_type: Option<enums::PaymentMethodSubType>,
    pub payment_method_issuer: Option<String>,
    pub payment_method_issuer_code: Option<enums::PaymentMethodIssuerCode>,
    pub accepted_currency: Option<Vec<enums::Currency>>,
    pub scheme: Option<String>,
    pub token: Option<String>,
    pub cardholder_name: Option<Secret<String>>,
    pub issuer_name: Option<String>,
    pub issuer_country: Option<String>,
    pub payer_country: Option<Vec<String>>,
    pub is_stored: Option<bool>,
    pub swift_code: Option<String>,
    pub direct_debit_token: Option<String>,
    pub network_transaction_id: Option<String>,
    pub created_at: PrimitiveDateTime,
    pub last_modified: PrimitiveDateTime,
}

impl Default for PaymentMethodNew {
    fn default() -> Self {
        let now = common_utils::date_time::now();

        Self {
            customer_id: String::default(),
            merchant_id: String::default(),
            payment_method_id: String::default(),
            payment_method: enums::PaymentMethodType::default(),
            payment_method_type: Option::default(),
            payment_method_issuer: Option::default(),
            payment_method_issuer_code: Option::default(),
            accepted_currency: Option::default(),
            scheme: Option::default(),
            token: Option::default(),
            cardholder_name: Option::default(),
            issuer_name: Option::default(),
            issuer_country: Option::default(),
            payer_country: Option::default(),
            is_stored: Option::default(),
            swift_code: Option::default(),
            direct_debit_token: Option::default(),
            network_transaction_id: Option::default(),
            created_at: now,
            last_modified: now,
        }
    }
}
