use common_enums::ApiVersion;
use common_utils::{encryption::Encryption, pii, types::Description};
use diesel::{AsChangeset, Identifiable, Insertable, Queryable, Selectable};
use time::PrimitiveDateTime;

#[cfg(feature = "v1")]
use crate::schema::customers;
#[cfg(feature = "v2")]
use crate::{
    diesel_impl::RequiredFromNullableWithDefault, enums::DeleteStatus, schema_v2::customers,
};

#[cfg(feature = "v1")]
#[derive(
    Clone, Debug, router_derive::DebugAsDisplay, serde::Deserialize, serde::Serialize, Insertable,
)]
#[diesel(table_name = customers)]
pub struct CustomerNew {
    pub customer_id: common_utils::id_type::CustomerId,
    pub merchant_id: common_utils::id_type::MerchantId,
    pub name: Option<Encryption>,
    pub email: Option<Encryption>,
    pub phone: Option<Encryption>,
    pub description: Option<Description>,
    pub phone_country_code: Option<String>,
    pub metadata: Option<pii::SecretSerdeValue>,
    pub connector_customer: Option<pii::SecretSerdeValue>,
    pub created_at: PrimitiveDateTime,
    pub modified_at: PrimitiveDateTime,
    pub address_id: Option<String>,
    pub updated_by: Option<String>,
    pub version: ApiVersion,
    pub tax_registration_id: Option<Encryption>,
    pub created_by: Option<String>,
    pub last_modified_by: Option<String>,
    pub document_details: Option<Encryption>,
    pub id: Option<String>,
}

#[cfg(feature = "v1")]
impl CustomerNew {
    pub fn update_storage_scheme(&mut self, storage_scheme: common_enums::MerchantStorageScheme) {
        self.updated_by = Some(storage_scheme.to_string());
    }
}

#[cfg(feature = "v1")]
impl From<CustomerNew> for Customer {
    fn from(customer_new: CustomerNew) -> Self {
        Self {
            customer_id: customer_new.customer_id,
            merchant_id: customer_new.merchant_id,
            name: customer_new.name,
            email: customer_new.email,
            phone: customer_new.phone,
            phone_country_code: customer_new.phone_country_code,
            description: customer_new.description,
            created_at: customer_new.created_at,
            metadata: customer_new.metadata,
            connector_customer: customer_new.connector_customer,
            modified_at: customer_new.modified_at,
            address_id: customer_new.address_id,
            default_payment_method_id: None,
            updated_by: customer_new.updated_by,
            version: customer_new.version,
            tax_registration_id: customer_new.tax_registration_id,
            document_details: customer_new.document_details,
            created_by: customer_new.created_by,
            last_modified_by: customer_new.last_modified_by,
            id: customer_new.id,
        }
    }
}

#[cfg(feature = "v2")]
#[derive(
    Clone, Debug, Insertable, router_derive::DebugAsDisplay, serde::Deserialize, serde::Serialize,
)]
#[diesel(table_name = customers, primary_key(id))]
pub struct CustomerNew {
    pub merchant_id: common_utils::id_type::MerchantId,
    pub name: Option<Encryption>,
    pub email: Option<Encryption>,
    pub phone: Option<Encryption>,
    pub phone_country_code: Option<String>,
    pub description: Option<Description>,
    pub created_at: PrimitiveDateTime,
    pub metadata: Option<pii::SecretSerdeValue>,
    pub connector_customer: Option<common_types::customers::ConnectorCustomerMap>,
    pub modified_at: PrimitiveDateTime,
    pub default_payment_method_id: Option<common_utils::id_type::GlobalPaymentMethodId>,
    pub updated_by: Option<String>,
    pub version: ApiVersion,
    pub tax_registration_id: Option<Encryption>,
    pub created_by: Option<String>,
    pub last_modified_by: Option<String>,
    pub document_details: Option<Encryption>,
    pub id: common_utils::id_type::GlobalCustomerId,
    pub merchant_reference_id: Option<common_utils::id_type::CustomerId>,
    pub default_billing_address: Option<Encryption>,
    pub default_shipping_address: Option<Encryption>,
    pub status: DeleteStatus,
    pub customer_id: Option<common_utils::id_type::GlobalCustomerId>,
}

#[cfg(feature = "v2")]
impl CustomerNew {
    pub fn update_storage_scheme(&mut self, storage_scheme: common_enums::MerchantStorageScheme) {
        self.updated_by = Some(storage_scheme.to_string());
    }
}

#[cfg(feature = "v2")]
impl From<CustomerNew> for Customer {
    fn from(customer_new: CustomerNew) -> Self {
        Self {
            merchant_id: customer_new.merchant_id,
            name: customer_new.name,
            email: customer_new.email,
            phone: customer_new.phone,
            phone_country_code: customer_new.phone_country_code,
            description: customer_new.description,
            created_at: customer_new.created_at,
            metadata: customer_new.metadata,
            connector_customer: customer_new.connector_customer,
            modified_at: customer_new.modified_at,
            default_payment_method_id: None,
            updated_by: customer_new.updated_by,
            tax_registration_id: customer_new.tax_registration_id,
            document_details: customer_new.document_details,
            merchant_reference_id: customer_new.merchant_reference_id,
            default_billing_address: customer_new.default_billing_address,
            default_shipping_address: customer_new.default_shipping_address,
            id: customer_new.id,
            version: customer_new.version,
            status: customer_new.status,
            created_by: customer_new.created_by,
            last_modified_by: customer_new.last_modified_by,
            customer_id: customer_new.customer_id,
        }
    }
}

#[cfg(feature = "v1")]
#[derive(
    Clone, Debug, Identifiable, Queryable, Selectable, serde::Deserialize, serde::Serialize,
)]
#[diesel(table_name = customers, primary_key(customer_id, merchant_id), check_for_backend(diesel::pg::Pg))]
pub struct Customer {
    pub customer_id: common_utils::id_type::CustomerId,
    pub merchant_id: common_utils::id_type::MerchantId,
    pub name: Option<Encryption>,
    pub email: Option<Encryption>,
    pub phone: Option<Encryption>,
    pub phone_country_code: Option<String>,
    pub description: Option<Description>,
    pub created_at: PrimitiveDateTime,
    pub metadata: Option<pii::SecretSerdeValue>,
    pub connector_customer: Option<pii::SecretSerdeValue>,
    pub modified_at: PrimitiveDateTime,
    pub address_id: Option<String>,
    pub default_payment_method_id: Option<String>,
    pub updated_by: Option<String>,
    pub version: ApiVersion,
    pub tax_registration_id: Option<Encryption>,
    pub created_by: Option<String>,
    pub last_modified_by: Option<String>,
    pub document_details: Option<Encryption>,
    pub id: Option<String>,
}

#[cfg(feature = "v2")]
#[derive(
    Clone, Debug, Identifiable, Queryable, Selectable, serde::Serialize, serde::Deserialize,
)]
#[diesel(table_name = customers, primary_key(id))]
pub struct Customer {
    pub merchant_id: common_utils::id_type::MerchantId,
    pub name: Option<Encryption>,
    pub email: Option<Encryption>,
    pub phone: Option<Encryption>,
    pub phone_country_code: Option<String>,
    pub description: Option<Description>,
    pub created_at: PrimitiveDateTime,
    pub metadata: Option<pii::SecretSerdeValue>,
    pub connector_customer: Option<common_types::customers::ConnectorCustomerMap>,
    pub modified_at: PrimitiveDateTime,
    pub default_payment_method_id: Option<common_utils::id_type::GlobalPaymentMethodId>,
    pub updated_by: Option<String>,
    pub version: ApiVersion,
    pub tax_registration_id: Option<Encryption>,
    pub created_by: Option<String>,
    pub last_modified_by: Option<String>,
    pub document_details: Option<Encryption>,
    pub id: common_utils::id_type::GlobalCustomerId,
    pub merchant_reference_id: Option<common_utils::id_type::CustomerId>,
    pub default_billing_address: Option<Encryption>,
    pub default_shipping_address: Option<Encryption>,
    #[diesel(deserialize_as = RequiredFromNullableWithDefault<DeleteStatus>)]
    pub status: DeleteStatus,
    pub customer_id: Option<common_utils::id_type::GlobalCustomerId>,
}

#[cfg(feature = "v2")]
#[derive(Clone, Debug, diesel::Queryable, serde::Serialize, serde::Deserialize)]
pub struct CustomerGlobalIdMigrationRow {
    pub merchant_id: common_utils::id_type::MerchantId,
    pub customer_id: Option<String>,
    pub id: Option<String>,
    pub version: ApiVersion,
}

#[cfg(feature = "v1")]
#[derive(
    Clone, Debug, AsChangeset, router_derive::DebugAsDisplay, serde::Deserialize, serde::Serialize,
)]
#[diesel(table_name = customers)]
#[router_derive::apply_changeset(target = Customer)]
pub struct CustomerUpdateInternal {
    pub name: Option<Encryption>,
    pub email: Option<Encryption>,
    pub phone: Option<Encryption>,
    pub description: Option<Description>,
    pub phone_country_code: Option<String>,
    pub metadata: Option<pii::SecretSerdeValue>,
    pub modified_at: PrimitiveDateTime,
    pub connector_customer: Option<pii::SecretSerdeValue>,
    pub address_id: Option<String>,
    pub default_payment_method_id: Option<Option<String>>,
    pub updated_by: Option<String>,
    pub tax_registration_id: Option<Encryption>,
    pub last_modified_by: Option<String>,
    pub document_details: Option<Encryption>,
}

#[cfg(feature = "v2")]
#[derive(
    Clone, Debug, AsChangeset, router_derive::DebugAsDisplay, serde::Deserialize, serde::Serialize,
)]
#[diesel(table_name = customers)]
#[router_derive::apply_changeset(target = Customer)]
pub struct CustomerUpdateInternal {
    pub name: Option<Encryption>,
    pub email: Option<Encryption>,
    pub phone: Option<Encryption>,
    pub description: Option<Description>,
    pub phone_country_code: Option<String>,
    pub metadata: Option<pii::SecretSerdeValue>,
    pub modified_at: PrimitiveDateTime,
    pub connector_customer: Option<common_types::customers::ConnectorCustomerMap>,
    pub default_payment_method_id: Option<Option<common_utils::id_type::GlobalPaymentMethodId>>,
    pub updated_by: Option<String>,
    pub default_billing_address: Option<Encryption>,
    pub default_shipping_address: Option<Encryption>,
    pub status: Option<DeleteStatus>,
    pub tax_registration_id: Option<Encryption>,
    pub last_modified_by: Option<String>,
    pub document_details: Option<Encryption>,
}
