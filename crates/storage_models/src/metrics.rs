use router_env::{counter_metric, global_meter, histogram_metric, metrics_context};

metrics_context!(CONTEXT);
global_meter!(GLOBAL_METER, "ROUTER_API");

histogram_metric!(DB_REQUEST_TIME, GLOBAL_METER);
counter_metric!(DB_REQUEST_COUNT, GLOBAL_METER);

mod database_metric {
    use router_env::opentelemetry::KeyValue;
    use std::{future::Future, time::Instant};

    use super::*;
    #[derive(Debug)]
    pub enum DatabaseCallType {
        Create,
        Read,
        Update,
        Delete,
    }

    impl DatabaseCallType {
        fn as_str(&self) -> &'static str {

            match self {
                DatabaseCallType::Create => "create",
                DatabaseCallType::Read => "read",
                DatabaseCallType::Update => "update",
                DatabaseCallType::Delete => "delete",
            }
        }
    }

    pub async fn time_database_call<F: FnOnce() -> Fut, Fut: Future<Output = R>, R>(
        call_type: DatabaseCallType,
        future: F,
        table: impl ToString,
    ) -> R {
        let start = Instant::now();
        let output = future().await;
        DB_REQUEST_COUNT.add(
            &CONTEXT,
            1,
            &[
                KeyValue::new("call_type", call_type.as_str()),
                KeyValue::new("table", table.to_string()),
            ],
        );

        output
    }
}
