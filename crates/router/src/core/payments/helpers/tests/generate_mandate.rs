use std::borrow::Cow;

use common_types::payments::{AcceptanceType, CustomerAcceptance};
use common_utils::{id_type, types::MinorUnit};
use hyperswitch_domain_models::mandates::{MandateAmountData, MandateData, MandateDataType};

use super::super::generate_mandate;

/// A new mandate sets its own `created_at` rather than taking the database default.
#[test]
fn a_new_mandate_carries_its_own_created_at() {
    let mandate_data = MandateData {
        update_mandate_id: None,
        customer_acceptance: Some(CustomerAcceptance {
            acceptance_type: AcceptanceType::Offline,
            accepted_at: None,
            online: None,
        }),
        mandate_type: Some(MandateDataType::SingleUse(MandateAmountData {
            amount: MinorUnit::new(100),
            currency: common_enums::Currency::USD,
            start_date: None,
            end_date: None,
            metadata: None,
        })),
    };
    let customer_id = Some(id_type::CustomerId::try_from(Cow::Borrowed("cus_1")).unwrap());

    let before = common_utils::date_time::now();
    let mandate = generate_mandate(
        id_type::MerchantId::default(),
        id_type::PaymentId::try_from(Cow::Borrowed("pay_1")).unwrap(),
        "stripe".to_owned(),
        Some(mandate_data),
        &customer_id,
        "pm_1".to_owned(),
        None,
        None,
        None,
        None,
        None,
        None,
    )
    .unwrap()
    .expect("a mandate for mandate data and a customer");

    let after = common_utils::date_time::now();

    let created_at = mandate.created_at.expect("created_at is set");
    assert!(before <= created_at && created_at <= after, "{created_at}");
}
