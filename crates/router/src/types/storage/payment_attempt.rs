use diesel::{AsChangeset, Identifiable, Insertable, Queryable};
use serde::{Deserialize, Serialize};
use time::PrimitiveDateTime;

use crate::{schema::payment_attempt, types::enums, utils::date_time};

#[derive(Clone, Debug, Eq, PartialEq, Identifiable, Queryable, Serialize, Deserialize)]
#[diesel(table_name = payment_attempt)]
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

#[derive(Clone, Debug, Default, Insertable, router_derive::DebugAsDisplay)]
#[diesel(table_name = payment_attempt)]
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

#[derive(Clone, Debug)]
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

#[derive(Clone, Debug, Default, AsChangeset, router_derive::DebugAsDisplay)]
#[diesel(table_name = payment_attempt)]
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
            modified_at: date_time::now(),
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
                modified_at: Some(crate::utils::date_time::now()),
                ..Default::default()
            },
            PaymentAttemptUpdate::AuthenticationTypeUpdate {
                authentication_type,
            } => Self {
                authentication_type: Some(authentication_type),
                modified_at: Some(crate::utils::date_time::now()),
                ..Default::default()
            },
            PaymentAttemptUpdate::ConfirmUpdate {
                status,
                payment_method,
            } => Self {
                status: Some(status),
                payment_method,
                modified_at: Some(crate::utils::date_time::now()),
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
                modified_at: Some(crate::utils::date_time::now()),
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
                modified_at: Some(crate::utils::date_time::now()),
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
    use super::*;
    use crate::{
        configs::settings::Settings, db::payment_attempt::IPaymentAttempt, routes, services::Store,
        types,
    };

    #[actix_rt::test]
    async fn test_payment_attempt_insert() {
        let conf = Settings::new().expect("invalid settings");

        let state = routes::AppState {
            flow_name: String::from("default"),
            store: Store::new(&conf).await,
            conf,
        };

        // let conn = config.conn.get();

        let current_time = crate::utils::date_time::now();
        let payment_attempt = PaymentAttemptNew {
            payment_id: "1".to_string(),
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

        assert_eq!(response.payment_id, "1");
    }

    #[actix_rt::test]
    async fn test_find_payment_attempt() {
        use crate::configs::settings::Settings;
        let conf = Settings::new().expect("invalid settings");

        let state = routes::AppState {
            flow_name: String::from("default"),
            store: Store::new(&conf).await,
            conf,
        };
        let current_time = crate::utils::date_time::now();

        let payment_attempt = PaymentAttemptNew {
            payment_id: "1".to_string(),
            merchant_id: "1".to_string(),
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
            .find_payment_attempt_by_payment_id_merchant_id("1", "1")
            .await
            .unwrap();

        eprintln!("{:?}", response);

        assert_eq!(response.payment_id, "1");
    }

    #[actix_rt::test]
    async fn test_payment_attempt_mandate_field() {
        use crate::configs::settings::Settings;
        let conf = Settings::new().expect("invalid settings");
        let uuid = uuid::Uuid::new_v4().to_string();
        let state = routes::AppState {
            flow_name: String::from("default"),
            store: Store::new(&conf).await,
            conf,
        };
        let current_time = crate::utils::date_time::now();

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
