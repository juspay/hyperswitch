use super::{
    request::{CreateV1Request, CreateV2Request},
    response::{CreateV1Response, CreateV2Response},
};
use crate::services::clients::payment_methods::error::PaymentMethodClientError;

const DUMMY_PM_ID: &str = "pm_dummy";

impl TryFrom<&CreateV1Request> for CreateV2Request {
    type Error = PaymentMethodClientError;

    fn try_from(value: &CreateV1Request) -> Result<Self, Self::Error> {
        Ok(Self {
            body: Some(value.payload.clone()),
        })
    }
}

impl TryFrom<CreateV2Response> for CreateV1Response {
    type Error = PaymentMethodClientError;

    fn try_from(_: CreateV2Response) -> Result<Self, Self::Error> {
        Ok(Self {
            payment_method_id: DUMMY_PM_ID.to_string(),
            deleted: None,
        })
    }
}
