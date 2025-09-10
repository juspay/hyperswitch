/// This module will contain all ucs related implementations for different request types
mod ucs_impls {
    use std::str::FromStr;

    use common_utils::{id_type, ucs_interfaces};
    use router_env::logger;

    #[cfg(feature = "v2")]
    use crate::router_request_types::ExternalVaultProxyPaymentsData;
    use crate::router_request_types::PaymentsAuthorizeData;
    impl ucs_interfaces::UcsHeaderFromRequest for PaymentsAuthorizeData {
        fn get_ucs_reference_id(&self) -> Option<ucs_interfaces::UcsReferenceId> {
            self.merchant_order_reference_id
                .as_ref()
                .map(|merchant_order_reference_id| {
                    id_type::PaymentReferenceId::from_str(merchant_order_reference_id)
                })
                .transpose()
                .inspect_err(
                    |err| logger::warn!(error=?err,"Invalid merchant_order_reference_id received"),
                )
                .ok()
                .flatten()
                .map(ucs_interfaces::UcsReferenceId::Payment)
        }
    }
    #[cfg(feature = "v2")]
    impl ucs_interfaces::UcsHeaderFromRequest for ExternalVaultProxyPaymentsData {
        fn get_ucs_reference_id(&self) -> Option<ucs_interfaces::UcsReferenceId> {
            self.merchant_order_reference_id
                .as_ref()
                .map(|merchant_order_reference_id| {
                    id_type::PaymentReferenceId::from_str(merchant_order_reference_id)
                })
                .transpose()
                .inspect_err(
                    |err| logger::warn!(error=?err,"Invalid merchant_order_reference_id received"),
                )
                .ok()
                .flatten()
                .map(ucs_interfaces::UcsReferenceId::Payment)
        }
    }
}
