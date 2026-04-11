use common_enums::enums::MerchantStorageScheme;
#[cfg(feature = "v2")]
use common_enums::DeleteStatus;
use common_utils::{
    crypto::{self, Encryptable, OptionalEncryptableValue},
    encryption::Encryption,
    errors::CustomResult,
    ext_traits::ValueExt,
    id_type, pii,
    types::{keymanager::ToEncryptable, CreatedBy, Description},
};
use diesel_models::query::customers as query;
use hyperswitch_masking::{ExposeInterface, Secret};
use router_env::{instrument, tracing};
use rustc_hash::FxHashMap;
use serde_json::Value;
use time::PrimitiveDateTime;

#[cfg(feature = "v2")]
use crate::merchant_connector_account::MerchantConnectorAccountTypeDetails;
use crate::{merchant_key_store::MerchantKeyStore, platform};

#[cfg(feature = "v1")]
#[derive(Clone, Debug, router_derive::ToEncryption)]
pub struct Customer {
    pub customer_id: id_type::CustomerId,
    pub merchant_id: id_type::MerchantId,
    #[encrypt]
    pub name: Option<Encryptable<Secret<String>>>,
    #[encrypt]
    pub email: Option<Encryptable<Secret<String, pii::EmailStrategy>>>,
    #[encrypt]
    pub phone: Option<Encryptable<Secret<String>>>,
    pub phone_country_code: Option<String>,
    pub description: Option<Description>,
    pub created_at: PrimitiveDateTime,
    pub metadata: Option<pii::SecretSerdeValue>,
    pub modified_at: PrimitiveDateTime,
    pub connector_customer: Option<pii::SecretSerdeValue>,
    pub address_id: Option<String>,
    pub default_payment_method_id: Option<String>,
    pub updated_by: Option<String>,
    pub version: common_enums::ApiVersion,
    #[encrypt]
    pub tax_registration_id: Option<Encryptable<Secret<String>>>,
    pub document_details: OptionalEncryptableValue,
    pub created_by: Option<CreatedBy>,
    pub last_modified_by: Option<CreatedBy>,
}

#[cfg(feature = "v2")]
#[derive(Clone, Debug, router_derive::ToEncryption)]
pub struct Customer {
    pub merchant_id: id_type::MerchantId,
    #[encrypt]
    pub name: Option<Encryptable<Secret<String>>>,
    #[encrypt]
    pub email: Option<Encryptable<Secret<String, pii::EmailStrategy>>>,
    #[encrypt]
    pub phone: Option<Encryptable<Secret<String>>>,
    pub phone_country_code: Option<String>,
    pub description: Option<Description>,
    pub created_at: PrimitiveDateTime,
    pub metadata: Option<pii::SecretSerdeValue>,
    pub connector_customer: Option<common_types::customers::ConnectorCustomerMap>,
    pub modified_at: PrimitiveDateTime,
    pub default_payment_method_id: Option<id_type::GlobalPaymentMethodId>,
    pub updated_by: Option<String>,
    pub merchant_reference_id: Option<id_type::CustomerId>,
    pub default_billing_address: OptionalEncryptableValue,
    pub default_shipping_address: OptionalEncryptableValue,
    pub id: id_type::GlobalCustomerId,
    pub version: common_enums::ApiVersion,
    pub status: DeleteStatus,
    #[encrypt]
    pub tax_registration_id: Option<Encryptable<Secret<String>>>,
    pub document_details: OptionalEncryptableValue,
    pub created_by: Option<CreatedBy>,
    pub last_modified_by: Option<CreatedBy>,
}

impl Customer {
    /// Get the unique identifier of Customer
    #[cfg(feature = "v1")]
    pub fn get_id(&self) -> &id_type::CustomerId {
        &self.customer_id
    }

    /// Get the global identifier of Customer
    #[cfg(feature = "v2")]
    pub fn get_id(&self) -> &id_type::GlobalCustomerId {
        &self.id
    }

    /// Get the connector customer ID for the specified connector label, if present
    #[cfg(feature = "v1")]
    pub fn get_connector_customer_map(
        &self,
    ) -> FxHashMap<id_type::MerchantConnectorAccountId, String> {
        use hyperswitch_masking::PeekInterface;
        if let Some(connector_customer_value) = &self.connector_customer {
            connector_customer_value
                .peek()
                .clone()
                .parse_value("ConnectorCustomerMap")
                .unwrap_or_default()
        } else {
            FxHashMap::default()
        }
    }

    /// Get the connector customer ID for the specified connector label, if present
    #[cfg(feature = "v1")]
    pub fn get_connector_customer_id(&self, connector_label: &str) -> Option<&str> {
        use hyperswitch_masking::PeekInterface;

        self.connector_customer
            .as_ref()
            .and_then(|connector_customer_value| {
                connector_customer_value.peek().get(connector_label)
            })
            .and_then(|connector_customer| connector_customer.as_str())
    }

    /// Get the connector customer ID for the specified merchant connector account ID, if present
    #[cfg(feature = "v2")]
    pub fn get_connector_customer_id(
        &self,
        merchant_connector_account: &MerchantConnectorAccountTypeDetails,
    ) -> Option<&str> {
        match merchant_connector_account {
            MerchantConnectorAccountTypeDetails::MerchantConnectorAccount(account) => {
                let connector_account_id = account.get_id();
                self.connector_customer
                    .as_ref()?
                    .get(&connector_account_id)
                    .map(|connector_customer_id| connector_customer_id.as_str())
            }
            MerchantConnectorAccountTypeDetails::MerchantConnectorDetails(_) => None,
        }
    }
}

#[cfg(feature = "v2")]
#[derive(Clone, Debug)]
pub struct CustomerGeneralUpdate {
    pub name: crypto::OptionalEncryptableName,
    pub email: Box<crypto::OptionalEncryptableEmail>,
    pub phone: Box<crypto::OptionalEncryptablePhone>,
    pub description: Option<Description>,
    pub phone_country_code: Option<String>,
    pub metadata: Option<pii::SecretSerdeValue>,
    pub connector_customer: Box<Option<common_types::customers::ConnectorCustomerMap>>,
    pub default_billing_address: OptionalEncryptableValue,
    pub default_shipping_address: OptionalEncryptableValue,
    pub default_payment_method_id: Option<Option<id_type::GlobalPaymentMethodId>>,
    pub status: Option<DeleteStatus>,
    pub tax_registration_id: crypto::OptionalEncryptableSecretString,
    pub document_details: OptionalEncryptableValue,
    pub last_modified_by: Option<String>,
}

#[cfg(feature = "v2")]
#[derive(Clone, Debug)]
pub enum CustomerUpdate {
    Update(Box<CustomerGeneralUpdate>),
    ConnectorCustomer {
        connector_customer: Option<common_types::customers::ConnectorCustomerMap>,
        last_modified_by: Option<String>,
    },
    UpdateDefaultPaymentMethod {
        default_payment_method_id: Option<Option<id_type::GlobalPaymentMethodId>>,
        last_modified_by: Option<String>,
    },
}

#[cfg(feature = "v1")]
#[derive(Clone, Debug)]
pub enum CustomerUpdate {
    Update {
        name: crypto::OptionalEncryptableName,
        email: crypto::OptionalEncryptableEmail,
        phone: Box<crypto::OptionalEncryptablePhone>,
        description: Option<Description>,
        phone_country_code: Option<String>,
        metadata: Box<Option<pii::SecretSerdeValue>>,
        connector_customer: Box<Option<pii::SecretSerdeValue>>,
        address_id: Option<String>,
        tax_registration_id: crypto::OptionalEncryptableSecretString,
        document_details: Box<OptionalEncryptableValue>,
        last_modified_by: Option<String>,
    },
    ConnectorCustomer {
        connector_customer: Option<pii::SecretSerdeValue>,
        last_modified_by: Option<String>,
    },
    UpdateDefaultPaymentMethod {
        default_payment_method_id: Option<Option<String>>,
        last_modified_by: Option<String>,
    },
}

pub struct CustomerListConstraints {
    pub limit: u16,
    pub offset: Option<u32>,
    pub customer_id: Option<id_type::CustomerId>,
    pub time_range: Option<common_utils::types::TimeRange>,
}

impl From<CustomerListConstraints> for query::CustomerListConstraints {
    fn from(value: CustomerListConstraints) -> Self {
        Self {
            limit: i64::from(value.limit),
            offset: value.offset.map(i64::from),
            customer_id: value.customer_id,
            time_range: value.time_range,
        }
    }
}

#[async_trait::async_trait]
pub trait CustomerInterface {
    type Error;
    #[cfg(feature = "v1")]
    async fn delete_customer_by_customer_id_merchant_id(
        &self,
        customer_id: &id_type::CustomerId,
        merchant_id: &id_type::MerchantId,
    ) -> CustomResult<bool, Self::Error>;

    #[cfg(feature = "v1")]
    async fn find_customer_optional_by_customer_id_merchant_id(
        &self,
        customer_id: &id_type::CustomerId,
        merchant_id: &id_type::MerchantId,
        key_store: &MerchantKeyStore,
        storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<Option<Customer>, Self::Error>;

    /// This function is to retrieve customer details. If the customer is deleted, it returns
    /// customer details that contains the fields as Redacted
    #[cfg(feature = "v1")]
    async fn find_customer_optional_with_redacted_customer_details_by_customer_id_merchant_id(
        &self,
        customer_id: &id_type::CustomerId,
        merchant_id: &id_type::MerchantId,
        key_store: &MerchantKeyStore,
        storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<Option<Customer>, Self::Error>;

    #[cfg(feature = "v2")]
    async fn find_optional_by_merchant_id_merchant_reference_id(
        &self,
        customer_id: &id_type::CustomerId,
        merchant_id: &id_type::MerchantId,
        key_store: &MerchantKeyStore,
        storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<Option<Customer>, Self::Error>;

    #[cfg(feature = "v1")]
    #[allow(clippy::too_many_arguments)]
    async fn update_customer_by_customer_id_merchant_id(
        &self,
        customer_id: id_type::CustomerId,
        merchant_id: id_type::MerchantId,
        customer: Customer,
        customer_update: CustomerUpdate,
        key_store: &MerchantKeyStore,
        storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<Customer, Self::Error>;

    #[cfg(feature = "v1")]
    async fn find_customer_by_customer_id_merchant_id(
        &self,
        customer_id: &id_type::CustomerId,
        merchant_id: &id_type::MerchantId,
        key_store: &MerchantKeyStore,
        storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<Customer, Self::Error>;

    #[cfg(feature = "v2")]
    async fn find_customer_by_merchant_reference_id_merchant_id(
        &self,
        merchant_reference_id: &id_type::CustomerId,
        merchant_id: &id_type::MerchantId,
        key_store: &MerchantKeyStore,
        storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<Customer, Self::Error>;

    async fn list_customers_by_merchant_id(
        &self,
        merchant_id: &id_type::MerchantId,
        key_store: &MerchantKeyStore,
        constraints: CustomerListConstraints,
    ) -> CustomResult<Vec<Customer>, Self::Error>;

    async fn list_customers_by_merchant_id_with_count(
        &self,
        merchant_id: &id_type::MerchantId,
        key_store: &MerchantKeyStore,
        constraints: CustomerListConstraints,
    ) -> CustomResult<(Vec<Customer>, usize), Self::Error>;

    async fn insert_customer(
        &self,
        customer_data: Customer,
        key_store: &MerchantKeyStore,
        storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<Customer, Self::Error>;

    #[cfg(feature = "v2")]
    #[allow(clippy::too_many_arguments)]
    async fn update_customer_by_global_id(
        &self,
        id: &id_type::GlobalCustomerId,
        customer: Customer,
        customer_update: CustomerUpdate,
        key_store: &MerchantKeyStore,
        storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<Customer, Self::Error>;

    #[cfg(feature = "v2")]
    async fn find_customer_by_global_id(
        &self,
        id: &id_type::GlobalCustomerId,
        key_store: &MerchantKeyStore,
        storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<Customer, Self::Error>;

    #[cfg(feature = "v2")]
    async fn find_customer_by_global_id_merchant_id(
        &self,
        id: &id_type::GlobalCustomerId,
        merchant_id: &id_type::MerchantId,
        key_store: &MerchantKeyStore,
        storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<Customer, Self::Error>;
}

#[cfg(feature = "v1")]
#[instrument]
pub async fn update_connector_customer_in_customers(
    connector_label: &str,
    connector_customer_map: Option<&pii::SecretSerdeValue>,
    connector_customer_id: Option<String>,
    initiator: Option<&platform::Initiator>,
) -> Option<CustomerUpdate> {
    let mut connector_customer_map = connector_customer_map
        .and_then(|connector_customer| connector_customer.clone().expose().as_object().cloned())
        .unwrap_or_default();

    let updated_connector_customer_map = connector_customer_id.map(|connector_customer_id| {
        let connector_customer_value = Value::String(connector_customer_id);
        connector_customer_map.insert(connector_label.to_string(), connector_customer_value);
        connector_customer_map
    });

    updated_connector_customer_map
        .map(Value::Object)
        .map(
            |connector_customer_value| CustomerUpdate::ConnectorCustomer {
                connector_customer: Some(pii::SecretSerdeValue::new(connector_customer_value)),
                last_modified_by: initiator
                    .and_then(|initiator| initiator.to_created_by())
                    .map(|last_modified_by| last_modified_by.to_string()),
            },
        )
}

#[cfg(feature = "v2")]
#[instrument]
pub async fn update_connector_customer_in_customers(
    merchant_connector_account: &MerchantConnectorAccountTypeDetails,
    customer: Option<&Customer>,
    connector_customer_id: Option<String>,
    initiator: Option<&platform::Initiator>,
) -> Option<CustomerUpdate> {
    match merchant_connector_account {
        MerchantConnectorAccountTypeDetails::MerchantConnectorAccount(account) => {
            connector_customer_id.map(|new_conn_cust_id| {
                let connector_account_id = account.get_id().clone();
                let mut connector_customer_map = customer
                    .and_then(|customer| customer.connector_customer.clone())
                    .unwrap_or_default();
                connector_customer_map.insert(connector_account_id, new_conn_cust_id);
                CustomerUpdate::ConnectorCustomer {
                    connector_customer: Some(connector_customer_map),
                    last_modified_by: initiator
                        .and_then(|initiator| initiator.to_created_by())
                        .map(|last_modified_by| last_modified_by.to_string()),
                }
            })
        }
        // TODO: Construct connector_customer for MerchantConnectorDetails if required by connector.
        MerchantConnectorAccountTypeDetails::MerchantConnectorDetails(_) => {
            todo!("Handle connector_customer construction for MerchantConnectorDetails");
        }
    }
}
