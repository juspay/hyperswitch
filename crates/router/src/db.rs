#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(clippy::unwrap_used)]

// use std::sync::{Arc, Mutex};

pub mod address;
pub mod configs;
pub mod connector_response;
pub mod customers;
pub mod events;
pub mod locker_mock_up;
pub mod mandate;
pub mod merchant_account;
pub mod merchant_connector_account;
pub mod payment_attempt;
pub mod payment_intent;
pub mod payment_method;
pub mod process_tracker;
pub mod refund;
pub mod temp_card;

use dyn_clone::DynClone;

use crate::{
    configs::settings::Database,
    connection::{make_pg_pool, PgPool},
    services::Store,
};

#[async_trait::async_trait]
pub trait Db:
    Send
    + Sync
    + DynClone
    + payment_attempt::IPaymentAttempt
    + mandate::IMandate
    + address::IAddress
    + configs::IConfig
    + temp_card::ITempCard
    + customers::ICustomer
    + events::IEvent
    + merchant_account::IMerchantAccount
    + merchant_connector_account::IMerchantConnectorAccount
    + locker_mock_up::ILockerMockUp
    + payment_intent::IPaymentIntent
    + payment_method::IPaymentMethod
    + process_tracker::IProcessTracker
    + refund::IRefund
    + connector_response::IConnectorResponse
    + 'static
{
}

#[derive(Clone)]
pub struct SqlDb {
    pub conn: PgPool,
}

impl SqlDb {
    pub async fn new(database: &Database) -> Self {
        Self {
            conn: make_pg_pool(database, false).await,
        }
    }

    pub async fn test(database: &Database) -> Self {
        Self {
            conn: make_pg_pool(database, true).await,
        }
    }
}

#[async_trait::async_trait]
impl Db for Store {}

// #[derive(Clone, Default)]
// pub struct InMemoryDb {
//     payments: Arc<Mutex<Vec<PaymentAttempt>>>,
//     mandates: Arc<Mutex<Vec<Mandate>>>,
//     customers: Arc<Mutex<Vec<Customer>>>,
// }

// #[async_trait::async_trait]
// impl Db for InMemoryDb {
//     async fn find_optional_by_customer_id_merchant_id(
//         &self,
//         customer_id: &str,
//         merchant_id: &str,
//     ) -> CustomResult<Option<Customer>, errors::StorageError> {
//         todo!()
//     }

//     #[allow(clippy::unwrap_used)]
//     async fn insert_payment_attempt(
//         &self,
//         payment_attempt: PaymentAttemptNew,
//     ) -> CustomResult<PaymentAttempt, errors::StorageError> {
//         let mut payments = self.payments.lock().unwrap();
//         let id = payments.len() as i32;

//         let payment = PaymentAttempt {
//             id,
//             payment_id: payment_attempt.payment_id,
//             merchant_id: payment_attempt.merchant_id,
//             txn_id: payment_attempt.txn_id,
//             status: payment_attempt.status,
//             amount: payment_attempt.amount,
//             currency: payment_attempt.currency,
//             save_to_locker: payment_attempt.save_to_locker,
//             connector: payment_attempt.connector,
//             error_message: payment_attempt.error_message,
//             offer_amount: payment_attempt.offer_amount,
//             surcharge_amount: payment_attempt.surcharge_amount,
//             tax_amount: payment_attempt.tax_amount,
//             payment_method_id: payment_attempt.payment_method_id,
//             payment_method: payment_attempt.payment_method,
//             payment_flow: payment_attempt.payment_flow,
//             redirect: payment_attempt.redirect,
//             connector_transaction_id: payment_attempt.connector_transaction_id,
//             capture_method: payment_attempt.capture_method,
//             capture_on: payment_attempt.capture_on,
//             confirm: payment_attempt.confirm,
//             authentication_type: payment_attempt.authentication_type,
//             created_at: payment_attempt.created_at.unwrap(),
//             modified_at: payment_attempt.modified_at.unwrap(),
//             last_synced: payment_attempt.last_synced,
//         };
//         payments.push(payment.clone());
//         Ok(payment)
//     }

//     async fn delete_by_customer_id_merchant_id(
//         &self,
//         customer_id: &str,
//         merchant_id: &str,
//     ) -> CustomResult<bool, errors::StorageError> {
//         todo!()
//     }

//     async fn find_by_merchant_id_mandate_id(
//         &self,
//         merchant_id: &str,
//         mandate_id: &str,
//     ) -> CustomResult<Mandate, errors::StorageError> {
//         todo!()
//     }

//     async fn find_by_merchant_id_customer_id(
//         &self,
//         merchant_id: &str,
//         customer_id: &str,
//     ) -> CustomResult<Vec<Mandate>, errors::StorageError> {
//         todo!()
//     }

//     async fn update_by_merchant_id_mandate_id(
//         &self,
//         merchant_id: &str,
//         mandate_id: &str,
//         mandate: MandateUpdate,
//     ) -> CustomResult<Mandate, errors::StorageError> {
//         todo!()
//     }

//     #[allow(clippy::unwrap_used)]
//     async fn insert_mandate(
//         &self,
//         mandate: MandateNew,
//     ) -> CustomResult<Mandate, errors::StorageError> {
//         let mut mandates = self.mandates.lock().unwrap();
//         let id = mandates.len() as i32;

//         let mandate = Mandate {
//             id,
//             mandate_id: mandate.mandate_id,
//             customer_id: mandate.customer_id,
//             merchant_id: mandate.merchant_id,
//             payment_method_id: mandate.payment_method_id,
//             mandate_status: mandate.mandate_status,
//             mandate_type: mandate.mandate_type,
//             customer_accepted_at: mandate.customer_accepted_at,
//             customer_ip_address: mandate.customer_ip_address,
//             customer_user_agent: mandate.customer_user_agent,
//             network_transaction_id: mandate.network_transaction_id,
//             previous_transaction_id: mandate.previous_transaction_id,
//             created_at: mandate.created_at.unwrap(),
//         };
//         mandates.push(mandate.clone());
//         Ok(mandate)
//     }

//     async fn update_by_customer_id_merchant_id(
//         &self,
//         customer_id: String,
//         merchant_id: String,
//         update: CustomerUpdate,
//     ) -> CustomResult<Customer, errors::StorageError> {
//         let mut customers = self.customers.lock().unwrap();

//         let mut customer = customers
//             .iter_mut()
//             .find(|customer| {
//                 customer.customer_id == customer_id && customer.merchant_id == merchant_id
//             })
//             .unwrap();

//         match update {
//             CustomerUpdate::Update {
//                 name,
//                 email,
//                 phone,
//                 description,
//                 phone_country_code,
//                 address,
//                 metadata,
//             } => {
//                 customer.name = name;
//                 customer.email = email;
//                 customer.phone = phone;
//                 customer.description = description;
//                 customer.phone_country_code = phone_country_code;
//                 customer.address = address;
//                 customer.metadata = metadata;
//             }
//         }

//         Ok(customer.clone())
//     }

//     async fn find_by_customer_id_merchant_id(
//         &self,
//         customer_id: &str,
//         merchant_id: &str,
//     ) -> CustomResult<Customer, errors::StorageError> {
//         let customers = self.customers.lock().unwrap();
//         let customer = customers
//             .iter()
//             .find(|customer| {
//                 customer.merchant_id == merchant_id && customer.customer_id == customer_id
//             })
//             .unwrap()
//             .clone();
//         Ok(customer)
//     }

//     async fn insert_customer(
//         &self,
//         customer_data: CreateCustomerRequest,
//     ) -> CustomResult<Customer, errors::StorageError> {
//         let mut customers = self.customers.lock().unwrap();
//         let id = customers.len() as i32;

//         let customer = Customer {
//             id,
//             customer_id: customer_data.customer_id,
//             merchant_id: customer_data.merchant_id,
//             name: customer_data.name,
//             email: customer_data.email,
//             phone: customer_data.phone,
//             phone_country_code: customer_data.phone_country_code,
//             description: customer_data.description,
//             address: customer_data.address,
//             created_at: crate::utils::date_time::now(),
//             metadata: customer_data.metadata,
//         };
//         customers.push(customer.clone());
//         Ok(customer)
//     }
// }

dyn_clone::clone_trait_object!(Db);
