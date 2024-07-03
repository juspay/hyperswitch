use common_utils::pii::{self, Email};
use masking::Secret;
use serde::{Deserialize, Serialize};
use time::PrimitiveDateTime;

use crate::{
    connector::utils::PaymentsAuthorizeRequestData,
    core::errors,
    types::{self, api, domain, storage::enums},
};

//TODO: Fill the struct with respective fields
pub struct RazorpayRouterData<T> {
    pub amount: i64, // The type of amount that a connector accepts, for example, String, i64, f64, etc.
    pub router_data: T,
}

impl<T> TryFrom<(&api::CurrencyUnit, enums::Currency, i64, T)> for RazorpayRouterData<T> {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        (_currency_unit, _currency, amount, item): (&api::CurrencyUnit, enums::Currency, i64, T),
    ) -> Result<Self, Self::Error> {
        //Todo :  use utils to convert the amount to the type of amount that a connector accepts
        Ok(Self {
            amount,
            router_data: item,
        })
    }
}

//TODO: Fill the struct with respective fields
#[derive(Debug, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RazorpayPaymentsRequest {
    second_factor: SecondFactor,
    merchant_account: MerchantAccount,
    order_reference: OrderReference,
    txn_detail: TxnDetail,
    txn_card_info: TxnCardInfo,
    merchant_gateway_account: MerchantGatewayAccount,
    gateway: Gateway,
    // gateway: String,
    transaction_create_req: TransactionCreateReq,
    is_mesh_enabled: bool,
    order_metadata_v2: OrderMetadataV2,
}

#[derive(Debug, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]

pub struct SecondFactor {
    txn_id: String,
    id: String,
    status: SecondFactorStatus,
    #[serde(rename = "type")]
    sf_type: String,
    version: i32,
    #[serde(with = "common_utils::custom_serde::iso8601")]
    date_created: PrimitiveDateTime,
    #[serde(with = "common_utils::custom_serde::iso8601")]
    last_updated: PrimitiveDateTime,
    transaction_id: Option<String>,
    url: Option<String>,
    epg_transaction_id: Option<String>,
    transaction_detail_id: Option<String>,
    gateway_auth_required_params: Option<String>,
    authentication_account_id: Option<String>,
    can_accept_response: Option<bool>,
    challenges_attempted: Option<i32>,
    response_attempted: Option<i32>,
    partition_key: Option<String>,
}

#[derive(Debug, Serialize, Eq, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SecondFactorStatus {
    Pending,
    Init,
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MerchantAccount {
    id: i64,
    merchant_id: String,
    use_code_for_gateway_priority: bool,
    gateway_success_rate_based_outage_input: String,
    gateway_success_rate_based_decider_input: String,
    auto_refund_multiple_charged_transactions: bool,
    card_encoding_key: Option<String>,
    enable_unauthenticated_order_status_api: Option<bool>,
    enabled_instant_refund: Option<bool>,
    enable_reauthentication: Option<bool>,
    return_url: Option<String>,
    credit_balance: Option<i64>,
    internal_metadata: Option<String>,
    gateway_decided_by_health_enabled: Option<bool>,
    zip: Option<String>,
    enable_3d_secure_help_mail: Option<String>,
    payout_mid: Option<String>,
    gateway_priority_logic: Option<String>,
    enable_success_rate_based_gateway_elimination: Option<bool>,
    otp_enabled: Option<bool>,
    enable_sending_card_isin: Option<bool>,
    state: Option<String>,
    must_use_given_order_id_for_txn: Option<bool>,
    gateway_priority: Option<String>,

    timezone: Option<String>,
    user_id: Option<i64>,
    office_line_1: Option<String>,
    enable_save_card_before_auth: Option<bool>,
    office_line_2: Option<String>,
    merchant_legal_name: Option<String>,
    settlement_account_id: Option<i64>,
    external_metadata: Option<String>,
    office_line_3: Option<String>,
    enable_payment_response_hash: Option<bool>,
    prefix_merchant_id_for_card_key: Option<bool>,
    admin_contact_email: Option<String>,
    enable_reauthorization: Option<bool>,
    locker_id: Option<String>,
    enable_recapture: Option<bool>,
    contact_person_email: Option<String>,
    basilisk_key_id: Option<String>,
    whitelabel_enabled: Option<bool>,
    inline_checkout_enabled: Option<bool>,
    payu_merchant_key: Option<String>,
    encryption_key_ids: Option<String>,
    enable_gateway_reference_id_based_routing: Option<bool>,
    enabled: Option<bool>,
    enable_automatic_retry: Option<bool>,
    about_business: Option<String>,
    redirect_to_merchant_with_http_post: Option<bool>,
    webhook_api_version: Option<String>,
    express_checkout_enabled: Option<bool>,
    city: Option<String>,
    webhook_url: Option<String>,
    webhook_username: Option<String>,
    webhook_custom_headers: Option<String>,
    reverse_token_enabled: Option<bool>,

    webhook_configs: Option<String>,
    last_modified: Option<String>, // changed from `UTCTime` to `String` for simplicity
    token_locker_id: Option<String>,
    enable_sending_last_four_digits: Option<bool>,
    website: Option<String>,
    mobile: Option<String>,
    webhook_password: Option<String>,
    reseller_id: Option<String>,
    mobile_version: Option<String>,
    contact_person_primary: Option<String>,
    conflict_status_email: Option<String>,
    payu_test_mode: Option<bool>,
    payment_response_hash_key: Option<String>,
    enable_refunds_in_dashboard: Option<bool>,
    tenant_account_id: Option<String>,
    merchant_name: Option<String>,
    hdfc_test_mode: Option<bool>,
    enable_unauthenticated_card_add: Option<bool>,
    payu_salt: Option<String>,
    api_key: Option<String>,
    date_created: Option<String>, // changed from `UTCTime` to `String` for simplicity
    internal_hash_key: Option<String>,
    version: Option<i64>,

    mandatory_2fa: Option<bool>,
}

#[derive(Debug, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct OrderReference {
    id: String,
    amount: i64,
    currency: String,
    order_id: String,
    status: OrderStatus,
    merchant_id: String,
    order_uuid: String,
    order_type: String,
    version: i64,
    #[serde(with = "common_utils::custom_serde::iso8601")]
    date_created: PrimitiveDateTime,
    #[serde(with = "common_utils::custom_serde::iso8601")]
    last_modified: PrimitiveDateTime,
    return_url: Option<String>,
    billing_address_id: Option<String>,
    internal_metadata: Option<String>,
    mandate_feature: Option<MandateFeature>,
    udf6: Option<String>,
    udf1: Option<String>,
    partition_key: Option<String>,
    amount_refunded: Option<i64>,
    customer_phone: Option<String>,
    description: Option<String>,
    customer_email: Option<Email>,
    customer_id: Option<String>,
    refunded_entirely: Option<bool>,
}

#[derive(Debug, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum MandateFeature {
    Disabled,
}

#[derive(Debug, Serialize, Eq, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum OrderStatus {
    PendingAuthentication,
}

#[derive(Debug, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TxnDetail {
    order_id: String,
    internal_metadata: Option<String>,
    express_checkout: bool,
    txn_mode: String,
    merchant_id: String,
    txn_flow_type: String,
    status: String,
    net_amount: i64,
    txn_id: String,
    txn_amount: i64,
    emi_tenure: i32,
    txn_uuid: String,
    source_object: String,
    id: String,
    partition_key: Option<String>,
    add_to_locker: bool,
    currency: String,
    username: String,
    txn_object_type: String,
    is_emi: bool,
    gateway: Gateway,
    last_modified: Option<PrimitiveDateTime>, // changed from `UTCTime` to `String` for simplicity
    merchant_gateway_account_id: i64,
    internal_tracking_info: Option<String>,
    txn_type: String, // renamed to avoid conflict with the keyword "type"
    redirect: bool,
    date_created: Option<PrimitiveDateTime>, // changed from `UTCTime` to `String` for simplicity
    version: i64,
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TxnCardInfo {
    txn_detail_id: String,
    txn_id: String,
    payment_method_type: String,
    id: String,
    partition_key: Option<String>,
    card_type: String,
    payment_method: String,
    payment_source: Option<Secret<String, pii::UpiVpaMaskingStrategy>>,
    card_issuer_bank_name: String,
    date_created: Option<PrimitiveDateTime>, // changed from `UTCTime` to `String` for simplicity
}

#[derive(Debug, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MerchantGatewayAccount {
    merchant_id: String,
    disabled: bool,
    id: i64,
    account_details: String,
    payment_methods: String,
    test_mode: bool,
    enforce_payment_method_acceptance: bool,
    gateway: Gateway,
    last_modified: PrimitiveDateTime, // changed from `UTCTime` to `String` for simplicity
    date_created: PrimitiveDateTime,  // changed from `UTCTime` to `String` for simplicity
    version: i64,
    supported_payment_flows: Option<String>,
    is_juspay_account: bool,
}

#[derive(Debug, Serialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct TransactionCreateReq {
    merchant_id: String,
}

#[derive(Debug, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct OrderMetadataV2 {
    order_reference_id: String,
    #[serde(with = "common_utils::custom_serde::iso8601")]
    last_updated: PrimitiveDateTime,
    browser: String,
    operating_system: String,
    id: String,
    ip_address: Option<String>,
    partition_key: Option<String>,
    user_agent: String,
    browser_version: String,
    mobile: Option<String>,
    metadata: String,
    #[serde(with = "common_utils::custom_serde::iso8601")]
    date_created: PrimitiveDateTime,
}

#[derive(Debug, Serialize, Eq, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Gateway {
    Razorpay,
    YesBiz,
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
pub struct RazorpayCard {
    number: cards::CardNumber,
    expiry_month: Secret<String>,
    expiry_year: Secret<String>,
    cvc: Secret<String>,
    complete: bool,
}

impl TryFrom<&RazorpayRouterData<&types::PaymentsAuthorizeRouterData>> for RazorpayPaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &RazorpayRouterData<&types::PaymentsAuthorizeRouterData>,
    ) -> Result<Self, Self::Error> {
        let request = &item.router_data.request;
        match request.payment_method_data.clone() {
            domain::PaymentMethodData::Upi(upi_type) => match upi_type {
                domain::UpiData::UpiCollect(upi_data) => {
                    // match request.order_details.clone() {
                    //     //payment_attempt metadata
                    //     Some(order_details) => {
                    let second_factor = SecondFactor { //connector_meta_data
                            txn_id:item.router_data.attempt_id.clone(),
                            id: "100002345089".to_string(),
                            status: SecondFactorStatus::Pending,
                            transaction_id: Some("azhar_test-aneel1719496390-3".to_string()),
                            sf_type: "VBV".to_string(), 
                            url: Some("https://api.juspay.in/v2/pay/start/com.swiggy/mozjbTGtwWJ7EenZSBw?cardIssuerBankName%3DUPI%26cardType%3DUPI%26paymentMethod%3DUPI%26paymentMethodType%3DUPI".to_string()),
                            version: 0,
                            epg_transaction_id: None,
                            transaction_detail_id: Some("100002500863".to_string()),
                            gateway_auth_required_params: Some("{\"mandateRegRefId\":\"mozjbTGtwWJ7EenZSBw\",\"isNewton360Transaction\":\"true\",\"euler-api-gateway\":[\"sendCollect\"],\"tr\":\"com.swiggy-201946381000660-1\"}".to_string()),
                            authentication_account_id: None,
                            can_accept_response: Some(true),
                            challenges_attempted: Some(0),
                            response_attempted: Some(0),
                            date_created: common_utils::date_time::now(),
                            last_updated: common_utils::date_time::now(),
                            partition_key: None,
                    };
                    let merchant_account = MerchantAccount {
                        id: 205,
                        merchant_id: item.router_data.merchant_id.clone(),
                        auto_refund_multiple_charged_transactions: false,
                        use_code_for_gateway_priority: true,
                        gateway_success_rate_based_outage_input: "{\"merchantOutagePaymentMethodWiseInputs\":[],\"defaultMerchantOutageDownMaxCountThreshold\":3,\"defaultMerchantOutageFluctuateThreshold\":0.902,\"defaultMerchantOutageFluctuateMaxCountThreshold\":3,\"defaultMerchantOutageDownThreshold\":0.902}".to_string(),
                        gateway_success_rate_based_decider_input: "{\"defaultEliminationThreshold\":0.3,\"defaultEliminationLevel\":\"PAYMENT_METHOD\",\"defaultSelectionLevel\":\"PAYMENT_MODE\",\"defaultGlobalEliminationLevel\":\"PAYMENT_METHOD\",\"defaultGlobalEliminationMaxCountThreshold\":2,\"defaultGlobalEliminationThreshold\":0.5,\"gatewayWiseInputs\":[],\"globalGatewayWiseInputs\":[]}".to_string(),
                        card_encoding_key: None,
                        enable_unauthenticated_order_status_api: Some(false),
                        enabled_instant_refund: Some(true),
                        enable_reauthentication: Some(false),
                        return_url: Some("http://www.swiggy.com/justpay/response.php".to_string()),
                        credit_balance: None,
                        internal_metadata: Some("{\"track\":\"F2\",\"industry\":\"Hyperlocal\",\"integration_type\":[\"EC_SDK\"]}".to_string()),
                        gateway_decided_by_health_enabled: Some(true),
                        zip: None,
                        enable_3d_secure_help_mail: None,
                        payout_mid: Some("com.swiggy".to_string()),
                        gateway_priority_logic: None,
                        enable_success_rate_based_gateway_elimination:Some(true),
                        otp_enabled: Some(false),
                        enable_sending_card_isin: Some(true),
                        state: Some("Karnataka".to_string()),
                        must_use_given_order_id_for_txn: Some(false),
                        gateway_priority: Some("PAYU".to_string()),
                        timezone: Some("Asia/Kolkata".to_string()),
                        user_id: Some(1054),
                        office_line_1: Some("#806".to_string()),
                        enable_save_card_before_auth: Some(false),
                        office_line_2: Some("5th Cross".to_string()),
                        merchant_legal_name: Some("Bundl Technologies Private Limited.".to_string()),
                        settlement_account_id: Some(56),
                        external_metadata: None,
                        office_line_3: Some("Johnnagar".to_string()),
                        enable_payment_response_hash: Some(true),
                        prefix_merchant_id_for_card_key: Some(false),
                        admin_contact_email: None,
                        enable_reauthorization: Some(false),
                        locker_id: None,
                        enable_recapture: Some(false),
                        contact_person_email: None,
                        basilisk_key_id: Some("516af52b-ae1a-40cf-9e42-16436e9ede74".to_string()),
                        whitelabel_enabled: Some(true),
                        inline_checkout_enabled: Some(false),
                        payu_merchant_key: None,
                        encryption_key_ids: None,
                        enable_gateway_reference_id_based_routing: Some(true),
                        enabled: Some(true),
                        enable_automatic_retry: Some(false),
                        about_business: None,
                        redirect_to_merchant_with_http_post: Some(false),
                        webhook_api_version: None,
                        express_checkout_enabled: Some(false),
                        city: None,
                        webhook_url: None, // doubt
                        webhook_username: None,
                        webhook_custom_headers: Some("".to_string()),
                        reverse_token_enabled: Some(false),
                        webhook_configs: Some("{\"addFullGatewayResponse\":true,\"webhookEvents\":{\"AUTO_REFUND_INITIATED\":false,\"AUTO_REFUND_SUCCEEDED\":false,\"AUTO_REFUND_FAILED\":false,\"REFUND_MANUAL_REVIEW_NEEDED\":true,\"CHARGEBACK_ALREADY_REFUNDED\":false,\"CHARGEBACK_CANCELED\":false,\"CHARGEBACK_EXPIRED\":false,\"CHARGEBACK_EVIDENCE_REQUIRED\":false,\"CHARGEBACK_RECEIVED\":false,\"CHARGEBACK_RESOLVED_IN_CUSTOMER_FAVOUR\":false,\"CHARGEBACK_RESOLVED_IN_MERCHANT_FAVOUR\":false,\"CHARGEBACK_UNDER_REVIEW\":false,\"MANDATE_ACTIVATED\":false,\"MANDATE_CREATED\":false,\"MANDATE_EXPIRED\":false,\"MANDATE_FAILED\":false,\"MANDATE_PAUSED\":false,\"MANDATE_REVOKED\":false,\"NOTIFICATION_SUCCEEDED\":false,\"NOTIFICATION_FAILED\":false,\"ORDER_AUTHORIZED\":false,\"ORDER_FAILED\":true,\"ORDER_PARTIAL_CHARGED\":false,\"ORDER_REFUNDED\":true,\"ORDER_REFUND_FAILED\":true,\"ORDER_SUCCEEDED\":true,\"ORDER_VOIDED\":false,\"ORDER_VOID_FAILED\":false,\"ORDER_CAPTURE_FAILED\":false,\"TXN_CREATED\":false,\"TXN_CHARGED\":false,\"TXN_FAILED\":false,\"TOKEN_STATUS_UPDATED\":false}}".to_string()),
                        last_modified: Some("2024-06-20T08:12:13Z".to_string()),
                        token_locker_id: None,
                        enable_sending_last_four_digits: Some(true),
                        website: Some("www.swiggy.in".to_string()),
                        mobile: None,
                        webhook_password: None,
                        reseller_id: Some("swiggy_master".to_string()),
                        mobile_version: Some("2_1".to_string()),
                        contact_person_primary: Some("Rahul".to_string()),
                        conflict_status_email: None,
                        payu_test_mode: Some(false),
                        payment_response_hash_key: None,
                        enable_refunds_in_dashboard: Some(true),
                        tenant_account_id: None,
                        merchant_name: Some("Swiggy".to_string()),
                        hdfc_test_mode: Some(false),
                        enable_unauthenticated_card_add: Some(true),
                        payu_salt: None,
                        api_key: None,
                        date_created: Some("2015-02-03T19:09:41Z".to_string()),
                        internal_hash_key: None,
                        version: Some(23),
                        mandatory_2fa: Some(false),
                    };
                    let order_reference = OrderReference { //payment_intent
                        id: "20694748743".to_string(),
                                order_id:item.router_data.connector_request_reference_id.clone(), //payment_id
                                return_url: Some(request.get_router_return_url()?),
                                billing_address_id: Some("100002801749".to_string()),
                                internal_metadata: Some("{\"gateway_ref_ids\":{},\"pii_hash\":{\"customer_email_hash\":\"03748cc53fb4ea7e86ff213f9f0707150766a8eb6ad1b71e7acc7fa08198a7ee\",\"customer_phone_hash\":\"b50e114c143215046d0c356c749e274052a4f837273d7b32096c780559143104\"}}".to_string()),
                                amount: item.amount,
                                merchant_id: item.router_data.merchant_id.clone(),
                                mandate_feature: Some(MandateFeature::Disabled),
                                status: OrderStatus::PendingAuthentication,
                                udf6: Some("Swiggy-iOS".to_string()), //----
                                order_type: "ORDER_PAYMENT".to_string(),
                                udf1: Some("11453723".to_string()),
                                partition_key: None,
                                amount_refunded: None,
                                customer_phone: None,
                                description: item.router_data.description.clone(),
                                currency: request.currency.to_string(),
                                last_modified: common_utils::date_time::now(),
                                customer_email: request.email.clone(),
                                customer_id: request.customer_id.clone(),
                                order_uuid: "ordeh_e45aae46108347c481495117d204aa90".to_string(),
                                date_created: common_utils::date_time::now(),
                                refunded_entirely: Some(false),
                                version: 1,
                    };
                    let txn_detail = TxnDetail {
                        //payment_attempt
                        order_id: item.router_data.connector_request_reference_id.clone(), //payment_id
                        internal_metadata: None,
                        express_checkout: false,
                        txn_mode: "PROD".to_string(),
                        merchant_id: item.router_data.merchant_id.clone(),
                        txn_flow_type: "COLLECT".to_string(),
                        status: "PENDING_VBV".to_string(),
                        net_amount: item.amount,
                        txn_id: item.router_data.attempt_id.clone(),
                        txn_amount: item.amount,
                        emi_tenure: 0,
                        txn_uuid: "mozjbTGtwWJ7EenZSBw".to_string(),
                        source_object: "UPI_COLLECT".to_string(),
                        id: "20270366630".to_string(),
                        partition_key: None,
                        add_to_locker: false,
                        currency: request.currency.to_string(),
                        username: "TRANSACTION".to_string(),
                        txn_object_type: "ORDER_PAYMENT".to_string(),
                        is_emi: false,
                        gateway: Gateway::Razorpay,
                        last_modified: Some(common_utils::date_time::now()),
                        merchant_gateway_account_id: 11476,
                        internal_tracking_info: None,
                        txn_type: "AUTH_AND_SETTLE".to_string(),
                        redirect: true,
                        date_created: Some(common_utils::date_time::now()),
                        version: 0,
                    };
                    let txn_card_info = TxnCardInfo {
                        txn_detail_id: "100002500863".to_string(),
                        txn_id: item.router_data.attempt_id.clone(),
                        payment_method_type: "UPI".to_string(),
                        id: "20182282902".to_string(),
                        partition_key: None,
                        card_type: "UPI".to_string(),
                        payment_method: "UPI".to_string(),
                        payment_source: upi_data.vpa_id,
                        card_issuer_bank_name: "UPI".to_string(),
                        date_created: Some(common_utils::date_time::now()),
                    };

                    let merchant_gateway_account = MerchantGatewayAccount { //mca
                                merchant_id: item.router_data.merchant_id.clone(),
                                disabled: false,
                                id: 46519,
                                account_details: "{\"razorpayId\": \"rzp_test_4UX9WwyEpxGkRv\",\"razorpaySecret\": \"4xzFIa6BEXNyhhHG6zdlm41B\",\"razorpayWebhooksSecret\": \"\",\"tokenType\": \"\",\"accessToken\": \"\",\"refreshToken\": \"\",\"publicToken\": \"\",\"timestamp\": 1674137595,\"expiresIn\": 7776000,\"disableAutoCapture\": \"false\",\"cardDirectOtpEnabled\": \"true\",\"waitingPageExpiryInSeconds\": \"\",\"payeeVpa\": \"\",\"subscription\": \"true\",\"onlySubscription\": \"false\",\"enableEmandate\": \"false\",\"isPreAuthEnabled\": \"false\",\"merchID\": \"\",\"username\": \"\",\"password\": \"\",\"certFilename\": \"\",\"certContent\": \"\",\"certContentLastModified\": \"\",\"soapKey\": \"\",\"visaOboApiKey\": \"\",\"visaOboOrgUnitId\": \"\",\"visaOboApiIdentifier\": \"\",\"gatewayMerchantId\": \"80oXBj51MHGmwH\",\"viesEnabled\": \"false\"}".to_string(),
                                payment_methods: "{\"paymentMethods\":[\"QR\",\"UPI\",\"UPI_QR\",\"GOOGLEPAY\"]}".to_string(),
                                test_mode: false,
                                enforce_payment_method_acceptance: true,
                                gateway: Gateway::Razorpay,
                                last_modified: common_utils::date_time::now(),
                                date_created: common_utils::date_time::now(),
                                version: 0,
                                supported_payment_flows: None,
                                is_juspay_account: false,
                            };

                    let gateway = Gateway::Razorpay;
                    let transaction_create_req = TransactionCreateReq {
                        merchant_id: item.router_data.merchant_id.clone(),
                    };
                    let is_mesh_enabled = false;

                    //payment_intent_meta_data
                    let order_metadata_v2 = OrderMetadataV2 {
                                    order_reference_id: "100004135919".to_string(),
                                    last_updated: common_utils::date_time::now(),
                                    browser: "okhttp".to_string(),
                                    operating_system: "unknown".to_string(),
                                    id: "100004066855".to_string(),
                                    ip_address: None,
                                    partition_key: None,
                                    user_agent: "PostmanRuntime/7.32.3".to_string(),
                                    browser_version: "7.32.3".to_string(),
                                    mobile: None,
                                    metadata: "{\"payment_links\":{\"iframe\":\"https://payments.juspay.in/payment-page/order/ordeh_5638be025e42485e9906a389abe9cda4\",\"web\":\"https://payments.juspay.in/payment-page/order/ordeh_5638be025e42485e9906a389abe9cda4\",\"mobile\":\"https://payments.juspay.in/payment-page/order/ordeh_5638be025e42485e9906a389abe9cda4\"},\"order_expiry\":\"2024-06-23T08:13:02Z\",\"payment_page_client_id\":\"com.swiggy\"}".to_string(),
                                    date_created: common_utils::date_time::now(),
                            };

                    Ok(Self {
                        second_factor,
                        merchant_account,
                        order_reference,
                        txn_detail,
                        txn_card_info,
                        merchant_gateway_account,
                        gateway,
                        transaction_create_req,
                        is_mesh_enabled,
                        order_metadata_v2,
                    })
                    //     }
                    //     None => Err(report!(errors::ConnectorError::MissingRequiredField {
                    //         field_name: "order_details"
                    //     })),
                    // }
                }
                hyperswitch_domain_models::payment_method_data::UpiData::UpiIntent(_) => Err(
                    errors::ConnectorError::NotImplemented("Payment methods".to_string()).into(),
                ),
            },
            _ => Err(errors::ConnectorError::NotImplemented("Payment methods".to_string()).into()),
        }
    }
}

//TODO: Fill the struct with respective fields
// Auth Struct
pub struct RazorpayAuthType {
    pub(super) api_key: Secret<String>,
}

impl TryFrom<&types::ConnectorAuthType> for RazorpayAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &types::ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            types::ConnectorAuthType::HeaderKey { api_key } => Ok(Self {
                api_key: api_key.to_owned(),
            }),
            _ => Err(errors::ConnectorError::FailedToObtainAuthType.into()),
        }
    }
}
// PaymentsResponse
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RazorpayPaymentsResponse {
    // status: RazorpayPaymentStatus,
    // id: String,
    contents: Contents,
    tag: Tag,
    code: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Tag {
    Stateless,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiMetadata {
    ext_api_tag: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Contents {
    send_webhook: bool,                          //
    second_factor: Option<SecondFactorResponse>, //
    pgr_response: Option<String>,
    api_metadata: ApiMetadata,
    pgr_info: PgrInfo,
    txn_status: TxnStatus, //
    #[serde(rename = "updatePGR")]
    update_pgr: bool, //
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SecondFactorResponse {
    authentication_account_id: Option<String>,
    can_accept_response: bool,
    challenges_attempted: u32,
    date_created: String,
    epg_txn_id: String,
    gateway_auth_req_params: Option<String>,
    id: u64,
    last_updated: String,
    partition_key: Option<String>,
    response_attempted: u32,
    status: String,
    txn_detail_id: Option<String>,
    txn_id: String,
    #[serde(rename = "type")]
    sf_type: String,
    url: String,
    version: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PgrInfo {
    resp_code: String,
    resp_message: Option<String>,
    response_xml: String,
    resptype: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TxnStatus {
    PendingVbv,
    Authorizing,
}

impl From<TxnStatus> for enums::AttemptStatus {
    fn from(item: TxnStatus) -> Self {
        match item {
            TxnStatus::Authorizing => Self::Charged,
            TxnStatus::PendingVbv => Self::Pending,
        }
    }
}

impl<F, T>
    TryFrom<types::ResponseRouterData<F, RazorpayPaymentsResponse, T, types::PaymentsResponseData>>
    for types::RouterData<F, T, types::PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<
            F,
            RazorpayPaymentsResponse,
            T,
            types::PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        let second_factor = item.response.contents.second_factor;
        match second_factor {
            Some(second_factor) => Ok(Self {
                status: enums::AttemptStatus::from(item.response.contents.txn_status),
                response: Ok(types::PaymentsResponseData::TransactionResponse {
                    resource_id: types::ResponseId::ConnectorTransactionId(second_factor.txn_id),
                    redirection_data: None,
                    mandate_reference: None,
                    connector_metadata: None,
                    network_txn_id: None,
                    connector_response_reference_id: None,
                    incremental_authorization_allowed: None,
                    charge_id: None,
                }),
                ..item.data
            }),
            None => Ok(Self {
                status: enums::AttemptStatus::from(item.response.contents.txn_status),
                response: Ok(types::PaymentsResponseData::TransactionResponse {
                    resource_id: types::ResponseId::NoResponseId,
                    redirection_data: None,
                    mandate_reference: None,
                    connector_metadata: None,
                    network_txn_id: None,
                    connector_response_reference_id: None,
                    incremental_authorization_allowed: None,
                    charge_id: None,
                }),
                ..item.data
            }),
        }
    }
}

//TODO: Fill the struct with respective fields
// REFUND :
// Type definition for RefundRequest
#[derive(Default, Debug, Serialize)]
pub struct RazorpayRefundRequest {
    pub amount: i64,
}

impl<F> TryFrom<&RazorpayRouterData<&types::RefundsRouterData<F>>> for RazorpayRefundRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &RazorpayRouterData<&types::RefundsRouterData<F>>,
    ) -> Result<Self, Self::Error> {
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

impl TryFrom<types::RefundsResponseRouterData<api::Execute, RefundResponse>>
    for types::RefundsRouterData<api::Execute>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::RefundsResponseRouterData<api::Execute, RefundResponse>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(types::RefundsResponseData {
                connector_refund_id: item.response.id.to_string(),
                refund_status: enums::RefundStatus::from(item.response.status),
            }),
            ..item.data
        })
    }
}

impl TryFrom<types::RefundsResponseRouterData<api::RSync, RefundResponse>>
    for types::RefundsRouterData<api::RSync>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::RefundsResponseRouterData<api::RSync, RefundResponse>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(types::RefundsResponseData {
                connector_refund_id: item.response.id.to_string(),
                refund_status: enums::RefundStatus::from(item.response.status),
            }),
            ..item.data
        })
    }
}

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
pub struct RazorpayErrorResponse {
    pub code: u16,
    pub error_code: Option<String>,
    pub status: String,
    pub error: bool,
    pub error_message: String,
    pub error_info: ErrorInfo,
}

#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
pub struct ErrorInfo {
    pub code: String,
    pub user_message: String,
    pub developer_message: String,
    pub fields: Vec<Fields>,
}

#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
pub struct Fields {
    pub field_name: String,
    pub reason: String,
}

#[derive(Debug, Serialize, Eq, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum RazorpayWebhookEventType {
    Disabled,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RazorpayWebhookEvent {
    pub payload: RazorpayWebhookPayload,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RazorpayWebhookPayload {
    pub refund: Option<RazorpayRefundWebhookPayload>,
    pub payment: RazorpayPaymentWebhookPayload,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RazorpayPaymentWebhookPayload {
    pub entity: WebhookPaymentEntity,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RazorpayRefundWebhookPayload {
    pub entity: WebhookRefundEntity,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WebhookRefundEntity {
    pub id: String,
    pub status: RazorpayRefundStatus,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WebhookPaymentEntity {
    pub id: String,
    pub status: RazorpayPaymentStatus,
}

#[derive(Debug, Serialize, Eq, PartialEq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RazorpayPaymentStatus {
    Created,
    Authorized,
    Captured,
    // Refunded,
    Failed,
}

#[derive(Debug, Serialize, Eq, PartialEq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RazorpayRefundStatus {
    Pending,
    Processed,
    Failed,
}

impl TryFrom<RazorpayWebhookPayload> for api_models::webhooks::IncomingWebhookEvent {
    type Error = errors::ConnectorError;
    fn try_from(webhook_payload: RazorpayWebhookPayload) -> Result<Self, Self::Error> {
        webhook_payload.refund.map_or(
            match webhook_payload.payment.entity.status {
                RazorpayPaymentStatus::Created => {
                    Some(api_models::webhooks::IncomingWebhookEvent::PaymentIntentProcessing)
                }
                RazorpayPaymentStatus::Authorized => {
                    Some(api_models::webhooks::IncomingWebhookEvent::PaymentIntentAuthorizationSuccess)
                }
                RazorpayPaymentStatus::Captured => {
                    Some(api_models::webhooks::IncomingWebhookEvent::PaymentIntentSuccess)
                }
                RazorpayPaymentStatus::Failed => {
                    Some(api_models::webhooks::IncomingWebhookEvent::PaymentIntentFailure)
                }
            },
            |refund_data| match refund_data.entity.status {
                RazorpayRefundStatus::Pending => {
                    None
                }
                RazorpayRefundStatus::Processed => {
                    Some(api_models::webhooks::IncomingWebhookEvent::RefundSuccess)
                }
                RazorpayRefundStatus::Failed => {
                    Some(api_models::webhooks::IncomingWebhookEvent::RefundFailure)
                }
            },
        ).ok_or(errors::ConnectorError::WebhookEventTypeNotFound)
    }
}
