pub struct SplitRefundInput {
    pub refund_request: Option<common_types::refunds::SplitRefund>,
    pub payment_charges: Option<common_types::payments::ConnectorChargeResponseData>,
    pub split_payment_request: Option<common_types::payments::SplitPaymentsRequest>,
    pub charge_id: Option<String>,
}

// impl TryFrom<SplitRefundInput> for router_request_types::SplitRefundsRequest {
//     type Error = Report<errors::ApiErrorResponse>;

//     fn try_from(value: SplitRefundInput) -> Result<Self, Self::Error> {
//         let SplitRefundInput {
//             refund_request,
//             payment_charges,
//             charge_id,
//         } = value;

//         match refund_request {
//             common_types::refunds::SplitRefund::StripeSplitRefund(stripe_refund) => {
//                 if let Some((split_charge_id, options)) =
//                  validator::validate_stripe_charge_refund(
//                     &charge_id,
//                     &Some(refund_request),
//                     None,
//                     &Some(payment_charges),
//                 )? {

//                 Ok(Self::StripeSplitRefund(
//                     router_request_types::StripeSplitRefund {
//                         charge_id: split_charge_id.clone(),
//                         transfer_account_id: stripe_refund.transfer_account_id.clone(),
//                         charge_type: stripe_refund.charge_type.clone(),
//                         options,
//                     },
//                 ))
//             } else {
//                 None
//             }
//             }
//             common_types::refunds::SplitRefund::AdyenSplitRefund(adyen_refund) => {
//                         Ok(Self::AdyenSplitRefund(
//                                 adyen_refund,
//                         ))
//                     }
//         }
//     }
// }
