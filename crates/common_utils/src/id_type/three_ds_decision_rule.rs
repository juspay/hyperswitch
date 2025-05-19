use std::str::FromStr;

crate::id_type!(
    ThreeDSDecisionRuleId,
    "A type for three_ds_decision_rule_id that can be used for three_ds_decision_rule ids"
);
crate::impl_id_type_methods!(ThreeDSDecisionRuleId, "three_ds_decision_rule_id");

crate::impl_try_from_cow_str_id_type!(ThreeDSDecisionRuleId, "three_ds_decision_rule_id");
crate::impl_generate_id_id_type!(ThreeDSDecisionRuleId, "three_ds_decision_rule");
crate::impl_serializable_secret_id_type!(ThreeDSDecisionRuleId);
crate::impl_queryable_id_type!(ThreeDSDecisionRuleId);
crate::impl_to_sql_from_sql_id_type!(ThreeDSDecisionRuleId);

crate::impl_debug_id_type!(ThreeDSDecisionRuleId);

impl FromStr for ThreeDSDecisionRuleId {
    type Err = error_stack::Report<crate::errors::ValidationError>;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let cow_string = std::borrow::Cow::Owned(s.to_string());
        Self::try_from(cow_string)
    }
}
