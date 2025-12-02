use api_models::payments::PaymentMethodData;
use common_utils::types::MinorUnit;

#[derive(Clone, Debug)]
pub struct PaymentMethodDetails {
    pub payment_method_data: PaymentMethodData,
    pub payment_method_type: common_enums::PaymentMethod,
    pub payment_method_subtype: common_enums::PaymentMethodType,
}
/// There can be multiple gift-cards, but at most one non-gift card PM
pub struct PaymentMethodAmountSplit {
    pub balance_pm_split: Vec<PaymentMethodDetailsWithSplitAmount>,
    pub non_balance_pm_split: Option<PaymentMethodDetailsWithSplitAmount>,
}

#[derive(Clone, Debug)]
pub struct PaymentMethodDetailsWithSplitAmount {
    pub payment_method_details: PaymentMethodDetails,
    pub split_amount: MinorUnit,
}
