use akita::{AkitaTable, BaseMapper};
use frunk::LabelledGeneric;
use router_core::store::PaymentsPort;
use router_core::types;

struct Akita(akita::Akita);

#[async_trait::async_trait]
impl PaymentsPort for Akita {
    async fn list(&self) -> Vec<crate::types::Payment> {
        todo!()
    }

    async fn create(&self, payment: crate::types::Payment) -> crate::types::Payment {
        let model: Payment = frunk::labelled_convert_from(payment);
        frunk::labelled_convert_from(model.insert::<Payment, _>(&self.0).unwrap().unwrap())
    }

    async fn find_by_id(&self, id: u64) -> Option<crate::types::Payment> {
        let payment = Payment::default();
        payment.find_by_id(&self.0, id).unwrap().map(frunk::labelled_convert_from)
    }
}

#[derive(Default, LabelledGeneric)]
pub struct NewPayment {
    pub amount: u64,
}

#[derive(AkitaTable, Clone, Debug, Default, PartialEq, Eq, LabelledGeneric)]
pub struct Payment {
    #[table_id(name = "id")]
    pub id: u64,
    pub amount: u64,
}
