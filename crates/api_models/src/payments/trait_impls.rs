use common_enums::enums;
use common_utils::errors;

use crate::payments;

impl crate::ValidateFieldAndGet<payments::PaymentsRequest>
    for common_utils::types::RequestExtendedAuthorizationBool
{
    fn validate_field_and_get(
        &self,
        request: &payments::PaymentsRequest,
    ) -> errors::CustomResult<Self, errors::ValidationError>
    where
        Self: Sized,
    {
        match request.capture_method{
            Some(enums::CaptureMethod::Automatic)
            | Some(enums::CaptureMethod::Scheduled)
            | None => Err(error_stack::report!(errors::ValidationError::InvalidValue { message: "request_extended_authorization must be sent only if capture method is manual or manual_multiple".to_string() })),
            Some(enums::CaptureMethod::Manual)
            | Some(enums::CaptureMethod::ManualMultiple) => Ok(self.clone())
        }
    }
}
