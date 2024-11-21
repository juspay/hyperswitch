use common_enums::enums;
use common_utils::types::{MinorUnit};
use hyperswitch_domain_models::{
    payment_method_data::PaymentMethodData,
    router_data::{self, ConnectorAuthType, ErrorResponse, RouterData},
    router_flow_types::{refunds::{Execute, RSync}, Capture},
    router_request_types::{PaymentsAuthorizeData, PaymentsCaptureData, PaymentsSyncData, ResponseId, PaymentsCancelData},
    router_response_types::{PaymentsResponseData, RefundsResponseData},
    types::{PaymentsAuthorizeRouterData, PaymentsCaptureRouterData, PaymentsSyncRouterData, RefundsRouterData, PaymentsCancelRouterData},
};
use hyperswitch_interfaces::errors;
use masking::{ExposeInterface, Secret};
use serde::{Deserialize, Serialize};
use strum::Display;
use crate::{connectors::novalnet::transformers::get_error_response, utils::PaymentsAuthorizeRequestData};
use crate::{
    types::{RefundsResponseRouterData, ResponseRouterData},
    //utils::RequestData,
};

use super::Jpmorgan;
pub struct JpmorganRouterData<T> {
    pub amount: MinorUnit,
    pub router_data: T,
}

impl<T> From<(MinorUnit, T)> for JpmorganRouterData<T> {
    fn from((amount, item): (MinorUnit, T)) -> Self {
        //Todo :  use utils to convert the amount to the type of amount that a connector accepts
        Self {
            amount,
            router_data: item,
        }
    }
}

#[derive(Default, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JpmorganPaymentsRequest {
    capture_method: String,
    amount: MinorUnit,
    currency: String,
    merchant: JpmorganMerchant,
    payment_method_type: JpmorganPaymentMethodType,
}

#[derive(Default, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JpmorganCard {
    account_number: Secret<String>,
    expiry: Expiry,
    is_bill_payment: bool,
}

#[derive(Default, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JpmorganPaymentMethodType {
    card: JpmorganCard,
}

#[derive(Default, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Expiry {
    month: Secret<String>,
    year: Secret<String>,
}

#[derive(Serialize, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JpmorganMerchantSoftware {
    company_name: String,
    product_name: String,
}

#[derive(Default, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JpmorganMerchant {
    merchant_software: JpmorganMerchantSoftware,
}

fn map_capture_method (capture_method : enums::CaptureMethod) -> String {
    match capture_method {
        enums::CaptureMethod::Automatic => String::from("NOW"),
        enums::CaptureMethod::Manual | enums::CaptureMethod::ManualMultiple => String::from("MANUAL"),
        enums::CaptureMethod::Scheduled => String::from("DELAYED"),
    }
}

impl TryFrom<&JpmorganRouterData<&PaymentsAuthorizeRouterData>> for JpmorganPaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &JpmorganRouterData<&PaymentsAuthorizeRouterData>,
    ) -> Result<Self, Self::Error> {
        match item.router_data.request.payment_method_data.clone() {
            PaymentMethodData::Card(req_card) => {

                let capture_method = if let Some(method)=item.router_data.request.capture_method{
                    map_capture_method(method)
                }else{
                    String::from("AUTOMATIC")
                };
                println!("Capture Method : {}",capture_method);

                let currency = item.router_data.request.currency.to_string();
                println!("Currency : {}", currency);

                let merchant_software = JpmorganMerchantSoftware {
                    company_name: String::from("JPMC"),
                    product_name: String::from("Hyperswitch"), //could be Amazon or something else, subject to change
                };
                //hardcoded as of now

                let merchant = JpmorganMerchant { merchant_software };

                let expiry: Expiry = Expiry {
                    month: req_card.card_exp_month,
                    year: req_card.card_exp_year,
                };

                let account_number = Secret::new(req_card.card_number.to_string());

                let card = JpmorganCard {
                    account_number,
                    expiry,
                    is_bill_payment: item.router_data.request.is_auto_capture()?,
                };

                let payment_method_type = JpmorganPaymentMethodType { card };

                Ok(Self {
                    capture_method,
                    currency,
                    amount: item.amount.clone(),
                    merchant,
                    payment_method_type,
                })
            }
            _ => Err(errors::ConnectorError::NotImplemented("Payment method".to_string()).into()),
        }
    }
}

//TODO: Fill the struct with respective fields
// Auth Struct
//in jpm, we get a client id and secret and using these two, we have a curl, we make an api call and we get a access token in res with an expiry time as well
pub struct JpmorganAuthType {
    //pub(super) client_id: Secret<String>,
    //pub(super) client_secret : Secret<String>,
    //pub(super) api_key: Secret<String>,
    pub(super) access_token : Secret<String>,
}

impl TryFrom<&ConnectorAuthType> for JpmorganAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            ConnectorAuthType::HeaderKey { api_key } => Ok(Self {
                access_token: api_key.to_owned(),
            }),
            _ => Err(errors::ConnectorError::FailedToObtainAuthType.into()),
        }
    }
}
// PaymentsResponse
//TODO: Append the remaining status flags
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "UPPERCASE")]
pub enum JpmorganTransactionStatus {
    Success,
    Denied,
    Error,
}

#[derive(Default, Debug, Display, Serialize, Deserialize, Clone)]
#[serde(rename_all="UPPERCASE")]
pub enum JpmorganTransactionState {
    Closed,
    Authorized,
    Voided,
    #[default]
    Pending,
    Declined,
    Error,
}

#[derive(Default, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JpmorganPaymentsResponse {
    transaction_id: String,
    request_id: Option<String>,
    transaction_state: JpmorganTransactionState,
    response_status: String,
    response_code: String,
    response_message: Option<String>,
    payment_method_type: Option<PaymentMethodType>,
    capture_method: Option<String>,
    is_capture: Option<bool>,
    initiator_type: Option<String>,
    account_on_file: Option<String>,
    transaction_date: Option<String>,
    approval_code: Option<String>,
    host_message: Option<String>,
    amount: Option<i64>,
    currency: Option<String>,
    remaining_refundable_amount: Option<i64>,
    remaining_auth_amount: Option<i64>,
    host_reference_id: Option<String>,
    merchant: Option<Merchant>,
}

#[derive(Default, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Merchant {
    merchant_id: Option<String>,
    merchant_software: MerchantSoftware,
    merchant_category_code: Option<String>,
}

#[derive(Default, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MerchantSoftware {
    company_name: String,
    product_name: String,
    version: Option<String>,
}

#[derive(Default, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PaymentMethodType {
    card: Option<Card>,
}

#[derive(Default, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Card {
    expiry: Option<ExpiryResponse>,
    card_type: Option<String>,
    card_type_name: Option<String>,
    is_bill_payment: Option<bool>,
    masked_account_number: Option<String>,
    card_type_indicators: Option<CardTypeIndicators>,
    network_response: Option<NetworkResponse>,
}

#[derive(Default, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NetworkResponse {
    address_verification_result: Option<String>,
    address_verification_result_code: Option<String>,
    card_verification_result_code: Option<String>,
}

#[derive(Default, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExpiryResponse {
    month: Option<i32>,
    year: Option<i32>,
}

#[derive(Default, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CardTypeIndicators {
    issuance_country_code: Option<String>,
    is_durbin_regulated: Option<bool>,
    card_product_types: Vec<String>,
}

pub trait FromTransactionState {
    fn from_transaction_state(transaction_state: String) -> Self;
}

impl FromTransactionState for common_enums::AttemptStatus {
    fn from_transaction_state(transaction_state: String) -> Self {
        match transaction_state.as_str() {
            "Authorized" => common_enums::AttemptStatus::Authorized,
            "Closed" => common_enums::AttemptStatus::Charged,
            "Declined" | "Error" => common_enums::AttemptStatus::Failure,
            "Pending" => common_enums::AttemptStatus::Pending,
            "Voided" => common_enums::AttemptStatus::Voided,    //subject to change, when doing void/cancel flow
            _ => common_enums::AttemptStatus::Failure, // Default case
        }
    }
}

pub trait FromResponseStatus {
    fn from_response_status(transaction_state: String) -> Self;
}

impl FromResponseStatus for common_enums::AttemptStatus {
    fn from_response_status(transaction_state: String) -> Self {
        match transaction_state.as_str() {
            "Success" => common_enums::AttemptStatus::Voided,    //subject to change, when doing void/cancel flow
            _ => common_enums::AttemptStatus::Failure, // Default case
        }
    }
}

impl From<JpmorganTransactionStatus> for common_enums::AttemptStatus {
    fn from(item: JpmorganTransactionStatus) -> Self {
        match item {
            JpmorganTransactionStatus::Success => Self::Charged,
            //JpmorganTransactionStatus::Pending => Self::Pending,
            JpmorganTransactionStatus::Denied | JpmorganTransactionStatus::Error => Self::Failure,
        }
    }
}

impl<F,T> TryFrom<ResponseRouterData<F, JpmorganPaymentsResponse, T, PaymentsResponseData>>
    for RouterData<F, T, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(
        item: ResponseRouterData<F, JpmorganPaymentsResponse, T, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {

        //let transaction_status = convert_transaction_state(&item.response.response_status);

        //let mut transaction_state = item.response.transaction_state.to_string();
        let mut transaction_state = item.response.transaction_state.to_string();
        println!("###########  BEFORE TRANSACTION STATE {}", transaction_state);

        if transaction_state == String::from("Closed") {
            let cm = item.response.capture_method.clone();
            if cm == Some(String::from("NOW")){
                transaction_state = String::from("Closed");
            } else if cm == Some(String::from("MANUAL")){
                transaction_state = String::from("Authorized")
            } 
        }

        // match item.data.request.is_auto_capture()? {
        //     true => transaction_state = String::from("Closed"),
        //     false => transaction_state = String::from("Authorized")
        // }             

        println!("########### AFTER TRANSACTION STATE {}", transaction_state);

        let status = common_enums::AttemptStatus::from_transaction_state(transaction_state);
        println!("######### STATE as STATUS {}", status);

        let connector_response_reference_id = Some(item.response.transaction_id.clone());

        let resource_id = ResponseId::ConnectorTransactionId(item.response.transaction_id.clone());

        //let connector_response_reference_id = item.response.host_reference_id.clone();

        Ok(Self {
            //status: common_enums::AttemptStatus::from(transaction_status),
            status,
            response: Ok(PaymentsResponseData::TransactionResponse {
                resource_id,
                redirection_data: Box::new(None),
                mandate_reference: Box::new(None),
                connector_metadata: None,
                network_txn_id: None,
                connector_response_reference_id,
                incremental_authorization_allowed: None,
                charge_id: None,
            }),
            ..item.data
        })
    }
}

#[derive(Default, Debug, Serialize, Deserialize)]
#[serde(rename_all="camelCase")]
pub struct JpmorganCaptureRequest {
    capture_method : Option<String>,
    merchant : Option<MerchantCapReq>,
    recurring : Option<RecurringCapReq>,
    installment : Option<InstallmentCapReq>,
    payment_method_type : Option<PaymentMethodTypeCapReq>,
    ship_to : Option<ShipToCapReq>,
    initiator_type : Option<String>,
    account_on_file : Option<String>,
    original_transaction_id : Option<String>,
    is_amount_final : Option<bool>,
    amount : MinorUnit,
    currency : Option<String>,
    merchant_order_number : Option<String>,
    risk : Option<RiskCapReq>,
    retail_addenda : Option<RetailAddendaCapReq>,
    account_holder : Option<AccountHolderCapReq>,
    statement_descriptor : Option<String>,
    partial_authorization_support: Option<String>,
    payment_request_id: Option<String>,
    multi_capture : Option<MultiCapture>,
    sub_merchant_supplemental_data : Option<SubMerchantSupplementalData>,
}

//sub merchant supplemental data starting here 

#[derive(Default, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubMerchantSupplementalData {
    pub custom_data: Option<CustomData>,
    pub service_address: Option<ServiceAddress>,
    pub business_information: Option<BusinessInformation>,
    pub partner_service: Option<PartnerService>,
    pub shipping_info: Option<ShippingInfo>,
    pub recurring_billing: Option<RecurringBilling>,
    pub merchant_reported_revenue: Option<MerchantReportedRevenue>,
    pub order_information: Option<OrderInformation>,
    pub consumer_device: Option<ConsumerDevice>,
    pub merchant_identification: Option<MerchantIdentification>,
}

#[derive(Default, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CustomData{
    pub external_transaction_reference_number: Option<String>,
    pub external_transaction_type: Option<String>,
    pub external_merchant_id: Option<String>,
    pub merchant_order_reference_id: Option<String>,
    pub external_batch_id: Option<String>,
    pub merchant_expected_deposit_date: Option<String>,
}

#[derive(Default, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServiceAddress {
    pub line1: Option<String>,
    pub line2: Option<String>,
    pub city: Option<String>,
    pub state: Option<String>,
    pub postal_code: Option<String>,
    pub country_code: Option<String>,
}

#[derive(Default, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BusinessInformation {
    pub organization_legal_name: Option<String>,
    pub client_business_description_text: Option<String>,
    pub organization_d_b_a_name: Option<String>,
}

#[derive(Default, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PartnerService {
    pub external_vendor_product_name: Option<String>,
    pub currency: Option<String>,
    pub external_monthly_service_fee_amount: Option<i64>,
}

#[derive(Default, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShippingInfo {
    pub shipping_carrier_name: Option<String>,
    pub expected_merchant_product_delivery_date: Option<String>,
}

#[derive(Default, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecurringBilling {
    pub billing_schedule_update_timestamp: Option<String>,
    pub payment_frequency_code: Option<String>,
    pub billing_cycle_sequence_number: Option<String>,
    pub initiator_type: Option<String>,
    pub billing_cycles_total_count: Option<i32>,
}

#[derive(Default, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MerchantReportedRevenue {
    pub amount: Option<i64>,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    pub currency: Option<String>,
    pub amount_type: Option<String>,
}

#[derive(Default, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OrderInformation {
    pub order_items: Option<Vec<OrderItem>>,
    pub receipt_url: Option<String>,
    pub payment_notes: Option<String>,
    pub merchant_url: Option<String>,
    pub terms_url: Option<String>,
}

#[derive(Default, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OrderItem {
    pub merchant_product_identifier: Option<String>,
    pub line_item_description_text: Option<String>,
    pub unit_price_amount: Option<i64>,
    pub line_item_unit_quantity: Option<String>,
    pub item_comodity_code: Option<String>,
    pub chosen_shipping_option: Option<String>,
    pub merchant_campaign_name: Option<String>,
}

#[derive(Default, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConsumerDevice {
    pub session_id: Option<String>,
}

#[derive(Default, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MerchantIdentification {
    pub sub_merchant_id: Option<String>,
    pub service_entitlement_number: Option<String>,
    pub seller_identifier: Option<String>,
}

#[derive(Default, Debug, Serialize, Deserialize)]
#[serde(rename_all="camelCase")]
pub struct MultiCapture {
    multi_capture_sequence_number : Option<String>,
    multi_capture_record_count : Option<i32>,
    is_final_capture : Option<bool>,
}

#[derive(Default, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AccountHolderCapReq {
    reference_id: Option<String>,
    consumer_id_creation_date: String,
    full_name: Option<String>,
    email: Option<String>,
    mobile: Option<PhoneNumber>,
    phone: Option<PhoneNumber>,
    i_p_address: Option<String>,
    billing_address: Option<BillingAddress>,
    national_id: Option<String>,
    first_name: Option<String>,
    last_name: Option<String>,
    middle_name: Option<String>,
    consumer_profile_info: Option<ConsumerProfileInfo>,
}

#[derive(Default, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PhoneNumber {
    country_code: Option<i32>,
    phone_number: String,
}

#[derive(Default, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BillingAddress {
    line1: Option<String>,
    line2: Option<String>,
    city: Option<String>,
    state: Option<String>,
    postal_code: Option<String>,
    country_code: Option<String>,
}

#[derive(Default, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ConsumerProfileInfo {
    consumer_profile_request_type: Option<String>,
    legacy_consumer_profile_id: Option<String>,
    external_consumer_profile_identifier: Option<String>,
}

#[derive(Default, Debug, Serialize, Deserialize)]
#[serde(rename_all="camelCase")]
pub struct RetailAddendaCapReq{
    purchase_order_number : Option<String>,
    order_date : Option<String>,
    tax_amount : Option<i64>,
    is_taxable : Option<bool>,
    level3 : Option<Level3>,
    gratuity_amount : Option<i64>,
    surcharge_amount : Option<i64>,
    health_care_data : Option<HealthCareDataCapReq>,
}

#[derive(Default, Debug, Serialize, Deserialize)]
#[serde(rename_all="camelCase")]
pub struct HealthCareDataCapReq {
    total_healthcare_amount: Option<i64>,
    total_vision_amount: Option<i64>,
    total_clinic_amount: Option<i64>,
    total_dental_amount: Option<i64>,
    total_prescription_amount: Option<i64>,
    is_i_i_a_s: Option<bool>,
}

#[derive(Default, Debug, Serialize, Deserialize)]
#[serde(rename_all="camelCase")]
pub struct Level3{
    total_shipping_amount : Option<i64>,
    duty_amount : Option<i64>,
    ship_to_address_postal_code : Option<String>,
    ship_to_address_country_code : Option<String>,
    ship_from_address_postal_code : Option<String>,
    total_transaction_discount_amount : Option<i64>,
    value_added_tax_amount : Option<i64>,
    value_added_tax_percent : Option<String>,
    shipping_value_added_tax_percent : Option<String>,
    order_discount_treatment_code : Option<String>,
    value_added_tax_invoice_reference_number : Option<String>,
    shipping_value_added_tax_amount : Option<i64>,
    party_tax_government_issued_identifier : Option<String>,
    alternate_tax_amount : Option<i64>,
    line_items : Option<LineItemsCapReq>,
    transaction_advices : Vec<TransactionAdvicesCapReq>,
    tax_treatment_code : Option<String>,
}

#[derive(Default, Debug, Serialize, Deserialize)]
#[serde(rename_all="camelCase")]
pub struct LineItemsCapReq{
    line_item_description_text: Option<String>, 
    merchant_product_identifier: Option<String>, 
    item_commodity_code: Option<String>,
    line_item_unit_quantity: Option<String>,
    line_item_unit_of_measure_code: Option<String>, 
    unit_price_amount: Option<i64>, 
    tax_inclusive_line_item_total_amount: Option<i64>,
    transaction_discount_amount: Option<i64>, 
    purchase_transaction_discount_percent: Option<String>, 
    line_item_discount_treatment_code: Option<String>, 
    line_item_detail_code: Option<String>, 
    line_item_tax_indicator: Option<bool>,
    line_item_discount_indicator: Option<bool>,
    line_item_taxes: Option<Vec<LineItemTaxesCapReq>>, 
}

#[derive(Default, Debug, Serialize, Deserialize)]
#[serde(rename_all="camelCase")]
pub struct LineItemTaxesCapReq{
    tax_type_code : Option<String>,
    line_item_tax_amount : Option<i64>,
    tax_percent : Option<String>,
}

#[derive(Default, Debug, Serialize, Deserialize)]
#[serde(rename_all="camelCase")]
pub struct TransactionAdvicesCapReq{
    
}

#[derive(Default, Debug, Serialize, Deserialize)]
#[serde(rename_all="camelCase")]
pub struct RiskCapReq{
    request_fraud_score : Option<bool>,
    transaction_risk_score : Option<i32>,
    token_risk_score : Option<i32>,
}

#[derive(Default, Debug, Serialize, Deserialize)]
#[serde(rename_all="camelCase")]
pub struct ShipToCapReq{
    shipping_description : Option<String>,
    shipping_address : Option<ShippingAddressCapReq>,
    full_name : Option<String>,
    email : Option<String>,
    mobile : Option<MobileCapReq>,
    phone : Option<PhoneCapReq>,
    first_name : Option<String>,
    last_name : Option<String>,
    middle_name : Option<String>,
}

#[derive(Default, Debug, Serialize, Deserialize)]
#[serde(rename_all="camelCase")]
pub struct PhoneCapReq {
    country_code : Option<i32>,
    phone_number : Option<String>,
}

#[derive(Default, Debug, Serialize, Deserialize)]
#[serde(rename_all="camelCase")]
pub struct MobileCapReq {
    country_code : Option<i32>,
    phone_number : Option<String>,
}

#[derive(Default, Debug, Serialize, Deserialize)]
#[serde(rename_all="camelCase")]
pub struct ShippingAddressCapReq {
    line1 : Option<String>,
    line2 : Option<String>,
    city : Option<String>,
    state : Option<String>,
    postal_code : Option<String>,
    country_code : Option<String>,

}

#[derive(Default, Debug, Serialize, Deserialize)]
#[serde(rename_all="camelCase")]
pub struct PaymentMethodTypeCapReq {
    card : Option<CardCapReq>,
}

#[derive(Default, Debug, Serialize, Deserialize)]
#[serde(rename_all="camelCase")]
pub struct CardCapReq {
    account_number_type : Option<String>,
    account_number : String,
    expiry : Option<ExpiryCapReq>,
    wallet_provider : Option<String>,
    cvv : Option<String>,
    original_network_transaction_id : Option<String>,
    is_bill_payment : Option<bool>,
    account_updater : Option<AccountUpdaterCapReq>,
    authentication : Option<AuthenticationCapReq>,
    encryption_integrity_check : Option<String>,
    preferred_payment_network_name_list : Vec<String>,
    merchant_sales_channel_name : Option<String>,
    merchant_preferred_routing : Option<String>,
    card_type_funding : Option<String>,
    pie_key_id : Option<String>,
    pie_phase_id : Option<String>,
    //payment_authentication_request : Option<PaymentAuthenticationCapReq>,     //requires 3ds, do it later
    encrypted_payload : Option<String>,
}

#[derive(Default, Debug, Serialize, Deserialize)]
#[serde(rename_all="camelCase")]
pub struct PaymentAuthenticationCapReq{

}

#[derive(Default, Debug, Serialize, Deserialize)]
#[serde(rename_all="camelCase")]
pub struct AuthenticationCapReq{
    authentication_id : Option<String>,
    //three_d_s     //do it later 
    electronic_commerce_indicator : Option<String>,
    token_authentication_value : Option<String>, 
    s_c_a_exemption_reason : Option<String>,
}

#[derive(Default, Debug, Serialize, Deserialize)]
#[serde(rename_all="camelCase")]
pub struct AccountUpdaterCapReq{
    request_account_updater : Option<bool>,
}

#[derive(Default, Debug, Serialize, Deserialize)]
#[serde(rename_all="camelCase")]
pub struct ExpiryCapReq {
    month : i32,
    year : i32,
}

#[derive(Default, Debug, Serialize, Deserialize)]
#[serde(rename_all="camelCase")]
pub struct InstallmentCapReq {
    installment_count : Option<i32>,
    total_installments : Option<i32>,
    number_of_deferrals : Option<i32>,
    plan_id : Option<String>,
}

#[derive(Default, Debug, Serialize, Deserialize)]
#[serde(rename_all="camelCase")]
pub struct RecurringCapReq {
    recurring_sequence : Option<String>,
    agreement_id : Option<String>,
    payment_agreement_expiry_date : Option<String>,      //this will be string<date>, just recheck again
    is_variable_amount : Option<bool>,
}

#[derive(Default, Debug, Serialize, Deserialize)]
#[serde(rename_all="camelCase")]
pub struct MerchantCapReq{
    merchant_software : MerchantSoftwareCapReq,
    merchant_category_code : Option<String>,
    merchant_logo_url : Option<String>,
    soft_merchant : SoftMerchantCapReq,
}

#[derive(Default, Debug, Serialize, Deserialize)]
#[serde(rename_all="camelCase")]
pub struct MerchantSoftwareCapReq {
    company_name : Option<String>,
    product_name : Option<String>,
    version : Option<String>,
    software_id : Option<String>,
}

#[derive(Default, Debug, Serialize, Deserialize)]
#[serde(rename_all="camelCase")]
pub struct SoftMerchantCapReq {
    name : Option<String>,
    phone : Option<String>,
    email : Option<String>,
    url : Option<String>,
    merchant_purchase_description : Option<String>,
    visa_merchant_verification_value_id : Option<String>,
    master_card_merchant_verification_value_id : Option<String>,
    merchant_incorporation_status : Option<String>,
    foreign_merchant_indicator : Option<bool>,
}

#[derive(Debug, Default, Copy, Serialize, Deserialize, Clone)]
#[serde(rename_all="UPPERCASE")]
pub enum CaptureMethod {
    #[default]
    Now,
    Delayed,
    Manual,
}

impl TryFrom<&JpmorganRouterData<&PaymentsCaptureRouterData>> for JpmorganCaptureRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &JpmorganRouterData<&PaymentsCaptureRouterData>,
    ) -> Result<Self, Self::Error> {
        let capture_method = match item.router_data.request.capture_method.as_ref(){
            Some(cm) => Some(cm.to_string()),
            None => None,
        };
        println!("$$$$#####****  CAPTURE METHOD : {:?}", capture_method);

        // let merchant_software = JpmorganMerchantSoftware {
        //     company_name: String::from("JPMC"),
        //     product_name: String::from("Hyperswitch"), //could be Amazon or something else, subject to change
        // };
        //hardcoded as of now

        //let merchant = JpmorganMerchant { merchant_software };

        let currency = Some(item.router_data.request.currency.to_string());

        let amount = item.amount;
        Ok(Self{ 
            capture_method,
            merchant: None,
            recurring: None,
            installment:None,
            payment_method_type: None,
            ship_to:None,
            initiator_type: None,
            account_on_file: None,
            original_transaction_id: None,
            is_amount_final: None,
            amount,
            currency,
            merchant_order_number: None,
            risk: None,
            retail_addenda: None,
            account_holder: None,
            statement_descriptor: None,
            partial_authorization_support: None,
            payment_request_id: None,
            multi_capture: None,
            sub_merchant_supplemental_data: None,
        })
    }
}

//made changes here in JpmorganTransactionState
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all="camelCase")]
pub struct JpmorganCaptureResponse {
    pub transaction_id : String,
    pub request_id : String,
    pub transaction_state : JpmorganTransactionState,
    pub response_status : JpmorganTransactionStatus,
    pub response_code : String,
    pub response_message : String,
    pub payment_method_type : PaymentMethodTypeCapRes,
}

#[derive(Default, Debug, Serialize, Deserialize)]
#[serde(rename_all="camelCase")]
pub struct PaymentMethodTypeCapRes {
    pub card : Option<CardCapRes>,
}

#[derive(Default, Debug, Serialize, Deserialize)]
#[serde(rename_all="camelCase")]
pub struct CardCapRes{
    pub card_type : Option<String>,
    pub card_type_name : Option<String>,
    unmasked_account_number : Option<String>,
}

impl From<JpmorganTransactionState> for common_enums::AttemptStatus {
    fn from(item: JpmorganTransactionState) -> Self {
        match item {
            JpmorganTransactionState::Authorized => common_enums::AttemptStatus::Authorized,
            JpmorganTransactionState::Closed => common_enums::AttemptStatus::Charged,
            JpmorganTransactionState::Declined | JpmorganTransactionState::Error =>  common_enums::AttemptStatus::Failure,
            JpmorganTransactionState::Pending => common_enums::AttemptStatus::Pending,
            JpmorganTransactionState::Voided => common_enums::AttemptStatus::Voided,            
        }
    }
}

impl <F,T> TryFrom<ResponseRouterData<F, JpmorganCaptureResponse, T, PaymentsResponseData>>
        for RouterData<F, T, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, JpmorganCaptureResponse, T, PaymentsResponseData>
    ) -> Result<Self, Self::Error> {
        
        let transaction_state = item.response.transaction_state.to_string();
        println!("############# TRANSACTION STATE IN CAPTURE FLOW {}", transaction_state);

        let status = common_enums::AttemptStatus::from_transaction_state(transaction_state);
        println!("############## STATE AS STATUS IN CAPTURE FLOW {}", status);

        let transaction_id = item.response.transaction_id.clone();
        let connector_response_reference_id = Some (transaction_id.clone());

        let resource_id = ResponseId::ConnectorTransactionId(transaction_id.clone());

        Ok(Self {
            status: status,
            response: Ok(PaymentsResponseData::TransactionResponse { 
                resource_id: resource_id,
                redirection_data: Box::new(None),
                mandate_reference: Box::new(None),
                connector_metadata: None,
                network_txn_id: None,
                connector_response_reference_id: connector_response_reference_id,
                incremental_authorization_allowed: None,
                charge_id: None,
            }),
            ..item.data
        })

    }
}

//response struct for PSync 
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all="camelCase")]
pub struct JpmorganPSyncResponse {
    transaction_id : String,
    request_id : String,
    transaction_state : JpmorganTransactionState,
    //response_status : String,
    response_status : JpmorganResponseStatus,
    response_code : String,
    response_message : String,
    payment_method_type : PaymentMethodType,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all="UPPERCASE")]
pub enum JpmorganResponseStatus {
    Success,
    Denied, 
    Error,
}

impl<F,PaymentsSyncData> TryFrom<ResponseRouterData<F, JpmorganPSyncResponse, PaymentsSyncData, PaymentsResponseData>>
    for RouterData<F, PaymentsSyncData, PaymentsResponseData>{
        type Error = error_stack::Report<errors::ConnectorError>;
        fn try_from(
            item: ResponseRouterData<F, JpmorganPSyncResponse, PaymentsSyncData, PaymentsResponseData>
        ) -> Result<Self, Self::Error>{
            // match item.response.response_status {
            //     JpmorganResponseStatus::Success => {
                    let transaction_state = item.response.transaction_state.to_string();
                    let status = common_enums::AttemptStatus::from_transaction_state(transaction_state);

                    let transaction_id = item.response.transaction_id.clone();
                    let connector_response_reference_id = Some(transaction_id.clone());

                    let resource_id = ResponseId::ConnectorTransactionId(transaction_id.clone());

                    Ok(Self {
                        status,
                        response: Ok(PaymentsResponseData::TransactionResponse { 
                            resource_id,
                            redirection_data: Box::new(None),
                            mandate_reference: Box::new(None),
                            connector_metadata: None,
                            network_txn_id: None,
                            connector_response_reference_id,
                            incremental_authorization_allowed: None,
                            charge_id: None,
                        }),
                        ..item.data
                    })

                // }   
                // JpmorganResponseStatus::Denied | JpmorganResponseStatus::Error => {
                //     let response = Err(get_error_response(item.response.response_status, item.http_code));
                //     Ok(Self {
                //         response,
                //         ..item.data
                //     })
                // }
            }
        // }
    }

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TransactionData {
    payment_type: Option<String>,
    status_code: i32,
    txn_secret: Option<String>,
    tid: Option<Secret<i64>>,
    test_mode: Option<i8>,
    status: Option<JpmorganTransactionStatus>,
}

#[derive(Default, Debug, Serialize, Deserialize)]
#[serde(rename_all="camelCase")]
pub struct JpmorganRefundRequest {
    pub merchant : MerchantRefundReq,
    pub amount: MinorUnit,
}

#[derive(Default, Debug, Serialize, Deserialize)]
#[serde(rename_all="camelCase")]
pub struct MerchantRefundReq {
    pub merchant_software : MerchantSoftware
}

impl<F> TryFrom<&JpmorganRouterData<&RefundsRouterData<F>>> for JpmorganRefundRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &JpmorganRouterData<&RefundsRouterData<F>>) 
        -> Result<Self, Self::Error> {

        let merchant_software = MerchantSoftware{
            company_name : String::from("JPMC"),    //According to documentation, it should be the company name of software integrated to this API. If merchant is directly integrated, send "JPMC."
            product_name : String::from("Hyperswitch"),      //According to documentation, it should be the name of the product used for marketing purposes from a customer perspective. I. e. what the customer would recognize.
            //https://developer.payments.jpmorgan.com/api/commerce/online-payments/online-payments#/operations/V2PaymentPost
            
            version : Some(String::from("1.235"))       //recheck, seek guidance
        };
        
        let merchant = MerchantRefundReq {
            merchant_software
        };

        let amount = item.amount;

        Ok(Self {
            merchant,
            amount,
        })
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all="camelCase")]
pub struct JpmorganRefundResponse {
    pub transaction_id : Option<String>,
    pub request_id : String,
    pub transaction_state : JpmorganTransactionState,
    pub amount : MinorUnit,
    pub currency : String,
    pub response_status : JpmorganResponseStatus,
    pub response_code : String,
    pub response_message : String,
    pub transaction_reference_id : Option<String>, 
    pub remaining_refundable_amount : Option<i64>,
}

// Type definition for Refund Response

#[allow(dead_code)]
#[derive(Debug, Serialize, Default, Deserialize, Clone)]
pub enum RefundStatus {
    Succeeded,
    Failed,
    #[default]
    Processing,
}

impl From<RefundStatus> for enums::RefundStatus {
    fn from(item: RefundStatus) -> Self {
        match item {
            RefundStatus::Succeeded => Self::Success,
            RefundStatus::Failed => Self::Failure,
            RefundStatus::Processing => Self::Pending,
            //TODO: Review mapping
        }
    }
}

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct RefundResponse {
    id: String,
    status: RefundStatus,
}

pub trait FromRefundState {
    fn from_transaction_state(transaction_state: String) -> Self;
}

impl FromRefundState for common_enums::RefundStatus {
    fn from_transaction_state(transaction_state: String) -> Self {
        match transaction_state.as_str() {
            "Closed" | "Authorized"=> common_enums::RefundStatus::Success,
            "Declined" | "Error" => common_enums::RefundStatus::Failure,
            "Pending" => common_enums::RefundStatus::Pending,
            _ => common_enums::RefundStatus::Failure,
        }
    }
}

impl TryFrom<RefundsResponseRouterData<Execute, JpmorganRefundResponse>> 
        for RefundsRouterData<Execute> 
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: RefundsResponseRouterData<Execute, JpmorganRefundResponse>,
    ) -> Result<Self, Self::Error> {
        let refund_id = item.response.transaction_id.clone().ok_or(errors::ConnectorError::ResponseHandlingFailed)?;
    
        let transaction_state = item.response.transaction_state.to_string();
        let status = common_enums::RefundStatus::from_transaction_state(transaction_state);

        Ok(Self{
            response: Ok(RefundsResponseData{
                connector_refund_id: refund_id,
                refund_status: enums::RefundStatus::from(status),
            }),
            ..item.data
        })
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all="camelCase")]
pub struct JpmorganRefundSyncResponse {
    transaction_id : String,
    request_id : String,
    transaction_state : JpmorganTransactionState,
    amount : MinorUnit,
    currency : String,
    response_status : JpmorganResponseStatus,
    response_code : String,
}

impl TryFrom<RefundsResponseRouterData<RSync, JpmorganRefundSyncResponse>> for RefundsRouterData<RSync> {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: RefundsResponseRouterData<RSync, JpmorganRefundSyncResponse>,
    ) -> Result<Self, Self::Error> {
        let refund_id = item.response.transaction_id.clone();
        let transaction_state = item.response.transaction_state.to_string();
        let status = common_enums::RefundStatus::from_transaction_state(transaction_state);
        Ok(Self {
            response: Ok(RefundsResponseData {
                connector_refund_id : refund_id,
                refund_status: status,
            }),
            ..item.data
        })
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all="camelCase")]
pub struct JpmorganCancelRequest {
    //is_capture: Option<bool>,
    //capture_method : Option<String>,
    pub amount : Option<i64>,
    pub is_void : Option<bool>,
    pub reversal_reason : Option<String>,
}

impl TryFrom<JpmorganRouterData<&PaymentsCancelRouterData>> for JpmorganCancelRequest{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: JpmorganRouterData<&PaymentsCancelRouterData>) 
        -> Result<Self, Self::Error>{

        let is_void = Some(true);
        let amount = item.router_data.request.amount.clone();
        let reversal_reason = item.router_data.request.cancellation_reason.clone();
        Ok(Self{
            amount,
            is_void,
            reversal_reason,
        })
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all="camelCase")]
pub struct JpmorganCancelResponse{
    transaction_id : String,
    request_id : String,
    //transaction_state : JpmorganTransactionState,
    response_status : JpmorganResponseStatus,
    response_code : String,
    response_message : String,
    payment_method_type : JpmorganPaymentMethodTypeCancelResponse,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all="camelCase")]
pub struct JpmorganPaymentMethodTypeCancelResponse{
    pub card : CardCancelResponse
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all="camelCase")]
pub struct CardCancelResponse {
    pub card_type : String, 
    pub card_type_name : String,
    //pub is_bill_payment : bool,
    //pub masked_account_number : Secret<String>,
}

impl<F> TryFrom<ResponseRouterData<F, JpmorganCancelResponse, PaymentsCancelData, PaymentsResponseData>>
        for RouterData<F, PaymentsCancelData, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item : ResponseRouterData<F, JpmorganCancelResponse, PaymentsCancelData, PaymentsResponseData>) 
        -> Result<Self, Self::Error> {
        let response_status;
        match item.response.response_status {
            JpmorganResponseStatus::Success => response_status = String::from("Success"),
            JpmorganResponseStatus::Denied => response_status = String::from("Denied"),
            JpmorganResponseStatus::Error => response_status = String::from("Error")
        }

        let status = common_enums::AttemptStatus::from_response_status(response_status);

        let transaction_id = item.response.transaction_id.clone();

        let resource_id = ResponseId::ConnectorTransactionId(transaction_id.clone());

        let connector_response_reference_id = Some(transaction_id.clone());

        Ok(Self{
            status,
            response : Ok(PaymentsResponseData::TransactionResponse {
                resource_id,
                redirection_data : Box::new(None),
                mandate_reference : Box::new(None),
                connector_metadata : None,
                network_txn_id : None,
                connector_response_reference_id,
                incremental_authorization_allowed : None,
                charge_id : None,
            }),
            ..item.data
        })
        
    } 
}

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
pub struct JpmorganErrorResponse {
    pub status_code: u16,
    pub code: String,
    pub message: String,
    pub reason: Option<String>,
}
