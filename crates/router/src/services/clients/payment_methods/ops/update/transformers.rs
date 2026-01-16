use crate::services::clients::payment_methods::error::PaymentMethodClientError;

use super::request::{UpdateV1Request, UpdateV2Request};
use super::response::{UpdateV1Response, UpdateV2Response};

const DUMMY_PM_ID: &str = "pm_dummy";

impl TryFrom<&UpdateV1Request> for UpdateV2Request {
    type Error = PaymentMethodClientError;

    fn try_from(value: &UpdateV1Request) -> Result<Self, Self::Error> {
        Ok(Self {
            body: Some(value.payload.clone()),
        })
    }
}

impl TryFrom<UpdateV2Response> for UpdateV1Response {
    type Error = PaymentMethodClientError;

    fn try_from(_: UpdateV2Response) -> Result<Self, Self::Error> {
        Ok(Self {
            payment_method_id: DUMMY_PM_ID.to_string(),
            deleted: None,
        })
    }
}
