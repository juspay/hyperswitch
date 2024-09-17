use crate::errors;

pub struct DisputeListConstraints {
    pub dispute_id: Option<String>,
    pub payment_id: Option<common_utils::id_type::PaymentId>,
    pub limit: Option<u32>,
    pub offset: Option<u32>,
    pub profile_id: Option<Vec<common_utils::id_type::ProfileId>>,
    pub dispute_status: Option<Vec<common_enums::DisputeStatus>>,
    pub dispute_stage: Option<Vec<common_enums::DisputeStage>>,
    pub reason: Option<String>,
    pub connector: Option<Vec<String>>,
    pub merchant_connector_id: Option<common_utils::id_type::MerchantConnectorAccountId>,
    pub currency: Option<Vec<common_enums::Currency>>,
    pub time_range: Option<api_models::payments::TimeRange>,
}

impl
    TryFrom<(
        api_models::disputes::DisputeListGetConstraints,
        Option<Vec<common_utils::id_type::ProfileId>>,
    )> for DisputeListConstraints
{
    type Error = error_stack::Report<errors::api_error_response::ApiErrorResponse>;
    fn try_from(
        (value, auth_profile_id_list): (
            api_models::disputes::DisputeListGetConstraints,
            Option<Vec<common_utils::id_type::ProfileId>>,
        ),
    ) -> Result<Self, Self::Error> {
        let profile_id_from_request_body = value.profile_id;
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
            dispute_id: value.dispute_id,
            payment_id: value.payment_id,
            limit: value.limit,
            offset: value.offset,
            profile_id: profile_id_list,
            dispute_status: value.dispute_status,
            dispute_stage: value.dispute_stage,
            reason: value.reason,
            connector: value.connector,
            merchant_connector_id: value.merchant_connector_id,
            currency: value.currency,
            time_range: value.time_range,
        })
    }
}
