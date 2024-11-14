use common_enums::enums;
use common_utils::types::StringMinorUnit;
use hyperswitch_domain_models::{
    payment_method_data::PaymentMethodData,
    router_data::{ConnectorAuthType, RouterData},
    router_flow_types::{refunds::{Execute, RSync}, Capture},
    router_request_types::{ResponseId, PaymentsCaptureData,},
    router_response_types::{PaymentsResponseData, RefundsResponseData},
    types::{PaymentsAuthorizeRouterData, PaymentsCaptureRouterData, RefundsRouterData},
};
use hyperswitch_interfaces::errors;
use masking::{ExposeInterface, Secret};
use serde::{Deserialize, Serialize};
use strum::Display;
use crate::utils::PaymentsAuthorizeRequestData;
use crate::{
    types::{RefundsResponseRouterData, ResponseRouterData},
    //utils::RequestData,
};
pub struct JpmorganRouterData<T> {
    pub amount: StringMinorUnit,
    pub router_data: T,
}

impl<T> From<(StringMinorUnit, T)> for JpmorganRouterData<T> {
    fn from((amount, item): (StringMinorUnit, T)) -> Self {
        //Todo :  use utils to convert the amount to the type of amount that a connector accepts
        Self {
            amount,
            router_data: item,
        }
    }
}

#[derive(Default, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JpmorganPaymentsRequest {
    capture_method: String,
    amount: StringMinorUnit,
    currency: String,
    merchant: JpmorganMerchant,
    payment_method_type: JpmorganPaymentMethodType,
}

#[derive(Default, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JpmorganCard {
    account_number: Secret<String>,
    expiry: Expiry,
    is_bill_payment: bool,
}

#[derive(Default, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JpmorganPaymentMethodType {
    card: JpmorganCard,
}

#[derive(Default, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Expiry {
    month: Secret<String>,
    year: Secret<String>,
}

#[derive(Serialize, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct JpmorganMerchantSoftware {
    company_name: String,
    product_name: String,
}

#[derive(Default, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JpmorganMerchant {
    merchant_software: JpmorganMerchantSoftware,
}

impl TryFrom<&JpmorganRouterData<&PaymentsAuthorizeRouterData>> for JpmorganPaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &JpmorganRouterData<&PaymentsAuthorizeRouterData>,
    ) -> Result<Self, Self::Error> {
        match item.router_data.request.payment_method_data.clone() {
            PaymentMethodData::Card(req_card) => {
                let capture_method: String = if item.router_data.request.is_auto_capture().unwrap()
                {
                    String::from("NOW")
                } else {
                    String::from("MANUAL")
                };

                let currency: String = String::from("USD");
                //hardcoded as of now

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

                let card = JpmorganCard {
                    //in my case i used acc num
                    account_number: String::from("4012000033330026").into(), //keeping a dummy val as of now
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
    pub(super) api_key: Secret<String>,
}

impl TryFrom<&ConnectorAuthType> for JpmorganAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            ConnectorAuthType::HeaderKey { api_key } => Ok(Self {
                api_key: api_key.to_owned(),
            }),
            _ => Err(errors::ConnectorError::FailedToObtainAuthType.into()),
        }
    }
}
// PaymentsResponse
//TODO: Append the remaining status flags
#[derive(Debug, Default, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "UPPERCASE")]
pub enum JpmorganTransactionStatus {
    Success,
    #[default]
    Pending,
    Denied,
    Error,
}

impl From<JpmorganTransactionStatus> for common_enums::AttemptStatus {
    fn from(item: JpmorganTransactionStatus) -> Self {
        match item {
            JpmorganTransactionStatus::Success => Self::Charged,
            JpmorganTransactionStatus::Denied | JpmorganTransactionStatus::Error => Self::Failure,
            JpmorganTransactionStatus::Pending => Self::Pending,
            //JpmorganTransactionStatus::Processing => Self::Authorizing,
            //more fields to add here
        }
    }
}

#[derive(Default, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JpmorganPaymentsResponse {
    transaction_id: String,
    request_id: Option<String>,
    transaction_state: String,
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

fn convert_transaction_state(transaction_state: &str) -> common_enums::AttemptStatus {
    // Map the string value of `transaction_state` to the appropriate AttemptStatus variant
    match transaction_state {
        "Authorized" => common_enums::AttemptStatus::Authorized,
        "AuthorizationFailed" => common_enums::AttemptStatus::AuthorizationFailed,
        "Charged" => common_enums::AttemptStatus::Charged,
        "PaymentMethodAwaited" => common_enums::AttemptStatus::PaymentMethodAwaited,
        "Failure" => common_enums::AttemptStatus::Failure,
        // Handle other cases if needed, using the most suitable AttemptStatus variant
        _ => common_enums::AttemptStatus::default(), // Default to Pending if no match is found
    }
}

impl<F, T> TryFrom<ResponseRouterData<F, JpmorganPaymentsResponse, T, PaymentsResponseData>>
    for RouterData<F, T, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(
        item: ResponseRouterData<F, JpmorganPaymentsResponse, T, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        let status = convert_transaction_state(&item.response.transaction_state.as_ref());

        let resource_id = ResponseId::ConnectorTransactionId(item.response.transaction_id.clone());

        let connector_response_reference_id = item.response.host_reference_id.clone();

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
    amount : Option<i64>,
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

impl CaptureMethod {
    fn convert_to_string_capture_method (&self) -> String {
        match self {
            CaptureMethod::Now => String::from("NOW"),
            CaptureMethod::Delayed => String::from("DELAYED"),
            CaptureMethod::Manual => String::from("MANUAL"),
        }
    }
}

impl TryFrom<&JpmorganRouterData<&PaymentsCaptureRouterData>> for JpmorganCaptureRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &JpmorganRouterData<&PaymentsCaptureRouterData>,
    ) -> Result<Self, Self::Error> {
        let capture_method : Option<String> = Some(CaptureMethod::Manual.convert_to_string_capture_method().to_string()) ;

        Ok(Self{
            capture_method,
            merchant : None,
            recurring: None,
            installment: None,
            payment_method_type: None,
            ship_to: None,
            initiator_type: None,
            account_on_file: None,
            original_transaction_id: None,
            is_amount_final: None,
            amount: None,
            currency: None,
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

#[derive(Default, Debug, Serialize, Deserialize)]
#[serde(rename_all="camelCase")]
pub struct JpmorganCaptureResponse {
    pub transaction_id : String,
    pub request_id : String,
    pub transaction_state : String,
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

//impl for Response starts here

/*impl From<JpmorganTransactionStatus> for common_enums::AttemptStatus {
    fn from(item: JpmorganTransactionStatus) -> Self {
        match item {
            JpmorganTransactionStatus::Success => Self::Charged,
            JpmorganTransactionStatus::Error | JpmorganTransactionStatus::Denied => Self::Failure,
            JpmorganTransactionStatus::Pending => Self::Pending
        }
    }
}*/

impl<F>TryFrom<ResponseRouterData<F, JpmorganCaptureResponse, PaymentsCaptureData, PaymentsResponseData>>
    for RouterData<F, PaymentsCaptureData, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F,JpmorganCaptureResponse,PaymentsCaptureData,PaymentsResponseData>
    ) -> Result<Self, Self::Error> {
        //match item.response.response_status{
            //JpmorganTransactionStatus::Success | JpmorganTransactionStatus::Pending => {
                
                let transaction_status = item.response.response_status.clone();

                let status = common_enums::AttemptStatus::from(transaction_status);

                let transaction_id = item.response.transaction_id.clone();

                Ok(Self {
                    status,
                    response : Ok(PaymentsResponseData::TransactionResponse{
                        resource_id : ResponseId::ConnectorTransactionId(transaction_id.clone()),
                        redirection_data : Box::new(None), 
                        mandate_reference : Box::new(None),
                        connector_metadata : None,
                        network_txn_id : None, 
                        connector_response_reference_id : Some(transaction_id),
                        incremental_authorization_allowed : None,
                        charge_id : None, 
                    }),
                    ..item.data
                })
            //}
            //JpmorganTransactionStatus::Denied | JpmorganTransactionStatus::Error => {

            //}
            
        }
    }
//}

//impl for Response ends here 

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TransactionData {
    payment_type: Option<String>,
    status_code: i32,
    txn_secret: Option<String>,
    tid: Option<Secret<i64>>,
    test_mode: Option<i8>,
    status: Option<JpmorganTransactionStatus>,
}

#[derive(Default, Debug, Serialize)]
pub struct JpmorganRefundRequest {
    pub amount: StringMinorUnit,
}

impl<F> TryFrom<&JpmorganRouterData<&RefundsRouterData<F>>> for JpmorganRefundRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &JpmorganRouterData<&RefundsRouterData<F>>) -> Result<Self, Self::Error> {
        Ok(Self {
            amount: item.amount.to_owned(),
        })
    }
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

impl TryFrom<RefundsResponseRouterData<Execute, RefundResponse>> for RefundsRouterData<Execute> {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: RefundsResponseRouterData<Execute, RefundResponse>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(RefundsResponseData {
                connector_refund_id: item.response.id.to_string(),
                refund_status: enums::RefundStatus::from(item.response.status),
            }),
            ..item.data
        })
    }
}

impl TryFrom<RefundsResponseRouterData<RSync, RefundResponse>> for RefundsRouterData<RSync> {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: RefundsResponseRouterData<RSync, RefundResponse>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(RefundsResponseData {
                connector_refund_id: item.response.id.to_string(),
                refund_status: enums::RefundStatus::from(item.response.status),
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
