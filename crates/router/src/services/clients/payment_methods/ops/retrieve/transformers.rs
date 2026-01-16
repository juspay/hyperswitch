use crate::services::clients::payment_methods::error::PaymentMethodClientError;

use super::request::{RetrieveV1Request, RetrieveV2Request};
use super::response::{RetrieveV1Response, RetrieveV2Response};

const DUMMY_PM_ID: &str = "pm_dummy";

impl TryFrom<&RetrieveV1Request> for RetrieveV2Request {
    type Error = PaymentMethodClientError;

    fn try_from(value: &RetrieveV1Request) -> Result<Self, Self::Error> {
        let _payment_method_id = value.payment_method_id.as_str();
        Ok(Self { body: None })
    }
}

impl TryFrom<RetrieveV2Response> for RetrieveV1Response {
    type Error = PaymentMethodClientError;

    fn try_from(_: RetrieveV2Response) -> Result<Self, Self::Error> {
        Ok(Self {
            payment_method_id: DUMMY_PM_ID.to_string(),
            deleted: None,
        })
    }
}
