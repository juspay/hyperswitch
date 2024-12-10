
use hyperswitch_interfaces::{
    configs::Connectors,
    api::{self, ConnectorCommon, ConnectorValidation}
};


#[derive(Debug, Clone)]
pub struct CtpMastercard;


impl ConnectorCommon for CtpMastercard {
    fn id(&self) -> &'static str {
        "ctp_mastercard"
    }

    fn base_url<'a>(&self, _connectors: &'a Connectors) -> &'a str {
        ""
    }
}

impl ConnectorValidation for CtpMastercard {}
impl api::Payment for CtpMastercard {}
impl api::PaymentSession for CtpMastercard {}
impl api::ConnectorAccessToken for CtpMastercard {}
impl api::MandateSetup for CtpMastercard {}
impl api::PaymentAuthorize for CtpMastercard {}
impl api::PaymentSync for CtpMastercard {}
impl api::PaymentCapture for CtpMastercard {}
impl api::PaymentVoid for CtpMastercard {}
impl api::Refund for CtpMastercard {}
impl api::RefundExecute for CtpMastercard {}
impl api::RefundSync for CtpMastercard {}
impl api::PaymentToken for CtpMastercard {}
