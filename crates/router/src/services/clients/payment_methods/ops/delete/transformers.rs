use super::{
    request::{DeleteV1Request, DeleteV2Request},
    response::{DeleteV1Response, DeleteV2Response},
};
use crate::services::clients::payment_methods::error::PaymentMethodClientError;

const DUMMY_PM_ID: &str = "pm_dummy";

impl TryFrom<&DeleteV1Request> for DeleteV2Request {
    type Error = PaymentMethodClientError;

    fn try_from(value: &DeleteV1Request) -> Result<Self, Self::Error> {
        let _payment_method_id = value.payment_method_id.as_str();
        Ok(Self { body: None })
    }
}

impl TryFrom<DeleteV2Response> for DeleteV1Response {
    type Error = PaymentMethodClientError;

    fn try_from(_: DeleteV2Response) -> Result<Self, Self::Error> {
        Ok(Self {
            payment_method_id: DUMMY_PM_ID.to_string(),
            deleted: Some(true),
        })
    }
}
