pub mod transformers;

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use actix_web::ResponseError;
use common_utils::{
    errors::CustomResult, ext_traits::BytesExt, pii, request::{Method, Request, RequestBuilder, RequestContent}, types::{AmountConvertor, StringMinorUnit, StringMinorUnitForConnector}
};
use error_stack::{report, ResultExt};
use hyperswitch_domain_models::{
    router_data::{AccessToken, ConnectorAuthType, ErrorResponse, RouterData},
    router_flow_types::{
        access_token_auth::AccessTokenAuth,
        payments::{Authorize, Capture, PSync, PaymentMethodToken, Session, SetupMandate, Void},
        refunds::{Execute, RSync},
    },
    router_request_types::{
        AccessTokenRequestData, PaymentMethodTokenizationData, PaymentsAuthorizeData,
        PaymentsCancelData, PaymentsCaptureData, PaymentsSessionData, PaymentsSyncData,
        RefundsData, SetupMandateRequestData,
    },
    router_response_types::{PaymentsResponseData, RefundsResponseData},
    types::{
        PaymentsAuthorizeRouterData, PaymentsCaptureRouterData, PaymentsSyncRouterData,
        PayoutsRouterData, RefundSyncRouterData, RefundsRouterData,
    },
};
#[cfg(feature = "payouts")]
use hyperswitch_domain_models::{
    router_flow_types::payouts::{
        PoCancel, PoCreate, PoFulfill, PoQuote, PoRecipient, PoRecipientAccount,
    },
    router_request_types::PayoutsData,
    router_response_types::PayoutsResponseData,
};
use hyperswitch_interfaces::{
    api::{self, ConnectorCommon, ConnectorCommonExt, ConnectorIntegration, ConnectorValidation},
    configs::Connectors,
    errors,
    events::connector_api_logs::ConnectorEvent,
    types::{self, PayoutCreateType, Response},
    webhooks,
};
use josekit::{jws::{EdDSA, JwsHeader, JwsSigner, ES256}, jwt::{self, JwtPayload}, JoseError, Map, Value};

use masking::{ExposeInterface, Mask};
use serde_json::json;
use transformers as nomupay;

use crate::{constants::headers, types::ResponseRouterData, utils};

#[derive(Clone)]
pub struct Nomupay {
    amount_converter: &'static (dyn AmountConvertor<Output = StringMinorUnit> + Sync),
}

impl Nomupay {
    pub fn new() -> &'static Self {
        &Self {
            amount_converter: &StringMinorUnitForConnector,
        }
    }
}

impl api::Payment for Nomupay {}
impl api::PaymentSession for Nomupay {}
impl api::ConnectorAccessToken for Nomupay {}
impl api::MandateSetup for Nomupay {}
impl api::PaymentAuthorize for Nomupay {}
impl api::PaymentSync for Nomupay {}
impl api::PaymentCapture for Nomupay {}
impl api::PaymentVoid for Nomupay {}
impl api::Refund for Nomupay {}
impl api::RefundExecute for Nomupay {}
impl api::RefundSync for Nomupay {}
impl api::PaymentToken for Nomupay {}

impl ConnectorIntegration<PaymentMethodToken, PaymentMethodTokenizationData, PaymentsResponseData>
    for Nomupay
{
    // Not Implemented (R)
}

impl<Flow, Request, Response> ConnectorCommonExt<Flow, Request, Response> for Nomupay
where
    Self: ConnectorIntegration<Flow, Request, Response>,
{
    fn build_headers(
        &self,
        req: &RouterData<Flow, Request, Response>,
        _connectors: &Connectors,
    ) -> CustomResult<Vec<(String, masking::Maskable<String>)>, errors::ConnectorError> {
        let mut header = vec![(
            headers::CONTENT_TYPE.to_string(),
            self.get_content_type().to_string().into(),
        )];
        let mut api_key = self.get_auth_header(&req.connector_auth_type)?;
        header.append(&mut api_key);
        Ok(header)
    }
}


// #[derive(Serialize, Deserialize, Debug)]
// pub struct NomupayMetadata {
//     pub private_key: String,
// }

impl TryFrom<&Option<pii::SecretSerdeValue>> for nomupay::NomupayMetadata {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(meta_data: &Option<pii::SecretSerdeValue>) -> Result<Self, Self::Error> {
        let metadata: Self = utils::to_connector_meta_from_secret::<Self>(meta_data.clone())
            .change_context(errors::ConnectorError::InvalidConnectorConfig {
                config: "metadata",
            })?;
        Ok(metadata)
    }
}


fn get_body_in_map_form(body: &RequestContent)->Result<Map<String, Value>, errors::ConnectorError> {
    let error = errors::ConnectorError::GenericError { error_message: "josh error".to_string(), error_object: Value::String("unable to create the body".to_string())};
    match body {
        RequestContent::Json(json_body) => {
            let json_str;
            if serde_json::to_string(&json_body).is_err(){
                return Err(error)
            }
            else{
                json_str = serde_json::to_string(&json_body).unwrap();
            } 

            // Parse the JSON string into `serde_json::Value`
            let value: Value ;
            if serde_json::from_str::<String>(&json_str).is_err(){
                return Err(error)
            }
            else{
                value=serde_json::from_str(&json_str).unwrap();
            }

            // Ensure the parsed JSON is an object and convert it to `Map<String, Value>`
            if let Value::Object(map) = value {
                // Now you have a `Map<String, Value>`
                println!("Parsed Map: {:?}", map);
                Ok(map)
            } else {
                return Err(error)
            }
        },
        _=> return Err(error)
    }
}



fn get_private_key(
    metadata: Option<pii::SecretSerdeValue>,
) -> Result<String, errors::ConnectorError> {
    match nomupay::NomupayMetadata::try_from(&metadata) {
        Ok(nomupay_metadata) =>  Ok(nomupay_metadata.private_key),
        Err(_e) =>  Err(errors::ConnectorError::NoConnectorMetaData),
    }
}


fn box_to_jwt_payload(body: Box<dyn masking::ErasedMaskSerialize + Send>) -> Result<JwtPayload, errors::ConnectorError> {
    let error = errors::ConnectorError::GenericError { error_message: "josh error".to_string(), error_object: Value::String("unable to create the body".to_string())};
    // Step 1: Serialize `body` to JSON
    let str_result =  serde_json::to_string(&body);
    let json_str ;
    if str_result.is_ok(){
        json_str=str_result.unwrap();

        let map_result:Result<Map<String, Value>, serde_json::Error>  =serde_json::from_str(&json_str);
        let parsed_json: Map<String, Value> = map_result.unwrap();

        // Step 3: Use the `from_map` method to populate JwtPayload
        let jwt_payload_result = JwtPayload::from_map(parsed_json);
        if jwt_payload_result.is_ok(){
            let jwt_payload = jwt_payload_result.unwrap();
            return Ok(jwt_payload)
        }
    }
    return Err(error)
}


fn get_signature(
    metadata: Option<pii::SecretSerdeValue>,
    auth: nomupay::NomupayAuthType,
    body: RequestContent,
    method: String,
    path: String,

) -> Result<String, errors::ConnectorError> {
    match body {
        RequestContent::Json(masked_json) => {
            let error = errors::ConnectorError::GenericError { error_message: "josh error".to_string(), error_object: Value::String("unable to create the body".to_string())};
            // Calculate expiration time in seconds
            let expiration_time = SystemTime::now() + Duration::from_secs(4 * 60);
            let expires_in = match expiration_time.duration_since(UNIX_EPOCH) {
                Ok(duration) => duration.as_secs(),
                Err(_e) => 0,
            };
            
            // Generate JWT headers-----------------------------------------
            // let custom_headers =  serde_json::json!({
            //     "alg": "ES256",
            //     "aud": format!("{} {}", method, path),
            //     "exp": expires_in,
            //     "kid": auth.kid,
            // });

            let mut option_map = Map::new();
            option_map.insert("alg".to_string(), json!(format!("ES256")));
            option_map.insert("aud".to_string(), json!(format!("{} {}", method, path)));
            option_map.insert("exp".to_string(), json!(expires_in));
            option_map.insert("kid".to_string(), json!(auth.kid));

            let header_result = JwsHeader::from_map(option_map);
            if header_result.is_err(){
                return Err(error);
            }
            let header = header_result.unwrap();


            //Payload------------------------------------------------------
            let mut sample_payload = JwtPayload::new();
            sample_payload.set_subject("subject");


            // let body_map = json_request_to_map(&body);
            let payload_result = box_to_jwt_payload(masked_json);
            if payload_result.is_err() {
                return Err(error);
            }
            let payload = payload_result.unwrap();

            // let optional_payload = serde_json::to_value(masked_json)
            //         .change_context(errors::ConnectorError::RequestEncodingFailed)?
            //         .as_object()
            //         .cloned();
                


            // Private KEY------------------------------------------------------
            let private_key = get_private_key(metadata.to_owned())?;
            // let ES256_signer = JwsSigner::new("ES256")?;
            // Signing JWT------------------------------------------------------
            let signer;
            if  ES256.signer_from_pem(&private_key).is_err(){
                return Err(errors::ConnectorError::GenericError { error_message: "josh error".to_string(), error_object: Value::String("ES256 signer failed".to_string()) });
            }
            else{
                signer = ES256.signer_from_pem(&private_key).unwrap();
            }

            let nomupay_jwt_result= jwt::encode_with_signer(&payload, &header, &signer);
            if nomupay_jwt_result.is_err(){
                return Err(errors::ConnectorError::GenericError { error_message: "josh error".to_string(), error_object: Value::String("jwt generation failed".to_string()) });
            }
            let nomupay_jwt = nomupay_jwt_result.unwrap();

            let jws_blocks: Vec<&str> = nomupay_jwt.split('.').collect();
            let jws_detached = format!("{}..{}", jws_blocks[0], jws_blocks[2]);

            Ok(jws_detached)
        }
        _ => Ok("no json body found".to_string()),
    }
}

impl ConnectorCommon for Nomupay {
    fn id(&self) -> &'static str {
        "nomupay"
    }

    fn get_currency_unit(&self) -> api::CurrencyUnit {
        api::CurrencyUnit::Base
        //    TODO! Check connector documentation, on which unit they are processing the currency.
        //    If the connector accepts amount in lower unit ( i.e cents for USD) then return api::CurrencyUnit::Minor,
        //    if connector accepts amount in base unit (i.e dollars for USD) then return api::CurrencyUnit::Base
    }

    fn common_get_content_type(&self) -> &'static str {
        "application/json"
    }

    fn base_url<'a>(&self, connectors: &'a Connectors) -> &'a str {
        connectors.nomupay.base_url.as_ref()
    }

    fn get_auth_header(
        &self,
        auth_type: &ConnectorAuthType,
    ) -> CustomResult<Vec<(String, masking::Maskable<String>)>, errors::ConnectorError> {
        let auth = nomupay::NomupayAuthType::try_from(auth_type)
            .change_context(errors::ConnectorError::FailedToObtainAuthType)?;
        Ok(vec![(
            headers::AUTHORIZATION.to_string(),
            auth.kid.expose().into_masked(),
        )])
    }

    fn build_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        let response: nomupay::NomupayErrorResponse = res
            .response
            .parse_struct("NomupayErrorResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);

        Ok(ErrorResponse {
            status_code: res.status_code,
            code: response.code,
            message: response.message,
            reason: response.reason,
            attempt_status: None,
            connector_transaction_id: None,
        })
    }
}

impl ConnectorValidation for Nomupay {
    //TODO: implement functions when support enabled
}

impl ConnectorIntegration<Session, PaymentsSessionData, PaymentsResponseData> for Nomupay {
    //TODO: implement sessions flow
}

impl ConnectorIntegration<AccessTokenAuth, AccessTokenRequestData, AccessToken> for Nomupay {}

impl ConnectorIntegration<SetupMandate, SetupMandateRequestData, PaymentsResponseData> for Nomupay {}

impl ConnectorIntegration<Authorize, PaymentsAuthorizeData, PaymentsResponseData> for Nomupay {
    fn get_headers(
        &self,
        req: &PaymentsAuthorizeRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, masking::Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        _req: &PaymentsAuthorizeRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Err(errors::ConnectorError::NotImplemented("get_url method".to_string()).into())
    }

    fn get_request_body(
        &self,
        req: &PaymentsAuthorizeRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let amount = utils::convert_amount(
            self.amount_converter,
            req.request.minor_amount,
            req.request.currency,
        )?;

        let connector_router_data = nomupay::NomupayRouterData::from((amount, req));
        let connector_req = nomupay::NomupayPaymentsRequest::try_from(&connector_router_data)?;
        Ok(RequestContent::Json(Box::new(connector_req)))
    }

    fn build_request(
        &self,
        req: &PaymentsAuthorizeRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Ok(Some(
            RequestBuilder::new()
                .method(Method::Post)
                .url(&types::PaymentsAuthorizeType::get_url(
                    self, req, connectors,
                )?)
                .attach_default_headers()
                .headers(types::PaymentsAuthorizeType::get_headers(
                    self, req, connectors,
                )?)
                .set_body(types::PaymentsAuthorizeType::get_request_body(
                    self, req, connectors,
                )?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &PaymentsAuthorizeRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<PaymentsAuthorizeRouterData, errors::ConnectorError> {
        let response: nomupay::NomupayPaymentsResponse = res
            .response
            .parse_struct("Nomupay PaymentsAuthorizeResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);
        RouterData::try_from(ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        })
    }

    fn get_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res, event_builder)
    }
}

impl ConnectorIntegration<PSync, PaymentsSyncData, PaymentsResponseData> for Nomupay {
    fn get_headers(
        &self,
        req: &PaymentsSyncRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, masking::Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        _req: &PaymentsSyncRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Err(errors::ConnectorError::NotImplemented("get_url method".to_string()).into())
    }

    fn build_request(
        &self,
        req: &PaymentsSyncRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Ok(Some(
            RequestBuilder::new()
                .method(Method::Get)
                .url(&types::PaymentsSyncType::get_url(self, req, connectors)?)
                .attach_default_headers()
                .headers(types::PaymentsSyncType::get_headers(self, req, connectors)?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &PaymentsSyncRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<PaymentsSyncRouterData, errors::ConnectorError> {
        let response: nomupay::NomupayPaymentsResponse = res
            .response
            .parse_struct("nomupay PaymentsSyncResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);
        RouterData::try_from(ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        })
    }

    fn get_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res, event_builder)
    }
}

impl ConnectorIntegration<Capture, PaymentsCaptureData, PaymentsResponseData> for Nomupay {
    fn get_headers(
        &self,
        req: &PaymentsCaptureRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, masking::Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        _req: &PaymentsCaptureRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Err(errors::ConnectorError::NotImplemented("get_url method".to_string()).into())
    }

    fn get_request_body(
        &self,
        _req: &PaymentsCaptureRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        Err(errors::ConnectorError::NotImplemented("get_request_body method".to_string()).into())
    }

    fn build_request(
        &self,
        req: &PaymentsCaptureRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Ok(Some(
            RequestBuilder::new()
                .method(Method::Post)
                .url(&types::PaymentsCaptureType::get_url(self, req, connectors)?)
                .attach_default_headers()
                .headers(types::PaymentsCaptureType::get_headers(
                    self, req, connectors,
                )?)
                .set_body(types::PaymentsCaptureType::get_request_body(
                    self, req, connectors,
                )?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &PaymentsCaptureRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<PaymentsCaptureRouterData, errors::ConnectorError> {
        let response: nomupay::NomupayPaymentsResponse = res
            .response
            .parse_struct("Nomupay PaymentsCaptureResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);
        RouterData::try_from(ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        })
    }

    fn get_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res, event_builder)
    }
}

impl ConnectorIntegration<Void, PaymentsCancelData, PaymentsResponseData> for Nomupay {}

impl ConnectorIntegration<Execute, RefundsData, RefundsResponseData> for Nomupay {
    fn get_headers(
        &self,
        req: &RefundsRouterData<Execute>,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, masking::Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        _req: &RefundsRouterData<Execute>,
        _connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Err(errors::ConnectorError::NotImplemented("get_url method".to_string()).into())
    }

    fn get_request_body(
        &self,
        req: &RefundsRouterData<Execute>,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let refund_amount = utils::convert_amount(
            self.amount_converter,
            req.request.minor_refund_amount,
            req.request.currency,
        )?;

        let connector_router_data = nomupay::NomupayRouterData::from((refund_amount, req));
        let connector_req = nomupay::NomupayRefundRequest::try_from(&connector_router_data)?;
        Ok(RequestContent::Json(Box::new(connector_req)))
    }

    fn build_request(
        &self,
        req: &RefundsRouterData<Execute>,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        let request = RequestBuilder::new()
            .method(Method::Post)
            .url(&types::RefundExecuteType::get_url(self, req, connectors)?)
            .attach_default_headers()
            .headers(types::RefundExecuteType::get_headers(
                self, req, connectors,
            )?)
            .set_body(types::RefundExecuteType::get_request_body(
                self, req, connectors,
            )?)
            .build();
        Ok(Some(request))
    }

    fn handle_response(
        &self,
        data: &RefundsRouterData<Execute>,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<RefundsRouterData<Execute>, errors::ConnectorError> {
        let response: nomupay::RefundResponse = res
            .response
            .parse_struct("nomupay RefundResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);
        RouterData::try_from(ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        })
    }

    fn get_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res, event_builder)
    }
}

impl ConnectorIntegration<RSync, RefundsData, RefundsResponseData> for Nomupay {
    fn get_headers(
        &self,
        req: &RefundSyncRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, masking::Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        _req: &RefundSyncRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Err(errors::ConnectorError::NotImplemented("get_url method".to_string()).into())
    }

    fn build_request(
        &self,
        req: &RefundSyncRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Ok(Some(
            RequestBuilder::new()
                .method(Method::Get)
                .url(&types::RefundSyncType::get_url(self, req, connectors)?)
                .attach_default_headers()
                .headers(types::RefundSyncType::get_headers(self, req, connectors)?)
                .set_body(types::RefundSyncType::get_request_body(
                    self, req, connectors,
                )?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &RefundSyncRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<RefundSyncRouterData, errors::ConnectorError> {
        let response: nomupay::RefundResponse = res
            .response
            .parse_struct("nomupay RefundSyncResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);
        RouterData::try_from(ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        })
    }

    fn get_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res, event_builder)
    }
}

#[cfg(feature = "payouts")]
impl ConnectorIntegration<PoRecipient, PayoutsData, PayoutsResponseData> for Nomupay {
    fn get_url(
        &self,
        _req: &PayoutsRouterData<PoRecipient>,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!(
            "{}/v1alpha1/sub-account",
            connectors.nomupay.base_url
        ))
    }

    fn get_headers(
        &self,
        req: &PayoutsRouterData<PoRecipient>,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, masking::Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_request_body(
        &self,
        req: &PayoutsRouterData<PoRecipient>,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let connector_req = nomupay::OnboardSubAccountRequest::try_from(req)?;
        Ok(RequestContent::Json(Box::new(connector_req)))
    }

    fn build_request(
        &self,
        req: &PayoutsRouterData<PoRecipient>,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {

        let body = types::PayoutRecipientType::get_request_body(self, req, connectors)?;
        let auth = nomupay::NomupayAuthType::try_from(&req.connector_auth_type)
            .change_context(errors::ConnectorError::FailedToObtainAuthType)?;
        let sign = get_signature(req.connector_meta_data.to_owned(), auth, body, "POST".to_string(), "/v1alpha1/sub-account".to_string())?;
        let mut headers = types::PayoutRecipientType::get_headers(self, req, connectors)?;
        headers.push(("X-Signature".to_string(), masking::Maskable::Normal(sign)));
        let request = RequestBuilder::new()
            .method(Method::Post)
            .url(&types::PayoutRecipientType::get_url(self, req, connectors)?)
            .attach_default_headers()
            .headers(types::PayoutRecipientType::get_headers(
                self, req, connectors,
            )?)
            .set_body(types::PayoutRecipientType::get_request_body(
                self, req, connectors,
            )?)
            .build();


        // let request = RequestBuilder::new()
        //     .method(Method::Post)
        //     .url(&types::PayoutRecipientType::get_url(self, req, connectors)?)
        //     .attach_default_headers()
        //     .headers(types::PayoutRecipientType::get_headers(
        //         self, req, connectors,
        //     )?)
        //     .set_body(types::PayoutRecipientType::get_request_body(
        //         self, req, connectors,
        //     )?)
        //     .build();

        Ok(Some(request))
    }

    // #[instrument(skip_all)]
    fn handle_response(
        &self,
        data: &PayoutsRouterData<PoRecipient>,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<PayoutsRouterData<PoRecipient>, errors::ConnectorError> {
        let response: nomupay::OnboardSubAccountResponse = res
            .response
            .parse_struct("NomupayRecipientCreateResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);

        RouterData::try_from(ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        })
    }

    fn get_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res, event_builder)
    }
}

#[cfg(feature = "payouts")]
impl ConnectorIntegration<PoRecipientAccount, PayoutsData, PayoutsResponseData> for Nomupay {
    fn get_url(
        &self,
        req: &PayoutsRouterData<PoRecipientAccount>,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let sid = req
            .request
            .connector_payout_id
            .to_owned()
            .ok_or(errors::ConnectorError::MissingRequiredField { field_name: "id" })?;

        Ok(format!(
            "{}/v1alpha1/sub-account/{}/transfer-method",
            connectors.nomupay.base_url, sid
        ))
    }

    fn get_headers(
        &self,
        req: &PayoutsRouterData<PoRecipientAccount>,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, masking::Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_request_body(
        &self,
        req: &PayoutsRouterData<PoRecipientAccount>,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let connector_req = nomupay::OnboardTransferMethodRequest::try_from(req)?;
        Ok(RequestContent::Json(Box::new(connector_req)))
    }

    fn build_request(
        &self,
        req: &PayoutsRouterData<PoRecipientAccount>,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        let body = types::PayoutRecipientAccountType::get_request_body(self, req, connectors)?;
        let auth = nomupay::NomupayAuthType::try_from(&req.connector_auth_type)
            .change_context(errors::ConnectorError::FailedToObtainAuthType)?;

        let sid = req
            .request
            .connector_payout_id
            .to_owned()
            .ok_or(errors::ConnectorError::MissingRequiredField { field_name: "id" })?;

        let sign = get_signature(req.connector_meta_data.to_owned(), auth, body, "POST".to_string(), 
            format!("/v1alpha1/sub-account/{}/transfer-method", sid))?;

        let mut headers = types::PayoutRecipientAccountType::get_headers(self, req, connectors)?;
        headers.push(("X-Signature".to_string(), masking::Maskable::Normal(sign)));
        let request = RequestBuilder::new()
            .method(Method::Post)
            .url(&types::PayoutRecipientAccountType::get_url(self, req, connectors)?)
            .attach_default_headers()
            .headers(types::PayoutRecipientAccountType::get_headers(
                self, req, connectors,
            )?)
            .set_body(types::PayoutRecipientAccountType::get_request_body(
                self, req, connectors,
            )?)
            .build();

        Ok(Some(request))
    }

    // #[instrument(skip_all)]
    fn handle_response(
        &self,
        data: &PayoutsRouterData<PoRecipientAccount>,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<PayoutsRouterData<PoRecipientAccount>, errors::ConnectorError> {
        let response: nomupay::OnboardTransferMethodResponse = res
            .response
            .parse_struct("NomupayRecipientAccountCreateResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);

        RouterData::try_from(ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        })
    }

    fn get_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res, event_builder)
    }
}

#[cfg(feature = "payouts")]
impl ConnectorIntegration<PoFulfill, PayoutsData, PayoutsResponseData> for Nomupay {
    fn get_url(
        &self,
        _req: &PayoutsRouterData<PoFulfill>,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        // let auth = nomupay::NomupayAuthType::try_from(&req.connector_auth_type)
        //     .change_context(errors::ConnectorError::FailedToObtainAuthType)?;
        // let transfer_id = req.request.connector_payout_id.to_owned().ok_or(
        //     errors::ConnectorError::MissingRequiredField {
        //         field_name: "transfer_id",
        //     },
        // )?;
        Ok(format!("{}/v1alpha1/payments", connectors.nomupay.base_url))
    }

    fn get_headers(
        &self,
        req: &PayoutsRouterData<PoFulfill>,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, masking::Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_request_body(
        &self,
        req: &PayoutsRouterData<PoFulfill>,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let connector_req = nomupay::PaymentRequest::try_from(req)?;
        Ok(RequestContent::Json(Box::new(connector_req)))
    }

    fn build_request(
        &self,
        req: &PayoutsRouterData<PoFulfill>,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {

        let body = types::PayoutFulfillType::get_request_body(self, req, connectors)?;
        let auth = nomupay::NomupayAuthType::try_from(&req.connector_auth_type)
            .change_context(errors::ConnectorError::FailedToObtainAuthType)?;
        let sign = get_signature(req.connector_meta_data.to_owned(), auth, body, "POST".to_string(), "/v1alpha1/payments".to_string())?;
        let mut headers = types::PayoutFulfillType::get_headers(self, req, connectors)?;
        headers.push(("X-Signature".to_string(), masking::Maskable::Normal(sign)));
        let request = RequestBuilder::new()
            .method(Method::Post)
            .url(&types::PayoutFulfillType::get_url(self, req, connectors)?)
            .attach_default_headers()
            .headers(types::PayoutFulfillType::get_headers(
                self, req, connectors,
            )?)
            .set_body(types::PayoutFulfillType::get_request_body(
                self, req, connectors,
            )?)
            .build();

        Ok(Some(request))
    }

    fn handle_response(
        &self,
        data: &PayoutsRouterData<PoFulfill>,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<PayoutsRouterData<PoFulfill>, errors::ConnectorError> {
        let response: nomupay::PaymentResponse =
            res.response
                .parse_struct("WiseFulfillResponse")
                .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);

        RouterData::try_from(ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        })
    }

    fn get_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res, event_builder)
    }
}

#[async_trait::async_trait]
impl webhooks::IncomingWebhook for Nomupay {
    fn get_webhook_object_reference_id(
        &self,
        _request: &webhooks::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<api_models::webhooks::ObjectReferenceId, errors::ConnectorError> {
        Err(report!(errors::ConnectorError::WebhooksNotImplemented))
    }

    fn get_webhook_event_type(
        &self,
        _request: &webhooks::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<api_models::webhooks::IncomingWebhookEvent, errors::ConnectorError> {
        Err(report!(errors::ConnectorError::WebhooksNotImplemented))
    }

    fn get_webhook_resource_object(
        &self,
        _request: &webhooks::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<Box<dyn masking::ErasedMaskSerialize>, errors::ConnectorError> {
        Err(report!(errors::ConnectorError::WebhooksNotImplemented))
    }
}
