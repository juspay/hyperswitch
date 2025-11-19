use api_models::payments::PaymentMethodData;
use common_utils::types::MinorUnit;

/// There can be multiple gift-cards, but at most one non-gift card PM
pub struct PaymentMethodAmountSplit {
    pub balance_pm_split: Vec<(PaymentMethodData, MinorUnit)>,
    pub non_balance_pm_split: Option<(PaymentMethodData, MinorUnit)>,
}
