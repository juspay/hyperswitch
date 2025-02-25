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
    pub time_range: Option<common_utils::types::TimeRange>,
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
        let api_models::disputes::DisputeListGetConstraints {
            dispute_id,
            payment_id,
            limit,
            offset,
            profile_id,
            dispute_status,
            dispute_stage,
            reason,
            connector,
            merchant_connector_id,
            currency,
            time_range,
        } = value;
        let profile_id_from_request_body = profile_id;
        // Match both the profile ID from the request body and the list of authenticated profile IDs coming from auth layer
        let profile_id_list = match (profile_id_from_request_body, auth_profile_id_list) {
            (None, None) => None,
            // Case when the request body profile ID is None, but authenticated profile IDs are available, return the auth list
            (None, Some(auth_profile_id_list)) => Some(auth_profile_id_list),
            // Case when the request body profile ID is provided, but the auth list is None, create a vector with the request body profile ID
            (Some(profile_id_from_request_body), None) => Some(vec![profile_id_from_request_body]),
            (Some(profile_id_from_request_body), Some(auth_profile_id_list)) => {
                // Check if the profile ID from the request body is present in the authenticated profile ID list
                let profile_id_from_request_body_is_available_in_auth_profile_id_list =
                    auth_profile_id_list.contains(&profile_id_from_request_body);

                if profile_id_from_request_body_is_available_in_auth_profile_id_list {
                    Some(vec![profile_id_from_request_body])
                } else {
                    // If the profile ID is not valid, return an error indicating access is not available
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
            dispute_id,
            payment_id,
            limit,
            offset,
            profile_id: profile_id_list,
            dispute_status,
            dispute_stage,
            reason,
            connector,
            merchant_connector_id,
            currency,
            time_range,
        })
    }
}
