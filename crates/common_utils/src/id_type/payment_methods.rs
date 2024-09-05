use crate::id_type::global_id::GlobalId;

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct GlobalPaymentMethodId(GlobalId);
