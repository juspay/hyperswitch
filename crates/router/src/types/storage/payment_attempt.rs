#[cfg(feature = "diesel")]
use diesel::{AsChangeset, Identifiable, Insertable, Queryable};
use serde::{Deserialize, Serialize};
use time::PrimitiveDateTime;

#[cfg(feature = "diesel")]
use crate::schema::payment_attempt;
use crate::types::enums;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "diesel", derive(Identifiable, Queryable))]
#[cfg_attr(feature = "diesel", diesel(table_name = payment_attempt))]
#[cfg_attr(feature = "sqlx", derive(sqlx::FromRow))]
pub struct PaymentAttempt {
    pub id: i32,
    pub payment_id: String,
    pub merchant_id: String,
    pub txn_id: String,
    pub status: enums::AttemptStatus,
    pub amount: i32,
    pub currency: Option<enums::Currency>,
    pub save_to_locker: Option<bool>,
    pub connector: String,
    pub error_message: Option<String>,
    pub offer_amount: Option<i32>,
    pub surcharge_amount: Option<i32>,
    pub tax_amount: Option<i32>,
    pub payment_method_id: Option<String>,
    pub payment_method: Option<enums::PaymentMethodType>,
    pub payment_flow: Option<enums::PaymentFlow>,
    pub redirect: Option<bool>,
    pub connector_transaction_id: Option<String>,
    pub capture_method: Option<enums::CaptureMethod>,
    pub capture_on: Option<PrimitiveDateTime>,
    pub confirm: bool,
    pub authentication_type: Option<enums::AuthenticationType>,
    pub created_at: PrimitiveDateTime,
    pub modified_at: PrimitiveDateTime,
    pub last_synced: Option<PrimitiveDateTime>,
    pub cancellation_reason: Option<String>,
    pub amount_to_capture: Option<i32>,
    pub mandate_id: Option<String>,
    pub browser_info: Option<serde_json::Value>,
}

#[cfg(feature = "sqlx")]
#[allow(clippy::needless_borrow)]
impl sqlx::encode::Encode<'_, sqlx::Postgres> for PaymentAttempt {
    fn encode_by_ref(&self, buf: &mut sqlx::postgres::PgArgumentBuffer) -> sqlx::encode::IsNull {
        let mut encoder = sqlx::postgres::types::PgRecordEncoder::new(buf);
        encoder.encode(&self.id);
        encoder.encode(&self.payment_id);
        encoder.encode(&self.merchant_id);
        encoder.encode(&self.txn_id);
        encoder.encode(&self.status);
        encoder.encode(&self.amount);
        encoder.encode(&self.currency);
        encoder.encode(&self.save_to_locker);
        encoder.encode(&self.connector);
        encoder.encode(&self.error_message);
        encoder.encode(&self.offer_amount);
        encoder.encode(&self.surcharge_amount);
        encoder.encode(&self.tax_amount);
        encoder.encode(&self.payment_method_id);
        encoder.encode(&self.payment_method);
        encoder.encode(&self.payment_flow);
        encoder.encode(&self.redirect);
        encoder.encode(&self.connector_transaction_id);
        encoder.encode(&self.capture_method);
        encoder.encode(&self.capture_on);
        encoder.encode(&self.confirm);
        encoder.encode(&self.authentication_type);
        encoder.encode(&self.created_at);
        encoder.encode(&self.modified_at);
        encoder.encode(&self.last_synced);
        encoder.finish();
        sqlx::encode::IsNull::No
    }
}

#[cfg(feature = "sqlx")]
impl<'r> sqlx::decode::Decode<'r, sqlx::Postgres> for PaymentAttempt {
    fn decode(
        value: sqlx::postgres::PgValueRef<'r>,
    ) -> Result<Self, Box<dyn std::error::Error + 'static + Send + Sync>> {
        let mut decoder = sqlx::postgres::types::PgRecordDecoder::new(value)?;
        let id = decoder.try_decode()?;
        let payment_id = decoder.try_decode()?;
        let merchant_id = decoder.try_decode()?;
        let txn_id = decoder.try_decode()?;
        let status = decoder.try_decode()?;
        let amount = decoder.try_decode()?;
        let currency = decoder.try_decode()?;
        let save_to_locker = decoder.try_decode()?;
        let connector = decoder.try_decode()?;
        let error_message = decoder.try_decode()?;
        let offer_amount = decoder.try_decode()?;
        let surcharge_amount = decoder.try_decode()?;
        let tax_amount = decoder.try_decode()?;
        let payment_method_id = decoder.try_decode()?;
        let payment_method = decoder.try_decode()?;
        let payment_flow = decoder.try_decode()?;
        let redirect = decoder.try_decode()?;
        let connector_transaction_id = decoder.try_decode()?;
        let capture_method = decoder.try_decode()?;
        let capture_on = decoder.try_decode()?;
        let confirm = decoder.try_decode()?;
        let authentication_type = decoder.try_decode()?;
        let created_at = decoder.try_decode()?;
        let modified_at = decoder.try_decode()?;
        let last_synced = decoder.try_decode()?;
        let cancellation_reason = decoder.try_decode()?;
        let amount_to_capture = decoder.try_decode()?;

        Ok(PaymentAttempt {
            id,
            payment_id,
            merchant_id,
            txn_id,
            status,
            amount,
            currency,
            save_to_locker,
            connector,
            error_message,
            offer_amount,
            surcharge_amount,
            tax_amount,
            payment_method_id,
            payment_method,
            payment_flow,
            redirect,
            connector_transaction_id,
            capture_method,
            capture_on,
            confirm,
            authentication_type,
            created_at,
            modified_at,
            last_synced,
            cancellation_reason,
            amount_to_capture,
            mandate_id: None,
            browser_info: None,
        })
    }
}

#[cfg(feature = "sqlx")]
impl sqlx::Type<sqlx::Postgres> for PaymentAttempt {
    fn type_info() -> sqlx::postgres::PgTypeInfo {
        sqlx::postgres::PgTypeInfo::with_name("PaymentAttempt")
    }
}

#[derive(Clone, Debug, Default, router_derive::DebugAsDisplay)]
#[cfg_attr(feature = "diesel", derive(Insertable))]
#[cfg_attr(feature = "diesel", diesel(table_name = payment_attempt))]
pub struct PaymentAttemptNew {
    pub payment_id: String,
    pub merchant_id: String,
    pub txn_id: String,
    pub status: enums::AttemptStatus,
    pub amount: i32,
    pub currency: Option<enums::Currency>,
    // pub auto_capture: Option<bool>,
    pub save_to_locker: Option<bool>,
    pub connector: String,
    pub error_message: Option<String>,
    pub offer_amount: Option<i32>,
    pub surcharge_amount: Option<i32>,
    pub tax_amount: Option<i32>,
    pub payment_method_id: Option<String>,
    pub payment_method: Option<enums::PaymentMethodType>,
    pub payment_flow: Option<enums::PaymentFlow>,
    pub redirect: Option<bool>,
    pub connector_transaction_id: Option<String>,
    pub capture_method: Option<enums::CaptureMethod>,
    pub capture_on: Option<PrimitiveDateTime>,
    pub confirm: bool,
    pub authentication_type: Option<enums::AuthenticationType>,
    pub created_at: Option<PrimitiveDateTime>,
    pub modified_at: Option<PrimitiveDateTime>,
    pub last_synced: Option<PrimitiveDateTime>,
    pub cancellation_reason: Option<String>,
    pub amount_to_capture: Option<i32>,
    pub mandate_id: Option<String>,
    pub browser_info: Option<serde_json::Value>,
}

#[cfg(feature = "sqlx")]
#[allow(clippy::needless_borrow)]
impl PaymentAttemptNew {
    fn insert_query(&self, table: &str) -> String {
        let sqlquery = format!("insert into {} ( {} ) values ( {} ) returning *",table,"payment_id , merchant_id , txn_id , status , amount , currency , save_to_locker , connector , error_message , offer_amount , surcharge_amount , tax_amount , payment_method_id , payment_method , payment_flow , redirect , connector_transaction_id , capture_method , capture_on , confirm , authentication_type , created_at , modified_at , last_synced","$1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18,$19,$20,$21,$22,$23,$24");
        sqlquery
    }

    pub async fn insert<T>(&self, pool: &sqlx::PgPool, table: &str) -> Result<T, sqlx::Error>
    where
        T: Send,
        T: for<'c> sqlx::FromRow<'c, sqlx::postgres::PgRow>,
        T: std::marker::Unpin,
    {
        let sql = self.insert_query(table);
        let res: T = sqlx::query_as::<_, T>(&sql)
            .bind(&self.payment_id)
            .bind(&self.merchant_id)
            .bind(&self.txn_id)
            .bind(&self.status)
            .bind(&self.amount)
            .bind(&self.currency)
            .bind(&self.save_to_locker)
            .bind(&self.connector)
            .bind(&self.error_message)
            .bind(&self.offer_amount)
            .bind(&self.surcharge_amount)
            .bind(&self.tax_amount)
            .bind(&self.payment_method_id)
            .bind(&self.payment_method)
            .bind(&self.payment_flow)
            .bind(&self.redirect)
            .bind(&self.connector_transaction_id)
            .bind(&self.capture_method)
            .bind(&self.capture_on)
            .bind(&self.confirm)
            .bind(&self.authentication_type)
            .bind(&self.created_at)
            .bind(&self.modified_at)
            .bind(&self.last_synced)
            .fetch_one(pool)
            .await?;
        Ok(res)
    }
}

#[cfg(feature = "sqlx")]
#[allow(clippy::needless_borrow)]
impl sqlx::encode::Encode<'_, sqlx::Postgres> for PaymentAttemptNew {
    fn encode_by_ref(&self, buf: &mut sqlx::postgres::PgArgumentBuffer) -> sqlx::encode::IsNull {
        let mut encoder = sqlx::postgres::types::PgRecordEncoder::new(buf);
        encoder.encode(&self.payment_id);
        encoder.encode(&self.merchant_id);
        encoder.encode(&self.txn_id);
        encoder.encode(&self.status);
        encoder.encode(&self.amount);
        encoder.encode(&self.currency);
        encoder.encode(&self.save_to_locker);
        encoder.encode(&self.connector);
        encoder.encode(&self.error_message);
        encoder.encode(&self.offer_amount);
        encoder.encode(&self.surcharge_amount);
        encoder.encode(&self.tax_amount);
        encoder.encode(&self.payment_method_id);
        encoder.encode(&self.payment_method);
        encoder.encode(&self.payment_flow);
        encoder.encode(&self.redirect);
        encoder.encode(&self.connector_transaction_id);
        encoder.encode(&self.capture_method);
        encoder.encode(&self.capture_on);
        encoder.encode(&self.confirm);
        encoder.encode(&self.authentication_type);
        encoder.encode(&self.created_at);
        encoder.encode(&self.modified_at);
        encoder.encode(&self.last_synced);
        encoder.finish();
        sqlx::encode::IsNull::No
    }
}

#[cfg(feature = "sqlx")]
impl<'r> sqlx::decode::Decode<'r, sqlx::Postgres> for PaymentAttemptNew {
    fn decode(
        value: sqlx::postgres::PgValueRef<'r>,
    ) -> Result<Self, Box<dyn std::error::Error + 'static + Send + Sync>> {
        let mut decoder = sqlx::postgres::types::PgRecordDecoder::new(value)?;
        let payment_id = decoder.try_decode()?;
        let merchant_id = decoder.try_decode()?;
        let txn_id = decoder.try_decode()?;
        let status = decoder.try_decode()?;
        let amount = decoder.try_decode()?;
        let currency = decoder.try_decode()?;
        let save_to_locker = decoder.try_decode()?;
        let connector = decoder.try_decode()?;
        let error_message = decoder.try_decode()?;
        let offer_amount = decoder.try_decode()?;
        let surcharge_amount = decoder.try_decode()?;
        let tax_amount = decoder.try_decode()?;
        let payment_method_id = decoder.try_decode()?;
        let payment_method = decoder.try_decode()?;
        let payment_flow = decoder.try_decode()?;
        let redirect = decoder.try_decode()?;
        let connector_transaction_id = decoder.try_decode()?;
        let capture_method = decoder.try_decode()?;
        let capture_on = decoder.try_decode()?;
        let confirm = decoder.try_decode()?;
        let authentication_type = decoder.try_decode()?;
        let created_at = decoder.try_decode()?;
        let modified_at = decoder.try_decode()?;
        let last_synced = decoder.try_decode()?;
        let cancellation_reason = decoder.try_decode()?;
        let amount_to_capture = decoder.try_decode()?;

        Ok(PaymentAttemptNew {
            payment_id,
            merchant_id,
            txn_id,
            status,
            amount,
            currency,
            save_to_locker,
            connector,
            error_message,
            offer_amount,
            surcharge_amount,
            tax_amount,
            payment_method_id,
            payment_method,
            payment_flow,
            redirect,
            connector_transaction_id,
            capture_method,
            capture_on,
            confirm,
            authentication_type,
            created_at,
            modified_at,
            last_synced,
            cancellation_reason,
            amount_to_capture,
            mandate_id: None,
            browser_info: None,
        })
    }
}

#[cfg(feature = "sqlx")]
impl sqlx::Type<sqlx::Postgres> for PaymentAttemptNew {
    fn type_info() -> sqlx::postgres::PgTypeInfo {
        sqlx::postgres::PgTypeInfo::with_name("PaymentAttemptNew")
    }
}

#[derive(Debug, Clone)]
pub enum PaymentAttemptUpdate {
    Update {
        amount: i32,
        currency: enums::Currency,
        status: enums::AttemptStatus,
        authentication_type: Option<enums::AuthenticationType>,
        payment_method: Option<enums::PaymentMethodType>,
    },
    AuthenticationTypeUpdate {
        authentication_type: enums::AuthenticationType,
    },
    ConfirmUpdate {
        status: enums::AttemptStatus,
        payment_method: Option<enums::PaymentMethodType>,
        browser_info: Option<serde_json::Value>,
    },
    VoidUpdate {
        status: enums::AttemptStatus,
        cancellation_reason: Option<String>,
    },
    ResponseUpdate {
        status: enums::AttemptStatus,
        connector_transaction_id: Option<String>,
        authentication_type: Option<enums::AuthenticationType>,
        payment_method_id: Option<Option<String>>,
        redirect: Option<bool>,
        mandate_id: Option<String>,
    },
    StatusUpdate {
        status: enums::AttemptStatus,
    },
    ErrorUpdate {
        status: enums::AttemptStatus,
        error_message: Option<String>,
    },
}

#[derive(Clone, Debug, Default, router_derive::DebugAsDisplay)]
#[cfg_attr(feature = "diesel", derive(AsChangeset))]
#[cfg_attr(feature = "diesel", diesel(table_name = payment_attempt))]
#[allow(dead_code)]
pub(super) struct PaymentAttemptUpdateInternal {
    amount: Option<i32>,
    currency: Option<enums::Currency>,
    status: Option<enums::AttemptStatus>,
    connector_transaction_id: Option<String>,
    authentication_type: Option<enums::AuthenticationType>,
    payment_method: Option<enums::PaymentMethodType>,
    error_message: Option<String>,
    payment_method_id: Option<Option<String>>,
    cancellation_reason: Option<String>,
    modified_at: Option<PrimitiveDateTime>,
    redirect: Option<bool>,
    mandate_id: Option<String>,
    browser_info: Option<serde_json::Value>,
}

impl PaymentAttemptUpdate {
    pub fn apply_changeset(self, source: PaymentAttempt) -> PaymentAttempt {
        let pa_update: PaymentAttemptUpdateInternal = self.into();
        PaymentAttempt {
            amount: pa_update.amount.unwrap_or(source.amount),
            currency: pa_update.currency.or(source.currency),
            status: pa_update.status.unwrap_or(source.status),
            connector_transaction_id: pa_update
                .connector_transaction_id
                .or(source.connector_transaction_id),
            authentication_type: pa_update.authentication_type.or(source.authentication_type),
            payment_method: pa_update.payment_method.or(source.payment_method),
            error_message: pa_update.error_message.or(source.error_message),
            payment_method_id: pa_update
                .payment_method_id
                .unwrap_or(source.payment_method_id),
            browser_info: pa_update.browser_info,
            modified_at: common_utils::date_time::now(),
            ..source
        }
    }
}

impl From<PaymentAttemptUpdate> for PaymentAttemptUpdateInternal {
    fn from(payment_attempt_update: PaymentAttemptUpdate) -> Self {
        match payment_attempt_update {
            PaymentAttemptUpdate::Update {
                amount,
                currency,
                status,
                // connector_transaction_id,
                authentication_type,
                payment_method,
            } => Self {
                amount: Some(amount),
                currency: Some(currency),
                status: Some(status),
                // connector_transaction_id,
                authentication_type,
                payment_method,
                modified_at: Some(common_utils::date_time::now()),
                ..Default::default()
            },
            PaymentAttemptUpdate::AuthenticationTypeUpdate {
                authentication_type,
            } => Self {
                authentication_type: Some(authentication_type),
                modified_at: Some(common_utils::date_time::now()),
                ..Default::default()
            },
            PaymentAttemptUpdate::ConfirmUpdate {
                status,
                payment_method,
                browser_info,
            } => Self {
                status: Some(status),
                payment_method,
                modified_at: Some(common_utils::date_time::now()),
                browser_info,
                ..Default::default()
            },
            PaymentAttemptUpdate::VoidUpdate {
                status,
                cancellation_reason,
            } => Self {
                status: Some(status),
                cancellation_reason,
                ..Default::default()
            },
            PaymentAttemptUpdate::ResponseUpdate {
                status,
                connector_transaction_id,
                authentication_type,
                payment_method_id,
                redirect,
                mandate_id,
            } => Self {
                status: Some(status),
                connector_transaction_id,
                authentication_type,
                payment_method_id,
                modified_at: Some(common_utils::date_time::now()),
                redirect,
                mandate_id,
                ..Default::default()
            },
            PaymentAttemptUpdate::ErrorUpdate {
                status,
                error_message,
            } => Self {
                status: Some(status),
                error_message,
                modified_at: Some(common_utils::date_time::now()),
                ..Default::default()
            },
            PaymentAttemptUpdate::StatusUpdate { status } => Self {
                status: Some(status),
                ..Default::default()
            },
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used)]

    use uuid::Uuid;

    use super::*;
    use crate::{configs::settings::Settings, db::StorageImpl, routes, types};

    #[actix_rt::test]
    #[ignore]
    async fn test_payment_attempt_insert() {
        let conf = Settings::new().expect("invalid settings");

        let state = routes::AppState::with_storage(conf, StorageImpl::DieselPostgresqlTest).await;

        let payment_id = Uuid::new_v4().to_string();
        let current_time = common_utils::date_time::now();
        let payment_attempt = PaymentAttemptNew {
            payment_id: payment_id.clone(),
            connector: types::Connector::Dummy.to_string(),
            created_at: current_time.into(),
            modified_at: current_time.into(),
            ..PaymentAttemptNew::default()
        };

        let response = state
            .store
            .insert_payment_attempt(payment_attempt)
            .await
            .unwrap();
        eprintln!("{:?}", response);

        assert_eq!(response.payment_id, payment_id.clone());
    }

    #[actix_rt::test]
    async fn test_find_payment_attempt() {
        use crate::configs::settings::Settings;
        let conf = Settings::new().expect("invalid settings");
        let state = routes::AppState::with_storage(conf, StorageImpl::DieselPostgresqlTest).await;

        let current_time = common_utils::date_time::now();
        let payment_id = Uuid::new_v4().to_string();
        let merchant_id = Uuid::new_v4().to_string();

        let payment_attempt = PaymentAttemptNew {
            payment_id: payment_id.clone(),
            merchant_id: merchant_id.clone(),
            connector: types::Connector::Dummy.to_string(),
            created_at: current_time.into(),
            modified_at: current_time.into(),
            ..PaymentAttemptNew::default()
        };
        state
            .store
            .insert_payment_attempt(payment_attempt)
            .await
            .unwrap();

        let response = state
            .store
            .find_payment_attempt_by_payment_id_merchant_id(&payment_id, &merchant_id)
            .await
            .unwrap();

        eprintln!("{:?}", response);

        assert_eq!(response.payment_id, payment_id);
    }

    #[actix_rt::test]
    async fn test_payment_attempt_mandate_field() {
        use crate::configs::settings::Settings;
        let conf = Settings::new().expect("invalid settings");
        let uuid = uuid::Uuid::new_v4().to_string();
        let state = routes::AppState::with_storage(conf, StorageImpl::DieselPostgresqlTest).await;
        let current_time = common_utils::date_time::now();

        let payment_attempt = PaymentAttemptNew {
            payment_id: uuid.clone(),
            merchant_id: "1".to_string(),
            connector: types::Connector::Dummy.to_string(),
            created_at: current_time.into(),
            modified_at: current_time.into(),
            // Adding a mandate_id
            mandate_id: Some("man_121212".to_string()),
            ..PaymentAttemptNew::default()
        };
        state
            .store
            .insert_payment_attempt(payment_attempt)
            .await
            .unwrap();

        let response = state
            .store
            .find_payment_attempt_by_payment_id_merchant_id(&uuid, "1")
            .await
            .unwrap();
        // checking it after fetch
        assert_eq!(response.mandate_id, Some("man_121212".to_string()));
    }
}
