use common_utils::{
    pii::{self, Email},
    types::FloatMajorUnit,
};
use error_stack::ResultExt;
use hyperswitch_domain_models::payment_method_data::UpiCollectData;
use masking::Secret;
use rand::Rng;
use serde::{Deserialize, Serialize};
use time::PrimitiveDateTime;

use crate::{
    core::errors,
    types::{self, api, domain, storage::enums},
};

//TODO: Fill the struct with respective fields
pub struct RazorpayRouterData<T> {
    pub amount: FloatMajorUnit, // The type of amount that a connector accepts, for example, String, i64, f64, etc.
    pub router_data: T,
}

impl<T> TryFrom<(FloatMajorUnit, T)> for RazorpayRouterData<T> {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from((amount, item): (FloatMajorUnit, T)) -> Result<Self, Self::Error> {
        Ok(Self {
            amount,
            router_data: item,
        })
    }
}
//TODO: Fill the struct with respective fields
#[derive(Debug, Serialize, PartialEq)]
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

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]

pub struct SecondFactor {
    txn_id: String,
    id: String,
    status: SecondFactorStatus,
    #[serde(rename = "type")]
    sf_type: String,
    version: i32,
    #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
    date_created: Option<PrimitiveDateTime>,
    #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
    last_updated: Option<PrimitiveDateTime>,
    transaction_id: Option<String>,
    url: Option<String>,
    epg_txn_id: Option<String>,
    transaction_detail_id: Option<String>,
    gateway_auth_required_params: Option<String>,
    authentication_account_id: Option<String>,
    can_accept_response: Option<bool>,
    challenges_attempted: Option<i32>,
    response_attempted: Option<i32>,
    partition_key: Option<String>,
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SecondFactorStatus {
    Pending,
    #[default]
    Init,
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MerchantAccount {
    id: i64,
    merchant_id: Secret<String>,
    use_code_for_gateway_priority: bool,
    auto_refund_multiple_charged_transactions: bool,
    gateway_success_rate_based_outage_input: Option<String>,
    gateway_success_rate_based_decider_input: Option<String>,
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

#[derive(Default, Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct OrderReference {
    id: String,
    amount: FloatMajorUnit,
    currency: String,
    order_id: String,
    status: OrderStatus,
    merchant_id: Secret<String>,
    order_uuid: String,
    order_type: String,
    version: i64,
    #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
    date_created: Option<PrimitiveDateTime>,
    #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
    last_modified: Option<PrimitiveDateTime>,
    return_url: Option<String>,
    billing_address_id: Option<String>,
    internal_metadata: Option<String>,
    mandate_feature: Option<MandateFeature>,
    udf6: Option<String>,
    udf1: Option<String>,
    partition_key: Option<String>,
    amount_refunded: Option<FloatMajorUnit>,
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

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum OrderStatus {
    #[default]
    PendingAuthentication,
}

#[derive(Default, Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TxnDetail {
    status: String,
    merchant_id: Secret<String>,
    txn_id: String,
    express_checkout: bool,
    is_emi: bool,
    net_amount: FloatMajorUnit,
    txn_amount: FloatMajorUnit,
    emi_tenure: i32,
    txn_uuid: String,
    currency: String,
    version: i64,
    redirect: bool,
    id: String,
    #[serde(rename = "type")]
    txn_type: String,
    order_id: String,
    add_to_locker: bool,
    merchant_gateway_account_id: i64,
    txn_mode: String,
    #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
    date_created: Option<PrimitiveDateTime>,
    #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
    last_modified: Option<PrimitiveDateTime>,
    gateway: Gateway,
    internal_metadata: Option<String>,
    txn_flow_type: Option<String>,
    source_object: Option<String>,
    partition_key: Option<String>,
    username: Option<String>,
    txn_object_type: Option<String>,
    internal_tracking_info: Option<String>,
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TxnCardInfo {
    txn_detail_id: String,
    txn_id: String,
    payment_method_type: String,
    id: String,
    payment_method: String,
    payment_source: Secret<String, pii::UpiVpaMaskingStrategy>,
    date_created: Option<PrimitiveDateTime>,
    partition_key: Option<String>,
    card_type: Option<String>,

    card_issuer_bank_name: Option<String>,
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MerchantGatewayAccount {
    merchant_id: Secret<String>,
    gateway: Gateway,
    account_details: String,
    version: i64,
    id: i64,
    test_mode: bool,
    #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
    date_created: Option<PrimitiveDateTime>,
    #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
    last_modified: Option<PrimitiveDateTime>,
    disabled: bool,
    payment_methods: Option<String>,

    enforce_payment_method_acceptance: Option<bool>,

    supported_payment_flows: Option<String>,
    is_juspay_account: Option<bool>,
}

#[derive(Debug, Serialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct TransactionCreateReq {
    merchant_id: Secret<String>,
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct OrderMetadataV2 {
    id: String,
    order_reference_id: String,
    #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
    date_created: Option<PrimitiveDateTime>,
    #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
    last_updated: Option<PrimitiveDateTime>,
    browser: Option<String>,
    operating_system: Option<String>,
    ip_address: Option<String>,
    partition_key: Option<String>,
    user_agent: Option<String>,
    browser_version: Option<String>,
    mobile: Option<String>,
    metadata: Option<String>,
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Gateway {
    #[default]
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

fn generate_12_digit_number() -> u64 {
    let mut rng = rand::thread_rng();
    rng.gen_range(100000000000..=999999999999)
}

impl TryFrom<&RazorpayRouterData<&types::PaymentsAuthorizeRouterData>> for RazorpayPaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &RazorpayRouterData<&types::PaymentsAuthorizeRouterData>,
    ) -> Result<Self, Self::Error> {
        let request = &item.router_data.request;
        let txn_card_info = match request.payment_method_data.clone() {
            domain::PaymentMethodData::Upi(upi_type) => match upi_type {
                domain::UpiData::UpiCollect(upi_data) => TxnCardInfo::try_from((item, upi_data)),
                hyperswitch_domain_models::payment_method_data::UpiData::UpiIntent(_) => Err(
                    errors::ConnectorError::NotImplemented("Payment methods".to_string()).into(),
                ),
            },
            _ => Err(errors::ConnectorError::NotImplemented("Payment methods".to_string()).into()),
        }?;
        let auth = RazorpayAuthType::try_from(&item.router_data.connector_auth_type)?;
        let second_factor = SecondFactor::try_from(item)?;
        let merchant_account = MerchantAccount::try_from(item)?;
        let order_reference = OrderReference::try_from(item)?; //payment_intent
        let txn_detail = TxnDetail::try_from(item)?; //payment_attempt

        let merchant_gateway_account = MerchantGatewayAccount::try_from(item)?;

        let gateway = Gateway::Razorpay;
        let transaction_create_req = TransactionCreateReq {
            merchant_id: auth.merchant_id,
        };
        let is_mesh_enabled = false;

        //payment_intent_meta_data
        let order_metadata_v2 = OrderMetadataV2::try_from(item)?;

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
    }
}

impl TryFrom<&RazorpayRouterData<&types::PaymentsAuthorizeRouterData>> for SecondFactor {
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(
        item: &RazorpayRouterData<&types::PaymentsAuthorizeRouterData>,
    ) -> Result<Self, Self::Error> {
        let ref_id = generate_12_digit_number();

        Ok(Self {
            txn_id: item.router_data.connector_request_reference_id.clone(),
            id: ref_id.to_string(),
            status: SecondFactorStatus::Pending,
            version: 0,
            sf_type: "VBV".to_string(),
            date_created: Some(common_utils::date_time::now()),
            last_updated: Some(common_utils::date_time::now()),
            ..Default::default()
        })
    }
}

impl TryFrom<&RazorpayRouterData<&types::PaymentsAuthorizeRouterData>> for MerchantAccount {
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(
        item: &RazorpayRouterData<&types::PaymentsAuthorizeRouterData>,
    ) -> Result<Self, Self::Error> {
        let auth = RazorpayAuthType::try_from(&item.router_data.connector_auth_type)?;
        Ok(Self {
            id: 205,
            merchant_id: auth.merchant_id,
            auto_refund_multiple_charged_transactions: false,
            use_code_for_gateway_priority: true,
            ..Default::default()
        })
    }
}

impl TryFrom<&RazorpayRouterData<&types::PaymentsAuthorizeRouterData>> for OrderReference {
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(
        item: &RazorpayRouterData<&types::PaymentsAuthorizeRouterData>,
    ) -> Result<Self, Self::Error> {
        let ref_id = generate_12_digit_number();
        let auth = RazorpayAuthType::try_from(&item.router_data.connector_auth_type)?;
        Ok(Self {
            id: ref_id.to_string(),
            amount: item.amount,
            currency: item.router_data.request.currency.to_string(),
            status: OrderStatus::PendingAuthentication,
            merchant_id: auth.merchant_id.clone(),
            order_id: item.router_data.connector_request_reference_id.clone(), //payment_id
            version: 1,
            order_type: "ORDER_PAYMENT".to_string(),
            order_uuid: item.router_data.connector_request_reference_id.clone(),
            date_created: Some(common_utils::date_time::now()),
            last_modified: Some(common_utils::date_time::now()),
            ..Default::default()
        })
    }
}

impl
    TryFrom<(
        &RazorpayRouterData<&types::PaymentsAuthorizeRouterData>,
        UpiCollectData,
    )> for TxnCardInfo
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        payment_data: (
            &RazorpayRouterData<&types::PaymentsAuthorizeRouterData>,
            UpiCollectData,
        ),
    ) -> Result<Self, Self::Error> {
        let item = payment_data.0;
        let upi_data = payment_data.1;
        let ref_id = generate_12_digit_number();
        Ok(TxnCardInfo {
            txn_detail_id: ref_id.to_string(),
            txn_id: item.router_data.connector_request_reference_id.clone(),
            payment_method_type: "UPI".to_string(),
            id: ref_id.to_string(),
            payment_method: "UPI".to_string(),
            payment_source: upi_data.vpa_id.ok_or(
                errors::ConnectorError::MissingRequiredField {
                    field_name: "vpa_id",
                },
            )?,
            date_created: Some(common_utils::date_time::now()),
            ..Default::default()
        })
    }
}

impl TryFrom<&RazorpayRouterData<&types::PaymentsAuthorizeRouterData>> for TxnDetail {
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(
        item: &RazorpayRouterData<&types::PaymentsAuthorizeRouterData>,
    ) -> Result<Self, Self::Error> {
        let ref_id = generate_12_digit_number();
        let auth = RazorpayAuthType::try_from(&item.router_data.connector_auth_type)?;
        Ok(Self {
            order_id: item.router_data.connector_request_reference_id.clone(), //payment_id
            express_checkout: false,
            txn_mode: "PROD".to_string(),
            merchant_id: auth.merchant_id,
            status: "PENDING_VBV".to_string(),
            net_amount: item.amount,
            txn_id: item.router_data.connector_request_reference_id.clone(),
            txn_amount: item.amount,
            emi_tenure: 0,
            txn_uuid: "mozjbTGtwWJ7EenZSBw".to_string(),
            id: ref_id.to_string(),
            merchant_gateway_account_id: 11476,
            txn_type: "AUTH_AND_SETTLE".to_string(),
            redirect: true,
            version: 0,
            add_to_locker: false,
            currency: item.router_data.request.currency.to_string(),
            is_emi: false,
            gateway: Gateway::Razorpay,
            date_created: Some(common_utils::date_time::now()),
            last_modified: Some(common_utils::date_time::now()),
            ..Default::default()
        })
    }
}

impl TryFrom<&RazorpayRouterData<&types::PaymentsAuthorizeRouterData>> for MerchantGatewayAccount {
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(
        item: &RazorpayRouterData<&types::PaymentsAuthorizeRouterData>,
    ) -> Result<Self, Self::Error> {
        let auth = RazorpayAuthType::try_from(&item.router_data.connector_auth_type)?;
        Ok(Self {
           merchant_id: auth.merchant_id,
                                gateway: Gateway::Razorpay,
                                disabled: false,
                                id: 46519,
                                account_details: "{\"razorpayId\": \"rzp_test_4UX9WwyEpxGkRv\",\"razorpaySecret\": \"4xzFIa6BEXNyhhHG6zdlm41B\"}".to_string(),
                                test_mode: false,
                               ..Default::default()
        })
    }
}
impl TryFrom<&RazorpayRouterData<&types::PaymentsAuthorizeRouterData>> for OrderMetadataV2 {
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(
        _item: &RazorpayRouterData<&types::PaymentsAuthorizeRouterData>,
    ) -> Result<Self, Self::Error> {
        let ref_id = generate_12_digit_number();
        Ok(Self {
            id: ref_id.to_string(),
            order_reference_id: ref_id.to_string(),
            date_created: Some(common_utils::date_time::now()),
            last_updated: Some(common_utils::date_time::now()),

            ..Default::default()
        })
    }
}

//TODO: Fill the struct with respective fields
// Auth Struct
pub struct RazorpayAuthType {
    pub(super) api_key: Secret<String>,
    pub(super) merchant_id: Secret<String>,
}

impl TryFrom<&types::ConnectorAuthType> for RazorpayAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &types::ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            types::ConnectorAuthType::BodyKey { api_key, key1 } => Ok(Self {
                api_key: api_key.to_owned(),
                merchant_id: key1.to_owned(),
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
    id: u64,
    #[serde(rename = "type")]
    sf_type: String,
    version: i32,
    date_created: String,
    last_updated: String,
    epg_txn_id: String,
    status: String,
    txn_id: String,
    authentication_account_id: Option<String>,
    can_accept_response: Option<bool>,
    challenges_attempted: Option<i32>,
    gateway_auth_req_params: Option<String>,
    partition_key: Option<String>,
    response_attempted: Option<i32>,
    txn_detail_id: Option<String>,
    url: Option<String>,
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
            TxnStatus::Authorizing => Self::Pending,
            TxnStatus::PendingVbv => Self::Failure,
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
                    resource_id: types::ResponseId::ConnectorTransactionId(
                        second_factor.epg_txn_id,
                    ),
                    redirection_data: None,
                    mandate_reference: None,
                    connector_metadata: None,
                    network_txn_id: None,
                    connector_response_reference_id: Some(second_factor.txn_id),
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

//psync
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RazorpayCreateSyncRequest {
    txn_detail: TxnDetail,
    merchant_gateway_account: MerchantGatewayAccount,
    order_reference: OrderReference,
    second_factor: SecondFactor,
    gateway_txn_data: GatewayTxnData,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GatewayTxnData {
    id: String,
    version: i32,
    gateway_data: String,
    gateway_status: String,
    match_status: String,
    txn_detail_id: String,
    #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
    date_created: Option<PrimitiveDateTime>,
    #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
    last_updated: Option<PrimitiveDateTime>,
}

impl TryFrom<RazorpayRouterData<&types::PaymentsSyncRouterData>> for RazorpayCreateSyncRequest {
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(
        item: RazorpayRouterData<&types::PaymentsSyncRouterData>,
    ) -> Result<Self, Self::Error> {
        let ref_id = generate_12_digit_number();
        let auth = RazorpayAuthType::try_from(&item.router_data.connector_auth_type)?;
        let connector_request_reference_id = item
            .router_data
            .request
            .connector_transaction_id
            .get_connector_transaction_id()
            .change_context(errors::ConnectorError::MissingConnectorTransactionID)?;
        let txn_idd = &item.router_data.connector_request_reference_id;

        let second_factor = SecondFactor {
            txn_id: txn_idd.clone(),
            id: ref_id.to_string(),
            status: SecondFactorStatus::Pending,
            version: 0,
            sf_type: "VBV".to_string(),
            date_created: Some(common_utils::date_time::now()),
            last_updated: Some(common_utils::date_time::now()),
            epg_txn_id: Some(connector_request_reference_id.clone()),
            ..Default::default()
        };
        let order_reference = OrderReference {
            id: ref_id.to_string(),
            amount: item.amount.clone(),
            currency: item.router_data.request.currency.to_string(),
            status: OrderStatus::PendingAuthentication,
            merchant_id: auth.merchant_id.clone(),
            order_id: txn_idd.clone(), //payment_id
            version: 1,
            order_type: "ORDER_PAYMENT".to_string(),
            order_uuid: txn_idd.clone(),
            date_created: Some(common_utils::date_time::now()),
            last_modified: Some(common_utils::date_time::now()),
            ..Default::default()
        };
        let txn_detail = TxnDetail {
            order_id: txn_idd.clone(), //payment_id
            express_checkout: false,
            txn_mode: "PROD".to_string(),
            merchant_id: auth.merchant_id.clone(),
            status: "AUTHORIZING".to_string(),
            net_amount: item.amount.clone(),
            txn_id: txn_idd.clone(),
            txn_amount: item.amount.clone(),
            emi_tenure: 0,
            txn_uuid: "mozjbTGtwWJ7EenZSBw".to_string(),
            id: ref_id.to_string(),
            merchant_gateway_account_id: 11476,
            txn_type: "AUTH_AND_SETTLE".to_string(),
            redirect: true,
            version: 0,
            add_to_locker: false,
            currency: item.router_data.request.currency.to_string(),
            is_emi: false,
            gateway: Gateway::Razorpay,
            date_created: Some(common_utils::date_time::now()),
            last_modified: Some(common_utils::date_time::now()),
            ..Default::default()
        };
        let merchant_gateway_account = MerchantGatewayAccount{
              gateway: Gateway::Razorpay,
                                disabled: false,
                                id: 46519,
                                account_details: "{\"razorpayId\": \"rzp_test_4UX9WwyEpxGkRv\",\"razorpaySecret\": \"4xzFIa6BEXNyhhHG6zdlm41B\"}".to_string(),
                                test_mode: false,
                                merchant_id: auth.merchant_id,
                               ..Default::default()
         };
        let gateway_txn_data = GatewayTxnData {
            id: ref_id.to_string(),
            version: 0,
            gateway_data: "".to_string(),
            gateway_status: "S".to_string(),
            match_status: "S".to_string(),
            txn_detail_id: ref_id.to_string(),
            date_created: Some(common_utils::date_time::now()),
            last_updated: Some(common_utils::date_time::now()),
        };
        Ok(Self {
            second_factor,
            merchant_gateway_account,
            order_reference,
            txn_detail,
            gateway_txn_data,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RazorpaySyncResponse {
    status: RazorpapPsyncResponse,
    is_stateful: bool,
    second_factor: SecondFactorResponse,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]

pub enum RazorpapPsyncResponse {
    Charged,
    Pending,
    Authorizing,
}

impl From<RazorpapPsyncResponse> for enums::AttemptStatus {
    fn from(item: RazorpapPsyncResponse) -> Self {
        match item {
            RazorpapPsyncResponse::Charged => Self::Charged,
            RazorpapPsyncResponse::Pending => Self::Pending,
            RazorpapPsyncResponse::Authorizing => Self::Pending,
        }
    }
}

impl<F, T>
    TryFrom<types::ResponseRouterData<F, RazorpaySyncResponse, T, types::PaymentsResponseData>>
    for types::RouterData<F, T, types::PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<F, RazorpaySyncResponse, T, types::PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            status: enums::AttemptStatus::from(item.response.status),
            response: Ok(types::PaymentsResponseData::TransactionResponse {
                resource_id: types::ResponseId::ConnectorTransactionId(
                    item.response.second_factor.epg_txn_id,
                ),
                redirection_data: None,
                mandate_reference: None,
                connector_metadata: None,
                network_txn_id: None,
                connector_response_reference_id: Some(item.response.second_factor.txn_id),
                incremental_authorization_allowed: None,
                charge_id: None,
            }),
            ..item.data
        })
    }
}

//TODO: Fill the struct with respective fields
// REFUND :
// Type definition for RefundRequest
#[derive(Default, Debug, Serialize)]
pub struct RazorpayRefundRequest {
    pub amount: FloatMajorUnit,
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
