use std::sync::{Arc, Mutex};

use router_core::store::PaymentsPort;
use router_core::types;

#[derive(Default)]
pub struct InMemoryPayments {
    values: Arc<Mutex<Vec<types::Payment>>>,
}

#[async_trait::async_trait]
impl PaymentsPort for InMemoryPayments {
    async fn list(&self) -> Vec<crate::types::Payment> {
        self.values.lock().unwrap().clone()
    }

    async fn create(&self, payment: crate::types::Payment) -> crate::types::Payment {
        let mut values = self.values.lock().unwrap();
        values.push(payment.clone());
        payment
    }

    async fn find_by_id(&self, id: u64) -> Option<crate::types::Payment> {
        let values = self.values.lock().unwrap();

        values.iter().find(|payment| payment.id == id).cloned()
    }
}
