//! Mandates interface
use api_models::payments;
use common_types::payments as common_payments_types;
use hyperswitch_domain_models::payment_method_data;

#[allow(missing_docs)]
pub trait MandateBehaviour {
    fn get_amount(&self) -> i64;
    fn get_setup_future_usage(&self) -> Option<common_enums::FutureUsage>;
    fn get_mandate_id(&self) -> Option<&payments::MandateIds>;
    fn set_mandate_id(&mut self, new_mandate_id: Option<payments::MandateIds>);
    fn get_payment_method_data(&self) -> payment_method_data::PaymentMethodData;
    fn get_setup_mandate_details(
        &self,
    ) -> Option<&hyperswitch_domain_models::mandates::MandateData>;
    fn get_customer_acceptance(&self) -> Option<common_payments_types::CustomerAcceptance>;
}
