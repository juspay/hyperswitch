use crate::errors;

pub struct RefundListConstraints {
    pub payment_id: Option<common_utils::id_type::PaymentId>,
    pub refund_id: Option<String>,
    pub profile_id: Option<Vec<common_utils::id_type::ProfileId>>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
    pub time_range: Option<common_utils::types::TimeRange>,
    pub amount_filter: Option<api_models::payments::AmountFilter>,
    pub connector: Option<Vec<String>>,
    pub merchant_connector_id: Option<Vec<common_utils::id_type::MerchantConnectorAccountId>>,
    pub currency: Option<Vec<common_enums::Currency>>,
    pub refund_status: Option<Vec<common_enums::RefundStatus>>,
}

impl
    TryFrom<(
        api_models::refunds::RefundListRequest,
        Option<Vec<common_utils::id_type::ProfileId>>,
    )> for RefundListConstraints
{
    type Error = error_stack::Report<errors::api_error_response::ApiErrorResponse>;

    fn try_from(
        (value, auth_profile_id_list): (
            api_models::refunds::RefundListRequest,
            Option<Vec<common_utils::id_type::ProfileId>>,
        ),
    ) -> Result<Self, Self::Error> {
        let api_models::refunds::RefundListRequest {
            connector,
            currency,
            refund_status,
            payment_id,
            refund_id,
            profile_id,
            limit,
            offset,
            time_range,
            amount_filter,
            merchant_connector_id,
        } = value;
        let profile_id_from_request_body = profile_id;
        let profile_id_list = match (profile_id_from_request_body, auth_profile_id_list) {
            (None, None) => None,
            (None, Some(auth_profile_id_list)) => Some(auth_profile_id_list),
            (Some(profile_id_from_request_body), None) => Some(vec![profile_id_from_request_body]),
            (Some(profile_id_from_request_body), Some(auth_profile_id_list)) => {
                let profile_id_from_request_body_is_available_in_auth_profile_id_list =
                    auth_profile_id_list.contains(&profile_id_from_request_body);

                if profile_id_from_request_body_is_available_in_auth_profile_id_list {
                    Some(vec![profile_id_from_request_body])
                } else {
                    // This scenario is very unlikely to happen
                    return Err(error_stack::Report::new(
                        errors::api_error_response::ApiErrorResponse::PreconditionFailed {
                            message: format!(
                                "Access not available for the given profile_id {:?}",
                                profile_id_from_request_body
                            ),
                        },
                    ));
                }
            }
        };
        Ok(Self {
            payment_id,
            refund_id,
            profile_id: profile_id_list,
            limit,
            offset,
            time_range,
            amount_filter,
            connector,
            merchant_connector_id,
            currency,
            refund_status,
        })
    }
}
