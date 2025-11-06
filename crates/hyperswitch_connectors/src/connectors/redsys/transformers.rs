use base64::Engine;
use common_enums::enums;
use common_utils::{
    consts::BASE64_ENGINE,
    crypto::{EncodeMessage, SignMessage},
    ext_traits::{Encode, ValueExt},
    types::StringMinorUnit,
};
use error_stack::ResultExt;
use hyperswitch_domain_models::{
    address::Address,
    payment_method_data::PaymentMethodData,
    router_data::{ConnectorAuthType, ErrorResponse, RouterData},
    router_flow_types::refunds::{Execute, RSync},
    router_request_types::{
        BrowserInformation, CompleteAuthorizeData, PaymentsAuthorizeData, PaymentsSyncData,
        ResponseId,
    },
    router_response_types::{PaymentsResponseData, RedirectForm, RefundsResponseData},
    types::{
        PaymentsAuthorizeRouterData, PaymentsCancelRouterData, PaymentsCaptureRouterData,
        PaymentsCompleteAuthorizeRouterData, PaymentsPreProcessingRouterData,
        PaymentsSyncRouterData, RefundSyncRouterData, RefundsRouterData,
    },
};
use hyperswitch_interfaces::errors;
use masking::{ExposeInterface, PeekInterface, Secret};
use serde::{Deserialize, Serialize};

use crate::{
    types::{
        PaymentsCancelResponseRouterData, PaymentsCaptureResponseRouterData,
        PaymentsPreprocessingResponseRouterData, RefundsResponseRouterData, ResponseRouterData,
    },
    utils::{
        self as connector_utils, AddressDetailsData, BrowserInformationData, CardData,
        PaymentsAuthorizeRequestData, PaymentsCompleteAuthorizeRequestData,
        PaymentsPreProcessingRequestData, RouterData as _,
    },
};
type Error = error_stack::Report<errors::ConnectorError>;

const DS_VERSION: &str = "0.0";
const SIGNATURE_VERSION: &str = "HMAC_SHA256_V1";
const XMLNS_WEB_URL: &str = "http://webservices.apl02.redsys.es";
pub const REDSYS_SOAP_ACTION: &str = "consultaOperaciones";

// Specifies the type of transaction for XML requests
pub mod transaction_type {
    pub const PAYMENT: &str = "0";
    pub const PREAUTHORIZATION: &str = "1";
    pub const CONFIRMATION: &str = "2";
    pub const REFUND: &str = "3";
    pub const CANCELLATION: &str = "9";
}

pub struct RedsysRouterData<T> {
    pub amount: StringMinorUnit,
    pub currency: api_models::enums::Currency,
    pub router_data: T,
}

impl<T> From<(StringMinorUnit, T, api_models::enums::Currency)> for RedsysRouterData<T> {
    fn from((amount, item, currency): (StringMinorUnit, T, api_models::enums::Currency)) -> Self {
        Self {
            amount,
            currency,
            router_data: item,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub struct PaymentsRequest {
    ds_merchant_emv3ds: Option<EmvThreedsData>,
    ds_merchant_transactiontype: RedsysTransactionType,
    ds_merchant_currency: String,
    ds_merchant_pan: cards::CardNumber,
    ds_merchant_merchantcode: Secret<String>,
    ds_merchant_terminal: Secret<String>,
    ds_merchant_order: String,
    ds_merchant_amount: StringMinorUnit,
    ds_merchant_expirydate: Secret<String>,
    ds_merchant_cvv2: Secret<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EmvThreedsData {
    three_d_s_info: RedsysThreeDsInfo,
    protocol_version: Option<String>,
    browser_accept_header: Option<String>,
    browser_user_agent: Option<String>,
    browser_java_enabled: Option<bool>,
    browser_javascript_enabled: Option<bool>,
    browser_language: Option<String>,
    browser_color_depth: Option<String>,
    browser_screen_height: Option<String>,
    browser_screen_width: Option<String>,
    browser_t_z: Option<String>,
    browser_i_p: Option<Secret<String, common_utils::pii::IpAddress>>,
    three_d_s_server_trans_i_d: Option<String>,
    notification_u_r_l: Option<String>,
    three_d_s_comp_ind: Option<ThreeDSCompInd>,
    cres: Option<String>,
    #[serde(flatten)]
    billing_data: Option<BillingData>,
    #[serde(flatten)]
    shipping_data: Option<ShippingData>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BillingData {
    bill_addr_city: Option<String>,
    bill_addr_country: Option<String>,
    bill_addr_line1: Option<Secret<String>>,
    bill_addr_line2: Option<Secret<String>>,
    bill_addr_line3: Option<Secret<String>>,
    bill_addr_postal_code: Option<Secret<String>>,
    bill_addr_state: Option<Secret<String>>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ShippingData {
    ship_addr_city: Option<String>,
    ship_addr_country: Option<String>,
    ship_addr_line1: Option<Secret<String>>,
    ship_addr_line2: Option<Secret<String>>,
    ship_addr_line3: Option<Secret<String>>,
    ship_addr_postal_code: Option<Secret<String>>,
    ship_addr_state: Option<Secret<String>>,
}

impl EmvThreedsData {
    pub fn new(three_d_s_info: RedsysThreeDsInfo) -> Self {
        Self {
            three_d_s_info,
            protocol_version: None,
            browser_accept_header: None,
            browser_user_agent: None,
            browser_java_enabled: None,
            browser_javascript_enabled: None,
            browser_language: None,
            browser_color_depth: None,
            browser_screen_height: None,
            browser_screen_width: None,
            browser_t_z: None,
            browser_i_p: None,
            three_d_s_server_trans_i_d: None,
            notification_u_r_l: None,
            three_d_s_comp_ind: None,
            cres: None,
            billing_data: None,
            shipping_data: None,
        }
    }

    pub fn add_browser_data(mut self, browser_info: BrowserInformation) -> Result<Self, Error> {
        self.browser_accept_header = Some(browser_info.get_accept_header()?);
        self.browser_user_agent = Some(browser_info.get_user_agent()?);
        self.browser_java_enabled = Some(browser_info.get_java_enabled()?);
        self.browser_javascript_enabled = browser_info.get_java_script_enabled().ok();
        self.browser_language = Some(browser_info.get_language()?);
        self.browser_color_depth = Some(browser_info.get_color_depth()?.to_string());
        self.browser_screen_height = Some(browser_info.get_screen_height()?.to_string());
        self.browser_screen_width = Some(browser_info.get_screen_width()?.to_string());
        self.browser_t_z = Some(browser_info.get_time_zone()?.to_string());
        self.browser_i_p = Some(browser_info.get_ip_address()?);
        Ok(self)
    }

    pub fn set_three_d_s_server_trans_i_d(mut self, three_d_s_server_trans_i_d: String) -> Self {
        self.three_d_s_server_trans_i_d = Some(three_d_s_server_trans_i_d);
        self
    }

    pub fn set_protocol_version(mut self, protocol_version: String) -> Self {
        self.protocol_version = Some(protocol_version);
        self
    }

    pub fn set_notification_u_r_l(mut self, notification_u_r_l: String) -> Self {
        self.notification_u_r_l = Some(notification_u_r_l);
        self
    }

    pub fn set_three_d_s_comp_ind(mut self, three_d_s_comp_ind: ThreeDSCompInd) -> Self {
        self.three_d_s_comp_ind = Some(three_d_s_comp_ind);
        self
    }

    pub fn set_three_d_s_cres(mut self, cres: String) -> Self {
        self.cres = Some(cres);
        self
    }

    fn get_state_code(state: Secret<String>) -> Result<Secret<String>, Error> {
        let state = connector_utils::normalize_string(state.expose())
            .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        let addr_state_value = if state.len() > 3 {
            let addr_state = match state.as_str() {
                "acoruna" | "lacoruna" | "esc" => Ok("C"),
                "alacant" | "esa" | "alicante" => Ok("A"),
                "albacete" | "esab" => Ok("AB"),
                "almeria" | "esal" => Ok("AL"),
                "andalucia" | "esan" => Ok("AN"),
                "araba" | "esvi" => Ok("VI"),
                "aragon" | "esar" => Ok("AR"),
                "asturias" | "eso" => Ok("O"),
                "asturiasprincipadode" | "principadodeasturias" | "esas" => Ok("AS"),
                "badajoz" | "esba" => Ok("BA"),
                "barcelona" | "esb" => Ok("B"),
                "bizkaia" | "esbi" => Ok("BI"),
                "burgos" | "esbu" => Ok("BU"),
                "canarias" | "escn" => Ok("CN"),
                "cantabria" | "ess" => Ok("S"),
                "castello" | "escs" => Ok("CS"),
                "castellon" => Ok("C"),
                "castillayleon" | "escl" => Ok("CL"),
                "castillalamancha" | "escm" => Ok("CM"),
                "cataluna" | "catalunya" | "esct" => Ok("CT"),
                "ceuta" | "esce" => Ok("CE"),
                "ciudadreal" | "escr" | "ciudad" => Ok("CR"),
                "cuenca" | "escu" => Ok("CU"),
                "caceres" | "escc" => Ok("CC"),
                "cadiz" | "esca" => Ok("CA"),
                "cordoba" | "esco" => Ok("CO"),
                "euskalherria" | "espv" => Ok("PV"),
                "extremadura" | "esex" => Ok("EX"),
                "galicia" | "esga" => Ok("GA"),
                "gipuzkoa" | "esss" => Ok("SS"),
                "girona" | "esgi" | "gerona" => Ok("GI"),
                "granada" | "esgr" => Ok("GR"),
                "guadalajara" | "esgu" => Ok("GU"),
                "huelva" | "esh" => Ok("H"),
                "huesca" | "eshu" => Ok("HU"),
                "illesbalears" | "islasbaleares" | "espm" => Ok("PM"),
                "esib" => Ok("IB"),
                "jaen" | "esj" => Ok("J"),
                "larioja" | "eslo" => Ok("LO"),
                "esri" => Ok("RI"),
                "laspalmas" | "palmas" | "esgc" => Ok("GC"),
                "leon" => Ok("LE"),
                "lleida" | "lerida" | "esl" => Ok("L"),
                "lugo" | "eslu" => Ok("LU"),
                "madrid" | "esm" => Ok("M"),
                "comunidaddemadrid" | "madridcomunidadde" | "esmd" => Ok("MD"),
                "melilla" | "esml" => Ok("ML"),
                "murcia" | "esmu" => Ok("MU"),
                "murciaregionde" | "regiondemurcia" | "esmc" => Ok("MC"),
                "malaga" | "esma" => Ok("MA"),
                "nafarroa" | "esnc" => Ok("NC"),
                "nafarroakoforukomunitatea" | "esna" => Ok("NA"),
                "navarra" => Ok("NA"),
                "navarracomunidadforalde" | "comunidadforaldenavarra" => Ok("NC"),
                "ourense" | "orense" | "esor" => Ok("OR"),
                "palencia" | "esp" => Ok("P"),
                "paisvasco" => Ok("PV"),
                "pontevedra" | "espo" => Ok("PO"),
                "salamanca" | "essa" => Ok("SA"),
                "santacruzdetenerife" | "estf" => Ok("TF"),
                "segovia" | "essg" => Ok("SG"),
                "sevilla" | "esse" => Ok("SE"),
                "soria" | "esso" => Ok("SO"),
                "tarragona" | "est" => Ok("T"),
                "teruel" | "este" => Ok("TE"),
                "toledo" | "esto" => Ok("TO"),
                "valencia" | "esv" => Ok("V"),
                "valencianacomunidad" | "esvc" => Ok("VC"),
                "valencianacomunitat" => Ok("V"),
                "valladolid" | "esva" => Ok("VA"),
                "zamora" | "esza" => Ok("ZA"),
                "zaragoza" | "esz" => Ok("Z"),
                "alava" => Ok("VI"),
                "avila" | "esav" => Ok("AV"),
                _ => Err(errors::ConnectorError::InvalidDataFormat {
                    field_name: "address.state",
                }),
            }?;
            addr_state.to_string()
        } else {
            state.to_string()
        };
        Ok(Secret::new(addr_state_value))
    }

    pub fn set_billing_data(mut self, address: Option<&Address>) -> Result<Self, Error> {
        self.billing_data = address
            .and_then(|address| {
                address.address.as_ref().map(|address_details| {
                    let state = address_details
                        .get_optional_state()
                        .map(Self::get_state_code)
                        .transpose();

                    match state {
                        Ok(bill_addr_state) => Ok(BillingData {
                            bill_addr_city: address_details.get_optional_city(),
                            bill_addr_country: address_details.get_optional_country().map(
                                |country| {
                                    common_enums::CountryAlpha2::from_alpha2_to_alpha3(country)
                                        .to_string()
                                },
                            ),
                            bill_addr_line1: address_details.get_optional_line1(),
                            bill_addr_line2: address_details.get_optional_line2(),
                            bill_addr_line3: address_details.get_optional_line3(),
                            bill_addr_postal_code: address_details.get_optional_zip(),
                            bill_addr_state,
                        }),
                        Err(err) => Err(err),
                    }
                })
            })
            .transpose()?;
        Ok(self)
    }
    pub fn set_shipping_data(mut self, address: Option<&Address>) -> Result<Self, Error> {
        self.shipping_data = address
            .and_then(|address| {
                address.address.as_ref().map(|address_details| {
                    let state = address_details
                        .get_optional_state()
                        .map(Self::get_state_code)
                        .transpose();
                    match state {
                        Ok(ship_addr_state) => Ok(ShippingData {
                            ship_addr_city: address_details.get_optional_city(),
                            ship_addr_country: address_details.get_optional_country().map(
                                |country| {
                                    common_enums::CountryAlpha2::from_alpha2_to_alpha3(country)
                                        .to_string()
                                },
                            ),
                            ship_addr_line1: address_details.get_optional_line1(),
                            ship_addr_line2: address_details.get_optional_line2(),
                            ship_addr_line3: address_details.get_optional_line3(),
                            ship_addr_postal_code: address_details.get_optional_zip(),
                            ship_addr_state,
                        }),
                        Err(err) => Err(err),
                    }
                })
            })
            .transpose()?;
        Ok(self)
    }
}

#[derive(Debug)]
pub struct RedsysCardData {
    card_number: cards::CardNumber,
    expiry_date: Secret<String>,
    cvv2: Secret<String>,
}

impl TryFrom<&Option<PaymentMethodData>> for RedsysCardData {
    type Error = Error;
    fn try_from(payment_method_data: &Option<PaymentMethodData>) -> Result<Self, Self::Error> {
        match payment_method_data {
            Some(PaymentMethodData::Card(card)) => {
                let year = card.get_card_expiry_year_2_digit()?.expose();
                let month = card.get_card_expiry_month_2_digit()?.expose();
                let expiry_date = Secret::new(format!("{year}{month}"));
                Ok(Self {
                    card_number: card.card_number.clone(),
                    expiry_date,
                    cvv2: card.card_cvc.clone(),
                })
            }
            Some(PaymentMethodData::Wallet(..))
            | Some(PaymentMethodData::PayLater(..))
            | Some(PaymentMethodData::BankDebit(..))
            | Some(PaymentMethodData::BankRedirect(..))
            | Some(PaymentMethodData::BankTransfer(..))
            | Some(PaymentMethodData::Crypto(..))
            | Some(PaymentMethodData::MandatePayment)
            | Some(PaymentMethodData::GiftCard(..))
            | Some(PaymentMethodData::Voucher(..))
            | Some(PaymentMethodData::CardRedirect(..))
            | Some(PaymentMethodData::Reward)
            | Some(PaymentMethodData::RealTimePayment(..))
            | Some(PaymentMethodData::MobilePayment(..))
            | Some(PaymentMethodData::Upi(..))
            | Some(PaymentMethodData::OpenBanking(_))
            | Some(PaymentMethodData::CardToken(..))
            | Some(PaymentMethodData::NetworkToken(..))
            | Some(PaymentMethodData::CardDetailsForNetworkTransactionId(_))
            | None => Err(errors::ConnectorError::NotImplemented(
                connector_utils::get_unimplemented_payment_method_error_message("redsys"),
            )
            .into()),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum RedsysThreeDsInfo {
    CardData,
    CardConfiguration,
    ChallengeRequest,
    ChallengeResponse,
    AuthenticationData,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum RedsysTransactionType {
    #[serde(rename = "0")]
    Payment,
    #[serde(rename = "1")]
    Preauthorization,
    #[serde(rename = "2")]
    Confirmation,
    #[serde(rename = "3")]
    Refund,
    #[serde(rename = "9")]
    Cancellation,
}

pub struct RedsysAuthType {
    pub(super) merchant_id: Secret<String>,
    pub(super) terminal_id: Secret<String>,
    pub(super) sha256_pwd: Secret<String>,
}

impl TryFrom<&ConnectorAuthType> for RedsysAuthType {
    type Error = Error;
    fn try_from(auth_type: &ConnectorAuthType) -> Result<Self, Self::Error> {
        if let ConnectorAuthType::SignatureKey {
            api_key,
            key1,
            api_secret,
        } = auth_type
        {
            Ok(Self {
                merchant_id: api_key.to_owned(),
                terminal_id: key1.to_owned(),
                sha256_pwd: api_secret.to_owned(),
            })
        } else {
            Err(errors::ConnectorError::FailedToObtainAuthType)?
        }
    }
}

impl TryFrom<&RedsysRouterData<&PaymentsPreProcessingRouterData>> for RedsysTransaction {
    type Error = Error;
    fn try_from(
        item: &RedsysRouterData<&PaymentsPreProcessingRouterData>,
    ) -> Result<Self, Self::Error> {
        let auth = RedsysAuthType::try_from(&item.router_data.connector_auth_type)?;
        if !item.router_data.is_three_ds() {
            Err(errors::ConnectorError::NotSupported {
                message: "PreProcessing flow for no-3ds cards".to_string(),
                connector: "redsys",
            })?
        };
        let redsys_preprocessing_request = if item.router_data.auth_type
            == enums::AuthenticationType::ThreeDs
        {
            let ds_merchant_emv3ds = Some(EmvThreedsData::new(RedsysThreeDsInfo::CardData));
            let ds_merchant_transactiontype = if item.router_data.request.is_auto_capture()? {
                RedsysTransactionType::Payment
            } else {
                RedsysTransactionType::Preauthorization
            };

            let ds_merchant_order = if item.router_data.connector_request_reference_id.len() <= 12 {
                Ok(item.router_data.connector_request_reference_id.clone())
            } else {
                Err(errors::ConnectorError::MaxFieldLengthViolated {
                    connector: "Redsys".to_string(),
                    field_name: "ds_merchant_order".to_string(),
                    max_length: 12,
                    received_length: item.router_data.connector_request_reference_id.len(),
                })
            }?;

            let card_data =
                RedsysCardData::try_from(&item.router_data.request.payment_method_data)?;
            Ok(PaymentsRequest {
                ds_merchant_emv3ds,
                ds_merchant_transactiontype,
                ds_merchant_currency: item.currency.iso_4217().to_owned(),
                ds_merchant_pan: card_data.card_number,
                ds_merchant_merchantcode: auth.merchant_id.clone(),
                ds_merchant_terminal: auth.terminal_id.clone(),
                ds_merchant_order,
                ds_merchant_amount: item.amount.clone(),
                ds_merchant_expirydate: card_data.expiry_date,
                ds_merchant_cvv2: card_data.cvv2,
            })
        } else {
            Err(errors::ConnectorError::FlowNotSupported {
                flow: "PreProcessing".to_string(),
                connector: "redsys".to_string(),
            })
        }?;
        router_env::logger::info!(connector_preprocessing_request=?redsys_preprocessing_request);

        Self::try_from((&redsys_preprocessing_request, &auth))
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(untagged)]
pub enum RedsysResponse {
    RedsysResponse(RedsysTransaction),
    RedsysErrorResponse(RedsysErrorResponse),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct RedsysErrorResponse {
    pub error_code: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CardPSD2 {
    Y,
    N,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ThreeDSCompInd {
    Y,
    N,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RedsysPaymentsResponse {
    #[serde(rename = "Ds_Order")]
    ds_order: String,
    #[serde(rename = "Ds_EMV3DS")]
    ds_emv3ds: Option<RedsysEmv3DSData>,
    #[serde(rename = "Ds_Card_PSD2")]
    ds_card_psd2: Option<CardPSD2>,
    #[serde(rename = "Ds_Response")]
    ds_response: Option<DsResponse>,
    #[serde(rename = "Ds_AuthorisationCode")]
    ds_authorisation_code: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DsResponse(String);

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RedsysEmv3DSData {
    protocol_version: String,
    three_d_s_server_trans_i_d: Option<String>,
    three_d_s_info: Option<RedsysThreeDsInfo>,
    three_d_s_method_u_r_l: Option<String>,
    acs_u_r_l: Option<String>,
    creq: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThreedsInvokeRequest {
    three_d_s_server_trans_i_d: String,
    three_d_s_method_notification_u_r_l: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RedsysThreeDsInvokeData {
    pub three_ds_method_url: String,
    pub three_ds_method_data: String,
    pub message_version: String,
    pub directory_server_id: String,
    pub three_ds_method_data_submission: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ThreeDsInvokeExempt {
    pub three_d_s_server_trans_i_d: String,
    pub message_version: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RedsysTransaction {
    #[serde(rename = "Ds_SignatureVersion")]
    ds_signature_version: String,
    #[serde(rename = "Ds_MerchantParameters")]
    ds_merchant_parameters: Secret<String>,
    #[serde(rename = "Ds_Signature")]
    ds_signature: Secret<String>,
}

fn to_connector_response_data<T>(connector_response: &str) -> Result<T, Error>
where
    T: serde::de::DeserializeOwned,
{
    let decoded_bytes = BASE64_ENGINE
        .decode(connector_response)
        .change_context(errors::ConnectorError::ResponseDeserializationFailed)
        .attach_printable("Failed to decode Base64")?;

    let response_data: T = serde_json::from_slice(&decoded_bytes)
        .change_context(errors::ConnectorError::ResponseHandlingFailed)?;

    Ok(response_data)
}

impl TryFrom<PaymentsPreprocessingResponseRouterData<RedsysResponse>>
    for PaymentsPreProcessingRouterData
{
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(
        item: PaymentsPreprocessingResponseRouterData<RedsysResponse>,
    ) -> Result<Self, Self::Error> {
        match item.response.clone() {
            RedsysResponse::RedsysResponse(response) => {
                let response_data: RedsysPaymentsResponse =
                    to_connector_response_data(&response.ds_merchant_parameters.clone().expose())?;
                router_env::logger::info!(connector_preprocessing_response=?response_data);
                handle_redsys_preprocessing_response(item, &response_data)
            }
            RedsysResponse::RedsysErrorResponse(response) => {
                let response = Err(ErrorResponse {
                    code: response.error_code.clone(),
                    message: response.error_code.clone(),
                    reason: Some(response.error_code.clone()),
                    status_code: item.http_code,
                    attempt_status: None,
                    connector_transaction_id: None,
                    network_advice_code: None,
                    network_decline_code: None,
                    network_error_message: None,
                    connector_metadata: None,
                });

                Ok(Self {
                    status: enums::AttemptStatus::Failure,
                    response,
                    ..item.data
                })
            }
        }
    }
}

fn handle_redsys_preprocessing_response(
    item: PaymentsPreprocessingResponseRouterData<RedsysResponse>,
    response_data: &RedsysPaymentsResponse,
) -> Result<PaymentsPreProcessingRouterData, error_stack::Report<errors::ConnectorError>> {
    match (
        response_data
            .ds_emv3ds
            .as_ref()
            .and_then(|emv3ds_data| emv3ds_data.three_d_s_method_u_r_l.clone()),
        response_data
            .ds_emv3ds
            .as_ref()
            .and_then(|emv3ds_data| emv3ds_data.three_d_s_server_trans_i_d.clone()),
        response_data
            .ds_emv3ds
            .as_ref()
            .map(|emv3ds_data| emv3ds_data.protocol_version.clone()),
    ) {
        (
            Some(three_d_s_method_u_r_l),
            Some(three_d_s_server_trans_i_d),
            Some(protocol_version),
        ) => handle_threeds_invoke(
            item,
            response_data,
            three_d_s_method_u_r_l,
            three_d_s_server_trans_i_d,
            protocol_version,
        ),
        (None, Some(three_d_s_server_trans_i_d), Some(protocol_version)) => {
            handle_threeds_invoke_exempt(
                item,
                response_data,
                three_d_s_server_trans_i_d,
                protocol_version,
            )
        }
        _ => Err(errors::ConnectorError::NotSupported {
            message: "3DS payment with a non-3DS card".to_owned(),
            connector: "redsys",
        }
        .into()),
    }
}

fn handle_threeds_invoke(
    item: PaymentsPreprocessingResponseRouterData<RedsysResponse>,
    response_data: &RedsysPaymentsResponse,
    three_d_s_method_u_r_l: String,
    three_d_s_server_trans_i_d: String,
    protocol_version: String,
) -> Result<PaymentsPreProcessingRouterData, error_stack::Report<errors::ConnectorError>> {
    let three_d_s_method_notification_u_r_l = item.data.request.get_webhook_url()?;

    let threeds_invoke_data = ThreedsInvokeRequest {
        three_d_s_server_trans_i_d: three_d_s_method_u_r_l.clone(),
        three_d_s_method_notification_u_r_l,
    };

    let three_ds_data_string = threeds_invoke_data
        .encode_to_string_of_json()
        .change_context(errors::ConnectorError::RequestEncodingFailed)?;

    let three_ds_method_data = BASE64_ENGINE.encode(&three_ds_data_string);

    let three_ds_data = RedsysThreeDsInvokeData {
        three_ds_method_url: three_d_s_method_u_r_l,
        three_ds_method_data,
        message_version: protocol_version.clone(),
        directory_server_id: three_d_s_server_trans_i_d,
        three_ds_method_data_submission: true,
    };

    let connector_metadata = Some(
        serde_json::to_value(&three_ds_data)
            .change_context(errors::ConnectorError::RequestEncodingFailed)
            .attach_printable("Failed to serialize ThreeDsData")?,
    );

    Ok(RouterData {
        status: enums::AttemptStatus::AuthenticationPending,
        response: Ok(PaymentsResponseData::TransactionResponse {
            resource_id: ResponseId::ConnectorTransactionId(response_data.ds_order.clone()),
            redirection_data: Box::new(None),
            mandate_reference: Box::new(None),
            connector_metadata,
            network_txn_id: None,
            connector_response_reference_id: Some(response_data.ds_order.clone()),
            incremental_authorization_allowed: None,
            charges: None,
        }),
        ..item.data
    })
}

fn handle_threeds_invoke_exempt(
    item: PaymentsPreprocessingResponseRouterData<RedsysResponse>,
    response_data: &RedsysPaymentsResponse,
    three_d_s_server_trans_i_d: String,
    protocol_version: String,
) -> Result<PaymentsPreProcessingRouterData, error_stack::Report<errors::ConnectorError>> {
    let three_ds_data = ThreeDsInvokeExempt {
        message_version: protocol_version.clone(),
        three_d_s_server_trans_i_d,
    };

    let connector_metadata = Some(
        serde_json::to_value(&three_ds_data)
            .change_context(errors::ConnectorError::RequestEncodingFailed)
            .attach_printable("Failed to serialize ThreeDsData")?,
    );

    Ok(RouterData {
        status: enums::AttemptStatus::AuthenticationPending,
        response: Ok(PaymentsResponseData::TransactionResponse {
            resource_id: ResponseId::ConnectorTransactionId(response_data.ds_order.clone()),
            redirection_data: Box::new(None),
            mandate_reference: Box::new(None),
            connector_metadata,
            network_txn_id: None,
            connector_response_reference_id: Some(response_data.ds_order.clone()),
            incremental_authorization_allowed: None,
            charges: None,
        }),
        ..item.data
    })
}

fn des_encrypt(
    message: &str,
    key: &str,
) -> Result<Vec<u8>, error_stack::Report<errors::ConnectorError>> {
    // Connector decrypts the signature using an initialization vector (IV) set to all zeros
    let iv_array = [0u8; common_utils::crypto::TripleDesEde3CBC::TRIPLE_DES_IV_LENGTH];
    let iv = iv_array.to_vec();
    let key_bytes = BASE64_ENGINE
        .decode(key)
        .change_context(errors::ConnectorError::RequestEncodingFailed)
        .attach_printable("Base64 decoding failed")?;
    let triple_des =
        common_utils::crypto::TripleDesEde3CBC::new(Some(enums::CryptoPadding::ZeroPadding), iv)
            .change_context(errors::ConnectorError::RequestEncodingFailed)
            .attach_printable("Triple DES encryption failed")?;
    let encrypted = triple_des
        .encode_message(&key_bytes, message.as_bytes())
        .change_context(errors::ConnectorError::RequestEncodingFailed)
        .attach_printable("Triple DES encryption failed")?;
    let expected_len =
        encrypted.len() - common_utils::crypto::TripleDesEde3CBC::TRIPLE_DES_IV_LENGTH;
    let encrypted_trimmed = encrypted
        .get(..expected_len)
        .ok_or(errors::ConnectorError::RequestEncodingFailed)
        .attach_printable("Failed to trim encrypted data to the expected length")?;
    Ok(encrypted_trimmed.to_vec())
}

fn get_signature(
    order_id: &str,
    params: &str,
    key: &str,
) -> Result<String, error_stack::Report<errors::ConnectorError>> {
    let secret_ko = des_encrypt(order_id, key)?;
    let result = common_utils::crypto::HmacSha256::sign_message(
        &common_utils::crypto::HmacSha256,
        &secret_ko,
        params.as_bytes(),
    )
    .map_err(|_| errors::ConnectorError::RequestEncodingFailed)?;
    let encoded = BASE64_ENGINE.encode(result);
    Ok(encoded)
}

pub trait SignatureCalculationData {
    fn get_merchant_parameters(&self) -> Result<String, Error>;
    fn get_order_id(&self) -> String;
}

impl SignatureCalculationData for PaymentsRequest {
    fn get_merchant_parameters(&self) -> Result<String, Error> {
        self.encode_to_string_of_json()
            .change_context(errors::ConnectorError::RequestEncodingFailed)
    }

    fn get_order_id(&self) -> String {
        self.ds_merchant_order.clone()
    }
}

impl SignatureCalculationData for RedsysOperationRequest {
    fn get_merchant_parameters(&self) -> Result<String, Error> {
        self.encode_to_string_of_json()
            .change_context(errors::ConnectorError::RequestEncodingFailed)
    }

    fn get_order_id(&self) -> String {
        self.ds_merchant_order.clone()
    }
}

impl<T> TryFrom<(&T, &RedsysAuthType)> for RedsysTransaction
where
    T: SignatureCalculationData,
{
    type Error = Error;
    fn try_from(data: (&T, &RedsysAuthType)) -> Result<Self, Self::Error> {
        let (request_data, auth) = data;
        let merchant_parameters = request_data.get_merchant_parameters()?;
        let ds_merchant_parameters = BASE64_ENGINE.encode(&merchant_parameters);
        let sha256_pwd = auth.sha256_pwd.clone().expose();
        let ds_merchant_order = request_data.get_order_id();
        let signature = get_signature(&ds_merchant_order, &ds_merchant_parameters, &sha256_pwd)?;
        Ok(Self {
            ds_signature_version: SIGNATURE_VERSION.to_string(),
            ds_merchant_parameters: Secret::new(ds_merchant_parameters),
            ds_signature: Secret::new(signature),
        })
    }
}

fn get_redsys_attempt_status(
    ds_response: DsResponse,
    capture_method: Option<enums::CaptureMethod>,
) -> Result<enums::AttemptStatus, error_stack::Report<errors::ConnectorError>> {
    // Redsys consistently provides a 4-digit response code, where numbers ranging from 0000 to 0099 indicate successful transactions
    if ds_response.0.starts_with("00") && ds_response.0.as_str() != "0002" {
        match capture_method {
            Some(enums::CaptureMethod::Automatic) | None => Ok(enums::AttemptStatus::Charged),
            Some(enums::CaptureMethod::Manual) => Ok(enums::AttemptStatus::Authorized),
            _ => Err(errors::ConnectorError::CaptureMethodNotSupported.into()),
        }
    } else {
        match ds_response.0.as_str() {
            "0900" => Ok(enums::AttemptStatus::Charged),
            "0400" | "0481" | "0940" | "9915" => Ok(enums::AttemptStatus::Voided),
            "0950" => Ok(enums::AttemptStatus::VoidFailed),
            "0112" | "0195" | "8210" | "8220" | "9998" | "9999" => {
                Ok(enums::AttemptStatus::AuthenticationPending)
            }
            "0129" | "0184" | "9256" | "9257" => Ok(enums::AttemptStatus::AuthenticationFailed),
            "0107" | "0300" => Ok(enums::AttemptStatus::Pending),
            "0101" | "0102" | "0104" | "0106" | "0110" | "0114" | "0115" | "0116" | "0117"
            | "0118" | "0121" | "0123" | "0125" | "0126" | "0130" | "0162" | "0163" | "0171"
            | "0172" | "0173" | "0174" | "0180" | "0181" | "0182" | "0187" | "0190" | "0191"
            | "0193" | "0201" | "0202" | "0204" | "0206" | "0290" | "0881" | "0904" | "0909"
            | "0912" | "0913" | "0941" | "0944" | "0945" | "0965" | "9912" | "9064" | "9078"
            | "9093" | "9094" | "9104" | "9218" | "9253" | "9261" | "9997" | "0002" => {
                Ok(enums::AttemptStatus::Failure)
            }
            error => Err(errors::ConnectorError::ResponseHandlingFailed)
                .attach_printable(format!("Received Unknown Status:{error}"))?,
        }
    }
}

impl TryFrom<&RedsysRouterData<&PaymentsAuthorizeRouterData>> for RedsysTransaction {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &RedsysRouterData<&PaymentsAuthorizeRouterData>,
    ) -> Result<Self, Self::Error> {
        if !item.router_data.is_three_ds() {
            Err(errors::ConnectorError::NotSupported {
                message: "No-3DS cards".to_string(),
                connector: "redsys",
            })?
        };
        let auth = RedsysAuthType::try_from(&item.router_data.connector_auth_type)?;
        let ds_merchant_transactiontype = if item.router_data.request.is_auto_capture()? {
            RedsysTransactionType::Payment
        } else {
            RedsysTransactionType::Preauthorization
        };
        let card_data =
            RedsysCardData::try_from(&Some(item.router_data.request.payment_method_data.clone()))?;
        let (connector_meta_data, ds_merchant_order) = match &item.router_data.response {
            Ok(PaymentsResponseData::TransactionResponse {
                resource_id: ResponseId::ConnectorTransactionId(order_id),
                connector_metadata,
                ..
            }) => (connector_metadata.clone(), order_id.clone()),
            _ => Err(errors::ConnectorError::ResponseHandlingFailed)?,
        };
        let threeds_meta_data =
            connector_utils::to_connector_meta::<ThreeDsInvokeExempt>(connector_meta_data.clone())?;
        let emv3ds_data = EmvThreedsData::new(RedsysThreeDsInfo::AuthenticationData)
            .set_three_d_s_server_trans_i_d(threeds_meta_data.three_d_s_server_trans_i_d)
            .set_protocol_version(threeds_meta_data.message_version)
            .set_notification_u_r_l(item.router_data.request.get_complete_authorize_url()?)
            .add_browser_data(item.router_data.request.get_browser_info()?)?
            .set_three_d_s_comp_ind(ThreeDSCompInd::N)
            .set_billing_data(item.router_data.get_optional_billing())?
            .set_shipping_data(item.router_data.get_optional_shipping())?;

        let payment_authorize_request = PaymentsRequest {
            ds_merchant_emv3ds: Some(emv3ds_data),
            ds_merchant_transactiontype,
            ds_merchant_currency: item.currency.iso_4217().to_owned(),
            ds_merchant_pan: card_data.card_number,
            ds_merchant_merchantcode: auth.merchant_id.clone(),
            ds_merchant_terminal: auth.terminal_id.clone(),
            ds_merchant_order,
            ds_merchant_amount: item.amount.clone(),
            ds_merchant_expirydate: card_data.expiry_date,
            ds_merchant_cvv2: card_data.cvv2,
        };
        router_env::logger::info!(connector_authorize_request=?payment_authorize_request);
        Self::try_from((&payment_authorize_request, &auth))
    }
}

fn build_threeds_form(ds_emv3ds: &RedsysEmv3DSData) -> Result<RedirectForm, Error> {
    let creq = ds_emv3ds
        .creq
        .clone()
        .ok_or(errors::ConnectorError::ResponseDeserializationFailed)?;

    let endpoint = ds_emv3ds
        .acs_u_r_l
        .clone()
        .ok_or(errors::ConnectorError::ResponseDeserializationFailed)?;

    let mut form_fields = std::collections::HashMap::new();
    form_fields.insert("creq".to_string(), creq);

    Ok(RedirectForm::Form {
        endpoint,
        method: common_utils::request::Method::Post,
        form_fields,
    })
}

impl<F> TryFrom<ResponseRouterData<F, RedsysResponse, PaymentsAuthorizeData, PaymentsResponseData>>
    for RouterData<F, PaymentsAuthorizeData, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, RedsysResponse, PaymentsAuthorizeData, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        let (response, status) = match item.response.clone() {
            RedsysResponse::RedsysResponse(transaction_response) => {
                let connector_metadata = match item.data.response {
                    Ok(PaymentsResponseData::TransactionResponse {
                        connector_metadata, ..
                    }) => connector_metadata,
                    _ => None,
                };
                let response_data: RedsysPaymentsResponse = to_connector_response_data(
                    &transaction_response.ds_merchant_parameters.clone().expose(),
                )?;
                router_env::logger::info!(connector_authorize_response=?response_data);
                get_payments_response(
                    response_data,
                    item.data.request.capture_method,
                    connector_metadata,
                    item.http_code,
                )?
            }
            RedsysResponse::RedsysErrorResponse(response) => {
                let response = Err(ErrorResponse {
                    code: response.error_code.clone(),
                    message: response.error_code.clone(),
                    reason: Some(response.error_code.clone()),
                    status_code: item.http_code,
                    attempt_status: None,
                    connector_transaction_id: None,
                    network_advice_code: None,
                    network_decline_code: None,
                    network_error_message: None,
                    connector_metadata: None,
                });

                (response, enums::AttemptStatus::Failure)
            }
        };
        Ok(Self {
            status,
            response,
            ..item.data
        })
    }
}
#[derive(Debug, Serialize, Deserialize)]
pub struct ThreedsChallengeResponse {
    cres: String,
}

impl TryFrom<&RedsysRouterData<&PaymentsCompleteAuthorizeRouterData>> for RedsysTransaction {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &RedsysRouterData<&PaymentsCompleteAuthorizeRouterData>,
    ) -> Result<Self, Self::Error> {
        if !item.router_data.is_three_ds() {
            Err(errors::ConnectorError::NotSupported {
                message: "PaymentsComplete flow for no-3ds cards".to_string(),
                connector: "redsys",
            })?
        };
        let card_data =
            RedsysCardData::try_from(&item.router_data.request.payment_method_data.clone())?;
        let auth = RedsysAuthType::try_from(&item.router_data.connector_auth_type)?;
        let redirect_response = item
            .router_data
            .request
            .get_redirect_response_payload()
            .ok()
            .clone()
            .map(|payload_data| {
                payload_data
                    .parse_value::<ThreedsChallengeResponse>("Redsys ThreedsChallengeResponse")
                    .change_context(errors::ConnectorError::ResponseDeserializationFailed)
            })
            .transpose()?;
        let billing_data = item.router_data.get_optional_billing();
        let shipping_data = item.router_data.get_optional_shipping();

        let emv3ds_data = match redirect_response {
            Some(payload) => {
                if let Ok(threeds_invoke_meta_data) =
                    connector_utils::to_connector_meta::<RedsysThreeDsInvokeData>(
                        item.router_data.request.connector_meta.clone(),
                    )
                {
                    EmvThreedsData::new(RedsysThreeDsInfo::ChallengeResponse)
                        .set_protocol_version(threeds_invoke_meta_data.message_version)
                        .set_three_d_s_cres(payload.cres)
                        .set_billing_data(billing_data)?
                        .set_shipping_data(shipping_data)?
                } else if let Ok(threeds_meta_data) =
                    connector_utils::to_connector_meta::<ThreeDsInvokeExempt>(
                        item.router_data.request.connector_meta.clone(),
                    )
                {
                    EmvThreedsData::new(RedsysThreeDsInfo::ChallengeResponse)
                        .set_protocol_version(threeds_meta_data.message_version)
                        .set_three_d_s_cres(payload.cres)
                        .set_billing_data(billing_data)?
                        .set_shipping_data(shipping_data)?
                } else {
                    Err(errors::ConnectorError::RequestEncodingFailed)?
                }
            }
            None => {
                if let Ok(threeds_invoke_meta_data) =
                    connector_utils::to_connector_meta::<RedsysThreeDsInvokeData>(
                        item.router_data.request.connector_meta.clone(),
                    )
                {
                    let three_d_s_comp_ind = ThreeDSCompInd::from(
                        item.router_data.request.get_threeds_method_comp_ind()?,
                    );
                    let browser_info = item.router_data.request.get_browser_info()?;
                    let complete_authorize_url =
                        item.router_data.request.get_complete_authorize_url()?;
                    EmvThreedsData::new(RedsysThreeDsInfo::AuthenticationData)
                        .set_three_d_s_server_trans_i_d(
                            threeds_invoke_meta_data.directory_server_id,
                        )
                        .set_protocol_version(threeds_invoke_meta_data.message_version)
                        .set_three_d_s_comp_ind(three_d_s_comp_ind)
                        .add_browser_data(browser_info)?
                        .set_notification_u_r_l(complete_authorize_url)
                        .set_billing_data(billing_data)?
                        .set_shipping_data(shipping_data)?
                } else {
                    Err(errors::ConnectorError::NoConnectorMetaData)?
                }
            }
        };

        let ds_merchant_transactiontype = if item.router_data.request.is_auto_capture()? {
            RedsysTransactionType::Payment
        } else {
            RedsysTransactionType::Preauthorization
        };
        let ds_merchant_order = item
            .router_data
            .request
            .connector_transaction_id
            .clone()
            .ok_or(errors::ConnectorError::RequestEncodingFailed)
            .attach_printable("Missing connector_transaction_id")?;

        let complete_authorize_response = PaymentsRequest {
            ds_merchant_emv3ds: Some(emv3ds_data),
            ds_merchant_transactiontype,
            ds_merchant_currency: item.currency.iso_4217().to_owned(),
            ds_merchant_pan: card_data.card_number,
            ds_merchant_merchantcode: auth.merchant_id.clone(),
            ds_merchant_terminal: auth.terminal_id.clone(),
            ds_merchant_order,
            ds_merchant_amount: item.amount.clone(),
            ds_merchant_expirydate: card_data.expiry_date,
            ds_merchant_cvv2: card_data.cvv2,
        };
        router_env::logger::info!(connector_complete_authorize_request=?complete_authorize_response);
        Self::try_from((&complete_authorize_response, &auth))
    }
}

impl<F> TryFrom<ResponseRouterData<F, RedsysResponse, CompleteAuthorizeData, PaymentsResponseData>>
    for RouterData<F, CompleteAuthorizeData, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, RedsysResponse, CompleteAuthorizeData, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        let (response, status) = match item.response.clone() {
            RedsysResponse::RedsysResponse(transaction_response) => {
                let response_data: RedsysPaymentsResponse = to_connector_response_data(
                    &transaction_response.ds_merchant_parameters.clone().expose(),
                )?;
                router_env::logger::info!(connector_complete_authorize_response=?response_data);

                get_payments_response(
                    response_data,
                    item.data.request.capture_method,
                    None,
                    item.http_code,
                )?
            }
            RedsysResponse::RedsysErrorResponse(response) => {
                let response = Err(ErrorResponse {
                    code: response.error_code.clone(),
                    message: response.error_code.clone(),
                    reason: Some(response.error_code.clone()),
                    status_code: item.http_code,
                    attempt_status: None,
                    connector_transaction_id: None,
                    network_advice_code: None,
                    network_decline_code: None,
                    network_error_message: None,
                    connector_metadata: None,
                });

                (response, enums::AttemptStatus::Failure)
            }
        };
        Ok(Self {
            status,
            response,
            ..item.data
        })
    }
}

impl From<api_models::payments::ThreeDsCompletionIndicator> for ThreeDSCompInd {
    fn from(threeds_compl_flag: api_models::payments::ThreeDsCompletionIndicator) -> Self {
        match threeds_compl_flag {
            api_models::payments::ThreeDsCompletionIndicator::Success => Self::Y,
            api_models::payments::ThreeDsCompletionIndicator::Failure
            | api_models::payments::ThreeDsCompletionIndicator::NotAvailable => Self::N,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub struct RedsysOperationRequest {
    ds_merchant_order: String,
    ds_merchant_merchantcode: Secret<String>,
    ds_merchant_terminal: Secret<String>,
    ds_merchant_currency: String,
    ds_merchant_transactiontype: RedsysTransactionType,
    ds_merchant_amount: StringMinorUnit,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RedsysOperationsResponse {
    #[serde(rename = "Ds_Order")]
    ds_order: String,
    #[serde(rename = "Ds_Response")]
    ds_response: DsResponse,
    #[serde(rename = "Ds_AuthorisationCode")]
    ds_authorisation_code: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(untagged)]
pub enum RedsysOperationResponse {
    RedsysOperationResponse(RedsysTransaction),
    RedsysOperationsErrorResponse(RedsysErrorResponse),
}

impl TryFrom<&RedsysRouterData<&PaymentsCaptureRouterData>> for RedsysTransaction {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &RedsysRouterData<&PaymentsCaptureRouterData>) -> Result<Self, Self::Error> {
        let auth = RedsysAuthType::try_from(&item.router_data.connector_auth_type)?;
        let redys_capture_request = RedsysOperationRequest {
            ds_merchant_order: item.router_data.request.connector_transaction_id.clone(),
            ds_merchant_merchantcode: auth.merchant_id.clone(),
            ds_merchant_terminal: auth.terminal_id.clone(),
            ds_merchant_currency: item.router_data.request.currency.iso_4217().to_owned(),
            ds_merchant_transactiontype: RedsysTransactionType::Confirmation,
            ds_merchant_amount: item.amount.clone(),
        };
        router_env::logger::info!(connector_capture_request=?redys_capture_request);
        Self::try_from((&redys_capture_request, &auth))
    }
}

impl TryFrom<PaymentsCaptureResponseRouterData<RedsysResponse>> for PaymentsCaptureRouterData {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: PaymentsCaptureResponseRouterData<RedsysResponse>,
    ) -> Result<Self, Self::Error> {
        let (response, status) = match item.response {
            RedsysResponse::RedsysResponse(redsys_transaction_response) => {
                let response_data: RedsysOperationsResponse = to_connector_response_data(
                    &redsys_transaction_response
                        .ds_merchant_parameters
                        .clone()
                        .expose(),
                )?;
                router_env::logger::info!(connector_capture_response=?response_data);
                let status = get_redsys_attempt_status(response_data.ds_response.clone(), None)?;

                let response = if connector_utils::is_payment_failure(status) {
                    Err(ErrorResponse {
                        code: response_data.ds_response.0.clone(),
                        message: response_data.ds_response.0.clone(),
                        reason: Some(response_data.ds_response.0.clone()),
                        status_code: item.http_code,
                        attempt_status: None,
                        connector_transaction_id: Some(response_data.ds_order.clone()),
                        network_advice_code: None,
                        network_decline_code: None,
                        network_error_message: None,
                        connector_metadata: None,
                    })
                } else {
                    Ok(PaymentsResponseData::TransactionResponse {
                        resource_id: ResponseId::ConnectorTransactionId(
                            response_data.ds_order.clone(),
                        ),
                        redirection_data: Box::new(None),
                        mandate_reference: Box::new(None),
                        connector_metadata: None,
                        network_txn_id: None,
                        connector_response_reference_id: Some(response_data.ds_order.clone()),
                        incremental_authorization_allowed: None,
                        charges: None,
                    })
                };
                (response, status)
            }
            RedsysResponse::RedsysErrorResponse(error_response) => {
                let response = Err(ErrorResponse {
                    code: error_response.error_code.clone(),
                    message: error_response.error_code.clone(),
                    reason: Some(error_response.error_code.clone()),
                    status_code: item.http_code,
                    attempt_status: None,
                    connector_transaction_id: None,
                    network_advice_code: None,
                    network_decline_code: None,
                    network_error_message: None,
                    connector_metadata: None,
                });
                (response, enums::AttemptStatus::Failure)
            }
        };
        Ok(Self {
            status,
            response,
            ..item.data
        })
    }
}

impl TryFrom<&RedsysRouterData<&PaymentsCancelRouterData>> for RedsysTransaction {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &RedsysRouterData<&PaymentsCancelRouterData>) -> Result<Self, Self::Error> {
        let auth = RedsysAuthType::try_from(&item.router_data.connector_auth_type)?;
        let redsys_cancel_request = RedsysOperationRequest {
            ds_merchant_order: item.router_data.request.connector_transaction_id.clone(),
            ds_merchant_merchantcode: auth.merchant_id.clone(),
            ds_merchant_terminal: auth.terminal_id.clone(),
            ds_merchant_currency: item.currency.iso_4217().to_owned(),
            ds_merchant_transactiontype: RedsysTransactionType::Cancellation,
            ds_merchant_amount: item.amount.clone(),
        };
        router_env::logger::info!(connector_cancel_request=?redsys_cancel_request);
        Self::try_from((&redsys_cancel_request, &auth))
    }
}

impl<F> TryFrom<&RedsysRouterData<&RefundsRouterData<F>>> for RedsysTransaction {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &RedsysRouterData<&RefundsRouterData<F>>) -> Result<Self, Self::Error> {
        let auth = RedsysAuthType::try_from(&item.router_data.connector_auth_type)?;
        let redsys_refund_request = RedsysOperationRequest {
            ds_merchant_order: item.router_data.request.connector_transaction_id.clone(),
            ds_merchant_merchantcode: auth.merchant_id.clone(),
            ds_merchant_terminal: auth.terminal_id.clone(),
            ds_merchant_currency: item.currency.iso_4217().to_owned(),
            ds_merchant_transactiontype: RedsysTransactionType::Refund,
            ds_merchant_amount: item.amount.clone(),
        };
        router_env::logger::info!(connector_refund_request=?redsys_refund_request);
        Self::try_from((&redsys_refund_request, &auth))
    }
}

impl TryFrom<PaymentsCancelResponseRouterData<RedsysResponse>> for PaymentsCancelRouterData {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: PaymentsCancelResponseRouterData<RedsysResponse>,
    ) -> Result<Self, Self::Error> {
        let (response, status) = match item.response {
            RedsysResponse::RedsysResponse(redsys_transaction_response) => {
                let response_data: RedsysOperationsResponse = to_connector_response_data(
                    &redsys_transaction_response
                        .ds_merchant_parameters
                        .clone()
                        .expose(),
                )?;
                router_env::logger::info!(connector_cancel_response=?response_data);

                let status = get_redsys_attempt_status(response_data.ds_response.clone(), None)?;

                let response = if connector_utils::is_payment_failure(status) {
                    Err(ErrorResponse {
                        code: response_data.ds_response.0.clone(),
                        message: response_data.ds_response.0.clone(),
                        reason: Some(response_data.ds_response.0.clone()),
                        status_code: item.http_code,
                        attempt_status: None,
                        connector_transaction_id: Some(response_data.ds_order.clone()),
                        network_advice_code: None,
                        network_decline_code: None,
                        network_error_message: None,
                        connector_metadata: None,
                    })
                } else {
                    Ok(PaymentsResponseData::TransactionResponse {
                        resource_id: ResponseId::ConnectorTransactionId(
                            response_data.ds_order.clone(),
                        ),
                        redirection_data: Box::new(None),
                        mandate_reference: Box::new(None),
                        connector_metadata: None,
                        network_txn_id: None,
                        connector_response_reference_id: Some(response_data.ds_order.clone()),
                        incremental_authorization_allowed: None,
                        charges: None,
                    })
                };
                (response, status)
            }
            RedsysResponse::RedsysErrorResponse(error_response) => {
                let response = Err(ErrorResponse {
                    code: error_response.error_code.clone(),
                    message: error_response.error_code.clone(),
                    reason: Some(error_response.error_code.clone()),
                    status_code: item.http_code,
                    attempt_status: None,
                    connector_transaction_id: None,
                    network_advice_code: None,
                    network_decline_code: None,
                    network_error_message: None,
                    connector_metadata: None,
                });

                (response, enums::AttemptStatus::VoidFailed)
            }
        };
        Ok(Self {
            status,
            response,
            ..item.data
        })
    }
}

impl TryFrom<DsResponse> for enums::RefundStatus {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(ds_response: DsResponse) -> Result<Self, Self::Error> {
        match ds_response.0.as_str() {
            "0900" => Ok(Self::Success),
            "9999" => Ok(Self::Pending),
            "0950" | "0172" | "174" => Ok(Self::Failure),
            unknown_status => Err(errors::ConnectorError::ResponseHandlingFailed)
                .attach_printable(format!("Received unknown status:{unknown_status}"))?,
        }
    }
}

impl TryFrom<RefundsResponseRouterData<Execute, RedsysResponse>> for RefundsRouterData<Execute> {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: RefundsResponseRouterData<Execute, RedsysResponse>,
    ) -> Result<Self, Self::Error> {
        let response = match item.response {
            RedsysResponse::RedsysResponse(redsys_transaction) => {
                let response_data: RedsysOperationsResponse = to_connector_response_data(
                    &redsys_transaction.ds_merchant_parameters.clone().expose(),
                )?;
                router_env::logger::info!(connector_refund_response=?response_data);

                let refund_status =
                    enums::RefundStatus::try_from(response_data.ds_response.clone())?;

                if connector_utils::is_refund_failure(refund_status) {
                    Err(ErrorResponse {
                        code: response_data.ds_response.0.clone(),
                        message: response_data.ds_response.0.clone(),
                        reason: Some(response_data.ds_response.0.clone()),
                        status_code: item.http_code,
                        attempt_status: None,
                        connector_transaction_id: None,
                        network_advice_code: None,
                        network_decline_code: None,
                        network_error_message: None,
                        connector_metadata: None,
                    })
                } else {
                    Ok(RefundsResponseData {
                        connector_refund_id: response_data.ds_order,
                        refund_status,
                    })
                }
            }
            RedsysResponse::RedsysErrorResponse(redsys_error_response) => Err(ErrorResponse {
                code: redsys_error_response.error_code.clone(),
                message: redsys_error_response.error_code.clone(),
                reason: Some(redsys_error_response.error_code.clone()),
                status_code: item.http_code,
                attempt_status: None,
                connector_transaction_id: None,
                network_advice_code: None,
                network_decline_code: None,
                network_error_message: None,
                connector_metadata: None,
            }),
        };

        Ok(Self {
            response,
            ..item.data
        })
    }
}

fn get_payments_response(
    redsys_payments_response: RedsysPaymentsResponse,
    capture_method: Option<enums::CaptureMethod>,
    connector_metadata: Option<josekit::Value>,
    http_code: u16,
) -> Result<
    (
        Result<PaymentsResponseData, ErrorResponse>,
        enums::AttemptStatus,
    ),
    Error,
> {
    if let Some(ds_response) = redsys_payments_response.ds_response {
        let status = get_redsys_attempt_status(ds_response.clone(), capture_method)?;
        let response = if connector_utils::is_payment_failure(status) {
            Err(ErrorResponse {
                code: ds_response.0.clone(),
                message: ds_response.0.clone(),
                reason: Some(ds_response.0.clone()),
                status_code: http_code,
                attempt_status: None,
                connector_transaction_id: Some(redsys_payments_response.ds_order.clone()),
                network_advice_code: None,
                network_decline_code: None,
                network_error_message: None,
                connector_metadata: None,
            })
        } else {
            Ok(PaymentsResponseData::TransactionResponse {
                resource_id: ResponseId::ConnectorTransactionId(
                    redsys_payments_response.ds_order.clone(),
                ),
                redirection_data: Box::new(None),
                mandate_reference: Box::new(None),
                connector_metadata,
                network_txn_id: None,
                connector_response_reference_id: Some(redsys_payments_response.ds_order.clone()),
                incremental_authorization_allowed: None,
                charges: None,
            })
        };

        Ok((response, status))
    } else {
        let redirection_form = redsys_payments_response
            .ds_emv3ds
            .map(|ds_emv3ds| build_threeds_form(&ds_emv3ds))
            .transpose()?;
        let response = Ok(PaymentsResponseData::TransactionResponse {
            resource_id: ResponseId::ConnectorTransactionId(
                redsys_payments_response.ds_order.clone(),
            ),
            redirection_data: Box::new(redirection_form),
            mandate_reference: Box::new(None),
            connector_metadata,
            network_txn_id: None,
            connector_response_reference_id: Some(redsys_payments_response.ds_order.clone()),
            incremental_authorization_allowed: None,
            charges: None,
        });

        Ok((response, enums::AttemptStatus::AuthenticationPending))
    }
}

#[derive(Debug, Serialize)]
pub struct Messages {
    #[serde(rename = "Version")]
    version: VersionData,
    #[serde(rename = "Signature")]
    signature: String,
    #[serde(rename = "SignatureVersion")]
    signature_version: String,
}

#[derive(Debug, Serialize)]
#[serde(rename = "Version")]
pub struct VersionData {
    #[serde(rename = "@Ds_Version")]
    ds_version: String,
    #[serde(rename = "Message")]
    message: Message,
}

#[derive(Debug, Serialize)]
pub struct Message {
    #[serde(rename = "Transaction")]
    transaction: RedsysSyncRequest,
}

#[derive(Debug, Serialize)]
#[serde(rename = "Transaction")]
pub struct RedsysSyncRequest {
    #[serde(rename = "Ds_MerchantCode")]
    ds_merchant_code: Secret<String>,
    #[serde(rename = "Ds_Terminal")]
    ds_terminal: Secret<String>,
    #[serde(rename = "Ds_Order")]
    ds_order: String,
    #[serde(rename = "Ds_TransactionType")]
    ds_transaction_type: String,
}

fn get_transaction_type(
    status: enums::AttemptStatus,
    capture_method: Option<enums::CaptureMethod>,
) -> Result<String, errors::ConnectorError> {
    match status {
        enums::AttemptStatus::AuthenticationPending
        | enums::AttemptStatus::AuthenticationSuccessful
        | enums::AttemptStatus::Started
        | enums::AttemptStatus::Authorizing
        | enums::AttemptStatus::Authorized
        | enums::AttemptStatus::DeviceDataCollectionPending => match capture_method {
            Some(enums::CaptureMethod::Automatic) | None => {
                Ok(transaction_type::PAYMENT.to_owned())
            }
            Some(enums::CaptureMethod::Manual) => Ok(transaction_type::PREAUTHORIZATION.to_owned()),
            Some(capture_method) => Err(errors::ConnectorError::NotSupported {
                message: capture_method.to_string(),
                connector: "redsys",
            }),
        },
        enums::AttemptStatus::VoidInitiated => Ok(transaction_type::CANCELLATION.to_owned()),
        enums::AttemptStatus::PartialChargedAndChargeable
        | enums::AttemptStatus::CaptureInitiated => Ok(transaction_type::CONFIRMATION.to_owned()),
        enums::AttemptStatus::Pending => match capture_method {
            Some(enums::CaptureMethod::Automatic) | None => {
                Ok(transaction_type::PAYMENT.to_owned())
            }
            Some(enums::CaptureMethod::Manual) => Ok(transaction_type::CONFIRMATION.to_owned()),
            Some(capture_method) => Err(errors::ConnectorError::NotSupported {
                message: capture_method.to_string(),
                connector: "redsys",
            }),
        },
        other_attempt_status => Err(errors::ConnectorError::NotSupported {
            message: format!("Payment sync after terminal status: {other_attempt_status} payment"),
            connector: "redsys",
        }),
    }
}

fn construct_sync_request(
    order_id: String,
    transaction_type: String,
    auth: RedsysAuthType,
) -> Result<Vec<u8>, Error> {
    let transaction_data = RedsysSyncRequest {
        ds_merchant_code: auth.merchant_id,
        ds_terminal: auth.terminal_id,
        ds_transaction_type: transaction_type,
        ds_order: order_id.clone(),
    };
    let version = VersionData {
        ds_version: DS_VERSION.to_owned(),
        message: Message {
            transaction: transaction_data,
        },
    };
    let version_data = quick_xml::se::to_string(&version)
        .change_context(errors::ConnectorError::RequestEncodingFailed)?;

    let signature = get_signature(&order_id, &version_data, auth.sha256_pwd.peek())?;

    let messages = Messages {
        version,
        signature,
        signature_version: SIGNATURE_VERSION.to_owned(),
    };

    let cdata = quick_xml::se::to_string(&messages)
        .change_context(errors::ConnectorError::RequestEncodingFailed)?;

    let body = format!(
        r#"<soapenv:Envelope xmlns:soapenv="{}" xmlns:web="{}"><soapenv:Header/><soapenv:Body><web:consultaOperaciones><cadenaXML><![CDATA[{}]]></cadenaXML></web:consultaOperaciones></soapenv:Body></soapenv:Envelope>"#,
        common_utils::consts::SOAP_ENV_NAMESPACE,
        XMLNS_WEB_URL,
        cdata
    );

    Ok(body.as_bytes().to_vec())
}

pub fn build_payment_sync_request(item: &PaymentsSyncRouterData) -> Result<Vec<u8>, Error> {
    let transaction_type = get_transaction_type(item.status, item.request.capture_method)?;
    let auth = RedsysAuthType::try_from(&item.connector_auth_type)?;
    let connector_transaction_id = item
        .request
        .connector_transaction_id
        .get_connector_transaction_id()
        .change_context(errors::ConnectorError::MissingConnectorTransactionID)?;
    construct_sync_request(connector_transaction_id, transaction_type, auth)
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename = "soapenv:envelope")]
pub struct RedsysSyncResponse {
    #[serde(rename = "@xmlns:soapenv")]
    xmlns_soapenv: String,
    #[serde(rename = "@xmlns:soapenc")]
    xmlns_soapenc: String,
    #[serde(rename = "@xmlns:xsd")]
    xmlns_xsd: String,
    #[serde(rename = "@xmlns:xsi")]
    xmlns_xsi: String,
    #[serde(rename = "header")]
    header: Option<SoapHeader>,
    #[serde(rename = "body")]
    body: SyncResponseBody,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SoapHeader {}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub struct SyncResponseBody {
    consultaoperacionesresponse: ConsultaOperacionesResponse,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub struct ConsultaOperacionesResponse {
    #[serde(rename = "@xmlns:p259")]
    xmlns_p259: String,
    consultaoperacionesreturn: ConsultaOperacionesReturn,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub struct ConsultaOperacionesReturn {
    messages: MessagesResponseData,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub struct MessagesResponseData {
    version: VersionResponseData,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub struct VersionResponseData {
    #[serde(rename = "@ds_version")]
    ds_version: String,
    message: MessageResponseType,
}

// The response will contain either a sync transaction data or error data.
// Since the XML parser does not support enums for this case, we use Option to handle both scenarios.
// If both are present or both are absent, an error is thrown.
#[derive(Debug, Serialize, Deserialize)]
pub struct MessageResponseType {
    response: Option<RedsysSyncResponseData>,
    errormsg: Option<SyncErrorCode>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SyncErrorCode {
    ds_errorcode: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct RedsysSyncResponseData {
    ds_order: String,
    ds_transactiontype: String,
    ds_amount: Option<String>,
    ds_currency: Option<String>,
    ds_securepayment: Option<String>,
    ds_state: Option<String>,
    ds_response: Option<DsResponse>,
}

impl<F> TryFrom<ResponseRouterData<F, RedsysSyncResponse, PaymentsSyncData, PaymentsResponseData>>
    for RouterData<F, PaymentsSyncData, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(
        item: ResponseRouterData<F, RedsysSyncResponse, PaymentsSyncData, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        let message_data = item
            .response
            .body
            .consultaoperacionesresponse
            .consultaoperacionesreturn
            .messages
            .version
            .message;
        let (status, response) = match (message_data.response, message_data.errormsg) {
            (Some(response), None) => {
                if let Some(ds_response) = response.ds_response {
                    let status = get_redsys_attempt_status(
                        ds_response.clone(),
                        item.data.request.capture_method,
                    )?;

                    if connector_utils::is_payment_failure(status) {
                        let payment_response = Err(ErrorResponse {
                            status_code: item.http_code,
                            code: ds_response.0.clone(),
                            message: ds_response.0.clone(),
                            reason: Some(ds_response.0.clone()),
                            attempt_status: None,
                            connector_transaction_id: None,
                            network_advice_code: None,
                            network_decline_code: None,
                            network_error_message: None,
                            connector_metadata: None,
                        });
                        (status, payment_response)
                    } else {
                        let payment_response = Ok(PaymentsResponseData::TransactionResponse {
                            resource_id: ResponseId::ConnectorTransactionId(
                                response.ds_order.clone(),
                            ),
                            redirection_data: Box::new(None),
                            mandate_reference: Box::new(None),
                            connector_metadata: None,
                            network_txn_id: None,
                            connector_response_reference_id: Some(response.ds_order.clone()),
                            incremental_authorization_allowed: None,
                            charges: None,
                        });
                        (status, payment_response)
                    }
                } else {
                    // When the payment is in authentication or still processing
                    let payment_response = Ok(PaymentsResponseData::TransactionResponse {
                        resource_id: ResponseId::ConnectorTransactionId(response.ds_order.clone()),
                        redirection_data: Box::new(None),
                        mandate_reference: Box::new(None),
                        connector_metadata: None,
                        network_txn_id: None,
                        connector_response_reference_id: Some(response.ds_order.clone()),
                        incremental_authorization_allowed: None,
                        charges: None,
                    });

                    (item.data.status, payment_response)
                }
            }
            (None, Some(errormsg)) => {
                let error_code = errormsg.ds_errorcode.clone();
                let response = Err(ErrorResponse {
                    code: error_code.clone(),
                    message: error_code.clone(),
                    reason: Some(error_code),
                    status_code: item.http_code,
                    attempt_status: None,
                    connector_transaction_id: None,
                    network_advice_code: None,
                    network_decline_code: None,
                    network_error_message: None,
                    connector_metadata: None,
                });
                (item.data.status, response)
            }
            (Some(_), Some(_)) | (None, None) => {
                Err(errors::ConnectorError::ResponseHandlingFailed)?
            }
        };

        Ok(Self {
            status,
            response,
            ..item.data
        })
    }
}

pub fn build_refund_sync_request(item: &RefundSyncRouterData) -> Result<Vec<u8>, Error> {
    let transaction_type = transaction_type::REFUND.to_owned();
    let auth = RedsysAuthType::try_from(&item.connector_auth_type)?;
    let connector_transaction_id = item.request.connector_transaction_id.clone();
    construct_sync_request(connector_transaction_id, transaction_type, auth)
}

impl TryFrom<RefundsResponseRouterData<RSync, RedsysSyncResponse>> for RefundsRouterData<RSync> {
    type Error = Error;
    fn try_from(
        item: RefundsResponseRouterData<RSync, RedsysSyncResponse>,
    ) -> Result<Self, Self::Error> {
        let message_data = item
            .response
            .body
            .consultaoperacionesresponse
            .consultaoperacionesreturn
            .messages
            .version
            .message;
        let response = match (message_data.response, message_data.errormsg) {
            (None, Some(errormsg)) => {
                let error_code = errormsg.ds_errorcode.clone();
                Err(ErrorResponse {
                    code: error_code.clone(),
                    message: error_code.clone(),
                    reason: Some(error_code),
                    status_code: item.http_code,
                    attempt_status: None,
                    connector_transaction_id: None,
                    network_advice_code: None,
                    network_decline_code: None,
                    network_error_message: None,
                    connector_metadata: None,
                })
            }
            (Some(response), None) => {
                if let Some(ds_response) = response.ds_response {
                    let refund_status = enums::RefundStatus::try_from(ds_response.clone())?;

                    if connector_utils::is_refund_failure(refund_status) {
                        Err(ErrorResponse {
                            status_code: item.http_code,
                            code: ds_response.0.clone(),
                            message: ds_response.0.clone(),
                            reason: Some(ds_response.0.clone()),
                            attempt_status: None,
                            connector_transaction_id: None,
                            network_advice_code: None,
                            network_decline_code: None,
                            network_error_message: None,
                            connector_metadata: None,
                        })
                    } else {
                        Ok(RefundsResponseData {
                            connector_refund_id: response.ds_order,
                            refund_status,
                        })
                    }
                } else {
                    // When the refund is pending
                    Ok(RefundsResponseData {
                        connector_refund_id: response.ds_order,
                        refund_status: enums::RefundStatus::Pending,
                    })
                }
            }
            (Some(_), Some(_)) | (None, None) => {
                Err(errors::ConnectorError::ResponseHandlingFailed)?
            }
        };

        Ok(Self {
            response,
            ..item.data
        })
    }
}
