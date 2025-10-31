use scylla::*;

#[derive(Debug)]
pub enum CassTypeError {
    DeserializeError
}

#[derive(Debug)]
pub enum CassInsertError {
    InsertError
}

#[derive(Debug)]
pub enum CassFindError {
    FindError
}

#[async_trait::async_trait]
pub trait CassStuff: Sized {
    type ColType;
    type RowType;
    type Store;
    type PartitionKey : IsPartitionKey<Self>;
    fn get_prepared_values(&self) -> Vec<Self::ColType>;
    fn from_row(row: Self::RowType) -> Result<Self, CassTypeError>;
    fn table_name() -> &'static str;
    fn keyspace() -> &'static str;
    fn get_insert_cql_statement() -> &'static str;
    fn get_cql_string() -> &'static str;

    //insert
    async fn insert(self, store : &Self::Store) -> Result<(), CassInsertError>;

    //Find
    async fn find(store : &Store, partition_key: Self::PartitionKey, cluster_key : Option<String>) -> Result<Self, CassFindError>;
}

pub trait IsPartitionKey<M : CassStuff>{
    fn find_query_builder(self, cluster_key : Option<String>, query_builder : &mut FindQueryBuilder);
}

struct FindQueryBuilder{
    query : Vec<String>,
    // this must be over T
    binds : Vec<String>,
}

impl FindQueryBuilder{
    fn build_query(self) -> String{
        self.query.join(" ")
    }
}

use diesel_models::PaymentIntent;
use crate::redis::kv_store::PartitionKey;

use scylla::frame::response::row::Row;
use scylla::frame::value::Value;

impl PaymentIntent for CassStuff{
    type RowType = Row;
    type ColType = Value;
    type PartitionKey : PartitionKey;
    type Store = scylla::Session

    async fn insert(self, store : &Self::Store) -> Result<(), CassInsertError>{
        let query = Self::get_cql_string();
        store.query_unpaged(query,self.get_prepared_values().into()).await
            .map_err(|e| println!("insert query error {:?}", e))
    }

    fn get_prepared_values(&self) -> Vec<Value> {
        vec![
            Value::Text(self.payment_id.to_string()),
            Value::Text(self.merchant_id.to_string()),
            Value::Text(self.status.to_string()),
            Value::BigInt(self.amount.into()),
            Value::Text(self.currency.as_ref().map_or("".to_string(), |c| c.to_string())),
            Value::BigInt(self.amount_captured.map_or(0, |a| a.into())),
            Value::Text(self.customer_id.as_ref().map_or("".to_string(), |c| c.to_string())),
            Value::Text(self.description.clone().unwrap_or_default()),
            Value::Text(self.return_url.clone().unwrap_or_default()),
            Value::Text(serde_json::to_string(&self.metadata).unwrap_or_default()),
            Value::Text(self.connector_id.clone().unwrap_or_default()),
            Value::Text(self.shipping_address_id.clone().unwrap_or_default()),
            Value::Text(self.billing_address_id.clone().unwrap_or_default()),
            Value::Text(self.statement_descriptor_name.clone().unwrap_or_default()),
            Value::Text(self.statement_descriptor_suffix.clone().unwrap_or_default()),
            Value::Timestamp(self.created_at.assume_utc().unix_timestamp()),
            Value::Timestamp(self.modified_at.assume_utc().unix_timestamp()),
            Value::Timestamp(self.last_synced.map_or(0, |t| t.assume_utc().unix_timestamp())),
            Value::Text(self.setup_future_usage.as_ref().map_or("".to_string(), |s| s.to_string())),
            Value::Boolean(self.off_session.unwrap_or(false)),
            Value::Text(self.client_secret.clone().unwrap_or_default()),
            Value::Text(self.active_attempt_id.clone()),
            Value::Text(self.business_country.as_ref().map_or("".to_string(), |c| c.to_string())),
            Value::Text(self.business_label.clone().unwrap_or_default()),
            Value::Text(serde_json::to_string(&self.order_details).unwrap_or_default()),
            Value::Text(serde_json::to_string(&self.allowed_payment_method_types).unwrap_or_default()),
            Value::Text(serde_json::to_string(&self.connector_metadata).unwrap_or_default()),
            Value::Text(serde_json::to_string(&self.feature_metadata).unwrap_or_default()),
            Value::SmallInt(self.attempt_count),
            Value::Text(self.profile_id.as_ref().map_or("".to_string(), |p| p.to_string())),
            Value::Text(self.merchant_decision.clone().unwrap_or_default()),
            Value::Text(self.payment_link_id.clone().unwrap_or_default()),
            Value::Text(self.payment_confirm_source.as_ref().map_or("".to_string(), |s| s.to_string())),
            Value::Text(self.updated_by.clone()),
            Value::Boolean(self.surcharge_applicable.unwrap_or(false)),
            Value::Text(self.request_incremental_authorization.as_ref().map_or("".to_string(), |r| r.to_string())),
            Value::Boolean(self.incremental_authorization_allowed.unwrap_or(false)),
            Value::Int(self.authorization_count.unwrap_or(0)),
            Value::Timestamp(self.session_expiry.map_or(0, |t| t.assume_utc().unix_timestamp())),
            Value::Text(self.fingerprint_id.clone().unwrap_or_default()),
            Value::Boolean(self.request_external_three_ds_authentication.unwrap_or(false)),
            Value::Text(serde_json::to_string(&self.charges).unwrap_or_default()),
            Value::Text(serde_json::to_string(&self.frm_metadata).unwrap_or_default()),
            Value::Text(serde_json::to_string(&self.customer_details).unwrap_or_default()),
            Value::Text(serde_json::to_string(&self.billing_details).unwrap_or_default()),
            Value::Text(self.merchant_order_reference_id.clone().unwrap_or_default()),
            Value::Text(serde_json::to_string(&self.shipping_details).unwrap_or_default()),
            Value::Boolean(self.is_payment_processor_token_flow.unwrap_or(false)),
            Value::BigInt(self.shipping_cost.map_or(0, |s| s.into())),
            Value::Text(self.organization_id.to_string()),
            Value::Text(serde_json::to_string(&self.tax_details).unwrap_or_default()),
            Value::Boolean(self.skip_external_tax_calculation.unwrap_or(false)),
            Value::Boolean(self.request_extended_authorization.map_or(false, |r| r.into())),
            Value::Text(self.psd2_sca_exemption_type.as_ref().map_or("".to_string(), |p| p.to_string())),
            Value::Text(serde_json::to_string(&self.split_payments).unwrap_or_default()),
            Value::Text(self.platform_merchant_id.as_ref().map_or("".to_string(), |p| p.to_string())),
            Value::Boolean(self.force_3ds_challenge.unwrap_or(false)),
            Value::Boolean(self.force_3ds_challenge_trigger.unwrap_or(false)),
            Value::Text(self.processor_merchant_id.as_ref().map_or("".to_string(), |p| p.to_string())),
            Value::Text(self.created_by.clone().unwrap_or_default()),
            Value::Boolean(self.is_iframe_redirection_enabled.unwrap_or(false)),
            Value::Text(self.extended_return_url.clone().unwrap_or_default()),
            Value::Boolean(self.is_payment_id_from_merchant.unwrap_or(false)),
            Value::Text(self.payment_channel.as_ref().map_or("".to_string(), |p| p.to_string())),
            Value::Text(self.tax_status.as_ref().map_or("".to_string(), |t| t.to_string())),
            Value::BigInt(self.discount_amount.map_or(0, |d| d.into())),
            Value::BigInt(self.shipping_amount_tax.map_or(0, |s| s.into())),
            Value::BigInt(self.duty_amount.map_or(0, |d| d.into())),
            Value::Timestamp(self.order_date.map_or(0, |t| t.assume_utc().unix_timestamp())),
            Value::Boolean(self.enable_partial_authorization.map_or(false, |e| e.into())),
            Value::Boolean(self.enable_overcapture.map_or(false, |e| e.into())),
            Value::Text(self.mit_category.as_ref().map_or("".to_string(), |m| m.to_string())),
        ]
    }

    fn from_row(row: &Self::RowType) -> Result<Self, FromCqlValError> {
        let payment_id = String::from_cql(row.columns[0].as_ref().ok_or(FromCqlValError::BadCqlType)?)?;
        let merchant_id = String::from_cql(row.columns[1].as_ref().ok_or(FromCqlValError::BadCqlType)?)?;
        let status = String::from_cql(row.columns[2].as_ref().ok_or(FromCqlValError::BadCqlType)?)?;
        let amount = i64::from_cql(row.columns[3].as_ref().ok_or(FromCqlValError::BadCqlType)?)?;
        let currency = String::from_cql(row.columns[4].as_ref().ok_or(FromCqlValError::BadCqlType)?)?;
        let amount_captured = i64::from_cql(row.columns[5].as_ref().ok_or(FromCqlValError::BadCqlType)?)?;
        let customer_id = String::from_cql(row.columns[6].as_ref().ok_or(FromCqlValError::BadCqlType)?)?;
        let description = String::from_cql(row.columns[7].as_ref().ok_or(FromCqlValError::BadCqlType)?)?;
        let return_url = String::from_cql(row.columns[8].as_ref().ok_or(FromCqlValError::BadCqlType)?)?;
        let metadata = String::from_cql(row.columns[9].as_ref().ok_or(FromCqlValError::BadCqlType)?)?;
        let connector_id = String::from_cql(row.columns[10].as_ref().ok_or(FromCqlValError::BadCqlType)?)?;
        let shipping_address_id = String::from_cql(row.columns[11].as_ref().ok_or(FromCqlValError::BadCqlType)?)?;
        let billing_address_id = String::from_cql(row.columns[12].as_ref().ok_or(FromCqlValError::BadCqlType)?)?;
        let statement_descriptor_name = String::from_cql(row.columns[13].as_ref().ok_or(FromCqlValError::BadCqlType)?)?;
        let statement_descriptor_suffix = String::from_cql(row.columns[14].as_ref().ok_or(FromCqlValError::BadCqlType)?)?;
        let created_at = i64::from_cql(row.columns[15].as_ref().ok_or(FromCqlValError::BadCqlType)?)?;
        let modified_at = i64::from_cql(row.columns[16].as_ref().ok_or(FromCqlValError::BadCqlType)?)?;
        let last_synced = i64::from_cql(row.columns[17].as_ref().ok_or(FromCqlValError::BadCqlType)?)?;
        let setup_future_usage = String::from_cql(row.columns[18].as_ref().ok_or(FromCqlValError::BadCqlType)?)?;
        let off_session = bool::from_cql(row.columns[19].as_ref().ok_or(FromCqlValError::BadCqlType)?)?;
        let client_secret = String::from_cql(row.columns[20].as_ref().ok_or(FromCqlValError::BadCqlType)?)?;
        let active_attempt_id = String::from_cql(row.columns[21].as_ref().ok_or(FromCqlValError::BadCqlType)?)?;
        let business_country = String::from_cql(row.columns[22].as_ref().ok_or(FromCqlValError::BadCqlType)?)?;
        let business_label = String::from_cql(row.columns[23].as_ref().ok_or(FromCqlValError::BadCqlType)?)?;
        let order_details = String::from_cql(row.columns[24].as_ref().ok_or(FromCqlValError::BadCqlType)?)?;
        let allowed_payment_method_types = String::from_cql(row.columns[25].as_ref().ok_or(FromCqlValError::BadCqlType)?)?;
        let connector_metadata = String::from_cql(row.columns[26].as_ref().ok_or(FromCqlValError::BadCqlType)?)?;
        let feature_metadata = String::from_cql(row.columns[27].as_ref().ok_or(FromCqlValError::BadCqlType)?)?;
        let attempt_count = i16::from_cql(row.columns[28].as_ref().ok_or(FromCqlValError::BadCqlType)?)?;
        let profile_id = String::from_cql(row.columns[29].as_ref().ok_or(FromCqlValError::BadCqlType)?)?;
        let merchant_decision = String::from_cql(row.columns[30].as_ref().ok_or(FromCqlValError::BadCqlType)?)?;
        let payment_link_id = String::from_cql(row.columns[31].as_ref().ok_or(FromCqlValError::BadCqlType)?)?;
        let payment_confirm_source = String::from_cql(row.columns[32].as_ref().ok_or(FromCqlValError::BadCqlType)?)?;
        let updated_by = String::from_cql(row.columns[33].as_ref().ok_or(FromCqlValError::BadCqlType)?)?;
        let surcharge_applicable = bool::from_cql(row.columns[34].as_ref().ok_or(FromCqlValError::BadCqlType)?)?;
        let request_incremental_authorization = String::from_cql(row.columns[35].as_ref().ok_or(FromCqlValError::BadCqlType)?)?;
        let incremental_authorization_allowed = bool::from_cql(row.columns[36].as_ref().ok_or(FromCqlValError::BadCqlType)?)?;
        let authorization_count = i32::from_cql(row.columns[37].as_ref().ok_or(FromCqlValError::BadCqlType)?)?;
        let session_expiry = i64::from_cql(row.columns[38].as_ref().ok_or(FromCqlValError::BadCqlType)?)?;
        let fingerprint_id = String::from_cql(row.columns[39].as_ref().ok_or(FromCqlValError::BadCqlType)?)?;
        let request_external_three_ds_authentication = bool::from_cql(row.columns[40].as_ref().ok_or(FromCqlValError::BadCqlType)?)?;
        let charges = String::from_cql(row.columns[41].as_ref().ok_or(FromCqlValError::BadCqlType)?)?;
        let frm_metadata = String::from_cql(row.columns[42].as_ref().ok_or(FromCqlValError::BadCqlType)?)?;
        let customer_details = String::from_cql(row.columns[43].as_ref().ok_or(FromCqlValError::BadCqlType)?)?;
        let billing_details = String::from_cql(row.columns[44].as_ref().ok_or(FromCqlValError::BadCqlType)?)?;
        let merchant_order_reference_id = String::from_cql(row.columns[45].as_ref().ok_or(FromCqlValError::BadCqlType)?)?;
        let shipping_details = String::from_cql(row.columns[46].as_ref().ok_or(FromCqlValError::BadCqlType)?)?;
        let is_payment_processor_token_flow = bool::from_cql(row.columns[47].as_ref().ok_or(FromCqlValError::BadCqlType)?)?;
        let shipping_cost = i64::from_cql(row.columns[48].as_ref().ok_or(FromCqlValError::BadCqlType)?)?;
        let organization_id = String::from_cql(row.columns[49].as_ref().ok_or(FromCqlValError::BadCqlType)?)?;
        let tax_details = String::from_cql(row.columns[50].as_ref().ok_or(FromCqlValError::BadCqlType)?)?;
        let skip_external_tax_calculation = bool::from_cql(row.columns[51].as_ref().ok_or(FromCqlValError::BadCqlType)?)?;
        let request_extended_authorization = bool::from_cql(row.columns[52].as_ref().ok_or(FromCqlValError::BadCqlType)?)?;
        let psd2_sca_exemption_type = String::from_cql(row.columns[53].as_ref().ok_or(FromCqlValError::BadCqlType)?)?;
        let split_payments = String::from_cql(row.columns[54].as_ref().ok_or(FromCqlValError::BadCqlType)?)?;
        let platform_merchant_id = String::from_cql(row.columns[55].as_ref().ok_or(FromCqlValError::BadCqlType)?)?;
        let force_3ds_challenge = bool::from_cql(row.columns[56].as_ref().ok_or(FromCqlValError::BadCqlType)?)?;
        let force_3ds_challenge_trigger = bool::from_cql(row.columns[57].as_ref().ok_or(FromCqlValError::BadCqlType)?)?;
        let processor_merchant_id = String::from_cql(row.columns[58].as_ref().ok_or(FromCqlValError::BadCqlType)?)?;
        let created_by = String::from_cql(row.columns[59].as_ref().ok_or(FromCqlValError::BadCqlType)?)?;
        let is_iframe_redirection_enabled = bool::from_cql(row.columns[60].as_ref().ok_or(FromCqlValError::BadCqlType)?)?;
        let extended_return_url = String::from_cql(row.columns[61].as_ref().ok_or(FromCqlValError::BadCqlType)?)?;
        let is_payment_id_from_merchant = bool::from_cql(row.columns[62].as_ref().ok_or(FromCqlValError::BadCqlType)?)?;
        let payment_channel = String::from_cql(row.columns[63].as_ref().ok_or(FromCqlValError::BadCqlType)?)?;
        let tax_status = String::from_cql(row.columns[64].as_ref().ok_or(FromCqlValError::BadCqlType)?)?;
        let discount_amount = i64::from_cql(row.columns[65].as_ref().ok_or(FromCqlValError::BadCqlType)?)?;
        let shipping_amount_tax = i64::from_cql(row.columns[66].as_ref().ok_or(FromCqlValError::BadCqlType)?)?;
        let duty_amount = i64::from_cql(row.columns[67].as_ref().ok_or(FromCqlValError::BadCqlType)?)?;
        let order_date = i64::from_cql(row.columns[68].as_ref().ok_or(FromCqlValError::BadCqlType)?)?;
        let enable_partial_authorization = bool::from_cql(row.columns[69].as_ref().ok_or(FromCqlValError::BadCqlType)?)?;
        let enable_overcapture = bool::from_cql(row.columns[70].as_ref().ok_or(FromCqlValError::BadCqlType)?)?;
        let mit_category = String::from_cql(row.columns[71].as_ref().ok_or(FromCqlValError::BadCqlType)?)?;

        Ok(Self {
            payment_id: payment_id.parse()?,
            merchant_id: merchant_id.parse()?,
            status: status.parse()?,
            amount: MinorUnit::new(amount),
            currency: Some(currency.parse()?),
            amount_captured: if amount_captured == 0 { None } else { Some(MinorUnit::new(amount_captured)) },
            customer_id: if customer_id.is_empty() { None } else { Some(customer_id.parse()?) },
            description: if description.is_empty() { None } else { Some(description) },
            return_url: if return_url.is_empty() { None } else { Some(return_url) },
            metadata: if metadata.is_empty() { None } else { serde_json::from_str(&metadata).ok() },
            connector_id: if connector_id.is_empty() { None } else { Some(connector_id) },
            shipping_address_id: if shipping_address_id.is_empty() { None } else { Some(shipping_address_id) },
            billing_address_id: if billing_address_id.is_empty() { None } else { Some(billing_address_id) },
            statement_descriptor_name: if statement_descriptor_name.is_empty() { None } else { Some(statement_descriptor_name) },
            statement_descriptor_suffix: if statement_descriptor_suffix.is_empty() { None } else { Some(statement_descriptor_suffix) },
            created_at: PrimitiveDateTime::from_unix_timestamp(created_at)?,
            modified_at: PrimitiveDateTime::from_unix_timestamp(modified_at)?,
            last_synced: if last_synced == 0 { None } else { Some(PrimitiveDateTime::from_unix_timestamp(last_synced)?) },
            setup_future_usage: if setup_future_usage.is_empty() { None } else { Some(setup_future_usage.parse()?) },
            off_session: Some(off_session),
            client_secret: if client_secret.is_empty() { None } else { Some(client_secret) },
            active_attempt_id: active_attempt_id,
            business_country: if business_country.is_empty() { None } else { Some(business_country.parse()?) },
            business_label: if business_label.is_empty() { None } else { Some(business_label) },
            order_details: if order_details.is_empty() { None } else { serde_json::from_str(&order_details).ok() },
            allowed_payment_method_types: if allowed_payment_method_types.is_empty() { None } else { serde_json::from_str(&allowed_payment_method_types).ok() },
            connector_metadata: if connector_metadata.is_empty() { None } else { serde_json::from_str(&connector_metadata).ok() },
            feature_metadata: if feature_metadata.is_empty() { None } else { serde_json::from_str(&feature_metadata).ok() },
            attempt_count,
            profile_id: if profile_id.is_empty() { None } else { Some(profile_id.parse()?) },
            merchant_decision: if merchant_decision.is_empty() { None } else { Some(merchant_decision) },
            payment_link_id: if payment_link_id.is_empty() { None } else { Some(payment_link_id) },
            payment_confirm_source: if payment_confirm_source.is_empty() { None } else { Some(payment_confirm_source.parse()?) },
            updated_by,
            surcharge_applicable: Some(surcharge_applicable),
            request_incremental_authorization: if request_incremental_authorization.is_empty() { None } else { Some(request_incremental_authorization.parse()?) },
            incremental_authorization_allowed: Some(incremental_authorization_allowed),
            authorization_count: Some(authorization_count),
            session_expiry: if session_expiry == 0 { None } else { Some(PrimitiveDateTime::from_unix_timestamp(session_expiry)?) },
            fingerprint_id: if fingerprint_id.is_empty() { None } else { Some(fingerprint_id) },
            request_external_three_ds_authentication: Some(request_external_three_ds_authentication),
            charges: if charges.is_empty() { None } else { serde_json::from_str(&charges).ok() },
            frm_metadata: if frm_metadata.is_empty() { None } else { serde_json::from_str(&frm_metadata).ok() },
            customer_details: if customer_details.is_empty() { None } else { serde_json::from_str(&customer_details).ok() },
            billing_details: if billing_details.is_empty() { None } else { serde_json::from_str(&billing_details).ok() },
            merchant_order_reference_id: if merchant_order_reference_id.is_empty() { None } else { Some(merchant_order_reference_id) },
            shipping_details: if shipping_details.is_empty() { None } else { serde_json::from_str(&shipping_details).ok() },
            is_payment_processor_token_flow: Some(is_payment_processor_token_flow),
            shipping_cost: if shipping_cost == 0 { None } else { Some(MinorUnit::new(shipping_cost)) },
            organization_id: organization_id.parse()?,
            tax_details: if tax_details.is_empty() { None } else { serde_json::from_str(&tax_details).ok() },
            skip_external_tax_calculation: Some(skip_external_tax_calculation),
            request_extended_authorization: Some(request_extended_authorization.into()),
            psd2_sca_exemption_type: if psd2_sca_exemption_type.is_empty() { None } else { Some(psd2_sca_exemption_type.parse()?) },
            split_payments: if split_payments.is_empty() { None } else { serde_json::from_str(&split_payments).ok() },
            platform_merchant_id: if platform_merchant_id.is_empty() { None } else { Some(platform_merchant_id.parse()?) },
            force_3ds_challenge: Some(force_3ds_challenge),
            force_3ds_challenge_trigger: Some(force_3ds_challenge_trigger),
            processor_merchant_id: if processor_merchant_id.is_empty() { None } else { Some(processor_merchant_id.parse()?) },
            created_by: if created_by.is_empty() { None } else { Some(created_by) },
            is_iframe_redirection_enabled: Some(is_iframe_redirection_enabled),
            extended_return_url: if extended_return_url.is_empty() { None } else { Some(extended_return_url) },
            is_payment_id_from_merchant: Some(is_payment_id_from_merchant),
            payment_channel: if payment_channel.is_empty() { None } else { Some(payment_channel.parse()?) },
            tax_status: if tax_status.is_empty() { None } else { Some(tax_status.parse()?) },
            discount_amount: if discount_amount == 0 { None } else { Some(MinorUnit::new(discount_amount)) },
            shipping_amount_tax: if shipping_amount_tax == 0 { None } else { Some(MinorUnit::new(shipping_amount_tax)) },
            duty_amount: if duty_amount == 0 { None } else { Some(MinorUnit::new(duty_amount)) },
            order_date: if order_date == 0 { None } else { Some(PrimitiveDateTime::from_unix_timestamp(order_date)?) },
            enable_partial_authorization: Some(enable_partial_authorization.into()),
            enable_overcapture: Some(enable_overcapture.into()),
            mit_category: if mit_category.is_empty() { None } else { Some(mit_category.parse()?) },
        })
    }

    fn table_name() -> &'static str{
        "payment_intent"
    }

    fn keyspace() -> &'static str{
        "trackers"
    }

    fn get_cql_string() -> &'static str {
        "CREATE TABLE IF NOT EXISTS payment_intent (
            payment_id text,
            merchant_id text,
            status text,
            amount bigint,
            currency text,
            amount_captured bigint,
            customer_id text,
            description text,
            return_url text,
            metadata text,
            connector_id text,
            shipping_address_id text,
            billing_address_id text,
            statement_descriptor_name text,
            statement_descriptor_suffix text,
            created_at timestamp,
            modified_at timestamp,
            last_synced timestamp,
            setup_future_usage text,
            off_session boolean,
            client_secret text,
            active_attempt_id text,
            business_country text,
            business_label text,
            order_details text,
            allowed_payment_method_types text,
            connector_metadata text,
            feature_metadata text,
            attempt_count smallint,
            profile_id text,
            merchant_decision text,
            payment_link_id text,
            payment_confirm_source text,
            updated_by text,
            surcharge_applicable boolean,
            request_incremental_authorization text,
            incremental_authorization_allowed boolean,
            authorization_count int,
            session_expiry timestamp,
            fingerprint_id text,
            request_external_three_ds_authentication boolean,
            charges text,
            frm_metadata text,
            customer_details text,
            billing_details text,
            merchant_order_reference_id text,
            shipping_details text,
            is_payment_processor_token_flow boolean,
            shipping_cost bigint,
            organization_id text,
            tax_details text,
            skip_external_tax_calculation boolean,
            request_extended_authorization boolean,
            psd2_sca_exemption_type text,
            split_payments text,
            platform_merchant_id text,
            force_3ds_challenge boolean,
            force_3ds_challenge_trigger boolean,
            processor_merchant_id text,
            created_by text,
            is_iframe_redirection_enabled boolean,
            extended_return_url text,
            is_payment_id_from_merchant boolean,
            payment_channel text,
            tax_status text,
            discount_amount bigint,
            shipping_amount_tax bigint,
            duty_amount bigint,
            order_date timestamp,
            enable_partial_authorization boolean,
            enable_overcapture boolean,
            mit_category text,
            PRIMARY KEY ((payment_id, merchant_id))
        )"
    }

    fn get_insert_cql_statement() -> &'static str {
        "INSERT INTO payment_intent (
            payment_id,
            merchant_id,
            status,
            amount,
            currency,
            amount_captured,
            customer_id,
            description,
            return_url,
            metadata,
            connector_id,
            shipping_address_id,
            billing_address_id,
            statement_descriptor_name,
            statement_descriptor_suffix,
            created_at,
            modified_at,
            last_synced,
            setup_future_usage,
            off_session,
            client_secret,
            active_attempt_id,
            business_country,
            business_label,
            order_details,
            allowed_payment_method_types,
            connector_metadata,
            feature_metadata,
            attempt_count,
            profile_id,
            merchant_decision,
            payment_link_id,
            payment_confirm_source,
            updated_by,
            surcharge_applicable,
            request_incremental_authorization,
            incremental_authorization_allowed,
            authorization_count,
            session_expiry,
            fingerprint_id,
            request_external_three_ds_authentication,
            charges,
            frm_metadata,
            customer_details,
            billing_details,
            merchant_order_reference_id,
            shipping_details,
            is_payment_processor_token_flow,
            shipping_cost,
            organization_id,
            tax_details,
            skip_external_tax_calculation,
            request_extended_authorization,
            psd2_sca_exemption_type,
            split_payments,
            platform_merchant_id,
            force_3ds_challenge,
            force_3ds_challenge_trigger,
            processor_merchant_id,
            created_by,
            is_iframe_redirection_enabled,
            extended_return_url,
            is_payment_id_from_merchant,
            payment_channel,
            tax_status,
            discount_amount,
            shipping_amount_tax,
            duty_amount,
            order_date,
            enable_partial_authorization,
            enable_overcapture,
            mit_category
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
    }

}   